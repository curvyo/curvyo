//! `Session`'s History tab: the rows of the global history as the panel reads them, and the hover
//! link from a row to the objects it touched (`specs/0020-undo-redo` criteria 32 to 40).

use curvyo_document_core::{Author, HistoryRow, ObjectShape, Role, Status, StepKind, StepRef};
use curvyo_ui_core::{known_operation_name, objects_summary, operation_name, summary_shapes};
use serde_json::{Value, json};

use super::Session;

fn kind_word(kind: StepKind) -> &'static str {
    match kind {
        StepKind::Create => "create",
        StepKind::Delete => "delete",
        StepKind::Replace => "replace",
        StepKind::Change => "change",
        StepKind::Document => "document",
    }
}

fn shape_word(shape: ObjectShape) -> &'static str {
    match shape {
        ObjectShape::Path => "path",
        ObjectShape::Rectangle => "rectangle",
        ObjectShape::Ellipse => "ellipse",
        ObjectShape::Polygon => "polygon",
        ObjectShape::Star => "star",
        ObjectShape::Group => "group",
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

fn author_word(author: Author) -> &'static str {
    match author {
        Author::You => "you",
        Author::Earlier => "earlier",
    }
}

/// The name, whether the table knows the label, the row class and the summary of a row.
fn row_words(row: &HistoryRow, summary: Option<String>) -> (String, bool, &'static str, String) {
    let summary = summary.unwrap_or_default();
    if row.label == curvyo_document_core::WIPE_LABEL {
        return ("History wiped".to_owned(), true, "step", summary);
    }
    if row.label == curvyo_document_core::LABEL_WIPE_OBJECT {
        let object = row.objects.first().map_or_else(String::new, |o| {
            format!("{}, {}", shape_name(o.shape), o.id.short_id())
        });
        let name = format!("Wipe history of {object}");
        return (name, true, "floor", summary);
    }
    if let Some(version) = &row.version {
        // "Go to version a1b2c3d", line 2 "Resize" (`specs/0042-history-branches` criterion 11).
        let name = format!("{} {}", operation_name(&row.label), version.shown_id);
        return (
            name,
            true,
            "step",
            operation_name(&version.label).to_owned(),
        );
    }
    if let Some(revert) = &row.reverts {
        let verb = if revert.redo { "Redo" } else { "Undo" };
        let name = format!("{verb} {}", operation_name(&revert.label));
        let noun = if revert.redo { "restores" } else { "reverts" };
        let class = if revert.redo { "redoes" } else { "undoes" };
        return (name, true, class, format!("{noun} {}", revert.shown_id));
    }
    (
        operation_name(&row.label).to_owned(),
        known_operation_name(&row.label).is_some(),
        "step",
        summary,
    )
}

/// The kind of an object as a title, for "Wipe history of Rectangle, 3f9a1c".
fn shape_name(shape: ObjectShape) -> &'static str {
    match shape {
        ObjectShape::Path => "Path",
        ObjectShape::Rectangle => "Rectangle",
        ObjectShape::Ellipse => "Ellipse",
        ObjectShape::Polygon => "Polygon",
        ObjectShape::Star => "Star",
        ObjectShape::Group => "Group",
    }
}

/// A step reference as the panel sends it back: `<peer>@<counter>`.
fn step_text(step: StepRef) -> String {
    format!("{}@{}", step.peer(), step.counter())
}

impl Session {
    fn row_json(&self, row: &HistoryRow) -> Value {
        let touched: Vec<_> = row.objects.iter().map(|o| (o.shape, o.role)).collect();
        let shapes = summary_shapes(&row.label, &touched);
        let (name, known, class, summary) = row_words(row, objects_summary(&shapes));
        let objects: Vec<Value> = row
            .objects
            .iter()
            .map(|o| {
                json!({
                    "id": serde_json::to_value(o.id).unwrap_or_default(),
                    "shape": shape_word(o.shape),
                    "role": role_word(o.role),
                    "alive": self.document.object(o.id).is_some(),
                })
            })
            .collect();
        let (status, undone_by) = match (row.status, row.undone_by) {
            (Status::Applied, Some(author)) => ("undone-by", author_word(author)),
            (Status::Applied, None) => ("applied", ""),
            (Status::Undone, _) => ("undone", ""),
            (Status::OnBranch, _) => ("branch", ""),
        };
        json!({
            "id": row.shown_id,
            "step": step_text(row.step),
            "label": row.label,
            "name": name,
            "known": known,
            "author": author_word(row.author),
            "time": row.time.as_secs(),
            "status": status,
            "undoneBy": undone_by,
            "class": class,
            "kind": kind_word(row.kind),
            "summary": summary,
            "objects": objects,
        })
    }

