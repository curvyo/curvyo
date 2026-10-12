//! Every step in the operation log, in order, with the stacks each peer's commits tell: the source
//! of the history list (`specs/0020-undo-redo` criteria 32 to 38). Built from the log on first use
//! and extended by the changes committed or imported since.

use std::collections::{BTreeMap, HashMap};

use loro::{LoroDoc, VersionVector};

use super::stack_machine::{BranchMark, Event, StackMachine, Status};
use super::step_header::{
    KEY_OBJECT, KEY_OBJECT_REDONE, KEY_OBJECT_UNDONE, KEY_REDONE, KEY_UNDONE, LABEL_NEW_DOCUMENT,
    LABEL_OBJECT_REDO, LABEL_OBJECT_UNDO, LABEL_REDO, LABEL_UNDO, LABEL_WIPE, LABEL_WIPE_OBJECT,
    StepHeader,
};
use super::step_id::{StepRef, shown_ids};
use super::step_log::LogStep;

/// Where a step sits in the order of the log: the Lamport clock of its first operation, then its
/// peer (ADR 0014 §6: the main-line order of concurrent steps), then its counter.
pub(crate) type Position = (u32, u64, i32);

/// An object undo or redo that names the step it took back or applied again
/// (`specs/0041-object-history` criterion 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mark {
    /// The object undo or redo step.
    pub by: StepRef,
    /// Whether it is a redo.
    pub redo: bool,
    /// The object the maker had selected.
    pub object: Option<loro::TreeID>,
}

/// The steps of the log and the stacks of every peer.
#[derive(Debug, Default)]
pub(crate) struct StepIndex {
    indexed: VersionVector,
    steps: BTreeMap<Position, LogStep>,
    /// The newest step of each peer, which the peer's next change may extend.
    newest: HashMap<u64, Position>,
    machines: HashMap<u64, StackMachine>,
    /// Where the newest wipe sits: nothing before it is listed (the shallow snapshot keeps one
    /// change from before it).
    wiped_at: Option<Position>,
    positions: HashMap<StepRef, Position>,
    /// The start counters of each peer's steps, to find the step an operation belongs to.
    by_peer: HashMap<u64, BTreeMap<i32, Position>>,
    /// The object undos and redos that name a step, by the step they name.
    marks: HashMap<StepRef, Vec<Mark>>,
    shown: HashMap<StepRef, String>,
    shown_stale: bool,
}

impl StepIndex {
    /// Reads the changes committed or imported since the last call.
    pub(crate) fn extend(&mut self, doc: &LoroDoc) {
        let end = doc.oplog_vv();
        if end == self.indexed {
            return;
        }
        for (&peer, &last) in end.iter() {
            // After a wipe the log starts later than counter zero.
            let floor = doc.shallow_since_vv().get(&peer).copied().unwrap_or(0);
            let mut counter = self.indexed.get(&peer).copied().unwrap_or(0).max(floor);
            while counter < last {
                let Some(change) = doc.get_change(loro::ID::new(peer, counter)) else {
                    // The log starts later than counter zero after a wipe.
                    counter += 1;
                    continue;
                };
                let message = change.message.as_deref().unwrap_or_default().to_owned();
                let next = change.id.counter + i32::try_from(change.len).unwrap_or(i32::MAX);
                if change.id.counter < counter {
                    // Loro merged this commit into the change that was indexed before: the
                    // step it belongs to grew.
                    if let Some(step) = self.newest.get(&peer).and_then(|p| self.steps.get_mut(p)) {
                        step.end = next;
                    }
                } else {
                    self.note_change(peer, &change, &message, next);
                }
                counter = next;
            }
        }
        self.indexed = end;
    }

    fn note_change(&mut self, peer: u64, change: &loro::ChangeMeta, message: &str, next: i32) {
        let extends = self.newest.get(&peer).and_then(|position| {
            let step = self.steps.get(position)?;
            (step.message == message && step.end == change.id.counter).then_some(*position)
        });
        if let Some(position) = extends {
            if let Some(step) = self.steps.get_mut(&position) {
                step.end = next;
            }
            return;
        }
        let step = StepRef::new(peer, change.id.counter);
        let header = StepHeader::parse(message);
        let label = header
            .as_ref()
            .map_or_else(|| message.to_owned(), |h| h.label.clone());
        let position = (change.lamport, peer, change.id.counter);
        if let Some(event) = event_of(header.as_ref(), step) {
            self.machines
                .entry(peer)
                .or_insert_with(|| StackMachine::new(usize::MAX))
                .apply(event);
        }
        if label == LABEL_WIPE && self.wiped_at.is_none_or(|at| at < position) {
            self.wiped_at = Some(position);
        }
        if let Some((target, mark)) = mark_of(header.as_ref(), step) {
            self.marks.entry(target).or_default().push(mark);
        }
        self.positions.insert(step, position);
        self.by_peer
            .entry(peer)
            .or_default()
            .insert(change.id.counter, position);
        self.shown_stale = true;
        self.steps.insert(
            position,
            LogStep {
                step,
                header,
                label,
                message: message.to_owned(),
                end: next,
                before: change.deps.clone(),
            },
        );
        self.newest.insert(peer, position);
    }

