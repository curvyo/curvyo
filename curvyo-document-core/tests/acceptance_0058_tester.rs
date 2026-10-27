//! Tester acceptance cases of `specs/0058-bitmap-import` for the model, the asset store and the
//! format-25 container (criteria 16, 17, 23, 25, 32, 38, 42, 44, 45; ADR 0017 §1 to §3). Written
//! from the specification; they cover the edges the branch's own tests leave out: the 512 MiB cap
//! at its boundary, dedup at the cap, which operations may drop an asset (only the history wipe),
//! the history across a save, and container damage beyond the four golden variants.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use curvyo_document_core::{
    Affine, AssetError, AssetHash, CURRENT_FORMAT_VERSION, CopySource, Document, ImageInsertError,
    ImageSnapshot, MAX_PROJECT_ASSET_BYTES, NewImage, NodeKind, ObjectSnapshot, OpenError, Point,
    Vec2, pack, unpack,
};

const MIB: usize = 1024 * 1024;

fn picture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images")
            .join(name),
    )
    .unwrap()
}

fn new_image(bytes: &[u8], name: &str) -> NewImage {
    NewImage {
        bytes: Arc::from(bytes),
        placement: Affine::new(40.0, 0.0, 0.0, 20.0, 10.0, 10.0),
        name: name.to_owned(),
    }
}

fn images(document: &Document) -> Vec<ImageSnapshot> {
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

fn asset_names(bytes: &[u8]) -> Vec<String> {
    members(bytes)
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| name.starts_with("assets/"))
        .collect()
}

