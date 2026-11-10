//! Manufacturings in the document model (`specs/0093-laser-workbench-shell` criteria 4, 9, 10, 12;
//! ADR 0032, ADR 0033, amended by ADR 0042 and `specs/0098-page-kinds`): the laser page owning its
//! manufacturing, the skip predicate per consumer, open validation and the frozen goldens
//! `laser_workbench_v36.curvyo`, `laser_workbench_v35.curvyo` and
//! `manufacturing_unknown_process_v35.curvyo`, which open by the migrations of ADR 0039 §6 and
//! ADR 0042 §9. Placements come only from older files (`0098` criterion 12).

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
    OpenError, PageKind, Point, RectBounds, StepRef, pack, unpack,
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

fn laser() -> NewOutputPage<'static> {
    NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: Some(generic_copy()),
        work_area: DocumentSize::from_mm(400.0, 300.0),
    }
}

/// The manufacturing of a new laser page.
fn add_laser(document: &Document) -> NodeId {
    document
        .add_output_page(laser())
        .unwrap()
        .kind
        .manufacturing()
        .unwrap()
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

/// A drawing with one rectangle and a laser page.
fn with_laser() -> (Document, NodeId, NodeId) {
    let document = Document::new(1);
    let object = rect(&document, 20.0);
    let id = add_laser(&document);
    (document, object, id)
}

/// The frozen format-36 golden: a rectangle, a laser of 600 x 400 mm placing page 1, an engraved
/// grid shown and one hidden.
fn placed() -> Document {
    unpack(2, &std::fs::read(fixture_path(GOLDEN)).unwrap()).unwrap()
}

// ---- `0098` criterion 4, 16: the laser page owns its manufacturing ----------------------

#[test]
fn add_output_page_makes_laser_1_listed_and_empty_in_one_step() {
    let document = Document::new(1);
    let _ = rect(&document, 20.0);
    let steps = document.own_step_count();
    let page = document.add_output_page(laser()).unwrap();
    assert_eq!(document.own_step_count(), steps + 1);
    assert_eq!(last_label(&document), "add_output_page");
    assert_eq!(page.name, "Laser 1");
    assert_eq!(page.origin, Point::new(239.7, 0.0));
    let id = page.kind.manufacturing().unwrap();
    let made = document.manufacturing(id).unwrap();
    assert_eq!(made.process, "curvyo.laser");
    assert_eq!(made.profile.as_ref(), Some(generic_copy()));
    assert_eq!(made.work_area(), DocumentSize::from_mm(400.0, 300.0));
    assert_eq!(made.page.as_ref(), Some(&page));
    assert!(made.placements.is_empty(), "a new laser page holds nothing");
    assert!(made.fixtures.is_empty());
    let pages = document.pages();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[1], page);
    let second = document.add_output_page(laser()).unwrap();
    assert_eq!(second.name, "Laser 2");
}

#[test]
fn undo_of_add_output_page_removes_it_and_its_machine_data() {
    let document = Document::new(1);
    let before = json(&document);
    let id = add_laser(&document);
    let step = last_step(&document);
    document.undo_step(step).unwrap();
    assert!(document.manufacturing(id).is_none());
    assert_eq!(document.manufacturings(), Vec::new());
    assert_eq!(json(&document), before);
}

