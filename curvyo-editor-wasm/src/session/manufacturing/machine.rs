//! The session's machine profiles (`specs/0026-machine-profile`, ADR 0038 §7): the library and
//! the last pick, the commands of the Machine block, and the outbox of writes the host performs.
//! Library edits are no history step; edits of the manufacturing's copy are one step each.

use curvyo_document_core::{DocumentSize, FixtureRecord, Length, PageCorner, ProfileChange};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::{
    Connection, CopyRead, LibraryWrite, MachineProfile, MachineProfiles, OriginCorner, ProfileCopy,
    clamp_address, clamp_baud, clamp_feed, clamp_name, clamp_port, clamp_power, clamp_work_area,
    is_profile_id, new_profile_id, read_choice, read_copy, save_as_name, write_choice, write_copy,
};
use curvyo_laser_core::{FixtureKind, FixturePart, job_parts};

use super::super::Session;
use super::commands::workbench_of;

/// `record` with its jobs as a profile write keeps them: known kinds only, each tick a part of
/// the fixture's kind, in Parts-list order (ADR 0052 §7).
fn saved_jobs(mut record: FixtureRecord) -> FixtureRecord {
    let kind = FixtureKind::from_stored(&record.kind);
    let slots = record.slots.len();
    record
        .jobs
        .retain(|job| OperationKind::parse(&job.kind).is_some());
    for job in &mut record.jobs {
        let (parts, _) = job_parts(kind, slots, &job.targets);
        job.targets = parts.into_iter().map(FixturePart::target).collect();
        job.targets.sort_unstable();
    }
    record
}

/// The notice of a pick or reset that removed fixtures (criterion 29, ADR 0050 section 6): it
/// informs about a removal, so it is not a success notice.
const FIXTURES_REMOVED_NOTICE: &str = "Fixtures removed (undo restores them).";

