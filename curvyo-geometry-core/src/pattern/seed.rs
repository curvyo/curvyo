//! The seeded random stream of the seeded generators (ADR 0029 §5): `SplitMix64` over the pattern's
//! seed, the same numbers on every platform and in wasm. Its algorithm is part of the format.

use curvyo_document_core::{PatternSeed, splitmix64};

/// The golden ratio increment of `SplitMix64`.
const INCREMENT: u64 = 0x9E37_79B9_7F4A_7C15;

/// A stream of numbers drawn from one seed.
pub(crate) struct Stream {
    state: u64,
}

impl Stream {
    /// The stream of `seed`.
    pub(crate) fn new(seed: PatternSeed) -> Self {
        Self {
            state: u64::from(seed.get()),
        }
    }

    /// The next 64 bits.
    pub(crate) fn next_u64(&mut self) -> u64 {
        let out = splitmix64(self.state);
        self.state = self.state.wrapping_add(INCREMENT);
        out
    }

    /// The next number in `[0, 1)`, from the top 53 bits.
    pub(crate) fn next_unit(&mut self) -> f64 {
        #[allow(clippy::cast_precision_loss)] // 53 bits are exact in an f64
        let top = (self.next_u64() >> 11) as f64;
        top * (1.0 / 9_007_199_254_740_992.0)
    }

    /// The next number in `[-1, 1)`.
    pub(crate) fn next_signed(&mut self) -> f64 {
        2.0 * self.next_unit() - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stream_is_fixed_by_its_seed() {
        let mut a = Stream::new(PatternSeed::new(5));
        let mut b = Stream::new(PatternSeed::new(5));
        let mut c = Stream::new(PatternSeed::new(6));
        let xs: Vec<u64> = (0..4).map(|_| a.next_u64()).collect();
        let ys: Vec<u64> = (0..4).map(|_| b.next_u64()).collect();
        assert_eq!(xs, ys);
        assert_ne!(xs[0], c.next_u64());
        for _ in 0..100 {
            let u = a.next_unit();
            assert!((0.0..1.0).contains(&u));
        }
    }
}
