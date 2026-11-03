//! Independent acceptance tests for `specs/0093-laser-workbench-shell` at the document model and
//! file format (ADR 0032, ADR 0033): the manufacturing, its owned output page, placements and
//! fixtures, delete and undo (criteria 2, 3, 5, 9, 10, 12, 13), the laser golden read against the
//! criterion's own list, and the unknown-process round trip with data a build cannot know
//! (process values, an unknown fixture kind, an unknown child) kept unchanged.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewManufacturing,
    NodeId, OpenError, Point, RectBounds, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};

const GRID: &str = "curvyo.laser/engraved_grid";
const TOL: f64 = 1e-9;

fn laser(place: Option<&str>) -> NewManufacturing<'_> {
    NewManufacturing {
        process: "curvyo.laser",
        label: "Laser",
        machine_name: "Generic laser",
        work_area: DocumentSize::from_mm(400.0, 300.0),
        place,
    }
}

fn grid(spacing: f64) -> FixtureParams {
    serde_json::from_value(serde_json::json!({
        "spacing": {"mm": spacing},
        "line_width": {"mm": 0.1},
        "origin_x": {"mm": 0.0},
        "origin_y": {"mm": 0.0},
    }))
    .unwrap()
}

fn rect(d: &Document, x: f64, y: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + 30.0, y + 20.0),
    ))
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

