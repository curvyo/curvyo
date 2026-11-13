//! The record part of this session's material steps (ADR 0055 §9, ADR 0058 §8): per step the
//! texts of the records it changed and the bytes of the photo it changed, in memory only, at most
//! 16 MiB; Ctrl+Z and Redo write the other side back when the record is still what the step left.

use std::collections::VecDeque;

use curvyo_document_core::{Direction, StepRef};
use curvyo_laser_core::library::PhotoWrite;

use super::super::Session;
use super::{MaterialWrite, now_ms, short_name};

/// The most record text and photo bytes the journal keeps (ADR 0055 §9 (e)).
pub(in crate::session) const JOURNAL_LIMIT_BYTES: usize = 16 * 1024 * 1024;

/// One record a step changed: its texts around the step (`None`: absent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) struct RecordTexts {
    /// The record id.
    pub(in crate::session) id: String,
    /// The material's name, for the notices.
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

/// One step's record part, or `None` once dropped for the limit.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    step: StepRef,
    name: String,
    kept: Option<(Vec<RecordTexts>, Vec<PhotoBytes>)>,
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

/// The journal of this session's material steps, oldest first.
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
        records: Vec<RecordTexts>,
        photos: Vec<PhotoBytes>,
    ) {
        self.entries.retain(|entry| entry.step != step);
        let name = records.first().map(|r| r.name.clone()).unwrap_or_default();
        self.entries.push_back(Entry {
            step,
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

    fn recount(&mut self) {
        self.bytes = self.entries.iter().map(Entry::bytes).sum();
    }

    fn entry_mut(&mut self, step: StepRef) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|entry| entry.step == step)
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
    /// written back when it still is what the step left, and the photo with it; else, or when
    /// the part was dropped, the notice that replaces the undo notice. `None` for a step without
    /// a record part.
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
        let Some((records, photos)) = entry.kept.as_mut() else {
            return Some(format!("Only the project was {done}; {name} is too old."));
        };
        let undo = direction == Direction::Undo;
        // Every record must still be what the step left, or nothing is written back.
        let current = records.iter().all(|r| {
            let expected = if undo { &r.after } else { &r.before };
            state.library.text(&r.id) == expected.as_deref()
        });
        if !current {
            return Some(format!(
                "{name} changed since; the material was not {done}."
            ));
        }
        for record in records.iter_mut() {
            let (expected, earlier) = if undo {
                (record.after.clone(), record.before.clone())
            } else {
                (record.before.clone(), record.after.clone())
            };
            let Ok(reverted) =
                state
                    .library
                    .revert(&record.id, expected.as_deref(), earlier.as_deref(), now)
            else {
                continue;
            };
            // The text now on disk is what the next undo or redo of this step expects.
            if undo {
                record.before = reverted.text;
            } else {
                record.after = reverted.text;
            }
            if let Some(write) = reverted.write {
                state.outbox.push(MaterialWrite::from(write));
            }
        }
        let photo_writes: Vec<PhotoWrite> = photos
            .iter()
            .map(|photo| {
                let bytes = if undo { &photo.before } else { &photo.after };
                match bytes {
                    Some(bytes) => PhotoWrite::Write {
                        name: photo.name.clone(),
                        bytes: bytes.clone(),
                    },
                    None => PhotoWrite::Remove {
                        name: photo.name.clone(),
                    },
                }
            })
            .collect();
        for write in photo_writes {
            state.queue_photo(write);
        }
        state.journal.recount();
        None
    }

    /// The journal (tests).
    #[cfg(test)]
    pub(in crate::session) fn material_journal(&mut self) -> &mut Journal {
        &mut self.materials.journal
    }
}
