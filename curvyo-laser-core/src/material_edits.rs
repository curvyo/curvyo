//! The edits of the maker's material records (ADR 0055 §2, §4, §9): Save as material, set edits
//! and removals, rename, thickness and the undo and redo of a library step, each with the write
//! the host must do.

use crate::material::{
    CellIndex, HistoryEntry, HistoryEvent, MAX_HISTORY_ENTRIES, MAX_MATERIAL_NAME_CHARS,
    MachineTie, Material, MaterialRead, MaterialSet, SetKind, SetSource, Settings, Thickness,
    next_modified,
};
use crate::material_file::{MAX_MATERIAL_BYTES, read_material, write_material};
use crate::materials::{Entry, Materials, entry_of};
use crate::record_envelope::{LibraryWrite, is_record_id, record_file_name};
use crate::test_grid::GridRange;

/// Why a library edit wrote nothing; the text is the notice.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LibraryRefusal {
    /// No readable material has the id.
    #[error("The material was not found.")]
    Unknown,
    /// The id is taken, or names a record left as it was at load.
    #[error("A material with this id exists.")]
    Taken,
    /// A name outside 1 to 60 characters.
    #[error("Names are 1 to 60 characters.")]
    InvalidName,
    /// The edit changes nothing.
    #[error("Nothing changed.")]
    Unchanged,
    /// The material has no set of the kind.
    #[error("{name} has no {kind} set.")]
    NoSet {
        /// The material's name.
        name: String,
        /// The kind's word.
        kind: &'static str,
    },
    /// The edit would pass 500 history entries.
    #[error("{name} has 500 history entries; nothing was saved.")]
    FullHistory {
        /// The material's name.
        name: String,
    },
    /// The edit would pass 256 KiB of record text.
    #[error("{name} would be over 256 KiB; nothing was saved.")]
    TooLarge {
        /// The material's name.
        name: String,
    },
    /// A record of a newer format is never written or removed.
    #[error("The material needs a newer Curvyo.")]
    Newer,
}

/// Why an undo or redo left a record as it is (ADR 0055 §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RevertRefused {
    /// The record is no longer what the step left.
    #[error("changed since")]
    Changed,
}

/// The material a Save as material makes (spec criterion 15).
#[derive(Debug, Clone, PartialEq)]
pub struct NewMaterial {
    /// A fresh lowercase UUID v4.
    pub id: String,
    /// 1 to 60 characters.
    pub name: String,
    /// The thickness.
    pub thickness: Thickness,
    /// The page's machine profile.
    pub machine: MachineTie,
}

/// Where Save as material puts the test.
#[derive(Debug, Clone, PartialEq)]
pub enum SaveTarget {
    /// A new material.
    New(NewMaterial),
    /// The material with this id: its set of the test's kind is added or replaced.
    Existing(String),
}

/// A marked cell of a test grid, as Save as material keeps it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TestResult {
    /// The grid's kind, the set it makes.
    pub kind: SetKind,
    /// The grid's fields.
    pub grid: GridRange,
    /// The marked cell.
    pub cell: CellIndex,
    /// The numbers of the set (the cell's power and speed, the passes typed).
    pub set: Settings,
}

/// A record written back by an undo or redo, and the text the record now has (`None` when
/// removed), which the next undo or redo of that step expects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reverted {
    /// The host write; `None` when the record already is as asked.
    pub write: Option<LibraryWrite>,
    /// The record's text after the write.
    pub text: Option<String>,
}

impl Materials {
    /// Save as material (spec criterion 15): a new record, or the test's kind set added to or
    /// replaced in an existing one; the history gets a `tested` entry `entry_id` dated `now`.
    ///
    /// # Errors
    /// A taken id or invalid name of a new material, an unknown material, or an edit past 500
    /// entries or 256 KiB.
    pub fn save_test(
        &mut self,
        target: SaveTarget,
        test: TestResult,
        entry_id: &str,
        now: u64,
    ) -> Result<LibraryWrite, LibraryRefusal> {
        let add_test = |material: &mut Material| {
            material.sets.insert(
                test.kind,
                MaterialSet {
                    settings: test.set,
                    source: SetSource::Test(entry_id.to_owned()),
                },
            );
            material.history.push(HistoryEntry {
                id: entry_id.to_owned(),
                date: now,
                kind: test.kind,
                event: HistoryEvent::Tested {
                    grid: test.grid,
                    cell: test.cell,
                    set: test.set,
                },
            });
            Ok(())
        };
        match target {
            SaveTarget::Existing(id) => self.edit(&id, now, add_test),
            SaveTarget::New(new) => {
                if !is_record_id(&new.id)
                    || self.entry(&new.id).is_some()
                    || self.is_skipped(&new.id)
                {
                    return Err(LibraryRefusal::Taken);
                }
                if !is_valid_material_name(&new.name) {
                    return Err(LibraryRefusal::InvalidName);
                }
                let mut material = Material {
                    id: new.id,
                    modified: 0,
                    name: new.name,
                    thickness: new.thickness,
                    machine: new.machine,
                    sets: std::collections::BTreeMap::new(),
                    history: Vec::new(),
                };
                add_test(&mut material)?;
                material.modified = next_modified(0, now);
                self.commit(material)
            }
        }
    }

