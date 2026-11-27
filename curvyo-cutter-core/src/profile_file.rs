//! Reads and writes one cutter profile record of kind `cutter` in `machines/`
//! (`specs/0159-cutting-plotter` criteria 2, 18, `adrs.md` PL2) on the shared record envelope
//! (ADR 0097 §4), and merges a synced record by the cutter reader's outcome.

use curvyo_library_core::{
    BaudRate, Connection, EnvelopeError, Family, MergeReason, RecordMerge, merge_record,
    open_family, pretty, record_file_name, record_id,
};
use serde::{Deserialize, Serialize};

use crate::mat::MatSize;
use crate::numbers::{PressurePct, SpeedMmS};
use crate::profile::{
    CutterProfile, CutterValues, ForceRange, OriginCorner, OutputFormat, WorkArea,
};

/// The record format this build writes: 9, one above the laser's 8, so an older build skips a
/// cutter record as newer (PL2).
pub const RECORD_FORMAT_VERSION: u64 = 9;
/// The largest record text that is read.
pub const MAX_RECORD_BYTES: usize = 64 * 1024;

/// Why a record was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RecordError {
    /// Not a valid cutter record: skipped, counted, left untouched.
    #[error("the record is damaged")]
    Damaged,
    /// Written by a newer build: skipped and never overwritten.
    #[error("the record is from a newer version")]
    Newer,
    /// A record of another machine family: that family's reader reads it.
    #[error("the record is of another machine family")]
    OtherFamily,
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
    origin: String,
    output_format: String,
    force_min: u64,
    force_max: u64,
    max_speed_mm_s: SpeedMmS,
    default_pressure_pct: PressurePct,
    default_speed_mm_s: SpeedMmS,
    mats: Vec<String>,
    connection: ConnectionJson,
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

/// The profile the record file `file_name` holds.
///
/// # Errors
/// [`RecordError::OtherFamily`] for a record of another known family; [`RecordError::Newer`] for
/// a `format_version` above 9 or an unknown `kind`; [`RecordError::Damaged`] for a text over
/// 64 KiB, a JSON or type error, a missing or unknown key, a value out of its range, an unknown
/// spelling, an id that differs from the file name, or invalid values. Nothing is written on read.
pub fn read_record(file_name: &str, text: &str) -> Result<CutterProfile, RecordError> {
    let (value, _) = open_family(
        text,
        MAX_RECORD_BYTES,
        Family::Cutter,
        RECORD_FORMAT_VERSION,
        RECORD_FORMAT_VERSION,
    )
    .map_err(|error| match error {
        EnvelopeError::Newer => RecordError::Newer,
        EnvelopeError::OtherFamily(_) => RecordError::OtherFamily,
        _ => RecordError::Damaged,
    })?;
    let json: RecordJson = serde_json::from_value(value).map_err(|_| RecordError::Damaged)?;
    if json.kind != Family::Cutter.key() || record_id(file_name) != Some(json.id.as_str()) {
        return Err(RecordError::Damaged);
    }
    let mats = json
        .mats
        .iter()
        .map(|key| MatSize::parse(key))
        .collect::<Option<Vec<_>>>()
        .ok_or(RecordError::Damaged)?;
    let values = CutterValues {
        name: json.name,
        work_area: WorkArea::from_mm(json.work_area_mm.width, json.work_area_mm.height)
            .ok_or(RecordError::Damaged)?,
        origin: OriginCorner::parse(&json.origin).ok_or(RecordError::Damaged)?,
        output_format: OutputFormat::parse(&json.output_format).ok_or(RecordError::Damaged)?,
        force: ForceRange::new(json.force_min, json.force_max).ok_or(RecordError::Damaged)?,
        max_speed: json.max_speed_mm_s,
        default_pressure: json.default_pressure_pct,
        default_speed: json.default_speed_mm_s,
        mats,
        connection: Connection {
            connector_address: json.connection.connector_address,
            controller_port: json.connection.controller_port,
            baud_rate: json.connection.baud_rate,
        },
    };
    if !values.is_valid() {
        return Err(RecordError::Damaged);
    }
    Ok(CutterProfile {
        id: json.id,
        modified: json.modified,
        values,
    })
}

