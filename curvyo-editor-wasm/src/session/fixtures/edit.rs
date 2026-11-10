//! The one resolver of a fixture's move, turn and resize, for a drag and a typed entry alike
//! (`specs/0102-laser-fixtures` criteria 14, 18, 22, 23, 37, 38; feature `adrs.md` §6a.5,
//! §6a.6): each returns the fixture's new shape, matrix and moved slot centres. Pure.

use curvyo_document_core::{Affine, Angle, Length, NodeId, Point, Vec2};
use curvyo_laser_core::{ScrewPart, StopPart, board_side_range, screw_diameters, stop_legs};
use curvyo_ui_core::{ExtentSigns, ResizeDirection, resize_local_box_with_prior};

use super::placed::{PlacedFixture, Shape};

/// A fixture's geometry after an edit: what a drag shows and what its release writes.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct FixtureEdit {
    /// The new shape (sizes).
    pub shape: Shape,
    /// The new matrix, frame to page-local mm; always rigid.
    pub matrix: Affine,
    /// The slots whose centre moved, with their new centre in the board's frame.
    pub slots: Vec<(NodeId, Point)>,
}

/// `start` unchanged.
fn same(start: &PlacedFixture) -> FixtureEdit {
    FixtureEdit {
        shape: start.shape.clone(),
        matrix: start.matrix,
        slots: Vec::new(),
    }
}

/// `start` moved by `delta` (page mm, equal to document mm).
pub(super) fn translate(start: &PlacedFixture, delta: Vec2) -> FixtureEdit {
    FixtureEdit {
        matrix: Affine::translation(delta).compose(start.matrix),
        ..same(start)
    }
}

/// `start` turned by `angle` about the point `pivot` of its own frame.
pub(super) fn rotate(start: &PlacedFixture, pivot: Point, angle: Angle) -> FixtureEdit {
    let on_page = start.matrix.apply_point(pivot);
    FixtureEdit {
        matrix: Affine::rotation_about(on_page, angle).compose(start.matrix),
        ..same(start)
    }
}

/// Which side of a box a resize handle moves on one axis: `1` the far side, `-1` the near
/// side, `0` neither.
fn sides(direction: ResizeDirection) -> (f64, f64) {
    use ResizeDirection::{E, N, Ne, Nw, S, Se, Sw, W};
    let x = match direction {
        Ne | E | Se => 1.0,
        Nw | W | Sw => -1.0,
        N | S => 0.0,
    };
    let y = match direction {
        Se | S | Sw => 1.0,
        Ne | N | Nw => -1.0,
        E | W => 0.0,
    };
    (x, y)
}

/// The new frame origin, in the old frame, of a box resized from `(w0, h0)` to `(w, h)` by the
/// handle `direction`: the opposite side stays, or with `centre` the box's centre.
fn resized_origin(
    direction: ResizeDirection,
    (w0, h0): (f64, f64),
    (w, h): (f64, f64),
    centre: bool,
) -> Vec2 {
    let (sx, sy) = sides(direction);
    let axis = |side: f64, start: f64, new: f64| {
        if centre {
            (start - new) / 2.0
        } else if side < 0.0 {
            start - new
        } else {
            0.0
        }
    };
    Vec2::new(axis(sx, w0, w), axis(sy, h0, h))
}

/// A holder or clamp of `width` by `height`, its frame origin moved by `shift` in its own frame:
/// the matrix follows, and each slot keeps its place on the sheet (`adrs.md` §6a.5).
fn board_at(start: &PlacedFixture, width: Length, height: Length, shift: Vec2) -> FixtureEdit {
    let shape = match start.shape {
        Shape::Clamp { .. } => Shape::Clamp { width, height },
        _ => Shape::Board { width, height },
    };
    let slots = if shift.length() > 0.0 {
        start
            .slots
            .iter()
            .map(|s| {
                let c = s.slot.centre;
                (s.id, Point::new(c.x - shift.x, c.y - shift.y))
            })
            .collect()
    } else {
        Vec::new()
    };
    FixtureEdit {
        shape,
        matrix: start.matrix.compose(Affine::translation(shift)),
        slots,
    }
}

/// The width and height of a holder or clamp; `None` for every other kind.
fn board_size(start: &PlacedFixture) -> Option<(f64, f64)> {
    match start.shape {
        Shape::Board { width, height } | Shape::Clamp { width, height } => {
            Some((width.as_mm(), height.as_mm()))
        }
        _ => None,
    }
}

/// A resize drag of a holder or clamp by `direction`, the handle moved by `local` (in its own
/// frame): the rectangle's arithmetic with Shift (from the centre) and Ctrl (proportional); each
/// side stops at 1 and 1,000 mm and never turns the board over.
pub(super) fn resize(
    start: &PlacedFixture,
    direction: ResizeDirection,
    local: Vec2,
    (shift, ctrl): (bool, bool),
) -> FixtureEdit {
    let Some((w0, h0)) = board_size(start) else {
        return same(start);
    };
    let resized = resize_local_box_with_prior(
        Point::new(0.0, 0.0),
        Point::new(w0, h0),
        direction,
        local,
        (shift, ctrl),
        ExtentSigns::default(),
    );
    let range = board_side_range(1.0);
    let w = range.clamp(Length::from_mm(resized.sx * w0)).0;
    let h = range.clamp(Length::from_mm(resized.sy * h0)).0;
    let origin = resized_origin(direction, (w0, h0), (w.as_mm(), h.as_mm()), shift);
    board_at(start, w, h, origin)
}

