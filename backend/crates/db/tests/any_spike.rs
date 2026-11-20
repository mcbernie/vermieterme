//! Spike: what the sqlx `Any` driver can do portably on SQLite and MariaDB. The findings
//! are documented in `vermieterme_db::dialect`; these tests keep them true.

// Test bodies are plain async fns run by `dual_test!`, so clippy's in-test allowance
// does not reach them.
#![allow(clippy::unwrap_used)]

use sqlx::{AnyConnection, Row};
use vermieterme_db::testing::TestDb;
use vermieterme_db::{Db, DbError, Dialect, dual_test};

/// Inserts a VPI row (global table with a unique key, handy for the spike).
async fn insert_vpi(conn: &mut AnyConnection, id: &str, month: i64, tenths: i64) {
    sqlx::query(
        "INSERT INTO vpi_entry (id, year, month, value_tenths, base_year, created_at, updated_at) \
         VALUES (?, 2025, ?, ?, 2020, 1, 1)",
    )
    .bind(id)
    .bind(month)
    .bind(tenths)
    .execute(&mut *conn)
    .await
    .unwrap();
}

/// i64, text, NULL and 0/1 flags round-trip; booleans are read as integers.
async fn types_round_trip(db: &Db) {
    let mut conn = db.pool().acquire().await.unwrap();
    sqlx::query(
        "INSERT INTO account (id, name, locked, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind("a1")
    .bind("Müller & Söhne – ß")
    .bind(1_i64)
    .bind(i64::MAX)
    .bind(-1_i64)
    .execute(&mut *conn)
    .await
    .unwrap();
    let row = sqlx::query("SELECT name, locked, created_at, updated_at FROM account WHERE id = ?")
        .bind("a1")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("name"), "Müller & Söhne – ß");
    assert_eq!(row.get::<i64, _>("locked"), 1);
    assert_eq!(row.get::<i64, _>("created_at"), i64::MAX);
    assert_eq!(row.get::<i64, _>("updated_at"), -1);
    assert!(
        row.try_get::<bool, _>("locked").is_err(),
        "bool ist über Any nicht lesbar"
    );

    let none: Option<String> = None;
    sqlx::query(
        "INSERT INTO app_user (id, account_id, email, name, password_hash, created_at, updated_at) \
         VALUES ('u1', NULL, 'x@example.org', 'X', ?, 1, 1)",
    )
    .bind(none)
    .execute(&mut *conn)
    .await
    .unwrap();
    let row = sqlx::query("SELECT account_id, password_hash FROM app_user")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(row.get::<Option<String>, _>("account_id"), None);
    assert_eq!(row.get::<Option<String>, _>("password_hash"), None);
}
dual_test!(spike_types, types_round_trip);

