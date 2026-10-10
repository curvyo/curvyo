//! Undo and redo of one step by restore over versions (ADR 0014 §1).
//!
//! For each object the step touched the engine knows the object before the step (`B`), after it
//! (`A`) and now (`N`). An undo writes `B` wherever `N` is still `A`; a redo writes `A` wherever
//! `N` is still `B`. A field someone else changed since keeps their value. An object the step
//! created goes away only if nobody changed it; an object the step deleted comes back with its old
//! id (`revive.rs`) only if the step's delete is still what deleted it. The result is one ordinary
//! commit, so it replicates like any edit.

use std::collections::BTreeMap;

use loro::{FractionalIndex, LoroDoc, TreeID};

use super::checkout::{Detached, NodeAt};
use super::error::HistoryError;
use super::node_value::{Node, Tally, read_map, restore_map};
use super::report::{Direction, ObjectReport, ObjectResult, RestoreReport, Role, SkipReason};
use super::revive::revive_node;
use super::step_header::{KEY_REDONE, KEY_UNDONE, LABEL_REDO, LABEL_UNDO};
use super::step_id::StepRef;
use super::step_log::{LogStep, find_step};
use super::touched::Touched;
use crate::document::{Document, OBJECTS_TREE, ROOT_MAP};
use crate::path_model::NodeId;

/// The touched objects at one version.
struct Snapshot {
    nodes: BTreeMap<TreeID, NodeAt>,
    root: BTreeMap<String, Node>,
    roots: Vec<TreeID>,
}

/// What the engine decided to do for one touched object.
enum Action {
    Revive { id: TreeID, index: FractionalIndex },
    Delete(TreeID),
    Merge { id: TreeID },
    Skip(TreeID, SkipReason),
}

struct Planned {
    role: Role,
    action: Action,
}

fn node_id(id: TreeID) -> NodeId {
    NodeId::from_parts(id.peer, id.counter)
}

fn alive(doc: &LoroDoc, id: TreeID) -> bool {
    let tree = doc.get_tree(OBJECTS_TREE);
    tree.contains(id) && matches!(tree.is_node_deleted(&id), Ok(false))
}

impl Document {
    /// Takes `step` back: writes what the step changed back, field by field, wherever nobody
    /// changed the field since (`specs/0020-undo-redo` criteria 16 to 20). Everything is one
    /// commit, labelled `undo`, that replicates like any edit. Nothing is written when every
    /// part of the step was skipped.
    ///
    /// # Errors
    /// [`HistoryError::NoSuchStep`] if the step is not in the log, [`HistoryError::Unreadable`] if
    /// the versions around it cannot be read, [`HistoryError::Revive`] if a deleted object could
    /// not be brought back.
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
        doc.commit();
        let log = find_step(doc, step).ok_or(HistoryError::NoSuchStep)?;
        let touched = self.touched_of(&log);
        let (before, after) = read_versions(doc, &log, &touched)?;
        let (from, to) = match direction {
            Direction::Undo => (&after, &before),
            Direction::Redo => (&before, &after),
        };
        let plan = self.plan(&log, direction, &touched, &before, &after);

