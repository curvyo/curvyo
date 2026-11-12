//! The items behind the summary counts (`specs/0101-laser-operations` criterion 17): which items
//! each count holds, so a count button can select them. The counts follow `leaf_status` as
//! `curvyo_laser_core::operations::summarize` does; the view checks the two agree. The tab lists
//! no member of a clone or a group (`specs/0111` criterion 14).

use std::collections::HashSet;

use curvyo_document_core::NodeId;
use curvyo_laser_core::operations::{Hidden, Leaf, LeafStatus, OperationList, leaf_status};
use serde::Serialize;

use super::super::Session;
use super::index::IndexedLeaf;

/// The items of each summary count, in the order of the output page's leaves, each once.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ItemLists {
    /// `<M> items`: the running items.
    pub running: Vec<NodeId>,
    /// `<K> items not assigned.`
    pub not_assigned: Vec<NodeId>,
    /// `<J> items run while hidden.`
    pub runs_while_hidden: Vec<NodeId>,
    /// `<H> items sit in operations with Output off.`
    pub output_off: Vec<NodeId>,
}

/// The id a count button selects for a leaf: the first node of its chain. A clone member selects
/// its clone, a placed page's inner row its source object.
fn select_id(leaf: &IndexedLeaf) -> Option<NodeId> {
    leaf.chain.first().and_then(|path| path.first().copied())
}

/// A list of ids, each once.
#[derive(Default)]
struct Bucket {
    ids: Vec<NodeId>,
    seen: HashSet<NodeId>,
}

impl Bucket {
    fn push(&mut self, id: Option<NodeId>) {
        if let Some(id) = id
            && self.seen.insert(id)
        {
            self.ids.push(id);
        }
    }
}

/// The item lists of the output page's leaves under `list`.
pub(super) fn item_lists(list: &OperationList, leaves: &[IndexedLeaf]) -> ItemLists {
    let (mut running, mut not_assigned, mut hidden, mut off) = (
        Bucket::default(),
        Bucket::default(),
        Bucket::default(),
        Bucket::default(),
    );
    let rules = list.rules();
    for leaf in leaves {
        let id = select_id(leaf);
        let status = leaf_status(
            list,
            &Leaf {
                chain: &leaf.chain,
                hidden: leaf.hidden,
                shape: leaf.shape,
            },
        );
        match status {
            LeafStatus::Absent => {}
            LeafStatus::NotAssigned => not_assigned.push(id),
            LeafStatus::Assigned {
                assignment,
                running: runs,
            } => {
                if !runs.is_empty() {
                    running.push(id);
                    if leaf.hidden != Hidden::No {
                        hidden.push(id);
                    }
                }
                if assignment
                    .operations
                    .iter()
                    .any(|&at| !rules[at].output && rules[at].kind.known().is_some())
                {
                    off.push(id);
                }
            }
        }
    }
    ItemLists {
        running: running.ids,
        not_assigned: not_assigned.ids,
        runs_while_hidden: hidden.ids,
        output_off: off.ids,
    }
}

impl Session {
    /// Selects the items of the summary count `which` (`running`, `not_assigned`,
    /// `runs_while_hidden`, `output_off`), the count button of criterion 17. `false` for an
    /// unknown count or on another page; the selection is not a step.
    pub fn select_operation_items(&mut self, which: &str) -> bool {
        let Some(view) = self.operations_view() else {
            return false;
        };
        let ids = match which {
            "running" => view.items.running,
            "not_assigned" => view.items.not_assigned,
            "runs_while_hidden" => view.items.runs_while_hidden,
            "output_off" => view.items.output_off,
            _ => return false,
        };
        self.selection.set(&ids);
        true
    }
}
