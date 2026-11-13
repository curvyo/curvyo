//! Where the cells and burned labels of a test grid of `specs/0112` lie (criteria 3, 7, 10;
//! ADR 0057 §1): a fixed 2 mm gap and 3 mm text, the label lanes from the caller's text widths.

use curvyo_document_core::{Length, Point};

use crate::test_recipe::{Recipe, Side, TestRecipe};

/// The gap between cells in mm, constant and never stored (criterion 3).
pub const GAP_MM: f64 = 2.0;
/// The height of the burned text in mm (Proposal, criterion 10).
pub const TEXT_MM: f64 = 3.0;
/// The space between a label lane and the cells, in mm (Proposal).
const LANE_GAP_MM: f64 = 1.0;

/// What a burned label shows (`x.curvyo.laser.grid_label`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LabelPart {
    /// The test id, at the top left.
    Id,
    /// The title of the columns' parameter.
    ColumnTitle,
    /// The title of the rows' parameter.
    RowTitle,
    /// The value of one column.
    ColumnValue,
    /// The value of one row.
    RowValue,
}

impl LabelPart {
    /// Every part, in the order the layout lists them.
    pub const ALL: [Self; 5] = [
        Self::Id,
        Self::ColumnTitle,
        Self::RowTitle,
        Self::ColumnValue,
        Self::RowValue,
    ];

    /// The stored text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::ColumnTitle => "column_title",
            Self::RowTitle => "row_title",
            Self::ColumnValue => "column_value",
            Self::RowValue => "row_value",
        }
    }

    /// The part a stored text names.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == text)
    }
}

/// One cell as laid out, in grid coordinates (top left at 0, 0; y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaidCell {
    /// Its column, from 0.
    pub column: u8,
    /// Its row, from 0.
    pub row: u8,
    /// The top-left corner of its square.
    pub min: Point,
    /// The square's side.
    pub side: Length,
}

/// One burned label as laid out: its text and the left end of its baseline.
#[derive(Debug, Clone, PartialEq)]
pub struct LaidLabel {
    /// What it shows.
    pub part: LabelPart,
    /// The text.
    pub text: String,
    /// The left end of the baseline.
    pub origin: Point,
}

/// A whole grid as laid out.
#[derive(Debug, Clone, PartialEq)]
pub struct TestLayout {
    /// The cells, row by row from the top left.
    pub cells: Vec<LaidCell>,
    /// The labels: id, column title, row title, column values left to right, row values top
    /// down.
    pub labels: Vec<LaidLabel>,
    /// The width with labels.
    pub width: Length,
    /// The height with labels.
    pub height: Length,
}

/// The layout of `recipe`, `measure` giving the width of a text at [`TEXT_MM`] in the
/// document's default font (core shapes no text). Line 1 holds the id and the column title,
/// line 2 the row title and the column values; the row values sit left of their rows.
#[must_use]
pub fn layout(recipe: &Recipe, measure: impl Fn(&str) -> Length) -> TestLayout {
    let test = &recipe.test;
    let (columns, rows) = (test.axis(Side::Columns), test.axis(Side::Rows));
    let column_texts: Vec<String> = (0..columns.count())
        .map(|i| columns.value(i).to_string())
        .collect();
    let row_texts: Vec<String> = (0..rows.count())
        .map(|i| rows.value(i).to_string())
        .collect();
    let id = recipe.id.to_string();
    let (column_title, row_title) = (columns.param().title(), rows.param().title());
    let lane = std::iter::once(id.as_str())
        .chain(std::iter::once(row_title))
        .chain(row_texts.iter().map(String::as_str))
        .map(|t| measure(t).as_mm())
        .fold(0.0, f64::max);
    let (cell, h) = (test.cell().as_mm(), TEXT_MM);
    let (x0, y0) = (lane + LANE_GAP_MM, 2.0 * h + 2.0 * LANE_GAP_MM);
    let pitch = cell + GAP_MM;
    let frame = Frame {
        x0,
        y0,
        pitch,
        cell,
    };
    let cells = laid_cells(test, &frame);
    let titles = Titles {
        id: &id,
        column: column_title,
        row: row_title,
    };
    let labels = laid_labels(&titles, &column_texts, &row_texts, &frame);
    let cells_right = x0 + f64::from(columns.count()) * pitch - GAP_MM;
    let width = cells_right.max(x0 + measure(column_title).as_mm());
    TestLayout {
        cells,
        labels,
        width: Length::from_mm(width),
        height: Length::from_mm(y0 + f64::from(rows.count()) * pitch - GAP_MM),
    }
}

/// Where the cells start (`x0`, `y0`), how far apart they sit and how wide each is, in mm.
struct Frame {
    x0: f64,
    y0: f64,
    pitch: f64,
    cell: f64,
}

/// The texts of the layout's two title lines.
struct Titles<'a> {
    id: &'a str,
    column: &'a str,
    row: &'a str,
}

