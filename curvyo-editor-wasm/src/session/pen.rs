//! `Session`'s Pen-tool glue (`specs/0002-path-node-editing`): finishing
//! the in-progress path, the pen's close-target cue and the pen's drag
//! threshold. Split out of `session/mod.rs` (`docs/technical-debt.md`,
//! "`Session` is one module past the size limit").

use std::cell::RefCell;

use curvyo_document_core::{DocumentVersion, Length, NodeId, Point};
use curvyo_render_core::{ClosingCue, PenCue};
use curvyo_ui_core::{
    EndNodeIndex, JoinType, PenPointerUpOutcome as PointerUpOutcome, PenTarget, PressAction,
    constrain_segment, continuation_of, format_degrees, format_status_length, pen_target,
};

use super::shapes::LiveReadout;
use super::{Session, Tool};

/// What the end-node index was built for: the document read and the entered group.
type EndIndexKey = (DocumentVersion, Option<NodeId>);

/// What the Pen caches and remembers between events (`specs/0034-pen-path-extension`): the index
/// of the end nodes of open paths, rebuilt only when the document changes (criterion 24), and the
/// target the latest press was decided on, which the cue keeps until the release (a drag from a
/// target is ignored and the cursor stays what it was).
#[derive(Debug, Default)]
pub(super) struct PenCueState {
    ends: RefCell<Option<(EndIndexKey, EndNodeIndex)>>,
    press: Option<PenTarget>,
    /// Where a node-placing press put the new node N: the handle snap turns about it
    /// (`specs/0072-pen-angle-snap-and-typed-values/` criterion 3).
    node: Option<Point>,
    join_notice: Option<PenJoinNotice>,
    snap_press_offset: Option<curvyo_document_core::Vec2>,
}

/// The notice after a Pen join that moved outlines of the removed object into the survivor
/// (`specs/0055-compound-path-node-editing` criterion 9): how many, and a sequence number that
/// changes with every such join, so the host shows it once per join, at the release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PenJoinNotice {
    /// How many outlines moved besides the joined one.
    pub moved_outlines: usize,
    /// Changes with every join that moved outlines; never 0.
    pub seq: u32,
}

impl PenCueState {
    /// Forgets the target of a press that ended.
    pub(super) fn release(&mut self) {
        self.press = None;
        self.node = None;
    }
}

impl Session {
    pub(super) fn drag_threshold(&self) -> Length {
        Length::from_mm(self.transform_handle_tolerances().drag_threshold_mm)
    }

    /// Acceptance criterion 3 / the dedicated "finish path" action
    /// (Enter, or a double-click the host has already recognized).
    pub fn finish_pen(&mut self) {
        self.end_snapping_drag();
        // A finish closes an open entry chip without adding (`0072` criterion 22).
        self.cancel_pen_entry();
        self.recording(|this| {
            this.pen.finish(&this.document);
        });
    }

    /// A Pen release: commits what the press decided, remembers the notice of a join that moved
    /// outlines (`0055` criterion 9), and forgets the press.
    pub(super) fn pen_pointer_up(&mut self, point: Point) {
        let threshold = self.drag_threshold();
        let outcome = self
            .pen
            .pointer_up(&mut self.minter, &self.document, point, threshold);
        if let PointerUpOutcome::Joined { moved_outlines, .. } = outcome
            && moved_outlines > 0
        {
            let seq = self
                .pen_cue
                .join_notice
                .map_or(1, |notice| notice.seq.wrapping_add(1).max(1));
            self.pen_cue.join_notice = Some(PenJoinNotice {
                moved_outlines,
                seq,
            });
        }
        self.pen_cue.release();
        self.pen_cue.snap_press_offset = None;
        self.end_snapping_drag();
    }

    /// The notice of the latest Pen join that moved outlines into the survivor, if any.
    #[must_use]
    pub fn pen_join_notice(&self) -> Option<PenJoinNotice> {
        self.pen_cue.join_notice
    }

    /// The in-progress pen path's placed nodes, for the host's
    /// rubber-band/live-curve preview — `None` when idle or the node
    /// tool is active.
    #[must_use]
    pub fn pen_in_progress(&self) -> Option<&[curvyo_document_core::NewAnchor]> {
        if self.tool == Tool::Pen {
            self.pen.in_progress_nodes()
        } else {
            None
        }
    }

