//! A press on one of the five kind buttons of the Select bar (`specs/0111-laser-operation-tools`
//! criteria 1 to 7 and 9; ADR 0056 §3): the selection goes into the last operation of that kind,
//! or into a new "`<Kind> <N>`" when there is none or Shift is held, in one step.

use curvyo_document_core::{NewOperation, NodeId};
use curvyo_laser_core::operations::{
    KindRead, OperationKind, OperationList, insert_index, last_of_kind, next_operation_name,
};
use curvyo_ui_core::Scope;
use serde::Serialize;

use super::super::Session;
use super::ActiveOperations;
use super::assign::target_keys;
use super::view_json::{KindView, kind_view};

/// The five kinds in bar order (criterion 1).
const BAR_ORDER: [OperationKind; 5] = [
    OperationKind::Cut,
    OperationKind::Score,
    OperationKind::Engrave,
    OperationKind::EngraveOutline,
    OperationKind::CutOut,
];

/// One kind button of the Select bar (criterion 2; UX notes §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KindButtonView {
    /// The kind.
    pub kind: KindView,
    /// What a press does, the accessible description: "Assigns to Engrave 1" or "Creates Cut 3".
    pub description: String,
}

impl Session {
    /// A press on the kind button `kind` (`cut`, `score`, `engrave`, `engrave_outline`,
    /// `cut_out`), with `shift` held or not (criteria 3 to 5): the selection is assigned to the
    /// last operation of that kind, or to a new one at the end of its section (Show and Output
    /// on), in one step `Assign to <name>`; a group by the group rule (criterion 9). Guides, dimensions
    /// and test grid items are skipped with the notice of `0101` criterion 10. The operation
    /// written to; `None` when nothing changes (no step), for an unknown kind, another page, or a
    /// selection with no target.
    pub fn press_operation_kind(&mut self, kind: &str, shift: bool) -> Option<NodeId> {
        self.operations.notice = None;
        self.operations.status = None;
        let kind = OperationKind::parse(kind)?;
        let active = self.active_operations()?;
        let selection = self.selection_targets();
        if self.refuse_no_targets(&selection, true) {
            return None;
        }
        let list = OperationList::new(&active.operations, active.cuts_last);
        let leaves = self.active_leaves();
        let reuse = if shift {
            None
        } else {
            last_of_kind(&active.operations, kind)
        };
        let written = if let Some(at) = reuse {
            let keys = target_keys(&selection, &list, at, &leaves, true);
            let (id, name) = (active.operations[at].id, active.operations[at].name.clone());
            let changed = self.recording_named(format!("Assign to {name}"), |this| {
                this.document
                    .set_operation_members(id, &keys, true)
                    .unwrap_or(false)
            });
            if !changed {
                return None;
            }
            self.operations.status = Some(format!("Assigned to {name}."));
            id
        } else {
            // A position no operation has: every key is new to it.
            let keys = target_keys(&selection, &list, active.operations.len(), &leaves, true);
            let name =
                next_operation_name(kind, active.operations.iter().map(|op| op.name.as_str()));
            let index = insert_index(&active.kinds(), kind.section(), active.cuts_last);
            let new = NewOperation {
                kind: kind.as_str(),
                name: &name,
            };
            let added = self.recording_named(format!("Assign to {name}"), |this| {
                this.document
                    .add_operation(active.manufacturing, &new, index, &keys)
                    .ok()
            });
            let Some(id) = added else {
                // Never silently nothing (`specs/0028` UX review).
                self.operations.notice = Some(super::ADD_REFUSED_NOTICE.to_owned());
                return None;
            };
            self.operations.status = Some(format!("{name} added."));
            id
        };
        self.note_skipped(&selection, true);
        Some(written)
    }

    /// The kind buttons of the Select bar (criteria 1, 2), in bar order; `None` (absent) with no
    /// selection, a selection that holds an out-of-context object or a test grid's item. `active` is the active laser
    /// page's operations.
    pub(super) fn select_bar_kinds(
        &self,
        active: &ActiveOperations,
    ) -> Option<Vec<KindButtonView>> {
        if self.selection.ids().is_empty() || self.edit_scope() != Scope::Edit {
            return None;
        }
        // A grid selection has the Test grid bar, not the kind buttons (`specs/0112` UX G4).
        if self.selection.ids().iter().any(|&id| self.in_test_grid(id)) {
            return None;
        }
        let names = || active.operations.iter().map(|op| op.name.as_str());
        Some(
            BAR_ORDER
                .iter()
                .map(|&kind| KindButtonView {
                    kind: kind_view(KindRead::Known(kind)),
                    description: match last_of_kind(&active.operations, kind) {
                        Some(at) => format!("Assigns to {}", active.operations[at].name),
                        None => format!("Creates {}", next_operation_name(kind, names())),
                    },
                })
                .collect(),
        )
    }
}
