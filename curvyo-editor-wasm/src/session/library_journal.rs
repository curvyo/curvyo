//! The record part of this session's library steps (ADR 0055 §9, ADR 0058 §8, ADR 0060 §5a): per
//! step the texts of the material or machine records it changed and the bytes of the photo it
//! changed, in memory only, at most 16 MiB together.

use std::collections::VecDeque;

use curvyo_document_core::{Direction, StepRef};

use super::Session;

/// The most record text and photo bytes the journal keeps (ADR 0055 §9 (e)).
pub(in crate::session) const JOURNAL_LIMIT_BYTES: usize = 16 * 1024 * 1024;

/// A material or profile name as the undo notices write it: at most 24 characters, else cut with
/// "…" (UX notes §13 of `0028`).
pub(in crate::session) fn short_name(name: &str) -> String {
    if name.chars().count() <= 24 {
        return name.to_owned();
    }
    let cut: String = name.chars().take(23).collect();
    format!("{cut}…")
}

/// Which library a record of the journal belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum RecordKind {
    /// A material record (`0112`).
    Material,
    /// A machine profile record whose sets a Fixtures tab edit changed (`0113` criterion 45).
    Machine,
    /// A machine record of any family a Machines tab edit changed (ADR 0098 decision 4).
    MachineEdit,
}

/// One record a step changed: its texts around the step (`None`: absent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) struct RecordTexts {
    /// The record id.
    pub(in crate::session) id: String,
    /// The material's or the profile's name, for the notices.
    pub(in crate::session) name: String,
    /// The text before the step.
    pub(in crate::session) before: Option<String>,
    /// The text after it.
    pub(in crate::session) after: Option<String>,
}

/// One photo file a step changed: its bytes around the step (`None`: absent or unknown).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) struct PhotoBytes {
    /// Its file name.
    pub(in crate::session) name: String,
    /// The bytes before the step.
    pub(in crate::session) before: Option<Vec<u8>>,
    /// The bytes after it.
    pub(in crate::session) after: Option<Vec<u8>>,
}

/// The record part of one step: its records and photos, `None` once dropped for the limit.
type Kept = Option<(Vec<RecordTexts>, Vec<PhotoBytes>)>;

/// One step's record part.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    step: StepRef,
    kind: RecordKind,
    name: String,
    kept: Kept,
}

fn opt_len<T: AsRef<[u8]>>(value: Option<&T>) -> usize {
    value.map_or(0, |v| v.as_ref().len())
}

impl Entry {
    fn bytes(&self) -> usize {
        self.kept.as_ref().map_or(0, |(records, photos)| {
            records
                .iter()
                .map(|r| opt_len(r.before.as_ref()) + opt_len(r.after.as_ref()))
                .sum::<usize>()
                + photos
                    .iter()
                    .map(|p| opt_len(p.before.as_ref()) + opt_len(p.after.as_ref()))
                    .sum::<usize>()
        })
    }
}

/// The journal of this session's library steps, oldest first.
#[derive(Debug, Default)]
pub(in crate::session) struct Journal {
    entries: VecDeque<Entry>,
    bytes: usize,
    limit: Option<usize>,
}

impl Journal {
    /// Keeps the record part of the step `step`; the oldest parts go first past the limit.
    pub(in crate::session) fn record(
        &mut self,
        step: StepRef,
        kind: RecordKind,
        records: Vec<RecordTexts>,
        photos: Vec<PhotoBytes>,
    ) {
        self.entries.retain(|entry| entry.step != step);
        let name = records.first().map(|r| r.name.clone()).unwrap_or_default();
        self.entries.push_back(Entry {
            step,
            kind,
            name,
            kept: Some((records, photos)),
        });
        self.recount();
        let limit = self.limit.unwrap_or(JOURNAL_LIMIT_BYTES);
        for entry in &mut self.entries {
            if self.bytes <= limit {
                break;
            }
            let freed = entry.bytes();
            entry.kept = None;
            self.bytes -= freed;
        }
    }

    /// Forgets the record part of `step` (a step whose record write failed and was undone).
    pub(in crate::session) fn forget(&mut self, step: StepRef) {
        self.entries.retain(|entry| entry.step != step);
        self.recount();
    }

    /// The machine record a step of this journal wrote, with its name and its text before.
    pub(in crate::session) fn machine_record(&self, step: StepRef) -> Option<&RecordTexts> {
        let entry = self.entries.iter().find(|entry| entry.step == step)?;
        (entry.kind == RecordKind::Machine)
            .then(|| entry.kept.as_ref()?.0.first())
            .flatten()
    }

    fn recount(&mut self) {
        self.bytes = self.entries.iter().map(Entry::bytes).sum();
    }

    /// The bytes the journal holds (tests).
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
    /// The record part of an undo or redo of `step` whose document part was done: each record is
    /// written back when it still is what the step left (a material's photo with it); else, or
    /// when the part was dropped, the notice that replaces the undo notice. `None` for a step
    /// without a record part.
    pub(in crate::session) fn library_history_followup(
        &mut self,
        direction: Direction,
        step: StepRef,
    ) -> Option<String> {
        let entry = self
            .library_journal
            .entries
            .iter_mut()
            .find(|entry| entry.step == step)?;
        let name = short_name(&entry.name);
        let kind = entry.kind;
        let Some((mut records, photos)) = entry.kept.take() else {
            let done = match direction {
                Direction::Undo => "undone",
                Direction::Redo => "redone",
            };
            return Some(match kind {
                RecordKind::Material | RecordKind::MachineEdit => {
                    format!("Only the project was {done}; {name} is too old.")
                }
                RecordKind::Machine => {
                    self.align_fixture_pages_after_history();
                    format!("{name} sets can no longer be undone.")
                }
            });
        };
        let notice = match kind {
            RecordKind::Material => {
                self.revert_material_records(direction, &name, &mut records, &photos)
            }
            RecordKind::Machine => self.revert_machine_records(direction, &name, &mut records),
            RecordKind::MachineEdit => {
                self.revert_machine_edit_records(direction, &name, &mut records)
            }
        };
        if let Some(entry) = self
            .library_journal
            .entries
            .iter_mut()
            .find(|entry| entry.step == step)
        {
            entry.kept = Some((records, photos));
        }
        self.library_journal.recount();
        if kind == RecordKind::Machine {
            self.align_fixture_pages_after_history();
        }
        notice
    }

    /// The journal (tests).
    #[cfg(test)]
    pub(in crate::session) fn library_journal(&mut self) -> &mut Journal {
        &mut self.library_journal
    }
}
