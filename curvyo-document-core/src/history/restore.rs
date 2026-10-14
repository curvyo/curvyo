//! Undo and redo of one step by restore over versions (ADR 0014 §1).
//!
//! For each object the step touched the engine knows the object before the step (`B`), after it
//! (`A`) and now (`N`). An undo writes `B` wherever `N` is still `A`; a redo writes `A` wherever
//! `N` is still `B`. A field someone else changed since keeps their value. An object the step
//! created goes away only if nobody changed it; an object the step deleted comes back with its old
//! id (`revive.rs`) only if the step's delete is still what deleted it. The result is one ordinary
//! commit, so it replicates like any edit.

use std::collections::BTreeMap;

use loro::{LoroDoc, LoroMap, LoroTree, TreeID};

use super::checkout::{Detached, NodeAt};
use super::error::HistoryError;
use super::lineage::encode_node;
use super::merge::{Tally, restore_map};
use super::node_value::Node;
use super::report::{
    Direction, ObjectReport, ObjectResult, ObjectShape, RestoreReport, Role, SkipReason,
};
use super::restore_plan::{Action, Plan, PlanInput, Snapshot, revival_indices};
use super::revive::{Revival, revive_nodes};
use super::step_header::{
    KEY_OBJECT, KEY_OBJECT_REDONE, KEY_OBJECT_UNDONE, KEY_REDONE, KEY_UNDONE, LABEL_OBJECT_REDO,
    LABEL_OBJECT_UNDO, LABEL_REDO, LABEL_UNDO, StepHeader,
};
use super::step_id::StepRef;
use super::step_log::{LogStep, find_step};
use super::touched::Touched;
use crate::clone_codec::{KEY_CLONE_LINK, link_from_value};
use crate::document::{Document, OBJECTS_TREE, ROOT_MAP};
use crate::path_model::NodeId;

fn node_id(id: TreeID) -> NodeId {
    NodeId::from_parts(id.peer, id.counter)
}

/// What a restore is for.
#[derive(Debug, Clone, Copy)]
pub(super) enum Mode {
    /// Ctrl+Z, Ctrl+Shift+Z: the whole step, for the maker's own stacks.
    Plain,
    /// An object undo or redo for `object` (`specs/0041-object-history` criterion 6): a new step
    /// of its own kind that names the step and the object. `only` cuts the step to that object;
    /// a Replace step is taken back whole.
    Object {
        /// The object the maker has selected.
        object: TreeID,
        /// Whether only this object's part of the step is restored.
        only: bool,
        /// What the header of the new step says it did.
        named: Named,
    },
}

/// The step an object undo or redo says it took back or applied again.
#[derive(Debug, Clone, Copy)]
pub(super) struct Named {
    /// The step.
    pub step: StepRef,
    /// Whether the new step is an object redo.
    pub redo: bool,
}

impl Document {
    /// Takes `step` back: writes what the step changed back, field by field, wherever nobody
    /// changed the field since (`specs/0020-undo-redo` criteria 16 to 20). Everything is one
    /// commit, labelled `undo`, that replicates like any edit. Nothing is written when every
    /// part of the step was skipped.
    ///
    /// # Errors
    /// [`HistoryError::NoSuchStep`] if the step is not in the log, [`HistoryError::Unreadable`] if
    /// the versions around it cannot be read, [`HistoryError::Revive`] if the deleted objects could
    /// not be brought back (then nothing was written).
    pub fn undo_step(&self, step: StepRef) -> Result<RestoreReport, HistoryError> {
        self.restore_step(step, Direction::Undo, Mode::Plain)
    }

    /// Applies `step` again after an undo, the mirror image of [`Document::undo_step`].
    ///
    /// # Errors
    /// As [`Document::undo_step`].
    pub fn redo_step(&self, step: StepRef) -> Result<RestoreReport, HistoryError> {
        self.restore_step(step, Direction::Redo, Mode::Plain)
    }