/// One write the host must perform for the library or the last pick (ADR 0038 §1, §6).
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

    /// Writes `copy` and the page size `page` as one step labelled `change`; with `fixtures`
    /// the page's fixtures are replaced by that set in the same step (ADR 0050 §4).
    fn write_active_copy(
        &mut self,
        id: curvyo_document_core::NodeId,
        copy: &ProfileCopy,
        page: DocumentSize,
        change: ProfileChange,
        fixtures: Option<&[FixtureRecord]>,
    ) -> bool {
        let map = write_copy(copy);
        let origin = copy.values.origin;
        let anchor = PageCorner {
            right: origin.is_right(),
            bottom: origin.is_bottom(),
        };
        let pages = self.document.pages();
        let Some(contents) = self.page_contents() else {
            return false;
        };
        // The fixtures a pick or reset removes say so, with the undo that brings them back
        // (criterion 29, ADR 0050 section 6). Nothing is posted when none went.
        let before = match fixtures {
            Some(_) => self.active_fixture_set(),
            None => Vec::new(),
        };
        let changed = self.recording_unguarded(|this| {
            match fixtures {
                Some(set) => this.document.set_manufacturing_profile_and_fixtures(
                    id, &map, page, anchor, change, set, &contents,
                ),
                None => this
                    .document
                    .set_manufacturing_profile(id, &map, page, anchor, change, &contents),
            }
            .unwrap_or(false)
        });
        if let Some(output) = self.document.manufacturing(id).and_then(|m| m.page) {
            self.note_moved_pages(&pages, &output.id);
        }
        if let Some(set) = fixtures
            && changed
            && before.iter().any(|fixture| !set.contains(fixture))
        {
            self.post_fixture_notice(FIXTURES_REMOVED_NOTICE);
        }
        changed
    }

    /// The fixtures of the active page as a profile set (`specs/0102` criterion 28), its jobs as
    /// a profile write keeps them, so "Differs from profile" compares like with like (ADR 0052
    /// §7): a job of an unknown kind and a tick of no part of the kind are left out.
    pub(super) fn active_fixture_set(&self) -> Vec<FixtureRecord> {
        self.active_manufacturing()
            .map(|m| m.fixtures.iter().map(|f| saved_jobs(f.record())).collect())
            .unwrap_or_default()
    }

    /// Picks the library profile `id` (criterion 5): the copy takes its values, the output page
    /// its work area and the page's fixtures its set (Generic laser: none; `0102` criterion 29)
    /// in one step; remembered as this computer's last pick.
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
            Some(&profile.fixtures),
        );
        self.remember_machine_pick(&profile.id);
        changed
    }

    /// Keeps `id` as this computer's last pick (criterion 10).
    pub(in crate::session) fn remember_machine_pick(&mut self, id: &str) {
        let machine = &mut self.manufacturing.machine;
        machine.last_picked = Some(id.to_owned());
        machine.outbox.push(HostWrite::Choice(write_choice(id)));
    }

    /// Edits one value of the copy (criterion 7): `edit` changes the values and returns the
    /// notice; one step with `change`'s label.
    pub(in crate::session) fn edit_copy(
        &mut self,
        change: ProfileChange,
        edit: impl FnOnce(&mut ProfileCopy) -> Option<Option<String>>,
    ) -> Option<String> {
        let active = self.active_copy()?;
        let mut copy = active.read.copy;
        let notice = edit(&mut copy)?;
        self.write_active_copy(active.id, &copy, active.page, change, None);
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

    /// Edit work area of a user profile (`0098` criterion 15): each side set into the profile
    /// range; a library write, no history step and no change to any page until Reset from
    /// profile. `None` for the built-in or an unknown profile; else the first notice.
    pub fn set_profile_work_area(
        &mut self,
        id: &str,
        width: Length,
        height: Length,
        now_ms: i64,
    ) -> Option<Option<String>> {
        let profile = self.manufacturing.machine.profiles.find(id)?.clone();
        let area = clamp_work_area(DocumentSize::new(width, height));
        let write =
            self.manufacturing
                .machine
                .profiles
                .update(id, profile.values, area.value, now_ms)?;
        self.manufacturing
            .machine
            .outbox
            .push(HostWrite::Library(write));
        Some(area.notice)
    }

    /// Reset from profile (criterion 7): the copy, the page and the page's fixtures take the
    /// library record's values and set in one step (`0102` criterion 28).
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
            Some(&record.fixtures),
        )
    }

    /// Save as profile (criteria 3, 8): a user profile from the values the block shows, added to
    /// the library (no history), and the copy points at it in one step. A copy whose id is a
    /// user id this library lacks is saved under that id, unless a record of that id was skipped
    /// at load (it is never written); otherwise a new id is made from `random`. Returns the
    /// notice of a work area cut into the profile range.
    pub fn save_machine_profile(&mut self, random: [u8; 16], now_ms: i64) -> Option<String> {
        let active = self.active_copy()?;
        let profiles = &self.manufacturing.machine.profiles;
        let copy = active.read.copy;
        let id = copy
            .id
            .clone()
            .filter(|id| {
                is_profile_id(id) && profiles.find(id).is_none() && !profiles.is_skipped(id)
            })
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
            fixtures: self.active_fixture_set(),
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
            None,
        );
        area.notice
    }

    /// Update profile (criterion 7): the user record the copy came from takes the copy's values,
    /// the page size (cut into the profile range) and the page's fixtures (`0102` criterion 28);
    /// no history step. Returns the notice.
    pub fn update_machine_profile(&mut self, now_ms: i64) -> Option<String> {
        let active = self.active_copy()?;
        let id = active.read.copy.id?;
        let area = clamp_work_area(active.page);
        let fixtures = self.active_fixture_set();
        let write = self.manufacturing.machine.profiles.update_with_fixtures(
            &id,
            active.read.copy.values,
            area.value,
            fixtures,
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
