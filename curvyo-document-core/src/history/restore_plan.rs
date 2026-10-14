//! What an undo or redo decides to do for each object a step touched, from the versions before and
//! after the step and the live document. Reads only; `restore.rs` applies the plan.

use std::collections::{BTreeMap, HashMap, HashSet};

use loro::{FractionalIndex, LoroDoc, TreeID, TreeParentId};

use super::checkout::NodeAt;
use super::node_value::{Node, read_map};
use super::report::{Direction, Role, SkipReason};
use super::step_header::{
    KEY_OBJECT_REDONE, KEY_REDONE, KEY_UNDONE, LABEL_OBJECT_REDO, LABEL_REDO, LABEL_UNDO,
    StepHeader,
};
use super::step_id::StepRef;
use super::step_log::{LogStep, find_step};
use super::touched::Touched;
use crate::document::OBJECTS_TREE;

/// The touched objects at one version.
pub(super) struct Snapshot {
    pub(super) nodes: BTreeMap<TreeID, NodeAt>,
    pub(super) root: BTreeMap<String, Node>,
    /// The children, in z-order at that version, of every parent a created or deleted object
    /// had (`None` is the document root): where a returning object finds its neighbours.
    pub(super) siblings: BTreeMap<Option<TreeID>, Vec<TreeID>>,
}

