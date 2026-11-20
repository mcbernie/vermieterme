//! The landlord data of an account (one row per account). Personal data.

use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row};

use crate::error::DbResult;
use crate::row::{new_id, now_millis};
use crate::scope::AccountScope;
use crate::tx::in_transaction;

/// The landlord of an account (sender of the statements).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landlord {
    /// Primary key.
    pub id: String,
    /// The values.
    pub data: LandlordData,
    /// Creation time (Unix ms).
    pub created_at: i64,
    /// Last change (Unix ms).
    pub updated_at: i64,
}

/// The editable values (validated and trimmed by the caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandlordData {
    /// Name.
    pub name: String,
    /// Street and house number.
    pub street: String,
    /// Postal code.
    pub zip: String,
    /// City.
    pub city: String,
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
}

/// Column list read by [`from_row`].
macro_rules! columns {
    () => {
        "id, name, street, zip, city, phone, email, bank_name, iban, account_holder, \
         created_at, updated_at"
    };
}

/// Maps a row selected with `columns!()`.
fn from_row(row: &AnyRow) -> DbResult<Landlord> {
    Ok(Landlord {
        id: row.try_get("id")?,
        data: LandlordData {
            name: row.try_get("name")?,
            street: row.try_get("street")?,
            zip: row.try_get("zip")?,
            city: row.try_get("city")?,
            phone: row.try_get("phone")?,
            email: row.try_get("email")?,
            bank_name: row.try_get("bank_name")?,
            iban: row.try_get("iban")?,
            account_holder: row.try_get("account_holder")?,
        },
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Reads the landlord of the account, `None` while nothing is stored.
pub async fn get(conn: &mut AnyConnection, scope: &AccountScope) -> DbResult<Option<Landlord>> {
    let row = sqlx::query(concat!(
        "SELECT ",
        columns!(),
        " FROM landlord WHERE account_id = ?"
    ))
    .bind(scope.account_id())
    .fetch_optional(&mut *conn)
    .await?;
    row.as_ref().map(from_row).transpose()
}

/// Stores the landlord: updates the existing row or creates it (one per account).
pub async fn save(
    conn: &mut AnyConnection,
    scope: &AccountScope,
    data: &LandlordData,
) -> DbResult<Landlord> {
    in_transaction(conn, async |tx| {
        let now = now_millis();
        let result = sqlx::query(
            "UPDATE landlord SET name = ?, street = ?, zip = ?, city = ?, phone = ?, email = ?, \
             bank_name = ?, iban = ?, account_holder = ?, updated_at = ? WHERE account_id = ?",
        )
        .bind(&data.name)
        .bind(&data.street)
        .bind(&data.zip)
        .bind(&data.city)
        .bind(data.phone.as_deref())
        .bind(data.email.as_deref())
        .bind(data.bank_name.as_deref())
        .bind(data.iban.as_deref())
        .bind(data.account_holder.as_deref())
        .bind(now)
        .bind(scope.account_id())
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO landlord (id, account_id, name, street, zip, city, phone, email, \
                 bank_name, iban, account_holder, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(new_id())
            .bind(scope.account_id())
            .bind(&data.name)
            .bind(&data.street)
            .bind(&data.zip)
            .bind(&data.city)
            .bind(data.phone.as_deref())
            .bind(data.email.as_deref())
            .bind(data.bank_name.as_deref())
            .bind(data.iban.as_deref())
            .bind(data.account_holder.as_deref())
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        get(tx, scope).await?.ok_or(crate::DbError::NotFound)
    })
    .await
}