    /// The step with this identity.
    pub(crate) fn get(&self, step: StepRef) -> Option<&LogStep> {
        self.steps.get(self.positions.get(&step)?)
    }

    /// Where the step sits in the order of the log.
    pub(crate) fn position(&self, step: StepRef) -> Option<Position> {
        self.positions.get(&step).copied()
    }

    /// The step that holds the operation `counter` of `peer`.
    pub(crate) fn step_at(&self, peer: u64, counter: i32) -> Option<StepRef> {
        let (_, position) = self.by_peer.get(&peer)?.range(..=counter).next_back()?;
        let step = self.steps.get(position)?;
        (counter < step.end).then_some(step.step)
    }

    /// The object undos and redos that name `step`, oldest first.
    pub(crate) fn marks_on(&self, step: StepRef) -> &[Mark] {
        self.marks.get(&step).map_or(&[], Vec::as_slice)
    }

    /// Makes [`StepIndex::shown`] current: the shown id of every row step.
    pub(crate) fn ensure_shown(&mut self) {
        if !self.shown_stale {
            return;
        }
        let all: Vec<StepRef> = self
            .steps()
            .filter(|step| is_row(step))
            .map(|step| step.step)
            .collect();
        self.shown = all.iter().copied().zip(shown_ids(&all)).collect();
        self.shown_stale = false;
    }

    /// The shown ids, as of the last [`StepIndex::ensure_shown`].
    pub(crate) fn shown(&self) -> &HashMap<StepRef, String> {
        &self.shown
    }

    /// All steps, oldest first.
    pub(crate) fn steps(&self) -> impl DoubleEndedIterator<Item = &LogStep> {
        match self.wiped_at {
            Some(at) => self.steps.range(at..),
            None => self.steps.range(..),
        }
        .map(|(_, step)| step)
    }

    /// The stacks of `peer` as its commits tell them.
    pub(crate) fn machine(&self, peer: u64) -> StackMachine {
        self.machines
            .get(&peer)
            .cloned()
            .unwrap_or_else(|| StackMachine::new(usize::MAX))
    }

    /// The branch a step's own peer left it on, while it is on one.
    pub(crate) fn branch_of(&self, step: StepRef) -> Option<BranchMark> {
        self.machines.get(&step.peer())?.branch_of(step)
    }

    /// The status of a step.
    pub(crate) fn status(&self, step: StepRef) -> Status {
        self.machines
            .get(&step.peer())
            .map_or(Status::Applied, |machine| machine.status(step))
    }
}

/// The step an object undo or redo names, with the mark it leaves on it.
fn mark_of(header: Option<&StepHeader>, step: StepRef) -> Option<(StepRef, Mark)> {
    let header = header?;
    let redo = match header.label.as_str() {
        LABEL_OBJECT_UNDO => false,
        LABEL_OBJECT_REDO => true,
        _ => return None,
    };
    let key = if redo {
        KEY_OBJECT_REDONE
    } else {
        KEY_OBJECT_UNDONE
    };
    let target = header.extra_value(key).and_then(StepRef::decode)?;
    let object = header.extra_value(KEY_OBJECT).and_then(decode_object);
    Some((
        target,
        Mark {
            by: step,
            redo,
            object,
        },
    ))
}

/// The object text of a header: `<peer>:<counter>`.
pub(crate) fn decode_object(text: &str) -> Option<loro::TreeID> {
    let (peer, counter) = text.split_once(':')?;
    Some(loro::TreeID::new(peer.parse().ok()?, counter.parse().ok()?))
}

/// The event a step is for the stacks of its peer.
fn event_of(header: Option<&StepHeader>, step: StepRef) -> Option<Event> {
    let Some(header) = header else {
        return Some(Event::Do(step));
    };
    let target = |key: &str| header.extra_value(key).and_then(StepRef::decode);
    match header.label.as_str() {
        LABEL_NEW_DOCUMENT | LABEL_WIPE | LABEL_WIPE_OBJECT => None,
        LABEL_UNDO => target(KEY_UNDONE).map(Event::Undo),
        LABEL_REDO => target(KEY_REDONE).map(Event::Redo),
        _ => Some(Event::Do(step)),
    }
}

/// Whether a step is a row of the history list: the setup of a new document, an undo and a redo
/// are not.
pub(crate) fn is_row(step: &LogStep) -> bool {
    !matches!(
        step.label.as_str(),
        LABEL_NEW_DOCUMENT | LABEL_UNDO | LABEL_REDO
    ) || step.header.is_none()
}

impl crate::document::Document {
    /// The stacks and step statuses of `peer` as its commits in the log tell them (the history
    /// list's source, `specs/0020-undo-redo` criterion 55), without a limit on the stacks.
    #[must_use]
    pub fn stacks_in_log(&self, peer: u64) -> StackMachine {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        let mut index = self.step_index.borrow_mut();
        index.extend(doc);
        index.machine(peer)
    }
}
