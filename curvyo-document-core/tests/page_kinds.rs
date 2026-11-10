//! Page kinds in the document model (`specs/0098-page-kinds` criteria 2 to 5, 14, 16, 18 c, 19,
//! 20; ADR 0042): the kind register and its read, Add page per kind, the fixed size of output and
//! unknown pages, the corner-anchored work-area change, the format-36 migration and the goldens
//! `page_kinds_v37.curvyo`, `page_kind_damaged_v37.curvyo`, `page_kind_unknown_v37.curvyo` and
//! `page_kinds_migration_v36.curvyo`. `CURVYO_WRITE_FIXTURES=1` rewrites them.

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
    ObjectSnapshot, PageCorner, PageError, PageKind, Point, ProfileChange, RectBounds, Style, Vec2,
    pack, unpack,
};
use curvyo_sketch_core::{Constraint, Position, Relation, Sketch};
use loro::{LoroDoc, LoroValue, TreeParentId};

const GOLDEN: &str = "page_kinds_v37.curvyo";
const DAMAGED: &str = "page_kind_damaged_v37.curvyo";
const UNKNOWN: &str = "page_kind_unknown_v37.curvyo";
const MIGRATION: &str = "page_kinds_migration_v36.curvyo";
const GRID: &str = "curvyo.laser/engraved_grid";
const TOL: f64 = 1e-9;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn profile(name: &str, origin: &str) -> FixtureParams {
    serde_json::from_value(serde_json::json!({
        "id": name,
        "name": name,
        "origin": origin,
        "max_feed_mm_per_min": 6000,
        "max_power": 1000,
        "laser_mode": true,
    }))
    .unwrap()
}

fn laser(copy: &FixtureParams, width: f64, height: f64) -> NewOutputPage<'_> {
    NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: Some(copy),
        work_area: DocumentSize::from_mm(width, height),
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

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo_last(d: &Document) {
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
}

/// Three connected lines, the first held horizontal.
fn three_lines() -> Sketch {
    let mut graph = curvyo_sketch_core::create_polyline(
        &Sketch::new("origin".into()),
        "edge",
        &[
            Position::mm(0.0, 0.0),
            Position::mm(40.0, 0.0),
            Position::mm(40.0, 30.0),
            Position::mm(10.0, 30.0),
        ],
        false,
    )
    .unwrap();
    let line = graph
        .entities
        .iter()
        .find_map(|(id, e)| {
            matches!(e, curvyo_sketch_core::Entity::Line { .. }).then(|| id.clone())
        })
        .unwrap();
    graph.constraints.insert(
        "flat".into(),
        Constraint::driving(Relation::Horizontal(line)),
    );
    graph
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

/// `loro` as a container of format `version`.
fn container(loro: &LoroDoc, version: u32) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format_version": version,
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

fn raw(d: &Document) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro
}

// ---- criteria 2, 3, 5: Add page per kind --------------------------------------------------

