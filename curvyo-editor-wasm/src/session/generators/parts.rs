//! Detach part and Attach part of the selected rounded box and the Parts rows of its block
//! (`specs/0147-rounded-box` criterion 15, ADR 0082 §10, UX notes §9): a part gets a copy of the
//! box's operations of its own, or goes back to the box's.

use curvyo_document_core::NodeId;
use curvyo_laser_core::operations::{ChainScope, OperationList};
use curvyo_ui_core::instance;
use serde_json::{Value, json};

use super::super::Session;

/// The refusal when the box runs with no operation, so there is nothing to copy.
const NO_OPERATION_NOTICE: &str = "Assign the box to an operation first. Nothing was changed.";

impl Session {
    /// The one selected generated object, `None` for none or several.
    fn the_selected_box(&self) -> Option<NodeId> {
        match self.selected_generated()[..] {
            [id] if self.selection.ids().len() == 1 => Some(id),
            _ => None,
        }
    }

    /// The operations (positions in the stored list) the box `id` itself runs with: the nearest
    /// rule from its own key up, which is what a detached part is copied from.
    fn box_operations(&self, id: NodeId) -> Vec<curvyo_document_core::OperationSnapshot> {
        let Some(active) = self.active_operations() else {
            return Vec::new();
        };
        let list = OperationList::new(&active.operations, active.cuts_last);
        let chain = self.document.key_chain(&[id]);
        let Some(assignment) = list.nearest(&chain, ChainScope::Whole) else {
            return Vec::new();
        };
        let mut positions = assignment.operations;
        positions.sort_unstable();
        positions
            .into_iter()
            .filter_map(|at| active.operations.get(at))
            .filter(|op| !op.grid)
            .cloned()
            .collect()
    }

    /// Detach part: gives the part `key` of the selected rounded box a copy of the box's
    /// operations of its own, named "<operation> · <part>" and placed right after it. One step
    /// "Detach part". `false` when nothing is selected, the part does not exist, it is detached
    /// already or the box has no operation (a notice says so).
    pub fn detach_selected_part(&mut self, key: &str) -> bool {
        let Some(id) = self.the_selected_box() else {
            return false;
        };
        let Some(snapshot) = self.document.generated(id) else {
            return false;
        };
        let Some(name) = instance::part_name(key) else {
            return false;
        };
        let keys = instance::part_keys(&self.manufacturing.extensions, &snapshot);
        let sources: Vec<NodeId> = self.box_operations(id).iter().map(|op| op.id).collect();
        if sources.is_empty() {
            self.transform_notice = Some(NO_OPERATION_NOTICE);
            return false;
        }
        self.recording_named("Detach part".to_owned(), |this| {
            this.document
                .detach_part(id, key, name, &sources, &keys)
                .is_ok()
        })
    }

    /// Attach part: the part `key` of the selected rounded box runs with the box's operations
    /// again. One step "Attach part". `false` when it was not detached.
    pub fn attach_selected_part(&mut self, key: &str) -> bool {
        let Some(id) = self.the_selected_box() else {
            return false;
        };
        self.recording_named("Attach part".to_owned(), |this| {
            this.document.attach_part(id, key).unwrap_or(false)
        })
    }

    /// The Parts rows of the block as JSON values, each with whether it is detached and the names
    /// of the operations it runs with now (the part's own when detached, else the box's).
    pub(super) fn parts_rows(&self, id: NodeId, parts: &[instance::PartView]) -> Vec<Value> {
        let detached = self.document.detached_parts(id);
        let own = self.box_operations(id);
        let names = |ops: &[curvyo_document_core::OperationSnapshot]| {
            ops.iter()
                .map(|op| op.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let all = self
            .active_operations()
            .map(|active| active.operations)
            .unwrap_or_default();
        parts
            .iter()
            .map(|part| {
                let is_detached = detached.contains(&part.key);
                let operation = if is_detached {
                    id.with_part(&part.key)
                        .map(|target| {
                            all.iter()
                                .filter(|op| op.members.iter().any(|m| m.as_slice() == [target]))
                                .cloned()
                                .collect::<Vec<_>>()
                        })
                        .map(|ops| names(&ops))
                        .unwrap_or_default()
                } else {
                    names(&own)
                };
                json!({
                    "key": part.key,
                    "name": part.name,
                    "detached": is_detached,
                    "operation": operation,
                    "can_detach": is_detached || !own.is_empty(),
                })
            })
            .collect()
    }
}
