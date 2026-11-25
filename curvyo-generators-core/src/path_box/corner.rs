//! A finger joint at a corner of any angle (ADR 0083 §5): how far the fingers and the notches
//! of a piece's end reach, for the piece that owns the corner and the one that mates with it.

use crate::fingers::{finger_count, is_notched};
use crate::wall::Step;

/// The zone and the fingers' reach never exceed this many wall thicknesses (a corner sharper than
/// 30 degrees, ADR 0083 §5).
const CAP: f64 = 2.0;

/// A corner as the joint needs it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Corner {
    /// The turn of the walk (radians, at least zero).
    pub(crate) turn: f64,
    /// Whether the walls' outer faces meet beyond the inner corner (the walk turns clockwise).
    pub(crate) convex: bool,
    /// How long the overlap zone is along a wall's inner face: `t / sin turn`, at most 2 t.
    pub(crate) z: f64,
    /// How far a finger of a convex corner reaches past the inner corner: the middle of the
    /// zone's slanted end, `t (tan(turn / 2) + cot(turn) / 2)`, at most 2 t.
    pub(crate) m: f64,
}

impl Corner {
    /// The corner of a walk that turns by `signed_turn` (positive clockwise on screen) between
    /// walls of thickness `t`.
    pub(crate) fn new(signed_turn: f64, t: f64) -> Self {
        let turn = signed_turn.abs();
        let z = (t / libm::sin(turn)).min(CAP * t);
        let m = (t * (libm::tan(turn / 2.0) + 0.5 / libm::tan(turn))).min(CAP * t);
        Self {
            turn,
            convex: signed_turn > 0.0,
            z,
            m,
        }
    }

    #[cfg(test)]
    /// The largest distance a finger tip stands from the mating wall's outer face along the
    /// strip: `t |cot turn| / 2` (ADR 0083 §5).
    pub(crate) fn tip_error(&self, t: f64) -> f64 {
        t * (0.5 / libm::tan(self.turn)).abs()
    }

    /// The solid margin between the corner and the first slit column.
    pub(crate) fn margin(&self, t: f64) -> f64 {
        self.z + t
    }
}

/// Which end of a piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum End {
    /// The end at strip coordinate 0.
    Start,
    /// The end at the strip's length.
    Finish,
}

/// The steps of one end edge, top to bottom. The owner keeps the even pieces of the edge (the
/// first and the last are solid); the mating piece keeps the odd ones.
pub(crate) fn steps(
    height: f64,
    t: f64,
    owner: bool,
    corner: &Corner,
    (end, length): (End, f64),
) -> Vec<Step> {
    let count = finger_count(height, 2.0 * t);
    let h = height / f64::from(count);
    (0..count)
        .map(|index| {
            let notch = is_notched(index, owner);
            let x = match (end, corner.convex, notch) {
                (End::Finish, true, false) => length + corner.m,
                (End::Finish, true, true) | (End::Finish, false, false) => length,
                (End::Finish, false, true) => length - corner.z,
                (End::Start, true, false) => -corner.m,
                (End::Start, true, true) | (End::Start, false, false) => 0.0,
                (End::Start, false, true) => corner.z,
            };
            Step {
                y0: h * f64::from(index),
                y1: h * f64::from(index + 1),
                x,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: f64 = 3.0;

    #[test]
    fn a_right_angle_has_a_zone_of_one_thickness_and_no_tip_error() {
        let corner = Corner::new(std::f64::consts::FRAC_PI_2, T);
        assert!((corner.z - T).abs() < 1e-9 && (corner.m - T).abs() < 1e-9);
        assert!(corner.tip_error(T) < 1e-9);
        assert!(
            (corner.margin(T) - 2.0 * T).abs() < 1e-9,
            "2 t at 90 degrees"
        );
    }

    #[test]
    fn the_tip_error_is_the_bound_of_the_spec_at_60_120_and_30_degrees() {
        for (degrees, factor) in [(60.0, 0.288_675), (120.0, 0.288_675), (30.0, 0.866_025)] {
            let corner = Corner::new(f64::to_radians(degrees), T);
            assert!(
                (corner.tip_error(T) - factor * T).abs() < 1e-4,
                "{degrees}: {}",
                corner.tip_error(T)
            );
        }
    }

    /// The finger tip against the mating wall's outer face along the strip, at the inner and the
    /// outer face: the tip lies in the middle of the slanted end, so it is off by the bound.
    #[test]
    fn a_finger_tip_stands_within_the_bound_of_the_slanted_end() {
        for degrees in [60.0, 90.0, 135.0] {
            let turn = f64::to_radians(degrees);
            let corner = Corner::new(turn, T);
            let inner = T / libm::sin(turn);
            let outer = T * libm::tan(turn / 2.0);
            // Unless a cap cut the reach, the tip is the middle of the two.
            assert!(
                (corner.m.min(2.0 * T) - f64::midpoint(inner, outer).min(2.0 * T)).abs() < 1e-9
            );
            for face in [inner, outer] {
                assert!(
                    (corner.m - face).abs() <= corner.tip_error(T) + 1e-9,
                    "{degrees}"
                );
            }
        }
    }

    #[test]
    fn owner_and_mate_fingers_alternate_with_the_same_centres() {
        for degrees in [60.0, 90.0, 135.0, -60.0, -90.0] {
            let corner = Corner::new(f64::to_radians(degrees), T);
            let (height, length) = (47.0, 30.0);
            let owner = steps(height, T, true, &corner, (End::Finish, length));
            let mate = steps(height, T, false, &corner, (End::Start, 0.0));
            assert_eq!(owner.len(), mate.len());
            assert_eq!(owner.len() % 2, 1, "an odd count");
            for (a, b) in owner.iter().zip(&mate) {
                assert!(
                    (a.y0 - b.y0).abs() < 1e-9 && (a.y1 - b.y1).abs() < 1e-9,
                    "same centres"
                );
                let a_finger = if corner.convex {
                    a.x > length
                } else {
                    a.x >= length
                };
                let b_finger = if corner.convex { b.x < 0.0 } else { b.x <= 0.0 };
                assert_ne!(a_finger, b_finger, "exactly one piece holds each height");
            }
            assert!(
                (owner[0].x - (length - corner.z)).abs() > 1e-9 || corner.convex,
                "the owner's first piece is solid"
            );
            assert!(owner[0].x >= length - 1e-9 && owner[owner.len() - 1].x >= length - 1e-9);
        }
    }

    #[test]
    fn a_corner_sharper_than_30_degrees_caps_the_zone_at_two_thicknesses() {
        let corner = Corner::new(f64::to_radians(165.0), T);
        assert!((corner.z - 2.0 * T).abs() < 1e-9 && corner.m <= 2.0 * T + 1e-9);
    }
}
