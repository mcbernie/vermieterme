//! Permission mapping – the one place that says which role may use which kind of
//! endpoint (security baseline "Zugriff", plan "Scope").
//!
//! Every handler declares its permission through the extractor in its signature:
//!
//! | Access | Extractor | Who |
//! |---|---|---|
//! | [`Access::Public`] | none | everyone (login, links, health) |
//! | [`Access::Session`] | [`super::CurrentUser`] | signed-in users (profile, session, logout) |
//! | [`Access::Account`] | [`super::AccountCtx`] | `member` (own), `admin` (own or switched) |
//! | [`Access::Admin`] | [`super::AdminCtx`] | `admin` only |
//! | [`Access::Tenant`] | tenant-JWT extractor (P7) | tenants of the app, never web users |
//!
//! Extractors call [`require`]; handlers never compare roles themselves.

use vermieterme_db::repo::user::Role;

use crate::error::ApiError;

/// Kind of access an endpoint needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// No sign-in.
    Public,
    /// Any valid web session.
    Session,
    /// A web session working inside an account (customer data).
    Account,
    /// Administration across accounts.
    Admin,
    /// Tenant app (JWT); no web role has it.
    Tenant,
}

/// Whether a web user with `role` has `access`.
pub const fn allows(role: Role, access: Access) -> bool {
    match access {
        Access::Public | Access::Session | Access::Account => true,
        Access::Admin => matches!(role, Role::Admin),
        Access::Tenant => false,
    }
}

/// `Ok` if `role` has `access`, else `403 forbidden`.
pub fn require(role: Role, access: Access) -> Result<(), ApiError> {
    if allows(role, access) {
        Ok(())
    } else {
        Err(ApiError::forbidden("Keine Berechtigung"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_cannot_administrate() {
        assert!(allows(Role::Member, Access::Account));
        assert!(!allows(Role::Member, Access::Admin));
        assert!(allows(Role::Admin, Access::Admin));
        assert!(allows(Role::Admin, Access::Account));
    }

    #[test]
    fn web_users_are_no_tenants() {
        assert!(!allows(Role::Admin, Access::Tenant));
        assert!(!allows(Role::Member, Access::Tenant));
        assert_eq!(
            require(Role::Member, Access::Admin).unwrap_err().status(),
            axum::http::StatusCode::FORBIDDEN
        );
    }
}
