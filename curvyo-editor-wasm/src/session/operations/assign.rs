//! Assigning and removing the selection's items, by tick and by number (`specs/0101` criteria 8
//! to 10; `specs/0111` criteria 9, 10 and 14; ADR 0056 §1, §2). A group is one target whose
//! assignment reaches every leaf (the group rule of `curvyo-laser-core`); a clone is assigned as a
//! whole and its members never are. Guides and dimensions are never targets.

use std::collections::HashSet;

use curvyo_document_core::{NodeId, ObjectSnapshot};
use curvyo_laser_core::operations::{
    GroupTick, OperationList, group_assign_keys, group_tick, group_unassign_keys,
};

use super::super::Session;
use super::index::IndexedLeaf;

/// The selection as operation targets.
#[derive(Debug, Default)]
pub(super) struct Selection {
    /// The assignable items: own items, groups, clones and placed pages, each by its own node.
    pub(super) targets: Vec<NodeId>,
    /// The targets that are groups (criterion 9).
    pub(super) groups: HashSet<NodeId>,
    /// The selected guides (named in the notice).
    pub(super) guides: usize,
    /// Every selected item that is not a target (guides and dimensions).
    pub(super) skipped: usize,
    /// The selected items of a test grid: they run with their grid row only (`specs/0028`
    /// criterion 6), so they are neither targets nor counted as skipped.
    pub(super) in_grid: usize,
}

/// What a group target reaches, read from the leaf index (no drawing read, `adrs.md` "Goldens and
/// budgets"): its inner nodes at any depth and the chains of its leaves.
#[derive(Debug, Default)]
pub(super) struct GroupParts {
    /// The group's nodes below it that a leaf chain names, inner groups and leaves alike.
    pub(super) inner: Vec<NodeId>,
    /// The chains of the group's leaves, as the nearest rule reads them.
    pub(super) chains: Vec<Vec<Vec<NodeId>>>,
}

/// The parts of `group` among the output page's `leaves`. A legacy path key (more than one id)
/// is no node of the group (ADR 0056 §2); inner rows of placed pages are not reached.
pub(super) fn group_parts(leaves: &[IndexedLeaf], group: NodeId) -> GroupParts {
    let mut parts = GroupParts::default();
    let mut seen = HashSet::new();
    for leaf in leaves {
        let Some(at) = leaf.chain.iter().position(|key| key.as_slice() == [group]) else {
            continue;
        };
        for key in &leaf.chain[..at] {
            if let [node] = key.as_slice()
                && seen.insert(*node)
            {
                parts.inner.push(*node);
            }
        }
        parts.chains.push(leaf.chain.clone());
    }
    parts
}

/// How one target stands in the operation at position `op`: a group by its leaves (criterion
/// 10), any other target by its own key.
pub(super) fn target_tick(
    selection: &Selection,
    target: NodeId,
    list: &OperationList,
    op: usize,
    leaves: &[IndexedLeaf],
) -> GroupTick {
    if selection.groups.contains(&target) {
        group_tick(&group_parts(leaves, target).chains, list, op)
    } else if list.operations_of(&[target]).contains(&op) {
        GroupTick::Checked
    } else {
        GroupTick::Unchecked
    }
}

