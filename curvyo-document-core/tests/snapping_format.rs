//! The stored shape of the snapping settings (`specs/0060-snapping` criteria 1, 3 and 14,
//! `adrs.md` decisions 1 and 2): seven root registers, defaults on absence, strict validation on
//! open, one labelled commit that writes only the registers that changed, the golden
//! `snapping_v27.curvyo` and five damaged files. `CURVYO_WRITE_FIXTURES=1` rewrites the
//! fixtures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read, Write};
use std::ops::ControlFlow;
use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, DocumentSnapping,
    NewAnchor, NodeId, OpenError, Point, RectBounds, pack, unpack,
};
use loro::{LoroDoc, LoroValue};

/// The version this slice takes. A rebase onto another bump renumbers this constant, the
/// fixtures and their names (`specs/README.md`, "`format_version` plan").
const THIS_SLICE_VERSION: u32 = 27;
const GOLDEN: &str = "snapping_v27.curvyo";
const DAMAGED: [&str; 5] = [
    "snapping_enabled_type_v27.curvyo",
    "snapping_capture_type_v27.curvyo",
    "snapping_capture_negative_v27.curvyo",
    "snapping_release_nan_v27.curvyo",
    "snapping_objects_container_v27.curvyo",
];

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn loro_of(document: &Document) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro
}

fn root_value(document: &Document, key: &str) -> Option<LoroValue> {
    loro_of(document)
        .get_map("root")
        .get(key)
        .map(|value| value.get_deep_value())
}

fn labels(document: &Document) -> Vec<String> {
    let loro = loro_of(document);
    let ids: Vec<loro::ID> = loro.oplog_frontiers().iter().collect();
    let mut out: Vec<(u32, String)> = Vec::new();
    loro.travel_change_ancestors(&ids, &mut |m| {
        out.push((
            m.lamport,
            m.message
                .map(|s| s.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        ControlFlow::Continue(())
    })
    .unwrap();
    out.sort();
    out.into_iter().map(|(_, label)| label).collect()
}

fn op_count(document: &Document) -> i32 {
    loro_of(document).oplog_vv().values().copied().sum()
}

fn container_from_loro(loro_bytes: &[u8]) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        ("document.loro", loro_bytes.to_vec()),
        ("document.json", b"{}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// A file whose root map carries the given extra values, as a hand-made or damaged file would.
fn file_with_root(values: &[(&str, LoroValue)]) -> Vec<u8> {
    let loro = loro_of(&Document::new(1));
    loro.set_peer_id(1).unwrap();
    let root = loro.get_map("root");
    for (key, value) in values {
        root.insert(key, value.clone()).unwrap();
    }
    loro.commit();
    container_from_loro(&loro.export(loro::ExportMode::Snapshot).unwrap())
}

fn manifest_version(bytes: &[u8]) -> u32 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut manifest = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut manifest)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    u32::try_from(manifest["format_version"].as_u64().unwrap()).unwrap()
}

fn damaged_files() -> [(&'static str, Vec<u8>); 5] {
    let container = {
        let loro = loro_of(&Document::new(1));
        loro.set_peer_id(1).unwrap();
        let root = loro.get_map("root");
        let text = root
            .insert_container("snapping_objects", loro::LoroText::new())
            .unwrap();
        text.insert(0, "true").unwrap();
        loro.commit();
        container_from_loro(&loro.export(loro::ExportMode::Snapshot).unwrap())
    };
    [
        (
            DAMAGED[0],
            file_with_root(&[("snapping_enabled", LoroValue::from("yes"))]),
        ),
        (
            DAMAGED[1],
            file_with_root(&[("snapping_capture_px", LoroValue::from("8"))]),
        ),
        (
            DAMAGED[2],
            file_with_root(&[("snapping_capture_px", LoroValue::Double(-1.0))]),
        ),
        (
            DAMAGED[3],
            file_with_root(&[("snapping_release_px", LoroValue::Double(f64::NAN))]),
        ),
        (DAMAGED[4], container),
    ]
}

fn golden_document() -> Document {
    let document = Document::new(1);
    let path = document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(40.0, 30.0)),
        ],
        false,
    );
    document.set_guide(&[path], true).unwrap();
    let _ = document.create_rect(RectBounds {
        origin: Point::new(60.0, 10.0),
        width: curvyo_document_core::Length::from_mm(20.0),
        height: curvyo_document_core::Length::from_mm(30.0),
    });
    assert!(document.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        guides: true,
        nodes: true,
        page: true,
        capture_px: 6.0,
        release_px: 20.0,
    }));
    document
}

// ---- criteria 1 and 3: the value and its seven registers ----------------------

