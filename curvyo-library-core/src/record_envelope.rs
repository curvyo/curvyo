//! The envelope every per-user record of every machine family shares (ADR 0097 §4; ADR 0055 §7;
//! ADR 0038 §2, §3, §5).
//!
//! A record is the file `<uuid>.json` (browser: one storage entry) whose text is a JSON object
//! with a `format_version`; this module holds the id and file-name rule, the size limit and
//! `kind` and `format_version` gate applied before a record's own keys are read, the canonical
//! text form, and the write the host must do.

use std::fmt::Write as _;

use serde::Serialize;
use serde_json::Value;

use crate::record_kind::{Family, KindRead, record_kind};

/// The file name suffix of a record.
pub const RECORD_SUFFIX: &str = ".json";

/// What the host must do to keep one record (one file or one browser-storage entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryWrite {
    /// Replace the record `id` with `text` (atomic write).
    Write {
        /// The record id; the file is `<id>.json`.
        id: String,
        /// The record text.
        text: String,
    },
    /// Remove the record `id`.
    Remove {
        /// The record id.
        id: String,
    },
}

/// Why the envelope of a record text did not open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeError {
    /// The text is longer than the record's limit.
    TooLarge,
    /// The text is not a JSON value.
    NotJson,
    /// `format_version` is missing, not a whole number, or below the oldest format read.
    BadVersion,
    /// `format_version` is above the newest format this build reads (ADR 0004 §9), or `kind`
    /// names a family this build does not know (ADR 0097 §4).
    Newer,
    /// `kind` names another family this build knows: that family's reader reads it, and this one
    /// skips it silently (ADR 0097 §4).
    OtherFamily(Family),
}

/// A new record id: a lowercase UUID v4 from 16 random bytes the caller passes in (core has no
/// random source; ADR 0038 §5, ADR 0055 §1).
#[must_use]
pub fn new_record_id(random: [u8; 16]) -> String {
    let mut bytes = random;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = bytes.iter().fold(String::with_capacity(32), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    });
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Whether `id` is a lowercase UUID v4 (the id of a user record and its file's name).
#[must_use]
pub fn is_record_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, &b)| match index {
            8 | 13 | 18 | 23 => b == b'-',
            14 => b == b'4',
            19 => matches!(b, b'8' | b'9' | b'a' | b'b'),
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
        })
}

/// The id a record file name `<uuid>.json` carries; `None` for any other name, which is ignored.
#[must_use]
pub fn record_id(file_name: &str) -> Option<&str> {
    file_name
        .strip_suffix(RECORD_SUFFIX)
        .filter(|id| is_record_id(id))
}

/// The file name of the record `id`.
#[must_use]
pub fn record_file_name(id: &str) -> String {
    format!("{id}{RECORD_SUFFIX}")
}

/// The JSON value of a record text and its `format_version`, once the text is at most
/// `max_bytes` long and its version lies within `oldest..=newest`. Nothing else is checked.
///
/// # Errors
/// [`EnvelopeError`] for the first gate the text fails.
pub fn open(
    text: &str,
    max_bytes: usize,
    oldest: u64,
    newest: u64,
) -> Result<(Value, u64), EnvelopeError> {
    if text.len() > max_bytes {
        return Err(EnvelopeError::TooLarge);
    }
    let value: Value = serde_json::from_str(text).map_err(|_| EnvelopeError::NotJson)?;
    version_of(value, oldest, newest)
}

/// As [`open`] for a record of `family`, sorted by `kind` before its version is gated (ADR 0097
/// §4): another known family is [`EnvelopeError::OtherFamily`], an unknown kind
/// [`EnvelopeError::Newer`]. A missing or non-text `kind` passes, for the family's reader to
/// find as damage.
///
/// # Errors
/// [`EnvelopeError`] for the first gate the text fails.
pub fn open_family(
    text: &str,
    max_bytes: usize,
    family: Family,
    oldest: u64,
    newest: u64,
) -> Result<(Value, u64), EnvelopeError> {
    if text.len() > max_bytes {
        return Err(EnvelopeError::TooLarge);
    }
    let value: Value = serde_json::from_str(text).map_err(|_| EnvelopeError::NotJson)?;
    match record_kind(&value) {
        KindRead::Family(found) if found != family => {
            return Err(EnvelopeError::OtherFamily(found));
        }
        KindRead::Unknown => return Err(EnvelopeError::Newer),
        KindRead::Family(_) | KindRead::Missing => {}
    }
    version_of(value, oldest, newest)
}

