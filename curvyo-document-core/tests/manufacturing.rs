//! Manufacturings in the document model (`specs/0093-laser-workbench-shell` criteria 2, 3, 4, 5,
//! 9, 10, 12, 13; ADR 0032, ADR 0033): the commands and their undo, the skip predicate per
//! consumer, the output page out of the drawing's page list, open validation and the goldens
//! `laser_workbench_v36.curvyo` and the frozen format-35 `laser_workbench_v35.curvyo` and
//! `manufacturing_unknown_process_v35.curvyo`, which open by the migration of ADR 0037 §6.
//! `CURVYO_WRITE_FIXTURES=1` rewrites `laser_workbench_v36.curvyo`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewManufacturing,
    NodeId, OpenError, Point, RectBounds, StepRef, pack, unpack,
};
use loro::{LoroDoc, LoroValue};

const GOLDEN: &str = "laser_workbench_v36.curvyo";
const GOLDEN_V35: &str = "laser_workbench_v35.curvyo";
const UNKNOWN: &str = "manufacturing_unknown_process_v35.curvyo";
const GRID: &str = "curvyo.laser/engraved_grid";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// The copy of the built-in Generic laser as the laser extension writes it.
fn generic_copy() -> &'static FixtureParams {
    static COPY: std::sync::OnceLock<FixtureParams> = std::sync::OnceLock::new();
    COPY.get_or_init(|| {
        serde_json::from_value(serde_json::json!({
            "id": "generic-laser",
            "name": "Generic laser",
            "origin": "bottom_left",
            "max_feed_mm_per_min": 6000,
            "max_power": 1000,
            "laser_mode": true,
        }))
        .unwrap()
    })
}

fn laser(place: Option<&str>) -> NewManufacturing<'_> {
    NewManufacturing {
        process: "curvyo.laser",
        label: "Laser",
        profile: Some(generic_copy()),
        work_area: DocumentSize::from_mm(400.0, 300.0),
        place,
    }
}

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn last_step(document: &Document) -> StepRef {
    document.own_steps().last().unwrap().step
}

fn last_label(document: &Document) -> String {
    document.own_steps().last().unwrap().label.clone()
}

fn json(document: &Document) -> serde_json::Value {
    serde_json::from_slice(&document.export_json().unwrap()).unwrap()
}

fn grid_params(spacing: f64) -> FixtureParams {
    serde_json::from_value(serde_json::json!({
        "spacing": {"mm": spacing},
        "line_width": {"mm": 0.1},
        "origin_x": {"mm": 0.0},
        "origin_y": {"mm": 0.0},
    }))
    .unwrap()
}

/// A drawing with one rectangle and a laser manufacturing placing the first page.
fn with_laser() -> (Document, NodeId, NodeId) {
    let document = Document::new(1);
    let object = rect(&document, 20.0);
    let first = document.pages()[0].id.clone();
    let id = document.add_manufacturing(laser(Some(&first))).unwrap();
    (document, object, id)
}

// ---- criterion 2, 3, 5: add ------------------------------------------------------------

#[test]
fn add_manufacturing_makes_laser_1_with_its_output_page_and_a_placement_in_one_step() {
    let document = Document::new(1);
    let _ = rect(&document, 20.0);
    let steps = document.own_step_count();
    let first = document.pages()[0].id.clone();
    let id = document.add_manufacturing(laser(Some(&first))).unwrap();
    assert_eq!(document.own_step_count(), steps + 1);
    assert_eq!(last_label(&document), "add_manufacturing");
    let made = document.manufacturing(id).unwrap();
    assert_eq!(made.name, "Laser 1");
    assert_eq!(made.process, "curvyo.laser");
    assert_eq!(made.profile.as_ref(), Some(generic_copy()));
    let page = made.page.as_ref().unwrap();
    assert_eq!(made.work_area(), DocumentSize::from_mm(400.0, 300.0));
    assert_eq!(page.origin, Point::new(0.0, 0.0));
    assert_eq!(made.placements.len(), 1);
    assert_eq!(made.placements[0].page, first);
    assert_eq!(made.placements[0].matrix, Affine::IDENTITY);
    assert!(
        made.fixtures.is_empty(),
        "a new manufacturing has no fixture"
    );
    let second = document.add_manufacturing(laser(None)).unwrap();
    assert_eq!(document.manufacturing(second).unwrap().name, "Laser 2");
}

