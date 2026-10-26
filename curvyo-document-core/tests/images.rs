//! Image objects and the asset store (`specs/0058-bitmap-import` criteria 13 to 15, 23, 25, 38,
//! 44, 45; ADR 0017 §1 to §3).

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::io::{Cursor, Read};
use std::sync::Arc;

use curvyo_document_core::{
    Affine, AssetError, AssetHash, CloneError, CopySource, Document, INITIAL_LAYER_NAME,
    ImageInsertError, MAX_PROJECT_ASSET_BYTES, NewImage, NodeKind, ObjectSnapshot, Vec2, pack,
    unpack,
};

const PNG: &[u8] = include_bytes!("../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");
const JPEG: &[u8] =
    include_bytes!("../../curvyo-image-core/tests/fixtures/images/jpeg_baseline_jfif_300dpi.jpg");

fn image(bytes: &[u8], x: f64) -> NewImage {
    NewImage {
        bytes: Arc::from(bytes),
        placement: Affine::new(30.0, 0.0, 0.0, 10.0, x, 20.0),
        name: "photo.png".to_owned(),
    }
}

fn images(document: &Document) -> Vec<curvyo_document_core::ImageSnapshot> {
    document
        .read_objects()
        .objects
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Image(image) => Some(image.clone()),
            _ => None,
        })
        .collect()
}

fn members(bytes: &[u8]) -> Vec<(String, Vec<u8>, zip::CompressionMethod)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut out = Vec::new();
            let method = file.compression();
            let name = file.name().unwrap().to_string();
            file.read_to_end(&mut out).unwrap();
            (name, out, method)
        })
        .collect()
}

#[test]
fn ac15_an_import_into_an_empty_document_makes_the_initial_layer_in_one_step() {
    let document = Document::new(1);
    document.set_draw_into_initial_layer(true);
    let ids = document.add_images(&[image(PNG, 10.0)]).unwrap();
    assert_eq!(ids.len(), 1);
    assert_eq!(document.own_steps().len(), 1);
    assert_eq!(document.own_steps()[0].label, "import_image");
    let reads = document.read_nodes();
    let layer = reads
        .iter()
        .find(|r| matches!(r.kind, NodeKind::Layer))
        .unwrap();
    assert_eq!(layer.registers.name.as_deref(), Some(INITIAL_LAYER_NAME));
    let leaf = reads.iter().find(|r| r.id == ids[0]).unwrap();
    assert_eq!(leaf.parent, Some(layer.id));
    assert_eq!(leaf.registers.name.as_deref(), Some("photo.png"));
    let step = document.own_steps()[0].step;
    document.undo_step(step).unwrap();
    assert!(
        document.read_nodes().is_empty(),
        "undo removes the image and the layer"
    );
}

#[test]
fn ac14_images_go_on_top_and_several_are_one_step_in_order() {
    let document = Document::new(1);
    let _ = document.create_rect(curvyo_document_core::RectBounds::from_corners(
        curvyo_document_core::Point::new(0.0, 0.0),
        curvyo_document_core::Point::new(5.0, 5.0),
    ));
    let ids = document
        .add_images(&[image(PNG, 0.0), image(JPEG, 10.0)])
        .unwrap();
    assert_eq!(document.own_steps().last().unwrap().label, "import_images");
    let order = document.object_ids();
    assert_eq!(&order[1..], &ids[..], "on top, in picker order");
}

