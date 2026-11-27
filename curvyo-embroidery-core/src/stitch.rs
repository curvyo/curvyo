//! The stitch model (`specs/0160-embroidery` criteria 10, 14 to 17, `adrs.md` EM2, EM3, EM10):
//! needle positions in whole tenths of a mm from the hoop centre, Y up, and what the machine does
//! at each.

/// A coordinate in whole tenths of a millimetre, the DST unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TenthMm(i32);

impl TenthMm {
    /// `tenths` tenths of a mm.
    #[must_use]
    pub const fn new(tenths: i32) -> Self {
        Self(tenths)
    }

    /// `mm` rounded half away from zero to whole tenths, saturating far outside any hoop.
    #[must_use]
    pub fn of_mm(mm: f64) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        // invariant: clamped to the i32 range before the cast; NaN casts to 0
        let tenths = (mm * 10.0)
            .round()
            .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
        Self(tenths)
    }

    /// The tenths.
    #[must_use]
    pub const fn get(self) -> i32 {
        self.0
    }

    /// The value in mm.
    #[must_use]
    pub fn mm(self) -> f64 {
        f64::from(self.0) / 10.0
    }
}

/// A needle position: tenths of a mm from the hoop centre, X right, Y up (EM10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StitchPoint {
    /// Right of the centre.
    pub x: TenthMm,
    /// Above the centre.
    pub y: TenthMm,
}

impl StitchPoint {
    /// The hoop centre, where the machine starts.
    pub const ORIGIN: Self = Self::new(0, 0);

    /// The point `x`, `y` in tenths.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self {
            x: TenthMm::new(x),
            y: TenthMm::new(y),
        }
    }

    /// The distance to `other` in mm.
    #[must_use]
    pub fn distance_mm(self, other: Self) -> f64 {
        let dx = f64::from(other.x.get() - self.x.get());
        let dy = f64::from(other.y.get() - self.y.get());
        dx.hypot(dy) / 10.0
    }
}

/// What the machine does at a stitch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StitchKind {
    /// The needle goes down at the point.
    Normal,
    /// The frame moves to the point without stitching.
    Jump,
    /// The thread is cut where the needle stands.
    Trim,
    /// The machine stops for the next thread (the DST stop record).
    ColourChange,
    /// The design ends.
    End,
}

/// One entry of the stitch list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stitch {
    /// Where the needle stands after it.
    pub at: StitchPoint,
    /// What happens.
    pub kind: StitchKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn millimetres_round_to_whole_tenths() {
        assert_eq!(TenthMm::of_mm(1.25).get(), 13);
        assert_eq!(TenthMm::of_mm(-1.25).get(), -13);
        assert_eq!(TenthMm::of_mm(0.04).get(), 0);
        assert_eq!(TenthMm::new(121).mm(), 12.1);
        assert_eq!(
            StitchPoint::new(0, 0).distance_mm(StitchPoint::new(30, 40)),
            5.0
        );
    }
}
