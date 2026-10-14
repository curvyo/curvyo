//! The rows of an object's timeline, what the Object scope of the History tab reads
//! (`specs/0041-object-history` criteria 1 to 4, 19 to 21, 24, 26, 28).
//!
//! A row is derived from the log like a row of the global history; the object adds the role the
//! step had for it, the number of other objects the step touched, the objects a created object came
//! from, and the status an object undo gave the step.

use std::collections::HashSet;

use loro::TreeID;

use super::lineage::clone_origin;
use super::object_timeline::{Entry, EntryClass, Reader, Targets, TimelineStatus};
use super::report::Role;
use super::rows::{Author, StepKind, kind_of_step};
use super::step_id::StepRef;
use super::touched::Touched;
use crate::document::Document;
use crate::group_tree::is_layer_node;
use crate::path_model::NodeId;
use crate::units::Timestamp;

/// What a row of an object's timeline is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectRowClass {
    /// A step an interaction made.
    Step,
    /// An object undo (or, with `redo`, an object redo) of the step `step`.
    Reverts {
        /// The step it took back or applied again.
        step: StepRef,
        /// Its shown id.
        shown_id: String,
        /// Its commit label.
        label: String,
        /// Whether this is an object redo.
        redo: bool,
    },
    /// The step that made the object as a copy of `source` ("Duplicate of 3f9a1c").
    CloneStart {
        /// The object it was copied from.
        source: NodeId,
    },
    /// The wipe of the object's history ("History wiped"): the last row of the timeline.
    Floor,
}

/// How a row of an object's timeline stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectStatus {
    /// In effect.
    Applied,
    /// Taken back with Ctrl+Z (or replaced after that).
    Undone,
    /// Taken back by an object undo of this author.
    UndoneBy(Author),
}

/// One row of an object's timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRow {
    /// The step's identity (the same as in the global history).
    pub step: StepRef,
    /// The shown id of the step.
    pub shown_id: String,
    /// The commit label.
    pub label: String,
    /// Who made the step.
    pub author: Author,
    /// When; unknown for a step of an old file.
    pub time: Timestamp,
    /// What kind of row this is.
    pub class: ObjectRowClass,
    /// What the step did overall.
    pub kind: StepKind,
    /// What the step did to the object this row is about.
    pub role: Role,
    /// How many other objects the step touched.
    pub others: usize,
    /// For a created object: the objects the step deleted (the operands of a Boolean).
    pub sources: Vec<NodeId>,
    /// The object this row was inherited from; `None` for the object's own rows.
    pub inherited_from: Option<NodeId>,
    /// Whether the step is in effect.
    pub status: ObjectStatus,
}

/// A step an object undo or redo would act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectTarget {
    /// The step.
    pub step: StepRef,
    /// Its shown id.
    pub shown_id: String,
    /// Its commit label.
    pub label: String,
    /// Who made it.
    pub author: Author,
    /// When.
    pub time: Timestamp,
    /// The object it was inherited from, when the step is one of the rows a copy took over.
    pub inherited_from: Option<NodeId>,
}

/// The steps Ctrl+U and Ctrl+Shift+U would act on for one object.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObjectTargets {
    /// The latest applied step.
    pub undo: Option<ObjectTarget>,
    /// The step the last object undo took back.
    pub redo: Option<ObjectTarget>,
    /// How many steps an object undo could take back, the first one included. An object undo
    /// passes over steps that write nothing (`ObjectRestore::passed_over`).
    pub undo_candidates: usize,
}

pub(super) fn tree_id(id: NodeId) -> TreeID {
    TreeID::new(id.peer, id.counter)
}

pub(super) fn node_id(id: TreeID) -> NodeId {
    NodeId::from_parts(id.peer, id.counter)
}

impl Document {
    /// The number of rows in the timeline of `id`, inherited rows and the floor included. Builds
    /// the per-object index on the first call.
    #[must_use]
    pub fn object_history_len(&self, id: NodeId) -> usize {
        self.with_reader(|reader, _| reader.timeline(tree_id(id)).entries.len())
    }

    /// `count` rows of the timeline of `id` from the `start`-th newest, newest first.
    #[must_use]
    pub fn object_history_page(&self, id: NodeId, start: usize, count: usize) -> Vec<ObjectRow> {
        self.sync_object_index();
        self.step_index.borrow_mut().ensure_shown();
        self.with_reader(|reader, me| {
            let subject = tree_id(id);
            let timeline = reader.timeline(subject);
            timeline
                .entries
                .iter()
                .rev()
                .skip(start)
                .take(count)
                .map(|entry| {
                    let status = timeline.status(entry, reader.steps);
                    self.object_row(reader, me, subject, entry, status)
                })
                .collect()
        })
    }

    /// The steps Ctrl+U and Ctrl+Shift+U would act on for `id`.
    #[must_use]
    pub fn object_history_targets(&self, id: NodeId) -> ObjectTargets {
        let object = tree_id(id);
        let mut targets = self.targets_of(object);
        if targets.undo.is_some_and(|entry| entry.from.is_some()) {
            // The first step an inherited press would write, not the first it would pass over.
            targets.undo = self.first_writable_undo(object);
        }
        ObjectTargets {
            undo: targets.undo.and_then(|entry| self.target_of(&entry)),
            redo: targets.redo.and_then(|entry| self.target_of(&entry)),
            undo_candidates: targets.undo_candidates,
        }
    }

    /// Whether the object has rows it took over from the object it was copied from.
    #[must_use]
    pub fn object_inherits(&self, id: NodeId) -> bool {
        self.with_reader(|reader, _| {
            reader
                .timeline(tree_id(id))
                .entries
                .iter()
                .any(|entry| entry.from.is_some())
        })
    }

