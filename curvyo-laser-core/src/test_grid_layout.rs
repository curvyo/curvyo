//! Where the cells and labels of a test grid lie (`specs/0028` criteria 2 to 5, 9, 10): the
//! layout from the label lane widths, the bed fit, the first free spot, the kept mark and the
//! hand-edited comparison.

use curvyo_document_core::{DocumentSize, Length, Point};

use crate::machine_profile::FeedRate;
use crate::material::{CellIndex, PowerPercent, SetKind};
use crate::test_grid::GridRecipe;

/// The tolerance of every comparison of built items to the layout, in mm.
pub const GRID_TOLERANCE_MM: f64 = 1e-3;
/// The text of the unit line (criterion 4).
pub const UNIT_TEXT: &str = "% power / mm/min";
/// The space between the label lanes and the cells, in mm (Proposal).
const LANE_GAP_MM: f64 = 1.0;

/// What a label item of a grid shows (`x.curvyo.laser.grid_label`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LabelTag {
    /// A column's power.
    Power,
    /// A row's speed.
    Speed,
    /// The unit line.
    Unit,
}

impl LabelTag {
    /// The stored text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Speed => "speed",
            Self::Unit => "unit",
        }
    }

    /// The tag a stored text names.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Power, Self::Speed, Self::Unit]
            .into_iter()
            .find(|tag| tag.as_str() == text)
    }
}

/// The widths of the label texts, measured by the caller in the document's default font at the
/// label height (core shapes no text).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelLanes {
    /// The widest speed label.
    pub speed_width: Length,
    /// The unit line.
    pub unit_width: Length,
}

/// One cell as laid out, in grid coordinates (top left at 0, 0; y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellPlace {
    /// Its index.
    pub cell: CellIndex,
    /// Its power.
    pub power: PowerPercent,
    /// Its speed.
    pub speed: FeedRate,
    /// The top-left corner of its square.
    pub min: Point,
    /// The square's side.
    pub side: Length,
}

/// One label as laid out: its text and the left end of its baseline.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelPlace {
    /// What it shows.
    pub tag: LabelTag,
    /// The text.
    pub text: String,
    /// The left end of the baseline.
    pub origin: Point,
}

/// A whole grid as laid out.
#[derive(Debug, Clone, PartialEq)]
pub struct TestGridLayout {
    /// The cells, row by row from the top left.
    pub cells: Vec<CellPlace>,
    /// The labels: the unit line, the power labels left to right, the speed labels top down.
    pub labels: Vec<LabelPlace>,
    /// The size with labels.
    pub width: Length,
    /// The size with labels.
    pub height: Length,
}

/// The texts of the speed labels of `recipe`, top down, for the caller to measure.
#[must_use]
pub fn speed_texts(recipe: &GridRecipe) -> Vec<String> {
    (0..recipe.range.rows)
        .map(|row| recipe.row_speed(row).get().to_string())
        .collect()
}

