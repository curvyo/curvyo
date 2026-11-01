//! The pure plan of "Arrange in a grid" (`specs/0061-align-and-distribute/`
//! criteria 19 to 21): one translation per selected unit from the boxes, the
//! counts and the gap, written as one commit.

use curvyo_document_core::{Length, Vec2};

use crate::align_commit::{AlignInput, AlignOutcome, BoxedUnits, commit_plan, unit_boxes};
use crate::align_labels::ARRANGE_GRID_COMMIT_LABEL;
use crate::align_plan::{AlignAxis, AlignBox, translation};

/// One translation per input box, in the input's order: the boxes in reading
/// order (lines from top to bottom, each from left to right) fill a grid of
/// `columns` per row, row by row (criterion 20). Every cell is as wide as the
/// widest box and as high as the highest, `gap_mm` lies between neighbouring
/// cells and each box is centred in its cell. The grid's top-left corner is
/// the top-left corner of the boxes' union — unless the boxes already form
/// this very grid, whose union corner lies inside its first cells: then
/// nothing moves, so pressing Grid again with the same values is a no-op
/// (criterion 21). `None` when `rows` x `columns` holds fewer cells than
/// boxes (criterion 19).
#[must_use]
pub fn grid_translations(
    boxes: &[AlignBox],
    rows: u32,
    columns: u32,
    gap_mm: f64,
) -> Option<Vec<Vec2>> {
    let cells = u64::from(rows) * u64::from(columns);
    if cells < u64::try_from(boxes.len()).unwrap_or(u64::MAX) {
        return None;
    }
    let Some(union) = AlignBox::union(boxes) else {
        return Some(Vec::new());
    };
    let grid = Grid {
        cell_w: max_extent(boxes, AlignAxis::Horizontal),
        cell_h: max_extent(boxes, AlignAxis::Vertical),
        gap_mm,
        columns: usize::try_from(columns).unwrap_or(usize::MAX),
        order: reading_order(boxes),
    };
    let (inset_x, inset_y) = grid.first_cell_insets(boxes);
    let in_place = grid.moves(boxes, union.min.x - inset_x, union.min.y - inset_y);
    if in_place.iter().all(|delta| *delta == Vec2::ZERO) {
        return Some(in_place);
    }
    Some(grid.moves(boxes, union.min.x, union.min.y))
}

/// The cells of one grid press: their size, the gap, the row length and the
/// boxes in reading order.
struct Grid {
    cell_w: f64,
    cell_h: f64,
    gap_mm: f64,
    columns: usize,
    order: Vec<usize>,
}

impl Grid {
    /// The row and the column of reading rank `rank`.
    fn cell_of(&self, rank: usize) -> (usize, usize) {
        (rank / self.columns, rank % self.columns)
    }

    /// How far the boxes of the first column and of the first row sit inside
    /// their cells once centred: the distance from the grid's corner to the
    /// union corner of an arranged grid.
    fn first_cell_insets(&self, boxes: &[AlignBox]) -> (f64, f64) {
        let (mut widest, mut highest) = (0.0_f64, 0.0_f64);
        for (rank, &index) in self.order.iter().enumerate() {
            let (row, column) = self.cell_of(rank);
            if column == 0 {
                widest = widest.max(boxes[index].extent(AlignAxis::Horizontal));
            }
            if row == 0 {
                highest = highest.max(boxes[index].extent(AlignAxis::Vertical));
            }
        }
        ((self.cell_w - widest) / 2.0, (self.cell_h - highest) / 2.0)
    }

    /// One translation per box, centring each in its cell of the grid whose
    /// top-left corner is (`left`, `top`).
    fn moves(&self, boxes: &[AlignBox], left: f64, top: f64) -> Vec<Vec2> {
        let mut moves = vec![Vec2::ZERO; boxes.len()];
        for (rank, &index) in self.order.iter().enumerate() {
            let (row, column) = self.cell_of(rank);
            let centre_x = left + count(column) * (self.cell_w + self.gap_mm) + self.cell_w / 2.0;
            let centre_y = top + count(row) * (self.cell_h + self.gap_mm) + self.cell_h / 2.0;
            let dx = translation(
                AlignAxis::Horizontal,
                centre_x - boxes[index].centre(AlignAxis::Horizontal),
            );
            let dy = translation(
                AlignAxis::Vertical,
                centre_y - boxes[index].centre(AlignAxis::Vertical),
            );
            moves[index] = Vec2::new(dx.x, dy.y);
        }
        moves
    }
}

/// The largest extent of `boxes` on `axis`: the cell size.
fn max_extent(boxes: &[AlignBox], axis: AlignAxis) -> f64 {
    boxes.iter().map(|b| b.extent(axis)).fold(0.0, f64::max)
}

/// The indices of `boxes` in reading order (criterion 20): lines from top to
/// bottom, each line from left to right. The boxes are taken by their top
/// edges; a box joins the current line when its top edge lies at or above
/// the middle of the line's vertical extent so far. So a top-aligned row of
/// different heights reads as one line from left to right, and a grid that
/// is already arranged (every box of a row centred on the same line, the
/// next row's tops below it) reads in its own order again (criterion 21).
/// Ties keep the input (paint) order.
fn reading_order(boxes: &[AlignBox]) -> Vec<usize> {
    let mut by_top: Vec<usize> = (0..boxes.len()).collect();
    by_top.sort_by(|&a, &b| boxes[a].min.y.total_cmp(&boxes[b].min.y).then(a.cmp(&b)));
    let mut lines: Vec<(f64, f64, Vec<usize>)> = Vec::new();
    for index in by_top {
        let box_ = boxes[index];
        match lines.last_mut() {
            Some((top, bottom, members)) if box_.min.y <= f64::midpoint(*top, *bottom) => {
                *bottom = bottom.max(box_.max.y);
                members.push(index);
            }
            _ => lines.push((box_.min.y, box_.max.y, vec![index])),
        }
    }
    lines
        .into_iter()
        .flat_map(|(_, _, mut members)| {
            members.sort_by(|&a, &b| {
                boxes[a]
                    .centre(AlignAxis::Horizontal)
                    .total_cmp(&boxes[b].centre(AlignAxis::Horizontal))
                    .then(a.cmp(&b))
            });
            members
        })
        .collect()
}

/// A cell index as a float factor.
fn count(index: usize) -> f64 {
    f64::from(u32::try_from(index).unwrap_or(u32::MAX))
}

/// Arranges the selected units of `input` in a grid of `rows` x `columns`
/// with `gap` between the cells, as one commit "Arrange in a grid"
/// (criteria 20, 21). Fewer than two selected units, or too few cells, write
/// nothing: [`AlignOutcome::NothingSelected`] and
/// [`AlignOutcome::TooFewCells`].
#[must_use]
pub fn arrange_selection_in_grid(
    input: &AlignInput,
    rows: u32,
    columns: u32,
    gap: Length,
) -> AlignOutcome {
    let selected = input.units.selected(input.selection.ids());
    let BoxedUnits { units, boxes } = unit_boxes(input.objects, &selected);
    if boxes.len() < 2 {
        return AlignOutcome::NothingSelected;
    }
    match grid_translations(&boxes, rows, columns, gap.as_mm()) {
        Some(plan) => commit_plan(
            input.document,
            input.objects,
            &units,
            &plan,
            ARRANGE_GRID_COMMIT_LABEL,
        ),
        None => AlignOutcome::TooFewCells,
    }
}
