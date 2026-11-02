//! The typed model of a pattern fill (`specs/0071-pattern-fill`, ADR 0029 §2): which generator,
//! its parameters as newtypes, the margin, Lines or Shapes, and the seed. No Loro type appears
//! here; the stored form is `pattern_codec`, the parameter table `pattern_schema`.

use serde::{Deserialize, Serialize};

use crate::path_model::NodeId;
use crate::pattern_schema::specs;
use crate::units::{Angle, Length};

/// What a fill paints when it is on: a solid colour or a pattern (`fill_paint`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FillPaint {
    /// The solid colour (the panel's "Solid").
    #[default]
    Colour,
    /// The generated pattern.
    Pattern,
}

/// How the pieces are painted: stroked open lines or filled closed outlines (criterion 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternMode {
    /// Pieces drawn with the shape's stroke (engraving, plotting).
    #[default]
    Lines,
    /// Closed pieces filled with the shape's fill colour (cutting a perforation).
    Shapes,
}

/// The seed of a seeded generator: stored, so reopening does not reshuffle (criterion 17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PatternSeed(u32);

impl PatternSeed {
    /// A seed with this value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// The value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// The fresh seed of a new pattern on `node`: `SplitMix64` of its peer and counter, so the
    /// same shape always gets the same fresh seed and no clock or random source is read.
    #[must_use]
    pub fn fresh(node: NodeId) -> Self {
        #[allow(clippy::cast_sign_loss)] // the counter's bits, not its value
        let counter = u64::from(node.counter as u32);
        Self::low_bits(splitmix64(node.peer ^ splitmix64(counter)))
    }

    /// The seed Re-roll gives: `SplitMix64` of this one.
    #[must_use]
    pub fn next(self) -> Self {
        Self::low_bits(splitmix64(u64::from(self.0)))
    }

    #[allow(clippy::cast_possible_truncation)] // the low 32 bits are the seed
    const fn low_bits(value: u64) -> Self {
        Self(value as u32)
    }
}

/// One step of `SplitMix64` (Steele, Lea, Flood 2014) on `state`: the mixing function of the
/// seeds and of the generators' random stream. Its constants are part of the format.
#[must_use]
pub const fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A whole number of lines or rings, at least 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternCount(u32);

impl PatternCount {
    /// The count, if it is at least 1.
    #[must_use]
    pub const fn new(count: u32) -> Option<Self> {
        if count >= 1 { Some(Self(count)) } else { None }
    }

    /// The count.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A share in `[0, 1]` (a jitter or a density; the panel shows percent).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PatternFraction(f64);

impl PatternFraction {
    /// The share, if it is finite and in `[0, 1]`.
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        (value.is_finite() && (0.0..=1.0).contains(&value)).then_some(Self(value))
    }

    /// The share.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// The ten generators, in the picker's order (criterion 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeneratorKind {
    /// Parallel lines.
    Hatch,
    /// Hatch at two angles.
    Crosshatch,
    /// Two perpendicular line sets.
    Grid,
    /// Round dots on a staggered lattice.
    Dots,
    /// Regular hexagons.
    Honeycomb,
    /// Running bond bricks.
    BrickWall,
    /// Scattered, non-overlapping leaves.
    LeafForest,
    /// Seigaiha: concentric half circles in offset rows.
    SeaWaves,
    /// Parallel sine lines.
    Waves,
    /// Lines blending from the top to the bottom boundary.
    InterpolationWaves,
}

impl GeneratorKind {
    /// Every generator in the picker's order.
    pub const ALL: [Self; 10] = [
        Self::Hatch,
        Self::Crosshatch,
        Self::Grid,
        Self::Dots,
        Self::Honeycomb,
        Self::BrickWall,
        Self::LeafForest,
        Self::SeaWaves,
        Self::Waves,
        Self::InterpolationWaves,
    ];

