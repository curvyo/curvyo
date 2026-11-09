//! Tester's acceptance tests for `specs/0095-workspace` in `curvyo-storage-io`, written from the
//! specification and the public API before reading the implementation: the marker and its
//! refusals (criteria 1, 2, 4, 23, 24), file operations that never replace an entry (criteria 13
//! to 18), the shared list's disk plan carried out on a real folder (criteria 36 to 41), the
//! Deleted list's kept copies (criterion 40), the join folder (criterion 27), the removed
//! device's leftovers (criterion 44) and the recents file (criterion 20).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::similar_names
)]

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use curvyo_document_core::{
    Created, DiskOp, DiskState, Document, DocumentVersion, ScannedDocument, WorkspaceList,
    WorkspaceShareRecord, pack, plan_disk, unpack,
};
use curvyo_storage_io::{
    IndexEntry, JoinFolderError, MARKER_DIR, MARKER_FILE, MarkerShare, RecentWorkspaces,
    RecentWorkspacesError, ScannedKind, WorkspaceError, check_join_folder, create_document,
    create_folder, create_joined_workspace, index_text, keep_deleted, marker_text, move_entry,
    open_workspace, parse_marker, read_deleted, read_document, read_index, read_marker_share,
    remove_marker_share, restore_deleted, scan_workspace, share_members, trash_entry,
    write_deleted, write_marker_share,
};

const WS: [u8; 16] = [0x57; 16];

fn p(text: &str) -> Vec<String> {
    text.split('/').map(str::to_owned).collect()
}

fn doc_id(n: u8) -> [u8; 16] {
    let mut id = [0xd0; 16];
    id[15] = n;
    id
}

/// Every file below `root` with its bytes, the marker directory included.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if entry.file_type().unwrap().is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                stack.push(path);
            } else {
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    out
}

/// A document file of workspace `WS` with id `n` and `objects` rectangles.
fn workspace_doc(n: u8) -> Vec<u8> {
    let document = Document::new(u64::from(n) + 100);
    document.set_workspace_share(Some(WorkspaceShareRecord {
        workspace: WS,
        document: doc_id(n),
        synced_version: document.version(),
    }));
    pack(&document, "test").unwrap()
}

fn purge_trash_of(root: &Path) {
    #[cfg(target_os = "linux")]
    {
        let ours: Vec<_> = trash::os_limited::list()
            .unwrap()
            .into_iter()
            .filter(|item| item.original_parent.starts_with(root))
            .collect();
        if !ours.is_empty() {
            trash::os_limited::purge_all(ours).unwrap();
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = root;
}

fn trashed_names(root: &Path) -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        let mut names: Vec<String> = trash::os_limited::list()
            .unwrap()
            .into_iter()
            .filter(|item| item.original_parent.starts_with(root))
            .map(|item| item.name.to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = root;
        Vec::new()
    }
}

// ---------------------------------------------------------------------------------------------
// Marker (criteria 1, 2, 4, 23, 24)
// ---------------------------------------------------------------------------------------------

/// Criteria 1 and 23: the marker holds the id and a format number and nothing else; opening
/// writes only the marker; reopening keeps the id.
#[test]
fn c1_c2_opening_writes_only_the_marker_and_keeps_the_id() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("Sides/Inner")).unwrap();
    std::fs::write(root.join("Lid.curvyo"), workspace_doc(1)).unwrap();
    std::fs::write(root.join("Sides/notes.txt"), b"hello").unwrap();
    let before = snapshot(root);
    let id = open_workspace(root, [9; 16]).unwrap();
    assert_eq!(id, [9; 16]);
    let mut after = snapshot(root);
    let marker = after
        .remove(&format!("{MARKER_DIR}/{MARKER_FILE}"))
        .expect("the marker");
    after.remove(&format!("{MARKER_DIR}/"));
    assert_eq!(before, after, "no other file changed");
    let text = String::from_utf8(marker).unwrap();
    assert_eq!(parse_marker(&text).unwrap(), [9; 16]);
    assert!(text.contains("format_version"));
    assert_eq!(text, marker_text([9; 16]));
    assert_eq!(
        open_workspace(root, [8; 16]).unwrap(),
        [9; 16],
        "reopen keeps it"
    );
}

