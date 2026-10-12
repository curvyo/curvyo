//! Reading steps out of the operation log: a step is all commits of one peer with the same
//! message header, which Loro may have merged into one change or split into several (ADR 0014 §2).

use loro::{Frontiers, IdSpan, LoroDoc};

use super::step_header::{StepHeader, continues};
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
    /// The commit message as written.
    pub message: String,
    /// One past the counter of the step's last operation.
    pub end: i32,
    /// The version the step started from: the dependencies of its first change.
    pub before: Frontiers,
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
    let mut message = first.message.as_deref().unwrap_or_default().to_owned();
    let mut end = first.id.counter + i32::try_from(first.len).ok()?;
    while let Some(next) = doc.get_change(loro::ID::new(step.peer(), end)) {
        let next_message = next.message.as_deref().unwrap_or_default();
        if next.id.counter != end || !continues(&message, next_message) {
            break;
        }
        // A go-to exploration names its latest version.
        next_message.clone_into(&mut message);
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
        message,
        end,
        before: first.deps.clone(),
    })
}
