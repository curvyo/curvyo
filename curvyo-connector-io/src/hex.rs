//! Lower-case hex text of byte strings, the form of every key, hash and token on the wire.

const DIGITS: &[u8; 16] = b"0123456789abcdef";

/// `bytes` as lower-case hex.
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|b| [DIGITS[usize::from(b >> 4)], DIGITS[usize::from(b & 15)]])
        .map(char::from)
        .collect()
}

/// The bytes of a hex text (either case); `None` for an odd length or another character.
#[must_use]
pub fn decode(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    if !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_refuses_bad_text() {
        assert_eq!(encode(&[0, 15, 16, 255]), "000f10ff");
        assert_eq!(decode("000F10ff"), Some(vec![0, 15, 16, 255]));
        assert_eq!(decode("abc"), None);
        assert_eq!(decode("zz"), None);
        assert_eq!(decode("é1"), None);
        assert_eq!(decode("+f"), None);
        assert_eq!(decode("-1"), None);
    }
}
