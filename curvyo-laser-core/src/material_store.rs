//! The maker's format-2 material records as one list (ADR 0055 §3, §5; ADR 0058 §4, §6): loaded
//! from the texts the host read, ordered, and changed one record at a time.

use std::cmp::Ordering;

use serde_json::Value;

use crate::material::next_modified;
use crate::material_preferences::{read_order, write_order};
use crate::material_record::{Material, MaterialRead};
use crate::material_record_edits::RecordRefusal;
use crate::material_record_file::{MAX_RECORD_TEXT, load_record, write_record};
use crate::record_envelope::{LibraryWrite, record_file_name, record_id};
use crate::test_id::TestId;

/// At most this many records are read, in file-name order (ADR 0055 §3).
pub const MAX_MATERIALS: usize = 1_000;

/// Why a change of the library wrote nothing; the text is the notice.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreRefusal {
    /// No readable record has the id.
    #[error("The material was not found.")]
    Unknown,
    /// The record is of a newer format and is never written.
    #[error("The material needs a newer Curvyo.")]
    Newer,
    /// The record text would pass 512 KiB (ADR 0058 §6): the write is refused.
    #[error("Could not save {name}: it is over 512 KiB.")]
    TooLarge {
        /// The material's name.
        name: String,
    },
    /// The record edit refused.
    #[error(transparent)]
    Record(#[from] RecordRefusal),
}

/// Why an undo or redo of a library step left the record alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RevertRefused {
    /// The record changed since the step, or was left unread at load.
    #[error("The material changed since.")]
    Changed,
}

/// What an undo or redo wrote: the host write and the record's new text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reverted {
    /// The host write, `None` when nothing was to be written.
    pub write: Option<LibraryWrite>,
    /// The record's text now, `None` when it is absent.
    pub text: Option<String>,
}

/// One record as loaded: its read, its current text, and the name an unreadable record names.
#[derive(Debug, Clone, PartialEq)]
struct Entry {
    read: MaterialRead,
    text: String,
    name: Option<String>,
}

/// The library: every record that has a `<uuid>.json` name, the list order and the load
/// notices.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Library {
    entries: Vec<Entry>,
    reads: Vec<MaterialRead>,
    skipped: Vec<String>,
    order: Vec<String>,
    notices: Vec<String>,
}

/// The name an unreadable record's JSON names, if it parses.
fn name_hint(text: &str) -> Option<String> {
    if text.len() > MAX_RECORD_TEXT {
        return None;
    }
    let value = serde_json::from_str::<Value>(text).ok()?;
    value.get("name")?.as_str().map(str::to_owned)
}

fn entry_of(file_name: &str, text: String) -> Entry {
    let read = load_record(file_name, &text);
    let name = match &read {
        MaterialRead::Loaded(_) => None,
        _ => name_hint(&text),
    };
    Entry { read, text, name }
}

impl Library {
    /// The library from `(file name, text)` pairs and the text of `material-order.json`: names
    /// that are not `<uuid>.json` are ignored; a damaged, newer or older record is kept as such
    /// with one notice; records past [`MAX_MATERIALS`] are left unread and never written.
    #[must_use]
    pub fn load(files: &[(String, String)], order_text: Option<&str>) -> Self {
        let mut records: Vec<(&str, &String, &String)> = files
            .iter()
            .filter_map(|(name, text)| record_id(name).map(|id| (id, name, text)))
            .collect();
        records.sort_by(|a, b| a.1.cmp(b.1));
        let mut library = Self {
            order: order_text.map(read_order).unwrap_or_default(),
            ..Self::default()
        };
        for (index, (id, name, text)) in records.into_iter().enumerate() {
            if index >= MAX_MATERIALS {
                library.skipped.push(id.to_owned());
                continue;
            }
            let entry = entry_of(name, text.clone());
            let shown = entry.name.clone().unwrap_or_else(|| name.clone());
            match &entry.read {
                MaterialRead::Damaged { reason, .. } => {
                    library.notices.push(format!("{name}: {reason}"));
                }
                MaterialRead::Newer { .. } => {
                    library
                        .notices
                        .push(format!("{name}: needs a newer Curvyo."));
                }
                MaterialRead::Older { .. } => {
                    library
                        .notices
                        .push(format!("Material {shown} has an older format."));
                }
                MaterialRead::Loaded(_) => {}
            }
            library.entries.push(entry);
        }
        library.sync_reads();
        library
    }

