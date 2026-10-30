//! Sketch quantities retain physical dimensions during formula evaluation.
use crate::SketchError;
use serde::{Deserialize, Serialize};

/// A quantity's physical dimension; compound dimensions are not sketch inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dimension {
    /// Unitless ratio or multiplier.
    Scalar,
    /// Distance measured internally in millimetres.
    Length,
    /// Angle measured internally in radians.
    Angle,
}

/// A finite dimensioned value in canonical units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawQuantity")]
pub struct Quantity {
    value: f64,
    dimension: Dimension,
}
impl Quantity {
    /// Constructs a finite value with explicit dimension.
    ///
    /// # Errors
    /// Rejects non-finite numeric input.
    pub fn new(value: f64, dimension: Dimension) -> Result<Self, SketchError> {
        if !value.is_finite() {
            return Err(SketchError::Formula("non-finite value".into()));
        }
        Ok(Self { value, dimension })
    }
    /// Constructs a millimetre value.
    ///
    /// # Errors
    /// Rejects non-finite numeric input.
    pub fn length(value: f64) -> Result<Self, SketchError> {
        Self::new(value, Dimension::Length)
    }
    /// Constructs a radian value.
    ///
    /// # Errors
    /// Rejects non-finite numeric input.
    pub fn angle(value: f64) -> Result<Self, SketchError> {
        Self::new(value, Dimension::Angle)
    }
    /// Constructs a dimensionless value.
    ///
    /// # Errors
    /// Rejects non-finite numeric input.
    pub fn scalar(value: f64) -> Result<Self, SketchError> {
        Self::new(value, Dimension::Scalar)
    }
    /// Returns the canonical numeric value.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.value
    }
    /// Returns the physical dimension.
    #[must_use]
    pub const fn dimension(self) -> Dimension {
        self.dimension
    }
    pub(crate) fn add(self, b: Self, subtract: bool) -> Result<Self, SketchError> {
        if self.dimension != b.dimension {
            return Err(SketchError::Formula(
                "addition requires matching units".into(),
            ));
        }
        Self::new(
            if subtract {
                self.value - b.value
            } else {
                self.value + b.value
            },
            self.dimension,
        )
    }
    pub(crate) fn multiply(self, b: Self, divide: bool) -> Result<Self, SketchError> {
        let dimension = if divide && self.dimension == b.dimension {
            Dimension::Scalar
        } else if b.dimension == Dimension::Scalar {
            self.dimension
        } else if !divide && self.dimension == Dimension::Scalar {
            b.dimension
        } else {
            return Err(SketchError::Formula("unsupported compound units".into()));
        };
        Self::new(
            if divide {
                self.value / b.value
            } else {
                self.value * b.value
            },
            dimension,
        )
    }
}

/// A signed millimetre coordinate or distance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Length(pub f64);
impl Length {
    /// Returns whether the coordinate is finite.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}
/// A signed radian angle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Angle(pub f64);
/// A point in the sketch's local coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// Local horizontal coordinate.
    pub x: Length,
    /// Local vertical coordinate.
    pub y: Length,
}
impl Position {
    /// Constructs a local point from millimetres.
    #[must_use]
    pub const fn mm(x: f64, y: f64) -> Self {
        Self {
            x: Length(x),
            y: Length(y),
        }
    }
    /// Euclidean separation in millimetres.
    #[must_use]
    pub fn distance(self, other: Self) -> Length {
        Length((self.x.0 - other.x.0).hypot(self.y.0 - other.y.0))
    }
}

#[derive(Deserialize)]
struct RawQuantity {
    value: f64,
    dimension: Dimension,
}
impl TryFrom<RawQuantity> for Quantity {
    type Error = SketchError;
    fn try_from(raw: RawQuantity) -> Result<Self, Self::Error> {
        Self::new(raw.value, raw.dimension)
    }
}
