//! `Session`'s undo and redo (`specs/0020-undo-redo`): recording the steps the maker makes with the
//! selection around them, taking them back and bringing them back, and the words of the notice.

use std::collections::{HashMap, HashSet};

use curvyo_document_core::{
    AnchorId, Direction, FAR_STEP_OPS, LABEL_OBJECT_REDO, NodeId, SMALL_STEP_OPS, StepRef,
    Timestamp,
};
use curvyo_ui_core::{
    FINISH_PEN_FIRST, SelectionSnapshot, UndoStack, nothing_notice, objects_summary,
    operation_name, restore_notice, summary_shapes,
};

use super::{KeyOutcome, Session, Workbench};

/// What an undo or redo press did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryResult {
    /// The one-line notice for the maker.
    pub notice: String,
    /// Whether the document changed.
    pub changed: bool,
}

/// The reading of the wall clock, injected so that tests do not depend on it.
pub type Clock = fn() -> Timestamp;

/// The wall clock of this platform in whole seconds.
#[cfg(not(target_arch = "wasm32"))]
pub fn wall_clock() -> Timestamp {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    Timestamp::from_secs(seconds)
}

/// The wall clock of this platform in whole seconds.
#[cfg(target_arch = "wasm32")]
pub fn wall_clock() -> Timestamp {
    // The browser's clock in milliseconds since the epoch; always finite and positive.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let seconds = (web_sys::js_sys::Date::now() / 1000.0).max(0.0) as u64;
    Timestamp::from_secs(seconds)
}

/// The history state of a session: the stacks, how many of the document's steps they have seen,
/// and the clock.
pub(super) struct HistoryState {
    pub(super) stack: UndoStack,
    recorded: usize,
    clock: Clock,
    pub(super) notice: Option<String>,
    /// The folded and unfolded branches of the History list (`history_list.rs`).
    pub(super) list: super::history_list::ListState,
    /// The Mode switch and the version a preview shows (`history_preview.rs`).
    pub(super) preview: super::history_preview::PreviewState,
}

impl HistoryState {
    pub(super) fn new() -> Self {
        Self {
            stack: UndoStack::new(),
            recorded: 0,
            clock: wall_clock,
            notice: None,
            list: super::history_list::ListState::default(),
            preview: super::history_preview::PreviewState::default(),
        }
    }
}

impl Session {
    /// The wall clock steps are stamped with.
    pub(super) fn history_clock(&self) -> Clock {
        self.history.clock
    }

    /// Empties both stacks (a wipe cut the log they point into).
    pub(super) fn reset_history_stacks(&mut self) {
        self.history.stack = UndoStack::new();
        self.history.recorded = 0;
        self.history.notice = None;
    }

    /// Replaces the wall clock steps are stamped with (tests).
    pub fn set_clock(&mut self, clock: Clock) {
        self.history.clock = clock;
    }

    /// The selection as the stacks keep it.
    pub(super) fn selection_snapshot(&self) -> SelectionSnapshot {
        SelectionSnapshot {
            objects: self.selection.ids().to_vec(),
            nodes: self.node.selection().node_pairs().to_vec(),
        }
    }

    /// The guarded write of every edit command (`specs/0106` criterion 9, `adrs.md` decision 4):
    /// with a move-only selection the body does not run, nothing is written and no step is
    /// recorded; otherwise as [`Session::recording_unguarded`]. A command is guarded unless it
    /// opts out, so a new command is an edit by default.
    pub(super) fn recording<R: Default>(&mut self, body: impl FnOnce(&mut Self) -> R) -> R {
        self.sync_context();
        if self.edit_scope() == curvyo_ui_core::Scope::MoveOnly {
            self.edit_context.refused = true;
            return R::default();
        }
        self.recording_unguarded(body)
    }

