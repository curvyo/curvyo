//! The items behind the summary counts and the clone member rows (`specs/0101-laser-operations`
//! criteria 11, 17): which items each count holds, so a count button can select them, and the
//! members of each clone, so the tab can list them. The counts follow `leaf_status` as
//! `curvyo_laser_core::operations::summarize` does; the view checks the two agree.

use std::collections::HashSet;

use curvyo_document_core::{NodeId, OperationSnapshot};
use curvyo_laser_core::operations::{Hidden, Leaf, LeafStatus, OperationList, leaf_status};
use serde::Serialize;

use super::super::Session;
use super::OperationsViewMode;
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

/// One clone member row of the tab (criterion 11, Frage 167 B): read-only, assignable by path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CloneMemberView {
    /// The clone the member is in.
    pub clone: NodeId,
    /// The clone's name in the tab ("Clone 1", the same for its members).
    pub clone_name: String,
    /// The member's own node (the original's member).
    pub member: NodeId,
    /// The member's name in its clone ("Member 1").
    pub name: String,
    /// The operations that hold this member through the clone, in list order.
    pub operations: Vec<NodeId>,
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

/// The clone member rows: every leaf that is a member of a clone, in the order of the leaves, with
/// the clones numbered in the order they first appear and the operations that hold its path.
pub(super) fn clone_member_rows(
    operations: &[OperationSnapshot],
    leaves: &[IndexedLeaf],
) -> Vec<CloneMemberView> {
    let mut clones: Vec<NodeId> = Vec::new();
    let mut counts: Vec<usize> = Vec::new();
    let mut rows = Vec::new();
    for leaf in leaves {
        let Some(path) = leaf.chain.first().filter(|path| path.len() >= 2) else {
            continue;
        };
        let clone = path[0];
        let index = clones.iter().position(|c| *c == clone).unwrap_or_else(|| {
            clones.push(clone);
            counts.push(0);
            clones.len() - 1
        });
        counts[index] += 1;
        rows.push(CloneMemberView {
            clone,
            clone_name: format!("Clone {}", index + 1),
            member: path[path.len() - 1],
            name: format!("Member {}", counts[index]),
            operations: operations
                .iter()
                .filter(|op| op.members.contains(path))
                .map(|op| op.id)
                .collect(),
        });
    }
    rows
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

    /// Sets how the output page is drawn (criterion 18). Session state: no step, not saved.
    pub fn set_operations_view_mode(&mut self, mode: OperationsViewMode) -> bool {
        if self.operations.view_mode == mode {
            return false;
        }
        self.operations.view_mode = mode;
        true
    }
}
