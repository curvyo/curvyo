//! How the Pencil maps pen pressure to a stroke width (`specs/0158-pen-display` slice 1
//! criteria 3, 4, 7; ADR 0095 decision 6; the rules of `0032` criteria 3, 9 and 14).

use curvyo_document_core::Length;

/// Lowest and highest Min width in millimetres.
const MIN_RANGE_MM: (f64, f64) = (0.05, 20.0);
/// Lowest and highest Max width in millimetres.
const MAX_RANGE_MM: (f64, f64) = (0.1, 50.0);
/// The pressures a device reports when it has none to report (`0032` criterion 3): 0 for a pen
/// without a pressure axis, 0.5 for a pressed mouse button in Pointer Events.
const NO_PRESSURE_VALUES: [f64; 2] = [0.0, 0.5];
/// How many raw pressures the trailing mean takes: the sample and the two before it.
const SMOOTHING_WINDOW: usize = 3;

/// How pressure maps to width between Min and Max.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PressureCurve {
    /// Light pressure already gives a wide line: `p^0.5`.
    Soft,
    /// Width in proportion to pressure: `p`.
    #[default]
    Linear,
    /// It takes firm pressure to widen the line: `p²`.
    Firm,
}

impl PressureCurve {
    /// The curve applied to a pressure, clamped to 0 to 1 first (not-a-number reads as 0).
    #[must_use]
    pub fn apply(self, pressure: f64) -> f64 {
        let p = if pressure.is_finite() {
            pressure.clamp(0.0, 1.0)
        } else {
            0.0
        };
        match self {
            Self::Soft => p.sqrt(),
            Self::Linear => p,
            Self::Firm => p * p,
        }
    }
}

/// The Pencil's Min and Max widths (criterion 4): Max is never below Min.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PencilWidths {
    min: Length,
    max: Length,
}

impl Default for PencilWidths {
    fn default() -> Self {
        Self {
            min: Length::from_mm(0.2),
            max: Length::from_mm(2.0),
        }
    }
}

/// `value` clamped to `range`; not-a-number becomes the lower end.
fn clamped(value: f64, (low, high): (f64, f64)) -> f64 {
    if value.is_nan() {
        low
    } else {
        value.clamp(low, high)
    }
}

impl PencilWidths {
    /// The width at no pressure.
    #[must_use]
    pub const fn min(&self) -> Length {
        self.min
    }

    /// The width at full pressure.
    #[must_use]
    pub const fn max(&self) -> Length {
        self.max
    }

    /// Sets Min, clamped to 0.05 to 20 mm; a Max below it is raised to it (UX notes §2).
    pub fn set_min(&mut self, width: Length) {
        let min = clamped(width.as_mm(), MIN_RANGE_MM);
        self.min = Length::from_mm(min);
        self.max = Length::from_mm(self.max.as_mm().max(min));
    }

    /// Sets Max, clamped to 0.1 to 50 mm and raised to Min if below it.
    pub fn set_max(&mut self, width: Length) {
        let max = clamped(width.as_mm(), MAX_RANGE_MM).max(self.min.as_mm());
        self.max = Length::from_mm(max);
    }

    /// `Min + (Max − Min) × curve(p)` for a smoothed pressure `p` (criterion 3).
    #[must_use]
    pub fn width_for(&self, curve: PressureCurve, smoothed: f64) -> Length {
        let (low, high) = (self.min.as_mm(), self.max.as_mm());
        Length::from_mm(low + (high - low) * curve.apply(smoothed))
    }

    /// The width tolerance of a stroke's profile: 5 % of Max − Min (criterion 5).
    #[must_use]
    pub fn profile_tolerance(&self) -> Length {
        Length::from_mm(PROFILE_SHARE * (self.max.as_mm() - self.min.as_mm()))
    }
}

/// The share of Max − Min a profile may stray from the sampled widths (criterion 5, Proposal).
const PROFILE_SHARE: f64 = 0.05;

/// Whether a pressure says the device measures pressure: not 0 and not 0.5 (`0032` criterion 3).
#[must_use]
pub fn is_usable_pressure(pressure: f64) -> bool {
    pressure.is_finite() && !NO_PRESSURE_VALUES.contains(&pressure)
}

