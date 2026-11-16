//! The grid of a pin or a screw as the canvas and the row edit it (`specs/0113-fixture-sets-and-pins`
//! criteria 18 to 21; UX notes 6.1, 6.2): the Diameter, Columns and Rows knobs, their drags and
//! readouts, and the typed Diameter, counts, spacings and Size with their clamping notices.

use curvyo_document_core::{Length, NodeId, Point};
use curvyo_laser_core::{
    CentreGrid, GridCounts, MAX_GRID_CENTRES, MAX_GRID_COUNT, MmRange, clamp_count, mark_range,
    pin_diameter_range, spacing_range,
};
use curvyo_ui_core::OrientedBox;

use super::super::Session;
use super::edit::FixtureEdit;
use super::handles::{FixtureHandle, at};
use super::params::{clamp_notice, number};
use super::placed::PlacedFixture;
use super::shape::Shape;

/// The Columns and Rows knobs of a pin or screw grid: at the last centre of row 1 and of column 1,
/// each only while its count is above 1 (UX notes F9).
pub(super) fn grid_knobs(
    fixture: &PlacedFixture,
    box_: &OrientedBox,
) -> Vec<(FixtureHandle, Point)> {
    let Some(grid) = fixture.shape.centre_grid() else {
        return Vec::new();
    };
    let (columns, rows) = (grid.counts.columns(), grid.counts.rows());
    let mut out = Vec::new();
    if columns > 1 {
        let x = f64::from(columns - 1) * grid.column_spacing.as_mm();
        out.push((FixtureHandle::Columns, at(box_, (x, 0.0))));
    }
    if rows > 1 {
        let y = f64::from(rows - 1) * grid.row_spacing.as_mm();
        out.push((FixtureHandle::Rows, at(box_, (0.0, y))));
    }
    out
}

/// `shape` with its grid replaced by `grid`.
fn with_grid(shape: &Shape, grid: CentreGrid) -> Shape {
    match shape {
        Shape::Pin { diameter, .. } => Shape::Pin {
            diameter: *diameter,
            grid,
        },
        Shape::Screw { thread, head, .. } => Shape::Screw {
            thread: *thread,
            head: *head,
            grid,
        },
        other => other.clone(),
    }
}

/// `start` with its shape replaced, the matrix and slots kept.
fn reshaped(start: &PlacedFixture, shape: Shape) -> FixtureEdit {
    FixtureEdit {
        shape,
        matrix: start.matrix,
        slots: Vec::new(),
    }
}

/// A drag of the Diameter, Columns or Rows knob: `local` is the pointer in the fixture's frame,
/// whose origin is the first centre. The value stops at its limit.
pub(super) fn knob_edit(start: &PlacedFixture, handle: FixtureHandle, local: Point) -> FixtureEdit {
    let Some(grid) = start.shape.centre_grid() else {
        return reshaped(start, start.shape.clone());
    };
    match (handle, &start.shape) {
        (FixtureHandle::Diameter, Shape::Pin { .. }) => {
            let (diameter, _) = pin_diameter_range().clamp(Length::from_mm(2.0 * local.x));
            reshaped(start, Shape::Pin { diameter, grid })
        }
        (FixtureHandle::Columns, _) if grid.counts.columns() > 1 => {
            let spacing = local.x / f64::from(grid.counts.columns() - 1);
            let (column_spacing, _) = spacing_range().clamp(Length::from_mm(spacing));
            reshaped(
                start,
                with_grid(
                    &start.shape,
                    CentreGrid {
                        column_spacing,
                        ..grid
                    },
                ),
            )
        }
        (FixtureHandle::Rows, _) if grid.counts.rows() > 1 => {
            let spacing = local.y / f64::from(grid.counts.rows() - 1);
            let (row_spacing, _) = spacing_range().clamp(Length::from_mm(spacing));
            reshaped(
                start,
                with_grid(
                    &start.shape,
                    CentreGrid {
                        row_spacing,
                        ..grid
                    },
                ),
            )
        }
        _ => reshaped(start, start.shape.clone()),
    }
}

