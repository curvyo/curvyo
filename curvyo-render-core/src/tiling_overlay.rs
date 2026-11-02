//! The circle guide of an open circle tiling (`specs/0069-tiling` UX notes §12.4): its dashed
//! arc in `--preview-new` at 50 %, the dotted rest, and the accent centre and radius handles and
//! the start and sweep knobs above it.

use curvyo_document_core::{Point, Vec2, ViewTransform};

use crate::glyphs::{self, DrawList};
use crate::theme;

/// The dashed path: 1 px, `--preview-new` at 50 %, dashes 4 px with 3 px gaps.
const PATH_PX: f64 = 1.0;
const PATH_COLOUR: crate::RgbaColor = theme::PREVIEW_NEW.with_alpha(128);
/// The unswept rest of the circle: dotted 1 px at 20 %.
const REST_COLOUR: crate::RgbaColor = theme::PREVIEW_NEW.with_alpha(51);
const DOT_PX: f64 = 1.0;
const DOT_GAP_PX: f64 = 3.0;
/// A rest shorter than this (radians) is not drawn: the sweep is a full turn.
const FULL_TURN_TOLERANCE_RAD: f64 = 1e-9;
/// More dashes than this draw the path solid (a dash count in the millions would stall a frame).
const MAX_DASHES: f64 = 4096.0;
/// Pieces per drawn turn: the polygon that stands for the circle.
const PIECES_PER_TURN: f64 = 96.0;
/// The centre handle: a 12 px ring with a cross; the radius handle: a 10 px square.
const CENTRE_PX: f64 = 12.0;
const RADIUS_SQUARE_PX: f64 = 10.0;
/// The handles' accent line and its white casing on each side.
const HANDLE_LINE_PX: f64 = 2.0;
const CASING_PX: f64 = 1.0;

/// The point at `angle` (counterclockwise on screen) on the circle.
fn on_circle(centre: Point, radius: f64, angle: f64) -> Point {
    Point::new(
        radius.mul_add(angle.cos(), centre.x),
        radius.mul_add(-angle.sin(), centre.y),
    )
}

/// The arc from `from` to `to` (radians) as straight pieces.
fn arc(
    list: &mut DrawList,
    (centre, radius): (Point, f64),
    (from, to): (f64, f64),
    (width, colour): (f64, crate::RgbaColor),
) {
    let pieces = ((to - from) / std::f64::consts::TAU * PIECES_PER_TURN)
        .ceil()
        .max(1.0);
    let step = (to - from) / pieces;
    let mut at = from;
    let mut previous = on_circle(centre, radius, at);
    // invariant: `pieces` is at least 1 and at most `PIECES_PER_TURN` per dash or turn.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    for _ in 0..pieces as u32 {
        at += step;
        let next = on_circle(centre, radius, at);
        list.extend(glyphs::thick_line(previous, next, width, colour));
        previous = next;
    }
}

/// The arc from `from` to `to` (radians, `from` < `to`) in dashes of `dash_px` with gaps of
/// `gap_px`; solid when the dashes would be too many.
fn dashed_arc(
    list: &mut DrawList,
    (centre, radius): (Point, f64),
    (from, to): (f64, f64),
    (dash_px, gap_px, colour): (f64, f64, crate::RgbaColor),
    px: f64,
) {
    let (dash, gap) = (dash_px * px / radius, gap_px * px / radius);
    let dashes = ((to - from) / (dash + gap)).floor();
    if !dashes.is_finite() || dashes > MAX_DASHES || dashes < 1.0 {
        arc(list, (centre, radius), (from, to), (PATH_PX * px, colour));
        return;
    }
    // invariant: `dashes` is a whole number from 1 to `MAX_DASHES`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    for i in 0..dashes as u32 {
        let at = f64::from(i).mul_add(dash + gap, from);
        arc(
            list,
            (centre, radius),
            (at, at + dash),
            (PATH_PX * px, colour),
        );
    }
}

/// The centre handle: a ring with a cross, accent on its white casing.
fn centre_handle(list: &mut DrawList, centre: Point, px: f64) {
    let arm = Vec2::new(CENTRE_PX / 2.0 * px, 0.0);
    let up = Vec2::new(0.0, CENTRE_PX / 2.0 * px);
    for (width, colour) in [
        (HANDLE_LINE_PX + 2.0 * CASING_PX, theme::SELECTION_CASING),
        (HANDLE_LINE_PX, theme::ACCENT),
    ] {
        let diameter = (CENTRE_PX + width - HANDLE_LINE_PX) * px;
        list.extend(glyphs::ring(centre, diameter, width * px, colour));
        for d in [arm, up] {
            list.extend(glyphs::thick_line(
                centre.translated(d.negated()),
                centre.translated(d),
                width * px,
                colour,
            ));
        }
    }
}

/// The radius handle: a white square with an accent outline on its casing.
fn radius_handle(list: &mut DrawList, at: Point, px: f64) {
    let half = RADIUS_SQUARE_PX / 2.0 * px;
    let corners = [
        Point::new(at.x - half, at.y - half),
        Point::new(at.x + half, at.y - half),
        Point::new(at.x + half, at.y + half),
        Point::new(at.x - half, at.y + half),
    ];
    list.extend(glyphs::square(
        at,
        (RADIUS_SQUARE_PX + 2.0 * CASING_PX) * px,
        theme::SELECTION_CASING,
    ));
    list.extend(glyphs::quad_outline(
        corners,
        HANDLE_LINE_PX * px,
        theme::ACCENT,
    ));
}