        let (label, key) = match direction {
            Direction::Undo => (LABEL_UNDO, KEY_UNDONE),
            Direction::Redo => (LABEL_REDO, KEY_REDONE),
        };
        let revives = plan
            .iter()
            .filter(|p| matches!(p.action, Action::Revive { .. }))
            .count();
        let header = (revives > 0
            || plan.iter().any(|p| !matches!(p.action, Action::Skip(..)))
            || touched.root)
            .then(|| self.next_header(label, &[(key, step.encode())]));
        let mut wrote = false;
        if let Some(header) = &header {
            for planned in &plan {
                if let Action::Revive { id, index } = &planned.action {
                    revive_node(doc, *id, index.clone(), &header.format())?;
                    wrote = true;
                }
            }
        }
        let tree = doc.get_tree(OBJECTS_TREE);
        let mut objects = Vec::with_capacity(plan.len());
        for planned in &plan {
            let (id, result) = match &planned.action {
                Action::Revive { id, .. } => (*id, ObjectResult::Restored),
                Action::Skip(id, reason) => (*id, ObjectResult::Skipped(*reason)),
                Action::Delete(id) => {
                    let done = tree.delete(*id).is_ok();
                    let result = if done {
                        ObjectResult::Restored
                    } else {
                        ObjectResult::Skipped(SkipReason::DeletedLater)
                    };
                    (*id, result)
                }
                Action::Merge { id } => {
                    let (Some(from_node), Some(to_node), Ok(meta)) =
                        (from.nodes.get(id), to.nodes.get(id), tree.get_meta(*id))
                    else {
                        objects.push(ObjectReport {
                            id: node_id(*id),
                            role: planned.role,
                            result: ObjectResult::Skipped(SkipReason::DeletedLater),
                        });
                        continue;
                    };
                    let mut tally = Tally::default();
                    restore_map(&meta, &from_node.meta, &to_node.meta, &mut tally);
                    (*id, result_of(tally))
                }
            };
            objects.push(ObjectReport {
                id: node_id(id),
                role: planned.role,
                result,
            });
        }
        let root = touched.root.then(|| {
            let mut tally = Tally::default();
            restore_map(&doc.get_map(ROOT_MAP), &from.root, &to.root, &mut tally);
            result_of(tally)
        });
        let has_writes = doc.get_pending_txn_len() > 0;
        if let Some(header) = &header {
            if has_writes {
                self.commit_with_header(header);
            } else if wrote {
                self.note_last_commit(header);
            }
        }
        Ok(RestoreReport {
            direction,
            objects,
            root,
            wrote: wrote || has_writes,
        })
    }

    /// Decides what to do for each touched object (reads only).
    fn plan(
        &self,
        log: &LogStep,
        direction: Direction,
        touched: &Touched,
        before: &Snapshot,
        after: &Snapshot,
    ) -> Vec<Planned> {
        let doc = self.loro();
        let tree = doc.get_tree(OBJECTS_TREE);
        let live_meta = |id: TreeID| tree.get_meta(id).map(|m| read_map(&m)).ok();
        let mut plan = Vec::new();
        let mut push = |role: Role, action: Action| plan.push(Planned { role, action });
        // A node's position when it comes back: the version it was alive at decides the order.
        let revival = |id: TreeID, snapshot: &Snapshot| {
            let wanted = snapshot.nodes.get(&id).and_then(|n| n.index.clone());
            Action::Revive {
                id,
                index: revival_index(doc, id, wanted, &snapshot.roots),
            }
        };
        let in_step = |id: TreeID| {
            tree.get_last_move_id(&id).is_some_and(|last| {
                last.peer == log.step.peer()
                    && last.counter >= log.step.counter()
                    && last.counter < log.end
            })
        };
        let by_me = |id: TreeID| {
            tree.get_last_move_id(&id)
                .is_some_and(|last| last.peer == doc.peer_id())
        };
        for &id in &touched.created {
            let now_alive = alive(doc, id);
            let created_meta = after.nodes.get(&id).map(|n| &n.meta);
            match direction {
                Direction::Undo if !now_alive => {
                    push(Role::Created, Action::Skip(id, SkipReason::DeletedLater))
                }
                Direction::Undo if live_meta(id).as_ref() == created_meta => {
                    push(Role::Created, Action::Delete(id));
                }
                Direction::Undo => push(Role::Created, Action::Skip(id, SkipReason::ChangedLater)),
                Direction::Redo if now_alive => {
                    push(Role::Created, Action::Skip(id, SkipReason::ChangedLater))
                }
                Direction::Redo if by_me(id) => push(Role::Created, revival(id, after)),
                Direction::Redo => push(Role::Created, Action::Skip(id, SkipReason::DeletedLater)),
            }
        }
        for &id in &touched.deleted {
            let now_alive = alive(doc, id);
            let deleted_meta = before.nodes.get(&id).map(|n| &n.meta);
            match direction {
                Direction::Undo if now_alive => {
                    push(Role::Deleted, Action::Skip(id, SkipReason::ChangedLater))
                }
                Direction::Undo if in_step(id) => push(Role::Deleted, revival(id, before)),
                Direction::Undo => push(Role::Deleted, Action::Skip(id, SkipReason::DeletedLater)),
                Direction::Redo if !now_alive => {
                    push(Role::Deleted, Action::Skip(id, SkipReason::DeletedLater))
                }
                Direction::Redo if live_meta(id).as_ref() == deleted_meta => {
                    push(Role::Deleted, Action::Delete(id));
                }
                Direction::Redo => push(Role::Deleted, Action::Skip(id, SkipReason::ChangedLater)),
            }
        }
        for &id in &touched.changed {
            if alive(doc, id) {
                push(Role::Changed, Action::Merge { id });
            } else {
                push(Role::Changed, Action::Skip(id, SkipReason::DeletedLater));
            }
        }
        plan
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
    let ids: Vec<TreeID> = touched.nodes().collect();
    let read = |guard: &Detached<'_>| Snapshot {
        nodes: ids.iter().map(|&id| (id, guard.node(id))).collect(),
        root: if touched.root {
            guard.root()
        } else {
            BTreeMap::new()
        },
        roots: guard.roots(),
    };
    let guard = Detached::at(doc, &log.before)?;
    let before = read(&guard);
    guard.goto(&log.after())?;
    let after = read(&guard);
    Ok((before, after))
}

/// The position a revived node takes: its old one while that still lies between its nearest
/// living neighbours of that version, else one between them.
fn revival_index(
    doc: &LoroDoc,
    id: TreeID,
    wanted: Option<FractionalIndex>,
    siblings: &[TreeID],
) -> FractionalIndex {
    let tree = doc.get_tree(OBJECTS_TREE);
    let index_of = |sibling: TreeID| {
        tree.fractional_index(sibling)
            .map(FractionalIndex::from_hex_string)
    };
    let position = siblings
        .iter()
        .position(|s| *s == id)
        .unwrap_or(siblings.len());
    let lower = siblings[..position]
        .iter()
        .rev()
        .copied()
        .filter(|s| alive(doc, *s))
        .find_map(index_of);
    let upper = siblings
        .get(position + 1..)
        .unwrap_or_default()
        .iter()
        .copied()
        .filter(|s| alive(doc, *s))
        .find_map(index_of);
    let fits = |old: &FractionalIndex| {
        lower.as_ref().is_none_or(|l| l < old) && upper.as_ref().is_none_or(|u| old < u)
    };
    match wanted {
        Some(old) if fits(&old) => old,
        _ => FractionalIndex::new(lower.as_ref(), upper.as_ref()).unwrap_or_default(),
    }
}
