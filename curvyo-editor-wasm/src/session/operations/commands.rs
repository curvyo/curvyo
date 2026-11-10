//! The commands of an operation's own row and of the list: add, rename, colour, delete, margin,
//! the Show and Output switches, Cuts last and the reorder (`specs/0101-laser-operations`
//! criteria 2 to 6, 15, 16, 23). Each is one history step with the label of the history table.

use curvyo_document_core::{Length, NewOperation, NodeId, OperationSwitch};
use curvyo_laser_core::operations::{
    OperationKind, insert_index, may_move, next_operation_name, run_sequence,
};

use super::super::Session;

impl Session {
    /// Adds an operation of the stored kind `kind` (`cut`, `score`, `engrave`, `engrave_outline`,
    /// `cut_out`) on the active laser page, named as criterion 2 says and placed in
    /// its section. With items selected they are assigned to it in the same step (criterion 8).
    /// `None` for an unknown kind, another page, or a selection of only guides and dimensions
    /// (nothing is added then, and the refusal notice is shown).
    pub fn press_operation_kind(&mut self, kind: &str) -> Option<NodeId> {
        self.operations.notice = None;
        let kind = OperationKind::parse(kind)?;
        let active = self.active_operations()?;
        let selection = self.selection_targets();
        if selection.targets.is_empty() && selection.skipped > 0 {
            if selection.guides > 0 {
                self.operations.notice = Some(super::NEVER_RUN_NOTICE.to_owned());
            }
            return None;
        }
        let names: Vec<&str> = active
            .operations
            .iter()
            .map(|op| op.name.as_str())
            .collect();
        let name = next_operation_name(kind, names);
        let index = insert_index(&active.kinds(), kind.section(), active.cuts_last);
        let new = NewOperation {
            kind: kind.as_str(),
            name: &name,
        };
        self.note_skipped_guides(selection.guides);
        self.recording_unguarded(|this| {
            this.document
                .add_operation(active.manufacturing, &new, index, &selection.targets)
                .ok()
        })
    }

    /// Renames operation `id`; an empty name reverts (criterion 3). `false` when nothing changes.
    pub fn rename_operation(&mut self, id: NodeId, name: &str) -> bool {
        self.operations.notice = None;
        self.is_active_operation(id)
            && self.recording_unguarded(|this| {
                this.document.rename_operation(id, name).unwrap_or(false)
            })
    }

    /// Deletes operation `id` with every membership it holds; no drawing object is touched
    /// (criterion 4). One step, undone with its memberships.
    pub fn delete_operation(&mut self, id: NodeId) -> bool {
        self.operations.notice = None;
        self.is_active_operation(id)
            && self.recording_unguarded(|this| this.document.delete_operation(id).is_ok())
    }

    /// Sets the Cut-out margin of operation `id` in mm (criterion 23). A negative or non-finite
    /// value is refused. `false` when nothing changes.
    pub fn set_operation_margin(&mut self, id: NodeId, margin_mm: f64) -> bool {
        self.operations.notice = None;
        self.is_active_operation(id)
            && self.recording_unguarded(|this| {
                this.document
                    .set_operation_margin(id, Length::from_mm(margin_mm))
                    .unwrap_or(false)
            })
    }

    /// Sets the Show or Output switch of operation `id` (criteria 15, 16). Only Output changes
    /// what runs. `false` when it already reads so.
    pub fn set_operation_switch(&mut self, id: NodeId, which: OperationSwitch, on: bool) -> bool {
        self.operations.notice = None;
        self.is_active_operation(id)
            && self.recording_unguarded(|this| {
                this.document
                    .set_operation_switch(id, which, on)
                    .unwrap_or(false)
            })
    }

    /// Switches Cuts last (criterion 6). On, the operations are put in run order: the marking
    /// ones, then the cutting ones, each group keeping its order. Off, nothing moves. One step
    /// "Cuts last on" or "Cuts last off".
    pub fn set_cuts_last(&mut self, on: bool) -> bool {
        self.operations.notice = None;
        let Some(active) = self.active_operations() else {
            return false;
        };
        let order: Vec<NodeId> = run_sequence(&active.kinds(), on)
            .into_iter()
            .map(|at| active.operations[at].id)
            .collect();
        self.recording_unguarded(|this| {
            this.document
                .set_cuts_last(active.manufacturing, on, &order)
                .unwrap_or(false)
        })
    }

    /// Moves operation `id` to `to` in the run order shown (criteria 5, 6). With Cuts last on a
    /// move that leaves its section is refused (the line is not crossed). `false` when refused or
    /// when it already sits there.
    pub fn move_operation(&mut self, id: NodeId, to: usize) -> bool {
        self.operations.notice = None;
        let Some(active) = self.active_operations() else {
            return false;
        };
        let kinds = active.kinds();
        let shown = run_sequence(&kinds, active.cuts_last);
        let Some(from) = shown.iter().position(|&at| active.operations[at].id == id) else {
            return false;
        };
        let shown_kinds: Vec<_> = shown.iter().map(|&at| kinds[at]).collect();
        if !may_move(&shown_kinds, from, to, active.cuts_last) {
            return false;
        }
        self.recording_unguarded(|this| this.document.move_operation(id, to).unwrap_or(false))
    }
}
