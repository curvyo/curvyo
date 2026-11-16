//! The centres of a pin or screw grid and its limits (`specs/0113-fixture-sets-and-pins`
//! criteria 19 and 21; ADR 0059 §4).

use curvyo_document_core::{Length, Point};

use crate::fixture_kind::MmRange;

/// The most columns or rows of a grid.
pub const MAX_GRID_COUNT: u32 = 50;
/// The most centres of one grid: columns times rows (Proposal).
pub const MAX_GRID_CENTRES: u32 = 400;

/// The columns and rows of a pin or screw grid: each 1 to [`MAX_GRID_COUNT`], their product at
/// most [`MAX_GRID_CENTRES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridCounts {
    columns: u32,
    rows: u32,
}

impl GridCounts {
    /// A single pin or screw: one column, one row.
    pub const SINGLE: Self = Self {
        columns: 1,
        rows: 1,
    };

    /// The counts when both lie within the limits; `None` otherwise.
    #[must_use]
    pub const fn new(columns: u32, rows: u32) -> Option<Self> {
        if columns == 0
            || rows == 0
            || columns > MAX_GRID_COUNT
            || rows > MAX_GRID_COUNT
            || columns * rows > MAX_GRID_CENTRES
        {
            None
        } else {
            Some(Self { columns, rows })
        }
    }

    /// The number of columns.
    #[must_use]
    pub const fn columns(self) -> u32 {
        self.columns
    }

    /// The number of rows.
    #[must_use]
    pub const fn rows(self) -> u32 {
        self.rows
    }

    /// The number of centres.
    #[must_use]
    pub const fn centres(self) -> u32 {
        self.columns * self.rows
    }
}

/// The count nearest to `wanted` that is allowed beside the other count `other`: at least 1, at
/// most [`MAX_GRID_COUNT`] and with a product of at most [`MAX_GRID_CENTRES`]. Returns the count
/// and whether `wanted` had to move (the caller shows its notice).
#[must_use]
pub fn clamp_count(other: u32, wanted: i64) -> (u32, bool) {
    let other = other.clamp(1, MAX_GRID_COUNT);
    let largest = (MAX_GRID_CENTRES / other).min(MAX_GRID_COUNT);
    let count = wanted.clamp(1, i64::from(largest));
    // invariant: `count` lies within 1..=50 after the clamp.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = count as u32;
    (count, i64::from(count) != wanted)
}

/// The centre-to-centre spacing of the columns or the rows: 1 to 1,000 mm, 20 by default.
#[must_use]
pub fn spacing_range() -> MmRange {
    MmRange::new(1.0, 1000.0, 20.0)
}

/// The Centre mark's Size (the diameter of its circle): 0.5 to 50 mm, 4 by default.
#[must_use]
pub fn mark_range() -> MmRange {
    MmRange::new(0.5, 50.0, 4.0)
}

/// The centres of a grid whose first centre is `origin`: columns run in +x by `column_spacing`,
/// rows in +y by `row_spacing` (down on the screen), row by row.
#[must_use]
pub fn centres(
    origin: Point,
    counts: GridCounts,
    column_spacing: Length,
    row_spacing: Length,
) -> Vec<Point> {
    let (dx, dy) = (column_spacing.as_mm(), row_spacing.as_mm());
    (0..counts.rows)
        .flat_map(|row| {
            (0..counts.columns).map(move |column| {
                Point::new(
                    origin.x + f64::from(column) * dx,
                    origin.y + f64::from(row) * dy,
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_lie_within_fifty_and_four_hundred_centres() {
        assert_eq!(GridCounts::new(1, 1), Some(GridCounts::SINGLE));
        let most = GridCounts::new(50, 8).unwrap();
        assert_eq!(most.centres(), 400);
        assert_eq!(GridCounts::new(50, 9), None, "450 centres");
        assert_eq!(GridCounts::new(0, 3), None);
        assert_eq!(GridCounts::new(51, 1), None);
    }

    #[test]
    fn a_count_that_would_pass_the_limits_gives_way_to_the_largest_allowed() {
        assert_eq!(clamp_count(50, 9), (8, true), "50 x 9 is set to 50 x 8");
        assert_eq!(clamp_count(8, 50), (50, false));
        assert_eq!(clamp_count(1, 60), (50, true));
        assert_eq!(clamp_count(3, 0), (1, true));
        assert_eq!(clamp_count(3, -4), (1, true));
        assert_eq!(clamp_count(20, 20), (20, false));
        assert_eq!(clamp_count(20, 21), (20, true));
    }

    #[test]
    fn spacing_and_mark_limits_move_to_the_nearest_limit() {
        let mm = Length::from_mm;
        assert_eq!(spacing_range().default, mm(20.0));
        assert_eq!(spacing_range().clamp(mm(0.5)), (mm(1.0), true));
        assert_eq!(spacing_range().clamp(mm(1500.0)), (mm(1000.0), true));
        assert_eq!(mark_range().default, mm(4.0));
        assert_eq!(mark_range().clamp(mm(0.1)), (mm(0.5), true));
        assert_eq!(mark_range().clamp(mm(60.0)), (mm(50.0), true));
    }

    #[test]
    fn centres_run_right_by_column_and_down_by_row_from_the_first() {
        let mm = Length::from_mm;
        let single = centres(Point::new(5.0, 7.0), GridCounts::SINGLE, mm(20.0), mm(20.0));
        assert_eq!(single, [Point::new(5.0, 7.0)]);
        let grid = centres(
            Point::new(10.0, 10.0),
            GridCounts::new(3, 2).unwrap(),
            mm(30.0),
            mm(15.0),
        );
        assert_eq!(
            grid,
            [
                Point::new(10.0, 10.0),
                Point::new(40.0, 10.0),
                Point::new(70.0, 10.0),
                Point::new(10.0, 25.0),
                Point::new(40.0, 25.0),
                Point::new(70.0, 25.0),
            ]
        );
        let most = centres(
            Point::new(0.0, 0.0),
            GridCounts::new(50, 8).unwrap(),
            mm(1.0),
            mm(1.0),
        );
        assert_eq!(most.len(), 400);
    }
}
