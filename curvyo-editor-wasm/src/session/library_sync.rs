//! The device group's library stream as this session's libraries see it (ADR 0062 decision 5,
//! `specs/0116-library-sync` criteria 3, 7 to 15, 21 to 27): arrivals applied through the
//! record merge, host writes queued, local edits tapped for the group's list. Every notice is
//! `curvyo-ui-core::library_text`'s.

use curvyo_document_core::LibraryKind;
use curvyo_laser_core::library::{
    Material, MaterialRead, PhotoWrite, read_record as read_material, remove_photo,
    write_record as write_material,
};
use curvyo_laser_core::{
    LibraryWrite, MergeReason, RecordMerge, conflict_id, merge_machine_record,
    merge_material_record, read_record as read_machine, record_file_name,
    write_record as write_machine,
};
use curvyo_ui_core::library_text as texts;

use crate::device_group_glue::{ArrivalChange, DeviceGroupGlue};

use super::Session;
use super::materials::MaterialWrite;

/// One local library edit, tapped from the host write lists for the group's list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryEdit {
    /// The record `id` was written with `text` (a new item or an edit).
    Write {
        /// Which library.
        kind: LibraryKind,
        /// The record id.
        id: String,
        /// The record text.
        text: String,
    },
    /// The record `id` was deleted.
    Remove {
        /// Which library.
        kind: LibraryKind,
        /// The record id.
        id: String,
    },
}

/// What the session keeps for the library sync: the local edits not yet handed to the group's
/// list, and the notices of the arrivals not yet shown.
#[derive(Debug, Default)]
pub(in crate::session) struct LibrarySyncState {
    pub(in crate::session) edits: Vec<LibraryEdit>,
    notices: std::collections::VecDeque<String>,
    /// An arrival changed a local record or queued a host write since the page last asked.
    applied: bool,
}

impl LibrarySyncState {
    /// Taps a library write the host write list produced.
    pub(in crate::session) fn tap_write(&mut self, kind: LibraryKind, id: &str, text: &str) {
        self.edits.push(LibraryEdit::Write {
            kind,
            id: id.to_owned(),
            text: text.to_owned(),
        });
    }

    /// Taps a library removal the host write list produced.
    pub(in crate::session) fn tap_remove(&mut self, kind: LibraryKind, id: &str) {
        self.edits.push(LibraryEdit::Remove {
            kind,
            id: id.to_owned(),
        });
    }
}

/// The photo removals deleted with `material` (ADR 0058 §4).
fn photo_removals(material: &Material) -> Vec<PhotoWrite> {
    material
        .tests
        .iter()
        .filter_map(|test| {
            test.photo
                .map(|kind| remove_photo(&material.id, test.id, kind))
        })
        .collect()
}

impl Session {
    /// Every local machine and material record as `(kind, id, text)`, for the first sync
    /// (criterion 3) and the rows of task 7.
    #[must_use]
    pub fn library_items(&self) -> Vec<(LibraryKind, String, String)> {
        let mut items = Vec::new();
        for profile in self.manufacturing.machine.profiles.listed() {
            if self.manufacturing.machine.profiles.is_user(&profile.id) {
                items.push((
                    LibraryKind::Machine,
                    profile.id.clone(),
                    write_machine(profile),
                ));
            }
        }
        // The maker's cutter profiles travel in the same machines list (`0159` criterion 18).
        for profile in &self.manufacturing.plotter.profiles {
            items.push((
                LibraryKind::Machine,
                profile.id.clone(),
                curvyo_cutter_core::write_record(profile),
            ));
        }
        for read in self.materials.library().listed() {
            if matches!(read, MaterialRead::Loaded(_))
                && let Some(text) = self.materials.library().text(read.id())
            {
                items.push((LibraryKind::Material, read.id().to_owned(), text.to_owned()));
            }
        }
        items
    }

    /// The first sync (criterion 3): writes every local item of this device on. How many were
    /// written.
    pub fn library_first_sync(&mut self, group: &mut DeviceGroupGlue) -> usize {
        let items = self.library_items();
        group.library_first_sync(&items)
    }

