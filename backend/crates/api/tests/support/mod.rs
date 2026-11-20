//! Test harness: runs the real router (with all layers) on a local port against a test
//! database and talks to it with a minimal HTTP/1.0 client (no extra dependency).

#![allow(dead_code)] // Not every test binary uses every helper.

use std::net::SocketAddr;

use serde_json::Value;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use vermieterme_api::auth::password;
use vermieterme_api::config::Config;
use vermieterme_api::routes::{AppState, router};
use vermieterme_db::repo::account::{self, Account};
use vermieterme_db::repo::user::{self, NewUser, Role, User};
use vermieterme_db::{AdminScope, Db, PasswordHash};

/// `AUTH_SECRET` of all tests (synthetic).
pub const SECRET: &str = "test-secret-test-secret-test-secret";
/// Password of the seeded users.
pub const PASSWORD: &str = "richtig-geheim-1";

/// Configuration with defaults plus `pairs`.
pub fn config(pairs: &[(&str, &str)]) -> Config {
    let pairs: Vec<(String, String)> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .chain([("AUTH_SECRET".to_owned(), SECRET.to_owned())])
        .collect();
    Config::from_lookup(move |name| {
        pairs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    })
    .unwrap()
}

/// A running server.
pub struct TestApp {
    /// Address of the server.
    pub addr: SocketAddr,
    /// Its database.
    pub db: Db,
}

impl TestApp {
    /// Starts the router on `127.0.0.1:0` with the default test configuration.
    pub async fn start(db: &Db) -> Self {
        Self::start_with(db, config(&[])).await
    }

    /// Starts the router with `config`.
    pub async fn start_with(db: &Db, config: Config) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = router(AppState::new(db.clone(), config))
            .into_make_service_with_connect_info::<SocketAddr>();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            addr,
            db: db.clone(),
        }
    }

    /// Sends a request; `cookie` is the `vm_session=…` pair, mutating requests get the
    /// CSRF header unless `extra` replaces it (pass `("x-csrf-guard", "")` to omit).
    pub async fn send(
        &self,
        method: &str,
        path: &str,
        cookie: Option<&str>,
        body: Option<Value>,
        extra: &[(&str, &str)],
    ) -> Resp {
        let body = body.map(|b| b.to_string()).unwrap_or_default();
        let mut head = format!("{method} {path} HTTP/1.0\r\nHost: {}\r\n", self.addr);
        if !body.is_empty() {
            head.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            ));
        }
        if let Some(cookie) = cookie {
            head.push_str(&format!("Cookie: {cookie}\r\n"));
        }
        let custom_csrf = extra
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("x-csrf-guard"));
        if method != "GET" && !custom_csrf {
            head.push_str("X-CSRF-Guard: 1\r\n");
        }
        for (k, v) in extra.iter().filter(|(_, v)| !v.is_empty()) {
            head.push_str(&format!("{k}: {v}\r\n"));
        }
        head.push_str("\r\n");
        let mut stream = TcpStream::connect(self.addr).await.unwrap();
        head.push_str(&body);
        stream.write_all(head.as_bytes()).await.unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await.unwrap();
        Resp::parse(&raw)
    }

    /// `GET` with optional session.
    pub async fn get(&self, path: &str, cookie: Option<&str>) -> Resp {
        self.send("GET", path, cookie, None, &[]).await
    }

    /// Mutating request with CSRF header; `Value::Null` sends no body (for handlers that
    /// read none: an unread body makes the server reset the HTTP/1.0 connection).
    pub async fn call(&self, method: &str, path: &str, cookie: Option<&str>, body: Value) -> Resp {
        let body = (!body.is_null()).then_some(body);
        self.send(method, path, cookie, body, &[]).await
    }

    /// Signs in and returns the cookie pair; panics unless 200.
    pub async fn login(&self, email: &str, password: &str) -> String {
        let body = serde_json::json!({"email": email, "password": password});
        let resp = self.call("POST", "/api/auth/login", None, body).await;
        assert_eq!(resp.status, 200, "{:?}", resp.body);
        resp.session_cookie().unwrap()
    }
}

/// A parsed response.
#[derive(Debug)]
pub struct Resp {
    /// Status code.
    pub status: u16,
    /// Headers (lower-case names).
    pub headers: Vec<(String, String)>,
    /// JSON body (`Null` if empty or not JSON).
    pub body: Value,
}

impl Resp {
    /// Parses a full HTTP/1.x response.
    fn parse(raw: &[u8]) -> Self {
        let text = String::from_utf8_lossy(raw);
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .unwrap()
            .split(' ')
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let headers = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_owned()))
            .collect();
        let body = serde_json::from_str(body).unwrap_or(Value::Null);
        Self {
            status,
            headers,
            body,
        }
    }

    /// First header of `name`.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// `vm_session=<token>` from `Set-Cookie`, if a non-empty one was set.
    pub fn session_cookie(&self) -> Option<String> {
        let value = self.header("set-cookie")?;
        let pair = value.split(';').next()?.trim();
        (pair.len() > "vm_session=".len()).then(|| pair.to_owned())
    }
}

/// Creates an account.
#[allow(clippy::disallowed_methods)] // Test seed data, no request involved.
pub async fn seed_account(db: &Db, name: &str) -> Account {
    let mut conn = db.pool().acquire().await.unwrap();
    account::create(&mut conn, &AdminScope::system(), name)
        .await
        .unwrap()
}

/// Creates a user with [`PASSWORD`] (argon2id).
pub async fn seed_user(db: &Db, account_id: Option<&str>, email: &str, role: Role) -> User {
    let hash = password::hash(PASSWORD.to_owned()).await.unwrap();
    seed_user_with_hash(db, account_id, email, role, hash).await
}

/// Creates a user with a given stored hash (e.g. legacy bcrypt).
#[allow(clippy::disallowed_methods)] // Test seed data, no request involved.
pub async fn seed_user_with_hash(
    db: &Db,
    account_id: Option<&str>,
    email: &str,
    role: Role,
    hash: PasswordHash,
) -> User {
    let mut conn = db.pool().acquire().await.unwrap();
    let new = NewUser {
        email: email.to_owned(),
        name: "Test".to_owned(),
        role,
        password_hash: Some(hash),
    };
    user::create_for_admin(&mut conn, &AdminScope::system(), account_id, &new)
        .await
        .unwrap()
}

/// The stored hash of a user (for the rehash test).
pub async fn stored_hash(db: &Db, email: &str) -> String {
    let mut conn = db.pool().acquire().await.unwrap();
    let record = user::find_login(&mut conn, email).await.unwrap().unwrap();
    record.password_hash.unwrap().as_str().to_owned()
}

/// Number of audit entries with `action`.
pub async fn audit_count(db: &Db, action: &str) -> i64 {
    use sqlx::Row as _;
    let mut conn = db.pool().acquire().await.unwrap();
    sqlx::query("SELECT COUNT(*) AS n FROM audit_log WHERE action = ?")
        .bind(action)
        .fetch_one(&mut *conn)
        .await
        .unwrap()
        .get("n")
}
