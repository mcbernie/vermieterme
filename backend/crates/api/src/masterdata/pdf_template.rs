//! `GET/PUT /api/pdf-template`: the PDF template of the account. Without a stored one
//! the default of the old web code is returned.

use axum::extract::State;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vermieterme_db::repo::masterdata::pdf_template::{self, CONFIG_MAX, PdfTemplate};

use super::audited;
use super::fields::TEXT_MAX;
use super::pdf_config::TemplateConfig;
use crate::auth::AccountCtx;
use crate::error::json::ApiJson;
use crate::error::validate::Validator;
use crate::error::{ApiError, ApiResult};
use crate::routes::AppState;

/// Message used if the row vanishes between write and read (cannot happen in practice).
const NOT_FOUND: &str = "Vorlage nicht gefunden";
/// Name of the template without a stored one.
const DEFAULT_NAME: &str = "Standard";

/// The template in `GET` responses; `id` is `null` for the default.
#[derive(Debug, Serialize)]
pub struct TemplateJson {
    id: Option<String>,
    name: String,
    config: Value,
}

/// Response of `PUT`.
#[derive(Debug, Serialize)]
pub struct SavedJson {
    id: String,
    name: String,
}

/// Request of `PUT /api/pdf-template`.
#[derive(Debug, Deserialize)]
pub struct TemplateBody {
    config: Value,
    name: Option<String>,
}

/// The stored template or the default.
pub async fn get(
    State(state): State<AppState>,
    ctx: AccountCtx,
) -> ApiResult<axum::Json<TemplateJson>> {
    let mut conn = state.conn().await?;
    let json = match pdf_template::get(&mut conn, &ctx.scope).await? {
        Some(PdfTemplate {
            id, name, config, ..
        }) => TemplateJson {
            id: Some(id),
            name,
            config: serde_json::from_str(&config).map_err(|_| ApiError::internal())?,
        },
        None => TemplateJson {
            id: None,
            name: DEFAULT_NAME.to_owned(),
            config: TemplateConfig::default_value(),
        },
    };
    Ok(axum::Json(json))
}

/// Stores the template; an invalid or oversized configuration is 400.
pub async fn save(
    State(state): State<AppState>,
    ctx: AccountCtx,
    ApiJson(body): ApiJson<TemplateBody>,
) -> ApiResult<axum::Json<SavedJson>> {
    let mut v = Validator::new();
    if let Err(message) = TemplateConfig::check(&body.config) {
        v.fail("config", message);
    }
    let config = body.config.to_string();
    if config.chars().count() > CONFIG_MAX {
        v.fail("config", "Die Vorlage ist zu groß");
    }
    let name = v.opt_text("name", "Name", body.name.as_deref(), TEXT_MAX);
    v.finish()?;
    let saved = audited(
        &state,
        &ctx,
        ("pdf_template.save", "pdf_template"),
        NOT_FOUND,
        |t: &PdfTemplate| t.id.clone(),
        async |tx| pdf_template::save(tx, &ctx.scope, &config, name.as_deref()).await,
    )
    .await?;
    Ok(axum::Json(SavedJson {
        id: saved.id,
        name: saved.name,
    }))
}
