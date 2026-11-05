//! The marker's ended record of a removed or left workspace (`specs/0097-membership-lifecycle`
//! criteria 15, 20 and 33; ADR 0041 decision 6): the marker keeps `share.ended.json`, its list
//! snapshot and `index.json`, loses `share.json` and `keyring.log`, and every document keeps its
//! `share.json` v2; such a workspace reads as not shared until Sync again writes the share files.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use curvyo_document_core::{Document, WorkspaceShareRecord, pack, unpack};
use curvyo_storage_io::{
    MARKER_DIR, MarkerShare, create_document, end_marker_share, open_workspace, read_document,
    read_index, read_marker_ended, read_marker_share, remove_marker_share, write_index,
    write_marker_share,
};

const WS: [u8; 16] = [0x57; 16];

fn p(text: &str) -> Vec<String> {
    text.split('/').map(str::to_owned).collect()
}

fn shared_document() -> Vec<u8> {
    let doc = Document::new(5);
    doc.set_workspace_share(Some(WorkspaceShareRecord {
        workspace: WS,
        document: [0xd1; 16],
        synced_version: doc.version(),
    }));
    pack(&doc, "test").unwrap()
}

fn marker() -> MarkerShare {
    MarkerShare {
        share_json: b"{\"format_version\":1}".to_vec(),
        keyring_log: vec![1, 2, 3],
        list: vec![4, 5, 6],
    }
}

/// Criteria 15 and 20: ending keeps the record, the list and the index, drops the marker's
/// `share.json` and `keyring.log`, and leaves every document's `share.json` v2 in place.
#[test]
fn an_ended_workspace_keeps_its_record_list_index_and_document_records() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    open_workspace(&root, WS).unwrap();
    create_document(&root, &p("Lid.curvyo"), &shared_document()).unwrap();
    write_marker_share(&root, &marker()).unwrap();
    write_index(&root, &[([0xd1; 16], p("Lid.curvyo"))]).unwrap();

    end_marker_share(&root, b"{\"ended\":\"left\"}").unwrap();

    assert_eq!(
        read_marker_share(&root).unwrap(),
        None,
        "reads as not shared"
    );
    let ended = read_marker_ended(&root).unwrap().expect("an ended record");
    assert_eq!(ended.ended_json, b"{\"ended\":\"left\"}");
    assert_eq!(
        ended.list,
        vec![4, 5, 6],
        "the list the cursor counts stays"
    );
    assert_eq!(read_index(&root).len(), 1, "the index stays");
    let marker_dir = root.join(MARKER_DIR);
    assert!(!marker_dir.join("share.json").exists());
    assert!(!marker_dir.join("keyring.log").exists());
    let lid = unpack(9, &read_document(&root, &p("Lid.curvyo")).unwrap()).unwrap();
    assert_eq!(
        lid.workspace_share().map(|r| r.workspace),
        Some(WS),
        "the document keeps share.json v2"
    );
    // Idempotent: a crash before the end of the first call does no harm.
    end_marker_share(&root, b"{\"ended\":\"left\"}").unwrap();
}

/// Criterion 33 and ADR 0041 decision 6: a workspace without an ended record has none;
/// complete share files beside an ended record win (as `share.json` does in a document), and
/// an ended record beside the kept list or a stray share file reads as not shared, never as
/// damaged.
#[test]
fn complete_share_files_win_over_an_ended_record() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    open_workspace(root, WS).unwrap();
    assert_eq!(read_marker_ended(root).unwrap(), None);
    write_marker_share(root, &marker()).unwrap();
    fs::write(root.join(MARKER_DIR).join("share.ended.json"), b"{}").unwrap();
    assert_eq!(read_marker_share(root).unwrap(), Some(marker()));
    assert_eq!(read_marker_ended(root).unwrap(), None);
    fs::remove_file(root.join(MARKER_DIR).join("keyring.log")).unwrap();
    assert_eq!(read_marker_share(root).unwrap(), None);
    assert!(read_marker_ended(root).unwrap().is_some());
}

/// Criterion 21: Sync again writes the share files and the ended record goes; the full removal
/// (no way back) drops the record too.
#[test]
fn sync_again_and_the_full_removal_drop_the_ended_record() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    open_workspace(root, WS).unwrap();
    write_marker_share(root, &marker()).unwrap();
    end_marker_share(root, b"{}").unwrap();
    write_marker_share(root, &marker()).unwrap();
    assert_eq!(read_marker_share(root).unwrap(), Some(marker()));
    assert_eq!(read_marker_ended(root).unwrap(), None);

    end_marker_share(root, b"{}").unwrap();
    remove_marker_share(root).unwrap();
    assert_eq!(read_marker_ended(root).unwrap(), None);
    assert!(!root.join(MARKER_DIR).join("list.loro").exists());
}

/// An ended record without a list (an older marker) gives an empty list.
#[test]
fn an_ended_record_without_a_list_reads_an_empty_list() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    open_workspace(root, WS).unwrap();
    end_marker_share(root, b"{}").unwrap();
    assert_eq!(
        read_marker_ended(root).unwrap().unwrap().list,
        Vec::<u8>::new()
    );
}
