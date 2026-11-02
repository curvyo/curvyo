//! The placements of a tiling (`specs/0069-tiling` criteria 6 to 13 and 17): one affine per cell
//! for the grid and the circle, with mirror, offset lines and the variance of each cell.
//!
//! Angles run counterclockwise on screen (Y down), 0 pointing right. The instance affine is
//! `translate(cell + offset) · rotate(angle) · scale · mirror`, each about the cell box centre.

use curvyo_document_core::{
    Affine, Angle, CellKey, CircleOptions, GridOptions, Length, OffsetLines, Percent, Point,
    SpacingMode, TilingCell, TilingLayout, TilingRecipe, Vec2,
};

use super::variance::{CellVariance, cell_variance};

/// A sweep this close to a full turn spaces the instances over the whole circle.
const FULL_TURN_TOLERANCE_RAD: f64 = 1e-9;

/// The box of the source (the box Align and Distribute use, criterion 5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellBox {
    /// The top-left corner.
    pub min: Point,
    /// The bottom-right corner.
    pub max: Point,
}

impl CellBox {
    /// The centre of the box.
    #[must_use]
    pub fn centre(self) -> Point {
        Point::new(
            f64::midpoint(self.min.x, self.max.x),
            f64::midpoint(self.min.y, self.max.y),
        )
    }

    /// Width and height.
    #[must_use]
    pub fn size(self) -> Vec2 {
        Vec2::new(self.max.x - self.min.x, self.max.y - self.min.y)
    }
}

/// The centre-to-centre distance of the grid's columns and rows.
#[must_use]
pub fn grid_pitch(cell_box: CellBox, grid: &GridOptions) -> Vec2 {
    let size = cell_box.size();
    match grid.spacing_mode {
        SpacingMode::Gap => Vec2::new(
            size.x + grid.spacing_x.as_mm(),
            size.y + grid.spacing_y.as_mm(),
        ),
        SpacingMode::Pitch => Vec2::new(grid.spacing_x.as_mm(), grid.spacing_y.as_mm()),
    }
}

/// The spacing values after switching the mode, so the layout does not move (criterion 7).
#[must_use]
pub fn converted_spacing(
    cell_box: CellBox,
    grid: &GridOptions,
    to: SpacingMode,
) -> (Length, Length) {
    if grid.spacing_mode == to {
        return (grid.spacing_x, grid.spacing_y);
    }
    let size = cell_box.size();
    let sign: f64 = if to == SpacingMode::Pitch { 1.0 } else { -1.0 };
    (
        Length::from_mm(sign.mul_add(size.x, grid.spacing_x.as_mm())),
        Length::from_mm(sign.mul_add(size.y, grid.spacing_y.as_mm())),
    )
}

/// The circle options of a new tiling (criterion 11): 12 instances over a full turn from 0°, the
/// radius half the larger side plus 10 mm, the centre left of the box by the radius.
#[must_use]
pub fn default_circle(cell_box: CellBox) -> CircleOptions {
    let size = cell_box.size();
    let radius = size.x.max(size.y) / 2.0 + 10.0;
    CircleOptions {
        count: 12,
        centre: Vec2::new(-radius, 0.0),
        radius: Length::from_mm(radius),
        start: Angle::from_radians(0.0),
        sweep: Angle::from_radians(std::f64::consts::TAU),
        face_centre: false,
    }
}

/// The angle of instance `index` on the circle (criterion 12), radians counterclockwise.
#[must_use]
fn circle_angle(circle: &CircleOptions, index: u32) -> f64 {
    let start = circle.start.as_radians();
    let sweep = circle.sweep.as_radians();
    let k = f64::from(index);
    if (sweep.abs() - std::f64::consts::TAU).abs() <= FULL_TURN_TOLERANCE_RAD {
        start + k * sweep / f64::from(circle.count)
    } else if circle.count > 1 {
        start + k * sweep / f64::from(circle.count - 1)
    } else {
        start
    }
}

/// Every cell key of the layout in use, in key order; the kept source's cell left out.
fn cell_keys(recipe: &TilingRecipe) -> impl Iterator<Item = CellKey> + '_ {
    let (rows, columns) = match recipe.layout {
        TilingLayout::Grid => (recipe.grid.rows, recipe.grid.columns),
        TilingLayout::Circle => (1, recipe.circle.count),
    };
    (0..rows)
        .flat_map(move |row| (0..columns).map(move |column| CellKey { row, column }))
        .filter(move |key| !(recipe.keep_source && *key == CellKey::ORIGIN))
}

