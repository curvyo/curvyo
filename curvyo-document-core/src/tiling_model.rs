//! The values of a stored tiling (ADR 0030 §1 and §3): the recipe, the cell key and the per-cell
//! placement and style shift that the caller computes and the tiling command writes.
//!
//! The document never computes a layout (ADR 0011): `curvyo-ui-core`'s `tiling_layout` turns a
//! recipe and the source box into [`TilingCell`]s, and [`crate::Document::apply_tiling`] writes
//! them.

use thiserror::Error;

use crate::affine::Affine;
use crate::path_model::{Color, NodeId};
use crate::units::{Angle, Length, Vec2};

/// The commit label of a new tiling (history name "Tile", `specs/0069-tiling` criterion 37).
pub const TILE_COMMIT_LABEL: &str = "tile";
/// The commit label of a re-applied tiling (history name "Edit tiling").
pub const EDIT_TILING_COMMIT_LABEL: &str = "edit_tiling";
/// The commit label of Release tiling (history name "Release tiling").
pub const RELEASE_TILING_COMMIT_LABEL: &str = "release_tiling";

/// Above this many instances the bar warns and the preview draws boxes only (criterion 36).
pub const TILING_WARN_COUNT: u32 = 10_000;
/// More instances than this are refused (criterion 36, ADR 0030 §4).
pub const TILING_MAX_COUNT: u32 = 100_000;
/// The name of a tiling group (TILE-RESULT-GROUP A).
pub const TILING_GROUP_NAME: &str = "Tiling";
/// Placements and copy geometry closer than this to the stored value are not rewritten
/// (ADR 0030 §3; a peer whose `sin` differs in the last bit writes nothing), and a tiling
/// instance whose source change is rigid within it counts as held (ADR 0030 §2).
pub(crate) const TILING_WRITE_TOLERANCE_MM: f64 = 1e-9;

/// A percentage (scale variance in percent, opacity variance in percentage points): a newtype,
/// so it is never confused with a length or an angle (`CLAUDE.md` §5).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Percent(f64);

impl Percent {
    /// The percentage `value` (50 is a half).
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    /// The plain percentage.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Which of the two layouts a tiling uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TilingLayout {
    /// Columns and rows.
    Grid,
    /// Instances around a centre.
    Circle,
}

/// What the grid spacing means (TILE-SPACING A).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacingMode {
    /// Edge to edge of the cell boxes; 0 makes them touch.
    Gap,
    /// Centre to centre.
    Pitch,
}

/// Which grid lines are shifted by half a cell (TILE-EXTRAS A).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetLines {
    /// No line is shifted.
    None,
    /// Rows 1, 3, ... shift to the right.
    Rows,
    /// Columns 1, 3, ... shift downward.
    Columns,
}

/// Whether the instances are linked clones or independent copies (TILE-RESULT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TilingResult {
    /// Linked clones (images are copies anyway).
    Clones,
    /// Independent copies.
    Copies,
}

/// The grid options (criteria 6 to 10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridOptions {
    /// Columns, 1 or more.
    pub columns: u32,
    /// Rows, 1 or more.
    pub rows: u32,
    /// The X spacing (a gap or a pitch, by `spacing_mode`).
    pub spacing_x: Length,
    /// The Y spacing.
    pub spacing_y: Length,
    /// What the spacing means.
    pub spacing_mode: SpacingMode,
    /// Rows 1, 3, ... are flipped about their own horizontal centre line.
    pub mirror_rows: bool,
    /// Columns 1, 3, ... are flipped about their own vertical centre line.
    pub mirror_columns: bool,
    /// Which lines are shifted.
    pub offset: OffsetLines,
    /// The shift, in percent of the pitch.
    pub shift: Percent,
}

/// The circle options (criteria 11 to 14).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircleOptions {
    /// Instances, 1 or more.
    pub count: u32,
    /// The centre, relative to the cell box centre (criterion 14).
    pub centre: Vec2,
    /// The radius, more than 0.
    pub radius: Length,
    /// The angle of instance 0, counterclockwise on screen, 0 pointing right.
    pub start: Angle,
    /// The swept angle, -360 to 360 degrees; negative runs clockwise.
    pub sweep: Angle,
    /// Each instance turns with its angle on the circle.
    pub face_centre: bool,
}

