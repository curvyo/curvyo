//! The maker's machine profiles as one list (ADR 0038 §3, §4, §7): loaded from the record texts
//! the host read, edited in memory, and every edit names the write the host must do.

use crate::machine_profile::{
    Connection, GENERIC_LASER_ID, MachineProfile, ProfileValues, copy_name,
};
use crate::machine_profile_file::{RecordError, read_record, write_record};
use crate::record_envelope::{LibraryWrite, record_id};

/// At most this many records are read, in file-name order.
pub const MAX_RECORDS: usize = 1_000;

/// The library: the built-in plus the user profiles that loaded, and how many were skipped.
#[derive(Debug, Clone, PartialEq)]
pub struct MachineProfiles {
    builtin: MachineProfile,
    user: Vec<MachineProfile>,
    damaged: usize,
    newer: usize,
    /// The ids of the records that did not load; they are never written (ADR 0004 §9).
    skipped: Vec<String>,
}

impl Default for MachineProfiles {
    fn default() -> Self {
        Self {
            builtin: MachineProfile::generic_laser(),
            user: Vec::new(),
            damaged: 0,
            newer: 0,
            skipped: Vec::new(),
        }
    }
}

impl MachineProfiles {
    /// The library from `(file name, text)` pairs: names that are not `<uuid>.json` are ignored,
    /// a damaged or newer record is skipped and counted (criterion 13), every other one loads.
    /// The ids of skipped records and of records past [`MAX_RECORDS`] are kept so that no later
    /// write replaces them.
    #[must_use]
    pub fn load(entries: &[(String, String)]) -> Self {
        let mut records: Vec<(&str, &String, &String)> = entries
            .iter()
            .filter_map(|(name, text)| record_id(name).map(|id| (id, name, text)))
            .collect();
        records.sort_by(|a, b| a.1.cmp(b.1));
        let mut library = Self::default();
        for (index, (id, name, text)) in records.into_iter().enumerate() {
            if index >= MAX_RECORDS {
                library.skipped.push(id.to_owned());
                continue;
            }
            match read_record(name, text) {
                Ok(profile) => library.user.push(profile),
                Err(error) => {
                    match error {
                        RecordError::Damaged => library.damaged += 1,
                        RecordError::Newer => library.newer += 1,
                    }
                    library.skipped.push(id.to_owned());
                }
            }
        }
        library
    }

    /// Whether `id` names a record that was left as it is at load (damaged, newer or past the
    /// limit): it is never written, so Save as profile takes a new id instead.
    #[must_use]
    pub fn is_skipped(&self, id: &str) -> bool {
        self.skipped.iter().any(|skipped| skipped == id)
    }

    /// The notices of the last load (UX notes §2 item 1), one per kind that applies.
    #[must_use]
    pub fn load_notices(&self) -> Vec<String> {
        let mut notices = Vec::new();
        let line = |count: usize, what: &str| {
            if count == 1 {
                format!("1 profile {what}. It was left as it is.")
            } else {
                format!("{count} profiles {what}. They were left as they are.")
            }
        };
        if self.damaged > 0 {
            notices.push(line(self.damaged, "could not be read"));
        }
        if self.newer > 0 {
            let what = if self.newer == 1 {
                "is from a newer version"
            } else {
                "are from a newer version"
            };
            notices.push(line(self.newer, what));
        }
        notices
    }

