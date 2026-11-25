//! The jog steps, their feeds and the travel clamp (`specs/0118` criteria 16, 17, 18).

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::millimetres::Millimetres;

/// How long a continuous jog lives without a keep-alive (criterion 18, Frage 309 A).
pub const JOG_DEAD_MAN: Duration = Duration::from_millis(300);

/// The distance below which a jog counts as at the edge of the travel, in millimetres.
pub const EDGE: f64 = 0.01;

/// A machine axis the jog pad drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JogAxis {
    /// The X axis.
    X,
    /// The Y axis.
    Y,
}

impl JogAxis {
    /// The axis' place in a position `[x, y, z]`.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
        }
    }

    /// The axis word of G-code.
    #[must_use]
    pub fn letter(self) -> char {
        match self {
            Self::X => 'X',
            Self::Y => 'Y',
        }
    }
}

/// The way along an axis, in machine coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JogDirection {
    /// Towards larger machine coordinates.
    Plus,
    /// Towards smaller machine coordinates.
    Minus,
}

impl JogDirection {
    /// `1.0` or `-1.0`.
    #[must_use]
    pub fn sign(self) -> f64 {
        match self {
            Self::Plus => 1.0,
            Self::Minus => -1.0,
        }
    }
}

/// The distance of one jog press: a closed set, never a typed number (Frage 315 A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JogStep {
    /// 0.1 mm.
    #[serde(rename = "0.1")]
    Tenth,
    /// 1 mm.
    #[serde(rename = "1")]
    One,
    /// 10 mm.
    #[serde(rename = "10")]
    Ten,
    /// 50 mm.
    #[serde(rename = "50")]
    Fifty,
}

/// A feed rate in millimetres per minute.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Feed(pub f64);

impl JogStep {
    /// The distance.
    #[must_use]
    pub fn distance(self) -> Millimetres {
        Millimetres(match self {
            Self::Tenth => 0.1,
            Self::One => 1.0,
            Self::Ten => 10.0,
            Self::Fifty => 50.0,
        })
    }

    /// The step's jog feed: 3,000 mm/min for 10 and 50 mm, 1,000 for 1 mm, 300 for 0.1 mm
    /// (criterion 16); a controller caps it at its own maximum rate.
    #[must_use]
    pub fn feed(self) -> Feed {
        Feed(match self {
            Self::Tenth => 300.0,
            Self::One => 1_000.0,
            Self::Ten | Self::Fifty => 3_000.0,
        })
    }
}

/// The travel of X and Y in machine coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TravelBox {
    /// The smallest X and Y.
    pub min: [f64; 2],
    /// The largest X and Y.
    pub max: [f64; 2],
}

impl TravelBox {
    /// The box of a machine with `travel` (`$130`, `$131`): `[-travel, 0]` on each axis, or
    /// `[0, travel]` for an axis in `positive` (homed towards its positive end in a build that
    /// forces the origin to 0 there). A travel below 0 counts by its size and one that is not
    /// finite as 0, so `min <= max` on each axis and no clamp into the box can panic.
    #[must_use]
    pub fn new(travel: [f64; 2], positive: [bool; 2]) -> Self {
        let pick = |i: usize| {
            // invariant: `size` is finite and at least 0, so each pair below is ordered.
            let size = if travel[i].is_finite() {
                travel[i].abs()
            } else {
                0.0
            };
            if positive[i] {
                (0.0, size)
            } else {
                (-size, 0.0)
            }
        };
        let (x, y) = (pick(0), pick(1));
        Self {
            min: [x.0, y.0],
            max: [x.1, y.1],
        }
    }
}

/// The distance a jog of `delta` along `axis` may go from `from` (machine coordinates): `delta`
/// itself, a shorter one that ends at the edge, or `None` at the edge (nothing is written).
#[must_use]
pub fn clamp(
    from: [f64; 2],
    axis: JogAxis,
    delta: Millimetres,
    within: &TravelBox,
) -> Option<Millimetres> {
    let i = axis.index();
    let target = (from[i] + delta.0).clamp(within.min[i], within.max[i]);
    let allowed = target - from[i];
    (allowed.abs() > EDGE && allowed * delta.0 > 0.0).then_some(Millimetres(allowed))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOX: TravelBox = TravelBox {
        min: [-400.0, -300.0],
        max: [0.0, 0.0],
    };

    fn go(from: [f64; 2], axis: JogAxis, delta: f64, within: &TravelBox) -> Option<f64> {
        clamp(from, axis, Millimetres(delta), within).map(|m| m.0)
    }

    #[test]
    fn feeds_and_distances_are_the_closed_set() {
        let table = [
            (JogStep::Tenth, 0.1, 300.0),
            (JogStep::One, 1.0, 1_000.0),
            (JogStep::Ten, 10.0, 3_000.0),
            (JogStep::Fifty, 50.0, 3_000.0),
        ];
        for (step, distance, feed) in table {
            assert!((step.distance().0 - distance).abs() < 1e-12);
            assert!((step.feed().0 - feed).abs() < 1e-12);
        }
    }

    #[test]
    fn inside_the_box_a_jog_is_unchanged() {
        assert_eq!(go([-100.0, -100.0], JogAxis::X, 10.0, &BOX), Some(10.0));
        assert_eq!(go([-100.0, -100.0], JogAxis::Y, -50.0, &BOX), Some(-50.0));
    }

    #[test]
    fn a_jog_past_the_edge_moves_to_the_edge() {
        assert_eq!(go([-5.0, -100.0], JogAxis::X, 50.0, &BOX), Some(5.0));
        assert_eq!(go([-390.0, -100.0], JogAxis::X, -50.0, &BOX), Some(-10.0));
        assert_eq!(go([-5.0, -295.0], JogAxis::Y, -50.0, &BOX), Some(-5.0));
    }

    #[test]
    fn at_the_edge_nothing_is_written() {
        assert_eq!(go([0.0, -100.0], JogAxis::X, 1.0, &BOX), None);
        assert_eq!(go([-0.005, -100.0], JogAxis::X, 1.0, &BOX), None);
        assert_eq!(go([-400.0, -300.0], JogAxis::Y, -0.1, &BOX), None);
        // The way back from the edge is free.
        assert_eq!(go([0.0, -100.0], JogAxis::X, -1.0, &BOX), Some(-1.0));
    }

    #[test]
    fn a_step_just_over_the_tolerance_still_moves() {
        let moved = go([-0.1, 0.0], JogAxis::X, 1.0, &BOX);
        assert!(moved.is_some_and(|d| (d - 0.1).abs() < 1e-9));
    }

    #[test]
    fn the_box_follows_the_travel_and_the_positive_axes() {
        let standard = TravelBox::new([400.0, 300.0], [false, false]);
        assert_eq!(standard, BOX);
        let forced = TravelBox::new([400.0, 300.0], [true, false]);
        assert_eq!(forced.min, [0.0, -300.0]);
        assert_eq!(forced.max, [400.0, 0.0]);
        assert_eq!(go([0.0, -10.0], JogAxis::X, -5.0, &forced), None);
        assert_eq!(go([395.0, -10.0], JogAxis::X, 50.0, &forced), Some(5.0));
        let both = TravelBox::new([400.0, 300.0], [true, true]);
        assert_eq!(go([10.0, 295.0], JogAxis::Y, 50.0, &both), Some(5.0));
    }
}
