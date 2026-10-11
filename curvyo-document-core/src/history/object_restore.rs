//! Object undo and object redo (`specs/0041-object-history` criteria 5 to 15, 25, 40).
//!
//! An object undo takes back one object's part of its latest applied step, as a new step of the
//! maker's own: the engine of `0020` restricted to the object for a Change, Create or Delete step,
//! the whole step for a Replace step. A step an object inherited from the object it was copied from
//! is not an operation of this object: the object's registers are set to the values they had
//! before that step.

use std::collections::BTreeMap;

use loro::TreeID;

use super::checkout::Detached;
use super::error::HistoryError;
use super::lineage::{KEY_CLONE_OF, KEY_HISTORY_FLOOR, encode_node};
use super::node_value::Node;
use super::object_rows::{ObjectTarget, node_id, tree_id};
use super::object_timeline::Entry;
use super::report::{Direction, ObjectReport, ObjectResult, RestoreReport, Role, SkipReason};
use super::restore::{Mode, Named, kind_of};
use super::restore_plan::alive;
use super::rows::{StepKind, kind_of_touched};
use super::step_header::{KEY_OBJECT, KEY_OBJECT_UNDONE, LABEL_OBJECT_UNDO};
use super::step_id::StepRef;
use super::step_log::find_step;
use crate::document::{Document, OBJECTS_TREE};
use crate::path_model::NodeId;

/// What an object undo or redo did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRestore {
    /// The step that was taken back or applied again.
    pub target: ObjectTarget,
    /// What was written, per object.
    pub report: RestoreReport,
    /// Whether the object no longer exists (its creation was taken back).
    pub object_gone: bool,
    /// Whether the whole step was restored because it replaced objects (a Boolean, Combine).
    pub whole_step: bool,
}

impl Document {
    /// Takes the latest applied step of the object back, for this object only. `Ok(None)` when
    /// there is nothing to take back.
    ///
    /// # Errors
    /// As [`Document::undo_step`].
    pub fn undo_object_step(&self, id: NodeId) -> Result<Option<ObjectRestore>, HistoryError> {
        self.restore_object(id, Direction::Undo)
    }

    /// Brings back the step the last object undo took back, for this object only. `Ok(None)`
    /// when there is nothing to bring back.
    ///
    /// # Errors
    /// As [`Document::undo_step`].
    pub fn redo_object_step(&self, id: NodeId) -> Result<Option<ObjectRestore>, HistoryError> {
        self.restore_object(id, Direction::Redo)
    }

    fn restore_object(
        &self,
        id: NodeId,
        direction: Direction,
    ) -> Result<Option<ObjectRestore>, HistoryError> {
        let object = tree_id(id);
        let Some((entry, mark, target)) = self.object_action(object, direction) else {
            return Ok(None);
        };
        let doc = self.loro();
        let log = find_step(doc, entry.step).ok_or(HistoryError::NoSuchStep)?;
        let touched = self.touched_of(&log);
        let whole_step = entry.from.is_none() && kind_of_touched(&touched) == StepKind::Replace;
        let report = match (entry.from, direction, mark) {
            (None, _, _) => {
                let named = Named {
                    step: entry.step,
                    redo: direction == Direction::Redo,
                };
                let mode = Mode::Object {
                    object,
                    only: !whole_step,
                    named,
                };
                self.restore_step(entry.step, direction, mode)?
            }
            (Some(source), Direction::Undo, _) => self.undo_inherited(object, source, &log)?,
            (Some(_), Direction::Redo, Some(mark)) => {
                let named = Named {
                    step: entry.step,
                    redo: true,
                };
                let mode = Mode::Object {
                    object,
                    only: true,
                    named,
                };
                self.restore_step(mark, Direction::Undo, mode)?
            }
            (Some(_), Direction::Redo, None) => return Ok(None),
        };
        Ok(Some(ObjectRestore {
            target,
            report,
            object_gone: !alive(doc, object),
            whole_step,
        }))
    }

    /// The step the press acts on, the object undo that took it back (for a redo), and its
    /// description.
    fn object_action(
        &self,
        object: TreeID,
        direction: Direction,
    ) -> Option<(Entry, Option<StepRef>, ObjectTarget)> {
        let targets = self.targets_of(object);
        let (entry, mark) = match direction {
            Direction::Undo => (targets.undo?, None),
            Direction::Redo => (targets.redo?, targets.redo_mark),
        };
        let target = self.target_of(&entry)?;
        Some((entry, mark, target))
    }

