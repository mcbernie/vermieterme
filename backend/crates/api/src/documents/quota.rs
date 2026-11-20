//! Limits of uploads per account (decision 44, #124): the storage quota
//! (`UPLOAD_QUOTA_BYTES`, checked by the repository inside the insert transaction) and at
//! most [`UPLOADS_PER_HOUR`] uploads per hour.

use std::time::Duration;

use vermieterme_db::repo::documents::QuotaExceeded;

use crate::auth::AccountCtx;
use crate::auth::rate_limit::{Attempt, Policy, RateLimiter};
use crate::error::{ApiError, ApiResult};

/// Uploads an account may start per hour.
pub const UPLOADS_PER_HOUR: u32 = 60;

/// Rate limit of uploads, keyed by account: after [`UPLOADS_PER_HOUR`] uploads within an
/// hour the account waits an hour (429 with `Retry-After`).
pub const UPLOADS_PER_ACCOUNT: Policy = Policy {
    prefix: "upload-account",
    max_failures: UPLOADS_PER_HOUR,
    window: Duration::from_secs(3600),
    lockout: Duration::from_secs(3600),
    reset_on_success: false,
};

/// Reserves one upload of the account, or 429. The reservation is armed, so it counts
/// as one upload however the request ends (stored, refused or cancelled) once the
/// returned attempt is dropped.
pub fn begin_upload<'a>(limiter: &'a RateLimiter, ctx: &AccountCtx) -> ApiResult<Attempt<'a>> {
    let mut attempt = limiter
        .begin(&UPLOADS_PER_ACCOUNT, ctx.scope.account_id())
        .map_err(ApiError::rate_limited)?;
    attempt.arm();
    Ok(attempt)
}

impl From<QuotaExceeded> for ApiError {
    /// 413 `quota_exceeded`.
    fn from(_: QuotaExceeded) -> Self {
        ApiError::quota_exceeded()
    }
}
