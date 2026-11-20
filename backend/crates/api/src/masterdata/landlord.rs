//! `GET/PUT /api/settings`: the landlord data of the account (sender of the statements).
//! Personal data: values never appear in messages, logs or audit entries.

use axum::extract::State;
use serde::{Deserialize, Serialize};
use vermieterme_db::repo::masterdata::landlord::{self, Landlord, LandlordData};

use super::audited;
use super::fields::{SHORT_MAX, TEXT_MAX, ZIP_MAX};
use crate::auth::AccountCtx;
use crate::error::ApiResult;
use crate::error::json::{ApiJson, timestamp};
use crate::error::validate::Validator;
use crate::routes::AppState;

/// Message used if the row vanishes between write and read (cannot happen in practice).
const NOT_FOUND: &str = "Vermieterdaten nicht gefunden";

/// The landlord in responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LandlordJson {
    id: String,
    name: String,
    street: String,
    zip: String,
    city: String,
    phone: Option<String>,
    email: Option<String>,
    bank_name: Option<String>,
    iban: Option<String>,
    account_holder: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<Landlord> for LandlordJson {
    /// Response form with ISO timestamps.
    fn from(l: Landlord) -> Self {
        let d = l.data;
        Self {
            id: l.id,
            name: d.name,
            street: d.street,
            zip: d.zip,
            city: d.city,
            phone: d.phone,
            email: d.email,
            bank_name: d.bank_name,
            iban: d.iban,
            account_holder: d.account_holder,
            created_at: timestamp(l.created_at),
            updated_at: timestamp(l.updated_at),
        }
    }
}

/// Request of `PUT /api/settings`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LandlordBody {
    name: String,
    street: String,
    zip: String,
    city: String,
    phone: Option<String>,
    email: Option<String>,
    bank_name: Option<String>,
    iban: Option<String>,
    account_holder: Option<String>,
}

impl LandlordBody {
    /// Validated, trimmed values; empty optional text becomes `null`.
    fn validate(&self) -> ApiResult<LandlordData> {
        let mut v = Validator::new();
        let data = LandlordData {
            name: v.text("name", "Name", &self.name, TEXT_MAX),
            street: v.text("street", "Straße", &self.street, TEXT_MAX),
            zip: v.text("zip", "Postleitzahl", &self.zip, ZIP_MAX),
            city: v.text("city", "Ort", &self.city, TEXT_MAX),
            phone: v.opt_text("phone", "Telefon", self.phone.as_deref(), SHORT_MAX),
            email: v.opt_email("email", self.email.as_deref()),
            bank_name: v.opt_text("bankName", "Bank", self.bank_name.as_deref(), TEXT_MAX),
            iban: v.opt_iban("iban", self.iban.as_deref()),
            account_holder: v.opt_text(
                "accountHolder",
                "Kontoinhaber",
                self.account_holder.as_deref(),
                TEXT_MAX,
            ),
        };
        v.finish()?;
        Ok(data)
    }
}

/// The landlord of the account, `null` while none is stored.
pub async fn get(
    State(state): State<AppState>,
    ctx: AccountCtx,
) -> ApiResult<axum::Json<Option<LandlordJson>>> {
    let mut conn = state.conn().await?;
    let found = landlord::get(&mut conn, &ctx.scope).await?;
    Ok(axum::Json(found.map(LandlordJson::from)))
}

/// Stores the landlord (creates or replaces).
pub async fn save(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiJson(body): ApiJson<LandlordBody>,
) -> ApiResult<axum::Json<LandlordJson>> {
    let data = body.validate()?;
    let saved = audited(
        &state,
        &ctx,
        ("landlord.save", "landlord"),
        NOT_FOUND,
        |l: &Landlord| l.id.clone(),
        async |tx| landlord::save(tx, &ctx.scope, &data).await,
    )
    .await?;
    Ok(axum::Json(saved.into()))
}