    /// The profile `id`, the built-in included.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&MachineProfile> {
        if id == GENERIC_LASER_ID {
            return Some(&self.builtin);
        }
        self.user.iter().find(|profile| profile.id == id)
    }

    /// Whether `id` is a user profile of this library.
    #[must_use]
    pub fn is_user(&self, id: &str) -> bool {
        self.user.iter().any(|profile| profile.id == id)
    }

    /// The list order (UX notes §3): Generic laser first, then user profiles by name ignoring
    /// case, ties by id.
    #[must_use]
    pub fn listed(&self) -> Vec<&MachineProfile> {
        let mut user: Vec<(String, &MachineProfile)> = self
            .user
            .iter()
            .map(|profile| (profile.values.name.to_lowercase(), profile))
            .collect();
        user.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.id.cmp(&b.1.id)));
        std::iter::once(&self.builtin)
            .chain(user.into_iter().map(|(_, profile)| profile))
            .collect()
    }

    /// Whether any profile, the built-in included, has the name `name`.
    #[must_use]
    pub fn name_taken(&self, name: &str) -> bool {
        self.builtin.values.name == name || self.user.iter().any(|p| p.values.name == name)
    }

    /// Adds or replaces the user profile `profile` (Save as profile; a copy's id from another
    /// computer is kept, criterion 8). `None` for the built-in's id and for the id of a skipped
    /// record (ADR 0004 §9).
    pub fn add(&mut self, profile: MachineProfile) -> Option<LibraryWrite> {
        if profile.is_builtin() || self.is_skipped(&profile.id) {
            return None;
        }
        let write = write_of(&profile);
        match self.user.iter_mut().find(|p| p.id == profile.id) {
            Some(slot) => *slot = profile,
            None => self.user.push(profile),
        }
        Some(write)
    }

    /// Duplicate (criterion 2, 3): a user profile with `id`'s values named "`<name> copy`" (then
    /// numbered), under `new_id`. `None` for an unknown profile.
    pub fn duplicate(&mut self, id: &str, new_id: String, modified: i64) -> Option<LibraryWrite> {
        let source = self.find(id)?.clone();
        let name = copy_name(&source.values.name, |name| self.name_taken(name));
        let profile = MachineProfile {
            id: new_id,
            modified,
            values: ProfileValues {
                name,
                ..source.values
            },
            work_area: source.work_area,
            connection: source.connection,
            fixtures: source.fixtures,
        };
        self.add(profile)
    }

    /// Renames the user profile `id` to the accepted `name`. `None` for the built-in, an unknown
    /// profile or an unchanged name.
    pub fn rename(&mut self, id: &str, name: String, modified: i64) -> Option<LibraryWrite> {
        let profile = self.user_mut(id)?;
        if profile.values.name == name {
            return None;
        }
        profile.values.name = name;
        profile.modified = modified;
        Some(write_of(profile))
    }

    /// Update profile (criterion 7): overwrites the values and work area of the user profile
    /// `id`; its id and connection stay. `None` for the built-in or an unknown profile.
    pub fn update(
        &mut self,
        id: &str,
        values: ProfileValues,
        work_area: curvyo_document_core::DocumentSize,
        modified: i64,
    ) -> Option<LibraryWrite> {
        let profile = self.user_mut(id)?;
        profile.values = values;
        profile.work_area = work_area;
        profile.modified = modified;
        Some(write_of(profile))
    }

    /// Update profile with the page's fixtures (`specs/0102-laser-fixtures` criterion 28): as
    /// [`MachineProfiles::update`], and the set becomes `fixtures`.
    pub fn update_with_fixtures(
        &mut self,
        id: &str,
        values: ProfileValues,
        work_area: curvyo_document_core::DocumentSize,
        fixtures: Vec<curvyo_document_core::FixtureRecord>,
        modified: i64,
    ) -> Option<LibraryWrite> {
        self.user_mut(id)?.fixtures = fixtures;
        self.update(id, values, work_area, modified)
    }

    /// Sets the connection of the user profile `id` (criterion 9). `None` for the built-in, an
    /// unknown profile or an unchanged connection.
    pub fn set_connection(
        &mut self,
        id: &str,
        connection: Connection,
        modified: i64,
    ) -> Option<LibraryWrite> {
        let profile = self.user_mut(id)?;
        if profile.connection == connection {
            return None;
        }
        profile.connection = connection;
        profile.modified = modified;
        Some(write_of(profile))
    }

    /// Deletes the user profile `id`. `None` for the built-in or an unknown profile.
    pub fn delete(&mut self, id: &str) -> Option<LibraryWrite> {
        let index = self.user.iter().position(|profile| profile.id == id)?;
        self.user.remove(index);
        Some(LibraryWrite::Remove { id: id.to_owned() })
    }

    fn user_mut(&mut self, id: &str) -> Option<&mut MachineProfile> {
        self.user.iter_mut().find(|profile| profile.id == id)
    }
}