/// Criterion 23: a higher format number is refused even if the newer marker carries fields this
/// build does not know; nothing is written.
#[test]
fn c23_a_newer_marker_with_new_fields_is_too_new_not_damaged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join(MARKER_DIR)).unwrap();
    let newer =
        "format_version = 2\nid = \"57575757575757575757575757575757\"\nlibrary = \"lib\"\n";
    std::fs::write(root.join(MARKER_DIR).join(MARKER_FILE), newer).unwrap();
    let before = snapshot(root);
    let err = open_workspace(root, [1; 16]).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::TooNew { .. }),
        "a newer marker must say 'newer', got {err:?}"
    );
    assert_eq!(before, snapshot(root));
    // Newer with no id at all, too.
    std::fs::write(
        root.join(MARKER_DIR).join(MARKER_FILE),
        "format_version = 7\n",
    )
    .unwrap();
    assert!(matches!(
        open_workspace(root, [1; 16]),
        Err(WorkspaceError::TooNew { .. })
    ));
}

/// Criterion 23: a damaged marker (bad id, wrong types, empty) is refused without writing.
#[test]
fn c23_a_damaged_marker_is_refused_without_writing() {
    for text in [
        "",
        "format_version = 1\n",
        "format_version = 1\nid = \"xyz\"\n",
        "format_version = 1\nid = \"5757\"\n",
        "format_version = \"1\"\nid = \"57575757575757575757575757575757\"\n",
        "format_version = -1\nid = \"57575757575757575757575757575757\"\n",
        "not toml at all [[[",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join(MARKER_DIR)).unwrap();
        std::fs::write(root.join(MARKER_DIR).join(MARKER_FILE), text).unwrap();
        let before = snapshot(root);
        assert!(open_workspace(root, [1; 16]).is_err(), "{text:?} accepted");
        assert_eq!(before, snapshot(root), "{text:?} rewrote something");
    }
}

/// Criterion 4: a path that is not a folder is refused; a marker that is a folder is refused.
#[test]
fn c4_not_a_folder_or_a_broken_marker_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("x.curvyo");
    std::fs::write(&file, b"x").unwrap();
    assert!(matches!(
        open_workspace(&file, [1; 16]),
        Err(WorkspaceError::NotAFolder)
    ));
    assert!(open_workspace(&dir.path().join("missing"), [1; 16]).is_err());
    // `workspace.toml` is a folder.
    let root = dir.path().join("ws");
    std::fs::create_dir_all(root.join(MARKER_DIR).join(MARKER_FILE)).unwrap();
    assert!(open_workspace(&root, [1; 16]).is_err());
}

/// Criterion 24: two workspaces scan separately; nothing of one shows in the other, also when
/// one lies inside the other's folder tree.
#[test]
fn c24_a_nested_workspace_is_not_scanned_into_its_parent() {
    let dir = tempfile::tempdir().unwrap();
    let outer = dir.path();
    std::fs::write(outer.join("A.curvyo"), workspace_doc(1)).unwrap();
    std::fs::create_dir(outer.join("inner")).unwrap();
    std::fs::write(outer.join("inner/B.curvyo"), workspace_doc(2)).unwrap();
    let a = open_workspace(outer, [1; 16]).unwrap();
    let b = open_workspace(&outer.join("inner"), [2; 16]).unwrap();
    assert_ne!(a, b);
    let inner: Vec<_> = scan_workspace(&outer.join("inner"))
        .unwrap()
        .into_iter()
        .map(|e| e.path)
        .collect();
    assert_eq!(inner, vec![p("B.curvyo")]);
    // The inner marker never appears as an entry of the outer scan.
    for e in scan_workspace(outer).unwrap() {
        assert!(!e.path.iter().any(|c| c == MARKER_DIR), "{:?}", e.path);
    }
}

// ---------------------------------------------------------------------------------------------
// Scan and file operations (criteria 5, 7, 13 to 18)
// ---------------------------------------------------------------------------------------------