    /// Every record as the settings rule reads it (ADR 0058 §7).
    #[must_use]
    pub fn reads(&self) -> &[MaterialRead] {
        &self.reads
    }

    /// One notice per damaged, newer or older record of the load.
    #[must_use]
    pub fn load_notices(&self) -> &[String] {
        &self.notices
    }

    /// The readable material `id`.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&Material> {
        match &self.entry(id)?.read {
            MaterialRead::Loaded(material) => Some(material),
            _ => None,
        }
    }

    /// The current text of the record `id`, what the undo journal keeps (ADR 0055 §9).
    #[must_use]
    pub fn text(&self, id: &str) -> Option<&str> {
        self.entry(id).map(|entry| entry.text.as_str())
    }

    /// The name of `id`: a readable material's, or the one an unreadable record's JSON names.
    #[must_use]
    pub fn name_of(&self, id: &str) -> Option<&str> {
        let entry = self.entry(id)?;
        match &entry.read {
            MaterialRead::Loaded(material) => Some(&material.name),
            _ => entry.name.as_deref(),
        }
    }

    /// The ids `material-order.json` lists, unknown ones included.
    #[must_use]
    pub fn order(&self) -> &[String] {
        &self.order
    }

    /// The rows of the Materials tab (ADR 0055 §5 without the machine filter, ADR 0058): listed
    /// ids first in their order, then the others by name ignoring case, ties by id.
    #[must_use]
    pub fn listed(&self) -> Vec<&MaterialRead> {
        let mut rows: Vec<(Option<usize>, String, &Entry)> = self
            .entries
            .iter()
            .map(|entry| {
                let id = entry.read.id();
                let place = self.order.iter().position(|listed| listed == id);
                let name = match &entry.read {
                    MaterialRead::Loaded(material) => material.name.to_lowercase(),
                    _ => entry.name.as_deref().unwrap_or_default().to_lowercase(),
                };
                (place, name, entry)
            })
            .collect();
        rows.sort_by(|a, b| match (a.0, b.0) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.1.cmp(&b.1).then_with(|| a.2.read.id().cmp(b.2.read.id())),
        });
        rows.into_iter().map(|(_, _, entry)| &entry.read).collect()
    }

    /// The live test ids of every loaded record (ADR 0058 §4).
    pub fn test_ids(&self) -> impl Iterator<Item = TestId> + '_ {
        crate::material_record::live_test_ids(&self.reads)
    }

    /// The readable material holding the test `id`.
    #[must_use]
    pub fn test_owner(&self, id: TestId) -> Option<&Material> {
        self.entries.iter().find_map(|entry| match &entry.read {
            MaterialRead::Loaded(material) if material.test(id).is_some() => Some(material),
            _ => None,
        })
    }

    /// Adds `material` as a new record.
    ///
    /// # Errors
    /// [`StoreRefusal::TooLarge`]; an id already taken reads as [`StoreRefusal::Unknown`].
    pub fn add(&mut self, material: Material) -> Result<LibraryWrite, StoreRefusal> {
        if self.entry(&material.id).is_some() {
            return Err(StoreRefusal::Unknown);
        }
        let text = checked_text(&material)?;
        let id = material.id.clone();
        self.entries.push(Entry {
            read: MaterialRead::Loaded(material),
            text: text.clone(),
            name: None,
        });
        self.sync_reads();
        Ok(LibraryWrite::Write { id, text })
    }

    /// Applies `change` to a copy of the readable material `id` and keeps it when the record
    /// text stays within 512 KiB (ADR 0058 §6).
    ///
    /// # Errors
    /// [`StoreRefusal::Unknown`], [`StoreRefusal::TooLarge`] or the record's refusal; nothing
    /// changes then.
    pub fn edit(
        &mut self,
        id: &str,
        change: impl FnOnce(&mut Material) -> Result<(), RecordRefusal>,
    ) -> Result<LibraryWrite, StoreRefusal> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.read.id() == id)
            .ok_or(StoreRefusal::Unknown)?;
        let MaterialRead::Loaded(material) = &self.entries[index].read else {
            return Err(StoreRefusal::Unknown);
        };
        let mut material = material.clone();
        change(&mut material)?;
        let text = checked_text(&material)?;
        self.entries[index] = Entry {
            read: MaterialRead::Loaded(material),
            text: text.clone(),
            name: None,
        };
        self.sync_reads();
        Ok(LibraryWrite::Write {
            id: id.to_owned(),
            text,
        })
    }

    /// Deletes the record `id`, a damaged or older one included (a user action, not a read);
    /// the material it held, so its tests' ids can be retired and its photos removed.
    ///
    /// # Errors
    /// An unknown record, or one of a newer format.
    pub fn delete(&mut self, id: &str) -> Result<(LibraryWrite, Option<Material>), StoreRefusal> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.read.id() == id)
            .ok_or(StoreRefusal::Unknown)?;
        if matches!(self.entries[index].read, MaterialRead::Newer { .. }) {
            return Err(StoreRefusal::Newer);
        }
        let entry = self.entries.remove(index);
        self.sync_reads();
        let material = match entry.read {
            MaterialRead::Loaded(material) => Some(material),
            _ => None,
        };
        Ok((LibraryWrite::Remove { id: id.to_owned() }, material))
    }

    /// Undo or redo of a library step (ADR 0055 §9): when the record `id` still has the text
    /// `expected` (`None`: absent), `earlier` is written back (`None`: removed) with a new
    /// `modified` from `now`; an unreadable earlier text goes back as it was.
    ///
    /// # Errors
    /// [`RevertRefused::Changed`] when the record changed since or was left unread at load.
    pub fn revert(
        &mut self,
        id: &str,
        expected: Option<&str>,
        earlier: Option<&str>,
        now: u64,
    ) -> Result<Reverted, RevertRefused> {
        if self.skipped.iter().any(|s| s == id) || self.text(id) != expected {
            return Err(RevertRefused::Changed);
        }
        let current_modified = self.find(id).map_or(0, |material| material.modified);
        let Some(earlier) = earlier else {
            if expected.is_none() {
                return Ok(Reverted {
                    write: None,
                    text: None,
                });
            }
            self.entries.retain(|entry| entry.read.id() != id);
            self.sync_reads();
            return Ok(Reverted {
                write: Some(LibraryWrite::Remove { id: id.to_owned() }),
                text: None,
            });
        };
        let name = record_file_name(id);
        let text = match load_record(&name, earlier) {
            MaterialRead::Loaded(mut material) => {
                material.modified = next_modified(current_modified.max(material.modified), now);
                write_record(&material)
            }
            _ => earlier.to_owned(),
        };
        let entry = entry_of(&name, text.clone());
        match self.entries.iter_mut().find(|e| e.read.id() == id) {
            Some(slot) => *slot = entry,
            None => self.entries.push(entry),
        }
        self.sync_reads();
        Ok(Reverted {
            write: Some(LibraryWrite::Write {
                id: id.to_owned(),
                text: text.clone(),
            }),
            text: Some(text),
        })
    }

    /// Puts the record `id` back to exactly `text` (`None`: absent), as it was before an edit
    /// whose step was refused; no host write follows.
    pub fn restore(&mut self, id: &str, text: Option<&str>) {
        self.entries.retain(|entry| entry.read.id() != id);
        if let Some(text) = text {
            self.entries
                .push(entry_of(&record_file_name(id), text.to_owned()));
        }
        self.sync_reads();
    }

    /// Moves the row `id` to `index` of the rows and returns the text of `material-order.json`:
    /// the rows in their new order; ids no record has are dropped by this write. `None` when `id`
    /// is no row.
    pub fn move_row(&mut self, id: &str, index: usize) -> Option<String> {
        let mut rows: Vec<String> = self
            .listed()
            .iter()
            .map(|read| read.id().to_owned())
            .collect();
        let from = rows.iter().position(|row| row == id)?;
        let moved = rows.remove(from);
        rows.insert(index.min(rows.len()), moved);
        self.order = rows;
        Some(write_order(&self.order))
    }

    fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.read.id() == id)
    }

    fn sync_reads(&mut self) {
        self.reads = self
            .entries
            .iter()
            .map(|entry| entry.read.clone())
            .collect();
    }
}

/// The record text of `material`, refused over 512 KiB.
fn checked_text(material: &Material) -> Result<String, StoreRefusal> {
    let text = write_record(material);
    if text.len() > MAX_RECORD_TEXT {
        return Err(StoreRefusal::TooLarge {
            name: material.name.clone(),
        });
    }
    Ok(text)
}

#[cfg(test)]
#[path = "material_store_tests.rs"]
mod tests;
