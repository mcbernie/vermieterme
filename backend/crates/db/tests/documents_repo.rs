//! Documents repository: create with assignment checks, list filters, delete returning the
//! stored file, the file names of cascading deletes (#106) and the tenant-app visibility
//! rule (#68, #71); foreign ids behave like missing ones.

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

mod billing_support;

use billing_support::{new_period, property, setup, unit};
use vermieterme_db::repo::billing::period;
use vermieterme_db::repo::documents::{self, NewDocument, Owner, files_of, tenant_view};
use vermieterme_db::repo::masterdata::tenant::{self, TenantData};
use vermieterme_db::{AccountScope, Db, DbError, TenantScope, dual_test};

/// A tenant in `unit_id`; returns its id.
async fn tenant(db: &Db, scope: &AccountScope, unit_id: &str) -> String {
    let mut c = db.pool().acquire().await.unwrap();
    let data = TenantData {
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
    };
    tenant::create(&mut c, scope, &data).await.unwrap().id
}

/// Stores a document with the file name `file` and the given assignment.
async fn doc(
    db: &Db,
    scope: &AccountScope,
    file: &str,
    period: Option<&str>,
    tenant: Option<&str>,
) -> Result<String, DbError> {
    let mut c = db.pool().acquire().await.unwrap();
    let new = NewDocument {
        file_name: file.into(),
        original_name: "Rechnung.pdf".into(),
        mime_type: "application/pdf".into(),
        size_bytes: 42,
        category: "invoice".into(),
        billing_period_id: period.map(Into::into),
        tenant_id: tenant.map(Into::into),
    };
    documents::create(&mut c, scope, &new).await.map(|d| d.id)
}

/// Sorted file names of `owner`'s cascade.
async fn files(db: &Db, scope: &AccountScope, owner: Owner<'_>) -> Result<Vec<String>, DbError> {
    let mut c = db.pool().acquire().await.unwrap();
    let mut names = files_of(&mut c, scope, owner).await?;
    names.sort();
    Ok(names)
}