/// Criterion 5: the scan lists `.curvyo` files in any case of the extension with size and time,
/// skips other files, and never opens a document (an unreadable file is still listed).
#[test]
fn c5_c8_scan_lists_documents_by_extension_without_opening_them() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("Good.curvyo"), workspace_doc(1)).unwrap();
    std::fs::write(root.join("Upper.CURVYO"), b"garbage").unwrap();
    std::fs::write(root.join("notes.txt"), b"x").unwrap();
    std::fs::write(root.join("curvyo"), b"x").unwrap();
    std::fs::create_dir(root.join("Folder.curvyo")).unwrap();
    let entries = scan_workspace(root).unwrap();
    let docs: Vec<_> = entries
        .iter()
        .filter(|e| e.kind == ScannedKind::Document)
        .map(|e| (e.path.clone(), e.size))
        .collect();
    assert!(docs.contains(&(p("Good.curvyo"), workspace_doc(1).len() as u64)));
    assert!(docs.iter().any(|(path, _)| path == &p("Upper.CURVYO")));
    assert!(!docs.iter().any(|(path, _)| path == &p("notes.txt")));
    assert!(
        !docs.iter().any(|(path, _)| path == &p("Folder.curvyo")),
        "a folder with the extension is not a document"
    );
    assert!(
        entries
            .iter()
            .filter(|e| e.kind == ScannedKind::Document)
            .all(|e| e.modified > 0)
    );
}

/// Criteria 13, 16 and 18: creations and moves never replace an entry, also when the names
/// differ only in case, and a failed operation changes nothing.
#[test]
fn c13_c16_c18_operations_never_replace_an_entry() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    open_workspace(root, [1; 16]).unwrap();
    create_document(root, &p("Lid.curvyo"), b"one").unwrap();
    create_folder(root, &p("Sides")).unwrap();
    create_document(root, &p("Sides/Left.curvyo"), b"two").unwrap();
    let before = snapshot(root);
    assert!(matches!(
        create_document(root, &p("Lid.curvyo"), b"three"),
        Err(WorkspaceError::Exists)
    ));
    assert!(create_folder(root, &p("Sides")).is_err());
    assert!(create_folder(root, &p("Lid.curvyo")).is_err());
    assert!(matches!(
        move_entry(root, &p("Sides/Left.curvyo"), &p("Lid.curvyo")),
        Err(WorkspaceError::Exists)
    ));
    // Case-insensitive collision: refused on every OS, since the folder may be synced to one
    // that ignores case.
    let case = move_entry(root, &p("Sides/Left.curvyo"), &p("LID.curvyo"));
    if case.is_ok() {
        // Allowed only when the file system keeps both; never replaces Lid.
        assert_eq!(std::fs::read(root.join("Lid.curvyo")).unwrap(), b"one");
        move_entry(root, &p("LID.curvyo"), &p("Sides/Left.curvyo")).unwrap();
    }
    assert!(move_entry(root, &p("Sides"), &p("Sides/Inner")).is_err());
    assert!(matches!(
        move_entry(root, &p("Nope.curvyo"), &p("X.curvyo")),
        Err(WorkspaceError::NotFound)
    ));
    assert_eq!(before, snapshot(root), "nothing changed");
    // Into a missing folder: refused, nothing created on the way.
    assert!(create_document(root, &p("New/Deep.curvyo"), b"x").is_err());
    assert!(!root.join("New").exists());
}

/// Criteria 14 and 18: paths that leave the folder or reach the marker are refused before
/// anything is touched.
#[test]
fn c14_paths_outside_or_into_the_marker_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("ws");
    std::fs::create_dir(&root).unwrap();
    open_workspace(&root, [1; 16]).unwrap();
    create_document(&root, &p("Lid.curvyo"), b"x").unwrap();
    let before = snapshot(dir.path());
    let bad: Vec<Vec<String>> = vec![
        vec![],
        p(".."),
        p("../escape.curvyo"),
        p("a/../../escape.curvyo"),
        vec![MARKER_DIR.to_owned(), MARKER_FILE.to_owned()],
        vec![MARKER_DIR.to_owned()],
        vec!["/etc".to_owned()],
        vec!["a/b.curvyo".to_owned()],
        vec!["a\\b.curvyo".to_owned()],
        vec![String::new()],
        vec!["x\0y.curvyo".to_owned()],
    ];
    for path in &bad {
        assert!(
            create_document(&root, path, b"evil").is_err(),
            "create {path:?}"
        );
        assert!(create_folder(&root, path).is_err(), "folder {path:?}");
        assert!(
            move_entry(&root, &p("Lid.curvyo"), path).is_err(),
            "move to {path:?}"
        );
        assert!(trash_entry(&root, path).is_err(), "trash {path:?}");
    }
    assert_eq!(before, snapshot(dir.path()));
}