/// The layout of `recipe` with the measured `lanes`.
#[must_use]
pub fn layout(recipe: &GridRecipe, lanes: LabelLanes) -> TestGridLayout {
    let range = &recipe.range;
    let (cell, gap) = (range.cell.as_mm(), range.gap.as_mm());
    let h = recipe.labels.height.as_mm();
    let (x0, y0) = if recipe.labels.on {
        (
            lanes.speed_width.as_mm() + LANE_GAP_MM,
            2.0 * h + 2.0 * LANE_GAP_MM,
        )
    } else {
        (0.0, 0.0)
    };
    let pitch = cell + gap;
    let mut cells = Vec::with_capacity(usize::from(range.columns) * usize::from(range.rows));
    for row in 0..range.rows {
        for column in 0..range.columns {
            cells.push(CellPlace {
                cell: CellIndex { column, row },
                power: recipe.column_power(column),
                speed: recipe.row_speed(row),
                min: Point::new(x0 + f64::from(column) * pitch, y0 + f64::from(row) * pitch),
                side: range.cell,
            });
        }
    }
    let mut labels = Vec::new();
    if recipe.labels.on {
        labels.push(LabelPlace {
            tag: LabelTag::Unit,
            text: UNIT_TEXT.to_owned(),
            origin: Point::new(0.0, h),
        });
        for column in 0..range.columns {
            labels.push(LabelPlace {
                tag: LabelTag::Power,
                text: recipe.column_power(column).to_string(),
                origin: Point::new(x0 + f64::from(column) * pitch, 2.0 * h + LANE_GAP_MM),
            });
        }
        for (row, text) in (0..range.rows).zip(speed_texts(recipe)) {
            labels.push(LabelPlace {
                tag: LabelTag::Speed,
                text,
                origin: Point::new(0.0, y0 + f64::from(row) * pitch + f64::midpoint(cell, h)),
            });
        }
    }
    let cells_width = x0 + f64::from(range.columns) * pitch - gap;
    let width = if recipe.labels.on {
        cells_width.max(lanes.unit_width.as_mm())
    } else {
        cells_width
    };
    TestGridLayout {
        cells,
        labels,
        width: Length::from_mm(width),
        height: Length::from_mm(y0 + f64::from(range.rows) * pitch - gap),
    }
}

/// Whether a grid of `width` x `height` fits the bed `bed`; else the cause of criterion 2
/// ("Grid is 260 mm wide, the bed is 200 mm.").
///
/// # Errors
/// The cause text.
pub fn fit_bed(width: Length, height: Length, bed: DocumentSize) -> Result<(), String> {
    let too = |what: &str, grid: Length, bed: Length| {
        format!(
            "Grid is {} mm {what}, the bed is {} mm.",
            mm_text(grid),
            mm_text(bed)
        )
    };
    if width.as_mm() > bed.width.as_mm() + GRID_TOLERANCE_MM {
        return Err(too("wide", width, bed.width));
    }
    if height.as_mm() > bed.height.as_mm() + GRID_TOLERANCE_MM {
        return Err(too("high", height, bed.height));
    }
    Ok(())
}

/// The refusal of the add button when no spot is free (UX 4).
#[must_use]
pub fn no_free_spot_text(width: Length, height: Length) -> String {
    format!(
        "No free spot on the bed for a {} × {} mm grid.",
        mm_text(width),
        mm_text(height)
    )
}

/// A length in mm as the grid texts write it: whole when it is, else one decimal, rounded up so
/// a grid never reads as fitting when it does not.
fn mm_text(length: Length) -> String {
    let tenths = (length.as_mm() * 10.0 - GRID_TOLERANCE_MM).ceil() / 10.0;
    if tenths.fract().abs() < 1e-9 {
        format!("{tenths:.0}")
    } else {
        format!("{tenths:.1}")
    }
}

/// The first free spot for a grid of `width` x `height` on the bed `bed` (bed coordinates, top
/// left at 0, 0): the top-left corner of the bed, else the first place found left to right, top
/// to bottom where it touches none of the `occupied` boxes (min, max) (criterion 3). `None` when
/// none is free.
#[must_use]
pub fn free_spot(
    width: Length,
    height: Length,
    bed: DocumentSize,
    occupied: &[(Point, Point)],
) -> Option<Point> {
    let (w, h) = (width.as_mm(), height.as_mm());
    let mut xs: Vec<f64> = std::iter::once(0.0)
        .chain(occupied.iter().map(|(_, max)| max.x))
        .collect();
    let mut ys: Vec<f64> = std::iter::once(0.0)
        .chain(occupied.iter().map(|(_, max)| max.y))
        .collect();
    xs.sort_by(f64::total_cmp);
    ys.sort_by(f64::total_cmp);
    let fits = |x: f64, y: f64| {
        x >= -GRID_TOLERANCE_MM
            && y >= -GRID_TOLERANCE_MM
            && x + w <= bed.width.as_mm() + GRID_TOLERANCE_MM
            && y + h <= bed.height.as_mm() + GRID_TOLERANCE_MM
            && occupied.iter().all(|(min, max)| {
                x + w <= min.x + GRID_TOLERANCE_MM
                    || max.x <= x + GRID_TOLERANCE_MM
                    || y + h <= min.y + GRID_TOLERANCE_MM
                    || max.y <= y + GRID_TOLERANCE_MM
            })
    };
    ys.iter()
        .flat_map(|&y| xs.iter().map(move |&x| (x, y)))
        .find(|&(x, y)| fits(x, y))
        .map(|(x, y)| Point::new(x, y))
}