/// The cell's displacement from the source, its mirror signs and its layout turn.
fn cell_frame(cell_box: CellBox, recipe: &TilingRecipe, key: CellKey) -> (Vec2, (f64, f64), f64) {
    match recipe.layout {
        TilingLayout::Grid => {
            let grid = &recipe.grid;
            let pitch = grid_pitch(cell_box, grid);
            let shift = grid.shift.get() / 100.0;
            let mut at = Vec2::new(
                f64::from(key.column) * pitch.x,
                f64::from(key.row) * pitch.y,
            );
            match grid.offset {
                OffsetLines::Rows if key.row % 2 == 1 => at.x += shift * pitch.x,
                OffsetLines::Columns if key.column % 2 == 1 => at.y += shift * pitch.y,
                _ => {}
            }
            let flip = |on: bool, line: u32| if on && line % 2 == 1 { -1.0 } else { 1.0 };
            let mirror = (
                flip(grid.mirror_columns, key.column),
                flip(grid.mirror_rows, key.row),
            );
            (at, mirror, 0.0)
        }
        TilingLayout::Circle => {
            let circle = &recipe.circle;
            let theta = circle_angle(circle, key.column);
            let r = circle.radius.as_mm();
            let centre = cell_box.centre().translated(circle.centre);
            let at = Point::new(
                r.mul_add(theta.cos(), centre.x),
                (-r).mul_add(theta.sin(), centre.y),
            );
            let turn = if circle.face_centre {
                theta - circle.start.as_radians()
            } else {
                0.0
            };
            (cell_box.centre().vector_to(at), (1.0, 1.0), turn)
        }
    }
}

/// The affine of one instance: about the box centre `c`, mirror and scale, then the turn
/// (counterclockwise on screen), then the move to the cell plus the offset.
#[allow(clippy::many_single_char_names)] // the SVG names of the six entries
fn instance_affine(c: Point, at: Vec2, mirror: (f64, f64), turn: f64, v: &CellVariance) -> Affine {
    let (sin, cos) = (-(turn + v.rotation.as_radians())).sin_cos();
    let (dx, dy) = (v.scale_x * mirror.0, v.scale_y * mirror.1);
    let (a, b, cc, d) = (cos * dx, sin * dx, -sin * dy, cos * dy);
    let target = c.translated(at).translated(v.offset);
    let e = target.x - a.mul_add(c.x, cc * c.y);
    let f = target.y - b.mul_add(c.x, d * c.y);
    Affine::new(a, b, cc, d, e, f)
}

/// The cells of `recipe` for a source with box `cell_box` (criterion 17): every generated cell
/// in key order with its affine and style shift; the kept source's cell left out. Pure: the same
/// recipe and box give the same cells on every run and platform.
#[must_use]
pub fn tiling_layout(cell_box: CellBox, recipe: &TilingRecipe) -> Vec<TilingCell> {
    let c = cell_box.centre();
    cell_keys(recipe)
        .map(|key| {
            let variance = cell_variance(&recipe.variance, recipe.seed, key);
            let (at, mirror, turn) = cell_frame(cell_box, recipe, key);
            TilingCell {
                key,
                affine: instance_affine(c, at, mirror, turn, &variance),
                opacity: variance.opacity,
                hue: variance.hue,
            }
        })
        .collect()
}

/// The recipe of a new tiling for a source with box `cell_box` and a fresh `seed` (criteria 6,
/// 11, 19, 22, 23): a 5 × 4 grid with 5 mm gaps, the default circle, no variance, clones, the
/// source kept in place.
#[must_use]
pub fn default_recipe(cell_box: CellBox, seed: u32) -> TilingRecipe {
    TilingRecipe {
        layout: TilingLayout::Grid,
        grid: GridOptions {
            columns: 5,
            rows: 4,
            spacing_x: Length::from_mm(5.0),
            spacing_y: Length::from_mm(5.0),
            spacing_mode: SpacingMode::Gap,
            mirror_rows: false,
            mirror_columns: false,
            offset: OffsetLines::None,
            shift: Percent::new(50.0),
        },
        circle: default_circle(cell_box),
        variance: curvyo_document_core::Variance::default(),
        seed,
        result: curvyo_document_core::TilingResult::Clones,
        keep_source: true,
    }
}
