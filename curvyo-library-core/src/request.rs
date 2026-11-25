//! A request the host may send (ADR 0085 decision 2): one GET to an `https` address, with nothing
//! in it but the protocol's own path and query.

use crate::error::RequestError;

/// The ready Curvyo catalogue (Frage 334 A): GitHub raw files of `curvyo/templates`.
pub const READY_ADDRESS: &str = "https://raw.githubusercontent.com/curvyo/templates/main/";

/// The only host the ready entry may be read from; a redirect to another host is refused.
pub const READY_HOST: &str = "raw.githubusercontent.com";

/// What a source is (ADR 0085 decision 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// A Git catalogue read as files at a base address.
    Git,
    /// An Iconify-compatible API at a base address.
    Iconify,
}

/// One GET request to an `https` URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    url: String,
    pin_host: bool,
}

impl Request {
    /// The request for `url` below the source `base` (already normalised by [`base_address`]).
    pub(crate) fn below(base: &str, rest: &str) -> Self {
        Self {
            url: format!("{base}{rest}"),
            pin_host: is_ready(base),
        }
    }

    /// The URL to GET.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Whether a redirect to another host is refused (the ready entry, ADR 0085 decision 1).
    #[must_use]
    pub const fn pins_host(&self) -> bool {
        self.pin_host
    }
}

/// Whether `address` is the ready Curvyo catalogue.
#[must_use]
pub fn is_ready(address: &str) -> bool {
    address == READY_ADDRESS
}

/// The host of an `https` URL, lower case, without a port.
#[must_use]
pub fn host_of(url: &str) -> Option<String> {
    let rest = strip_https(url)?;
    let authority = rest.split('/').next()?;
    let host = authority.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

fn strip_https(url: &str) -> Option<&str> {
    let (scheme, rest) = url.split_at_checked(8)?;
    scheme.eq_ignore_ascii_case("https://").then_some(rest)
}

const MAX_ADDRESS_CHARS: usize = 500;

fn checked(text: &str) -> Result<String, RequestError> {
    let text = text.trim();
    let rest = strip_https(text).ok_or(RequestError::NotHttps)?;
    if text.chars().count() > MAX_ADDRESS_CHARS
        || rest
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '#')
    {
        return Err(RequestError::Shape);
    }
    let (authority, path) = rest.split_once('/').map_or((rest, ""), |(a, p)| (a, p));
    if authority.is_empty() {
        return Err(RequestError::NotHttps);
    }
    let host_ok = authority
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'));
    if !host_ok || authority.starts_with(['.', '-', ':']) || path.contains(['@', '\\']) {
        return Err(RequestError::Shape);
    }
    // The protocol's own query is added by the builders; an address carries none.
    if path.contains('?') {
        return Err(RequestError::Shape);
    }
    Ok(format!("https://{}/{path}", authority.to_ascii_lowercase()))
}

/// The normalised base address of a source (ADR 0085 decision 1): `https`, lower-case host, no
/// user name, query or fragment; a Git address ends in `/`, an Iconify one does not.
///
/// # Errors
/// [`RequestError`] when the text is not such an address.
pub fn base_address(kind: SourceKind, text: &str) -> Result<String, RequestError> {
    let full = checked(text)?;
    Ok(match kind {
        SourceKind::Git if full.ends_with('/') => full,
        SourceKind::Git => format!("{full}/"),
        SourceKind::Iconify => full.trim_end_matches('/').to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_base_address_is_normalised_by_kind() {
        assert_eq!(
            base_address(SourceKind::Git, " HTTPS://Example.ORG/cat ").unwrap(),
            "https://example.org/cat/"
        );
        assert_eq!(
            base_address(SourceKind::Iconify, "https://api.iconify.design/").unwrap(),
            "https://api.iconify.design"
        );
        assert_eq!(
            base_address(SourceKind::Git, "https://example.org").unwrap(),
            "https://example.org/"
        );
    }

    #[test]
    fn only_a_plain_https_address_is_taken() {
        for bad in [
            "http://example.org/",
            "example.org",
            "https://",
            "https://user@example.org/",
            "https://example.org/?x=1",
            "https://example.org/#top",
            "https://exa mple.org/",
            "ftp://example.org/",
        ] {
            assert!(base_address(SourceKind::Git, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_ready_entry_pins_its_host() {
        assert!(Request::below(READY_ADDRESS, "curvyo-catalogue.json").pins_host());
        assert!(!Request::below("https://example.org/", "x").pins_host());
        assert_eq!(host_of(READY_ADDRESS).as_deref(), Some(READY_HOST));
        assert_eq!(
            host_of("https://Example.org:8443/a").as_deref(),
            Some("example.org")
        );
    }
}
