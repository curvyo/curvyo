//! Tester acceptance tests for `specs/0097-membership-lifecycle` on the file formats (criteria
//! 13, 16, 20, 21, 31 and 33; ADR 0041 decisions 3 and 6), written from the specification
//! before the implementation was read: the ended record leaks nothing beyond `share.json`, an
//! older shared file opens local and saves cleanly, a partial share set beside an ended record is
//! not shared, a workspace record replaces a legacy ended record, and older devices lists read no
//! `left` mark. Ported to ADR 0044 decision 3 (containers of documents shared on their own are
//! legacy inputs).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Read, Write};

use curvyo_document_core::{
    DevicesList, Document, EndReason, EndedRecord, WorkspaceShareRecord, pack, unpack,
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
    let marker = record(&document, EndReason::Removed).to_bytes().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&marker).unwrap();
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
    let text = String::from_utf8(marker).unwrap();
    assert!(!text.contains("Secret Plan Name"));
    assert_eq!(json["ended"], "removed");
    assert_eq!(json["format_version"], 1);
    for left in [EndReason::Left, EndReason::Removed] {
        let r = record(&document, left);
        assert_eq!(EndedRecord::from_bytes(&r.to_bytes().unwrap()), Some(r));
    }
}

/// Criteria 13 and 20 on an older file, since ADR 0044 decision 3: a `shared_v34.curvyo` opens
/// as a local file and saves without `share.json` and `keyring.log`, with the same content.
#[test]
fn an_older_shared_file_opens_local_and_saves_cleanly() {
    let bytes = fixture("shared_v34.curvyo");
    let document = unpack(1, &bytes).unwrap();
    assert!(!document.is_shared());
    assert!(document.legacy_share());
    let content = document.export_json().unwrap();
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
    assert!(!opened.legacy_share());
    assert_eq!(opened.export_json().unwrap(), content);
}

/// An ended record beside a partial share set (`share.json` without `keyring.log`) is not
/// shared.
#[test]
fn an_ended_record_beside_a_partial_share_set_is_not_shared() {
    let shared = fixture("shared_v34.curvyo");
    let ended = member(&fixture("ended_v36.curvyo"), "share.ended.json").unwrap();
    let partial = with_member(&shared, "keyring.log", None);
    let partial = with_member(&partial, "share.ended.json", Some(&ended));
    let opened = unpack(2, &partial).unwrap();
    assert!(!opened.is_shared());
    assert_eq!(opened.workspace_share(), None);
}

/// A legacy ended file given a workspace record (Share or Add to workspace, ADR 0044 decision
/// 3) is a workspace document once saved, and the ended record does not come back with it.
#[test]
fn a_workspace_record_replaces_a_legacy_ended_record() {
    let document = unpack(4, &fixture("ended_v36.curvyo")).unwrap();
    document.set_workspace_share(Some(WorkspaceShareRecord {
        workspace: [0x2a; 16],
        document: [0x2b; 16],
        synced_version: document.version(),
    }));
    let saved = pack(&document, "t").unwrap();
    assert!(!names(&saved).iter().any(|n| n == "share.ended.json"));
    let opened = unpack(2, &saved).unwrap();
    assert!(opened.is_shared());
    assert!(opened.workspace_share().is_some());
    assert!(!opened.legacy_share());
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
