//! Migration 0001 in both dialects: same logical schema, account isolation by foreign
//! keys, cascades.

// Test bodies are plain async fns run by `dual_test!`, so clippy's in-test allowance
// does not reach them.
#![allow(clippy::unwrap_used)]

#[path = "support/introspect.rs"]
mod introspect;

use sqlx::Row;
use vermieterme_db::testing::TestDb;
use vermieterme_db::{Db, DbError, dual_test};

/// Tables without account reference (global or authentication data).
const UNSCOPED: &[&str] = &[
    "account",
    "app_user",
    "audit_log",
    "user_identity",
    "user_token",
    "vpi_entry",
    "web_session",
];

const ALL_TABLES: &[&str] = &[
    "account",
    "app_user",
    "audit_log",
    "billing_period",
    "cost",
    "cost_category",
    "cost_unit_amount",
    "document",
    "landlord",
    "pdf_template",
    "prepayment",
    "property",
    "rent_change",
    "tenant",
    "tenant_invite_code",
    "unit",
    "user_identity",
    "user_token",
    "vpi_entry",
    "web_session",
];

/// All tables exist; every business table has a NOT NULL `account_id`.
async fn has_all_tables_with_account_id(db: &Db) {
    let tables = introspect::schema(db).await;
    let names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ALL_TABLES);
    for t in tables
        .iter()
        .filter(|t| !UNSCOPED.contains(&t.name.as_str()))
    {
        let col = t.columns.iter().find(|c| c.name == "account_id");
        assert!(
            col.is_some_and(|c| c.not_null),
            "{} ohne account_id",
            t.name
        );
    }
    // Guards the introspection itself: a reader returning nothing would make the
    // dialect comparison pass trivially.
    let fks: usize = tables.iter().map(|t| t.foreign_keys.len()).sum();
    let indexes: usize = tables.iter().map(|t| t.indexes.len()).sum();
    assert_eq!((fks, indexes), (24, 33));
    // Every text column except the free-form TEXT ones has a length limit (#88).
    let unlimited: Vec<String> = tables
        .iter()
        .flat_map(|t| t.columns.iter().map(move |c| (t, c)))
        .filter(|(_, c)| c.class == "text" && c.max_len.is_none())
        .map(|(t, c)| format!("{}.{}", t.name, c.name))
        .collect();
    assert_eq!(unlimited, ["pdf_template.config", "rent_change.reason"]);
    let cost = tables
        .iter()
        .find(|t| t.name == "cost_unit_amount")
        .unwrap();
    assert_eq!(cost.primary_key, ["account_id", "cost_id", "unit_id"]);
}
dual_test!(schema_tables, has_all_tables_with_account_id);

/// Money columns are integers; `version` columns exist for #67; `unit.kind` for #2.
async fn has_required_columns(db: &Db) {
    let tables = introspect::schema(db).await;
    let col = |table: &str, column: &str| {
        tables
            .iter()
            .find(|t| t.name == table)
            .and_then(|t| t.columns.iter().find(|c| c.name == column))
            .cloned()
            .unwrap_or_else(|| panic!("{table}.{column} fehlt"))
    };
    for (t, c) in [
        ("cost", "total_amount_cents"),
        ("cost_unit_amount", "amount_cents"),
        ("prepayment", "monthly_amount_cents"),
        ("rent_change", "amount_cents"),
        ("cost", "version"),
        ("prepayment", "version"),
        ("tenant", "app_tokens_revoked_at"),
    ] {
        assert_eq!(col(t, c).class, "int", "{t}.{c}");
    }
    assert!(
        !col("cost", "total_amount_cents").not_null,
        "#25: Betrag darf leer sein"
    );
    assert_eq!(col("unit", "kind").default.as_deref(), Some("apartment"));
    assert_eq!(col("billing_period", "start_date").class, "text");
}
dual_test!(schema_columns, has_required_columns);

/// Both dialects define exactly the same logical schema.
#[tokio::test]
async fn mysql_schema_matches_sqlite() {
    let Some(mysql) = TestDb::mysql("mysql_schema_matches_sqlite").await else {
        return;
    };
    let sqlite = TestDb::sqlite().await;
    let left = introspect::schema(sqlite.db()).await;
    let right = introspect::schema(mysql.db()).await;
    assert_eq!(left.len(), right.len());
    for (l, r) in left.iter().zip(&right) {
        assert_eq!(l, r, "Tabelle {} weicht ab", l.name);
    }
}

/// Inserts an account and a property; returns their ids.
async fn account_with_property(conn: &mut sqlx::AnyConnection, name: &str) -> (String, String) {
    let account = format!("acc-{name}");
    let property = format!("prop-{name}");
    sqlx::query("INSERT INTO account (id, name, created_at, updated_at) VALUES (?, ?, 1, 1)")
        .bind(&account)
        .bind(name)
        .execute(&mut *conn)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO property (id, account_id, street, zip, city, created_at, updated_at) \
         VALUES (?, ?, 'Musterweg 1', '12345', 'Musterstadt', 1, 1)",
    )
    .bind(&property)
    .bind(&account)
    .execute(&mut *conn)
    .await
    .unwrap();
    (account, property)
}

