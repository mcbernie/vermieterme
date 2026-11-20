//! Shared set-up of the master data tests: two accounts with one member each, an admin
//! without account, and short creators for the master data objects.

#![allow(dead_code)] // Not every test binary uses every helper.
#![allow(clippy::unwrap_used)]

use serde_json::{Value, json};
use vermieterme_db::Db;
use vermieterme_db::repo::user::Role;

use super::support::{PASSWORD, Resp, TestApp, seed_account, seed_user};

/// Two accounts, their members' sessions and an admin session.
pub struct World {
    /// The running server.
    pub app: TestApp,
    /// Session cookie of the member of account A.
    pub a: String,
    /// Session cookie of the member of account B.
    pub b: String,
    /// Session cookie of the admin (no own account, not switched).
    pub admin: String,
    /// Id of account A.
    pub account_a: String,
}

/// Starts the server and creates [`World`].
pub async fn world(db: &Db) -> World {
    let app = TestApp::start(db).await;
    let a = seed_account(db, "Kunde A").await;
    let b = seed_account(db, "Kunde B").await;
    seed_user(db, Some(&a.id), "a@example.org", Role::Member).await;
    seed_user(db, Some(&b.id), "b@example.org", Role::Member).await;
    seed_user(db, None, "admin@example.org", Role::Admin).await;
    World {
        a: app.login("a@example.org", PASSWORD).await,
        b: app.login("b@example.org", PASSWORD).await,
        admin: app.login("admin@example.org", PASSWORD).await,
        account_a: a.id,
        app,
    }
}

impl World {
    /// The admin works inside account A (support switch).
    pub async fn admin_enters_a(&self) {
        let body = json!({"accountId": self.account_a});
        let resp = self
            .app
            .call("POST", "/api/admin/switch", Some(&self.admin), body)
            .await;
        assert_eq!(resp.status, 200, "{:?}", resp.body);
    }

    /// Sends a request with a body and expects `status`; returns the response.
    pub async fn expect(
        &self,
        status: u16,
        method: &str,
        path: &str,
        cookie: &str,
        body: Value,
    ) -> Resp {
        let resp = self.app.call(method, path, Some(cookie), body).await;
        assert_eq!(resp.status, status, "{method} {path}: {:?}", resp.body);
        resp
    }

    /// Creates a property with `totalShares` 100 for `cookie`; returns its id.
    pub async fn property(&self, cookie: &str) -> String {
        let body = json!({"street": "Musterweg 1", "zip": "12345", "city": "Beispielstadt", "totalShares": 100});
        let resp = self
            .expect(201, "POST", "/api/properties", cookie, body)
            .await;
        id_of(&resp)
    }

    /// Creates a unit with `shares` in the property; returns its id.
    pub async fn unit(&self, cookie: &str, property_id: &str, shares: i64) -> String {
        let body = json!({"propertyId": property_id, "name": "Wohnung 1", "floor": "EG", "shares": shares});
        let resp = self.expect(201, "POST", "/api/units", cookie, body).await;
        id_of(&resp)
    }

    /// Creates a tenant in the unit; returns its id.
    pub async fn tenant(&self, cookie: &str, unit_id: &str, extra: Value) -> String {
        let mut body = json!({
            "unitId": unit_id, "salutation": "Frau", "firstName": "Erika",
            "lastName": "Muster", "moveInDate": "2024-01-01"
        });
        if let (Some(base), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
            base.extend(extra.clone());
        }
        let resp = self.expect(201, "POST", "/api/tenants", cookie, body).await;
        id_of(&resp)
    }
}

/// The `id` of a response body.
pub fn id_of(resp: &Resp) -> String {
    resp.body["id"].as_str().unwrap().to_owned()
}
