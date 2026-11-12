//! Format 42 (`specs/0110-pen-modes-variable-stroke` criterion 8, ADR 0053): the golden
//! `pen_modes_v42.curvyo` reads back every Spiro and B-spline control, the Spiro placement and the
//! width points identical, an ordinary path stays ordinary, and a build that stops at 40 refuses
//! it. `CURVYO_WRITE_FIXTURES=1` rewrites the golden.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorId, AnchorKind, CURRENT_FORMAT_VERSION, CurveKind, Document, NewAnchor,
    OpenError, Point, pack, unpack,
};

const GOLDEN: &str = "pen_modes_v42.curvyo";

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(GOLDEN)
}

fn node(k: u64, x: f64, y: f64, kind: AnchorKind) -> NewAnchor {
    let mut anchor = NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
    anchor.kind = kind;
    anchor
}

fn golden_document() -> Document {
    let d = Document::new(1);
    let smooth = AnchorKind::Symmetric;
    let spiro = d
        .create_editable_curve(
            &[
                node(1, 10.0, 20.0, smooth),
                node(2, 30.0, 10.0, AnchorKind::Corner),
                node(3, 50.0, 20.0, smooth),
                node(4, 70.0, 10.0, smooth),
            ],
            false,
            CurveKind::Spiro,
        )
        .unwrap();
    d.set_spiro_transform(spiro, Affine::new(1.0, 0.0, 0.0, 2.0, 5.0, 3.0))
        .unwrap();
    let tapered = d.create_path(
        &[
            node(20, 10.0, 120.0, AnchorKind::Corner),
            node(21, 90.0, 120.0, AnchorKind::Corner),
        ],
        false,
    );
    // One step, as the Style panel's Taper both writes it.
    d.batch("set_width_profile", |d| {
        for (k, position, multiplier) in [(30, 0.0, 0.2), (31, 0.5, 2.0), (32, 1.0, 0.2)] {
            d.set_width_point(tapered, AnchorId::new(1, k), position, multiplier)
                .unwrap();
        }
    });
    // One step for the last two paths: the history never holds two equal labels in a row at
    // time 0 (`acceptance_0020_tester_more` c36 reads every fixture).
    d.batch("create_path", |d| {
        d.create_editable_curve(
            &[
                node(10, 10.0, 60.0, smooth),
                node(11, 40.0, 50.0, smooth),
                node(12, 70.0, 60.0, smooth),
                node(13, 40.0, 90.0, smooth),
            ],
            true,
            CurveKind::BSpline,
        )
        .unwrap();
        let _ordinary = d.create_path(
            &[
                node(40, 10.0, 150.0, AnchorKind::Corner),
                node(41, 90.0, 150.0, AnchorKind::Corner),
            ],
            false,
        );
    });
    d
}

fn read_golden() -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(), pack(&golden_document(), "0.1.0").unwrap()).unwrap();
    }
    std::fs::read(fixture_path()).unwrap()
}

fn with_manifest_version(bytes: &[u8], version: u32) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().unwrap().to_string();
        let mut content = Vec::new();
        file.read_to_end(&mut content).unwrap();
        if name == "manifest.json" {
            let mut json: serde_json::Value = serde_json::from_slice(&content).unwrap();
            json["format_version"] = serde_json::json!(version);
            content = serde_json::to_vec(&json).unwrap();
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&content).unwrap();
    }
    writer.finish().unwrap().into_inner()
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

#[test]
fn the_golden_reads_back_every_curve_control_placement_and_width_point() {
    let expected = golden_document();
    let bytes = read_golden();
    assert_eq!(manifest_version(&bytes), 42);
    // Format 44 since `specs/0111-laser-operation-tools` (ADR 0056 §8).
    assert_eq!(CURRENT_FORMAT_VERSION, 44);
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let (a, b) = (expected.read_objects(), opened.read_objects());
    assert_eq!(a.objects, b.objects);
    let kinds: Vec<_> = b
        .objects
        .iter()
        .filter_map(|o| o.drawn_path().map(|p| (p.curve_kind, p.width_points.len())))
        .collect();
    assert_eq!(
        kinds,
        [
            (CurveKind::Spiro, 0),
            (CurveKind::Bezier, 3),
            (CurveKind::BSpline, 0),
            (CurveKind::Bezier, 0),
        ]
    );
    let spiro = b.objects[0].drawn_path().unwrap();
    assert_eq!(
        spiro.spiro_transform.as_deref().copied(),
        Some(Affine::new(1.0, 0.0, 0.0, 2.0, 5.0, 3.0))
    );
    assert_eq!(
        spiro.anchors[1].kind,
        AnchorKind::Corner,
        "a corner stays a corner"
    );
}

#[test]
fn a_build_of_format_40_refuses_the_golden_as_saved_by_a_newer_version() {
    // A format-40 build checks `format_version` against its own 40: the golden's 42 is newer, so
    // it is refused before the curve kinds and width points could be dropped (ADR 0053 §6). This
    // build plays that check one version up.
    let newer = with_manifest_version(&read_golden(), CURRENT_FORMAT_VERSION + 1);
    assert!(matches!(
        unpack(2, &newer),
        Err(OpenError::FormatTooNew {
            found: 45,
            supported: 44
        })
    ));
}
