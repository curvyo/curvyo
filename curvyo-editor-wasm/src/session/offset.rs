//! The Offset tool's corner-handle drag in the session (`specs/0038-path-offset` criteria 2, 5 to
//! 27, 52, 55, 56): from the remembered values to one commit on release or on the chip's Enter.
//!
//! The preview is computed once per frame from the current values and cached by document
//! version, selection and request, so what a release writes is what the last frame showed
//! (criterion 18).

use std::cell::RefCell;

use curvyo_document_core::{Angle, DocumentVersion, NodeId, Point};
use curvyo_ui_core::{
    CORNER_SIGNS, HANDLE_HIT_RADIUS_PX, Modifiers, ObjectSelection, OffsetDirection, OffsetJoin,
    OffsetPlan, OffsetRefusal, OffsetRequest, OffsetResultChoice, OffsetSide, OffsetSides,
    OffsetValues, OrientedBox, PressTarget, classify_press, drag_distance, handle_positions,
    hit_test_objects, offset_box, offset_operands, plan_offset, snap_distance,
};

use super::refusal::RefusalMarks;
use super::{Session, Tool};

/// The label of the commit an offset makes (criterion 16).
pub const OFFSET_COMMIT_LABEL: &str = "offset_path";

/// How far, in screen pixels, the pointer moves from a corner press before the drag preview takes
/// over from the standing preview (`ux-rework.md` section 3).
const DEAD_ZONE_PX: f64 = 3.0;

/// The Offset tool's state: the remembered values (session memory, criterion 7), the drag in
/// flight, the handle under the pointer, the corner style set for the next drag and the cached
/// preview.
#[derive(Debug, Default)]
pub(super) struct OffsetTool {
    pub(super) values: OffsetValues,
    pub(super) drag: Option<OffsetDrag>,
    pub(super) hovered: Option<usize>,
    /// The corner style a double-click on a corner handle set for the next written drag
    /// (criterion 56).
    pub(super) pending_join: Option<OffsetJoin>,
    /// The pointer is over the center handle.
    pub(super) center_hovered: bool,
    /// The Offset entry chip is open (criterion 53).
    pub(super) chip_open: bool,
    outcome: Option<OffsetOutcome>,
    preview: RefCell<Option<Cached>>,
}

/// A handle drag in flight.
#[derive(Debug, Clone, Copy)]
pub(super) struct OffsetDrag {
    pub(super) corner: usize,
    start: Point,
    angle: Angle,
    /// The signed, snapped, stopped distance: positive outward.
    pub(super) d: f64,
    /// The last distance at which something was left.
    last_good: f64,
    /// The pointer has left the dead zone around the press: the drag preview has taken over
    /// from the standing preview.
    pub(super) moved: bool,
    pub(super) stop: Option<DragStop>,
}

/// Why the handle stopped (criterion 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragStop {
    /// Further in, the whole selection vanishes.
    NothingLeft,
}

/// One computed plan and what it was computed from.
#[derive(Debug, Clone)]
struct Cached {
    version: DocumentVersion,
    selection: Vec<NodeId>,
    request: OffsetRequest,
    result: Result<OffsetPlan, OffsetRefusal>,
}

/// What a release or the chip's Enter did (criterion 24).
#[derive(Debug, Clone, PartialEq)]
pub enum OffsetOutcome {
    /// Nothing: no drag, a distance of 0, or no operand.
    Ignored,
    /// Written in one commit; the results are selected.
    Applied {
        /// New objects.
        added: usize,
        /// Changed originals.
        changed: usize,
        /// The unsigned distance in millimetres.
        distance_mm: f64,
        /// Outward, inward or both sides.
        direction: OffsetDirection,
        /// Operands whose inset vanished.
        vanished: usize,
        /// Operands.
        of: usize,
    },
    /// Refused; nothing was written. Offenders are outlined.
    Refused {
        /// Why.
        refusal: OffsetRefusal,
        /// The unsigned distance, for the sentence of an empty inset.
        distance_mm: f64,
    },
    /// The selection holds a group (criterion 32); its leaves are outlined.
    RefusedGroups {
        /// How many leaves the selected groups hold.
        leaves: usize,
    },
}

impl Session {
    /// The remembered values (criterion 7).
    #[must_use]
    pub const fn offset_values(&self) -> OffsetValues {
        self.offset.values
    }