    /// The stored id (`fill_pattern.generator`), part of the format.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Hatch => "hatch",
            Self::Crosshatch => "crosshatch",
            Self::Grid => "grid",
            Self::Dots => "dots",
            Self::Honeycomb => "honeycomb",
            Self::BrickWall => "brick_wall",
            Self::LeafForest => "leaf_forest",
            Self::SeaWaves => "sea_waves",
            Self::Waves => "waves",
            Self::InterpolationWaves => "interpolation_waves",
        }
    }

    /// The generator with this stored id.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }

    /// The name the picker shows ("Brick wall").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Hatch => "Hatch",
            Self::Crosshatch => "Crosshatch",
            Self::Grid => "Grid",
            Self::Dots => "Dots",
            Self::Honeycomb => "Honeycomb",
            Self::BrickWall => "Brick wall",
            Self::LeafForest => "Leaf forest",
            Self::SeaWaves => "Sea waves",
            Self::Waves => "Waves",
            Self::InterpolationWaves => "Interpolation waves",
        }
    }

    /// The commit label of choosing this generator (history name "Pattern: Honeycomb" and so on).
    #[must_use]
    pub const fn commit_label(self) -> &'static str {
        match self {
            Self::Hatch => "pattern_generator_hatch",
            Self::Crosshatch => "pattern_generator_crosshatch",
            Self::Grid => "pattern_generator_grid",
            Self::Dots => "pattern_generator_dots",
            Self::Honeycomb => "pattern_generator_honeycomb",
            Self::BrickWall => "pattern_generator_brick_wall",
            Self::LeafForest => "pattern_generator_leaf_forest",
            Self::SeaWaves => "pattern_generator_sea_waves",
            Self::Waves => "pattern_generator_waves",
            Self::InterpolationWaves => "pattern_generator_interpolation_waves",
        }
    }

    /// Whether the seed changes the pieces (criterion 17): Leaf forest only.
    #[must_use]
    pub const fn is_seeded(self) -> bool {
        matches!(self, Self::LeafForest)
    }

    /// Whether "Whole pieces only" applies (criterion 29): the generator's pieces are shapes of
    /// their own (a dot, a cell, a brick, a leaf, a scale of rings), not lines across the region.
    #[must_use]
    pub const fn has_whole_pieces(self) -> bool {
        matches!(
            self,
            Self::Dots | Self::Honeycomb | Self::BrickWall | Self::LeafForest | Self::SeaWaves
        )
    }
}

/// One parameter of a generator; which generator has which is `pattern_schema`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternParam {
    /// Line, dot or wave spacing, mm.
    Spacing,
    /// The angle of the lines, degrees.
    Angle,
    /// Crosshatch's second angle, degrees.
    SecondAngle,
    /// Dot diameter, mm.
    Diameter,
    /// Honeycomb's distance between cell centres, mm.
    CellSize,
    /// Honeycomb's wall width, mm.
    Wall,
    /// Brick pitch along a row, mm.
    BrickWidth,
    /// Brick pitch between rows, mm.
    BrickHeight,
    /// The joint between bricks, mm.
    Joint,
    /// Leaf length, mm.
    LeafSize,
    /// Leaf size jitter, a share.
    SizeJitter,
    /// Leaf rotation jitter, degrees.
    RotationJitter,
    /// The share of the area the leaves aim to cover.
    Density,
    /// Sea waves' outer radius, mm.
    ScaleRadius,
    /// Sea waves' rings per scale.
    Rings,
    /// Sea waves' distance between rings, mm.
    RingSpacing,
    /// Waves' wavelength, mm.
    Wavelength,
    /// Waves' amplitude, mm.
    Amplitude,
    /// Interpolation waves' line count.
    Lines,
}

impl PatternParam {
    /// The commit label of an edit of this parameter (history name "Pattern spacing" and so on).
    #[must_use]
    pub const fn commit_label(self) -> &'static str {
        match self {
            Self::Spacing => "pattern_spacing",
            Self::Angle => "pattern_angle",
            Self::SecondAngle => "pattern_second_angle",
            Self::Diameter => "pattern_diameter",
            Self::CellSize => "pattern_cell_size",
            Self::Wall => "pattern_wall",
            Self::BrickWidth => "pattern_width",
            Self::BrickHeight => "pattern_height",
            Self::Joint => "pattern_joint",
            Self::LeafSize => "pattern_leaf_size",
            Self::SizeJitter => "pattern_size_jitter",
            Self::RotationJitter => "pattern_rotation_jitter",
            Self::Density => "pattern_density",
            Self::ScaleRadius => "pattern_scale_radius",
            Self::Rings => "pattern_rings",
            Self::RingSpacing => "pattern_ring_spacing",
            Self::Wavelength => "pattern_wavelength",
            Self::Amplitude => "pattern_amplitude",
            Self::Lines => "pattern_lines_count",
        }
    }
}

