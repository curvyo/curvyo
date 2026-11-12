//! The record part of this session's material steps (ADR 0055 §9): per step the material's text
//! before and after it, in memory only, at most 16 MiB; Ctrl+Z and Redo write the other text back
//! when the record is still what the step left.

use std::collections::VecDeque;

use curvyo_document_core::{Direction, StepRef};

use super::super::Session;
use super::{MaterialWrite, now_ms, short_name};

/// The most record text the journal keeps (ADR 0055 §9 (e)).
pub(in crate::session) const JOURNAL_LIMIT_BYTES: usize = 16 * 1024 * 1024;

/// One material step: the record texts around it (`None`: absent), or `None` once dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    step: StepRef,
    id: String,
    name: String,
    texts: Option<(Option<String>, Option<String>)>,
}

impl Entry {
    fn bytes(&self) -> usize {
        self.texts.as_ref().map_or(0, |(before, after)| {
            before.as_ref().map_or(0, String::len) + after.as_ref().map_or(0, String::len)
        })
    }
}

/// The journal of this session's material steps, oldest first.
#[derive(Debug, Default)]
pub(in crate::session) struct Journal {
    entries: VecDeque<Entry>,
    bytes: usize,
    limit: Option<usize>,
}

impl Journal {
    /// Keeps the texts of the step `step` on material `id` named `name`; the oldest texts go
    /// first past the limit.
    pub(in crate::session) fn record(
        &mut self,
        step: StepRef,
        id: &str,
        name: &str,
        before: Option<String>,
        after: Option<String>,
    ) {
        self.entries.retain(|entry| entry.step != step);
        let entry = Entry {
            step,
            id: id.to_owned(),
            name: name.to_owned(),
            texts: Some((before, after)),
        };
        self.entries.push_back(entry);
        self.recount();
        let limit = self.limit.unwrap_or(JOURNAL_LIMIT_BYTES);
        for entry in &mut self.entries {
            if self.bytes <= limit {
                break;
            }
            let freed = entry.bytes();
            entry.texts = None;
            self.bytes -= freed;
        }
    }

    fn recount(&mut self) {
        self.bytes = self.entries.iter().map(Entry::bytes).sum();
    }

    fn entry_mut(&mut self, step: StepRef) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|entry| entry.step == step)
    }

    /// The record text the journal holds (tests).
    #[cfg(test)]
    pub(in crate::session) fn bytes(&self) -> usize {
        self.bytes
    }

    /// A smaller limit (tests of the 16 MiB rule).
    #[cfg(test)]
    pub(in crate::session) fn set_limit(&mut self, limit: usize) {
        self.limit = Some(limit);
    }
}

impl Session {
    /// The record part of an undo or redo of `step` whose document part was done: the record is
    /// written back when it still is what the step left; else, or when its texts were dropped,
    /// the notice that replaces the undo notice. `None` for a step that is no material step.
    pub(in crate::session) fn material_history_followup(
        &mut self,
        direction: Direction,
        step: StepRef,
    ) -> Option<String> {
        let now = now_ms(self);
        let state = &mut self.materials;
        let entry = state.journal.entry_mut(step)?;
        let name = short_name(&entry.name);
        let done = match direction {
            Direction::Undo => "undone",
            Direction::Redo => "redone",
        };
        let Some((before, after)) = entry.texts.as_mut() else {
            return Some(format!("Only the project was {done}; {name} is too old."));
        };
        let (expected, earlier) = match direction {
            Direction::Undo => (after.clone(), before.clone()),
            Direction::Redo => (before.clone(), after.clone()),
        };
        let id = entry.id.clone();
        match state
            .library
            .revert(&id, expected.as_deref(), earlier.as_deref(), now)
        {
            Ok(reverted) => {
                // The text now on disk is what the next undo or redo of this step expects.
                match direction {
                    Direction::Undo => *before = reverted.text,
                    Direction::Redo => *after = reverted.text,
                }
                state.journal.recount();
                if let Some(write) = reverted.write {
                    state.outbox.push(MaterialWrite::from(write));
                }
                None
            }
            Err(_) => Some(format!(
                "{name} changed since; the material was not {done}."
            )),
        }
    }

    /// The journal (tests).
    #[cfg(test)]
    pub(in crate::session) fn material_journal(&mut self) -> &mut Journal {
        &mut self.materials.journal
    }
}
