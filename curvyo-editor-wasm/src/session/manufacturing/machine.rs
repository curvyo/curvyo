//! The session's machine profiles (`specs/0026-machine-profile`, ADR 0036 §7): the library and
//! the last pick, the commands of the Machine block, and the outbox of writes the host performs.
//! Library edits are no history step; edits of the manufacturing's copy are one step each.

use curvyo_document_core::{DocumentSize, Length, ProfileChange};
use curvyo_laser_core::{
    Connection, CopyRead, LibraryWrite, MachineProfile, MachineProfiles, OriginCorner, ProfileCopy,
    clamp_address, clamp_baud, clamp_feed, clamp_name, clamp_port, clamp_power, clamp_work_area,
    clamp_work_side, is_profile_id, new_profile_id, read_choice, read_copy, save_as_name,
    write_choice, write_copy,
};

use super::super::Session;
use super::commands::workbench_of;

/// One write the host must perform for the library or the last pick (ADR 0036 §1, §6).
#[derive(Debug, Clone, PartialEq, Eq)]
enum HostWrite {
    /// A record write or removal.
    Library(LibraryWrite),
    /// The new text of `machine-choice.json`.
    Choice(String),
}

/// What the session keeps of the maker's machine profiles: per user, never in a project.
#[derive(Debug, Default)]
pub(in crate::session) struct MachineState {
    /// The library.
    pub(in crate::session) profiles: MachineProfiles,
    /// The profile last picked on this computer.
    last_picked: Option<String>,
    /// The writes the host has not taken yet.
    outbox: Vec<HostWrite>,
}

/// The active manufacturing's copy as read, with its id and page.
pub(in crate::session) struct ActiveCopy {
    pub(in crate::session) id: curvyo_document_core::NodeId,
    pub(in crate::session) read: CopyRead,
    pub(in crate::session) page: DocumentSize,
}

impl Session {
    /// Reads the records the host listed: `(file name, text)` pairs (criteria 4, 13).
    pub fn load_machine_profiles(&mut self, entries: &[(String, String)]) {
        self.manufacturing.machine.profiles = MachineProfiles::load(entries);
    }

    /// Reads the records the host listed as the JSON array `[{"name", "text"}]`; a text that is
    /// not such an array loads no user profile.
    pub fn load_machine_profiles_json(&mut self, json: &str) {
        #[derive(serde::Deserialize)]
        struct Entry {
            name: String,
            text: String,
        }
        let entries: Vec<(String, String)> = serde_json::from_str::<Vec<Entry>>(json)
            .unwrap_or_default()
            .into_iter()
            .map(|entry| (entry.name, entry.text))
            .collect();
        self.load_machine_profiles(&entries);
    }

    /// Reads the kept `machine-choice.json` (criterion 10).
    pub fn load_machine_choice(&mut self, text: &str) {
        self.manufacturing.machine.last_picked = read_choice(text);
    }

    /// The host writes made since the last call, in order, as JSON: `{"kind": "write", "id",
    /// "text"}`, `{"kind": "remove", "id"}` or `{"kind": "choice", "text"}`.
    pub fn take_machine_writes_json(&mut self) -> String {
        let writes: Vec<serde_json::Value> = std::mem::take(&mut self.manufacturing.machine.outbox)
            .into_iter()
            .map(|write| match write {
                HostWrite::Library(LibraryWrite::Write { id, text }) => {
                    serde_json::json!({"kind": "write", "id": id, "text": text})
                }
                HostWrite::Library(LibraryWrite::Remove { id }) => {
                    serde_json::json!({"kind": "remove", "id": id})
                }
                HostWrite::Choice(text) => serde_json::json!({"kind": "choice", "text": text}),
            })
            .collect();
        serde_json::Value::from(writes).to_string()
    }

    /// The library.
    #[must_use]
    pub const fn machine_profiles(&self) -> &MachineProfiles {
        &self.manufacturing.machine.profiles
    }

    /// The profile a new manufacturing starts with: the last pick on this computer, else Generic
    /// laser, also when that profile no longer exists (criterion 10).
    pub(in crate::session) fn initial_machine_profile(&self) -> MachineProfile {
        let machine = &self.manufacturing.machine;
        machine
            .last_picked
            .as_deref()
            .and_then(|id| machine.profiles.find(id))
            .cloned()
            .unwrap_or_else(MachineProfile::generic_laser)
    }

