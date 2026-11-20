//! Client address for rate limits. `X-Forwarded-For` is honoured only if the direct peer
//! is listed in `TRUSTED_PROXIES` (security baseline "Betrieb"); otherwise the header is
//! ignored, so clients cannot choose their own rate-limit key.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::HeaderMap;
use axum::http::request::Parts;

use crate::config::Cidr;
use crate::error::ApiError;
use crate::routes::AppState;

/// Address of the client of a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientIp(pub IpAddr);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = ApiError;

    /// Peer address from `ConnectInfo` (the server is started with
    /// `into_make_service_with_connect_info`), resolved through trusted proxies. Without
    /// connection info (never in the real server) all requests share one key.
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |c| c.0.ip());
        Ok(Self(resolve(
            peer,
            &parts.headers,
            &state.config.trusted_proxies,
        )))
    }
}

/// The client address: `peer`, or – if `peer` is a trusted proxy – the right-most
/// address of `X-Forwarded-For` that is not itself a trusted proxy. Unparsable entries
/// end the walk (the address before them is used).
pub fn resolve(peer: IpAddr, headers: &HeaderMap, trusted: &[Cidr]) -> IpAddr {
    let is_trusted = |ip: IpAddr| trusted.iter().any(|net| net.contains(ip));
    if !is_trusted(peer) {
        return peer;
    }
    let mut client = peer;
    let hops = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .collect::<Vec<_>>();
    for hop in hops.into_iter().rev() {
        let Ok(ip) = hop.parse::<IpAddr>() else { break };
        client = ip;
        if !is_trusted(ip) {
            break;
        }
    }
    client
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(xff: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", xff.parse().unwrap());
        h
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn untrusted_peer_ignores_header() {
        let got = resolve(ip("203.0.113.9"), &headers("1.2.3.4"), &[]);
        assert_eq!(got, ip("203.0.113.9"));
    }

    #[test]
    fn trusted_proxy_chain_yields_first_untrusted_hop() {
        let trusted = [Cidr::parse("10.0.0.0/8").unwrap()];
        let h = headers("6.6.6.6, 198.51.100.4, 10.0.0.2");
        assert_eq!(resolve(ip("10.0.0.1"), &h, &trusted), ip("198.51.100.4"));
    }

    #[test]
    fn garbage_in_header_stops_the_walk() {
        let trusted = [Cidr::parse("10.0.0.1").unwrap()];
        assert_eq!(
            resolve(ip("10.0.0.1"), &headers("unbekannt"), &trusted),
            ip("10.0.0.1")
        );
        assert_eq!(
            resolve(ip("10.0.0.1"), &HeaderMap::new(), &trusted),
            ip("10.0.0.1")
        );
    }
}