    /// Edit material (spec criterion 22): the set of `kind` gets typed numbers, source `edited`,
    /// with an `edited` history entry.
    ///
    /// # Errors
    /// An unknown material, no set of `kind`, equal numbers, or an edit past the limits.
    pub fn edit_set(
        &mut self,
        id: &str,
        kind: SetKind,
        settings: Settings,
        entry_id: String,
        now: u64,
    ) -> Result<LibraryWrite, LibraryRefusal> {
        self.edit(id, now, |material| {
            let refusal = no_set(material, kind);
            let set = material.sets.get_mut(&kind).ok_or(refusal)?;
            if set.settings == settings {
                return Err(LibraryRefusal::Unchanged);
            }
            *set = MaterialSet {
                settings,
                source: SetSource::Edited,
            };
            material.history.push(HistoryEntry {
                id: entry_id,
                date: now,
                kind,
                event: HistoryEvent::Edited { set: settings },
            });
            Ok(())
        })
    }

    /// Removes the set of `kind` (spec criterion 22) with a `removed` history entry.
    ///
    /// # Errors
    /// An unknown material, no set of `kind`, or an edit past the limits.
    pub fn remove_set(
        &mut self,
        id: &str,
        kind: SetKind,
        entry_id: String,
        now: u64,
    ) -> Result<LibraryWrite, LibraryRefusal> {
        self.edit(id, now, |material| {
            if material.sets.remove(&kind).is_none() {
                return Err(no_set(material, kind));
            }
            material.history.push(HistoryEntry {
                id: entry_id,
                date: now,
                kind,
                event: HistoryEvent::Removed,
            });
            Ok(())
        })
    }

    /// Renames the material `id` (spec criterion 21).
    ///
    /// # Errors
    /// An unknown material, an invalid or unchanged name.
    pub fn rename(
        &mut self,
        id: &str,
        name: String,
        now: u64,
    ) -> Result<LibraryWrite, LibraryRefusal> {
        if !is_valid_material_name(&name) {
            return Err(LibraryRefusal::InvalidName);
        }
        self.edit(id, now, |material| {
            if material.name == name {
                return Err(LibraryRefusal::Unchanged);
            }
            material.name = name;
            Ok(())
        })
    }

    /// Sets the thickness of the material `id`.
    ///
    /// # Errors
    /// An unknown material or an unchanged thickness.
    pub fn set_thickness(
        &mut self,
        id: &str,
        thickness: Thickness,
        now: u64,
    ) -> Result<LibraryWrite, LibraryRefusal> {
        self.edit(id, now, |material| {
            if material.thickness == thickness {
                return Err(LibraryRefusal::Unchanged);
            }
            material.thickness = thickness;
            Ok(())
        })
    }

    /// Undo or redo of a library step (ADR 0055 §9, Frage 190 A): when the record `id` still has
    /// the text `expected` (`None`: absent), `earlier` is written back (`None`: removed) with a
    /// new `modified` from `now`; a damaged earlier text goes back as it was.
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
        if self.is_skipped(id) || self.text(id) != expected {
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
        let text = match read_material(&name, earlier) {
            Ok(mut material) => {
                material.modified = next_modified(current_modified.max(material.modified), now);
                write_material(&material)
            }
            Err(_) => earlier.to_owned(),
        };
        let entry = entry_of(id, &name, text.clone());
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

    /// Applies `change` to a copy of the readable material `id`, stamps `modified` from `now`
    /// and commits it.
    fn edit(
        &mut self,
        id: &str,
        now: u64,
        change: impl FnOnce(&mut Material) -> Result<(), LibraryRefusal>,
    ) -> Result<LibraryWrite, LibraryRefusal> {
        let Some(MaterialRead::Loaded(material)) = self.entry(id).map(|entry| &entry.read) else {
            return Err(LibraryRefusal::Unknown);
        };
        let mut material = material.clone();
        change(&mut material)?;
        material.modified = next_modified(material.modified, now);
        self.commit(material)
    }

    /// Keeps `material` and returns its write, unless it passes 500 history entries or 256 KiB
    /// (ADR 0055 §4): then nothing changes.
    fn commit(&mut self, material: Material) -> Result<LibraryWrite, LibraryRefusal> {
        if material.history.len() > MAX_HISTORY_ENTRIES {
            return Err(LibraryRefusal::FullHistory {
                name: material.name,
            });
        }
        let text = write_material(&material);
        if text.len() > MAX_MATERIAL_BYTES {
            return Err(LibraryRefusal::TooLarge {
                name: material.name,
            });
        }
        let id = material.id.clone();
        let entry = Entry {
            read: MaterialRead::Loaded(material),
            text: text.clone(),
            machine: None,
            name: None,
        };
        match self.entries.iter_mut().find(|e| e.read.id() == id) {
            Some(slot) => *slot = entry,
            None => self.entries.push(entry),
        }
        self.sync_reads();
        Ok(LibraryWrite::Write { id, text })
    }
}

fn no_set(material: &Material, kind: SetKind) -> LibraryRefusal {
    LibraryRefusal::NoSet {
        name: material.name.clone(),
        kind: kind.label(),
    }
}

/// Whether `name` is a material name: not blank, at most 60 characters.
fn is_valid_material_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= MAX_MATERIAL_NAME_CHARS
}
