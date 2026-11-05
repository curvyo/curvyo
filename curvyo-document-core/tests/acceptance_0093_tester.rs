//! Independent acceptance tests for `specs/0093-laser-workbench-shell` at the document model and
//! file format (ADR 0032, ADR 0033): the manufacturing, its owned output page, placements and
//! fixtures, delete and undo (criteria 2, 3, 5, 9, 10, 12, 13), the laser golden read against the
//! criterion's own list, and the unknown-process round trip with data a build cannot know
//! (process values, an unknown fixture kind, an unknown child) kept unchanged.
//!
//! Updated for `specs/0098-page-kinds` (its "Changes to accepted specs" table): criteria 2 and 3
//! of `0093` are now Add page with Kind Laser and a read-only work area (`0098` 4, 13), a new laser
//! page holds no placement (`0098` 4), and delete is Delete page (`0098` 16). Placements come from
//! older files, so the placement tests build one in the raw file.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewOutputPage, NodeId,
    OpenError, Point, RectBounds, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};

const GRID: &str = "curvyo.laser/engraved_grid";
const TOL: f64 = 1e-9;

fn laser() -> NewOutputPage<'static> {
    NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(400.0, 300.0),
    }
}

/// Add page with Kind Laser; the manufacturing that holds its machine data.
fn add(d: &Document) -> NodeId {
    d.add_output_page(laser())
        .unwrap()
        .kind
        .manufacturing()
        .unwrap()
}

/// `d` with a placement of `page` at identity under its first manufacturing, as an older file
/// holds it, reopened.
fn with_placement(d: &Document, page: &str) -> Document {
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    add_raw_placement(&loro, page);
    loro.commit();
    unpack(9, &container(&loro)).unwrap()
}

