//! Reads the logical schema (tables, columns, keys, indexes, foreign keys) of a migrated
//! database in a dialect-neutral form, so SQLite and MySQL can be compared.

use std::collections::BTreeSet;

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};
use vermieterme_db::{Db, Dialect};

/// A column: name, type class (`int`/`text`), NOT NULL, normalised default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub class: String,
    pub not_null: bool,
    pub default: Option<String>,
    /// Maximum length in characters (MySQL `VARCHAR(n)`/`CHAR(n)`, SQLite `CHECK`).
    pub max_len: Option<i64>,
}

/// An index other than the primary key: name, unique, columns in order.
pub type Index = (String, bool, Vec<String>);
/// A foreign key: columns, referenced table, referenced columns, delete rule.
pub type ForeignKey = (Vec<String>, String, Vec<String>, String);

/// A table in dialect-neutral form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub indexes: BTreeSet<Index>,
    pub foreign_keys: BTreeSet<ForeignKey>,
}

/// Reads all application tables (without the sqlx migration table), sorted by name.
pub async fn schema(db: &Db) -> Vec<Table> {
    let mut conn = db.pool().acquire().await.unwrap();
    let names = table_names(&mut conn, db.dialect()).await;
    let mut tables = Vec::new();
    for name in names {
        tables.push(match db.dialect() {
            Dialect::Sqlite => sqlite_table(&mut conn, name).await,
            Dialect::MySql => mysql_table(&mut conn, name).await,
        });
    }
    tables
}

/// Names of all application tables.
async fn table_names(conn: &mut AnyConnection, dialect: Dialect) -> Vec<String> {
    let sql = dialect.pick(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
         AND name <> '_sqlx_migrations' ORDER BY name",
        "SELECT TABLE_NAME AS name FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() \
         AND TABLE_NAME <> '_sqlx_migrations' ORDER BY TABLE_NAME",
    );
    let rows = sqlx::query(sql).fetch_all(&mut *conn).await.unwrap();
    rows.iter().map(|r| r.get::<String, _>("name")).collect()
}

/// Maps a native column type to `int` or `text`.
fn class(native: &str) -> String {
    let t = native.to_ascii_lowercase();
    if t.contains("int") {
        "int".into()
    } else if t.contains("char") || t.contains("text") {
        "text".into()
    } else {
        format!("other:{t}")
    }
}

/// Normalises a default: `NULL` -> none, quotes removed (MariaDB quotes string defaults).
fn default(raw: Option<String>) -> Option<String> {
    let raw = raw?;
    if raw.eq_ignore_ascii_case("null") {
        return None;
    }
    Some(raw.trim_matches('\'').to_owned())
}

/// Normalises a delete rule (`RESTRICT` and `NO ACTION` behave the same here).
fn rule(raw: &str) -> String {
    match raw.to_ascii_uppercase().as_str() {
        "RESTRICT" | "NO ACTION" => "NO ACTION".into(),
        other => other.to_owned(),
    }
}

