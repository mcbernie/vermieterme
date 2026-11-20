//! Tenants (Mieter). Personal data: never log or echo values.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use super::unit;
use crate::error::{DbError, DbResult};
use crate::row::{new_id, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction;

/// Allowed values of [`TenantData::lease_type`].
pub const LEASE_TYPES: [&str; 3] = ["standard", "index", "staffel"];

/// The editable values of a tenant (request and stored form; validated and trimmed by
/// the caller). Dates are `YYYY-MM-DD`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantData {
    /// The unit the tenant rents.
    pub unit_id: String,
    /// Salutation (Herr, Frau, ...).
    pub salutation: String,
    /// First name.
    pub first_name: String,
    /// Last name.
    pub last_name: String,
    /// Salutation of a second tenant.
    pub salutation2: Option<String>,
    /// First name of a second tenant.
    pub first_name2: Option<String>,
    /// Last name of a second tenant.
    pub last_name2: Option<String>,
    /// Phone number.
    pub phone: Option<String>,
    /// E-mail address.
    pub email: Option<String>,
    /// Bank name.
    pub bank_name: Option<String>,
    /// IBAN (checked, grouped).
    pub iban: Option<String>,
    /// Account holder.
    pub account_holder: Option<String>,
    /// Move-in date.
    pub move_in_date: String,
    /// Move-out date.
    pub move_out_date: Option<String>,
    /// One of [`LEASE_TYPES`].
    pub lease_type: String,
    /// Base year of the index series (index leases).
    pub index_base_year: Option<i64>,
    /// VPI value at the last adjustment in tenths.
    pub index_reference_tenths: Option<i64>,
    /// Date of the last adjustment.
    pub index_reference_date: Option<String>,
    /// Minimum months between adjustments.
    pub index_min_months: i64,
}

/// A stored tenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tenant {
    /// Primary key.
    pub id: String,
    /// The values.
    pub data: TenantData,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

/// Column list read by [`from_row`].
macro_rules! columns {
    () => {
        "t.id, t.unit_id, t.salutation, t.first_name, t.last_name, t.salutation2, \
         t.first_name2, t.last_name2, t.phone, t.email, t.bank_name, t.iban, t.account_holder, \
         t.move_in_date, t.move_out_date, t.lease_type, t.index_base_year, \
         t.index_reference_tenths, t.index_reference_date, t.index_min_months, \
         t.created_at, t.updated_at"
    };
}

/// Maps a row selected with `columns!()`.
pub(super) fn from_row(row: &AnyRow) -> DbResult<Tenant> {
    Ok(Tenant {
        id: row.try_get("id")?,
        data: TenantData {
            unit_id: row.try_get("unit_id")?,
            salutation: row.try_get("salutation")?,
            first_name: row.try_get("first_name")?,
            last_name: row.try_get("last_name")?,
            salutation2: row.try_get("salutation2")?,
            first_name2: row.try_get("first_name2")?,
            last_name2: row.try_get("last_name2")?,
            phone: row.try_get("phone")?,
            email: row.try_get("email")?,
            bank_name: row.try_get("bank_name")?,
            iban: row.try_get("iban")?,
            account_holder: row.try_get("account_holder")?,
            move_in_date: row.try_get("move_in_date")?,
            move_out_date: row.try_get("move_out_date")?,
            lease_type: row.try_get("lease_type")?,
            index_base_year: row.try_get("index_base_year")?,
            index_reference_tenths: row.try_get("index_reference_tenths")?,
            index_reference_date: row.try_get("index_reference_date")?,
            index_min_months: row.try_get("index_min_months")?,
        },
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Lists the tenants of the account (optionally of one unit), newest first.
pub async fn list(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    unit_id: Option<&str>,
) -> DbResult<Vec<Tenant>> {
    let sql = match unit_id {
        Some(_) => concat!(
            "SELECT ",
            columns!(),
            " FROM tenant t WHERE t.account_id = ? AND t.unit_id = ? \
             ORDER BY t.created_at DESC, t.id DESC"
        ),
        None => concat!(
            "SELECT ",
            columns!(),
            " FROM tenant t WHERE t.account_id = ? ORDER BY t.created_at DESC, t.id DESC"
        ),
    };
    let mut query = sqlx::query(sql).bind(scope.account_id());
    if let Some(unit_id) = unit_id {
        query = query.bind(unit_id);
    }
    let rows = query.fetch_all(&mut *conn).await?;
    rows.iter().map(from_row).collect()
}

/// Lists the tenants of all units of one property (detail of a property).
pub async fn list_for_property(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    property_id: &str,
) -> DbResult<Vec<Tenant>> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM tenant t JOIN unit u ON u.account_id = t.account_id AND u.id = t.unit_id \
         WHERE t.account_id = ? AND u.property_id = ? ORDER BY t.created_at DESC, t.id DESC"
    ))
    .bind(scope.account_id())
    .bind(property_id)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}