    /// Runs `f` on the index of the end nodes of open ordinary paths, rebuilding it first if the
    /// document changed since it was built.
    pub(super) fn with_end_index<R>(&self, f: impl FnOnce(&EndNodeIndex) -> R) -> R {
        // Only the open paths of the entered group are targets (`0023` criterion 30).
        let key = (self.document.version(), self.context_group());
        let mut cache = self.pen_cue.ends.borrow_mut();
        if cache.as_ref().is_none_or(|(held, _)| *held != key) {
            // Only open paths that are direct children of the context are targets, so a path
            // inside a group is none at the root (criterion 30).
            let units = self.units();
            let paths: Vec<_> = self
                .paths()
                .into_iter()
                // A hidden or locked path is no target (`0050` criterion 19).
                .filter(|path| units.group_of(path.id).is_none() && !units.is_inert(path.id))
                .collect();
            *cache = Some((key, EndNodeIndex::from_paths(&paths)));
        }
        // invariant: the cache was filled just above.
        #[allow(clippy::unwrap_used)]
        f(&cache.as_ref().unwrap().1)
    }

    /// What a Pen press at the pointer would do now: while a press is down, what it was decided
    /// to do; otherwise the live target. `None` outside the Pen tool, off the canvas and while the
    /// eyedropper is picking (the pointer is picking a colour, not aiming a press).
    #[must_use]
    pub fn pen_target(&self) -> Option<PenTarget> {
        if self.tool != Tool::Pen || self.colour_pick_target().is_some() {
            return None;
        }
        if self.button_down {
            return self.pen_cue.press.clone();
        }
        let point = self.pointer_position?;
        let radius = self.point_tolerance_as_length();
        Some(
            self.with_end_index(|index| {
                pen_target(&self.pen, index, point, radius, self.held.shift)
            }),
        )
    }

    /// A Pen press at `point`: decide the action (`pen_target`), remember it for the cue, and begin
    /// the gesture.
    pub(super) fn pen_pointer_down(&mut self, point: Point, shift: bool) {
        let radius = self.point_tolerance_as_length();
        let target =
            self.with_end_index(|index| pen_target(&self.pen, index, point, radius, shift));
        let action = match &target {
            PenTarget::Continue(end) => self
                .paths()
                .iter()
                .find(|path| path.id == end.path)
                .map_or(PressAction::Place, |path| {
                    PressAction::Continue(continuation_of(end, path))
                }),
            PenTarget::Join(end) => PressAction::Join(end.clone()),
            PenTarget::Close { join, .. } => PressAction::Close(*join),
            PenTarget::Place | PenTarget::NewPathAt | PenTarget::PlaceOverEnd => PressAction::Place,
        };
        // The target test ran on the raw pointer; only a placed node is snapped (`0072`
        // criteria 2, 5), with Ctrl as the press event reported it.
        let point = if action == PressAction::Place {
            self.begin_snapping_drag_for_shape();
            let raw = self.pen_constrained(point, self.held.ctrl && !self.held.alt);
            let direction = (self.held.ctrl && !self.held.alt)
                .then(|| self.pen_last_node().map(|last| last.vector_to(raw)))
                .flatten();
            let result =
                self.resolve_snapping_direction(&curvyo_ui_core::SnapProbe::point(raw), direction);
            let node = raw.translated(result.offset);
            self.pen_cue.snap_press_offset = Some(point.vector_to(node));
            self.pen_cue.node = Some(node);
            node
        } else {
            point
        };
        self.pen_cue.press = Some(target);
        self.pen.pointer_down_action(point, action);
    }

    /// The last node L of the path in progress, `None` before the first node is placed.
    pub(super) fn pen_last_node(&self) -> Option<Point> {
        self.pen.in_progress_nodes()?.last().map(|node| node.point)
    }

    /// `point` with the Ctrl rule from the last node when `ctrl` is held and there is one
    /// (`0072` criteria 1, 7); otherwise `point`.
    fn pen_constrained(&self, point: Point, ctrl: bool) -> Point {
        match self.pen_last_node() {
            Some(last) if ctrl => constrain_segment(last, point),
            _ => point,
        }
    }

    pub(super) fn pen_snap_hover(&mut self, point: Point) {
        if self.button_down || self.pen_last_node().is_none() {
            return;
        }
        if !self.snapping.drag_in_flight() {
            self.begin_snapping_drag_for_shape();
        }
        let raw = self.pen_constrained(point, self.held.ctrl && !self.held.alt);
        let direction = (self.held.ctrl && !self.held.alt)
            .then(|| self.pen_last_node().map(|last| last.vector_to(raw)))
            .flatten();
        self.resolve_snapping_direction(&curvyo_ui_core::SnapProbe::point(raw), direction);
    }

    /// The pointer of a node drag as the handle end: snapped about the new node N while Ctrl is
    /// held, read live (`0072` criterion 3). The node itself never moves.
    pub(super) fn pen_handle_point(&self, pointer: Point, ctrl: bool) -> Point {
        let pointer = pointer.translated(
            self.pen_cue
                .snap_press_offset
                .unwrap_or(curvyo_document_core::Vec2::ZERO),
        );
        match self.pen_cue.node {
            Some(node) if ctrl => constrain_segment(node, pointer),
            _ => pointer,
        }
    }

