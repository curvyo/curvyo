//! Tester's acceptance tests for `specs/0094-workspace` in `curvyo-editor-wasm`, written from the
//! specification and the public API before reading the implementation: Share workspace's
//! preparation (criterion 25), take-in of a document shared on its own and its refusal
//! (criteria 42, 49, WS-SEPARATE B), Duplicate (criterion 19) and the background writer
//! (criterion 32).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::similar_names
)]

use std::path::Path;

use curvyo_crypto_core::keyring::{Keyring, decode_log, encode_log};
use curvyo_crypto_core::{Identity, IdentityKeys};
use curvyo_document_core::{
    Document, DocumentVersion, EntryKind, Length, Point, RectBounds, WorkspaceShareRecord, pack,
    unpack,
};
use curvyo_editor_wasm::{
    BackgroundKind, BackgroundWriter, NotAdded, TakeIn, WRITE_INTERVAL_MS, Waiting,
    WorkspaceShareBuilder, take_in_check, workspace_copy_bytes,
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

/// The participants of a `0092` file's own keyring.
fn own_participants(bytes: &[u8]) -> Vec<Identity> {
    let record = unpack(1, bytes).unwrap().share().expect("a 0092 file");
    let keyring = Keyring::replay(&decode_log(&record.keyring_log).unwrap()).unwrap();
    keyring
        .participants()
        .into_iter()
        .map(|p| p.identity)
        .collect()
}

fn keys(n: u8) -> IdentityKeys {
    IdentityKeys::from_entropy(&[n; 64])
}

/// A `0092` file shared on its own by device `n`: its genesis keyring holds `n` alone.
fn shared_on_its_own(n: u8, rects: u32) -> Vec<u8> {
    let room = curvyo_sync_core::RoomId([0x5a; 16]);
    let session = curvyo_sync_core::ClientSession::new(
        curvyo_sync_core::SessionConfig {
            room,
            identity: keys(n),
            peer_id: u64::from(n),
            nonce_seed: [n; 32],
            entropy_seed: [n.wrapping_add(1); 32],
            since_seq: 0,
            start: curvyo_sync_core::SessionStart::Share,
        },
        curvyo_sync_core::Millis(1_800_000_000_000),
    )
    .unwrap();
    let doc = drawing(u64::from(n) << 8, rects);
    doc.set_share(Some(curvyo_document_core::ShareRecord {
        server: "ws://relay.test:8787".to_owned(),
        room: room.0,
        server_seq: 0,
        synced_version: doc.version(),
        keyring_log: encode_log(&session.keyring_entries()).unwrap(),
    }));
    pack(&doc, "test").unwrap()
}

fn stranger() -> Identity {
    IdentityKeys::from_entropy(&[0x77; 64]).identity()
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

// ---------------------------------------------------------------------------------------------
// Take-in (criteria 42, 49)
// ---------------------------------------------------------------------------------------------

/// Criteria 42 and 49: a `0092` document joins only when every participant of its own room is
/// a workspace participant; a plain document joins as is.
#[test]
fn c42_c49_take_in_needs_every_own_participant_in_the_workspace() {
    let shared = shared_on_its_own(1, 2);
    let own = own_participants(&shared);
    assert_eq!(own, vec![keys(1).identity()]);
    assert_eq!(take_in_check(&shared, &[]), TakeIn::Refused);
    assert_eq!(take_in_check(&shared, &[stranger()]), TakeIn::Refused);
    let mut more = own.clone();
    more.push(stranger());
    assert_eq!(
        take_in_check(&shared, &more),
        TakeIn::Allowed { people: own.len() }
    );
    if own.len() > 1 {
        assert_eq!(take_in_check(&shared, &own[..1]), TakeIn::Refused);
    }
    let plain = pack(&drawing(3, 1), "test").unwrap();
    assert_eq!(take_in_check(&plain, &[]), TakeIn::Plain);
    // Bytes that are no document are never taken in silently.
    assert_ne!(
        take_in_check(b"garbage", &own),
        TakeIn::Allowed { people: 0 }
    );
}

/// Criteria 25, 42 and 49 at Share: a document shared on its own with a stranger stays out
/// ("Not added"), one whose participants are all members is taken in with its history and loses
/// its keyring; every added file gets `share.json` v2 with its id.
#[test]
fn c25_c42_c49_share_takes_in_or_leaves_out_documents_shared_on_their_own() {
    let shared = shared_on_its_own(1, 2);
    let own = own_participants(&shared);
    let original = unpack(5, &shared).unwrap();

    // The sharing device is not in the document's own room: refused.
    let mut builder = WorkspaceShareBuilder::new(WS, 0xa1, 1_800_000_000, [4; 16]);
    builder.set_member(stranger());
    assert_eq!(
        builder.add_document(&p("Lid.curvyo"), &shared, id(1)),
        Err(NotAdded::SharedOnItsOwn)
    );
    assert_eq!(builder.document_count(), 0);
    assert_eq!(builder.not_added().len(), 1);
    assert!(builder.taken_in().is_empty());

    // The sharing device is its only participant: taken in.
    {
        let mut builder = WorkspaceShareBuilder::new(WS, 0xa1, 1_800_000_000, [4; 16]);
        builder.set_member(own[0]);
        builder
            .add_document(&p("Sides/Lid.curvyo"), &shared, id(1))
            .unwrap();
        assert_eq!(builder.taken_in(), &[(p("Sides/Lid.curvyo"), id(1))]);
        let (prepared, files) = builder.finish([6; 16]);
        assert_eq!(files.len(), 1);
        let file = unpack(7, &files[0].1).unwrap();
        assert!(file.share().is_none(), "the old room record and keyring go");
        let record = file.workspace_share().unwrap();
        assert_eq!((record.workspace, record.document), (WS, id(1)));
        assert_eq!(
            file.export_json().unwrap(),
            original.export_json().unwrap(),
            "same content"
        );
        // Same history: the original's updates import into the taken-in document as nothing new.
        let all = original.export_updates(&DocumentVersion::never()).unwrap();
        let before = file.version();
        file.import_updates(&all).unwrap();
        assert_eq!(
            file.version(),
            before,
            "later merges of the old history work"
        );
        let streams = assemble(&prepared.parts);
        assert!(streams.iter().any(|s| s.stream == id(1)));
    }
}

/// Criterion 25: every readable document gets its id and `share.json` v2, empty folders are
/// shared, unreadable files and names the list refuses stay out with their reason; the
/// snapshot holds the list first, then every document.
#[test]
fn c25_share_prepares_every_document_and_the_list() {
    let mut builder = WorkspaceShareBuilder::new(WS, 0xa1, 1_800_000_000, [4; 16]);
    builder.set_member(stranger());
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
    builder.set_member(stranger());
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
    assert!(opened.share().is_none());
    assert!(opened.workspace_share().is_none());
    assert_eq!(
        opened.export_json().unwrap(),
        unpack(4, &shared).unwrap().export_json().unwrap()
    );
    let ws = fixture("workspace_shared_v35.curvyo");
    let copy = unpack(5, &workspace_copy_bytes(&ws, "test").unwrap()).unwrap();
    assert!(copy.workspace_share().is_none() && copy.share().is_none());
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
