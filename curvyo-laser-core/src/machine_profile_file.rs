//! Reads and writes one machine profile record and the last-pick file (ADR 0038 §2, §3, §6), on
//! the shared record envelope (ADR 0055 §7).

use curvyo_document_core::DocumentSize;
use serde::{Deserialize, Serialize};

use crate::machine_profile::{
    BaudRate, Connection, DefaultNumbers, FeedRate, MAX_CONNECTION_CHARS, MachineProfile,
    OriginCorner, OutputFormat, PowerValue, ProfileValues, is_valid_name, is_valid_work_side,
};
use crate::machine_profile_sets::{SetJson, read_sets, write_sets};
use crate::material::PowerPercent;
use crate::record_envelope::{EnvelopeError, open, pretty, record_id};

/// The record format this build writes: 7 adds `default_power_pct` and `default_speed_mm_min`
/// after `output_format` (`specs/0150` adrs.md decision 2), both always written; formats 5 and 6
/// read the built-in pair, the speed limited to the max feed. Format 6 adds `air_assist` and
/// `output_format` after `motorised_z` (ADR 0065 §3), both always written. Format 5 holds the named `fixture_sets`
/// (ADR 0060 §1), the only home of a set: every set, fixture and job carries its record `id`,
/// sets and fixtures carry `locked`, and a set has no `active`. Format 5 is still read (air
/// assist off, GRBL) and not rewritten on read; formats 1 to 4 are older and skipped (ADR 0060
/// §3).
pub const RECORD_FORMAT_VERSION: u64 = 7;
/// The oldest record format this build reads: the named sets of format 5.
const RECORD_FORMAT_SETS: u64 = 5;
/// The record format of air assist and the output format (ADR 0065 §3).
const RECORD_FORMAT_OUTPUT: u64 = 6;
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
    /// Format 6 only (ADR 0065 §3); in a format 5 record it is an unknown key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    air_assist: Option<bool>,
    /// Format 6 only, the spelling of an [`OutputFormat`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    output_format: Option<String>,
    /// Format 7 only (`specs/0150`): Default power in percent, one decimal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_power_pct: Option<f64>,
    /// Format 7 only: Default speed in mm/min, at most the max feed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_speed_mm_min: Option<u64>,
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
/// [`RecordError::Newer`] for a `format_version` above 7; [`RecordError::Older`] for formats 1
/// to 4; [`RecordError::Damaged`] for a text over 256 KiB, a JSON or type error, a missing or
/// unknown key (`fixtures` included; `air_assist` or `output_format` in a format 5 record, a
/// default key in a format 5 or 6 record), an `output_format` this build does not know, a value
/// out of its range (a Default speed above the max feed among them), a kind other than `laser`,
/// an id that differs from the file name, or damaged fixture sets (ADR 0060 §2). A format 5
/// record reads air assist off and GRBL (ADR 0065 §3); formats 5 and 6 read the built-in default
/// pair. Nothing is written on read.
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
    if version < RECORD_FORMAT_SETS {
        return Err(RecordError::Older);
    }
    let record: RecordJson = serde_json::from_value(value).map_err(|_| RecordError::Damaged)?;
    let (air_assist, output_format) = output_values(version, &record)?;
    let defaults = default_values(version, &record)?;
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
            air_assist,
            output_format,
            defaults,
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

/// Air assist and the output format of a record of `version` (ADR 0065 §3): format 5 has
/// neither key and reads off and GRBL; format 6 has both, with a known output format.
fn output_values(version: u64, record: &RecordJson) -> Result<(bool, OutputFormat), RecordError> {
    match (version, record.air_assist, record.output_format.as_deref()) {
        (RECORD_FORMAT_SETS, None, None) => Ok((false, OutputFormat::Grbl)),
        (RECORD_FORMAT_OUTPUT.., Some(air_assist), Some(format)) => OutputFormat::parse(format)
            .map(|format| (air_assist, format))
            .ok_or(RecordError::Damaged),
        _ => Err(RecordError::Damaged),
    }
}

