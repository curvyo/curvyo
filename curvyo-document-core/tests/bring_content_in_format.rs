//! Format 38 (`specs/0100-bring-content-in` criterion 21, ADR 0045 §9): the golden
//! `bring_content_in_v38.curvyo` reads back every clone, placed page and own item identical, and a
//! build that stops at 37 refuses it. `CURVYO_WRITE_FIXTURES=1` rewrites the golden.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, AnchorId, Angle, CURRENT_FORMAT_VERSION, Document, DocumentSize, FontKind, FontRef,
    Length, NewAnchor, NewImage, NewOutputPage, ObjectSnapshot, OpenError, Point, RectBounds,
    TextLayoutValues, Vec2, pack, unpack,
};

const GOLDEN: &str = "bring_content_in_v38.curvyo";

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(GOLDEN)
}

fn golden_document() -> Document {
    let d = Document::new(1);
    let drawing = d.pages()[0].clone();
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let font = FontRef {
        family: "Source Sans 3".into(),
        style: "Regular".into(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    let text = d
        .create_text(
            "ANNA",
            &font,
            Length::from_mm(10.0),
            Affine::translation(Vec2::new(5.0, 40.0)),
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png"),
    )
    .unwrap();
    let image = d
        .add_images(&[NewImage {
            bytes: Arc::from(bytes),
            placement: Affine::new(20.0, 0.0, 0.0, 10.0, 30.0, 0.0),
            name: "photo".into(),
        }])
        .unwrap()[0];
    let rect = d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(20.0, 10.0),
    ));
    let c = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
    let path = d.create_path(&[c(1, 60.0, 0.0), c(2, 70.0, 0.0), c(3, 70.0, 10.0)], true);
    let group = d.group_objects(&[text, image, rect, path]).unwrap();
    let on_laser = Vec2::new(
        laser.origin.x - drawing.origin.x + 20.0,
        laser.origin.y - drawing.origin.y + 20.0,
    );
    let group_clone = d.place_linked_clones(&[group], on_laser).unwrap()[0];
    d.linked_clone(&[text, image], &[]).unwrap();
    d.place_linked_clones(&[group_clone], Vec2::new(0.0, 60.0))
        .unwrap();
    d.place_page(
        &laser.id,
        &drawing.id,
        Affine::translation(Vec2::new(150.0, 10.0)),
    )
    .unwrap();
    let turned = Affine::translation(Vec2::new(300.0, 50.0))
        .compose(Affine::rotation_about(
            Point::new(0.0, 0.0),
            Angle::from_radians(std::f64::consts::FRAC_PI_2),
        ))
        .compose(Affine::scale(0.5, 0.5));
    // An object moved onto the laser page with Alt: a plain own item there.
    let own = d.create_path(&[c(10, 0.0, 200.0), c(11, 10.0, 200.0)], false);
    d.translate_objects(&[own], on_laser).unwrap();
    d.place_page(&laser.id, &drawing.id, turned).unwrap();
    d
}

fn read_golden() -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(), pack(&golden_document(), "0.1.0").unwrap()).unwrap();
    }
    std::fs::read(fixture_path()).unwrap()
}

fn manifest(bytes: &[u8]) -> serde_json::Value {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn the_golden_reads_back_every_clone_placed_page_and_own_item() {
    let expected = golden_document();
    let bytes = read_golden();
    assert_eq!(manifest(&bytes)["format_version"], 38);
    // `specs/0101-laser-operations` bumps to 39 (ADR 0046 §4) and `specs/0102-laser-fixtures`
    // to 40 (ADR 0049); the v38 golden still opens.
    // Format 41 since `specs/0103-fixture-jobs` (ADR 0052 §6).
    // Format 42 since `specs/0110-pen-modes-variable-stroke` (ADR 0053).
    // Format 44 since `specs/0111-laser-operation-tools` (ADR 0056 §8).
    assert_eq!(CURRENT_FORMAT_VERSION, 45);
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let (a, b) = (expected.read_objects(), opened.read_objects());
    assert_eq!(b.orphans, Vec::new());
    assert_eq!(a.objects.len(), b.objects.len());
    let clones = a
        .objects
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Clone(_)))
        .count();
    assert_eq!(clones, 4, "group clone, chain, text and image clone");
    for (x, y) in a.objects.iter().zip(b.objects.iter()) {
        match (x, y) {
            (ObjectSnapshot::Clone(x), ObjectSnapshot::Clone(y)) => {
                assert_eq!(x.chain.origin, y.chain.origin);
                assert_eq!(x.chain.matrix, y.chain.matrix);
                assert_eq!(x.chain.own_placement, y.chain.own_placement);
                assert_eq!(x.chain.picture, y.chain.picture, "drawn alike");
            }
            (x, y) => assert_eq!(x, y),
        }
    }
    let placed = |d: &Document| -> Vec<(String, Affine)> {
        d.placed_pages()
            .iter()
            .map(|p| (p.source.as_ref().unwrap().id.clone(), p.matrix))
            .collect()
    };
    assert_eq!(placed(&expected), placed(&opened));
    assert_eq!(placed(&opened).len(), 2);
}

#[test]
fn a_build_of_format_37_refuses_the_golden_as_saved_by_a_newer_version() {
    // A format-37 build checks `format_version` against its own 37: the golden's 38 is newer,
    // so it is refused before its clones could be read as orphans and deleted (ADR 0045 §9).
    // This build plays that check one version up: a file one version newer than this build is
    // refused the same way.
    let bytes = read_golden();
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes[..])).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().unwrap().to_string();
        let mut content = Vec::new();
        file.read_to_end(&mut content).unwrap();
        if name == "manifest.json" {
            let mut json: serde_json::Value = serde_json::from_slice(&content).unwrap();
            json["format_version"] = serde_json::json!(CURRENT_FORMAT_VERSION + 1);
            content = serde_json::to_vec(&json).unwrap();
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&content).unwrap();
    }
    let newer = writer.finish().unwrap().into_inner();
    assert!(matches!(
        unpack(2, &newer),
        Err(OpenError::FormatTooNew {
            found: 46,
            supported: 45
        })
    ));
}

#[test]
fn a_format_37_golden_opens_unchanged_and_is_not_rewritten() {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/page_kinds_v37.curvyo"),
    )
    .unwrap();
    assert_eq!(manifest(&bytes)["format_version"], 37);
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0);
}
