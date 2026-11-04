//! The container members that make a file shared (ADR 0031 decision 9, ADR 0037 decision 3,
//! ADR 0004 §1): `share.json` version 1 with `keyring.log` for a document shared on its own
//! (both present or both absent), or `share.json` version 2 alone for a workspace document.
//! A shared workspace's marker directory keeps a `share.json` of the version 1 shape too
//! (ADR 0037 decision 1), read and written here as bytes.

use std::io::{Cursor, Read};

use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::document::DocumentVersion;
use crate::error::{OpenError, SaveError};

const SHARE_MEMBER: &str = "share.json";
const KEYRING_MEMBER: &str = "keyring.log";

/// The newest `format_version` of `share.json` this build reads. A document shared on its own is
/// written as version 1, a workspace document as version 2.
pub const CURRENT_SHARE_FORMAT_VERSION: u32 = 2;
/// `share.json` of a document shared on its own room.
const ROOM_SHARE_VERSION: u32 = 1;
/// `share.json` of a workspace document.
const WORKSPACE_SHARE_VERSION: u32 = 2;

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
        match read_workspace_share(bytes).ok()? {
            Sharing::Workspace(record) => Some(record),
            Sharing::Room(_) => None,
        }
    }
}

/// The record a file carries: at most one of the two kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Sharing {
    /// Shared on its own room.
    Room(ShareRecord),
    /// A document of a shared workspace.
    Workspace(WorkspaceShareRecord),
}

impl crate::Document {
    /// The share record of a document shared on its own room, `None` otherwise.
    #[must_use]
    pub fn share(&self) -> Option<ShareRecord> {
        match self.share.borrow().as_ref() {
            Some(Sharing::Room(record)) => Some(record.clone()),
            _ => None,
        }
    }

    /// Sets the room record written with the next save, replacing a workspace record, or clears
    /// either; not a history step. A removed device clears it and keeps a plain local file
    /// (R-COLLAB-010).
    pub fn set_share(&self, record: Option<ShareRecord>) {
        self.set_sharing(record.map(Sharing::Room));
    }

    /// The record of a workspace document, `None` otherwise (`specs/0095-workspace` criterion
    /// 15).
    #[must_use]
    pub fn workspace_share(&self) -> Option<WorkspaceShareRecord> {
        match self.share.borrow().as_ref() {
            Some(Sharing::Workspace(record)) => Some(record.clone()),
            _ => None,
        }
    }

    /// Sets the workspace record written with the next save, replacing a room record, or clears
    /// either; not a history step.
    pub fn set_workspace_share(&self, record: Option<WorkspaceShareRecord>) {
        self.set_sharing(record.map(Sharing::Workspace));
    }

