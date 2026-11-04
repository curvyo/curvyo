//! The machine profile copy in the project file (`specs/0026-machine-profile` criteria 5, 6, 8 and
//! 12; ADR 0039): the `mfg_profile` register, its one-step writes, the migration of format 35, the
//! open check and the goldens `machine_profile_v36.curvyo`, `machine_profile_damaged_v36.curvyo`
//! and `mfg_profile_not_a_map_v36.curvyo`. `CURVYO_WRITE_FIXTURES=1` rewrites them.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewManufacturing, NodeId,
    OpenError, Point, ProfileChange, RectBounds, pack, unpack,
};
use loro::{LoroDoc, LoroValue};

const THIS_SLICE_VERSION: u32 = 36;
const GOLDEN: &str = "machine_profile_v36.curvyo";
const DAMAGED: &str = "machine_profile_damaged_v36.curvyo";
const NOT_A_MAP: &str = "mfg_profile_not_a_map_v36.curvyo";
const USER_ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn copy(id: &str, name: &str, feed: u32) -> FixtureParams {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": name,
        "origin": "top_right",
        "max_feed_mm_per_min": feed,
        "max_power": 255,
        "laser_mode": false,
    }))
    .unwrap()
}

fn with_laser(profile: &FixtureParams) -> (Document, NodeId) {
    let document = Document::new(1);
    let _ = document.create_rect(RectBounds::from_corners(
        Point::new(20.0, 20.0),
        Point::new(30.0, 30.0),
    ));
    let id = document
        .add_manufacturing(NewManufacturing {
            process: "curvyo.laser",
            label: "Laser",
            profile: Some(profile),
            work_area: DocumentSize::from_mm(400.0, 300.0),
            place: Some("first"),
        })
        .unwrap();
    (document, id)
}

/// A user profile id, an edited copy and a 600 x 400 mm output page (criterion 12).
fn golden_document() -> Document {
    let (document, id) = with_laser(&copy(USER_ID, "K40 in the garage", 6000));
    document
        .set_manufacturing_profile(
            id,
            &copy(USER_ID, "K40 in the garage", 12_000),
            DocumentSize::from_mm(600.0, 400.0),
            ProfileChange::EditMaxFeed,
        )
        .unwrap();
    document
}

/// The golden with `mfg_profile` replaced by `value`, saved as this build's container.
fn with_profile_value(value: LoroValue) -> Vec<u8> {
    let loro = LoroDoc::new();
    loro.import(&golden_document().export_loro_snapshot().unwrap())
        .unwrap();
    manufacturing_meta(&loro)
        .insert("mfg_profile", value)
        .unwrap();
    loro.commit();
    repack(&loro)
}

fn damaged_profile() -> LoroValue {
    let map: std::collections::HashMap<String, LoroValue> = [
        ("id".to_owned(), LoroValue::from(USER_ID)),
        ("name".to_owned(), LoroValue::from("n".repeat(61).as_str())),
        ("origin".to_owned(), LoroValue::from("top_right")),
        (
            "max_feed_mm_per_min".to_owned(),
            LoroValue::Double(f64::NAN),
        ),
        ("max_power".to_owned(), LoroValue::from("high")),
        ("laser_mode".to_owned(), LoroValue::Bool(false)),
    ]
    .into();
    LoroValue::from(map)
}

fn manufacturing_meta(loro: &LoroDoc) -> loro::LoroMap {
    let tree = loro.get_tree("paths");
    let node = tree
        .children(loro::TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
        .unwrap();
    tree.get_meta(node).unwrap()
}

fn repack(loro: &LoroDoc) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": curvyo_document_core::CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "0.1.0",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("manifest.json", options).unwrap();
    writer.write_all(manifest.to_string().as_bytes()).unwrap();
    writer.start_file("document.loro", options).unwrap();
    writer
        .write_all(&loro.export(loro::ExportMode::Snapshot).unwrap())
        .unwrap();
    writer.start_file("document.json", options).unwrap();
    writer.write_all(b"{}").unwrap();
    writer.finish().unwrap().into_inner()
}

fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
        std::fs::write(fixture_path(DAMAGED), with_profile_value(damaged_profile())).unwrap();
        std::fs::write(
            fixture_path(NOT_A_MAP),
            with_profile_value(LoroValue::from("K40")),
        )
        .unwrap();
    }
}