fn json(d: &Document) -> serde_json::Value {
    serde_json::from_slice(&d.export_json().unwrap()).unwrap()
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

/// `loro` written as a container of this build's format.
fn container(loro: &LoroDoc) -> Vec<u8> {
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

fn manufacturing_node(loro: &LoroDoc) -> loro::TreeID {
    let tree = loro.get_tree("paths");
    tree.children(TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
        .unwrap()
}

/// The whole value of `loro`'s manufacturing subtree and its owned page record, for comparing a
/// file before and after a round trip by meaning.
fn manufacturing_value(loro: &LoroDoc) -> serde_json::Value {
    let tree = loro.get_tree("paths");
    let node = manufacturing_node(loro);
    let meta = tree.get_meta(node).unwrap().get_deep_value();
    let children: Vec<String> = tree
        .children(TreeParentId::Node(node))
        .unwrap()
        .into_iter()
        .map(|c| format!("{:?}", tree.get_meta(c).unwrap().get_deep_value()))
        .collect();
    let root = loro.get_map("root").get_deep_value();
    let page_keys: Vec<String> = match &root {
        LoroValue::Map(map) => map
            .iter()
            .filter(|(k, _)| k.starts_with("page."))
            .map(|(k, v)| format!("{k}={v:?}"))
            .collect(),
        _ => Vec::new(),
    };
    let mut page_keys = page_keys;
    page_keys.sort();
    serde_json::json!({
        "meta": format!("{meta:?}"),
        "children": children,
        "pages": page_keys,
    })
}

// ---- criteria 2, 3, 5 ------------------------------------------------------------------------

#[test]
fn ac2_add_is_one_step_and_its_undo_leaves_the_document_as_before() {
    let d = Document::new(1);
    let _ = rect(&d, 10.0, 10.0);
    let before = json(&d);
    let pages = d.pages();
    let steps = d.own_step_count();
    let id = d.add_manufacturing(laser(Some(&pages[0].id))).unwrap();
    assert_eq!(
        d.own_step_count(),
        steps + 1,
        "Add manufacturing is one step"
    );
    let m = d.manufacturing(id).unwrap();
    assert_eq!(m.name, "Laser 1");
    assert_eq!(m.machine_name, "Generic laser");
    undo_last(&d);
    assert!(d.manufacturings().is_empty());
    assert_eq!(d.pages(), pages);
    assert_eq!(
        json(&d),
        before,
        "undo removes manufacturing and output page"
    );
}

#[test]
fn ac2_the_next_manufacturing_takes_the_next_free_number() {
    let d = Document::new(1);
    let a = d.add_manufacturing(laser(None)).unwrap();
    let b = d.add_manufacturing(laser(None)).unwrap();
    assert_eq!(d.manufacturing(b).unwrap().name, "Laser 2");
    d.delete_manufacturing(a).unwrap();
    let c = d.add_manufacturing(laser(None)).unwrap();
    assert_eq!(
        d.manufacturing(c).unwrap().name,
        "Laser 1",
        "the smallest free number"
    );
}

#[test]
fn ac3_the_output_page_is_the_work_area_and_follows_it_in_one_step() {
    let d = Document::new(1);
    let first = d.pages()[0].id.clone();
    let id = d.add_manufacturing(laser(Some(&first))).unwrap();
    let m = d.manufacturing(id).unwrap();
    let page = m.page.clone().unwrap();
    assert_eq!(page.size, DocumentSize::from_mm(400.0, 300.0));
    assert!(
        d.pages().iter().all(|p| p.id != page.id),
        "owned page out of the drawing page list"
    );
    let matrix = m.placements[0].matrix;
    let steps = d.own_step_count();
    assert!(
        d.set_work_area(id, DocumentSize::from_mm(600.0, 400.0))
            .unwrap()
    );
    assert_eq!(d.own_step_count(), steps + 1);
    let m = d.manufacturing(id).unwrap();
    assert_eq!(m.work_area(), DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(m.page.unwrap().size, DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(
        m.placements[0].matrix, matrix,
        "placed content keeps its place relative to the top-left corner"
    );
    undo_last(&d);
    assert_eq!(
        d.manufacturing(id).unwrap().work_area(),
        DocumentSize::from_mm(400.0, 300.0)
    );
}

#[test]
fn ac3_an_invalid_work_area_is_refused_and_writes_nothing() {
    let d = Document::new(1);
    let id = d.add_manufacturing(laser(None)).unwrap();
    let steps = d.own_step_count();
    for (w, h) in [
        (0.0, 300.0),
        (-5.0, 300.0),
        (f64::NAN, 300.0),
        (400.0, f64::INFINITY),
    ] {
        assert!(
            d.set_work_area(id, DocumentSize::from_mm(w, h)).is_err(),
            "{w} x {h}"
        );
    }
    assert_eq!(d.own_step_count(), steps);
    assert_eq!(
        d.manufacturing(id).unwrap().work_area(),
        DocumentSize::from_mm(400.0, 300.0)
    );
}

#[test]
fn ac5_the_placement_is_a_reference_with_identity_matrix_and_no_copied_objects() {
    let d = Document::new(1);
    let a = rect(&d, 10.0, 10.0);
    let b = rect(&d, 60.0, 40.0);
    let first = d.pages()[0].id.clone();
    let objects_before = d.object_ids();
    let id = d.add_manufacturing(laser(Some(&first))).unwrap();
    let m = d.manufacturing(id).unwrap();
    assert_eq!(m.placements.len(), 1);
    assert_eq!(m.placements[0].page, first);
    assert_eq!(m.placements[0].matrix, Affine::IDENTITY);
    assert_eq!(
        d.object_ids(),
        objects_before,
        "no drawing object is copied"
    );
    assert_eq!(
        d.children(None),
        vec![a, b],
        "the drawing tree is unchanged"
    );
    assert!(!d.is_in_manufacturing(a));
    assert!(d.is_in_manufacturing(m.placements[0].id));
}

// ---- criteria 9, 10 ----------------------------------------------------------------------------

#[test]
fn ac9_ac10_fixtures_are_not_drawing_objects_and_hide_is_one_saved_step() {
    let d = Document::new(1);
    let o = rect(&d, 10.0, 10.0);
    let id = d.add_manufacturing(laser(Some("first"))).unwrap();
    assert!(d.manufacturing(id).unwrap().fixtures.is_empty());
    let f = d
        .add_fixture(id, GRID, "Engraved grid", &grid(10.0))
        .unwrap();
    assert!(d.is_in_manufacturing(f));
    assert_eq!(
        d.object_ids(),
        vec![o],
        "a fixture is not part of the drawing"
    );
    assert!(
        d.read_objects().objects.iter().all(|s| s.id() == o),
        "the drawing read holds no fixture"
    );
    let steps = d.own_step_count();
    assert!(d.set_fixture_visible(f, false).unwrap());
    assert_eq!(d.own_step_count(), steps + 1);
    assert!(
        !d.set_fixture_visible(f, false).unwrap(),
        "no change, no step"
    );
    assert_eq!(d.own_step_count(), steps + 1);
    let back = unpack(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert!(!back.manufacturing(id).unwrap().fixtures[0].visible);
}

// ---- criterion 13 ----------------------------------------------------------------------------

#[test]
fn ac13_delete_takes_page_and_fixtures_leaves_drawing_and_undo_restores_everything() {
    let d = Document::new(1);
    let _ = rect(&d, 10.0, 10.0);
    let second = d.add_page();
    let _ = rect(&d, second.origin.x + 5.0, second.origin.y + 5.0);
    let id = d.add_manufacturing(laser(Some(&second.id))).unwrap();
    d.set_work_area(id, DocumentSize::from_mm(600.0, 400.0))
        .unwrap();
    let shown = d
        .add_fixture(id, GRID, "Engraved grid", &grid(10.0))
        .unwrap();
    let hidden = d
        .add_fixture(id, GRID, "Engraved grid", &grid(25.0))
        .unwrap();
    d.set_fixture_visible(hidden, false).unwrap();
    let full = d.manufacturing(id).unwrap();
    let pages = d.pages();
    let objects = d.read_objects().objects.clone();
    let before = json(&d);

    let steps = d.own_step_count();
    d.delete_manufacturing(id).unwrap();
    assert_eq!(d.own_step_count(), steps + 1, "delete is one step");
    assert!(d.manufacturings().is_empty());
    assert!(d.manufacturing(id).is_none());
    assert_eq!(d.pages(), pages, "drawing pages untouched");
    assert_eq!(d.read_objects().objects, objects, "drawing untouched");
    let after = json(&d);
    let text = after.to_string();
    assert!(
        !text.contains(&full.page.as_ref().unwrap().id),
        "the output page record is gone from document.json"
    );

    undo_last(&d);
    let restored = d.manufacturing(id).unwrap();
    assert_eq!(
        restored, full,
        "manufacturing, output page, fixtures restored"
    );
    assert_eq!(restored.fixtures.len(), 2);
    assert!(restored.fixtures.iter().any(|f| f.id == shown && f.visible));
    assert!(
        restored
            .fixtures
            .iter()
            .any(|f| f.id == hidden && !f.visible)
    );
    assert_eq!(d.pages(), pages);
    assert_eq!(d.read_objects().objects, objects);
    assert_eq!(json(&d), before);
}

#[test]
fn ac13_deleting_twice_or_a_drawing_node_is_refused() {
    let d = Document::new(1);
    let o = rect(&d, 10.0, 10.0);
    let id = d.add_manufacturing(laser(None)).unwrap();
    d.delete_manufacturing(id).unwrap();
    let steps = d.own_step_count();
    assert!(d.delete_manufacturing(id).is_err());
    assert!(
        d.delete_manufacturing(o).is_err(),
        "a drawing object is no manufacturing"
    );
    assert_eq!(d.own_step_count(), steps);
    assert_eq!(d.object_ids(), vec![o]);
}

#[test]
fn ac5_a_placement_of_a_deleted_page_is_kept_and_undo_brings_the_page_back() {
    let d = Document::new(1);
    let second = d.add_page();
    let id = d.add_manufacturing(laser(Some(&second.id))).unwrap();
    d.delete_page(&second.id).unwrap();
    let m = d.manufacturing(id).unwrap();
    assert_eq!(m.placements.len(), 1, "no orphan repair (ADR 0032 §7)");
    assert_eq!(m.placements[0].page, second.id);
    let reopened = unpack(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(reopened.manufacturing(id).unwrap().placements.len(), 1);
    undo_last(&d);
    assert!(d.pages().iter().any(|p| p.id == second.id));
}

// ---- criterion 12 ----------------------------------------------------------------------------

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format!("laser_workbench_v{CURRENT_FORMAT_VERSION}.curvyo"))
}

#[test]
fn ac12_the_laser_golden_holds_what_the_criterion_lists() {
    let bytes = std::fs::read(golden_path()).expect("golden named after this build's format");
    assert_eq!(manifest_version(&bytes), u64::from(CURRENT_FORMAT_VERSION));
    let d = unpack(7, &bytes).unwrap();
    let all = d.manufacturings();
    assert_eq!(all.len(), 1);
    let m = &all[0];
    assert_eq!(m.process, "curvyo.laser");
    assert_eq!(m.machine_name, "Generic laser");
    assert_eq!(m.work_area(), DocumentSize::from_mm(600.0, 400.0));
    let page = m.page.as_ref().unwrap();
    assert_eq!(page.size, DocumentSize::from_mm(600.0, 400.0));
    assert!(d.pages().iter().all(|p| p.id != page.id));
    assert_eq!(m.fixtures.len(), 2);
    assert_eq!(m.fixtures.iter().filter(|f| !f.visible).count(), 1);
    assert!(m.fixtures.iter().all(|f| f.kind == GRID));
    assert_eq!(m.placements.len(), 1);
    assert!(
        d.pages().iter().any(|p| p.id == m.placements[0].page),
        "the placement names a drawing page of the file"
    );
    // Saved again, it is the same.
    let again = pack(&d, "0.1.0").unwrap();
    let e = unpack(8, &again).unwrap();
    assert_eq!(e.manufacturings(), d.manufacturings());
    assert_eq!(e.pages(), d.pages());
    assert_eq!(
        manufacturing_value(&stored_loro(&again)),
        manufacturing_value(&stored_loro(&bytes))
    );
}

#[test]
fn ac12_save_and_reopen_of_a_fresh_project_is_identical() {
    let d = Document::new(1);
    let _ = rect(&d, 10.0, 10.0);
    let id = d.add_manufacturing(laser(Some("first"))).unwrap();
    d.set_work_area(id, DocumentSize::from_mm(600.0, 400.0))
        .unwrap();
    d.add_fixture(id, GRID, "Engraved grid", &grid(10.0))
        .unwrap();
    let hidden = d
        .add_fixture(id, GRID, "Engraved grid", &grid(5.0))
        .unwrap();
    d.set_fixture_visible(hidden, false).unwrap();
    let back = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(back.manufacturings(), d.manufacturings());
    assert_eq!(back.pages(), d.pages());
    assert_eq!(back.read_objects().objects, d.read_objects().objects);
}

/// A project with a manufacturing of a process no build knows: process values, a fixture of an
/// unknown kind with arbitrary parameters, and a placement.
fn unknown_process_file() -> Vec<u8> {
    let d = Document::new(1);
    let _ = rect(&d, 10.0, 10.0);
    let id = d
        .add_manufacturing(NewManufacturing {
            process: "acme.waterjet",
            label: "Waterjet",
            machine_name: "Acme 3000",
            work_area: DocumentSize::from_mm(1500.0, 3000.0),
            place: Some("first"),
        })
        .unwrap();
    let params: FixtureParams = serde_json::from_value(serde_json::json!({
        "jaw": {"mm": 42.5},
        "label": "left vise",
        "count": 3,
    }))
    .unwrap();
    d.add_fixture(id, "acme.waterjet/vise", "Vise", &params)
        .unwrap();
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = loro.get_tree("paths");
    let meta = tree.get_meta(manufacturing_node(&loro)).unwrap();
    meta.insert("x.acme.waterjet.pressure", LoroValue::Double(4100.0))
        .unwrap();
    meta.insert("x.acme.waterjet.abrasive", LoroValue::from("garnet"))
        .unwrap();
    loro.commit();
    container(&loro)
}

#[test]
fn ac12_an_unknown_process_opens_and_saves_back_unchanged() {
    let bytes = unknown_process_file();
    let d = unpack(4, &bytes).expect("a file with an unknown process opens");
    let m = &d.manufacturings()[0];
    assert_eq!(m.process, "acme.waterjet");
    assert_eq!(m.name, "Waterjet 1");
    assert_eq!(m.fixtures.len(), 1);
    assert_eq!(m.fixtures[0].kind, "acme.waterjet/vise");
    assert_eq!(m.fixtures[0].params["jaw"], serde_json::json!({"mm": 42.5}));
    assert_eq!(
        m.process_values["x.acme.waterjet.pressure"],
        serde_json::json!(4100.0)
    );
    assert_eq!(
        m.page.as_ref().unwrap().size,
        DocumentSize::from_mm(1500.0, 3000.0)
    );
    let saved = pack(&d, "0.1.0").unwrap();
    assert_eq!(
        stored_loro(&saved).oplog_vv(),
        stored_loro(&bytes).oplog_vv(),
        "opening and saving wrote nothing"
    );
    assert_eq!(
        manufacturing_value(&stored_loro(&saved)),
        manufacturing_value(&stored_loro(&bytes))
    );
    assert_eq!(
        unpack(5, &saved).unwrap().manufacturings(),
        d.manufacturings()
    );
}

#[test]
fn ac12_an_unknown_process_can_still_be_deleted_and_undone() {
    let d = unpack(4, &unknown_process_file()).unwrap();
    let id = d.manufacturings()[0].id;
    let full = d.manufacturing(id).unwrap();
    d.delete_manufacturing(id).unwrap();
    assert!(d.manufacturings().is_empty());
    undo_last(&d);
    assert_eq!(d.manufacturing(id).unwrap(), full);
}

#[test]
fn ac12_a_huge_placement_matrix_is_damage() {
    let loro = stored_loro(&unknown_process_file());
    let tree = loro.get_tree("paths");
    let node = manufacturing_node(&loro);
    let placement = tree
        .children(TreeParentId::Node(node))
        .unwrap()
        .into_iter()
        .map(|c| tree.get_meta(c).unwrap())
        .find(|m| {
            m.get("shape")
                .is_some_and(|v| v.get_deep_value() == LoroValue::from("placement"))
        })
        .unwrap();
    let link: std::collections::HashMap<String, LoroValue> = [
        ("page".to_owned(), LoroValue::from("first")),
        (
            "matrix".to_owned(),
            LoroValue::from(vec![
                LoroValue::Double(1.0),
                LoroValue::Double(0.0),
                LoroValue::Double(0.0),
                LoroValue::Double(1.0),
                LoroValue::Double(2e7),
                LoroValue::Double(0.0),
            ]),
        ),
    ]
    .into();
    placement
        .insert("placement_link", LoroValue::from(link))
        .unwrap();
    loro.commit();
    assert!(matches!(
        unpack(2, &container(&loro)),
        Err(OpenError::Damaged)
    ));
}

#[test]
fn ac12_a_file_without_manufacturing_has_no_manufacturing_record() {
    let d = Document::new(1);
    let _ = rect(&d, 10.0, 10.0);
    let back = unpack(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert!(back.manufacturings().is_empty());
    assert!((back.pages()[0].origin.x).abs() < TOL);
}
