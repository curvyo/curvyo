//! The Pencil's session settings and how they turn pen pressure into stroke widths: smoothing
//! first, the curve second, then the Minimum-to-Maximum mapping (`specs/0032-pen-tablet-input`
//! criteria 9 and 14).

use curvyo_document_core::Length;

/// Lowest and highest Minimum width in millimetres (criterion 14).
const MINIMUM_RANGE_MM: (f64, f64) = (0.05, 20.0);
/// Lowest and highest Maximum width in millimetres (criterion 14).
const MAXIMUM_RANGE_MM: (f64, f64) = (0.1, 50.0);

/// How pressure maps to width between Minimum and Maximum.
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
    /// The curve applied to a pressure from 0 to 1 (clamped to that range first).
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

/// The Pencil's settings for the session; never saved in the document (criteria 14, 16).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PencilSettings {
    minimum: Length,
    maximum: Length,
    /// The pressure curve.
    pub curve: PressureCurve,
}

impl Default for PencilSettings {
    fn default() -> Self {
        Self {
            minimum: Length::from_mm(0.2),
            maximum: Length::from_mm(2.0),
            curve: PressureCurve::Linear,
        }
    }
}

/// `value` clamped to `range`; NaN becomes the lower end.
fn clamped(value: f64, (low, high): (f64, f64)) -> f64 {
    if value.is_nan() {
        low
    } else {
        value.clamp(low, high)
    }
}

impl PencilSettings {
    /// The width at no pressure.
    #[must_use]
    pub const fn minimum(&self) -> Length {
        self.minimum
    }

    /// The width at full pressure, never below [`Self::minimum`].
    #[must_use]
    pub const fn maximum(&self) -> Length {
        self.maximum
    }

    /// Sets the Minimum width, clamped to 0.05 to 20 mm; a Maximum below it is raised to it.
    pub fn set_minimum(&mut self, width: Length) {
        let minimum = clamped(width.as_mm(), MINIMUM_RANGE_MM);
        self.minimum = Length::from_mm(minimum);
        self.maximum = Length::from_mm(self.maximum.as_mm().max(minimum));
    }

    /// Sets the Maximum width, clamped to 0.1 to 50 mm and raised to the Minimum if below it.
    pub fn set_maximum(&mut self, width: Length) {
        let maximum = clamped(width.as_mm(), MAXIMUM_RANGE_MM).max(self.minimum.as_mm());
        self.maximum = Length::from_mm(maximum);
    }

    /// The width for a smoothed pressure: `Minimum + (Maximum - Minimum) x curve(p)`.
    #[must_use]
    pub fn width_for(&self, smoothed_pressure: f64) -> Length {
        let (low, high) = (self.minimum.as_mm(), self.maximum.as_mm());
        Length::from_mm(low + (high - low) * self.curve.apply(smoothed_pressure))
    }

    /// The width of every sample of a stroke from its raw pressures.
    #[must_use]
    pub fn widths(&self, pressures: &[f64]) -> Vec<Length> {
        smooth_pressures(pressures)
            .into_iter()
            .map(|p| self.width_for(p))
            .collect()
    }
}

/// Each pressure replaced by the mean of itself and the two raw pressures before it (fewer at
/// the start).
#[must_use]
pub fn smooth_pressures(pressures: &[f64]) -> Vec<f64> {
    (0..pressures.len())
        .map(|index| {
            let window = &pressures[index.saturating_sub(2)..=index];
            // At most three values, exact as a float.
            #[allow(clippy::cast_precision_loss)]
            let count = window.len() as f64;
            window.iter().sum::<f64>() / count
        })
        .collect()
}