    /// The steps an object undo of `object` may take back, newest first.
    pub(super) fn undo_candidates_of(&self, object: TreeID) -> Vec<Entry> {
        self.with_reader(|reader, _| reader.timeline(object).undo_candidates(reader.steps))
    }

    /// The entries an object undo and an object redo of `object` would act on.
    pub(super) fn targets_of(&self, object: TreeID) -> Targets {
        self.with_reader(|reader, _| reader.timeline(object).targets(reader.steps))
    }

    /// The description of the step an entry stands for.
    pub(super) fn target_of(&self, entry: &Entry) -> Option<ObjectTarget> {
        self.sync_object_index();
        self.step_index.borrow_mut().ensure_shown();
        self.with_reader(|reader, me| {
            let log = reader.steps.get(entry.step)?;
            Some(ObjectTarget {
                step: entry.step,
                shown_id: reader
                    .steps
                    .shown()
                    .get(&entry.step)
                    .cloned()
                    .unwrap_or_default(),
                label: log.label.clone(),
                author: author_of(entry.step, me),
                time: log.header.as_ref().map_or(Timestamp::UNKNOWN, |h| h.time),
                inherited_from: entry.from.map(node_id),
            })
        })
    }

    /// How many rows a wipe of `id`'s history would hide: the rows of the timeline except the
    /// floor row.
    #[must_use]
    pub fn object_wipe_count(&self, id: NodeId) -> usize {
        self.with_reader(|reader, _| {
            reader
                .timeline(tree_id(id))
                .entries
                .iter()
                .filter(|entry| entry.class != EntryClass::Floor)
                .count()
        })
    }

    /// Runs `read` with the three indexes brought up to the log.
    fn with_reader<R>(&self, read: impl FnOnce(&Reader<'_>, u64) -> R) -> R {
        self.sync_object_index();
        let steps = self.step_index.borrow();
        let objects = self.object_index.borrow();
        let reader = Reader {
            doc: self.loro(),
            steps: &steps,
            objects: &objects,
        };
        read(&reader, self.loro().peer_id())
    }

    fn object_row(
        &self,
        reader: &Reader<'_>,
        me: u64,
        top: TreeID,
        entry: &Entry,
        status: TimelineStatus,
    ) -> ObjectRow {
        let subject = entry.from.unwrap_or(top);
        let log = reader.steps.get(entry.step);
        let touched = log.map(|log| {
            let changes = reader.doc.export_json_in_id_span(log.span());
            self.container_index.borrow().touched(reader.doc, &changes)
        });
        let touched = touched.unwrap_or_default();
        let shown = |step: StepRef| reader.steps.shown().get(&step).cloned().unwrap_or_default();
        let class = match entry.class {
            EntryClass::Floor => ObjectRowClass::Floor,
            EntryClass::Reverts { target, redo } => ObjectRowClass::Reverts {
                step: target,
                shown_id: shown(target),
                label: reader
                    .steps
                    .get(target)
                    .map(|t| t.label.clone())
                    .unwrap_or_default(),
                redo,
            },
            EntryClass::Do => match clone_origin(reader.doc, subject) {
                Some((source, _))
                    if reader.steps.step_at(subject.peer, subject.counter) == Some(entry.step) =>
                {
                    ObjectRowClass::CloneStart {
                        source: node_id(source),
                    }
                }
                _ => ObjectRowClass::Step,
            },
        };
        let created_here = touched.created.contains(&subject);
        ObjectRow {
            step: entry.step,
            shown_id: shown(entry.step),
            label: log.map(|log| log.label.clone()).unwrap_or_default(),
            author: author_of(entry.step, me),
            time: log
                .and_then(|log| log.header.as_ref())
                .map_or(Timestamp::UNKNOWN, |h| h.time),
            class,
            kind: kind_of_step(&log.map_or(String::new(), |l| l.label.clone()), &touched),
            role: role_of(&touched, subject),
            others: other_objects(
                &touched,
                subject,
                &reader.doc.get_tree(crate::document::OBJECTS_TREE),
            ),
            sources: if created_here {
                touched.deleted.iter().map(|id| node_id(*id)).collect()
            } else {
                Vec::new()
            },
            inherited_from: entry.from.map(node_id),
            status: match status {
                TimelineStatus::Applied => ObjectStatus::Applied,
                TimelineStatus::Undone => ObjectStatus::Undone,
                TimelineStatus::UndoneBy(by) => ObjectStatus::UndoneBy(author_of(by, me)),
            },
        }
    }
}

fn author_of(step: StepRef, me: u64) -> Author {
    if step.peer() == me {
        Author::You
    } else {
        Author::Earlier
    }
}

/// What the step did to `subject`.
fn role_of(touched: &Touched, subject: TreeID) -> Role {
    if touched.created.contains(&subject) {
        Role::Created
    } else if touched.deleted.contains(&subject) {
        Role::Deleted
    } else if touched.moved.contains(&subject) && !touched.changed.contains(&subject) {
        Role::Moved
    } else {
        Role::Changed
    }
}

/// How many objects other than `subject` the step touched. A layer the step created is not
/// counted: it is the Initial Layer made with the object (`specs/0050-layers-and-objects-panel`
/// criterion 35), part of the object's own step.
fn other_objects(touched: &Touched, subject: TreeID, tree: &loro::LoroTree) -> usize {
    touched
        .nodes()
        .filter(|id| *id != subject)
        .filter(|id| !(touched.created.contains(id) && is_layer_node(tree, *id)))
        .collect::<HashSet<_>>()
        .len()
}