    /// The copy of the active manufacturing, `None` outside one or when its workbench has no
    /// Machine block.
    pub(in crate::session) fn active_copy(&self) -> Option<ActiveCopy> {
        let manufacturing = self.active_manufacturing()?;
        let contribution = workbench_of(&manufacturing.process)?;
        let has_block = contribution
            .tabs
            .iter()
            .flat_map(|tab| &tab.blocks)
            .any(|block| matches!(block, curvyo_extension_core::Block::MachineProfile { .. }));
        if !has_block {
            return None;
        }
        let read = read_copy(
            manufacturing.profile.as_ref(),
            manufacturing.legacy_machine_name.as_deref(),
        );
        Some(ActiveCopy {
            id: manufacturing.id,
            page: manufacturing.work_area(),
            read,
        })
    }

    /// Writes `copy` and the page size `page` as one step labelled `change`.
    fn write_active_copy(
        &mut self,
        id: curvyo_document_core::NodeId,
        copy: &ProfileCopy,
        page: DocumentSize,
        change: ProfileChange,
    ) -> bool {
        let map = write_copy(copy);
        self.recording(|this| {
            this.document
                .set_manufacturing_profile(id, &map, page, change)
                .unwrap_or(false)
        })
    }

    /// Picks the library profile `id` (criterion 5): the copy takes its values and the output
    /// page its work area in one step; remembered as this computer's last pick.
    pub fn pick_machine_profile(&mut self, id: &str) -> bool {
        let Some(active) = self.active_copy() else {
            return false;
        };
        let Some(profile) = self.manufacturing.machine.profiles.find(id).cloned() else {
            return false;
        };
        let changed = self.write_active_copy(
            active.id,
            &ProfileCopy::of(&profile),
            profile.work_area,
            ProfileChange::Pick,
        );
        let machine = &mut self.manufacturing.machine;
        machine.last_picked = Some(profile.id.clone());
        machine
            .outbox
            .push(HostWrite::Choice(write_choice(&profile.id)));
        changed
    }

    /// Edits one value of the copy (criterion 7): `edit` changes the values and returns the
    /// notice; one step with `change`'s label.
    fn edit_copy(
        &mut self,
        change: ProfileChange,
        edit: impl FnOnce(&mut ProfileCopy) -> Option<Option<String>>,
    ) -> Option<String> {
        let active = self.active_copy()?;
        let mut copy = active.read.copy;
        let notice = edit(&mut copy)?;
        self.write_active_copy(active.id, &copy, active.page, change);
        notice
    }

    /// The Name field: trimmed, cut to 60 characters; an empty name changes nothing.
    pub fn set_machine_name(&mut self, typed: &str) -> Option<String> {
        self.edit_copy(ProfileChange::EditName, |copy| {
            let limited = clamp_name(typed)?;
            copy.values.name = limited.value;
            Some(limited.notice)
        })
    }

    /// The origin corner by its stored name (`bottom_left`).
    pub fn set_machine_origin(&mut self, name: &str) -> bool {
        let Some(corner) = OriginCorner::from_name(name) else {
            return false;
        };
        let before = self.document.version();
        self.edit_copy(ProfileChange::EditOrigin, |copy| {
            copy.values.origin = corner;
            Some(None)
        });
        self.document.version() != before
    }

    /// The Max feed field, rounded and set into its range.
    pub fn set_machine_max_feed(&mut self, typed: f64) -> Option<String> {
        self.edit_copy(ProfileChange::EditMaxFeed, |copy| {
            let limited = clamp_feed(typed)?;
            copy.values.max_feed = limited.value;
            Some(limited.notice)
        })
    }

    /// The Max power field, rounded and set into its range.
    pub fn set_machine_max_power(&mut self, typed: f64) -> Option<String> {
        self.edit_copy(ProfileChange::EditMaxPower, |copy| {
            let limited = clamp_power(typed)?;
            copy.values.max_power = limited.value;
            Some(limited.notice)
        })
    }

    /// The Laser mode switch.
    pub fn set_machine_laser_mode(&mut self, on: bool) {
        self.edit_copy(ProfileChange::EditLaserMode, |copy| {
            copy.values.laser_mode = on;
            Some(None)
        });
    }

    /// The Width and Height fields, each set into the profile range (`0093` criterion 3): the
    /// output page follows in one step. Whether anything changed, and the first notice.
    pub fn set_work_area_limited(
        &mut self,
        width: Length,
        height: Length,
    ) -> (bool, Option<String>) {
        let Some(manufacturing) = self.active_manufacturing() else {
            return (false, None);
        };
        let (Some(width), Some(height)) = (
            clamp_work_side("Width", width),
            clamp_work_side("Height", height),
        ) else {
            return (false, None);
        };
        let notice = width.notice.or(height.notice);
        let changed = self.recording(|this| {
            this.document
                .set_work_area(
                    manufacturing.id,
                    DocumentSize::new(width.value, height.value),
                )
                .unwrap_or(false)
        });
        (changed, notice)
    }

