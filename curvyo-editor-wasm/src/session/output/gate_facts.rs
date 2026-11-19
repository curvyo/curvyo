//! The gate's facts beside the job input (`specs/0029` criterion 5; `specs/0123` criterion 48):
//! the items Not assigned, running while hidden or in an Output off operation, and the vector
//! leaves of an Image operation that are no picture.

use curvyo_document_core::{NodeId, OperationSnapshot};
use curvyo_laser_core::job::GateFacts;
use curvyo_laser_core::operations::{KindRead, LeafStatus, OperationKind, OperationList, kind_run};

use super::super::operations::index::{IndexedLeaf, LeafSource};
use super::super::operations::items::item_lists_of;
use super::input::select_id;

/// What the gate knows beside the job (criterion 5): items Not assigned, items that run while
/// hidden, operations with Output off that hold items and their items; the item lists are the
/// `0101` summary's (`item_lists_of`), so a check's button selects what the count counts.
pub(super) fn gate_facts(
    operations: &[OperationSnapshot],
    list: &OperationList,
    leaves: &[IndexedLeaf],
    statuses: &[LeafStatus],
) -> GateFacts {
    let items = item_lists_of(list, leaves, statuses);
    GateFacts {
        not_assigned: items.not_assigned,
        hidden_running: items.runs_while_hidden,
        output_off: operations
            .iter()
            .filter(|op| !op.output && !op.members.is_empty())
            .filter(|op| KindRead::of(op.kind.as_deref()).known().is_some())
            .map(|op| op.id)
            .collect(),
        output_off_items: items.output_off,
        not_pictures: not_pictures(operations, list, leaves, statuses),
    }
}

/// Per Image operation with Output on, in run order, the vector leaves assigned to it that are
/// no picture (`specs/0123` criterion 48), each once; a placed page's frame is its container,
/// no vector shape, and never counts.
fn not_pictures(
    operations: &[OperationSnapshot],
    list: &OperationList,
    leaves: &[IndexedLeaf],
    statuses: &[LeafStatus],
) -> Vec<(NodeId, Vec<NodeId>)> {
    let mut out = Vec::new();
    for &at in list.sequence() {
        let op = &operations[at];
        if !op.output || KindRead::of(op.kind.as_deref()).known() != Some(OperationKind::Image) {
            continue;
        }
        let mut items: Vec<NodeId> = Vec::new();
        for (leaf, status) in leaves.iter().zip(statuses) {
            let LeafStatus::Assigned { assignment, .. } = status else {
                continue;
            };
            if assignment.operations.contains(&at)
                && !matches!(leaf.source, LeafSource::PlacedFrame(_))
                && kind_run(OperationKind::Image, leaf.shape).is_empty()
                && let Some(id) = select_id(leaf)
                && !items.contains(&id)
            {
                items.push(id);
            }
        }
        if !items.is_empty() {
            out.push((op.id, items));
        }
    }
    out
}