#[test]
fn ac23_the_same_bytes_are_one_asset_and_duplicate_shares_it() {
    let document = Document::new(1);
    let first = document.add_images(&[image(PNG, 0.0)]).unwrap()[0];
    document.add_images(&[image(PNG, 50.0)]).unwrap();
    document
        .duplicate_objects(
            &[CopySource {
                id: first,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(5.0, 5.0),
        )
        .unwrap();
    let all = images(&document);
    assert_eq!(all.len(), 3);
    assert!(all.iter().all(|i| i.asset == AssetHash::of(PNG)));
    assert_eq!(document.asset_bytes(), PNG.len() as u64);
    let bytes = pack(&document, "t").unwrap();
    let assets: Vec<_> = members(&bytes)
        .into_iter()
        .filter(|(name, ..)| name.starts_with("assets/"))
        .collect();
    assert_eq!(assets.len(), 1);
    assert_eq!(
        assets[0].0,
        format!("assets/{}.png", AssetHash::of(PNG).as_str())
    );
    assert_eq!(assets[0].1, PNG, "verbatim");
    assert_eq!(assets[0].2, zip::CompressionMethod::Stored);
}

#[test]
fn ac44_save_and_open_keep_frames_and_assets_and_a_second_save_is_identical() {
    let document = Document::new(1);
    let ids = document
        .add_images(&[image(PNG, 0.0), image(JPEG, 40.0)])
        .unwrap();
    let rotated = document.object(ids[1]).unwrap().rotated(
        curvyo_document_core::Point::new(50.0, 25.0),
        curvyo_document_core::Angle::from_radians(0.5),
    );
    document
        .transform_objects(std::slice::from_ref(&rotated), false)
        .unwrap();
    let saved = pack(&document, "t").unwrap();
    let opened = unpack(2, &saved).unwrap();
    assert_eq!(images(&opened), images(&document));
    assert_eq!(opened.object(ids[1]), Some(rotated));
    assert_eq!(opened.asset(&AssetHash::of(JPEG)).as_deref(), Some(JPEG));
    let again = pack(&opened, "t").unwrap();
    let first: Vec<_> = members(&saved)
        .into_iter()
        .filter(|(n, ..)| n != "document.loro")
        .collect();
    let second: Vec<_> = members(&again)
        .into_iter()
        .filter(|(n, ..)| n != "document.loro")
        .collect();
    assert_eq!(first, second);
    let json = String::from_utf8(opened.export_json().unwrap()).unwrap();
    assert!(
        json.contains(AssetHash::of(PNG).as_str()) && json.contains("\"image\""),
        "{json}"
    );
}

#[test]
fn ac25_only_the_history_wipe_prunes_an_asset_nothing_names() {
    let mut document = Document::new(1);
    let id = document.add_images(&[image(PNG, 0.0)]).unwrap()[0];
    document.delete_objects(&[id]).unwrap();
    let kept = pack(&document, "t").unwrap();
    assert!(
        members(&kept)
            .iter()
            .any(|(n, ..)| n.starts_with("assets/"))
    );
    document.wipe_history().unwrap();
    let wiped = pack(&document, "t").unwrap();
    assert!(
        !members(&wiped)
            .iter()
            .any(|(n, ..)| n.starts_with("assets/"))
    );
}

#[test]
fn ac17_the_project_cap_refuses_and_writes_nothing() {
    let document = Document::new(1);
    let big: Vec<u8> = vec![7; usize::try_from(MAX_PROJECT_ASSET_BYTES / 2 + 1).unwrap()];
    document.add_images(&[image(&big, 0.0)]).unwrap();
    let mut other = big.clone();
    other[0] = 8;
    let steps = document.own_steps().len();
    assert_eq!(
        document.add_images(&[image(&other, 0.0)]),
        Err(ImageInsertError::Assets(AssetError::ProjectFull {
            adds: other.len() as u64
        }))
    );
    assert_eq!(document.own_steps().len(), steps);
    assert_eq!(images(&document).len(), 1);
}

#[test]
fn ac38_an_image_is_never_an_original_of_a_linked_clone() {
    let document = Document::new(1);
    let id = document.add_images(&[image(PNG, 0.0)]).unwrap()[0];
    assert_eq!(
        document.linked_clone(&[id], &[]),
        Err(CloneError::NotClonable)
    );
}

#[test]
fn ac44_a_missing_asset_opens_and_keeps_the_frame() {
    let document = Document::new(1);
    document.add_images(&[image(PNG, 0.0)]).unwrap();
    let saved = pack(&document, "t").unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes, _) in members(&saved) {
        if name.starts_with("assets/") {
            continue;
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, &bytes).unwrap();
    }
    let without = writer.finish().unwrap().into_inner();
    let opened = unpack(2, &without).unwrap();
    assert_eq!(images(&opened), images(&document));
    assert!(opened.asset(&AssetHash::of(PNG)).is_none());
}
