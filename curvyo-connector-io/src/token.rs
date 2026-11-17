//! The pairing token: 32 random bytes the app sends inside TLS on every connection.

use ring::digest;
use ring::rand::{SecureRandom, SystemRandom};
use zeroize::Zeroizing;

use crate::hex;

/// A bearer secret, wiped from memory on drop and never printed (ADR 0007 §4).
pub struct Token(Zeroizing<[u8; 32]>);

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(..)")
    }
}

impl Token {
    /// A new token from the system's random source; `None` when it has none.
    #[must_use]
    pub fn generate() -> Option<Self> {
        let mut bytes = Zeroizing::new([0; 32]);
        SystemRandom::new().fill(bytes.as_mut()).ok()?;
        Some(Self(bytes))
    }

    /// The wire form: 64 lower-case hex characters.
    #[must_use]
    pub fn to_hex(&self) -> Zeroizing<String> {
        Zeroizing::new(hex::encode(self.0.as_ref()))
    }

    /// Reads the wire form.
    #[must_use]
    pub fn from_hex(text: &str) -> Option<Self> {
        let bytes = Zeroizing::new(hex::decode(text)?);
        let array: [u8; 32] = bytes.as_slice().try_into().ok()?;
        Some(Self(Zeroizing::new(array)))
    }

    /// The SHA-256 of the token, which is all the connector stores.
    #[must_use]
    pub fn sha256_hex(&self) -> String {
        hex::encode(digest::digest(&digest::SHA256, self.0.as_ref()).as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_round_trips_hides_itself_and_hashes() {
        let token = Token::generate().unwrap();
        let text = token.to_hex();
        assert_eq!(text.len(), 64);
        let again = Token::from_hex(&text).unwrap();
        assert_eq!(again.sha256_hex(), token.sha256_hex());
        assert_eq!(format!("{token:?}"), "Token(..)");
        assert!(Token::from_hex("abcd").is_none());
        assert_ne!(Token::generate().unwrap().to_hex(), text);
    }
}
