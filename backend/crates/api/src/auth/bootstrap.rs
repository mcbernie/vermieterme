//! Admin bootstrap at start (plan "Auth", #69): if no admin exists and `ADMIN_EMAIL` and
//! `ADMIN_PASSWORD` are set, the first admin is created. Afterwards the variables are
//! ignored; the environment password is never checked at sign-in (only the stored hash).

use vermieterme_db::repo::account;
use vermieterme_db::repo::user::{self, NewUser, Role, UserPatch};
use vermieterme_db::{AdminScope, Db, DbError, in_transaction};

use super::password;
use crate::config::Config;
use crate::error::validate::Validator;

/// What the bootstrap did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// `ADMIN_EMAIL` or `ADMIN_PASSWORD` not set.
    NotConfigured,
    /// An admin exists already; nothing changed.
    AdminExists,
    /// A new admin was created (member of the first account, if there is one).
    Created,
    /// The existing user with `ADMIN_EMAIL` became admin (decision 21, migrated data);
    /// their password stays.
    Promoted,
}

/// Failure of the bootstrap; aborts the start.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    /// `ADMIN_EMAIL`/`ADMIN_PASSWORD` break the input rules.
    #[error("ADMIN_EMAIL oder ADMIN_PASSWORD ungültig: {0}")]
    Invalid(String),
    /// Hashing the password failed.
    #[error("Passwort konnte nicht gehasht werden")]
    Hash,
    /// Database error.
    #[error(transparent)]
    Db(#[from] DbError),
}

/// Runs the bootstrap once at start.
#[allow(clippy::disallowed_methods)] // System scope: start-up bootstrap, no request involved.
pub async fn run(db: &Db, config: &Config) -> Result<Outcome, BootstrapError> {
    let (Some(email), Some(secret)) = (&config.admin_email, &config.admin_password) else {
        return Ok(Outcome::NotConfigured);
    };
    let mut conn = db.pool().acquire().await.map_err(DbError::from)?;
    // Checked first, so stale variables of an installation with admin never block a start.
    if user::admin_exists(&mut conn).await? {
        return Ok(Outcome::AdminExists);
    }
    let mut v = Validator::new();
    let email = v.email("email", email);
    v.password("password", secret.expose());
    v.finish()
        .map_err(|e| BootstrapError::Invalid(e.message().to_owned()))?;
    let hash = password::hash(secret.expose().to_owned())
        .await
        .map_err(|_| BootstrapError::Hash)?;
    let system = AdminScope::system();
    // Re-checked inside the transaction against a concurrent start.
    let outcome = in_transaction(&mut conn, async |tx| {
        if user::admin_exists(tx).await? {
            return Ok(Outcome::AdminExists);
        }
        if let Some(existing) = user::find_login(tx, &email).await? {
            let patch = UserPatch {
                role: Some(Role::Admin),
                ..UserPatch::default()
            };
            user::update_for_admin(tx, &system, &existing.user.id, &patch).await?;
            return Ok(Outcome::Promoted);
        }
        let first = account::first(tx, &system).await?;
        let new = NewUser {
            email: email.clone(),
            name: "Administrator".to_owned(),
            role: Role::Admin,
            password_hash: Some(hash),
        };
        user::create_for_admin(tx, &system, first.as_ref().map(|a| a.id.as_str()), &new).await?;
        Ok(Outcome::Created)
    })
    .await?;
    Ok(outcome)
}
