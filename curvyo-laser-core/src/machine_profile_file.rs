//! Reads and writes one machine profile record and the last-pick file (ADR 0038 §2, §3, §6).

use curvyo_document_core::DocumentSize;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::machine_profile::{
    BaudRate, Connection, FeedRate, MAX_CONNECTION_CHARS, MachineProfile, OriginCorner, PowerValue,
    ProfileValues, is_profile_id, is_valid_name, is_valid_work_side,
};
use crate::machine_profile_fixtures::{FixtureJson, read_set, write_set};

/// The record format this build writes and the newest it reads: 2 adds the fixture set
/// (ADR 0050 §1).
pub const RECORD_FORMAT_VERSION: u64 = 2;
/// The record format without a fixture set, still read (ADR 0050 §2).
const RECORD_FORMAT_NO_FIXTURES: u64 = 1;
/// The format of `machine-choice.json`, unchanged by the record's format 2.
const CHOICE_FORMAT_VERSION: u64 = 1;
/// The largest record text that is read; a longer one is damage (64 KiB since format 2).
pub const MAX_RECORD_BYTES: usize = 64 * 1024;
/// The file name suffix of a record.
pub const RECORD_SUFFIX: &str = ".json";
/// The only machine family this build stores.
const KIND_LASER: &str = "laser";

/// Why a record was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordError {
    /// Not a valid record of this format: skipped, counted, left untouched.
    Damaged,
    /// Written by a newer build: skipped and never overwritten (ADR 0004 §9).
    Newer,
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Damaged => "the record is damaged",
            Self::Newer => "the record is from a newer version",
        })
    }
}

impl std::error::Error for RecordError {}

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
    connection: ConnectionJson,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fixtures: Option<Vec<FixtureJson>>,
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

/// The id a record file name `<uuid>.json` carries; `None` for any other name, which is ignored.
#[must_use]
pub fn record_id(file_name: &str) -> Option<&str> {
    file_name
        .strip_suffix(RECORD_SUFFIX)
        .filter(|id| is_profile_id(id))
}

/// The file name of the record `id`.
#[must_use]
pub fn record_file_name(id: &str) -> String {
    format!("{id}{RECORD_SUFFIX}")
}

/// The profile the record file `file_name` holds.
///
/// # Errors
/// [`RecordError::Newer`] for a `format_version` above 2; [`RecordError::Damaged`] for a text over
/// 64 KiB, a JSON or type error, an unknown key (`fixtures` in a format 1 record), a value out of
/// its range, a kind other than `laser`, an id that differs from the file name, or a format 2
/// record whose fixture set is missing or damaged (ADR 0050 §2). A format 1 record reads with an
/// empty set.
pub fn read_record(file_name: &str, text: &str) -> Result<MachineProfile, RecordError> {
    if text.len() > MAX_RECORD_BYTES {
        return Err(RecordError::Damaged);
    }
    let value: Value = serde_json::from_str(text).map_err(|_| RecordError::Damaged)?;
    let version = match value.get("format_version").and_then(Value::as_u64) {
        Some(version @ (RECORD_FORMAT_NO_FIXTURES | RECORD_FORMAT_VERSION)) => version,
        Some(version) if version > RECORD_FORMAT_VERSION => return Err(RecordError::Newer),
        _ => return Err(RecordError::Damaged),
    };
    let record: RecordJson = serde_json::from_value(value).map_err(|_| RecordError::Damaged)?;
    let fixtures = match (version, record.fixtures) {
        (RECORD_FORMAT_NO_FIXTURES, None) => Vec::new(),
        (RECORD_FORMAT_VERSION, Some(set)) => read_set(set).ok_or(RecordError::Damaged)?,
        _ => return Err(RecordError::Damaged),
    };
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
        },
        work_area: DocumentSize::from_mm(record.work_area_mm.width, record.work_area_mm.height),
        connection: Connection {
            connector_address: record.connection.connector_address,
            controller_port: record.connection.controller_port,
            baud_rate: record.connection.baud_rate,
        },
        fixtures,
    })
}

/// The canonical text of a record, always format 2 with its `fixtures` (ADR 0050 §3): fixed key
/// order, two-space indent, trailing newline, so read, write, read is the identity.
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
        connection: ConnectionJson {
            connector_address: profile.connection.connector_address.clone(),
            controller_port: profile.connection.controller_port.clone(),
            baud_rate: profile.connection.baud_rate,
        },
        fixtures: Some(write_set(&profile.fixtures)),
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

