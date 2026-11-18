//! The types of a gradient fill (ADR 0066 §3): its geometry as fractions of the shape's box and
//! its validated stop list. No Loro type appears here; the stored form is `gradient_codec`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::path_model::Color;

/// Why building a gradient value failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GradientParamError {
    /// A box fraction must be a finite number.
    #[error("a gradient point must be a finite number")]
    NotFinite,
    /// The second radius over the first must be a finite number above 0.
    #[error("the radial ratio must be above 0")]
    RatioNotPositive,
    /// A stop offset must lie in `[0, 1]`.
    #[error("a stop position must be between 0 and 1")]
    OffsetOutOfRange,
    /// A gradient has at least two stops.
    #[error("a gradient needs at least two stops")]
    TooFewStops,
    /// The first stop sits at 0 and the last at 1.
    #[error("the first stop must be at 0 and the last at 1")]
    EndsNotAtZeroAndOne,
    /// Offsets never decrease along the list (equal ones are a hard edge).
    #[error("stop positions must not decrease")]
    Unordered,
}

/// A coordinate as a fraction of the shape's box: 0 is the left (top) edge, 1 the right (bottom)
/// edge; any finite value is allowed, so a point may lie outside the box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct BoxFraction(f64);

impl BoxFraction {
    /// Validates `value` as finite.
    ///
    /// # Errors
    /// [`GradientParamError::NotFinite`] otherwise.
    pub fn new(value: f64) -> Result<Self, GradientParamError> {
        if value.is_finite() {
            // `+ 0.0` folds a negative zero, so a stored `-0` never differs from `0`.
            Ok(Self(value + 0.0))
        } else {
            Err(GradientParamError::NotFinite)
        }
    }

    /// The fraction.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for BoxFraction {
    type Error = GradientParamError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<BoxFraction> for f64 {
    fn from(fraction: BoxFraction) -> Self {
        fraction.0
    }
}

/// A point of the gradient in the shape's box, as two [`BoxFraction`]s.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoxPoint {
    /// Across the box width.
    pub x: BoxFraction,
    /// Down the box height.
    pub y: BoxFraction,
}

impl BoxPoint {
    /// The point `(x, y)` in box fractions.
    ///
    /// # Errors
    /// [`GradientParamError::NotFinite`] for a coordinate that is not finite.
    pub fn new(x: f64, y: f64) -> Result<Self, GradientParamError> {
        Ok(Self {
            x: BoxFraction::new(x)?,
            y: BoxFraction::new(y)?,
        })
    }

    /// A point from constants known finite.
    const fn known(x: f64, y: f64) -> Self {
        Self {
            x: BoxFraction(x),
            y: BoxFraction(y),
        }
    }
}

/// The second radius of an elliptical radial gradient over its first: finite, above 0; 1 is a
/// circle in the box frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct AxisRatio(f64);

impl AxisRatio {
    /// A circle.
    pub const CIRCLE: Self = Self(1.0);

    /// Validates `value` as finite and above 0.
    ///
    /// # Errors
    /// [`GradientParamError::RatioNotPositive`] otherwise.
    pub fn new(value: f64) -> Result<Self, GradientParamError> {
        if value.is_finite() && value > 0.0 {
            Ok(Self(value))
        } else {
            Err(GradientParamError::RatioNotPositive)
        }
    }

    /// The ratio.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for AxisRatio {
    type Error = GradientParamError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<AxisRatio> for f64 {
    fn from(ratio: AxisRatio) -> Self {
        ratio.0
    }
}

/// Linear or Radial (the Kind switch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GradientKind {
    /// The shade runs along a line.
    Linear,
    /// The shade runs outward from a centre.
    Radial,
}

/// Where the gradient sits in the shape's box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum GradientGeometry {
    /// From `start` (offset 0) to `end` (offset 1); lines across it have one colour.
    Linear {
        /// The Start handle.
        start: BoxPoint,
        /// The End handle.
        end: BoxPoint,
    },
    /// From `centre` (offset 0) out to the ellipse through `radius` (offset 1), whose second
    /// axis is the first turned a quarter in the box frame and scaled by `ratio`.
    Radial {
        /// The centre handle.
        centre: BoxPoint,
        /// The radius handle, on the first axis.
        radius: BoxPoint,
        /// The second radius over the first.
        ratio: AxisRatio,
    },
}

impl GradientGeometry {
    /// Linear from the middle of the left edge to the middle of the right edge (criterion 4).
    pub const DEFAULT_LINEAR: Self = Self::Linear {
        start: BoxPoint::known(0.0, 0.5),
        end: BoxPoint::known(1.0, 0.5),
    };