/// One variance channel (criterion 16): a random range and the steps per row and column, in the
/// channel's unit. The circle uses `per_column` as its per-step value.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Channel<U> {
    /// The random range: a draw lands in `-random .. random`. 0 or more.
    pub random: U,
    /// Added once per row index.
    pub per_row: U,
    /// Added once per column index (circle: per instance index).
    pub per_column: U,
}

/// The variance of every channel (criterion 15); all zero by default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Variance {
    /// Rotation about the instance's box centre, counterclockwise on screen.
    pub rotation: Channel<Angle>,
    /// Scale X in percent (the factor is `1 + v / 100`).
    pub scale_x: Channel<Percent>,
    /// Scale Y in percent; unused while `uniform_scale` is on.
    pub scale_y: Channel<Percent>,
    /// Offset X.
    pub offset_x: Channel<Length>,
    /// Offset Y.
    pub offset_y: Channel<Length>,
    /// Opacity in percentage points of the object's opacity.
    pub opacity: Channel<Percent>,
    /// Hue shift of the fill and stroke colours.
    pub hue: Channel<Angle>,
    /// Scale Y follows scale X (uniform scale only in this version).
    pub uniform_scale: bool,
}

impl Default for Variance {
    fn default() -> Self {
        let zero_angle = Channel {
            random: Angle::from_radians(0.0),
            per_row: Angle::from_radians(0.0),
            per_column: Angle::from_radians(0.0),
        };
        let zero_length = Channel {
            random: Length::from_mm(0.0),
            per_row: Length::from_mm(0.0),
            per_column: Length::from_mm(0.0),
        };
        Self {
            rotation: zero_angle,
            scale_x: Channel::default(),
            scale_y: Channel::default(),
            offset_x: zero_length,
            offset_y: zero_length,
            opacity: Channel::default(),
            hue: zero_angle,
            uniform_scale: true,
        }
    }
}

impl Variance {
    /// Whether the opacity or hue variance is on, so instances get their own style (ADR 0030 §2).
    #[must_use]
    pub fn varies_style(&self) -> bool {
        let percent = |c: &Channel<Percent>| {
            c.random.get() != 0.0 || c.per_row.get() != 0.0 || c.per_column.get() != 0.0
        };
        let angle = |c: &Channel<Angle>| {
            c.random.as_radians() != 0.0
                || c.per_row.as_radians() != 0.0
                || c.per_column.as_radians() != 0.0
        };
        percent(&self.opacity) || angle(&self.hue)
    }
}

/// The whole recipe of a tiling, one atomic register (ADR 0030 §1). Both layouts keep their
/// values, so switching between them loses nothing (criterion 27).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TilingRecipe {
    /// The layout in use.
    pub layout: TilingLayout,
    /// The grid options.
    pub grid: GridOptions,
    /// The circle options.
    pub circle: CircleOptions,
    /// The variance.
    pub variance: Variance,
    /// The generator seed (criterion 19).
    pub seed: u32,
    /// Clones or copies.
    pub result: TilingResult,
    /// The source stands for cell (0, 0) inside the group.
    pub keep_source: bool,
}

/// Why a recipe cannot be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RecipeError {
    /// A count is 0.
    #[error("the count must be 1 or more")]
    CountZero,
    /// More instances than [`TILING_MAX_COUNT`].
    #[error("too many instances, {TILING_MAX_COUNT} at most")]
    TooMany,
    /// A number is not finite.
    #[error("a number is not finite")]
    NotFinite,
    /// A pitch, a radius or a random range is out of its range.
    #[error("a value is out of its range")]
    OutOfRange,
}

impl TilingRecipe {
    /// The number of instances of the layout in use, the kept source counted (criterion 36).
    #[must_use]
    pub fn instance_count(&self) -> u64 {
        match self.layout {
            TilingLayout::Grid => u64::from(self.grid.columns) * u64::from(self.grid.rows),
            TilingLayout::Circle => u64::from(self.circle.count),
        }
    }

    /// Whether `key` is a cell of the layout in use (grid: row and column in range; circle: row
    /// 0 and the index in range).
    pub(crate) fn has_cell(&self, key: CellKey) -> bool {
        match self.layout {
            TilingLayout::Grid => key.row < self.grid.rows && key.column < self.grid.columns,
            TilingLayout::Circle => key.row == 0 && key.column < self.circle.count,
        }
    }

