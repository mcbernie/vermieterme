//! Parallel changes of master data on two connections: the share rules hold under
//! concurrency, first saves of one-row-per-account data do not collide, and moving a
//! tenant or a unit follows its rules (tokens revoked, units in billings stay).

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

use sqlx::Row;
use vermieterme_db::repo::account;
use vermieterme_db::repo::masterdata::{
    Rejection, landlord, pdf_template, property, tenant, unit, vpi,
};
use vermieterme_db::{AccountScope, AdminScope, Db, dual_test};

/// A scope of a new account.
async fn scope(db: &Db, name: &str) -> AccountScope {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();
    let a = account::create(&mut c, &admin, name).await.unwrap();
    admin.act_in(a.id)
}

/// A property with `total_shares`.
fn prop(total_shares: i64) -> property::PropertyInput {
    property::PropertyInput {
        street: "Musterweg 1".into(),
        zip: "12345".into(),
        city: "Beispielstadt".into(),
        total_shares,
    }
}

/// A unit input.
fn unit_input(property_id: &str, shares: i64) -> unit::UnitInput {
    unit::UnitInput {
        property_id: property_id.into(),
        name: "Wohnung 1".into(),
        floor: "EG".into(),
        shares,
        kind: "apartment".into(),
    }
}

/// A tenant in `unit_id`.
fn tenant_data(unit_id: &str) -> tenant::TenantData {
    tenant::TenantData {
        unit_id: unit_id.into(),
        salutation: "Frau".into(),
        first_name: "Erika".into(),
        last_name: "Muster".into(),
        salutation2: None,
        first_name2: None,
        last_name2: None,
        phone: None,
        email: None,
        bank_name: None,
        iban: None,
        account_holder: None,
        move_in_date: "2024-01-01".into(),
        move_out_date: None,
        lease_type: "standard".into(),
        index_base_year: None,
        index_reference_tenths: None,
        index_reference_date: None,
        index_min_months: 12,
    }
}

/// A property total lowered to 50 and a unit raised to 80 shares (of 100) at the same
/// time: each change alone is fine, together they break the rule, so exactly one must be
/// refused whichever runs first. Repeated, because a missing lock would show only
/// sometimes.
async fn parallel_share_checks(db: &Db) {
    for round in 0..8 {
        let a = scope(db, &format!("A{round}")).await;
        let mut c1 = db.pool().acquire().await.unwrap();
        let mut c2 = db.pool().acquire().await.unwrap();
        let p = property::create(&mut c1, &a, &prop(100)).await.unwrap();
        let u = unit::create(&mut c1, &a, &unit_input(&p.id, 10))
            .await
            .unwrap()
            .unwrap();
        let (low, high) = (prop(50), unit_input(&p.id, 80));
        let (lower, raise) = tokio::join!(
            property::update(&mut c1, &a, &p.id, &low),
            unit::update(&mut c2, &a, &u.id, &high),
        );
        let (lower, raise) = (lower.unwrap(), raise.unwrap());
        assert!(
            lower.is_ok() != raise.is_ok(),
            "genau eine Änderung muss scheitern: {lower:?} / {raise:?}"
        );
        let stored = property::get(&mut c1, &a, &p.id).await.unwrap();
        let unit_now = unit::get(&mut c1, &a, &u.id).await.unwrap();
        assert!(unit_now.shares <= stored.total_shares);
        if let Err(rejection) = lower {
            assert_eq!(rejection, Rejection::TotalBelowUnitShares);
        }
        if let Err(rejection) = raise {
            assert_eq!(rejection, Rejection::SharesAboveTotal);
        }
    }
}
dual_test!(shares_parallel, parallel_share_checks);

/// Two first saves at the same time: both succeed, one row remains.
async fn parallel_first_saves(db: &Db) {
    let a = scope(db, "A").await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    let data = |name: &str| landlord::LandlordData {
        name: name.into(),
        street: "Weg 2".into(),
        zip: "54321".into(),
        city: "Stadt".into(),
        phone: None,
        email: None,
        bank_name: None,
        iban: None,
        account_holder: None,
    };
    let (one, two) = (data("Eins"), data("Zwei"));
    let (x, y) = tokio::join!(
        landlord::save(&mut c1, &a, &one),
        landlord::save(&mut c2, &a, &two),
    );
    assert_eq!(x.unwrap().id, y.unwrap().id);
    let (x, y) = tokio::join!(
        pdf_template::save(&mut c1, &a, "{\"a\":1}", None),
        pdf_template::save(&mut c2, &a, "{\"a\":2}", None),
    );
    assert_eq!(x.unwrap().id, y.unwrap().id);
    let entry = vpi::VpiRow {
        year: 2024,
        month: 5,
        value_tenths: 1196,
        base_year: 2020,
    };
    let admin = AdminScope::system();
    let other = vpi::VpiRow {
        value_tenths: 1200,
        ..entry
    };
    let (x, y) = tokio::join!(
        vpi::upsert(&mut c1, &admin, &entry),
        vpi::upsert(&mut c2, &admin, &other),
    );
    assert_eq!(x.unwrap().id, y.unwrap().id);
    assert_eq!(vpi::list(&mut c1).await.unwrap().len(), 1);
    for (table, sql) in [
        ("landlord", "SELECT COUNT(*) AS n FROM landlord"),
        ("pdf_template", "SELECT COUNT(*) AS n FROM pdf_template"),
    ] {
        let n: i64 = sqlx::query(sql).fetch_one(&mut *c1).await.unwrap().get("n");
        assert_eq!(n, 1, "{table}");
    }
}
dual_test!(first_saves_parallel, parallel_first_saves);