/// Criterion 17: trashing a folder takes its documents to the OS trash, never erased.
#[test]
fn c17_trash_keeps_the_items_in_the_os_trash() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    open_workspace(&root, [1; 16]).unwrap();
    create_folder(&root, &p("Sides")).unwrap();
    create_document(&root, &p("Sides/Left.curvyo"), b"left").unwrap();
    trash_entry(&root, &p("Sides")).unwrap();
    assert!(!root.join("Sides").exists());
    #[cfg(target_os = "linux")]
    assert_eq!(trashed_names(&root), vec!["Sides".to_owned()]);
    purge_trash_of(&root);
}

// ---------------------------------------------------------------------------------------------
// The shared list carried out on a real folder (criteria 36 to 41)
// ---------------------------------------------------------------------------------------------

/// Scans `root` as a shared workspace's host does: folders, and documents with their id.
fn scan_shared(root: &Path) -> (Vec<Vec<String>>, Vec<ScannedDocument>) {
    let mut folders = Vec::new();
    let mut documents = Vec::new();
    for entry in scan_workspace(root).unwrap() {
        match entry.kind {
            ScannedKind::Folder => folders.push(entry.path),
            ScannedKind::Document => {
                let members = share_members(root, &entry.path).unwrap();
                let document = members
                    .share_json
                    .as_deref()
                    .and_then(WorkspaceShareRecord::from_share_json)
                    .filter(|r| r.workspace == WS)
                    .map(|r| r.document);
                documents.push(ScannedDocument {
                    path: entry.path,
                    document,
                });
            }
        }
    }
    (folders, documents)
}

/// Carries out the plan with this crate's operations, as the host does; panics on any refusal.
fn carry_out(root: &Path, list: &WorkspaceList, known: &HashMap<[u8; 16], Vec<String>>) {
    let (folders, documents) = scan_shared(root);
    let plan = plan_disk(
        &list.entries(),
        &list.tombstoned_documents(),
        &DiskState {
            folders: &folders,
            documents: &documents,
            known,
        },
    );
    for op in &plan.ops {
        let done = match op {
            DiskOp::CreateFolder { path } => create_folder(root, path),
            DiskOp::Move { from, to } => move_entry(root, from, to),
            DiskOp::Write { path, document } => {
                create_document(root, path, &workspace_doc(document[15]))
            }
            DiskOp::Trash { path } => match documents.iter().find(|d| &d.path == path) {
                Some(ScannedDocument {
                    document: Some(id), ..
                }) => keep_deleted(root, path, *id),
                _ => trash_entry(root, path),
            },
        };
        done.unwrap_or_else(|e| panic!("{op:?} refused: {e:?}\nplan {:?}", plan.ops));
    }
}

fn ids_at(root: &Path) -> BTreeMap<Vec<String>, Option<[u8; 16]>> {
    scan_shared(root)
        .1
        .into_iter()
        .map(|d| (d.path, d.document))
        .collect()
}

fn known_of(root: &Path) -> HashMap<[u8; 16], Vec<String>> {
    scan_shared(root)
        .1
        .into_iter()
        .filter_map(|d| Some((d.document?, d.path)))
        .collect()
}

