//! The Select tool's move drag: what a release would commit for a press
//! point, a pointer position and the Shift and Ctrl of that moment
//! (`specs/0010-edit-interaction-polish/adrs.md`, decision 5; criteria 26 to 38).
//! [`MoveDrag::resolve`] is the one resolving function the live preview and
//! the release both call.

use std::collections::HashSet;

use curvyo_document_core::{Document, NodeId, ObjectSnapshot, Point, Vec2};

use super::{SelectDrag, SelectTool};
use crate::anchor_id_minter::AnchorIdMinter;
use crate::modifiers::Modifiers;
use crate::object_bounds::object_outline_bounds;
use crate::object_selection::ObjectSelection;
use crate::transform_commit::{MOVE_EQUAL_EPSILON_MM, commit_move};
use crate::transform_drag::DragOrigin;
use crate::units::Units;

/// Two absolute displacements closer than this (millimetres) are equal: an
/// exact tie of the axis choice keeps the previous axis (criterion 30).
const AXIS_TIE_EPSILON_MM: f64 = 1e-12;

/// One of the document's two axes, the one an axis-locked move runs along
/// (the document's own, never the rotated axes of a rotated object).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// The horizontal axis: the offset is `(Dx, 0)`.
    X,
    /// The vertical axis: the offset is `(0, Dy)`.
    Y,
}

/// What a move drag would commit right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveResolution {
    /// The displacement of the selection, after any axis lock.
    pub offset: Vec2,
    /// Whether the result is a copy of the selection, not a move of it:
    /// Ctrl is down.
    pub copy: bool,
    /// The axis the move is locked to, `None` while Shift is up.
    pub axis: Option<Axis>,
}

/// A move in progress (body or centre handle press).
#[derive(Debug, Clone)]
pub(super) struct MoveDrag {
    pub(super) origin: DragOrigin,
    /// Whether the press was on the centre handle (it then shows its
    /// dragging look).
    pub(super) from_center: bool,
    /// The object a Shift press landed on: a release that never left the dead
    /// zone toggles it (slice 4's Shift-click), a drag that leaves it does not.
    pub(super) pending_toggle: Option<NodeId>,
    /// The object a Shift press landed on while it was not selected: it joins
    /// the selection in the frame the drag leaves the dead zone.
    pub(super) joins: Option<NodeId>,
    /// The axis the lock chose on the latest pointer event: kept only to
    /// break an exact tie.
    pub(super) last_axis: Option<Axis>,
    /// The true bounding box of the objects the drag moves, read once at the press (an object
    /// a Shift press adds is counted in) (`specs/0047-transform-polish/` criterion 33); the
    /// reference box is this box moved by the live offset.
    start_bounds: Option<(Point, Point)>,
}

/// `displacement` limited to the axis on which it is larger, chosen again on every call (no
/// latch, no hysteresis); an exact tie keeps `last_axis`, x if there was none. The one rule of
/// an axis-locked move and of an axis-locked segment bend (`0031` criterion 13).
pub(crate) fn lock_axis(displacement: Vec2, last_axis: Option<Axis>) -> (Vec2, Axis) {
    let (across_x, across_y) = (displacement.x.abs(), displacement.y.abs());
    let axis = if (across_x - across_y).abs() <= AXIS_TIE_EPSILON_MM {
        last_axis.unwrap_or(Axis::X)
    } else if across_x > across_y {
        Axis::X
    } else {
        Axis::Y
    };
    let locked = match axis {
        Axis::X => Vec2::new(displacement.x, 0.0),
        Axis::Y => Vec2::new(0.0, displacement.y),
    };
    (locked, axis)
}

impl MoveDrag {
    /// A move pressed at `origin`, whose selection (with `joins`, the object a Shift press
    /// adds when the drag leaves the dead zone) has the tight bounds [`selected_bounds`] finds.
    pub(super) fn new(
        origin: DragOrigin,
        from_center: bool,
        objects: &[ObjectSnapshot],
        selection: &ObjectSelection,
        joins: Option<NodeId>,
    ) -> Self {
        Self {
            origin,
            from_center,
            pending_toggle: None,
            joins,
            last_axis: None,
            start_bounds: selected_bounds(objects, selection, joins),
        }
    }

    /// The offset and copy flag the pointer at `current` would commit with
    /// the modifiers as given, from the press point alone: `D` is the
    /// displacement from the press. With Shift the axis is the one with the
    /// larger `|D|`, chosen again on every call (no latch, no hysteresis;
    /// an exact tie keeps `last_axis`, x if there was none); without it the
    /// offset is `D` itself, with nothing accumulated.
    pub(super) fn resolve(&self, current: Point, modifiers: Modifiers) -> MoveResolution {
        let displacement = self.origin.down_at.vector_to(current);
        let (offset, axis) = if modifiers.shift {
            let (locked, axis) = lock_axis(displacement, self.last_axis);
            (locked, Some(axis))
        } else {
            (displacement, None)
        };
        MoveResolution {
            offset,
            copy: modifiers.ctrl,
            axis,
        }
    }
}

