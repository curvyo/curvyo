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
    /// How many newer steps the press passed over because they wrote nothing: steps a copy
    /// inherited that only moved or reshaped the original.
    pub passed_over: usize,
}

/// How many candidates one press tries; each one reads two versions, except a step that wrote no
/// register.
const MAX_PASSED_OVER: usize = 256;

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
        if direction == Direction::Redo {
            let targets = self.targets_of(object);
            let Some(entry) = targets.redo else {
                return Ok(None);
            };
            return self.restore_entry(object, entry, direction, targets.redo_mark);
        }
        // An inherited step may write nothing (it only moved or reshaped the original, whose
        // place the copy does not share): such a step is passed over to the next one.
        let candidates = self.undo_candidates_of(object);
        let mut first = None;
        for (passed_over, entry) in candidates.into_iter().take(MAX_PASSED_OVER + 1).enumerate() {
            let Some(restore) = self.restore_entry(object, entry, direction, None)? else {
                continue;
            };
            if restore.report.wrote || entry.from.is_none() {
                return Ok(Some(ObjectRestore {
                    passed_over,
                    ..restore
                }));
            }
            first.get_or_insert(restore);
        }
        Ok(first)
    }

    /// Takes `entry` back or brings it again, for `object`.
    fn restore_entry(
        &self,
        object: TreeID,
        entry: Entry,
        direction: Direction,
        mark: Option<StepRef>,
    ) -> Result<Option<ObjectRestore>, HistoryError> {
        let Some(target) = self.target_of(&entry) else {
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
            // A copy that is gone has nothing to take back from the original's steps.
            (Some(_), Direction::Undo, _) if !alive(doc, object) => return Ok(None),
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
            passed_over: 0,
        }))
    }

    /// Takes back a step the object inherited from `source`: the registers the step changed in
    /// the original are set back to what they were before it, where the copy still holds the
    /// value the original had after it (criterion 7). Nothing the original did to its anchors or
    /// its place is written into the copy, and the step that made the original or a copy is
    /// never taken back this way: the copy would vanish.
    fn undo_inherited(
        &self,
        object: TreeID,
        source: TreeID,
        log: &super::step_log::LogStep,
    ) -> Result<RestoreReport, HistoryError> {
        let doc = self.loro();
        doc.commit();
        let first = doc.oplog_vv().get(&doc.peer_id()).copied().unwrap_or(0);
        let (writes, kept) = self.plan_registers(log, source, object)?;
        let wrote = !writes.is_empty();
        let mut step = None;
        if wrote && let Ok(meta) = doc.get_tree(OBJECTS_TREE).get_meta(object) {
            let header = self.next_header(
                LABEL_OBJECT_UNDO,
                &[
                    (KEY_OBJECT_UNDONE, log.step.encode()),
                    (KEY_OBJECT, encode_node(object)),
                ],
            );
            for (key, value) in &writes {
                // Both writes only touch a plain register of a live meta map.
                let _ = match value {
                    Some(value) => meta.insert(key, value.clone()),
                    None => meta.delete(key),
                };
            }
            self.commit_with_header(&header);
            let new = StepRef::new(doc.peer_id(), first);
            self.commit_state
                .register_own(new, LABEL_OBJECT_UNDO, Some(log.label.clone()));
            step = Some(new);
        }
        let result = match (wrote, kept) {
            (false, _) => ObjectResult::Skipped(SkipReason::ChangedLater),
            (true, 0) => ObjectResult::Restored,
            (true, _) => ObjectResult::Partial,
        };
        Ok(RestoreReport {
            direction: Direction::Undo,
            objects: vec![ObjectReport {
                id: node_id(object),
                shape: kind_of(doc, object),
                role: Role::Changed,
                result,
                kept,
            }],
            root: None,
            wrote: step.is_some(),
            step,
        })
    }

    /// What undoing an inherited step would write into the copy, and how many fields keep their
    /// value. Reads two versions of the original, unless the step wrote no register of it.
    fn plan_registers(
        &self,
        log: &super::step_log::LogStep,
        source: TreeID,
        object: TreeID,
    ) -> Result<(Vec<RegisterWrite>, usize), HistoryError> {
        let doc = self.loro();
        if !self.wrote_a_register(log, source) {
            return Ok((Vec::new(), 0));
        }
        let (before, after) = {
            let guard = Detached::at(doc, &log.before)?;
            let before = guard.node(source).meta;
            guard.goto(&log.after())?;
            (before, guard.node(source).meta)
        };
        let Ok(meta) = doc.get_tree(OBJECTS_TREE).get_meta(object) else {
            return Ok((Vec::new(), 0));
        };
        let now = super::node_value::read_map(&meta);
        let (mut writes, mut kept) = (Vec::new(), 0);
        let keys: std::collections::BTreeSet<&String> = before
            .keys()
            .chain(after.keys())
            .filter(|key| key.as_str() != KEY_CLONE_OF && key.as_str() != KEY_HISTORY_FLOOR)
            .collect();
        for key in keys {
            if before.get(key) == after.get(key) {
                continue;
            }
            if FRAME_KEYS.contains(&key.as_str())
                && let (Some(b), Some(a), Some(n)) = (before.get(key), after.get(key), now.get(key))
                && let Some((merged, written, left)) = merge_frame(b, a, n)
            {
                kept += left;
                if written > 0 {
                    writes.push((key.clone(), Some(merged)));
                }
                continue;
            }
            // Criterion 7: a field the copy holds differently from the original after the step
            // (its displacement at the copy, a later edit by anyone) keeps its value.
            if now.get(key) != after.get(key) {
                kept += 1;
                continue;
            }
            match plain_value(&before, key) {
                Plain::Value(value) => writes.push((key.clone(), Some(value))),
                Plain::Absent => writes.push((key.clone(), None)),
                Plain::Nested => kept += 1,
            }
        }
        Ok((writes, kept))
    }

    /// Whether the step wrote a register of the original's meta map. A step that only edited
    /// nested lists (a path's anchors) did not, and costs no checkout to pass over.
    fn wrote_a_register(&self, log: &super::step_log::LogStep, source: TreeID) -> bool {
        let meta = source.associated_meta_container();
        self.loro()
            .export_json_in_id_span(log.span())
            .iter()
            .flat_map(|change| &change.ops)
            .any(|op| {
                op.container == meta
                    && matches!(
                        &op.content,
                        loro::JsonOpContent::Map(
                            loro::JsonMapOp::Insert { .. } | loro::JsonMapOp::Delete { .. }
                        )
                    )
            })
    }

    /// The first step an object undo of `object` would really write, looking past the steps it
    /// would pass over: what the Undo step button offers. Remembered until the log changes.
    pub(super) fn first_writable_undo(&self, object: TreeID) -> Option<Entry> {
        let stamp = self.loro().len_ops();
        if let Some((at, of, found)) = self.writable_cache.borrow().as_ref()
            && *at == stamp
            && *of == object
        {
            return *found;
        }
        let found = self
            .undo_candidates_of(object)
            .into_iter()
            .take(MAX_PASSED_OVER + 1)
            .find(|entry| {
                let Some(source) = entry.from else {
                    return true;
                };
                find_step(self.loro(), entry.step)
                    .and_then(|log| self.plan_registers(&log, source, object).ok())
                    .is_some_and(|(writes, _)| !writes.is_empty())
            });
        *self.writable_cache.borrow_mut() = Some((stamp, object, found));
        found
    }
}

