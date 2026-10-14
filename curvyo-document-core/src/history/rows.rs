//! The rows of the global history: what the History tab reads (`specs/0020-undo-redo` criteria 32
//! to 38, 60). A row is derived from the log, never stored: its kind and its touched objects come
//! from the step's operations, its status from the stacks the commits tell.

use super::report::{ObjectShape, Role};
use super::restore::kind_of;
use super::stack_machine::Status;
use super::step_header::{
    KEY_OBJECT_REDONE, KEY_OBJECT_UNDONE, LABEL_OBJECT_REDO, LABEL_OBJECT_UNDO,
};
use super::step_id::StepRef;
use super::step_index::{StepIndex, is_row};
use super::touched::Touched;
use crate::document::Document;
use crate::path_model::NodeId;
use crate::units::Timestamp;

/// What a step did, derived from its operations and not from its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    /// Only created objects.
    Create,
    /// Only deleted objects.
    Delete,
    /// Created and deleted objects, or objects moved to another parent.
    Replace,
    /// Only changed objects.
    Change,
    /// No object: the document's own settings, or a wipe.
    Document,
}

/// Who made a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Author {
    /// This session.
    You,
    /// Any other peer, an earlier session of this file included.
    Earlier,
}

/// An object a step touched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TouchedObject {
    /// The object.
    pub id: NodeId,
    /// What it is (read from the object, which stays readable after a delete).
    pub shape: ObjectShape,
    /// What the step did to it.
    pub role: Role,
}

/// One row of the history list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    /// The step's identity.
    pub step: StepRef,
    /// The short id shown on the row, at least seven characters, unique in the document.
    pub shown_id: String,
    /// The commit label (`translate_objects`), or the whole message of an old file.
    pub label: String,
    /// The peer's session that made the step.
    pub author: Author,
    /// The peer id (the tooltip names it as "Earlier session").
    pub peer: u64,
    /// When the step was made; unknown for a step from a file written before steps had times.
    pub time: Timestamp,
    /// What the step did.
    pub kind: StepKind,
    /// The objects it touched, in the order of its operations.
    pub objects: Vec<TouchedObject>,
    /// Whether the step is in effect, undone, or replaced on a branch.
    pub status: Status,
    /// For an object undo or redo: the step it took back or applied again.
    pub reverts: Option<RowRevert>,
    /// Who took the step back with an object undo, when that took back the step as a whole
    /// (`specs/0041-object-history` criterion 8). Only set while `status` is applied.
    pub undone_by: Option<Author>,
}

/// The step an object undo or redo named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowRevert {
    /// The step.
    pub step: StepRef,
    /// Its shown id.
    pub shown_id: String,
    /// Its commit label.
    pub label: String,
    /// Whether the row is an object redo.
    pub redo: bool,
}

/// Whether a command is about the document and not about its objects: the objects only follow the
/// page (a resize or a fit moves them to keep their place on it), so the maker sees no object
/// changed, and a row lists none.
#[must_use]
pub fn is_document_level_label(label: &str) -> bool {
    matches!(label, "resize_document" | "fit_document_to_content")
}

/// Who took the step back as a whole with an object undo, if anybody did. A Replace step is taken
/// back whole by one object undo; any other step only when every object it touched was.
fn undone_by(index: &StepIndex, step: StepRef, touched: &Touched, me: u64) -> Option<Author> {
    let mut marks: Vec<_> = index
        .marks_on(step)
        .iter()
        .filter(|mark| index.status(mark.by) == Status::Applied)
        .filter_map(|mark| Some((index.position(mark.by)?, *mark)))
        .collect();
    marks.sort_unstable_by_key(|(position, _)| *position);
    let latest_undo = |objects: Option<loro::TreeID>| {
        marks
            .iter()
            .rev()
            .find(|(_, mark)| objects.is_none() || mark.object == objects)
            .filter(|(_, mark)| !mark.redo)
            .map(|(_, mark)| mark.by)
    };
    let by = if kind_of_touched(touched) == StepKind::Replace {
        latest_undo(None)?
    } else {
        let mut nodes = touched.nodes().peekable();
        nodes.peek()?;
        nodes
            .map(|node| latest_undo(Some(node)))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .next()?
    };
    Some(if by.peer() == me {
        Author::You
    } else {
        Author::Earlier
    })
}

