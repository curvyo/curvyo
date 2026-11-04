//! The normal form of a relay server address: both devices of a pairing bind the same text into
//! the `CPace` channel identifier (ADR 0040 decision 4), and every connection uses it.

/// A server address as the maker typed it, normalised to `ws://host:port` or `wss://host:port`
/// (`specs/0092` UX notes §2.1): `host:port`, `ws://host:port` and `wss://host:port`, a trailing
/// `/` allowed, scheme and host folded to lower case. `None` for anything else ("Not a server
/// address.").
#[must_use]
pub fn normalize_server_address(text: &str) -> Option<String> {
    let text = text.trim();
    let lower = text.to_ascii_lowercase();
    let (scheme, rest) = if lower.starts_with("ws://") {
        ("ws", &text[5..])
    } else if lower.starts_with("wss://") {
        ("wss", &text[6..])
    } else if text.contains("://") {
        return None;
    } else {
        ("ws", text)
    };
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    let (host, port) = rest.rsplit_once(':')?;
    let port: u16 = port.parse().ok().filter(|p| *p > 0)?;
    let host_ok = !host.is_empty()
        && host.len() <= 253
        && (host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            || (host.starts_with('[') && host.ends_with(']') && host.len() > 2));
    host_ok.then(|| format!("{scheme}://{}:{port}", host.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_addresses_are_normalised() {
        assert_eq!(
            normalize_server_address("192.168.1.20:8787").as_deref(),
            Some("ws://192.168.1.20:8787")
        );
        assert_eq!(
            normalize_server_address(" wss://Sync.Example.org:443/ ").as_deref(),
            Some("wss://sync.example.org:443")
        );
        assert_eq!(
            normalize_server_address("WS://Relay.Local:8787/").as_deref(),
            Some("ws://relay.local:8787")
        );
        assert_eq!(
            normalize_server_address("ws://[::1]:8787").as_deref(),
            Some("ws://[::1]:8787")
        );
        for bad in ["", "host", "http://h:1", "h:0", "h:99999", "a b:1", ":80"] {
            assert_eq!(normalize_server_address(bad), None, "{bad}");
        }
    }
}
