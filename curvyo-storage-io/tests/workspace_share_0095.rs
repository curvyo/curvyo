//! The disk side of a shared workspace (`specs/0095-workspace` criteria 25, 27, 28 and 33; ADR
//! 0037 decisions 1, 4 and 9): the marker's share files, the fresh id, replacing a document,
//! `index.json`, and the folder a joined workspace lands in.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use curvyo_document_core::{Document, pack};
use curvyo_storage_io::{
    DocumentProbe, IndexEntry, JoinFolderError, MARKER_DIR, MarkerShare, WorkspaceError,
    check_join_folder, create_joined_workspace, index_text, open_workspace, parse_marker,
    probe_document, read_marker_share, replace_document, replace_workspace_id, with_thumbnail,
    write_index, write_marker_share,
};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nthumb";

fn p(text: &str) -> Vec<String> {
    text.split('/').map(str::to_owned).collect()
}

fn share() -> MarkerShare {
    MarkerShare {
        share_json: b"{\"format_version\":1}".to_vec(),
        keyring_log: vec![1, 2, 3],
        list: vec![4, 5],
    }
}

fn marker_id(root: &Path) -> [u8; 16] {
    parse_marker(&fs::read_to_string(root.join(MARKER_DIR).join("workspace.toml")).unwrap())
        .unwrap()
}

/// Criteria 25 and 33: the marker's three share files are written and read back; a workspace
/// without them is local; one of them alone is damage.
#[test]
fn the_marker_share_files_round_trip() {
    let work = tempfile::tempdir().unwrap();
    open_workspace(work.path(), [1; 16]).unwrap();
    assert_eq!(read_marker_share(work.path()).unwrap(), None);
    write_marker_share(work.path(), &share()).unwrap();
    assert_eq!(read_marker_share(work.path()).unwrap(), Some(share()));
    fs::remove_file(work.path().join(MARKER_DIR).join("list.loro")).unwrap();
    assert!(matches!(
        read_marker_share(work.path()),
        Err(WorkspaceError::Damaged)
    ));
}

/// ADR 0037 decision 4: Share writes a fresh id into `workspace.toml`.
#[test]
fn share_writes_a_fresh_workspace_id() {
    let work = tempfile::tempdir().unwrap();
    open_workspace(work.path(), [1; 16]).unwrap();
    replace_workspace_id(work.path(), [2; 16]).unwrap();
    assert_eq!(marker_id(work.path()), [2; 16]);
    assert_eq!(open_workspace(work.path(), [3; 16]).unwrap(), [2; 16]);
}

/// Criterion 25: a document's file is replaced by its shared bytes and keeps its thumbnail;
/// a missing file or a path outside is refused.
#[test]
fn replacing_a_document_keeps_its_thumbnail() {
    let work = tempfile::tempdir().unwrap();
    let plain = pack(&Document::new(1), "0.1.0").unwrap();
    fs::write(
        work.path().join("Lid.curvyo"),
        with_thumbnail(&plain, PNG).unwrap(),
    )
    .unwrap();
    let shared = pack(&Document::new(2), "0.1.0").unwrap();
    replace_document(work.path(), &p("Lid.curvyo"), &shared).unwrap();
    assert_eq!(
        probe_document(work.path(), &p("Lid.curvyo")).unwrap(),
        DocumentProbe::Readable {
            thumbnail: Some(PNG.to_vec())
        }
    );
    assert!(matches!(
        replace_document(work.path(), &p("Gone.curvyo"), &shared),
        Err(WorkspaceError::NotFound)
    ));
    assert!(matches!(
        replace_document(work.path(), &p("../x.curvyo"), &shared),
        Err(WorkspaceError::OutsideWorkspace)
    ));
}

/// ADR 0037 decision 1: `index.json` names each document id's file with its size and time;
/// golden text.
#[test]
fn the_index_names_each_document_file() {
    let mut documents = BTreeMap::new();
    documents.insert(
        [0x11; 16],
        IndexEntry {
            path: p("Sides/Left.curvyo"),
            size: 1234,
            modified: 1_800_000_000,
        },
    );
    let golden = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/index_v1.json"),
    )
    .unwrap();
    assert_eq!(index_text(&documents), golden.trim_end());

    let work = tempfile::tempdir().unwrap();
    open_workspace(work.path(), [1; 16]).unwrap();
    fs::write(work.path().join("Lid.curvyo"), b"lid").unwrap();
    write_index(
        work.path(),
        &[
            ([0x22; 16], p("Lid.curvyo")),
            ([0x33; 16], p("Gone.curvyo")),
        ],
    )
    .unwrap();
    let written: serde_json::Value =
        serde_json::from_slice(&fs::read(work.path().join(MARKER_DIR).join("index.json")).unwrap())
            .unwrap();
    assert_eq!(written["format_version"], 1);
    let docs = written["documents"].as_object().unwrap();
    assert_eq!(docs.len(), 1, "a missing file is left out");
    assert_eq!(docs["22222222222222222222222222222222"]["size"], 3);
}

/// Criterion 27 and `ux-notes.md` section 8: a joined workspace needs an empty folder outside
/// any workspace; it then gets the link's workspace id and nothing else.
#[test]
fn a_joined_workspace_lands_in_an_empty_folder_outside_any_workspace() {
    let outer = tempfile::tempdir().unwrap();
    let empty = outer.path().join("Box job");
    fs::create_dir(&empty).unwrap();
    assert_eq!(check_join_folder(&empty), Ok(()));
    let full = outer.path().join("Full");
    fs::create_dir(&full).unwrap();
    fs::write(full.join("note.txt"), b"x").unwrap();
    assert_eq!(check_join_folder(&full), Err(JoinFolderError::NotEmpty));
    assert_eq!(
        check_join_folder(&outer.path().join("Missing")),
        Err(JoinFolderError::NotWritable)
    );
    let workspace = outer.path().join("Workspace");
    fs::create_dir(&workspace).unwrap();
    open_workspace(&workspace, [1; 16]).unwrap();
    let inside = workspace.join("Inside");
    fs::create_dir(&inside).unwrap();
    assert_eq!(
        check_join_folder(&inside),
        Err(JoinFolderError::InsideWorkspace)
    );
    assert_eq!(JoinFolderError::InsideWorkspace.code(), "inside-workspace");

    create_joined_workspace(&empty, [0x57; 16]).unwrap();
    assert_eq!(marker_id(&empty), [0x57; 16]);
    let entries: Vec<_> = fs::read_dir(&empty).unwrap().collect();
    assert_eq!(entries.len(), 1, "only the marker directory");
    assert!(create_joined_workspace(&empty, [0x58; 16]).is_err());
}