    pub(super) fn restore_step(
        &self,
        step: StepRef,
        direction: Direction,
        mode: Mode,
    ) -> Result<RestoreReport, HistoryError> {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        // The step this restore writes, if it writes one, starts at the next operation.
        let first = doc.oplog_vv().get(&doc.peer_id()).copied().unwrap_or(0);
        let log = find_step(doc, step).ok_or(HistoryError::NoSuchStep)?;
        let mut touched = self.touched_of(&log);
        if let Mode::Object {
            object, only: true, ..
        } = mode
        {
            touched = touched.only(object);
        }
        let (before, after) = read_versions(doc, &log, &touched)?;
        let plan = PlanInput {
            doc,
            log: &log,
            direction,
            touched: &touched,
            before: &before,
            after: &after,
        }
        .plan();
        let (label, key, named) = match (mode, direction) {
            (Mode::Plain, Direction::Undo) => (LABEL_UNDO, KEY_UNDONE, step),
            (Mode::Plain, Direction::Redo) => (LABEL_REDO, KEY_REDONE, step),
            (Mode::Object { named, .. }, _) if named.redo => {
                (LABEL_OBJECT_REDO, KEY_OBJECT_REDONE, named.step)
            }
            (Mode::Object { named, .. }, _) => (LABEL_OBJECT_UNDO, KEY_OBJECT_UNDONE, named.step),
        };
        let mut extra = vec![(key, named.encode())];
        if let Mode::Object { object, .. } = mode {
            extra.push((KEY_OBJECT, encode_node(object)));
        }
        let header = (plan.writes() || touched.root).then(|| self.next_header(label, &extra));
        let revived = match &header {
            Some(header) => self.revive_planned(&plan, &before, &after, header)?,
            None => false,
        };
        let (from, to) = match direction {
            Direction::Undo => (&after, &before),
            Direction::Redo => (&before, &after),
        };
        let objects = self.apply_plan(&plan, from, to);
        let root = touched.root.then(|| {
            let mut tally = Tally::default();
            restore_map(&doc.get_map(ROOT_MAP), &from.root, &to.root, &mut tally);
            result_of(tally)
        });
        let wrote = revived || doc.get_pending_txn_len() > 0;
        let mut written = None;
        if let Some(header) = &header {
            self.commit_with_header(header);
            if wrote && matches!(mode, Mode::Object { .. }) {
                let new = StepRef::new(doc.peer_id(), first);
                let reverts = find_step(doc, named).map(|reverted| reverted.label);
                self.commit_state.register_own(new, label, reverts);
                written = Some(new);
            }
        }
        Ok(RestoreReport {
            direction,
            objects,
            root,
            wrote,
            step: written,
        })
    }

    /// Brings every object the plan revives back, in one change. Returns whether there was one.
    pub(super) fn revive_planned(
        &self,
        plan: &Plan,
        before: &Snapshot,
        after: &Snapshot,
        header: &StepHeader,
    ) -> Result<bool, HistoryError> {
        let doc = self.loro();
        let mut places = revival_indices(doc, &plan.revive_from_before, before);
        places.extend(revival_indices(doc, &plan.revive_from_after, after));
        let nodes: Vec<Revival> = plan
            .items
            .iter()
            .filter_map(|p| match p.action {
                Action::Revive(id) => places.remove(&id).map(|place| Revival {
                    target: id,
                    parent: place.parent,
                    index: place.index,
                }),
                _ => None,
            })
            .collect();
        let nodes = parents_first(nodes);
        revive_nodes(doc, &nodes, &header.format())?;
        Ok(!nodes.is_empty())
    }

