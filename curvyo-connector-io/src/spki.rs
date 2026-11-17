//! The SHA-256 of a public key (SPKI), the thing the app pins, and its printed fingerprint.

use ring::digest;

use crate::hex;

/// SHA-256 of the DER `SubjectPublicKeyInfo` of a connector's key.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SpkiHash([u8; 32]);

impl SpkiHash {
    /// Hashes the DER bytes of a `SubjectPublicKeyInfo`.
    #[must_use]
    pub fn of_spki_der(der: &[u8]) -> Self {
        let digest = digest::digest(&digest::SHA256, der);
        let mut hash = [0; 32];
        hash.copy_from_slice(digest.as_ref());
        Self(hash)
    }

    /// The 32 raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// 64 lower-case hex characters, the stored form.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    /// Reads the stored form.
    #[must_use]
    pub fn from_hex(text: &str) -> Option<Self> {
        let bytes: [u8; 32] = hex::decode(text)?.try_into().ok()?;
        Some(Self(bytes))
    }

    /// The printed form (Q1): the first 16 bytes as 8 groups of 4 upper-case hex characters.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        let text = hex::encode(&self.0[..16]).to_ascii_uppercase();
        text.as_bytes()
            .chunks(4)
            .map(|group| String::from_utf8_lossy(group).into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fingerprint_is_eight_groups_of_four_and_the_hash_round_trips() {
        let hash = SpkiHash::of_spki_der(b"abc");
        // SHA-256("abc") = ba7816bf 8f01cfea 414140de 5dae2223 ...
        assert_eq!(
            hash.fingerprint(),
            "BA78 16BF 8F01 CFEA 4141 40DE 5DAE 2223"
        );
        assert_eq!(SpkiHash::from_hex(&hash.to_hex()), Some(hash));
        assert_eq!(hash.to_hex().len(), 64);
        assert_eq!(SpkiHash::from_hex("00"), None);
    }
}
