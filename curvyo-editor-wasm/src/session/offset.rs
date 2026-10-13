//! The Offset tool in the session (`specs/0038-path-offset` criteria 2, 5 to 27): the bar's
//! values, the four handles, the drag with its modifiers and the inset stop, the preview, the
//! write on release or Apply, and selecting with the tool.
//!
//! The preview is computed once per frame from the current values and cached by document
//! version, selection and request, so what a release writes is what the last frame showed
//! (criterion 18).

use std::cell::RefCell;

use curvyo_document_core::{Angle, DocumentVersion, NodeId, ObjectSnapshot, Point};
use curvyo_ui_core::{
    CORNER_SIGNS, HANDLE_HIT_RADIUS_PX, KindNote, Modifiers, ObjectSelection, OffsetBar, OffsetCap,
    OffsetDirection, OffsetJoin, OffsetPlan, OffsetRefusal, OffsetRequest, OffsetResultChoice,
    OffsetSide, OffsetSides, OrientedBox, PressTarget, classify_press, drag_distance,
    handle_positions, hit_test_objects, offset_box, offset_operands, plan_offset, snap_distance,
};

use super::refusal::RefusalMarks;
use super::{Session, Tool};

/// The label of the commit an offset makes (criterion 16).
pub const OFFSET_COMMIT_LABEL: &str = "offset_path";

/// Above this many operand nodes the preview is the grown box (criterion 18).
const PREVIEW_NODE_LIMIT: usize = 2000;

/// The nodes a primitive counts for the preview limit.
const PRIMITIVE_NODES: usize = 8;

/// The Offset tool's state: the bar's values (session memory, criterion 7), the drag in flight,
/// the handle under the pointer and the cached preview.
#[derive(Debug, Default)]
pub(super) struct OffsetTool {
    bar: OffsetBar,
    drag: Option<OffsetDrag>,
    hovered: Option<usize>,
    bar_focus: bool,
    outcome: Option<OffsetOutcome>,
    preview: RefCell<Option<Cached>>,
}

/// A handle drag in flight.
#[derive(Debug, Clone, Copy)]
struct OffsetDrag {
    corner: usize,
    start: Point,
    angle: Angle,
    /// The signed, snapped, stopped distance: positive outward.
    d: f64,
    /// The last distance at which something was left.
    last_good: f64,
    stop: Option<DragStop>,
}

/// Why the handle stopped (criteria 12, 30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragStop {
    /// Further in, the whole selection vanishes.
    NothingLeft,
    /// Only open paths are selected: they have no inside.
    InsetNeedsClosed,
}

/// One computed plan and what it was computed from.
#[derive(Debug, Clone)]
struct Cached {
    version: DocumentVersion,
    selection: Vec<NodeId>,
    request: OffsetRequest,
    result: Result<OffsetPlan, OffsetRefusal>,
}

/// What a release or Apply did (criterion 24).
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
        /// Open paths left out of an inset.
        open_left_out: usize,
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

/// What the bar, the readout and the canvas show (criteria 4, 5, 12, 15, 18).
// Flags the host reads one by one, not a state machine.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetView {
    /// Operands in the selection (groups not counted).
    pub operands: usize,
    /// An open path is selected: Ends applies.
    pub has_open: bool,
    /// The selection holds a group.
    pub has_group: bool,
    /// The handles are drawn.
    pub handles: bool,
    /// A drag is in flight.
    pub dragging: bool,
    /// The signed distance shown (the drag's, or the bar's with its Direction).
    pub distance_mm: f64,
    /// Both sides (Shift during a drag, or Direction Both).
    pub both: bool,
    /// The release or Apply changes the original.
    pub change: bool,
    /// Where the handle stopped, if it did.
    pub stop: Option<DragStop>,
    /// Operands whose inset vanishes in the preview.
    pub vanished: usize,
    /// How many operands the preview covers.
    pub of: usize,
    /// The preview is replaced by the grown box.
    pub large: bool,
    /// Stays a primitive or becomes a path.
    pub kind_note: Option<KindNote>,
    /// Why the preview shows nothing, if it is refused.
    pub refusal: Option<OffsetRefusal>,
}

impl Session {
    /// The bar's values.
    #[must_use]
    pub const fn offset_bar(&self) -> OffsetBar {
        self.offset.bar
    }

