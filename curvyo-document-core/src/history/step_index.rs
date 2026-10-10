//! Every step in the operation log, in order, with the stacks each peer's commits tell: the source
//! of the history list (`specs/0020-undo-redo` criteria 32 to 38). Built from the log on first use
//! and extended by the changes committed or imported since.

use std::collections::{BTreeMap, HashMap};

use loro::{LoroDoc, VersionVector};

use super::stack_machine::{Event, StackMachine, Status};
use super::step_header::{
    KEY_REDONE, KEY_UNDONE, LABEL_NEW_DOCUMENT, LABEL_REDO, LABEL_UNDO, LABEL_WIPE, StepHeader,
};
use super::step_id::StepRef;
use super::step_log::LogStep;

/// Where a step sits in the order of the log: the Lamport clock of its first operation, then its
/// peer (ADR 0014 §6: the main-line order of concurrent steps), then its counter.
type Position = (u32, u64, i32);

/// The steps of the log and the stacks of every peer.
#[derive(Debug, Default)]
pub(crate) struct StepIndex {
    indexed: VersionVector,
    steps: BTreeMap<Position, LogStep>,
    /// The newest step of each peer, which the peer's next change may extend.
    newest: HashMap<u64, Position>,
    machines: HashMap<u64, StackMachine>,
}

impl StepIndex {
    /// Reads the changes committed or imported since the last call.
    pub(crate) fn extend(&mut self, doc: &LoroDoc) {
        let end = doc.oplog_vv();
        if end == self.indexed {
            return;
        }
        for (&peer, &last) in end.iter() {
            let mut counter = self.indexed.get(&peer).copied().unwrap_or(0);
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

    /// All steps, oldest first.
    pub(crate) fn steps(&self) -> impl DoubleEndedIterator<Item = &LogStep> {
        self.steps.values()
    }

    /// The stacks of `peer` as its commits tell them.
    pub(crate) fn machine(&self, peer: u64) -> StackMachine {
        self.machines
            .get(&peer)
            .cloned()
            .unwrap_or_else(|| StackMachine::new(usize::MAX))
    }

    /// The status of a step.
    pub(crate) fn status(&self, step: StepRef) -> Status {
        self.machines
            .get(&step.peer())
            .map_or(Status::Applied, |machine| machine.status(step))
    }
}

/// The event a step is for the stacks of its peer.
fn event_of(header: Option<&StepHeader>, step: StepRef) -> Option<Event> {
    let Some(header) = header else {
        return Some(Event::Do(step));
    };
    let target = |key: &str| header.extra_value(key).and_then(StepRef::decode);
    match header.label.as_str() {
        LABEL_NEW_DOCUMENT | LABEL_WIPE => None,
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