    /// The readout of a node drag: the handle's length and angle, "30.1 mm, 0°" (`0072`
    /// criterion 8). `None` with the button up or before the drag passes the threshold. The host
    /// shows it in the create-drag readout's place; [`Session::live_readout`] stays the
    /// marquee's and the creation tools'.
    #[must_use]
    pub fn pen_drag_readout(&self) -> Option<LiveReadout> {
        if !self.button_down {
            return None;
        }
        let pointer = self.pointer_position?;
        let handle = self.pen_handle_point(pointer, self.held.ctrl);
        let anchor = self
            .pen
            .pending_anchor(self.minter.peek(), handle, self.drag_threshold())?;
        let vector = anchor.handle_out;
        if vector.length() <= self.drag_threshold().as_mm() {
            return None;
        }
        let unit = self.display_unit();
        let text = format!(
            "{} {}, {}",
            format_status_length(curvyo_document_core::Length::from_mm(vector.length()), unit),
            unit.symbol(),
            format_degrees(vector.y.atan2(vector.x).to_degrees())
        );
        Some(LiveReadout {
            text,
            anchor: pointer,
        })
    }

    /// A secondary press with the Pen (`0072` criterion 19a, customer 2026-10-19): with a path in
    /// progress it finishes the path as Enter does and places nothing. Returns whether it did;
    /// with no path the press goes on as before.
    pub fn pen_secondary_press(&mut self) -> bool {
        if self.tool != Tool::Pen || !self.pen_path_open() {
            return false;
        }
        self.finish_pen();
        true
    }

    /// The Pen's cue as render data: the end node a press would continue or join, and the closing
    /// segment of a close target (`0034` criteria 1, 7, 17).
    pub(super) fn pen_cue_data(&self) -> PenCue {
        let mut cue = PenCue::default();
        match self.pen_target() {
            Some(PenTarget::Continue(end) | PenTarget::Join(end)) => {
                cue.target = Some((end.anchor.point, end.anchor.kind));
            }
            Some(PenTarget::Close { join, .. }) => {
                cue.closing = self.pen.closing_preview(join).map(|preview| ClosingCue {
                    from: preview.from,
                    to: preview.to,
                    node: preview.closing_node,
                });
            }
            _ => {}
        }
        cue
    }

    /// The direction, as a unit vector in document space (y down, as on the screen), from the
    /// path in progress toward the side away from it at the target of a close or a join: where a
    /// hint chip can sit without covering the closing segment or the handles of the nodes it is
    /// about. `(0, 0)` when there is nothing to be away from.
    #[must_use]
    pub fn pen_cue_away(&self) -> (f64, f64) {
        let target = match self.pen_target() {
            Some(PenTarget::Join(end)) => end.anchor.point,
            Some(PenTarget::Close { .. }) => match self.pen_in_progress().and_then(<[_]>::first) {
                Some(first) => first.point,
                None => return (0.0, 0.0),
            },
            _ => return (0.0, 0.0),
        };
        let Some(nodes) = self.pen_in_progress() else {
            return (0.0, 0.0);
        };
        let others: Vec<_> = nodes.iter().filter(|node| node.point != target).collect();
        if others.is_empty() {
            return (0.0, 0.0);
        }
        #[allow(clippy::cast_precision_loss)]
        let count = others.len() as f64;
        let (sum_x, sum_y) = others.iter().fold((0.0, 0.0), |(x, y), node| {
            (x + node.point.x, y + node.point.y)
        });
        let (dx, dy) = (target.x - sum_x / count, target.y - sum_y / count);
        let length = dx.hypot(dy);
        if length < 1e-9 {
            (0.0, 0.0)
        } else {
            (dx / length, dy / length)
        }
    }

    /// The point the rubber band runs to: the target's node centre over a join target, nothing
    /// over a close target (the closing segment replaces it), else the pointer.
    pub(super) fn pen_rubber_band_end(&self) -> Option<Point> {
        // An open entry chip draws the typed point (`0072` criterion 17).
        if self.pen_entry().is_some() {
            return self.pen_entry_preview_end();
        }
        self.pen_segment_end()
    }

    /// The end of the live segment: the target's node over a join target, nothing over a close
    /// target, else the pointer, Ctrl-constrained with the button up (`0072` criteria 1, 5).
    pub(super) fn pen_segment_end(&self) -> Option<Point> {
        match self.pen_target() {
            Some(PenTarget::Join(end)) => Some(end.anchor.point),
            Some(PenTarget::Close { .. }) => None,
            _ => self.pointer_position.map(|pointer| {
                self.pen_constrained(
                    pointer,
                    self.held.ctrl && !self.held.alt && !self.button_down,
                )
                .translated(
                    self.snap_result()
                        .map_or(curvyo_document_core::Vec2::ZERO, |r| r.offset),
                )
            }),
        }
    }