/// The mark after a rebuild from `old` to `new` (criterion 9): the cell of `new` with the same
/// power and speed as the marked cell of `old`, else none.
#[must_use]
pub fn kept_mark(old: &GridRecipe, mark: CellIndex, new: &GridRecipe) -> Option<CellIndex> {
    let (power, speed) = (old.column_power(mark.column), old.row_speed(mark.row));
    let column = (0..new.range.columns).find(|&c| new.column_power(c) == power)?;
    let row = (0..new.range.rows).find(|&r| new.row_speed(r) == speed)?;
    Some(CellIndex { column, row })
}

/// A tagged cell item of a grid as built, in page coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuiltCell {
    /// Its stored index.
    pub cell: CellIndex,
    /// Its stored power.
    pub power: PowerPercent,
    /// Its stored speed.
    pub speed: FeedRate,
    /// The top-left corner of its outline's box.
    pub min: Point,
    /// The bottom-right corner of its outline's box.
    pub max: Point,
    /// Whether it is an unturned rectangle.
    pub square: bool,
    /// Whether its fill is on.
    pub filled: bool,
}

/// A tagged label item of a grid as built, in page coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct BuiltLabel {
    /// Its stored tag.
    pub tag: LabelTag,
    /// Its text.
    pub text: String,
    /// The left end of its baseline, `None` when its placement is not a plain move.
    pub origin: Option<Point>,
}

/// Whether the built items differ from `layout` of a grid of kind `kind` (criterion 10): a cell
/// or label missing, added, moved against the others, restyled or retyped. Moving the whole grid
/// is no hand edit.
#[must_use]
pub fn hand_edited(
    layout: &TestGridLayout,
    kind: SetKind,
    cells: &[BuiltCell],
    labels: &[BuiltLabel],
) -> bool {
    let Some(first) = layout.cells.first() else {
        return !cells.is_empty();
    };
    let Some(anchor) = cells.iter().find(|c| c.cell == first.cell) else {
        return true;
    };
    let offset = (anchor.min.x - first.min.x, anchor.min.y - first.min.y);
    let near = |a: f64, b: f64| (a - b).abs() <= GRID_TOLERANCE_MM;
    let same_place = |built: Point, laid: Point| {
        near(built.x, laid.x + offset.0) && near(built.y, laid.y + offset.1)
    };
    if cells.len() != layout.cells.len() || labels.len() != layout.labels.len() {
        return true;
    }
    let filled = kind == SetKind::Engrave;
    let cells_match = layout.cells.iter().all(|laid| {
        cells.iter().any(|built| {
            let side = laid.side.as_mm();
            built.cell == laid.cell
                && built.power == laid.power
                && built.speed == laid.speed
                && built.square
                && built.filled == filled
                && same_place(built.min, laid.min)
                && near(built.max.x - built.min.x, side)
                && near(built.max.y - built.min.y, side)
        })
    });
    let mut unmatched: Vec<&BuiltLabel> = labels.iter().collect();
    let labels_match = layout.labels.iter().all(|laid| {
        let found = unmatched.iter().position(|built| {
            built.tag == laid.tag
                && built.text == laid.text
                && built.origin.is_some_and(|at| same_place(at, laid.origin))
        });
        found.map(|at| unmatched.swap_remove(at)).is_some()
    });
    !(cells_match && labels_match)
}

#[cfg(test)]
#[path = "test_grid_layout_tests.rs"]
mod tests;
