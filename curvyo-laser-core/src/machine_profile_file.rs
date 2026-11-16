//! Reads and writes one machine profile record and the last-pick file (ADR 0038 §2, §3, §6), on
//! the shared record envelope (ADR 0055 §7).

use curvyo_document_core::DocumentSize;
use serde::{Deserialize, Serialize};

use crate::machine_profile::{
    BaudRate, Connection, FeedRate, MAX_CONNECTION_CHARS, MachineProfile, OriginCorner, PowerValue,
    ProfileValues, is_valid_name, is_valid_work_side,
};
use crate::machine_profile_sets::{SetJson, read_sets, write_sets};
use crate::record_envelope::{EnvelopeError, open, pretty, record_id};

/// The record format this build writes and the only one it reads: 5 holds the named
/// `fixture_sets` (ADR 0060 §1), the only home of a set: every set, fixture and job carries its
/// record `id`, sets and fixtures carry `locked`, and a set has no `active` (the page's On this
/// page stays in the project file). Formats 1 to 4 are older and skipped (ADR 0060 §3).
pub const RECORD_FORMAT_VERSION: u64 = 5;
/// The oldest record format there is; formats from it below [`RECORD_FORMAT_VERSION`] are older.
const RECORD_FORMAT_FIRST: u64 = 1;
/// The format of `machine-choice.json`, unchanged by the record's formats.
const CHOICE_FORMAT_VERSION: u64 = 1;
/// The largest record text that is read; a longer one is damage (256 KiB since format 5, as the
/// maximal sets in the pretty form are above 128 KiB).
pub const MAX_RECORD_BYTES: usize = 256 * 1024;
/// The only machine family this build stores.
const KIND_LASER: &str = "laser";