/// The version gate of [`open`] and [`open_family`].
fn version_of(value: Value, oldest: u64, newest: u64) -> Result<(Value, u64), EnvelopeError> {
    match value.get("format_version").and_then(Value::as_u64) {
        Some(version) if version > newest => Err(EnvelopeError::Newer),
        Some(version) if version >= oldest => Ok((value, version)),
        _ => Err(EnvelopeError::BadVersion),
    }
}

/// The canonical text of a record or preference file: the value's field order, two-space indent,
/// trailing newline, so read, write, read is the identity.
///
/// # Panics
/// Only for a value with a map whose keys are not strings or a serialiser that fails; record and
/// preference values are plain structs of strings, numbers, booleans and maps, so never.
#[must_use]
pub fn pretty(value: &impl Serialize) -> String {
    // invariant: plain structs of strings, numbers, booleans and maps always serialise.
    #[allow(clippy::unwrap_used)]
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_id_is_a_lowercase_uuid_v4() {
        let id = new_record_id([0xff; 16]);
        assert_eq!(id, "ffffffff-ffff-4fff-bfff-ffffffffffff");
        assert!(is_record_id(&id));
        assert!(is_record_id(&new_record_id([0; 16])));
        assert!(!is_record_id("generic-laser"));
        assert!(!is_record_id("cricut-maker-3"));
        assert!(!is_record_id("FFFFFFFF-FFFF-4FFF-BFFF-FFFFFFFFFFFF"));
        assert!(!is_record_id("ffffffff-ffff-1fff-bfff-ffffffffffff"));
    }

    #[test]
    fn only_uuid_file_names_are_records() {
        let id = new_record_id([7; 16]);
        assert_eq!(record_id(&record_file_name(&id)), Some(id.as_str()));
        assert_eq!(record_id("not-a-uuid.json"), None);
        assert_eq!(record_id(&format!("{id}.json.sync-conflict")), None);
        assert_eq!(record_id(&id), None);
    }

    /// ADR 0097 §4: `kind` first, so a record of another family at a version above this
    /// family's newest is that family's, never "newer".
    #[test]
    fn a_family_envelope_sorts_by_kind_before_the_version() {
        let open = |text: &str| open_family(text, 128, Family::Laser, 5, 8);
        assert_eq!(
            open(r#"{"format_version": 9, "kind": "cutter"}"#),
            Err(EnvelopeError::OtherFamily(Family::Cutter))
        );
        assert_eq!(
            open(r#"{"format_version": 9, "kind": "cnc"}"#),
            Err(EnvelopeError::OtherFamily(Family::Cnc))
        );
        assert_eq!(
            open(r#"{"format_version": 6, "kind": "plasma"}"#),
            Err(EnvelopeError::Newer)
        );
        assert_eq!(
            open(r#"{"format_version": 8, "kind": "laser"}"#).unwrap().1,
            8
        );
        assert_eq!(
            open(r#"{"format_version": 9, "kind": "laser"}"#),
            Err(EnvelopeError::Newer)
        );
        assert_eq!(open(r#"{"format_version": 6, "kind": 3}"#).unwrap().1, 6);
        assert_eq!(open(r#"{"format_version": 6}"#).unwrap().1, 6);
    }

    #[test]
    fn the_envelope_gates_size_json_and_version() {
        assert_eq!(open("{\"format_version\": 2}", 64, 1, 3).unwrap().1, 2);
        assert_eq!(
            open("{\"format_version\": 4}", 64, 1, 3),
            Err(EnvelopeError::Newer)
        );
        for bad in [
            "{\"format_version\": 0}",
            "{\"format_version\": \"1\"}",
            "{}",
            "[]",
        ] {
            assert_eq!(open(bad, 64, 1, 3), Err(EnvelopeError::BadVersion), "{bad}");
        }
        assert_eq!(open("{", 64, 1, 3), Err(EnvelopeError::NotJson));
        assert_eq!(
            open(&" ".repeat(65), 64, 1, 3),
            Err(EnvelopeError::TooLarge)
        );
    }
}
