//! Independent acceptance tests for `specs/0098-page-kinds` at the document model and file format
//! (ADR 0042): Add page of each kind (criteria 2 to 4), the fixed kind (5), the laser page listed
//! and in the plane (10), placements from older files (12), the corner-anchored resize (14),
//! Delete page on a laser page (16), the kind read with unknown, damaged and owned registers
//! (18 c, d, 20), the format-36 conversion (19) and the goldens (19, 20).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::case_sensitive_file_extension_comparisons,
    missing_docs
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    AnchorSnapshot, CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewOutputPage,
    NodeId, ObjectSnapshot, Page, PageCorner, PageError, PageKind, Point, ProfileChange,
    RectBounds, Style, Vec2, pack, unpack,
};
use curvyo_sketch_core::{Constraint, Position, Relation, Sketch};
use loro::{LoroDoc, LoroValue, TreeParentId};

const LASER: &str = "curvyo.laser";
const GRID: &str = "curvyo.laser/engraved_grid";
const TOL: f64 = 1e-6;

// ---- helpers ------------------------------------------------------------------------------------

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn copy(id: &str, origin: &str) -> FixtureParams {
    serde_json::from_value(serde_json::json!({
        "id": id, "name": id, "origin": origin,
        "max_feed_mm_per_min": 6000, "max_power": 1000, "laser_mode": true,
    }))
    .unwrap()
}

fn grid(spacing: f64) -> FixtureParams {
    serde_json::from_value(serde_json::json!({
        "spacing": {"mm": spacing}, "line_width": {"mm": 0.1},
        "origin_x": {"mm": 0.0}, "origin_y": {"mm": 0.0},
    }))
    .unwrap()
}

fn add_laser(d: &Document, profile: &FixtureParams, w: f64, h: f64) -> Page {
    d.add_output_page(NewOutputPage {
        process: LASER,
        label: "Laser",
        profile: Some(profile),
        work_area: DocumentSize::from_mm(w, h),
    })
    .unwrap()
}

fn rect(d: &Document, x: f64, y: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + 30.0, y + 20.0),
    ))
}