/// The revocation time of the tenant-app tokens of a tenant.
async fn revoked_at(db: &Db, id: &str) -> Option<i64> {
    let mut c = db.pool().acquire().await.unwrap();
    sqlx::query("SELECT app_tokens_revoked_at AS r FROM tenant WHERE id = ?")
        .bind(id)
        .fetch_one(&mut *c)
        .await
        .unwrap()
        .get("r")
}

/// Moving a tenant to another unit revokes the app tokens (decision 36); other changes
/// leave them alone.
async fn tenant_move_revokes_tokens(db: &Db) {
    let a = scope(db, "A").await;
    let mut c = db.pool().acquire().await.unwrap();
    let p = property::create(&mut c, &a, &prop(100)).await.unwrap();
    let u1 = unit::create(&mut c, &a, &unit_input(&p.id, 10))
        .await
        .unwrap()
        .unwrap();
    let u2 = unit::create(&mut c, &a, &unit_input(&p.id, 20))
        .await
        .unwrap()
        .unwrap();
    let t = tenant::create(&mut c, &a, &tenant_data(&u1.id))
        .await
        .unwrap();
    let mut edit = tenant_data(&u1.id);
    edit.phone = Some("0123".into());
    tenant::update(&mut c, &a, &t.id, &edit).await.unwrap();
    assert_eq!(revoked_at(db, &t.id).await, None);
    tenant::update(&mut c, &a, &t.id, &tenant_data(&u2.id))
        .await
        .unwrap();
    let first = revoked_at(db, &t.id).await;
    assert!(first.is_some());
    // Same unit again: untouched; move back: a new, later revocation time.
    tenant::update(&mut c, &a, &t.id, &tenant_data(&u2.id))
        .await
        .unwrap();
    assert_eq!(revoked_at(db, &t.id).await, first);
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    tenant::update(&mut c, &a, &t.id, &tenant_data(&u1.id))
        .await
        .unwrap();
    assert!(revoked_at(db, &t.id).await > first);
}
dual_test!(tenant_move_tokens, tenant_move_revokes_tokens);

/// A unit with a prepayment cannot change the property (decision 37); without billing
/// data it can.
async fn unit_move_rules(db: &Db) {
    let a = scope(db, "A").await;
    let mut c = db.pool().acquire().await.unwrap();
    let p1 = property::create(&mut c, &a, &prop(100)).await.unwrap();
    let p2 = property::create(&mut c, &a, &prop(100)).await.unwrap();
    let free = unit::create(&mut c, &a, &unit_input(&p1.id, 10))
        .await
        .unwrap()
        .unwrap();
    let used = unit::create(&mut c, &a, &unit_input(&p1.id, 10))
        .await
        .unwrap()
        .unwrap();
    sqlx::query(
        "INSERT INTO billing_period (id, account_id, property_id, start_date, end_date, \
         created_at, updated_at) VALUES ('bp1', ?, ?, '2024-01-01', '2024-12-31', 1, 1)",
    )
    .bind(a.account_id())
    .bind(&p1.id)
    .execute(&mut *c)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO prepayment (id, account_id, billing_period_id, unit_id, \
         monthly_amount_cents, created_at, updated_at) VALUES ('pp1', ?, 'bp1', ?, 5000, 1, 1)",
    )
    .bind(a.account_id())
    .bind(&used.id)
    .execute(&mut *c)
    .await
    .unwrap();

    let refused = unit::update(&mut c, &a, &used.id, &unit_input(&p2.id, 10)).await;
    assert_eq!(refused.unwrap().unwrap_err(), Rejection::UnitInBillings);
    // Staying in the property stays possible.
    let same = unit::update(&mut c, &a, &used.id, &unit_input(&p1.id, 15)).await;
    assert!(same.unwrap().is_ok());
    let moved = unit::update(&mut c, &a, &free.id, &unit_input(&p2.id, 10)).await;
    assert_eq!(moved.unwrap().unwrap().property_id, p2.id);
}
dual_test!(unit_move, unit_move_rules);