    /// Sets the Distance (criterion 6); returns whether it was within 0.01..1000 mm.
    pub fn set_offset_distance(&mut self, distance_mm: f64) -> bool {
        self.offset.values.set_distance_mm(distance_mm)
    }

    /// Sets the Direction.
    pub fn set_offset_direction(&mut self, direction: OffsetDirection) {
        self.offset.values.set_direction(direction);
    }

    /// Sets the Corners.
    pub fn set_offset_join(&mut self, join: OffsetJoin) {
        self.offset.values.set_join(join);
    }

    /// Sets the Mode.
    pub fn set_offset_result(&mut self, result: OffsetResultChoice) {
        self.offset.values.set_result(result);
    }

    /// What the last release did, once (the host shows the notice).
    pub fn take_offset_outcome(&mut self) -> Option<OffsetOutcome> {
        self.offset.outcome.take()
    }

    /// The selection's operands, the leaves of selected groups left out.
    pub(super) fn offset_selection(&self) -> ObjectSelection {
        let in_groups = self.selected_group_leaves();
        let mut selection = self.selection.clone();
        if !in_groups.is_empty() {
            let ids: Vec<NodeId> = selection
                .ids()
                .iter()
                .copied()
                .filter(|id| !in_groups.contains(id))
                .collect();
            selection.set(&ids);
        }
        selection
    }

    /// The operand box the handles sit on: over the closed operands only (criterion 55).
    pub(super) fn offset_box_now(&self) -> Option<OrientedBox> {
        let objects = self.objects();
        offset_box(&offset_operands(&objects, &self.offset_selection()))
    }

    /// Whether the handles are drawn: the Offset tool with a closed operand selected (a group
    /// alone or only open paths draw none, criteria 5, 55).
    pub(super) fn offset_handles_shown(&self) -> bool {
        self.tool == Tool::Offset && self.offset_box_now().is_some()
    }

    /// The handle at `point`, if the handles are drawn.
    pub(super) fn offset_handle_at(&self, point: Point) -> Option<usize> {
        if !self.offset_handles_shown() {
            return None;
        }
        let view = self.view();
        let bounds = self.offset_box_now()?;
        let reach = HANDLE_HIT_RADIUS_PX / view.scale();
        handle_positions(&bounds, view.scale())
            .iter()
            .position(|handle| handle.vector_to(point).length() <= reach)
    }

    /// The request of a drag at the signed `d` with `held` keys, or of the remembered values
    /// (the standing preview and the chip's Enter). The corner style is the pending one of a
    /// double-click, else the remembered Corners (criterion 56).
    pub(super) fn offset_request(&self, signed: Option<(f64, Modifiers)>) -> OffsetRequest {
        let values = &self.offset.values;
        let (distance_mm, sides) = match signed {
            Some((d, held)) => {
                let dragged = if d < 0.0 {
                    OffsetSide::In
                } else {
                    OffsetSide::Out
                };
                (
                    d.abs(),
                    OffsetSides::of(dragged, held.shift, values.changes_original(held.ctrl)),
                )
            }
            None => (values.distance_mm(), values.entry_sides()),
        };
        OffsetRequest {
            distance_mm,
            sides,
            join: self.offset_join(),
        }
    }

    /// The plan for `request`, from the cache when nothing changed.
    pub(super) fn offset_plan_for(
        &self,
        request: OffsetRequest,
    ) -> Result<OffsetPlan, OffsetRefusal> {
        let version = self.document.version();
        let selection = self.offset_selection();
        if let Some(cached) = self.offset.preview.borrow().as_ref()
            && cached.version == version
            && cached.request == request
            && cached.selection == selection.ids()
        {
            return cached.result.clone();
        }
        let result = plan_offset(&self.objects(), &selection, &request);
        self.offset.preview.replace(Some(Cached {
            version,
            selection: selection.ids().to_vec(),
            request,
            result: result.clone(),
        }));
        result
    }

