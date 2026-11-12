//! `specs/0109-page-autolayout`: the page commands run the auto-layout in their own step and
//! carry the objects the caller says each page owns (ADR 0051 §1, §4 to §7).
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewOutputPage, NodeId,
    ObjectSnapshot, PageContents, PageKind, Point, RectBounds, StepRef, Style, Vec2, unpack,
};
use curvyo_sketch_core as sk;

fn rect(d: &Document, x: f64, y: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + 10.0, y + 10.0),
    ))
}

fn origin(d: &Document, id: &str) -> Point {
    d.pages().into_iter().find(|p| p.id == id).unwrap().origin
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn last_step(d: &Document) -> StepRef {
    d.own_steps().last().unwrap().step
}

fn json(d: &Document) -> serde_json::Value {
    serde_json::from_slice(&d.export_json().unwrap()).unwrap()
}

fn contents(entries: &[(&str, &[NodeId])]) -> PageContents {
    entries
        .iter()
        .map(|(page, ids)| ((*page).to_owned(), ids.to_vec()))
        .collect()
}

fn assert_moved(d: &Document, id: NodeId, before: &curvyo_document_core::ObjectSnapshot, by: Vec2) {
    assert_eq!(d.object(id).unwrap(), before.translated(by));
}

fn generic_copy() -> FixtureParams {
    serde_json::from_value(serde_json::json!({
        "id": "generic-laser",
        "name": "Generic laser",
        "origin": "bottom_left",
        "max_feed_mm_per_min": 6000,
        "max_power": 1000,
        "laser_mode": true,
    }))
    .unwrap()
}

fn add_laser(d: &Document) -> String {
    let copy = generic_copy();
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: Some(&copy),
        work_area: DocumentSize::from_mm(400.0, 300.0),
    })
    .unwrap()
    .id
}

fn landscape() -> DocumentSize {
    DocumentSize::from_mm(297.0, 210.0)
}

#[test]
fn c1_c14_c17_c26_landscape_page_1_moves_the_laser_page_with_its_items_in_one_step() {
    let d = Document::new(1);
    let drawing = rect(&d, 20.0, 20.0);
    let laser = add_laser(&d);
    let own = rect(&d, 400.0, 100.0);
    let pasteboard = rect(&d, 1000.0, 1000.0);
    let (drawing_0, own_0, paste_0) = (
        d.object(drawing).unwrap(),
        d.object(own).unwrap(),
        d.object(pasteboard).unwrap(),
    );
    let laser_0 = origin(&d, &laser);
    let before = json(&d);
    let steps = d.own_step_count();
    let owned = contents(&[("first", &[drawing]), (&laser, &[own])]);
    assert!(d.resize_page("first", landscape(), &owned).unwrap());
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), "resize_page");
    assert_eq!(origin(&d, &laser), laser_0.translated(Vec2::new(43.5, 0.0)));
    assert_moved(&d, own, &own_0, Vec2::new(43.5, 0.0));
    assert_eq!(
        d.object(drawing).unwrap(),
        drawing_0,
        "the changed page's drawing stays"
    );
    assert_eq!(
        d.object(pasteboard).unwrap(),
        paste_0,
        "the pasteboard stays"
    );
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(json(&d), before);
}

#[test]
fn c19_c26_placements_and_fixtures_follow_the_frame_and_undo_restores_byte_identically() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/laser_workbench_v36.curvyo");
    let d = unpack(2, &std::fs::read(path).unwrap()).unwrap();
    let made = d.manufacturings()[0].clone();
    let laser = made.page.clone().unwrap();
    let before = json(&d);
    let first = d.pages()[0].clone();
    let turned = DocumentSize::from_mm(first.size.height.as_mm(), first.size.width.as_mm());
    assert!(
        d.resize_page(&first.id, turned, &PageContents::new())
            .unwrap()
    );
    let moved = d.manufacturings()[0].clone();
    assert_ne!(moved.page.as_ref().unwrap().origin, laser.origin);
    assert_eq!(
        moved.placements, made.placements,
        "page-local, written by no one"
    );
    assert_eq!(moved.fixtures, made.fixtures);
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(json(&d), before);
}