fn rezip(members: &[(String, Vec<u8>)], deflate: bool) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(if deflate {
            zip::CompressionMethod::Deflated
        } else {
            zip::CompressionMethod::Stored
        })
        .large_file(true);
    for (name, bytes) in members {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn last_step(document: &Document) -> curvyo_document_core::StepRef {
    document.own_steps().last().unwrap().step
}

/// A distinct picture of `len` bytes: a PNG signature, then `fill`, so the bytes differ by `fill`.
fn blob(len: usize, fill: u8) -> Vec<u8> {
    let mut bytes = vec![fill; len];
    bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    bytes
}

// --- criteria 16, 17, 23: the asset store and its cap ------------------------------------------

#[test]
fn ac16_ac17_exactly_512_mib_of_pictures_fit_and_one_byte_more_is_refused() {
    let document = Document::new(1);
    let hundred = 100 * MIB;
    let files: Vec<_> = (0..5_u8)
        .map(|i| new_image(&blob(hundred, i + 1), "big.png"))
        .chain(std::iter::once(new_image(&blob(12 * MIB, 9), "rest.png")))
        .collect();
    document.add_images(&files).unwrap();
    assert_eq!(document.asset_bytes(), MAX_PROJECT_ASSET_BYTES);
    let steps = document.own_steps().len();
    let one_more = blob(8, 42);
    assert_eq!(
        document.add_images(&[new_image(&one_more, "tiny.png")]),
        Err(ImageInsertError::Assets(AssetError::ProjectFull {
            adds: 8
        }))
    );
    assert_eq!(document.own_steps().len(), steps, "no history step");
    assert_eq!(images(&document).len(), 6, "no half-made object");
    // The same bytes again add nothing, so they are not refused at the cap (criterion 23).
    document
        .add_images(&[new_image(&blob(12 * MIB, 9), "again.png")])
        .unwrap();
    assert_eq!(document.asset_bytes(), MAX_PROJECT_ASSET_BYTES);
    assert_eq!(images(&document).len(), 7);
}

#[test]
fn ac23_the_same_bytes_twice_in_one_batch_are_one_asset_and_counted_once() {
    let document = Document::new(1);
    let png = picture("png_rgb_8.png");
    document
        .add_images(&[new_image(&png, "a.png"), new_image(&png, "b.png")])
        .unwrap();
    assert_eq!(document.asset_bytes(), png.len() as u64);
    let near_cap = blob(
        usize::try_from(MAX_PROJECT_ASSET_BYTES).unwrap() / 2 - png.len(),
        3,
    );
    let fresh = Document::new(1);
    // Two copies of a picture of just under half the cap: counted once, so they fit.
    fresh
        .add_images(&[new_image(&near_cap, "x.png"), new_image(&near_cap, "y.png")])
        .unwrap();
    assert_eq!(fresh.asset_bytes(), near_cap.len() as u64);
}

#[test]
fn ac23_importing_a_2_000_000_byte_png_twice_grows_the_file_by_one_copy() {
    let mut png = picture("png_rgb_8.png");
    png.resize(2_000_000, 0x5A);
    let without = Document::new(1);
    without.set_draw_into_initial_layer(true);
    let base = pack(&without, "t").unwrap().len();
    let with = Document::new(1);
    with.set_draw_into_initial_layer(true);
    with.add_images(&[new_image(&png, "one.png")]).unwrap();
    with.add_images(&[new_image(&png, "two.png")]).unwrap();
    let saved = pack(&with, "t").unwrap();
    assert!(
        saved.len() <= base + 2_000_000 + 64 * 1024,
        "{} bytes over the empty project",
        saved.len() - base
    );
    assert_eq!(asset_names(&saved).len(), 1);
}

#[test]
fn ac2_a_placement_that_is_not_finite_or_beyond_the_limit_writes_nothing() {
    let document = Document::new(1);
    let png = picture("png_rgb_8.png");
    for placement in [
        Affine::new(f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0),
        Affine::new(1.0, 0.0, 0.0, 1.0, f64::INFINITY, 0.0),
        Affine::new(2e7, 0.0, 0.0, 1.0, 0.0, 0.0),
    ] {
        let image = NewImage {
            bytes: Arc::from(png.as_slice()),
            placement,
            name: "x.png".to_owned(),
        };
        assert_eq!(
            document.add_images(&[image]),
            Err(ImageInsertError::InvalidPlacement)
        );
    }
    assert_eq!(document.own_steps().len(), 0);
    assert_eq!(document.asset_bytes(), 0, "no orphan asset");
}

// --- criterion 25: only the history wipe drops an asset ----------------------------------------

#[test]
fn ac25_delete_then_undo_keeps_the_asset_and_brings_the_picture_back() {
    let document = Document::new(1);
    let png = picture("png_rgba_8.png");
    let id = document.add_images(&[new_image(&png, "a.png")]).unwrap()[0];
    document.delete_objects(&[id]).unwrap();
    assert_eq!(images(&document).len(), 0);
    assert_eq!(
        document.asset(&AssetHash::of(&png)).as_deref(),
        Some(&png[..])
    );
    document.undo_step(last_step(&document)).unwrap();
    let back = images(&document);
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].asset, AssetHash::of(&png));
    assert_eq!(document.asset(&back[0].asset).as_deref(), Some(&png[..]));
}

#[test]
fn ac25_ac45_undo_of_the_import_and_its_redo_need_no_bytes_again() {
    let document = Document::new(1);
    let png = picture("png_rgba_8.png");
    document.add_images(&[new_image(&png, "a.png")]).unwrap();
    let import = last_step(&document);
    document.undo_step(import).unwrap();
    assert_eq!(images(&document).len(), 0);
    assert!(
        asset_names(&pack(&document, "t").unwrap()).len() == 1,
        "history holds it"
    );
    let undo = last_step(&document);
    document.redo_step(undo).unwrap();
    let back = images(&document);
    assert_eq!(back.len(), 1);
    assert_eq!(document.asset(&back[0].asset).as_deref(), Some(&png[..]));
}

#[test]
fn ac25_deleting_the_layer_or_wiping_the_object_history_keeps_the_asset() {
    let document = Document::new(1);
    document.set_draw_into_initial_layer(true);
    let png = picture("png_rgba_8.png");
    let id = document.add_images(&[new_image(&png, "a.png")]).unwrap()[0];
    document.wipe_object_history(id).unwrap();
    assert!(
        document.asset(&AssetHash::of(&png)).is_some(),
        "object wipe"
    );
    let layer = document
        .read_nodes()
        .into_iter()
        .find(|r| matches!(r.kind, NodeKind::Layer))
        .unwrap()
        .id;
    document.delete_layer(layer).unwrap();
    assert_eq!(images(&document).len(), 0);
    assert_eq!(
        asset_names(&pack(&document, "t").unwrap()).len(),
        1,
        "layer delete"
    );
}

