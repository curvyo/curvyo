//! `Session`'s object history (`specs/0041-object-history`): Ctrl+U and Ctrl+Shift+U for the one
//! selected object, the rows and the subject of the Object scope of the History tab, and the wipe
//! of one object's history.

use curvyo_document_core::{
    Author, Direction, HistoryError, NodeId, ObjectRow, ObjectRowClass, ObjectStatus, ObjectTarget,
    Role, StepKind,
};
use curvyo_ui_core::{
    FINISH_PEN_FIRST, SELECT_ONE_OBJECT, SelectionSnapshot, known_operation_name, object_kind_name,
    object_nothing_notice, object_restore_notice, object_row_name, object_row_summary,
    operation_name,
};
use serde_json::{Value, json};

use super::history::HistoryResult;
use super::keys::{KeyAction, KeyHint, KeyInput, KeyOutcome, KeyState};
use super::{Session, Tool};

/// What a press on "Wipe" of an object's history did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectWipeOutcome {
    /// A drag, an open chip, an unfinished Pen path or no single object: ignored.
    Ignored,
    /// The log could not take the step; the document is as it was.
    Failed,
    /// The history of the object was hidden; the number is the rows hidden.
    Wiped(usize),
}

fn author_word(author: Author) -> &'static str {
    match author {
        Author::You => "you",
        Author::Earlier => "earlier",
    }
}

fn nothing(notice: &str) -> HistoryResult {
    HistoryResult {
        notice: notice.to_owned(),
        changed: false,
    }
}

impl Session {
    /// The one selected object the object history acts on; `None` for none or several.
    #[must_use]
    pub fn history_object(&self) -> Option<NodeId> {
        match self.selection.ids() {
            [id] => Some(*id),
            _ => None,
        }
    }

    /// The step an object undo or redo of the selected object would act on.
    fn object_target(&mut self, direction: Direction) -> Option<ObjectTarget> {
        self.sync_history();
        let id = self.history_object()?;
        let targets = self.document.object_history_targets(id);
        match direction {
            Direction::Undo => targets.undo,
            Direction::Redo => targets.redo,
        }
    }

    /// The operation name of the step an object undo or redo would act on, for the tooltips and
    /// the Edit menu.
    pub fn object_history_target_name(&mut self, direction: Direction) -> Option<String> {
        self.object_target(direction)
            .map(|target| operation_name(&target.label).to_owned())
    }

    /// Whether the press would take long enough to show the long-operation state first: the
    /// per-object index or the history has not been read since the file was opened, or the step is
    /// large or far behind (`specs/0041-object-history` criterion 17).
    pub fn object_history_needs_busy_state(&mut self, direction: Direction) -> bool {
        self.sync_history();
        if self.history_object().is_none() {
            return false;
        }
        if !self.document.history_is_warm() || !self.document.object_history_is_cheap() {
            return true;
        }
        self.object_target(direction).is_some_and(|target| {
            self.document
                .step_op_count(target.step)
                .is_some_and(|ops| ops > curvyo_document_core::SMALL_STEP_OPS)
                || self
                    .document
                    .step_distance(target.step)
                    .is_some_and(|ops| ops > curvyo_document_core::FAR_STEP_OPS)
        })
    }

    /// The key press of Ctrl+U or Ctrl+Shift+U that passed the gate.
    pub(super) fn object_history_key(&mut self, direction: Direction) -> KeyOutcome {
        let large = self.object_history_needs_busy_state(direction)
            && self.object_target(direction).is_some();
        match (direction, large) {
            (Direction::Undo, true) => KeyOutcome::ObjectUndoBusy,
            (Direction::Redo, true) => KeyOutcome::ObjectRedoBusy,
            (Direction::Undo, false) => {
                let result = self.object_undo();
                self.history.notice = Some(result.notice);
                KeyOutcome::ObjectUndo
            }
            (Direction::Redo, false) => {
                let result = self.object_redo();
                self.history.notice = Some(result.notice);
                KeyOutcome::ObjectRedo
            }
        }
    }