    /// Takes back a step the object inherited from `source`: a creation removes the object, a
    /// change sets the plain registers the step wrote back to what they were before it. Anchor
    /// lists are left alone: a copy has anchor ids of its own.
    fn undo_inherited(
        &self,
        object: TreeID,
        source: TreeID,
        log: &super::step_log::LogStep,
    ) -> Result<RestoreReport, HistoryError> {
        let doc = self.loro();
        doc.commit();
        let first = doc.oplog_vv().get(&doc.peer_id()).copied().unwrap_or(0);
        let touched = self.touched_of(log).only(source);
        let header = self.next_header(
            LABEL_OBJECT_UNDO,
            &[
                (KEY_OBJECT_UNDONE, log.step.encode()),
                (KEY_OBJECT, encode_node(object)),
            ],
        );
        let tree = doc.get_tree(OBJECTS_TREE);
        let (role, written, kept) = if !touched.created.is_empty() {
            let gone = alive(doc, object) && tree.delete(object).is_ok();
            (Role::Created, usize::from(gone), 0)
        } else if touched.changed.is_empty() {
            (Role::Changed, 0, 0)
        } else {
            let (written, kept) = self.restore_registers(log, source, object)?;
            (Role::Changed, written, kept)
        };
        let wrote = written > 0;
        let mut step = None;
        if wrote {
            self.commit_with_header(&header);
            let new = StepRef::new(doc.peer_id(), first);
            self.commit_state
                .register_own(new, LABEL_OBJECT_UNDO, Some(log.label.clone()));
            step = Some(new);
        }
        let result = match (written, kept) {
            (0, _) => ObjectResult::Skipped(SkipReason::ChangedLater),
            (_, 0) => ObjectResult::Restored,
            _ => ObjectResult::Partial,
        };
        Ok(RestoreReport {
            direction: Direction::Undo,
            objects: vec![ObjectReport {
                id: node_id(object),
                shape: kind_of(doc, object),
                role,
                result,
                kept,
            }],
            root: None,
            wrote,
            step,
        })
    }

    /// Writes the plain registers `touched` changed in `source` at the step back to `object`, as
    /// they were before the step. Returns what was written and what was left (nested values).
    fn restore_registers(
        &self,
        log: &super::step_log::LogStep,
        source: TreeID,
        object: TreeID,
    ) -> Result<(usize, usize), HistoryError> {
        let doc = self.loro();
        let (before, after) = {
            let guard = Detached::at(doc, &log.before)?;
            let before = guard.node(source).meta;
            guard.goto(&log.after())?;
            (before, guard.node(source).meta)
        };
        let Ok(meta) = doc.get_tree(OBJECTS_TREE).get_meta(object) else {
            return Ok((0, 0));
        };
        let keys: Vec<&String> = before
            .keys()
            .chain(after.keys())
            .filter(|key| key.as_str() != KEY_CLONE_OF && key.as_str() != KEY_HISTORY_FLOOR)
            .collect();
        let (mut written, mut kept) = (0, 0);
        let mut seen = std::collections::BTreeSet::new();
        for key in keys {
            if !seen.insert(key) || before.get(key) == after.get(key) {
                continue;
            }
            match plain_value(&before, key) {
                Plain::Value(value) => {
                    if meta.insert(key, value).is_ok() {
                        written += 1;
                    }
                }
                Plain::Absent => {
                    if meta.delete(key).is_ok() {
                        written += 1;
                    }
                }
                Plain::Nested => kept += 1,
            }
        }
        Ok((written, kept))
    }
}

/// What a key holds, as far as a restore of plain registers is concerned.
enum Plain {
    /// A plain value.
    Value(loro::LoroValue),
    /// No such key.
    Absent,
    /// A nested container, which is left alone.
    Nested,
}

fn plain_value(meta: &BTreeMap<String, Node>, key: &str) -> Plain {
    match meta.get(key) {
        None => Plain::Absent,
        Some(Node::Value(value)) => Plain::Value(value.clone()),
        Some(_) => Plain::Nested,
    }
}