#[test]
fn ac25_the_history_wipe_keeps_assets_still_named_by_a_duplicate_or_a_hidden_grouped_image() {
    let mut document = Document::new(1);
    let png = picture("png_rgba_8.png");
    let jpeg = picture("jpeg_gray.jpg");
    let webp = picture("webp_lossy.webp");
    let ids = document
        .add_images(&[
            new_image(&png, "a.png"),
            new_image(&jpeg, "b.jpg"),
            new_image(&webp, "c.webp"),
        ])
        .unwrap();
    document
        .duplicate_objects(
            &[CopySource {
                id: ids[0],
                anchor_ids: Vec::new(),
            }],
            Vec2::new(5.0, 5.0),
        )
        .unwrap();
    document.delete_objects(&[ids[0]]).unwrap(); // the duplicate still names the PNG
    let group = document.group_objects(&[ids[1], ids[2]]).unwrap();
    document.set_node_visible(&[group], false).unwrap();
    document.delete_objects(&[ids[2]]).unwrap(); // nothing names the WebP any more
    document.wipe_history().unwrap();
    let saved = pack(&document, "t").unwrap();
    let names = asset_names(&saved);
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(document.asset(&AssetHash::of(&png)).is_some());
    assert!(document.asset(&AssetHash::of(&jpeg)).is_some());
    assert!(document.asset(&AssetHash::of(&webp)).is_none());
}

#[test]
fn ac25_ac44_a_saved_project_keeps_history_only_assets_so_undo_after_open_shows_the_picture() {
    let document = Document::new(1);
    let png = picture("png_rgba_8.png");
    let id = document.add_images(&[new_image(&png, "a.png")]).unwrap()[0];
    document.delete_objects(&[id]).unwrap();
    let opened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    assert_eq!(images(&opened).len(), 0);
    assert_eq!(
        opened.asset(&AssetHash::of(&png)).as_deref(),
        Some(&png[..])
    );
    // The delete step was saved with the history; undo it in the opened project.
    let delete = last_step(&document);
    opened.undo_step(delete).unwrap();
    assert_eq!(images(&opened).len(), 1);
    assert_eq!(
        opened.asset(&images(&opened)[0].asset).as_deref(),
        Some(&png[..])
    );
}

// --- criteria 32, 42: the frame and the pixel map ----------------------------------------------

#[test]
fn ac42_the_pixel_map_after_a_quarter_turn_and_a_flip_matches_the_hand_computed_affine() {
    // 4 x 2 px in a 40 x 20 mm frame at (10, 10); turned 90 degrees clockwise on screen about its
    // centre (30, 20), then flipped horizontally about x = 30.
    let document = Document::new(1);
    let id = document
        .add_images(&[new_image(&picture("png_rgb_8.png"), "a.png")])
        .unwrap()[0];
    let ObjectSnapshot::Image(image) = document.object(id).unwrap() else {
        panic!("an image")
    };
    let turn = Affine::new(0.0, 1.0, -1.0, 0.0, 50.0, -10.0);
    let flip = Affine::new(-1.0, 0.0, 0.0, 1.0, 60.0, 0.0);
    let mapped = image.mapped_by(turn).mapped_by(flip);
    document
        .transform_objects(&[ObjectSnapshot::Image(mapped)], false)
        .unwrap();
    let ObjectSnapshot::Image(stored) = document.object(id).unwrap() else {
        panic!("still an image after a turn and a flip")
    };
    let map = stored.pixel_map(4, 2);
    for ((px, py), (x, y)) in [
        ((3.0, 1.0), (30.0, 30.0)),
        ((0.0, 0.0), (20.0, 0.0)),
        ((4.0, 2.0), (40.0, 40.0)),
        ((4.0, 0.0), (20.0, 40.0)),
    ] {
        let p = map.apply_point(Point::new(px, py));
        assert!(
            (p.x - x).abs() < 1e-6 && (p.y - y).abs() < 1e-6,
            "pixel ({px}, {py}) at ({}, {}), expected ({x}, {y})",
            p.x,
            p.y
        );
    }
}

