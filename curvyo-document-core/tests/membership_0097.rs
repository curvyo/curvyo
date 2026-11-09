//! The formats of the membership lifecycle (`specs/0097-membership-lifecycle` criteria 13, 16,
//! 20 and 33; ADR 0041 decisions 3 and 6): the ended record `share.ended.json` version 1 as a
//! marker file, the container member of `0097` builds read as a legacy local file (ADR 0044
//! decision 3), and the devices list's optional `left` mark. The marker goldens regenerate with
//! `UPDATE_0097_FIXTURES=1`; `ended_v36.curvyo` is a legacy input.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Read, Write};

use curvyo_document_core::{
    DevicesList, Document, EndReason, EndedRecord, Length, Point, RectBounds, pack, unpack,
};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn updating() -> bool {
    std::env::var_os("UPDATE_0097_FIXTURES").is_some()
}

fn document() -> Document {
    let document = Document::new(97);
    let _ = document.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    });
    document
}

fn ended(document: &Document, ended: EndReason) -> EndedRecord {
    EndedRecord {
        server: "ws://192.168.1.20:8787".to_owned(),
        room: [7; 16],
        server_seq: 42,
        synced_version: document.version(),
        ended,
        heads: vec![[0x41; 32], [0x42; 32]],
    }
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

/// Rewrites one member of a container (or drops it when `with` is `None`).
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

/// The golden `share_ended_v1.json` (marker file); `specs/0105` criteria 4 and 5: the legacy
/// container `ended_v36.curvyo` opens as an editable local document with the legacy flag, and
/// saved it holds no ended record and opens without the flag.
#[test]
fn golden_share_ended_v1() {
    let json_path = fixture("share_ended_v1.json");
    if updating() {
        let record = ended(&document(), EndReason::Removed);
        std::fs::write(&json_path, record.to_bytes().unwrap()).unwrap();
    }
    let json = std::fs::read(&json_path).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(value["format_version"], 1);
    assert_eq!(value["server"], "ws://192.168.1.20:8787");
    assert_eq!(value["room"], "07070707070707070707070707070707");
    assert_eq!(value["server_seq"], 42);
    assert_eq!(value["ended"], "removed");
    assert_eq!(value["heads"][0], "41".repeat(32));
    assert_eq!(value["heads"][1], "42".repeat(32));
    let record = EndedRecord::from_bytes(&json).unwrap();
    assert_eq!(record.ended, EndReason::Removed);
    assert_eq!(record.room, [7; 16]);
    assert_eq!(record.heads, vec![[0x41; 32], [0x42; 32]]);

    let bytes = std::fs::read(fixture("ended_v36.curvyo")).unwrap();
    assert!(
        member(&bytes, "share.ended.json").is_some(),
        "a legacy input"
    );
    let opened = unpack(2, &bytes).unwrap();
    assert!(opened.legacy_share());
    assert!(!opened.is_shared());
    let saved = pack(&opened, "t").unwrap();
    assert!(!names(&saved).iter().any(|n| n.starts_with("share")));
    assert!(!unpack(3, &saved).unwrap().legacy_share());
}

/// Criterion 33 and ADR 0041 decision 6: `manifest.json` lists no members (its keys are
/// unchanged), so this build opens a container with any unknown member like a plain one.
#[test]
fn an_unknown_member_opens_the_file_as_local() {
    let bytes = std::fs::read(fixture("ended_v36.curvyo")).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&member(&bytes, "manifest.json").unwrap()).unwrap();
    let mut keys: Vec<&str> = manifest
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["app_version", "format_version", "loro_snapshot_version"]
    );
    let ended = member(&bytes, "share.ended.json").unwrap();
    let unknown = with_member(&bytes, "share.ended.json", None);
    let unknown = with_member(&unknown, "something.new.json", Some(&ended));
    let opened = unpack(2, &unknown).unwrap();
    assert!(!opened.legacy_share());
    assert!(!opened.is_shared());
    assert_eq!(
        opened.export_json().unwrap(),
        unpack(3, &bytes).unwrap().export_json().unwrap()
    );
}