/// A typed width and height of a holder or clamp from the chip of `direction` (`None`: the S
/// chip, about the centre): each value is set to 1 to 1,000 mm. Returns the edit and, per field,
/// whether its value had to move.
pub(super) fn board_sized(
    start: &PlacedFixture,
    direction: Option<ResizeDirection>,
    width: Length,
    height: Length,
) -> (FixtureEdit, [bool; 2]) {
    let Some((w0, h0)) = board_size(start) else {
        return (same(start), [false; 2]);
    };
    let range = board_side_range(1.0);
    let (w, moved_w) = range.clamp(width);
    let (h, moved_h) = range.clamp(height);
    let origin = resized_origin(
        direction.unwrap_or(ResizeDirection::Se),
        (w0, h0),
        (w.as_mm(), h.as_mm()),
        direction.is_none(),
    );
    (board_at(start, w, h, origin), [moved_w, moved_h])
}

/// A stop's sizes `[leg 1, leg 2, thickness]`; `None` for every other kind.
pub(super) fn stop_sizes(start: &PlacedFixture) -> Option<[Length; 3]> {
    match start.shape {
        Shape::Stop {
            leg_a,
            leg_b,
            thickness,
        } => Some([leg_a, leg_b, thickness]),
        _ => None,
    }
}

/// A stop with `part` set to `value`, within its limits (D24). A leg grows from the outer corner.
/// A thickness keeps the inner edges of both legs (the inner corner) and moves the outer edges,
/// so the frame origin moves by the change on both axes (criterion 23, D23); the legs keep their
/// lengths. Returns whether `value` had to move.
pub(super) fn stop_set(
    start: &PlacedFixture,
    part: StopPart,
    value: Length,
) -> (FixtureEdit, bool) {
    let Some(sizes) = stop_sizes(start) else {
        return (same(start), false);
    };
    let ([leg_a, leg_b, thickness], moved) = stop_legs(part, value, sizes);
    let grown = sizes[2].as_mm() - thickness.as_mm();
    let edit = FixtureEdit {
        shape: Shape::Stop {
            leg_a,
            leg_b,
            thickness,
        },
        matrix: start
            .matrix
            .compose(Affine::translation(Vec2::new(grown, grown))),
        slots: Vec::new(),
    };
    (edit, moved)
}

/// A screw's `(thread, head)`; `None` for every other kind.
pub(super) fn screw_sizes(start: &PlacedFixture) -> Option<(Length, Length)> {
    match start.shape {
        Shape::Screw { thread, head } => Some((thread, head)),
        _ => None,
    }
}

/// A screw with `part` set to `value`: the thread never above the head (D26); the centre stays.
/// Returns whether `value` had to move.
pub(super) fn screw_set(
    start: &PlacedFixture,
    part: ScrewPart,
    value: Length,
) -> (FixtureEdit, bool) {
    let Some((thread, head)) = screw_sizes(start) else {
        return (same(start), false);
    };
    let (thread, head, moved) = screw_diameters(part, value, thread, head);
    let edit = FixtureEdit {
        shape: Shape::Screw { thread, head },
        ..same(start)
    };
    (edit, moved)
}

/// A drag of a stop's leg squircle or thickness knob, or of a screw's knob: `local` is the
/// pointer in the fixture's own frame, `moved` the pointer's move in that frame since the press.
/// A drag stops at its limit (D24).
pub(super) fn knob(
    start: &PlacedFixture,
    handle: super::handles::FixtureHandle,
    local: Point,
    moved: Vec2,
) -> FixtureEdit {
    use super::handles::FixtureHandle;
    let mm = Length::from_mm;
    match (handle, stop_sizes(start)) {
        (FixtureHandle::Leg(StopPart::LegA), Some([a, ..])) => {
            return stop_set(start, StopPart::LegA, mm(a.as_mm() + moved.x)).0;
        }
        (FixtureHandle::Leg(StopPart::LegB), Some([_, b, _])) => {
            return stop_set(start, StopPart::LegB, mm(b.as_mm() + moved.y)).0;
        }
        (FixtureHandle::Thickness, Some([_, _, t])) => {
            // The knob is on the outer edge of leg 1: up (against +y) is thicker.
            return stop_set(start, StopPart::Thickness, mm(t.as_mm() - moved.y)).0;
        }
        _ => {}
    }
    match handle {
        FixtureHandle::Head => screw_set(start, ScrewPart::Head, mm(2.0 * local.x)).0,
        FixtureHandle::Thread => screw_set(start, ScrewPart::Thread, mm(-2.0 * local.x)).0,
        _ => same(start),
    }
}
