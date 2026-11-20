//! Shared fixtures of the repository tests.

use sqlx::AnyConnection;
use vermieterme_db::repo::account;
use vermieterme_db::repo::user::{self, NewUser, Role};
use vermieterme_db::{AccountScope, AdminScope};

/// One hour in milliseconds.
pub const HOUR: i64 = 3_600_000;
/// A fixed "now" (2027-01-15) for expiry tests.
pub const NOW: i64 = 1_800_000_000_000;

/// Creates an account with one user of `role`; returns scope and user id.
pub async fn setup(c: &mut AnyConnection, email: &str, role: Role) -> (AccountScope, String) {
    let admin = AdminScope::system();
    let scope = admin.act_in(account::create(c, &admin, "A").await.unwrap().id);
    let new = NewUser {
        email: email.into(),
        name: "N".into(),
        role,
        password_hash: None,
    };
    let user = user::create(c, &scope, &new).await.unwrap();
    (scope, user.id)
}
