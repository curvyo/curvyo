//! A deterministic byte source over caller-supplied entropy.

use curvyo_crypto_core::NonceStream;

/// Draws 32-byte secrets and jitter from one 32-byte seed the caller passes in: block *n* is
/// `NonceStream::nonce_at(seed, n)`, a `ChaCha20` keystream block, so the output is as
/// unpredictable as the seed. The crate itself draws no randomness (ADR 0031 decision 6).
pub(crate) struct Entropy {
    seed: [u8; 32],
    counter: u64,
}

impl Entropy {
    pub(crate) fn new(seed: [u8; 32]) -> Self {
        Self { seed, counter: 0 }
    }

    fn block(&mut self) -> [u8; 24] {
        let block = NonceStream::nonce_at(&self.seed, self.counter);
        self.counter = self.counter.wrapping_add(1);
        block
    }

    /// 32 fresh bytes (an epoch key, a wrap ephemeral, an invite seed).
    pub(crate) fn secret(&mut self) -> [u8; 32] {
        let first = self.block();
        let second = self.block();
        let mut out = [0; 32];
        out[..24].copy_from_slice(&first);
        out[24..].copy_from_slice(&second[..8]);
        out
    }

    /// A value in `0..=max`, for jitter.
    pub(crate) fn below_or_equal(&mut self, max: u64) -> u64 {
        let block = self.block();
        let mut word = [0; 8];
        word.copy_from_slice(&block[..8]);
        u64::from_le_bytes(word) % max.saturating_add(1).max(1)
    }
}
