//! The disk side of a shared workspace's live list (`specs/0094-workspace` criteria 36, 40 and
//! 41): the sharing members a scan reads per file, the `index.json` cache read back, and the
//! kept copies of deleted documents that Restore writes again.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use curvyo_document_core::{Document, WorkspaceShareRecord, pack};
use curvyo_storage_io::{
    MARKER_DIR, ShareMembers, WorkspaceError, create_document, keep_deleted, open_workspace,
    read_deleted, read_document, read_index, restore_deleted, share_members, write_deleted,
    write_index,
};

const LID: [u8; 16] = [0x11; 16];

fn p(text: &str) -> Vec<String> {
    text.split('/').map(str::to_owned).collect()
}

fn workspace() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    open_workspace(&root, [7; 16]).unwrap();
    (dir, root)
}

fn shared_bytes() -> Vec<u8> {
    let document = Document::new(5);
    document.set_workspace_share(Some(WorkspaceShareRecord {
        workspace: [0x57; 16],
        document: LID,
        synced_version: document.version(),
    }));
    pack(&document, "test").unwrap()
}

/// Criteria 36 and 41: the scan reads `share.json` and whether a keyring is there, without
/// opening the document; a plain file and a file that is no container have neither.
#[test]
fn the_sharing_members_are_read_without_opening_the_document() {
    let (_dir, root) = workspace();
    create_document(&root, &p("Lid.curvyo"), &shared_bytes()).unwrap();
    create_document(
        &root,
        &p("Plain.curvyo"),
        &pack(&Document::new(6), "t").unwrap(),
    )
    .unwrap();
    create_document(&root, &p("Junk.curvyo"), b"not a zip").unwrap();
    let lid = share_members(&root, &p("Lid.curvyo")).unwrap();
    let record = WorkspaceShareRecord::from_share_json(&lid.share_json.unwrap()).unwrap();
    assert_eq!(record.document, LID);
    assert!(!lid.keyring);
    assert_eq!(
        share_members(&root, &p("Plain.curvyo")).unwrap(),
        ShareMembers::default()
    );
    assert_eq!(
        share_members(&root, &p("Junk.curvyo")).unwrap(),
        ShareMembers::default()
    );
    assert!(matches!(
        share_members(&root, &p("Gone.curvyo")),
        Err(WorkspaceError::NotFound)
    ));
}

/// ADR 0036 decision 6: `index.json` reads back as id to path; a damaged file is an empty cache.
#[test]
fn the_index_reads_back_and_damage_is_an_empty_cache() {
    let (_dir, root) = workspace();
    create_document(&root, &p("Lid.curvyo"), &shared_bytes()).unwrap();
    write_index(&root, &[(LID, p("Lid.curvyo"))]).unwrap();
    let index = read_index(&root);
    assert_eq!(index.get(&LID), Some(&p("Lid.curvyo")));
    fs::write(root.join(MARKER_DIR).join("index.json"), b"{").unwrap();
    assert!(read_index(&root).is_empty());
}

/// Criterion 40: a deleted document's file goes to the OS trash after a copy is kept; Restore
/// writes the copy at its place again and drops it; a background write updates the copy.
#[test]
fn a_deleted_document_is_kept_trashed_and_restored() {
    let (_dir, root) = workspace();
    let bytes = shared_bytes();
    create_document(&root, &p("Lid.curvyo"), &bytes).unwrap();
    keep_deleted(&root, &p("Lid.curvyo"), LID).unwrap();
    assert!(!root.join("Lid.curvyo").exists());
    assert_eq!(read_deleted(&root, LID).unwrap(), bytes);
    #[cfg(target_os = "linux")]
    {
        let ours: Vec<_> = trash::os_limited::list()
            .unwrap()
            .into_iter()
            .filter(|item| item.original_parent == root)
            .collect();
        assert_eq!(ours.len(), 1, "the file is in the trash, not erased");
        trash::os_limited::purge_all(ours).unwrap();
    }
    write_deleted(&root, LID, b"newer").unwrap();
    assert_eq!(read_deleted(&root, LID).unwrap(), b"newer");
    write_deleted(&root, LID, &bytes).unwrap();
    restore_deleted(&root, LID, &p("Lid.curvyo")).unwrap();
    assert_eq!(read_document(&root, &p("Lid.curvyo")).unwrap(), bytes);
    assert!(matches!(
        read_deleted(&root, LID),
        Err(WorkspaceError::NotFound)
    ));
    assert!(matches!(
        restore_deleted(&root, LID, &p("Lid.curvyo")),
        Err(WorkspaceError::NotFound)
    ));
}

/// Criterion 44: a removed member's workspace keeps its marker and documents, not its share files.
#[test]
fn removal_leaves_a_local_workspace() {
    let (_dir, root) = workspace();
    create_document(&root, &p("Lid.curvyo"), &shared_bytes()).unwrap();
    curvyo_storage_io::write_marker_share(
        &root,
        &curvyo_storage_io::MarkerShare {
            share_json: b"{}".to_vec(),
            keyring_log: vec![1],
            list: vec![2],
        },
    )
    .unwrap();
    write_index(&root, &[(LID, p("Lid.curvyo"))]).unwrap();
    curvyo_storage_io::remove_marker_share(&root).unwrap();
    assert!(
        curvyo_storage_io::read_marker_share(&root)
            .unwrap()
            .is_none()
    );
    assert!(read_index(&root).is_empty());
    assert!(root.join(MARKER_DIR).join("workspace.toml").exists());
    assert!(root.join("Lid.curvyo").exists());
    curvyo_storage_io::remove_marker_share(&root).unwrap();
}

/// Criterion 44: every document of the removed member's workspace becomes a local file: its
/// `share.json` v2 naming this workspace goes, its content and thumbnail stay; a plain file is
/// not touched.
#[test]
fn removal_turns_every_document_of_the_workspace_local() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    open_workspace(&root, [0x57; 16]).unwrap();
    let png = b"\x89PNG\r\n\x1a\nrest".to_vec();
    let with_png = curvyo_storage_io::with_thumbnail(&shared_bytes(), &png).unwrap();
    create_document(&root, &p("Lid.curvyo"), &with_png).unwrap();
    fs::create_dir(root.join("Sides")).unwrap();
    create_document(&root, &p("Sides/Left.curvyo"), &shared_bytes()).unwrap();
    let plain = pack(&Document::new(6), "test").unwrap();
    create_document(&root, &p("Plain.curvyo"), &plain).unwrap();

    curvyo_storage_io::remove_marker_share(&root).unwrap();

    for path in ["Lid.curvyo", "Sides/Left.curvyo"] {
        let members = share_members(&root, &p(path)).unwrap();
        assert_eq!(members, ShareMembers::default(), "{path}");
        let bytes = read_document(&root, &p(path)).unwrap();
        let opened = curvyo_document_core::unpack(9, &bytes).unwrap();
        assert!(opened.workspace_share().is_none(), "{path}");
    }
    assert!(matches!(
        curvyo_storage_io::probe_document(&root, &p("Lid.curvyo")).unwrap(),
        curvyo_storage_io::DocumentProbe::Readable { thumbnail: Some(t) } if t == png
    ));
    assert_eq!(fs::read(root.join("Plain.curvyo")).unwrap(), plain);
}
