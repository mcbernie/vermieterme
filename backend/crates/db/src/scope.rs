//! Scope types: every repository function with account (customer) reference takes one.
//!
//! A scope says *on whose behalf* data is read or written. The HTTP layer builds it from
//! the authenticated session (never from request parameters); repositories filter every
//! query on it, so a foreign id behaves exactly like a missing one ([`crate::DbError::NotFound`]).
//!
//! - [`AccountScope`]: data of one account (customer). Built from the session's account
//!   or, for an admin who switched into a customer, via [`AdminScope::act_in`].
//! - [`TenantScope`]: a tenant-app request; account, tenant, unit and property are
//!   determined server-side from the tenant.
//! - [`AdminScope`]: cross-account administration (accounts, all users, audit log).
//! - [`UserScope`]: the signed-in user's own row (profile, password).
//!
//! # Who may construct a scope
//!
//! The constructors are `pub` (the HTTP crate needs them) but listed under
//! `disallowed-methods` in `backend/clippy.toml`, so `just lint` fails on every call
//! that is not explicitly allowed. Allowed places, each with
//! `#[allow(clippy::disallowed_methods)]` and a comment saying why:
//! - `api/src/auth*`: after verifying the session, the tenant JWT or the admin role,
//! - the admin bootstrap at start and `legacy_import` ([`AdminScope::system`]),
//! - tests.
//!
//! Inside this crate no scope is constructed from loose strings.
//!
//! # No widening between scopes
//!
//! No scope hands out a broader one. In particular a [`TenantScope`] never yields an
//! [`AccountScope`]: a tenant-app handler gets the account id as a plain string (to filter
//! the tenant's own rows inside this crate), never a scope for the customer's data. The
//! only way to an [`AccountScope`] besides the session check is [`AdminScope::act_in`],
//! which needs an admin scope that already reaches every account. [`UserScope`] gives
//! only its user id.
//!
//! ```
//! # use vermieterme_db::TenantScope;
//! fn own_account(tenant: &TenantScope) -> &str {
//!     tenant.account_id()
//! }
//! ```
//!
//! ```compile_fail
//! # use vermieterme_db::{AccountScope, TenantScope};
//! fn widen(tenant: &TenantScope) -> &AccountScope {
//!     tenant.account()
//! }
//! ```
//!
//! ```compile_fail
//! # use vermieterme_db::{AccountScope, TenantScope};
//! fn widen(tenant: TenantScope) -> AccountScope {
//!     tenant.into()
//! }
//! ```

/// Access to the data of exactly one account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountScope {
    account_id: String,
}

impl AccountScope {
    /// Scope for the account of an authenticated session.
    ///
    /// Callers: only the session check in `api/src/auth*` (account id from the server-side
    /// session, never from the request) and tests. Admins switching into a customer use
    /// [`AdminScope::act_in`]. Disallowed elsewhere by clippy.
    pub fn from_session(account_id: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
        }
    }

    /// The account all queries are restricted to.
    pub fn account_id(&self) -> &str {
        &self.account_id
    }
}

/// Access of the tenant app: one tenant of one account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantScope {
    account_id: String,
    tenant_id: String,
    unit_id: String,
    property_id: String,
}

impl TenantScope {
    /// Scope for a tenant whose account, unit and property were looked up in the database
    /// from the verified tenant id (never taken from the request or the token alone).
    ///
    /// Callers: only the tenant-JWT check in `api/src/auth*` and tests. Disallowed
    /// elsewhere by clippy.
    pub fn from_verified_tenant(
        account_id: impl Into<String>,
        tenant_id: impl Into<String>,
        unit_id: impl Into<String>,
        property_id: impl Into<String>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            tenant_id: tenant_id.into(),
            unit_id: unit_id.into(),
            property_id: property_id.into(),
        }
    }

    /// The account of the tenant, as a plain id for filtering the tenant's own rows.
    /// Deliberately no [`AccountScope`]: the tenant app must not reach the customer's
    /// data (see the module documentation).
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// The tenant.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// The unit the tenant rents.
    pub fn unit_id(&self) -> &str {
        &self.unit_id
    }

    /// The property of that unit.
    pub fn property_id(&self) -> &str {
        &self.property_id
    }
}

/// Cross-account administration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminScope {
    actor_user_id: Option<String>,
}

impl AdminScope {
    /// Scope of a signed-in user whose role `admin` was verified against the database.
    ///
    /// Callers: only the admin check in `api/src/auth*` and tests. Disallowed elsewhere
    /// by clippy.
    pub fn verified_admin(user_id: impl Into<String>) -> Self {
        Self {
            actor_user_id: Some(user_id.into()),
        }
    }

    /// Scope of the application itself, with full cross-account access.
    ///
    /// Callers: only the admin bootstrap at start, `legacy_import` and tests – never a
    /// request handler. Disallowed elsewhere by clippy.
    pub fn system() -> Self {
        Self {
            actor_user_id: None,
        }
    }

    /// The acting admin, `None` for the system.
    pub fn actor_user_id(&self) -> Option<&str> {
        self.actor_user_id.as_deref()
    }

    /// Account scope for an admin working inside a customer (support switch). Every
    /// change made with it must be written to the audit log by the caller.
    pub fn act_in(&self, account_id: impl Into<String>) -> AccountScope {
        AccountScope {
            account_id: account_id.into(),
        }
    }
}

/// The signed-in user acting on their own row (profile, password).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserScope {
    user_id: String,
}

impl UserScope {
    /// Scope for the user of an authenticated session.
    ///
    /// Callers: only the session check in `api/src/auth*` (and the invitation/reset flow
    /// there, with the user id from a consumed one-time link) and tests. Disallowed
    /// elsewhere by clippy.
    pub fn from_session(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
        }
    }

    /// The user.
    pub fn user_id(&self) -> &str {
        &self.user_id
    }
}