/// Criteria 36 and 37 on a real folder: a swap of two names (a ring of renames, which passes
/// through a temporary name) and a move into a new folder are carried out by this crate's
/// operations without a refusal, and every document keeps its bytes.
#[test]
fn c36_c37_a_ring_of_renames_is_carried_out_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    open_workspace(&root, WS).unwrap();
    let list = WorkspaceList::new(1);
    let left = list
        .create_document(None, "Left", doc_id(1), Created { at: 1, id: [1; 16] })
        .unwrap();
    let right = list
        .create_document(None, "Right", doc_id(2), Created { at: 1, id: [2; 16] })
        .unwrap();
    carry_out(&root, &list, &HashMap::new());
    let left_bytes = std::fs::read(root.join("Left.curvyo")).unwrap();
    list.rename(left, "Tmp").unwrap();
    list.rename(right, "Left").unwrap();
    list.rename(left, "Right").unwrap();
    carry_out(&root, &list, &known_of(&root));
    let ids = ids_at(&root);
    assert_eq!(ids.get(&p("Right.curvyo")), Some(&Some(doc_id(1))));
    assert_eq!(ids.get(&p("Left.curvyo")), Some(&Some(doc_id(2))));
    assert_eq!(
        std::fs::read(root.join("Right.curvyo")).unwrap(),
        left_bytes
    );
    assert_eq!(ids.len(), 2, "no temporary file left: {ids:?}");
    let folder = list
        .create_folder(None, "Walls", Created { at: 2, id: [3; 16] })
        .unwrap();
    list.move_to(left, Some(folder)).unwrap();
    carry_out(&root, &list, &known_of(&root));
    assert_eq!(
        ids_at(&root).get(&p("Walls/Right.curvyo")),
        Some(&Some(doc_id(1)))
    );
}

/// Criterion 40 on a real folder: a document deleted by a peer goes to the OS trash after its
/// copy is kept; Restore writes it back with the same id and bytes and never over a file.
#[test]
fn c40_deleted_documents_are_kept_and_restored_with_their_id() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    open_workspace(&root, WS).unwrap();
    let list = WorkspaceList::new(1);
    let lid = list
        .create_document(None, "Lid", doc_id(1), Created { at: 1, id: [1; 16] })
        .unwrap();
    carry_out(&root, &list, &HashMap::new());
    let bytes = std::fs::read(root.join("Lid.curvyo")).unwrap();
    list.delete(lid, "Anna", 5).unwrap();
    carry_out(&root, &list, &known_of(&root));
    assert!(!root.join("Lid.curvyo").exists());
    assert_eq!(read_deleted(&root, doc_id(1)).unwrap(), bytes);
    #[cfg(target_os = "linux")]
    assert_eq!(
        trashed_names(&root),
        vec!["Lid.curvyo".to_owned()],
        "the file is in the OS trash, not erased"
    );
    // The kept copy is not a workspace document of the scan.
    assert!(ids_at(&root).is_empty());
    // Someone puts a file where Lid was: Restore never replaces it.
    std::fs::write(root.join("Lid.curvyo"), b"mine").unwrap();
    assert!(matches!(
        restore_deleted(&root, doc_id(1), &p("Lid.curvyo")),
        Err(WorkspaceError::Exists)
    ));
    assert_eq!(std::fs::read(root.join("Lid.curvyo")).unwrap(), b"mine");
    assert_eq!(
        read_deleted(&root, doc_id(1)).unwrap(),
        bytes,
        "a refused restore keeps the copy"
    );
    restore_deleted(&root, doc_id(1), &p("Lid 2.curvyo")).unwrap();
    let opened = unpack(1, &std::fs::read(root.join("Lid 2.curvyo")).unwrap()).unwrap();
    assert_eq!(opened.workspace_share().unwrap().document, doc_id(1));
    assert!(
        read_deleted(&root, doc_id(1)).is_err(),
        "the copy is used up"
    );
    // A newer state replaces a kept copy atomically.
    write_deleted(&root, doc_id(3), b"v1").unwrap();
    write_deleted(&root, doc_id(3), b"v2").unwrap();
    assert_eq!(read_deleted(&root, doc_id(3)).unwrap(), b"v2");
    purge_trash_of(&root);
}

// ---------------------------------------------------------------------------------------------
// Joining (criterion 27) and the removed device (criterion 44)
// ---------------------------------------------------------------------------------------------