/// The keys to write into (`add`) or out of the operation at position `op` for every target:
/// a group's by the group rule, any other target its own. Each key once.
pub(super) fn target_keys(
    selection: &Selection,
    list: &OperationList,
    op: usize,
    leaves: &[IndexedLeaf],
    add: bool,
) -> Vec<Vec<NodeId>> {
    let mut seen = HashSet::new();
    let mut keys = Vec::new();
    for &target in &selection.targets {
        let nodes = if selection.groups.contains(&target) {
            let parts = group_parts(leaves, target);
            if add {
                group_assign_keys(target, &parts.inner, list, op)
            } else {
                group_unassign_keys(target, &parts.inner, list, op)
            }
        } else {
            vec![target]
        };
        keys.extend(
            nodes
                .into_iter()
                .filter(|node| seen.insert(*node))
                .map(|node| vec![node]),
        );
    }
    keys
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
    /// The selection's assignable items, guides and dimensions counted apart. A selected group is
    /// one target, the group node (criterion 9): its leaves are not.
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
                if unit.is_group {
                    selection.groups.insert(id);
                }
                selection.targets.push(id);
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

    /// The refusal of a selection with no target (only guides, dimensions or grid items): the
    /// notice is set and `true` returned, nothing is written.
    pub(super) fn refuse_no_targets(&mut self, selection: &Selection, adding: bool) -> bool {
        if !selection.targets.is_empty() {
            return false;
        }
        // Only guides and grid items name themselves in the notice; dimensions are skipped
        // silently.
        if selection.guides > 0 {
            self.operations.notice = Some(super::NEVER_RUN_NOTICE.to_owned());
        } else if adding && selection.in_grid > 0 {
            self.operations.notice = Some(super::GRID_ITEMS_NOTICE.to_owned());
        }
        true
    }

    /// Ticks (`member` true) or unticks the selection's items in operation `id` (criterion 8: a
    /// tick row, or a drag of the selection onto the row; `specs/0111` criterion 10 for a group,
    /// a mixed tick adding the rest). One step, `Assign to <name>` or `Remove from <name>`.
    pub fn set_selection_in_operation(&mut self, id: NodeId, member: bool) -> bool {
        self.operations.notice = None;
        let Some(active) = self.active_operations() else {
            return false;
        };
        let Some(at) = active.operations.iter().position(|op| op.id == id) else {
            return false;
        };
        let selection = self.selection_targets();
        self.write_targets(&active, at, &selection, member)
    }

    /// Number key `number` (1 to 9): toggles the selection in operation `number` of the list
    /// (criterion 8). When every target is in it they are removed; otherwise (none or some) all
    /// are added. The digits count the rows as shown and skip test grid rows (`specs/0028` UX
    /// notes §5).
    pub fn toggle_selection_by_number(&mut self, number: usize) -> bool {
        self.operations.notice = None;
        let Some(active) = self.active_operations() else {
            return false;
        };
        let list = OperationList::new(&active.operations, active.cuts_last);
        let Some(at) = number.checked_sub(1).and_then(|n| {
            list.sequence()
                .iter()
                .copied()
                .filter(|&at| !active.operations[at].grid)
                .nth(n)
        }) else {
            return false;
        };
        let selection = self.selection_targets();
        let leaves = self.active_leaves();
        let all_in = !selection.targets.is_empty()
            && selection.targets.iter().all(|&target| {
                target_tick(&selection, target, &list, at, &leaves) == GroupTick::Checked
            });
        self.write_targets(&active, at, &selection, !all_in)
    }

    /// The leaves of the active laser page's output page, empty on another page.
    pub(super) fn active_leaves(&self) -> std::sync::Arc<[IndexedLeaf]> {
        self.active_manufacturing().map_or_else(
            || std::sync::Arc::from(Vec::new()),
            |manufacturing| self.output_leaves(&manufacturing),
        )
    }

    /// Writes the selection's keys into or out of the operation at position `at`, one step; a
    /// selection of only guides, dimensions or test grid items writes nothing and sets the
    /// refusal notice.
    fn write_targets(
        &mut self,
        active: &super::ActiveOperations,
        at: usize,
        selection: &Selection,
        member: bool,
    ) -> bool {
        if self.refuse_no_targets(selection, member) {
            return false;
        }
        self.note_skipped(selection, member);
        let list = OperationList::new(&active.operations, active.cuts_last);
        let keys = target_keys(selection, &list, at, &self.active_leaves(), member);
        let op = &active.operations[at];
        let verb = if member { "Assign to" } else { "Remove from" };
        let id = op.id;
        self.recording_named(format!("{verb} {}", op.name), |this| {
            let done = this
                .document
                .set_operation_members(id, &keys, member)
                .unwrap_or(false);
            // A membership edit of a Lasered grid's row gives the grid a new id (`0112` 11).
            if done {
                this.renew_lasered_grids_of(&[id]);
            }
            done
        })
    }
}