/// A unit cannot point at a property of another account (composite foreign key).
async fn rejects_cross_account_reference(db: &Db) {
    let mut conn = db.pool().acquire().await.unwrap();
    let (_, prop_a) = account_with_property(&mut conn, "a").await;
    let (acc_b, _) = account_with_property(&mut conn, "b").await;
    let err = sqlx::query(
        "INSERT INTO unit (id, account_id, property_id, name, floor, shares, created_at, \
         updated_at) VALUES ('u1', ?, ?, 'EG', 'EG', 10, 1, 1)",
    )
    .bind(&acc_b)
    .bind(&prop_a)
    .execute(&mut *conn)
    .await
    .unwrap_err();
    assert!(matches!(DbError::from(err), DbError::Sqlx(_)));
}
dual_test!(schema_account_isolation, rejects_cross_account_reference);

/// Deleting a property removes its units; a category with costs cannot be deleted.
async fn cascades_and_restricts(db: &Db) {
    let mut conn = db.pool().acquire().await.unwrap();
    let (acc, prop) = account_with_property(&mut conn, "a").await;
    let statements = [
        "INSERT INTO unit (id, account_id, property_id, name, floor, shares, created_at, updated_at) \
         VALUES ('u1', ?, ?, 'EG', 'EG', 10, 1, 1)",
        "INSERT INTO billing_period (id, account_id, property_id, start_date, end_date, created_at, \
         updated_at) VALUES ('bp1', ?, ?, '2025-01-01', '2025-12-31', 1, 1)",
    ];
    for sql in statements {
        sqlx::query(sql)
            .bind(&acc)
            .bind(&prop)
            .execute(&mut *conn)
            .await
            .unwrap();
    }
    sqlx::query(
        "INSERT INTO cost_category (id, account_id, name, distribution_key, created_at, updated_at) \
         VALUES ('cc1', ?, 'Grundsteuer', 'per_notice', 1, 1)",
    )
    .bind(&acc)
    .execute(&mut *conn)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO cost (id, account_id, billing_period_id, cost_category_id, total_amount_cents, \
         created_at, updated_at) VALUES ('c1', ?, 'bp1', 'cc1', 12345, 1, 1)",
    )
    .bind(&acc)
    .execute(&mut *conn)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO cost_unit_amount (account_id, cost_id, unit_id, amount_cents, created_at, \
         updated_at) VALUES (?, 'c1', 'u1', 4000, 1, 1)",
    )
    .bind(&acc)
    .execute(&mut *conn)
    .await
    .unwrap();

    let restrict = sqlx::query("DELETE FROM cost_category WHERE id = 'cc1'")
        .execute(&mut *conn)
        .await;
    assert!(
        restrict.is_err(),
        "Kostenart mit Kosten darf nicht gelöscht werden"
    );

    sqlx::query("DELETE FROM property WHERE id = ?")
        .bind(&prop)
        .execute(&mut *conn)
        .await
        .unwrap();
    for table in ["unit", "billing_period", "cost", "cost_unit_amount"] {
        let sql = match table {
            "unit" => "SELECT COUNT(*) AS n FROM unit",
            "billing_period" => "SELECT COUNT(*) AS n FROM billing_period",
            "cost" => "SELECT COUNT(*) AS n FROM cost",
            _ => "SELECT COUNT(*) AS n FROM cost_unit_amount",
        };
        let n: i64 = sqlx::query(sql)
            .fetch_one(&mut *conn)
            .await
            .unwrap()
            .get("n");
        assert_eq!(n, 0, "{table} nicht kaskadiert gelöscht");
    }
}
dual_test!(schema_cascades, cascades_and_restricts);

/// Migrating twice is a no-op.
async fn migrations_are_idempotent(db: &Db) {
    db.migrate().await.unwrap();
    let n: i64 = sqlx::query("SELECT COUNT(*) AS n FROM _sqlx_migrations")
        .fetch_one(db.pool())
        .await
        .unwrap()
        .get("n");
    assert_eq!(
        n,
        i64::try_from(vermieterme_db::migration_versions().len()).unwrap()
    );
}
dual_test!(schema_migrate_twice, migrations_are_idempotent);

/// A too long value is rejected the same way in both dialects (#88), never truncated.
async fn too_long_values_are_rejected(db: &Db) {
    let mut conn = db.pool().acquire().await.unwrap();
    account_with_property(&mut conn, "a").await;
    let err = sqlx::query(
        "INSERT INTO property (id, account_id, street, zip, city, created_at, updated_at) \
         VALUES ('p2', 'acc-a', 'Musterweg 1', ?, 'Musterstadt', 1, 1)",
    )
    .bind("1".repeat(17))
    .execute(&mut *conn)
    .await
    .unwrap_err();
    assert!(matches!(DbError::from(err), DbError::Constraint));
    let err = sqlx::query("UPDATE account SET name = ? WHERE id = 'acc-a'")
        .bind("ä".repeat(256))
        .execute(&mut *conn)
        .await
        .unwrap_err();
    assert!(matches!(DbError::from(err), DbError::Constraint));
    // The limit counts characters, not bytes, in both dialects.
    sqlx::query("UPDATE account SET name = ? WHERE id = 'acc-a'")
        .bind("ä".repeat(255))
        .execute(&mut *conn)
        .await
        .unwrap();
}
dual_test!(schema_length_limits, too_long_values_are_rejected);

/// Every pooled MySQL connection runs in strict mode.
#[tokio::test]
async fn mysql_connections_are_strict() {
    let Some(mysql) = TestDb::mysql("mysql_connections_are_strict").await else {
        return;
    };
    let mode: String = sqlx::query("SELECT @@SESSION.sql_mode AS m")
        .fetch_one(mysql.db().pool())
        .await
        .unwrap()
        .get("m");
    assert!(mode.contains("STRICT_ALL_TABLES"), "{mode}");
}