/// Why a parameter value was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PatternParamError {
    /// The generator has no such parameter.
    #[error("the generator has no such parameter")]
    NotAParameter,
    /// Not finite, below the minimum, above the maximum, or not a whole number for a count.
    #[error("the value is out of range")]
    OutOfRange,
    /// Joint or Wall is not smaller than the cell it separates.
    #[error("the value must be smaller than the cell")]
    WiderThanCell,
}

/// A generator with its typed parameters (ADR 0029 §2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "generator", rename_all = "snake_case")]
#[allow(missing_docs)] // the fields are named by `PatternParam`
pub enum PatternGenerator {
    Hatch {
        spacing: Length,
        angle: Angle,
    },
    Crosshatch {
        spacing: Length,
        angle: Angle,
        second_angle: Angle,
    },
    Grid {
        spacing: Length,
        angle: Angle,
    },
    Dots {
        spacing: Length,
        diameter: Length,
    },
    Honeycomb {
        cell_size: Length,
        wall: Length,
    },
    BrickWall {
        width: Length,
        height: Length,
        joint: Length,
    },
    LeafForest {
        leaf_size: Length,
        size_jitter: PatternFraction,
        rotation_jitter: Angle,
        density: PatternFraction,
    },
    SeaWaves {
        radius: Length,
        rings: PatternCount,
        ring_spacing: Length,
    },
    Waves {
        wavelength: Length,
        amplitude: Length,
        spacing: Length,
        angle: Angle,
    },
    InterpolationWaves {
        lines: PatternCount,
    },
}

impl PatternGenerator {
    /// The generator's kind.
    #[must_use]
    pub const fn kind(&self) -> GeneratorKind {
        match self {
            Self::Hatch { .. } => GeneratorKind::Hatch,
            Self::Crosshatch { .. } => GeneratorKind::Crosshatch,
            Self::Grid { .. } => GeneratorKind::Grid,
            Self::Dots { .. } => GeneratorKind::Dots,
            Self::Honeycomb { .. } => GeneratorKind::Honeycomb,
            Self::BrickWall { .. } => GeneratorKind::BrickWall,
            Self::LeafForest { .. } => GeneratorKind::LeafForest,
            Self::SeaWaves { .. } => GeneratorKind::SeaWaves,
            Self::Waves { .. } => GeneratorKind::Waves,
            Self::InterpolationWaves { .. } => GeneratorKind::InterpolationWaves,
        }
    }

    /// The generator `kind` at its frozen defaults.
    #[must_use]
    pub fn defaults(kind: GeneratorKind) -> Self {
        let values: Vec<f64> = specs(kind).iter().map(|spec| spec.default).collect();
        // invariant: the frozen defaults satisfy their own table (tested for every kind); the
        // fallback is never taken.
        Self::from_values(kind, &values).unwrap_or(Self::InterpolationWaves {
            lines: PatternCount(12),
        })
    }
}

/// A shape's pattern fill: generator, margin, mode, seed and whole pieces (ADR 0029 §1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PatternFill {
    /// The generator and its parameters.
    pub generator: PatternGenerator,
    /// No piece comes closer than this to the outline or a hole edge (criterion 14).
    pub margin: Length,
    /// Lines or Shapes.
    pub mode: PatternMode,
    /// The seed of a seeded generator; kept for the others.
    pub seed: PatternSeed,
    /// Only pieces the region and margin leave whole are drawn, cut ones are left out
    /// (criterion 29); kept, and without effect, for a generator without whole pieces.
    pub whole_pieces: bool,
}

impl PatternFill {
    /// A new pattern on `node`: Hatch at its defaults, no margin, Lines, the node's fresh seed.
    #[must_use]
    pub fn fresh(node: NodeId) -> Self {
        Self {
            seed: PatternSeed::fresh(node),
            ..Self::default()
        }
    }
}

