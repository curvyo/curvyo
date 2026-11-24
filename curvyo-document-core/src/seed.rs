//! The one stored seed type of the document and `SplitMix64`, the mixing function behind it
//! (`specs/0071-pattern-fill` criterion 17, `specs/0145-shape-spiro-handles/adrs.md` decision 8).

use serde::{Deserialize, Serialize};

use crate::path_model::NodeId;

/// A stored seed: of a seeded pattern generator (`0071` criterion 17) and of a randomized polygon or
/// star (`0145`); stored, so reopening does not reshuffle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Seed(u32);

impl Seed {
    /// A seed with this value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// The value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// The fresh seed of a new pattern on `node`: `SplitMix64` of its peer and counter, so the
    /// same shape always gets the same fresh seed and no clock or random source is read.
    #[must_use]
    pub fn fresh(node: NodeId) -> Self {
        #[allow(clippy::cast_sign_loss)] // the counter's bits, not its value
        let counter = u64::from(node.counter as u32);
        Self::low_bits(splitmix64(node.peer ^ splitmix64(counter)))
    }

    /// The seed Re-roll gives: `SplitMix64` of this one.
    #[must_use]
    pub fn next(self) -> Self {
        Self::low_bits(splitmix64(u64::from(self.0)))
    }

    #[allow(clippy::cast_possible_truncation)] // the low 32 bits are the seed
    const fn low_bits(value: u64) -> Self {
        Self(value as u32)
    }
}

/// One step of `SplitMix64` (Steele, Lea, Flood 2014) on `state`: the mixing function of the
/// seeds and of the generators' random stream. Its constants are part of the format.
#[must_use]
pub const fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_seed_is_stable_per_node_and_differs_between_nodes() {
        let a = NodeId::from_parts(7, 1);
        let b = NodeId::from_parts(7, 2);
        assert_eq!(Seed::fresh(a), Seed::fresh(a));
        assert_ne!(Seed::fresh(a), Seed::fresh(b));
        let seed = Seed::fresh(a);
        assert_ne!(seed.next(), seed);
        assert_eq!(seed.next(), seed.next());
    }

    #[test]
    fn splitmix_is_the_reference_algorithm() {
        // The first output of the reference SplitMix64 stream seeded with 0.
        assert_eq!(splitmix64(0), 0xE220_A839_7B1D_CDAF);
    }
}
