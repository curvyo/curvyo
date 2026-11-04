//! The one-time pairing code `XXXX-XXXX` (ADR 0040 decision 4): a relay slot and a secret
//! that never reaches the relay, in Crockford base 32.

use core::fmt;

use crate::CryptoError;

/// Crockford's base-32 alphabet: digits and capitals without I, L, O and U.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// The characters of the slot part (15 bits).
const SLOT_CHARS: usize = 3;
/// The characters of the secret part (25 bits).
const SECRET_CHARS: usize = 5;
/// The number of relay slots, `2^15`.
pub const PAIR_SLOTS: u16 = 1 << 15;
/// The number of possible secrets, `2^25`.
const SECRET_COUNT: u32 = 1 << 25;

/// The 25-bit secret part of a pairing code: the `CPace` password. Never sent to the relay;
/// never printed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PairSecret(u32);

impl PairSecret {
    /// The secret from 4 bytes of passed-in entropy (the low 25 bits; uniform because `2^25`
    /// divides `2^32`).
    #[must_use]
    pub fn from_entropy(entropy: [u8; 4]) -> Self {
        Self(u32::from_le_bytes(entropy) % SECRET_COUNT)
    }

    /// The `CPace` password bytes: the 25-bit value, 4 bytes big-endian.
    #[must_use]
    pub fn password(&self) -> [u8; 4] {
        self.0.to_be_bytes()
    }
}

impl fmt::Debug for PairSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PairSecret(..)")
    }
}

/// A pairing code: the relay slot (below [`PAIR_SLOTS`]) and the secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairCode {
    slot: u16,
    secret: PairSecret,
}

impl PairCode {
    /// A code from the relay's slot and the opener's secret.
    ///
    /// # Errors
    /// `Malformed` when the slot is not below [`PAIR_SLOTS`].
    pub fn new(slot: u16, secret: PairSecret) -> Result<Self, CryptoError> {
        if slot >= PAIR_SLOTS {
            return Err(CryptoError::Malformed);
        }
        Ok(Self { slot, secret })
    }

    /// Reads a typed code: case is folded, spaces and dashes are ignored, I and L read as 1 and
    /// O as 0.
    ///
    /// # Errors
    /// `Malformed` when it does not hold exactly 8 characters of the alphabet (U and other
    /// characters are refused).
    pub fn parse(text: &str) -> Result<Self, CryptoError> {
        let mut values = Vec::with_capacity(SLOT_CHARS + SECRET_CHARS);
        for character in text.chars() {
            if character == '-' || character.is_whitespace() {
                continue;
            }
            values.push(symbol_value(character).ok_or(CryptoError::Malformed)?);
            if values.len() > SLOT_CHARS + SECRET_CHARS {
                return Err(CryptoError::Malformed);
            }
        }
        if values.len() != SLOT_CHARS + SECRET_CHARS {
            return Err(CryptoError::Malformed);
        }
        let (slot, secret) = values.split_at(SLOT_CHARS);
        let slot = u16::try_from(number(slot)).map_err(|_| CryptoError::Malformed)?;
        Self::new(slot, PairSecret(number(secret)))
    }

    /// The relay slot.
    #[must_use]
    pub fn slot(&self) -> u16 {
        self.slot
    }

    /// The secret part.
    #[must_use]
    pub fn secret(&self) -> PairSecret {
        self.secret
    }
}

impl fmt::Display for PairCode {
    /// The code as shown: two groups of four, `XXXX-XXXX`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut shown = symbols(u32::from(self.slot), SLOT_CHARS);
        shown.extend(symbols(self.secret.0, SECRET_CHARS));
        let (first, second) = shown.split_at(4);
        let first: String = first.iter().collect();
        let second: String = second.iter().collect();
        write!(f, "{first}-{second}")
    }
}

/// The `count` base-32 symbols of `value`, most significant first.
fn symbols(value: u32, count: usize) -> Vec<char> {
    (0..count)
        .rev()
        .map(|place| char::from(ALPHABET[((value >> (5 * place)) & 31) as usize]))
        .collect()
}

/// The value of base-32 symbols, most significant first.
fn number(values: &[u8]) -> u32 {
    values
        .iter()
        .fold(0, |acc, &value| (acc << 5) | u32::from(value))
}

/// The value of one typed character, with Crockford's folding.
fn symbol_value(character: char) -> Option<u8> {
    let folded = match character.to_ascii_uppercase() {
        'I' | 'L' => '1',
        'O' => '0',
        other => other,
    };
    let byte = u8::try_from(folded).ok()?;
    ALPHABET
        .iter()
        .position(|&symbol| symbol == byte)
        .and_then(|index| u8::try_from(index).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_is_shown_in_two_groups_of_four() {
        let code = PairCode::new(0, PairSecret(0)).unwrap();
        assert_eq!(code.to_string(), "0000-0000");
        let code = PairCode::new(PAIR_SLOTS - 1, PairSecret(SECRET_COUNT - 1)).unwrap();
        assert_eq!(code.to_string(), "ZZZZ-ZZZZ");
        let code = PairCode::new(1, PairSecret(32)).unwrap();
        assert_eq!(code.to_string(), "0010-0010");
    }

    #[test]
    fn typed_codes_fold_case_separators_and_look_alikes() {
        let code = PairCode::parse("K7Q2-9M4X").unwrap();
        assert_eq!(PairCode::parse("k7q29m4x").unwrap(), code);
        assert_eq!(PairCode::parse(" k7q2 9m4x ").unwrap(), code);
        assert_eq!(
            PairCode::parse("ilo0-0000").unwrap(),
            PairCode::parse("1100-0000").unwrap()
        );
        assert_eq!(code.to_string(), "K7Q2-9M4X");
    }

    #[test]
    fn malformed_codes_are_refused() {
        for text in [
            "",
            "K7Q2-9M4",
            "K7Q2-9M4XX",
            "K7Q2-9M4U",
            "K7Q2-9M4!",
            "K7Q2-9M4Ä",
        ] {
            assert_eq!(PairCode::parse(text), Err(CryptoError::Malformed), "{text}");
        }
        assert_eq!(
            PairCode::new(PAIR_SLOTS, PairSecret(0)),
            Err(CryptoError::Malformed)
        );
    }

    #[test]
    fn the_secret_takes_the_low_25_bits_and_is_not_printed() {
        let secret = PairSecret::from_entropy([0xff; 4]);
        assert_eq!(secret.password(), (SECRET_COUNT - 1).to_be_bytes());
        assert_eq!(format!("{secret:?}"), "PairSecret(..)");
        let code = PairCode::new(5, secret).unwrap();
        assert!(!format!("{code:?}").contains("33554431"));
    }
}
