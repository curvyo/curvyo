//! Tests of the workspace channel's routing, join writes and marker cursor (`specs/0095-workspace`
//! criteria 28, 30, 32).

use curvyo_document_core::Created;
use curvyo_sync_core::{encode_inner_record, encode_snapshot_stream, split_snapshot};

use super::*;

const WORKSPACE: [u8; 16] = [0x57; 16];
const LID: [u8; 16] = [0x11; 16];
const SIDE: [u8; 16] = [0x22; 16];

fn created(n: u8) -> Created {
    Created {
        at: 1_800_000_000,
        id: [n; 16],
    }
}

fn list() -> WorkspaceList {
    let list = WorkspaceList::new(9);
    let sides = list.create_folder(None, "Sides", created(1)).unwrap();
    list.create_document(None, "Lid", LID, created(2)).unwrap();
    list.create_document(Some(sides), "Left", SIDE, created(3))
        .unwrap();
    list
}

fn parts(list: &WorkspaceList) -> Vec<Vec<u8>> {
    let stream = encode_snapshot_stream(&[
        StreamEntry {
            stream: WORKSPACE,
            bytes: list.snapshot().unwrap(),
        },
        StreamEntry {
            stream: LID,
            bytes: b"lid".to_vec(),
        },
        StreamEntry {
            stream: SIDE,
            bytes: b"side".to_vec(),
        },
    ]);
    split_snapshot(&stream, [1; 16])
}

fn joining() -> WorkspaceChannel {
    let mut channel = WorkspaceChannel::new(WORKSPACE, "Box job", WorkspaceList::new(5), 5);
    channel.joining = true;
    channel
}

/// Criterion 28: a join's snapshot gives the list, then the folders, then each document
/// as a file at its place, and the count of documents received.
#[test]
fn a_joined_snapshot_becomes_folders_then_documents() {
    let mut channel = joining();
    let list = list();
    for part in parts(&list) {
        assert_eq!(
            channel.route(4, RecordKind::Snapshot, &part),
            Routed::Done(true)
        );
    }
    assert!(!channel.joining);
    let writes: Vec<WorkspaceWrite> = channel.writes.drain(..).collect();
    assert_eq!(
        writes[0],
        WorkspaceWrite::Folder {
            path: vec!["Sides".to_owned()]
        }
    );
    assert!(matches!(
        &writes[1],
        WorkspaceWrite::Document { path, document: LID, seq: 4, bytes }
            if path == &["Lid.curvyo"] && bytes == b"lid"
    ));
    assert!(matches!(
        &writes[2],
        WorkspaceWrite::Document { path, document: SIDE, .. }
            if path == &["Sides", "Left.curvyo"]
    ));
    assert_eq!(channel.receiving(), Some((0, 2)));
    assert_eq!(channel.persisted_cursor(4), 3, "files not written yet");
    channel.document_written(LID);
    channel.document_written(SIDE);
    assert_eq!(channel.receiving(), None);
    assert_eq!(channel.persisted_cursor(4), 4);
}

/// Criteria 30 and 32: a live record goes to the open document, the list or the background
/// queue by its inner stream; the marker cursor waits for every file.
#[test]
fn live_records_are_routed_by_their_stream() {
    let mut channel = WorkspaceChannel::new(WORKSPACE, "Box job", list(), 5);
    channel.stream = Some(LID);
    let to_open = encode_inner_record(LID, 7, b"lid update");
    assert_eq!(
        channel.route(5, RecordKind::Update, &to_open),
        Routed::Open(RecordKind::Update, b"lid update".to_vec())
    );
    let to_side = encode_inner_record(SIDE, 7, b"side update");
    assert_eq!(
        channel.route(6, RecordKind::Update, &to_side),
        Routed::Done(true)
    );
    let other = WorkspaceList::from_snapshot(8, &channel.list.snapshot().unwrap()).unwrap();
    let since = other.version();
    other.create_folder(None, "Back", created(4)).unwrap();
    let to_list = encode_inner_record(WORKSPACE, 8, &other.export_updates(&since).unwrap());
    assert_eq!(
        channel.route(7, RecordKind::Update, &to_list),
        Routed::Done(true)
    );
    assert_eq!(channel.list.entries().len(), 4);
    assert_eq!(
        channel.persisted_cursor(7),
        4,
        "the open document is unsaved"
    );
    channel.open_saved(5);
    assert_eq!(channel.persisted_cursor(7), 5, "Side's record waits");
    let handed = channel.take_background();
    assert_eq!(handed.len(), 1);
    assert_eq!(handed[0].document, SIDE);
    assert_eq!(channel.persisted_cursor(7), 5);
    channel.background_written(SIDE, 6);
    assert_eq!(channel.persisted_cursor(7), 7);
    assert_eq!(
        channel.route(8, RecordKind::Update, b"short"),
        Routed::Done(false)
    );
}

/// ADR 0037 decision 8: leaving a document with records not in its file puts them back in
/// the background queue; opening one gives back its records still waiting.
#[test]
fn switching_documents_moves_their_waiting_records() {
    let mut channel = WorkspaceChannel::new(WORKSPACE, "Box job", list(), 5);
    channel.stream = Some(LID);
    channel.route(5, RecordKind::Update, &encode_inner_record(LID, 7, b"a"));
    channel.route(6, RecordKind::Update, &encode_inner_record(SIDE, 7, b"b"));
    let handed = channel.take_background();
    assert_eq!(handed.len(), 1);
    let reopened = channel.switch_stream(Some(SIDE));
    assert_eq!(reopened.len(), 1);
    assert_eq!(reopened[0].bytes, b"b");
    assert_eq!(channel.background.len(), 1);
    assert_eq!(channel.background[0].document, LID);
    assert_eq!(channel.persisted_cursor(9), 4);
    channel.background_written(SIDE, 6);
    assert_eq!(
        channel.unsaved.len(),
        1,
        "the open one still waits for its save"
    );
}
