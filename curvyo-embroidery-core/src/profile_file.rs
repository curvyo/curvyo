//! Reads and writes one embroidery profile record of kind `embroidery` in `machines/`
//! (`specs/0160-embroidery` criterion 2, `adrs.md` EM2) on the shared record envelope (ADR 0097
//! §4), and merges a synced record by this reader's outcome.

use curvyo_library_core::{
    EnvelopeError, Family, MergeReason, RecordMerge, merge_record, open_family, pretty,
    record_file_name, record_id,
};
use serde::{Deserialize, Serialize};

use crate::hoop::HoopSize;
use crate::profile::{EmbroideryProfile, EmbroideryValues, OutputFormat, StitchesPerMin};
use crate::stitch::TenthMm;

/// The record format this build writes: 9, one above the laser's 8, so an older build skips an
/// embroidery record as newer (as the cutter's, `0159` PL2).
pub const RECORD_FORMAT_VERSION: u64 = 9;
/// The largest record text that is read.
pub const MAX_RECORD_BYTES: usize = 64 * 1024;

/// Why a record was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RecordError {
    /// Not a valid embroidery record: skipped, counted, left untouched.
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
    hoops: Vec<String>,
    max_stitch_mm: f64,
    speed_spm: u64,
}

/// The profile the record file `file_name` holds.
///
/// # Errors
/// [`RecordError::OtherFamily`] for a record of another known family; [`RecordError::Newer`] for
/// a `format_version` above 9 or an unknown `kind`; [`RecordError::Damaged`] for a text over
/// 64 KiB, a JSON or type error, a missing or unknown key, an unknown spelling, a value out of
/// its range, an id that differs from the file name, or invalid values. Nothing is written.
pub fn read_record(file_name: &str, text: &str) -> Result<EmbroideryProfile, RecordError> {
    let (value, _) = open_family(
        text,
        MAX_RECORD_BYTES,
        Family::Embroidery,
        RECORD_FORMAT_VERSION,
        RECORD_FORMAT_VERSION,
    )
    .map_err(|error| match error {
        EnvelopeError::Newer => RecordError::Newer,
        EnvelopeError::OtherFamily(_) => RecordError::OtherFamily,
        _ => RecordError::Damaged,
    })?;
    let json: RecordJson = serde_json::from_value(value).map_err(|_| RecordError::Damaged)?;
    if json.kind != Family::Embroidery.key() || record_id(file_name) != Some(json.id.as_str()) {
        return Err(RecordError::Damaged);
    }
    let hoops = json
        .hoops
        .iter()
        .map(|key| HoopSize::parse(key))
        .collect::<Option<Vec<_>>>()
        .ok_or(RecordError::Damaged)?;
    let values = EmbroideryValues {
        name: json.name,
        output_format: OutputFormat::parse(&json.output_format).ok_or(RecordError::Damaged)?,
        hoops,
        max_stitch: TenthMm::of_mm(json.max_stitch_mm),
        speed: StitchesPerMin::new(json.speed_spm).ok_or(RecordError::Damaged)?,
    };
    if !values.is_valid() {
        return Err(RecordError::Damaged);
    }
    Ok(EmbroideryProfile {
        id: json.id,
        modified: json.modified,
        values,
    })
}

/// The record text of `profile`: canonical, so read, write, read is the identity.
#[must_use]
pub fn write_record(profile: &EmbroideryProfile) -> String {
    let v = &profile.values;
    pretty(&RecordJson {
        format_version: RECORD_FORMAT_VERSION,
        id: profile.id.clone(),
        kind: Family::Embroidery.key().to_owned(),
        modified: profile.modified,
        name: v.name.clone(),
        output_format: v.output_format.key().to_owned(),
        hoops: v.hoops.iter().map(|h| h.key().to_owned()).collect(),
        max_stitch_mm: v.max_stitch.mm(),
        speed_spm: u64::from(v.speed.get()),
    })
}

/// Merges the incoming embroidery record `incoming` against `local`, the record file `id` holds,
/// by this reader's outcome (criterion 2, ADR 0097 §4).
#[must_use]
pub fn merge_embroidery_record(id: &str, incoming: &str, local: Option<&str>) -> RecordMerge {
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
    use crate::builtins::W6_EU7;

    const ID: &str = "0b5f6c2e-9d4a-4c1b-8e7f-1a2b3c4d5e6f";

    fn user() -> EmbroideryProfile {
        let mut profile = W6_EU7.profile();
        profile.id = ID.to_owned();
        profile.modified = 1_700_000_000_000;
        profile.values.name = "My EU7".to_owned();
        profile
    }

    #[test]
    fn a_record_writes_format_9_of_kind_embroidery_and_reads_back() {
        let profile = user();
        let text = write_record(&profile);
        assert!(text.contains("\"format_version\": 9"), "{text}");
        assert!(text.contains("\"kind\": \"embroidery\""));
        let read = read_record(&record_file_name(ID), &text).unwrap();
        assert_eq!(read, profile);
        assert_eq!(write_record(&read), text);
    }

    #[test]
    fn another_family_a_damaged_and_a_newer_record_are_skipped() {
        let name = record_file_name(ID);
        let cutter = format!(r#"{{"format_version": 9, "id": "{ID}", "kind": "cutter"}}"#);
        assert_eq!(read_record(&name, &cutter), Err(RecordError::OtherFamily));
        let damaged = write_record(&user()).replace("\"100x100\"", "\"9x9\"");
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
            merge_embroidery_record(ID, &incoming, Some(&local)),
            RecordMerge::IncomingWins {
                loser: Some(local.clone())
            }
        );
    }
}