impl SelectTool {
    /// Records the pointer's position and the modifiers for the drag in
    /// flight, so the 3 px dead zone, once left, stays left (criterion 41's "a
    /// drag"), the axis lock remembers its axis for an exact tie, and a Shift
    /// press on an unselected object joins the selection in the frame the move
    /// leaves the dead zone (criterion 29). A no-op when idle.
    pub fn pointer_moved(
        &mut self,
        point: Point,
        modifiers: Modifiers,
        selection: &mut ObjectSelection,
    ) {
        match &mut self.drag {
            SelectDrag::Moving(drag) => {
                drag.origin.note(point);
                if drag.origin.is_active_at(point) {
                    if let Some(id) = drag.joins.take() {
                        selection.add_all(self.units.leaves_with(&id));
                    }
                    drag.last_axis = drag.resolve(point, modifiers).axis.or(drag.last_axis);
                }
            }
            SelectDrag::Transforming(drag) => {
                drag.origin.note(point);
                drag.note_frame(point, modifiers.shift, modifiers.ctrl);
            }
            SelectDrag::GroupTransforming(drag) => {
                drag.origin.note(point);
                drag.note_frame(point, modifiers.shift, modifiers.ctrl);
            }
            SelectDrag::Marquee(_) | SelectDrag::Lasso(_) => self.gesture_pointer_moved(point),
            SelectDrag::None => {}
        }
    }

    /// What the move drag in flight would commit with the pointer at `pointer`
    /// and the modifiers as given: `None` when no move runs or it is still
    /// inside the dead zone. Read live by the preview, the readout and the
    /// origin axes, so a Shift or Ctrl change with the pointer at rest reaches
    /// them (criterion 26).
    #[must_use]
    pub fn live_move(&self, pointer: Point, shift: bool, ctrl: bool) -> Option<MoveResolution> {
        match &self.drag {
            SelectDrag::Moving(drag) if drag.origin.is_active_at(pointer) => {
                Some(drag.resolve(pointer, Modifiers::new(shift, ctrl)))
            }
            SelectDrag::Moving(_)
            | SelectDrag::Transforming(_)
            | SelectDrag::GroupTransforming(_)
            | SelectDrag::Marquee(_)
            | SelectDrag::Lasso(_)
            | SelectDrag::None => None,
        }
    }

    /// The true bounding box of the selection as the move drag in flight would leave it, for the
    /// pale red reference box (`specs/0047-transform-polish/` criterion 33): the tight bounds
    /// of the selected objects before the drag, moved by the live offset (the copy travels with
    /// the same offset). `None` when no move runs, inside the dead zone, or while the offset is
    /// zero (a drag back to its start). The bounds were read at the press; a frame costs a sum.
    #[must_use]
    pub fn live_move_box(&self, pointer: Point, modifiers: Modifiers) -> Option<(Point, Point)> {
        let SelectDrag::Moving(drag) = &self.drag else {
            return None;
        };
        if !drag.origin.is_active_at(pointer) {
            return None;
        }
        let offset = drag.resolve(pointer, modifiers).offset;
        if offset.length() <= MOVE_EQUAL_EPSILON_MM {
            return None;
        }
        let (min, max) = drag.start_bounds?;
        Some((min.translated(offset), max.translated(offset)))
    }

    /// Whether a move (not a handle drag) is in flight, past the dead zone or
    /// not.
    #[must_use]
    pub const fn move_in_flight(&self) -> bool {
        matches!(self.drag, SelectDrag::Moving(_))
    }

    /// The release of a move at `point` with the modifiers of the release
    /// event. A release that never left the dead zone is a click: it toggles
    /// the object a Shift press landed on and writes nothing else. Otherwise
    /// the resolved offset is committed as a move, or, with Ctrl down, as a
    /// copy of every selected object that becomes the selection; a zero offset
    /// (a drag back to the start, a locked axis ending at 0) writes nothing,
    /// with and without Ctrl (criteria 29, 31, 33, 35, 36).
    pub(super) fn finish_move(
        drag: &MoveDrag,
        document: &Document,
        objects: &[ObjectSnapshot],
        selection: &mut ObjectSelection,
        (point, modifiers): (Point, Modifiers),
        (minter, units): (&mut AnchorIdMinter, &Units),
    ) {
        if !drag.origin.is_active_at(point) {
            if let Some(id) = drag.pending_toggle {
                selection.toggle_all(units.leaves_with(&id));
            }
            return;
        }
        // The drag left the dead zone with no `pointer_moved` in between (a
        // flick, a touch): the pressed object still joins the selection.
        if let Some(id) = drag.joins {
            selection.add_all(units.leaves_with(&id));
        }
        let MoveResolution { offset, copy, .. } = drag.resolve(point, modifiers);
        if offset.length() <= MOVE_EQUAL_EPSILON_MM {
            return;
        }
        selection.retain_existing(objects);
        if selection.is_empty() {
            return;
        }
        if let Some(copies) = commit_move(document, selection.ids(), offset, copy, (minter, units))
        {
            selection.set(&copies);
        }
    }
}

/// The tight bounds of the selected objects and `joins` together, `None` when there are none.
fn selected_bounds(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    joins: Option<NodeId>,
) -> Option<(Point, Point)> {
    let selected: HashSet<NodeId> = selection.ids().iter().copied().chain(joins).collect();
    objects
        .iter()
        .filter(|object| selected.contains(&object.id()))
        .map(object_outline_bounds)
        .reduce(|(min, max), (low, high)| {
            (
                Point::new(min.x.min(low.x), min.y.min(low.y)),
                Point::new(max.x.max(high.x), max.y.max(high.y)),
            )
        })
}
