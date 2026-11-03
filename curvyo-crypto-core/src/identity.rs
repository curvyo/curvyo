//! A device's long-lived identity keys (ADR 0010 §1), their public half and its fingerprint.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::{CryptoError, agreement};

/// A participant's ed25519 public key: the id under which the keyring names a member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemberId(pub [u8; 32]);

/// A participant's public identity: the ed25519 key it signs with and the X25519 key wraps
/// are sealed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity {
    /// The signing key, also the member's id.
    pub member: MemberId,
    /// The X25519 agreement key, the target of key wraps.
    pub agreement: [u8; 32],
}

impl Identity {
    /// The 64-byte encoding: signing key ‖ agreement key.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0; 64];
        bytes[..32].copy_from_slice(&self.member.0);
        bytes[32..].copy_from_slice(&self.agreement);
        bytes
    }

    /// Reads the 64-byte encoding.
    #[must_use]
    pub fn from_bytes(bytes: &[u8; 64]) -> Self {
        let mut member = [0; 32];
        let mut agreement = [0; 32];
        member.copy_from_slice(&bytes[..32]);
        agreement.copy_from_slice(&bytes[32..]);
        Self {
            member: MemberId(member),
            agreement,
        }
    }

    /// The human-readable fingerprint: the first 10 bytes of SHA-256 over both public keys,
    /// in RFC 4648 base32 (`A`-`Z`, `2`-`7`), four groups of four separated by spaces.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        let digest = Sha256::digest(self.to_bytes());
        let mut text = String::with_capacity(19);
        for (index, symbol) in base32(&digest[..10]).chars().enumerate() {
            if index > 0 && index % 4 == 0 {
                text.push(' ');
            }
            text.push(symbol);
        }
        text
    }
}

/// RFC 4648 base32 of a byte string whose bit length is a multiple of 5 bits times 8.
fn base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut text = String::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            text.push(char::from(ALPHABET[((buffer >> bits) & 31) as usize]));
        }
    }
    if bits > 0 {
        text.push(char::from(ALPHABET[((buffer << (5 - bits)) & 31) as usize]));
    }
    text
}

/// A device's secret identity keys. Made from 64 bytes of caller-supplied entropy; wiped on
/// drop.
#[derive(Clone)]
pub struct IdentityKeys {
    /// Wipes itself on drop (`ed25519-dalek`'s `zeroize` feature).
    signing: SigningKey,
    agreement_secret: [u8; 32],
}

impl Drop for IdentityKeys {
    fn drop(&mut self) {
        self.agreement_secret.zeroize();
    }
}

impl IdentityKeys {
    /// The first 32 bytes are the ed25519 seed (RFC 8032 secret key), the last 32 the X25519
    /// secret. The same bytes always give the same keys, so they are what the credential store
    /// keeps.
    #[must_use]
    pub fn from_entropy(entropy: &[u8; 64]) -> Self {
        let mut seed = Zeroizing::new([0; 32]);
        seed.copy_from_slice(&entropy[..32]);
        let mut agreement_secret = [0; 32];
        agreement_secret.copy_from_slice(&entropy[32..]);
        Self {
            signing: SigningKey::from_bytes(&seed),
            agreement_secret,
        }
    }

    /// The 64 bytes `from_entropy` takes back.
    #[must_use]
    pub fn to_entropy(&self) -> Zeroizing<[u8; 64]> {
        let mut bytes = Zeroizing::new([0; 64]);
        bytes[..32].copy_from_slice(&self.signing.to_bytes());
        bytes[32..].copy_from_slice(&self.agreement_secret);
        bytes
    }

    /// The public identity.
    #[must_use]
    pub fn identity(&self) -> Identity {
        Identity {
            member: MemberId(self.signing.verifying_key().to_bytes()),
            agreement: agreement::public_key(&self.agreement_secret),
        }
    }

    /// An ed25519 signature over `message`.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing.sign(message).to_bytes()
    }

    pub(crate) fn agreement_secret(&self) -> &[u8; 32] {
        &self.agreement_secret
    }
}

/// Verifies an ed25519 signature strictly (no malleable or small-order keys).
///
/// # Errors
/// `BadSignature` when the key is not a valid point or the signature does not verify.
pub fn verify(signer: &MemberId, message: &[u8], signature: &[u8; 64]) -> Result<(), CryptoError> {
    let key = VerifyingKey::from_bytes(&signer.0).map_err(|_| CryptoError::BadSignature)?;
    key.verify_strict(message, &Signature::from_bytes(signature))
        .map_err(|_| CryptoError::BadSignature)
}

pub(crate) fn signing_key_from_seed(seed: &[u8; 32]) -> SigningKey {
    SigningKey::from_bytes(seed)
}
