//! Reads a workspace document by its path inside the workspace folder (`specs/0095-workspace`
//! criteria 6, 8, 9): its bytes for opening, or only what a tree row shows, the container's
//! `thumbnail.png` or why the file cannot be read, without opening the document.

use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use std::path::Path;

use zip::ZipArchive;

use crate::workspace::{WorkspaceError, io_error, resolve};

/// The container member that holds the document's thumbnail (ADR 0004 §1).
pub(crate) const THUMBNAIL_MEMBER: &str = "thumbnail.png";
/// The member every `.curvyo` container has.
const MANIFEST_MEMBER: &str = "manifest.json";
/// The largest thumbnail a row reads; a larger one is shown as the placeholder **(Proposal)**.
pub const MAX_THUMBNAIL_BYTES: u64 = 1024 * 1024;
/// The largest manifest read to check the container.
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

/// What a tree row shows of a document file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentProbe {
    /// A container with a manifest; its thumbnail when it has one of at most
    /// [`MAX_THUMBNAIL_BYTES`].
    Readable {
        /// The PNG bytes of `thumbnail.png`.
        thumbnail: Option<Vec<u8>>,
    },
    /// Not a zip archive, or one without a manifest: not a Curvyo file.
    NotCurvyo,
    /// A zip archive that cannot be read through.
    Damaged,
}

/// The bytes of the document at `path`.
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::NotFound`], [`WorkspaceError::Io`].
pub fn read_document(root: &Path, path: &[String]) -> Result<Vec<u8>, WorkspaceError> {
    let target = resolve(root, path)?;
    fs::read(&target).map_err(|source| match source.kind() {
        ErrorKind::NotFound => WorkspaceError::NotFound,
        _ => io_error(&target, source),
    })
}

/// What the row of the document at `path` shows: its thumbnail, or why it cannot be read. Only
/// the zip directory, the manifest and the thumbnail are read; nothing is written.
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::NotFound`], [`WorkspaceError::Io`].
pub fn probe_document(root: &Path, path: &[String]) -> Result<DocumentProbe, WorkspaceError> {
    let target = resolve(root, path)?;
    let mut file = File::open(&target).map_err(|source| match source.kind() {
        ErrorKind::NotFound => WorkspaceError::NotFound,
        _ => io_error(&target, source),
    })?;
    let mut signature = [0_u8; 4];
    if file.read_exact(&mut signature).is_err()
        || !(signature == *b"PK\x03\x04" || signature == *b"PK\x05\x06")
    {
        return Ok(DocumentProbe::NotCurvyo);
    }
    let Ok(mut archive) = ZipArchive::new(file) else {
        return Ok(DocumentProbe::Damaged);
    };
    match read_member(&mut archive, MANIFEST_MEMBER, MAX_MANIFEST_BYTES) {
        Member::Missing => return Ok(DocumentProbe::NotCurvyo),
        Member::Broken => return Ok(DocumentProbe::Damaged),
        Member::TooLarge | Member::Read(_) => {}
    }
    let thumbnail = match read_member(&mut archive, THUMBNAIL_MEMBER, MAX_THUMBNAIL_BYTES) {
        Member::Read(bytes) => Some(bytes),
        Member::Broken => return Ok(DocumentProbe::Damaged),
        Member::Missing | Member::TooLarge => None,
    };
    Ok(DocumentProbe::Readable { thumbnail })
}

/// The container's `share.json` member.
pub(crate) const SHARE_MEMBER: &str = "share.json";
/// The keyring member of a document shared on its own room.
const KEYRING_MEMBER: &str = "keyring.log";
/// The largest `share.json` read.
const MAX_SHARE_BYTES: u64 = 64 * 1024;

/// What a document file says about its sharing, read without opening the document: a shared
/// workspace's scan reads the document id from it (`specs/0095-workspace` criteria 36 and 41).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShareMembers {
    /// The bytes of `share.json`, when present and readable.
    pub share_json: Option<Vec<u8>>,
    /// Whether a `keyring.log` is present: the document is shared on its own room.
    pub keyring: bool,
}

/// The sharing members of the document at `path`; a file that is no container has none.
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::NotFound`], [`WorkspaceError::Io`].
pub fn share_members(root: &Path, path: &[String]) -> Result<ShareMembers, WorkspaceError> {
    let target = resolve(root, path)?;
    let file = File::open(&target).map_err(|source| match source.kind() {
        ErrorKind::NotFound => WorkspaceError::NotFound,
        _ => io_error(&target, source),
    })?;
    let Ok(mut archive) = ZipArchive::new(file) else {
        return Ok(ShareMembers::default());
    };
    let share_json = match read_member(&mut archive, SHARE_MEMBER, MAX_SHARE_BYTES) {
        Member::Read(bytes) => Some(bytes),
        Member::Missing | Member::TooLarge | Member::Broken => None,
    };
    let keyring = archive.index_for_name(KEYRING_MEMBER).is_some();
    Ok(ShareMembers {
        share_json,
        keyring,
    })
}

enum Member {
    Missing,
    TooLarge,
    Broken,
    Read(Vec<u8>),
}

fn read_member(archive: &mut ZipArchive<File>, name: &str, limit: u64) -> Member {
    let mut member = match archive.by_name(name) {
        Ok(member) => member,
        Err(zip::result::ZipError::FileNotFound) => return Member::Missing,
        Err(_) => return Member::Broken,
    };
    if member.size() > limit {
        return Member::TooLarge;
    }
    let mut bytes = Vec::new();
    match member.by_ref().take(limit + 1).read_to_end(&mut bytes) {
        Ok(_) if bytes.len() as u64 <= limit => Member::Read(bytes),
        Ok(_) => Member::TooLarge,
        Err(_) => Member::Broken,
    }
}