/// The cells of `test`'s grid, row by row.
fn laid_cells(test: &TestRecipe, frame: &Frame) -> Vec<LaidCell> {
    let (columns, rows) = (test.axis(Side::Columns), test.axis(Side::Rows));
    let mut cells = Vec::with_capacity(usize::from(columns.count()) * usize::from(rows.count()));
    for row in 0..rows.count() {
        for column in 0..columns.count() {
            cells.push(LaidCell {
                column,
                row,
                min: Point::new(
                    frame.x0 + f64::from(column) * frame.pitch,
                    frame.y0 + f64::from(row) * frame.pitch,
                ),
                side: test.cell(),
            });
        }
    }
    cells
}

/// The id, the two titles, the column values above and the row values left of the cells.
fn laid_labels(
    titles: &Titles<'_>,
    column_texts: &[String],
    row_texts: &[String],
    frame: &Frame,
) -> Vec<LaidLabel> {
    let h = TEXT_MM;
    let line_2 = 2.0 * h + LANE_GAP_MM;
    let label = |part, text: &str, x, y| LaidLabel {
        part,
        text: text.to_owned(),
        origin: Point::new(x, y),
    };
    let mut labels = vec![
        label(LabelPart::Id, titles.id, 0.0, h),
        label(LabelPart::ColumnTitle, titles.column, frame.x0, h),
        label(LabelPart::RowTitle, titles.row, 0.0, line_2),
    ];
    for (column, t) in (0..).zip(column_texts) {
        labels.push(label(
            LabelPart::ColumnValue,
            t,
            frame.x0 + f64::from(column) * frame.pitch,
            line_2,
        ));
    }
    for (row, t) in (0..).zip(row_texts) {
        let y = frame.y0 + f64::from(row) * frame.pitch + f64::midpoint(frame.cell, h);
        labels.push(label(LabelPart::RowValue, t, 0.0, y));
    }
    labels
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn recipe() -> Recipe {
        Recipe::from_value(&json!({
            "id": "K7Q2M",
            "kind": "cut",
            "columns": { "param": "power", "min": 20.0, "max": 100.0, "count": 5 },
            "rows": { "param": "speed", "min": 600, "max": 6000, "count": 5 },
            "fixed": { "passes": 1 },
            "cell_mm": 10.0,
            "crosshatch": false
        }))
        .unwrap()
    }

    /// 2 mm per character, so widths are easy to follow.
    fn measure(text: &str) -> Length {
        #[allow(clippy::cast_precision_loss)]
        Length::from_mm(2.0 * text.chars().count() as f64)
    }

    #[test]
    fn a_five_by_five_grid_has_its_cells_two_mm_apart_and_every_label() {
        let laid = layout(&recipe(), measure);
        assert_eq!(laid.cells.len(), 25);
        let x0 = 12.0 + 1.0; // the widest of the lane is "mm/min", 12 mm
        assert_eq!(laid.cells[0].min, Point::new(x0, 8.0));
        assert_eq!(laid.cells[1].min.x - laid.cells[0].min.x, 12.0, "10 + 2 mm");
        assert_eq!((laid.cells[5].column, laid.cells[5].row), (0, 1));
        let parts: Vec<_> = laid
            .labels
            .iter()
            .map(|l| (l.part, l.text.as_str()))
            .collect();
        assert_eq!(
            &parts[..4],
            [
                (LabelPart::Id, "K7Q2M"),
                (LabelPart::ColumnTitle, "Power %"),
                (LabelPart::RowTitle, "mm/min"),
                (LabelPart::ColumnValue, "20"),
            ]
        );
        assert_eq!(parts[8], (LabelPart::RowValue, "600"));
        assert_eq!(parts[9], (LabelPart::RowValue, "1950"));
        assert_eq!(laid.labels.len(), 3 + 5 + 5);
        assert_eq!(laid.width.as_mm(), x0 + 5.0 * 12.0 - 2.0);
        assert_eq!(laid.height.as_mm(), 8.0 + 5.0 * 12.0 - 2.0);
    }

    #[test]
    fn a_long_column_title_widens_a_narrow_grid() {
        let mut value = recipe().to_value();
        value["columns"] = json!({ "param": "passes", "min": 1, "max": 2, "count": 2 });
        value["fixed"] = json!({ "power_pct": 50.0 });
        value["cell_mm"] = json!(3.0);
        let laid = layout(&Recipe::from_value(&value).unwrap(), measure);
        let x0 = 13.0;
        assert_eq!(
            laid.width.as_mm(),
            x0 + 12.0,
            "\"Passes\" is 12 mm, the cells 8 mm"
        );
    }

    #[test]
    fn label_parts_read_their_texts_only() {
        for part in LabelPart::ALL {
            assert_eq!(LabelPart::parse(part.as_str()), Some(part));
        }
        assert_eq!(LabelPart::parse("power"), None);
    }
}