fn add_raw_placement(loro: &LoroDoc, page: &str) {
    let tree = loro.get_tree("paths");
    let node = manufacturing_node(loro);
    let child = tree.create(TreeParentId::Node(node)).unwrap();
    let meta = tree.get_meta(child).unwrap();
    meta.insert("shape", "placement").unwrap();
    let identity: Vec<LoroValue> = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        .into_iter()
        .map(LoroValue::Double)
        .collect();
    let link: std::collections::HashMap<String, LoroValue> = [
        ("page".to_owned(), LoroValue::from(page)),
        ("matrix".to_owned(), LoroValue::from(identity)),
    ]
    .into();
    meta.insert("placement_link", LoroValue::from(link))
        .unwrap();
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

/// The manufacturing node's registers and children, without the page record.
fn manufacturing_meta_and_children(loro: &LoroDoc) -> (serde_json::Value, serde_json::Value) {
    let value = manufacturing_value(loro);
    (value["meta"].clone(), value["children"].clone())
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
    let id = add(&d);
    assert_eq!(d.own_step_count(), steps + 1, "Add laser page is one step");
    let m = d.manufacturing(id).unwrap();
    assert_eq!(m.page.as_ref().unwrap().name, "Laser 1");
    assert_eq!(
        m.legacy_machine_name, None,
        "format 36 writes no placeholder name"
    );
    undo_last(&d);
    assert!(d.manufacturings().is_empty());
    assert_eq!(d.pages(), pages);
    assert_eq!(json(&d), before, "undo removes the laser page and its data");
}

#[test]
fn ac2_the_next_laser_page_takes_the_next_free_number() {
    let d = Document::new(1);
    let a = d.add_output_page(laser()).unwrap();
    let b = d.add_output_page(laser()).unwrap();
    assert_eq!(b.name, "Laser 2");
    d.delete_page(&a.id).unwrap();
    let c = d.add_output_page(laser()).unwrap();
    assert_eq!(c.name, "Laser 1", "the smallest free number");
}

#[test]
fn ac3_the_output_page_is_the_work_area_and_no_size_command_writes_it() {
    let d = Document::new(1);
    let id = add(&d);
    let page = d.manufacturing(id).unwrap().page.unwrap();
    assert_eq!(page.size, DocumentSize::from_mm(400.0, 300.0));
    assert!(
        d.pages().iter().any(|p| p.id == page.id),
        "the laser page is listed (`0098` criterion 10)"
    );
    let steps = d.own_step_count();
    assert!(
        d.resize_page(&page.id, DocumentSize::from_mm(600.0, 400.0))
            .is_err()
    );
    assert!(
        d.fit_page_to_content(&page.id, (Point::new(0.0, 0.0), Point::new(5.0, 5.0)))
            .is_err()
    );
    assert_eq!(d.own_step_count(), steps);
}

#[test]
fn ac3_an_invalid_work_area_is_refused_and_writes_nothing() {
    let d = Document::new(1);
    let steps = d.own_step_count();
    for (w, h) in [
        (0.0, 300.0),
        (-5.0, 300.0),
        (f64::NAN, 300.0),
        (400.0, f64::INFINITY),
    ] {
        let new = NewOutputPage {
            work_area: DocumentSize::from_mm(w, h),
            ..laser()
        };
        assert!(d.add_output_page(new).is_err(), "{w} x {h}");
    }
    assert_eq!(d.own_step_count(), steps);
    assert!(d.manufacturings().is_empty());
}

#[test]
fn ac5_a_new_laser_page_places_nothing_and_copies_no_object() {
    let d = Document::new(1);
    let a = rect(&d, 10.0, 10.0);
    let b = rect(&d, 60.0, 40.0);
    let objects_before = d.object_ids();
    let id = add(&d);
    let m = d.manufacturing(id).unwrap();
    assert!(m.placements.is_empty(), "`0098` criterion 4");
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
    let placed = with_placement(&d, "first");
    let m = placed.manufacturing(id).unwrap();
    assert_eq!(m.placements[0].page, "first");
    assert_eq!(m.placements[0].matrix, Affine::IDENTITY);
    assert!(!placed.is_in_manufacturing(a));
    assert!(placed.is_in_manufacturing(m.placements[0].id));
}

// ---- criteria 9, 10 ----------------------------------------------------------------------------

#[test]
fn ac9_ac10_fixtures_are_not_drawing_objects_and_hide_is_one_saved_step() {
    let d = Document::new(1);
    let o = rect(&d, 10.0, 10.0);
    let id = add(&d);
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
    let id = add(&d);
    let d = with_placement(&d, &second.id);
    let shown = d
        .add_fixture(id, GRID, "Engraved grid", &grid(10.0))
        .unwrap();
    let hidden = d
        .add_fixture(id, GRID, "Engraved grid", &grid(25.0))
        .unwrap();
    d.set_fixture_visible(hidden, false).unwrap();
    let full = d.manufacturing(id).unwrap();
    let output = full.page.clone().unwrap();
    let pages = d.pages();
    let drawing_pages: Vec<_> = pages
        .iter()
        .filter(|p| p.id != output.id)
        .cloned()
        .collect();
    let objects = d.read_objects().objects.clone();
    let before = json(&d);

    let steps = d.own_step_count();
    d.delete_page(&output.id).unwrap();
    assert_eq!(d.own_step_count(), steps + 1, "delete is one step");
    assert!(d.manufacturings().is_empty());
    assert!(d.manufacturing(id).is_none());
    assert_eq!(d.pages(), drawing_pages, "drawing pages untouched");
    assert_eq!(d.read_objects().objects, objects, "drawing untouched");
    let after = json(&d);
    assert!(
        !after.to_string().contains(&output.id),
        "the output page record is gone from document.json"
    );

    undo_last(&d);
    let restored = d.manufacturing(id).unwrap();
    assert_eq!(
        restored, full,
        "manufacturing, output page, fixtures restored"
    );
    assert_eq!(restored.fixtures.len(), 2);
    assert_eq!(restored.placements.len(), 1);
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
fn ac13_deleting_twice_or_the_last_page_is_refused() {
    let d = Document::new(1);
    let o = rect(&d, 10.0, 10.0);
    let page = d.add_output_page(laser()).unwrap();
    d.delete_page(&page.id).unwrap();
    let steps = d.own_step_count();
    assert!(d.delete_page(&page.id).is_err());
    assert!(d.delete_page("first").is_err(), "the last page stays");
    assert_eq!(d.own_step_count(), steps);
    assert_eq!(d.object_ids(), vec![o]);
}

#[test]
fn ac5_a_placement_of_a_deleted_page_is_kept_and_undo_brings_the_page_back() {
    let d = Document::new(1);
    let second = d.add_page();
    let id = add(&d);
    let d = with_placement(&d, &second.id);
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
        .join("laser_workbench_v36.curvyo")
}

#[test]
fn ac12_the_laser_golden_holds_what_the_criterion_lists() {
    let bytes = std::fs::read(golden_path()).expect("the frozen format-36 golden");
    assert_eq!(manifest_version(&bytes), 36);
    let d = unpack(7, &bytes).unwrap();
    let all = d.manufacturings();
    assert_eq!(all.len(), 1);
    let m = &all[0];
    assert_eq!(m.process, "curvyo.laser");
    assert_eq!(
        m.legacy_machine_name, None,
        "format 36 writes no placeholder name"
    );
    assert_eq!(m.work_area(), DocumentSize::from_mm(600.0, 400.0));
    let page = m.page.as_ref().unwrap();
    assert_eq!(page.size, DocumentSize::from_mm(600.0, 400.0));
    assert!(
        d.pages().iter().any(|p| p.id == page.id),
        "listed since `0098` criterion 10"
    );
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
        manufacturing_meta_and_children(&stored_loro(&again)),
        manufacturing_meta_and_children(&stored_loro(&bytes)),
        "the open-time conversion only places the page"
    );
}

#[test]
fn ac12_save_and_reopen_of_a_fresh_project_is_identical() {
    let d = Document::new(1);
    let _ = rect(&d, 10.0, 10.0);
    let id = add(&d);
    let d = with_placement(&d, "first");
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
        .add_output_page(NewOutputPage {
            process: "acme.waterjet",
            label: "Waterjet",
            profile: None,
            work_area: DocumentSize::from_mm(1500.0, 3000.0),
        })
        .unwrap()
        .kind
        .manufacturing()
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
    add_raw_placement(&loro, "first");
    loro.commit();
    container(&loro)
}

#[test]
fn ac12_an_unknown_process_opens_and_saves_back_unchanged() {
    let bytes = unknown_process_file();
    let d = unpack(4, &bytes).expect("a file with an unknown process opens");
    let m = &d.manufacturings()[0];
    assert_eq!(m.process, "acme.waterjet");
    assert_eq!(m.page.as_ref().unwrap().name, "Waterjet 1");
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
    d.delete_page(&full.page.clone().unwrap().id).unwrap();
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