#[test]
fn the_output_page_is_listed_with_its_kind_and_its_size_is_fixed() {
    let (document, _, id) = with_laser();
    let pages = document.pages();
    assert_eq!(pages.len(), 2);
    assert_eq!(
        pages[1].kind,
        PageKind::Output {
            manufacturing: id,
            process: "curvyo.laser".into()
        }
    );
    assert_eq!(json(&document)["pages"].as_array().unwrap().len(), 2);
    assert_eq!(json(&document)["pages"][1]["kind"], "curvyo.laser");
    assert!(matches!(
        document.resize_page(
            &pages[1].id,
            DocumentSize::from_mm(10.0, 10.0),
            &curvyo_document_core::PageContents::new()
        ),
        Err(curvyo_document_core::PageError::FixedSize)
    ));
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
        .add_fixture(
            id,
            &curvyo_document_core::NewFixture {
                add: curvyo_document_core::FixtureAdd::Grid,
                kind: GRID,
                label: "Engraved grid",
                params: &grid_params(10.0),
                matrix: None,
            },
        )
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
        .add_fixture(
            id,
            &curvyo_document_core::NewFixture {
                add: curvyo_document_core::FixtureAdd::Grid,
                kind: GRID,
                label: "Engraved grid",
                params: &grid_params(10.0),
                matrix: None,
            },
        )
        .unwrap();
    let json = json(&document);
    assert_eq!(json["objects"].as_array().unwrap().len(), 1);
    assert_eq!(json["manufacturings"][0]["page"]["name"], "Laser 1");
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
        .add_fixture(
            id,
            &curvyo_document_core::NewFixture {
                add: curvyo_document_core::FixtureAdd::Grid,
                kind: GRID,
                label: "Engraved grid",
                params: &grid_params(10.0),
                matrix: None,
            },
        )
        .unwrap();
    let second = document
        .add_fixture(
            id,
            &curvyo_document_core::NewFixture {
                add: curvyo_document_core::FixtureAdd::Grid,
                kind: GRID,
                label: "Engraved grid",
                params: &grid_params(10.0),
                matrix: None,
            },
        )
        .unwrap();
    let names: Vec<String> = document
        .manufacturing(id)
        .unwrap()
        .fixtures
        .into_iter()
        .map(|f| f.name)
        .collect();
    assert_eq!(names, vec!["Engraved grid 1", "Engraved grid 2"]);
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

// ---- `0098` criterion 16: delete -------------------------------------------------------

#[test]
fn delete_page_takes_the_manufacturing_and_undo_restores_all_of_it() {
    let document = placed();
    let id = document.manufacturings()[0].id;
    let page = document.manufacturings()[0].page.clone().unwrap();
    let before = json(&document);
    let drawing = document.read_objects().objects.clone();
    let roots = document.children(None);
    document
        .delete_page(&page.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert_eq!(last_label(&document), "delete_page");
    assert_eq!(document.manufacturings(), Vec::new());
    assert_eq!(document.read_objects().objects, drawing);
    assert_eq!(document.children(None), roots);
    assert_eq!(document.pages().len(), 1);
    document.undo_step(last_step(&document)).unwrap();
    assert_eq!(json(&document), before);
    let restored = document.manufacturing(id).unwrap();
    assert_eq!(restored.fixtures.len(), 2);
    assert_eq!(restored.placements.len(), 1);
}

#[test]
fn a_placement_is_deleted_alone_in_one_step() {
    let document = placed();
    let made = document.manufacturings()[0].clone();
    document.delete_placement(made.placements[0].id).unwrap();
    assert_eq!(last_label(&document), "delete_placement");
    assert!(
        document
            .manufacturing(made.id)
            .unwrap()
            .placements
            .is_empty()
    );
    assert_eq!(document.pages().len(), 2);
    assert!(document.delete_placement(made.fixtures[0].id).is_err());
    document.undo_step(last_step(&document)).unwrap();
    assert_eq!(document.manufacturing(made.id).unwrap(), made);
}

// ---- criterion 12: format -----------------------------------------------------------------

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
fn the_format_36_laser_golden_opens_with_its_output_page_in_the_plane() {
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&bytes), 36);
    let document = unpack(2, &bytes).unwrap();
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
    assert_eq!(made.placements[0].matrix, Affine::IDENTITY);
    let pages = document.pages();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[1].origin, Point::new(239.7, 0.0));
    assert_eq!(made.page.as_ref(), Some(&pages[1]));
    // The conversion is no step of this session.
    assert_eq!(document.own_step_count(), 0);
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
    assert_eq!(document.own_step_count(), 0);
}

#[test]
fn an_unknown_process_opens_and_saves_back_unchanged() {
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
    // The process registers are saved back unchanged; the only write is the open-time
    // conversion, which places the output page.
    let stored = stored_loro(&saved);
    let node = stored
        .get_tree("paths")
        .children(loro::TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| {
            stored
                .get_tree("paths")
                .get_meta(n)
                .unwrap()
                .get("manufacturing")
                .is_some()
        })
        .unwrap();
    assert_eq!(
        stored
            .get_tree("paths")
            .get_meta(node)
            .unwrap()
            .get_deep_value(),
        manufacturing_meta(&stored_loro(&bytes))
    );
}

fn manufacturing_meta(loro: &LoroDoc) -> LoroValue {
    let tree = loro.get_tree("paths");
    let node = tree
        .children(loro::TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
        .unwrap();
    tree.get_meta(node).unwrap().get_deep_value()
}

/// The golden with one register replaced, saved as this build's container.
fn damaged(edit: impl FnOnce(&LoroDoc)) -> Vec<u8> {
    let loro = stored_loro(&std::fs::read(fixture_path(GOLDEN)).unwrap());
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
    // Since format 40 a fixture's values read per field (ADR 0049 §6): parameters that are not a
    // map read empty and count one damaged value; the file opens.
    let per_field = damaged(|loro| {
        first_child(loro, "fixture")
            .insert("fixture_params", LoroValue::from("dense"))
            .unwrap();
    });
    let opened = unpack(2, &per_field).unwrap();
    let fixture = &opened.manufacturings()[0].fixtures[0];
    assert!(fixture.params.is_empty());
    assert_eq!(fixture.damaged, 1);
}
