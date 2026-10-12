//! `Session`'s preview of a version from the History list: a press on a row in Preview mode reads
//! the version as plain copies and the canvas draws them; nothing is written, replicated or saved
//! (`specs/0042-history-branches` criteria 14 and 16 to 21).

use curvyo_document_core::{HistoricalDocument, NodeId, ObjectSnapshot, StepRef, ViewTransform};
use curvyo_render_core::{DrawList, build_history_preview};
use curvyo_ui_core::HistoryItem;
use serde_json::{Value, json};

use super::Session;

/// How many rows back a preview may read before the long-operation state shows first
/// (criterion 21, ADR 0014 §13).
pub const FAR_PREVIEW_ROWS: usize = 1_000;

/// What the preview shows and the words of its readout or banner.
#[derive(Debug, Clone)]
struct Shown {
    /// The selected object of an Object-scope preview; `None` for a Document-scope preview.
    subject: Option<NodeId>,
    version: HistoricalDocument,
    /// The pressed row as the list has it: shown id, name, author and time.
    row: Value,
    /// The number of steps in the history when the preview started.
    steps_then: usize,
}

/// The preview state of a session. View state: never saved.
#[derive(Debug, Default)]
pub(super) struct PreviewState {
    /// The Mode switch: Live instead of Preview (criterion 14). Kept for the session.
    pub(super) live: bool,
    shown: Option<Shown>,
}

/// What a press on a row of the History list did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowPress {
    /// A drag, a chip, a long operation or an unfinished Pen path: ignored (criterion 16).
    Ignored,
    /// The item is no step row, or the scope has no subject.
    NotARow,
    /// The version shows over the canvas.
    Previewed,
    /// The pressed row is the version the object already has: the preview ended (criterion 18).
    Ended,
}

impl Session {
    /// The Mode switch of the Object scope: `true` for Live.
    pub fn set_history_live(&mut self, live: bool) {
        self.history.preview.live = live;
        if live {
            self.end_history_preview();
        }
    }

    /// Whether the Mode switch stands at Live.
    #[must_use]
    pub fn history_live(&self) -> bool {
        self.history.preview.live
    }

    /// Whether a press on item `index` reads a version far enough back that the long-operation
    /// state shows first (criterion 21).
    pub fn history_item_is_far(&mut self, object: bool, index: usize) -> bool {
        self.history_row_at(object, index)
            .is_some_and(|row| row >= FAR_PREVIEW_ROWS)
    }

    /// A press, Enter or Space on item `index` of the list in Preview mode: shows that version.
    pub fn preview_history_item(&mut self, object: bool, index: usize) -> RowPress {
        if self.operation_in_flight() || self.select.has_entry() || self.pen_path_open() {
            return RowPress::Ignored;
        }
        let Some(row) = self.history_row_at(object, index) else {
            return RowPress::NotARow;
        };
        let steps_then = self.document.history_len();
        let shown = if object {
            let Some(subject) = self.history_object() else {
                return RowPress::NotARow;
            };
            let Some(typed) = self.document.object_history_page(subject, row, 1).pop() else {
                return RowPress::NotARow;
            };
            let target = typed.inherited_from.unwrap_or(subject);
            let Ok(version) = self.document.preview_object_at(typed.step, target) else {
                return RowPress::NotARow;
            };
            let current: Vec<ObjectSnapshot> = self.document.object(subject).into_iter().collect();
            if target == subject && version.objects == current {
                self.end_history_preview();
                return RowPress::Ended;
            }
            Shown {
                subject: Some(subject),
                version,
                row: self
                    .object_history_row_values(row, 1)
                    .pop()
                    .unwrap_or_default(),
                steps_then,
            }
        } else {
            let Some(typed) = self.document.history_page(row, 1).pop() else {
                return RowPress::NotARow;
            };
            let Ok(version) = self.document.preview_document_at(typed.step) else {
                return RowPress::NotARow;
            };
            Shown {
                subject: None,
                version,
                row: self.history_row_values(row, 1).pop().unwrap_or_default(),
                steps_then,
            }
        };
        self.history.preview.shown = Some(shown);
        RowPress::Previewed
    }

    /// Ends the preview: Escape in the list, the "Now" row, Back to live, a change of tab or a
    /// collapsed panel (criteria 18 and 19).
    pub fn end_history_preview(&mut self) {
        self.history.preview.shown = None;
    }

    /// The preview as the panel and the canvas overlay read it: `{"kind": "object" | "document",
    /// "id", "name", "author", "time", "since"}`, or `{"kind": ""}` without one. An Object-scope
    /// preview ends here once its object is no longer the one selected (criterion 18).
    pub fn history_preview_json(&mut self) -> String {
        if self
            .history
            .preview
            .shown
            .as_ref()
            .is_some_and(|shown| shown.subject.is_some() && shown.subject != self.history_object())
        {
            self.end_history_preview();
        }
        let Some(shown) = &self.history.preview.shown else {
            return json!({"kind": ""}).to_string();
        };
        let since = self.document.history_len().saturating_sub(shown.steps_then);
        json!({
            "kind": if shown.subject.is_some() { "object" } else { "document" },
            "id": shown.row["id"],
            "name": shown.row["name"],
            "author": shown.row["author"],
            "time": shown.row["time"],
            "step": step_text(shown.version.step),
            "since": since,
        })
        .to_string()
    }

    /// Whether a Document-scope preview pauses editing (criterion 19).
    #[must_use]
    pub fn document_preview_active(&self) -> bool {
        self.history
            .preview
            .shown
            .as_ref()
            .is_some_and(|shown| shown.subject.is_none())
    }

    /// A press on the canvas while a preview shows: an Object-scope preview ends and the press
    /// does nothing else; during a Document-scope preview the press is ignored. Returns whether
    /// the press was taken.
    pub(super) fn history_preview_takes_press(&mut self) -> bool {
        match &self.history.preview.shown {
            None => false,
            Some(shown) if shown.subject.is_none() => true,
            Some(_) => {
                self.end_history_preview();
                true
            }
        }
    }

    /// The objects a Document-scope preview draws in place of the document.
    pub(super) fn document_preview_objects(&self) -> Option<&[ObjectSnapshot]> {
        let shown = self.history.preview.shown.as_ref()?;
        shown
            .subject
            .is_none()
            .then_some(&shown.version.objects[..])
    }

    /// An Object-scope preview over the selected object: the version's ghost and its blue outline
    /// (criterion 17); empty without one.
    pub(super) fn object_preview_list(&self, view: ViewTransform) -> DrawList {
        match self.history.preview.shown.as_ref() {
            Some(shown) if shown.subject.is_some() && shown.subject == self.history_object() => {
                build_history_preview(&shown.version.objects, view)
            }
            _ => DrawList::default(),
        }
    }

    /// The row index of item `index`, when it is a step row.
    fn history_row_at(&mut self, object: bool, index: usize) -> Option<usize> {
        match self.history_list_item(object, index)? {
            HistoryItem::Row(row) => Some(row),
            _ => None,
        }
    }
}

fn step_text(step: StepRef) -> String {
    format!("{}@{}", step.peer(), step.counter())
}
