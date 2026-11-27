//! Reads and writes one CNC profile record of kind `cnc` in `machines/` (`specs/0161-cnc-output`
//! criterion 2, `adrs.md` CN1) on the shared record envelope (ADR 0097 §4), and merges a synced
//! record by this reader's outcome.

use curvyo_document_core::{DocumentSize, Length};
use curvyo_library_core::{
    BaudRate, Connection, EnvelopeError, Family, MergeReason, RecordMerge, merge_record,
    open_family, pretty, record_file_name, record_id,
};
use serde::{Deserialize, Serialize};

use crate::profile::{CncProfile, CncValues, OutputFormat};
use crate::units::{AccelMmS2, FeedMmMin, Rpm};

/// The record format this build writes: 9, one above the laser's 8, as the other families'.
pub const RECORD_FORMAT_VERSION: u64 = 9;
/// The largest record text that is read.
pub const MAX_RECORD_BYTES: usize = 64 * 1024;

/// Why a record was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RecordError {
    /// Not a valid CNC record: skipped, counted, left untouched.
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
    output_format: String,
    work_area_mm: AreaJson,
    z_travel_mm: f64,
    spindle_max_rpm: u64,
    max_rate_mm_min: [f64; 3],
    acceleration_mm_s2: [f64; 3],
    safe_height_mm: f64,
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

fn values_of(json: RecordJson) -> Option<(String, i64, CncValues)> {
    let rates = json.max_rate_mm_min.map(FeedMmMin::new);
    let accels = json.acceleration_mm_s2.map(AccelMmS2::new);
    let values = CncValues {
        name: json.name,
        output_format: OutputFormat::parse(&json.output_format)?,
        work_area: DocumentSize::from_mm(json.work_area_mm.width, json.work_area_mm.height),
        z_travel: Length::from_mm(json.z_travel_mm),
        spindle_max: Rpm::new(json.spindle_max_rpm)?,
        max_rate: [rates[0]?, rates[1]?, rates[2]?],
        acceleration: [accels[0]?, accels[1]?, accels[2]?],
        safe_height: Length::from_mm(json.safe_height_mm),
        connection: Connection {
            connector_address: json.connection.connector_address,
            controller_port: json.connection.controller_port,
            baud_rate: json.connection.baud_rate,
        },
    };
    values
        .is_valid()
        .then_some((json.id, json.modified, values))
}

/// The profile the record file `file_name` holds.
///
/// # Errors
/// [`RecordError::OtherFamily`] for a record of another known family; [`RecordError::Newer`] for
/// a `format_version` above 9 or an unknown `kind`; [`RecordError::Damaged`] for a text over
/// 64 KiB, a JSON or type error, a missing or unknown key, an unknown spelling, a value out of
/// its range, an id that differs from the file name, or invalid values. Nothing is written.
pub fn read_record(file_name: &str, text: &str) -> Result<CncProfile, RecordError> {
    let (value, _) = open_family(
        text,
        MAX_RECORD_BYTES,
        Family::Cnc,
        RECORD_FORMAT_VERSION,
        RECORD_FORMAT_VERSION,
    )
    .map_err(|error| match error {
        EnvelopeError::Newer => RecordError::Newer,
        EnvelopeError::OtherFamily(_) => RecordError::OtherFamily,
        _ => RecordError::Damaged,
    })?;
    let json: RecordJson = serde_json::from_value(value).map_err(|_| RecordError::Damaged)?;
    if json.kind != Family::Cnc.key() || record_id(file_name) != Some(json.id.as_str()) {
        return Err(RecordError::Damaged);
    }
    let (id, modified, values) = values_of(json).ok_or(RecordError::Damaged)?;
    Ok(CncProfile {
        id,
        modified,
        values,
    })
}

/// The record text of `profile`: canonical, so read, write, read is the identity.
#[must_use]
pub fn write_record(profile: &CncProfile) -> String {
    let v = &profile.values;
    pretty(&RecordJson {
        format_version: RECORD_FORMAT_VERSION,
        id: profile.id.clone(),
        kind: Family::Cnc.key().to_owned(),
        modified: profile.modified,
        name: v.name.clone(),
        output_format: v.output_format.key().to_owned(),
        work_area_mm: AreaJson {
            width: v.work_area.width.as_mm(),
            height: v.work_area.height.as_mm(),
        },
        z_travel_mm: v.z_travel.as_mm(),
        spindle_max_rpm: u64::from(v.spindle_max.get()),
        max_rate_mm_min: v.max_rate.map(FeedMmMin::get),
        acceleration_mm_s2: v.acceleration.map(AccelMmS2::get),
        safe_height_mm: v.safe_height.as_mm(),
        connection: ConnectionJson {
            connector_address: v.connection.connector_address.clone(),
            controller_port: v.connection.controller_port.clone(),
            baud_rate: v.connection.baud_rate,
        },
    })
}

/// Merges the incoming CNC record `incoming` against `local`, the record file `id` holds, by
/// this reader's outcome (criterion 2, ADR 0097 §4).
#[must_use]
pub fn merge_cnc_record(id: &str, incoming: &str, local: Option<&str>) -> RecordMerge {
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
    use crate::builtins::GENERIC_GRBL_ROUTER;

    const ID: &str = "0b5f6c2e-9d4a-4c1b-8e7f-1a2b3c4d5e6f";

    fn user() -> CncProfile {
        let mut profile = GENERIC_GRBL_ROUTER.profile();
        profile.id = ID.to_owned();
        profile.modified = 1_700_000_000_000;
        profile.values.name = "My router".to_owned();
        profile.values.connection.connector_address = "192.168.1.20:7461".to_owned();
        profile
    }

    #[test]
    fn a_record_writes_format_9_of_kind_cnc_and_reads_back() {
        let profile = user();
        let text = write_record(&profile);
        assert!(text.contains("\"format_version\": 9"), "{text}");
        assert!(text.contains("\"kind\": \"cnc\""));
        let read = read_record(&record_file_name(ID), &text).unwrap();
        assert_eq!(read, profile);
        assert_eq!(write_record(&read), text);
    }

    #[test]
    fn another_family_a_damaged_and_a_newer_record_are_skipped() {
        let name = record_file_name(ID);
        let cutter = format!(r#"{{"format_version": 9, "id": "{ID}", "kind": "cutter"}}"#);
        assert_eq!(read_record(&name, &cutter), Err(RecordError::OtherFamily));
        let damaged = write_record(&user()).replace("\"grbl\"", "\"mach3\"");
        assert_eq!(read_record(&name, &damaged), Err(RecordError::Damaged));
        let newer =
            write_record(&user()).replace("\"format_version\": 9", "\"format_version\": 10");
        assert_eq!(read_record(&name, &newer), Err(RecordError::Newer));
    }

    #[test]
    fn the_newer_record_wins_a_merge() {
        let local = user();
        let mut incoming = user();
        incoming.modified += 1;
        let (local, incoming) = (write_record(&local), write_record(&incoming));
        assert_eq!(
            merge_cnc_record(ID, &incoming, Some(&local)),
            RecordMerge::IncomingWins {
                loser: Some(local.clone())
            }
        );
    }
}
