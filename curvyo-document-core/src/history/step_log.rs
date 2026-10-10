//! Reading steps out of the operation log: a step is all commits of one peer with the same
//! message header, which Loro may have merged into one change or split into several (ADR 0014 §2).

use loro::{Frontiers, IdSpan, LoroDoc};

use super::stack_machine::Event;
use super::step_header::{
    KEY_REDONE, KEY_UNDONE, LABEL_NEW_DOCUMENT, LABEL_REDO, LABEL_UNDO, LABEL_WIPE, StepHeader,
};
use super::step_id::StepRef;

/// One step as the log holds it.
#[derive(Debug, Clone)]
pub(crate) struct LogStep {
    /// The step's identity.
    pub step: StepRef,
    /// The parsed header; `None` for a legacy message.
    pub header: Option<StepHeader>,
    /// The commit label: the header's label, or the whole legacy message.
    pub label: String,
    /// One past the counter of the step's last operation.
    pub end: i32,
    /// The version the step started from: the dependencies of its first change.
    pub before: Frontiers,
    /// The Lamport clock of the step's first operation.
    pub lamport: u32,
}

impl LogStep {
    /// The version right after the step: its last operation and everything it depends on.
    pub(crate) fn after(&self) -> Frontiers {
        Frontiers::from(loro::ID::new(self.step.peer(), self.end - 1))
    }

    /// The operations of the step.
    pub(crate) fn span(&self) -> IdSpan {
        IdSpan::new(self.step.peer(), self.step.counter(), self.end)
    }

    /// The number of operations in the step.
    pub(crate) fn len(&self) -> usize {
        usize::try_from(self.end - self.step.counter()).unwrap_or(0)
    }
}

/// Finds the step that starts at `step` in the log.
pub(crate) fn find_step(doc: &LoroDoc, step: StepRef) -> Option<LogStep> {
    let first = doc.get_change(step.id())?;
    if first.id.counter != step.counter() {
        return None;
    }
    let message = first.message.as_deref().unwrap_or_default().to_owned();
    let mut end = first.id.counter + i32::try_from(first.len).ok()?;
    while let Some(next) = doc.get_change(loro::ID::new(step.peer(), end)) {
        if next.id.counter != end || next.message.as_deref().unwrap_or_default() != message {
            break;
        }
        end += i32::try_from(next.len).ok()?;
    }
    let header = StepHeader::parse(&message);
    let label = header
        .as_ref()
        .map_or_else(|| message.clone(), |h| h.label.clone());
    Some(LogStep {
        step,
        header,
        label,
        end,
        before: first.deps.clone(),
        lamport: first.lamport,
    })
}

/// The events of one peer's stacks as its commits in the log tell them, oldest first: a step is a
/// `Do`, an `undo` commit an `Undo` of the step it names, a `redo` commit a `Redo`. The reserved
/// setup and wipe commits are no events.
pub(crate) fn peer_events(doc: &LoroDoc, peer: u64) -> Vec<Event> {
    let mut events = Vec::new();
    let mut counter = 0;
    let mut previous: Option<String> = None;
    while let Some(change) = doc.get_change(loro::ID::new(peer, counter)) {
        let message = change.message.as_deref().unwrap_or_default().to_owned();
        let next = change.id.counter + i32::try_from(change.len).unwrap_or(i32::MAX);
        if previous.as_deref() != Some(message.as_str()) {
            if let Some(event) = event_of(&message, StepRef::new(peer, change.id.counter)) {
                events.push(event);
            }
        }
        previous = Some(message);
        counter = next;
    }
    events
}

fn event_of(message: &str, step: StepRef) -> Option<Event> {
    let Some(header) = StepHeader::parse(message) else {
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
