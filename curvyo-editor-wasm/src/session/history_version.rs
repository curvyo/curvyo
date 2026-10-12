//! `Session`'s writes of a past version from the History list: Live mode's go to version (one
//! step, explored by further presses, taken back by Restore previous state) and Clone of any
//! version (`specs/0042-history-branches` criteria 10 to 13, 15, 16, 22 to 26 and 31).

use curvyo_document_core::{NodeId, StepRef};
use curvyo_ui_core::{RESTORED_NOTICE, cloned_notice, went_to_notice};

use super::Session;

/// The go-to step the maker is exploring in. View state: never saved.
#[derive(Debug, Default)]
pub(super) struct VersionState {
    exploring: Option<(NodeId, StepRef)>,
}

/// What a press on a row in Live mode did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoToOutcome {
    /// A drag, a chip, a long operation or an unfinished Pen path: ignored (criterion 16).
    Ignored,
    /// The item is no step row, or no single object is selected.
    NotARow,
    /// The object already has that version: nothing was written.
    Same,
    /// The object went to the version; the notice says so.
    Went(String),
}

/// What a press on Clone did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloneOutcome {
    /// A drag, a chip, a long operation or an unfinished Pen path: ignored (criterion 26).
    Ignored,
    /// No such row or touched object, a group (criterion 25), or a version that cannot be read.
    Refused,
    /// The clone exists and is selected; the notice says so.
    Cloned(String),
}

impl Session {
    /// Clone on item `index`: in the Object scope the version of the selected object (or of the
    /// object an inherited row came from); in the Document scope the version of the `touched`-th
    /// object the step touched (criteria 22 and 24). The clone is selected.
    pub fn clone_history_item(
        &mut self,
        object: bool,
        index: usize,
        touched: usize,
    ) -> CloneOutcome {
        if self.operation_in_flight() || self.select.has_entry() || self.pen_path_open() {
            return CloneOutcome::Ignored;
        }
        let Some(row) = self.history_row_at(object, index) else {
            return CloneOutcome::Refused;
        };
        let target = if object {
            self.history_object().and_then(|subject| {
                let typed = self.document.object_history_page(subject, row, 1).pop()?;
                Some((
                    typed.inherited_from.unwrap_or(subject),
                    typed.step,
                    typed.shown_id,
                ))
            })
        } else {
            self.document.history_page(row, 1).pop().and_then(|typed| {
                let id = typed.objects.get(touched)?.id;
                Some((id, typed.step, typed.shown_id))
            })
        };
        let Some((source, step, shown_id)) = target else {
            return CloneOutcome::Refused;
        };
        let result = self.recording(|this| {
            let minter = &mut this.minter;
            this.document
                .clone_version(source, step, &mut || minter.mint())
        });
        match result {
            Ok(clone) => {
                self.selection.select_single(clone);
                CloneOutcome::Cloned(cloned_notice(&shown_id))
            }
            Err(_) => CloneOutcome::Refused,
        }
    }

    /// A press, Enter or Space on item `index` of the Object scope in Live mode: the selected
    /// object goes to that version (criterion 10), continuing the exploration step while it is the
    /// top of the stack (criterion 11).
    pub fn go_to_history_item(&mut self, index: usize) -> GoToOutcome {
        if self.operation_in_flight() || self.select.has_entry() || self.pen_path_open() {
            return GoToOutcome::Ignored;
        }
        let Some(row) = self.history_row_at(true, index) else {
            return GoToOutcome::NotARow;
        };
        let Some(subject) = self.history_object() else {
            return GoToOutcome::NotARow;
        };
        let Some(typed) = self.document.object_history_page(subject, row, 1).pop() else {
            return GoToOutcome::NotARow;
        };
        let explore = self.history_exploring();
        let result = self.recording(|this| {
            this.document
                .go_to_version(subject, typed.step, typed.inherited_from, explore)
        });
        match result {
            Ok(report) => match report.step {
                Some(step) => {
                    self.history.version.exploring = Some((subject, step));
                    GoToOutcome::Went(went_to_notice(&typed.shown_id, report.others_reverted))
                }
                None => GoToOutcome::Same,
            },
            Err(_) => GoToOutcome::NotARow,
        }
    }

    /// Whether an exploration lasts: its step is the top of the maker's stack and its object is
    /// the one selected (criterion 12). Restore previous state is offered only then.
    pub fn history_exploring(&mut self) -> bool {
        self.sync_history();
        let Some((object, step)) = self.history.version.exploring else {
            return false;
        };
        let lasts = self.history_object() == Some(object)
            && self
                .history
                .stack
                .undo_target()
                .is_some_and(|top| top.step == step);
        if !lasts {
            self.history.version.exploring = None;
        }
        lasts
    }

    /// Restore previous state: Ctrl+Z on the exploration step, with the notice of criterion 12.
    /// Returns the notice, or `None` when no exploration lasts.
    pub fn restore_exploration(&mut self) -> Option<String> {
        if !self.history_exploring() {
            return None;
        }
        let result = self.undo();
        self.history.version.exploring = None;
        result.changed.then(|| RESTORED_NOTICE.to_owned())
    }

    /// A change of tab ends the exploration: the step stays a normal step (criterion 13).
    pub fn end_exploration(&mut self) {
        self.history.version.exploring = None;
    }
}