    pub(crate) fn set_sharing(&self, sharing: Option<Sharing>) {
        self.commit_state.set_shared(sharing.is_some());
        *self.share.borrow_mut() = sharing;
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

/// The members of `sharing`: `share.json`, then `keyring.log` for a room record.
pub(crate) fn share_members(sharing: &Sharing) -> Result<Vec<(&'static str, Vec<u8>)>, SaveError> {
    match sharing {
        Sharing::Room(record) => {
            let json = ShareJson {
                format_version: ROOM_SHARE_VERSION,
                server: record.server.clone(),
                room: hex(&record.room),
                server_seq: record.server_seq,
                synced_version: record.synced_version.to_text(),
            };
            let json = serde_json::to_vec(&json).map_err(|_| SaveError::Container)?;
            Ok(vec![
                (SHARE_MEMBER, json),
                (KEYRING_MEMBER, record.keyring_log.clone()),
            ])
        }
        Sharing::Workspace(record) => {
            let json = WorkspaceShareJson {
                format_version: WORKSPACE_SHARE_VERSION,
                workspace: hex(&record.workspace),
                document: hex(&record.document),
                synced_version: record.synced_version.to_text(),
            };
            let json = serde_json::to_vec(&json).map_err(|_| SaveError::Container)?;
            Ok(vec![(SHARE_MEMBER, json)])
        }
    }
}

/// The share record of `archive`, `None` for a file that was never shared.
///
/// # Errors
/// [`OpenError::Damaged`] for a version 1 record without its keyring or the other way round, a
/// version 2 record beside a keyring, or a malformed `share.json`; [`OpenError::FormatTooNew`]
/// for a `share.json` newer than this build reads.
pub(crate) fn read_share(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
) -> Result<Option<Sharing>, OpenError> {
    let json = read_optional(archive, SHARE_MEMBER, MAX_KEYRING_BYTES)?;
    let keyring = read_optional(archive, KEYRING_MEMBER, MAX_KEYRING_BYTES)?;
    let Some(json) = json else {
        return match keyring {
            None => Ok(None),
            Some(_) => Err(OpenError::Damaged),
        };
    };
    let version: ShareJsonVersion =
        serde_json::from_slice(&json).map_err(|_| OpenError::Damaged)?;
    if version.format_version > CURRENT_SHARE_FORMAT_VERSION {
        return Err(OpenError::FormatTooNew {
            found: version.format_version,
            supported: CURRENT_SHARE_FORMAT_VERSION,
        });
    }
    match (version.format_version, keyring) {
        (WORKSPACE_SHARE_VERSION, None) => read_workspace_share(&json).map(Some),
        (ROOM_SHARE_VERSION, Some(keyring_log)) => read_room_share(&json, keyring_log).map(Some),
        _ => Err(OpenError::Damaged),
    }
}

fn read_room_share(json: &[u8], keyring_log: Vec<u8>) -> Result<Sharing, OpenError> {
    let json: ShareJson = serde_json::from_slice(json).map_err(|_| OpenError::Damaged)?;
    Ok(Sharing::Room(ShareRecord {
        server: json.server,
        room: unhex(&json.room).ok_or(OpenError::Damaged)?,
        server_seq: json.server_seq,
        synced_version: DocumentVersion::from_text(&json.synced_version)
            .ok_or(OpenError::Damaged)?,
        keyring_log,
    }))
}

fn read_workspace_share(json: &[u8]) -> Result<Sharing, OpenError> {
    let json: WorkspaceShareJson = serde_json::from_slice(json).map_err(|_| OpenError::Damaged)?;
    Ok(Sharing::Workspace(WorkspaceShareRecord {
        workspace: unhex(&json.workspace).ok_or(OpenError::Damaged)?,
        document: unhex(&json.document).ok_or(OpenError::Damaged)?,
        synced_version: DocumentVersion::from_text(&json.synced_version)
            .ok_or(OpenError::Damaged)?,
    }))
}

/// What a container says about its sharing, read without opening the document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContainerShareMembers {
    /// The bytes of `share.json`, when present and not too large.
    pub share_json: Option<Vec<u8>>,
    /// Whether a `keyring.log` is present: the document is shared on its own room.
    pub keyring: bool,
}

/// The raw `share.json` of the container `bytes` and whether it holds a keyring, read without
/// opening the document: what a shared workspace's scan needs per file where no file system
/// does it (`specs/0096-own-devices-sync` criterion 35, the browser). A file that is no
/// container, or whose `share.json` is too large, has neither.
#[must_use]
pub fn container_share_members(bytes: &[u8]) -> ContainerShareMembers {
    let Ok(mut archive) = ZipArchive::new(Cursor::new(bytes)) else {
        return ContainerShareMembers::default();
    };
    let share_json = read_optional(&mut archive, SHARE_MEMBER, MAX_KEYRING_BYTES)
        .ok()
        .flatten();
    let keyring = archive.index_for_name(KEYRING_MEMBER).is_some();
    ContainerShareMembers {
        share_json,
        keyring,
    }
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
        assert!(!members.keyring);
        let none = ContainerShareMembers::default();
        assert_eq!(container_share_members(b"plain"), none);
        let plain = crate::pack(&crate::Document::new(4), "t").unwrap();
        assert_eq!(container_share_members(&plain), none);
    }
}