    /// Radial from the box centre to the middle of the right edge, a circle (criterion 4).
    pub const DEFAULT_RADIAL: Self = Self::Radial {
        centre: BoxPoint::known(0.5, 0.5),
        radius: BoxPoint::known(1.0, 0.5),
        ratio: AxisRatio::CIRCLE,
    };

    /// The default geometry of `kind`.
    #[must_use]
    pub const fn default_for(kind: GradientKind) -> Self {
        match kind {
            GradientKind::Linear => Self::DEFAULT_LINEAR,
            GradientKind::Radial => Self::DEFAULT_RADIAL,
        }
    }

    /// Linear or Radial.
    #[must_use]
    pub const fn kind(&self) -> GradientKind {
        match self {
            Self::Linear { .. } => GradientKind::Linear,
            Self::Radial { .. } => GradientKind::Radial,
        }
    }
}

/// A stop's position along the gradient, in `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct StopOffset(f64);

impl StopOffset {
    /// The start of the gradient.
    pub const START: Self = Self(0.0);
    /// The end of the gradient.
    pub const END: Self = Self(1.0);

    /// Validates `value` against `[0, 1]`.
    ///
    /// # Errors
    /// [`GradientParamError::OffsetOutOfRange`] otherwise.
    pub fn new(value: f64) -> Result<Self, GradientParamError> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value + 0.0))
        } else {
            Err(GradientParamError::OffsetOutOfRange)
        }
    }

    /// `value` clamped into `[lo, hi]`; a value that is not finite reads as `lo`.
    #[must_use]
    pub fn clamped(value: f64, lo: Self, hi: Self) -> Self {
        if value.is_finite() {
            Self(value.clamp(lo.0, hi.0.max(lo.0)) + 0.0)
        } else {
            lo
        }
    }

    /// The position.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for StopOffset {
    type Error = GradientParamError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<StopOffset> for f64 {
    fn from(offset: StopOffset) -> Self {
        offset.0
    }
}

/// One colour stop: an RGB colour, its alpha (the last two hex digits) and its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GradientStop {
    /// The colour.
    pub colour: Color,
    /// Its alpha, 255 opaque.
    pub alpha: u8,
    /// Where it sits along the gradient.
    pub offset: StopOffset,
}

impl Eq for StopOffset {}

/// The validated stop list: at least two stops, the first at 0, the last at 1, positions never
/// decreasing; no upper limit (Frage 232 B). Shared, so a snapshot clone copies a pointer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<GradientStop>", into = "Vec<GradientStop>")]
pub struct GradientStops(Arc<[GradientStop]>);

impl GradientStops {
    /// Validates `stops` against the list rules (ADR 0066 §1).
    ///
    /// # Errors
    /// The first rule `stops` breaks.
    pub fn new(stops: Vec<GradientStop>) -> Result<Self, GradientParamError> {
        let (Some(first), Some(last)) = (stops.first(), stops.last()) else {
            return Err(GradientParamError::TooFewStops);
        };
        if stops.len() < 2 {
            return Err(GradientParamError::TooFewStops);
        }
        if first.offset != StopOffset::START || last.offset != StopOffset::END {
            return Err(GradientParamError::EndsNotAtZeroAndOne);
        }
        if stops.windows(2).any(|pair| pair[1].offset < pair[0].offset) {
            return Err(GradientParamError::Unordered);
        }
        Ok(Self(stops.into()))
    }

    /// Two stops: `start` at 0 and `end` at 1 (criterion 2's defaults).
    #[must_use]
    pub fn two(start: (Color, u8), end: (Color, u8)) -> Self {
        Self(Arc::from([
            GradientStop {
                colour: start.0,
                alpha: start.1,
                offset: StopOffset::START,
            },
            GradientStop {
                colour: end.0,
                alpha: end.1,
                offset: StopOffset::END,
            },
        ]))
    }

    /// The stops in order.
    #[must_use]
    pub fn as_slice(&self) -> &[GradientStop] {
        &self.0
    }

    /// How many stops there are (at least 2).
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always `false`: a stop list has at least two stops.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl TryFrom<Vec<GradientStop>> for GradientStops {
    type Error = GradientParamError;

    fn try_from(stops: Vec<GradientStop>) -> Result<Self, Self::Error> {
        Self::new(stops)
    }
}

impl From<GradientStops> for Vec<GradientStop> {
    fn from(stops: GradientStops) -> Self {
        stops.0.to_vec()
    }
}

/// A whole gradient fill: where it sits and its stops (the two registers `fill_gradient` and
/// `fill_gradient_stops`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientFill {
    /// Linear or Radial and its points.
    pub geometry: GradientGeometry,
    /// The colour stops.
    pub stops: GradientStops,
}