    /// A press in the Offset tool: on a handle a drag begins; on an object it is selected
    /// (Shift adds or removes) and nothing moves; on empty canvas a marquee begins (criterion 27).
    pub(super) fn offset_pointer_down(&mut self, point: Point, shift: bool) {
        self.command_refusal = None;
        // A press outside the chip closes it and is not swallowed (`ux-rework.md` 2.8).
        self.offset.chip_open = false;
        // The center handle claims the press: no marquee, no deselect, no move (criterion 53).
        if self.offset_center_at(point) {
            return;
        }
        if let Some(corner) = self.offset_handle_at(point)
            && let Some(bounds) = self.offset_box_now()
        {
            self.offset.drag = Some(OffsetDrag {
                corner,
                start: point,
                angle: bounds.angle,
                d: 0.0,
                last_good: 0.0,
                moved: false,
                stop: None,
            });
            return;
        }
        let objects = self.objects();
        let tolerance = self.object_tolerance();
        if let Some(&hit) = hit_test_objects(&objects, point, tolerance).first() {
            let units = self.units();
            let members = units.leaves_with(&hit);
            if shift {
                self.selection.toggle_all(members);
            } else if !self.selection.contains(hit) {
                self.selection.set(members);
            }
            return;
        }
        // The Select tool's marquee, armed against no selection: its handles are not drawn in
        // this tool, so they must not catch the press. The release combines with the real one.
        let mut nothing = ObjectSelection::new();
        let modifiers = Modifiers { shift, ..self.held };
        let handle_tolerances = self.transform_handle_tolerances();
        let target = classify_press(
            &objects,
            &nothing,
            point,
            tolerance,
            handle_tolerances,
            modifiers,
        );
        if matches!(target, PressTarget::Empty | PressTarget::Lasso) {
            self.select.pointer_down(
                &objects,
                &mut nothing,
                point,
                tolerance,
                handle_tolerances,
                modifiers,
            );
        }
    }

    /// The pointer moved in the Offset tool: the handle under it, or the drag's distance.
    pub(super) fn offset_hover(&mut self, point: Point) {
        if self.select.drag_in_flight() {
            self.select_hover(point, self.held);
            return;
        }
        let Some(mut drag) = self.offset.drag else {
            self.offset.hovered = self.offset_handle_at(point);
            self.offset.center_hovered = self.offset_center_at(point);
            return;
        };
        let scale = self.view().scale();
        if !drag.moved {
            // The standing preview stays until the pointer leaves the dead zone; the drag starts
            // from d = 0 at the press point (`ux-rework.md` section 3).
            if drag.start.vector_to(point).length() * scale <= DEAD_ZONE_PX {
                return;
            }
            drag.moved = true;
        }
        let raw = drag_distance(
            drag.angle,
            CORNER_SIGNS[drag.corner],
            drag.start.vector_to(point),
        );
        let mut d = snap_distance(raw, scale, self.offset.values.distance_mm());
        drag.stop = None;
        if d < 0.0 && !self.offset_selection_is_large() {
            match self.offset_plan_for(self.offset_request(Some((d, self.held)))) {
                Err(OffsetRefusal::Empty) => {
                    d = drag.last_good;
                    drag.stop = Some(DragStop::NothingLeft);
                }
                _ => drag.last_good = d,
            }
        }
        drag.d = d;
        self.offset.drag = Some(drag);
    }

    /// The release in the Offset tool: a drag with a distance writes its result in one commit
    /// (criterion 16), with the modifiers of this event (criterion 19); a marquee ends.
    pub(super) fn offset_pointer_up(&mut self, point: Point, shift: bool, ctrl: bool) {
        if self.select.drag_in_flight() {
            self.select_pointer_up(point, shift, ctrl);
            return;
        }
        let Some(drag) = self.offset.drag.take() else {
            return;
        };
        self.offset.hovered = self.offset_handle_at(point);
        // At the inset stop the readout says "Nothing is left": a release there writes nothing
        // (criterion 12).
        if drag.d == 0.0 || drag.stop == Some(DragStop::NothingLeft) {
            return;
        }
        let request = self.offset_request(Some((drag.d, Modifiers::new(shift, ctrl))));
        let direction = if shift {
            OffsetDirection::Both
        } else if drag.d < 0.0 {
            OffsetDirection::In
        } else {
            OffsetDirection::Out
        };
        let outcome = self.write_offset_plan(request, direction);
        if matches!(outcome, OffsetOutcome::Applied { .. }) {
            // The written distance is remembered (criterion 6); the pending corner style was for
            // this drag (criterion 56). A drag never changes Direction (criterion 10).
            self.offset.values.set_distance_mm(drag.d.abs());
            self.offset.pending_join = None;
        }
        self.offset.outcome = Some(outcome);
    }

