//! The timeline of one object: the steps that touched it, the rows it inherited from the object it
//! was copied from, what its history was cut at, and which step an object undo or redo would act
//! on (`specs/0041-object-history` criteria 1, 8, 11, 24 to 26, 30, 41).
//!
//! Nothing here is stored. The entries are read from the per-object index; the floor comes from
//! the object's wipe step; the inherited entries are the source's entries in the causal past of the
//! version at the copy. Which step is the latest applied one comes from the stack machine of
//! `0020`, run over the entries oldest first with the object undo and redo marks as events.

use std::collections::HashMap;

use loro::{LoroDoc, TreeID, VersionVector};

use super::lineage::{clone_origin, decode_node};
use super::object_index::ObjectIndex;
use super::rows::is_document_level_label;
use super::stack_machine::{Event, StackMachine, Status};
use super::step_header::{
    KEY_OBJECT, KEY_OBJECT_REDONE, KEY_OBJECT_UNDONE, LABEL_NEW_DOCUMENT, LABEL_OBJECT_REDO,
    LABEL_OBJECT_UNDO, LABEL_REDO, LABEL_UNDO, LABEL_WIPE, LABEL_WIPE_OBJECT,
};
use super::step_id::StepRef;
use super::step_index::{Position, StepIndex};

/// How many copies of copies the lineage follows: a guard against a damaged file, not a limit a
/// maker meets.
const MAX_LINEAGE_DEPTH: usize = 64;

/// What an entry of a timeline is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryClass {
    /// A step an interaction made.
    Do,
    /// An object undo or redo of the step `target`.
    Reverts {
        /// The step it took back or applied again.
        target: StepRef,
        /// Whether it is a redo.
        redo: bool,
    },
    /// The wipe of this object's history: the floor.
    Floor,
}

/// One step in a timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Entry {
    pub step: StepRef,
    pub class: EntryClass,
    /// The object whose own timeline the step belongs to, for a step inherited from the object a
    /// copy was made from; `None` for the object's own steps.
    pub from: Option<TreeID>,
}

/// What reads the three indexes; the indexes must be current.
pub(crate) struct Reader<'a> {
    pub doc: &'a LoroDoc,
    pub steps: &'a StepIndex,
    pub objects: &'a ObjectIndex,
}

/// The entries of one object, oldest first, and the stacks they tell.
pub(crate) struct Timeline {
    pub entries: Vec<Entry>,
    machine: StackMachine,
    /// The object undo that took a step back, by the step.
    undone_by: HashMap<StepRef, StepRef>,
    /// The step that made the object, which is no target of an object undo when the object has
    /// inherited rows to go back into.
    creation_skipped: Option<StepRef>,
}

/// The entries an object undo and an object redo would act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Targets {
    pub undo: Option<Entry>,
    pub redo: Option<Entry>,
    /// The object undo that took the redo target back.
    pub redo_mark: Option<StepRef>,
}

