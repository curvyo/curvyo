//! `share.json` version 2 of a workspace document (`specs/0095-workspace` criteria 15 and 36,
//! ADR 0037 decision 3): the workspace and the document id, no relay and no keyring; version 1
//! of a document shared on its own is still read.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Read, Write};

use curvyo_document_core::{
    Document, Length, OpenError, Point, RectBounds, ShareRecord, WorkspaceRoomRecord,
    WorkspaceShareRecord, pack, pack_snapshot, unpack,
};

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn drawn() -> Document {
    let document = Document::new(34);
    let _ = document.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    });
    document
}

fn record(document: &Document) -> WorkspaceShareRecord {
    WorkspaceShareRecord {
        workspace: [0x11; 16],
        document: [0xd0; 16],
        synced_version: document.version(),
    }
}

fn member(bytes: &[u8], name: &str) -> Option<Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = archive.by_name(name).ok()?;
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    Some(out)
}

/// Rewrites one member of a container.
fn with_share_json(bytes: &[u8], json: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let name = file.name().unwrap().into_owned();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        writer.start_file(name.clone(), options).unwrap();
        writer
            .write_all(if name == "share.json" { json } else { &data })
            .unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// Criterion 15: the document id travels with the file, through save and open, and a workspace
/// document is shared (its history is not wiped).
#[test]
fn a_workspace_document_round_trips_its_record() {
    let document = drawn();
    document.set_workspace_share(Some(record(&document)));
    assert!(document.is_shared());
    assert_eq!(document.share(), None);
    let bytes = pack(&document, "t").unwrap();
    assert!(member(&bytes, "keyring.log").is_none());
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.workspace_share(), Some(record(&document)));
    assert_eq!(opened.share(), None);
    assert!(opened.is_shared());
}

/// A file holds one record: setting one replaces the other, clearing leaves a local file.
#[test]
fn a_file_holds_one_share_record_at_most() {
    let document = drawn();
    document.set_share(Some(ShareRecord {
        server: "ws://x".to_owned(),
        room: [7; 16],
        server_seq: 1,
        synced_version: document.version(),
        keyring_log: vec![1],
    }));
    document.set_workspace_share(Some(record(&document)));
    assert_eq!(document.share(), None);
    let opened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    assert_eq!(opened.workspace_share(), Some(record(&document)));
    opened.set_workspace_share(None);
    assert!(!opened.is_shared());
    let plain = unpack(3, &pack(&opened, "t").unwrap()).unwrap();
    assert_eq!(plain.workspace_share(), None);
    assert!(member(&pack(&opened, "t").unwrap(), "share.json").is_none());
}

/// A document's bytes inside a workspace snapshot carry no record (ADR 0037 decision 7).
#[test]
fn a_snapshot_carries_no_workspace_record() {
    let document = drawn();
    document.set_workspace_share(Some(record(&document)));
    let bytes = pack_snapshot(&document, "t").unwrap();
    assert!(member(&bytes, "share.json").is_none());
    assert_eq!(unpack(2, &bytes).unwrap().workspace_share(), None);
}

/// The golden file of version 2; regenerate with `UPDATE_0095_FIXTURES=1`.
#[test]
fn golden_workspace_shared_v35() {
    let path = fixture_path("workspace_shared_v35.curvyo");
    if std::env::var_os("UPDATE_0095_FIXTURES").is_some() {
        let document = drawn();
        document.set_workspace_share(Some(record(&document)));
        std::fs::write(&path, pack(&document, "golden").unwrap()).unwrap();
    }
    let bytes = std::fs::read(&path).unwrap();
    let opened = unpack(2, &bytes).unwrap();
    let share = opened.workspace_share().expect("workspace document");
    assert_eq!(share.workspace, [0x11; 16]);
    assert_eq!(share.document, [0xd0; 16]);
    assert_eq!(share.synced_version, opened.version());
    let json: serde_json::Value =
        serde_json::from_slice(&member(&bytes, "share.json").unwrap()).unwrap();
    assert_eq!(json["format_version"], 2);
    assert_eq!(json["workspace"], "11111111111111111111111111111111");
    assert_eq!(json["document"], "d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0");
    assert_eq!(
        json.as_object().unwrap().len(),
        4,
        "no server, room, cursor or keyring: {json}"
    );
}