    /// Escape or the other button during a drag: nothing is written and the remembered values
    /// and a pending corner style stay (criteria 17, 56). Returns whether a drag was cancelled.
    pub fn cancel_offset_drag(&mut self) -> bool {
        self.offset.drag.take().is_some()
    }

    /// The corner style the standing preview and the next drag use: the pending one of a
    /// double-click, else the remembered Corners (criterion 56).
    pub(super) fn offset_join(&self) -> OffsetJoin {
        self.offset
            .pending_join
            .unwrap_or_else(|| self.offset.values.join())
    }

    /// The Offset tool is left: a drag in flight is cancelled and a pending corner style is
    /// dropped (criterion 56).
    pub(super) fn leave_offset_tool(&mut self) {
        self.cancel_offset_drag();
        self.offset.pending_join = None;
        self.offset.chip_open = false;
    }

    /// Whether a drag past 0, not at a stop, is in flight that makes a new object (Ctrl does not change the
    /// original): the move's plus badge shows (criterion 15).
    pub(super) fn offset_drag_makes_copy(&self) -> bool {
        self.offset.drag.is_some_and(|drag| {
            drag.d != 0.0
                && drag.stop.is_none()
                && !self.offset.values.changes_original(self.held.ctrl)
        })
    }

    /// Whether an Offset handle drag is in flight.
    pub(super) const fn offset_drag_in_flight(&self) -> bool {
        self.offset.drag.is_some()
    }

    /// Enter in the Offset entry chip: writes the remembered values for the selection in one
    /// commit (criteria 25, 26, 54).
    pub fn apply_offset(&mut self) -> OffsetOutcome {
        self.recording(|this| {
            if this.tool != Tool::Offset || this.offset.drag.is_some() {
                return OffsetOutcome::Ignored;
            }
            let request = this.offset_request(None);
            let direction = this.offset.values.direction();
            let outcome = this.write_offset_plan(request, direction);
            // A refusal keeps the chip open with its message (`ux-rework.md` 2.7).
            if matches!(outcome, OffsetOutcome::Applied { .. }) {
                this.offset.chip_open = false;
            }
            outcome
        })
    }

