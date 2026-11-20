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

/// Access to the data of exactly one account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountScope {
    account_id: String,
}

impl AccountScope {
    /// Scope for the account of an authenticated session. Call only with the account id
    /// taken from the server-side session, never with an id from the request.
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
    account: AccountScope,
    tenant_id: String,
    unit_id: String,
    property_id: String,
}

impl TenantScope {
    /// Scope for a tenant whose account, unit and property were looked up in the database
    /// from the verified tenant id (never taken from the request or the token alone).
    pub fn from_verified_tenant(
        account_id: impl Into<String>,
        tenant_id: impl Into<String>,
        unit_id: impl Into<String>,
        property_id: impl Into<String>,
    ) -> Self {
        Self {
            account: AccountScope::from_session(account_id),
            tenant_id: tenant_id.into(),
            unit_id: unit_id.into(),
            property_id: property_id.into(),
        }
    }

    /// The account of the tenant.
    pub fn account(&self) -> &AccountScope {
        &self.account
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

/// Cross-account administration. Built by the HTTP layer only after checking the role
/// `admin`, or by the system itself (bootstrap, legacy import).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminScope {
    actor_user_id: Option<String>,
}

impl AdminScope {
    /// Scope of a signed-in user whose role `admin` was verified.
    pub fn verified_admin(user_id: impl Into<String>) -> Self {
        Self {
            actor_user_id: Some(user_id.into()),
        }
    }

    /// Scope of the application itself (admin bootstrap at start, legacy import).
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
        AccountScope::from_session(account_id)
    }
}

/// The signed-in user acting on their own row (profile, password).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserScope {
    user_id: String,
}

impl UserScope {
    /// Scope for the user of an authenticated session.
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
