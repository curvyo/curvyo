//! The codec of `recovery.json`, the small record beside a recovery copy
//! (`specs/0136-autosave-and-restore` criteria 4, 6, 7 and 9, ADR 0079 decision 3): the name,
//! the saved file it was based on, the time of the last step and the page count, in one JSON
//! object with its own `format_version`.

use serde::{Deserialize, Serialize};

/// The record's own format version (ADR 0004 §9); `.curvyo` is unchanged.
pub const RECOVERY_FORMAT: u32 = 1;

/// The saved file a copy was based on: its path and what it looked like at open or Save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryBase {
    /// The file's path.
    pub path: String,
    /// Its modification time, ms since the Unix epoch.
    pub modified_ms: u64,
    /// Its length in bytes.
    pub length: u64,
}

/// What a recovery copy says about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryRecord {
    /// [`RECOVERY_FORMAT`].
    pub format_version: u32,
    /// The session key, 32 hex digits.
    pub key: String,
    /// The project's name (the saved file's name), `None` for a never-saved project.
    pub name: Option<String>,
    /// The saved file the copy was based on, `None` for a never-saved project.
    pub base: Option<RecoveryBase>,
    /// The wall-clock time of the last step, ms since the Unix epoch, for display only.
    pub last_step_ms: u64,
    /// The page count.
    pub pages: u32,
}

/// Why a record was not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RecordError {
    /// It is not a readable record (truncated, wrong shape).
    #[error("the recovery record is damaged")]
    Damaged,
    /// It was made by a newer Curvyo.
    #[error("the recovery record was made by a newer version")]
    Newer,
}

#[derive(Deserialize)]
struct Header {
    format_version: Option<u32>,
}

/// Reads `bytes` as a record; a newer `format_version` is refused before the rest is read.
///
/// # Errors
/// [`RecordError::Damaged`] or [`RecordError::Newer`].
pub fn decode_record(bytes: &[u8]) -> Result<RecoveryRecord, RecordError> {
    let header: Header = serde_json::from_slice(bytes).map_err(|_| RecordError::Damaged)?;
    match header.format_version {
        Some(version) if version > RECOVERY_FORMAT => Err(RecordError::Newer),
        Some(_) => serde_json::from_slice(bytes).map_err(|_| RecordError::Damaged),
        None => Err(RecordError::Damaged),
    }
}

/// The record's bytes.
#[must_use]
pub fn encode_record(record: &RecoveryRecord) -> Vec<u8> {
    // A record of strings and numbers always serialises.
    serde_json::to_vec(record).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!(
            "{}/tests/fixtures/recovery/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read(path).unwrap()
    }

    fn sample() -> RecoveryRecord {
        RecoveryRecord {
            format_version: RECOVERY_FORMAT,
            key: "00112233445566778899aabbccddeeff".to_owned(),
            name: Some("Poster.curvyo".to_owned()),
            base: Some(RecoveryBase {
                path: "/home/maker/Poster.curvyo".to_owned(),
                modified_ms: 1_763_899_380_000,
                length: 4096,
            }),
            last_step_ms: 1_763_899_999_000,
            pages: 3,
        }
    }

    #[test]
    fn the_current_golden_reads_and_writes_back_equal() {
        let bytes = fixture("current.json");
        let record = decode_record(&bytes).unwrap();
        assert_eq!(record, sample());
        assert_eq!(decode_record(&encode_record(&record)).unwrap(), record);
    }

    #[test]
    fn a_never_saved_record_has_no_name_and_no_base() {
        let record = decode_record(&fixture("untitled.json")).unwrap();
        assert_eq!(record.name, None);
        assert_eq!(record.base, None);
        assert_eq!(record.pages, 1);
    }

    #[test]
    fn a_newer_record_is_refused() {
        assert_eq!(
            decode_record(&fixture("newer.json")),
            Err(RecordError::Newer)
        );
    }

    #[test]
    fn a_truncated_record_is_damaged() {
        assert_eq!(
            decode_record(&fixture("truncated.json")),
            Err(RecordError::Damaged)
        );
        assert_eq!(decode_record(b""), Err(RecordError::Damaged));
        assert_eq!(decode_record(b"{}"), Err(RecordError::Damaged));
    }
}