    /// Plans `request`, refuses or writes it in one commit, and selects what it produced or
    /// changed (criterion 23).
    fn write_offset_plan(
        &mut self,
        request: OffsetRequest,
        direction: OffsetDirection,
    ) -> OffsetOutcome {
        let in_groups = self.selected_group_leaves();
        if !in_groups.is_empty() {
            let leaves = in_groups.len();
            self.command_refusal = Some(RefusalMarks::new(in_groups, &self.selection));
            return OffsetOutcome::RefusedGroups { leaves };
        }
        let plan = match self.offset_plan_for(request) {
            Ok(plan) => plan,
            Err(OffsetRefusal::ZeroDistance | OffsetRefusal::NoOperands) => {
                return OffsetOutcome::Ignored;
            }
            Err(refusal) => {
                self.command_refusal = match &refusal {
                    OffsetRefusal::OutOfRange { offenders, .. }
                    | OffsetRefusal::NoArea { offenders, .. } => {
                        Some(RefusalMarks::new(offenders.clone(), &self.selection))
                    }
                    OffsetRefusal::NoOperands
                    | OffsetRefusal::ZeroDistance
                    | OffsetRefusal::Empty => None,
                };
                return OffsetOutcome::Refused {
                    refusal,
                    distance_mm: request.distance_mm,
                };
            }
        };
        let (added, changed) = (plan.added(), plan.changed());
        let (vanished, of) = (plan.vanished, plan.of);
        let writes = plan.into_writes(&mut self.minter);
        let Ok(ids) = self.document.write_offset(&writes, OFFSET_COMMIT_LABEL) else {
            // invariant: the plan was computed from the objects of this document version, with
            // fresh anchor ids, so the write cannot fail.
            debug_assert!(false, "a computed offset could not be written");
            return OffsetOutcome::Ignored;
        };
        self.selection.set(&ids);
        self.hovered_object = None;
        OffsetOutcome::Applied {
            added,
            changed,
            distance_mm: request.distance_mm,
            direction,
            vanished,
            of,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{
        AnchorId, Length, NewAnchor, ObjectSnapshot, RectBounds, Shape, Vec2,
    };

    fn rect(session: &Session, x: f64, side: f64) -> NodeId {
        session.document.create_rect(RectBounds {
            origin: Point::new(x, 0.0),
            width: Length::from_mm(side),
            height: Length::from_mm(side),
        })
    }

    fn square_path(session: &Session, x: f64, side: f64, first: u64) -> NodeId {
        let corners = [(x, 0.0), (x + side, 0.0), (x + side, side), (x, side)];
        let anchors: Vec<NewAnchor> = corners
            .into_iter()
            .zip(first..)
            .map(|((cx, cy), n)| NewAnchor::corner(AnchorId::new(9, n), Point::new(cx, cy)))
            .collect();
        session.document.create_path(&anchors, true)
    }

    fn offset_session() -> (Session, NodeId) {
        let mut session = Session::new(1);
        let square = rect(&session, 0.0, 20.0);
        session.set_tool(Tool::Offset);
        session.selection.select_single(square);
        (session, square)
    }

    /// The bottom-right handle of the current selection.
    fn bottom_right(session: &Session) -> Point {
        let bounds = session.offset_box_now().unwrap();
        handle_positions(&bounds, session.view().scale())[2]
    }

    fn along(from: Point, d: f64) -> Point {
        from.translated(Vec2::new(d, d))
    }

    /// A drag of the bottom-right handle by `by` along the diagonal. The default view shows
    /// 1 px per mm, where the snap to the remembered value (6 px) catches every small distance, so
    /// the remembered value is set to the dragged one first.
    fn drag(session: &mut Session, by: f64, shift: bool, ctrl: bool) {
        session.set_offset_distance(by.abs());
        let from = bottom_right(session);
        session.pointer_down(from, shift);
        session.modifiers_changed(shift, ctrl, false);
        let to = along(from, by);
        session.pointer_hover(to, shift, ctrl);
        session.pointer_up(to, shift, ctrl);
        session.modifiers_changed(false, false, false);
    }

    fn rect_width(session: &Session, id: NodeId) -> f64 {
        let Some(ObjectSnapshot::Primitive(primitive)) = session.document.object(id) else {
            panic!("a rectangle");
        };
        let Shape::Rect { bounds, .. } = primitive.shape else {
            panic!("a rectangle");
        };
        bounds.width.as_mm()
    }

    #[test]
    fn ac16_ac20_ac23_a_release_writes_a_copy_below_and_selects_it() {
        let (mut session, square) = offset_session();
        let steps = session.undo_len();
        drag(&mut session, 2.0, false, false);
        assert_eq!(session.undo_len(), steps + 1, "one commit");
        let ids = session.document.object_ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[1], square, "the outset goes below the original");
        assert_eq!(session.selection.ids(), &[ids[0]]);
        assert!(
            (rect_width(&session, ids[0]) - 24.0).abs() < 1e-9,
            "a rectangle stays one"
        );
        assert!(matches!(
            session.take_offset_outcome(),
            Some(OffsetOutcome::Applied {
                added: 1,
                changed: 0,
                ..
            })
        ));
        assert_eq!(session.tool(), Tool::Offset, "the tool stays");
        assert!(
            (session.offset_values().distance_mm() - 2.0).abs() < 1e-9,
            "the written distance is remembered"
        );
    }

    #[test]
    fn ac19_shift_gives_both_sides_and_ctrl_changes_the_original() {
        let (mut session, square) = offset_session();
        drag(&mut session, 2.0, true, false);
        let ids = session.document.object_ids();
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[1], square, "outer below, inner above");
        session.selection.select_single(square);
        drag(&mut session, -2.0, false, true);
        assert_eq!(session.document.object_ids().len(), 3, "Ctrl adds nothing");
        assert_eq!(session.selection.ids(), &[square]);
        assert!((rect_width(&session, square) - 16.0).abs() < 1e-9);
    }

    #[test]
    fn ac12_an_inset_stops_where_something_is_left_and_a_release_there_writes_nothing() {
        let mut session = Session::new(1);
        let small = square_path(&session, 0.0, 4.0, 1);
        session.set_tool(Tool::Offset);
        session.selection.select_single(small);
        let from = bottom_right(&session);
        session.pointer_down(from, false);
        session.pointer_hover(along(from, -1.0), false, false);
        session.pointer_hover(along(from, -5.0), false, false);
        let view = session.offset_view();
        assert_eq!(view.stop, Some(DragStop::NothingLeft));
        assert!(view.distance_mm < 0.0 && view.distance_mm > -2.0);
        // A release at the stop writes nothing (criterion 12).
        session.pointer_up(along(from, -5.0), false, false);
        assert_eq!(session.document.object_ids(), vec![small]);
        assert_eq!(session.take_offset_outcome(), None);
    }