    /// Takes the latest applied step of the selected object back (Ctrl+U, the Undo step button).
    pub fn object_undo(&mut self) -> HistoryResult {
        self.press_object_history(Direction::Undo)
    }

    /// Brings back the step the last object undo took back (Ctrl+Shift+U, the Redo step button).
    pub fn object_redo(&mut self) -> HistoryResult {
        self.press_object_history(Direction::Redo)
    }

    fn press_object_history(&mut self, direction: Direction) -> HistoryResult {
        if self.operation_in_flight() || self.select.has_entry() {
            return nothing("");
        }
        if self.pen_path_open() {
            return nothing(FINISH_PEN_FIRST);
        }
        if !matches!(self.tool, Tool::Select | Tool::Node) {
            return nothing("");
        }
        let Some(id) = self.history_object() else {
            return nothing(SELECT_ONE_OBJECT);
        };
        self.flush_select_bar_preview();
        self.flush_style_preview();
        self.sync_history();
        self.document.set_step_time((self.history_clock())());
        let before = self.selection_snapshot();
        let outcome = match direction {
            Direction::Undo => self.document.undo_object_step(id),
            Direction::Redo => self.document.redo_object_step(id),
        };
        let restore = match outcome {
            Ok(Some(restore)) => restore,
            Ok(None) => return nothing(object_nothing_notice(direction)),
            Err(_) => {
                let verb = if direction == Direction::Undo {
                    "undo"
                } else {
                    "redo"
                };
                return nothing(&format!("Could not {verb} the step of this object."));
            }
        };
        if restore.report.wrote {
            // The selection stays on the object, or is empty when the step removed it.
            let survivors = if self.document.object(id).is_some() {
                vec![id]
            } else {
                Vec::new()
            };
            let snapshot = SelectionSnapshot {
                objects: survivors,
                nodes: self.selection_snapshot().nodes,
            };
            self.restore_selection(&snapshot);
            self.command_refusal = None;
            self.record_new_steps(&before);
        }
        HistoryResult {
            notice: object_restore_notice(&restore),
            changed: restore.report.wrote,
        }
    }

    /// Hides everything before now from the history of the selected object and drops the stack
    /// entries that touched only this object (`specs/0041-object-history` criteria 30 and 31).
    pub fn wipe_object_history(&mut self) -> ObjectWipeOutcome {
        if self.operation_in_flight() || self.select.has_entry() || self.pen_path_open() {
            return ObjectWipeOutcome::Ignored;
        }
        let Some(id) = self.history_object() else {
            return ObjectWipeOutcome::Ignored;
        };
        self.sync_history();
        self.document.set_step_time((self.history_clock())());
        match self.document.wipe_object_history(id) {
            Ok(hidden) => {
                let document = &self.document;
                self.history.stack.retain_steps(|step| {
                    let touched = document.touched_objects(step);
                    touched.is_empty() || touched.iter().any(|object| *object != id)
                });
                self.history_hover.clear();
                ObjectWipeOutcome::Wiped(hidden)
            }
            Err(HistoryError::NoSteps | HistoryError::NoSuchObject) => ObjectWipeOutcome::Ignored,
            Err(_) => ObjectWipeOutcome::Failed,
        }
    }

    /// The subject of the Object scope as one JSON object: the kind and ids of the selected object,
    /// the steps its buttons would act on, and what a wipe would hide. Empty for none or several
    /// selected objects.
    pub fn object_history_info_json(&mut self) -> String {
        self.sync_history();
        let Some(id) = self.history_object() else {
            return String::new();
        };
        let Some(object) = self.document.object(id) else {
            return String::new();
        };
        let targets = self.document.object_history_targets(id);
        let target = |target: Option<ObjectTarget>| -> Value {
            target.map_or(Value::Null, |target| {
                json!({
                    "id": target.shown_id,
                    "name": operation_name(&target.label),
                    "author": author_word(target.author),
                    "time": target.time.as_secs(),
                })
            })
        };
        json!({
            "key": id.full_id(),
            "kind": object_kind_name(&object),
            "id": id.short_id(),
            "fullId": id.full_id(),
            "undo": target(targets.undo),
            "redo": target(targets.redo),
            "wipeCount": self.document.object_wipe_count(id),
            "total": self.document.object_history_len(id),
        })
        .to_string()
    }