#[test]
fn undo_of_add_manufacturing_removes_it_and_its_output_page() {
    let document = Document::new(1);
    let before = json(&document);
    let id = document.add_manufacturing(laser(Some("first"))).unwrap();
    let step = last_step(&document);
    document.undo_step(step).unwrap();
    assert!(document.manufacturing(id).is_none());
    assert_eq!(document.manufacturings(), Vec::new());
    assert_eq!(json(&document), before);
}

#[test]
fn the_output_page_is_not_a_drawing_page() {
    let (document, _, _) = with_laser();
    let pages = document.pages();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].id, "first");
    assert_eq!(json(&document)["pages"].as_array().unwrap().len(), 1);
}

#[test]
fn a_work_area_edit_resizes_the_output_page_in_one_step_and_keeps_the_placement() {
    let (document, _, id) = with_laser();
    let steps = document.own_step_count();
    assert!(
        document
            .set_work_area(id, DocumentSize::from_mm(600.0, 400.0))
            .unwrap()
    );
    assert_eq!(document.own_step_count(), steps + 1);
    assert_eq!(last_label(&document), "set_work_area");
    let made = document.manufacturing(id).unwrap();
    assert_eq!(made.work_area(), DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(made.placements[0].matrix, Affine::IDENTITY);
    assert!(
        !document
            .set_work_area(id, DocumentSize::from_mm(600.0, 400.0))
            .unwrap()
    );
    assert!(
        document
            .set_work_area(id, DocumentSize::from_mm(0.0, 400.0))
            .is_err()
    );
    document.undo_step(last_step(&document)).unwrap();
    assert_eq!(
        document.manufacturing(id).unwrap().work_area(),
        DocumentSize::from_mm(400.0, 300.0)
    );
}

// ---- criterion 4, 10: the skip predicate, one test per consumer --------------------------

#[test]
fn layers_rows_do_not_list_the_manufacturing() {
    let (document, object, id) = with_laser();
    assert_eq!(document.children(None), vec![object]);
    assert!(document.read_nodes().iter().all(|read| read.id != id));
}

#[test]
fn the_drawing_read_for_hit_test_marquee_select_all_and_page_membership_skips_it() {
    let (document, object, id) = with_laser();
    let fixture = document
        .add_fixture(id, GRID, "Engraved grid", &grid_params(10.0))
        .unwrap();
    let read = document.read_objects();
    let ids: Vec<NodeId> = read
        .objects
        .iter()
        .map(curvyo_document_core::ObjectSnapshot::id)
        .collect();
    assert_eq!(ids, vec![object]);
    assert_eq!(document.object_ids(), vec![object]);
    assert!(document.is_in_manufacturing(id));
    assert!(document.is_in_manufacturing(fixture));
    assert!(!document.is_in_manufacturing(object));
}

#[test]
fn the_drawing_export_lists_no_manufacturing_object() {
    let (document, _, id) = with_laser();
    document
        .add_fixture(id, GRID, "Engraved grid", &grid_params(10.0))
        .unwrap();
    let json = json(&document);
    assert_eq!(json["objects"].as_array().unwrap().len(), 1);
    assert_eq!(json["manufacturings"][0]["name"], "Laser 1");
    assert_eq!(json["manufacturings"][0]["fixtures"][0]["kind"], GRID);
}

#[test]
fn repairs_leave_the_manufacturing_alone() {
    let (document, object, id) = with_laser();
    let before = document.manufacturing(id).unwrap();
    // Deleting the last drawing object sweeps emptied groups and repairs clones; neither
    // touches the manufacturing.
    document.delete_objects(&[object]).unwrap();
    assert_eq!(document.manufacturing(id).unwrap(), before);
}

// ---- criterion 9, 10: fixtures ----------------------------------------------------------

#[test]
fn fixtures_are_added_hidden_edited_and_removed_one_step_each() {
    let (document, _, id) = with_laser();
    let steps = document.own_step_count();
    let first = document
        .add_fixture(id, GRID, "Engraved grid", &grid_params(10.0))
        .unwrap();
    let second = document
        .add_fixture(id, GRID, "Engraved grid", &grid_params(10.0))
        .unwrap();
    let names: Vec<String> = document
        .manufacturing(id)
        .unwrap()
        .fixtures
        .into_iter()
        .map(|f| f.name)
        .collect();
    assert_eq!(names, vec!["Engraved grid", "Engraved grid 2"]);
    assert!(document.set_fixture_visible(second, false).unwrap());
    assert!(!document.set_fixture_visible(second, false).unwrap());
    assert!(
        document
            .set_fixture_params(first, &grid_params(5.0))
            .unwrap()
    );
    assert!(
        !document
            .set_fixture_params(first, &grid_params(5.0))
            .unwrap()
    );
    let made = document.manufacturing(id).unwrap();
    assert!(!made.fixtures[1].visible);
    assert_eq!(made.fixtures[0].params, grid_params(5.0));
    document.delete_fixture(first).unwrap();
    assert_eq!(document.own_step_count(), steps + 5);
    assert_eq!(document.manufacturing(id).unwrap().fixtures.len(), 1);
    assert!(document.delete_fixture(id).is_err(), "not a fixture");
}

// ---- criterion 13: delete ---------------------------------------------------------------

#[test]
fn delete_manufacturing_takes_page_and_fixtures_and_undo_restores_all_of_it() {
    let (document, object, id) = with_laser();
    document
        .add_fixture(id, GRID, "Engraved grid", &grid_params(10.0))
        .unwrap();
    let before = json(&document);
    let drawing = document.read_objects().objects.clone();
    document.delete_manufacturing(id).unwrap();
    assert_eq!(last_label(&document), "delete_manufacturing");
    assert_eq!(document.manufacturings(), Vec::new());
    assert_eq!(document.read_objects().objects, drawing);
    assert_eq!(document.children(None), vec![object]);
    assert_eq!(document.pages().len(), 1);
    document.undo_step(last_step(&document)).unwrap();
    assert_eq!(json(&document), before);
    let restored = document.manufacturing(id).unwrap();
    assert_eq!(restored.fixtures.len(), 1);
    assert!(restored.page.is_some());
}

// ---- criterion 12: format -----------------------------------------------------------------

fn golden_document() -> Document {
    let document = Document::new(1);
    let _ = rect(&document, 20.0);
    let id = document.add_manufacturing(laser(Some("first"))).unwrap();
    document
        .set_work_area(id, DocumentSize::from_mm(600.0, 400.0))
        .unwrap();
    let shown = document
        .add_fixture(id, GRID, "Engraved grid", &grid_params(25.0))
        .unwrap();
    // A step between the two adds, so no two history rows in a row carry one label.
    document
        .set_fixture_params(shown, &grid_params(10.0))
        .unwrap();
    let hidden = document
        .add_fixture(id, GRID, "Engraved grid", &grid_params(25.0))
        .unwrap();
    document.set_fixture_visible(hidden, false).unwrap();
    document
}

/// `loro` as a container of this build's format.
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
    }
}