#[test]
fn c22_c27_a_drop_pushes_the_page_it_lands_on_with_its_objects_in_one_move_page_step() {
    let d = Document::new(1);
    let second = d.add_page().id;
    let a = rect(&d, 20.0, 20.0);
    let b = rect(&d, 300.0, 20.0);
    let b_0 = d.object(b).unwrap();
    let before = json(&d);
    let owned = contents(&[("first", &[a]), (&second, &[b])]);
    // "first" lands 100 mm onto "second".
    assert!(d.move_page("first", Vec2::new(139.7, 0.0), &owned).unwrap());
    assert_eq!(last_label(&d), "move_page");
    assert_eq!(origin(&d, "first"), Point::new(139.7, 0.0));
    let pushed = origin(&d, &second);
    assert!((pushed.x - (139.7 + 210.0 + 29.7)).abs() < 1e-9);
    assert_moved(&d, b, &b_0, Vec2::new(pushed.x - 239.7, 0.0));
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(json(&d), before);
}

#[test]
fn c35_c37_c41_deleting_the_middle_page_closes_the_hole_in_one_delete_page_step() {
    let d = Document::new(1);
    let b = d.add_page().id;
    let c = d.add_page().id;
    let on_a = rect(&d, 20.0, 20.0);
    let on_b = rect(&d, 260.0, 20.0);
    let on_c = rect(&d, 500.0, 20.0);
    let (a_0, b_0, c_0) = (
        d.object(on_a).unwrap(),
        d.object(on_b).unwrap(),
        d.object(on_c).unwrap(),
    );
    let before = json(&d);
    let steps = d.own_step_count();
    let owned = contents(&[("first", &[on_a]), (&b, &[on_b]), (&c, &[on_c])]);
    d.delete_page(&b, &owned).unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), "delete_page");
    assert!((origin(&d, &c).x - 239.7).abs() < 1e-9);
    assert_moved(&d, on_c, &c_0, Vec2::new(-239.7, 0.0));
    assert_eq!(
        d.object(on_b).unwrap(),
        b_0,
        "the deleted page's drawing stays"
    );
    assert_eq!(d.object(on_a).unwrap(), a_0);
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(json(&d), before);
}

#[test]
fn c36_deleting_a_middle_laser_page_closes_the_hole_and_takes_its_machine_data() {
    let d = Document::new(1);
    let laser = add_laser(&d);
    let p3 = d.add_page().id;
    let own = rect(&d, 400.0, 100.0);
    let own_0 = d.object(own).unwrap();
    let before = json(&d);
    d.delete_page(&laser, &contents(&[(&laser, &[own])]))
        .unwrap();
    assert_eq!(last_label(&d), "delete_page");
    assert_eq!(d.manufacturings(), Vec::new());
    // The laser page's gap is 40 mm: the next page closes in by 400 + 40 mm.
    assert!((origin(&d, &p3).x - 239.7).abs() < 1e-9);
    assert_eq!(
        d.object(own).unwrap(),
        own_0,
        "made on the laser page, stays"
    );
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(json(&d), before);
}

/// The anchor box of a path, clone or sketch.
fn bounds(d: &Document, id: NodeId) -> (Point, Point) {
    let path = match d.object(id).unwrap() {
        ObjectSnapshot::Clone(clone) => *clone.resolved,
        ObjectSnapshot::Path(path) => path,
        ObjectSnapshot::Sketch(sketch) => sketch.resolved,
        other => panic!("no outline: {other:?}"),
    };
    let mut min = Point::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for anchor in path.all_anchors() {
        min = Point::new(min.x.min(anchor.point.x), min.y.min(anchor.point.y));
        max = Point::new(max.x.max(anchor.point.x), max.y.max(anchor.point.y));
    }
    (min, max)
}

fn assert_box_moved(after: (Point, Point), before: (Point, Point), by: Vec2) {
    for (a, b) in [(after.0, before.0), (after.1, before.1)] {
        assert!(
            (a.x - b.x - by.x).abs() < 1e-6 && (a.y - b.y - by.y).abs() < 1e-6,
            "{after:?} is not {before:?} moved by {by:?}"
        );
    }
}

