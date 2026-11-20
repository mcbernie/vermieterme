//! `POST /api/vpi/fetch`: retrieves index values from the Bundesbank (SDMX-JSON) without
//! storing them; the user decides which ones to save. The base URL comes from
//! `VPI_API_BASE` (tests point it at a local server); the parsing is
//! `vermieterme_domain::vpi::parse_bundesbank_response`.

use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use vermieterme_domain::vpi::{VpiDataPoint, base_year_code, parse_bundesbank_response};

use crate::auth::AccountCtx;
use crate::error::ApiError;
use crate::error::json::ApiJson;
use crate::routes::AppState;

/// Time limit of the whole retrieval (as before).
const TIMEOUT: Duration = Duration::from_secs(15);
/// Largest accepted answer (bytes); a monthly series of a few years is a few kilobytes.
const BODY_MAX: usize = 5 * 1024 * 1024;
/// Years of history requested besides the base year.
const HISTORY_YEARS: i32 = 5;
/// Message when nothing could be retrieved or understood.
const UPSTREAM_MESSAGE: &str =
    "Fehler beim Abrufen der VPI-Daten. Bitte VPI-Werte manuell eintragen.";
/// Message when the Bundesbank has no data.
const EMPTY_MESSAGE: &str = "Keine VPI-Daten gefunden. Bitte VPI-Werte manuell eintragen.";

/// Request body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchBody {
    base_year: i64,
}

/// One retrieved month.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EntryJson {
    year: i32,
    month: u8,
    value_tenths: i64,
}

/// Response body.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FetchJson {
    entries: Vec<EntryJson>,
    base_year: i64,
}

/// The external service failed (status, network, timeout, unreadable answer).
#[derive(Debug)]
struct UpstreamError;

impl IntoResponse for UpstreamError {
    /// `502 {"error": …, "code": "upstream"}` in the shape of the contract's error table.
    fn into_response(self) -> Response {
        let body = serde_json::json!({"error": UPSTREAM_MESSAGE, "code": "upstream"});
        (StatusCode::BAD_GATEWAY, axum::Json(body)).into_response()
    }
}

/// Retrieves the values of a base year.
pub async fn fetch(
    State(state): State<AppState>,
    _ctx: AccountCtx,
    ApiJson(body): ApiJson<FetchBody>,
) -> Response {
    let Some(code) = i32::try_from(body.base_year).ok().and_then(base_year_code) else {
        return ApiError::field(
            "baseYear",
            "Basisjahr wird nicht unterstützt (2010, 2015, 2020)",
        )
        .into_response();
    };
    let today = OffsetDateTime::now_utc();
    // The supported base years are in the i32 range (checked above).
    let base_year = i32::try_from(body.base_year).unwrap_or(2020);
    let start = base_year.max(today.year() - HISTORY_YEARS);
    let url = format!(
        "{}/BBDP1/M.DE.N.VPI.C.A00000.{code}.A?detail=dataonly&startPeriod={start}-01&endPeriod={}-{:02}",
        state.config.vpi_api_base.trim_end_matches('/'),
        today.year(),
        u8::from(today.month()),
    );
    match request(&url).await {
        Err(UpstreamError) => UpstreamError.into_response(),
        Ok(points) if points.is_empty() => ApiError::not_found(EMPTY_MESSAGE).into_response(),
        Ok(points) => axum::Json(FetchJson {
            entries: points
                .into_iter()
                .map(|p| EntryJson {
                    year: p.year,
                    month: p.month,
                    value_tenths: p.value.tenths(),
                })
                .collect(),
            base_year: body.base_year,
        })
        .into_response(),
    }
}

/// Gets and parses one series. No redirects are followed and the answer is size-limited;
/// the URL and the answer are never logged.
async fn request(url: &str) -> Result<Vec<VpiDataPoint>, UpstreamError> {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| UpstreamError)?;
    let mut response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|_| UpstreamError)?;
    if !response.status().is_success() {
        return Err(UpstreamError);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| UpstreamError)? {
        if bytes.len() + chunk.len() > BODY_MAX {
            return Err(UpstreamError);
        }
        bytes.extend_from_slice(&chunk);
    }
    let text = String::from_utf8(bytes).map_err(|_| UpstreamError)?;
    parse_bundesbank_response(&text).map_err(|_| UpstreamError)
}