    /// Whether the continued path and the path of a join target differ in style (`0034` criterion
    /// 7's third chip line). `false` unless the Pen continues a path and its press target is a join.
    #[must_use]
    pub fn pen_join_style_differs(&self) -> bool {
        let Some(PenTarget::Join(end)) = self.pen_target() else {
            return false;
        };
        let Some(continuation) = self.pen.continuation() else {
            return false;
        };
        let (Some(own), Some(other)) = (
            self.document.path(continuation.path),
            self.document.path(end.path),
        ) else {
            return false;
        };
        own.style != other.style
    }

    /// The join a close target applies and the one "as drawn", for the chip.
    #[must_use]
    pub fn pen_close_joins(&self) -> Option<(JoinType, JoinType, bool)> {
        match self.pen_target()? {
            PenTarget::Close {
                join,
                as_drawn,
                shift,
            } => Some((join, as_drawn, shift)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::Point;

    use super::super::{Session, Tool};

    const L: Point = Point::new(100.0, 50.0);

    fn ctrl_click(session: &mut Session, at: Point, ctrl: bool) {
        session.modifiers_changed(false, ctrl, false);
        session.pointer_down(at, false);
        session.pointer_up(at, false, ctrl);
    }

    fn pen_at_l() -> Session {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        ctrl_click(&mut session, L, true);
        session
    }

    fn near(a: Point, x: f64, y: f64) -> bool {
        (a.x - x).abs() < 0.005 && (a.y - y).abs() < 0.005
    }

    fn nodes(session: &Session) -> Vec<Point> {
        session
            .pen_in_progress()
            .unwrap_or_default()
            .iter()
            .map(|node| node.point)
            .collect()
    }

    /// Criteria 1, 7: the first node ignores Ctrl; the rubber band follows Ctrl live.
    #[test]
    fn ctrl_constrains_the_rubber_band_from_the_last_node() {
        let mut session = pen_at_l();
        assert_eq!(nodes(&session), [L]);
        session.pointer_hover(Point::new(130.0, 52.0), false, true);
        let end = session.pen_rubber_band_end().expect("a rubber band");
        assert!(near(end, 130.067, 50.0), "{end:?}");
        session.modifiers_changed(false, false, false);
        assert_eq!(session.pen_rubber_band_end(), Some(Point::new(130.0, 52.0)));
    }

    /// Criterion 2: three Ctrl-clicks off by up to 3 degrees give exact horizontals and verticals.
    #[test]
    fn ctrl_clicks_place_nodes_at_the_constrained_point() {
        let mut session = pen_at_l();
        for at in [
            Point::new(130.0, 52.0),
            Point::new(131.0, 80.0),
            Point::new(101.0, 81.0),
        ] {
            ctrl_click(&mut session, at, true);
        }
        let placed = nodes(&session);
        assert!(near(placed[1], 130.067, 50.0));
        assert!((placed[2].x - placed[1].x).abs() < 1e-9);
        assert!((placed[3].y - placed[2].y).abs() < 1e-9);
    }

    /// Criteria 3, 8: the node stays at the press, Ctrl snaps the handle direction live, the
    /// readout shows the handle.
    #[test]
    fn ctrl_snaps_the_handle_direction_and_never_moves_the_node() {
        let mut session = pen_at_l();
        let n = Point::new(130.0, 50.0);
        session.modifiers_changed(false, false, false);
        session.pointer_down(n, false);
        session.pointer_hover(Point::new(150.0, 52.0), false, true);
        let readout = session.pen_drag_readout().expect("a readout").text;
        assert_eq!(readout, "20.1 mm, 0°");
        session.pointer_up(Point::new(150.0, 52.0), false, true);
        let node = session.pen_in_progress().unwrap()[1];
        assert_eq!(node.point, n);
        assert!(near(n.translated(node.handle_out), 150.1, 50.0));
        assert!(near(n.translated(node.handle_in), 109.9, 50.0));
        assert!(session.pen_drag_readout().is_none());
    }

    /// Criterion 5: the target test runs on the raw pointer, so a Ctrl-click on the first node
    /// closes the path.
    #[test]
    fn a_ctrl_click_on_the_first_node_still_closes() {
        let mut session = pen_at_l();
        ctrl_click(&mut session, Point::new(140.0, 50.0), false);
        ctrl_click(&mut session, Point::new(140.0, 90.0), false);
        ctrl_click(&mut session, Point::new(100.5, 50.5), true);
        assert!(session.pen_in_progress().is_none());
        let path = session.document.object_ids()[0];
        assert!(session.document.path(path).unwrap().closed);
    }
}