    /// Runs `body`, which may commit steps, and puts every step it started on the undo stack with
    /// the selection before it ran and the one it left. Every entry point that writes the
    /// document goes through here, so one maker's interaction is one stack entry. Only move
    /// commands, commands that are not about the selection and the input dispatchers call it
    /// directly; every other command calls [`Session::recording`].
    pub(super) fn recording_unguarded<R>(&mut self, body: impl FnOnce(&mut Self) -> R) -> R {
        self.sync_context();
        let move_only = self.edit_scope() == curvyo_ui_core::Scope::MoveOnly;
        // The repair count belongs to this command alone (review S2).
        let _ = self.document.take_clone_repair();
        let _ = self.document.take_kept_carriers();
        let _ = self.document.take_clone_refusal();
        let _ = self.document.take_placement_refusal();
        let before = self.selection_snapshot();
        self.document.set_step_time((self.history.clock)());
        let result = body(self);
        // A placed page kept on its laser page says so (`specs/0100` criterion 18).
        if self.document.take_placement_refusal() {
            self.transform_notice = Some(super::manufacturing::OFF_PAGE_NOTICE);
        }
        // An edit ends an object preview; the object is drawn as it now is (`0042` criterion 18).
        self.end_object_preview();
        // The second net: an edit reached through an unguarded entry with a move-only selection.
        debug_assert!(
            !move_only
                || self
                    .document
                    .own_steps_from(self.history.recorded)
                    .iter()
                    .all(|step| step.reverts.is_some()
                        || curvyo_ui_core::step_scope(&step.label)
                            != curvyo_ui_core::StepScope::Edit),
            "an edit step was written for a move-only selection"
        );
        self.record_new_steps(&before);
        // An entered group the step removed is left now, and a screen reader is told (`0023`
        // criterion 20).
        self.sync_context();
        result
    }

    /// Whether the command running now has written a step that is not on the undo stack
    /// yet: something was committed.
    pub(super) fn has_unrecorded_steps(&self) -> bool {
        !self
            .document
            .own_steps_from(self.history.recorded)
            .is_empty()
    }

    /// Puts the steps the document started since the last look on the undo stack.
    pub(super) fn record_new_steps(&mut self, before: &SelectionSnapshot) {
        let new = self.document.own_steps_from(self.history.recorded);
        if new.is_empty() {
            return;
        }
        let after = self.selection_snapshot();
        for step in &new {
            let name = step.reverts.as_ref().map(|reverted| {
                let verb = if step.label == LABEL_OBJECT_REDO {
                    "Redo"
                } else {
                    "Undo"
                };
                format!("{verb} {}", operation_name(reverted))
            });
            self.history.stack.record_named(
                step.step,
                &step.label,
                name,
                before.clone(),
                after.clone(),
            );
        }
        self.history.recorded += new.len();
    }

    /// Records steps no entry point accounted for, with the selection as it is now. A safety net
    /// that runs before every undo, redo and press.
    pub(super) fn sync_history(&mut self) {
        let now = self.selection_snapshot();
        self.record_new_steps(&now);
    }

    /// The step an undo press would act on, and how many operations it has.
    fn target_step(&self, direction: Direction) -> Option<(StepRef, String)> {
        let target = match direction {
            Direction::Undo => self.history.stack.undo_target(),
            Direction::Redo => self.history.stack.redo_target(),
        }?;
        Some((target.step, target.name))
    }

    /// Whether the press would take long enough to show the long-operation state first
    /// (`specs/0020-undo-redo` criterion 30): the step is large, or the history has not been read
    /// since the file was opened.
    #[must_use]
    pub fn history_needs_busy_state(&mut self, direction: Direction) -> bool {
        self.sync_history();
        if !self.document.history_is_warm() {
            return true;
        }
        self.target_step(direction).is_some_and(|(step, _)| {
            self.document
                .step_op_count(step)
                .is_some_and(|ops| ops > SMALL_STEP_OPS)
                || self
                    .document
                    .step_distance(step)
                    .is_some_and(|ops| ops > FAR_STEP_OPS)
        })
    }

    /// Reads the document's history in the background of the first paint, so that the first undo
    /// of an opened file is as fast as later ones.
    pub fn warm_history(&mut self) {
        self.document.warm_history();
    }

