//! The value table view of a pattern generator (`specs/0071-pattern-fill`, ADR 0029 §2): its
//! parameters as stored numbers (mm, degrees, share, count) in the table's order, read, set and
//! checked against `pattern_schema`, and carried over to another generator.

use crate::pattern_model::{
    GeneratorKind, PatternCount, PatternFraction, PatternGenerator, PatternParam, PatternParamError,
};
use crate::pattern_schema::{ParamKind, spec_of, specs};
use crate::units::{Angle, Length};

/// Degrees as stored and shown, from radians: rounded to 1e-9 so a value read and written back
/// is unchanged.
fn degrees(angle: Angle) -> f64 {
    (angle.as_radians().to_degrees() * 1e9).round() / 1e9 + 0.0
}

fn radians(degrees: f64) -> Angle {
    Angle::from_radians(degrees.to_radians())
}

fn mm(value: f64) -> Length {
    Length::from_mm(value)
}

fn fraction(value: f64) -> Result<PatternFraction, PatternParamError> {
    PatternFraction::new(value).ok_or(PatternParamError::OutOfRange)
}

fn count(value: f64) -> Result<PatternCount, PatternParamError> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // checked by the table
    PatternCount::new(value as u32).ok_or(PatternParamError::OutOfRange)
}

/// Honeycomb from Cell size and Wall; the wall must be thinner than the cell.
fn honeycomb(v: &[f64]) -> Result<PatternGenerator, PatternParamError> {
    if v[1] >= v[0] {
        return Err(PatternParamError::WiderThanCell);
    }
    Ok(PatternGenerator::Honeycomb {
        cell_size: mm(v[0]),
        wall: mm(v[1]),
    })
}

/// Brick wall from Width, Height and Joint; the joint must be thinner than the brick.
fn brick_wall(v: &[f64]) -> Result<PatternGenerator, PatternParamError> {
    if v[2] >= v[0].min(v[1]) {
        return Err(PatternParamError::WiderThanCell);
    }
    Ok(PatternGenerator::BrickWall {
        width: mm(v[0]),
        height: mm(v[1]),
        joint: mm(v[2]),
    })
}

impl PatternGenerator {
    /// The parameter values in the table's order, in stored units (mm, degrees, share, count).
    #[must_use]
    pub fn values(&self) -> Vec<f64> {
        let mm = Length::as_mm;
        match *self {
            Self::Hatch { spacing, angle } | Self::Grid { spacing, angle } => {
                vec![mm(spacing), degrees(angle)]
            }
            Self::Crosshatch {
                spacing,
                angle,
                second_angle,
            } => vec![mm(spacing), degrees(angle), degrees(second_angle)],
            Self::Dots { spacing, diameter } => vec![mm(spacing), mm(diameter)],
            Self::Honeycomb { cell_size, wall } => vec![mm(cell_size), mm(wall)],
            Self::BrickWall {
                width,
                height,
                joint,
            } => vec![mm(width), mm(height), mm(joint)],
            Self::LeafForest {
                leaf_size,
                size_jitter,
                rotation_jitter,
                density,
            } => vec![
                mm(leaf_size),
                size_jitter.get(),
                degrees(rotation_jitter),
                density.get(),
            ],
            Self::SeaWaves {
                radius,
                rings,
                ring_spacing,
            } => vec![mm(radius), f64::from(rings.get()), mm(ring_spacing)],
            Self::Waves {
                wavelength,
                amplitude,
                spacing,
                angle,
            } => vec![mm(wavelength), mm(amplitude), mm(spacing), degrees(angle)],
            Self::InterpolationWaves { lines } => vec![f64::from(lines.get())],
        }
    }

    /// The value of `param`, if this generator has it.
    #[must_use]
    pub fn get(&self, param: PatternParam) -> Option<f64> {
        let at = specs(self.kind()).iter().position(|s| s.param == param)?;
        self.values().get(at).copied()
    }

    /// This generator with `param` set to `value` (stored units).
    ///
    /// # Errors
    /// [`PatternParamError::NotAParameter`] when the generator lacks it, or the range errors of
    /// [`PatternGenerator::from_values`].
    pub fn with(&self, param: PatternParam, value: f64) -> Result<Self, PatternParamError> {
        let kind = self.kind();
        let at = specs(kind)
            .iter()
            .position(|s| s.param == param)
            .ok_or(PatternParamError::NotAParameter)?;
        let mut values = self.values();
        values[at] = value;
        Self::from_values(kind, &values)
    }

    /// The generator `kind` from its values in the table's order, each checked against its
    /// kind, minimum and maximum, and Joint and Wall against their cell.
    ///
    /// # Errors
    /// [`PatternParamError::OutOfRange`] or [`PatternParamError::WiderThanCell`].
    pub fn from_values(kind: GeneratorKind, values: &[f64]) -> Result<Self, PatternParamError> {
        let table = specs(kind);
        if values.len() != table.len() {
            return Err(PatternParamError::OutOfRange);
        }
        for (spec, value) in table.iter().zip(values) {
            if !spec.accepts(*value) {
                return Err(PatternParamError::OutOfRange);
            }
        }
        Self::build(kind, values)
    }

    /// The generator `kind` from values already checked against the table.
    fn build(kind: GeneratorKind, v: &[f64]) -> Result<Self, PatternParamError> {
        Ok(match kind {
            GeneratorKind::Hatch => Self::Hatch {
                spacing: mm(v[0]),
                angle: radians(v[1]),
            },
            GeneratorKind::Crosshatch => Self::Crosshatch {
                spacing: mm(v[0]),
                angle: radians(v[1]),
                second_angle: radians(v[2]),
            },
            GeneratorKind::Grid => Self::Grid {
                spacing: mm(v[0]),
                angle: radians(v[1]),
            },
            GeneratorKind::Dots => Self::Dots {
                spacing: mm(v[0]),
                diameter: mm(v[1]),
            },
            GeneratorKind::Honeycomb => honeycomb(v)?,
            GeneratorKind::BrickWall => brick_wall(v)?,
            GeneratorKind::LeafForest => Self::LeafForest {
                leaf_size: mm(v[0]),
                size_jitter: fraction(v[1])?,
                rotation_jitter: radians(v[2]),
                density: fraction(v[3])?,
            },
            GeneratorKind::SeaWaves => Self::SeaWaves {
                radius: mm(v[0]),
                rings: count(v[1])?,
                ring_spacing: mm(v[2]),
            },
            GeneratorKind::Waves => Self::Waves {
                wavelength: mm(v[0]),
                amplitude: mm(v[1]),
                spacing: mm(v[2]),
                angle: radians(v[3]),
            },
            GeneratorKind::InterpolationWaves => Self::InterpolationWaves {
                lines: count(v[0])?,
            },
        })
    }

    /// The generator `kind` taking over the values of this one where `kind` has the same
    /// parameter of the same kind (Spacing, Angle), the others at `kind`'s defaults (UX notes
    /// §5). A carried value that breaks a cross-field rule falls back to the defaults.
    #[must_use]
    pub fn switched_to(&self, kind: GeneratorKind) -> Self {
        let carried: Vec<f64> = specs(kind)
            .iter()
            .map(|spec| {
                self.get(spec.param)
                    .filter(|value| {
                        spec_of(self.kind(), spec.param).is_some_and(|own| own.kind == spec.kind)
                            && spec.accepts(*value)
                            && spec.kind != ParamKind::Count
                    })
                    .unwrap_or(spec.default)
            })
            .collect();
        Self::from_values(kind, &carried).unwrap_or_else(|_| Self::defaults(kind))
    }
}