/// The kind of a step: from what it touched, except the wipe of one object's history, which
/// changes no object (`specs/0041-object-history` criterion 30).
pub(super) fn kind_of_step(label: &str, touched: &Touched) -> StepKind {
    if label == super::step_header::LABEL_WIPE_OBJECT {
        StepKind::Document
    } else {
        kind_of_touched(touched)
    }
}

/// The kind of a step from what it touched.
pub(super) fn kind_of_touched(touched: &Touched) -> StepKind {
    let created = !touched.created.is_empty();
    let deleted = !touched.deleted.is_empty();
    let moved = !touched.moved.is_empty();
    let changed = !touched.changed.is_empty();
    match (created, deleted, moved, changed) {
        (true, true, _, _) | (_, _, true, _) => StepKind::Replace,
        (true, false, false, _) => StepKind::Create,
        (false, true, false, _) => StepKind::Delete,
        (false, false, false, true) => StepKind::Change,
        (false, false, false, false) => StepKind::Document,
    }
}

impl Document {
    /// The number of steps in the history: every step of every peer, not the undo and redo
    /// commits and not a new document's setup.
    #[must_use]
    pub fn history_len(&self) -> usize {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        let mut index = self.step_index.borrow_mut();
        index.extend(doc);
        index.steps().filter(|step| is_row(step)).count()
    }

    /// The objects `step` touched (created, deleted, changed), for the hover link of a row.
    #[must_use]
    pub fn touched_objects(&self, step: StepRef) -> Vec<NodeId> {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        self.container_index.borrow_mut().extend(doc);
        let Some(log) = super::step_log::find_step(doc, step) else {
            return Vec::new();
        };
        let changes = doc.export_json_in_id_span(log.span());
        let touched = self.container_index.borrow().touched(doc, &changes);
        touched
            .created
            .iter()
            .chain(&touched.deleted)
            .chain(&touched.changed)
            .map(|id| NodeId::from_parts(id.peer, id.counter))
            .collect()
    }

    /// The nodes `step` created: a subset of [`Document::touched_objects`]. The Object history's
    /// wipe counts a layer the step created with the object (the Initial Layer of
    /// `specs/0050-layers-and-objects-panel` criterion 35) as part of that object's own step.
    #[must_use]
    pub fn created_objects(&self, step: StepRef) -> Vec<NodeId> {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        self.container_index.borrow_mut().extend(doc);
        let Some(log) = super::step_log::find_step(doc, step) else {
            return Vec::new();
        };
        let changes = doc.export_json_in_id_span(log.span());
        let touched = self.container_index.borrow().touched(doc, &changes);
        touched
            .created
            .iter()
            .map(|id| NodeId::from_parts(id.peer, id.counter))
            .collect()
    }

    /// The shown id of the newest step that is in effect: what the pinned "Now" row names.
    #[must_use]
    pub fn history_newest_applied_id(&self) -> Option<String> {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        let mut index = self.step_index.borrow_mut();
        index.extend(doc);
        index.ensure_shown();
        let newest = index
            .steps()
            .rev()
            .filter(|step| is_row(step))
            .find(|step| index.status(step.step) == Status::Applied)?
            .step;
        index.shown().get(&newest).cloned()
    }