    #[test]
    fn ac17_escape_cancels_the_drag_and_writes_nothing() {
        let (mut session, _) = offset_session();
        let from = bottom_right(&session);
        session.pointer_down(from, false);
        session.pointer_hover(along(from, 3.0), false, false);
        assert!(session.offset_view().dragging);
        assert_eq!(
            session.escape(),
            super::super::keys::EscapeStep::CancelledDrag
        );
        session.pointer_up(from, false, false);
        assert_eq!(session.document.object_ids().len(), 1);
    }

    #[test]
    fn ac10_ac17_a_drag_keeps_the_values_and_a_stop_shows_no_plus() {
        let (mut session, _) = offset_session();
        session.set_offset_distance(1.0);
        let from = bottom_right(&session);
        session.pointer_down(from, false);
        session.pointer_hover(along(from, -3.0), true, false);
        assert!(session.offset_drag_makes_copy());
        assert_eq!(
            session.offset_values(),
            OffsetValues::default(),
            "a drag changes no remembered value"
        );
        assert_eq!(
            session.escape(),
            super::super::keys::EscapeStep::CancelledDrag
        );
        assert_eq!(session.offset_values(), OffsetValues::default());

        let mut small = Session::new(1);
        let square = square_path(&small, 0.0, 4.0, 1);
        small.set_tool(Tool::Offset);
        small.selection.select_single(square);
        let from = bottom_right(&small);
        small.pointer_down(from, false);
        small.pointer_hover(along(from, -5.0), false, false);
        assert_eq!(small.offset_view().stop, Some(DragStop::NothingLeft));
        assert!(!small.offset_drag_makes_copy(), "no plus at the stop");
    }

    #[test]
    fn ac6_ac10_a_written_drag_remembers_its_distance_but_not_its_direction() {
        let (mut session, _) = offset_session();
        let from = bottom_right(&session);
        session.pointer_down(from, false);
        session.pointer_hover(along(from, -3.0), false, false);
        session.pointer_up(along(from, -3.0), false, false);
        assert!(matches!(
            session.take_offset_outcome(),
            Some(OffsetOutcome::Applied {
                direction: OffsetDirection::In,
                ..
            })
        ));
        assert!((session.offset_values().distance_mm() - 3.0).abs() < 1e-9);
        assert_eq!(session.offset_values().direction(), OffsetDirection::Out);
    }

    #[test]
    fn ac52_the_standing_preview_is_what_enter_writes() {
        let mut session = Session::new(1);
        let square = rect(&session, 0.0, 20.0);
        session.selection.select_single(square);
        assert_eq!(session.offset_preview_request(), None, "Select tool: none");
        session.set_tool(Tool::Offset);
        let standing = session
            .offset_preview_request()
            .expect("a preview at once, without a drag");
        assert_eq!(standing, session.offset_request(None));
        assert!(
            (standing.distance_mm - 1.0).abs() < 1e-9,
            "1 mm on first use"
        );
        let plan = session.offset_plan_for(standing).expect("a result");
        assert_eq!(plan.items.len(), 1);
        session.set_offset_direction(OffsetDirection::Both);
        assert_eq!(
            session.offset_preview_request().map(|r| r.sides),
            Some(session.offset_values().entry_sides()),
            "the preview follows the values"
        );
        session.selection.clear();
        assert!(session.offset_plan_for(standing).is_err(), "no operand");
        assert_eq!(
            session.offset_draw_list(session.view()),
            curvyo_render_core::DrawList::default()
        );
    }

    #[test]
    fn ac52_after_a_write_the_preview_stands_for_the_new_selection() {
        let (mut session, square) = offset_session();
        drag(&mut session, 2.0, false, false);
        assert_ne!(session.selection.ids(), &[square]);
        let plan = session
            .offset_plan_for(session.offset_preview_request().unwrap())
            .unwrap();
        assert_eq!(plan.items[0].operand, session.selection.ids()[0]);
    }