#[test]
fn ac32_a_skew_and_a_negative_scale_stay_an_image_with_exactly_mapped_corners() {
    let document = Document::new(1);
    let id = document
        .add_images(&[new_image(&picture("png_rgb_8.png"), "a.png")])
        .unwrap()[0];
    let ObjectSnapshot::Image(image) = document.object(id).unwrap() else {
        panic!("an image")
    };
    let gesture = Affine::new(-1.5, 0.25, 0.4, 2.0, 7.0, -3.0);
    let before = image.corners();
    document
        .transform_objects(&[ObjectSnapshot::Image(image.mapped_by(gesture))], false)
        .unwrap();
    let saved = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    let ObjectSnapshot::Image(after) = saved.object(id).unwrap() else {
        panic!("a skew must not convert an image")
    };
    for (old, new) in before.iter().zip(after.corners()) {
        let want = gesture.apply_point(*old);
        assert!(
            (want.x - new.x).abs() < 1e-6 && (want.y - new.y).abs() < 1e-6,
            "{want:?} vs {new:?}"
        );
    }
}

// --- criterion 38: clones ----------------------------------------------------------------------

#[test]
fn ac38_a_group_holding_an_image_is_not_a_way_around_the_clone_refusal() {
    // "Linked clone is not offered for a selection that contains an image": a selected group
    // with an image inside contains one.
    let document = Document::new(1);
    let id = document
        .add_images(&[new_image(&picture("png_rgb_8.png"), "a.png")])
        .unwrap()[0];
    let rect = document.create_rect(curvyo_document_core::RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(5.0, 5.0),
    ));
    let group = document.group_objects(&[id, rect]).unwrap();
    assert!(document.linked_clone(&[group], &[]).is_err());
}

// --- criterion 44: format 25 and damaged containers --------------------------------------------

fn saved_with_one_png() -> (Vec<u8>, Vec<u8>) {
    let document = Document::new(1);
    let png = picture("png_rgba_8.png");
    document.add_images(&[new_image(&png, "a.png")]).unwrap();
    (pack(&document, "t").unwrap(), png)
}

#[test]
fn ac44_a_newer_format_version_is_refused_as_saved_by_a_newer_version() {
    let (saved, _) = saved_with_one_png();
    let newer: Vec<_> = members(&saved)
        .into_iter()
        .map(|(name, bytes)| {
            if name == "manifest.json" {
                let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                value["format_version"] = serde_json::json!(CURRENT_FORMAT_VERSION + 1);
                (name, serde_json::to_vec(&value).unwrap())
            } else {
                (name, bytes)
            }
        })
        .collect();
    assert!(matches!(
        unpack(2, &rezip(&newer, false)),
        Err(OpenError::FormatTooNew { found, supported })
            if found == CURRENT_FORMAT_VERSION + 1 && supported == CURRENT_FORMAT_VERSION
    ));
}

#[test]
fn ac44_an_older_project_without_images_opens_and_saves_without_an_assets_folder() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/groups_v11.curvyo");
    let opened = unpack(2, &std::fs::read(path).unwrap()).unwrap();
    assert_eq!(images(&opened).len(), 0);
    assert_eq!(asset_names(&pack(&opened, "t").unwrap()).len(), 0);
}

#[test]
fn ac44_a_truncated_asset_opens_with_the_frame_kept_and_the_bytes_not_trusted() {
    let (saved, png) = saved_with_one_png();
    let cut: Vec<_> = members(&saved)
        .into_iter()
        .map(|(name, bytes)| {
            if name.starts_with("assets/") {
                let half = bytes.len() / 2;
                (name, bytes[..half].to_vec())
            } else {
                (name, bytes)
            }
        })
        .collect();
    let opened = unpack(2, &rezip(&cut, false)).unwrap();
    assert_eq!(images(&opened).len(), 1);
    let hash = AssetHash::of(&png);
    let held = opened.asset(&hash);
    assert!(
        held.as_deref()
            .is_none_or(|bytes| AssetHash::of(bytes) != hash),
        "a truncated asset is never handed out as the picture"
    );
}