fn page(d: &Document, id: &str) -> Page {
    d.pages().into_iter().find(|p| p.id == id).unwrap()
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo_last(d: &Document) {
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
}

fn overlap(a: &Page, b: &Page) -> bool {
    let ax1 = a.origin.x + a.size.width.as_mm();
    let ay1 = a.origin.y + a.size.height.as_mm();
    let bx1 = b.origin.x + b.size.width.as_mm();
    let by1 = b.origin.y + b.size.height.as_mm();
    a.origin.x < bx1 - TOL
        && b.origin.x < ax1 - TOL
        && a.origin.y < by1 - TOL
        && b.origin.y < ay1 - TOL
}

fn assert_no_overlap(pages: &[Page]) {
    for (i, a) in pages.iter().enumerate() {
        for b in &pages[i + 1..] {
            assert!(!overlap(a, b), "{} overlaps {}", a.name, b.name);
        }
    }
}

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

fn stored(bytes: &[u8]) -> LoroDoc {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut data = Vec::new();
    archive
        .by_name("document.loro")
        .unwrap()
        .read_to_end(&mut data)
        .unwrap();
    let loro = LoroDoc::new();
    loro.import(&data).unwrap();
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

fn root_value(loro: &LoroDoc, key: &str) -> Option<LoroValue> {
    loro.get_map("root")
        .get(key)
        .and_then(|v| v.into_value().ok())
}

fn manufacturing_nodes(loro: &LoroDoc) -> Vec<loro::TreeID> {
    let tree = loro.get_tree("paths");
    tree.children(TreeParentId::Root)
        .unwrap()
        .into_iter()
        .filter(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
        .collect()
}

fn add_raw_placement(loro: &LoroDoc, node: loro::TreeID, page: &str) {
    let tree = loro.get_tree("paths");
    let child = tree.create(TreeParentId::Node(node)).unwrap();
    let meta = tree.get_meta(child).unwrap();
    meta.insert("shape", "placement").unwrap();
    let matrix: Vec<LoroValue> = [1.0, 0.0, 0.0, 1.0, 5.0, 7.0]
        .into_iter()
        .map(LoroValue::Double)
        .collect();
    let link: std::collections::HashMap<String, LoroValue> = [
        ("page".to_owned(), LoroValue::from(page)),
        ("matrix".to_owned(), LoroValue::from(matrix)),
    ]
    .into();
    meta.insert("placement_link", LoroValue::from(link))
        .unwrap();
}

fn tree_id(id: NodeId) -> loro::TreeID {
    let value = serde_json::to_value(id).unwrap();
    loro::TreeID::new(
        value["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(value["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn outline_of(o: &ObjectSnapshot) -> Option<Vec<AnchorSnapshot>> {
    match o {
        ObjectSnapshot::Clone(c) => Some(c.resolved.anchors.clone()),
        ObjectSnapshot::Sketch(s) => Some(s.resolved.anchors.clone()),
        ObjectSnapshot::Path(p) => Some(p.anchors.clone()),
        _ => None,
    }
}

fn same(a: &[AnchorSnapshot], b: &[AnchorSnapshot]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(p, q)| {
            (p.point.x - q.point.x).abs() < TOL && (p.point.y - q.point.y).abs() < TOL
        })
}

// ---- criteria 2 to 4: Add page of each kind ------------------------------------------------------

#[test]
fn ac2_add_drawing_page_takes_the_size_of_the_last_drawing_page_not_of_the_last_page() {
    let d = Document::new(1);
    let p2 = d.add_page_of(&PageKind::Drawing).unwrap();
    assert_eq!(p2.name, "Page 2");
    assert_eq!(p2.kind, PageKind::Drawing);
    d.resize_page(
        &p2.id,
        DocumentSize::from_mm(125.0, 75.0),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let s = d.add_page_of(&PageKind::Sketch).unwrap();
    d.resize_page(
        &s.id,
        DocumentSize::from_mm(50.0, 40.0),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    let steps = d.own_step_count();
    let p3 = d.add_page_of(&PageKind::Drawing).unwrap();
    assert_eq!(d.own_step_count(), steps + 1, "one undo step");
    assert_eq!(last_label(&d), "add_page");
    assert_eq!(p3.name, "Page 3");
    assert_eq!(p3.size, DocumentSize::from_mm(125.0, 75.0));
    let all = d.pages();
    assert_eq!(all.last().unwrap().id, p3.id, "last in the list");
    let before = &all[all.len() - 2];
    assert!(
        p3.origin.x >= before.origin.x + before.size.width.as_mm() - TOL,
        "right of the last page"
    );
    assert_no_overlap(&all);
    undo_last(&d);
    assert!(d.pages().iter().all(|p| p.id != p3.id));
}

#[test]
fn ac2_a_new_document_starts_with_one_drawing_page_and_add_page_default_size() {
    let d = Document::new(1);
    let pages = d.pages();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].kind, PageKind::Drawing);
    let p = d.add_page();
    assert_eq!(p.kind, PageKind::Drawing);
    assert_eq!(p.size, pages[0].size);
}

#[test]
fn ac2_names_never_repeat_an_existing_name() {
    let d = Document::new(1);
    let p2 = d.add_page_of(&PageKind::Drawing).unwrap();
    d.rename_page(&p2.id, "Page 3").unwrap();
    let next = d.add_page_of(&PageKind::Drawing).unwrap();
    let names: Vec<_> = d.pages().into_iter().map(|p| p.name).collect();
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "{names:?}");
    assert!(next.name.starts_with("Page "));
}

#[test]
fn ac3_sketch_page_size_falls_back_from_last_sketch_to_last_drawing_page() {
    let d = Document::new(1);
    d.resize_page(
        "first",
        DocumentSize::from_mm(125.0, 75.0),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    let steps = d.own_step_count();
    let s1 = d.add_page_of(&PageKind::Sketch).unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), "add_sketch_page");
    assert_eq!(s1.name, "Sketch 1");
    assert_eq!(s1.kind, PageKind::Sketch);
    assert_eq!(
        s1.size,
        DocumentSize::from_mm(125.0, 75.0),
        "last drawing page"
    );
    assert_eq!(
        d.page_sketch(&s1.id),
        None,
        "lazy: no node before the first edit"
    );
    d.resize_page(
        &s1.id,
        DocumentSize::from_mm(60.0, 50.0),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let s2 = d.add_page_of(&PageKind::Sketch).unwrap();
    assert_eq!(s2.name, "Sketch 2");
    assert_eq!(
        s2.size,
        DocumentSize::from_mm(60.0, 50.0),
        "last sketch page"
    );
    assert_no_overlap(&d.pages());
}

#[test]
fn ac3_the_sketch_of_a_sketch_page_has_its_origin_at_the_page_centre() {
    let d = Document::new(1);
    let s = d.add_page_of(&PageKind::Sketch).unwrap();
    let id = d
        .create_page_sketch(&s.id, &three_lines(), &Style::default())
        .unwrap();
    let zero = d
        .sketch(id)
        .unwrap()
        .placement
        .apply_point(Point::new(0.0, 0.0));
    assert!((zero.x - (s.origin.x + s.size.width.as_mm() / 2.0)).abs() < TOL);
    assert!((zero.y - (s.origin.y + s.size.height.as_mm() / 2.0)).abs() < TOL);
    assert_eq!(d.page_sketch(&s.id), Some(id));
    assert_eq!(d.sketch_page(id).as_deref(), Some(s.id.as_str()));
}

#[test]
fn ac3_only_a_sketch_page_holds_a_sketch() {
    let d = Document::new(1);
    let l = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    let steps = d.own_step_count();
    assert!(
        d.create_page_sketch("first", &three_lines(), &Style::default())
            .is_err()
    );
    assert!(
        d.create_page_sketch(&l.id, &three_lines(), &Style::default())
            .is_err()
    );
    assert_eq!(d.own_step_count(), steps, "a refusal writes nothing");
}

#[test]
fn ac4_add_laser_page_has_the_work_area_no_fixture_no_placement_and_one_step() {
    let d = Document::new(1);
    let steps = d.own_step_count();
    let l = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), "add_output_page");
    assert_eq!(l.name, "Laser 1");
    assert_eq!(l.size, DocumentSize::from_mm(400.0, 300.0));
    let PageKind::Output {
        manufacturing,
        process,
    } = &l.kind
    else {
        panic!("an output page")
    };
    assert_eq!(process, LASER);
    let m = d.manufacturing(*manufacturing).unwrap();
    assert!(m.fixtures.is_empty());
    assert!(m.placements.is_empty());
    assert_eq!(m.page.as_ref().unwrap().id, l.id);
    assert_eq!(m.profile.unwrap(), copy("generic-laser", "bottom_left"));
    let first = page(&d, "first");
    assert!(l.origin.x >= first.origin.x + first.size.width.as_mm() - TOL);
    let first_step = d.own_steps().last().unwrap().step;
    let l2 = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    assert_eq!(l2.name, "Laser 2");
    assert_no_overlap(&d.pages());
    undo_last(&d);
    d.undo_step(first_step).unwrap();
    assert_eq!(d.pages().len(), 1);
    assert!(
        d.manufacturings().is_empty(),
        "undo removes the machine data too"
    );
}

#[test]
fn ac4_a_work_area_outside_the_page_limits_is_refused_without_writing() {
    let d = Document::new(1);
    let steps = d.own_step_count();
    let bad = d.add_output_page(NewOutputPage {
        process: LASER,
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(0.0, 300.0),
    });
    assert!(bad.is_err());
    assert_eq!(d.own_step_count(), steps);
    assert_eq!(d.pages().len(), 1);
    assert!(d.manufacturings().is_empty());
}

// ---- criterion 5: the kind is fixed -------------------------------------------------------------

#[test]
fn ac5_no_command_creates_an_output_or_unknown_page_through_add_page_or_changes_a_kind() {
    let d = Document::new(1);
    let l = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    let steps = d.own_step_count();
    assert!(matches!(
        d.add_page_of(&PageKind::Unknown("template".into())),
        Err(PageError::FixedSize)
    ));
    assert!(d.add_page_of(&l.kind).is_err());
    assert_eq!(d.own_step_count(), steps);
    let s = d.add_page_of(&PageKind::Sketch).unwrap();
    d.rename_page(&s.id, "Renamed").unwrap();
    d.resize_page(
        &s.id,
        DocumentSize::from_mm(90.0, 90.0),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    d.reorder_page(&s.id, 0).unwrap();
    d.move_page(
        &s.id,
        Vec2::new(0.0, 500.0),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    assert_eq!(page(&d, &s.id).kind, PageKind::Sketch);
    d.rename_page(&l.id, "Bed").unwrap();
    d.reorder_page(&l.id, 0).unwrap();
    assert_eq!(page(&d, &l.id).kind, l.kind);
    assert_eq!(page(&d, "first").kind, PageKind::Drawing);
}

#[test]
fn ac5_the_kind_survives_save_and_reopen_and_undo_of_later_edits() {
    let d = Document::new(1);
    let s = d.add_page_of(&PageKind::Sketch).unwrap();
    d.rename_page(&s.id, "Bracket").unwrap();
    undo_last(&d);
    let again = unpack(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(page(&again, &s.id).kind, PageKind::Sketch);
}

// ---- criterion 10: listed, in the plane, reorder and rename -------------------------------------

#[test]
fn ac10_the_laser_page_is_listed_in_its_place_reordered_and_renamed_like_any_page() {
    let d = Document::new(1);
    let l = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    let p = d.add_page_of(&PageKind::Drawing).unwrap();
    let ids: Vec<_> = d.pages().into_iter().map(|p| p.id).collect();
    assert_eq!(ids, vec!["first".to_owned(), l.id.clone(), p.id.clone()]);
    assert!(d.reorder_page(&l.id, 0).unwrap());
    assert_eq!(d.pages()[0].id, l.id);
    assert!(d.rename_page(&l.id, "Bed").unwrap());
    assert_eq!(page(&d, &l.id).name, "Bed");
    assert_eq!(d.page_kind(&l.id), Some(l.kind.clone()));
    // A frame move is allowed and carries the laser page (ADR 0042 §3).
    let own = rect(&d, l.origin.x + 10.0, l.origin.y + 10.0);
    assert!(
        d.move_page(
            &l.id,
            Vec2::new(0.0, 400.0),
            &curvyo_document_core::PageContents::from([(l.id.clone(), vec![own])])
        )
        .unwrap()
    );
    let moved = page(&d, &l.id);
    assert!((moved.origin.y - (l.origin.y + 400.0)).abs() < TOL);
    let ObjectSnapshot::Primitive(_) = d.object(own).unwrap() else {
        panic!("rect")
    };
}

// ---- criterion 12: a placement from an older file -----------------------------------------------

#[test]
fn ac12_deleting_a_placement_removes_only_the_reference_and_undo_restores_it() {
    let d = unpack(3, &fixture("laser_workbench_v36.curvyo")).unwrap();
    let m = d.manufacturings().into_iter().next().unwrap();
    assert!(!m.placements.is_empty(), "the golden holds a placement");
    let placement = m.placements[0].id;
    let target = m.placements[0].page.clone();
    let pages = d.pages().len();
    let steps = d.own_step_count();
    d.delete_placement(placement).unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    let after = d.manufacturing(m.id).unwrap();
    assert_eq!(after.placements.len(), m.placements.len() - 1);
    assert_eq!(d.pages().len(), pages, "the placed drawing page stays");
    assert!(d.pages().iter().any(|p| p.id == target));
    undo_last(&d);
    assert_eq!(d.manufacturing(m.id).unwrap().placements, m.placements);
    assert!(
        d.delete_placement(m.id).is_err(),
        "a manufacturing is no placement"
    );
}

// ---- criterion 14: size from the profile about the origin corner ---------------------------------

fn resize_case(origin: &str, corner: PageCorner, expect_dx: f64, expect_dy: f64) {
    let d = Document::new(1);
    let l = add_laser(&d, &copy("generic-laser", origin), 400.0, 300.0);
    let id = l.kind.manufacturing().unwrap();
    let own = rect(&d, l.origin.x + 50.0, l.origin.y + 50.0);
    let own_before = d.object(own).unwrap();
    let steps = d.own_step_count();
    assert!(
        d.set_manufacturing_profile(
            id,
            &copy("big", origin),
            DocumentSize::from_mm(600.0, 400.0),
            corner,
            ProfileChange::Pick,
            &curvyo_document_core::PageContents::new(),
        )
        .unwrap()
    );
    assert_eq!(d.own_step_count(), steps + 1, "one undo step");
    let after = page(&d, &l.id);
    assert_eq!(after.size, DocumentSize::from_mm(600.0, 400.0));
    assert!(
        (after.origin.x - (l.origin.x + expect_dx)).abs() < TOL,
        "{origin} x"
    );
    assert!(
        (after.origin.y - (l.origin.y + expect_dy)).abs() < TOL,
        "{origin} y"
    );
    // The anchor corner keeps its place in the plane.
    let cx = |p: &Page| {
        p.origin.x
            + if corner.right {
                p.size.width.as_mm()
            } else {
                0.0
            }
    };
    let cy = |p: &Page| {
        p.origin.y
            + if corner.bottom {
                p.size.height.as_mm()
            } else {
                0.0
            }
    };
    assert!((cx(&after) - cx(&l)).abs() < TOL && (cy(&after) - cy(&l)).abs() < TOL);
    assert_eq!(
        d.object(own).unwrap(),
        own_before,
        "own items are not written"
    );
    undo_last(&d);
    assert_eq!(page(&d, &l.id), l);
}

#[test]
fn ac14_bottom_left_origin_grows_up_and_right() {
    resize_case(
        "bottom_left",
        PageCorner {
            right: false,
            bottom: true,
        },
        0.0,
        -100.0,
    );
}

#[test]
fn ac14_top_right_origin_grows_down_and_left() {
    resize_case(
        "top_right",
        PageCorner {
            right: true,
            bottom: false,
        },
        -200.0,
        0.0,
    );
}

#[test]
fn ac14_top_left_origin_keeps_the_frame_origin() {
    resize_case(
        "top_left",
        PageCorner {
            right: false,
            bottom: false,
        },
        0.0,
        0.0,
    );
}

#[test]
fn ac14_placements_keep_their_global_place_through_a_profile_resize() {
    let d0 = Document::new(1);
    let l = add_laser(&d0, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    let loro = stored(&pack(&d0, "0.1.0").unwrap());
    add_raw_placement(&loro, manufacturing_nodes(&loro)[0], "first");
    loro.commit();
    let d = unpack(2, &container(&loro, CURRENT_FORMAT_VERSION)).unwrap();
    let id = d.manufacturings()[0].id;
    let global = |d: &Document| {
        let m = d.manufacturing(id).unwrap();
        let p = m.page.unwrap();
        let local = m.placements[0].matrix.apply_point(Point::new(0.0, 0.0));
        Point::new(p.origin.x + local.x, p.origin.y + local.y)
    };
    let before = global(&d);
    d.set_manufacturing_profile(
        id,
        &copy("big", "bottom_left"),
        DocumentSize::from_mm(600.0, 400.0),
        PageCorner {
            right: false,
            bottom: true,
        },
        ProfileChange::Pick,
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let after = global(&d);
    assert!((after.x - before.x).abs() < TOL && (after.y - before.y).abs() < TOL);
    assert!((page(&d, &l.id).origin.y - (l.origin.y - 100.0)).abs() < TOL);
}

#[test]
fn ac14_an_unchanged_profile_writes_nothing() {
    let d = Document::new(1);
    let generic = copy("generic-laser", "bottom_left");
    let l = add_laser(&d, &generic, 400.0, 300.0);
    let steps = d.own_step_count();
    let changed = d
        .set_manufacturing_profile(
            l.kind.manufacturing().unwrap(),
            &generic,
            DocumentSize::from_mm(400.0, 300.0),
            PageCorner {
                right: false,
                bottom: true,
            },
            ProfileChange::Pick,
            &curvyo_document_core::PageContents::new(),
        )
        .unwrap();
    assert!(!changed);
    assert_eq!(d.own_step_count(), steps);
}

// ---- criteria 13, 18 d: size refusal -----------------------------------------------------------

#[test]
fn ac13_a_laser_page_refuses_a_size_command_but_drawing_and_sketch_pages_take_it() {
    let d = Document::new(1);
    let l = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    let s = d.add_page_of(&PageKind::Sketch).unwrap();
    let steps = d.own_step_count();
    assert!(matches!(
        d.resize_page(
            &l.id,
            DocumentSize::from_mm(100.0, 100.0),
            &curvyo_document_core::PageContents::new()
        ),
        Err(PageError::FixedSize)
    ));
    assert_eq!(d.own_step_count(), steps);
    assert_eq!(page(&d, &l.id).size, DocumentSize::from_mm(400.0, 300.0));
    assert!(
        d.resize_page(
            &s.id,
            DocumentSize::from_mm(100.0, 100.0),
            &curvyo_document_core::PageContents::new()
        )
        .unwrap()
    );
    assert!(
        d.resize_page(
            "first",
            DocumentSize::from_mm(100.0, 100.0),
            &curvyo_document_core::PageContents::new()
        )
        .unwrap()
    );
}

// ---- criterion 16: Delete page on a laser page --------------------------------------------------

#[test]
fn ac16_deleting_a_laser_page_takes_its_machine_data_keeps_own_items_and_undo_restores_all() {
    let d = Document::new(1);
    let l = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    let id = l.kind.manufacturing().unwrap();
    d.add_fixture(
        id,
        &curvyo_document_core::NewFixture {
            add: curvyo_document_core::FixtureAdd::Grid,
            kind: GRID,
            label: "Engraved grid",
            params: &grid(10.0),
            matrix: None,
        },
    )
    .unwrap();
    let own = rect(&d, l.origin.x + 20.0, l.origin.y + 20.0);
    let own_before = d.object(own).unwrap();
    let first_before = page(&d, "first");
    let m_before = d.manufacturing(id).unwrap();
    let steps = d.own_step_count();
    d.delete_page(&l.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    assert!(d.pages().iter().all(|p| p.id != l.id));
    assert!(
        d.manufacturings().is_empty(),
        "the machine copy and fixtures go"
    );
    assert_eq!(
        d.object(own).unwrap(),
        own_before,
        "own items stay where they lay"
    );
    assert_eq!(page(&d, "first"), first_before, "drawing pages untouched");
    undo_last(&d);
    assert_eq!(page(&d, &l.id), l);
    let m_after = d.manufacturing(id).unwrap();
    assert_eq!(m_after.fixtures.len(), m_before.fixtures.len());
    assert_eq!(m_after.profile, m_before.profile);
}

#[test]
fn ac16_any_page_may_be_deleted_while_one_other_remains() {
    let d = Document::new(1);
    let l = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    d.delete_page("first", &curvyo_document_core::PageContents::new())
        .unwrap();
    assert_eq!(d.pages().len(), 1);
    assert_eq!(d.pages()[0].id, l.id);
    assert!(matches!(
        d.delete_page(&l.id, &curvyo_document_core::PageContents::new()),
        Err(PageError::LastPage)
    ));
    assert_eq!(
        d.manufacturings().len(),
        1,
        "a refused delete writes nothing"
    );
}

#[test]
fn ac16_a_saved_file_after_deleting_a_laser_page_has_no_manufacturing() {
    let d = Document::new(1);
    let l = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    d.delete_page(&l.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    let again = unpack(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(again.pages().len(), 1);
    assert!(again.manufacturings().is_empty());
}

// ---- criteria 18 c, 20: the kind read -----------------------------------------------------------

fn with_raw_kind(value: impl Into<LoroValue>) -> (Vec<u8>, String) {
    let d = Document::new(1);
    let p = d.add_page_of(&PageKind::Drawing).unwrap();
    let loro = stored(&pack(&d, "0.1.0").unwrap());
    loro.get_map("root")
        .insert(&format!("page.{}.kind", p.id), value.into())
        .unwrap();
    loro.commit();
    (container(&loro, CURRENT_FORMAT_VERSION), p.id)
}

#[test]
fn ac18c_an_unknown_kind_is_listed_kept_fixed_size_and_saved_back_unchanged() {
    let (bytes, id) = with_raw_kind("template");
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(page(&d, &id).kind, PageKind::Unknown("template".into()));
    assert!(matches!(
        d.resize_page(
            &id,
            DocumentSize::from_mm(10.0, 10.0),
            &curvyo_document_core::PageContents::new()
        ),
        Err(PageError::FixedSize)
    ));
    assert!(d.rename_page(&id, "Kept").unwrap());
    assert!(d.reorder_page(&id, 0).unwrap());
    assert!(
        d.move_page(
            &id,
            Vec2::new(5.0, 5.0),
            &curvyo_document_core::PageContents::new()
        )
        .unwrap()
    );
    let saved = pack(&d, "0.1.0").unwrap();
    assert_eq!(
        root_value(&stored(&saved), &format!("page.{id}.kind")),
        Some(LoroValue::from("template"))
    );
    let again = unpack(3, &saved).unwrap();
    assert_eq!(page(&again, &id).kind, PageKind::Unknown("template".into()));
    assert!(
        again
            .delete_page(&id, &curvyo_document_core::PageContents::new())
            .is_ok(),
        "an unknown page may be deleted"
    );
}

#[test]
fn ac20_a_damaged_kind_reads_as_drawing_and_is_not_rewritten() {
    for value in [
        LoroValue::from(7_i64),
        LoroValue::from(true),
        LoroValue::Null,
    ] {
        let (bytes, id) = with_raw_kind(value.clone());
        let d = unpack(2, &bytes).unwrap();
        assert_eq!(page(&d, &id).kind, PageKind::Drawing, "{value:?}");
        assert_eq!(d.own_step_count(), 0);
        let saved = pack(&d, "0.1.0").unwrap();
        assert_eq!(
            root_value(&stored(&saved), &format!("page.{id}.kind")),
            Some(value.clone()),
            "not rewritten on read"
        );
    }
}

#[test]
fn adr0042_a_kind_register_on_an_owned_page_is_ignored() {
    let d = Document::new(1);
    let l = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    let loro = stored(&pack(&d, "0.1.0").unwrap());
    loro.get_map("root")
        .insert(&format!("page.{}.kind", l.id), "sketch")
        .unwrap();
    loro.commit();
    let d = unpack(2, &container(&loro, CURRENT_FORMAT_VERSION)).unwrap();
    assert!(matches!(page(&d, &l.id).kind, PageKind::Output { .. }));
}

#[test]
fn adr0042_a_claim_of_a_sketch_on_a_drawing_page_reads_as_no_owner() {
    let d = Document::new(1);
    let s = d.add_page_of(&PageKind::Sketch).unwrap();
    let sk = d
        .create_page_sketch(&s.id, &three_lines(), &Style::default())
        .unwrap();
    let loro = stored(&pack(&d, "0.1.0").unwrap());
    let root = loro.get_map("root");
    let claim = root_value(&loro, &format!("page.{}.sketch_owner", s.id)).unwrap();
    root.insert("page.first.sketch_owner", claim).unwrap();
    loro.commit();
    let d = unpack(2, &container(&loro, CURRENT_FORMAT_VERSION)).unwrap();
    assert_eq!(d.page_sketch("first"), None);
    assert_eq!(d.page_sketch(&s.id), Some(sk));
    assert_eq!(d.own_step_count(), 0, "never repaired on read");
}

// ---- criteria 19, 20: goldens and format -------------------------------------------------------

#[test]
// Bumped to 38 by `specs/0100-bring-content-in` (ADR 0045 §9), to 39 by
// `specs/0101-laser-operations` (ADR 0046 §4), to 40 by `specs/0102-laser-fixtures` (ADR 0049).
fn ac20_save_writes_format_38() {
    // Format 41 since `specs/0103-fixture-jobs` (ADR 0052 §6).
    assert_eq!(CURRENT_FORMAT_VERSION, 41);
    let d = Document::new(1);
    assert_eq!(manifest_version(&pack(&d, "0.1.0").unwrap()), 41);
}

#[test]
fn ac20_the_page_kinds_golden_holds_the_criterion_project() {
    let bytes = fixture("page_kinds_v37.curvyo");
    assert_eq!(manifest_version(&bytes), 37);
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0);
    let pages = d.pages();
    let count = |f: fn(&PageKind) -> bool| pages.iter().filter(|p| f(&p.kind)).count();
    assert_eq!(count(|k| *k == PageKind::Drawing), 2);
    assert_eq!(count(|k| *k == PageKind::Sketch), 1);
    assert_eq!(count(|k| matches!(k, PageKind::Output { .. })), 2);
    assert_no_overlap(&pages);
    let sketch_page = pages.iter().find(|p| p.kind == PageKind::Sketch).unwrap();
    let sk = d
        .page_sketch(&sketch_page.id)
        .expect("the sketch page holds its sketch");
    let graph = d.sketch(sk).unwrap().graph;
    let lines = graph
        .entities
        .values()
        .filter(|e| matches!(e, curvyo_sketch_core::Entity::Line { .. }))
        .count();
    assert_eq!(lines, 3, "three lines");
    assert!(!graph.constraints.is_empty(), "constrained");
    let ms = d.manufacturings();
    assert_eq!(ms.len(), 2);
    assert_ne!(ms[0].profile, ms[1].profile, "different profiles");
    assert_ne!(ms[0].work_area(), ms[1].work_area());
    let fixtures: Vec<_> = ms.iter().flat_map(|m| m.fixtures.iter()).collect();
    assert_eq!(fixtures.len(), 1, "one engraved grid");
    assert_eq!(fixtures[0].kind, GRID);
    assert!(!fixtures[0].visible, "hidden");
    for m in &ms {
        let p = m.page.as_ref().unwrap();
        assert_eq!(
            p.size,
            m.work_area(),
            "a laser page is as big as its work area"
        );
    }
}

#[test]
fn ac20_save_and_reopen_keeps_every_page_and_the_machine_data() {
    let d = unpack(2, &fixture("page_kinds_v37.curvyo")).unwrap();
    let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(again.pages(), d.pages());
    assert_eq!(again.manufacturings(), d.manufacturings());
    assert_eq!(again.read_objects().objects, d.read_objects().objects);
}

#[test]
fn ac20_a_project_built_through_the_commands_round_trips() {
    let d = Document::new(1);
    let p2 = d.add_page_of(&PageKind::Drawing).unwrap();
    d.resize_page(
        &p2.id,
        DocumentSize::from_mm(125.0, 75.0),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    rect(&d, 10.0, 10.0);
    let s = d.add_page_of(&PageKind::Sketch).unwrap();
    d.create_page_sketch(&s.id, &three_lines(), &Style::default())
        .unwrap();
    let a = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    let b = add_laser(&d, &copy("big", "top_right"), 600.0, 400.0);
    let f = d
        .add_fixture(
            b.kind.manufacturing().unwrap(),
            &curvyo_document_core::NewFixture {
                add: curvyo_document_core::FixtureAdd::Grid,
                kind: GRID,
                label: "Engraved grid",
                params: &grid(25.0),
                matrix: None,
            },
        )
        .unwrap();
    d.set_fixture_visible(f, false).unwrap();
    rect(&d, a.origin.x + 5.0, a.origin.y + 5.0);
    d.reorder_page(&b.id, 1).unwrap();
    let again = unpack(9, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(again.pages(), d.pages());
    assert_eq!(again.manufacturings(), d.manufacturings());
    assert_eq!(again.read_objects().objects, d.read_objects().objects);
}

#[test]
fn ac20_the_damaged_golden_reads_as_drawing_and_is_kept() {
    let bytes = fixture("page_kind_damaged_v37.curvyo");
    let loro = stored(&bytes);
    let d = unpack(2, &bytes).unwrap();
    let damaged: Vec<_> = d
        .pages()
        .into_iter()
        .filter(|p| {
            root_value(&loro, &format!("page.{}.kind", p.id))
                .is_some_and(|v| !matches!(v, LoroValue::String(_)))
        })
        .collect();
    assert_eq!(damaged.len(), 1, "the golden holds one damaged kind");
    assert_eq!(damaged[0].kind, PageKind::Drawing);
    let key = format!("page.{}.kind", damaged[0].id);
    assert_eq!(
        root_value(&stored(&pack(&d, "0.1.0").unwrap()), &key),
        root_value(&loro, &key)
    );
}

#[test]
fn ac18c_the_unknown_golden_has_a_template_page_kept_unchanged() {
    let bytes = fixture("page_kind_unknown_v37.curvyo");
    let d = unpack(2, &bytes).unwrap();
    let unknown: Vec<_> = d
        .pages()
        .into_iter()
        .filter(|p| p.kind == PageKind::Unknown("template".into()))
        .collect();
    assert_eq!(unknown.len(), 1);
    let key = format!("page.{}.kind", unknown[0].id);
    assert_eq!(
        root_value(&stored(&pack(&d, "0.1.0").unwrap()), &key),
        Some(LoroValue::from("template"))
    );
}

// ---- criterion 19: the format-36 conversion -----------------------------------------------------

/// A format-36 file built by hand: page "first" with a rectangle owning a sketch (drawn at its
/// centre) and an external clone of it 160 mm lower, a second drawing page, and two
/// manufacturings whose output pages both sit at (0, 0), the first with a placement and a hidden
/// grid.
fn format_36_file() -> Vec<u8> {
    let d = Document::new(1);
    rect(&d, 10.0, 10.0);
    let s = d.add_page_of(&PageKind::Sketch).unwrap();
    d.move_page(
        &s.id,
        Vec2::new(-s.origin.x, -s.origin.y),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let sk = d
        .create_page_sketch(&s.id, &three_lines(), &Style::default())
        .unwrap();
    let clone = d.linked_clone(&[sk], &[]).unwrap()[0];
    d.translate_objects(&[clone], Vec2::new(0.0, 160.0))
        .unwrap();
    d.add_page_of(&PageKind::Drawing).unwrap();
    let a = add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    let fa = d
        .add_fixture(
            a.kind.manufacturing().unwrap(),
            &curvyo_document_core::NewFixture {
                add: curvyo_document_core::FixtureAdd::Grid,
                kind: GRID,
                label: "Engraved grid",
                params: &grid(10.0),
                matrix: None,
            },
        )
        .unwrap();
    d.set_fixture_visible(fa, false).unwrap();
    let b = add_laser(&d, &copy("big", "top_left"), 600.0, 400.0);
    let loro = stored(&pack(&d, "0.1.0").unwrap());
    let root = loro.get_map("root");
    // Format 36: no kind registers, the sketch claim on its drawing page, owned pages at (0, 0).
    let claim = root_value(&loro, &format!("page.{}.sketch_owner", s.id)).unwrap();
    let keys: Vec<String> = root.keys().map(|k| k.to_string()).collect();
    for key in keys {
        if key.ends_with(".kind") || key.starts_with(&format!("page.{}.", s.id)) {
            root.delete(&key).unwrap();
        }
    }
    root.insert("page.first.sketch_owner", claim).unwrap();
    // Laying the sketch page over page 1 pushed page 1 away (`specs/0109` criterion 22): put
    // page 1 back at (0, 0), where a format-36 file has it.
    let home: std::collections::HashMap<String, LoroValue> = [
        ("x".to_owned(), LoroValue::Double(0.0)),
        ("y".to_owned(), LoroValue::Double(0.0)),
        ("width".to_owned(), LoroValue::Double(210.0)),
        ("height".to_owned(), LoroValue::Double(297.0)),
    ]
    .into();
    root.insert("page.first.frame", LoroValue::from(home))
        .unwrap();
    for p in [&a, &b] {
        let frame: std::collections::HashMap<String, LoroValue> = [
            ("x".to_owned(), LoroValue::Double(0.0)),
            ("y".to_owned(), LoroValue::Double(0.0)),
            ("width".to_owned(), LoroValue::Double(p.size.width.as_mm())),
            (
                "height".to_owned(),
                LoroValue::Double(p.size.height.as_mm()),
            ),
        ]
        .into();
        root.insert(&format!("page.{}.frame", p.id), LoroValue::from(frame))
            .unwrap();
        root.insert(&format!("page.{}.order", p.id), 0.0).unwrap();
    }
    let tree = loro.get_tree("paths");
    let node = tree_id(sk);
    tree.get_meta(node)
        .unwrap()
        .insert("sketch_page", "first")
        .unwrap();
    add_raw_placement(&loro, manufacturing_nodes(&loro)[0], "first");
    root.insert("format_version", 36_i64).unwrap();
    loro.commit();
    container(&loro, 36)
}

/// The objects of `bytes` as the older build read them: no conversion.
fn objects_unconverted(bytes: &[u8]) -> Vec<ObjectSnapshot> {
    unpack(4, &container(&stored(bytes), CURRENT_FORMAT_VERSION))
        .unwrap()
        .read_objects()
        .objects
        .to_vec()
}

#[test]
fn ac19_a_format_36_file_converts_on_open_with_no_step() {
    let bytes = format_36_file();
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "no undo step");
    assert!(d.own_steps().is_empty());
    let pages = d.pages();
    assert_no_overlap(&pages);
    let outputs: Vec<_> = pages
        .iter()
        .filter(|p| matches!(p.kind, PageKind::Output { .. }))
        .collect();
    assert_eq!(outputs.len(), 2, "both output pages are laser pages");
    assert_eq!(
        pages.iter().filter(|p| p.kind == PageKind::Drawing).count(),
        2
    );
    let ms = d.manufacturings();
    assert_eq!(ms[0].placements.len(), 1, "placements kept");
    assert_eq!(ms[0].fixtures.len(), 1);
    assert!(!ms[0].fixtures[0].visible, "hidden state kept");
    for m in &ms {
        assert_eq!(m.page.as_ref().unwrap().size, m.work_area());
    }
    // Right of the drawing pages, in tree order of the manufacturings.
    let rightmost_drawing = pages
        .iter()
        .filter(|p| p.kind == PageKind::Drawing)
        .map(|p| p.origin.x + p.size.width.as_mm())
        .fold(f64::MIN, f64::max);
    for o in &outputs {
        assert!(
            o.origin.x >= rightmost_drawing - TOL,
            "{} right of the drawing pages",
            o.name
        );
    }
    let first_out = ms[0].page.as_ref().unwrap();
    let second_out = ms[1].page.as_ref().unwrap();
    assert!(first_out.origin.x < second_out.origin.x, "tree order");
}

#[test]
fn ac19b_the_sketch_moves_to_its_own_page_and_a_linked_clone_keeps_the_drawing() {
    let bytes = format_36_file();
    let before = objects_unconverted(&bytes);
    let d = unpack(2, &bytes).unwrap();
    let pages = d.pages();
    let first = page(&d, "first");
    let sp = pages
        .iter()
        .find(|p| p.kind == PageKind::Sketch)
        .expect("a sketch page");
    assert_eq!(sp.id, "first-sketch", "deterministic id");
    assert_eq!(sp.name, "Page 1 sketch");
    assert_eq!(sp.size, first.size, "same size");
    assert!(
        d.page_sketch("first").is_none(),
        "the drawing page owns no sketch"
    );
    let sk = d.page_sketch(&sp.id).expect("the claim moved");
    let zero = d
        .sketch(sk)
        .unwrap()
        .placement
        .apply_point(Point::new(0.0, 0.0));
    assert!((zero.x - (sp.origin.x + sp.size.width.as_mm() / 2.0)).abs() < TOL);
    assert!((zero.y - (sp.origin.y + sp.size.height.as_mm() / 2.0)).abs() < TOL);
    // The rectangle (other content) is kept on page 1.
    assert!(
        d.read_objects()
            .objects
            .iter()
            .any(|o| matches!(o, ObjectSnapshot::Primitive(_)))
    );
    // Every outline drawn before is drawn again by something other than the moved sketch: the new
    // clone at the sketch's old place and the external clone held where it was.
    let now = d.read_objects().objects.to_vec();
    let old: Vec<_> = before
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Clone(_) | ObjectSnapshot::Sketch(_)))
        .filter_map(outline_of)
        .collect();
    assert_eq!(old.len(), 2);
    for anchors in &old {
        assert!(
            now.iter()
                .filter(|o| matches!(o, ObjectSnapshot::Clone(_)))
                .filter_map(outline_of)
                .any(|a| same(&a, anchors)),
            "an outline of the old file is drawn by a clone at its place"
        );
    }
    let clones = now
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Clone(_)))
        .count();
    assert_eq!(clones, 2, "one new clone besides the external one");
}

#[test]
fn ac19b_the_new_clone_has_its_own_placement() {
    let bytes = format_36_file();
    let d = unpack(2, &bytes).unwrap();
    let before_ids: Vec<NodeId> = objects_unconverted(&bytes)
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Clone(c) => Some(c.resolved.id),
            _ => None,
        })
        .collect();
    let new_clone = d
        .read_objects()
        .objects
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Clone(c) if !before_ids.contains(&c.resolved.id) => Some(c.resolved.id),
            _ => None,
        })
        .expect("a new clone");
    let loro = stored(&pack(&d, "0.1.0").unwrap());
    let meta = loro
        .get_tree("paths")
        .get_meta(tree_id(new_clone))
        .unwrap()
        .get_deep_value();
    assert!(
        format!("{meta:?}").contains("own_placement\": Bool(true)"),
        "own placement on: {meta:?}"
    );
}

#[test]
fn ac19_the_converted_file_saves_as_the_current_format_and_does_not_convert_again() {
    let d = unpack(2, &format_36_file()).unwrap();
    let saved = pack(&d, "0.1.0").unwrap();
    // The current format since `specs/0100-bring-content-in` is 38.
    assert_eq!(manifest_version(&saved), u64::from(CURRENT_FORMAT_VERSION));
    let again = unpack(3, &saved).unwrap();
    assert_eq!(again.own_step_count(), 0);
    assert_eq!(again.pages(), d.pages());
    assert_eq!(again.read_objects().objects, d.read_objects().objects);
}

#[test]
fn ac19_two_peers_converting_the_same_file_write_the_same_sketch_page() {
    let bytes = format_36_file();
    let a = unpack(11, &bytes).unwrap();
    let b = unpack(12, &bytes).unwrap();
    let sa = a
        .pages()
        .into_iter()
        .find(|p| p.kind == PageKind::Sketch)
        .unwrap();
    let sb = b
        .pages()
        .into_iter()
        .find(|p| p.kind == PageKind::Sketch)
        .unwrap();
    assert_eq!(sa, sb);
}

#[test]
fn ac19_a_format_36_file_without_sketch_or_output_page_is_untouched() {
    let d = Document::new(1);
    rect(&d, 1.0, 1.0);
    d.add_page_of(&PageKind::Drawing).unwrap();
    let loro = stored(&pack(&d, "0.1.0").unwrap());
    let root = loro.get_map("root");
    let keys: Vec<String> = root.keys().map(|k| k.to_string()).collect();
    for key in keys.into_iter().filter(|k| k.ends_with(".kind")) {
        root.delete(&key).unwrap();
    }
    loro.commit();
    let frontier = loro.oplog_vv();
    let opened = unpack(2, &container(&loro, 36)).unwrap();
    assert_eq!(opened.own_step_count(), 0);
    let saved = stored(&pack(&opened, "0.1.0").unwrap());
    assert_eq!(saved.oplog_vv(), frontier, "no commit at all");
    assert!(opened.pages().iter().all(|p| p.kind == PageKind::Drawing));
}

#[test]
fn ac19b_a_page_whose_only_content_was_its_sketch_stays_a_drawing_page() {
    let bytes = fixture("sketch_page_owner_v29.curvyo");
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0);
    let first = page(&d, "first");
    assert_eq!(first.kind, PageKind::Drawing);
    let sp = d
        .pages()
        .into_iter()
        .find(|p| p.kind == PageKind::Sketch)
        .expect("its sketch got a sketch page");
    assert_eq!(sp.name, format!("{} sketch", first.name));
    assert!(d.page_sketch(&sp.id).is_some());
    assert_no_overlap(&d.pages());
}

#[test]
fn ac19_the_migration_golden_opens_to_the_decided_result() {
    let bytes = fixture("page_kinds_migration_v36.curvyo");
    assert_eq!(manifest_version(&bytes), 36);
    let before = objects_unconverted(&bytes);
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0);
    let pages = d.pages();
    assert_no_overlap(&pages);
    assert!(
        pages
            .iter()
            .any(|p| matches!(p.kind, PageKind::Output { .. }))
    );
    let sp = pages.iter().find(|p| p.kind == PageKind::Sketch).unwrap();
    assert_eq!(sp.name, "Page 1 sketch");
    let m = &d.manufacturings()[0];
    assert_eq!(m.placements.len(), 1, "an output page with a placement");
    assert!(
        m.fixtures.iter().any(|f| !f.visible),
        "and a hidden fixture"
    );
    let now = d.read_objects().objects.to_vec();
    for anchors in before
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Clone(_) | ObjectSnapshot::Sketch(_)))
        .filter_map(outline_of)
    {
        assert!(
            now.iter()
                .filter(|o| matches!(o, ObjectSnapshot::Clone(_)))
                .filter_map(outline_of)
                .any(|a| same(&a, &anchors)),
            "equal resolved outlines"
        );
    }
}