    /// Sets the Distance (criterion 6); returns whether it was within 0.01..1000 mm.
    pub fn set_offset_distance(&mut self, distance_mm: f64) -> bool {
        self.offset.bar.set_distance_mm(distance_mm)
    }

    /// Sets the Direction.
    pub fn set_offset_direction(&mut self, direction: OffsetDirection) {
        self.offset.bar.set_direction(direction);
    }

    /// Sets the Corners.
    pub fn set_offset_join(&mut self, join: OffsetJoin) {
        self.offset.bar.set_join(join);
    }

    /// Sets the Ends.
    pub fn set_offset_cap(&mut self, cap: OffsetCap) {
        self.offset.bar.set_cap(cap);
    }

    /// Sets the Result.
    pub fn set_offset_result(&mut self, result: OffsetResultChoice) {
        self.offset.bar.set_result(result);
    }

    /// Focus entered or left the bar: while it is there the preview of the bar's values shows
    /// (criterion 25).
    pub fn set_offset_bar_focus(&mut self, focused: bool) {
        self.offset.bar_focus = focused;
    }

    /// What the last release did, once (the host shows the notice).
    pub fn take_offset_outcome(&mut self) -> Option<OffsetOutcome> {
        self.offset.outcome.take()
    }

    /// The selection's operands, the leaves of selected groups left out.
    fn offset_selection(&self) -> ObjectSelection {
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

    /// The box the handles sit on, over the whole selection.
    fn offset_box_now(&self) -> Option<OrientedBox> {
        let objects = self.objects();
        let selected: Vec<&ObjectSnapshot> = objects
            .iter()
            .filter(|object| self.selection.contains(object.id()))
            .collect();
        offset_box(&selected)
    }

    /// Whether the handles are drawn: the Offset tool with an operand selected (a group alone
    /// draws none, criterion 5).
    fn offset_handles_shown(&self) -> bool {
        self.tool == Tool::Offset && !self.offset_selection().is_empty()
    }

    /// The handle at `point`, if the handles are drawn.
    fn offset_handle_at(&self, point: Point) -> Option<usize> {
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

    /// Whether `point` is on a handle: a double-click there focuses the Distance field
    /// (criterion 13).
    pub(super) fn offset_handle_hit(&self, point: Point) -> bool {
        self.offset_handle_at(point).is_some()
    }

    /// The request of a drag at the signed `d` with `held` keys, or of the bar.
    fn offset_request(&self, signed: Option<(f64, Modifiers)>) -> OffsetRequest {
        let bar = &self.offset.bar;
        let (distance_mm, sides) = match signed {
            Some((d, held)) => {
                let dragged = if d < 0.0 {
                    OffsetSide::In
                } else {
                    OffsetSide::Out
                };
                (
                    d.abs(),
                    OffsetSides::of(dragged, held.shift, bar.changes_original(held.ctrl)),
                )
            }
            None => (bar.distance_mm(), bar.apply_sides()),
        };
        OffsetRequest {
            distance_mm,
            sides,
            join: bar.join(),
            cap: bar.cap(),
        }
    }

    /// The plan for `request`, from the cache when nothing changed.
    fn offset_plan_for(&self, request: OffsetRequest) -> Result<OffsetPlan, OffsetRefusal> {
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

    /// Whether the operands have more nodes than the preview draws (criterion 18).
    fn offset_selection_is_large(&self) -> bool {
        let objects = self.objects();
        let selection = self.offset_selection();
        let nodes: usize = offset_operands(&objects, &selection)
            .iter()
            .map(|object| match object {
                ObjectSnapshot::Path(path) => path.all_anchors().count(),
                ObjectSnapshot::Primitive(_) => PRIMITIVE_NODES,
            })
            .sum();
        nodes > PREVIEW_NODE_LIMIT
    }

    /// The request the canvas previews now: the drag's, or the bar's while focus is in the bar.
    fn offset_preview_request(&self) -> Option<OffsetRequest> {
        if self.tool != Tool::Offset {
            return None;
        }
        if let Some(drag) = self.offset.drag {
            return (drag.d != 0.0).then(|| self.offset_request(Some((drag.d, self.held))));
        }
        self.offset.bar_focus.then(|| self.offset_request(None))
    }

    /// What the bar, readout and canvas show.
    #[must_use]
    pub fn offset_view(&self) -> OffsetView {
        let objects = self.objects();
        let selection = self.offset_selection();
        let operands = offset_operands(&objects, &selection);
        let has_open = operands.iter().any(|object| match object {
            ObjectSnapshot::Path(path) => path.subpaths().any(|subpath| !subpath.closed),
            ObjectSnapshot::Primitive(_) => false,
        });
        let drag = self.offset.drag;
        let bar = self.offset.bar;
        let (distance_mm, both, change) = match drag {
            Some(drag) => (
                drag.d,
                self.held.shift,
                bar.changes_original(self.held.ctrl),
            ),
            None => (
                if bar.direction() == OffsetDirection::In {
                    -bar.distance_mm()
                } else {
                    bar.distance_mm()
                },
                bar.direction() == OffsetDirection::Both,
                bar.result() == OffsetResultChoice::ChangeOriginal,
            ),
        };
        let large = self.offset_selection_is_large();
        let mut view = OffsetView {
            operands: operands.len(),
            has_open,
            has_group: !self.selected_group_leaves().is_empty(),
            handles: self.offset_handles_shown(),
            dragging: drag.is_some(),
            distance_mm,
            both,
            change,
            stop: drag.and_then(|drag| drag.stop),
            vanished: 0,
            of: operands.len(),
            large,
            kind_note: None,
            refusal: None,
        };
        if large {
            return view;
        }
        if let Some(request) = self.offset_preview_request() {
            match self.offset_plan_for(request) {
                Ok(plan) => {
                    view.vanished = plan.vanished;
                    view.kind_note = plan.kind_note;
                }
                Err(refusal) => view.refusal = Some(refusal),
            }
        }
        view
    }

    /// The Offset tool's canvas layer: the preview outlines (or the grown box), the selection box
    /// and the handles.
    pub(super) fn offset_draw_list(
        &self,
        view: curvyo_document_core::ViewTransform,
    ) -> curvyo_render_core::DrawList {
        let mut list = curvyo_render_core::DrawList::default();
        if self.tool != Tool::Offset {
            return list;
        }
        let Some(bounds) = self.offset_box_now() else {
            return list;
        };
        if let Some(request) = self.offset_preview_request() {
            if self.offset_selection_is_large() {
                let signed = if request.sides.out.is_some() {
                    request.distance_mm
                } else {
                    -request.distance_mm
                };
                list.extend(curvyo_render_core::build_offset_grown_box(
                    grown_box(&bounds, signed).document_corners(),
                    view,
                ));
            } else if let Ok(plan) = self.offset_plan_for(request) {
                list.extend(curvyo_render_core::build_offset_preview(
                    &plan.preview_outlines(),
                    view,
                ));
            }
        }
        if self.offset_handles_shown() {
            let placed = self
                .offset
                .drag
                .map_or(bounds, |drag| grown_box(&bounds, drag.d));
            list.extend(curvyo_render_core::build_offset_handles(
                bounds.document_corners(),
                handle_positions(&placed, view.scale()),
                curvyo_render_core::OffsetHandleState {
                    hovered: self.offset.hovered,
                    dragged: self.offset.drag.map(|drag| drag.corner),
                },
                view,
            ));
        }
        list
    }

    /// A press in the Offset tool: on a handle a drag begins; on an object it is selected
    /// (Shift adds or removes) and nothing moves; on empty canvas a marquee begins (criterion 27).
    pub(super) fn offset_pointer_down(&mut self, point: Point, shift: bool) {
        self.command_refusal = None;
        if let Some(corner) = self.offset_handle_at(point)
            && let Some(bounds) = self.offset_box_now()
        {
            self.offset.drag = Some(OffsetDrag {
                corner,
                start: point,
                angle: bounds.angle,
                d: 0.0,
                last_good: 0.0,
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
            return;
        };
        let scale = self.view().scale();
        let raw = drag_distance(
            drag.angle,
            CORNER_SIGNS[drag.corner],
            drag.start.vector_to(point),
        );
        let mut d = snap_distance(raw, scale, self.offset.bar.distance_mm());
        drag.stop = None;
        if d < 0.0 && !self.offset_selection_is_large() {
            match self.offset_plan_for(self.offset_request(Some((d, self.held)))) {
                Err(OffsetRefusal::Empty) => {
                    d = drag.last_good;
                    drag.stop = Some(DragStop::NothingLeft);
                }
                Err(OffsetRefusal::InsetOfOpenPaths { .. }) => {
                    d = 0.0;
                    drag.stop = Some(DragStop::InsetNeedsClosed);
                }
                _ => drag.last_good = d,
            }
        }
        drag.d = d;
        self.offset.drag = Some(drag);
        // The bar follows the drag (criteria 6, 10).
        if d != 0.0 {
            self.offset.bar.set_distance_mm(d.abs());
            self.offset.bar.set_direction(if self.held.shift {
                OffsetDirection::Both
            } else if d < 0.0 {
                OffsetDirection::In
            } else {
                OffsetDirection::Out
            });
        }
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
        if drag.d == 0.0 {
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
        self.offset.outcome = Some(outcome);
    }

    /// Escape or the other button during a drag: nothing is written (criterion 17). Returns
    /// whether a drag was cancelled.
    pub fn cancel_offset_drag(&mut self) -> bool {
        self.offset.drag.take().is_some()
    }

    /// Whether a drag past 0 is in flight that makes a new object (Ctrl does not change the
    /// original): the move's plus badge shows (criterion 15).
    pub(super) fn offset_drag_makes_copy(&self) -> bool {
        self.offset
            .drag
            .is_some_and(|drag| drag.d != 0.0 && !self.offset.bar.changes_original(self.held.ctrl))
    }

    /// Whether an Offset handle drag is in flight.
    pub(super) const fn offset_drag_in_flight(&self) -> bool {
        self.offset.drag.is_some()
    }

    /// Apply: writes the bar's values for the selection in one commit (criteria 25, 26).
    pub fn apply_offset(&mut self) -> OffsetOutcome {
        self.recording(|this| {
            if this.tool != Tool::Offset || this.offset.drag.is_some() {
                return OffsetOutcome::Ignored;
            }
            let request = this.offset_request(None);
            let direction = this.offset.bar.direction();
            this.write_offset_plan(request, direction)
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
                    OffsetRefusal::InsetOfOpenPaths { offenders, .. }
                    | OffsetRefusal::OutOfRange { offenders, .. }
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
        let (vanished, of, open_left_out) = (plan.vanished, plan.of, plan.open_left_out.len());
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
            open_left_out,
        }
    }
}

/// `bounds` grown by `d` on every side (shrunk at most to its middle), for the handles during a
/// drag and the grown box.
fn grown_box(bounds: &OrientedBox, d: f64) -> OrientedBox {
    let grow = d.max(-bounds.width().min(bounds.height()) / 2.0);
    OrientedBox {
        min: Point::new(bounds.min.x - grow, bounds.min.y - grow),
        max: Point::new(bounds.max.x + grow, bounds.max.y + grow),
        angle: bounds.angle,
        pivot: bounds.pivot,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{AnchorId, Length, NewAnchor, RectBounds, Shape, Vec2};

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
            .map(|((cx, cy), n)| NewAnchor::corner(AnchorId::new(1, n), Point::new(cx, cy)))
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
    /// 1 px per mm, where the snap to the bar's value (6 px) catches every small distance, so the
    /// bar holds the dragged value first.
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
            (session.offset_bar().distance_mm() - 2.0).abs() < 1e-9,
            "the bar follows"
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
    fn ac12_an_inset_stops_where_something_is_left() {
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
        assert!(session.cancel_offset_drag(), "Escape or the other button");
        assert_eq!(session.document.object_ids(), vec![small]);
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
    fn ac25_ac26_apply_writes_the_bar_values() {
        let (mut session, square) = offset_session();
        session.set_offset_distance(1.0);
        session.set_offset_direction(OffsetDirection::Both);
        session.set_offset_result(OffsetResultChoice::ChangeOriginal);
        session.set_offset_bar_focus(true);
        assert_eq!(
            session.offset_view().kind_note,
            Some(KindNote::Stays(curvyo_ui_core::PrimitiveKind::Rectangle))
        );
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
}