/// SUM needs a cast to SIGNED (MySQL returns DECIMAL otherwise); dates compare as text;
/// `LIMIT ?` binds.
async fn aggregates_dates_limit(db: &Db) {
    let mut conn = db.pool().acquire().await.unwrap();
    insert_vpi(&mut conn, "v1", 1, 1196).await;
    insert_vpi(&mut conn, "v2", 2, 1201).await;
    let row = sqlx::query(
        "SELECT CAST(COALESCE(SUM(value_tenths), 0) AS SIGNED) AS total, COUNT(*) AS n FROM vpi_entry",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(row.get::<i64, _>("total"), 2397);
    assert_eq!(row.get::<i64, _>("n"), 2);
    let empty = sqlx::query(
        "SELECT CAST(COALESCE(SUM(value_tenths), 0) AS SIGNED) AS total FROM vpi_entry WHERE month > 12",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(empty.get::<i64, _>("total"), 0);

    let row = sqlx::query("SELECT CASE WHEN ? < ? THEN 1 ELSE 0 END AS earlier")
        .bind("2025-09-30")
        .bind("2025-10-01")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(row.get::<i64, _>("earlier"), 1);

    let rows = sqlx::query("SELECT id FROM vpi_entry ORDER BY month LIMIT ?")
        .bind(1_i64)
        .fetch_all(&mut *conn)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
}
dual_test!(spike_aggregates, aggregates_dates_limit);

/// Upsert differs per dialect and is chosen with `Dialect::pick`.
async fn upsert_with_dialect(db: &Db) {
    let mut conn = db.pool().acquire().await.unwrap();
    let sql = Dialect::of(&conn).unwrap().pick(
        "INSERT INTO vpi_entry (id, year, month, value_tenths, base_year, created_at, updated_at) \
         VALUES (?, 2025, 1, ?, 2020, 1, 1) ON CONFLICT (year, month, base_year) \
         DO UPDATE SET value_tenths = excluded.value_tenths",
        "INSERT INTO vpi_entry (id, year, month, value_tenths, base_year, created_at, updated_at) \
         VALUES (?, 2025, 1, ?, 2020, 1, 1) \
         ON DUPLICATE KEY UPDATE value_tenths = VALUES(value_tenths)",
    );
    for (id, tenths) in [("v1", 1196), ("v2", 1199)] {
        sqlx::query(sql)
            .bind(id)
            .bind(tenths)
            .execute(&mut *conn)
            .await
            .unwrap();
    }
    let row = sqlx::query("SELECT id, value_tenths FROM vpi_entry")
        .fetch_all(&mut *conn)
        .await
        .unwrap();
    assert_eq!(row.len(), 1);
    assert_eq!(row[0].get::<String, _>("id"), "v1");
    assert_eq!(row[0].get::<i64, _>("value_tenths"), 1199);
    assert_eq!(Dialect::of(&conn).unwrap(), db.dialect());
}
dual_test!(spike_upsert, upsert_with_dialect);

/// Matched-but-unchanged rows count as affected (MySQL: CLIENT_FOUND_ROWS); version
/// checks and COALESCE partial updates work with one static statement.
async fn affected_rows_and_partial_update(db: &Db) {
    let mut conn = db.pool().acquire().await.unwrap();
    insert_vpi(&mut conn, "v1", 1, 1196).await;
    let same = sqlx::query("UPDATE vpi_entry SET value_tenths = 1196 WHERE id = 'v1'")
        .execute(&mut *conn)
        .await
        .unwrap();
    assert_eq!(same.rows_affected(), 1);

    let none: Option<i64> = None;
    sqlx::query("UPDATE vpi_entry SET value_tenths = COALESCE(?, value_tenths), base_year = COALESCE(?, base_year) WHERE id = 'v1'")
        .bind(none)
        .bind(Some(2015_i64))
        .execute(&mut *conn)
        .await
        .unwrap();
    let row = sqlx::query("SELECT value_tenths, base_year FROM vpi_entry WHERE id = 'v1'")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(
        (
            row.get::<i64, _>("value_tenths"),
            row.get::<i64, _>("base_year")
        ),
        (1196, 2015)
    );

    // Nullable column: flag + value, here clearing the field.
    let set = sqlx::query(
        "UPDATE app_user SET password_hash = CASE WHEN ? = 1 THEN ? ELSE password_hash END \
         WHERE id = 'missing'",
    )
    .bind(1_i64)
    .bind(None::<String>)
    .execute(&mut *conn)
    .await
    .unwrap();
    assert_eq!(set.rows_affected(), 0);
}
dual_test!(spike_affected_rows, affected_rows_and_partial_update);

/// Unique violations are recognised through `Any`; a rolled-back transaction leaves nothing.
async fn errors_and_transactions(db: &Db) {
    let mut conn = db.pool().acquire().await.unwrap();
    insert_vpi(&mut conn, "v1", 1, 1196).await;
    let err = sqlx::query(
        "INSERT INTO vpi_entry (id, year, month, value_tenths, base_year, created_at, updated_at) \
         VALUES ('v2', 2025, 1, 1, 2020, 1, 1)",
    )
    .execute(&mut *conn)
    .await
    .unwrap_err();
    assert!(matches!(DbError::from(err), DbError::Conflict));

    let mut tx = db.pool().begin().await.unwrap();
    insert_vpi(&mut tx, "v3", 3, 1).await;
    tx.rollback().await.unwrap();
    let n: i64 = sqlx::query("SELECT COUNT(*) AS n FROM vpi_entry")
        .fetch_one(&mut *conn)
        .await
        .unwrap()
        .get("n");
    assert_eq!(n, 1);
}
dual_test!(spike_errors, errors_and_transactions);

/// Native MySQL types that `Any` cannot decode – the reason for the portable schema types.
#[tokio::test]
async fn mysql_native_types_are_not_decodable() {
    let Some(test_db) = TestDb::mysql("mysql_native_types_are_not_decodable").await else {
        return;
    };
    let mut conn = test_db.db().pool().acquire().await.unwrap();
    for sql in [
        "SELECT CAST('2025-01-31' AS DATE) AS v",
        "SELECT CAST(1 AS DECIMAL(10,2)) AS v",
        "SELECT SUM(1) AS v",
    ] {
        let result = sqlx::query(sql).fetch_one(&mut *conn).await;
        let decoded = result.and_then(|row| row.try_get::<String, _>("v"));
        assert!(decoded.is_err(), "{sql} wäre doch lesbar");
    }
    sqlx::query("CREATE TABLE spike_flag (f TINYINT NOT NULL)")
        .execute(&mut *conn)
        .await
        .unwrap();
    sqlx::query("INSERT INTO spike_flag VALUES (1)")
        .execute(&mut *conn)
        .await
        .unwrap();
    let tiny = sqlx::query("SELECT f FROM spike_flag")
        .fetch_one(&mut *conn)
        .await;
    let tiny = tiny.and_then(|row| row.try_get::<i64, _>("f"));
    assert!(tiny.is_err(), "TINYINT wäre doch lesbar");
}

/// RETURNING is not portable: MySQL rejects it.
#[tokio::test]
async fn mysql_has_no_update_returning() {
    let Some(test_db) = TestDb::mysql("mysql_has_no_update_returning").await else {
        return;
    };
    let mut conn = test_db.db().pool().acquire().await.unwrap();
    insert_vpi(&mut conn, "v1", 1, 1196).await;
    let result = sqlx::query("UPDATE vpi_entry SET value_tenths = 1 WHERE id = 'v1' RETURNING id")
        .fetch_optional(&mut *conn)
        .await;
    assert!(result.is_err());
}