/// The record text of `profile`: canonical, so read, write, read is the identity.
#[must_use]
pub fn write_record(profile: &CutterProfile) -> String {
    let v = &profile.values;
    pretty(&RecordJson {
        format_version: RECORD_FORMAT_VERSION,
        id: profile.id.clone(),
        kind: Family::Cutter.key().to_owned(),
        modified: profile.modified,
        name: v.name.clone(),
        work_area_mm: AreaJson {
            width: v.work_area.width_mm(),
            height: v.work_area.height_mm(),
        },
        origin: v.origin.key().to_owned(),
        output_format: v.output_format.key().to_owned(),
        force_min: u64::from(v.force.min()),
        force_max: u64::from(v.force.max()),
        max_speed_mm_s: v.max_speed,
        default_pressure_pct: v.default_pressure,
        default_speed_mm_s: v.default_speed,
        mats: v.mats.iter().map(|m| m.key().to_owned()).collect(),
        connection: ConnectionJson {
            connector_address: v.connection.connector_address.clone(),
            controller_port: v.connection.controller_port.clone(),
            baud_rate: v.connection.baud_rate,
        },
    })
}

/// Merges the incoming cutter record `incoming` against `local`, the record file `id` holds, by
/// the cutter reader's outcome (criterion 18, ADR 0097 §4).
#[must_use]
pub fn merge_cutter_record(id: &str, incoming: &str, local: Option<&str>) -> RecordMerge {
    let name = record_file_name(id);
    let read = |text: &str| {
        read_record(&name, text)
            .map(|profile| profile.modified)
            .map_err(|error| match error {
                RecordError::Newer => MergeReason::Newer,
                RecordError::OtherFamily => MergeReason::OtherFamily,
                RecordError::Damaged => MergeReason::Damaged,
            })
    };
    merge_record(
        incoming,
        read(incoming),
        local.map(|local| (local, read(local))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::CRICUT_MAKER_3;

    const ID: &str = "0b5f6c2e-9d4a-4c1b-8e7f-1a2b3c4d5e6f";

    fn user() -> CutterProfile {
        let mut profile = CRICUT_MAKER_3.profile();
        profile.id = ID.to_owned();
        profile.modified = 1_700_000_000_000;
        profile.values.name = "My Maker".to_owned();
        profile
    }

    #[test]
    fn a_record_writes_format_9_of_kind_cutter_and_reads_back() {
        let profile = user();
        let text = write_record(&profile);
        assert!(text.contains("\"format_version\": 9"), "{text}");
        assert!(text.contains("\"kind\": \"cutter\""));
        let read = read_record(&record_file_name(ID), &text).unwrap();
        assert_eq!(read, profile);
        assert_eq!(write_record(&read), text);
    }

    #[test]
    fn a_laser_record_is_another_family_and_a_damaged_one_is_damaged() {
        let name = record_file_name(ID);
        let laser =
            format!(r#"{{"format_version": 8, "id": "{ID}", "kind": "laser", "modified": 1}}"#);
        assert_eq!(read_record(&name, &laser), Err(RecordError::OtherFamily));
        let text = write_record(&user()).replace("\"12x24\"", "\"9x9\"");
        assert_eq!(read_record(&name, &text), Err(RecordError::Damaged));
        let newer =
            write_record(&user()).replace("\"format_version\": 9", "\"format_version\": 10");
        assert_eq!(read_record(&name, &newer), Err(RecordError::Newer));
    }

    #[test]
    fn the_newer_record_wins_a_merge() {
        let local = user();
        let mut incoming = user();
        incoming.modified += 1;
        incoming.values.name = "From the other device".to_owned();
        let (local, incoming) = (write_record(&local), write_record(&incoming));
        assert_eq!(
            merge_cutter_record(ID, &incoming, Some(&local)),
            RecordMerge::IncomingWins {
                loser: Some(local.clone())
            }
        );
        let laser = format!(r#"{{"format_version": 8, "id": "{ID}", "kind": "laser"}}"#);
        assert_eq!(
            merge_cutter_record(ID, &laser, Some(&local)),
            RecordMerge::SkipIncoming {
                reason: MergeReason::OtherFamily
            }
        );
    }
}
