//! Reads and writes one machine profile record and the last-pick file (ADR 0038 §2, §3, §6), on
//! the shared record envelope (ADR 0055 §7).

use curvyo_document_core::DocumentSize;
use serde::{Deserialize, Serialize};

use crate::machine_profile::{
    BaudRate, Connection, FeedRate, MAX_CONNECTION_CHARS, MachineProfile, OriginCorner, PowerValue,
    ProfileValues, is_valid_name, is_valid_work_side,
};
use crate::machine_profile_fixtures::{FixtureJson, read_set, write_set};
use crate::record_envelope::{EnvelopeError, open, pretty, record_id};

/// The record format this build writes and the newest it reads: 2 added the fixture set
/// (ADR 0050 §1), 3 the fixture jobs of each entry (ADR 0052 §7), 4 adds `motorised_z`
/// (ADR 0056 §7).
pub const RECORD_FORMAT_VERSION: u64 = 4;
/// The record format with fixture jobs and without `motorised_z`, still read (ADR 0056 §7).
const RECORD_FORMAT_NO_Z: u64 = 3;
/// The record format with a fixture set without jobs, still read (ADR 0052 §7).
const RECORD_FORMAT_NO_JOBS: u64 = 2;
/// The record format without a fixture set, still read (ADR 0050 §2).
const RECORD_FORMAT_NO_FIXTURES: u64 = 1;
/// The format of `machine-choice.json`, unchanged by the record's formats 2 and 3.
const CHOICE_FORMAT_VERSION: u64 = 1;
/// The largest record text that is read; a longer one is damage (128 KiB since format 3, as a
/// maximal set in the pretty form is above 64 KiB).
pub const MAX_RECORD_BYTES: usize = 128 * 1024;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    motorised_z: Option<bool>,
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