    /// Carries out the moves, deletes and field merges of the plan and reports each object.
    /// Moves come first, so a group is empty when its own delete comes; deletes go deepest first,
    /// so a copied group's leaves go before the group; a node that still has children is never
    /// deleted.
    pub(super) fn apply_plan(
        &self,
        plan: &Plan,
        from: &Snapshot,
        to: &Snapshot,
    ) -> Vec<ObjectReport> {
        let doc = self.loro();
        let tree = doc.get_tree(OBJECTS_TREE);
        tree.enable_fractional_index(0);
        let mut results: Vec<Option<ObjectResult>> = vec![None; plan.items.len()];
        let moves: Vec<(usize, TreeID, Option<TreeID>)> = plan
            .items
            .iter()
            .enumerate()
            .filter_map(|(at, planned)| match planned.action {
                Action::Move(id, parent) => Some((at, id, parent)),
                _ => None,
            })
            .collect();
        for (at, result) in move_nodes(&tree, &moves, to) {
            results[at] = Some(result);
        }
        let mut deletes: Vec<usize> = (0..plan.items.len())
            .filter(|&at| matches!(plan.items[at].action, Action::Delete(_)))
            .collect();
        deletes.sort_by_key(|&at| match plan.items[at].action {
            Action::Delete(id) => std::cmp::Reverse(depth_of(&tree, id)),
            _ => std::cmp::Reverse(0),
        });
        // The clones that follow what the restore removes are repaired in the same step, never
        // left as orphans (`specs/0049-linked-clone` criterion 36 b).
        // Every deleted node, groups too: the repair expands a subtree itself.
        let removed: Vec<TreeID> = deletes
            .iter()
            .filter_map(|&at| match plan.items[at].action {
                Action::Delete(id) => Some(id),
                _ => None,
            })
            .collect();
        self.repair_clones_of_removed(&tree, &removed);
        for at in deletes {
            if let Action::Delete(id) = plan.items[at].action {
                // Taking back a created group must not hide members that stayed in it.
                let guard_children = plan.items[at].role == Role::Created;
                results[at] = Some(delete_node(&tree, id, guard_children));
            }
        }
        plan.items
            .iter()
            .zip(results)
            .map(|(planned, done)| {
                let mut kept = 0;
                let (id, result) = match (&planned.action, done) {
                    (Action::Revive(id), _) => (*id, ObjectResult::Restored),
                    (Action::Skip(id, reason), _) => (*id, ObjectResult::Skipped(*reason)),
                    (Action::Move(id, ..) | Action::Delete(id), Some(result)) => (*id, result),
                    (Action::Move(id, ..) | Action::Delete(id), None) => {
                        (*id, ObjectResult::Skipped(SkipReason::ChangedLater))
                    }
                    (Action::Merge(id), _) => {
                        let nodes = (from.nodes.get(id), to.nodes.get(id), tree.get_meta(*id));
                        let result = if let (Some(f), Some(t), Ok(meta)) = nodes {
                            if writes_a_dead_link(&tree, &meta, &f.meta, &t.meta) {
                                ObjectResult::Skipped(SkipReason::ChangedLater)
                            } else {
                                let mut tally = Tally::default();
                                restore_map(&meta, &f.meta, &t.meta, &mut tally);
                                kept = tally.kept;
                                result_of(tally)
                            }
                        } else {
                            ObjectResult::Skipped(SkipReason::DeletedLater)
                        };
                        (*id, result)
                    }
                };
                ObjectReport {
                    id: node_id(id),
                    shape: kind_of(doc, id),
                    role: planned.role,
                    result,
                    kept,
                }
            })
            .collect()
    }

    /// The objects the step touched, from its operations.
    pub(super) fn touched_of(&self, log: &LogStep) -> Touched {
        let doc = self.loro();
        let mut index = self.container_index.borrow_mut();
        index.extend(doc);
        let changes = doc.export_json_in_id_span(log.span());
        index.touched(doc, &changes)
    }
}

/// How many nodes lie above `id` in the tree.
fn depth_of(tree: &loro::LoroTree, id: TreeID) -> usize {
    let mut depth = 0;
    let mut current = id;
    while let Some(loro::TreeParentId::Node(parent)) = tree.parent(current) {
        depth += 1;
        current = parent;
        if depth > 1_000 {
            break;
        }
    }
    depth
}

