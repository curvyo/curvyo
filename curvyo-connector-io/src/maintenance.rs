//! The bounds of maintenance mode (A4, A5; addendum D6): which peers count as local, and the
//! setup window measured on the monotonic clock the caller passes in.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};

/// How long `ApplyConfig` is taken after a start in maintenance mode (ADR 0071 decision 2).
pub const WINDOW: Duration = Duration::from_secs(30 * 60);

/// Whether `ip` is on the local network: loopback, RFC 1918, link-local, 100.64.0.0/10, IPv6
/// loopback, ULA and link-local; an IPv4-mapped IPv6 address is judged as IPv4.
#[must_use]
pub fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_local_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_local_v4(v4),
            None => is_local_v6(v6),
        },
    }
}

fn is_local_v4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    let shared = a == 100 && (b & 0b1100_0000) == 64;
    ip.is_loopback() || ip.is_private() || ip.is_link_local() || shared
}

fn is_local_v6(ip: Ipv6Addr) -> bool {
    ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local()
}

/// The setup window: open from `opened` for `length`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaintenanceWindow {
    /// When the connector started in maintenance mode.
    pub opened: Instant,
    /// How long it stays open.
    pub length: Duration,
}

impl MaintenanceWindow {
    /// Whether `ApplyConfig` is still taken at `now`.
    #[must_use]
    pub fn is_open(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.opened) < self.length
    }

    /// How long it stays open from `now`; zero once closed.
    #[must_use]
    pub fn closes_in(&self, now: Instant) -> Duration {
        self.length
            .saturating_sub(now.saturating_duration_since(self.opened))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_local_addresses() {
        let local = [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.20",
            "169.254.10.1",
            "100.64.0.1",
            "100.127.255.254",
            "::1",
            "fd12:3456::1",
            "fc00::1",
            "fe80::1",
            "::ffff:192.168.1.20",
            "::ffff:127.0.0.1",
        ];
        let public = [
            "8.8.8.8",
            "172.32.0.1",
            "100.63.255.255",
            "100.128.0.1",
            "192.169.0.1",
            "2001:db8::1",
            "2a00:1450::1",
            "64:ff9b::808:808",
            "2002:c0a8:101::1",
            "2001:0:4136:e378::1",
            "::ffff:8.8.8.8",
            "0.0.0.0",
            "::",
        ];
        for text in local {
            assert!(is_local(text.parse().unwrap()), "{text}");
        }
        for text in public {
            assert!(!is_local(text.parse().unwrap()), "{text}");
        }
    }

    #[test]
    fn a5_the_window_by_made_up_instants() {
        let opened = Instant::now();
        let window = MaintenanceWindow {
            opened,
            length: WINDOW,
        };
        let at = |s: u64| opened + Duration::from_secs(s);
        assert!(window.is_open(at(0)));
        assert!(window.is_open(at(1799)));
        assert!(!window.is_open(at(1800)));
        assert!(!window.is_open(at(100_000)));
        assert_eq!(window.closes_in(at(120)), Duration::from_secs(1680));
        assert_eq!(window.closes_in(at(1800)), Duration::ZERO);
        assert_eq!(window.closes_in(at(5000)), Duration::ZERO);
    }
}
