//! The maker's materials as one list (ADR 0055 §3, §5): loaded from the record texts the host
//! read, filtered and ordered per machine; the record edits are `material_edits`.

use std::cmp::Ordering;

use serde_json::Value;

use crate::material::{Material, MaterialRead};
use crate::material_edits::LibraryRefusal;
use crate::material_file::{MAX_MATERIAL_BYTES, MaterialFileError, read_material};
use crate::material_preferences::{read_order, write_order};
use crate::record_envelope::{LibraryWrite, record_id};

/// At most this many records are read, in file-name order (ADR 0055 §3).
pub const MAX_MATERIALS: usize = 1_000;

/// One record as loaded: its read, its current text, and the machine and name a damaged or newer
/// record names when its JSON is readable.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Entry {
    pub(crate) read: MaterialRead,
    pub(crate) text: String,
    pub(crate) machine: Option<String>,
    pub(crate) name: Option<String>,
}

/// The library: every record that has a `<uuid>.json` name, the list order, and the notices of
/// the load.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Materials {
    pub(crate) entries: Vec<Entry>,
    reads: Vec<MaterialRead>,
    skipped: Vec<String>,
    order: Vec<String>,
    notices: Vec<String>,
}

impl Materials {
    /// The library from `(file name, text)` pairs and the text of `material-order.json`: names
    /// that are not `<uuid>.json` are ignored; a damaged or newer record is kept as such with one
    /// notice naming its file; records past [`MAX_MATERIALS`] are left unread and never written.
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
            let entry = entry_of(id, name, text.clone());
            match &entry.read {
                MaterialRead::Damaged { reason, .. } => {
                    library.notices.push(format!("{name}: {reason}"));
                }
                MaterialRead::Newer { .. } => {
                    library
                        .notices
                        .push(format!("{name}: needs a newer Curvyo."));
                }
                MaterialRead::Loaded(_) => {}
            }
            library.entries.push(entry);
        }
        library.sync_reads();
        library
    }

    /// Every record as the settings rule reads it (ADR 0054 §2).
    #[must_use]
    pub fn reads(&self) -> &[MaterialRead] {
        &self.reads
    }

    /// One notice per damaged or newer record of the load (spec criterion 29).
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

    /// The current text of the record `id`, what an undo journal keeps (ADR 0055 §9).
    #[must_use]
    pub fn text(&self, id: &str) -> Option<&str> {
        self.entry(id).map(|entry| entry.text.as_str())
    }

    /// The name of `id`: a readable material's, or the one a damaged or newer record's JSON names.
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

    /// The rows of the profile `profile_id` (ADR 0055 §5): readable materials of that machine and
    /// damaged or newer records naming it or no machine; listed ids first in their order, then
    /// the others by name ignoring case, ties by id.
    #[must_use]
    pub fn listed(&self, profile_id: &str) -> Vec<&MaterialRead> {
        let mut rows: Vec<(Option<usize>, String, &Entry)> = self
            .entries
            .iter()
            .filter(|entry| match &entry.read {
                MaterialRead::Loaded(material) => material.machine.id == profile_id,
                _ => entry.machine.as_deref().is_none_or(|id| id == profile_id),
            })
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

    /// Deletes the record `id`, a damaged one included (a user action, not a read).
    ///
    /// # Errors
    /// An unknown record, or one of a newer format.
    pub fn delete(&mut self, id: &str) -> Result<LibraryWrite, LibraryRefusal> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.read.id() == id)
            .ok_or(LibraryRefusal::Unknown)?;
        if matches!(self.entries[index].read, MaterialRead::Newer { .. }) {
            return Err(LibraryRefusal::Newer);
        }
        self.entries.remove(index);
        self.sync_reads();
        Ok(LibraryWrite::Remove { id: id.to_owned() })
    }

    /// Moves the row `id` of the profile `profile_id` to `index` of its rows and returns the text
    /// of `material-order.json`: this machine's rows in their new order, then the listed ids of
    /// other records; ids no record has are dropped by this write (ADR 0055 §5). `None` when `id`
    /// is not a row of the profile.
    pub fn move_row(&mut self, profile_id: &str, id: &str, index: usize) -> Option<String> {
        let mut rows: Vec<String> = self
            .listed(profile_id)
            .iter()
            .map(|read| read.id().to_owned())
            .collect();
        let from = rows.iter().position(|row| row == id)?;
        let moved = rows.remove(from);
        rows.insert(index.min(rows.len()), moved);
        let others: Vec<String> = self
            .order
            .iter()
            .filter(|listed| !rows.contains(listed) && self.entry(listed).is_some())
            .cloned()
            .collect();
        rows.extend(others);
        self.order = rows;
        Some(write_order(&self.order))
    }

    pub(crate) fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.read.id() == id)
    }

    pub(crate) fn is_skipped(&self, id: &str) -> bool {
        self.skipped.iter().any(|skipped| skipped == id)
    }

    pub(crate) fn sync_reads(&mut self) {
        self.reads = self
            .entries
            .iter()
            .map(|entry| entry.read.clone())
            .collect();
    }
}

/// The entry of the record `id` with `text`, read as the file `name`.
pub(crate) fn entry_of(id: &str, name: &str, text: String) -> Entry {
    let read = match read_material(name, &text) {
        Ok(material) => MaterialRead::Loaded(material),
        Err(MaterialFileError::Damaged(reason)) => MaterialRead::Damaged {
            id: id.to_owned(),
            reason,
        },
        Err(MaterialFileError::Newer) => MaterialRead::Newer { id: id.to_owned() },
    };
    let (machine, name) = match &read {
        MaterialRead::Loaded(_) => (None, None),
        _ => hints(&text),
    };
    Entry {
        read,
        text,
        machine,
        name,
    }
}

/// The machine id and name an unreadable record's JSON names, if it parses.
fn hints(text: &str) -> (Option<String>, Option<String>) {
    if text.len() > MAX_MATERIAL_BYTES {
        return (None, None);
    }
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return (None, None);
    };
    let text_at = |pointer: &str| {
        value
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    (text_at("/machine/id"), text_at("/name"))
}

#[cfg(test)]
#[path = "materials_tests.rs"]
mod tests;
