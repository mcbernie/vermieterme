//! Master data repositories: scope isolation (foreign id -> NotFound), the share rules,
//! categories in use, one row per account, and the global VPI values.

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

use vermieterme_db::repo::account;
use vermieterme_db::repo::masterdata::{
    Rejection, property, rent_change, suggestions, tenant, unit,
};
use vermieterme_db::{AccountScope, AdminScope, AnyConnection, Db, DbError, dual_test};

/// Two accounts.
async fn two_scopes(c: &mut AnyConnection) -> (AccountScope, AccountScope) {
    let admin = AdminScope::system();
    let a = account::create(c, &admin, "A").await.unwrap();
    let b = account::create(c, &admin, "B").await.unwrap();
    (admin.act_in(a.id), admin.act_in(b.id))
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

/// A tenant input; index lease when `index` is set.
fn tenant_data(unit_id: &str, index: bool) -> tenant::TenantData {
    tenant::TenantData {
        unit_id: unit_id.into(),
        salutation: "Frau".into(),
        first_name: "Erika".into(),
        last_name: "Muster".into(),
        salutation2: None,
        first_name2: None,
        last_name2: None,
        phone: None,
        email: Some("erika@example.org".into()),
        bank_name: None,
        iban: None,
        account_holder: None,
        move_in_date: "2024-01-01".into(),
        move_out_date: None,
        lease_type: if index { "index" } else { "standard" }.into(),
        index_base_year: index.then_some(2020),
        index_reference_tenths: index.then_some(1000),
        index_reference_date: None,
        index_min_months: 12,
    }
}

/// Properties and units: counts, scope isolation, share rules in both directions.
async fn property_and_unit_rules(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let (a, b) = two_scopes(&mut c).await;
    let p = property::create(&mut c, &a, &prop(100)).await.unwrap();
    let u = unit::create(&mut c, &a, &unit_input(&p.id, 60))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unit::KINDS[0], u.kind);

    let list = property::list(&mut c, &a).await.unwrap();
    assert_eq!((list.len(), list[0].unit_count), (1, 1));
    assert!(property::list(&mut c, &b).await.unwrap().is_empty());
    assert!(unit::list(&mut c, &b, None).await.unwrap().is_empty());
    assert!(matches!(
        property::get(&mut c, &b, &p.id).await,
        Err(DbError::NotFound)
    ));
    assert!(matches!(
        property::update(&mut c, &b, &p.id, &prop(200)).await,
        Err(DbError::NotFound)
    ));
    assert!(matches!(
        property::delete(&mut c, &b, &p.id).await,
        Err(DbError::NotFound)
    ));
    assert!(matches!(
        unit::create(&mut c, &b, &unit_input(&p.id, 1)).await,
        Err(DbError::NotFound)
    ));
    assert!(matches!(
        unit::update(&mut c, &b, &u.id, &unit_input(&p.id, 1)).await,
        Err(DbError::NotFound)
    ));
    assert!(matches!(
        unit::delete(&mut c, &b, &u.id).await,
        Err(DbError::NotFound)
    ));

    let too_many = unit::create(&mut c, &a, &unit_input(&p.id, 101)).await;
    assert_eq!(too_many.unwrap().unwrap_err(), Rejection::SharesAboveTotal);
    let sum_is_not_checked = unit::create(&mut c, &a, &unit_input(&p.id, 100)).await;
    assert!(sum_is_not_checked.unwrap().is_ok());
    let lower = property::update(&mut c, &a, &p.id, &prop(99)).await;
    assert_eq!(lower.unwrap().unwrap_err(), Rejection::TotalBelowUnitShares);
    assert_eq!(
        property::get(&mut c, &a, &p.id).await.unwrap().total_shares,
        100
    );
    let raise = property::update(&mut c, &a, &p.id, &prop(150)).await;
    assert_eq!(raise.unwrap().unwrap().total_shares, 150);
    let over = unit::update(&mut c, &a, &u.id, &unit_input(&p.id, 151)).await;
    assert_eq!(over.unwrap().unwrap_err(), Rejection::SharesAboveTotal);

    let units = unit::list(&mut c, &a, Some(&p.id)).await.unwrap();
    assert_eq!(units.len(), 2);
    property::delete(&mut c, &a, &p.id).await.unwrap();
    assert!(unit::list(&mut c, &a, None).await.unwrap().is_empty());
}
dual_test!(property_unit_rules, property_and_unit_rules);

/// Tenants and rent changes: foreign units are NotFound, foreign rows are invisible.
async fn tenant_and_rent_scope(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let (a, b) = two_scopes(&mut c).await;
    let p = property::create(&mut c, &a, &prop(100)).await.unwrap();
    let u = unit::create(&mut c, &a, &unit_input(&p.id, 10))
        .await
        .unwrap()
        .unwrap();

    assert!(matches!(
        tenant::create(&mut c, &b, &tenant_data(&u.id, false)).await,
        Err(DbError::NotFound)
    ));
    let t = tenant::create(&mut c, &a, &tenant_data(&u.id, false))
        .await
        .unwrap();
    assert_eq!(t.data.email.as_deref(), Some("erika@example.org"));
    assert_eq!(
        tenant::list_for_property(&mut c, &a, &p.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        tenant::list_for_property(&mut c, &b, &p.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        tenant::list(&mut c, &b, Some(&u.id))
            .await
            .unwrap()
            .is_empty()
    );
    for result in [
        tenant::get(&mut c, &b, &t.id).await.map(|_| ()),
        tenant::update(&mut c, &b, &t.id, &tenant_data(&u.id, false))
            .await
            .map(|_| ()),
        tenant::delete(&mut c, &b, &t.id).await,
    ] {
        assert!(matches!(result, Err(DbError::NotFound)));
    }
    let mut changed = tenant_data(&u.id, true);
    changed.phone = Some("0123".into());
    let updated = tenant::update(&mut c, &a, &t.id, &changed).await.unwrap();
    assert_eq!(updated.data, changed);

    let change = rent_change::RentChangeData {
        unit_id: u.id.clone(),
        change_type: "rent".into(),
        amount_cents: 65_000,
        effective_date: "2024-01-01".into(),
        reason: None,
    };
    assert!(matches!(
        rent_change::create(&mut c, &b, &change).await,
        Err(DbError::NotFound)
    ));
    let r = rent_change::create(&mut c, &a, &change).await.unwrap();
    assert!(
        rent_change::list(&mut c, &b, None)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        rent_change::delete(&mut c, &b, &r.id).await,
        Err(DbError::NotFound)
    ));
    let newer = rent_change::RentChangeData {
        amount_cents: 70_000,
        effective_date: "2025-01-01".into(),
        ..change.clone()
    };
    rent_change::create(&mut c, &a, &newer).await.unwrap();
    let list = rent_change::list(&mut c, &a, Some(&u.id)).await.unwrap();
    assert_eq!(list[0].data.amount_cents, 70_000, "latest date first");

    let index = suggestions::index_tenants(&mut c, &a).await.unwrap();
    assert_eq!(index.len(), 1);
    assert_eq!(index[0].current_rent_cents, Some(70_000));
    assert!(
        suggestions::index_tenants(&mut c, &b)
            .await
            .unwrap()
            .is_empty()
    );

    tenant::delete(&mut c, &a, &t.id).await.unwrap();
    assert!(
        suggestions::index_tenants(&mut c, &a)
            .await
            .unwrap()
            .is_empty()
    );
}
dual_test!(tenant_rent_scope, tenant_and_rent_scope);
