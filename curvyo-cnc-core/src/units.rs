//! The unit newtypes of CNC milling (`specs/0161-cnc-output` `adrs.md` CN1): every value a
//! number with its unit and its allowed range.

/// One micrometre in mm: the grid the pass depths are counted on.
const UM: f64 = 1_000.0;

/// A length in mm known to lie within `[min, max]`, rounded to whole micrometres.
fn within(mm: f64, min: f64, max: f64) -> Option<f64> {
    (mm.is_finite() && mm >= min - 1e-9 && mm <= max + 1e-9).then(|| (mm * UM).round() / UM)
}

macro_rules! mm_unit {
    ($(#[$doc:meta])* $name:ident, $min:expr, $max:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        pub struct $name(f64);

        impl $name {
            /// The smallest value.
            pub const MIN: f64 = $min;
            /// The largest value.
            pub const MAX: f64 = $max;

            /// `mm` when it lies within the range, rounded to whole micrometres.
            #[must_use]
            pub fn new(mm: f64) -> Option<Self> {
                within(mm, Self::MIN, Self::MAX).map(Self)
            }

            /// The value.
            #[must_use]
            pub const fn get(self) -> f64 {
                self.0
            }

            /// The value in whole micrometres.
            #[must_use]
            #[allow(clippy::cast_possible_truncation)] // invariant: within the range, far below i64
            pub fn um(self) -> i64 {
                (self.0 * UM).round() as i64
            }
        }
    };
}

mm_unit!(
    /// An end mill's diameter in mm, 0.1 to 25 (UX 11).
    ToolDiameter,
    0.1,
    25.0
);
mm_unit!(
    /// A cut depth below the stock top in mm, 0.1 up to the stock thickness (criterion 5).
    Depth,
    0.1,
    1_000.0
);
mm_unit!(
    /// The depth of one pass in mm, 0.1 to 25 (criterion 8).
    StepDown,
    0.1,
    25.0
);
mm_unit!(
    /// A feed or a rapid rate in mm/min, 1 to 50 000.
    FeedMmMin,
    1.0,
    50_000.0
);
mm_unit!(
    /// An axis acceleration in mm/s², 1 to 10 000.
    AccelMmS2,
    1.0,
    10_000.0
);

/// A spindle speed in whole revolutions per minute, 0 to 100 000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rpm(u32);

impl Rpm {
    /// The largest value.
    pub const MAX: u32 = 100_000;

    /// `n` when it is at most 100 000.
    #[must_use]
    pub fn new(n: u64) -> Option<Self> {
        u32::try_from(n).ok().filter(|n| *n <= Self::MAX).map(Self)
    }

    /// The value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// The number of flutes of an end mill, 1 to 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Flutes(u8);

impl Flutes {
    /// `n` when it is 1 to 8.
    #[must_use]
    pub fn new(n: u64) -> Option<Self> {
        u8::try_from(n)
            .ok()
            .filter(|n| (1..=8).contains(n))
            .map(Self)
    }

    /// The value.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_keep_their_ranges_and_round_to_micrometres() {
        assert!(ToolDiameter::new(0.09).is_none());
        assert_eq!(ToolDiameter::new(3.175).unwrap().um(), 3_175);
        assert!(ToolDiameter::new(25.1).is_none());
        assert!(StepDown::new(f64::NAN).is_none());
        assert_eq!(Depth::new(1.000_000_1).unwrap().get(), 1.0);
        assert!(Rpm::new(100_001).is_none());
        assert!(Flutes::new(0).is_none());
        assert_eq!(Flutes::new(2).unwrap().get(), 2);
    }
}
