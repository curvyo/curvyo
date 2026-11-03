//! The one error type of the crate.

/// Why a sealing, opening, wrapping or keyring operation refused its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CryptoError {
    /// The bytes are too short, too long or carry an unknown kind, flag or tag.
    #[error("the bytes are malformed")]
    Malformed,
    /// The bytes carry a format version this build does not read.
    #[error("unknown format version {0}")]
    UnknownVersion(u8),
    /// The envelope names another document than the one it is opened for.
    #[error("the envelope belongs to another document")]
    WrongDocument,
    /// The envelope is of another kind than the one expected.
    #[error("the envelope is of another kind")]
    WrongKind,
    /// The envelope's `key_id` is not the one of the key it is opened with.
    #[error("the envelope was sealed under another epoch key")]
    WrongKey,
    /// Authentication failed: a byte of the header, nonce or ciphertext was changed.
    #[error("the ciphertext failed authentication")]
    Unauthentic,
    /// A signature does not verify under the key it names.
    #[error("the signature is invalid")]
    BadSignature,
    /// A snapshot must be signed and is not, or a signer is missing.
    #[error("a snapshot must be signed")]
    Unsigned,
    /// The nonce stream's counter is used up.
    #[error("the nonce stream is exhausted")]
    NonceExhausted,
    /// A public key is not usable (a low-order point gives an all-zero shared secret).
    #[error("a public key is invalid")]
    BadPublicKey,
    /// A keyring log holds no valid genesis entry.
    #[error("the keyring has no valid genesis entry")]
    NoGenesis,
}
