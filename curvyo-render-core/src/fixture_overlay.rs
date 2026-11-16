//! The overlays of a laser fixture while it is selected, dragged, placed or fitted
//! (`specs/0102-laser-fixtures` UX notes §7, `specs/0113-fixture-sets-and-pins` UX notes F2).

use curvyo_document_core::{Point, ViewTransform};

use crate::color::RgbaColor;
use crate::fixture_look::{CUE_PX, DASH_PX, SlotOutline, fan, ring_of};
use crate::glyphs::{DrawList, dashed_line, push_thick_line};
use crate::theme;

/// The selected fixture's box (the accent, dashed); its handles are the rectangle's
/// (`build_transform_handles`).
#[must_use]
pub fn build_fixture_box(corners: [Point; 4], view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let mut list = DrawList::default();
    let dash = (DASH_PX.0 * px, DASH_PX.1 * px);
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        list.extend(dashed_line(a, b, 3.0 * px, dash, theme::SELECTION_CASING));
        list.extend(dashed_line(a, b, px, dash, theme::ACCENT));
    }
    list
}

/// The outline a fixture had when its drag began, as the rectangle's drag ghost: a 1.5 px line in
/// `--drag-ghost` over its casing (`specs/0102-laser-fixtures` UX notes §7.1).
#[must_use]
pub fn build_fixture_ghost(segments: &[(Point, Point)], view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let width = theme::LIVE_PREVIEW_STROKE_PX * px;
    let mut list = DrawList::default();
    for &(a, b) in segments {
        push_thick_line(&mut list, a, b, width, theme::DRAG_GHOST);
    }
    for &(a, b) in segments {
        push_thick_line(&mut list, a, b, width + 2.0 * px, theme::SELECTION_CASING);
    }
    list
}

/// A fixture that is not placed yet, as the dashed live preview: the hanging job fixture and the
/// Apply preview, 1.5 px `--preview-new` dashed 4 on 3 (`specs/0113-fixture-sets-and-pins` UX
/// notes F2). Not hit.
#[must_use]
pub fn build_fixture_preview(segments: &[(Point, Point)], view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let width = theme::LIVE_PREVIEW_STROKE_PX * px;
    let dash = (DASH_PX.0 * px, DASH_PX.1 * px);
    let mut list = DrawList::default();
    let mut phase = 0.0;
    let mut previous_end: Option<Point> = None;
    for &(a, b) in segments {
        let joined = previous_end.is_some_and(|p| p.vector_to(a).length() <= JOIN_MM);
        if !joined {
            phase = 0.0;
        }
        phase = push_dashed_side(&mut list, (a, b), phase, width, dash, theme::PREVIEW_NEW);
        previous_end = Some(b);
    }
    list
}

/// Two sides whose ends lie closer than this, in mm, join: the dash runs on from one to the next.
const JOIN_MM: f64 = 1e-9;

/// Appends the dashes of the side `a` to `b` to `list`, the pattern `(on, off)` starting `phase`
/// into it, and returns the phase at `b`, so a dash runs on round a corner.
fn push_dashed_side(
    list: &mut DrawList,
    (a, b): (Point, Point),
    mut phase: f64,
    width: f64,
    (on, off): (f64, f64),
    color: RgbaColor,
) -> f64 {
    let side = a.vector_to(b).length();
    if !(side.is_finite() && side > 0.0 && on + off > 0.0) {
        return phase;
    }
    let step = a.vector_to(b).scaled(1.0 / side);
    let mut at = 0.0;
    while at < side {
        let in_dash = phase < on;
        let left = if in_dash {
            on - phase
        } else {
            on + off - phase
        };
        // `used` moves the phase on exactly, so the walk always reaches the next dash or gap.
        let used = left.min(side - at);
        let end = (at + used).min(side);
        if in_dash {
            push_thick_line(
                list,
                a.translated(step.scaled(at)),
                a.translated(step.scaled(end)),
                width,
                color,
            );
        }
        phase = (phase + used) % (on + off);
        at = end;
    }
    phase
}

/// A slot lit as the fit target (`--accent-hover` fill, a 2 px accent outline over its casing),
/// or with `fill` false a selected slot's outline only.
#[must_use]
pub fn build_slot_cue(outline: &SlotOutline, fill: bool, view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let polygon = outline.polygon();
    let mut list = DrawList::default();
    if fill {
        fan(&mut list, &polygon, theme::ACCENT_HOVER);
    }
    ring_of(
        &mut list,
        &polygon,
        CUE_PX * 3.0 * px,
        theme::SELECTION_CASING,
    );
    ring_of(&mut list, &polygon, CUE_PX * px, theme::ACCENT);
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> ViewTransform {
        ViewTransform::identity()
    }

    /// The drawn length of `list`: its triangles' area over the line width.
    fn drawn_length(list: &DrawList, width: f64) -> f64 {
        let area: f64 = list
            .triangles
            .chunks(3)
            .map(|t| {
                let (a, b, c) = (t[0].position, t[1].position, t[2].position);
                ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0
            })
            .sum();
        area / width
    }

    #[test]
    fn a_previewed_circle_keeps_its_dash_pattern_around_the_whole_ring() {
        // A 10 mm pin at 1 px per mm: each of its 48 sides is shorter than one dash and gap,
        // so a pattern restarted on every side would read solid (UX notes F2, criterion 34).
        let ring = SlotOutline::Circle {
            centre: Point::new(50.0, 50.0),
            diameter_mm: 10.0,
        }
        .polygon();
        let sides: Vec<(Point, Point)> = (0..ring.len())
            .map(|i| (ring[i], ring[(i + 1) % ring.len()]))
            .collect();
        let around: f64 = sides.iter().map(|(a, b)| a.vector_to(*b).length()).sum();
        let list = build_fixture_preview(&sides, view());
        let drawn = drawn_length(&list, theme::LIVE_PREVIEW_STROKE_PX);
        let on = DASH_PX.0 / (DASH_PX.0 + DASH_PX.1);
        assert!(
            (drawn / around - on).abs() < 0.1,
            "drawn {drawn:.2} of {around:.2} mm"
        );
    }

    #[test]
    fn a_previewed_outline_restarts_its_pattern_where_it_jumps() {
        // Two separate sides (two rings of a screw, two pins) each start with a dash.
        let first = (Point::new(0.0, 0.0), Point::new(2.0, 0.0));
        let second = (Point::new(0.0, 10.0), Point::new(2.0, 10.0));
        let list = build_fixture_preview(&[first, second], view());
        let drawn = drawn_length(&list, theme::LIVE_PREVIEW_STROKE_PX);
        assert!((drawn - 4.0).abs() < 1e-6, "drawn {drawn}");
    }
}