    /// The per-item toggle (criteria 3, 4, 10): on writes the local record's text, off keeps
    /// the marker entry. Whether the item is on or off now.
    pub fn library_switch(
        &mut self,
        group: &mut DeviceGroupGlue,
        kind: LibraryKind,
        id: &str,
        on: bool,
    ) -> bool {
        if !on {
            group.library_switch_off(kind, id);
            return false;
        }
        let text = match kind {
            LibraryKind::Machine => self.machine_record_text(id),
            LibraryKind::Material => self.materials.library().text(id).map(str::to_owned),
        };
        match text {
            Some(text) => {
                group.library_set(kind, id, &text);
                true
            }
            None => false,
        }
    }

    /// Feeds this session's local library edits to the group's list, then applies the entries
    /// that arrived; the next notice, empty when there was none.
    pub fn library_sync_pump(&mut self, group: &mut DeviceGroupGlue) -> String {
        let edits = std::mem::take(&mut self.library.edits);
        group.library_local_edits(edits);
        self.apply_library_arrivals(group);
        self.take_library_notice().unwrap_or_default()
    }

    /// The library entries that arrived in the group, applied against the local records
    /// through the merge (ADR 0062 decision 5).
    pub fn apply_library_arrivals(&mut self, group: &mut DeviceGroupGlue) {
        let arrivals = group.take_library_arrivals();
        let mut machines = 0;
        let mut materials = 0;
        let mut only: Option<(LibraryKind, String)> = None;
        for arrival in arrivals {
            match arrival.change {
                ArrivalChange::Arrived {
                    sync: true,
                    text: Some(text),
                    previous,
                } => {
                    if self.library_arrived(
                        group,
                        arrival.kind,
                        &arrival.id,
                        &text,
                        previous.as_deref(),
                    ) {
                        only = Some((arrival.kind, arrival.id.clone()));
                        match arrival.kind {
                            LibraryKind::Machine => machines += 1,
                            LibraryKind::Material => materials += 1,
                        }
                    }
                }
                ArrivalChange::SwitchedOff => {
                    self.library
                        .notices
                        .push_back(texts::SYNC_OFF_NOTICE.to_owned());
                }
                ArrivalChange::Removed => {
                    self.library_removed(arrival.kind, &arrival.id);
                }
                ArrivalChange::Arrived { sync: false, .. }
                | ArrivalChange::Arrived {
                    sync: true,
                    text: None,
                    ..
                } => {}
            }
        }
        let notice = if machines + materials == 1
            && let Some((kind, id)) = only
            && let Some(name) = self.library_arrival_name(kind, &id)
        {
            match kind {
                LibraryKind::Machine => texts::machine_arrived_notice(&name),
                LibraryKind::Material => texts::material_arrived_notice(&name),
            }
        } else if machines + materials > 0 {
            texts::arrived_notice(0, machines, materials)
        } else {
            return;
        };
        self.library.notices.push_back(notice);
    }

    /// Whether arrivals changed local records or queued host writes since the last call: the
    /// page then flushes the writes and re-reads its rows, notice or none (an edit of an item
    /// the device holds is applied without a notice).
    pub fn take_library_applied(&mut self) -> bool {
        std::mem::take(&mut self.library.applied)
    }

    /// The next notice of a library arrival, once.
    pub fn take_library_notice(&mut self) -> Option<String> {
        self.library.notices.pop_front()
    }

    fn machine_record_text(&self, id: &str) -> Option<String> {
        self.manufacturing
            .machine
            .profiles
            .find(id)
            .map(write_machine)
            .or_else(|| {
                self.manufacturing
                    .plotter
                    .find(id)
                    .map(curvyo_cutter_core::write_record)
            })
    }

