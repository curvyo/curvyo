//! The words a mirror shows the maker (`specs/0056-mirror-and-negative-scale/` criteria
//! 3, 10 and 15 and the UX notes): the signed size in the live readout, the line that
//! names the mirror, the flip announcement and the two notices.

use crate::flip::FlipAxis;
use crate::transform_math::ZeroRecovery;

/// The notice of a flip that changes nothing (criterion 3).
pub const FLIP_UNCHANGED_NOTICE: &str = "The selection looks the same flipped. Nothing changed.";

/// `magnitude` millimetres with a real minus sign (U+2212) when `negative`, without the
/// unit: one decimal, two under 0.1 mm (criterion 10: "0.01", "−0.01", and "−0.0" just
/// past zero, since the sign follows the factor, not the rounded text).
#[must_use]
pub fn signed_size_text(magnitude: f64, negative: bool) -> String {
    let magnitude = magnitude.abs();
    let digits = if magnitude < 0.1 { 2 } else { 1 };
    let sign = if negative { "\u{2212}" } else { "" };
    format!("{sign}{magnitude:.digits$}")
}

/// The line under the readout and in the chip that names the mirror of signed factors
/// `(sx, sy)`: `None` when neither is negative.
#[must_use]
pub fn mirror_line(sx: f64, sy: f64) -> Option<&'static str> {
    match (sx < 0.0, sy < 0.0) {
        (true, true) => Some("Turned 180°"),
        (true, false) => Some("Mirrored left to right"),
        (false, true) => Some("Mirrored top to bottom"),
        (false, false) => None,
    }
}

/// The polite announcement of a flip (criterion 2).
#[must_use]
pub const fn flip_announcement(axis: FlipAxis) -> &'static str {
    match axis {
        FlipAxis::Horizontal => "Flipped horizontally",
        FlipAxis::Vertical => "Flipped vertically",
    }
}

/// The notice after a release or an Enter that set an axis to 0.01 mm (criterion 15):
/// `None` when no axis was set so.
#[must_use]
pub const fn zero_recovery_notice(recovery: ZeroRecovery) -> Option<&'static str> {
    match (recovery.width, recovery.height) {
        (true, true) => Some("Width and height set to 0.01 mm. Zero would flatten the shape."),
        (true, false) => {
            Some("Width set to 0.01 mm. A zero size would flatten the shape for good.")
        }
        (false, true) => {
            Some("Height set to 0.01 mm. A zero size would flatten the shape for good.")
        }
        (false, false) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criterion 10: a real minus, two decimals under 0.1 mm, the sign of the factor.
    #[test]
    fn signed_sizes() {
        assert_eq!(signed_size_text(12.5, true), "\u{2212}12.5");
        assert_eq!(signed_size_text(8.0, false), "8.0");
        assert_eq!(signed_size_text(0.01, false), "0.01");
        assert_eq!(signed_size_text(0.01, true), "\u{2212}0.01");
        assert_eq!(signed_size_text(0.0001, true), "\u{2212}0.00");
    }

    /// Criterion 15 and the UX notes: the texts stay within 70 characters.
    #[test]
    fn notices_fit_the_limit() {
        for recovery in [(true, false), (false, true), (true, true)] {
            let text = zero_recovery_notice(ZeroRecovery {
                width: recovery.0,
                height: recovery.1,
            })
            .expect("a notice");
            assert!(text.chars().count() <= 70, "{text}");
        }
        assert_eq!(zero_recovery_notice(ZeroRecovery::default()), None);
        assert!(FLIP_UNCHANGED_NOTICE.chars().count() <= 70);
    }
}
