//! The ended record `share.ended.json` version 1 (ADR 0041 decision 6): where a removed or left
//! shared workspace synced, kept in its marker directory. A record this build cannot read (newer
//! `format_version`, malformed) means "no way back", never an error **(Proposal)**. Builds of
//! `0097` also wrote it into containers of documents shared on their own; such a file opens as a
//! local document (ADR 0044 decision 3).

use serde::{Deserialize, Serialize};

use crate::container_share::{hex, unhex};
use crate::document::DocumentVersion;
use crate::error::SaveError;

/// The member's name in a container and the file's name in a workspace's marker.
pub(crate) const ENDED_MEMBER: &str = "share.ended.json";
/// The version this build writes and the newest it reads.
pub const ENDED_FORMAT_VERSION: u32 = 1;

/// Why the share ended for this device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    /// An admin removed this device.
    Removed,
    /// This device left by its own `RemoveMember`.
    Left,
}

/// Where a file synced before its share ended, and how far (no keys, no keyring).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndedRecord {
    /// The relay address.
    pub server: String,
    /// The room on that relay: a document's room, or the workspace id in a marker.
    pub room: [u8; 16],
    /// The relay sequence up to which this replica holds every record.
    pub server_seq: u64,
    /// The version the relay acknowledged last; later edits go out as offline edits on Sync
    /// again (`0092` criteria 17 to 19).
    pub synced_version: DocumentVersion,
    /// Removed or left.
    pub ended: EndReason,
    /// The keyring heads this device knew when the share ended (public entry hashes): the
    /// return check counts only an add back that builds on all of them, so a relay cannot pass
    /// off an older keyring as one (ADR 0041 decision 6).
    pub heads: Vec<[u8; 32]>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum EndedJsonReason {
    Removed,
    Left,
}

#[derive(Serialize, Deserialize)]
struct EndedJson {
    format_version: u32,
    server: String,
    room: String,
    server_seq: u64,
    synced_version: String,
    ended: EndedJsonReason,
    heads: Vec<String>,
}

#[derive(Deserialize)]
struct EndedJsonVersion {
    format_version: u32,
}

impl EndedRecord {
    /// The `share.ended.json` bytes.
    ///
    /// # Errors
    /// [`SaveError::Container`] if the JSON cannot be written.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SaveError> {
        serde_json::to_vec(&EndedJson {
            format_version: ENDED_FORMAT_VERSION,
            server: self.server.clone(),
            room: hex(&self.room),
            server_seq: self.server_seq,
            synced_version: self.synced_version.to_text(),
            ended: match self.ended {
                EndReason::Removed => EndedJsonReason::Removed,
                EndReason::Left => EndedJsonReason::Left,
            },
            heads: self.heads.iter().map(|h| hex(h)).collect(),
        })
        .map_err(|_| SaveError::Container)
    }

    /// Reads `share.ended.json`; `None` for a newer version or bytes that are not one (the way
    /// back is then an invite link).
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let version: EndedJsonVersion = serde_json::from_slice(bytes).ok()?;
        if version.format_version != ENDED_FORMAT_VERSION {
            return None;
        }
        let json: EndedJson = serde_json::from_slice(bytes).ok()?;
        Some(Self {
            server: json.server,
            room: unhex(&json.room)?,
            server_seq: json.server_seq,
            synced_version: DocumentVersion::from_text(&json.synced_version)?,
            ended: match json.ended {
                EndedJsonReason::Removed => EndReason::Removed,
                EndedJsonReason::Left => EndReason::Left,
            },
            heads: json
                .heads
                .iter()
                .map(|h| unhex_hash(h))
                .collect::<Option<Vec<_>>>()?,
        })
    }
}

/// A 64-digit lowercase hex entry hash.
fn unhex_hash(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 || !text.is_ascii() {
        return None;
    }
    let mut hash = [0_u8; 32];
    for (half, part) in hash.chunks_mut(16).zip([&text[..32], &text[32..]]) {
        half.copy_from_slice(&unhex(part)?);
    }
    Some(hash)
}