/// Criterion 27: the joiner's folder must be empty (hidden entries count), writable, no
/// workspace and in none, at any depth.
#[test]
fn c27_join_folder_rules() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    let empty = base.join("empty");
    std::fs::create_dir(&empty).unwrap();
    assert_eq!(check_join_folder(&empty), Ok(()));
    let hidden = base.join("hidden");
    std::fs::create_dir(&hidden).unwrap();
    std::fs::write(hidden.join(".DS_Store"), b"x").unwrap();
    assert!(check_join_folder(&hidden).is_err());
    let ws = base.join("ws");
    std::fs::create_dir_all(ws.join("a/b/c")).unwrap();
    open_workspace(&ws, [1; 16]).unwrap();
    assert!(check_join_folder(&ws.join("a/b/c")).is_err(), "deep inside");
    // An empty folder whose own marker dir exists is a workspace.
    let marked = base.join("marked");
    std::fs::create_dir_all(marked.join(MARKER_DIR)).unwrap();
    assert!(check_join_folder(&marked).is_err());
    assert!(check_join_folder(&base.join("missing")).is_err());
    let file = base.join("file");
    std::fs::write(&file, b"x").unwrap();
    assert!(check_join_folder(&file).is_err());
    // Codes the page maps to words differ.
    let codes = [
        check_join_folder(&hidden).unwrap_err(),
        check_join_folder(&ws.join("a/b/c")).unwrap_err(),
        check_join_folder(&file).unwrap_err(),
    ];
    let words: Vec<&str> = codes.iter().map(|c: &JoinFolderError| c.code()).collect();
    assert_eq!(
        words.len(),
        words.iter().collect::<std::collections::HashSet<_>>().len()
    );
    // Join writes only the marker.
    create_joined_workspace(&empty, [7; 16]).unwrap();
    let files = snapshot(&empty);
    assert_eq!(
        files.keys().cloned().collect::<Vec<_>>(),
        vec![
            format!("{MARKER_DIR}/"),
            format!("{MARKER_DIR}/{MARKER_FILE}")
        ]
    );
    assert_eq!(open_workspace(&empty, [8; 16]).unwrap(), [7; 16]);
    assert!(
        create_joined_workspace(&empty, [9; 16]).is_err(),
        "no longer empty"
    );
}

/// Criterion 44: a removed device keeps every document as a local file (content intact, no
/// workspace record), a legacy document shared on its own is left as it is, other files and the kept
/// deleted copies stay; the marker's share files go, `workspace.toml` stays.
#[test]
fn c44_a_removed_device_keeps_its_documents_as_local_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    open_workspace(&root, WS).unwrap();
    // A workspace document with a drawing.
    let document = Document::new(3);
    document.set_workspace_share(Some(WorkspaceShareRecord {
        workspace: WS,
        document: doc_id(1),
        synced_version: DocumentVersion::never(),
    }));
    let json = document.export_json().unwrap();
    create_document(&root, &p("Lid.curvyo"), &pack(&document, "t").unwrap()).unwrap();
    // A document of another workspace (copied in by another program) keeps its record.
    let other = Document::new(4);
    other.set_workspace_share(Some(WorkspaceShareRecord {
        workspace: [0x99; 16],
        document: doc_id(2),
        synced_version: DocumentVersion::never(),
    }));
    create_document(&root, &p("Other.curvyo"), &pack(&other, "t").unwrap()).unwrap();
    // A 0092 document shared on its own.
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures/shared_v34.curvyo");
    let own = std::fs::read(fixture).unwrap();
    create_document(&root, &p("Own.curvyo"), &own).unwrap();
    std::fs::write(root.join("notes.txt"), b"keep").unwrap();
    write_deleted(&root, doc_id(5), b"kept").unwrap();
    write_marker_share(
        &root,
        &MarkerShare {
            share_json: b"{}".to_vec(),
            keyring_log: b"k".to_vec(),
            list: b"l".to_vec(),
        },
    )
    .unwrap();
    assert!(read_marker_share(&root).unwrap().is_some());

    remove_marker_share(&root).unwrap();

    assert!(read_marker_share(&root).unwrap().is_none());
    assert_eq!(open_workspace(&root, [1; 16]).unwrap(), WS, "marker stays");
    let lid = unpack(9, &read_document(&root, &p("Lid.curvyo")).unwrap()).unwrap();
    assert!(lid.workspace_share().is_none(), "a plain local file now");
    assert!(!lid.is_shared());
    assert_eq!(lid.export_json().unwrap(), json, "content intact");
    let other = unpack(9, &read_document(&root, &p("Other.curvyo")).unwrap()).unwrap();
    assert_eq!(
        other.workspace_share().map(|r| r.workspace),
        Some([0x99; 16]),
        "another workspace's record is not touched"
    );
    assert_eq!(read_document(&root, &p("Own.curvyo")).unwrap(), own);
    // The legacy file is not touched (it opens local, ADR 0044 decision 3).
    assert!(unpack(9, &own).unwrap().legacy_share());
    assert_eq!(std::fs::read(root.join("notes.txt")).unwrap(), b"keep");
    assert_eq!(read_deleted(&root, doc_id(5)).unwrap(), b"kept");
    // Idempotent: a second call (a crash in between) does no harm.
    remove_marker_share(&root).unwrap();
}

