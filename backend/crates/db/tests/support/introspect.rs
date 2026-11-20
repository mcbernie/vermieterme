//! Reads the logical schema (tables, columns, keys, indexes, foreign keys) of a migrated
//! database in a dialect-neutral form, so SQLite and MySQL can be compared.

use std::collections::BTreeSet;

use sqlx::{AnyConnection, Row};
use vermieterme_db::{Db, Dialect};

/// A column: name, type class (`int`/`text`), NOT NULL, normalised default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub class: String,
    pub not_null: bool,
    pub default: Option<String>,
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

/// Reads one SQLite table via the pragma table functions.
async fn sqlite_table(conn: &mut AnyConnection, name: String) -> Table {
    let rows = sqlx::query(
        "SELECT name, type, \"notnull\" AS nn, dflt_value, pk FROM pragma_table_info(?) ORDER BY cid",
    )
    .bind(&name)
    .fetch_all(&mut *conn)
    .await
    .unwrap();
    let columns = rows
        .iter()
        .map(|r| Column {
            name: r.get("name"),
            class: class(&r.get::<String, _>("type")),
            not_null: r.get::<i64, _>("nn") != 0 || r.get::<i64, _>("pk") != 0,
            default: default(r.get("dflt_value")),
        })
        .collect();
    let mut pk: Vec<(i64, String)> = rows
        .iter()
        .filter(|r| r.get::<i64, _>("pk") > 0)
        .map(|r| (r.get("pk"), r.get("name")))
        .collect();
    pk.sort();

    let index_rows = sqlx::query("SELECT name, \"unique\" AS u, origin FROM pragma_index_list(?)")
        .bind(&name)
        .fetch_all(&mut *conn)
        .await
        .unwrap();
    let mut indexes = BTreeSet::new();
    for r in index_rows
        .iter()
        .filter(|r| r.get::<String, _>("origin") != "pk")
    {
        let index: String = r.get("name");
        let cols = sqlx::query("SELECT name FROM pragma_index_info(?) ORDER BY seqno")
            .bind(&index)
            .fetch_all(&mut *conn)
            .await
            .unwrap();
        let cols = cols.iter().map(|c| c.get("name")).collect();
        indexes.insert((index, r.get::<i64, _>("u") != 0, cols));
    }

    let fk_rows = sqlx::query(
        "SELECT id, \"table\" AS t, \"from\" AS f, \"to\" AS tt, on_delete \
         FROM pragma_foreign_key_list(?) ORDER BY id, seq",
    )
    .bind(&name)
    .fetch_all(&mut *conn)
    .await
    .unwrap();
    let foreign_keys = group_fks(fk_rows.iter().map(|r| {
        let id: i64 = r.get("id");
        let on_delete: String = r.get("on_delete");
        (
            id.to_string(),
            r.get("f"),
            r.get("t"),
            r.get("tt"),
            rule(&on_delete),
        )
    }));

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
    let rows = sqlx::query(
        "SELECT COLUMN_NAME AS name, DATA_TYPE AS dt, IS_NULLABLE AS nullable, \
         COLUMN_DEFAULT AS dflt FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
    )
    .bind(&name)
    .fetch_all(&mut *conn)
    .await
    .unwrap();
    let columns = rows
        .iter()
        .map(|r| Column {
            name: r.get("name"),
            class: class(&r.get::<String, _>("dt")),
            not_null: r.get::<String, _>("nullable") == "NO",
            default: default(r.get("dflt")),
        })
        .collect();

    let pk = sqlx::query(
        "SELECT COLUMN_NAME AS name FROM information_schema.KEY_COLUMN_USAGE \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND CONSTRAINT_NAME = 'PRIMARY' \
         ORDER BY ORDINAL_POSITION",
    )
    .bind(&name)
    .fetch_all(&mut *conn)
    .await
    .unwrap();

    let index_rows = sqlx::query(
        "SELECT INDEX_NAME AS name, NON_UNIQUE AS nu, COLUMN_NAME AS col \
         FROM information_schema.STATISTICS WHERE TABLE_SCHEMA = DATABASE() \
         AND TABLE_NAME = ? AND INDEX_NAME <> 'PRIMARY' ORDER BY INDEX_NAME, SEQ_IN_INDEX",
    )
    .bind(&name)
    .fetch_all(&mut *conn)
    .await
    .unwrap();
    let mut grouped: Vec<Index> = Vec::new();
    for r in &index_rows {
        let index: String = r.get("name");
        let col: String = r.get("col");
        match grouped.last_mut() {
            Some(last) if last.0 == index => last.2.push(col),
            _ => grouped.push((index, r.get::<i64, _>("nu") == 0, vec![col])),
        }
    }

    let fk_rows = sqlx::query(
        "SELECT k.CONSTRAINT_NAME AS id, k.COLUMN_NAME AS f, k.REFERENCED_TABLE_NAME AS t, \
         k.REFERENCED_COLUMN_NAME AS tt, r.DELETE_RULE AS on_delete \
         FROM information_schema.KEY_COLUMN_USAGE k \
         JOIN information_schema.REFERENTIAL_CONSTRAINTS r \
           ON r.CONSTRAINT_SCHEMA = k.CONSTRAINT_SCHEMA AND r.CONSTRAINT_NAME = k.CONSTRAINT_NAME \
          AND r.TABLE_NAME = k.TABLE_NAME \
         WHERE k.TABLE_SCHEMA = DATABASE() AND k.TABLE_NAME = ? \
           AND k.REFERENCED_TABLE_NAME IS NOT NULL \
         ORDER BY k.CONSTRAINT_NAME, k.ORDINAL_POSITION",
    )
    .bind(&name)
    .fetch_all(&mut *conn)
    .await
    .unwrap();
    let foreign_keys = group_fks(fk_rows.iter().map(|r| {
        let on_delete: String = r.get("on_delete");
        (
            r.get("id"),
            r.get("f"),
            r.get("t"),
            r.get("tt"),
            rule(&on_delete),
        )
    }));

    Table {
        name,
        columns,
        primary_key: pk.iter().map(|r| r.get("name")).collect(),
        indexes: grouped.into_iter().collect(),
        foreign_keys,
    }
}

/// Groups `(constraint, from, table, to, rule)` rows (ordered by constraint and position)
/// into foreign keys; constraint names are dropped (they differ between dialects).
fn group_fks(
    rows: impl Iterator<Item = (String, String, String, String, String)>,
) -> BTreeSet<ForeignKey> {
    let mut current: Option<(String, ForeignKey)> = None;
    let mut out = BTreeSet::new();
    for (id, from, table, to, rule) in rows {
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