    /// `count` rows of the history from the `start`-th newest, newest first.
    #[must_use]
    pub fn history_page(&self, start: usize, count: usize) -> Vec<HistoryRow> {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        self.container_index.borrow_mut().extend(doc);
        let mut guard = self.step_index.borrow_mut();
        guard.extend(doc);
        guard.ensure_shown();
        let index = &*guard;
        let shown = index.shown();
        let me = doc.peer_id();
        let chunk: Vec<_> = index
            .steps()
            .rev()
            .filter(|step| is_row(step))
            .skip(start)
            .take(count)
            .cloned()
            .collect();
        let containers = self.container_index.borrow();
        chunk
            .into_iter()
            .map(|log| {
                let changes = doc.export_json_in_id_span(log.span());
                let touched = containers.touched(doc, &changes);
                let objects = touched
                    .created
                    .iter()
                    .map(|id| (id, Role::Created))
                    .chain(touched.deleted.iter().map(|id| (id, Role::Deleted)))
                    .chain(touched.changed.iter().map(|id| (id, Role::Changed)))
                    .map(|(id, role)| TouchedObject {
                        id: NodeId::from_parts(id.peer, id.counter),
                        shape: kind_of(doc, *id),
                        role,
                    })
                    .collect();
                let document_level = is_document_level_label(&log.label);
                let objects = if document_level { Vec::new() } else { objects };
                HistoryRow {
                    step: log.step,
                    shown_id: shown.get(&log.step).cloned().unwrap_or_default(),
                    label: log.label.clone(),
                    author: if log.step.peer() == me {
                        Author::You
                    } else {
                        Author::Earlier
                    },
                    peer: log.step.peer(),
                    time: log.header.as_ref().map_or(Timestamp::UNKNOWN, |h| h.time),
                    kind: if document_level {
                        StepKind::Document
                    } else {
                        kind_of_step(&log.label, &touched)
                    },
                    objects,
                    status: index.status(log.step),
                    reverts: log.header.as_ref().and_then(|header| {
                        let (key, redo) = match header.label.as_str() {
                            LABEL_OBJECT_UNDO => (KEY_OBJECT_UNDONE, false),
                            LABEL_OBJECT_REDO => (KEY_OBJECT_REDONE, true),
                            _ => return None,
                        };
                        let step = StepRef::decode(header.extra_value(key)?)?;
                        Some(RowRevert {
                            step,
                            shown_id: shown.get(&step).cloned().unwrap_or_default(),
                            label: index.get(step).map(|t| t.label.clone()).unwrap_or_default(),
                            redo,
                        })
                    }),
                    undone_by: if index.status(log.step) == Status::Applied {
                        undone_by(index, log.step, &touched, me)
                    } else {
                        None
                    },
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kind_follows_what_the_step_did() {
        let id = loro::TreeID::new(1, 1);
        let with = |created: bool, deleted: bool, moved: bool, changed: bool| Touched {
            created: if created { vec![id] } else { vec![] },
            deleted: if deleted { vec![id] } else { vec![] },
            moved: if moved { vec![id] } else { vec![] },
            changed: if changed { vec![id] } else { vec![] },
            root: false,
        };
        assert_eq!(
            kind_of_touched(&with(true, false, false, false)),
            StepKind::Create
        );
        assert_eq!(
            kind_of_touched(&with(false, true, false, false)),
            StepKind::Delete
        );
        assert_eq!(
            kind_of_touched(&with(true, true, false, false)),
            StepKind::Replace
        );
        assert_eq!(
            kind_of_touched(&with(false, false, true, false)),
            StepKind::Replace
        );
        assert_eq!(
            kind_of_touched(&with(false, false, false, true)),
            StepKind::Change
        );
        assert_eq!(
            kind_of_touched(&with(false, false, false, false)),
            StepKind::Document
        );
    }

    #[test]
    fn adjacent_commits_with_one_label_of_an_old_file_are_one_row() {
        // A document written the way earlier builds wrote: plain labels, no header, and the root
        // setup riding in the first commit.
        let loro = loro::LoroDoc::new();
        loro.set_peer_id(7).unwrap();
        loro.get_map("root").insert("format_version", 9).unwrap();
        loro.commit_with(loro::CommitOptions::new().commit_msg("create_rect"));
        loro.get_map("root").insert("size_width_mm", 100.0).unwrap();
        loro.commit_with(loro::CommitOptions::new().commit_msg("create_rect"));
        loro.get_map("root")
            .insert("size_height_mm", 100.0)
            .unwrap();
        loro.commit_with(loro::CommitOptions::new().commit_msg("something_new"));
        let snapshot = loro.export(loro::ExportMode::Snapshot).unwrap();
        let document = Document::from_loro_snapshot(1, &snapshot).unwrap();
        let rows = document.history_page(0, document.history_len());
        assert_eq!(
            rows.len(),
            2,
            "the two adjacent create_rect commits are one row"
        );
        assert_eq!(rows[0].label, "something_new");
        assert_eq!(rows[1].label, "create_rect");
    }
}
