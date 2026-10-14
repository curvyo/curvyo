//! `Session`'s writes of a past version from the History list: applying a version to the document
//! or the selected object (Enter or a double-click on a row), and Clone of any version
//! (`specs/0042-history-branches` criteria 10, 15, 16, 22 to 26 and 31).

use curvyo_document_core::SMALL_STEP_OPS;
use curvyo_ui_core::{cloned_notice, went_to_notice};

use super::Session;

/// What Enter or a double-click on a row did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoToOutcome {
    /// A drag, a chip, a long operation or an unfinished Pen path: ignored (criterion 16).
    Ignored,
    /// The item is no step row, or no single object is selected.
    NotARow,
    /// The document or the object already has that version: nothing was written.
    Same,
    /// The version was applied as one step; the notice says so.
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

    /// The step of item `index` and, in the Object scope, the selected object and the object an
    /// inherited row came from.
    fn version_of_item(
        &mut self,
        object: bool,
        index: usize,
    ) -> Option<(
        curvyo_document_core::StepRef,
        String,
        Option<curvyo_document_core::NodeId>,
        Option<curvyo_document_core::NodeId>,
    )> {
        let row = self.history_row_at(object, index)?;
        if object {
            let subject = self.history_object()?;
            let typed = self.document.object_history_page(subject, row, 1).pop()?;
            Some((
                typed.step,
                typed.shown_id,
                Some(subject),
                typed.inherited_from,
            ))
        } else {
            let typed = self.document.history_page(row, 1).pop()?;
            Some((typed.step, typed.shown_id, None, None))
        }
    }

    /// Whether applying item `index` needs the long-operation state first: the version lies more
    /// than a small step's operations behind (criterion 10, ADR 0014 §1).
    pub fn history_apply_needs_busy_state(&mut self, object: bool, index: usize) -> bool {
        self.version_of_item(object, index)
            .and_then(|(step, ..)| self.document.step_distance(step))
            .is_some_and(|ops| ops > SMALL_STEP_OPS)
    }

    /// Enter or a double-click on item `index`: applies its version to the whole document, or in
    /// the Object scope to the selected object, as one step "Go to version" that Ctrl+Z takes
    /// back (criterion 10). A preview that shows ends.
    pub fn apply_history_item(&mut self, object: bool, index: usize) -> GoToOutcome {
        if self.operation_in_flight() || self.select.has_entry() || self.pen_path_open() {
            return GoToOutcome::Ignored;
        }
        let Some((step, shown_id, subject, source)) = self.version_of_item(object, index) else {
            return GoToOutcome::NotARow;
        };
        self.end_history_preview();
        let result = self.recording(|this| this.document.go_to_version(step, subject, source));
        match result {
            Ok(report) if report.step.is_some() => {
                GoToOutcome::Went(went_to_notice(&shown_id, report.others_reverted, object))
            }
            Ok(_) => GoToOutcome::Same,
            Err(_) => GoToOutcome::NotARow,
        }
    }
}