/// `saved` with the stored size of every `assets/` entry in the local header and the central
/// directory set to `size`.
fn with_asset_size(saved: &[u8], size: u32) -> Vec<u8> {
    let mut out = saved.to_vec();
    let mut at = 0;
    while at + 46 < out.len() {
        let signature = &out[at..at + 4];
        if signature == b"PK\x03\x04" {
            let name_len = usize::from(u16::from_le_bytes([out[at + 26], out[at + 27]]));
            if out[at + 30..at + 30 + name_len].starts_with(b"assets/") {
                out[at + 18..at + 22].copy_from_slice(&size.to_le_bytes());
                out[at + 22..at + 26].copy_from_slice(&size.to_le_bytes());
            }
        } else if signature == b"PK\x01\x02" {
            let name_len = usize::from(u16::from_le_bytes([out[at + 28], out[at + 29]]));
            if out[at + 46..at + 46 + name_len].starts_with(b"assets/") {
                out[at + 20..at + 24].copy_from_slice(&size.to_le_bytes());
                out[at + 24..at + 28].copy_from_slice(&size.to_le_bytes());
            }
        }
        at += 1;
    }
    out
}

#[test]
fn ac44_an_asset_entry_with_a_wrong_length_opens_with_the_frame_kept() {
    let (saved, png) = saved_with_one_png();
    let hash = AssetHash::of(&png);
    for size in [1, u32::try_from(png.len() + 64).unwrap(), 0xFFFF_FFF0] {
        let damaged = with_asset_size(&saved, size);
        let opened = unpack(2, &damaged).unwrap_or_else(|e| panic!("size {size}: {e:?}"));
        assert_eq!(images(&opened).len(), 1, "size {size}");
        let held = opened.asset(&hash);
        assert!(
            held.as_deref()
                .is_none_or(|bytes| AssetHash::of(bytes) != hash),
            "size {size}: wrong bytes handed out as the picture"
        );
    }
}

#[test]
fn ac44_a_deflated_asset_that_inflates_past_100_mib_is_cut_off_not_read_whole() {
    let (saved, png) = saved_with_one_png();
    let name = format!("assets/{}.png", AssetHash::of(&png).as_str());
    let mut bomb = png.clone();
    bomb.resize(160 * MIB, 0);
    let mut parts: Vec<_> = members(&saved)
        .into_iter()
        .filter(|(n, _)| *n != name)
        .collect();
    parts.push((name, bomb));
    let damaged = rezip(&parts, true);
    assert!(
        damaged.len() < 2 * MIB,
        "a deflate bomb: {} bytes",
        damaged.len()
    );
    let opened = unpack(2, &damaged).unwrap();
    assert_eq!(images(&opened).len(), 1);
    assert!(
        opened
            .asset(&AssetHash::of(&png))
            .is_none_or(|bytes| bytes.len() <= 100 * MIB),
        "the read is bounded by the file ceiling"
    );
}

#[test]
fn ac44_asset_names_that_are_not_a_hash_are_ignored() {
    let (saved, png) = saved_with_one_png();
    let mut parts = members(&saved);
    for odd in [
        "assets/../../escape.png",
        "assets/ABCDEF.png",
        "assets/",
        "assets/0000000000000000000000000000000000000000000000000000000000000000.png",
    ] {
        parts.push((odd.to_owned(), png.clone()));
    }
    let opened = unpack(2, &rezip(&parts, false)).unwrap();
    assert_eq!(images(&opened).len(), 1);
    assert_eq!(
        opened.asset(&AssetHash::of(&png)).as_deref(),
        Some(&png[..])
    );
    let names = asset_names(&pack(&opened, "t").unwrap());
    assert!(
        names
            .iter()
            .all(|n| !n.contains("..") && !n.contains("ABCDEF") && n.len() > "assets/".len()),
        "{names:?}"
    );
}

#[test]
fn ac44_document_json_names_the_hash_and_placement_but_carries_no_bytes() {
    let document = Document::new(1);
    let mut big = picture("png_rgb_8.png");
    big.resize(300_000, 1);
    document.add_images(&[new_image(&big, "a.png")]).unwrap();
    let json = document.export_json().unwrap();
    assert!(json.len() < 20_000, "document.json is {} bytes", json.len());
    let text = String::from_utf8(json).unwrap();
    assert!(text.contains(AssetHash::of(&big).as_str()));
}