    #[test]
    fn the_press_keeps_the_standing_preview_inside_the_dead_zone() {
        let (mut session, _) = offset_session();
        let standing = session.offset_preview_request();
        let from = bottom_right(&session);
        let scale = session.view().scale();
        session.pointer_down(from, false);
        session.pointer_hover(along(from, 1.5 / scale), false, false);
        assert_eq!(session.offset_preview_request(), standing);
        assert!(!session.offset_view().dragging);
        session.pointer_hover(along(from, 10.0 / scale), false, false);
        assert!(session.offset_view().dragging);
        assert_ne!(session.offset_preview_request(), standing);
    }

    #[test]
    fn ac56_a_double_click_steps_the_corner_style_for_the_next_drag() {
        let (mut session, _) = offset_session();
        let corner = bottom_right(&session);
        assert!(!session.double_click(Point::new(500.0, 500.0), false, false));
        assert_eq!(session.offset_view().pending_join, None);
        assert_eq!(
            session.double_click_hint(corner, false, false),
            super::super::DoubleClickHint::OffsetCornerStyle
        );
        assert_eq!(session.offset_view().pending_join, Some(OffsetJoin::Miter));
        assert_eq!(session.offset_join(), OffsetJoin::Miter);
        session.double_click_hint(corner, false, false);
        assert_eq!(session.offset_view().join, OffsetJoin::Bevel);
        assert_eq!(
            session.offset_preview_request().map(|r| r.join),
            Some(OffsetJoin::Bevel),
            "the standing preview uses it"
        );
        assert_eq!(session.offset_values().join(), OffsetJoin::Round);
        // A cancelled drag keeps it.
        session.pointer_down(corner, false);
        session.pointer_hover(along(corner, 4.0), false, false);
        assert!(session.cancel_offset_drag());
        assert_eq!(session.offset_view().pending_join, Some(OffsetJoin::Bevel));
        // A written drag uses it and consumes it.
        drag(&mut session, 2.0, false, false);
        assert_eq!(session.offset_view().pending_join, None);
        assert_eq!(session.offset_join(), OffsetJoin::Round);
        // A tool switch drops it.
        let corner = bottom_right(&session);
        session.double_click_hint(corner, false, false);
        session.set_tool(Tool::Select);
        session.set_tool(Tool::Offset);
        assert_eq!(session.offset_view().pending_join, None);
    }

    #[test]
    fn ac25_ac26_apply_writes_the_bar_values() {
        let (mut session, square) = offset_session();
        session.set_offset_distance(1.0);
        session.set_offset_direction(OffsetDirection::Both);
        session.set_offset_result(OffsetResultChoice::ChangeOriginal);
        let outcome = session.apply_offset();
        assert!(
            matches!(
                outcome,
                OffsetOutcome::Applied {
                    added: 1,
                    changed: 1,
                    ..
                }
            ),
            "{outcome:?}"
        );
        assert!(session.selection.contains(square));
        assert_eq!(session.selection.ids().len(), 2);
    }

    #[test]
    fn ac27_a_press_on_an_object_selects_it_and_moves_nothing() {
        let (mut session, square) = offset_session();
        let other = rect(&session, 50.0, 10.0);
        session.pointer_down(Point::new(55.0, 0.0), false);
        session.pointer_hover(Point::new(70.0, 20.0), false, false);
        session.pointer_up(Point::new(70.0, 20.0), false, false);
        assert_eq!(session.selection.ids(), &[other]);
        let Some(ObjectSnapshot::Primitive(unmoved)) = session.document.object(other) else {
            panic!("a rectangle");
        };
        let Shape::Rect { bounds, .. } = unmoved.shape else {
            panic!("a rectangle");
        };
        assert_eq!(bounds.origin, Point::new(50.0, 0.0));
        session.pointer_down(Point::new(0.0, 10.0), true);
        session.pointer_up(Point::new(0.0, 10.0), true, false);
        assert!(session.selection.contains(square) && session.selection.contains(other));
    }

    fn center(session: &Session) -> Point {
        session.offset_center().expect("the center handle")
    }