    /// The number of rows in the Object scope for the selected object.
    pub fn object_history_len(&mut self) -> usize {
        self.sync_history();
        self.history_object()
            .map_or(0, |id| self.document.object_history_len(id))
    }

    /// `count` rows of the selected object's timeline from the `start`-th newest, as a JSON array.
    pub fn object_history_rows_json(&mut self, start: usize, count: usize) -> String {
        self.sync_history();
        let Some(id) = self.history_object() else {
            return "[]".to_owned();
        };
        let rows: Vec<Value> = self
            .document
            .object_history_page(id, start, count)
            .iter()
            .map(object_row_json)
            .collect();
        Value::Array(rows).to_string()
    }
}

fn kind_word(kind: StepKind) -> &'static str {
    match kind {
        StepKind::Create => "create",
        StepKind::Delete => "delete",
        StepKind::Replace => "replace",
        StepKind::Change => "change",
        StepKind::Document => "document",
    }
}

fn role_word(role: Role) -> &'static str {
    match role {
        Role::Created => "created",
        Role::Deleted => "deleted",
        Role::Changed => "changed",
        Role::Moved => "moved",
    }
}

fn object_row_json(row: &ObjectRow) -> Value {
    let (class, reverts) = match &row.class {
        ObjectRowClass::Step => ("step", Value::Null),
        ObjectRowClass::Reverts { shown_id, redo, .. } => (
            if *redo { "redoes" } else { "undoes" },
            Value::String(shown_id.clone()),
        ),
        ObjectRowClass::CloneStart { source } => ("clone-start", Value::String(source.short_id())),
        ObjectRowClass::Floor => ("floor", Value::Null),
    };
    let (status, undone_by) = match row.status {
        ObjectStatus::Applied => ("applied", ""),
        ObjectStatus::Undone => ("undone", ""),
        ObjectStatus::UndoneBy(author) => ("undone-by", author_word(author)),
    };
    json!({
        "id": row.shown_id,
        "step": format!("{}@{}", row.step.peer(), row.step.counter()),
        "label": row.label,
        "name": object_row_name(row),
        "known": known_operation_name(&row.label).is_some() || !matches!(row.class, ObjectRowClass::Step),
        "author": author_word(row.author),
        "time": row.time.as_secs(),
        "class": class,
        "target": reverts,
        "kind": kind_word(row.kind),
        "role": role_word(row.role),
        "summary": object_row_summary(row),
        "status": status,
        "undoneBy": undone_by,
        "from": row.inherited_from.map(NodeId::short_id).unwrap_or_default(),
        "sources": row.sources.iter().map(|id| id.full_id()).collect::<Vec<_>>(),
        "objects": [],
    })
}

/// Which object-history key this is: Ctrl (Cmd) with the typed character u, Shift+U for the redo
/// (`specs/0041-object-history` criterion 5). Matched by the character, so QWERTZ works.
pub(super) fn object_history_direction(
    input: &KeyInput<'_>,
    letter: Option<char>,
) -> Option<Direction> {
    if !input.ctrl || input.alt || letter? != 'u' {
        return None;
    }
    Some(if input.shift {
        Direction::Redo
    } else {
        Direction::Undo
    })
}

/// The gate of the object history keys: the gate of undo and redo (criterion 6 of `0020`), and
/// only the Select tool and the Node tool act.
pub(super) fn object_history_action(
    direction: Direction,
    input: &KeyInput<'_>,
    state: KeyState,
) -> KeyAction {
    if input.dom_blocked || state.operation_in_flight || state.entry_open {
        return KeyAction::Ignore;
    }
    if !matches!(state.tool, Tool::Select | Tool::Node) {
        return KeyAction::Ignore;
    }
    if state.pen_open {
        return KeyAction::Hint(KeyHint::FinishPenFirst);
    }
    KeyAction::ObjectHistory(direction)
}
