//! XChaCha20-Poly1305 over byte slices (ADR 0008 §2), the one cipher of every seal and wrap.

use chacha20poly1305::aead::{AeadInOut, KeyInit};
use chacha20poly1305::{Key, Tag, XChaCha20Poly1305, XNonce};

use crate::CryptoError;

/// The length of the authentication tag appended to every ciphertext.
pub(crate) const TAG_LEN: usize = 16;

/// Encrypts `plaintext` under `key` and `nonce`, binding `associated_data`; returns
/// ciphertext ‖ tag.
pub(crate) fn seal(
    key: &[u8; 32],
    nonce: &[u8; 24],
    associated_data: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = XChaCha20Poly1305::new(&Key::from(*key));
    let mut buffer = Vec::with_capacity(plaintext.len() + TAG_LEN);
    buffer.extend_from_slice(plaintext);
    let tag = cipher
        .encrypt_inout_detached(
            &XNonce::from(*nonce),
            associated_data,
            buffer.as_mut_slice().into(),
        )
        .map_err(|_| CryptoError::Malformed)?;
    buffer.extend_from_slice(&tag);
    Ok(buffer)
}

/// Decrypts ciphertext ‖ tag; any change to key, nonce, associated data or bytes is
/// `Unauthentic`.
pub(crate) fn open(
    key: &[u8; 32],
    nonce: &[u8; 24],
    associated_data: &[u8],
    sealed: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let body_len = sealed
        .len()
        .checked_sub(TAG_LEN)
        .ok_or(CryptoError::Unauthentic)?;
    let (body, tag) = sealed.split_at(body_len);
    let tag = Tag::try_from(tag).map_err(|_| CryptoError::Unauthentic)?;
    let mut buffer = body.to_vec();
    XChaCha20Poly1305::new(&Key::from(*key))
        .decrypt_inout_detached(
            &XNonce::from(*nonce),
            associated_data,
            buffer.as_mut_slice().into(),
            &tag,
        )
        .map_err(|_| CryptoError::Unauthentic)?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }

    /// draft-irtf-cfrg-xchacha-03, appendix A.3.1.
    #[test]
    fn xchacha20_poly1305_matches_the_draft_vector() {
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
        let aad = hex("50515253c0c1c2c3c4c5c6c7");
        let key: [u8; 32] = hex("808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f")
            .try_into()
            .unwrap();
        let nonce: [u8; 24] = hex("404142434445464748494a4b4c4d4e4f5051525354555657")
            .try_into()
            .unwrap();
        let expected = hex(concat!(
            "bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb",
            "731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b452",
            "2f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff9",
            "21f9664c97637da9768812f615c68b13b52e",
            "c0875924c1c7987947deafd8780acf49",
        ));
        let sealed = seal(&key, &nonce, &aad, plaintext).unwrap();
        assert_eq!(sealed, expected);
        assert_eq!(open(&key, &nonce, &aad, &sealed).unwrap(), plaintext);
        assert_eq!(
            open(&key, &nonce, b"other", &sealed),
            Err(CryptoError::Unauthentic)
        );
    }
}
