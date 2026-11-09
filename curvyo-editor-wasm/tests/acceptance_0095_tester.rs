//! Tester's acceptance tests for `specs/0095-workspace` in `curvyo-editor-wasm`, written from the
//! specification and the public API before reading the implementation: Share workspace's
//! preparation (criterion 25; take-in and WS-SEPARATE B retired by ADR 0044: a legacy file is
//! shared like any other), Duplicate (criterion 19) and the background writer (criterion 32).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::similar_names
)]

use std::path::Path;

use curvyo_document_core::{
    Document, EntryKind, Length, Point, RectBounds, WorkspaceShareRecord, pack, unpack,
};
use curvyo_editor_wasm::{
    BackgroundKind, BackgroundWriter, NotAdded, WRITE_INTERVAL_MS, Waiting, WorkspaceShareBuilder,
    workspace_copy_bytes,
};
use curvyo_sync_core::{SnapshotAssembler, StreamEntry};

const WS: [u8; 16] = [0x57; 16];

fn p(text: &str) -> Vec<String> {
    text.split('/').map(str::to_owned).collect()
}

fn id(n: u8) -> [u8; 16] {
    let mut id = [0xd0; 16];
    id[15] = n;
    id
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn drawing(peer: u64, rects: u32) -> Document {
    let doc = Document::new(peer);
    for i in 0..rects {
        let _ = doc.create_rect(RectBounds {
            origin: Point::new(f64::from(i) * 10.0, 0.0),
            width: Length::from_mm(5.0),
            height: Length::from_mm(5.0),
        });
    }
    doc
}

fn assemble(parts: &[Vec<u8>]) -> Vec<StreamEntry> {
    let mut a = SnapshotAssembler::default();
    let mut out = Vec::new();
    for part in parts {
        out.extend(a.push_part(part).unwrap());
    }
    assert!(a.is_complete());
    out
}

/// ADR 0044 decision 3: a file shared on its own (`0092`, golden `shared_v34.curvyo`) lying in
/// the workspace folder is shared like any other: it gets `share.json` v2 with its id, keeps its
/// content, and its old members go.
#[test]
fn c25_a_legacy_shared_file_is_shared_like_any_other() {
    let legacy = fixture("shared_v34.curvyo");
    let original = unpack(5, &legacy).unwrap();
    let mut builder = WorkspaceShareBuilder::new(WS, 0xa1, 1_800_000_000, [4; 16]);
    builder
        .add_document(&p("Sides/Lid.curvyo"), &legacy, id(1))
        .unwrap();
    let (prepared, files) = builder.finish([6; 16]);
    assert_eq!(files.len(), 1);
    let names: Vec<String> = zip::ZipArchive::new(std::io::Cursor::new(&files[0].1[..]))
        .unwrap()
        .file_names()
        .map(|n| n.unwrap().into_owned())
        .collect();
    assert!(!names.iter().any(|n| n == "keyring.log"), "{names:?}");
    let file = unpack(7, &files[0].1).unwrap();
    let record = file.workspace_share().unwrap();
    assert_eq!((record.workspace, record.document), (WS, id(1)));
    assert_eq!(
        file.export_json().unwrap(),
        original.export_json().unwrap(),
        "same content"
    );
    assert!(assemble(&prepared.parts).iter().any(|s| s.stream == id(1)));
}

/// Criterion 25: every readable document gets its id and `share.json` v2, empty folders are
/// shared, unreadable files and names the list refuses stay out with their reason; the
/// snapshot holds the list first, then every document.
#[test]
fn c25_share_prepares_every_document_and_the_list() {
    let mut builder = WorkspaceShareBuilder::new(WS, 0xa1, 1_800_000_000, [4; 16]);
    let lid = pack(&drawing(2, 2), "test").unwrap();
    let left = pack(&drawing(3, 1), "test").unwrap();
    builder.add_document(&p("Lid.curvyo"), &lid, id(1)).unwrap();
    builder
        .add_document(&p("Sides/Left.curvyo"), &left, id(2))
        .unwrap();
    assert!(builder.add_folder(&p("Empty")));
    assert_eq!(
        builder.add_document(&p("Bad.curvyo"), b"not a container", id(3)),
        Err(NotAdded::Unreadable)
    );
    assert_eq!(
        builder.add_document(&p("CON.curvyo"), &left, id(4)),
        Err(NotAdded::Name)
    );
    assert_eq!(builder.document_count(), 2);
    let (prepared, files) = builder.finish([6; 16]);
    assert_eq!(prepared.workspace, WS);
    let mut paths: Vec<_> = files.iter().map(|(path, _)| path.clone()).collect();
    paths.sort();
    assert_eq!(paths, vec![p("Lid.curvyo"), p("Sides/Left.curvyo")]);
    for (path, bytes) in &files {
        let record = unpack(9, bytes).unwrap().workspace_share().unwrap();
        assert_eq!(record.workspace, WS);
        assert_eq!(
            record.document,
            if path == &p("Lid.curvyo") {
                id(1)
            } else {
                id(2)
            }
        );
    }
    let entries = prepared.list.entries();
    assert!(
        entries
            .iter()
            .any(|e| e.path == p("Empty") && e.kind == EntryKind::Folder)
    );
    assert!(
        entries
            .iter()
            .any(|e| e.path == p("Sides/Left") && e.document == Some(id(2)))
    );
    let streams = assemble(&prepared.parts);
    assert_eq!(streams[0].stream, WS, "the list first");
    let ids: Vec<_> = streams[1..].iter().map(|s| s.stream).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&id(1)) && ids.contains(&id(2)));
}