/// The default pair of a record of `version` (`specs/0150` adrs.md decision 2): formats 5 and 6
/// have neither key and read the built-in pair limited to the max feed; format 7 has both, power
/// 1 to 100 with one decimal, speed 1 to the record's max feed.
fn default_values(version: u64, record: &RecordJson) -> Result<DefaultNumbers, RecordError> {
    let max_feed = record.max_feed_mm_per_min;
    match (
        version,
        record.default_power_pct,
        record.default_speed_mm_min,
    ) {
        (RECORD_FORMAT_SETS | RECORD_FORMAT_OUTPUT, None, None) => {
            Ok(DefaultNumbers::BUILT_IN.limited_to(max_feed))
        }
        (RECORD_FORMAT_VERSION, Some(power), Some(speed)) => {
            let power = PowerPercent::from_percent(power).ok_or(RecordError::Damaged)?;
            let speed = FeedRate::new(speed)
                .filter(|speed| speed.get() <= max_feed.get())
                .ok_or(RecordError::Damaged)?;
            Ok(DefaultNumbers { power, speed })
        }
        _ => Err(RecordError::Damaged),
    }
}

/// The canonical text of a record, always format 7 with `air_assist`, `output_format`,
/// `default_power_pct` and `default_speed_mm_min` after `motorised_z` and its `fixture_sets` after `connection` (ADR 0060 §1, §4; ADR 0065 §3): fixed
/// key order, two-space indent, trailing newline, so read, write, read is the identity.
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
        air_assist: Some(profile.values.air_assist),
        output_format: Some(profile.values.output_format.as_str().to_owned()),
        default_power_pct: Some(profile.values.defaults.power.percent()),
        default_speed_mm_min: Some(profile.values.defaults.speed.get().into()),
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

    /// The two default keys as a format 7 record writes them for the built-in pair.
    const PAIR: &str = "  \"default_power_pct\": 10.0,\n  \"default_speed_mm_min\": 1000,\n";

    #[test]
    fn a_new_profile_writes_format_7_with_empty_sets_and_reads_back() {
        let mut profile = MachineProfile::generic_laser();
        ID.clone_into(&mut profile.id);
        let text = write_record(&profile);
        assert!(text.starts_with("{\n  \"format_version\": 7,"));
        assert!(text.contains(&format!(
            "  \"motorised_z\": false,\n  \"air_assist\": false,\n  \"output_format\": \"grbl\",\n{PAIR}  \"connection\""
        )));
        assert!(text.ends_with("  },\n  \"fixture_sets\": []\n}\n"));
        assert_eq!(read_record(&record_file_name(ID), &text), Ok(profile));
    }

    #[test]
    fn formats_1_to_4_are_older_and_above_7_newer() {
        let mut profile = MachineProfile::generic_laser();
        ID.clone_into(&mut profile.id);
        let text = write_record(&profile);
        let name = record_file_name(ID);
        for version in 1..=4 {
            let older = text.replacen(
                "\"format_version\": 7",
                &format!("\"format_version\": {version}"),
                1,
            );
            assert_eq!(read_record(&name, &older), Err(RecordError::Older));
        }
        let newer = text.replacen("\"format_version\": 7", "\"format_version\": 8", 1);
        assert_eq!(read_record(&name, &newer), Err(RecordError::Newer));
        let zero = text.replacen("\"format_version\": 7", "\"format_version\": 0", 1);
        assert_eq!(read_record(&name, &zero), Err(RecordError::Damaged));
    }

    #[test]
    fn format_5_reads_air_assist_off_and_grbl_and_refuses_either_new_key() {
        let mut profile = MachineProfile::generic_laser();
        ID.clone_into(&mut profile.id);
        profile.values.air_assist = true;
        let text = write_record(&profile).replacen(PAIR, "", 1);
        let name = record_file_name(ID);
        let five = text
            .replacen("\"format_version\": 7", "\"format_version\": 5", 1)
            .replacen(
                "  \"air_assist\": true,\n  \"output_format\": \"grbl\",\n",
                "",
                1,
            );
        let read = read_record(&name, &five).unwrap();
        assert!(!read.values.air_assist);
        assert_eq!(read.values.output_format, OutputFormat::Grbl);
        let six = text.replacen("\"format_version\": 7", "\"format_version\": 6", 1);
        for key in [
            "  \"air_assist\": true,\n",
            "  \"output_format\": \"grbl\",\n",
        ] {
            let with_key = text
                .replacen("\"format_version\": 7", "\"format_version\": 5", 1)
                .replacen(key, "", 1);
            assert_eq!(
                read_record(&name, &with_key),
                Err(RecordError::Damaged),
                "{key}"
            );
            let without = six.replacen(key, "", 1);
            assert_eq!(
                read_record(&name, &without),
                Err(RecordError::Damaged),
                "{key}"
            );
        }
        let marlin = six.replacen("\"grbl\"", "\"marlin\"", 1);
        assert_eq!(read_record(&name, &marlin), Err(RecordError::Damaged));
        assert_eq!(read_record(&name, &six).unwrap().values, profile.values);
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
