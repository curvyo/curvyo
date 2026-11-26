//! The part keys of generated objects as `'static` text (ADR 0082 §10): a part tag on a
//! [`crate::NodeId`] stays `Copy` and compares by its text.

use std::num::NonZeroU16;
use std::sync::Mutex;

/// The most distinct keys one process interns (a few per generator kind; the bound keeps a
/// hostile file from growing the table).
const MAX_KEYS: usize = 4096;
/// The longest key.
const MAX_LEN: usize = 32;

/// The interned keys; a key's tag is its position plus one.
static KEYS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// Whether `key` is a valid part key: 1 to 32 characters of `a-z`, `0-9` and `-`.
#[must_use]
pub(crate) fn is_valid(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_LEN
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The tag of `key`, the same for the same text; `None` for an invalid key or a full table.
pub(crate) fn intern(key: &str) -> Option<NonZeroU16> {
    if !is_valid(key) {
        return None;
    }
    let mut keys = KEYS.lock().ok()?;
    let at = if let Some(at) = keys.iter().position(|known| *known == key) {
        at
    } else {
        if keys.len() >= MAX_KEYS {
            return None;
        }
        keys.push(Box::leak(key.to_owned().into_boxed_str()));
        keys.len() - 1
    };
    NonZeroU16::new(u16::try_from(at + 1).ok()?)
}

/// The text of a tag [`intern`] gave.
pub(crate) fn text(tag: NonZeroU16) -> &'static str {
    KEYS.lock()
        .ok()
        .and_then(|keys| keys.get(usize::from(tag.get()) - 1).copied())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_interned_once_and_compares_by_text() {
        let a = intern("lid").unwrap();
        let b = intern(&String::from("lid")).unwrap();
        assert_eq!(a, b);
        assert_eq!(text(a), "lid");
        assert_eq!(text(intern("wall-12").unwrap()), "wall-12");
        assert_ne!(intern("wall-13"), intern("wall-12"));
    }

    #[test]
    fn an_invalid_key_is_refused() {
        for bad in ["", "Lid", "a b", "a/b", "a#b", &"x".repeat(33)] {
            assert_eq!(intern(bad), None, "{bad:?}");
        }
    }
}