fn polyline(points: &[(f64, f64)]) -> sk::Sketch {
    let pts: Vec<_> = points
        .iter()
        .map(|&(x, y)| sk::Position::mm(x, y))
        .collect();
    sk::create_polyline(&sk::Sketch::new("origin".into()), "edge", &pts, false).unwrap()
}

#[test]
fn c36_c41_deleting_a_sketch_page_closes_the_hole_under_its_own_label_and_a_moved_clone_becomes_paths_where_it_lands()
 {
    let d = Document::new(1);
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    let p3 = d.add_page().id;
    let sketch = d
        .create_page_sketch(
            &sp.id,
            &polyline(&[(-10.0, -10.0), (10.0, 10.0)]),
            &Style::default(),
        )
        .unwrap();
    // A clone of the sketch on the page behind the sketch page.
    let clone = d
        .place_linked_clone(sketch, Vec2::new(origin(&d, &p3).x - sp.origin.x, 0.0))
        .unwrap();
    let clone_0 = bounds(&d, clone);
    let before = json(&d);
    let shift = -(sp.size.width.as_mm() + 29.7);
    d.delete_page(&sp.id, &contents(&[(&sp.id, &[sketch]), (&p3, &[clone])]))
        .unwrap();
    assert_eq!(last_label(&d), "delete_sketch_page");
    assert!((origin(&d, &p3).x - 239.7).abs() < 1e-9);
    assert!(matches!(d.object(clone), Some(ObjectSnapshot::Path(_))));
    assert_box_moved(bounds(&d, clone), clone_0, Vec2::new(shift, 0.0));
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(json(&d), before);
}

#[test]
fn c19_a_clone_and_its_original_on_pages_that_move_apart_each_follow_their_own_page() {
    let d = Document::new(1);
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    let below = d.add_page().id;
    // Lay the third page under "first" (no page lies there, nothing is pushed).
    let to = Vec2::new(-origin(&d, &below).x, 297.0 + 29.7);
    assert!(d.move_page(&below, to, &PageContents::new()).unwrap());
    let sketch = d
        .create_page_sketch(
            &sp.id,
            &polyline(&[(-10.0, -10.0), (10.0, 10.0)]),
            &Style::default(),
        )
        .unwrap();
    let to_below = origin(&d, &below);
    let clone = d
        .place_linked_clone(
            sketch,
            Vec2::new(to_below.x + 105.0 - (sp.origin.x + 105.0), 326.7),
        )
        .unwrap();
    let (sketch_0, clone_0) = (bounds(&d, sketch), bounds(&d, clone));
    let owned = contents(&[(&sp.id, &[sketch]), (&below, &[clone])]);
    assert!(d.resize_page("first", landscape(), &owned).unwrap());
    assert_box_moved(bounds(&d, sketch), sketch_0, Vec2::new(43.5, 0.0));
    assert_box_moved(bounds(&d, clone), clone_0, Vec2::new(0.0, -43.5));
}

#[test]
fn c28_the_format_has_no_bump_of_its_own() {
    // The auto-layout bumps nothing; 40 is the bump of `0102` laser-fixtures (ADR 0049).
    // Format 41 since `specs/0103-fixture-jobs` (ADR 0052 §6).
    // Format 42 since `specs/0110-pen-modes-variable-stroke` (ADR 0053).
    // Format 43 since `specs/0028-material-test-library` (ADR 0054 §6).
    assert_eq!(CURRENT_FORMAT_VERSION, 43);
}

#[test]
fn c29_add_page_starts_a_new_row_when_its_place_is_crowded_and_moves_no_page() {
    let d = Document::new(1);
    let p2 = d.add_page().id;
    let p3 = d.add_page().id;
    // The maker drags the third page 10 mm right and puts it before the second in the list: the
    // place after the second, the last one, is now crowded.
    assert!(
        d.move_page(&p3, Vec2::new(10.0, 0.0), &PageContents::new())
            .unwrap()
    );
    d.reorder_page(&p3, 1).unwrap();
    let frames: Vec<_> = d.pages().iter().map(|p| p.origin).collect();
    let new = d.add_page();
    assert_eq!(new.origin, Point::new(0.0, 297.0 + 29.7));
    let after: Vec<_> = d.pages().iter().take(3).map(|p| p.origin).collect();
    assert_eq!(after, frames);
    let _ = p2;
}