    #[test]
    fn ac53_the_center_handle_claims_the_press_and_a_double_click_opens_the_chip() {
        let (mut session, square) = offset_session();
        let at = center(&session);
        assert!((at.x - 10.0).abs() < 1e-9 && (at.y - 10.0).abs() < 1e-9);
        let near = at.translated(Vec2::new(10.0 / session.view().scale(), 0.0));
        session.pointer_down(near, false);
        session.pointer_hover(near.translated(Vec2::new(30.0, 30.0)), false, false);
        session.pointer_up(near.translated(Vec2::new(30.0, 30.0)), false, false);
        assert_eq!(
            session.selection.ids(),
            &[square],
            "no marquee, no deselect"
        );
        assert!(!session.offset_chip_open(), "a single click does nothing");
        assert_eq!(
            session.double_click_hint(near, false, false),
            super::super::DoubleClickHint::OffsetChip
        );
        assert!(session.offset_chip_open());
        assert_eq!(session.cursor_hint(), "default");
        session.pointer_hover(at, false, false);
        assert_eq!(session.cursor_hint(), "pointer");
        // A press outside closes it without writing.
        let steps = session.undo_len();
        session.pointer_down(Point::new(400.0, 400.0), false);
        session.pointer_up(Point::new(400.0, 400.0), false, false);
        assert!(!session.offset_chip_open());
        assert_eq!(session.undo_len(), steps);
    }

    #[test]
    fn ac53_the_center_handle_is_hidden_during_a_corner_drag_and_without_operands() {
        let (mut session, _) = offset_session();
        assert_eq!(session.offset_handle_hint(), "");
        session.pointer_hover(center(&session), false, false);
        assert_eq!(session.offset_handle_hint(), "offset-center");
        let from = bottom_right(&session);
        session.pointer_hover(from, false, false);
        assert_eq!(session.offset_handle_hint(), "offset-corner");
        session.pointer_down(from, false);
        assert_eq!(session.offset_center(), None);
        assert!(!session.open_offset_chip(), "not during a drag");
        session.cancel_offset_drag();
        session.selection.clear();
        assert_eq!(session.offset_center(), None);
    }

    #[test]
    fn ac25_o_and_enter_open_the_chip_and_enter_in_it_writes_and_closes() {
        let (mut session, _) = offset_session();
        let press = |session: &mut Session, key: &str| {
            session
                .key_down(crate::KeyInput {
                    key,
                    ..crate::KeyInput::default()
                })
                .code()
        };
        assert_eq!(press(&mut session, "o"), "offset-chip");
        assert!(session.offset_chip_open());
        assert_eq!(press(&mut session, "o"), "ignored", "gated while open");
        session.close_offset_chip();
        assert_eq!(press(&mut session, "Enter"), "offset-chip");
        let steps = session.undo_len();
        assert!(matches!(
            session.apply_offset(),
            OffsetOutcome::Applied { added: 1, .. }
        ));
        assert_eq!(session.undo_len(), steps + 1);
        assert!(!session.offset_chip_open(), "Enter closes it");
        let (anchor, min, max) = session.offset_chip_anchor().expect("an anchor");
        assert!(min.x <= anchor.x && anchor.x <= max.x);
        assert!(
            max.x - min.x >= 24.0 - 1e-9,
            "the box around the standing preview"
        );
    }

    #[test]
    fn ac29_a_refused_enter_keeps_the_chip_open_and_a_pending_style_moves_into_it() {
        let (mut session, _) = offset_session();
        let corner = bottom_right(&session);
        session.double_click_hint(corner, false, false);
        assert!(session.open_offset_chip());
        assert_eq!(session.offset_values().join(), OffsetJoin::Miter);
        assert_eq!(session.offset_view().pending_join, None);
        session.set_offset_distance(10.0);
        session.set_offset_direction(OffsetDirection::In);
        assert!(matches!(
            session.apply_offset(),
            OffsetOutcome::Refused { .. }
        ));
        assert!(session.offset_chip_open(), "the refusal shows in the chip");
        session.set_tool(Tool::Select);
        assert!(!session.offset_chip_open());
    }

    #[test]
    fn ac54_escape_with_the_chip_open_only_closes_the_chip() {
        let (mut session, square) = offset_session();
        assert!(session.open_offset_chip());
        let outcome = session.key_down(crate::KeyInput {
            key: "Escape",
            ..crate::KeyInput::default()
        });
        assert_eq!(outcome.code(), "escape-entry");
        assert!(!session.offset_chip_open());
        assert_eq!(session.selection.ids(), &[square], "the selection stays");
        assert_eq!(session.tool(), Tool::Offset);
    }
}
