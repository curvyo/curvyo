//! What an undo or redo decides to do for each object a step touched, from the versions before and
//! after the step and the live document. Reads only; `restore.rs` applies the plan.

use std::collections::{BTreeMap, HashMap, HashSet};

use loro::{FractionalIndex, LoroDoc, TreeID};

use super::checkout::NodeAt;
use super::node_value::{Node, read_map};
use super::report::{Direction, Role, SkipReason};
use super::step_header::{
    KEY_OBJECT_REDONE, KEY_REDONE, LABEL_OBJECT_REDO, LABEL_REDO, StepHeader,
};
use super::step_id::StepRef;
use super::step_log::LogStep;
use super::touched::Touched;
use crate::document::OBJECTS_TREE;

/// The touched objects at one version.
pub(super) struct Snapshot {
    pub(super) nodes: BTreeMap<TreeID, NodeAt>,
    pub(super) root: BTreeMap<String, Node>,
    /// The objects in z-order at that version.
    pub(super) roots: Vec<TreeID>,
}

/// What the engine decided to do for one touched object.
pub(super) enum Action {
    Revive(TreeID),
    Delete(TreeID),
    Merge(TreeID),
    Skip(TreeID, SkipReason),
}

pub(super) struct Planned {
    pub(super) role: Role,
    pub(super) action: Action,
}

/// The decisions for one restore. Objects that come back are collected by the version whose
/// z-order places them.
#[derive(Default)]
pub(super) struct Plan {
    pub(super) items: Vec<Planned>,
    pub(super) revive_from_before: Vec<TreeID>,
    pub(super) revive_from_after: Vec<TreeID>,
}

impl Plan {
    fn push(&mut self, role: Role, action: Action) {
        self.items.push(Planned { role, action });
    }

    fn skip(&mut self, role: Role, id: TreeID, reason: SkipReason) {
        self.push(role, Action::Skip(id, reason));
    }

    /// Whether the plan writes anything: the header of the undo is only used then.
    pub(super) fn writes(&self) -> bool {
        self.items
            .iter()
            .any(|p| !matches!(p.action, Action::Skip(..)))
    }
}

pub(super) fn alive(doc: &LoroDoc, id: TreeID) -> bool {
    let tree = doc.get_tree(OBJECTS_TREE);
    tree.contains(id) && matches!(tree.is_node_deleted(&id), Ok(false))
}

/// Everything the plan reads.
pub(super) struct PlanInput<'a> {
    pub(super) doc: &'a LoroDoc,
    pub(super) log: &'a LogStep,
    pub(super) direction: Direction,
    pub(super) touched: &'a Touched,
    pub(super) before: &'a Snapshot,
    pub(super) after: &'a Snapshot,
}