    /// Reset from profile (criterion 7): the copy and the page take the library record's values
    /// in one step.
    pub fn reset_machine_profile(&mut self) -> bool {
        let Some(active) = self.active_copy() else {
            return false;
        };
        let Some(record) = active
            .read
            .copy
            .id
            .as_deref()
            .and_then(|id| self.manufacturing.machine.profiles.find(id))
            .cloned()
        else {
            return false;
        };
        self.write_active_copy(
            active.id,
            &ProfileCopy::of(&record),
            record.work_area,
            ProfileChange::Reset,
        )
    }

    /// Save as profile (criteria 3, 8): a user profile from the values the block shows, added to
    /// the library (no history), and the copy points at it in one step. A copy whose id is a
    /// user id this library lacks is saved under that id; otherwise a new id is made from
    /// `random`. Returns the notice of a work area cut into the profile range.
    pub fn save_machine_profile(&mut self, random: [u8; 16], now_ms: i64) -> Option<String> {
        let active = self.active_copy()?;
        let profiles = &self.manufacturing.machine.profiles;
        let copy = active.read.copy;
        let id = copy
            .id
            .clone()
            .filter(|id| is_profile_id(id) && profiles.find(id).is_none())
            .unwrap_or_else(|| new_profile_id(random));
        let name = save_as_name(&copy.values.name, |name| profiles.name_taken(name));
        let area = clamp_work_area(active.page);
        let profile = MachineProfile {
            id,
            modified: now_ms,
            values: curvyo_laser_core::ProfileValues {
                name,
                ..copy.values
            },
            work_area: area.value,
            connection: Connection::default(),
        };
        let write = self.manufacturing.machine.profiles.add(profile.clone())?;
        self.manufacturing
            .machine
            .outbox
            .push(HostWrite::Library(write));
        self.write_active_copy(
            active.id,
            &ProfileCopy::of(&profile),
            active.page,
            ProfileChange::SaveAs,
        );
        area.notice
    }

    /// Update profile (criterion 7): the user record the copy came from takes the copy's values
    /// and the page size (cut into the profile range); no history step. Returns the notice.
    pub fn update_machine_profile(&mut self, now_ms: i64) -> Option<String> {
        let active = self.active_copy()?;
        let id = active.read.copy.id?;
        let area = clamp_work_area(active.page);
        let write = self.manufacturing.machine.profiles.update(
            &id,
            active.read.copy.values,
            area.value,
            now_ms,
        )?;
        self.manufacturing
            .machine
            .outbox
            .push(HostWrite::Library(write));
        area.notice
    }

    /// Duplicate (criterion 2): the new profile's id, made from `random`.
    pub fn duplicate_machine_profile(
        &mut self,
        id: &str,
        random: [u8; 16],
        now_ms: i64,
    ) -> Option<String> {
        let new_id = new_profile_id(random);
        let write = self
            .manufacturing
            .machine
            .profiles
            .duplicate(id, new_id.clone(), now_ms)?;
        self.manufacturing
            .machine
            .outbox
            .push(HostWrite::Library(write));
        Some(new_id)
    }

    /// Renames the user profile `id`; an empty name changes nothing. Returns the notice.
    pub fn rename_machine_profile(&mut self, id: &str, typed: &str, now_ms: i64) -> Option<String> {
        let limited = clamp_name(typed)?;
        let write = self
            .manufacturing
            .machine
            .profiles
            .rename(id, limited.value, now_ms)?;
        self.manufacturing
            .machine
            .outbox
            .push(HostWrite::Library(write));
        limited.notice
    }

    /// Deletes the user profile `id`; manufacturings keep their copy (criterion 3).
    pub fn delete_machine_profile(&mut self, id: &str) -> bool {
        let Some(write) = self.manufacturing.machine.profiles.delete(id) else {
            return false;
        };
        self.manufacturing
            .machine
            .outbox
            .push(HostWrite::Library(write));
        true
    }

    /// The connection of the chosen user profile (criterion 9): a library write, no history
    /// step. Returns the first notice.
    pub fn set_machine_connection(
        &mut self,
        address: &str,
        port: &str,
        baud: f64,
        now_ms: i64,
    ) -> Option<String> {
        let active = self.active_copy()?;
        let id = active.read.copy.id?;
        let address = clamp_address(address);
        let port = clamp_port(port);
        let baud = clamp_baud(baud)?;
        let connection = Connection {
            connector_address: address.value,
            controller_port: port.value,
            baud_rate: baud.value,
        };
        let write = self
            .manufacturing
            .machine
            .profiles
            .set_connection(&id, connection, now_ms);
        if let Some(write) = write {
            self.manufacturing
                .machine
                .outbox
                .push(HostWrite::Library(write));
        }
        address.notice.or(port.notice).or(baud.notice)
    }
}
