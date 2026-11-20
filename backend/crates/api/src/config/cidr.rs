//! IP networks in CIDR notation (`TRUSTED_PROXIES`).

use std::net::IpAddr;

/// An IPv4 or IPv6 network, e.g. `10.0.0.0/8` or `fd00::/8`. A bare address counts as
/// a single host (`/32` or `/128`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    network: IpAddr,
    prefix: u8,
}

impl Cidr {
    /// Parses `addr[/prefix]`; `None` if malformed or the prefix exceeds the address
    /// length. An IPv4-mapped network (`::ffff:10.0.0.0/104`) is stored as the IPv4
    /// network (`10.0.0.0/8`); its prefix must cover the 96 mapping bits, so
    /// `::ffff:0.0.0.0/95` and shorter are rejected rather than widened.
    pub fn parse(text: &str) -> Option<Self> {
        let (addr, prefix) = match text.split_once('/') {
            Some((a, p)) => (a, Some(p)),
            None => (text, None),
        };
        let parsed: IpAddr = addr.trim().parse().ok()?;
        let max = width(parsed);
        let prefix = match prefix {
            Some(p) => p.trim().parse::<u8>().ok().filter(|p| *p <= max)?,
            None => max,
        };
        let network = canonical(parsed);
        // Mapped networks lose the 96 bits of the `::ffff:0:0/96` prefix.
        let prefix = prefix.checked_sub(max - width(network))?;
        Some(Self { network, prefix })
    }

    /// Whether `ip` lies in this network. IPv4-mapped IPv6 addresses (`::ffff:a.b.c.d`)
    /// are compared as IPv4.
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = canonical(ip);
        match (self.network, ip) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => same_prefix(
                u128::from(u32::from(net)),
                u128::from(u32::from(ip)),
                32,
                self.prefix,
            ),
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                same_prefix(u128::from(net), u128::from(ip), 128, self.prefix)
            }
            _ => false,
        }
    }
}

/// Address length in bits (32 or 128).
fn width(ip: IpAddr) -> u8 {
    if ip.is_ipv4() { 32 } else { 128 }
}

/// IPv4-mapped IPv6 address as IPv4, everything else unchanged.
fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(ip, IpAddr::V4),
        IpAddr::V4(_) => ip,
    }
}

/// Whether the first `prefix` of `width` bits of `a` and `b` are equal. A prefix longer
/// than `width` (excluded by [`Cidr::parse`]) never matches instead of overflowing.
fn same_prefix(a: u128, b: u128, width: u32, prefix: u8) -> bool {
    let Some(shift) = width.checked_sub(u32::from(prefix)) else {
        return false;
    };
    if shift >= 128 {
        return true;
    }
    (a >> shift) == (b >> shift)
}

/// Parses a comma-separated list; empty entries are ignored, any malformed one -> `None`.
pub fn parse_list(text: &str) -> Option<Vec<Cidr>> {
    text.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(Cidr::parse)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn ipv4_networks() {
        let net = Cidr::parse("10.1.0.0/16").unwrap();
        assert!(net.contains(ip("10.1.200.3")));
        assert!(!net.contains(ip("10.2.0.1")));
        assert!(net.contains(ip("::ffff:10.1.0.9")));
        assert!(Cidr::parse("0.0.0.0/0").unwrap().contains(ip("8.8.8.8")));
    }

    #[test]
    fn single_hosts_and_ipv6() {
        let host = Cidr::parse("127.0.0.1").unwrap();
        assert!(host.contains(ip("127.0.0.1")));
        assert!(!host.contains(ip("127.0.0.2")));
        let v6 = Cidr::parse("fd00::/8").unwrap();
        assert!(v6.contains(ip("fd12::1")));
        assert!(!v6.contains(ip("fe80::1")));
        assert!(!v6.contains(ip("10.0.0.1")));
    }

    #[test]
    fn rejects_malformed() {
        assert_eq!(Cidr::parse("10.0.0.0/33"), None);
        assert_eq!(Cidr::parse("host/8"), None);
        assert_eq!(parse_list("10.0.0.0/8, x"), None);
        assert_eq!(parse_list(" ").unwrap().len(), 0);
        assert_eq!(parse_list("10.0.0.0/8,,::1").unwrap().len(), 2);
        assert_eq!(Cidr::parse("::/129"), None);
        assert_eq!(Cidr::parse("10.0.0.0/255"), None);
    }

    #[test]
    fn ipv4_mapped_networks_become_ipv4() {
        let net = Cidr::parse("::ffff:10.0.0.0/104").unwrap();
        assert_eq!(net, Cidr::parse("10.0.0.0/8").unwrap());
        assert!(net.contains(ip("10.1.2.3")));
        assert!(net.contains(ip("::ffff:10.1.2.3")));
        assert!(!net.contains(ip("8.8.8.8")));
        assert!(!net.contains(ip("::ffff:8.8.8.8")));
        let host = Cidr::parse("::ffff:127.0.0.1").unwrap();
        assert!(host.contains(ip("127.0.0.1")));
        assert!(!host.contains(ip("127.0.0.2")));
        // Shorter than the mapping prefix would cover non-mapped addresses: rejected.
        assert_eq!(Cidr::parse("::ffff:10.0.0.0/8"), None);
        assert_eq!(Cidr::parse("::ffff:0.0.0.0/95"), None);
        assert!(
            Cidr::parse("::ffff:0.0.0.0/96")
                .unwrap()
                .contains(ip("8.8.8.8"))
        );
    }

    #[test]
    fn oversized_prefix_never_matches() {
        assert!(!same_prefix(0, 0, 32, 104));
        assert!(!same_prefix(0, 0, 128, 129));
    }
}