fn write_of(profile: &MachineProfile) -> LibraryWrite {
    LibraryWrite::Write {
        id: profile.id.clone(),
        text: write_record(profile),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_profile::OriginCorner;
    use crate::record_envelope::{new_record_id as new_profile_id, record_file_name};

    fn record(seed: u8, name: &str) -> (String, String) {
        let mut profile = MachineProfile::generic_laser();
        profile.id = new_profile_id([seed; 16]);
        profile.values.name = name.to_owned();
        (record_file_name(&profile.id), write_record(&profile))
    }

    #[test]
    fn good_records_load_and_bad_ones_are_counted_and_names_ignored() {
        let mut entries = vec![record(1, "b K40"), record(2, "A Ortur")];
        entries.push((record(3, "x").0, "{".to_owned()));
        let (name, text) = record(4, "newer");
        entries.push((
            name,
            text.replace("\"format_version\": 3", "\"format_version\": 7"),
        ));
        entries.push(("notes.txt".to_owned(), "{".to_owned()));
        let library = MachineProfiles::load(&entries);
        let names: Vec<&str> = library
            .listed()
            .iter()
            .map(|p| p.values.name.as_str())
            .collect();
        assert_eq!(names, vec!["Generic laser", "A Ortur", "b K40"]);
        assert_eq!(
            library.load_notices(),
            vec![
                "1 profile could not be read. It was left as it is.".to_owned(),
                "1 profile is from a newer version. It was left as it is.".to_owned(),
            ]
        );
        assert!(library.load_notices().iter().all(|n| n.len() <= 70));
    }

    #[test]
    fn the_built_in_cannot_be_renamed_updated_or_deleted_but_duplicates() {
        let mut library = MachineProfiles::default();
        assert!(library.rename(GENERIC_LASER_ID, "x".into(), 1).is_none());
        assert!(library.delete(GENERIC_LASER_ID).is_none());
        assert!(
            library
                .update(
                    GENERIC_LASER_ID,
                    ProfileValues::default(),
                    MachineProfile::generic_laser().work_area,
                    1
                )
                .is_none()
        );
        let id = new_profile_id([9; 16]);
        let write = library.duplicate(GENERIC_LASER_ID, id.clone(), 5).unwrap();
        assert!(matches!(&write, LibraryWrite::Write { id: written, .. } if *written == id));
        assert_eq!(library.find(&id).unwrap().values.name, "Generic laser copy");
        let again = new_profile_id([10; 16]);
        library
            .duplicate(GENERIC_LASER_ID, again.clone(), 6)
            .unwrap();
        assert_eq!(
            library.find(&again).unwrap().values.name,
            "Generic laser copy 2"
        );
    }

    #[test]
    fn edits_of_a_user_profile_return_their_writes() {
        let mut library = MachineProfiles::load(&[record(1, "K40")]);
        let id = new_profile_id([1; 16]);
        let LibraryWrite::Write { text, .. } = library.rename(&id, "Garage".into(), 7).unwrap()
        else {
            panic!("a rename writes");
        };
        assert!(text.contains("\"name\": \"Garage\""));
        assert!(library.rename(&id, "Garage".into(), 8).is_none());
        let values = ProfileValues {
            origin: OriginCorner::TopRight,
            ..ProfileValues::default()
        };
        library
            .update(&id, values, MachineProfile::generic_laser().work_area, 9)
            .unwrap();
        assert_eq!(
            library.find(&id).unwrap().values.origin,
            OriginCorner::TopRight
        );
        assert_eq!(
            library.delete(&id),
            Some(LibraryWrite::Remove { id: id.clone() })
        );
        assert!(library.find(&id).is_none());
    }
}