impl Reader<'_> {
    /// The timeline of `id`: its own entries after its floor, with the entries it inherited
    /// before them.
    pub(crate) fn timeline(&self, id: TreeID) -> Timeline {
        let entries = self.entries_of(id, None, 0);
        let inherited = entries.iter().any(|entry| entry.from.is_some());
        let (machine, undone_by) = self.run(&entries);
        Timeline {
            entries,
            machine,
            undone_by,
            creation_skipped: inherited
                .then(|| self.steps.step_at(id.peer, id.counter))
                .flatten(),
        }
    }

    /// The entries of `id` visible at `within` (everything when `None`), oldest first, preceded by
    /// the inherited ones unless the object's history was wiped.
    fn entries_of(&self, id: TreeID, within: Option<&VersionVector>, depth: usize) -> Vec<Entry> {
        let (own, wiped) = self.own_entries(id, within);
        let mut out = Vec::new();
        if !wiped
            && depth < MAX_LINEAGE_DEPTH
            && let Some((source, version)) = clone_origin(self.doc, id)
            && let Some(vv) = self.doc.frontiers_to_vv(&version)
        {
            out = self.entries_of(source, Some(&vv), depth + 1);
        }
        out.extend(own.into_iter().map(|mut entry| {
            if depth > 0 {
                entry.from = Some(id);
            }
            entry
        }));
        out
    }

    /// The steps that touched `id` at `within`, oldest first, after the object's floor. The flag
    /// says a floor cut the history.
    fn own_entries(&self, id: TreeID, within: Option<&VersionVector>) -> (Vec<Entry>, bool) {
        let mut listed: Vec<(Position, StepRef)> = self
            .objects
            .steps_of(id)
            .iter()
            .filter_map(|step| Some((self.steps.position(*step)?, *step)))
            .collect();
        listed.sort_unstable();
        listed.dedup();
        let seen_in = |vv: &VersionVector, step: StepRef| {
            self.steps
                .get(step)
                .is_some_and(|log| vv.get(&step.peer()).is_some_and(|end| *end >= log.end))
        };
        if let Some(vv) = within {
            listed.retain(|(_, step)| seen_in(vv, *step));
        }
        let mut entries: Vec<Entry> = listed
            .iter()
            .filter_map(|(_, step)| {
                Some(Entry {
                    step: *step,
                    class: self.class_of(*step, id)?,
                    from: None,
                })
            })
            .collect();
        let floor = entries
            .iter()
            .rev()
            .find(|entry| entry.class == EntryClass::Floor)
            .and_then(|entry| self.steps.get(entry.step))
            .and_then(|log| self.doc.frontiers_to_vv(&log.before));
        if let Some(floor) = &floor {
            entries.retain(|entry| !seen_in(floor, entry.step));
        }
        (entries, floor.is_some())
    }

    /// What a step is in the timeline of `id`; `None` for a step that is no row of it.
    fn class_of(&self, step: StepRef, id: TreeID) -> Option<EntryClass> {
        let log = self.steps.get(step)?;
        let Some(header) = &log.header else {
            return Some(EntryClass::Do);
        };
        let target = |key: &str| header.extra_value(key).and_then(StepRef::decode);
        match header.label.as_str() {
            LABEL_NEW_DOCUMENT | LABEL_UNDO | LABEL_REDO | LABEL_WIPE => None,
            label if is_document_level_label(label) => None,
            LABEL_OBJECT_UNDO => Some(EntryClass::Reverts {
                target: target(KEY_OBJECT_UNDONE)?,
                redo: false,
            }),
            LABEL_OBJECT_REDO => Some(EntryClass::Reverts {
                target: target(KEY_OBJECT_REDONE)?,
                redo: true,
            }),
            LABEL_WIPE_OBJECT => (header.extra_value(KEY_OBJECT).and_then(decode_node) == Some(id))
                .then_some(EntryClass::Floor),
            _ => Some(EntryClass::Do),
        }
    }

    /// Runs the stack machine over `entries`: an ordinary step is done, an object undo or redo that
    /// is itself in effect takes its target back or brings it again.
    fn run(&self, entries: &[Entry]) -> (StackMachine, HashMap<StepRef, StepRef>) {
        let mut machine = StackMachine::new(usize::MAX);
        let mut undone_by = HashMap::new();
        for entry in entries {
            match entry.class {
                EntryClass::Do => machine.apply(Event::Do(entry.step)),
                EntryClass::Reverts { target, redo }
                    if self.steps.status(entry.step) == Status::Applied =>
                {
                    if redo {
                        machine.apply(Event::Redo(target));
                        undone_by.remove(&target);
                    } else {
                        machine.apply(Event::Undo(target));
                        undone_by.insert(target, entry.step);
                    }
                }
                EntryClass::Reverts { .. } | EntryClass::Floor => {}
            }
        }
        (machine, undone_by)
    }
}

impl Timeline {
    /// How an entry stands: applied, taken back by its author's undo, or by an object undo.
    pub(crate) fn status(&self, entry: &Entry, steps: &StepIndex) -> TimelineStatus {
        if steps.status(entry.step) != Status::Applied {
            return TimelineStatus::Undone;
        }
        match entry.class {
            EntryClass::Do if self.machine.status(entry.step) != Status::Applied => self
                .undone_by
                .get(&entry.step)
                .map_or(TimelineStatus::Undone, |by| TimelineStatus::UndoneBy(*by)),
            _ => TimelineStatus::Applied,
        }
    }

    /// The steps an object undo and an object redo would act on now.
    pub(crate) fn targets(&self, steps: &StepIndex) -> Targets {
        let find = |step: StepRef| self.entries.iter().rev().find(|e| e.step == step).copied();
        let applied = |step: &StepRef| steps.status(*step) == Status::Applied;
        let undo = self
            .machine
            .undo_stack()
            .iter()
            .rev()
            .find(|step| applied(step) && Some(**step) != self.creation_skipped)
            .and_then(|step| find(*step));
        let redo = self.machine.top_redo().filter(applied).and_then(find);
        let redo_mark = redo.and_then(|entry| self.undone_by.get(&entry.step).copied());
        Targets {
            undo,
            redo,
            redo_mark,
        }
    }
}

/// How a row of a timeline stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimelineStatus {
    /// In effect.
    Applied,
    /// Taken back by its author's Ctrl+Z, or replaced after that.
    Undone,
    /// Taken back by this object undo.
    UndoneBy(StepRef),
}
