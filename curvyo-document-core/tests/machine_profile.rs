//! The machine profile copy in the project file (`specs/0026-machine-profile` criteria 5, 6, 8 and
//! 12; ADR 0039): the `mfg_profile` register, its one-step writes, the migration of format 35, the
//! open check and the frozen goldens `machine_profile_v36.curvyo`,
//! `machine_profile_damaged_v36.curvyo` and `mfg_profile_not_a_map_v36.curvyo`, which open
//! through the page-kinds migration (`specs/0098-page-kinds` criterion 19).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewOutputPage, NodeId,
    OpenError, PageCorner, Point, ProfileChange, RectBounds, pack, unpack,
};
use loro::{LoroDoc, LoroValue};

/// The format of the machine profile copy; later formats keep it.
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
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: Some(profile),
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap()
        .kind
        .manufacturing()
        .unwrap();
    (document, id)
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

// The copy arrived with format 36; later formats keep it.
const _: () = assert!(CURRENT_FORMAT_VERSION >= THIS_SLICE_VERSION);

#[test]
fn the_golden_reads_back_id_copy_and_work_area_identical() {
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    let document = unpack(2, &bytes).unwrap();
    let made = &document.manufacturings()[0];
    assert_eq!(
        made.profile,
        Some(copy(USER_ID, "K40 in the garage", 12_000))
    );
    assert_eq!(made.work_area(), DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(made.legacy_machine_name, None);
    assert_eq!(document.own_step_count(), 0, "opening adds no step");
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
                PageCorner::default(),
                ProfileChange::Pick,
                &curvyo_document_core::PageContents::new(),
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
                PageCorner::default(),
                ProfileChange::Pick,
                &curvyo_document_core::PageContents::new(),
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
            PageCorner::default(),
            ProfileChange::EditMaxFeed,
            &curvyo_document_core::PageContents::new(),
        )
        .unwrap();
    let after = document.manufacturing(made.id).unwrap();
    assert_eq!(after.profile, Some(profile));
    assert_eq!(after.legacy_machine_name, None);
}
