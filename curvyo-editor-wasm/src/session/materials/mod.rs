//! The maker's material library inside a session (`specs/0112-test-grid-and-materials` criteria
//! 16 to 31; ADR 0055 §7, §9; ADR 0058): records shared by all lasers, retired test ids and photo
//! bytes, with the Materials tab's view state. Each record edit is one document step of the open
//! project whose record part the undo journal keeps.

mod commands;
mod refresh;
mod step;
mod test_records;
#[cfg(test)]
pub(in crate::session) mod tests;
mod undo;
mod view_json;
mod view_types;

use std::collections::{BTreeMap, BTreeSet};

use curvyo_laser_core::LibraryWrite;
use curvyo_laser_core::grids::TestId;
use curvyo_laser_core::library::{Library, PhotoWrite, RetiredIds, read_retired};
use curvyo_laser_core::settings::MachineTie;
use serde::Serialize;

use super::Session;

pub(in crate::session) use step::LibraryChange;
pub use view_types::{MaterialsView, NumbersView, SetKindView};

/// One write the host performs for the library (the host write list of `adrs.md` finding 3), or
/// the new text of a preference file.
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
    /// The new text of `retired-test-ids.json`.
    Retired {
        /// The file text.
        text: String,
    },
    /// Write the photo file `name`; its bytes are taken with
    /// [`Session::take_material_photo_bytes`] by `handle`.
    WritePhoto {
        /// `<material id>.<test id>.jpg|png`.
        name: String,
        /// The byte handle.
        handle: u32,
    },
    /// Remove the photo file `name`.
    RemovePhoto {
        /// `<material id>.<test id>.jpg|png`.
        name: String,
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

/// The Materials tab's view state: never a step (UX notes 11).
#[derive(Debug, Default)]
pub(in crate::session) struct TabState {
    /// The unfolded material.
    unfolded: Option<String>,
    /// The open test of the unfolded material.
    open_test: Option<TestId>,
    /// The lasers folded open besides the page's, by profile id.
    open_lasers: BTreeSet<String>,
    /// Whether the page's laser block is folded.
    page_laser_folded: bool,
    /// Lasers picked by "+ Sets for another laser" that have no set yet, per material.
    picked_lasers: BTreeMap<String, Vec<MachineTie>>,
    /// Whether the "+ Sets for another laser" list is unfolded.
    laser_pick_open: bool,
    /// Whether the New material form is open.
    form_open: bool,
    /// The grid whose bar asked for a new material (UX G9): Add picks it for that grid.
    form_for_grid: Option<curvyo_document_core::NodeId>,
}

/// What the session keeps of the maker's materials: the library, the retired ids, the journal of
/// this session's record edits, the writes the host has not taken, the photo bytes it knows and
/// the tab's view state.
#[derive(Debug, Default)]
pub(in crate::session) struct MaterialsState {
    library: Library,
    retired: RetiredIds,
    retired_notice: Option<&'static str>,
    journal: undo::Journal,
    outbox: Vec<MaterialWrite>,
    /// Bytes of the photo writes the host has not taken, by handle.
    handles: BTreeMap<u32, Vec<u8>>,
    next_handle: u32,
    /// The photo bytes this session knows, by file name (written here or handed in by the host).
    photos: BTreeMap<String, Vec<u8>>,
    tab: TabState,
    notice: Option<String>,
}

impl MaterialsState {
    /// Queues a host write (the grid choice of `test_grid/` comes here too).
    pub(in crate::session) fn queue(&mut self, write: MaterialWrite) {
        self.outbox.push(write);
    }

    /// The library as loaded and edited.
    pub(in crate::session) const fn library(&self) -> &Library {
        &self.library
    }

    /// Whether `id` is taken: a live test id of the library or a retired one (ADR 0058 §4).
    pub(in crate::session) fn id_taken(&self, id: TestId) -> bool {
        self.retired.contains(&id) || self.library.test_ids().any(|live| live == id)
    }

    /// Queues the host side of a photo write.
    fn queue_photo(&mut self, write: PhotoWrite) {
        match write {
            PhotoWrite::Write { name, bytes } => {
                let handle = self.next_handle;
                self.next_handle = self.next_handle.wrapping_add(1);
                self.photos.insert(name.clone(), bytes.clone());
                self.handles.insert(handle, bytes);
                self.outbox.push(MaterialWrite::WritePhoto { name, handle });
            }
            PhotoWrite::Remove { name } => {
                self.photos.remove(&name);
                self.outbox.push(MaterialWrite::RemovePhoto { name });
            }
        }
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
/// notes §13 of `0028`).
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
        self.materials.library = Library::load(files, order);
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

    /// Reads `retired-test-ids.json` (`None`: absent); a damaged text reads as empty with one
    /// notice and is never rewritten by the read (ADR 0058 §4).
    pub fn load_retired_test_ids(&mut self, text: Option<&str>) {
        let (retired, notice) = read_retired(text);
        self.materials.retired = retired;
        self.materials.retired_notice = notice;
    }

    /// Hands in the bytes of the photo file `name` the host read to show it, so an undo of a
    /// later photo step can write them back (ADR 0058 §8).
    pub fn load_material_photo(&mut self, name: &str, bytes: &[u8]) {
        self.materials
            .photos
            .insert(name.to_owned(), bytes.to_vec());
    }

    /// The host writes made since the last call, in order, as JSON.
    pub fn take_material_writes_json(&mut self) -> String {
        let writes = std::mem::take(&mut self.materials.outbox);
        serde_json::to_string(&writes).unwrap_or_else(|_| "[]".to_owned())
    }

    /// The bytes of the photo write `handle`, once.
    pub fn take_material_photo_bytes(&mut self, handle: u32) -> Option<Vec<u8>> {
        self.materials.handles.remove(&handle)
    }

    /// The notice of the last material command, once.
    pub fn take_materials_notice(&mut self) -> Option<String> {
        self.materials.notice.take()
    }

    /// The library.
    #[must_use]
    pub const fn materials(&self) -> &Library {
        &self.materials.library
    }

    /// The profile id of the active laser page's copy.
    pub(in crate::session) fn active_profile_id(&self) -> Option<String> {
        self.active_copy()?.read.copy.id
    }

    /// The page's laser as a material names it, `None` without a laser page or for a copy that
    /// names no profile.
    pub(in crate::session) fn active_laser(&self) -> Option<MachineTie> {
        let copy = self.active_copy()?;
        Some(MachineTie {
            id: copy.read.copy.id.clone()?,
            name: copy.read.copy.values.name.clone(),
        })
    }
}
