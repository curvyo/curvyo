//! Format 52 (`specs/0135-image-optimisation` slice 2, criteria 5, 8, 18; ADR 0072 §3, §8): the
//! stack kinds `crop`, `resize`, `channel_mix` and `curves` on an image; a switched-on Crop makes
//! the drawn frame the placement's visible part while `image_placement` keeps the whole picture;
//! the golden `image_adjust_crop_v52.curvyo` reads back as plain values and saves back unchanged.
//! `CURVYO_WRITE_FIXTURES=1` rewrites the golden.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    AdjustEntry, AdjustStep, AdjustmentsWrite, Affine, AssetHash, CURRENT_FORMAT_VERSION, Document,
    NewImage, NodeId, ObjectSnapshot, Point, pack, unpack,
};
use serde_json::Value;

const GOLDEN: &str = "image_adjust_crop_v52.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (file.name().unwrap().to_string(), content)
        })
        .collect()
}

fn png() -> Arc<[u8]> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png");
    Arc::from(std::fs::read(path).unwrap().as_slice())
}

fn step(kind: &str, enabled: bool, fields: &[(&str, i64)]) -> AdjustEntry {
    AdjustEntry::Step(AdjustStep {
        kind: kind.to_owned(),
        enabled,
        fields: fields.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
    })
}

fn crop(enabled: bool, [l, t, w, h]: [i64; 4]) -> AdjustEntry {
    step(
        "crop",
        enabled,
        &[
            ("left_pct", l),
            ("top_pct", t),
            ("width_pct", w),
            ("height_pct", h),
        ],
    )
}

/// The stack of the picture "Slice 2": a Crop of 20 % on each side, Resize, Channel mix (red
/// filter) and an S curve, in pipeline order.
fn slice_2_stack() -> Vec<AdjustEntry> {
    vec![
        crop(true, [20, 20, 60, 60]),
        step("resize", true, &[("width_px", 120)]),
        step(
            "channel_mix",
            true,
            &[("red_pct", 100), ("green_pct", 0), ("blue_pct", 0)],
        ),
        step(
            "curves",
            true,
            &[
                ("in_0", 0),
                ("out_0", 0),
                ("in_1", 25),
                ("out_1", 15),
                ("in_2", 75),
                ("out_2", 85),
                ("in_3", 100),
                ("out_3", 100),
            ],
        ),
    ]
}

fn image(d: &Document, x: f64, name: &str) -> NodeId {
    d.add_images(&[NewImage {
        bytes: png(),
        placement: Affine::new(40.0, 0.0, 0.0, 30.0, x, 20.0),
        name: name.to_owned(),
    }])
    .unwrap()[0]
}

fn stack(d: &Document, id: NodeId, entries: Vec<AdjustEntry>) {
    let write = AdjustmentsWrite {
        steps: Some(entries),
        ..AdjustmentsWrite::default()
    };
    assert!(d.edit_image_adjustments(id, &write).unwrap());
}

fn golden_document() -> Document {
    let d = Document::new(1);
    let picture = image(&d, 10.0, "Slice 2");
    stack(&d, picture, slice_2_stack());
    let damaged = image(&d, 60.0, "Crop past the edge");
    stack(&d, damaged, vec![crop(true, [50, 0, 60, 100])]);
    let off = image(&d, 110.0, "Crop off");
    stack(&d, off, vec![crop(false, [10, 10, 10, 10])]);
    d
}

fn read_or_write() -> Vec<u8> {
    let path = fixture_path(GOLDEN);
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        let bytes = pack(&golden_document(), "0.1.0").unwrap();
        let comparable = |bytes: &[u8]| {
            let mut files = members(bytes);
            files.remove("document.loro");
            files
        };
        let unchanged = std::fs::read(&path).is_ok_and(|old| {
            zip::ZipArchive::new(Cursor::new(&old)).is_ok()
                && comparable(&old) == comparable(&bytes)
        });
        if !unchanged {
            let temp = path.with_extension(format!("curvyo.{}", std::process::id()));
            std::fs::write(&temp, &bytes).unwrap();
            std::fs::rename(&temp, &path).unwrap();
        }
    }
    std::fs::read(path).unwrap()
}

fn by_name(d: &Document, name: &str) -> ObjectSnapshot {
    d.read_objects()
        .objects
        .iter()
        .find(|o| d.node_registers(o.id()).unwrap().name.as_deref() == Some(name))
        .unwrap()
        .clone()
}

fn image_of(d: &Document, name: &str) -> curvyo_document_core::ImageSnapshot {
    match by_name(d, name) {
        ObjectSnapshot::Image(image) => image,
        other => panic!("an image, got {other:?}"),
    }
}

fn close(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
}

#[test]
fn the_golden_is_format_52_and_reads_back_every_slice_2_kind() {
    assert_eq!(CURRENT_FORMAT_VERSION, 64);
    let bytes = read_or_write();
    let manifest: Value = serde_json::from_slice(&members(&bytes)["manifest.json"]).unwrap();
    assert_eq!(manifest["format_version"], 52);
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "opening writes no step");
    let picture = image_of(&d, "Slice 2");
    assert_eq!(picture.adjustments.steps, slice_2_stack());
    // The drawn frame is the middle 60 % of the 40 x 30 mm placement at (10, 20).
    let corners = picture.corners();
    assert!(close(corners[0], Point::new(18.0, 26.0)), "{corners:?}");
    assert!(close(corners[2], Point::new(42.0, 44.0)), "{corners:?}");
    let full = picture.full_corners();
    assert!(close(full[0], Point::new(10.0, 20.0)) && close(full[2], Point::new(50.0, 50.0)));
    for name in ["Crop past the edge", "Crop off"] {
        let other = image_of(&d, name);
        assert_eq!(
            other.corners(),
            other.full_corners(),
            "{name}: the whole frame"
        );
        assert_eq!(other.adjustments.steps.len(), 1, "{name}: kept as stored");
    }
}

#[test]
fn the_golden_saves_back_unchanged() {
    let bytes = read_or_write();
    let d = unpack(2, &bytes).unwrap();
    let saved = pack(&d, "0.1.0").unwrap();
    // Saved by a build of format 53 (slice 3): the same content under its own version.
    let as_52 = |json: &[u8]| {
        String::from_utf8(json.to_vec()).unwrap().replacen(
            "\"format_version\": 64,",
            "\"format_version\": 52,",
            1,
        )
    };
    assert_eq!(
        as_52(&members(&saved)["document.json"]),
        as_52(&members(&bytes)["document.json"])
    );
    assert_eq!(
        saved,
        pack(&d, "0.1.0").unwrap(),
        "no adjusted pixel is stored"
    );
}

/// Criterion 18: a crop writes registers only; the asset's bytes (and so its SHA-256) are the
/// same, and removing the crop gives back the whole frame (lossless).
#[test]
fn a_crop_never_touches_the_asset_and_is_lossless() {
    let d = Document::new(1);
    let id = image(&d, 0.0, "P");
    let before = image_of(&d, "P");
    let hash: AssetHash = before.asset.clone();
    let bytes = d.asset(&hash).unwrap();
    stack(&d, id, vec![crop(true, [10, 0, 50, 100])]);
    let cropped = image_of(&d, "P");
    assert_eq!(cropped.placement, before.placement, "the placement is kept");
    assert_ne!(cropped.corners(), before.corners());
    assert_eq!(d.asset(&hash).unwrap(), bytes);
    assert_eq!(AssetHash::of(&bytes), hash);
    stack(&d, id, Vec::new());
    assert_eq!(image_of(&d, "P").corners(), before.corners());
}