fn manifest_version(bytes: &[u8]) -> u32 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
    u32::try_from(manifest["format_version"].as_u64().unwrap()).unwrap()
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

// The manufacturing shape of this file arrived with format 35; later formats keep it.
const _: () = assert!(CURRENT_FORMAT_VERSION >= 35);

#[test]
fn the_laser_golden_reads_back_identical_and_opening_it_writes_nothing() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&bytes), CURRENT_FORMAT_VERSION);
    let document = unpack(2, &bytes).unwrap();
    let version = document.version();
    let made = &document.manufacturings()[0];
    assert_eq!(made.name, "Laser 1");
    assert_eq!(made.process, "curvyo.laser");
    assert_eq!(made.profile.as_ref(), Some(generic_copy()));
    assert_eq!(made.work_area(), DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(made.fixtures.len(), 2);
    assert!(made.fixtures[0].visible);
    assert!(!made.fixtures[1].visible);
    assert_eq!(made.fixtures[1].params, grid_params(25.0));
    assert_eq!(made.placements.len(), 1);
    assert_eq!(made.placements[0].page, "first");
    assert_eq!(document.pages().len(), 1);
    assert_eq!(
        document.manufacturings(),
        golden_document().manufacturings()
    );
    assert_eq!(document.version(), version);
    assert_eq!(
        stored_loro(&bytes).oplog_vv(),
        stored_loro(&pack(&document, "0.1.0").unwrap()).oplog_vv()
    );
}