fn stored_loro(bytes: &[u8]) -> LoroDoc {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut stored = Vec::new();
    archive
        .by_name("document.loro")
        .unwrap()
        .read_to_end(&mut stored)
        .unwrap();
    let loro = LoroDoc::new();
    loro.import(&stored).unwrap();
    loro
}

fn stored_profile(bytes: &[u8]) -> LoroValue {
    manufacturing_meta(&stored_loro(bytes))
        .get("mfg_profile")
        .unwrap()
        .get_deep_value()
}

#[test]
fn the_format_version_is_this_slices() {
    assert_eq!(CURRENT_FORMAT_VERSION, THIS_SLICE_VERSION);
}

#[test]
fn the_golden_reads_back_id_copy_and_work_area_identical() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    let document = unpack(2, &bytes).unwrap();
    let version = document.version();
    let made = &document.manufacturings()[0];
    assert_eq!(
        made.profile,
        Some(copy(USER_ID, "K40 in the garage", 12_000))
    );
    assert_eq!(made.work_area(), DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(made.legacy_machine_name, None);
    assert_eq!(document.version(), version, "opening writes nothing");
    let LoroValue::Map(stored) = stored_profile(&bytes) else {
        panic!("one atomic map");
    };
    assert_eq!(
        stored.get("max_feed_mm_per_min"),
        Some(&LoroValue::I64(12_000))
    );
    let again = unpack(3, &pack(&document, "0.1.0").unwrap()).unwrap();
    assert_eq!(again.manufacturings(), document.manufacturings());
}

#[test]
fn a_damaged_copy_opens_and_saves_its_register_back_unchanged() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(DAMAGED)).unwrap();
    let document = unpack(2, &bytes).unwrap();
    let profile = document.manufacturings()[0].profile.clone().unwrap();
    assert_eq!(profile["max_feed_mm_per_min"], serde_json::Value::Null);
    assert_eq!(profile["max_power"], "high");
    let saved = pack(&document, "0.1.0").unwrap();
    assert_eq!(
        format!("{:?}", stored_profile(&saved)),
        format!("{:?}", stored_profile(&bytes)),
        "never rewritten on read"
    );
}

#[test]
fn a_profile_register_that_is_not_a_map_is_damage() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(NOT_A_MAP)).unwrap();
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
}

#[test]
fn a_profile_write_is_one_step_with_its_page_and_an_unchanged_one_writes_nothing() {
    let first = copy(USER_ID, "K40", 6000);
    let (document, id) = with_laser(&first);
    let steps = document.own_step_count();
    let other = copy("0f0e0d0c-0b0a-4908-8706-050403020100", "Ortur", 3000);
    assert!(
        document
            .set_manufacturing_profile(
                id,
                &other,
                DocumentSize::from_mm(500.0, 500.0),
                ProfileChange::Pick
            )
            .unwrap()
    );
    assert_eq!(document.own_step_count(), steps + 1);
    assert_eq!(document.own_steps().last().unwrap().label, "pick_profile");
    let made = document.manufacturing(id).unwrap();
    assert_eq!(made.profile, Some(other.clone()));
    assert_eq!(made.work_area(), DocumentSize::from_mm(500.0, 500.0));
    assert!(
        !document
            .set_manufacturing_profile(
                id,
                &other,
                DocumentSize::from_mm(500.0, 500.0),
                ProfileChange::Pick
            )
            .unwrap()
    );
    assert_eq!(document.own_step_count(), steps + 1);
}

#[test]
fn the_first_profile_write_of_a_format_35_file_drops_the_placeholder_name() {
    let bytes = std::fs::read(fixture_path("laser_workbench_v35.curvyo")).unwrap();
    let document = unpack(2, &bytes).unwrap();
    let made = document.manufacturings()[0].clone();
    assert_eq!(made.profile, None);
    let profile = copy("generic-laser", "Generic laser", 6000);
    document
        .set_manufacturing_profile(
            made.id,
            &profile,
            made.work_area(),
            ProfileChange::EditMaxFeed,
        )
        .unwrap();
    let after = document.manufacturing(made.id).unwrap();
    assert_eq!(after.profile, Some(profile));
    assert_eq!(after.legacy_machine_name, None);
}
