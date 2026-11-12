//! Assigning and removing items of the selection, of a clone member and by number (criteria 8, 9,
//! 10; ADR 0046 §2 and Frage 167 B). Guides and dimensions are never targets (criterion 10).

use curvyo_document_core::{NodeId, ObjectSnapshot};
use curvyo_laser_core::operations::OperationList;

use super::super::Session;

/// The selection as operation targets: each target is its path (one id for an own item).
#[derive(Debug, Default)]
pub(super) struct Selection {
    /// The assignable items, as paths.
    pub(super) targets: Vec<Vec<NodeId>>,
    /// The selected guides (named in the notice).
    pub(super) guides: usize,
    /// Every selected item that is not a target (guides and dimensions).
    pub(super) skipped: usize,
    /// The selected items of a test grid: they run with their grid row only (`specs/0028`
    /// criterion 6), so they are neither targets nor counted as skipped.
    pub(super) in_grid: usize,
}

/// The notice of criterion 10 for `guides` selected guides.
fn not_assigned_notice(guides: usize) -> String {
    if guides == 1 {
        format!("1 guide not assigned. {}", super::NEVER_RUN_NOTICE)
    } else {
        format!("{guides} guides not assigned. {}", super::NEVER_RUN_NOTICE)
    }
}

impl Session {
    /// The selection's assignable items, guides and dimensions counted apart. A selected group is one
    /// target, the group node (ADR 0046 rule 2, criteria 11, 12): its leaves are not.
    pub(super) fn selection_targets(&self) -> Selection {
        let read = self.document_read();
        let mut selection = Selection::default();
        for unit in self.units().selected(self.selection.ids()) {
            let id = unit.id;
            if self.in_test_grid(id) {
                selection.in_grid += 1;
            } else if read.guides.contains(&id) {
                selection.guides += 1;
                selection.skipped += 1;
            } else if read
                .leaves
                .iter()
                .any(|o| o.id() == id && matches!(o, ObjectSnapshot::Dimension(_)))
            {
                selection.skipped += 1;
            } else {
                selection.targets.push(vec![id]);
            }
        }
        selection
    }

    /// Sets the notice of criterion 10 when the selection held guides, else, for an assignment
    /// (`adding`), the grid notice when it held items of a test grid (the assigned rest goes on).
    pub(super) fn note_skipped(&mut self, selection: &Selection, adding: bool) {
        if selection.guides > 0 {
            self.operations.notice = Some(not_assigned_notice(selection.guides));
        } else if adding && selection.in_grid > 0 {
            self.operations.notice = Some(super::GRID_ITEMS_NOTICE.to_owned());
        }
    }

    /// Ticks (`member` true) or unticks the selection's items in operation `id` (criterion 8: a
    /// tick row, or a drag of the selection onto the row). One step, "Assign to" or "Remove from".
    pub fn set_selection_in_operation(&mut self, id: NodeId, member: bool) -> bool {
        self.operations.notice = None;
        if !self.is_active_operation(id) {
            return false;
        }
        let selection = self.selection_targets();
        self.write_targets(id, &selection, member)
    }

    /// Assigns (`member` true) or removes the member `member_node` of clone `clone` in operation
    /// `id`, by its path from the clone (Frage 167 B). `false` when `member_node` is not below the
    /// clone's original.
    pub fn set_clone_member_in_operation(
        &mut self,
        id: NodeId,
        clone: NodeId,
        member_node: NodeId,
        member: bool,
    ) -> bool {
        self.operations.notice = None;
        if !self.is_active_operation(id) {
            return false;
        }
        let Some(path) = self.document.clone_member_path(clone, member_node) else {
            return false;
        };
        self.recording_unguarded(|this| {
            this.document
                .set_operation_members(id, &[path], member)
                .unwrap_or(false)
        })
    }

    /// Number key `number` (1 to 9): toggles the selection in operation `number` of the list
    /// (criterion 8). Every selected item in it is removed; otherwise (none or some) all are added.
    /// The digits count the rows as shown and skip test grid rows (`specs/0028` UX notes §5).
    pub fn toggle_selection_by_number(&mut self, number: usize) -> bool {
        self.operations.notice = None;
        let Some(active) = self.active_operations() else {
            return false;
        };
        let list = OperationList::new(&active.operations, active.cuts_last);
        let Some(op) = number.checked_sub(1).and_then(|at| {
            list.sequence()
                .iter()
                .map(|&index| &active.operations[index])
                .filter(|op| !op.grid)
                .nth(at)
        }) else {
            return false;
        };
        let selection = self.selection_targets();
        let all_in = !selection.targets.is_empty()
            && selection
                .targets
                .iter()
                .all(|target| op.members.contains(target));
        let id = op.id;
        self.write_targets(id, &selection, !all_in)
    }

    /// Writes the selection's targets into or out of operation `id`, one step; a selection of only
    /// guides, dimensions or test grid items writes nothing and sets the refusal notice.
    fn write_targets(&mut self, id: NodeId, selection: &Selection, member: bool) -> bool {
        if selection.targets.is_empty() {
            // Only guides and grid items name themselves in the notice; dimensions are skipped
            // silently.
            if selection.guides > 0 {
                self.operations.notice = Some(super::NEVER_RUN_NOTICE.to_owned());
            } else if member && selection.in_grid > 0 {
                self.operations.notice = Some(super::GRID_ITEMS_NOTICE.to_owned());
            }
            return false;
        }
        self.note_skipped(selection, member);
        let targets = selection.targets.clone();
        self.recording_unguarded(|this| {
            this.document
                .set_operation_members(id, &targets, member)
                .unwrap_or(false)
        })
    }
}