/// The profile the record file `file_name` holds.
///
/// # Errors
/// [`RecordError::Newer`] for a `format_version` above 4; [`RecordError::Damaged`] for a text over
/// 128 KiB, a JSON or type error, an unknown key (`fixtures` in a format 1 record, `jobs` in a
/// format 2 entry, `output` in a job, `motorised_z` in a format 1 to 3 record), a value out of its
/// range, a kind other than `laser`, an id that differs from the file name, a format 2 to 4 record
/// whose fixture set is missing or damaged (ADR 0050 §2, ADR 0052 §7), or a format 4 record
/// without `motorised_z`. A format 1 record reads with an empty set, a format 2 record with no
/// jobs, a format 1 to 3 record without a motorised Z axis (ADR 0056 §7). Nothing is written on
/// read.
pub fn read_record(file_name: &str, text: &str) -> Result<MachineProfile, RecordError> {
    let (value, version) = open(
        text,
        MAX_RECORD_BYTES,
        RECORD_FORMAT_NO_FIXTURES,
        RECORD_FORMAT_VERSION,
    )
    .map_err(|error| match error {
        EnvelopeError::Newer => RecordError::Newer,
        EnvelopeError::TooLarge | EnvelopeError::NotJson | EnvelopeError::BadVersion => {
            RecordError::Damaged
        }
    })?;
    let record: RecordJson = serde_json::from_value(value).map_err(|_| RecordError::Damaged)?;
    let fixtures = match (version, record.fixtures) {
        (RECORD_FORMAT_NO_FIXTURES, None) => Vec::new(),
        (RECORD_FORMAT_NO_JOBS, Some(set)) => read_set(set, false).ok_or(RecordError::Damaged)?,
        (RECORD_FORMAT_NO_Z | RECORD_FORMAT_VERSION, Some(set)) => {
            read_set(set, true).ok_or(RecordError::Damaged)?
        }
        _ => return Err(RecordError::Damaged),
    };
    let motorised_z = match (version, record.motorised_z) {
        (RECORD_FORMAT_VERSION, Some(on)) => on,
        (RECORD_FORMAT_VERSION, None) | (_, Some(_)) => return Err(RecordError::Damaged),
        (_, None) => false,
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
            motorised_z,
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

/// The canonical text of a record, always format 4 with `motorised_z`, its `fixtures` and their
/// `jobs` (ADR 0050 §3, ADR 0052 §7, ADR 0056 §7): fixed key
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
        motorised_z: Some(profile.values.motorised_z),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record_envelope::record_file_name;

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
            "format2_jobs_key.json",
            "format3_fixture_jobs.json",
            "format3_max_set.json",
            "format3_damaged_token.json",
            "format3_output_key.json",
            "format3_motorised_z_key.json",
            "format4_motorised_z.json",
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
    fn a_format_1_record_reads_with_an_empty_set_and_any_write_is_format_4() {
        let text = fixture("format1_full_record.json");
        let profile = read_record(&record_file_name(ID), &text).unwrap();
        assert_eq!(profile.values.name, "K40 in the garage");
        assert_eq!(profile.connection.controller_port, "/dev/ttyUSB0");
        assert_eq!(profile.fixtures, Vec::new());
        let written = write_record(&profile);
        assert!(written.starts_with("{\n  \"format_version\": 4,"));
        assert!(written.contains("\n  \"motorised_z\": false,\n  \"connection\""));
        assert!(!profile.values.motorised_z);
        assert!(written.ends_with("  },\n  \"fixtures\": []\n}\n"));
        assert_eq!(
            read_record(&record_file_name(ID), &written).unwrap(),
            profile
        );
    }

    #[test]
    fn a_format_2_set_reads_without_jobs_and_writes_format_4() {
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
        assert!(profile.fixtures.iter().all(|f| f.jobs.is_empty()));
        let written = write_record(&profile);
        assert!(written.starts_with("{\n  \"format_version\": 4,"));
        assert_eq!(written.matches("\"jobs\": []").count(), 4);
        assert_eq!(
            read_record(&record_file_name(ID), &written).unwrap(),
            profile
        );
    }

    #[test]
    fn a_format_3_set_reads_its_jobs_without_a_motorised_z_axis_and_writes_format_4() {
        let text = fixture("format3_fixture_jobs.json");
        let profile = read_record(&record_file_name(ID), &text).unwrap();
        let holder = &profile.fixtures[1];
        assert_eq!(holder.jobs.len(), 2);
        assert_eq!(holder.jobs[0].kind, "engrave_outline");
        assert_eq!(holder.jobs[0].name, "Engrave + Outline 1");
        assert_eq!(holder.jobs[0].targets.len(), 3);
        assert_eq!(holder.jobs[0].targets[0].slot, None, "the board first");
        assert_eq!(holder.jobs[0].targets[2].slot, Some(1));
        assert!(!holder.jobs[1].show && holder.jobs[1].targets.is_empty());
        assert_eq!(
            profile.fixtures[3].jobs[0].targets[1].suffix.as_deref(),
            Some("thread")
        );
        assert!(!profile.values.motorised_z, "format 3 reads it as off");
        let format_4 = text
            .replacen("\"format_version\": 3", "\"format_version\": 4", 1)
            .replacen(
                "\"laser_mode\": false,\n",
                "\"laser_mode\": false,\n  \"motorised_z\": false,\n",
                1,
            );
        assert_eq!(write_record(&profile), format_4);
        assert!(!text.contains("output"), "a profile holds no Output");
    }

    #[test]
    fn a_format_4_record_reads_its_motorised_z_axis_and_writes_back_byte_identical() {
        let text = fixture("format4_motorised_z.json");
        let profile = read_record(&record_file_name(ID), &text).unwrap();
        assert!(profile.values.motorised_z);
        assert_eq!(profile.fixtures[1].jobs.len(), 2);
        assert_eq!(write_record(&profile), text);
        let without = text.replacen("  \"motorised_z\": true,\n", "", 1);
        assert_eq!(
            read_record(&record_file_name(ID), &without),
            Err(RecordError::Damaged),
            "format 4 always has it"
        );
        let not_bool = text.replacen("\"motorised_z\": true", "\"motorised_z\": 1", 1);
        assert_eq!(
            read_record(&record_file_name(ID), &not_bool),
            Err(RecordError::Damaged)
        );
        assert_eq!(
            read_record(
                &record_file_name(ID),
                &fixture("format3_motorised_z_key.json")
            ),
            Err(RecordError::Damaged),
            "the key in a format 3 record is unknown"
        );
    }

    #[test]
    fn the_maximal_set_reads_and_fits_the_record_limit() {
        let text = fixture("format3_max_set.json");
        assert!(text.len() > 64 * 1024 && text.len() <= MAX_RECORD_BYTES);
        let profile = read_record(&record_file_name(ID), &text).unwrap();
        let jobs: usize = profile.fixtures.iter().map(|f| f.jobs.len()).sum();
        let targets: usize = profile
            .fixtures
            .iter()
            .flat_map(|f| &f.jobs)
            .map(|j| j.targets.len())
            .sum();
        assert_eq!(
            (profile.fixtures.len(), jobs, targets),
            (
                crate::MAX_SET_FIXTURES,
                crate::MAX_SET_JOBS,
                crate::MAX_SET_TARGETS
            )
        );
        let one_more = text.replacen(
            "\"targets\": [\n",
            "\"targets\": [\n            \"board\",\n",
            1,
        );
        assert_eq!(
            read_record(&record_file_name(ID), &one_more),
            Err(RecordError::Damaged),
            "201 targets"
        );
    }

    #[test]
    fn a_bad_token_an_output_key_or_jobs_in_format_2_damage_the_record() {
        for case in [
            "format3_damaged_token.json",
            "format3_output_key.json",
            "format2_jobs_key.json",
        ] {
            assert_eq!(
                read_record(&record_file_name(ID), &fixture(case)),
                Err(RecordError::Damaged),
                "{case}"
            );
        }
        let text = fixture("format3_fixture_jobs.json");
        for (from, to, why) in [
            ("\"slot:1\"", "\"slot:2\"", "a slot out of range"),
            ("\"slot:1\"", "\"slot:0\"", "a duplicate"),
            (
                "\"kind\": \"score\"",
                "\"kind\": \"weld\"",
                "an unknown job kind",
            ),
            (
                "      \"jobs\": []\n",
                "      \"slots_left\": []\n",
                "a format 3 entry without jobs",
            ),
        ] {
            let damaged = text.replacen(from, to, 1);
            assert_ne!(damaged, text, "{why}");
            assert_eq!(
                read_record(&record_file_name(ID), &damaged),
                Err(RecordError::Damaged),
                "{why}"
            );
        }
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
