//! Where the tabs of each leaf of a Cut or Cut-out block are stored (`specs/0139` criteria 4, 5;
//! adrs L3, L7): the item's `tabs.<key>` record, read into the job input with its outline
//! references.

use curvyo_document_core::{NodeId, OperationSnapshot};
use curvyo_laser_core::job::{BlockInput, LeafGeometry};
use curvyo_laser_core::operations::{LeafStatus, OperationKind};
use curvyo_laser_core::settings::cuts_through;
use curvyo_laser_core::tabs::{LeafTabs, below_target};

use super::super::Session;
use super::super::operations::index::IndexedLeaf;

/// The rows of the output page's leaves one block was built from.
pub(super) struct CutRows<'a> {
    pub(super) leaves: &'a [IndexedLeaf],
    pub(super) statuses: &'a [LeafStatus],
    /// The rows that gave the block's leaves, in its order.
    pub(super) used: &'a [usize],
}

/// Where the tabs of one leaf (or Cut-out item) of a Cut or Cut-out block are stored: the
/// operation's `tabs.<key>` record and the outline references of the leaf (`specs/0139` L3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) struct CutKey {
    /// The operation.
    pub op: NodeId,
    /// The leaf's index in its block's `leaves`, or the item's in `cut_out`.
    pub at: usize,
    /// The leaf's (or item's) node in the block.
    pub node: NodeId,
    /// The membership key text of the assigned target.
    pub key: String,
    /// The node texts from below the target down to the leaf (empty for a Cut-out item).
    pub below: Vec<String>,
}

impl Session {
    /// Gives every leaf (or Cut-out item) of a Cut or Cut-out `block` the tabs of its item's
    /// record in `op` (`specs/0139` criteria 4, 5; adrs L3, L7) and notes where they are stored.
    pub(super) fn cut_tabs_into(
        &self,
        op: &OperationSnapshot,
        block: &mut BlockInput,
        rows: &CutRows<'_>,
        keys: &mut Vec<CutKey>,
    ) {
        let (leaves, statuses) = (rows.leaves, rows.statuses);
        let Some(kind) = block.kind.known().filter(|&k| cuts_through(k)) else {
            return;
        };
        if op.grid || op.grid_part.is_some() {
            return;
        }
        let records = self.document.operation_tabs(op.id).unwrap_or_default();
        let target_at = |at: usize| match &statuses[at] {
            LeafStatus::Assigned { assignment, .. } => Some(assignment.target.clone()),
            _ => None,
        };
        if kind == OperationKind::CutOut {
            for (at, item) in block.cut_out.iter_mut().enumerate() {
                let Some(target) = rows
                    .used
                    .iter()
                    .filter_map(|&row| target_at(row))
                    .find(|target| target.first() == Some(&item.item))
                else {
                    continue;
                };
                let key = curvyo_document_core::member_key_text(&target);
                item.cut = LeafTabs::of_contours(records.get(&key));
                keys.push(CutKey {
                    op: op.id,
                    at,
                    node: item.item,
                    key,
                    below: Vec::new(),
                });
            }
            return;
        }
        for (at, (input, &row)) in block.leaves.iter_mut().zip(rows.used).enumerate() {
            let (leaf, Some(target)) = (&leaves[row], target_at(row)) else {
                continue;
            };
            let key = curvyo_document_core::member_key_text(&target);
            let below: Vec<String> = below_target(&leaf.chain, &target)
                .into_iter()
                .map(|node| curvyo_document_core::member_key_text(&[node]))
                .collect();
            let outlines = match &input.geometry {
                LeafGeometry::Outlines(outlines) | LeafGeometry::Gradient { outlines, .. } => {
                    outlines.len()
                }
                LeafGeometry::Image { .. } => 1,
            };
            input.cut = LeafTabs::of(records.get(&key), &below, outlines);
            keys.push(CutKey {
                op: op.id,
                at,
                node: input.node,
                key,
                below,
            });
        }
    }
}