    /// A synced item's name from the local record that holds it now, for its notice.
    fn library_arrival_name(&self, kind: LibraryKind, id: &str) -> Option<String> {
        match kind {
            LibraryKind::Machine => self
                .manufacturing
                .machine
                .profiles
                .find(id)
                .map(|profile| profile.values.name.clone())
                .or_else(|| {
                    self.manufacturing
                        .plotter
                        .find(id)
                        .map(|profile| profile.values.name.clone())
                }),
            LibraryKind::Material => self.materials.library().name_of(id).map(str::to_owned),
        }
    }

    /// One arrived record text against the local record of the same id: the merge decides
    /// (ADR 0062 decision 5); the loser becomes the conflict copy, a local winner is written
    /// back to the stream. A local record that still equals the entry's `previous` text was not
    /// edited here: the arrival is an edit of the other device, not a conflict, and keeps no
    /// copy. Whether the item is new to this device: only a new item is an arrival of the
    /// notice (UX notes section 5), an edit of an item the device holds is applied silently.
    fn library_arrived(
        &mut self,
        group: &mut DeviceGroupGlue,
        kind: LibraryKind,
        id: &str,
        text: &str,
        previous: Option<&str>,
    ) -> bool {
        let local = match kind {
            LibraryKind::Machine => self.machine_record_text(id),
            LibraryKind::Material => self.materials.library().text(id).map(str::to_owned),
        };
        // The merge is the family's whose `kind` the incoming record names (ADR 0095 §4).
        let merge = match kind {
            LibraryKind::Machine if is_cutter_record(text) => {
                curvyo_cutter_core::merge_cutter_record(id, text, local.as_deref())
            }
            LibraryKind::Machine => merge_machine_record(id, text, local.as_deref()),
            LibraryKind::Material => merge_material_record(id, text, local.as_deref()),
        };
        match merge {
            // Another machine family's record is its own merge's to take, with no notice (ADR
            // 0095 §4).
            RecordMerge::Same
            | RecordMerge::SkipIncoming {
                reason: MergeReason::OtherFamily,
            } => false,
            RecordMerge::IncomingWins { loser } => {
                self.library_write_local(kind, id, text);
                let arrived = local.is_none();
                if let Some(loser) = loser.filter(|loser| Some(loser.as_str()) != previous) {
                    self.library_conflict(kind, id, &loser);
                }
                arrived
            }
            RecordMerge::LocalWins { conflict } => {
                if local.as_deref() != previous {
                    self.library_conflict(kind, id, &conflict);
                }
                if let Some(winner) = local {
                    group.library_set(kind, id, &winner);
                }
                false
            }
            RecordMerge::SkipIncoming { reason } => {
                let notice = match reason {
                    MergeReason::Damaged => texts::DAMAGED_NOTICE,
                    MergeReason::Newer => texts::NEWER_NOTICE,
                    MergeReason::Older | MergeReason::OtherFamily => texts::OLDER_NOTICE,
                };
                self.library.notices.push_back(notice.to_owned());
                false
            }
            RecordMerge::LocalNewer { conflict } => {
                self.library_conflict(kind, id, &conflict);
                self.library
                    .notices
                    .push_back(texts::LOCAL_NEWER_NOTICE.to_owned());
                false
            }
        }
    }

    /// The winner becomes the local record, written through the existing host write paths with
    /// the exact text that travelled (criterion 7).
    fn library_write_local(&mut self, kind: LibraryKind, id: &str, text: &str) {
        self.library.applied = true;
        match kind {
            LibraryKind::Machine => {
                if let Ok(profile) = read_machine(&record_file_name(id), text) {
                    let _ = self.manufacturing.machine.profiles.add(profile);
                } else if let Ok(profile) =
                    curvyo_cutter_core::read_record(&record_file_name(id), text)
                {
                    self.manufacturing.plotter.put(profile);
                }
                self.manufacturing.machine.queue(LibraryWrite::Write {
                    id: id.to_owned(),
                    text: text.to_owned(),
                });
            }
            LibraryKind::Material => {
                self.materials.library_mut().restore(id, Some(text));
                self.materials.queue(MaterialWrite::Write {
                    id: id.to_owned(),
                    text: text.to_owned(),
                });
            }
        }
    }

