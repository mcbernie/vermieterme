//! Extractors that authenticate a web request and hand out the matching scope. They are
//! the only place where request handling builds scopes (the constructors are
//! `disallowed-methods` elsewhere); handlers get scopes only through them.
//!
//! The permission each extractor enforces is defined in [`super::permissions`].

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use vermieterme_db::repo::audit::{self, NewAuditEntry};
use vermieterme_db::repo::session::{self, ActiveSession};
use vermieterme_db::repo::user::{Role, User};
use vermieterme_db::row::now_millis;
use vermieterme_db::{AccountScope, AdminScope, AnyConnection, UserScope};

use super::cookie;
use super::permissions::{Access, require};
use crate::error::{ApiError, ApiResult};
use crate::routes::AppState;

/// Minimum time between two `last_seen_at` updates of a session (fewer writes; the idle
/// timeout is measured in hours).
const TOUCH_INTERVAL_MS: i64 = 60_000;

/// A signed-in web user ([`Access::Session`]).
#[derive(Debug, Clone)]
pub struct CurrentUser {
    /// The user (not locked; account not locked unless admin).
    pub user: User,
    /// Id of the current session (kept on password change).
    pub session_id: String,
    /// Account an admin switched into.
    pub acting_account_id: Option<String>,
}

impl CurrentUser {
    /// Scope for the user's own row.
    #[allow(clippy::disallowed_methods)] // User id comes from the verified session.
    pub fn scope(&self) -> UserScope {
        UserScope::from_session(&self.user.id)
    }
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    /// Valid `vm_session` cookie -> user; otherwise `401 unauthenticated`. The result is
    /// cached in the request, so several extractors look the session up once.
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> ApiResult<Self> {
        if let Some(cached) = parts.extensions.get::<CurrentUser>() {
            return Ok(cached.clone());
        }
        let token = cookie::read(&parts.headers).ok_or_else(ApiError::no_session)?;
        let now = now_millis();
        let idle_since = super::session::idle_since(&state.config, now);
        let mut conn = state.conn().await?;
        let ActiveSession { session, user } =
            session::find_valid(&mut conn, &token, now, idle_since)
                .await?
                .ok_or_else(ApiError::no_session)?;
        if now - session.last_seen_at > TOUCH_INTERVAL_MS {
            session::touch(&mut conn, &session.id, now).await?;
        }
        let current = CurrentUser {
            user,
            session_id: session.id,
            acting_account_id: session.acting_account_id,
        };
        parts.extensions.insert(current.clone());
        Ok(current)
    }
}

/// A web user working inside an account ([`Access::Account`]): a member in their own
/// account, an admin in the account switched into (or their own).
#[derive(Debug, Clone)]
pub struct AccountCtx {
    /// Scope for all customer data of this request.
    pub scope: AccountScope,
    /// The signed-in user.
    pub user: User,
    /// Id of the current session.
    pub session_id: String,
    /// An admin works inside a foreign account (support switch).
    pub impersonating: bool,
}

impl AccountCtx {
    /// Records a change in the audit log if an admin made it inside a foreign account
    /// (plan "Kunde wechseln"). Call it in the transaction of the change. Handlers of
    /// customer data call this for every create, update and delete.
    pub async fn audit(
        &self,
        conn: &mut AnyConnection,
        action: &'static str,
        entity: &'static str,
        entity_id: Option<&str>,
    ) -> vermieterme_db::DbResult<()> {
        if !self.impersonating {
            return Ok(());
        }
        let entry = NewAuditEntry {
            user_id: Some(&self.user.id),
            account_id: Some(self.scope.account_id()),
            action,
            entity,
            entity_id,
        };
        audit::record(conn, &entry).await.map(|_| ())
    }
}

impl FromRequestParts<AppState> for AccountCtx {
    type Rejection = ApiError;

    /// Session -> account scope; an admin without own or switched account gets `403`.
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> ApiResult<Self> {
        let current = CurrentUser::from_request_parts(parts, state).await?;
        require(current.user.role, Access::Account)?;
        let (scope, impersonating) =
            active_account(&current.user, current.acting_account_id.as_deref())
                .ok_or_else(|| ApiError::forbidden("Bitte zuerst einen Kunden auswählen"))?;
        Ok(Self {
            scope,
            user: current.user,
            session_id: current.session_id,
            impersonating,
        })
    }
}

/// An admin ([`Access::Admin`]).
#[derive(Debug, Clone)]
pub struct AdminCtx {
    /// Cross-account scope of the admin.
    pub scope: AdminScope,
    /// The admin.
    pub user: User,
    /// Id of the current session (switching keeps it; only the acting account changes).
    pub session_id: String,
    /// Account currently switched into.
    pub acting_account_id: Option<String>,
}

impl FromRequestParts<AppState> for AdminCtx {
    type Rejection = ApiError;

    /// Session with role `admin` (verified in the database by the session lookup);
    /// members get `403 forbidden`.
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> ApiResult<Self> {
        let current = CurrentUser::from_request_parts(parts, state).await?;
        require(current.user.role, Access::Admin)?;
        Ok(Self {
            scope: admin_scope(&current.user),
            user: current.user,
            session_id: current.session_id,
            acting_account_id: current.acting_account_id,
        })
    }
}

/// The account a session works in and whether that is a foreign one: members their own
/// account; admins the switched account, else their own (decision 21), else none.
#[allow(clippy::disallowed_methods)] // Account ids come from the verified session and user row.
pub(crate) fn active_account(user: &User, acting: Option<&str>) -> Option<(AccountScope, bool)> {
    match (user.role, acting) {
        (Role::Admin, Some(acting)) => {
            let foreign = user.account_id.as_deref() != Some(acting);
            Some((admin_scope(user).act_in(acting), foreign))
        }
        _ => user
            .account_id
            .as_deref()
            .map(|own| (AccountScope::from_session(own), false)),
    }
}

/// Admin scope of a user whose role was verified by the session lookup.
#[allow(clippy::disallowed_methods)] // Callers checked `Role::Admin` from the database.
fn admin_scope(user: &User) -> AdminScope {
    AdminScope::verified_admin(&user.id)
}

/// Duration in milliseconds (saturating).
pub(crate) fn millis(duration: std::time::Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}