/// The readout of a Columns or Rows drag: "Spacing 120.0 mm", "min" or "max" at a limit.
pub(super) fn spacing_readout(handle: FixtureHandle, shape: &Shape) -> Option<String> {
    let grid = shape.centre_grid()?;
    let spacing = if handle == FixtureHandle::Columns {
        grid.column_spacing
    } else {
        grid.row_spacing
    };
    let range = spacing_range();
    let limit = if (spacing.as_mm() - range.min.as_mm()).abs() < 1e-9 {
        " min"
    } else if (spacing.as_mm() - range.max.as_mm()).abs() < 1e-9 {
        " max"
    } else {
        ""
    };
    Some(format!("Spacing {:.1} mm{limit}", spacing.as_mm()))
}

/// Which value of a pin or screw a row field or a chip field sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GridValue {
    /// A pin's diameter.
    Diameter,
    /// The number of columns.
    Columns,
    /// The number of rows.
    Rows,
    /// The spacing of the columns.
    ColumnSpacing,
    /// The spacing of the rows.
    RowSpacing,
    /// The Centre mark's Size.
    Mark,
}

impl GridValue {
    /// The value a row field key names: the stored key of ADR 0059 §4.
    pub(super) fn named(key: &str) -> Option<Self> {
        Some(match key {
            "diameter_mm" => Self::Diameter,
            "columns" => Self::Columns,
            "rows" => Self::Rows,
            "column_spacing_mm" => Self::ColumnSpacing,
            "row_spacing_mm" => Self::RowSpacing,
            "mark_mm" => Self::Mark,
            _ => return None,
        })
    }
}

/// A length set to its range, with the notice "`<word>` is `<min>` to `<max>` mm. Set to `<v>` mm.".
fn length_in(range: MmRange, word: &str, value: f64) -> (Length, Option<String>) {
    let (kept, moved) = range.clamp(Length::from_mm(value));
    let notice = moved.then(|| {
        clamp_notice(
            word,
            range.min.as_mm(),
            range.max.as_mm(),
            "mm",
            kept.as_mm(),
        )
    });
    (kept, notice)
}

/// A count set beside `other`: 1 to 50 ("Columns are 1 to 50. Set to 50."), and at most 400
/// centres ("At most 400 pins. Columns set to 20.", criterion 19).
fn count_in(word: &str, noun: &str, other: u32, value: f64) -> (u32, Option<String>) {
    // invariant: a finite value rounded and clamped to the i64 range below.
    #[allow(clippy::cast_possible_truncation)]
    let wanted = if value.is_finite() {
        value.round().clamp(-1e9, 1e9) as i64
    } else {
        1
    };
    let (count, moved) = clamp_count(other, wanted);
    let in_range = (1..=i64::from(MAX_GRID_COUNT)).contains(&wanted);
    let notice = moved.then(|| {
        if in_range {
            format!(
                "At most {} {noun}. {word} set to {count}.",
                number(f64::from(MAX_GRID_CENTRES))
            )
        } else {
            format!("{word} are 1 to {MAX_GRID_COUNT}. Set to {count}.")
        }
    });
    (count, notice)
}

