//! The material write-back of an undo or redo (ADR 0055 §9, ADR 0058 §8): the records and photo a
//! journalled step changed are written back when the records are still what the step left.

use curvyo_document_core::Direction;
use curvyo_laser_core::library::PhotoWrite;

use super::super::Session;
use super::super::library_journal::{PhotoBytes, RecordTexts};
use super::{MaterialWrite, now_ms};

impl Session {
    /// Writes the material `records` and `photos` of an undone or redone step back, each record
    /// when it still is what the step left (`name` is the short name for the notice); `records`
    /// take the texts now on disk. The notice that replaces the undo notice when a record
    /// changed since.
    pub(in crate::session) fn revert_material_records(
        &mut self,
        direction: Direction,
        name: &str,
        records: &mut [RecordTexts],
        photos: &[PhotoBytes],
    ) -> Option<String> {
        let now = now_ms(self);
        let state = &mut self.materials;
        let done = match direction {
            Direction::Undo => "undone",
            Direction::Redo => "redone",
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
            let Ok(reverted) = state.library.get_mut().revert(
                &record.id,
                expected.as_deref(),
                earlier.as_deref(),
                now,
            ) else {
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
        None
    }
}
