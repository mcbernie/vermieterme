//! Uploads racing cascading deletes (two connections, #106): a document either arrives
//! before the delete collects the files (then its file name is in the list and the row is
//! gone) or finds its assignment deleted (not found). Never a document whose file the
//! delete missed, never a deadlock. On SQLite the write lock serializes everything; the
//! MySQL runs are the real check.

// Test bodies are plain async fns run by `dual_test!` (see account_user_repo.rs).
#![allow(clippy::unwrap_used)]
// Tests build scopes directly (no session layer here); see vermieterme_db::scope.
#![allow(clippy::disallowed_methods)]

mod billing_support;

use billing_support::{new_period, property, scope, unit};
use vermieterme_db::in_transaction_read_committed;
use vermieterme_db::repo::billing::period;
use vermieterme_db::repo::documents::{self, NewDocument, Owner, files_of};
use vermieterme_db::repo::masterdata::property as property_repo;
use vermieterme_db::repo::masterdata::tenant::{self, TenantData};
use vermieterme_db::{AccountScope, AnyConnection, Db, DbError, DbResult, dual_test};

/// Rounds of each race: a missing lock shows only sometimes.
const ROUNDS: usize = 8;

/// A tenant in `unit_id`.
async fn tenant(db: &Db, scope: &AccountScope, unit_id: &str) -> String {
    let mut c = db.pool().acquire().await.unwrap();
    let data = TenantData {
        unit_id: unit_id.into(),
        salutation: "Herr".into(),
        first_name: "Max".into(),
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

/// A new document row named `file` with the assignment.
fn new_doc(file: &str, period: Option<&str>, tenant: Option<&str>) -> NewDocument {
    NewDocument {
        file_name: file.into(),
        original_name: "a.pdf".into(),
        mime_type: "application/pdf".into(),
        size_bytes: 1,
        category: "other".into(),
        billing_period_id: period.map(Into::into),
        tenant_id: tenant.map(Into::into),
    }
}

/// Collects the files of `owner` and deletes it with `delete`, in one transaction.
async fn cascade(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    owner: Owner<'_>,
    delete: impl AsyncFnOnce(&mut AnyConnection) -> DbResult<()>,
) -> DbResult<Vec<String>> {
    in_transaction_read_committed(conn, async |tx| {
        let files = files_of(tx, scope, owner).await?;
        delete(tx).await?;
        Ok(files)
    })
    .await
}

/// Checks one round: the upload either lost (not found) or its file is in the list.
fn check(round: usize, made: DbResult<documents::Document>, files: Vec<String>) {
    match made {
        Ok(doc) => assert!(files.contains(&doc.file_name), "Runde {round}: Datei fehlt"),
        Err(DbError::NotFound) => {}
        Err(other) => panic!("Runde {round}: {other:?}"),
    }
}

/// Uploads to a tenant and to a period while their property is deleted.
async fn upload_vs_property_delete(db: &Db) {
    let s = scope(db, "A").await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    let mut c3 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let pid = property(db, &s).await;
        let uid = unit(db, &s, &pid, 10).await;
        let tid = tenant(db, &s, &uid).await;
        let mut c = db.pool().acquire().await.unwrap();
        let input = new_period(&pid, "2024-01-01", "2024-12-31");
        let bp = period::create(&mut c, &s, &input)
            .await
            .unwrap()
            .unwrap()
            .id;
        drop(c);
        let (to_tenant, to_period) = (
            new_doc(&format!("t{round}"), None, Some(&tid)),
            new_doc(&format!("p{round}"), Some(&bp), None),
        );
        let (a, b, files) = tokio::join!(
            documents::create(&mut c1, &s, &to_tenant),
            documents::create(&mut c3, &s, &to_period),
            cascade(&mut c2, &s, Owner::Property(&pid), async |tx| {
                property_repo::delete(tx, &s, &pid).await
            }),
        );
        let files = files.unwrap();
        check(round, a, files.clone());
        check(round, b, files);
    }
    let mut c = db.pool().acquire().await.unwrap();
    let left = documents::list(&mut c, &s, None, None).await.unwrap();
    assert!(
        left.is_empty(),
        "{} Dokumente ohne Zuordnung übrig",
        left.len()
    );
}
dual_test!(upload_vs_property, upload_vs_property_delete);

/// Uploads to a tenant while the tenant is deleted.
async fn upload_vs_tenant_delete(db: &Db) {
    let s = scope(db, "A").await;
    let pid = property(db, &s).await;
    let uid = unit(db, &s, &pid, 10).await;
    let mut c1 = db.pool().acquire().await.unwrap();
    let mut c2 = db.pool().acquire().await.unwrap();
    for round in 0..ROUNDS {
        let tid = tenant(db, &s, &uid).await;
        let new = new_doc(&format!("t{round}"), None, Some(&tid));
        let (made, files) = tokio::join!(
            documents::create(&mut c1, &s, &new),
            cascade(&mut c2, &s, Owner::Tenant(&tid), async |tx| {
                tenant::delete(tx, &s, &tid).await
            }),
        );
        check(round, made, files.unwrap());
    }
}
dual_test!(upload_vs_tenant, upload_vs_tenant_delete);