/// Reads one tenant of the account.
pub async fn get(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<Tenant> {
    let row = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM tenant t WHERE t.account_id = ? AND t.id = ?"
    ))
    .bind(scope.account_id())
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(DbError::NotFound)?;
    from_row(&row)
}

/// Creates a tenant; a foreign or unknown unit is [`DbError::NotFound`].
pub async fn create(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    data: &TenantData,
) -> DbResult<Tenant> {
    in_transaction(conn, async |tx| {
        unit::ensure(tx, scope, &data.unit_id).await?;
        let id = new_id();
        let now = now_millis();
        sqlx::query(
            "INSERT INTO tenant (id, account_id, unit_id, salutation, first_name, last_name, \
             salutation2, first_name2, last_name2, phone, email, bank_name, iban, \
             account_holder, move_in_date, move_out_date, lease_type, index_base_year, \
             index_reference_tenths, index_reference_date, index_min_months, created_at, \
             updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
             ?, ?)",
        )
        .bind(&id)
        .bind(scope.account_id())
        .bind(&data.unit_id)
        .bind(&data.salutation)
        .bind(&data.first_name)
        .bind(&data.last_name)
        .bind(data.salutation2.as_deref())
        .bind(data.first_name2.as_deref())
        .bind(data.last_name2.as_deref())
        .bind(data.phone.as_deref())
        .bind(data.email.as_deref())
        .bind(data.bank_name.as_deref())
        .bind(data.iban.as_deref())
        .bind(data.account_holder.as_deref())
        .bind(&data.move_in_date)
        .bind(data.move_out_date.as_deref())
        .bind(&data.lease_type)
        .bind(data.index_base_year)
        .bind(data.index_reference_tenths)
        .bind(data.index_reference_date.as_deref())
        .bind(data.index_min_months)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        get(tx, scope, &id).await
    })
    .await
}

/// Replaces the values of a tenant (the unit may change; the invitation code and the
/// revocation time of the tenant app stay untouched).
pub async fn update(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    id: &str,
    data: &TenantData,
) -> DbResult<Tenant> {
    in_transaction(conn, async |tx| {
        get(tx, scope, id).await?;
        unit::ensure(tx, scope, &data.unit_id).await?;
        sqlx::query(
            "UPDATE tenant SET unit_id = ?, salutation = ?, first_name = ?, last_name = ?, \
             salutation2 = ?, first_name2 = ?, last_name2 = ?, phone = ?, email = ?, \
             bank_name = ?, iban = ?, account_holder = ?, move_in_date = ?, move_out_date = ?, \
             lease_type = ?, index_base_year = ?, index_reference_tenths = ?, \
             index_reference_date = ?, index_min_months = ?, updated_at = ? \
             WHERE account_id = ? AND id = ?",
        )
        .bind(&data.unit_id)
        .bind(&data.salutation)
        .bind(&data.first_name)
        .bind(&data.last_name)
        .bind(data.salutation2.as_deref())
        .bind(data.first_name2.as_deref())
        .bind(data.last_name2.as_deref())
        .bind(data.phone.as_deref())
        .bind(data.email.as_deref())
        .bind(data.bank_name.as_deref())
        .bind(data.iban.as_deref())
        .bind(data.account_holder.as_deref())
        .bind(&data.move_in_date)
        .bind(data.move_out_date.as_deref())
        .bind(&data.lease_type)
        .bind(data.index_base_year)
        .bind(data.index_reference_tenths)
        .bind(data.index_reference_date.as_deref())
        .bind(data.index_min_months)
        .bind(now_millis())
        .bind(scope.account_id())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        get(tx, scope, id).await
    })
    .await
}

/// Deletes a tenant (invitation code and documents go with it, cascade).
pub async fn delete(conn: &mut AnyConnection, scope: &AccountScope, id: &str) -> DbResult<()> {
    let result = sqlx::query("DELETE FROM tenant WHERE account_id = ? AND id = ?")
        .bind(scope.account_id())
        .bind(id)
        .execute(&mut *conn)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}