    /// The number of steps on the undo stack.
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.history.stack.undo_len()
    }

    /// The number of steps on the redo stack.
    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.history.stack.redo_len()
    }

    /// The label of the step an undo or redo press would act on.
    #[must_use]
    pub fn history_target_name(&mut self, direction: Direction) -> Option<String> {
        self.sync_history();
        self.target_step(direction).map(|(_, name)| name)
    }

    /// The key press of undo or redo that passed the gate: runs it, or asks the host for the
    /// long-operation state first when it would take long.
    pub(super) fn history_key(&mut self, direction: Direction) -> KeyOutcome {
        let large =
            self.history_needs_busy_state(direction) && self.target_step(direction).is_some();
        match (direction, large) {
            (Direction::Undo, true) => KeyOutcome::UndoBusy,
            (Direction::Redo, true) => KeyOutcome::RedoBusy,
            (Direction::Undo, false) => {
                let result = self.undo();
                self.history.notice = Some(result.notice);
                KeyOutcome::Undo
            }
            (Direction::Redo, false) => {
                let result = self.redo();
                self.history.notice = Some(result.notice);
                KeyOutcome::Redo
            }
        }
    }

    /// The notice the last history key press left, once.
    pub fn take_history_notice(&mut self) -> Option<String> {
        self.history.notice.take()
    }

    /// Takes the maker's latest step back (Ctrl+Z, the Undo button).
    pub fn undo(&mut self) -> HistoryResult {
        self.press_history(Direction::Undo)
    }

    /// Applies the maker's latest undone step again (Ctrl+Shift+Z, the Redo button).
    pub fn redo(&mut self) -> HistoryResult {
        self.press_history(Direction::Redo)
    }

    fn press_history(&mut self, direction: Direction) -> HistoryResult {
        // The gate of the keys (criterion 6) holds for every entry point: the History buttons come
        // here too. A document preview pauses editing (`0042` criterion 19).
        if self.operation_in_flight() || self.select.has_entry() || self.document_preview_active() {
            return HistoryResult {
                notice: String::new(),
                changed: false,
            };
        }
        self.end_snapping_drag();
        // An undo or redo ends an object preview first (`0042` criterion 18).
        self.end_object_preview();
        if self.pen_path_open() {
            return HistoryResult {
                notice: FINISH_PEN_FIRST.to_owned(),
                changed: false,
            };
        }
        // A history jump ends text edit mode first and writes its one step, so it acts on what
        // is on screen (`specs/0074-text-objects` criteria 7 and 9).
        self.finish_text_edit();
        self.flush_select_bar_preview();
        self.flush_style_preview();
        self.sync_history();
        // An undone or redone nudge step is over: its readout and announcement would be stale.
        self.nudge_run = None;
        let target = match direction {
            Direction::Undo => self.history.stack.undo_target(),
            Direction::Redo => self.history.stack.redo_target(),
        };
        let Some(target) = target else {
            return HistoryResult {
                notice: nothing_notice(direction).to_owned(),
                changed: false,
            };
        };
        let name = target.name.clone();
        self.document.set_step_time((self.history.clock)());
        let report = match direction {
            Direction::Undo => self.document.undo_step(target.step),
            Direction::Redo => self.document.redo_step(target.step),
        };
        let stack = &mut self.history.stack;
        let Ok(report) = report else {
            match direction {
                Direction::Undo => stack.consume_undo(),
                Direction::Redo => stack.consume_redo(),
            }
            let verb = if direction == Direction::Undo {
                "undo"
            } else {
                "redo"
            };
            return HistoryResult {
                notice: format!("Could not {verb} {name}."),
                changed: false,
            };
        };
        match (direction, report.wrote) {
            (Direction::Undo, true) => stack.undone(target.step),
            (Direction::Redo, true) => stack.redone(target.step),
            (Direction::Undo, false) => stack.consume_undo(),
            (Direction::Redo, false) => stack.consume_redo(),
        }
        if report.wrote {
            let snapshot = match direction {
                Direction::Undo => &target.before,
                Direction::Redo => &target.after,
            };
            self.restore_selection(snapshot);
            self.command_refusal = None;
            // A page the step removed or brought back hands the tools over (`specs/0098`).
            let workbench = self.sketch.workbench;
            self.sync_workbench();
            if self.sketch.workbench != workbench && self.sketch.workbench == Workbench::Sketch {
                // A sketch page the step brought back opens with nothing selected
                // (`specs/0099` criterion 15).
                self.selection.clear();
            }
        }
        let touched: Vec<_> = report.objects.iter().map(|o| (o.shape, o.role)).collect();
        let shapes = summary_shapes(&target.label, &touched);
        let summary = objects_summary(&shapes);
        let off_screen = report.wrote && self.selection_is_off_screen();
        // In a shared document an own step whose objects a participant deleted is skipped with
        // who did it (`specs/0092-cooperative-editing` criterion 22, at most 70 characters).
        let notice = self
            .deleted_by_peer_notice(&report)
            .unwrap_or_else(|| restore_notice(&name, &report, summary.as_deref(), off_screen));
        HistoryResult {
            notice,
            changed: report.wrote,
        }
    }

    /// Selects what `snapshot` held, leaving out what no longer exists. Reads only the objects the
    /// snapshot names, so the cost does not grow with the size of the drawing.
    pub(super) fn restore_selection(&mut self, snapshot: &SelectionSnapshot) {
        let ids: Vec<NodeId> = snapshot
            .objects
            .iter()
            .copied()
            .filter(|id| self.document.object(*id).is_some())
            .collect();
        self.selection.set(&ids);
        let mut anchors: HashMap<NodeId, HashSet<AnchorId>> = HashMap::new();
        let mut nodes: Vec<(NodeId, AnchorId)> = Vec::new();
        for &(path, anchor) in &snapshot.nodes {
            let known = anchors.entry(path).or_insert_with(|| {
                self.document
                    .path(path)
                    .map(|p| p.all_anchors().map(|a| a.id).collect())
                    .unwrap_or_default()
            });
            if known.contains(&anchor) {
                nodes.push((path, anchor));
            }
        }
        self.node.select_nodes(nodes);
        self.hovered_object = None;
        self.hovered = None;
    }

    /// Whether something is selected and none of it touches the visible canvas (the notice then
    /// says so; pan and zoom are the maker's, `specs/0020-undo-redo` criterion 24).
    fn selection_is_off_screen(&self) -> bool {
        let (width, height) = self.viewport.canvas_size();
        if self.selection.is_empty() || width <= 0.0 || height <= 0.0 {
            return false;
        }
        let view = self.viewport.view();
        let objects: Vec<_> = self
            .selection
            .ids()
            .iter()
            .filter_map(|id| self.document.object(*id))
            .collect();
        !objects.iter().any(|object| {
            let Some((low, high)) = curvyo_ui_core::object_bounds_with_artwork(object, &|path| {
                self.width_artwork(path)
            }) else {
                return false;
            };
            let (left, top) = view.document_to_screen(low);
            let (right, bottom) = view.document_to_screen(high);
            let (min_x, max_x) = (left.min(right), left.max(right));
            let (min_y, max_y) = (top.min(bottom), top.max(bottom));
            max_x >= 0.0 && min_x <= width && max_y >= 0.0 && min_y <= height
        })
    }
}