/// Puts the live nodes of `moves` (item index, node, target parent; `None` is the document root)
/// where the restored version `to` had them, one pass per parent. A node goes right after the
/// nearest node before it in that version's order that lives under the parent now (or was placed
/// a moment ago); with none before it, right before the nearest one after it; with no living
/// sibling at all, first. Siblings the version does not list never decide the place, so a peer's
/// objects that lie below the restored one stay below it.
fn move_nodes(
    tree: &loro::LoroTree,
    moves: &[(usize, TreeID, Option<TreeID>)],
    to: &Snapshot,
) -> Vec<(usize, ObjectResult)> {
    let mut done = Vec::with_capacity(moves.len());
    let mut parents: Vec<Option<TreeID>> = Vec::new();
    for &(_, _, parent) in moves {
        if !parents.contains(&parent) {
            parents.push(parent);
        }
    }
    for parent in parents {
        let parent_id = parent.map_or(loro::TreeParentId::Root, loro::TreeParentId::Node);
        let wanted: std::collections::HashMap<TreeID, usize> = moves
            .iter()
            .filter(|&&(_, _, p)| p == parent)
            .map(|&(at, id, _)| (id, at))
            .collect();
        let live: std::collections::HashSet<TreeID> = tree
            .children(parent_id)
            .unwrap_or_default()
            .into_iter()
            .filter(|child| !wanted.contains_key(child))
            .collect();
        let order: &[TreeID] = to.siblings.get(&parent).map_or(&[], Vec::as_slice);
        // The nearest later sibling that lives under the parent now, for each position.
        let mut above: Vec<Option<TreeID>> = vec![None; order.len()];
        let mut next = None;
        for (position, sibling) in order.iter().enumerate().rev() {
            above[position] = next;
            if live.contains(sibling) {
                next = Some(*sibling);
            }
        }
        let mut below: Option<TreeID> = None;
        let mut placed: std::collections::HashSet<TreeID> = std::collections::HashSet::new();
        for (position, sibling) in order.iter().enumerate() {
            if let Some(&at) = wanted.get(sibling) {
                done.push((
                    at,
                    placed_result(&match (below, above[position]) {
                        (Some(after), _) => tree.mov_after(*sibling, after),
                        (None, Some(before)) => tree.mov_before(*sibling, before),
                        (None, None) => tree.mov_to(*sibling, parent_id, 0),
                    }),
                ));
                placed.insert(*sibling);
                below = Some(*sibling);
            } else if live.contains(sibling) {
                below = Some(*sibling);
            }
        }
        // A node the version does not list (it cannot happen for a step's own nodes) goes last.
        for (&id, &at) in &wanted {
            if !placed.contains(&id) {
                let end = tree.children_num(parent_id).unwrap_or(0);
                done.push((
                    at,
                    placed_result(&tree.mov_to(id, parent_id, end.saturating_sub(1))),
                ));
            }
        }
    }
    done
}

/// The report of one move.
fn placed_result(moved: &loro::LoroResult<()>) -> ObjectResult {
    if moved.is_ok() {
        ObjectResult::Restored
    } else {
        ObjectResult::Skipped(SkipReason::ChangedLater)
    }
}

/// Deletes the live node `id`; with `keep_children` it stays when it still holds children (those a
/// peer or a skipped part of the step left in it).
fn delete_node(tree: &loro::LoroTree, id: TreeID, keep_children: bool) -> ObjectResult {
    if keep_children && tree.children_num(id).unwrap_or(0) > 0 {
        return ObjectResult::Skipped(SkipReason::ChangedLater);
    }
    if tree.delete(id).is_ok() {
        ObjectResult::Restored
    } else {
        ObjectResult::Skipped(SkipReason::DeletedLater)
    }
}

/// `nodes` with every node after the node it comes back under, when that comes back too.
fn parents_first(nodes: Vec<Revival>) -> Vec<Revival> {
    let mut rest = nodes;
    let mut ordered: Vec<Revival> = Vec::with_capacity(rest.len());
    while !rest.is_empty() {
        let waiting: Vec<TreeID> = rest.iter().map(|r| r.target).collect();
        let (ready, later): (Vec<_>, Vec<_>) = rest
            .into_iter()
            .partition(|r| r.parent.is_none_or(|p| !waiting.contains(&p)));
        if ready.is_empty() {
            // A cycle cannot be revived; hand the rest over and let the import refuse it.
            ordered.extend(later);
            break;
        }
        ordered.extend(ready);
        rest = later;
    }
    ordered
}