/// Create, list with filters, get, delete; foreign assignments and ids are not found.
async fn crud_and_scope(db: &Db) {
    let s = setup(db, "A").await;
    let other = setup(db, "B").await;
    let t = tenant(db, &s.scope, &s.units[0]).await;
    let p = s.period.id.as_str();
    let d1 = doc(db, &s.scope, "f1.pdf", Some(p), None).await.unwrap();
    let d2 = doc(db, &s.scope, "f2.pdf", Some(p), Some(&t))
        .await
        .unwrap();
    let d3 = doc(db, &s.scope, "f3.pdf", None, None).await.unwrap();

    let mut c = db.pool().acquire().await.unwrap();
    let all = documents::list(&mut c, &s.scope, None, None).await.unwrap();
    assert_eq!(all.len(), 3);
    let ids = |l: Vec<documents::Document>| l.into_iter().map(|d| d.id).collect::<Vec<_>>();
    let of_period = documents::list(&mut c, &s.scope, Some(p), None)
        .await
        .unwrap();
    assert_eq!(of_period.len(), 2);
    assert_eq!(
        ids(documents::list(&mut c, &s.scope, Some(p), Some(&t))
            .await
            .unwrap()),
        vec![d2.clone()]
    );
    assert!(
        documents::list(&mut c, &other.scope, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    let got = documents::get(&mut c, &s.scope, &d1).await.unwrap();
    assert_eq!((got.size_bytes, got.category.as_str()), (42, "invoice"));
    assert!(matches!(
        documents::get(&mut c, &other.scope, &d1).await,
        Err(DbError::NotFound)
    ));
    assert!(matches!(
        documents::delete(&mut c, &other.scope, &d3).await,
        Err(DbError::NotFound)
    ));
    assert_eq!(
        documents::delete(&mut c, &s.scope, &d3).await.unwrap(),
        "f3.pdf"
    );
    assert!(matches!(
        documents::get(&mut c, &s.scope, &d3).await,
        Err(DbError::NotFound)
    ));
    drop(c);

    // Assignments of another account are not found, nothing is stored.
    let foreign_period = Some(other.period.id.as_str());
    assert!(matches!(
        doc(db, &s.scope, "x.pdf", foreign_period, None).await,
        Err(DbError::NotFound)
    ));
    let foreign_tenant = tenant(db, &other.scope, &other.units[0]).await;
    assert!(matches!(
        doc(db, &s.scope, "x.pdf", None, Some(&foreign_tenant)).await,
        Err(DbError::NotFound)
    ));
    let mut c = db.pool().acquire().await.unwrap();
    assert_eq!(
        documents::list(&mut c, &s.scope, None, None)
            .await
            .unwrap()
            .len(),
        2
    );
}

dual_test!(crud_and_scope_cases, crud_and_scope);

/// The file names each cascading delete takes along, and only those.
async fn cascade_files(db: &Db) {
    let s = setup(db, "A").await;
    let other = setup(db, "B").await;
    let p = s.period.id.as_str();
    let t0 = tenant(db, &s.scope, &s.units[0]).await;
    let t1 = tenant(db, &s.scope, &s.units[1]).await;
    doc(db, &s.scope, "period.pdf", Some(p), None)
        .await
        .unwrap();
    doc(db, &s.scope, "t0.pdf", None, Some(&t0)).await.unwrap();
    doc(db, &s.scope, "t1.pdf", Some(p), Some(&t1))
        .await
        .unwrap();
    doc(db, &s.scope, "loose.pdf", None, None).await.unwrap();
    // A second property of the same account must not be reached.
    let p2 = property(db, &s.scope).await;
    let u2 = unit(db, &s.scope, &p2, 10).await;
    let t2 = tenant(db, &s.scope, &u2).await;
    doc(db, &s.scope, "p2.pdf", None, Some(&t2)).await.unwrap();

    let names = |list: &[&str]| list.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
    assert_eq!(
        files(db, &s.scope, Owner::Property(&s.property_id))
            .await
            .unwrap(),
        names(&["period.pdf", "t0.pdf", "t1.pdf"])
    );
    assert_eq!(
        files(db, &s.scope, Owner::Unit(&s.units[0])).await.unwrap(),
        names(&["t0.pdf"])
    );
    assert_eq!(
        files(db, &s.scope, Owner::Tenant(&t1)).await.unwrap(),
        names(&["t1.pdf"])
    );
    assert_eq!(
        files(db, &s.scope, Owner::BillingPeriod(p)).await.unwrap(),
        names(&["period.pdf", "t1.pdf"])
    );
    for owner in [
        Owner::Property(&s.property_id),
        Owner::Unit(&s.units[0]),
        Owner::Tenant(&t0),
        Owner::BillingPeriod(p),
    ] {
        assert!(
            matches!(files(db, &other.scope, owner).await, Err(DbError::NotFound)),
            "{owner:?}"
        );
    }

    // The cascade of the schema removes exactly these rows.
    let mut c = db.pool().acquire().await.unwrap();
    period::delete(&mut c, &s.scope, p).await.unwrap();
    let left = documents::list(&mut c, &s.scope, None, None).await.unwrap();
    let mut left: Vec<_> = left.into_iter().map(|d| d.file_name).collect();
    left.sort();
    assert_eq!(left, names(&["loose.pdf", "p2.pdf", "t0.pdf"]));
}

dual_test!(cascade_files_cases, cascade_files);

/// Tenant app: own documents and period documents without tenant; never those of another
/// tenant of the same period (#71), never unassigned ones (#68), never other properties.
async fn tenant_visibility(db: &Db) {
    let s = setup(db, "A").await;
    let p = s.period.id.as_str();
    let a = tenant(db, &s.scope, &s.units[0]).await;
    let b = tenant(db, &s.scope, &s.units[1]).await;
    let own = doc(db, &s.scope, "own.pdf", None, Some(&a)).await.unwrap();
    let own_period = doc(db, &s.scope, "a.pdf", Some(p), Some(&a)).await.unwrap();
    let b_period = doc(db, &s.scope, "b.pdf", Some(p), Some(&b)).await.unwrap();
    let shared = doc(db, &s.scope, "shared.pdf", Some(p), None)
        .await
        .unwrap();
    let loose = doc(db, &s.scope, "loose.pdf", None, None).await.unwrap();
    // A period of a second property (a period needs a unit).
    let p2 = property(db, &s.scope).await;
    unit(db, &s.scope, &p2, 10).await;
    let mut c = db.pool().acquire().await.unwrap();
    let input = new_period(&p2, "2024-01-01", "2024-12-31");
    let period2 = period::create(&mut c, &s.scope, &input)
        .await
        .unwrap()
        .unwrap();
    drop(c);
    let elsewhere = doc(db, &s.scope, "p2.pdf", Some(&period2.id), None)
        .await
        .unwrap();

    let scope_of = |t: &str, u: &str| {
        TenantScope::from_verified_tenant(s.scope.account_id(), t, u, &s.property_id)
    };
    let (sa, sb) = (scope_of(&a, &s.units[0]), scope_of(&b, &s.units[1]));
    let mut c = db.pool().acquire().await.unwrap();
    let seen = |list: Vec<documents::Document>| {
        let mut ids: Vec<_> = list.into_iter().map(|d| d.id).collect();
        ids.sort();
        ids
    };
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };
    assert_eq!(
        seen(tenant_view::list(&mut c, &sa).await.unwrap()),
        sorted(vec![own.clone(), own_period.clone(), shared.clone()])
    );
    assert_eq!(
        seen(tenant_view::list(&mut c, &sb).await.unwrap()),
        sorted(vec![b_period.clone(), shared.clone()])
    );
    assert_eq!(
        seen(tenant_view::list_of_period(&mut c, &sa, p).await.unwrap()),
        sorted(vec![own_period.clone(), shared.clone()])
    );
    assert_eq!(tenant_view::get(&mut c, &sa, &own).await.unwrap().id, own);
    for hidden in [&b_period, &loose, &elsewhere] {
        assert!(matches!(
            tenant_view::get(&mut c, &sa, hidden).await,
            Err(DbError::NotFound)
        ));
    }
    assert!(matches!(
        tenant_view::get(&mut c, &sb, &own_period).await,
        Err(DbError::NotFound)
    ));
}

dual_test!(tenant_visibility_cases, tenant_visibility);

/// Period and tenant of different properties are refused, nothing stored (decision 43).
async fn mixed_properties(db: &Db) {
    let s = setup(db, "A").await;
    let elsewhere = unit(db, &s.scope, &property(db, &s.scope).await, 10).await;
    let t = tenant(db, &s.scope, &elsewhere).await;
    let made = doc(db, &s.scope, "x.pdf", Some(&s.period.id), Some(&t)).await;
    assert!(matches!(made, Err(DbError::Constraint)), "{made:?}");
    let mut c = db.pool().acquire().await.unwrap();
    assert!(
        documents::list(&mut c, &s.scope, None, None)
            .await
            .unwrap()
            .is_empty()
    );
}

dual_test!(mixed_properties_cases, mixed_properties);
