//! Undo and redo of one step by restore over versions (ADR 0014 §1).
//!
//! For each object the step touched the engine knows the object before the step (`B`), after it
//! (`A`) and now (`N`). An undo writes `B` wherever `N` is still `A`; a redo writes `A` wherever
//! `N` is still `B`. A field someone else changed since keeps their value. An object the step
//! created goes away only if nobody changed it; an object the step deleted comes back with its old
//! id (`revive.rs`) only if the step's delete is still what deleted it. The result is one ordinary
//! commit, so it replicates like any edit.

use std::collections::BTreeMap;

use loro::{LoroDoc, TreeID};

use super::checkout::Detached;
use super::error::HistoryError;
use super::merge::{Tally, restore_map};
use super::report::{
    Direction, ObjectReport, ObjectResult, ObjectShape, RestoreReport, Role, SkipReason,
};
use super::restore_plan::{Action, Plan, PlanInput, Snapshot, revival_indices};
use super::revive::{Revival, revive_nodes};
use super::step_header::{KEY_REDONE, KEY_UNDONE, LABEL_REDO, LABEL_UNDO, StepHeader};
use super::step_id::StepRef;
use super::step_log::{LogStep, find_step};
use super::touched::Touched;
use crate::document::{Document, OBJECTS_TREE, ROOT_MAP};
use crate::path_model::NodeId;

fn node_id(id: TreeID) -> NodeId {
    NodeId::from_parts(id.peer, id.counter)
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
        self.restore_step(step, Direction::Undo)
    }

    /// Applies `step` again after an undo, the mirror image of [`Document::undo_step`].
    ///
    /// # Errors
    /// As [`Document::undo_step`].
    pub fn redo_step(&self, step: StepRef) -> Result<RestoreReport, HistoryError> {
        self.restore_step(step, Direction::Redo)
    }

    fn restore_step(
        &self,
        step: StepRef,
        direction: Direction,
    ) -> Result<RestoreReport, HistoryError> {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        let log = find_step(doc, step).ok_or(HistoryError::NoSuchStep)?;
        let touched = self.touched_of(&log);
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
        let (label, key) = match direction {
            Direction::Undo => (LABEL_UNDO, KEY_UNDONE),
            Direction::Redo => (LABEL_REDO, KEY_REDONE),
        };
        let header = (plan.writes() || touched.root)
            .then(|| self.next_header(label, &[(key, step.encode())]));
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
        if let Some(header) = &header {
            self.commit_with_header(header);
        }
        Ok(RestoreReport {
            direction,
            objects,
            root,
            wrote,
        })
    }

    /// Brings every object the plan revives back, in one change. Returns whether there was one.
    fn revive_planned(
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
    fn apply_plan(&self, plan: &Plan, from: &Snapshot, to: &Snapshot) -> Vec<ObjectReport> {
        let doc = self.loro();
        let tree = doc.get_tree(OBJECTS_TREE);
        tree.enable_fractional_index(0);
        let mut results: Vec<Option<ObjectResult>> = vec![None; plan.items.len()];
        for (at, planned) in plan.items.iter().enumerate() {
            if let Action::Move(id, parent) = &planned.action {
                let order: &[TreeID] = to.siblings.get(parent).map_or(&[], Vec::as_slice);
                results[at] = Some(move_node(&tree, *id, *parent, order));
            }
        }
        let mut deletes: Vec<usize> = (0..plan.items.len())
            .filter(|&at| matches!(plan.items[at].action, Action::Delete(_)))
            .collect();
        deletes.sort_by_key(|&at| match plan.items[at].action {
            Action::Delete(id) => std::cmp::Reverse(depth_of(&tree, id)),
            _ => std::cmp::Reverse(0),
        });
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
                            let mut tally = Tally::default();
                            restore_map(&meta, &f.meta, &t.meta, &mut tally);
                            result_of(tally)
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
                }
            })
            .collect()
    }

    /// The objects the step touched, from its operations.
    fn touched_of(&self, log: &LogStep) -> Touched {
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

/// Moves the live node `id` under `parent` (`None` is the document root), after the live
/// children of `parent` that come before it in `order`, the children the version being restored
/// had (it lists `id` itself).
fn move_node(
    tree: &loro::LoroTree,
    id: TreeID,
    parent: Option<TreeID>,
    order: &[TreeID],
) -> ObjectResult {
    let parent_id = parent.map_or(loro::TreeParentId::Root, loro::TreeParentId::Node);
    let live: Vec<TreeID> = tree.children(parent_id).unwrap_or_default();
    let position = order
        .iter()
        .take_while(|&&sibling| sibling != id)
        .filter(|&&sibling| sibling != id && live.contains(&sibling))
        .count();
    if tree.mov_to(id, parent_id, position).is_ok() {
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
    if crate::groups::meta_is_group(&meta) {
        return ObjectShape::Group;
    }
    match crate::shape_codec::read_shape_tag(&meta).as_deref() {
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

/// The children, at the version `guard` is checked out to, of every parent that a created,
/// deleted or moved object has at that version.
fn sibling_lists(guard: &Detached<'_>, touched: &Touched) -> BTreeMap<Option<TreeID>, Vec<TreeID>> {
    let mut lists = BTreeMap::new();
    for &id in touched
        .created
        .iter()
        .chain(&touched.deleted)
        .chain(&touched.moved)
    {
        let parent = guard.node(id).parent;
        lists
            .entry(parent)
            .or_insert_with(|| guard.children(parent));
    }
    lists
}

/// Reads the touched objects before and after the step, through one guarded checkout pair.
fn read_versions(
    doc: &LoroDoc,
    log: &LogStep,
    touched: &Touched,
) -> Result<(Snapshot, Snapshot), HistoryError> {
    let ids: Vec<TreeID> = touched.nodes().collect();
    let read = |guard: &Detached<'_>| Snapshot {
        nodes: ids.iter().map(|&id| (id, guard.node(id))).collect(),
        root: if touched.root {
            guard.root()
        } else {
            BTreeMap::new()
        },
        // The z-order is only read to place objects that come back.
        siblings: sibling_lists(guard, touched),
    };
    let guard = Detached::at(doc, &log.before)?;
    let before = read(&guard);
    guard.goto(&log.after())?;
    let after = read(&guard);
    Ok((before, after))
}