/// The kind of an object, read from its meta map (a deleted object's stays readable).
pub(crate) fn kind_of(doc: &LoroDoc, id: TreeID) -> ObjectShape {
    let Ok(meta) = doc.get_tree(OBJECTS_TREE).get_meta(id) else {
        return ObjectShape::Path;
    };
    if crate::group_tree::meta_is_group(&meta) {
        return ObjectShape::Group;
    }
    match crate::shape_codec::read_shape_tag(&meta).as_deref() {
        Some(crate::clone_codec::SHAPE_CLONE) => ObjectShape::Clone,
        Some(crate::shape_codec::SHAPE_RECT) => ObjectShape::Rectangle,
        Some(crate::shape_codec::SHAPE_ELLIPSE) => ObjectShape::Ellipse,
        Some(crate::shape_codec::SHAPE_POLYGON) => ObjectShape::Polygon,
        Some(crate::shape_codec::SHAPE_STAR) => ObjectShape::Star,
        _ => ObjectShape::Path,
    }
}

fn result_of(tally: Tally) -> ObjectResult {
    match (tally.written, tally.kept) {
        (_, 0) => ObjectResult::Restored,
        (0, _) => ObjectResult::Skipped(SkipReason::ChangedLater),
        _ => ObjectResult::Partial,
    }
}

/// Reads the touched objects before and after the step, through one guarded checkout pair.
fn read_versions(
    doc: &LoroDoc,
    log: &LogStep,
    touched: &Touched,
) -> Result<(Snapshot, Snapshot), HistoryError> {
    read_versions_between(doc, &log.before, &log.after(), touched)
}

/// The touched objects at the versions `before` and `after`, read under one checkout guard.
pub(super) fn read_versions_between(
    doc: &LoroDoc,
    before: &loro::Frontiers,
    after: &loro::Frontiers,
    touched: &Touched,
) -> Result<(Snapshot, Snapshot), HistoryError> {
    let ids: Vec<TreeID> = touched.nodes().collect();
    let read = |guard: &Detached<'_>| {
        let nodes: BTreeMap<TreeID, NodeAt> = ids.iter().map(|&id| (id, guard.node(id))).collect();
        // The children of the parents of created, deleted and moved objects, in z-order: where a
        // returning object finds its neighbours. Each node is read once, for both.
        let mut siblings = BTreeMap::new();
        for id in touched
            .created
            .iter()
            .chain(&touched.deleted)
            .chain(&touched.moved)
        {
            let parent = nodes.get(id).and_then(|node| node.parent);
            siblings
                .entry(parent)
                .or_insert_with(|| guard.children(parent));
        }
        let root = if touched.root {
            guard.root()
        } else {
            BTreeMap::new()
        };
        Snapshot {
            nodes,
            root,
            siblings,
        }
    };
    let guard = Detached::at(doc, before)?;
    let before = read(&guard);
    guard.goto(after)?;
    let after = read(&guard);
    Ok((before, after))
}

/// Undo rule (a) of `0049` criterion 36: the restore would write back a `clone_link` whose origin
/// is not a live path, compound path or clone after the restore. Such a node is left out of the
/// restore whole: it keeps the link it has now, or stays the path it is. Restoring only the other
/// fields would leave a clone tag with no link, a node no reader accepts.
fn writes_a_dead_link(
    tree: &LoroTree,
    meta: &LoroMap,
    from: &BTreeMap<String, Node>,
    to: &BTreeMap<String, Node>,
) -> bool {
    let (from_link, to_link) = (from.get(KEY_CLONE_LINK), to.get(KEY_CLONE_LINK));
    let now = meta.get(KEY_CLONE_LINK).as_ref().map(Node::read);
    if from_link == to_link || now.as_ref() != from_link {
        return false;
    }
    match to_link {
        Some(Node::Value(value)) => link_from_value(value)
            .is_some_and(|link| !crate::clone_write::origin_is_live(tree, link.origin)),
        _ => false,
    }
}