/// The trailing mean of the raw pressure at `index` and the two before it (fewer at the start;
/// `0032` criterion 9).
#[must_use]
pub fn smoothed_at(raw: &[f64], index: usize) -> f64 {
    let window = &raw[index.saturating_sub(SMOOTHING_WINDOW - 1)..=index];
    // At most three values: the count is exact as a float.
    #[allow(clippy::cast_precision_loss)]
    let count = window.len() as f64;
    window.iter().sum::<f64>() / count
}

/// Every raw pressure replaced by its trailing mean.
#[must_use]
pub fn smooth_pressures(raw: &[f64]) -> Vec<f64> {
    (0..raw.len()).map(|i| smoothed_at(raw, i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criterion_3_smoothing_numbers() {
        let smoothed = smooth_pressures(&[0.2, 0.2, 1.0, 0.2, 0.2]);
        for (got, want) in smoothed.iter().zip([0.2, 0.2, 0.467, 0.467, 0.467]) {
            assert!((got - want).abs() <= 0.001, "{smoothed:?}");
        }
    }

    #[test]
    fn criterion_3_curve_numbers() {
        assert!((PressureCurve::Soft.apply(0.25) - 0.5).abs() < 1e-12);
        assert!((PressureCurve::Linear.apply(0.25) - 0.25).abs() < 1e-12);
        assert!((PressureCurve::Firm.apply(0.25) - 0.0625).abs() < 1e-12);
        assert_eq!(PressureCurve::Linear.apply(f64::NAN), 0.0);
        assert_eq!(PressureCurve::Soft.apply(2.0), 1.0);
    }

    #[test]
    fn widths_map_min_to_max_and_soft_is_widest() {
        let widths = PencilWidths::default();
        assert!((widths.min().as_mm() - 0.2).abs() < 1e-12);
        assert!((widths.max().as_mm() - 2.0).abs() < 1e-12);
        let at = |curve, p| widths.width_for(curve, p).as_mm();
        assert!((at(PressureCurve::Linear, 0.0) - 0.2).abs() < 1e-12);
        assert!((at(PressureCurve::Linear, 1.0) - 2.0).abs() < 1e-12);
        assert!((at(PressureCurve::Linear, 0.25) - (0.2 + 1.8 * 0.25)).abs() < 1e-12);
        assert!(at(PressureCurve::Soft, 0.5) > at(PressureCurve::Linear, 0.5));
        assert!(at(PressureCurve::Linear, 0.5) > at(PressureCurve::Firm, 0.5));
    }

    #[test]
    fn min_and_max_keep_their_ranges_and_order() {
        let mut widths = PencilWidths::default();
        widths.set_max(Length::from_mm(0.01));
        assert!((widths.max().as_mm() - 0.2).abs() < 1e-12, "raised to Min");
        widths.set_min(Length::from_mm(5.0));
        assert!((widths.min().as_mm() - 5.0).abs() < 1e-12);
        assert!(
            (widths.max().as_mm() - 5.0).abs() < 1e-12,
            "Min above Max raises Max"
        );
        widths.set_min(Length::from_mm(100.0));
        assert!((widths.min().as_mm() - 20.0).abs() < 1e-12);
        widths.set_max(Length::from_mm(100.0));
        assert!((widths.max().as_mm() - 50.0).abs() < 1e-12);
        widths.set_min(Length::from_mm(0.0));
        assert!((widths.min().as_mm() - 0.05).abs() < 1e-12);
        widths.set_min(Length::from_mm(f64::NAN));
        assert!((widths.min().as_mm() - 0.05).abs() < 1e-12);
    }

    #[test]
    fn the_has_pressure_rule() {
        assert!(!is_usable_pressure(0.0));
        assert!(!is_usable_pressure(0.5));
        assert!(!is_usable_pressure(f64::NAN));
        assert!(is_usable_pressure(0.3));
        assert!(is_usable_pressure(1.0));
    }

    #[test]
    fn the_profile_tolerance_is_five_percent_of_the_range() {
        let tolerance = PencilWidths::default().profile_tolerance().as_mm();
        assert!((tolerance - 0.09).abs() < 1e-12);
    }
}
