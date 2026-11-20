//! Length limit of the free-text column `pdf_template.config` (#91): MySQL `TEXT` holds
//! at most 65 535 bytes, so both dialects cap it at 16 000 characters by a CHECK.

// Test bodies are plain async fns run by `dual_test!`, so clippy's in-test allowance
// does not reach them.
#![allow(clippy::unwrap_used)]

use vermieterme_db::{Db, DbError, dual_test};

/// Characters allowed in `pdf_template.config`.
const CONFIG_MAX: usize = 16_000;

/// Inserts a template with `config` for the account `acc`.
async fn insert_config(
    c: &mut sqlx::AnyConnection,
    id: &str,
    config: String,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO pdf_template (id, account_id, config, created_at, updated_at) \
         VALUES (?, 'acc', ?, 1, 1)",
    )
    .bind(id)
    .bind(config)
    .execute(&mut *c)
    .await
    .map(|_| ())
    .map_err(DbError::from)
}

/// The limit counts characters (multi-byte umlauts fit), one more is a `Constraint` error.
async fn config_limit(db: &Db) {
    let mut c = db.pool().acquire().await.unwrap();
    sqlx::query("INSERT INTO account (id, name, created_at, updated_at) VALUES ('acc', 'A', 1, 1)")
        .execute(&mut *c)
        .await
        .unwrap();
    let err = insert_config(&mut c, "t1", "x".repeat(CONFIG_MAX + 1))
        .await
        .unwrap_err();
    assert!(matches!(err, DbError::Constraint), "{err:?}");
    insert_config(&mut c, "t2", "ä".repeat(CONFIG_MAX))
        .await
        .unwrap();
}
dual_test!(schema_config_limit, config_limit);