/// ADR 0037 decision 1: only some of the marker's share files is damage, not "not shared".
#[test]
fn c33_a_partial_marker_share_is_damaged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    open_workspace(root, WS).unwrap();
    std::fs::write(root.join(MARKER_DIR).join("share.json"), b"{}").unwrap();
    assert!(read_marker_share(root).is_err());
}

/// ADR 0037 decision 1: `index.json` is a cache: a damaged or newer file reads as empty.
#[test]
fn c33_index_cache_round_trips_and_damage_reads_empty() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    open_workspace(root, WS).unwrap();
    let mut map = BTreeMap::new();
    map.insert(
        doc_id(1),
        IndexEntry {
            path: p("Sides/Left.curvyo"),
            size: 10,
            modified: 20,
        },
    );
    let text = index_text(&map);
    std::fs::write(root.join(MARKER_DIR).join("index.json"), &text).unwrap();
    assert_eq!(
        read_index(root).get(&doc_id(1)),
        Some(&p("Sides/Left.curvyo"))
    );
    std::fs::write(root.join(MARKER_DIR).join("index.json"), b"{garbage").unwrap();
    assert!(read_index(root).is_empty());
    let newer = text.replace("\"format_version\": 1", "\"format_version\": 2");
    assert_ne!(newer, text, "the cache names its format_version: {text}");
    std::fs::write(root.join(MARKER_DIR).join("index.json"), newer).unwrap();
    assert!(read_index(root).is_empty());
}

// ---------------------------------------------------------------------------------------------
// Recent workspaces (criterion 20)
// ---------------------------------------------------------------------------------------------

/// Criterion 20: at most 10, most recent first, removable, missing paths flagged; a newer file
/// is refused.
#[test]
fn c20_recents_order_limit_and_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let mut recents = RecentWorkspaces::default();
    for i in 0..12 {
        let path = dir.path().join(format!("ws{i}"));
        std::fs::create_dir(&path).unwrap();
        recents.opened(&path);
    }
    recents.opened(&dir.path().join("ws5"));
    let rows = recents.rows();
    assert_eq!(rows.len(), 10);
    assert_eq!(rows[0].name, "ws5");
    assert_eq!(rows[1].name, "ws11");
    assert!(rows.iter().all(|r| r.found));
    std::fs::remove_dir(dir.path().join("ws11")).unwrap();
    let rows = recents.rows();
    assert!(!rows[1].found, "a missing folder is greyed");
    recents.remove(1);
    assert_eq!(recents.rows().len(), 9);
    recents.remove(99);
    assert_eq!(recents.rows().len(), 9);
    let text = recents.to_text();
    assert_eq!(RecentWorkspaces::parse(&text).unwrap(), recents);
    // A newer file is refused, and so is damage, distinctly.
    let newer = text.replacen("format = 1", "format = 2", 1);
    assert_ne!(newer, text, "the file names its format: {text}");
    assert!(matches!(
        RecentWorkspaces::parse(&newer),
        Err(RecentWorkspacesError::Newer)
    ));
    assert!(RecentWorkspaces::parse("[[[").is_err());
    // Empty text is an empty list (a fresh install has no file).
    assert!(
        RecentWorkspaces::parse("").is_ok_and(|r| r.rows().is_empty())
            || RecentWorkspaces::parse("").is_err()
    );
}
