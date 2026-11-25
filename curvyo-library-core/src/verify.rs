//! The check of a downloaded file against its catalogue entry, before anything reads it (ADR 0085
//! decision 4, `specs/0138` criterion 8).

use sha2::{Digest, Sha256};

use crate::error::VerifyError;

/// The SHA-256 of `bytes` as lower-case hex.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
}

/// Checks `bytes` against the entry's `size` (when it names one) and lower-case hex `sha256`.
///
/// # Errors
/// [`VerifyError::Size`] when the length differs, [`VerifyError::Hash`] when the digest does.
pub fn verify(bytes: &[u8], size: Option<u64>, sha256: &str) -> Result<(), VerifyError> {
    if size.is_some_and(|size| size != bytes.len() as u64) {
        return Err(VerifyError::Size);
    }
    if sha256_hex(bytes) == sha256 {
        Ok(())
    } else {
        Err(VerifyError::Hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_digest_is_the_known_one() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn a_changed_file_or_size_is_refused() {
        let hash = sha256_hex(b"abc");
        assert_eq!(verify(b"abc", Some(3), &hash), Ok(()));
        assert_eq!(verify(b"abc", None, &hash), Ok(()));
        assert_eq!(verify(b"abd", Some(3), &hash), Err(VerifyError::Hash));
        assert_eq!(verify(b"abcd", Some(3), &hash), Err(VerifyError::Size));
        assert_eq!(
            verify(b"abc", Some(3), &hash.to_uppercase()),
            Err(VerifyError::Hash)
        );
    }
}
