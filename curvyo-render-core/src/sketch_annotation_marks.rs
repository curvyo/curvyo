//! Sketch dimension marks and the solved-geometry stroke, drawn at constant screen size (0066 AC31, AC32).
use crate::{DrawList, glyphs, stroke, theme};
use curvyo_document_core::{PathSnapshot, Point, Vec2, ViewTransform};

/// Line width of an unselected dimension.
const LINE_PX: f64 = 1.0;
/// Line width of a selected dimension.
const SELECTED_LINE_PX: f64 = 1.5;
/// Width of the hover band behind a dimension's lines.
const HOVER_BAND_PX: f64 = 6.0;
/// Arrowhead length and width.
const ARROW_LENGTH_PX: f64 = 8.0;
const ARROW_WIDTH_PX: f64 = 6.0;
/// The reference dash: 4px on, 3px off.
const DASH_ON_PX: f64 = 4.0;
const DASH_OFF_PX: f64 = 3.0;
/// Width of the solved-geometry stroke.
const SOLVED_PX: f64 = 1.5;

/// The marks of one dimension annotation in document coordinates, with its state.
#[derive(Debug, Clone, Copy)]
pub struct SketchDimensionMarks<'a> {
    /// Extension lines, dimension line, leaders and ticks.
    pub lines: &'a [(Point, Point)],
    /// Arrowheads: the tip and the unit direction from the tip toward the tail.
    pub arrows: &'a [(Point, Vec2)],
    /// A reference dimension: grey and dashed instead of red and solid.
    pub reference: bool,
    /// The pointer is over the annotation: an accent band behind its lines.
    pub hovered: bool,
    /// The annotation is selected: every part in the accent, 1.5px.
    pub selected: bool,
}

/// Draws one dimension's lines and arrowheads.
#[must_use]
pub fn build_sketch_dimension(marks: &SketchDimensionMarks<'_>, view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let mut list = DrawList::default();
    if marks.hovered {
        for &(a, b) in marks.lines {
            list.extend(glyphs::thick_line(
                a,
                b,
                HOVER_BAND_PX * px,
                theme::ACCENT_HOVER,
            ));
        }
    }
    let (color, width) = if marks.selected {
        (theme::ACCENT, SELECTED_LINE_PX * px)
    } else if marks.reference {
        (theme::DIM_REFERENCE, LINE_PX * px)
    } else {
        (theme::DIM_DRIVING, LINE_PX * px)
    };
    for &(a, b) in marks.lines {
        if marks.reference {
            list.extend(dashed(a, b, width, px, color));
        } else {
            list.extend(glyphs::thick_line(a, b, width, color));
        }
    }
    for &(tip, back) in marks.arrows {
        let back = back.normalized_to(ARROW_LENGTH_PX * px);
        let side = Vec2::new(-back.y, back.x).normalized_to(ARROW_WIDTH_PX / 2.0 * px);
        let base = tip.translated(back);
        list.extend(glyphs::filled_triangle(
            tip,
            base.translated(side),
            base.translated(side.negated()),
            color,
        ));
    }
    list
}

/// A line of 4px dashes with 3px gaps.
fn dashed(from: Point, to: Point, width: f64, px: f64, color: crate::RgbaColor) -> DrawList {
    let mut list = DrawList::default();
    let length = from.vector_to(to).length();
    if !length.is_finite() || length <= 0.0 {
        return list;
    }
    let step = from.vector_to(to).scaled(1.0 / length);
    let mut at = 0.0;
    while at < length {
        let end = (at + DASH_ON_PX * px).min(length);
        list.extend(glyphs::thick_line(
            from.translated(step.scaled(at)),
            from.translated(step.scaled(end)),
            width,
            color,
        ));
        at += (DASH_ON_PX + DASH_OFF_PX) * px;
    }
    list
}

/// Draws fully constrained sketch geometry in `--sketch-solved` over its artwork stroke.
#[must_use]
pub fn build_sketch_solved(path: &PathSnapshot, view: ViewTransform) -> DrawList {
    let mut list = DrawList::default();
    for outline in path.subpaths() {
        list.extend(stroke::path_stroke(
            outline.anchors,
            outline.closed,
            SOLVED_PX / view.scale(),
            theme::SKETCH_SOLVED,
            0.1 / view.scale(),
        ));
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks<'a>(
        lines: &'a [(Point, Point)],
        arrows: &'a [(Point, Vec2)],
        reference: bool,
    ) -> SketchDimensionMarks<'a> {
        SketchDimensionMarks {
            lines,
            arrows,
            reference,
            hovered: false,
            selected: false,
        }
    }
    fn colors(list: &DrawList) -> Vec<crate::RgbaColor> {
        let mut colors: Vec<_> = list.triangles.iter().map(|v| v.color).collect();
        colors.dedup();
        colors
    }

    #[test]
    fn a_driving_dimension_is_solid_red_and_a_reference_one_dashed_grey() {
        let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
        let line = [(Point::new(0.0, 0.0), Point::new(70.0, 0.0))];
        let driving = build_sketch_dimension(&marks(&line, &[], false), view);
        assert_eq!(colors(&driving), [theme::DIM_DRIVING]);
        assert_eq!(driving.triangles.len(), 6, "one quad");
        let reference = build_sketch_dimension(&marks(&line, &[], true), view);
        assert_eq!(colors(&reference), [theme::DIM_REFERENCE]);
        // 70px of 4 on / 3 off: ten dashes.
        assert_eq!(reference.triangles.len(), 10 * 6);
    }

    #[test]
    fn arrowheads_are_eight_by_six_pixels_at_every_zoom() {
        for scale in [0.25, 1.0, 8.0] {
            let view = ViewTransform::new(scale, Point::new(0.0, 0.0));
            let arrow = [(Point::new(0.0, 0.0), Vec2::new(1.0, 0.0))];
            let list = build_sketch_dimension(&marks(&[], &arrow, false), view);
            let xs: Vec<f64> = list
                .triangles
                .iter()
                .map(|v| v.position.x * scale)
                .collect();
            let ys: Vec<f64> = list
                .triangles
                .iter()
                .map(|v| v.position.y * scale)
                .collect();
            let span = |v: &[f64]| {
                v.iter().copied().fold(f64::MIN, f64::max)
                    - v.iter().copied().fold(f64::MAX, f64::min)
            };
            assert!(
                (span(&xs) - 8.0).abs() < 1e-9 && (span(&ys) - 6.0).abs() < 1e-9,
                "{scale}"
            );
        }
    }

    #[test]
    fn hover_adds_a_band_and_selection_uses_the_accent() {
        let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
        let line = [(Point::new(0.0, 0.0), Point::new(10.0, 0.0))];
        let mut state = marks(&line, &[], false);
        state.hovered = true;
        state.selected = true;
        assert_eq!(
            colors(&build_sketch_dimension(&state, view)),
            [theme::ACCENT_HOVER, theme::ACCENT]
        );
    }
}
