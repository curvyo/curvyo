//! The overhang notice: how far the placed content extends past the work area (`specs/0093`
//! criterion 8, UX notes §6).

use curvyo_extension_core::NoticeContext;

/// The id of the overhang `Notice` block.
pub const OVERHANG_NOTICE: &str = "overhang";

/// An axis overhangs when its overhang is above this many millimetres (the geometric tolerance of
/// UX notes §6).
pub const OVERHANG_TOLERANCE: f64 = 0.01;

/// "Extends past the work area by 40 x 12 mm", the sums of the overhang past both edges of each
/// axis, one decimal at most; `None` while nothing extends past the area. An axis past the
/// tolerance reads at least 0.1, so a shown notice never says "0 x 0 mm".
#[must_use]
pub fn overhang_text(context: &NoticeContext) -> Option<String> {
    let (min, max) = context.placed_bounds?;
    let past = |low: f64, high: f64, extent: f64| (-low).max(0.0) + (high - extent).max(0.0);
    let x = past(min.x, max.x, context.work_area.width.as_mm());
    let y = past(min.y, max.y, context.work_area.height.as_mm());
    let shown = |mm: f64| {
        if mm > OVERHANG_TOLERANCE {
            mm.max(0.1)
        } else {
            0.0
        }
    };
    let (x, y) = (shown(x), shown(y));
    if x == 0.0 && y == 0.0 {
        return None;
    }
    Some(format!(
        "Extends past the work area by {} x {} mm",
        one_decimal(x),
        one_decimal(y)
    ))
}

/// `mm` with one decimal at most, a trailing ".0" dropped.
fn one_decimal(mm: f64) -> String {
    let text = format!("{mm:.1}");
    text.strip_suffix(".0").map_or(text.clone(), str::to_owned)
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{DocumentSize, Point};

    use super::*;

    fn context(min: (f64, f64), max: (f64, f64)) -> NoticeContext {
        NoticeContext {
            work_area: DocumentSize::from_mm(400.0, 300.0),
            placed_bounds: Some((Point::new(min.0, min.1), Point::new(max.0, max.1))),
        }
    }

    #[test]
    fn content_inside_the_area_has_no_notice() {
        assert_eq!(overhang_text(&context((0.0, 0.0), (400.0, 300.0))), None);
        assert_eq!(overhang_text(&context((-0.005, 0.0), (400.0, 300.0))), None);
        let empty = NoticeContext {
            placed_bounds: None,
            ..context((0.0, 0.0), (1.0, 1.0))
        };
        assert_eq!(overhang_text(&empty), None);
    }

    #[test]
    fn the_notice_names_the_overhang_per_axis() {
        assert_eq!(
            overhang_text(&context((0.0, 0.0), (440.0, 312.0))).as_deref(),
            Some("Extends past the work area by 40 x 12 mm")
        );
        assert_eq!(
            overhang_text(&context((-2.25, 0.0), (400.0, 10.0))).as_deref(),
            Some("Extends past the work area by 2.2 x 0 mm")
        );
        assert_eq!(
            overhang_text(&context((0.0, 0.0), (400.04, 300.0))).as_deref(),
            Some("Extends past the work area by 0.1 x 0 mm")
        );
    }
}