    /// Checks the stored-shape rules of ADR 0030 §10 for both layouts: counts from 1 to the
    /// limit and finite numbers. Used at open (damage) and before every apply.
    ///
    /// # Errors
    /// [`RecipeError::CountZero`], [`RecipeError::TooMany`] or [`RecipeError::NotFinite`].
    pub fn check_stored(&self) -> Result<(), RecipeError> {
        let grid_count = u64::from(self.grid.columns) * u64::from(self.grid.rows);
        if self.grid.columns == 0 || self.grid.rows == 0 || self.circle.count == 0 {
            return Err(RecipeError::CountZero);
        }
        if grid_count > u64::from(TILING_MAX_COUNT) || self.circle.count > TILING_MAX_COUNT {
            return Err(RecipeError::TooMany);
        }
        if self.numbers().iter().all(|n| n.is_finite()) {
            Ok(())
        } else {
            Err(RecipeError::NotFinite)
        }
    }

    /// [`TilingRecipe::check_stored`] plus the field ranges of the bar: a pitch and a radius
    /// more than 0, a sweep within ±360°, a shift within 0 to 100 %, random ranges 0 or more.
    ///
    /// # Errors
    /// As [`TilingRecipe::check_stored`], and [`RecipeError::OutOfRange`].
    pub fn check(&self) -> Result<(), RecipeError> {
        self.check_stored()?;
        let v = &self.variance;
        let pitch_ok = self.grid.spacing_mode == SpacingMode::Gap
            || (self.grid.spacing_x.as_mm() > 0.0 && self.grid.spacing_y.as_mm() > 0.0);
        let sweep_limit = std::f64::consts::TAU + 1e-12;
        let ranges_ok = v.rotation.random.as_radians() >= 0.0
            && v.scale_x.random.get() >= 0.0
            && v.scale_y.random.get() >= 0.0
            && v.offset_x.random.as_mm() >= 0.0
            && v.offset_y.random.as_mm() >= 0.0
            && v.opacity.random.get() >= 0.0
            && v.hue.random.as_radians() >= 0.0;
        if pitch_ok
            && self.circle.radius.as_mm() > 0.0
            && self.circle.sweep.as_radians().abs() <= sweep_limit
            && (0.0..=100.0).contains(&self.grid.shift.get())
            && ranges_ok
        {
            Ok(())
        } else {
            Err(RecipeError::OutOfRange)
        }
    }

    fn numbers(&self) -> Vec<f64> {
        let g = &self.grid;
        let c = &self.circle;
        let v = &self.variance;
        let mut all = vec![
            g.spacing_x.as_mm(),
            g.spacing_y.as_mm(),
            g.shift.get(),
            c.centre.x,
            c.centre.y,
            c.radius.as_mm(),
            c.start.as_radians(),
            c.sweep.as_radians(),
        ];
        for ch in [v.rotation, v.hue] {
            all.extend([ch.random, ch.per_row, ch.per_column].map(Angle::as_radians));
        }
        for ch in [v.scale_x, v.scale_y, v.opacity] {
            all.extend([ch.random, ch.per_row, ch.per_column].map(Percent::get));
        }
        for ch in [v.offset_x, v.offset_y] {
            all.extend([ch.random, ch.per_row, ch.per_column].map(Length::as_mm));
        }
        all
    }
}

/// The identity of an instance (ADR 0030 §2): grid row and column; a circle uses row 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellKey {
    /// The row (0 for a circle).
    pub row: u32,
    /// The column (the instance index for a circle).
    pub column: u32,
}

impl CellKey {
    /// The cell of the kept source.
    pub const ORIGIN: Self = Self { row: 0, column: 0 };

    /// The cell's position in the generator stream, `row · 2^32 + column` (`adrs.md` decision 3).
    #[must_use]
    pub const fn index(self) -> u64 {
        ((self.row as u64) << 32) | self.column as u64
    }
}

/// What the caller computed for one generated cell: where the instance goes and how its style
/// shifts (ADR 0030 §3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TilingCell {
    /// The cell.
    pub key: CellKey,
    /// The instance affine `A_cell`, applied to the source in document space.
    pub affine: Affine,
    /// Added to the source's opacity, in percentage points; the result is clamped to 0..100 %.
    pub opacity: Percent,
    /// Turns the hue of the fill and stroke colours.
    pub hue: Angle,
}

