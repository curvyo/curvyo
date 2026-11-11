//! The container member that makes a file a workspace document (ADR 0037 decision 3, ADR 0044
//! decision 3): `share.json` version 2. A shared workspace's marker directory keeps a
//! `share.json` of the version 1 shape (ADR 0037 decision 1), read and written here as bytes.
//! Files a build of `0092` or `0097` shared on their own (`share.json` version 1, `keyring.log`,
//! `share.ended.json`) open as local documents with [`crate::Document::legacy_share`] set, and
//! the next save drops those members.

use std::io::{Cursor, Read};

use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::container_ended::ENDED_MEMBER;
use crate::document::DocumentVersion;
use crate::error::{OpenError, SaveError};

const SHARE_MEMBER: &str = "share.json";
/// The keyring of a document shared on its own: read only to recognise a legacy file.
const LEGACY_KEYRING_MEMBER: &str = "keyring.log";

/// The newest `format_version` of `share.json` this build reads. A workspace document is written
/// as version 2; version 1 was a document shared on its own and is read as a legacy file.
pub const CURRENT_SHARE_FORMAT_VERSION: u32 = 2;
/// The marker's `share.json` shape; in a container, a document shared on its own (legacy).
const ROOM_SHARE_VERSION: u32 = 1;
/// `share.json` of a workspace document.
const WORKSPACE_SHARE_VERSION: u32 = 2;

/// The bytes `share.json` may hold at most when read (64 MiB, the relay's message limit); a
/// longer member is damage.
const MAX_SHARE_BYTES: u64 = 64 * 1024 * 1024;

/// Which workspace a workspace document belongs to and its identity there, as kept in its file
/// (`share.json` version 2). The identity is never derived from the file's name or place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceShareRecord {
    /// The workspace id: the room of the workspace.
    pub workspace: [u8; 16],
    /// The document id: the document's stream in the workspace room.
    pub document: [u8; 16],
    /// The version the workspace room acknowledged last; updates after it go out on sync.
    pub synced_version: DocumentVersion,
}

/// Where a shared workspace syncs and how far, as its marker's `share.json` keeps it (ADR 0037
/// decision 1): the shape of version 1 with `room` = the workspace id, `server_seq` = the
/// workspace cursor and `synced_version` = the list's version the room acknowledged last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRoomRecord {
    /// The relay address.
    pub server: String,
    /// The workspace id, the room on that relay.
    pub workspace: [u8; 16],
    /// The relay sequence up to which every record is in a file.
    pub server_seq: u64,
    /// The list's version the room acknowledged last.
    pub synced_version: DocumentVersion,
}

impl WorkspaceRoomRecord {
    /// The marker's `share.json` bytes.
    ///
    /// # Errors
    /// [`SaveError::Container`] if the JSON cannot be written.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SaveError> {
        serde_json::to_vec(&ShareJson {
            format_version: ROOM_SHARE_VERSION,
            server: self.server.clone(),
            room: hex(&self.workspace),
            server_seq: self.server_seq,
            synced_version: self.synced_version.to_text(),
        })
        .map_err(|_| SaveError::Container)
    }

    /// Reads the marker's `share.json`.
    ///
    /// # Errors
    /// [`OpenError::FormatTooNew`] for a newer version; [`OpenError::Damaged`] otherwise.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, OpenError> {
        let version: ShareJsonVersion =
            serde_json::from_slice(bytes).map_err(|_| OpenError::Damaged)?;
        if version.format_version > ROOM_SHARE_VERSION {
            return Err(OpenError::FormatTooNew {
                found: version.format_version,
                supported: ROOM_SHARE_VERSION,
            });
        }
        let json: ShareJson = serde_json::from_slice(bytes).map_err(|_| OpenError::Damaged)?;
        if json.format_version != ROOM_SHARE_VERSION {
            return Err(OpenError::Damaged);
        }
        Ok(Self {
            server: json.server,
            workspace: unhex(&json.room).ok_or(OpenError::Damaged)?,
            server_seq: json.server_seq,
            synced_version: DocumentVersion::from_text(&json.synced_version)
                .ok_or(OpenError::Damaged)?,
        })
    }
}

impl WorkspaceShareRecord {
    /// The record in a container's `share.json` member when it is version 2 (a document of a
    /// shared workspace), read by a host's scan without opening the document
    /// (`specs/0095-workspace` criterion 41); `None` for any other content.
    #[must_use]
    pub fn from_share_json(bytes: &[u8]) -> Option<Self> {
        let version: ShareJsonVersion = serde_json::from_slice(bytes).ok()?;
        if version.format_version != WORKSPACE_SHARE_VERSION {
            return None;
        }
        read_workspace_share(bytes).ok()
    }
}

impl crate::Document {
    /// Records whether this document is currently attached to a live collaboration link.
    pub fn set_live_share(&self, active: bool) {
        self.live_share.set(active);
    }

    /// Whether curve-kind and width-profile edits are refused because the document is shared
    /// (ADR 0053, compatibility with older sync peers).
    pub(crate) fn extended_edits_blocked(&self) -> bool {
        self.is_shared() || self.workspace_share().is_some()
    }

    /// The record of a workspace document, `None` otherwise (`specs/0095-workspace` criterion
    /// 15).
    #[must_use]
    pub fn workspace_share(&self) -> Option<WorkspaceShareRecord> {
        self.share.borrow().clone()
    }

