//! Cost categories, landlord, PDF template and VPI repositories: isolation, categories in
//! use, one row per account, global values.

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

use vermieterme_db::repo::account;
use vermieterme_db::repo::masterdata::{
    Rejection, cost_category, landlord, pdf_template, property, vpi,
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

/// Categories: isolation and the refusal to delete one that costs use.
async fn category_rules(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let (a, b) = two_scopes(&mut c).await;
    let input = cost_category::CostCategoryInput {
        name: "Wasser".into(),
        distribution_key: "mea".into(),
        sort_order: 2,
    };
    let cat = cost_category::create(&mut c, &a, &input).await.unwrap();
    let first = cost_category::CostCategoryInput {
        name: "Müll".into(),
        sort_order: 1,
        ..input.clone()
    };
    cost_category::create(&mut c, &a, &first).await.unwrap();
    let names: Vec<_> = cost_category::list(&mut c, &a)
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.name)
        .collect();
    assert_eq!(names, ["Müll", "Wasser"]);
    assert!(cost_category::list(&mut c, &b).await.unwrap().is_empty());
    assert!(matches!(
        cost_category::update(&mut c, &b, &cat.id, &input).await,
        Err(DbError::NotFound)
    ));
    assert!(matches!(
        cost_category::delete(&mut c, &b, &cat.id).await,
        Err(DbError::NotFound)
    ));

    let p = property::create(&mut c, &a, &prop(100)).await.unwrap();
    for sql in [
        "INSERT INTO billing_period (id, account_id, property_id, start_date, end_date, \
         created_at, updated_at) VALUES ('bp1', ?, ?, '2024-01-01', '2024-12-31', 1, 1)",
        "INSERT INTO cost (id, account_id, billing_period_id, cost_category_id, created_at, \
         updated_at) VALUES ('c1', ?, 'bp1', ?, 1, 1)",
    ] {
        let second = if sql.contains("billing_period (") {
            &p.id
        } else {
            &cat.id
        };
        sqlx::query(sql)
            .bind(a.account_id())
            .bind(second)
            .execute(&mut *c)
            .await
            .unwrap();
    }
    let refused = cost_category::delete(&mut c, &a, &cat.id).await.unwrap();
    assert_eq!(refused.unwrap_err(), Rejection::CategoryInUse);
    assert_eq!(cost_category::list(&mut c, &a).await.unwrap().len(), 2);
    sqlx::query("DELETE FROM cost WHERE id = 'c1'")
        .execute(&mut *c)
        .await
        .unwrap();
    cost_category::delete(&mut c, &a, &cat.id)
        .await
        .unwrap()
        .unwrap();
}
dual_test!(category_in_use, category_rules);

/// Landlord and template: one row per account, saved twice updates, accounts separate.
async fn one_row_per_account(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let (a, b) = two_scopes(&mut c).await;
    assert_eq!(landlord::get(&mut c, &a).await.unwrap(), None);
    let data = landlord::LandlordData {
        name: "Max Vermieter".into(),
        street: "Weg 2".into(),
        zip: "54321".into(),
        city: "Stadt".into(),
        phone: None,
        email: None,
        bank_name: None,
        iban: None,
        account_holder: None,
    };
    let first = landlord::save(&mut c, &a, &data).await.unwrap();
    let again = landlord::LandlordData {
        phone: Some("0123".into()),
        ..data.clone()
    };
    let second = landlord::save(&mut c, &a, &again).await.unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(second.data.phone.as_deref(), Some("0123"));
    assert_eq!(landlord::get(&mut c, &b).await.unwrap(), None);

    assert_eq!(pdf_template::get(&mut c, &a).await.unwrap(), None);
    let t = pdf_template::save(&mut c, &a, "{\"a\":1}", None)
        .await
        .unwrap();
    assert_eq!(t.name, "Standard");
    let t2 = pdf_template::save(&mut c, &a, "{\"a\":2}", Some("Neu"))
        .await
        .unwrap();
    assert_eq!((t2.id.as_str(), t2.name.as_str()), (t.id.as_str(), "Neu"));
    let t3 = pdf_template::save(&mut c, &a, "{\"a\":3}", None)
        .await
        .unwrap();
    assert_eq!(t3.name, "Neu", "ohne Namen bleibt der Name");
    assert_eq!(pdf_template::get(&mut c, &b).await.unwrap(), None);
}
dual_test!(landlord_template_single_row, one_row_per_account);

/// VPI: the same period is updated, not duplicated; delete by id.
async fn vpi_values(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    let admin = AdminScope::system();
    let entry = vpi::VpiRow {
        year: 2024,
        month: 5,
        value_tenths: 1196,
        base_year: 2020,
    };
    let first = vpi::upsert(&mut c, &admin, &entry).await.unwrap();
    let changed = vpi::VpiRow {
        value_tenths: 1200,
        ..entry
    };
    let second = vpi::upsert(&mut c, &admin, &changed).await.unwrap();
    assert_eq!(first.id, second.id);
    let list = vpi::list(&mut c).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].entry.value_tenths, 1200);
    vpi::delete(&mut c, &admin, &first.id).await.unwrap();
    assert!(matches!(
        vpi::delete(&mut c, &admin, &first.id).await,
        Err(DbError::NotFound)
    ));
}
dual_test!(vpi_upsert, vpi_values);