/// Damaged variants of the marker's record are no way back (Proposal); in a container any of
/// them is a legacy local file, never an open error.
#[test]
fn a_damaged_or_newer_ended_record_is_no_way_back() {
    let newer_path = fixture("share_ended_v2.json");
    let damaged_path = fixture("share_ended_damaged.json");
    if updating() {
        std::fs::write(
            &newer_path,
            br#"{"format_version":2,"server":"ws://x","room":"07070707070707070707070707070707","server_seq":1,"synced_version":"","ended":"left"}"#,
        )
        .unwrap();
        std::fs::write(
            &damaged_path,
            br#"{"format_version":1,"server":"ws://x","room":"0707","server_seq":1,"synced_version":"","ended":"gone"}"#,
        )
        .unwrap();
    }
    let bytes = std::fs::read(fixture("ended_v36.curvyo")).unwrap();
    for path in [&newer_path, &damaged_path] {
        let json = std::fs::read(path).unwrap();
        assert_eq!(EndedRecord::from_bytes(&json), None);
        let container = with_member(&bytes, "share.ended.json", Some(&json));
        let opened = unpack(2, &container).unwrap();
        assert!(opened.legacy_share());
        assert!(!opened.is_shared());
    }
    assert_eq!(EndedRecord::from_bytes(b"not json"), None);
    // `heads` is part of version 1: a record without it, or with a short hash, is no way back.
    assert_eq!(
        EndedRecord::from_bytes(
            br#"{"format_version":1,"server":"ws://x","room":"07070707070707070707070707070707","server_seq":1,"synced_version":"97@15","ended":"left"}"#
        ),
        None
    );
    assert_eq!(
        EndedRecord::from_bytes(
            br#"{"format_version":1,"server":"ws://x","room":"07070707070707070707070707070707","server_seq":1,"synced_version":"97@15","ended":"left","heads":["41"]}"#
        ),
        None
    );
}

const W1: [u8; 16] = [0x57; 16];
const W2: [u8; 16] = [0x58; 16];

/// Criterion 16: the golden `devices_list_left_v1.loro`; `left` reads back, a switch clears it,
/// and a reader that does not know the field sees a plain switch-off.
#[test]
fn golden_devices_list_left_v1() {
    let path = fixture("devices_list_left_v1.loro");
    if updating() {
        let list = DevicesList::new(1);
        list.switch_workspace(W1, "Boxes", true).unwrap();
        list.leave_workspace(W1, "Boxes").unwrap();
        list.switch_workspace(W2, "Lamps", true).unwrap();
        std::fs::write(&path, list.snapshot().unwrap()).unwrap();
    }
    let bytes = std::fs::read(&path).unwrap();
    let list = DevicesList::from_snapshot(5, &bytes).unwrap();
    let left = list.workspace(&W1).unwrap();
    assert!(left.left && !left.on);
    assert!(!list.workspace(&W2).unwrap().left);
    assert_eq!(list.switched_on(), vec![W2]);
    // The entry's own fields: `on` is false, so an older reader shows a switch-off.
    let doc = loro::LoroDoc::new();
    doc.import(&bytes).unwrap();
    let value = doc.get_map("workspaces").get_deep_value();
    let entry = &value.as_map().unwrap()["57575757575757575757575757575757"];
    let fields = entry.as_map().unwrap();
    assert_eq!(fields.get("on"), Some(&loro::LoroValue::Bool(false)));
    assert_eq!(fields.get("left"), Some(&loro::LoroValue::Bool(true)));
    // Switching on clears the mark.
    list.switch_workspace(W1, "Boxes", true).unwrap();
    let back = list.workspace(&W1).unwrap();
    assert!(back.on && !back.left);
}