impl Default for PatternFill {
    /// Hatch at its defaults, no margin, Lines, seed 0, cut pieces drawn: what a paint of Pattern without a stored
    /// pattern reads as (only a merged document can hold one).
    fn default() -> Self {
        Self {
            generator: PatternGenerator::defaults(GeneratorKind::Hatch),
            margin: Length::from_mm(0.0),
            mode: PatternMode::Lines,
            seed: PatternSeed::new(0),
            whole_pieces: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_generator_round_trips_through_its_values() {
        for kind in GeneratorKind::ALL {
            let generator = PatternGenerator::defaults(kind);
            assert_eq!(generator.kind(), kind);
            assert_eq!(GeneratorKind::from_id(kind.id()), Some(kind));
            let again = PatternGenerator::from_values(kind, &generator.values()).unwrap();
            assert_eq!(again, generator, "{kind:?}");
        }
    }

    #[test]
    fn a_value_below_its_minimum_or_across_its_cell_is_refused() {
        let hatch = PatternGenerator::defaults(GeneratorKind::Hatch);
        assert_eq!(
            hatch.with(PatternParam::Spacing, 0.0),
            Err(PatternParamError::OutOfRange)
        );
        assert_eq!(
            hatch.with(PatternParam::Diameter, 1.0),
            Err(PatternParamError::NotAParameter)
        );
        let bricks = PatternGenerator::defaults(GeneratorKind::BrickWall);
        assert_eq!(
            bricks.with(PatternParam::Joint, 10.0),
            Err(PatternParamError::WiderThanCell)
        );
        let comb = PatternGenerator::defaults(GeneratorKind::Honeycomb);
        assert_eq!(
            comb.with(PatternParam::Wall, 6.0),
            Err(PatternParamError::WiderThanCell)
        );
        let waves = PatternGenerator::defaults(GeneratorKind::Waves);
        assert!(waves.with(PatternParam::Amplitude, 0.0).is_ok());
        let lines = PatternGenerator::defaults(GeneratorKind::InterpolationWaves);
        assert_eq!(
            lines.with(PatternParam::Lines, 2.5),
            Err(PatternParamError::OutOfRange)
        );
    }

    #[test]
    fn switching_carries_spacing_and_angle_and_resets_the_rest() {
        let hatch = PatternGenerator::defaults(GeneratorKind::Hatch)
            .with(PatternParam::Spacing, 3.5)
            .unwrap()
            .with(PatternParam::Angle, 30.0)
            .unwrap();
        let grid = hatch.switched_to(GeneratorKind::Grid);
        assert_eq!(grid.get(PatternParam::Spacing), Some(3.5));
        assert_eq!(grid.get(PatternParam::Angle), Some(30.0));
        let dots = hatch.switched_to(GeneratorKind::Dots);
        assert_eq!(dots.get(PatternParam::Spacing), Some(3.5));
        assert_eq!(dots.get(PatternParam::Diameter), Some(1.0));
        let comb = hatch.switched_to(GeneratorKind::Honeycomb);
        assert_eq!(comb, PatternGenerator::defaults(GeneratorKind::Honeycomb));
    }

    #[test]
    fn a_fresh_seed_is_stable_per_node_and_differs_between_nodes() {
        let a = NodeId::from_parts(7, 1);
        let b = NodeId::from_parts(7, 2);
        assert_eq!(PatternSeed::fresh(a), PatternSeed::fresh(a));
        assert_ne!(PatternSeed::fresh(a), PatternSeed::fresh(b));
        let seed = PatternSeed::fresh(a);
        assert_ne!(seed.next(), seed);
        assert_eq!(seed.next(), seed.next());
    }

    #[test]
    fn whole_pieces_apply_to_the_generators_of_shapes_only() {
        let with: Vec<GeneratorKind> = GeneratorKind::ALL
            .into_iter()
            .filter(|kind| kind.has_whole_pieces())
            .collect();
        assert_eq!(
            with,
            [
                GeneratorKind::Dots,
                GeneratorKind::Honeycomb,
                GeneratorKind::BrickWall,
                GeneratorKind::LeafForest,
                GeneratorKind::SeaWaves
            ]
        );
        assert!(!PatternFill::default().whole_pieces);
    }

    #[test]
    fn splitmix_is_the_reference_algorithm() {
        // The first output of the reference SplitMix64 stream seeded with 0.
        assert_eq!(splitmix64(0), 0xE220_A839_7B1D_CDAF);
    }
}