/// The shape of `fixture` with `value` set to `typed`, and the notice of a value that moved.
pub(super) fn set_value(
    fixture: &PlacedFixture,
    value: GridValue,
    typed: f64,
) -> Option<(Shape, Option<String>)> {
    let grid = fixture.shape.centre_grid()?;
    let noun = if matches!(fixture.shape, Shape::Pin { .. }) {
        "pins"
    } else {
        "screws"
    };
    let (columns, rows) = (grid.counts.columns(), grid.counts.rows());
    let counts = |c, r| GridCounts::new(c, r).unwrap_or(GridCounts::SINGLE);
    let (shape, notice) = match value {
        GridValue::Diameter => {
            let Shape::Pin { .. } = fixture.shape else {
                return None;
            };
            let (diameter, notice) = length_in(pin_diameter_range(), "Diameter", typed);
            (Shape::Pin { diameter, grid }, notice)
        }
        GridValue::Columns => {
            let (count, notice) = count_in("Columns", noun, rows, typed);
            let grid = CentreGrid {
                counts: counts(count, rows),
                ..grid
            };
            (with_grid(&fixture.shape, grid), notice)
        }
        GridValue::Rows => {
            let (count, notice) = count_in("Rows", noun, columns, typed);
            let grid = CentreGrid {
                counts: counts(columns, count),
                ..grid
            };
            (with_grid(&fixture.shape, grid), notice)
        }
        GridValue::ColumnSpacing | GridValue::RowSpacing => {
            let (spacing, notice) = length_in(spacing_range(), "Spacing", typed);
            let grid = if value == GridValue::ColumnSpacing {
                CentreGrid {
                    column_spacing: spacing,
                    ..grid
                }
            } else {
                CentreGrid {
                    row_spacing: spacing,
                    ..grid
                }
            };
            (with_grid(&fixture.shape, grid), notice)
        }
        GridValue::Mark => {
            let (mark, notice) = length_in(mark_range(), "Size", typed);
            (
                with_grid(&fixture.shape, CentreGrid { mark, ..grid }),
                notice,
            )
        }
    };
    Some((shape, notice))
}

impl Session {
    /// A row field of the pin or screw `id` (`0113` criteria 18 to 20, 25): `diameter_mm` (pin
    /// only), `columns`, `rows`, `column_spacing_mm`, `row_spacing_mm` or `mark_mm`, set to its
    /// nearest limit with a notice; one step "Resize fixture". `false` for another kind or key,
    /// a locked fixture, or a value already stored.
    pub fn set_fixture_grid_value(&mut self, id: NodeId, key: &str, typed: f64) -> bool {
        let Some(value) = GridValue::named(key) else {
            return false;
        };
        let Some((_, fixture)) = self.page_fixture(id) else {
            return false;
        };
        if fixture.keeps_place() {
            return false;
        }
        let Some((shape, notice)) = set_value(&fixture, value, typed) else {
            return false;
        };
        self.fixtures.notice = notice;
        if shape == fixture.shape {
            return false;
        }
        let edit = reshaped(&fixture, shape);
        self.write_fixture_edit(id, &edit, curvyo_document_core::RESIZE_FIXTURE_COMMIT_LABEL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_count_past_fifty_or_past_four_hundred_centres_says_so() {
        assert_eq!(
            count_in("Columns", "pins", 1, 60.0),
            (50, Some("Columns are 1 to 50. Set to 50.".to_owned()))
        );
        assert_eq!(
            count_in("Columns", "pins", 20, 30.0),
            (20, Some("At most 400 pins. Columns set to 20.".to_owned()))
        );
        assert_eq!(
            count_in("Rows", "screws", 2, 0.0),
            (1, Some("Rows are 1 to 50. Set to 1.".to_owned()))
        );
        assert_eq!(count_in("Rows", "pins", 8, 50.0), (50, None));
    }

    #[test]
    fn a_length_out_of_range_reads_its_limit_and_its_notice() {
        assert_eq!(
            length_in(pin_diameter_range(), "Diameter", 2000.0)
                .1
                .as_deref(),
            Some("Diameter is 0.5 to 1,000 mm. Set to 1,000 mm.")
        );
        assert_eq!(
            length_in(mark_range(), "Size", 60.0).1.as_deref(),
            Some("Size is 0.5 to 50 mm. Set to 50 mm.")
        );
        assert_eq!(
            length_in(spacing_range(), "Spacing", 0.2).1.as_deref(),
            Some("Spacing is 1 to 1,000 mm. Set to 1 mm.")
        );
    }
}
