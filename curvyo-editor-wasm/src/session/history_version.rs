//! `Session`'s Live mode of the Object scope: a press on a row goes to that version as one step,
//! further presses on the same object explore inside that step, and Restore previous state takes
//! the exploration back (`specs/0042-history-branches` criteria 10 to 13, 15, 16 and 31).

use curvyo_document_core::{NodeId, StepRef};
use curvyo_ui_core::{RESTORED_NOTICE, went_to_notice};

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

impl Session {
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
