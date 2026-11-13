//! Makes and checks 5-character test ids (ADR 0057 §4, `specs/0112` criterion 9).

use std::fmt;

/// The symbols of a test id, in the order a sampled byte maps to them.
const SYMBOLS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
/// The characters of a test id.
pub const TEST_ID_CHARS: usize = 5;
/// The first byte that is refused by the sampling: 252 = 7 x 36, so every symbol is equally
/// likely.
const SAMPLE_LIMIT: u8 = 252;

/// A test id: 5 characters of `0-9A-Z`, burned into the sheet next to its grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TestId([u8; TEST_ID_CHARS]);

impl TestId {
    /// The id `text`, `None` unless it is exactly 5 characters of `0-9A-Z` (lowercase refused).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let bytes: [u8; TEST_ID_CHARS] = text.as_bytes().try_into().ok()?;
        bytes
            .iter()
            .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase())
            .then_some(Self(bytes))
    }

    /// A new id from the caller's `random` bytes (core has no random source): each byte below
    /// 252 gives one symbol (`byte % 36`), others are skipped; an id for which `taken` is true is
    /// drawn again from the next bytes. `None` when the bytes run out first, and the caller
    /// retries with fresh ones (the session passes 64 bytes).
    #[must_use]
    pub fn make(random: &[u8], taken: &impl Fn(&Self) -> bool) -> Option<Self> {
        let mut symbols = random
            .iter()
            .filter(|&&b| b < SAMPLE_LIMIT)
            .map(|&b| SYMBOLS[usize::from(b % 36)]);
        loop {
            let mut id = [0; TEST_ID_CHARS];
            for slot in &mut id {
                *slot = symbols.next()?;
            }
            let id = Self(id);
            if !taken(&id) {
                return Some(id);
            }
        }
    }

    /// The id as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // invariant: built only from ASCII digits and capitals, always UTF-8.
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl fmt::Display for TestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FREE: fn(&TestId) -> bool = |_| false;

    #[test]
    fn parse_takes_five_digits_or_capitals_only() {
        assert_eq!(TestId::parse("K7Q2M").unwrap().as_str(), "K7Q2M");
        assert_eq!(TestId::parse("00000").unwrap().to_string(), "00000");
        for bad in ["k7q2m", "K7Q2", "K7Q2MX", "K7-2M", "K7Q2Ä", "", "K7Q 2"] {
            assert_eq!(TestId::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn every_byte_below_252_maps_each_symbol_seven_times() {
        let mut counts = [0_u32; 36];
        for byte in 0..SAMPLE_LIMIT {
            let id = TestId::make(&[byte; 5], &FREE).unwrap();
            let symbol = id.as_str().as_bytes()[0];
            let index = SYMBOLS.iter().position(|&s| s == symbol).unwrap();
            counts[index] += 1;
        }
        assert!(counts.iter().all(|&n| n == 7), "{counts:?}");
    }

    #[test]
    fn bytes_from_252_are_skipped() {
        let id = TestId::make(&[252, 255, 0, 1, 2, 3, 253, 35], &FREE).unwrap();
        assert_eq!(id.as_str(), "0123Z");
    }

    #[test]
    fn a_taken_id_is_drawn_again() {
        let taken = TestId::parse("00000").unwrap();
        let id = TestId::make(&[0, 0, 0, 0, 0, 1, 2, 3, 4, 5], &|id| *id == taken).unwrap();
        assert_eq!(id.as_str(), "12345");
    }

    #[test]
    fn running_out_of_bytes_gives_none() {
        assert_eq!(TestId::make(&[1, 2, 3, 4], &FREE), None);
        assert_eq!(TestId::make(&[255; 64], &FREE), None);
        assert_eq!(TestId::make(&[7; 64], &|_| true), None);
    }
}
