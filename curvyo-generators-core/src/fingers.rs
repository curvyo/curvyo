//! The finger split of a joint edge and the corner rule every plate generator follows.
//!
//! **Corner rule (ADR 0082 §1):** every corner where joints meet has exactly one owner. The
//! owner's pieces next to the corner are solid and the mating plates have gaps there, so a plate's
//! material is never joined through a single point. A box edge is cut into an odd number of
//! pieces; the plate that owns the corner cubes (the bottom and the lid) keeps the even pieces, the
//! plates that meet it keep the odd ones.

/// The number of fingers on an edge of `length`: odd, as close to `length / width` as possible.
pub(crate) fn finger_count(length: f64, width: f64) -> u32 {
    let pairs = ((length / width - 1.0) / 2.0).round().max(0.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pairs = pairs as u32;
    2 * pairs + 1
}

/// Whether piece `index` of an edge is notched in a plate that keeps the even pieces when
/// `even_solid` is true and the odd ones otherwise.
pub(crate) fn is_notched(index: u32, even_solid: bool) -> bool {
    index.is_multiple_of(2) != even_solid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_count_is_odd_and_near_length_over_width() {
        assert_eq!(finger_count(400.0, 4.0), 101);
        assert_eq!(finger_count(6.0, 6.0), 1);
        for n in 1..=9 {
            assert_eq!(finger_count(f64::from(n) * 6.0, 6.0) % 2, 1);
        }
    }

    #[test]
    fn the_owner_and_its_mate_split_every_piece() {
        for index in 0..9 {
            assert_ne!(is_notched(index, true), is_notched(index, false));
        }
        assert!(!is_notched(0, true), "the owner keeps the first piece");
    }
}