/// Why a record was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RecordError {
    /// Not a valid record of this format: skipped, counted, left untouched.
    #[error("the record is damaged")]
    Damaged,
    /// Written by a newer build: skipped and never overwritten (ADR 0004 §9).
    #[error("the record is from a newer version")]
    Newer,
    /// Of a format before 5: skipped, counted, left untouched, never read into sets (ADR 0060 §3).
    #[error("the record has an older format")]
    Older,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordJson {
    format_version: u64,
    id: String,
    kind: String,
    modified: i64,
    name: String,
    work_area_mm: AreaJson,
    origin: OriginCorner,
    max_feed_mm_per_min: FeedRate,
    max_power: PowerValue,
    laser_mode: bool,
    motorised_z: bool,
    connection: ConnectionJson,
    fixture_sets: Vec<SetJson>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AreaJson {
    width: f64,
    height: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionJson {
    connector_address: String,
    controller_port: String,
    baud_rate: BaudRate,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChoiceJson {
    format_version: u64,
    last_picked: String,
}

/// The profile the record file `file_name` holds.
///
/// # Errors
/// [`RecordError::Newer`] for a `format_version` above 5; [`RecordError::Older`] for formats 1
/// to 4; [`RecordError::Damaged`] for a text over 256 KiB, a JSON or type error, a missing or
/// unknown key (`fixtures` included), a value out of its range, a kind other than `laser`, an id
/// that differs from the file name, or damaged fixture sets (ADR 0060 §2). Nothing is written on
/// read.
pub fn read_record(file_name: &str, text: &str) -> Result<MachineProfile, RecordError> {
    let (value, version) = open(
        text,
        MAX_RECORD_BYTES,
        RECORD_FORMAT_FIRST,
        RECORD_FORMAT_VERSION,
    )
    .map_err(|error| match error {
        EnvelopeError::Newer => RecordError::Newer,
        EnvelopeError::TooLarge | EnvelopeError::NotJson | EnvelopeError::BadVersion => {
            RecordError::Damaged
        }
    })?;
    if version < RECORD_FORMAT_VERSION {
        return Err(RecordError::Older);
    }
    let record: RecordJson = serde_json::from_value(value).map_err(|_| RecordError::Damaged)?;
    let fixture_sets = read_sets(record.fixture_sets).ok_or(RecordError::Damaged)?;
    let valid = record_id(file_name) == Some(record.id.as_str())
        && record.kind == KIND_LASER
        && is_valid_name(&record.name)
        && is_valid_work_side(record.work_area_mm.width)
        && is_valid_work_side(record.work_area_mm.height)
        && record.connection.connector_address.chars().count() <= MAX_CONNECTION_CHARS
        && record.connection.controller_port.chars().count() <= MAX_CONNECTION_CHARS;
    if !valid {
        return Err(RecordError::Damaged);
    }
    Ok(MachineProfile {
        id: record.id,
        modified: record.modified,
        values: ProfileValues {
            name: record.name,
            origin: record.origin,
            max_feed: record.max_feed_mm_per_min,
            max_power: record.max_power,
            laser_mode: record.laser_mode,
            motorised_z: record.motorised_z,
        },
        work_area: DocumentSize::from_mm(record.work_area_mm.width, record.work_area_mm.height),
        connection: Connection {
            connector_address: record.connection.connector_address,
            controller_port: record.connection.controller_port,
            baud_rate: record.connection.baud_rate,
        },
        fixture_sets,
    })
}

/// The canonical text of a record, always format 5 with its `fixture_sets` after `connection`
/// (ADR 0060 §1, §4): fixed key order, two-space indent, trailing newline, so read, write, read is
/// the identity.
#[must_use]
pub fn write_record(profile: &MachineProfile) -> String {
    let record = RecordJson {
        format_version: RECORD_FORMAT_VERSION,
        id: profile.id.clone(),
        kind: KIND_LASER.to_owned(),
        modified: profile.modified,
        name: profile.values.name.clone(),
        work_area_mm: AreaJson {
            width: profile.work_area.width.as_mm(),
            height: profile.work_area.height.as_mm(),
        },
        origin: profile.values.origin,
        max_feed_mm_per_min: profile.values.max_feed,
        max_power: profile.values.max_power,
        laser_mode: profile.values.laser_mode,
        motorised_z: profile.values.motorised_z,
        connection: ConnectionJson {
            connector_address: profile.connection.connector_address.clone(),
            controller_port: profile.connection.controller_port.clone(),
            baud_rate: profile.connection.baud_rate,
        },
        fixture_sets: write_sets(&profile.fixture_sets),
    };
    pretty(&record)
}

/// The profile id `machine-choice.json` names as last picked; `None` for a missing, damaged or
/// newer file.
#[must_use]
pub fn read_choice(text: &str) -> Option<String> {
    serde_json::from_str::<ChoiceJson>(text)
        .ok()
        .filter(|choice| choice.format_version == CHOICE_FORMAT_VERSION)
        .map(|choice| choice.last_picked)
}

/// The text of `machine-choice.json` naming `id`.
#[must_use]
pub fn write_choice(id: &str) -> String {
    pretty(&ChoiceJson {
        format_version: CHOICE_FORMAT_VERSION,
        last_picked: id.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record_envelope::record_file_name;

    const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

    #[test]
    fn a_new_profile_writes_format_5_with_empty_sets_and_reads_back() {
        let mut profile = MachineProfile::generic_laser();
        ID.clone_into(&mut profile.id);
        let text = write_record(&profile);
        assert!(text.starts_with("{\n  \"format_version\": 5,"));
        assert!(text.ends_with("  },\n  \"fixture_sets\": []\n}\n"));
        assert_eq!(read_record(&record_file_name(ID), &text), Ok(profile));
    }

    #[test]
    fn formats_1_to_4_are_older_and_above_5_newer() {
        let mut profile = MachineProfile::generic_laser();
        ID.clone_into(&mut profile.id);
        let text = write_record(&profile);
        let name = record_file_name(ID);
        for version in 1..=4 {
            let older = text.replacen(
                "\"format_version\": 5",
                &format!("\"format_version\": {version}"),
                1,
            );
            assert_eq!(read_record(&name, &older), Err(RecordError::Older));
        }
        let newer = text.replacen("\"format_version\": 5", "\"format_version\": 6", 1);
        assert_eq!(read_record(&name, &newer), Err(RecordError::Newer));
        let zero = text.replacen("\"format_version\": 5", "\"format_version\": 0", 1);
        assert_eq!(read_record(&name, &zero), Err(RecordError::Damaged));
    }

    #[test]
    fn the_choice_file_round_trips() {
        let text = write_choice(ID);
        assert_eq!(
            text,
            format!("{{\n  \"format_version\": 1,\n  \"last_picked\": \"{ID}\"\n}}\n")
        );
        assert_eq!(read_choice(&text).as_deref(), Some(ID));
        assert_eq!(read_choice("{"), None);
        assert_eq!(
            read_choice(r#"{"format_version": 2, "last_picked": "x"}"#),
            None
        );
    }
}
