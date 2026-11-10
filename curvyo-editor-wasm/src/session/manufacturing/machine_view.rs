//! The Machine block's part of the manufacturing view JSON (`specs/0026-machine-profile` UX notes
//! §2 to §4): the copy, its state against the library, the connection when editable, the
//! library's rows and notices. "Differs from profile" is derived here on every read.

use curvyo_laser_core::{
    DAMAGED_NAME_NOTICE, GENERIC_LASER_ID, MachineProfile, MachineProfiles, differs,
};
use serde_json::{Value, json};

use super::super::Session;

/// One row of the profile list.
fn row(profile: &MachineProfile, chosen: Option<&str>) -> Value {
    json!({
        "id": profile.id,
        "name": profile.values.name,
        "width": profile.work_area.width.as_mm(),
        "height": profile.work_area.height.as_mm(),
        "builtin": profile.is_builtin(),
        "chosen": chosen == Some(profile.id.as_str()),
    })
}

/// The rows of `profiles` in list order.
#[must_use]
pub fn profile_rows(profiles: &MachineProfiles, chosen: Option<&str>) -> Vec<Value> {
    profiles
        .listed()
        .into_iter()
        .map(|profile| row(profile, chosen))
        .collect()
}

impl Session {
    /// The Machine block's values for the active manufacturing; `None` without one.
    pub(super) fn machine_view(&self) -> Option<Value> {
        let active = self.active_copy()?;
        let profiles = &self.manufacturing.machine.profiles;
        let copy = &active.read.copy;
        let record = copy.id.as_deref().and_then(|id| profiles.find(id));
        // "Differs from profile" compares the fixture set too, derived on every read (`0102`
        // criterion 29).
        let set = self.active_fixture_set();
        let state = match record {
            None => Some("missing"),
            Some(record) if differs(copy, active.page, record) || record.fixtures != set => {
                Some("differs")
            }
            Some(_) => None,
        };
        let user = record.filter(|record| !record.is_builtin());
        let connection = user.map(|record| {
            json!({
                "connector_address": record.connection.connector_address,
                "controller_port": record.connection.controller_port,
                "baud_rate": record.connection.baud_rate.get(),
            })
        });
        Some(json!({
            "copy": {
                "id": copy.id,
                "name": copy.values.name,
                "origin": copy.values.origin.name(),
                "max_feed": copy.values.max_feed.get(),
                "max_power": copy.values.max_power.get(),
                "laser_mode": copy.values.laser_mode,
                "width": active.page.width.as_mm(),
                "height": active.page.height.as_mm(),
            },
            "builtin": copy.id.as_deref() == Some(GENERIC_LASER_ID),
            "state": state,
            "can_update": state == Some("differs") && user.is_some(),
            "fixtures_in_profile": record.map_or(0, |record| record.fixtures.len()),
            "name_notice": active.read.name_damaged.then_some(DAMAGED_NAME_NOTICE),
            "connection": connection,
            "library_notices": profiles.load_notices(),
            "profiles": profile_rows(profiles, copy.id.as_deref()),
        }))
    }

    /// The machine list of the Add page group (`specs/0098` criterion 1): every profile, the one
    /// a new laser page starts with marked chosen (the last pick, else Generic laser).
    #[must_use]
    pub fn machine_profiles_json(&self) -> String {
        let start = self.initial_machine_profile();
        Value::Array(profile_rows(
            &self.manufacturing.machine.profiles,
            Some(start.id.as_str()),
        ))
        .to_string()
    }
}