/// What the engine decided to do for one touched object.
pub(super) enum Action {
    Revive(TreeID),
    Delete(TreeID),
    Merge(TreeID),
    /// Put a live object under a parent (`None` is the document root), in the order the version
    /// being restored had among that parent's children.
    Move(TreeID, Option<TreeID>),
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

/// The plan of applying a version (`specs/0042-history-branches` criterion 10, ADR 0014 §8): with
/// `before` the version and `after` now, every touched object goes to what it was at the version.
/// One alive now and absent then is deleted; one absent now and alive then comes back with its id;
/// one alive at both gets the version's fields and, when its parent or place changed, its old place.
/// No three-way check: `after` is the live state, so nobody changed anything since.
pub(super) fn plan_return(
    doc: &LoroDoc,
    touched: &Touched,
    before: &Snapshot,
    after: &Snapshot,
) -> Plan {
    let mut plan = Plan::default();
    let mut seen = HashSet::new();
    let moved: HashSet<TreeID> = touched.moved.iter().copied().collect();
    for id in touched.nodes() {
        if !seen.insert(id) {
            continue;
        }
        let then = before.nodes.get(&id).filter(|node| !node.meta.is_empty());
        match (alive(doc, id), then) {
            (true, None) => plan.push(Role::Created, Action::Delete(id)),
            (false, Some(then)) => {
                plan.push(Role::Deleted, Action::Revive(id));
                plan.revive_from_before.push(id);
                // A deleted object keeps the fields it had when it went; they go back too.
                if after.nodes.get(&id).is_none_or(|now| now.meta != then.meta) {
                    plan.push(Role::Changed, Action::Merge(id));
                }
            }
            (true, Some(then)) => {
                let now = after.nodes.get(&id);
                if now.is_none_or(|now| now.meta != then.meta) {
                    plan.push(Role::Changed, Action::Merge(id));
                }
                if moved.contains(&id) || now.is_none_or(|now| now.parent != then.parent) {
                    plan.push(Role::Moved, Action::Move(id, then.parent));
                }
            }
            (false, None) => {}
        }
    }
    plan
}

/// A meta map without the floor register of an object wipe.
fn without_floor(mut meta: BTreeMap<String, Node>) -> BTreeMap<String, Node> {
    meta.remove(super::lineage::KEY_HISTORY_FLOOR);
    meta
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
                self.plan_moved(&mut plan, id);
            }
        }
        plan
    }

    /// Whether `parent` (`None` is the document root) holds objects when the plan has run: it
    /// lives now, or it comes back in this restore.
    fn available(&self, parent: Option<TreeID>) -> bool {
        let Some(parent) = parent else {
            return true;
        };
        alive(self.doc, parent)
            || match self.direction {
                Direction::Undo => {
                    self.touched.deleted.contains(&parent) && self.deleted_by_this_step(parent)
                }
                Direction::Redo => {
                    self.touched.created.contains(&parent) && self.last_move_by_me(parent)
                }
            }
    }

    /// An object the step moved to another parent or place (grouping): put back where it was
    /// before the step (undo) or where the step put it (redo), if it is still under the parent the
    /// step left it under (undo) or had before (redo).
    fn plan_moved(&self, plan: &mut Plan, id: TreeID) {
        let (from, to) = match self.direction {
            Direction::Undo => (self.after, self.before),
            Direction::Redo => (self.before, self.after),
        };
        let (Some(from), Some(to)) = (from.nodes.get(&id), to.nodes.get(&id)) else {
            plan.skip(Role::Moved, id, SkipReason::ChangedLater);
            return;
        };
        if !alive(self.doc, id) {
            plan.skip(Role::Moved, id, SkipReason::DeletedLater);
            return;
        }
        let tree = self.doc.get_tree(OBJECTS_TREE);
        let now_parent = match tree.parent(id) {
            Some(TreeParentId::Node(parent)) => Some(parent),
            _ => None,
        };
        if to.index.is_none() || now_parent != from.parent {
            plan.skip(Role::Moved, id, SkipReason::ChangedLater);
        } else if !self.available(to.parent) {
            plan.skip(Role::Moved, id, SkipReason::DeletedLater);
        } else {
            plan.push(Role::Moved, Action::Move(id, to.parent));
        }
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
                    || (header.label == LABEL_UNDO
                        && header
                            .extra_value(KEY_UNDONE)
                            .and_then(StepRef::decode)
                            .is_some_and(|undone| self.later_revival_of(undone, id)))
            })
    }

    /// Whether `step` came after this step and brought `id` back (an apply of a version, `0042`):
    /// an undo of it deletes `id` again, which puts this step's delete back in effect.
    fn later_revival_of(&self, step: StepRef, id: TreeID) -> bool {
        let Some(later) = find_step(self.doc, step) else {
            return false;
        };
        let after_this = self
            .doc
            .frontiers_to_vv(&later.before)
            .and_then(|vv| vv.get(&self.log.step.peer()).copied())
            .is_some_and(|end| end >= self.log.end);
        after_this
            && self
                .doc
                .export_json_in_id_span(later.span())
                .iter()
                .flat_map(|change| &change.ops)
                .any(|op| {
                    matches!(
                        &op.content,
                        loro::JsonOpContent::Tree(loro::JsonTreeOp::Move { target, .. }) if *target == id
                    )
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
                // The floor an object wipe wrote is not a change of the object's look.
                let created = self
                    .after
                    .nodes
                    .get(&id)
                    .map(|n| without_floor(n.meta.clone()));
                if self.live_meta(id).map(without_floor) == created {
                    plan.push(Role::Created, Action::Delete(id));
                } else {
                    plan.skip(Role::Created, id, SkipReason::ChangedLater);
                }
            }
            Direction::Redo if now_alive => plan.skip(Role::Created, id, SkipReason::ChangedLater),
            Direction::Redo
                if self.last_move_by_me(id)
                    && !self.available(self.after.nodes.get(&id).and_then(|n| n.parent)) =>
            {
                plan.skip(Role::Created, id, SkipReason::DeletedLater);
            }
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
            Direction::Undo
                if self.deleted_by_this_step(id)
                    && !self.available(self.before.nodes.get(&id).and_then(|n| n.parent)) =>
            {
                plan.skip(Role::Deleted, id, SkipReason::DeletedLater);
            }
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

/// Where a node comes back to.
pub(super) struct Placement {
    /// The group it goes back into; `None` is the document root.
    pub(super) parent: Option<TreeID>,
    /// Its index among that parent's children.
    pub(super) index: FractionalIndex,
}

/// The places the nodes `ids` take when they come back, for the version `snapshot` whose z-order
/// places them. For each parent, one pass over its children at that version: a node keeps its old
/// index while that lies between its nearest living neighbours, else it gets one between them. A
/// node placed earlier counts as a living neighbour of the next, so two adjacent nodes never
/// share an index.
pub(super) fn revival_indices(
    doc: &LoroDoc,
    ids: &[TreeID],
    snapshot: &Snapshot,
) -> HashMap<TreeID, Placement> {
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
    let parent_of = |id: &TreeID| snapshot.nodes.get(id).and_then(|n| n.parent);
    let mut parents: Vec<Option<TreeID>> = Vec::new();
    for id in ids {
        let parent = parent_of(id);
        if !parents.contains(&parent) {
            parents.push(parent);
        }
    }
    let mut placed = HashMap::new();
    for parent in parents {
        let siblings: &[TreeID] = snapshot.siblings.get(&parent).map_or(&[], Vec::as_slice);
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
                placed.insert(*sibling, Placement { parent, index });
            } else if let Some(index) = index_now(*sibling) {
                lower = Some(index);
            }
        }
        // A node the version does not list (it cannot happen for a step's own nodes) goes last.
        for id in ids.iter().filter(|id| parent_of(id) == parent) {
            if !placed.contains_key(id) {
                let index = FractionalIndex::new(lower.as_ref(), None).unwrap_or_default();
                lower = Some(index.clone());
                placed.insert(*id, Placement { parent, index });
            }
        }
    }
    placed
}