    /// The loser of a merge becomes the conflict copy `<uuid>.conflict` (ADR 0062 decision 5),
    /// a host write beside the record, renamed "<name> (from your devices)" (criterion 9) when
    /// its text parses; the notice names the copy.
    fn library_conflict(&mut self, kind: LibraryKind, id: &str, loser: &str) {
        self.library.applied = true;
        let copy = conflict_id(id);
        let (text, name) = Self::conflict_copy(kind, id, loser);
        match kind {
            LibraryKind::Machine => {
                self.manufacturing
                    .machine
                    .queue(LibraryWrite::Write { id: copy, text });
            }
            LibraryKind::Material => {
                self.materials
                    .queue(MaterialWrite::Write { id: copy, text });
            }
        }
        let notice = match name {
            Some(name) => texts::conflict_notice(&name),
            None => texts::CONFLICT_UNNAMED_NOTICE.to_owned(),
        };
        self.library.notices.push_back(notice);
    }

    /// The conflict copy's record text (criterion 9): the loser renamed
    /// "<name> (from your devices)" when its text parses — everything else of the record stays
    /// as it travelled — else the loser's bytes as they are; with the name for the notice, or
    /// `None` when the text did not parse.
    fn conflict_copy(kind: LibraryKind, id: &str, loser: &str) -> (String, Option<String>) {
        let file = record_file_name(id);
        match kind {
            LibraryKind::Machine => match read_machine(&file, loser) {
                Ok(mut profile) => {
                    let name = texts::conflict_copy_name(&profile.values.name);
                    profile.values.name.clone_from(&name);
                    (write_machine(&profile), Some(name))
                }
                Err(_) => match curvyo_cutter_core::read_record(&file, loser) {
                    Ok(mut profile) => {
                        let name = texts::conflict_copy_name(&profile.values.name);
                        profile.values.name.clone_from(&name);
                        (curvyo_cutter_core::write_record(&profile), Some(name))
                    }
                    Err(_) => (loser.to_owned(), None),
                },
            },
            LibraryKind::Material => match read_material(&file, loser) {
                Ok(mut material) => {
                    let name = texts::conflict_copy_name(&material.name);
                    material.name.clone_from(&name);
                    (write_material(&material), Some(name))
                }
                Err(_) => (loser.to_owned(), None),
            },
        }
    }

    /// A deleted synced item (criterion 11): the local record goes with its photos and retired
    /// test ids, through the existing host write paths.
    fn library_removed(&mut self, kind: LibraryKind, id: &str) {
        let name = self.library_arrival_name(kind, id);
        let gone = match kind {
            LibraryKind::Machine => {
                let write = self.manufacturing.machine.profiles.delete(id).or_else(|| {
                    let plotter = &mut self.manufacturing.plotter.profiles;
                    let at = plotter.iter().position(|p| p.id == id)?;
                    plotter.remove(at);
                    Some(LibraryWrite::Remove { id: id.to_owned() })
                });
                let Some(write) = write else {
                    return;
                };
                self.manufacturing.machine.queue(write);
                true
            }
            LibraryKind::Material => {
                let Ok((write, material)) = self.materials.library_mut().delete(id) else {
                    return;
                };
                self.materials.queue(write.into());
                if let Some(material) = material {
                    for photo in photo_removals(&material) {
                        self.materials.queue_photo(photo);
                    }
                    self.retire_test_ids(material.tests.iter().map(|test| test.id).collect());
                }
                true
            }
        };
        if gone {
            self.library.applied = true;
            self.library.notices.push_back(match name {
                Some(name) => texts::deleted_notice(&name),
                None => texts::DELETED_UNNAMED_NOTICE.to_owned(),
            });
        }
    }
}

/// Whether `text` is a record of kind `cutter` (ADR 0095 §4: the merge follows the kind).
pub(in crate::session) fn is_cutter_record(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text)
        .is_ok_and(|value| value.get("kind").and_then(serde_json::Value::as_str) == Some("cutter"))
}
