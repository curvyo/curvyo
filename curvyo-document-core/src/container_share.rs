//! The two container members that make a file shared (ADR 0031 decision 9, ADR 0004 §1):
//! `share.json`, where the document syncs and how far, and `keyring.log`, the membership log kept
//! as opaque bytes. Both are present or both are absent.

use std::io::{Cursor, Read};

use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::document::DocumentVersion;
use crate::error::{OpenError, SaveError};

const SHARE_MEMBER: &str = "share.json";
const KEYRING_MEMBER: &str = "keyring.log";

/// The `format_version` of `share.json` this build writes and the newest it reads.
pub const CURRENT_SHARE_FORMAT_VERSION: u32 = 1;

/// The bytes the keyring log member may hold at most when read (64 MiB, the relay's message
/// limit); a longer member is damage.
const MAX_KEYRING_BYTES: u64 = 64 * 1024 * 1024;

/// Where a shared document syncs, how far it has synced, and its keyring, as kept in its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareRecord {
    /// The relay address the maker chose, for example `ws://192.168.1.20:8787`.
    pub server: String,
    /// The opaque room id on that relay.
    pub room: [u8; 16],
    /// The relay sequence of the last record this replica received.
    pub server_seq: u64,
    /// The version the relay acknowledged last; updates after it are pushed on connect.
    pub synced_version: DocumentVersion,
    /// The signed keyring log (ADR 0010 §2), opaque to this crate.
    pub keyring_log: Vec<u8>,
}

impl crate::Document {
    /// The share record this document's file carries, `None` for a document that is not shared.
    #[must_use]
    pub fn share(&self) -> Option<ShareRecord> {
        self.share.borrow().clone()
    }

    /// Sets or clears the share record written with the next save; not a history step. A removed
    /// device clears it and keeps a plain local file (R-COLLAB-010).
    pub fn set_share(&self, record: Option<ShareRecord>) {
        self.commit_state.set_shared(record.is_some());
        *self.share.borrow_mut() = record;
    }
}

#[derive(Serialize, Deserialize)]
struct ShareJson {
    format_version: u32,
    server: String,
    room: String,
    server_seq: u64,
    synced_version: String,
}

#[derive(Deserialize)]
struct ShareJsonVersion {
    format_version: u32,
}

/// The members of `record`: `share.json`, then `keyring.log`.
pub(crate) fn share_members(
    record: &ShareRecord,
) -> Result<[(&'static str, Vec<u8>); 2], SaveError> {
    let json = ShareJson {
        format_version: CURRENT_SHARE_FORMAT_VERSION,
        server: record.server.clone(),
        room: hex(&record.room),
        server_seq: record.server_seq,
        synced_version: record.synced_version.to_text(),
    };
    let json = serde_json::to_vec(&json).map_err(|_| SaveError::Container)?;
    Ok([
        (SHARE_MEMBER, json),
        (KEYRING_MEMBER, record.keyring_log.clone()),
    ])
}

/// The share record of `archive`, `None` for a file that was never shared.
///
/// # Errors
/// [`OpenError::Damaged`] for one member without the other or a malformed `share.json`;
/// [`OpenError::FormatTooNew`] for a `share.json` newer than this build reads.
pub(crate) fn read_share(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
) -> Result<Option<ShareRecord>, OpenError> {
    let json = read_optional(archive, SHARE_MEMBER, MAX_KEYRING_BYTES)?;
    let keyring = read_optional(archive, KEYRING_MEMBER, MAX_KEYRING_BYTES)?;
    let (json, keyring_log) = match (json, keyring) {
        (None, None) => return Ok(None),
        (Some(json), Some(keyring)) => (json, keyring),
        _ => return Err(OpenError::Damaged),
    };
    let version: ShareJsonVersion =
        serde_json::from_slice(&json).map_err(|_| OpenError::Damaged)?;
    if version.format_version > CURRENT_SHARE_FORMAT_VERSION {
        return Err(OpenError::FormatTooNew {
            found: version.format_version,
            supported: CURRENT_SHARE_FORMAT_VERSION,
        });
    }
    let json: ShareJson = serde_json::from_slice(&json).map_err(|_| OpenError::Damaged)?;
    Ok(Some(ShareRecord {
        server: json.server,
        room: unhex(&json.room).ok_or(OpenError::Damaged)?,
        server_seq: json.server_seq,
        synced_version: DocumentVersion::from_text(&json.synced_version)
            .ok_or(OpenError::Damaged)?,
        keyring_log,
    }))
}

fn read_optional(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    limit: u64,
) -> Result<Option<Vec<u8>>, OpenError> {
    let Ok(file) = archive.by_name(name) else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| OpenError::Damaged)?;
    if bytes.len() as u64 > limit {
        return Err(OpenError::Damaged);
    }
    Ok(Some(bytes))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        // invariant: writing into a String cannot fail.
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn unhex(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32 || !text.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut room = [0_u8; 16];
    for (index, byte) in room.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[2 * index..2 * index + 2], 16).ok()?;
    }
    Some(room)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_ids_round_trip_through_lowercase_hex() {
        let room = [0, 1, 0xab, 0xff, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0x10, 0xfe];
        assert_eq!(unhex(&hex(&room)), Some(room));
        assert_eq!(unhex("0707"), None);
        assert_eq!(unhex(&"AB".repeat(16)), None, "lowercase only");
        assert_eq!(unhex(&"zz".repeat(16)), None);
    }
}
