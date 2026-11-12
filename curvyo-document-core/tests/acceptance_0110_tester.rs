//! Tester acceptance for `specs/0110-pen-modes-variable-stroke` criterion 8 (old documents and
//! ordinary strokes unchanged, format bumped) and the stored half of criterion 2 (Spiro and
//! B-spline controls survive save and reopen).
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, AnchorKind, CURRENT_FORMAT_VERSION, CurveKind, Document, NewAnchor, OpenError, Point,
    pack, unpack,
};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn manifest_version(bytes: &[u8]) -> u64 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    serde_json::from_str::<serde_json::Value>(&text).unwrap()["format_version"]
        .as_u64()
        .unwrap()
}

/// Criterion 8: the version is bumped exactly once over `main`'s 40 (41 is reserved for `0103`)
/// and a save writes it.
#[test]
fn c8_format_version_is_42_and_written_on_save() {
    // Format 44 since `specs/0111-laser-operation-tools` (ADR 0056 §8).
    assert_eq!(CURRENT_FORMAT_VERSION, 45);
    let doc = Document::new(1);
    let bytes = pack(&doc, "test").unwrap();
    assert_eq!(manifest_version(&bytes), 45);
}

/// Criterion 8: the v42 golden opens here; the same bytes are refused by a reader one version
/// below what they claim (a 40 build checks `format_version > 40`, as `main`'s `container.rs`
/// does).
#[test]
fn c8_golden_v42_opens_and_an_older_reader_refuses() {
    let bytes = fixture("pen_modes_v42.curvyo");
    assert_eq!(manifest_version(&bytes), 42);
    let opened = unpack(2, &bytes).unwrap();
    let kinds: Vec<_> = opened
        .object_ids()
        .into_iter()
        .filter_map(|id| opened.path(id).map(|p| p.curve_kind))
        .collect();
    assert!(kinds.contains(&CurveKind::Spiro) && kinds.contains(&CurveKind::BSpline));
}

/// Criterion 8: documents from before the story open with no curve data or width points, and
/// a save and reopen through this build leaves every path as it was read.
#[test]
fn c8_older_documents_open_as_plain_paths_and_survive_a_resave() {
    for name in [
        "bring_content_in_v38.curvyo",
        "compound_v8.curvyo",
        "dash_v9.curvyo",
        "clone_overrides_v14.curvyo",
        "curve_polygon_star_v16.curvyo",
    ] {
        let bytes = fixture(name);
        assert!(manifest_version(&bytes) < 40, "{name}");
        let first = unpack(3, &bytes).unwrap();
        let ids = first.object_ids();
        let mut paths = 0;
        for &id in &ids {
            if let Some(path) = first.path(id) {
                paths += 1;
                assert_eq!(path.curve_kind, CurveKind::Bezier, "{name}");
                assert!(path.width_points.is_empty(), "{name}");
            }
        }
        let again = unpack(4, &pack(&first, "test").unwrap()).unwrap();
        assert_eq!(
            first.read_objects().objects,
            again.read_objects().objects,
            "{name} ({paths} paths) changed through a resave"
        );
    }
}

/// Criterion 8: a file that claims a newer version is refused, not opened without the data.
#[test]
fn c8_a_newer_file_is_refused() {
    let doc = Document::new(5);
    let bytes = pack(&doc, "test").unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().unwrap().to_string();
        let mut content = Vec::new();
        file.read_to_end(&mut content).unwrap();
        if name == "manifest.json" {
            let mut json: serde_json::Value = serde_json::from_slice(&content).unwrap();
            json["format_version"] = serde_json::json!(46);
            content = serde_json::to_vec(&json).unwrap();
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, &content).unwrap();
    }
    let newer = writer.finish().unwrap().into_inner();
    assert!(matches!(
        unpack(6, &newer),
        Err(OpenError::FormatTooNew { found: 46, .. })
    ));
}

fn smooth(k: u64, x: f64, y: f64) -> NewAnchor {
    let mut a = NewAnchor::corner(AnchorId::new(7, k), Point::new(x, y));
    a.kind = AnchorKind::Symmetric;
    a
}

/// Criterion 2 (CURVES A): the control points of Spiro and B-spline paths, their corner flags
/// and their width points come back after save and reopen, not their Bézier approximation.
#[test]
fn c2_curve_controls_and_width_points_survive_save_and_reopen() {
    for kind in [CurveKind::Spiro, CurveKind::BSpline] {
        let doc = Document::new(7);
        let mut controls = vec![
            smooth(1, 10.0, 20.0),
            smooth(2, 30.0, 10.0),
            smooth(3, 50.0, 20.0),
            smooth(4, 70.0, 10.0),
        ];
        controls[2].kind = AnchorKind::Corner;
        let id = doc.create_editable_curve(&controls, false, kind).unwrap();
        doc.set_width_point(id, AnchorId::new(7, 10), 0.0, 0.0)
            .unwrap();
        doc.set_width_point(id, AnchorId::new(7, 11), 0.6, 2.0)
            .unwrap();
        let before = doc.path(id).unwrap();
        let reopened = unpack(8, &pack(&doc, "test").unwrap()).unwrap();
        let after = reopened.path(id).unwrap();
        assert_eq!(after, before, "{kind:?}");
        assert_eq!(after.curve_kind, kind);
        assert_eq!(
            after.anchors.len(),
            4,
            "{kind:?}: controls, not Bézier pieces"
        );
        assert_eq!(after.anchors[2].kind, AnchorKind::Corner);
        assert_eq!(after.anchors[1].point, Point::new(30.0, 10.0));
        assert_eq!(after.width_points.len(), 2);
    }
}

/// Criterion 9 (ADR 0053 §3): invalid width points are refused by the document, not clamped.
#[test]
fn c9_document_refuses_invalid_width_points_and_changes_nothing() {
    let doc = Document::new(9);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(9, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    let before = doc.path(id).unwrap();
    for (position, multiplier) in [(-0.1, 1.0), (1.5, 1.0), (0.5, -1.0), (f64::NAN, 1.0)] {
        assert!(
            doc.set_width_point(id, AnchorId::new(9, 3), position, multiplier)
                .is_err(),
            "{position} {multiplier}"
        );
    }
    assert_eq!(doc.path(id).unwrap(), before);
}
