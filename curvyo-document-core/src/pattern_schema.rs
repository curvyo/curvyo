//! The parameter table of the pattern generators (ADR 0029 §2): per generator, its parameters in
//! the panel's order with their stored name, label, kind, frozen default and range. The codec and
//! the panel both read it; the defaults are part of the format.

use crate::pattern_model::{GeneratorKind, PatternParam};

/// What kind of value a parameter is, which fixes its stored unit and its panel scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// A length in millimetres.
    Length,
    /// An angle in degrees, any finite value.
    Angle,
    /// A share in `[0, 1]`, shown as percent.
    Percent,
    /// A whole number.
    Count,
    /// An angle in degrees from 0 to 180 (rotation jitter).
    Jitter,
}

/// One parameter of one generator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamSpec {
    /// Which parameter.
    pub param: PatternParam,
    /// The stored name in `fill_pattern.params`.
    pub name: &'static str,
    /// The panel's label ("Spacing").
    pub label: &'static str,
    /// Its kind.
    pub kind: ParamKind,
    /// The frozen default, stored units.
    pub default: f64,
    /// The smallest value.
    pub min: f64,
    /// The largest value.
    pub max: f64,
}

impl ParamSpec {
    /// Whether `value` is a valid stored value: finite, in range, whole for a count.
    #[must_use]
    pub fn accepts(&self, value: f64) -> bool {
        value.is_finite()
            && value >= self.min
            && value <= self.max
            && (self.kind != ParamKind::Count || value.fract() == 0.0)
    }
}

/// The smallest spacing-like length, mm (UX notes §5: typed 0.05 to 1000).
pub const MIN_LENGTH_MM: f64 = 0.05;

const fn length(
    param: PatternParam,
    name: &'static str,
    label: &'static str,
    default: f64,
) -> ParamSpec {
    ParamSpec {
        param,
        name,
        label,
        kind: ParamKind::Length,
        default,
        min: MIN_LENGTH_MM,
        max: f64::MAX,
    }
}

const fn angle(
    param: PatternParam,
    name: &'static str,
    label: &'static str,
    default: f64,
) -> ParamSpec {
    ParamSpec {
        param,
        name,
        label,
        kind: ParamKind::Angle,
        default,
        min: f64::MIN,
        max: f64::MAX,
    }
}

const fn percent(
    param: PatternParam,
    name: &'static str,
    label: &'static str,
    default: f64,
) -> ParamSpec {
    ParamSpec {
        param,
        name,
        label,
        kind: ParamKind::Percent,
        default,
        min: 0.0,
        max: 1.0,
    }
}

const fn count(
    param: PatternParam,
    name: &'static str,
    label: &'static str,
    default: f64,
    min: f64,
) -> ParamSpec {
    ParamSpec {
        param,
        name,
        label,
        kind: ParamKind::Count,
        default,
        min,
        max: 1_000_000.0,
    }
}

use PatternParam as P;

const SPACING_2: ParamSpec = length(P::Spacing, "spacing", "Spacing", 2.0);
const ANGLE_45: ParamSpec = angle(P::Angle, "angle", "Angle", 45.0);

const HATCH: &[ParamSpec] = &[SPACING_2, ANGLE_45];
const CROSSHATCH: &[ParamSpec] = &[
    SPACING_2,
    ANGLE_45,
    angle(P::SecondAngle, "second_angle", "Second angle", 135.0),
];
const GRID: &[ParamSpec] = &[
    length(P::Spacing, "spacing", "Spacing", 5.0),
    angle(P::Angle, "angle", "Angle", 0.0),
];
const DOTS: &[ParamSpec] = &[
    length(P::Spacing, "spacing", "Spacing", 3.0),
    length(P::Diameter, "diameter", "Diameter", 1.0),
];
const HONEYCOMB: &[ParamSpec] = &[
    length(P::CellSize, "cell_size", "Cell size", 6.0),
    length(P::Wall, "wall", "Wall", 0.5),
];
const BRICK_WALL: &[ParamSpec] = &[
    length(P::BrickWidth, "width", "Width", 20.0),
    length(P::BrickHeight, "height", "Height", 10.0),
    length(P::Joint, "joint", "Joint", 1.0),
];
const LEAF_FOREST: &[ParamSpec] = &[
    length(P::LeafSize, "leaf_size", "Leaf size", 6.0),
    percent(P::SizeJitter, "size_jitter", "Size jitter", 0.3),
    ParamSpec {
        param: P::RotationJitter,
        name: "rotation_jitter",
        label: "Rotation jitter",
        kind: ParamKind::Jitter,
        default: 180.0,
        min: 0.0,
        max: 180.0,
    },
    percent(P::Density, "density", "Density", 0.5),
];
const SEA_WAVES: &[ParamSpec] = &[
    length(P::ScaleRadius, "radius", "Scale radius", 5.0),
    count(P::Rings, "rings", "Rings", 3.0, 1.0),
    length(P::RingSpacing, "ring_spacing", "Ring spacing", 1.0),
];
const WAVES: &[ParamSpec] = &[
    length(P::Wavelength, "wavelength", "Wavelength", 10.0),
    ParamSpec {
        min: 0.0,
        ..length(P::Amplitude, "amplitude", "Amplitude", 2.0)
    },
    length(P::Spacing, "spacing", "Spacing", 2.0),
    angle(P::Angle, "angle", "Angle", 0.0),
];
const INTERPOLATION_WAVES: &[ParamSpec] = &[count(P::Lines, "lines", "Lines", 12.0, 2.0)];

/// The parameters of `kind` in the panel's order.
#[must_use]
pub const fn specs(kind: GeneratorKind) -> &'static [ParamSpec] {
    match kind {
        GeneratorKind::Hatch => HATCH,
        GeneratorKind::Crosshatch => CROSSHATCH,
        GeneratorKind::Grid => GRID,
        GeneratorKind::Dots => DOTS,
        GeneratorKind::Honeycomb => HONEYCOMB,
        GeneratorKind::BrickWall => BRICK_WALL,
        GeneratorKind::LeafForest => LEAF_FOREST,
        GeneratorKind::SeaWaves => SEA_WAVES,
        GeneratorKind::Waves => WAVES,
        GeneratorKind::InterpolationWaves => INTERPOLATION_WAVES,
    }
}

/// The spec of `param` in `kind`, if `kind` has it.
#[must_use]
pub fn spec_of(kind: GeneratorKind, param: PatternParam) -> Option<&'static ParamSpec> {
    specs(kind).iter().find(|spec| spec.param == param)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_is_accepted_and_names_are_unique_per_generator() {
        for kind in GeneratorKind::ALL {
            let table = specs(kind);
            assert_ne!(table.len(), 0);
            for spec in table {
                assert!(spec.accepts(spec.default), "{kind:?} {}", spec.name);
                let same = table.iter().filter(|s| s.name == spec.name).count();
                assert_eq!(same, 1, "{kind:?} {}", spec.name);
            }
        }
    }
}
