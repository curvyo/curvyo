//! Lays out a horizontal or vertical distance: the dimension line beyond the outer point.
use super::{
    BESIDE_TEXT_PX, DimensionMarks, EXTENSION_GAP_PX, EXTENSION_OVERRUN_PX, FIRST_RANK_PX,
    INSIDE_ARROWS_MIN_PX, RANK_STEP_PX, at, dot, linear_frame, span_marks, unit,
};
use curvyo_document_core::{Point, Vec2};

/// A distance along `direction`: the dimension line stands beyond the outer of the two points
/// (above for X, left for Y, like a linear one), each extension line runs from its own point.
pub(super) fn projected(
    (from, to): (Point, Point),
    direction: Vec2,
    label: Option<Point>,
    rank: usize,
    px: f64,
) -> DimensionMarks {
    let reference = from.translated(direction);
    let (along, normal, _) = linear_frame(from, reference);
    // Heights of the two points across the measured direction, from `from`.
    let rise = dot(from.vector_to(to), normal);
    #[allow(clippy::cast_precision_loss)] // a rank is a small count
    let level = label.map_or(
        rise.max(0.0) + (FIRST_RANK_PX + RANK_STEP_PX * rank as f64) * px,
        |l| dot(from.vector_to(l), normal),
    );
    let a = at(from, normal, level);
    let b = at(to, normal, level - rise);
    let mut lines = Vec::new();
    for (base, height) in [(from, level), (to, level - rise)] {
        if height.abs() > EXTENSION_GAP_PX * px {
            let side = if height < 0.0 { -1.0 } else { 1.0 };
            lines.push((
                at(base, normal, side * EXTENSION_GAP_PX * px),
                at(base, normal, height + side * EXTENSION_OVERRUN_PX * px),
            ));
        }
    }
    let span = a.vector_to(b);
    let length = span.length();
    let line_along = unit(span, along);
    let mut text = label.map_or_else(
        || {
            if length / px < INSIDE_ARROWS_MIN_PX {
                at(b, line_along, BESIDE_TEXT_PX * px)
            } else {
                at(a, line_along, length / 2.0)
            }
        },
        |l| at(a, line_along, dot(a.vector_to(l), line_along)),
    );
    let t = dot(a.vector_to(text), line_along);
    let arrows = span_marks(&mut lines, (a, b), (line_along, normal), (length, t), px);
    if !text.x.is_finite() || !text.y.is_finite() {
        text = a;
    }
    DimensionMarks {
        lines,
        arrows,
        text,
        rank,
    }
}
