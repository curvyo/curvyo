//! The range of Weld's Gap setting (`specs/0068-laserable-lettering` criterion 2, `adrs.md`
//! decision 7): 0 to 1 mm, default 0.1 mm; a value outside is set to the nearest limit and the
//! field says so. The value itself is the frontend's per-user setting, never a project key.

use curvyo_document_core::Length;

/// The Gap a new user starts with, and the fallback when the stored value cannot be read.
pub const WELD_GAP_DEFAULT: Length = Length::from_mm(0.1);

/// The largest Gap.
const MAX_GAP_MM: f64 = 1.0;

/// What [`clamp_weld_gap`] did to a typed value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapClamp {
    /// The value was in range.
    InRange,
    /// The value was above 1 mm and is now 1 mm.
    Largest,
    /// The value was below 0 mm and is now 0 mm.
    Smallest,
}

/// `mm` within 0 to 1 mm, and whether it was moved there; a value that is not a number is the
/// default.
#[must_use]
pub fn clamp_weld_gap(mm: f64) -> (Length, GapClamp) {
    if mm.is_nan() {
        (WELD_GAP_DEFAULT, GapClamp::InRange)
    } else if mm > MAX_GAP_MM {
        (Length::from_mm(MAX_GAP_MM), GapClamp::Largest)
    } else if mm < 0.0 {
        (Length::from_mm(0.0), GapClamp::Smallest)
    } else {
        (Length::from_mm(mm), GapClamp::InRange)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criterion 2: values outside 0 to 1 mm go to the nearest limit.
    #[test]
    fn ac02_the_gap_is_clamped_to_its_range() {
        assert_eq!(
            clamp_weld_gap(1.5),
            (Length::from_mm(1.0), GapClamp::Largest)
        );
        assert_eq!(
            clamp_weld_gap(-0.2),
            (Length::from_mm(0.0), GapClamp::Smallest)
        );
        assert_eq!(
            clamp_weld_gap(0.05),
            (Length::from_mm(0.05), GapClamp::InRange)
        );
        assert_eq!(clamp_weld_gap(f64::NAN).0, WELD_GAP_DEFAULT);
    }
}
