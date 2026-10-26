//! The image format (`specs/0058-bitmap-import` criterion 44, ADR 0017 §3): the golden project
//! `image_v25.curvyo` and its four damaged variants. `CURVYO_WRITE_FIXTURES=1` rewrites them.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, Angle, AssetHash, CURRENT_FORMAT_VERSION, CopySource, Document, NewImage, NodeKind,
    ObjectSnapshot, Opacity, Point, Vec2, pack, unpack,
};

/// The version this slice takes; a rebase onto another bump renumbers it and the fixtures.
const IMAGE_FORMAT_VERSION: u32 = 25;
const GOLDEN: &str = "image_v25.curvyo";
const DAMAGED: [&str; 4] = [
    "image_asset_missing_v25.curvyo",
    "image_asset_hash_mismatch_v25.curvyo",
    "image_asset_unsupported_v25.curvyo",
    "image_asset_empty_v25.curvyo",
];

const IMAGES: &str = "../curvyo-image-core/tests/fixtures/images/";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn picture(name: &str) -> Arc<[u8]> {
    Arc::from(
        std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(IMAGES)
                .join(name),
        )
        .unwrap(),
    )
}

fn new_image(name: &str, x: f64) -> NewImage {
    NewImage {
        bytes: picture(name),
        placement: Affine::new(40.0, 0.0, 0.0, 20.0, x, 20.0),
        name: name.to_owned(),
    }
}

/// A PNG with alpha, a JPEG, a WebP; one rotated 30 degrees, flipped and 50 % opaque; one in a
/// group; one hidden; one locked; one duplicate sharing an asset.
fn golden_document() -> Document {
    let d = Document::new(1);
    let ids = d
        .add_images(&[
            new_image("png_rgba_8.png", 10.0),
            new_image("jpeg_progressive.jpg", 60.0),
            new_image("webp_lossless_alpha.webp", 110.0),
            new_image("webp_lossy.webp", 160.0),
            new_image("png_gray_16.png", 10.0),
        ])
        .unwrap();
    let turned = d.object(ids[1]).unwrap().rotated(
        Point::new(80.0, 30.0),
        Angle::from_radians(30_f64.to_radians()),
    );
    let ObjectSnapshot::Image(turned) = turned else {
        unreachable!()
    };
    let flipped = turned.mapped_by(Affine::new(-1.0, 0.0, 0.0, 1.0, 160.0, 0.0));
    d.transform_objects(&[ObjectSnapshot::Image(flipped)], false)
        .unwrap();
    d.set_node_opacity(&[ids[1]], Opacity::new(0.5).unwrap())
        .unwrap();
    d.group_objects(&[ids[2], ids[3]]).unwrap();
    d.set_node_visible(&[ids[3]], false).unwrap();
    d.set_node_locked(&[ids[4]], true).unwrap();
    d.duplicate_objects(
        &[CopySource {
            id: ids[0],
            anchor_ids: Vec::new(),
        }],
        Vec2::new(0.0, 40.0),
    )
    .unwrap();
    d
}

fn members(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut out = Vec::new();
            let name = file.name().unwrap().to_string();
            file.read_to_end(&mut out).unwrap();
            (name, out)
        })
        .collect()
}

fn rezip(members: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in members {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// The golden file with the first asset (the PNG of the duplicate) changed by `damage`.
fn damaged(golden: &[u8], damage: usize) -> Vec<u8> {
    let png = format!(
        "assets/{}.png",
        AssetHash::of(&picture("png_rgba_8.png")).as_str()
    );
    let mut out = Vec::new();
    for (name, bytes) in members(golden) {
        if name != png {
            out.push((name, bytes));
            continue;
        }
        match damage {
            0 => {}
            1 => out.push((name, picture("png_gray_16.png").to_vec())),
            2 => out.push((name, b"GIF89a not a supported picture".to_vec())),
            _ => out.push((name, Vec::new())),
        }
    }
    rezip(&out)
}

fn images(d: &Document) -> Vec<curvyo_document_core::ImageSnapshot> {
    d.read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Image(i) => Some(i.clone()),
            _ => None,
        })
        .collect()
}

fn manifest_version(bytes: &[u8]) -> u32 {
    let manifest = members(bytes)
        .into_iter()
        .find(|(n, _)| n == "manifest.json")
        .unwrap()
        .1;
    let value: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    u32::try_from(value["format_version"].as_u64().unwrap()).unwrap()
}

#[test]
fn ac44_the_golden_project_opens_with_every_image_as_saved() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= IMAGE_FORMAT_VERSION);
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        let golden = pack(&golden_document(), "0.1.0").unwrap();
        std::fs::write(fixture_path(GOLDEN), &golden).unwrap();
        for (i, name) in DAMAGED.iter().enumerate() {
            std::fs::write(fixture_path(name), damaged(&golden, i)).unwrap();
        }
    }
    let stored = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&stored), IMAGE_FORMAT_VERSION);
    let opened = unpack(2, &stored).unwrap();
    let expected = golden_document();
    assert_eq!(images(&opened), images(&expected));
    let reads = opened.read_nodes();
    assert_eq!(
        reads
            .iter()
            .filter(|r| matches!(r.kind, NodeKind::Group))
            .count(),
        1
    );
    assert_eq!(reads.iter().filter(|r| !r.registers.visible).count(), 1);
    assert_eq!(reads.iter().filter(|r| r.registers.locked).count(), 1);
    assert_eq!(
        reads
            .iter()
            .filter(|r| (r.registers.opacity.get() - 0.5).abs() < 1e-12)
            .count(),
        1
    );
    let assets = members(&stored)
        .into_iter()
        .filter(|(n, _)| n.starts_with("assets/"))
        .count();
    assert_eq!(assets, 5, "one asset per distinct file");
    for image in images(&opened) {
        assert_eq!(
            AssetHash::of(&opened.asset(&image.asset).unwrap()),
            image.asset
        );
    }
}

#[test]
fn ac44_damaged_assets_open_with_every_frame_kept() {
    for name in DAMAGED {
        let opened = unpack(3, &std::fs::read(fixture_path(name)).unwrap()).unwrap();
        assert_eq!(images(&opened).len(), 6, "{name}");
        let png = AssetHash::of(&picture("png_rgba_8.png"));
        let held = opened.asset(&png);
        let readable = held
            .as_deref()
            .is_some_and(|bytes| AssetHash::of(bytes) == png);
        assert!(!readable, "{name}: the damaged asset is not the picture");
    }
}