/// The remembered answer of [`Document::first_writable_undo`]: the length of the log, the object
/// and the entry.
pub(crate) type WritableUndo = (usize, TreeID, Option<Entry>);

/// A register an object undo writes: the key and the value, `None` for a key to remove.
type RegisterWrite = (String, Option<loro::LoroValue>);

/// The registers that hold a position and a size together as four numbers: origin and size of a
/// rectangle, centre and radii of an ellipse, centre, radius and angle of a star.
const FRAME_KEYS: &[&str] = &[
    crate::shape_codec::KEY_RECT_BOUNDS,
    crate::shape_codec::KEY_ELLIPSE_FRAME,
    crate::shape_codec::KEY_STAR_FRAME,
];

/// The three-way merge of a frame, number by number: the copy gets back the numbers the step
/// changed where it still holds the original's value after the step (the size), and keeps the
/// others (its own place). Returns the merged list, what was written and what was kept.
fn merge_frame(before: &Node, after: &Node, now: &Node) -> Option<(loro::LoroValue, usize, usize)> {
    let list = |node: &Node| match node {
        Node::Value(loro::LoroValue::List(items)) if items.len() == 4 => Some(items.to_vec()),
        _ => None,
    };
    let (before, after, mut merged) = (list(before)?, list(after)?, list(now)?);
    let now = merged.clone();
    let (mut written, mut kept) = (0, 0);
    for index in 0..4 {
        if before[index] == after[index] {
            continue;
        }
        if now[index] == after[index] {
            merged[index] = before[index].clone();
            written += 1;
        } else {
            kept += 1;
        }
    }
    Some((loro::LoroValue::from(merged), written, kept))
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