/// Criterion 25 with names that differ only in case (possible on Linux): the list's shown name
/// and the file must agree, or the first sync renames the maker's file.
#[test]
fn c25_files_differing_in_case_keep_their_names_or_stay_out() {
    let mut builder = WorkspaceShareBuilder::new(WS, 0xa1, 1_800_000_000, [4; 16]);
    let a = pack(&drawing(2, 1), "test").unwrap();
    let b = pack(&drawing(3, 1), "test").unwrap();
    let ra = builder.add_document(&p("Lid.curvyo"), &a, id(1));
    let rb = builder.add_document(&p("lid.curvyo"), &b, id(2));
    let (prepared, files) = builder.finish([6; 16]);
    let shown: Vec<Vec<String>> = prepared
        .list
        .entries()
        .into_iter()
        .filter(|e| e.kind == EntryKind::Document)
        .map(|e| {
            let mut path = e.path;
            let last = path.pop().unwrap();
            path.push(format!("{last}.curvyo"));
            path
        })
        .collect();
    for (path, _) in &files {
        assert!(
            shown.contains(path),
            "file {path:?} is listed as {shown:?} (results {ra:?} {rb:?})"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Duplicate (criterion 19)
// ---------------------------------------------------------------------------------------------

/// Criterion 19: a copy has the same drawing and no membership: no room record, no keyring and
/// no workspace record.
#[test]
fn c19_a_copy_is_no_member_of_the_original() {
    let shared = fixture("shared_v34.curvyo");
    let copy = workspace_copy_bytes(&shared, "test").unwrap();
    let opened = unpack(3, &copy).unwrap();
    assert!(!opened.legacy_share());
    assert!(opened.workspace_share().is_none());
    assert_eq!(
        opened.export_json().unwrap(),
        unpack(4, &shared).unwrap().export_json().unwrap()
    );
    let ws = fixture("workspace_shared_v35.curvyo");
    let copy = unpack(5, &workspace_copy_bytes(&ws, "test").unwrap()).unwrap();
    assert!(copy.workspace_share().is_none() && !copy.is_shared());
    assert!(workspace_copy_bytes(b"garbage", "test").is_err());
}

// ---------------------------------------------------------------------------------------------
// Background writer (criterion 32)
// ---------------------------------------------------------------------------------------------

fn ws_file(doc: &Document) -> Vec<u8> {
    doc.set_workspace_share(Some(WorkspaceShareRecord {
        workspace: WS,
        document: id(1),
        synced_version: doc.version(),
    }));
    pack(doc, "test").unwrap()
}

/// Criterion 32: records wait per document and are written at most every 5 s; a duplicate or
/// replayed record changes nothing; the file keeps its workspace record; a file that does not
/// open keeps the records waiting.
#[test]
fn c32_background_writes_are_batched_idempotent_and_keep_the_record() {
    let remote = drawing(0x51, 0);
    let file = ws_file(&drawing(0x51, 0));
    // The remote edit: B's replica, as an update from the shared start.
    let base = unpack(0x52, &file).unwrap();
    let v = base.version();
    let _ = base.create_rect(RectBounds {
        origin: Point::new(1.0, 1.0),
        width: Length::from_mm(3.0),
        height: Length::from_mm(3.0),
    });
    let update = base.export_updates(&v).unwrap();
    drop(remote);
    let mut writer = BackgroundWriter::new(0x99);
    assert!(writer.is_idle());
    for seq in [5, 5, 6] {
        writer.add(
            id(1),
            Waiting {
                seq,
                kind: BackgroundKind::Update,
                bytes: update.clone(),
            },
        );
    }
    let t0 = 1_000_000;
    assert_eq!(writer.due(t0), vec![id(1)]);
    assert!(writer.due(t0).is_empty(), "asked once until answered");
    // Unreadable bytes: nothing written, records keep waiting.
    assert!(writer.write(id(1), b"garbage", t0).is_err());
    assert!(!writer.is_idle());
    let (bytes, through) = writer.write(id(1), &file, t0).unwrap();
    assert_eq!(through, 6);
    let written = unpack(1, &bytes).unwrap();
    assert_eq!(
        written.export_json().unwrap(),
        base.export_json().unwrap(),
        "the edit is in the file once"
    );
    let record = written.workspace_share().unwrap();
    assert_eq!((record.workspace, record.document), (WS, id(1)));
    assert!(writer.is_idle());
    // The next record waits for the interval.
    writer.add(
        id(1),
        Waiting {
            seq: 7,
            kind: BackgroundKind::Update,
            bytes: update.clone(),
        },
    );
    assert!(writer.due(t0 + WRITE_INTERVAL_MS - 1).is_empty());
    assert_eq!(writer.due(t0 + WRITE_INTERVAL_MS), vec![id(1)]);
    let (again, through) = writer.write(id(1), &bytes, t0 + WRITE_INTERVAL_MS).unwrap();
    assert_eq!(through, 7);
    assert_eq!(
        unpack(2, &again).unwrap().export_json().unwrap(),
        base.export_json().unwrap(),
        "a replayed record adds nothing"
    );
    // Forgetting drops what waits.
    writer.add(
        id(2),
        Waiting {
            seq: 9,
            kind: BackgroundKind::Update,
            bytes: update,
        },
    );
    assert_eq!(writer.forget(id(2)), Some(9));
    assert!(writer.is_idle());
}
