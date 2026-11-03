//! The per-session nonce source: a `ChaCha20` keystream over a caller-supplied seed.

use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher};
use zeroize::Zeroize;

use crate::CryptoError;

/// Yields 24-byte `XChaCha20` nonces as a pure function of a 32-byte seed and a counter
/// (ADR 0031 decision 6). The caller seeds it once per session with fresh entropy; the crate
/// itself draws none. Nonce *n* is the first 24 bytes of the `ChaCha20` keystream (RFC 8439,
/// block counter 0) under the seed as key and *n* (little-endian, padded to 12 bytes) as IV.
pub struct NonceStream {
    seed: [u8; 32],
    counter: u64,
}

impl Drop for NonceStream {
    fn drop(&mut self) {
        self.seed.zeroize();
    }
}

impl NonceStream {
    /// A stream starting at counter 0.
    #[must_use]
    pub fn new(seed: [u8; 32]) -> Self {
        Self { seed, counter: 0 }
    }

    /// The nonce at `counter` for `seed`.
    #[must_use]
    pub fn nonce_at(seed: &[u8; 32], counter: u64) -> [u8; 24] {
        let mut iv = [0; 12];
        iv[..8].copy_from_slice(&counter.to_le_bytes());
        let mut nonce = [0; 24];
        ChaCha20::new(&(*seed).into(), &iv.into()).apply_keystream(&mut nonce);
        nonce
    }

    /// The next nonce.
    ///
    /// # Errors
    /// `NonceExhausted` after 2^64 - 1 nonces; never in practice, but a counter never wraps.
    pub fn next_nonce(&mut self) -> Result<[u8; 24], CryptoError> {
        let nonce = Self::nonce_at(&self.seed, self.counter);
        self.counter = self
            .counter
            .checked_add(1)
            .ok_or(CryptoError::NonceExhausted)?;
        Ok(nonce)
    }
}
