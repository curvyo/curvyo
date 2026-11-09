//! Impure capabilities the desktop client needs, one module each
//! (ADR 0011 §2). This slice only needs the filesystem module; the other
//! modules (OS CSPRNG, clock, credential store, HTTP, git wire, relay
//! socket) belong to later stories and get no code here
//! (`specs/0001-project-file-foundation/adrs.md`).

// `CLAUDE.md` §5 allows unwrap/expect in tests; only production code is held
// to the stricter rule.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod container_thumbnail;
mod credentials;
mod devices;
mod filesystem;
mod folder_texts;
mod font_archives;
mod font_documents;
mod font_files;
mod font_subfolders;
mod fonts;
mod recent_workspaces;
mod room_keys;
mod svg_fonts;
mod workspace;
mod workspace_around;
mod workspace_deleted;
mod workspace_documents;
mod workspace_ended;
mod workspace_share_files;

pub use container_thumbnail::{ThumbnailError, with_thumbnail};
pub use credentials::{CredentialError, CredentialStore, KEY_FILE, credential_slot};
pub use devices::{
    DEFAULT_SYNCED_FOLDER, DEVICES_DIR, DEVICES_LIST_FILE, DeviceGroupFiles, DevicesError,
    GROUP_FILE, GROUP_FORMAT_VERSION, GROUP_KEYRING_FILE, GroupRecord, SyncedFolderError,
    arrival_folder, prepare_synced_folder, read_device_group, remove_device_group,
    safe_folder_name, write_device_group,
};
pub use filesystem::{
    FsError, ensure_parent_dir, read_optional, read_to_vec, rename_replacing, write_atomic,
};
pub use folder_texts::{read_dir_texts, remove_file};
pub use font_archives::{AddedArchive, ArchiveRefusal, add_font_archive, is_archive};
pub use font_documents::{MAX_SHOWN_DOCUMENT_BYTES, font_document_pdf, read_font_document};
pub use font_files::{AddedFonts, RemovedFonts, add_font_files, remove_font_files};
pub use font_subfolders::{FolderDocument, FontFolder, remove_add_leftovers};
pub use fonts::{
    FolderFonts, FontFace, FontSource, MAX_FONT_FILE_BYTES, list_font_folder, list_system_fonts,
};
pub use recent_workspaces::{
    MAX_RECENT_WORKSPACES, RECENT_WORKSPACES_FILE, RECENT_WORKSPACES_FORMAT, RecentWorkspace,
    RecentWorkspaces, RecentWorkspacesError,
};
pub use room_keys::{decode_room_keys, encode_room_keys};
pub use workspace::{
    MARKER_DIR, MARKER_FILE, ScannedEntry, ScannedKind, WORKSPACE_FORMAT_VERSION, WorkspaceError,
    create_document, create_folder, fresh_workspace_id, marker_text, move_entry, open_workspace,
    parse_marker, scan_workspace, trash_entry,
};
pub use workspace_around::{make_workspace_around, workspace_folder_for};
pub use workspace_deleted::{
    DELETED_DIR, keep_deleted, read_deleted, restore_deleted, write_deleted,
};
pub use workspace_documents::{
    DocumentProbe, MAX_THUMBNAIL_BYTES, ShareMembers, probe_document, read_document, share_members,
};
pub use workspace_ended::{EndedMarker, MARKER_ENDED_FILE, end_marker_share, read_marker_ended};
pub use workspace_share_files::{
    INDEX_FORMAT_VERSION, IndexEntry, JoinFolderError, MARKER_INDEX_FILE, MARKER_KEYRING_FILE,
    MARKER_LIST_FILE, MARKER_SHARE_FILE, MarkerShare, check_join_folder, create_joined_workspace,
    index_text, read_index, read_marker_share, remove_marker_share, replace_document,
    replace_workspace_id, write_index, write_marker_share,
};