/// How a start or sweep knob is drawn (the "Parameter handle" states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KnobState {
    /// White ground.
    #[default]
    Idle,
    /// The pointer is over it: `--accent-hover` ground.
    Hovered,
    /// Being dragged, or its entry box is open: solid `--accent` ground.
    Active,
}

/// The circle of an open circle tiling as the canvas draws it (`specs/0069-tiling` UX notes
/// §12.4), angles in radians counterclockwise on screen, points in document space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TilingCircleGuide {
    /// The circle's centre.
    pub centre: Point,
    /// The radius, millimetres.
    pub radius_mm: f64,
    /// The start angle.
    pub start: f64,
    /// The swept angle, -2π to 2π.
    pub sweep: f64,
    /// Where the radius handle sits (the middle of the arc).
    pub radius_knob: Point,
    /// Where the sweep knob sits (14 px before the start at a full turn).
    pub sweep_knob: Point,
    /// The look of the start knob.
    pub start_state: KnobState,
    /// The look of the sweep knob.
    pub sweep_state: KnobState,
}

/// The swept arc and the unswept rest as increasing angle ranges.
fn arc_ranges(start: f64, sweep: f64) -> ((f64, f64), (f64, f64)) {
    let turn = std::f64::consts::TAU;
    if sweep >= 0.0 {
        ((start, start + sweep), (start + sweep, start + turn))
    } else {
        ((start + sweep, start), (start, start + sweep + turn))
    }
}

/// The circle guide: the swept arc dashed, the unswept rest dotted, below a full turn the two
/// lines from the centre to the start and sweep knobs, then the centre, radius, start and sweep
/// handles, all at constant screen size.
#[must_use]
pub fn build_tiling_circle(guide: &TilingCircleGuide, view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let mut list = DrawList::default();
    let (centre, radius) = (guide.centre, guide.radius_mm);
    let start_knob = on_circle(centre, radius, guide.start);
    if radius.is_finite() && radius > 0.0 {
        let (swept, rest) = arc_ranges(guide.start, guide.sweep);
        let dashes = (theme::LASSO_DASH_PX, theme::LASSO_GAP_PX, PATH_COLOUR);
        dashed_arc(&mut list, (centre, radius), swept, dashes, px);
        if rest.1 - rest.0 > FULL_TURN_TOLERANCE_RAD {
            let dots = (DOT_PX, DOT_GAP_PX, REST_COLOUR);
            dashed_arc(&mut list, (centre, radius), rest, dots, px);
            for knob in [start_knob, guide.sweep_knob] {
                list.extend(glyphs::thick_line(centre, knob, PATH_PX * px, PATH_COLOUR));
            }
        }
    }
    centre_handle(&mut list, centre, px);
    radius_handle(&mut list, guide.radius_knob, px);
    for (at, state) in [
        (start_knob, guide.start_state),
        (guide.sweep_knob, guide.sweep_state),
    ] {
        list.extend(crate::select_decoration::parameter_handle_glyph(
            view,
            at,
            state == KnobState::Active,
            state == KnobState::Hovered,
        ));
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colours(list: &DrawList) -> Vec<crate::RgbaColor> {
        list.triangles.iter().map(|v| v.color).collect()
    }

    fn guide(radius_mm: f64, sweep: f64) -> TilingCircleGuide {
        let centre = Point::new(0.0, 0.0);
        TilingCircleGuide {
            centre,
            radius_mm,
            start: 0.0,
            sweep,
            radius_knob: on_circle(centre, radius_mm, sweep / 2.0),
            sweep_knob: on_circle(centre, radius_mm, sweep),
            start_state: KnobState::Idle,
            sweep_state: KnobState::Active,
        }
    }

    #[test]
    fn the_path_is_dashed_and_the_handles_are_accent() {
        let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
        let list = build_tiling_circle(&guide(100.0, std::f64::consts::TAU), view);
        let drawn = colours(&list);
        assert!(drawn.contains(&PATH_COLOUR));
        assert!(drawn.contains(&theme::ACCENT));
        assert!(!drawn.contains(&REST_COLOUR), "a full turn has no rest");
        // A dashed circle of radius 100 px: about 628 / 7 dashes, far fewer pieces than solid.
        let path = drawn.iter().filter(|c| **c == PATH_COLOUR).count() / 6;
        let dashes = (std::f64::consts::TAU * 100.0 / 7.0).floor();
        assert!(f64::from(u32::try_from(path).unwrap()) >= dashes);
        let solid = build_tiling_circle(&guide(1e9, std::f64::consts::TAU), view);
        assert!(
            colours(&solid).contains(&PATH_COLOUR),
            "too many dashes draw solid"
        );
    }

    #[test]
    fn a_partial_sweep_draws_the_rest_dotted() {
        let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
        for sweep in [2.0, -2.0] {
            let drawn = colours(&build_tiling_circle(&guide(100.0, sweep), view));
            assert!(drawn.contains(&REST_COLOUR));
            assert!(drawn.contains(&PATH_COLOUR));
        }
    }
}
