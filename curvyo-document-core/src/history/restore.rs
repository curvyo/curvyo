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
    Direction, ObjectReport, ObjectResult, ObjectShape, RestoreReport, SkipReason,
};
use super::restore_plan::{Action, Plan, PlanInput, Snapshot, revival_indices};
use super::revive::revive_nodes;
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
        let mut indices = revival_indices(doc, &plan.revive_from_before, before);
        indices.extend(revival_indices(doc, &plan.revive_from_after, after));
        let nodes: Vec<_> = plan
            .items
            .iter()
            .filter_map(|p| match p.action {
                Action::Revive(id) => indices.remove(&id).map(|index| (id, index)),
                _ => None,
            })
            .collect();
        revive_nodes(doc, &nodes, &header.format())?;
        Ok(!nodes.is_empty())
    }

    /// Carries out the deletes and field merges of the plan and reports each object.
    fn apply_plan(&self, plan: &Plan, from: &Snapshot, to: &Snapshot) -> Vec<ObjectReport> {
        let doc = self.loro();
        let tree = doc.get_tree(OBJECTS_TREE);
        plan.items
            .iter()
            .map(|planned| {
                let (id, result) = match &planned.action {
                    Action::Revive(id) => (*id, ObjectResult::Restored),
                    Action::Skip(id, reason) => (*id, ObjectResult::Skipped(*reason)),
                    Action::Delete(id) => {
                        let result = if tree.delete(*id).is_ok() {
                            ObjectResult::Restored
                        } else {
                            ObjectResult::Skipped(SkipReason::DeletedLater)
                        };
                        (*id, result)
                    }
                    Action::Merge(id) => {
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

/// The kind of an object, read from its meta map (a deleted object's stays readable).
pub(crate) fn kind_of(doc: &LoroDoc, id: TreeID) -> ObjectShape {
    let Ok(meta) = doc.get_tree(OBJECTS_TREE).get_meta(id) else {
        return ObjectShape::Path;
    };
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