#[test]
fn a_new_document_has_the_default_settings_and_stores_no_register() {
    let document = Document::new(1);
    let snapping = document.snapping();
    assert_eq!(snapping, DocumentSnapping::DEFAULT);
    assert!(!snapping.enabled);
    assert!(snapping.objects && snapping.guides && snapping.nodes && snapping.page);
    assert_eq!(snapping.capture_px, 8.0);
    assert_eq!(snapping.release_px, 14.0);
    for key in [
        "snapping_enabled",
        "snapping_objects",
        "snapping_guides",
        "snapping_nodes",
        "snapping_page",
        "snapping_capture_px",
        "snapping_release_px",
    ] {
        assert_eq!(root_value(&document, key), None, "{key}");
    }
}

#[test]
fn each_flag_writes_only_its_own_register() {
    let document = Document::new(1);
    assert!(document.set_snapping(DocumentSnapping {
        enabled: true,
        ..DocumentSnapping::DEFAULT
    }));
    assert_eq!(
        root_value(&document, "snapping_enabled"),
        Some(LoroValue::Bool(true))
    );
    assert_eq!(root_value(&document, "snapping_objects"), None);

    let document = Document::new(1);
    assert!(document.set_snapping(DocumentSnapping {
        objects: false,
        ..DocumentSnapping::DEFAULT
    }));
    assert_eq!(
        root_value(&document, "snapping_objects"),
        Some(LoroValue::Bool(false))
    );
    assert_eq!(root_value(&document, "snapping_guides"), None);
}

#[test]
fn a_zone_edit_writes_only_that_zone_as_a_double() {
    let document = Document::new(1);
    assert!(document.set_snapping(DocumentSnapping {
        capture_px: 6.0,
        release_px: 20.0,
        ..DocumentSnapping::DEFAULT
    }));
    assert_eq!(
        root_value(&document, "snapping_capture_px"),
        Some(LoroValue::Double(6.0))
    );
    assert_eq!(
        root_value(&document, "snapping_release_px"),
        Some(LoroValue::Double(20.0))
    );
    assert_eq!(root_value(&document, "snapping_page"), None);
}

#[test]
fn a_release_below_the_capture_is_raised_before_it_is_written() {
    let document = Document::new(1);
    assert!(document.set_snapping(DocumentSnapping {
        capture_px: 20.0,
        release_px: 5.0,
        ..DocumentSnapping::DEFAULT
    }));
    assert_eq!(
        root_value(&document, "snapping_release_px"),
        Some(LoroValue::Double(20.0))
    );
    assert_eq!(document.snapping().release_px, 20.0);
}

#[test]
fn an_invalid_zone_writes_nothing() {
    for (capture, release) in [(f64::NAN, 14.0), (8.0, f64::INFINITY), (-1.0, -2.0)] {
        let document = Document::new(1);
        let ops = op_count(&document);
        assert!(
            !document.set_snapping(DocumentSnapping {
                capture_px: capture,
                release_px: release,
                ..DocumentSnapping::DEFAULT
            }),
            "{capture}/{release}"
        );
        assert_eq!(op_count(&document), ops);
    }
}

#[test]
fn a_value_equal_to_the_stored_one_writes_nothing_and_makes_no_commit() {
    let document = Document::new(1);
    let ops = op_count(&document);
    let commits = labels(&document);
    assert!(!document.set_snapping(DocumentSnapping::DEFAULT));
    assert_eq!(op_count(&document), ops);
    let _ = document.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        capture_px: 6.0,
        ..DocumentSnapping::DEFAULT
    });
    let ops = op_count(&document);
    let commits_after = labels(&document);
    assert!(!document.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        capture_px: 6.0,
        ..DocumentSnapping::DEFAULT
    }));
    assert_eq!(op_count(&document), ops);
    assert_eq!(labels(&document), commits_after);
    assert_eq!(commits_after.len(), commits.len() + 1);
}

#[test]
fn every_edit_is_one_commit_labelled_set_document_snapping() {
    let document = Document::new(1);
    let before = labels(&document).len();
    assert!(document.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        nodes: false,
        page: false,
        capture_px: 6.0,
        release_px: 20.0,
    }));
    let after = labels(&document);
    assert_eq!(after.len(), before + 1, "seven registers in one commit");
    assert_eq!(after.last().unwrap(), "set_document_snapping");
}

// ---- criterion 3: a hand-written inverted pair reads as the capture zone -------

#[test]
fn a_stored_release_below_the_capture_reads_as_the_capture() {
    let bytes = file_with_root(&[
        ("snapping_capture_px", LoroValue::Double(20.0)),
        ("snapping_release_px", LoroValue::Double(3.0)),
    ]);
    let document = unpack(2, &bytes).unwrap();
    assert_eq!(document.snapping().capture_px, 20.0);
    assert_eq!(document.snapping().release_px, 20.0);
}

