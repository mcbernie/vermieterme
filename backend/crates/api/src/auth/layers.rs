//! Global layers of the router: CSRF guard, security headers, `Cache-Control` for the
//! API, the time limit of every request (`time_limit`) and the request trace.
//!
//! CSRF (plan "Auth"): mutating requests (`POST/PUT/PATCH/DELETE`) under `/api/` except
//! `/api/tenant-app/**` need `X-CSRF-Guard: 1` and – if an `Origin` header is sent – an
//! origin equal to the own host. Together with `SameSite=Lax` cookies. Cross-site forms
//! cannot set custom headers, and cross-site `fetch` with one needs a CORS preflight the
//! server never grants.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::error::ApiError;
use crate::routes::{AppRouter, AppState};

/// CSRF header name (interface list).
pub const CSRF_HEADER: &str = "x-csrf-guard";

/// Content security policy: own origin only, no inline scripts, no framing. Inline
/// styles stay allowed (React style attributes). Set only if a handler did not set one.
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
     img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; object-src 'none'; \
     base-uri 'self'; form-action 'self'; frame-ancestors 'none'";

/// Adds the API fallbacks ([`crate::error::fallback`]) and wraps the router in the global
/// layers (outermost last). The time limit is innermost, so its 503 gets the headers too.
pub fn apply(app: AppRouter, state: &AppState) -> AppRouter {
    let app = crate::error::fallback::apply(app).layer(middleware::from_fn_with_state(
        state.clone(),
        time_limit::time_limit,
    ));
    let headers = [
        (header::CONTENT_SECURITY_POLICY, CSP),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::X_FRAME_OPTIONS, "DENY"),
        (header::REFERRER_POLICY, "no-referrer"),
        (
            HeaderName::from_static("cross-origin-opener-policy"),
            "same-origin",
        ),
    ];
    let mut app = app
        .layer(middleware::from_fn_with_state(state.clone(), csrf))
        .layer(middleware::from_fn(no_store_for_api));
    for (name, value) in headers {
        app = app.layer(SetResponseHeaderLayer::if_not_present(
            name,
            HeaderValue::from_static(value),
        ));
    }
    app.layer(
        TraceLayer::new_for_http()
            // Path only: query strings are not logged.
            .make_span_with(|req: &Request| {
                tracing::info_span!("request", method = %req.method(), path = %req.uri().path())
            })
            .on_response(DefaultOnResponse::new().level(Level::INFO)),
    )
}

/// Rejects mutating API requests without the CSRF header or from a foreign origin.
async fn csrf(State(state): State<AppState>, req: Request, next: Next) -> Response {
    if needs_guard(req.method(), req.uri().path()) {
        let peer = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip());
        let behind_proxy = peer.is_some_and(|ip| {
            state
                .config
                .trusted_proxies
                .iter()
                .any(|net| net.contains(ip))
        });
        if !guard_ok(req.headers(), behind_proxy) {
            return ApiError::csrf().into_response();
        }
    }
    next.run(req).await
}

/// Whether a request must carry the CSRF guard.
fn needs_guard(method: &Method, path: &str) -> bool {
    let mutating = matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    );
    let tenant_app = path == "/api/tenant-app" || path.starts_with("/api/tenant-app/");
    mutating && path.starts_with("/api/") && !tenant_app
}

/// Header `X-CSRF-Guard: 1` present and `Origin` absent or equal to the own host
/// (`Host`, or `X-Forwarded-Host` when the peer is a trusted proxy).
///
/// Only `Host` is read, not the HTTP/2 `:authority`: the server speaks HTTP/1.1 behind
/// the reverse proxy (no HTTP/2 listener), where hyper fills `Host`. Operation: a proxy
/// that rewrites `Host` must either keep it or send `X-Forwarded-Host` and be listed in
/// `TRUSTED_PROXIES`; otherwise every mutating request fails with 403 `csrf`.
fn guard_ok(headers: &HeaderMap, behind_proxy: bool) -> bool {
    if headers
        .get(CSRF_HEADER)
        .is_none_or(|v| v.as_bytes() != b"1")
    {
        return false;
    }
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    let Some(origin_host) = origin.to_str().ok().and_then(host_of_origin) else {
        return false;
    };
    let own = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    let mut candidates = vec![own("host")];
    if behind_proxy {
        candidates.push(own("x-forwarded-host").map(|v| v.split(',').next().unwrap_or(v).trim()));
    }
    candidates
        .into_iter()
        .flatten()
        .any(|host| host.eq_ignore_ascii_case(origin_host))
}

/// `host[:port]` of an origin like `https://example.org:8443`; `None` for `null` or
/// anything that is not `scheme://host`.
fn host_of_origin(origin: &str) -> Option<&str> {
    let (scheme, rest) = origin.split_once("://")?;
    let valid =
        matches!(scheme, "http" | "https") && !rest.is_empty() && !rest.contains(['/', '@']);
    valid.then_some(rest)
}

/// `Cache-Control: no-store` for API responses that did not set their own (session
/// documents and personal data must not be cached).
async fn no_store_for_api(req: Request, next: Next) -> Response {
    let api = req.uri().path().starts_with("/api/");
    let mut response = next.run(req).await;
    if api && !response.headers().contains_key(header::CACHE_CONTROL) {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}

mod time_limit;

#[cfg(test)]
mod tests;