fn pretty(value: &impl Serialize) -> String {
    // invariant: plain structs of strings, numbers and booleans always serialise.
    #[allow(clippy::unwrap_used)]
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

    /// The golden record files, compiled in (this crate reads no file, ADR 0034 §1).
    fn fixture(name: &str) -> String {
        macro_rules! files {
            ($($file:literal),*) => {
                match name {
                    $($file => include_str!(concat!("../tests/fixtures/machine_profiles/", $file)),)*
                    _ => panic!("no fixture {name}"),
                }
            };
        }
        files!(
            "format1_full_record.json",
            "format2_fixtures.json",
            "format2_damaged_fixture.json",
            "format2_damaged_screw.json",
            "format2_damaged_matrix.json",
            "format2_unknown_kind.json",
            "format2_over_limit.json",
            "generic_laser_copy.json",
            "damaged_not_json.json",
            "damaged_wrong_type.json",
            "damaged_out_of_range.json",
            "damaged_unknown_key.json",
            "damaged_kind.json",
            "damaged_id_mismatch.json",
            "newer_format.json"
        )
        .to_owned()
    }

    #[test]
    fn a_format_1_record_reads_with_an_empty_set_and_any_write_is_format_2() {
        let text = fixture("format1_full_record.json");
        let profile = read_record(&record_file_name(ID), &text).unwrap();
        assert_eq!(profile.values.name, "K40 in the garage");
        assert_eq!(profile.connection.controller_port, "/dev/ttyUSB0");
        assert_eq!(profile.fixtures, Vec::new());
        let written = write_record(&profile);
        assert!(written.starts_with("{\n  \"format_version\": 2,"));
        assert!(written.ends_with("  },\n  \"fixtures\": []\n}\n"));
        assert_eq!(
            read_record(&record_file_name(ID), &written).unwrap(),
            profile
        );
    }

    #[test]
    fn a_format_2_set_reads_and_writes_back_byte_identical() {
        let text = fixture("format2_fixtures.json");
        let profile = read_record(&record_file_name(ID), &text).unwrap();
        let kinds: Vec<&str> = profile.fixtures.iter().map(|f| f.kind.as_str()).collect();
        assert_eq!(
            kinds,
            [
                "curvyo.laser/engraved_grid",
                "curvyo.laser/holder",
                "curvyo.laser/stop",
                "curvyo.laser/screw"
            ]
        );
        assert_eq!(profile.fixtures[3].params["thread_diameter_mm"], 6.0);
        assert!(profile.fixtures[3].slots.is_empty(), "a screw has no slots");
        assert_eq!(profile.fixtures[0].matrix, None, "the grid has no matrix");
        assert_eq!(profile.fixtures[1].slots.len(), 2);
        assert_eq!(profile.fixtures[1].slots[0].params["shape"], "circle");
        assert!(!profile.fixtures[2].active);
        assert_eq!(write_record(&profile), text);
    }

    #[test]
    fn a_damaged_set_damages_the_whole_record() {
        for case in [
            "format2_damaged_fixture.json",
            "format2_damaged_screw.json",
            "format2_damaged_matrix.json",
            "format2_unknown_kind.json",
            "format2_over_limit.json",
        ] {
            assert_eq!(
                read_record(&record_file_name(ID), &fixture(case)),
                Err(RecordError::Damaged),
                "{case}"
            );
        }
        let text_thread = fixture("format2_fixtures.json").replace(
            "\"thread_diameter_mm\": 6.0",
            "\"thread_diameter_mm\": \"M6\"",
        );
        assert_eq!(
            read_record(&record_file_name(ID), &text_thread),
            Err(RecordError::Damaged),
            "a text diameter damages the record"
        );
        let without_set = fixture("format1_full_record.json")
            .replace("\"format_version\": 1", "\"format_version\": 2");
        assert_eq!(
            read_record(&record_file_name(ID), &without_set),
            Err(RecordError::Damaged),
            "a format 2 record has its set"
        );
        let set_in_format_1 = fixture("format2_fixtures.json")
            .replace("\"format_version\": 2", "\"format_version\": 1");
        assert_eq!(
            read_record(&record_file_name(ID), &set_in_format_1),
            Err(RecordError::Damaged),
            "a format 1 record has no set"
        );
    }

    #[test]
    fn a_duplicated_built_in_reads_as_a_user_profile() {
        let text = fixture("generic_laser_copy.json");
        let name = "0f0e0d0c-0b0a-4908-8706-050403020100.json";
        let profile = read_record(name, &text).unwrap();
        assert_eq!(profile.values.name, "Generic laser copy");
        assert!(!profile.is_builtin());
        assert_eq!(read_record(name, &write_record(&profile)).unwrap(), profile);
    }

    #[test]
    fn every_damage_case_is_damaged_and_a_newer_record_is_newer() {
        let name = record_file_name(ID);
        for case in [
            "damaged_not_json.json",
            "damaged_wrong_type.json",
            "damaged_out_of_range.json",
            "damaged_unknown_key.json",
            "damaged_kind.json",
            "damaged_id_mismatch.json",
        ] {
            assert_eq!(
                read_record(&name, &fixture(case)),
                Err(RecordError::Damaged),
                "{case}"
            );
        }
        let big = format!(
            "{}{}",
            fixture("format1_full_record.json"),
            " ".repeat(MAX_RECORD_BYTES)
        );
        assert_eq!(read_record(&name, &big), Err(RecordError::Damaged));
        assert_eq!(
            read_record(&name, &fixture("newer_format.json")),
            Err(RecordError::Newer)
        );
    }

    #[test]
    fn only_uuid_file_names_are_records() {
        assert_eq!(record_id(&record_file_name(ID)), Some(ID));
        assert_eq!(record_id("not-a-uuid.json"), None);
        assert_eq!(record_id(&format!("{ID}.json.sync-conflict")), None);
        assert_eq!(record_id(ID), None);
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
