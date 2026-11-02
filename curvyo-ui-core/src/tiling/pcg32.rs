//! The seeded generator of the tiling variance (`specs/0069-tiling` criterion 20): M. E. O'Neill's
//! PCG32, the `pcg32_oneseq` variant of pcg-c, written by hand with its jump-ahead.
//!
//! 64-bit LCG state, multiplier 6364136223846793005, the fixed increment 1442695040888963407,
//! XSH-RR output of the state before each step, seeded as `pcg_oneseq_64_srandom_r`. The outputs
//! are integers, identical on every platform; `docs/guides/pcg32.md` documents the algorithm.

const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
const INCREMENT: u64 = 1_442_695_040_888_963_407;

/// One PCG32 stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pcg32 {
    state: u64,
}

impl Pcg32 {
    /// The stream of `seed`: state 0, one step, add the seed, one step.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        let mut rng = Self { state: 0 };
        rng.step();
        rng.state = rng.state.wrapping_add(seed);
        rng.step();
        rng
    }

    const fn step(&mut self) {
        self.state = self.state.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
    }

    /// The next output.
    pub const fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.step();
        // invariant: the shifts leave 32 significant bits and 5 bits, the truncations are exact.
        #[allow(clippy::cast_possible_truncation)]
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        #[allow(clippy::cast_possible_truncation)]
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// The next output as a unit value in `[0, 1)`: `x / 2^32`, exact in `f64`.
    pub fn next_unit(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }

    /// Skips `delta` outputs in O(log delta) steps (pcg-c's `pcg_advance_lcg_64`).
    pub const fn advance(&mut self, mut delta: u64) {
        let mut acc_mult: u64 = 1;
        let mut acc_plus: u64 = 0;
        let mut cur_mult = MULTIPLIER;
        let mut cur_plus = INCREMENT;
        while delta > 0 {
            if delta & 1 == 1 {
                acc_mult = acc_mult.wrapping_mul(cur_mult);
                acc_plus = acc_plus.wrapping_mul(cur_mult).wrapping_add(cur_plus);
            }
            cur_plus = cur_mult.wrapping_add(1).wrapping_mul(cur_plus);
            cur_mult = cur_mult.wrapping_mul(cur_mult);
            delta /= 2;
        }
        self.state = acc_mult.wrapping_mul(self.state).wrapping_add(acc_plus);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../tests/fixtures/tiling/pcg32.json");

    #[test]
    fn the_first_16_outputs_match_the_reference_c_implementation() {
        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        let sequences = fixture["sequences"].as_array().unwrap();
        assert_eq!(sequences.len(), 4);
        for sequence in sequences {
            let mut rng = Pcg32::new(sequence["seed"].as_u64().unwrap());
            for expected in sequence["outputs"].as_array().unwrap() {
                assert_eq!(u64::from(rng.next_u32()), expected.as_u64().unwrap());
            }
        }
    }

    #[test]
    fn advance_matches_the_reference_jump_ahead() {
        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        for case in fixture["advance"].as_array().unwrap() {
            let mut rng = Pcg32::new(case["seed"].as_u64().unwrap());
            rng.advance(case["delta"].as_u64().unwrap());
            assert_eq!(u64::from(rng.next_u32()), case["next"].as_u64().unwrap());
        }
    }

    #[test]
    fn advance_n_equals_n_single_steps() {
        for n in [0_u64, 1, 2, 7, 8, 1000, 4097] {
            let mut jumped = Pcg32::new(42);
            jumped.advance(n);
            let mut stepped = Pcg32::new(42);
            for _ in 0..n {
                stepped.next_u32();
            }
            assert_eq!(jumped, stepped, "n = {n}");
        }
    }

    #[test]
    fn a_unit_value_is_exact_and_below_one() {
        let mut rng = Pcg32::new(0);
        let unit = rng.next_unit();
        assert!((0.0..1.0).contains(&unit));
        assert!((unit - 3_894_649_422.0 / 4_294_967_296.0).abs() < f64::EPSILON);
    }
}
