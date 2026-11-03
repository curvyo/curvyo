//! X25519 key agreement (RFC 7748) on `curve25519-dalek`'s Montgomery ladder.

use curve25519_dalek::montgomery::MontgomeryPoint;

use crate::CryptoError;

/// The X25519 public key of a 32-byte secret (clamped as RFC 7748 §5 says).
pub(crate) fn public_key(secret: &[u8; 32]) -> [u8; 32] {
    MontgomeryPoint::mul_base_clamped(*secret).to_bytes()
}

/// The X25519 shared secret of `secret` and a peer's public key; a low-order peer key (an
/// all-zero result, RFC 7748 §6.1) is refused.
pub(crate) fn shared_secret(secret: &[u8; 32], public: &[u8; 32]) -> Result<[u8; 32], CryptoError> {
    let shared = MontgomeryPoint(*public).mul_clamped(*secret).to_bytes();
    if shared == [0; 32] {
        return Err(CryptoError::BadPublicKey);
    }
    Ok(shared)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex32(text: &str) -> [u8; 32] {
        let bytes: Vec<u8> = (0..64)
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect();
        bytes.try_into().unwrap()
    }

    /// RFC 7748 §6.1.
    #[test]
    fn x25519_matches_rfc_7748() {
        let alice = hex32("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        let bob = hex32("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
        let alice_public = public_key(&alice);
        let bob_public = public_key(&bob);
        assert_eq!(
            alice_public,
            hex32("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a")
        );
        assert_eq!(
            bob_public,
            hex32("de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f")
        );
        let shared = hex32("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742");
        assert_eq!(shared_secret(&alice, &bob_public).unwrap(), shared);
        assert_eq!(shared_secret(&bob, &alice_public).unwrap(), shared);
    }

    /// RFC 7748 §5.2, first test vector of the scalar multiplication.
    #[test]
    fn x25519_matches_the_rfc_7748_function_vector() {
        let scalar = hex32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
        let u = hex32("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c");
        assert_eq!(
            shared_secret(&scalar, &u).unwrap(),
            hex32("c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552")
        );
    }

    #[test]
    fn a_low_order_public_key_is_refused() {
        assert_eq!(
            shared_secret(&[7; 32], &[0; 32]),
            Err(CryptoError::BadPublicKey)
        );
    }
}