impl PlanInput<'_> {
    /// Decides what to do for each touched object.
    pub(super) fn plan(&self) -> Plan {
        let mut plan = Plan::default();
        for &id in &self.touched.created {
            self.plan_created(&mut plan, id);
        }
        for &id in &self.touched.deleted {
            self.plan_deleted(&mut plan, id);
        }
        for &id in &self.touched.changed {
            if alive(self.doc, id) {
                plan.push(Role::Changed, Action::Merge(id));
            } else {
                plan.skip(Role::Changed, id, SkipReason::DeletedLater);
            }
        }
        for &id in &self.touched.moved {
            if self.came_back_in_this_step(id) {
                // A node an object undo brought back is a Move of a deleted node: for the restore
                // it is a creation.
                self.plan_created(&mut plan, id);
            } else {
                // Z-order and parent moves of existing objects are not restored yet: no gesture
                // makes one (`adrs.md`, dated note). A later one fails visibly here.
                plan.skip(Role::Moved, id, SkipReason::ChangedLater);
            }
        }
        plan
    }

    /// Whether the node was deleted before the step and lives after it: the step revived it.
    fn came_back_in_this_step(&self, id: TreeID) -> bool {
        let meta_of = |snapshot: &Snapshot| snapshot.nodes.get(&id).map(|n| n.meta.is_empty());
        meta_of(self.before) == Some(true) && meta_of(self.after) == Some(false)
    }

    fn live_meta(&self, id: TreeID) -> Option<BTreeMap<String, Node>> {
        let tree = self.doc.get_tree(OBJECTS_TREE);
        tree.get_meta(id).map(|m| read_map(&m)).ok()
    }

    /// Whether the node's last tree move is the step's own delete, or the delete of a redo of this
    /// step (Delete, Ctrl+Z, Ctrl+Shift+Z, Ctrl+Z).
    fn deleted_by_this_step(&self, id: TreeID) -> bool {
        let tree = self.doc.get_tree(OBJECTS_TREE);
        let Some(last) = tree.get_last_move_id(&id) else {
            return false;
        };
        if last.peer == self.log.step.peer()
            && (self.log.step.counter()..self.log.end).contains(&last.counter)
        {
            return true;
        }
        self.doc
            .get_change(last)
            .and_then(|change| StepHeader::parse(change.message.as_deref()?))
            .is_some_and(|header| {
                let redone = |key: &str| {
                    header.extra_value(key).and_then(StepRef::decode) == Some(self.log.step)
                };
                (header.label == LABEL_REDO && redone(KEY_REDONE))
                    || (header.label == LABEL_OBJECT_REDO && redone(KEY_OBJECT_REDONE))
            })
    }

    fn last_move_by_me(&self, id: TreeID) -> bool {
        let tree = self.doc.get_tree(OBJECTS_TREE);
        tree.get_last_move_id(&id)
            .is_some_and(|last| last.peer == self.doc.peer_id())
    }

    fn plan_created(&self, plan: &mut Plan, id: TreeID) {
        let now_alive = alive(self.doc, id);
        match self.direction {
            Direction::Undo if !now_alive => plan.skip(Role::Created, id, SkipReason::DeletedLater),
            Direction::Undo => {
                let created = self.after.nodes.get(&id).map(|n| &n.meta);
                if self.live_meta(id).as_ref() == created {
                    plan.push(Role::Created, Action::Delete(id));
                } else {
                    plan.skip(Role::Created, id, SkipReason::ChangedLater);
                }
            }
            Direction::Redo if now_alive => plan.skip(Role::Created, id, SkipReason::ChangedLater),
            Direction::Redo if self.last_move_by_me(id) => {
                plan.push(Role::Created, Action::Revive(id));
                plan.revive_from_after.push(id);
            }
            Direction::Redo => plan.skip(Role::Created, id, SkipReason::DeletedLater),
        }
    }

    fn plan_deleted(&self, plan: &mut Plan, id: TreeID) {
        let now_alive = alive(self.doc, id);
        match self.direction {
            Direction::Undo if now_alive => plan.skip(Role::Deleted, id, SkipReason::ChangedLater),
            Direction::Undo if self.deleted_by_this_step(id) => {
                plan.push(Role::Deleted, Action::Revive(id));
                plan.revive_from_before.push(id);
            }
            Direction::Undo => plan.skip(Role::Deleted, id, SkipReason::DeletedLater),
            Direction::Redo if !now_alive => plan.skip(Role::Deleted, id, SkipReason::DeletedLater),
            Direction::Redo => {
                let deleted = self.before.nodes.get(&id).map(|n| &n.meta);
                if self.live_meta(id).as_ref() == deleted {
                    plan.push(Role::Deleted, Action::Delete(id));
                } else {
                    plan.skip(Role::Deleted, id, SkipReason::ChangedLater);
                }
            }
        }
    }
}

/// The positions the nodes `ids` take when they come back, for the version `snapshot` whose z-order
/// places them. One pass in that z-order: a node keeps its old index while that lies between its
/// nearest living neighbours, else it gets one between them. A node placed earlier counts as a
/// living neighbour of the next, so two adjacent nodes never share an index.
pub(super) fn revival_indices(
    doc: &LoroDoc,
    ids: &[TreeID],
    snapshot: &Snapshot,
) -> HashMap<TreeID, FractionalIndex> {
    let tree = doc.get_tree(OBJECTS_TREE);
    let coming: HashSet<TreeID> = ids.iter().copied().collect();
    let index_now = |id: TreeID| {
        alive(doc, id)
            .then(|| {
                tree.fractional_index(id)
                    .map(FractionalIndex::from_hex_string)
            })
            .flatten()
    };
    let siblings = &snapshot.roots;
    // The index of the nearest later sibling that lives now and is not coming back.
    let mut upper_of: Vec<Option<FractionalIndex>> = vec![None; siblings.len()];
    let mut upper = None;
    for (position, sibling) in siblings.iter().enumerate().rev() {
        upper_of[position].clone_from(&upper);
        if !coming.contains(sibling)
            && let Some(index) = index_now(*sibling)
        {
            upper = Some(index);
        }
    }
    let mut placed = HashMap::new();
    let mut lower: Option<FractionalIndex> = None;
    for (position, sibling) in siblings.iter().enumerate() {
        if coming.contains(sibling) {
            let wanted = snapshot.nodes.get(sibling).and_then(|n| n.index.clone());
            let upper = upper_of[position].as_ref();
            let fits = |old: &FractionalIndex| {
                lower.as_ref().is_none_or(|l| l < old) && upper.is_none_or(|u| old < u)
            };
            let index = match wanted {
                Some(old) if fits(&old) => old,
                _ => FractionalIndex::new(lower.as_ref(), upper).unwrap_or_default(),
            };
            lower = Some(index.clone());
            placed.insert(*sibling, index);
        } else if let Some(index) = index_now(*sibling) {
            lower = Some(index);
        }
    }
    // A node the version does not list (it cannot happen for a step's own nodes) goes last.
    for id in ids {
        placed
            .entry(*id)
            .or_insert_with(|| FractionalIndex::new(lower.as_ref(), None).unwrap_or_default());
    }
    placed
}
