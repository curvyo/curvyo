//! The Engraved grid fixture: thin lines at a spacing over the work area, through an origin
//! offset (`specs/0093-laser-workbench-shell` criterion 9, `adrs.md` decision 3).

use curvyo_document_core::{DocumentSize, Length, Point};
use curvyo_extension_core::{
    FixtureDrawing, LineSegment, Param, ParamKind, ParamValue, StoredParams,
};

/// The kind id of the Engraved grid inside the laser process.
pub const ENGRAVED_GRID: &str = "engraved_grid";

/// The parameters of the Engraved grid, with their defaults and bounds (UX notes §4).
#[must_use]
pub(crate) fn grid_params() -> Vec<Param> {
    let length = |key, label, min: f64, max: f64, default: f64| Param {
        key,
        label,
        tooltip: label,
        kind: ParamKind::Length {
            min: Length::from_mm(min),
            max: Length::from_mm(max),
        },
        default: ParamValue::Length(Length::from_mm(default)),
    };
    vec![
        length("spacing", "Spacing", 0.5, 1000.0, 10.0),
        length("line_width", "Line width", 0.01, 10.0, 0.1),
        length("origin_x", "Origin X", -10_000.0, 10_000.0, 0.0),
        length("origin_y", "Origin Y", -10_000.0, 10_000.0, 0.0),
    ]
}

/// The grid lines over `work_area`: every vertical line `x = origin_x + k · spacing` and every
/// horizontal line `y = origin_y + k · spacing` that lies within the area, edges included, each
/// across the whole area. At most `work / spacing + 1` lines per axis, bounded by the schema.
#[must_use]
pub fn engraved_grid(params: &StoredParams, work_area: DocumentSize) -> FixtureDrawing {
    let schema = grid_params();
    let value = |index: usize| {
        schema[index]
            .read(params)
            .length()
            .map_or(0.0, Length::as_mm)
    };
    let (spacing, line_width, origin_x, origin_y) = (value(0), value(1), value(2), value(3));
    let (width, height) = (work_area.width.as_mm(), work_area.height.as_mm());
    let mut lines = Vec::new();
    for x in positions(origin_x, spacing, width) {
        lines.push(LineSegment {
            start: Point::new(x, 0.0),
            end: Point::new(x, height),
        });
    }
    for y in positions(origin_y, spacing, height) {
        lines.push(LineSegment {
            start: Point::new(0.0, y),
            end: Point::new(width, y),
        });
    }
    FixtureDrawing {
        lines,
        line_width: Length::from_mm(line_width),
    }
}

/// The positions `origin + k · spacing` within `0..=extent` (a tolerance of 1e-9 mm at the edges).
fn positions(origin: f64, spacing: f64, extent: f64) -> impl Iterator<Item = f64> {
    const EDGE: f64 = 1e-9;
    let first = ((0.0 - origin) / spacing).ceil();
    let last = ((extent - origin) / spacing).floor();
    #[allow(clippy::cast_possible_truncation)] // bounded by the schema: at most 20,001 lines
    let (first, last) = (first as i64, last as i64);
    (first..=last)
        .map(move |k| {
            #[allow(clippy::cast_precision_loss)]
            let at = origin + k as f64 * spacing;
            at
        })
        .filter(move |at| *at >= -EDGE && *at <= extent + EDGE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(w: f64, h: f64) -> DocumentSize {
        DocumentSize::from_mm(w, h)
    }

    #[test]
    fn the_default_grid_covers_the_work_area_every_ten_millimetres() {
        let drawing = engraved_grid(&StoredParams::new(), area(400.0, 300.0));
        assert_eq!(drawing.line_width, Length::from_mm(0.1));
        let vertical = drawing
            .lines
            .iter()
            .filter(|l| l.start.x == l.end.x)
            .count();
        assert_eq!(vertical, 41);
        assert_eq!(drawing.lines.len(), 41 + 31);
        let last = drawing.lines.last().unwrap();
        assert_eq!(
            (last.start, last.end),
            (Point::new(0.0, 300.0), Point::new(400.0, 300.0))
        );
    }

    #[test]
    fn the_origin_offset_shifts_the_lines_and_the_spacing_spaces_them() {
        let params: StoredParams = serde_json::from_value(serde_json::json!({
            "spacing": {"mm": 25.0},
            "origin_x": {"mm": 5.0},
            "origin_y": {"mm": -3.0},
        }))
        .unwrap();
        let drawing = engraved_grid(&params, area(100.0, 50.0));
        let xs: Vec<f64> = drawing
            .lines
            .iter()
            .filter(|l| l.start.x == l.end.x)
            .map(|l| l.start.x)
            .collect();
        assert_eq!(xs, vec![5.0, 30.0, 55.0, 80.0]);
        let ys: Vec<f64> = drawing
            .lines
            .iter()
            .filter(|l| l.start.y == l.end.y)
            .map(|l| l.start.y)
            .collect();
        assert_eq!(ys, vec![22.0, 47.0]);
    }

    #[test]
    fn a_damaged_spacing_reads_as_the_default_and_the_line_count_stays_bounded() {
        let params: StoredParams =
            serde_json::from_value(serde_json::json!({"spacing": {"mm": 0.0}})).unwrap();
        assert_eq!(engraved_grid(&params, area(100.0, 100.0)).lines.len(), 22);
        let dense: StoredParams =
            serde_json::from_value(serde_json::json!({"spacing": {"mm": 0.5}})).unwrap();
        assert_eq!(
            engraved_grid(&dense, area(10_000.0, 10_000.0)).lines.len(),
            2 * 20_001
        );
    }
}