impl GradientFill {
    /// The fill a shape gets when Gradient is first chosen (criterion 2): Linear across the box,
    /// from the shape's Colour at its opacity to opaque white.
    #[must_use]
    pub fn first_for(colour: Color, alpha: u8) -> Self {
        Self {
            geometry: GradientGeometry::DEFAULT_LINEAR,
            stops: GradientStops::two((colour, alpha), (Color::WHITE, 255)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop(offset: f64) -> GradientStop {
        GradientStop {
            colour: Color::BLACK,
            alpha: 255,
            offset: StopOffset::new(offset).unwrap(),
        }
    }

    #[test]
    fn fractions_accept_any_finite_number() {
        for ok in [-3.0, 0.0, 0.5, 1.0, 12.5] {
            assert!(BoxFraction::new(ok).is_ok());
        }
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(BoxFraction::new(bad), Err(GradientParamError::NotFinite));
        }
        assert_eq!(
            BoxFraction::new(-0.0).unwrap().get().to_bits(),
            0.0f64.to_bits()
        );
    }

    #[test]
    fn a_ratio_is_above_zero() {
        assert!(AxisRatio::new(0.25).is_ok());
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                AxisRatio::new(bad),
                Err(GradientParamError::RatioNotPositive)
            );
        }
    }

    #[test]
    fn offsets_lie_in_the_unit_interval() {
        assert!(StopOffset::new(0.0).is_ok() && StopOffset::new(1.0).is_ok());
        for bad in [-0.01, 1.01, f64::NAN] {
            assert_eq!(
                StopOffset::new(bad),
                Err(GradientParamError::OffsetOutOfRange)
            );
        }
        let (lo, hi) = (StopOffset::new(0.2).unwrap(), StopOffset::new(0.4).unwrap());
        assert_eq!(StopOffset::clamped(0.9, lo, hi), hi);
        assert_eq!(StopOffset::clamped(0.1, lo, hi), lo);
        assert_eq!(StopOffset::clamped(f64::NAN, lo, hi), lo);
    }

    #[test]
    fn a_stop_list_follows_the_rules() {
        assert!(GradientStops::new(vec![stop(0.0), stop(1.0)]).is_ok());
        assert!(
            GradientStops::new(vec![stop(0.0), stop(0.5), stop(0.5), stop(1.0)]).is_ok(),
            "equal positions are a hard edge"
        );
        let cases = [
            (vec![], GradientParamError::TooFewStops),
            (vec![stop(0.0)], GradientParamError::TooFewStops),
            (
                vec![stop(0.1), stop(1.0)],
                GradientParamError::EndsNotAtZeroAndOne,
            ),
            (
                vec![stop(0.0), stop(0.9)],
                GradientParamError::EndsNotAtZeroAndOne,
            ),
            (
                vec![stop(0.0), stop(0.6), stop(0.4), stop(1.0)],
                GradientParamError::Unordered,
            ),
        ];
        for (stops, error) in cases {
            assert_eq!(GradientStops::new(stops.clone()), Err(error), "{stops:?}");
        }
    }

    #[test]
    fn there_is_no_upper_limit_on_stops() {
        let n = 100_000;
        let stops = (0..=n).map(|i| stop(f64::from(i) / f64::from(n))).collect();
        assert_eq!(GradientStops::new(stops).unwrap().len(), 100_001);
    }

    #[test]
    fn the_first_gradient_runs_from_the_colour_to_white_across_the_box() {
        let red = Color { r: 255, g: 0, b: 0 };
        let fill = GradientFill::first_for(red, 128);
        assert_eq!(fill.geometry, GradientGeometry::DEFAULT_LINEAR);
        let [a, b] = fill.stops.as_slice() else {
            panic!("two stops")
        };
        assert_eq!((a.colour, a.alpha, a.offset.get()), (red, 128, 0.0));
        assert_eq!(
            (b.colour, b.alpha, b.offset.get()),
            (Color::WHITE, 255, 1.0)
        );
    }

    #[test]
    fn serde_validates_the_list() {
        let fill = GradientFill::first_for(Color::BLACK, 255);
        let json = serde_json::to_string(&fill).unwrap();
        assert_eq!(serde_json::from_str::<GradientFill>(&json).unwrap(), fill);
        assert!(serde_json::from_str::<GradientStops>("[]").is_err());
    }
}