/// Limit `n` of a length check `<needle>n` in `sql` (SQLite table SQL `length(col) <= `,
/// MySQL check clauses ``char_length(`col`) <= ``).
fn length_check(sql: &str, needle: &str) -> Option<i64> {
    let start = sql.find(needle)? + needle.len();
    let digits: String = sql[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// Reads one SQLite table via the pragma table functions.
async fn sqlite_table(conn: &mut AnyConnection, name: String) -> Table {
    let sql = "SELECT name, type, \"notnull\" AS nn, dflt_value, pk FROM pragma_table_info(?) \
               ORDER BY cid";
    let rows = table_rows(conn, sql, &name).await;
    let master = "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?";
    let sql: String = table_rows(conn, master, &name).await[0].get("sql");
    let columns = rows
        .iter()
        .map(|r| {
            let column: String = r.get("name");
            Column {
                max_len: length_check(&sql, &format!("length({column}) <= ")),
                name: column,
                class: class(&r.get::<String, _>("type")),
                not_null: r.get::<i64, _>("nn") != 0 || r.get::<i64, _>("pk") != 0,
                default: default(r.get("dflt_value")),
            }
        })
        .collect();
    let mut pk: Vec<(i64, String)> = rows
        .iter()
        .filter(|r| r.get::<i64, _>("pk") > 0)
        .map(|r| (r.get("pk"), r.get("name")))
        .collect();
    pk.sort();

    let sql = "SELECT name, \"unique\" AS u, origin FROM pragma_index_list(?)";
    let index_rows = table_rows(conn, sql, &name).await;
    let mut indexes = BTreeSet::new();
    for r in index_rows
        .iter()
        .filter(|r| r.get::<String, _>("origin") != "pk")
    {
        let index: String = r.get("name");
        let sql = "SELECT name FROM pragma_index_info(?) ORDER BY seqno";
        let cols = table_rows(conn, sql, &index).await;
        let cols = cols.iter().map(|c| c.get("name")).collect();
        indexes.insert((index, r.get::<i64, _>("u") != 0, cols));
    }

    let fk_rows = table_rows(
        conn,
        "SELECT CAST(id AS TEXT) AS id, \"table\" AS t, \"from\" AS f, \"to\" AS tt, on_delete \
         FROM pragma_foreign_key_list(?) ORDER BY id, seq",
        &name,
    )
    .await;
    let foreign_keys = group_fks(&fk_rows);

    Table {
        name,
        columns,
        primary_key: pk.into_iter().map(|(_, c)| c).collect(),
        indexes,
        foreign_keys,
    }
}

/// Reads one MySQL table via information_schema.
async fn mysql_table(conn: &mut AnyConnection, name: String) -> Table {
    let rows = table_rows(
        conn,
        "SELECT COLUMN_NAME AS name, DATA_TYPE AS dt, IS_NULLABLE AS nullable, \
         COLUMN_DEFAULT AS dflt, CAST(CASE WHEN DATA_TYPE IN ('varchar', 'char') \
         THEN CHARACTER_MAXIMUM_LENGTH END AS SIGNED) AS max_len FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
        &name,
    )
    .await;
    // TEXT columns carry their limit as CHECK (CHAR_LENGTH(col) <= n).
    let checks = table_rows(
        conn,
        "SELECT cc.CHECK_CLAUSE AS c FROM information_schema.TABLE_CONSTRAINTS tc \
         JOIN information_schema.CHECK_CONSTRAINTS cc ON cc.CONSTRAINT_SCHEMA = tc.TABLE_SCHEMA \
         AND cc.CONSTRAINT_NAME = tc.CONSTRAINT_NAME WHERE tc.TABLE_SCHEMA = DATABASE() \
         AND tc.TABLE_NAME = ? AND tc.CONSTRAINT_TYPE = 'CHECK'",
        &name,
    )
    .await;
    let checks: Vec<String> = checks.iter().map(|r| r.get("c")).collect();
    let checks = checks.join(" ");
    let columns = rows
        .iter()
        .map(|r| Column {
            name: r.get("name"),
            class: class(&r.get::<String, _>("dt")),
            not_null: r.get::<String, _>("nullable") == "NO",
            default: default(r.get("dflt")),
            max_len: r.get::<Option<i64>, _>("max_len").or_else(|| {
                let needle = format!("char_length(`{}`) <= ", r.get::<String, _>("name"));
                length_check(&checks, &needle)
            }),
        })
        .collect();

    let pk = table_rows(
        conn,
        "SELECT COLUMN_NAME AS name FROM information_schema.KEY_COLUMN_USAGE \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND CONSTRAINT_NAME = 'PRIMARY' \
         ORDER BY ORDINAL_POSITION",
        &name,
    )
    .await;

    let index_rows = table_rows(
        conn,
        "SELECT INDEX_NAME AS name, NON_UNIQUE AS nu, COLUMN_NAME AS col \
         FROM information_schema.STATISTICS WHERE TABLE_SCHEMA = DATABASE() \
         AND TABLE_NAME = ? AND INDEX_NAME <> 'PRIMARY' ORDER BY INDEX_NAME, SEQ_IN_INDEX",
        &name,
    )
    .await;
    let mut grouped: Vec<Index> = Vec::new();
    for r in &index_rows {
        let index: String = r.get("name");
        let col: String = r.get("col");
        match grouped.last_mut() {
            Some(last) if last.0 == index => last.2.push(col),
            _ => grouped.push((index, r.get::<i64, _>("nu") == 0, vec![col])),
        }
    }

    let fk_rows = table_rows(
        conn,
        "SELECT k.CONSTRAINT_NAME AS id, k.COLUMN_NAME AS f, k.REFERENCED_TABLE_NAME AS t, \
         k.REFERENCED_COLUMN_NAME AS tt, r.DELETE_RULE AS on_delete \
         FROM information_schema.KEY_COLUMN_USAGE k \
         JOIN information_schema.REFERENTIAL_CONSTRAINTS r \
           ON r.CONSTRAINT_SCHEMA = k.CONSTRAINT_SCHEMA AND r.CONSTRAINT_NAME = k.CONSTRAINT_NAME \
          AND r.TABLE_NAME = k.TABLE_NAME \
         WHERE k.TABLE_SCHEMA = DATABASE() AND k.TABLE_NAME = ? \
           AND k.REFERENCED_TABLE_NAME IS NOT NULL \
         ORDER BY k.CONSTRAINT_NAME, k.ORDINAL_POSITION",
        &name,
    )
    .await;
    let foreign_keys = group_fks(&fk_rows);

    Table {
        name,
        columns,
        primary_key: pk.iter().map(|r| r.get("name")).collect(),
        indexes: grouped.into_iter().collect(),
        foreign_keys,
    }
}

/// Runs `sql` with a table or index name as the only parameter.
async fn table_rows(conn: &mut AnyConnection, sql: &'static str, name: &str) -> Vec<AnyRow> {
    sqlx::query(sql)
        .bind(name)
        .fetch_all(&mut *conn)
        .await
        .unwrap()
}

/// Groups foreign key rows (`id`, `f`, `t`, `tt`, `on_delete`; ordered by constraint and
/// position) into foreign keys; constraint names are dropped (they differ between
/// dialects).
fn group_fks(rows: &[AnyRow]) -> BTreeSet<ForeignKey> {
    let mut current: Option<(String, ForeignKey)> = None;
    let mut out = BTreeSet::new();
    for r in rows {
        let (id, from, to): (String, String, String) = (r.get("id"), r.get("f"), r.get("tt"));
        let (table, rule) = (r.get("t"), rule(&r.get::<String, _>("on_delete")));
        match &mut current {
            Some((cid, fk)) if *cid == id => {
                fk.0.push(from);
                fk.2.push(to);
            }
            _ => {
                if let Some((_, fk)) = current.take() {
                    out.insert(fk);
                }
                current = Some((id, (vec![from], table, vec![to], rule)));
            }
        }
    }
    if let Some((_, fk)) = current {
        out.insert(fk);
    }
    out
}
