//! The key of one part of a generated object (ADR 0082 §10): 1 to 15 characters of `a-z`, `0-9`
//! and `-`, held inline so a [`crate::NodeId`] stays `Copy` with no table behind it. Order and
//! hash follow the text, so the order of memberships never depends on what was decoded before.

use std::cmp::Ordering;

/// The longest key (`wall-4294967295` is 15).
pub(crate) const MAX_LEN: usize = 15;

/// A valid part key.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartKey {
    len: u8,
    bytes: [u8; MAX_LEN],
}

/// Whether `key` is a valid part key: 1 to 15 characters of `a-z`, `0-9` and `-`.
#[must_use]
pub(crate) fn is_valid(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_LEN
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

impl PartKey {
    /// The key for `text`, `None` when it is not valid.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        if !is_valid(text) {
            return None;
        }
        let mut bytes = [0; MAX_LEN];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        Some(Self {
            len: u8::try_from(text.len()).ok()?,
            bytes,
        })
    }

    /// The key's text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // invariant: the bytes were copied from a valid (ASCII) `&str` and `len` bounds them.
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

impl PartialOrd for PartKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PartKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl std::fmt::Debug for PartKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl std::fmt::Display for PartKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_compares_and_hashes_by_its_text() {
        let a = PartKey::new("lid").unwrap();
        assert_eq!(a, PartKey::new(&String::from("lid")).unwrap());
        assert_eq!(a.as_str(), "lid");
        assert!(PartKey::new("base").unwrap() < PartKey::new("lid").unwrap());
        assert!(
            PartKey::new("wall-10").unwrap() < PartKey::new("wall-9").unwrap(),
            "text order"
        );
        assert_eq!(
            PartKey::new("wall-4294967295").unwrap().as_str(),
            "wall-4294967295"
        );
    }

    #[test]
    fn an_invalid_key_is_refused() {
        for bad in ["", "Lid", "a b", "a/b", "a#b", &"x".repeat(16)] {
            assert_eq!(PartKey::new(bad), None, "{bad:?}");
        }
    }
}