#[test]
fn ac19_the_older_laser_goldens_open_through_the_conversion() {
    for name in ["laser_workbench_v36.curvyo", "machine_profile_v36.curvyo"] {
        let d = unpack(2, &fixture(name)).unwrap();
        assert_eq!(d.own_step_count(), 0, "{name}");
        let pages = d.pages();
        assert_no_overlap(&pages);
        let out = pages
            .iter()
            .find(|p| matches!(p.kind, PageKind::Output { .. }))
            .unwrap_or_else(|| panic!("{name}: a laser page is listed"));
        let m = d.manufacturing(out.kind.manufacturing().unwrap()).unwrap();
        assert_eq!(out.size, m.work_area(), "{name}");
    }
}

#[test]
fn ac2_edge_add_page_after_a_reorder_or_a_moved_page_lands_on_no_page() {
    let d = Document::new(1);
    let p2 = d.add_page_of(&PageKind::Drawing).unwrap();
    add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
    d.reorder_page(&p2.id, 0).unwrap();
    d.reorder_page("first", 0).unwrap();
    // List order now: first, p2, laser; move the laser page to the far left.
    let l = d.pages()[2].clone();
    d.move_page(
        &l.id,
        Vec2::new(-2000.0, 0.0),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    d.add_page_of(&PageKind::Sketch).unwrap();
    d.add_page_of(&PageKind::Drawing).unwrap();
    assert_no_overlap(&d.pages());
}

#[test]
fn ac2_edge_more_than_eight_pages_wrap_and_never_overlap() {
    let d = Document::new(1);
    for i in 0..12 {
        if i % 4 == 3 {
            add_laser(&d, &copy("generic-laser", "bottom_left"), 400.0, 300.0);
        } else if i % 2 == 0 {
            d.add_page_of(&PageKind::Sketch).unwrap();
        } else {
            d.add_page_of(&PageKind::Drawing).unwrap();
        }
    }
    assert_eq!(d.pages().len(), 13);
    assert_no_overlap(&d.pages());
}

#[test]
fn ac20_opening_a_format_37_file_with_laser_and_sketch_pages_writes_nothing() {
    for name in [
        "page_kinds_v37.curvyo",
        "page_kind_damaged_v37.curvyo",
        "page_kind_unknown_v37.curvyo",
    ] {
        let bytes = fixture(name);
        let before = stored(&bytes).oplog_vv();
        let d = unpack(2, &bytes).unwrap();
        let after = stored(&pack(&d, "0.1.0").unwrap()).oplog_vv();
        assert_eq!(after, before, "{name}: opening writes nothing");
    }
}