/// A malformed version 2 record is damage; a newer one is refused by its version.
#[test]
fn a_malformed_or_newer_version_2_record_is_refused() {
    let bytes = std::fs::read(fixture_path("workspace_shared_v35.curvyo")).unwrap();
    for bad in [
        &br#"{"format_version":2,"workspace":"1111","document":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","synced_version":""}"#[..],
        br#"{"format_version":2,"workspace":"11111111111111111111111111111111","synced_version":""}"#,
        br#"{"format_version":2,"workspace":"11111111111111111111111111111111","document":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","synced_version":"nope"}"#,
    ] {
        let damaged = with_share_json(&bytes, bad);
        assert!(
            matches!(unpack(2, &damaged), Err(OpenError::Damaged)),
            "{}",
            String::from_utf8_lossy(bad)
        );
    }
    let newer = with_share_json(&bytes, br#"{"format_version":3}"#);
    assert!(matches!(
        unpack(2, &newer),
        Err(OpenError::FormatTooNew {
            found: 3,
            supported: 2
        })
    ));
}

/// A version 2 record beside a keyring is damage: a workspace document has no keyring of its own.
#[test]
fn a_version_2_record_with_a_keyring_is_damaged() {
    let document = drawn();
    document.set_share(Some(ShareRecord {
        server: "ws://x".to_owned(),
        room: [7; 16],
        server_seq: 1,
        synced_version: document.version(),
        keyring_log: vec![1],
    }));
    let bytes = with_share_json(
        &pack(&document, "t").unwrap(),
        br#"{"format_version":2,"workspace":"11111111111111111111111111111111","document":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","synced_version":""}"#,
    );
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
}

/// ADR 0037 decision 1 (criteria 25, 33): the marker's `share.json` keeps the relay, the
/// workspace id as the room, the workspace cursor and the list's acknowledged version in the
/// version 1 shape; golden file `workspace_marker_share_v1.json`, regenerate with
/// `UPDATE_0095_FIXTURES=1`.
#[test]
fn golden_workspace_marker_share() {
    let list = curvyo_document_core::WorkspaceList::new(7);
    let record = WorkspaceRoomRecord {
        server: "ws://192.168.1.20:8787".to_owned(),
        workspace: [0x57; 16],
        server_seq: 42,
        synced_version: list.version(),
    };
    let path = fixture_path("workspace_marker_share_v1.json");
    if std::env::var_os("UPDATE_0095_FIXTURES").is_some() {
        std::fs::write(&path, record.to_bytes().unwrap()).unwrap();
    }
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(WorkspaceRoomRecord::from_bytes(&bytes).unwrap(), record);
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["format_version"], 1);
    assert_eq!(json["room"], "57575757575757575757575757575757");
    assert_eq!(json["server_seq"], 42);
}

/// A newer marker record is refused by its version, a broken one is damage.
#[test]
fn a_newer_or_broken_marker_share_is_refused() {
    let newer = br#"{"format_version":2,"server":"ws://a","room":"57575757575757575757575757575757","server_seq":1,"synced_version":""}"#;
    assert!(matches!(
        WorkspaceRoomRecord::from_bytes(newer),
        Err(OpenError::FormatTooNew { found: 2, .. })
    ));
    assert_eq!(
        WorkspaceRoomRecord::from_bytes(b"{}"),
        Err(OpenError::Damaged)
    );
    let short_room =
        br#"{"format_version":1,"server":"ws://a","room":"57","server_seq":1,"synced_version":""}"#;
    assert_eq!(
        WorkspaceRoomRecord::from_bytes(short_room),
        Err(OpenError::Damaged)
    );
}

/// Criterion 41: a host's scan reads the document id from the `share.json` member alone; a
/// version 1 record (shared on its own) or damage is no workspace document.
#[test]
fn a_scan_reads_the_document_id_from_share_json_alone() {
    let document = drawn();
    document.set_workspace_share(Some(record(&document)));
    let bytes = pack(&document, "test").unwrap();
    let json = member(&bytes, "share.json").unwrap();
    assert_eq!(
        WorkspaceShareRecord::from_share_json(&json),
        Some(record(&document))
    );
    assert_eq!(WorkspaceShareRecord::from_share_json(b"{"), None);
    let v1 = br#"{"format_version":1,"server":"s","room":"00000000000000000000000000000000","server_seq":0,"synced_version":""}"#;
    assert_eq!(WorkspaceShareRecord::from_share_json(v1), None);
}

/// Criterion 35: a document another member created arrives as the updates of its whole stream;
/// they open as that document.
#[test]
fn a_document_builds_from_the_updates_of_its_whole_stream() {
    let original = drawn();
    let updates = original
        .export_updates(&curvyo_document_core::DocumentVersion::never())
        .unwrap();
    let built = Document::from_updates(77, &updates).unwrap();
    assert_eq!(built.read_objects().objects.len(), 1);
    assert_eq!(built.version(), original.version());
    assert!(Document::from_updates(77, b"not updates").is_err());
    let before = original.version();
    let _ = original.create_rect(RectBounds {
        origin: Point::new(20.0, 0.0),
        width: Length::from_mm(1.0),
        height: Length::from_mm(1.0),
    });
    let later = original.export_updates(&before).unwrap();
    assert!(
        Document::from_updates(77, &later).is_err(),
        "updates without the stream's start make no document"
    );
}