// ---- save, close, open ----------------------------------------------------------

#[test]
fn every_register_survives_save_and_open_exactly() {
    let document = Document::new(1);
    let value = DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        nodes: true,
        page: false,
        capture_px: 6.0,
        release_px: 20.0,
    };
    assert!(document.set_snapping(value));
    let reopened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    assert_eq!(reopened.snapping(), value);
}

#[test]
fn document_json_carries_the_settings_and_the_current_version() {
    let document = Document::new(1);
    let _ = document.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        capture_px: 6.0,
        release_px: 20.0,
        ..DocumentSnapping::DEFAULT
    });
    let json: serde_json::Value = serde_json::from_slice(&document.export_json().unwrap()).unwrap();
    assert_eq!(json["snapping"]["enabled"], true);
    assert_eq!(json["snapping"]["objects"], false);
    assert_eq!(json["snapping"]["guides"], true);
    assert_eq!(json["snapping"]["nodes"], true);
    assert_eq!(json["snapping"]["page"], true);
    assert_eq!(json["snapping"]["capture_px"], 6.0);
    assert_eq!(json["snapping"]["release_px"], 20.0);
    assert_eq!(json["format_version"], CURRENT_FORMAT_VERSION);
}

// ---- criterion 2: the format version ---------------------------------------------

#[test]
fn this_slice_took_its_format_version_and_the_writer_declares_it() {
    let bytes = pack(&Document::new(1), "t").unwrap();
    assert!(manifest_version(&bytes) >= THIS_SLICE_VERSION);
    assert_eq!(manifest_version(&bytes), CURRENT_FORMAT_VERSION);
}

#[test]
fn a_file_of_the_new_version_without_the_registers_means_the_default() {
    let bytes = pack(&Document::new(1), "t").unwrap();
    assert_eq!(
        unpack(2, &bytes).unwrap().snapping(),
        DocumentSnapping::DEFAULT
    );
}

// ---- criterion 14: strict validation ----------------------------------------------

#[test]
fn each_malformed_register_is_refused_as_damaged() {
    for (name, bytes) in damaged_files() {
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}"
        );
    }
}

#[test]
fn the_damaged_fixtures_are_refused_as_damaged() {
    write_fixtures_on_request();
    for name in DAMAGED {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}"
        );
    }
}

#[test]
fn every_valid_edge_opens() {
    for zone in [0.0, 0.25, 1.0, 8.0, 100.0, 101.0, 1_000_000.0] {
        assert!(
            unpack(
                2,
                &file_with_root(&[("snapping_capture_px", LoroValue::Double(zone))])
            )
            .is_ok(),
            "{zone}"
        );
    }
    for enabled in [false, true] {
        assert!(
            unpack(
                2,
                &file_with_root(&[("snapping_enabled", LoroValue::Bool(enabled))])
            )
            .is_ok()
        );
    }
}

// ---- golden -------------------------------------------------------------------------

fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
        for (name, bytes) in damaged_files() {
            std::fs::write(fixture_path(name), bytes).unwrap();
        }
    }
}

#[test]
fn the_golden_reads_back_and_opening_it_writes_nothing() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&bytes), THIS_SLICE_VERSION);
    let document = unpack(2, &bytes).unwrap();
    let snapping = document.snapping();
    assert_eq!(
        snapping,
        DocumentSnapping {
            enabled: true,
            objects: false,
            guides: true,
            nodes: true,
            page: true,
            capture_px: 6.0,
            release_px: 20.0,
        }
    );
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut stored_bytes = Vec::new();
    archive
        .by_name("document.loro")
        .unwrap()
        .read_to_end(&mut stored_bytes)
        .unwrap();
    let stored = LoroDoc::new();
    stored.import(&stored_bytes).unwrap();
    assert_eq!(loro_of(&document).oplog_vv(), stored.oplog_vv());
}

/// A helper for the fixture writer test above: the golden's guide and rect are only there to
/// make the file representative, so a quick check keeps them from silently disappearing.
#[test]
fn the_golden_holds_a_guide_and_a_rect() {
    let document = golden_document();
    let guide: Vec<NodeId> = document
        .read_nodes()
        .into_iter()
        .filter(|read| read.guide)
        .filter_map(|read| read.object().map(curvyo_document_core::ObjectSnapshot::id))
        .collect();
    assert_eq!(guide.len(), 1);
    assert_eq!(
        document
            .read_nodes()
            .into_iter()
            .filter(|r| r.object().is_some())
            .count(),
        2
    );
}