#[test]
fn the_format_35_laser_golden_opens_as_generic_laser_and_writes_nothing() {
    let bytes = std::fs::read(fixture_path(GOLDEN_V35)).unwrap();
    assert_eq!(manifest_version(&bytes), 35);
    let document = unpack(2, &bytes).unwrap();
    let version = document.version();
    let made = &document.manufacturings()[0];
    assert_eq!(made.profile, None);
    assert_eq!(made.legacy_machine_name.as_deref(), Some("Generic laser"));
    assert_eq!(made.work_area(), DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(document.version(), version);
    assert_eq!(
        stored_loro(&bytes).oplog_vv(),
        stored_loro(&pack(&document, "0.1.0").unwrap()).oplog_vv()
    );
}

#[test]
fn an_unknown_process_opens_and_saves_back_unchanged() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(UNKNOWN)).unwrap();
    let document = unpack(2, &bytes).unwrap();
    let made = &document.manufacturings()[0];
    assert_eq!(made.process, "example.cnc");
    assert_eq!(made.name, "Laser 1");
    assert_eq!(
        made.process_values["x.example.cnc.spindle"],
        serde_json::json!(12_000.0)
    );
    let saved = pack(&document, "0.1.0").unwrap();
    let again = unpack(4, &saved).unwrap();
    assert_eq!(again.manufacturings(), document.manufacturings());
    assert_eq!(
        stored_loro(&saved).oplog_vv(),
        stored_loro(&bytes).oplog_vv()
    );
}

/// The golden with one register replaced, saved as this build's container.
fn damaged(edit: impl FnOnce(&LoroDoc)) -> Vec<u8> {
    let loro = LoroDoc::new();
    loro.import(&golden_document().export_loro_snapshot().unwrap())
        .unwrap();
    edit(&loro);
    loro.commit();
    repack(&loro)
}

fn first_child(loro: &LoroDoc, shape: &str) -> loro::LoroMap {
    let tree = loro.get_tree("paths");
    let node = tree
        .children(loro::TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
        .unwrap();
    tree.children(loro::TreeParentId::Node(node))
        .unwrap()
        .into_iter()
        .map(|c| tree.get_meta(c).unwrap())
        .find(|meta| {
            meta.get("shape")
                .is_some_and(|v| v.get_deep_value() == LoroValue::from(shape))
        })
        .unwrap()
}

#[test]
fn damaged_manufacturing_registers_are_refused_as_damaged() {
    let cases: Vec<Vec<u8>> = vec![
        damaged(|loro| {
            first_child(loro, "placement")
                .insert("placement_link", LoroValue::from("first"))
                .unwrap();
        }),
        damaged(|loro| {
            let link: std::collections::HashMap<String, LoroValue> = [
                ("page".to_owned(), LoroValue::from("first")),
                (
                    "matrix".to_owned(),
                    LoroValue::from(vec![LoroValue::Double(f64::NAN); 6]),
                ),
            ]
            .into();
            first_child(loro, "placement")
                .insert("placement_link", LoroValue::from(link))
                .unwrap();
        }),
        damaged(|loro| {
            first_child(loro, "fixture")
                .insert("fixture_params", LoroValue::from("dense"))
                .unwrap();
        }),
        damaged(|loro| {
            let tree = loro.get_tree("paths");
            let drawing = tree
                .children(loro::TreeParentId::Root)
                .unwrap()
                .into_iter()
                .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_none())
                .unwrap();
            let owner = format!("{}:{}", drawing.peer, drawing.counter);
            loro.get_map("root")
                .insert("page.first.owner", LoroValue::from(owner))
                .unwrap();
        }),
    ];
    for (index, bytes) in cases.iter().enumerate() {
        assert!(
            matches!(unpack(2, bytes), Err(OpenError::Damaged)),
            "case {index}"
        );
    }
    assert!(unpack(2, &damaged(|_| {})).is_ok(), "the undamaged control");
}
