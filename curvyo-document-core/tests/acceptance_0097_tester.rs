//! Tester acceptance tests for `specs/0097-membership-lifecycle` on the file formats (criteria
//! 13, 16, 20, 21, 31 and 33; ADR 0041 decisions 3 and 6), written from the specification
//! before the implementation was read: the ended record leaks nothing beyond `share.json`, an
//! older shared file ends cleanly, a partial share set beside an ended record is not shared,
//! Sync again replaces the ended record, and older devices lists read no `left` mark.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Read, Write};

use curvyo_document_core::{
    DevicesList, Document, EndReason, EndedRecord, ShareRecord, pack, unpack,
};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn names(bytes: &[u8]) -> Vec<String> {
    zip::ZipArchive::new(Cursor::new(bytes))
        .unwrap()
        .file_names()
        .map(|name| name.unwrap().into_owned())
        .collect()
}

fn member(bytes: &[u8], name: &str) -> Option<Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = archive.by_name(name).ok()?;
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    Some(out)
}

fn with_member(bytes: &[u8], name: &str, with: Option<&[u8]>) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let member = file.name().unwrap().into_owned();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if member == name {
            continue;
        }
        writer.start_file(member, options).unwrap();
        writer.write_all(&data).unwrap();
    }
    if let Some(data) = with {
        writer.start_file(name, options).unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn record(document: &Document, ended: EndReason) -> EndedRecord {
    EndedRecord {
        server: "ws://192.168.1.20:8787".to_owned(),
        room: [0x2a; 16],
        server_seq: 17,
        synced_version: document.version(),
        ended,
        heads: vec![[0x0c; 32]],
    }
}

/// Criteria 20 and 31: `share.ended.json` holds exactly server, room, cursor, synced version,
/// reason and public keyring heads: no keys, no keyring, no label, no document name.
#[test]
fn the_ended_record_holds_nothing_beyond_the_way_back() {
    let document = Document::new(9);
    document.set_ended_share(Some(record(&document, EndReason::Removed)));
    let bytes = pack(&document, "Secret Plan Name").unwrap();
    let json: serde_json::Value =
        serde_json::from_slice(&member(&bytes, "share.ended.json").unwrap()).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "ended",
            "format_version",
            "heads",
            "room",
            "server",
            "server_seq",
            "synced_version"
        ]
    );
    let text = String::from_utf8(member(&bytes, "share.ended.json").unwrap()).unwrap();
    assert!(!text.contains("Secret Plan Name"));
    assert_eq!(json["ended"], "removed");
    assert_eq!(json["format_version"], 1);
    for left in [EndReason::Left, EndReason::Removed] {
        let r = record(&document, left);
        assert_eq!(EndedRecord::from_bytes(&r.to_bytes().unwrap()), Some(r));
    }
}

/// Criteria 13 and 20 on an older file: a `shared_v34.curvyo` whose share ends saves without
/// `share.json` and `keyring.log`, with the ended record, and reopens as a local file with the
/// way back and the same content.
#[test]
fn an_older_shared_file_ends_cleanly() {
    let bytes = fixture("shared_v34.curvyo");
    let document = unpack(1, &bytes).unwrap();
    assert!(document.is_shared());
    let content = document.export_json().unwrap();
    let share = document.share().unwrap();
    let ended = EndedRecord {
        server: share.server.clone(),
        room: share.room,
        server_seq: share.server_seq,
        synced_version: share.synced_version.clone(),
        ended: EndReason::Left,
        heads: vec![[1; 32]],
    };
    document.set_ended_share(Some(ended.clone()));
    let saved = pack(&document, "t").unwrap();
    let names = names(&saved);
    assert!(
        !names
            .iter()
            .any(|n| n == "share.json" || n == "keyring.log"),
        "{names:?}"
    );
    let opened = unpack(2, &saved).unwrap();
    assert!(!opened.is_shared());
    assert_eq!(opened.ended_share(), Some(ended));
    assert_eq!(opened.export_json().unwrap(), content);
}

/// ADR 0041 decision 6: an ended record beside a partial share set (`share.json` without
/// `keyring.log`) is not shared.
#[test]
fn an_ended_record_beside_a_partial_share_set_is_not_shared() {
    let shared = fixture("shared_v34.curvyo");
    let ended = member(&fixture("ended_v36.curvyo"), "share.ended.json").unwrap();
    let partial = with_member(&shared, "keyring.log", None);
    let partial = with_member(&partial, "share.ended.json", Some(&ended));
    let opened = unpack(2, &partial).unwrap();
    assert!(!opened.is_shared());
    assert_eq!(opened.share(), None);
}

/// Criterion 21: Sync again gives the file a share record again; once saved the file is shared
/// and the ended record does not come back with it.
#[test]
fn sync_again_replaces_the_ended_record() {
    let document = Document::new(4);
    document.set_ended_share(Some(record(&document, EndReason::Removed)));
    document.set_share(Some(ShareRecord {
        server: "ws://192.168.1.20:8787".to_owned(),
        room: [0x2a; 16],
        server_seq: 17,
        synced_version: document.version(),
        keyring_log: vec![9, 9],
    }));
    let opened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    assert!(opened.is_shared());
    assert!(opened.share().is_some());
    assert_eq!(
        opened.ended_share(),
        None,
        "a shared file carries no ended record"
    );
}

/// Criterion 16 with older devices lists: the 0096 fixtures read every workspace without a
/// `left` mark; leaving and switching on again round-trip through a snapshot.
#[test]
fn older_devices_lists_have_no_left_mark_and_the_mark_round_trips() {
    for name in ["devices_list_v1.loro", "devices_list_v2.loro"] {
        let Ok(list) = DevicesList::from_snapshot(3, &fixture(name)) else {
            continue;
        };
        assert!(list.workspaces().iter().all(|w| !w.left), "{name}");
    }
    let list = DevicesList::new(1);
    list.leave_workspace([0x31; 16], "Shop").unwrap();
    let copy = DevicesList::from_snapshot(2, &list.snapshot().unwrap()).unwrap();
    let w = &copy.workspaces()[0];
    assert!(w.left && !w.on);
    assert_eq!(w.name, "Shop");
    copy.switch_workspace([0x31; 16], "Shop", true).unwrap();
    list.import_updates(&copy.export_updates(&list.version()).unwrap())
        .unwrap();
    let w = &list.workspaces()[0];
    assert!(!w.left && w.on, "switching on clears the mark");
}