    /// Sets the workspace record written with the next save, or clears it; not a history step.
    pub fn set_workspace_share(&self, record: Option<WorkspaceShareRecord>) {
        self.commit_state.set_shared(record.is_some());
        *self.share.borrow_mut() = record;
    }

    /// Whether the file this document was opened from held the share members of a document
    /// shared on its own (`share.json` version 1, `keyring.log`, `share.ended.json`; ADR 0044
    /// decision 3): it opened as a local document, and the next save drops them. Read only,
    /// for one notice.
    #[must_use]
    pub fn legacy_share(&self) -> bool {
        self.legacy_share.get()
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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceShareJson {
    format_version: u32,
    workspace: String,
    document: String,
    synced_version: String,
}

#[derive(Deserialize)]
struct ShareJsonVersion {
    format_version: u32,
}

/// The member `share.json` (version 2) of a workspace document's record.
pub(crate) fn share_member(
    record: &WorkspaceShareRecord,
) -> Result<(&'static str, Vec<u8>), SaveError> {
    let json = WorkspaceShareJson {
        format_version: WORKSPACE_SHARE_VERSION,
        workspace: hex(&record.workspace),
        document: hex(&record.document),
        synced_version: record.synced_version.to_text(),
    };
    let json = serde_json::to_vec(&json).map_err(|_| SaveError::Container)?;
    Ok((SHARE_MEMBER, json))
}

/// What a container's share members say (ADR 0044 decision 3): the workspace record, and whether
/// legacy members of a document shared on its own are present (the file opens as local).
///
/// # Errors
/// [`OpenError::Damaged`] for a malformed `share.json` or a version 2 record beside a keyring;
/// [`OpenError::FormatTooNew`] for a `share.json` newer than this build reads.
pub(crate) fn read_share(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
) -> Result<(Option<WorkspaceShareRecord>, bool), OpenError> {
    let json = read_optional(archive, SHARE_MEMBER, MAX_SHARE_BYTES)?;
    let keyring = archive.index_for_name(LEGACY_KEYRING_MEMBER).is_some();
    let Some(json) = json else {
        let ended = archive.index_for_name(ENDED_MEMBER).is_some();
        return Ok((None, keyring || ended));
    };
    let version: ShareJsonVersion =
        serde_json::from_slice(&json).map_err(|_| OpenError::Damaged)?;
    if version.format_version > CURRENT_SHARE_FORMAT_VERSION {
        return Err(OpenError::FormatTooNew {
            found: version.format_version,
            supported: CURRENT_SHARE_FORMAT_VERSION,
        });
    }
    match version.format_version {
        WORKSPACE_SHARE_VERSION if keyring => Err(OpenError::Damaged),
        // A stray ended record beside a workspace record is no legacy file: the record counts.
        WORKSPACE_SHARE_VERSION => Ok((Some(read_workspace_share(&json)?), false)),
        ROOM_SHARE_VERSION => Ok((None, true)),
        _ => Err(OpenError::Damaged),
    }
}

fn read_workspace_share(json: &[u8]) -> Result<WorkspaceShareRecord, OpenError> {
    let json: WorkspaceShareJson = serde_json::from_slice(json).map_err(|_| OpenError::Damaged)?;
    Ok(WorkspaceShareRecord {
        workspace: unhex(&json.workspace).ok_or(OpenError::Damaged)?,
        document: unhex(&json.document).ok_or(OpenError::Damaged)?,
        synced_version: DocumentVersion::from_text(&json.synced_version)
            .ok_or(OpenError::Damaged)?,
    })
}

/// What a container says about its sharing, read without opening the document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContainerShareMembers {
    /// The bytes of `share.json`, when present and not too large.
    pub share_json: Option<Vec<u8>>,
}

/// The raw `share.json` of the container `bytes`, read without opening the document: what a shared workspace's scan needs per file where no file system
/// does it (`specs/0096-own-devices-sync` criterion 35, the browser). A file that is no
/// container, or whose `share.json` is too large, has neither.
#[must_use]
pub fn container_share_members(bytes: &[u8]) -> ContainerShareMembers {
    let Ok(mut archive) = ZipArchive::new(Cursor::new(bytes)) else {
        return ContainerShareMembers::default();
    };
    let share_json = read_optional(&mut archive, SHARE_MEMBER, MAX_SHARE_BYTES)
        .ok()
        .flatten();
    ContainerShareMembers { share_json }
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

pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        // invariant: writing into a String cannot fail.
        let _ = write!(text, "{byte:02x}");
        text
    })
}

pub(crate) fn unhex(text: &str) -> Option<[u8; 16]> {
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

    /// A workspace document names its `share.json` and no keyring; plain bytes have neither.
    #[test]
    fn share_members_read_without_opening_the_document() {
        let document = crate::Document::new(3);
        document.set_workspace_share(Some(WorkspaceShareRecord {
            workspace: [0x57; 16],
            document: [0x11; 16],
            synced_version: document.version(),
        }));
        let bytes = crate::pack(&document, "t").unwrap();
        let members = container_share_members(&bytes);
        assert!(
            String::from_utf8(members.share_json.unwrap())
                .unwrap()
                .contains("\"format_version\":2")
        );
        let none = ContainerShareMembers::default();
        assert_eq!(container_share_members(b"plain"), none);
        let plain = crate::pack(&crate::Document::new(4), "t").unwrap();
        assert_eq!(container_share_members(&plain), none);
    }
}
