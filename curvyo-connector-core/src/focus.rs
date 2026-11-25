//! The bounds of a go to point (`specs/0118` 2d, adrs P2, P4): the pulse's power and length caps
//! the connector owns, the reach of a target, and the clamp of a target into the travel.

use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize};

use crate::jog::TravelBox;
use crate::millimetres::Millimetres;

/// The farthest a target may lie from the origin on either axis, in millimetres; a target
/// beyond is malformed and nothing moves.
pub const MAX_TARGET: Millimetres = Millimetres(10_000.0);

/// How close the head must stand to the target, per axis, for the pulse to fire (35 c).
pub const ARRIVED: Millimetres = Millimetres(0.05);

/// How much longer than its length a pulse may run before the connector stops it (35 c).
pub const PULSE_GRACE: Duration = Duration::from_secs(1);

/// Defines a `u16` newtype that refuses every value outside its range on decode.
macro_rules! bounded {
    ($(#[$doc:meta])* $name:ident, $min:expr, $max:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "u16", into = "u16")]
        pub struct $name(u16);

        impl $name {
            /// The smallest value.
            pub const MIN: u16 = $min;
            /// The largest value.
            pub const MAX: u16 = $max;

            /// The value `n`, `None` outside the range.
            #[must_use]
            pub fn new(n: u16) -> Option<Self> {
                (Self::MIN..=Self::MAX).contains(&n).then_some(Self(n))
            }

            /// The plain number.
            #[must_use]
            pub const fn get(self) -> u16 {
                self.0
            }
        }

        impl TryFrom<u16> for $name {
            type Error = &'static str;
            fn try_from(n: u16) -> Result<Self, Self::Error> {
                Self::new(n).ok_or("out of range")
            }
        }

        impl From<$name> for u16 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

bounded!(
    /// The pulse power in per mille of the controller's full power (`$30`): 1 to 50, so 0.1 to 5 %.
    PulsePower, 1, 50
);
bounded!(
    /// The pulse length in milliseconds: 50 to 2,000.
    PulseLength, 50, 2_000
);

impl PulseLength {
    /// The length as a duration.
    #[must_use]
    pub fn duration(self) -> Duration {
        Duration::from_millis(u64::from(self.0))
    }
}

/// The low-power pulse a go to point fires on arrival (criterion 35).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pulse {
    /// Its power.
    pub power_permille: PulsePower,
    /// Its length.
    pub length_ms: PulseLength,
}

/// Reads one coordinate of a target: a finite number within [`MAX_TARGET`].
///
/// # Errors
/// A custom error for a value that is not a number, not finite or out of reach.
pub(crate) fn within_reach<'de, D: Deserializer<'de>>(d: D) -> Result<Millimetres, D::Error> {
    let value = f64::deserialize(d)?;
    if value.is_finite() && value.abs() <= MAX_TARGET.0 {
        Ok(Millimetres(value))
    } else {
        Err(serde::de::Error::custom("target out of reach"))
    }
}

/// The target clamped into the travel: `target` (job file coordinates) plus the work offset `wco`
/// is the machine target, clamped per axis into `within`, and taken back to job file coordinates.
/// Without a box (`$130`/`$131` unknown) the target stays as it is; GRBL's limits apply.
#[must_use]
pub fn clamp_target(
    target: [Millimetres; 2],
    wco: [Millimetres; 2],
    within: Option<&TravelBox>,
) -> [Millimetres; 2] {
    let Some(within) = within else {
        return target;
    };
    [0, 1].map(|i| {
        let machine = (target[i].0 + wco[i].0).clamp(within.min[i], within.max[i]);
        Millimetres(machine - wco[i].0)
    })
}

/// Whether `position` stands within [`ARRIVED`] of `target` on X and Y.
#[must_use]
pub fn arrived(position: [Millimetres; 3], target: [Millimetres; 2]) -> bool {
    (0..2).all(|i| (position[i].0 - target[i].0).abs() <= ARRIVED.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mm(p: [f64; 2]) -> [Millimetres; 2] {
        p.map(Millimetres)
    }

    fn plain(p: [Millimetres; 2]) -> [f64; 2] {
        p.map(|m| m.0)
    }

    #[test]
    fn the_caps_refuse_everything_outside() {
        assert_eq!(PulsePower::new(0), None);
        assert_eq!(PulsePower::new(1).map(PulsePower::get), Some(1));
        assert_eq!(PulsePower::new(50).map(PulsePower::get), Some(50));
        assert_eq!(PulsePower::new(51), None);
        assert_eq!(PulseLength::new(49), None);
        assert_eq!(PulseLength::new(50).map(PulseLength::get), Some(50));
        assert_eq!(PulseLength::new(2_000).map(PulseLength::get), Some(2_000));
        assert_eq!(PulseLength::new(2_001), None);
        assert_eq!(
            PulseLength::new(300).map(PulseLength::duration),
            Some(Duration::from_millis(300))
        );
    }

    #[test]
    fn a_standard_box_clamps_each_axis_and_keeps_the_offset() {
        let within = TravelBox::new([400.0, 300.0], [false, false]);
        let wco = mm([-400.0, -300.0]);
        // Job file (0, 0) is machine (-400, -300): the far corner, inside.
        assert_eq!(
            plain(clamp_target(mm([10.0, 20.0]), wco, Some(&within))),
            [10.0, 20.0]
        );
        assert_eq!(
            plain(clamp_target(mm([450.0, -5.0]), wco, Some(&within))),
            [400.0, 0.0]
        );
        let none = [Millimetres(0.0); 2];
        assert_eq!(
            plain(clamp_target(mm([10.0, -20.0]), none, Some(&within))),
            [0.0, -20.0]
        );
    }

    #[test]
    fn a_positive_homing_axis_clamps_into_zero_to_travel() {
        let within = TravelBox::new([400.0, 300.0], [true, false]);
        let none = [Millimetres(0.0); 2];
        assert_eq!(
            plain(clamp_target(mm([-5.0, -310.0]), none, Some(&within))),
            [0.0, -300.0]
        );
        assert_eq!(
            plain(clamp_target(mm([401.0, -10.0]), none, Some(&within))),
            [400.0, -10.0]
        );
    }

    #[test]
    fn a_box_from_a_travel_below_zero_clamps_without_a_panic() {
        // A non-GRBL firmware may report `$130`/`$131` below 0; core must not panic on it.
        let within = TravelBox::new([-400.0, f64::NAN], [false, true]);
        assert!((0..2).all(|i| within.min[i] <= within.max[i]), "{within:?}");
        let none = [Millimetres(0.0); 2];
        assert_eq!(
            plain(clamp_target(mm([-500.0, 5.0]), none, Some(&within))),
            [-400.0, 0.0]
        );
    }

    #[test]
    fn without_a_box_the_target_stays() {
        assert_eq!(
            plain(clamp_target(mm([9_000.0, -9_000.0]), mm([1.0, 1.0]), None)),
            [9_000.0, -9_000.0]
        );
    }

    #[test]
    fn arrival_is_within_five_hundredths_per_axis() {
        let target = mm([10.0, 20.0]);
        let at = |x: f64, y: f64| [Millimetres(x), Millimetres(y), Millimetres(0.0)];
        assert!(arrived(at(10.049, 19.951), target));
        assert!(!arrived(at(10.051, 20.0), target));
        assert!(!arrived(at(10.0, 19.949), target));
    }
}