/// Which history key this is: Ctrl (Cmd) with the typed character z, Shift+Z or, off macOS, y
/// (`specs/0020-undo-redo` criteria 1 to 3, 7). Matched by the character, not the key's place on
/// the keyboard, so QWERTZ works.
pub(super) fn history_direction(
    input: &super::keys::KeyInput<'_>,
    letter: Option<char>,
) -> Option<Direction> {
    if !input.ctrl || input.alt {
        return None;
    }
    match (letter?, input.shift) {
        ('z', false) => Some(Direction::Undo),
        ('z', true) => Some(Direction::Redo),
        ('y', false) if !input.mac => Some(Direction::Redo),
        _ => None,
    }
}

/// The gate of undo and redo (criterion 6): text fields keep their own undo, a drag, an open entry
/// chip and a long operation block the key, an unfinished Pen path answers with a hint. Auto-repeat
/// is not gated (criterion 5).
pub(super) fn history_action(
    direction: Direction,
    input: &super::keys::KeyInput<'_>,
    state: super::key_table::KeyState,
) -> super::key_table::KeyAction {
    if input.dom_blocked || state.operation_in_flight || state.entry_open {
        return super::key_table::KeyAction::Ignore;
    }
    if state.pen_open {
        return super::key_table::KeyAction::Hint(super::keys::KeyHint::FinishPenFirst);
    }
    super::key_table::KeyAction::History(direction)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod variable_width_tests {
    use super::*;
    use curvyo_document_core::{AnchorId, NewAnchor, Point};

    #[test]
    fn a_variable_stroke_off_screen_is_measured_by_its_outline() {
        let mut session = Session::new(1);
        session.resize_viewport(100.0, 100.0);
        let id = session.document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(5000.0, 10.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(5030.0, 10.0)),
            ],
            false,
        );
        session
            .document
            .set_width_point(id, AnchorId::new(1, 3), 0.5, 2.0)
            .unwrap();
        session.selection.set(&[id]);
        assert!(session.selection_is_off_screen());
    }
}
