//! Tester acceptance tests for `specs/0105-sharing-concept` criteria 4 to 6 (LEGACY-SHARE A) and
//! the read table of ADR 0044 decision 3, written from the specification before the
//! implementation was read: the containers of documents shared on their own open as editable
//! local documents with one legacy flag, a save drops every legacy member, and workspace
//! documents read exactly as before.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Read, Write};

use curvyo_document_core::{
    CURRENT_SHARE_FORMAT_VERSION, Document, Length, OpenError, Point, RectBounds,
    WorkspaceShareRecord, pack, unpack,
};

const LEGACY_MEMBERS: [&str; 3] = ["share.json", "keyring.log", "share.ended.json"];

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

/// `base` with the extra members `extra` added (or replaced).
fn with_members(base: &[u8], extra: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(base)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let name = file.name().unwrap().into_owned();
        if extra.iter().any(|(n, _)| *n == name) {
            continue;
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        out.start_file(name, options).unwrap();
        out.write_all(&bytes).unwrap();
    }
    for (name, bytes) in extra {
        out.start_file(*name, options).unwrap();
        out.write_all(bytes).unwrap();
    }
    out.finish().unwrap().into_inner()
}

fn plain() -> Vec<u8> {
    let doc = Document::new(7);
    let _ = doc.create_rect(RectBounds {
        origin: Point::new(3.0, 4.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(6.0),
    });
    pack(&doc, "tester").unwrap()
}

const V1: &[u8] = br#"{"format_version":1,"server":"ws://10.0.0.2:8787","room":"07070707070707070707070707070707","server_seq":3,"synced_version":"2@7"}"#;
const V2: &[u8] = br#"{"format_version":2,"workspace":"11111111111111111111111111111111","document":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","synced_version":"2@7"}"#;
const ENDED: &[u8] = br#"{"format_version":1,"server":"ws://10.0.0.2:8787","room":"07070707070707070707070707070707","server_seq":3,"synced_version":"2@7","ended":"removed","heads":[]}"#;
const KEYRING: &[u8] = b"\x00\x01not read";

/// Opens `bytes`, checks it is an editable local document with the legacy flag, saves it, and
/// checks the saved file holds no legacy member, opens without the flag and keeps the drawing.
fn opens_local_and_saves_clean(bytes: &[u8]) {
    let doc = unpack(11, bytes).unwrap();
    assert!(doc.legacy_share(), "one legacy notice");
    assert!(doc.workspace_share().is_none(), "no share state of its own");
    let before = doc.export_json().unwrap();
    // Editable: an edit lands.
    let _ = doc.create_rect(RectBounds {
        origin: Point::new(50.0, 50.0),
        width: Length::from_mm(1.0),
        height: Length::from_mm(1.0),
    });
    let edited = doc.export_json().unwrap();
    assert_ne!(before, edited, "the document is editable");
    let saved = pack(&doc, "tester").unwrap();
    for name in LEGACY_MEMBERS {
        assert!(
            member(&saved, name).is_none(),
            "{name} in {:?}",
            names(&saved)
        );
    }
    let again = unpack(12, &saved).unwrap();
    assert!(!again.legacy_share(), "no notice after the save");
    assert!(again.workspace_share().is_none());
    assert_eq!(again.export_json().unwrap(), edited, "no change lost");
}

/// Criteria 4 and 5 on the golden `shared_v34.curvyo` (share record and keyring).
#[test]
fn c4_c5_shared_v34_opens_local_and_saves_without_its_members() {
    let golden = fixture("shared_v34.curvyo");
    assert!(member(&golden, "keyring.log").is_some(), "golden unchanged");
    opens_local_and_saves_clean(&golden);
}

/// Criteria 4 and 5 on the golden `ended_v36.curvyo` (ended record).
#[test]
fn c4_c5_ended_v36_opens_local_and_saves_without_its_ended_record() {
    let golden = fixture("ended_v36.curvyo");
    assert!(
        member(&golden, "share.ended.json").is_some(),
        "golden unchanged"
    );
    opens_local_and_saves_clean(&golden);
}

/// Criterion 4: opening reads; it does not change the bytes it was given (the in-memory half
/// of `open_writes_nothing`; the disk half is in `curvyo-storage-io`).
#[test]
fn c4_unpack_leaves_the_bytes_and_a_second_open_agrees() {
    for name in ["shared_v34.curvyo", "ended_v36.curvyo"] {
        let golden = fixture(name);
        let copy = golden.clone();
        let a = unpack(1, &golden).unwrap();
        let b = unpack(2, &golden).unwrap();
        assert_eq!(golden, copy);
        assert_eq!(a.export_json().unwrap(), b.export_json().unwrap(), "{name}");
    }
}

/// ADR 0044 decision 3, every row of the read table on a crafted container.
#[test]
fn adr0044_d3_the_read_table() {
    let base = plain();
    // `share.json` v1 alone, v1 with `keyring.log`, `keyring.log` alone: local with the flag.
    opens_local_and_saves_clean(&with_members(&base, &[("share.json", V1)]));
    opens_local_and_saves_clean(&with_members(
        &base,
        &[("share.json", V1), ("keyring.log", KEYRING)],
    ));
    opens_local_and_saves_clean(&with_members(&base, &[("keyring.log", KEYRING)]));
    // `share.ended.json`: local with the flag.
    opens_local_and_saves_clean(&with_members(&base, &[("share.ended.json", ENDED)]));
    // All three legacy members together.
    opens_local_and_saves_clean(&with_members(
        &base,
        &[
            ("share.json", V1),
            ("keyring.log", KEYRING),
            ("share.ended.json", ENDED),
        ],
    ));
    // `share.json` v2 alone: workspace document, unchanged on save.
    let workspace = with_members(&base, &[("share.json", V2)]);
    let doc = unpack(3, &workspace).unwrap();
    assert!(!doc.legacy_share());
    let record: WorkspaceShareRecord = doc.workspace_share().unwrap();
    assert_eq!(record.workspace, [0x11; 16]);
    assert_eq!(record.document, [0xd0; 16]);
    let saved = pack(&doc, "tester").unwrap();
    let again = unpack(4, &saved).unwrap().workspace_share().unwrap();
    assert_eq!(again, record, "a save keeps the record");
    assert!(member(&saved, "keyring.log").is_none());
    // `share.json` v2 and `keyring.log`: Damaged.
    assert!(matches!(
        unpack(
            5,
            &with_members(&base, &[("share.json", V2), ("keyring.log", KEYRING)])
        ),
        Err(OpenError::Damaged)
    ));
    // `share.json` newer than 2: FormatTooNew.
    let v3 = br#"{"format_version":3,"workspace":"11111111111111111111111111111111"}"#;
    assert_eq!(
        CURRENT_SHARE_FORMAT_VERSION, 2,
        "the reader's version does not change"
    );
    match unpack(6, &with_members(&base, &[("share.json", v3)])) {
        Err(OpenError::FormatTooNew { found, supported }) => {
            assert_eq!((found, supported), (3, 2));
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
}

/// The reader only becomes more lenient: a legacy v1 record with fields this build does not
/// know, or missing the server, still opens as local (nothing of it is read).
#[test]
fn adr0044_d3_a_v1_record_of_any_shape_opens_local() {
    let base = plain();
    for v1 in [
        br#"{"format_version":1}"#.as_slice(),
        br#"{"format_version":1,"server":"x","room":"zz","server_seq":1,"synced_version":"bad","extra":true}"#,
    ] {
        opens_local_and_saves_clean(&with_members(&base, &[("share.json", v1)]));
    }
}

/// Edge: a damaged `share.json` stays damaged (no silent local open of garbage).
#[test]
fn adr0044_d3_a_share_json_that_is_no_json_is_damaged() {
    let base = plain();
    assert!(matches!(
        unpack(1, &with_members(&base, &[("share.json", b"{not json")])),
        Err(OpenError::Damaged)
    ));
}

/// Criterion 6: the workspace golden `workspace_shared_v35.curvyo` opens exactly as before: a
/// workspace document with its record and no legacy flag; a save keeps the record.
#[test]
fn c6_workspace_shared_v35_reads_as_before() {
    let golden = fixture("workspace_shared_v35.curvyo");
    let doc = unpack(9, &golden).unwrap();
    assert!(!doc.legacy_share());
    let record = doc.workspace_share().unwrap();
    assert_eq!(record.workspace, [0x11; 16]);
    assert_eq!(record.document, [0xd0; 16]);
    let saved = pack(&doc, "tester").unwrap();
    assert_eq!(
        unpack(10, &saved).unwrap().workspace_share().unwrap(),
        record
    );
}

/// Criterion 6: the workspace marker's `share.json` (v1 shape) still reads.
#[test]
fn c6_the_marker_share_json_v1_still_reads() {
    let json = fixture("workspace_marker_share_v1.json");
    let record = curvyo_document_core::WorkspaceRoomRecord::from_bytes(&json).unwrap();
    assert_eq!(record.server, "ws://192.168.1.20:8787");
    assert_eq!(record.workspace, [0x57; 16]);
    assert_eq!(record.server_seq, 42);
}

/// Edge (not in the table): a workspace document (`share.json` v2) that also carries a stray
/// `share.ended.json` from an older build. It must stay a workspace document; the save drops
/// the ended record.
#[test]
fn edge_a_workspace_document_with_a_stray_ended_record() {
    let bytes = with_members(&plain(), &[("share.json", V2), ("share.ended.json", ENDED)]);
    let doc = unpack(1, &bytes).unwrap();
    assert!(
        doc.workspace_share().is_some(),
        "still a workspace document"
    );
    let saved = pack(&doc, "tester").unwrap();
    assert!(member(&saved, "share.ended.json").is_none());
    assert!(member(&saved, "share.json").is_some());
    assert!(!unpack(2, &saved).unwrap().legacy_share());
}
