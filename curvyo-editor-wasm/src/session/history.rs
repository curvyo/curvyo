//! `Session`'s undo and redo (`specs/0020-undo-redo`): recording the steps the maker makes with the
//! selection around them, taking them back and bringing them back, and the words of the notice.

use std::collections::{HashMap, HashSet};

use curvyo_document_core::{
    AnchorId, Direction, FAR_STEP_OPS, NodeId, SMALL_STEP_OPS, StepRef, Timestamp,
};
use curvyo_ui_core::{
    SelectionSnapshot, UndoStack, nothing_notice, object_bounds, objects_summary, operation_name,
    restore_notice,
};

use super::{KeyOutcome, Session};

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
    notice: Option<String>,
}

impl HistoryState {
    pub(super) fn new() -> Self {
        Self {
            stack: UndoStack::new(),
            recorded: 0,
            clock: wall_clock,
            notice: None,
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
    fn selection_snapshot(&self) -> SelectionSnapshot {
        SelectionSnapshot {
            objects: self.selection.ids().to_vec(),
            nodes: self.node.selection().node_pairs().to_vec(),
        }
    }

    /// Runs `body`, which may commit steps, and puts every step it started on the undo stack with
    /// the selection before it ran and the one it left. Every entry point that writes the
    /// document goes through here, so one maker's interaction is one stack entry.
    pub(super) fn recording<R>(&mut self, body: impl FnOnce(&mut Self) -> R) -> R {
        let before = self.selection_snapshot();
        self.document.set_step_time((self.history.clock)());
        let result = body(self);
        self.record_new_steps(&before);
        result
    }

    /// Puts the steps the document started since the last look on the undo stack.
    fn record_new_steps(&mut self, before: &SelectionSnapshot) {
        let new = self.document.own_steps_from(self.history.recorded);
        if new.is_empty() {
            return;
        }
        let after = self.selection_snapshot();
        for step in &new {
            self.history
                .stack
                .record(step.step, &step.label, before.clone(), after.clone());
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
        Some((target.step, target.label))
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
    pub fn history_target_name(&mut self, direction: Direction) -> Option<&'static str> {
        self.sync_history();
        self.target_step(direction)
            .map(|(_, label)| operation_name(&label))
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
        self.flush_select_bar_preview();
        self.flush_style_preview();
        self.sync_history();
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
        let name = operation_name(&target.label);
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
        }
        let shapes: Vec<_> = report.objects.iter().map(|o| o.shape).collect();
        let summary = objects_summary(&shapes);
        let off_screen = report.wrote && self.selection_is_off_screen();
        HistoryResult {
            notice: restore_notice(name, &report, summary.as_deref(), off_screen),
            changed: report.wrote,
        }
    }

    /// Selects what `snapshot` held, leaving out what no longer exists. Reads only the objects the
    /// snapshot names, so the cost does not grow with the size of the drawing.
    fn restore_selection(&mut self, snapshot: &SelectionSnapshot) {
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
        !self
            .selection
            .ids()
            .iter()
            .filter_map(|id| self.document.object(*id))
            .any(|object| {
                let (low, high) = object_bounds(&object);
                let (left, top) = view.document_to_screen(low);
                let (right, bottom) = view.document_to_screen(high);
                let (min_x, max_x) = (left.min(right), left.max(right));
                let (min_y, max_y) = (top.min(bottom), top.max(bottom));
                max_x >= 0.0 && min_x <= width && max_y >= 0.0 && min_y <= height
            })
    }
}