    /// Changes whenever the log does: the panel reads its rows again only then.
    pub fn history_stamp(&self) -> u32 {
        self.document.history_stamp()
    }

    /// The number of steps in the global history (the rows of the History list).
    pub fn history_len(&mut self) -> usize {
        self.sync_history();
        self.document.history_len()
    }

    /// `count` rows of the history from the `start`-th newest, newest first, as one JSON array.
    pub fn history_rows_json(&mut self, start: usize, count: usize) -> String {
        Value::Array(self.history_row_values(start, count)).to_string()
    }

    /// `count` rows of the history from the `start`-th newest, newest first.
    pub(super) fn history_row_values(&mut self, start: usize, count: usize) -> Vec<Value> {
        self.sync_history();
        self.document
            .history_page(start, count)
            .iter()
            .map(|row| self.row_json(row))
            .collect()
    }

    /// The shown id of the newest step in effect, for the pinned "Now" row; empty without a step.
    pub fn history_now_id(&mut self) -> String {
        self.sync_history();
        self.document
            .history_newest_applied_id()
            .unwrap_or_default()
    }

    /// The pointer or focus rests on the row of `step` (`<peer>@<counter>`), or on none (empty):
    /// the objects it touched that still exist show the hover box of `0014` (criterion 40).
    pub fn set_history_hover(&mut self, step: &str) {
        self.history_hover = step
            .split_once('@')
            .and_then(|(peer, counter)| {
                Some(StepRef::new(peer.parse().ok()?, counter.parse().ok()?))
            })
            .map(|step| self.document.touched_objects(step))
            .unwrap_or_default()
            .into_iter()
            .filter(|id| self.document.object(*id).is_some())
            .collect();
    }
}

/// What the footer of the History tab offers (`specs/0020-undo-redo` criteria 47 and 50).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeState {
    /// No step: the control is not in the tree.
    NoSteps,
    /// "Wipe history" is offered; the number is the steps it would remove.
    Offered(usize),
    /// The document is shared: the control gives way to a line.
    Shared,
}

/// What a press on "Wipe" did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeOutcome {
    /// A drag, an open chip or an unfinished Pen path: ignored (criterion 51).
    Ignored,
    /// The document is shared.
    Shared,
    /// The log could not be rewritten; the document is as it was.
    Failed,
    /// The history was wiped; the number is the steps removed.
    Wiped(usize),
}

impl Session {
    /// What the footer offers now.
    pub fn history_wipe_state(&mut self) -> WipeState {
        if self.document.is_shared() {
            return WipeState::Shared;
        }
        match self.history_len() {
            0 => WipeState::NoSteps,
            n => WipeState::Offered(n),
        }
    }

    /// Whether a wipe would be ignored because the Pen holds an unfinished path: the panel then
    /// shows the hint and does not open its confirm (`specs/0020-undo-redo` criteria 6e and 51).
    #[must_use]
    pub fn history_wipe_blocked(&self) -> bool {
        self.pen_path_open()
    }

    /// "Wipe" was confirmed: cuts the history, empties both stacks and writes the one step
    /// "History wiped". The drawing is unchanged.
    pub fn wipe_history(&mut self) -> WipeOutcome {
        if self.operation_in_flight() || self.select.has_entry() || self.pen_path_open() {
            return WipeOutcome::Ignored;
        }
        self.sync_history();
        self.document.set_step_time(self.history_clock()());
        match self.document.wipe_history() {
            Ok(removed) => {
                self.reset_history_stacks();
                self.history_hover.clear();
                WipeOutcome::Wiped(removed)
            }
            Err(curvyo_document_core::HistoryError::Wipe) => WipeOutcome::Failed,
            Err(curvyo_document_core::HistoryError::NoSteps) => WipeOutcome::Ignored,
            Err(_) => WipeOutcome::Shared,
        }
    }
}