#[test]
fn add_page_of_each_kind_takes_its_name_size_place_and_step() {
    let d = Document::new(1);
    d.resize_page(
        "first",
        DocumentSize::from_mm(125.0, 75.0),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let drawing = d.add_page_of(&PageKind::Drawing).unwrap();
    assert_eq!(last_label(&d), "add_page");
    assert_eq!(drawing.name, "Page 2");
    assert_eq!(drawing.size, DocumentSize::from_mm(125.0, 75.0));
    let sketch = d.add_page_of(&PageKind::Sketch).unwrap();
    assert_eq!(last_label(&d), "add_sketch_page");
    assert_eq!(sketch.name, "Sketch 1");
    assert_eq!(
        sketch.size,
        DocumentSize::from_mm(125.0, 75.0),
        "no sketch page yet: the last drawing page's size"
    );
    assert_eq!(sketch.kind, PageKind::Sketch);
    let right = drawing.origin.x + drawing.size.width.as_mm();
    assert!(sketch.origin.x > right, "right of the last page");
    assert_eq!(sketch.origin.y, drawing.origin.y);
    d.resize_page(
        &sketch.id,
        DocumentSize::from_mm(80.0, 80.0),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let next = d.add_page_of(&PageKind::Sketch).unwrap();
    assert_eq!(next.name, "Sketch 2");
    assert_eq!(next.size, DocumentSize::from_mm(80.0, 80.0));
    let page = d.add_page_of(&PageKind::Drawing).unwrap();
    assert_eq!(
        page.size,
        DocumentSize::from_mm(125.0, 75.0),
        "the last drawing page"
    );
    assert_eq!(d.pages().len(), 5);
    assert!(
        d.add_page_of(&PageKind::Unknown("template".into()))
            .is_err()
    );
    undo_last(&d);
    assert_eq!(d.pages().len(), 4);
}

#[test]
fn only_a_sketch_page_holds_a_sketch() {
    let d = Document::new(1);
    let before = d.version();
    assert!(
        d.create_page_sketch("first", &three_lines(), &Style::default())
            .is_err()
    );
    assert_eq!(d.version(), before);
    let page = d.add_page_of(&PageKind::Sketch).unwrap();
    let id = d
        .create_page_sketch(&page.id, &three_lines(), &Style::default())
        .unwrap();
    assert_eq!(d.page_sketch(&page.id), Some(id));
    let centre = d
        .sketch(id)
        .unwrap()
        .placement
        .apply_point(Point::new(0.0, 0.0));
    assert_eq!(
        centre,
        Point::new(
            page.origin.x + page.size.width.as_mm() / 2.0,
            page.origin.y + page.size.height.as_mm() / 2.0
        )
    );
}

// ---- criteria 4, 14, 16: laser pages ------------------------------------------------------

#[test]
fn a_profile_change_keeps_the_origin_corner_and_the_placed_content_in_the_plane() {
    let d = Document::new(1);
    let generic = profile("generic-laser", "bottom_left");
    let page = d.add_output_page(laser(&generic, 400.0, 300.0)).unwrap();
    let id = page.kind.manufacturing().unwrap();
    // A placement, as an older file holds it.
    let loro = raw(&d);
    let tree = loro.get_tree("paths");
    let node = tree
        .children(TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
        .unwrap();
    add_raw_placement(&loro, node, "first", [1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
    loro.commit();
    let d = unpack(2, &container(&loro, CURRENT_FORMAT_VERSION)).unwrap();
    let own = rect(&d, page.origin.x + 50.0, page.origin.y + 50.0);
    let own_before = d.object(own).unwrap();
    let before = d.manufacturing(id).unwrap();
    let global = |m: &curvyo_document_core::ManufacturingSnapshot| {
        let p = m.page.as_ref().unwrap();
        let local = m.placements[0].matrix.apply_point(Point::new(0.0, 0.0));
        Point::new(p.origin.x + local.x, p.origin.y + local.y)
    };
    let placed = global(&before);
    let big = profile("big", "bottom_left");
    let corner = PageCorner {
        right: false,
        bottom: true,
    };
    let steps = d.own_step_count();
    assert!(
        d.set_manufacturing_profile(
            id,
            &big,
            DocumentSize::from_mm(600.0, 400.0),
            corner,
            ProfileChange::Pick,
            &curvyo_document_core::PageContents::new()
        )
        .unwrap()
    );
    assert_eq!(d.own_step_count(), steps + 1);
    let after = d.manufacturing(id).unwrap();
    let frame = after.page.clone().unwrap();
    assert_eq!(frame.size, DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(frame.origin.x, page.origin.x);
    assert!(
        (frame.origin.y + 400.0 - (page.origin.y + 300.0)).abs() < TOL,
        "the bottom-left corner keeps its place"
    );
    let moved = global(&after);
    assert!((moved.x - placed.x).abs() < TOL && (moved.y - placed.y).abs() < TOL);
    assert_eq!(
        d.object(own).unwrap(),
        own_before,
        "own items are not written"
    );
    undo_last(&d);
    assert_eq!(d.manufacturing(id).unwrap(), before);
    // A top-right corner grows the frame left and down.
    let tr = PageCorner {
        right: true,
        bottom: false,
    };
    d.set_manufacturing_profile(
        id,
        &big,
        DocumentSize::from_mm(600.0, 400.0),
        tr,
        ProfileChange::Pick,
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let frame = d.manufacturing(id).unwrap().page.unwrap();
    assert!((frame.origin.x + 600.0 - (page.origin.x + 400.0)).abs() < TOL);
    assert_eq!(frame.origin.y, page.origin.y);
}

#[test]
fn deleting_a_laser_page_keeps_its_own_items_and_undo_restores_the_machine_data() {
    let d = Document::new(1);
    let generic = profile("generic-laser", "bottom_left");
    let page = d.add_output_page(laser(&generic, 400.0, 300.0)).unwrap();
    let id = page.kind.manufacturing().unwrap();
    d.add_fixture(id, GRID, "Engraved grid", &grid(10.0))
        .unwrap();
    let own = rect(&d, page.origin.x + 5.0, page.origin.y + 5.0);
    let full = d.manufacturing(id).unwrap();
    let steps = d.own_step_count();
    d.delete_page(&page.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    assert!(d.manufacturings().is_empty());
    assert!(d.object(own).is_some(), "own items stay where they lay");
    assert_eq!(d.pages().len(), 1);
    undo_last(&d);
    assert_eq!(d.manufacturing(id).unwrap(), full);
    assert_eq!(d.pages()[1], page);
}

#[test]
fn an_unknown_or_output_page_refuses_resize_and_fit_but_moves() {
    let d = unpack(
        1,
        &std::fs::read(fixture_path(UNKNOWN)).unwrap_or_else(|_| {
            write_goldens();
            std::fs::read(fixture_path(UNKNOWN)).unwrap()
        }),
    )
    .unwrap();
    let unknown = d
        .pages()
        .into_iter()
        .find(|p| matches!(p.kind, PageKind::Unknown(_)))
        .unwrap();
    assert!(matches!(
        d.resize_page(
            &unknown.id,
            DocumentSize::from_mm(50.0, 50.0),
            &curvyo_document_core::PageContents::new()
        ),
        Err(PageError::FixedSize)
    ));
    assert!(matches!(
        d.fit_page_to_content(
            &unknown.id,
            (Point::new(0.0, 0.0), Point::new(1.0, 1.0)),
            &curvyo_document_core::PageContents::new()
        ),
        Err(PageError::FixedSize)
    ));
    assert!(
        d.move_page(
            &unknown.id,
            Vec2::new(5.0, 0.0),
            &curvyo_document_core::PageContents::new()
        )
        .unwrap()
    );
    assert!(d.rename_page(&unknown.id, "Kept").unwrap());
}

// ---- criterion 20: format 37 and its goldens ----------------------------------------------

fn add_raw_placement(loro: &LoroDoc, node: loro::TreeID, page: &str, matrix: [f64; 6]) {
    let tree = loro.get_tree("paths");
    let child = tree.create(TreeParentId::Node(node)).unwrap();
    let meta = tree.get_meta(child).unwrap();
    meta.insert("shape", "placement").unwrap();
    let matrix: Vec<LoroValue> = matrix.into_iter().map(LoroValue::Double).collect();
    let link: std::collections::HashMap<String, LoroValue> = [
        ("page".to_owned(), LoroValue::from(page)),
        ("matrix".to_owned(), LoroValue::from(matrix)),
    ]
    .into();
    meta.insert("placement_link", LoroValue::from(link))
        .unwrap();
}

/// Two drawing pages, a sketch page with three constrained lines, two laser pages with different
/// profiles, one with an engraved grid that is hidden (criterion 20).
fn golden_document() -> Document {
    let d = Document::new(1);
    let _ = rect(&d, 10.0, 10.0);
    let second = d.add_page();
    let _ = rect(&d, second.origin.x + 10.0, second.origin.y + 10.0);
    let sketch = d.add_page_of(&PageKind::Sketch).unwrap();
    let generic = profile("generic-laser", "bottom_left");
    let k40 = profile("k40", "top_left");
    d.add_output_page(laser(&generic, 400.0, 300.0)).unwrap();
    // A step between the two adds, so no two history rows in a row carry one label.
    d.create_page_sketch(&sketch.id, &three_lines(), &Style::default())
        .unwrap();
    let other = d.add_output_page(laser(&k40, 300.0, 200.0)).unwrap();
    let id = other.kind.manufacturing().unwrap();
    let grid_id = d
        .add_fixture(id, GRID, "Engraved grid", &grid(10.0))
        .unwrap();
    d.set_fixture_visible(grid_id, false).unwrap();
    d
}

/// The golden with the second page's kind register replaced by `value`.
fn with_kind(value: LoroValue) -> Vec<u8> {
    let d = Document::new(1);
    let page = d.add_page();
    let loro = raw(&d);
    loro.get_map("root")
        .insert(&format!("page.{}.kind", page.id), value)
        .unwrap();
    loro.commit();
    container(&loro, CURRENT_FORMAT_VERSION)
}

/// A format-36 file: an output page with a placement and a hidden fixture at (0, 0), a drawing
/// page with a rectangle, its owning sketch and an external clone of it (ADR 0042 §10).
fn migration_file() -> Vec<u8> {
    let d = Document::new(1);
    let _ = rect(&d, 10.0, 10.0);
    // The sketch is made on a sketch page lying on page 1, then its claim moves to page 1.
    let holder = d.add_page_of(&PageKind::Sketch).unwrap();
    d.move_page(
        &holder.id,
        Vec2::new(-holder.origin.x, -holder.origin.y),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let sketch = d
        .create_page_sketch(&holder.id, &three_lines(), &Style::default())
        .unwrap();
    let clone = d.linked_clone(&[sketch], &[]).unwrap()[0];
    d.translate_objects(&[clone], Vec2::new(0.0, 160.0))
        .unwrap();
    let generic = profile("generic-laser", "bottom_left");
    let page = d.add_output_page(laser(&generic, 600.0, 400.0)).unwrap();
    let id = page.kind.manufacturing().unwrap();
    let hidden = d
        .add_fixture(id, GRID, "Engraved grid", &grid(25.0))
        .unwrap();
    d.set_fixture_visible(hidden, false).unwrap();
    let loro = raw(&d);
    let root = loro.get_map("root");
    let owner = root
        .get(&format!("page.{}.sketch_owner", holder.id))
        .unwrap()
        .get_deep_value();
    let keys: Vec<String> = root
        .keys()
        .map(|k| k.to_string())
        .filter(|k| k.starts_with(&format!("page.{}.", holder.id)))
        .collect();
    for key in keys {
        root.delete(&key).unwrap();
    }
    root.insert("page.first.sketch_owner", owner).unwrap();
    let frame: std::collections::HashMap<String, LoroValue> = [
        ("x".to_owned(), LoroValue::Double(0.0)),
        ("y".to_owned(), LoroValue::Double(0.0)),
        ("width".to_owned(), LoroValue::Double(600.0)),
        ("height".to_owned(), LoroValue::Double(400.0)),
    ]
    .into();
    root.insert(&format!("page.{}.frame", page.id), LoroValue::from(frame))
        .unwrap();
    root.insert(&format!("page.{}.order", page.id), 0.0)
        .unwrap();
    let tree = loro.get_tree("paths");
    let sketch_node =
        loro::TreeID::new(sketch_peer_counter(sketch).0, sketch_peer_counter(sketch).1);
    tree.get_meta(sketch_node)
        .unwrap()
        .insert("sketch_page", "first")
        .unwrap();
    let node = loro::TreeID::new(sketch_peer_counter(id).0, sketch_peer_counter(id).1);
    add_raw_placement(&loro, node, "first", [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    loro.commit();
    container(&loro, 36)
}

/// The `(peer, counter)` of a node, read through its `document.json` form.
fn sketch_peer_counter(id: NodeId) -> (u64, i32) {
    let value = serde_json::to_value(id).unwrap();
    (
        value["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(value["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn write_goldens() {
    std::fs::write(
        fixture_path(GOLDEN),
        pack(&golden_document(), "0.1.0").unwrap(),
    )
    .unwrap();
    std::fs::write(fixture_path(DAMAGED), with_kind(LoroValue::Double(3.0))).unwrap();
    std::fs::write(
        fixture_path(UNKNOWN),
        with_kind(LoroValue::from("template")),
    )
    .unwrap();
    std::fs::write(fixture_path(MIGRATION), migration_file()).unwrap();
}

fn read_golden(name: &str) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        write_goldens();
    }
    std::fs::read(fixture_path(name)).unwrap()
}

#[test]
fn the_page_kinds_golden_reads_back_every_page_and_the_machine_data_identical() {
    let bytes = read_golden(GOLDEN);
    let d = unpack(2, &bytes).unwrap();
    let fresh = golden_document();
    assert_eq!(d.pages(), fresh.pages());
    let kinds: Vec<String> = d
        .pages()
        .iter()
        .map(|p| {
            serde_json::to_value(&p.kind)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "drawing",
            "drawing",
            "sketch",
            "curvyo.laser",
            "curvyo.laser"
        ]
    );
    assert_eq!(d.manufacturings(), fresh.manufacturings());
    assert!(!d.manufacturings()[1].fixtures[0].visible);
    let sketch_page = &d.pages()[2];
    let sketch = d.page_sketch(&sketch_page.id).unwrap();
    assert_eq!(d.sketch(sketch).unwrap().graph, three_lines());
    assert_eq!(d.read_objects().objects, fresh.read_objects().objects);
    let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(again.pages(), d.pages());
    assert_eq!(again.manufacturings(), d.manufacturings());
    assert_eq!(d.own_step_count(), 0, "opening writes nothing");
}

#[test]
fn a_damaged_kind_reads_as_drawing_and_is_not_rewritten() {
    let bytes = read_golden(DAMAGED);
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.pages().len(), 2);
    assert_eq!(d.pages()[1].kind, PageKind::Drawing);
    let saved = pack(&d, "0.1.0").unwrap();
    let key = format!("page.{}.kind", d.pages()[1].id);
    assert_eq!(
        stored_loro(&saved)
            .get_map("root")
            .get(&key)
            .unwrap()
            .get_deep_value(),
        LoroValue::Double(3.0)
    );
}

#[test]
fn an_unknown_kind_is_listed_kept_and_saved_back_unchanged() {
    let bytes = read_golden(UNKNOWN);
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.pages()[1].kind, PageKind::Unknown("template".into()));
    let saved = pack(&d, "0.1.0").unwrap();
    assert_eq!(
        stored_loro(&saved).oplog_vv(),
        stored_loro(&bytes).oplog_vv(),
        "opening and saving wrote nothing"
    );
}

#[test]
fn a_format_36_file_converts_on_open_without_a_step_and_looks_the_same() {
    let bytes = read_golden(MIGRATION);
    let stored = stored_loro(&bytes);
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "no undo step");
    let pages = d.pages();
    assert_eq!(pages.len(), 3);
    assert_eq!(pages[0].id, "first");
    assert!(matches!(pages[1].kind, PageKind::Output { .. }));
    assert_eq!(
        pages[1].origin,
        Point::new(239.7, 0.0),
        "the Add page position"
    );
    assert_eq!(pages[1].size, DocumentSize::from_mm(600.0, 400.0));
    let made = &d.manufacturings()[0];
    assert_eq!(made.placements.len(), 1);
    assert_eq!(made.placements[0].matrix, Affine::IDENTITY);
    assert_eq!(made.fixtures.len(), 1);
    assert!(!made.fixtures[0].visible);
    assert_eq!(pages[2].id, "first-sketch");
    assert_eq!(pages[2].name, "Page 1 sketch");
    assert_eq!(pages[2].kind, PageKind::Sketch);
    assert_eq!(pages[2].size, pages[0].size);
    assert_eq!(pages[2].origin, Point::new(239.7 + 600.0 + 60.0, 0.0));
    assert!(d.page_sketch("first").is_none());
    let sketch = d.page_sketch("first-sketch").unwrap();
    let centre = d
        .sketch(sketch)
        .unwrap()
        .placement
        .apply_point(Point::new(0.0, 0.0));
    assert!((centre.x - (pages[2].origin.x + 105.0)).abs() < TOL);
    // Every outline that was drawn is drawn again: the old sketch's outline by the new clone, the
    // external clone where it was.
    let before = unpack_without_migration(&stored);
    let outline = |o: &ObjectSnapshot| match o {
        ObjectSnapshot::Clone(c) => c.resolved.clone(),
        ObjectSnapshot::Sketch(s) => s.resolved.clone(),
        other => panic!("unexpected {other:?}"),
    };
    let old: Vec<_> = before
        .iter()
        .filter(|o| !matches!(o, ObjectSnapshot::Primitive(_)))
        .map(|o| outline(o).anchors)
        .collect();
    let now = d.read_objects().objects.to_vec();
    for anchors in &old {
        assert!(
            now.iter()
                .filter(|o| !matches!(o, ObjectSnapshot::Primitive(_) | ObjectSnapshot::Sketch(_)))
                .any(|o| close(&outline(o).anchors, anchors)),
            "an outline of the old file is still drawn"
        );
    }
    // Saved, the conversion is in the file and does not run again.
    let saved = pack(&d, "0.1.0").unwrap();
    let again = unpack(3, &saved).unwrap();
    assert_eq!(again.pages(), pages);
    assert_eq!(again.read_objects().objects.len(), now.len());
}

fn close(
    a: &[curvyo_document_core::AnchorSnapshot],
    b: &[curvyo_document_core::AnchorSnapshot],
) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(p, q)| {
            (p.point.x - q.point.x).abs() < 1e-6 && (p.point.y - q.point.y).abs() < 1e-6
        })
}

/// The objects of the format-36 file as that build read them: the same snapshot opened as a
/// file of this format, so no conversion runs.
fn unpack_without_migration(loro: &LoroDoc) -> Vec<ObjectSnapshot> {
    unpack(4, &container(loro, CURRENT_FORMAT_VERSION))
        .unwrap()
        .read_objects()
        .objects
        .to_vec()
}

#[test]
fn the_older_laser_and_profile_goldens_open_through_the_conversion() {
    for name in ["laser_workbench_v36.curvyo", "machine_profile_v36.curvyo"] {
        let d = unpack(2, &std::fs::read(fixture_path(name)).unwrap()).unwrap();
        let pages = d.pages();
        assert_eq!(pages.len(), 2, "{name}");
        assert!(matches!(pages[1].kind, PageKind::Output { .. }), "{name}");
        assert!(pages[1].origin.x > pages[0].size.width.as_mm(), "{name}");
        assert_eq!(d.own_step_count(), 0, "{name}");
    }
}
