//! Billing of an account (contract section 7, P5): billing periods with copy and the
//! per-unit totals of the domain, cost positions with amounts per unit, prepayments.
//!
//! Every handler takes [`AccountCtx`](crate::auth::AccountCtx) and passes its scope to
//! the repositories, so foreign ids are 404. Every change runs with its audit entry in one
//! transaction (the master data helper [`audited_checked`]). Costs and prepayments are
//! saved with `PATCH`: only sent fields change, `null` clears, an outdated `version` is
//! 409 `version_conflict` with the stored state in `current` (#25, #67). The web PDF
//! (`/pdf`) comes with the PDF bridge (P8).

mod cost;
mod detail;
mod json;
mod period;
mod prepayment;

use axum::routing::{get, patch};
use serde::{Deserialize, Deserializer};
use vermieterme_db::repo::billing::Rejection;
use vermieterme_domain::Cents;

use self::json::{CostJson, PrepaymentJson, date_de};
use crate::error::ApiError;
use crate::error::validate::Validator;
use crate::routes::AppRouter;

/// Message for a missing or foreign billing period.
pub const NOT_FOUND: &str = "Abrechnungszeitraum nicht gefunden";

/// Routes of the area.
pub fn router() -> AppRouter {
    AppRouter::new()
        .route(
            "/api/billing-periods",
            get(period::list).post(period::create),
        )
        .route(
            "/api/billing-periods/{id}",
            get(period::detail)
                .put(period::update)
                .delete(period::remove),
        )
        .route("/api/billing-periods/{id}/costs", get(cost::list))
        .route(
            "/api/billing-periods/{id}/costs/{category_id}",
            patch(cost::save),
        )
        .route(
            "/api/billing-periods/{id}/costs/{category_id}/units/{unit_id}",
            patch(cost::save_unit),
        )
        .route(
            "/api/billing-periods/{id}/prepayments",
            get(prepayment::list),
        )
        .route(
            "/api/billing-periods/{id}/prepayments/{unit_id}",
            patch(prepayment::save),
        )
}

impl From<Rejection> for ApiError {
    /// The error answer for a refused billing change.
    fn from(rejection: Rejection) -> Self {
        match rejection {
            Rejection::Overlap { start, end } => ApiError::overlap(format!(
                "Überlappender Abrechnungszeitraum existiert bereits: {} – {}",
                date_de(&start),
                date_de(&end)
            )),
            Rejection::EndNotAfterStart => {
                ApiError::field("endDate", "Das Enddatum muss nach dem Beginndatum liegen")
            }
            Rejection::NoUnits => ApiError::field(
                "propertyId",
                "Das Objekt hat keine Einheiten. Bitte legen Sie zuerst Einheiten an.",
            ),
            Rejection::SourceNotFound => {
                ApiError::not_found("Abrechnungszeitraum zum Kopieren nicht gefunden")
            }
            Rejection::SourceOtherProperty => ApiError::field(
                "copyFromId",
                "Der Abrechnungszeitraum zum Kopieren gehört zu einem anderen Objekt",
            ),
            Rejection::AmountRequired => {
                ApiError::field("monthlyAmountCents", "Monatlicher Betrag fehlt")
            }
            Rejection::CostVersion(current) => conflict(CostJson::from(*current)),
            Rejection::PrepaymentVersion(current) => conflict(PrepaymentJson::from(*current)),
        }
    }
}

/// 409 `version_conflict` with `current` as JSON.
fn conflict(current: impl serde::Serialize) -> ApiError {
    ApiError::version_conflict(serde_json::to_value(current).unwrap_or_default())
}

/// Deserializer for patch fields that tell "absent" (`None`, via `#[serde(default)]`)
/// from `null` (`Some(None)`, clears the value) and a value (`Some(Some(v))`).
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

impl Validator {
    /// An amount in cents within the cent limit; `min` is 0 for amounts that cannot be
    /// negative (prepayments) and `-LIMIT` for costs (credits are possible).
    pub fn cents(&mut self, field: &str, label: &str, value: i64, non_negative: bool) -> i64 {
        let min = if non_negative { 0 } else { -Cents::LIMIT };
        self.int_in(field, label, value, (min, Cents::LIMIT))
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::present;

    /// A patch body with one nullable field.
    #[derive(Deserialize)]
    struct Body {
        #[serde(default, deserialize_with = "present")]
        value: Option<Option<i64>>,
    }

    #[test]
    fn absent_null_and_value_differ() {
        let parse = |s: &str| serde_json::from_str::<Body>(s).unwrap().value;
        assert_eq!(parse("{}"), None);
        assert_eq!(parse(r#"{"value": null}"#), Some(None));
        assert_eq!(parse(r#"{"value": 5}"#), Some(Some(5)));
    }
}
