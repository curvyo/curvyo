//! The maker's material library inside a session (`specs/0028-material-test-library` criteria 15
//! to 23, 29, 32; ADR 0055 §7, §9). The library is per user and never in a project; each record
//! edit is one document step of the open project, whose record part the undo journal keeps.

mod commands;
mod refresh;
#[cfg(test)]
mod tests;
mod undo;
mod view_json;

use curvyo_laser_core::LibraryWrite;
use curvyo_laser_core::material_library::Materials;
use serde::Serialize;

use super::Session;

pub use commands::SaveAsMaterial;
pub use view_json::{MaterialsView, NumbersView, SetKindView};

/// One write the host performs for the library (the host write list of `adrs.md` §6), or the new
/// text of a preference file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum MaterialWrite {
    /// Replace the record `id` with `text`.
    Write {
        /// The record id.
        id: String,
        /// The record text.
        text: String,
    },
    /// Remove the record `id`.
    Remove {
        /// The record id.
        id: String,
    },
    /// The new text of `material-order.json`.
    Order {
        /// The file text.
        text: String,
    },
    /// The new text of `test-grid-choice.json`.
    GridChoice {
        /// The file text.
        text: String,
    },
}

impl From<LibraryWrite> for MaterialWrite {
    fn from(write: LibraryWrite) -> Self {
        match write {
            LibraryWrite::Write { id, text } => Self::Write { id, text },
            LibraryWrite::Remove { id } => Self::Remove { id },
        }
    }
}

/// What the session keeps of the maker's materials: the library, the journal of this session's
/// record edits, the writes the host has not taken, the selected row and the last notice.
#[derive(Debug, Default)]
pub(in crate::session) struct MaterialsState {
    library: Materials,
    journal: undo::Journal,
    outbox: Vec<MaterialWrite>,
    selected: Option<String>,
    notice: Option<String>,
}

impl MaterialsState {
    /// Queues a host write (the grid choice of `test_grid/` comes here too).
    pub(in crate::session) fn queue(&mut self, write: MaterialWrite) {
        self.outbox.push(write);
    }

    /// The library as loaded and edited.
    pub(in crate::session) const fn library(&self) -> &Materials {
        &self.library
    }
}

/// The history clock in Unix milliseconds: steps are stamped in seconds, `modified` rises anyway
/// (ADR 0055 §2, `next_modified`).
pub(in crate::session) fn now_ms(session: &Session) -> u64 {
    (session.history_clock())().as_secs().saturating_mul(1000)
}

/// The first 16 bytes of `random` from `offset`, zero-padded: the caller passes
/// `crypto.getRandomValues`.
pub(in crate::session) fn sixteen(random: &[u8], offset: usize) -> [u8; 16] {
    let mut bytes = [0; 16];
    for (slot, byte) in bytes.iter_mut().zip(random.iter().skip(offset)) {
        *slot = *byte;
    }
    bytes
}

/// A material name as the undo notices write it: at most 24 characters, else cut with "…" (UX
/// notes §13).
pub(in crate::session) fn short_name(name: &str) -> String {
    if name.chars().count() <= 24 {
        return name.to_owned();
    }
    let cut: String = name.chars().take(23).collect();
    format!("{cut}…")
}

impl Session {
    /// Reads the records the host listed, `(file name, text)` pairs, and the text of
    /// `material-order.json` (criterion 17). The journal of this session stays: an undo checks
    /// the record text it finds.
    pub fn load_materials(&mut self, files: &[(String, String)], order: Option<&str>) {
        self.materials.library = Materials::load(files, order);
    }

    /// Reads the records as the JSON array `[{"name", "text"}]`; a text that is not such an array
    /// loads no record.
    pub fn load_materials_json(&mut self, json: &str, order: Option<&str>) {
        #[derive(serde::Deserialize)]
        struct Entry {
            name: String,
            text: String,
        }
        let files: Vec<(String, String)> = serde_json::from_str::<Vec<Entry>>(json)
            .unwrap_or_default()
            .into_iter()
            .map(|entry| (entry.name, entry.text))
            .collect();
        self.load_materials(&files, order);
    }

    /// The host writes made since the last call, in order, as the JSON list of `adrs.md` §6.
    pub fn take_material_writes_json(&mut self) -> String {
        let writes = std::mem::take(&mut self.materials.outbox);
        serde_json::to_string(&writes).unwrap_or_else(|_| "[]".to_owned())
    }

    /// The notice of the last material command, once.
    pub fn take_materials_notice(&mut self) -> Option<String> {
        self.materials.notice.take()
    }

    /// The library.
    #[must_use]
    pub const fn materials(&self) -> &Materials {
        &self.materials.library
    }
}