/// What [`crate::Document::apply_tiling`] writes.
#[derive(Debug, Clone, PartialEq)]
pub struct TilingRequest {
    /// `None` to create a tiling; the tiling group to re-apply.
    pub tiling: Option<NodeId>,
    /// The selected source nodes, top level, in selection order (create only; a re-apply uses
    /// the stored list).
    pub sources: Vec<NodeId>,
    /// The recipe.
    pub recipe: TilingRecipe,
    /// Every generated cell (the kept source's cell excluded), in key order.
    pub cells: Vec<TilingCell>,
    /// The "Object to path" outline of each primitive among the source leaves, with minted
    /// anchor ids (as for Linked clone); written in the same commit.
    pub conversions: Vec<(NodeId, Vec<crate::path_model::NewAnchor>)>,
}

/// Why a tiling command refused. Nothing is written in any case (ADR 0030 §4).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TilingError {
    /// No source object is left after skipping dimensions.
    #[error("Could not tile: the source is empty.")]
    EmptySource,
    /// A source object no longer exists (its id dangles; only Release is offered).
    #[error("Could not tile: a source object was deleted. Release the tiling to keep it.")]
    SourceMissing,
    /// A source object is on a locked layer or locked.
    #[error("Could not tile: a source object is locked.")]
    Locked,
    /// A source is an orphan clone or its chain is at the limit of 8.
    #[error("Could not tile: a linked clone cannot be cloned any further.")]
    CloneTooDeep,
    /// An instance group would be nested deeper than the group depth limit.
    #[error("Could not tile: the groups would be nested too deep.")]
    TooDeep,
    /// The recipe is out of range or over the count limit.
    #[error("Could not tile: {0}")]
    Recipe(RecipeError),
    /// A source contains a tiling.
    #[error("Could not tile: a tiling cannot be tiled. Release it first.")]
    ContainsTiling,
    /// A source is a layer.
    #[error("Could not tile: a layer cannot be tiled.")]
    IsALayer,
    /// The node to re-apply or release is not a tiling.
    #[error("Could not tile: this is not a tiling.")]
    NotATiling,
    /// The cells are not the layout of the recipe (a key twice, a key outside the layout, or the
    /// kept source's cell).
    #[error("Could not tile: the cells do not fit the recipe.")]
    Cells,
    /// The conversions are not exactly the primitive source leaves.
    #[error("Could not tile: every shape needs one conversion to a path.")]
    Conversions,
}

/// `color` with its hue turned by `shift`, its saturation and lightness kept (the HSL model).
#[must_use]
#[allow(clippy::many_single_char_names)] // the HSL formula's own names
pub fn hue_shifted(color: Color, shift: Angle) -> Color {
    let degrees = shift.as_radians().to_degrees();
    if degrees == 0.0 {
        return color;
    }
    let [r, g, b] = [color.r, color.g, color.b].map(|c| f64::from(c) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = f64::midpoint(max, min);
    let delta = max - min;
    if delta <= 1e-12 {
        return color;
    }
    let saturation = delta / (1.0 - 2.0f64.mul_add(lightness, -1.0).abs());
    let hue = if (max - r).abs() <= f64::EPSILON {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() <= f64::EPSILON {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let hue = (hue + degrees).rem_euclid(360.0);
    let chroma = (1.0 - 2.0f64.mul_add(lightness, -1.0).abs()) * saturation;
    let x = chroma * (1.0 - ((hue / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = lightness - chroma / 2.0;
    let (r1, g1, b1) = match hue {
        h if h < 60.0 => (chroma, x, 0.0),
        h if h < 120.0 => (x, chroma, 0.0),
        h if h < 180.0 => (0.0, chroma, x),
        h if h < 240.0 => (0.0, x, chroma),
        h if h < 300.0 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped to 0..=255
    let channel = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color {
        r: channel(r1),
        g: channel(g1),
        b: channel(b1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hue_shift_of_120_degrees_turns_red_into_green_and_grey_stays() {
        let red = Color { r: 255, g: 0, b: 0 };
        assert_eq!(
            hue_shifted(red, Angle::from_radians(120f64.to_radians())),
            Color { r: 0, g: 255, b: 0 }
        );
        let grey = Color {
            r: 90,
            g: 90,
            b: 90,
        };
        assert_eq!(hue_shifted(grey, Angle::from_radians(1.0)), grey);
    }

    #[test]
    fn the_cell_index_puts_the_row_in_the_high_word() {
        assert_eq!(CellKey { row: 2, column: 5 }.index(), (2 << 32) | 5);
    }
}
