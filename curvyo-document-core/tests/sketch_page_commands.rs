//! Sketch pages in the document model (`specs/0099-sketch-pages` criteria 4, 5, 8, 15, 17, 19;
//! ADR 0043): the fixed sketch, its frame move with clones held, placing a linked clone, Convert
//! to paths with the kind change, Delete with the clone repair, and the golden
//! `sketch_pages_v37.curvyo`. `CURVYO_WRITE_FIXTURES=1` rewrites it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorId, Angle, CONVERT_SKETCH_PAGE_COMMIT_LABEL, CURRENT_FORMAT_VERSION,
    DELETE_SKETCH_PAGE_COMMIT_LABEL, Document, DocumentSize, FixtureParams, NewOutputPage, NodeId,
    ObjectSnapshot, PLACE_SKETCH_CLONE_COMMIT_LABEL, PageKind, Point, RectBounds, Style, Vec2,
    pack, unpack,
};
use curvyo_sketch_core::{Constraint, Length, Position, Relation, Sketch};

const GOLDEN: &str = "sketch_pages_v37.curvyo";
const TOL: f64 = 0.01;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Three connected lines, the first held horizontal, and a circle.
fn lines_and_circle() -> Sketch {
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
    curvyo_sketch_core::create_circle(&graph, "hole", Position::mm(20.0, 15.0), Length(5.0))
        .unwrap()
}

/// A document with a drawing page and a sketch page holding a sketch.
fn with_sketch() -> (Document, String, NodeId) {
    let d = Document::new(1);
    let page = d.add_page_of(&PageKind::Sketch).unwrap();
    let sketch = d
        .create_page_sketch(&page.id, &lines_and_circle(), &Style::default())
        .unwrap();
    (d, page.id, sketch)
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo_last(d: &Document) {
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
}

/// The drawn outline's bounds of any object.
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

fn close(a: (Point, Point), b: (Point, Point)) -> bool {
    [
        (a.0.x, b.0.x),
        (a.0.y, b.0.y),
        (a.1.x, b.1.x),
        (a.1.y, b.1.y),
    ]
    .iter()
    .all(|(p, q)| (p - q).abs() <= TOL)
}

fn fresh_ids(d: &Document, id: NodeId, start: u64) -> Vec<AnchorId> {
    let count = d.sketch(id).unwrap().resolved.all_anchors().count() as u64;
    (0..count).map(|n| AnchorId::new(9, start + n)).collect()
}

// ---- criterion 4: the fixed sketch -------------------------------------------------------

#[test]
fn only_the_live_owner_of_a_sketch_page_is_fixed() {
    let (d, page, sketch) = with_sketch();
    assert!(d.is_fixed_sketch(sketch));
    let clone = d.place_linked_clone(sketch, Vec2::new(0.0, 0.0)).unwrap();
    assert!(!d.is_fixed_sketch(clone));
    let rect = d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(5.0, 5.0),
    ));
    assert!(!d.is_fixed_sketch(rect));
    d.convert_sketch_page(&page, &fresh_ids(&d, sketch, 0))
        .unwrap();
    assert!(!d.is_fixed_sketch(sketch), "a converted sketch is a path");
}

// ---- criterion 5: the frame move carries the sketch and holds every clone ----------------

#[test]
fn moving_a_sketch_frame_with_its_sketch_holds_every_clone_back() {
    let (d, page, sketch) = with_sketch();
    let plain = d.linked_clone(&[sketch], &[]).unwrap()[0];
    let placed = d.place_linked_clone(sketch, Vec2::new(0.0, 200.0)).unwrap();
    let before = (bounds(&d, sketch), bounds(&d, plain), bounds(&d, placed));
    d.move_page(
        &page,
        Vec2::new(30.0, -10.0),
        &curvyo_document_core::PageContents::from([(page.clone(), vec![sketch])]),
    )
    .unwrap();
    let moved = bounds(&d, sketch);
    assert!(close(
        moved,
        (
            before.0.0.translated(Vec2::new(30.0, -10.0)),
            before.0.1.translated(Vec2::new(30.0, -10.0))
        )
    ));
    assert!(
        close(bounds(&d, plain), before.1),
        "a clone without own placement stays"
    );
    assert!(close(bounds(&d, placed), before.2));
    assert_eq!(last_label(&d), "move_page");
}

// ---- criterion 8: placing a linked clone --------------------------------------------------

#[test]
fn placing_a_clone_is_one_step_with_own_placement_moved_by_the_translation() {
    let (d, _, sketch) = with_sketch();
    let steps = d.own_step_count();
    let clone = d
        .place_linked_clone(sketch, Vec2::new(100.0, 50.0))
        .unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), PLACE_SKETCH_CLONE_COMMIT_LABEL);
    let ObjectSnapshot::Clone(snapshot) = d.object(clone).unwrap() else {
        panic!("not a clone");
    };
    assert!(snapshot.chain.own_placement);
    assert_eq!(snapshot.chain.root, sketch);
    let (a, b) = (bounds(&d, sketch), bounds(&d, clone));
    assert!(close(
        b,
        (
            a.0.translated(Vec2::new(100.0, 50.0)),
            a.1.translated(Vec2::new(100.0, 50.0))
        )
    ));
    let top = d.object_ids();
    assert_eq!(top.last(), Some(&clone), "the clone is on top");
    undo_last(&d);
    assert!(d.object(clone).is_none());
}

#[test]
fn only_a_page_sketch_is_placed() {
    let (d, _, sketch) = with_sketch();
    let clone = d.place_linked_clone(sketch, Vec2::new(0.0, 0.0)).unwrap();
    let steps = d.own_step_count();
    assert!(d.place_linked_clone(clone, Vec2::new(1.0, 0.0)).is_err());
    assert!(
        d.place_linked_clone(sketch, Vec2::new(f64::NAN, 0.0))
            .is_err()
    );
    assert_eq!(d.own_step_count(), steps);
}

// ---- criterion 15: Convert to paths --------------------------------------------------------

#[test]
fn convert_makes_paths_and_a_drawing_page_in_one_step_and_undo_gives_the_sketch_back() {
    let (d, page, sketch) = with_sketch();
    let clone = d.place_linked_clone(sketch, Vec2::new(0.0, 300.0)).unwrap();
    let before = d.pages().into_iter().find(|p| p.id == page).unwrap();
    let outline = bounds(&d, clone);
    let steps = d.own_step_count();
    d.convert_sketch_page(&page, &fresh_ids(&d, sketch, 0))
        .unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), CONVERT_SKETCH_PAGE_COMMIT_LABEL);
    let after = d.pages().into_iter().find(|p| p.id == page).unwrap();
    assert_eq!(after.kind, PageKind::Drawing);
    assert_eq!(
        (after.name.clone(), after.origin, after.size),
        (before.name.clone(), before.origin, before.size)
    );
    assert!(matches!(d.object(sketch), Some(ObjectSnapshot::Path(_))));
    assert!(matches!(d.object(clone), Some(ObjectSnapshot::Clone(_))));
    assert!(close(bounds(&d, clone), outline));
    undo_last(&d);
    assert_eq!(d.page_kind(&page), Some(PageKind::Sketch));
    assert_eq!(d.page_sketch(&page), Some(sketch));
    assert_eq!(
        d.sketch(sketch).unwrap().graph.constraints.len(),
        lines_and_circle().constraints.len()
    );
}

#[test]
fn convert_refuses_a_page_without_a_sketch() {
    let d = Document::new(1);
    let page = d.add_page_of(&PageKind::Sketch).unwrap();
    assert!(d.convert_sketch_page(&page.id, &[]).is_err());
    assert!(d.convert_sketch_page("first", &[]).is_err());
}

// ---- criterion 17: Delete with the clone repair --------------------------------------------

#[test]
fn deleting_a_sketch_page_turns_its_clones_into_paths_and_undo_relinks_them() {
    let (d, page, sketch) = with_sketch();
    let frame = d.pages().into_iter().find(|p| p.id == page).unwrap();
    let over = d.create_rect(RectBounds::from_corners(
        Point::new(frame.origin.x + 10.0, frame.origin.y + 10.0),
        Point::new(frame.origin.x + 20.0, frame.origin.y + 20.0),
    ));
    let clones: Vec<NodeId> = [0.0, 100.0, 200.0]
        .iter()
        .map(|&y| d.place_linked_clone(sketch, Vec2::new(-300.0, y)).unwrap())
        .collect();
    let outlines: Vec<_> = clones.iter().map(|&c| bounds(&d, c)).collect();
    let over_before = d.object(over);
    let steps = d.own_step_count();
    d.delete_page(&page, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), DELETE_SKETCH_PAGE_COMMIT_LABEL);
    assert!(d.pages().iter().all(|p| p.id != page));
    assert!(d.object(sketch).is_none());
    for (clone, outline) in clones.iter().zip(&outlines) {
        assert!(matches!(d.object(*clone), Some(ObjectSnapshot::Path(_))));
        assert!(close(bounds(&d, *clone), *outline));
    }
    assert_eq!(d.object(over), over_before, "objects over the page stay");
    undo_last(&d);
    assert_eq!(d.page_sketch(&page), Some(sketch));
    for clone in &clones {
        assert!(matches!(d.object(*clone), Some(ObjectSnapshot::Clone(_))));
    }
}

#[test]
fn the_last_page_stays_and_an_empty_sketch_page_is_deleted_as_any_page() {
    let d = Document::new(1);
    let page = d.add_page_of(&PageKind::Sketch).unwrap();
    d.delete_page(&page.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert_eq!(last_label(&d), "delete_page");
    assert!(
        d.delete_page("first", &curvyo_document_core::PageContents::new())
            .is_err()
    );
}

// ---- criterion 19: the golden ------------------------------------------------------------

fn profile() -> FixtureParams {
    serde_json::from_value(serde_json::json!({
        "id": "generic-laser",
        "name": "generic-laser",
        "origin": "top_left",
        "max_feed_mm_per_min": 6000,
        "max_power": 1000,
        "laser_mode": true,
    }))
    .unwrap()
}

/// Two drawing pages, a laser page, a sketch page with three constrained lines and a circle, a
/// clone of it on a drawing page and one on the laser page turned by 12 degrees, a rectangle over
/// the sketch page, and a second sketch page converted to a drawing page.
fn golden_document() -> Document {
    let d = Document::new(1);
    let second = d.add_page();
    let copy = profile();
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: Some(&copy),
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let page = d.add_page_of(&PageKind::Sketch).unwrap();
    let sketch = d
        .create_page_sketch(&page.id, &lines_and_circle(), &Style::default())
        .unwrap();
    let centre = |p: &curvyo_document_core::Page| {
        Point::new(
            p.origin.x + p.size.width.as_mm() / 2.0,
            p.origin.y + p.size.height.as_mm() / 2.0,
        )
    };
    let here = centre(&page);
    let on_drawing = centre(&second);
    d.place_linked_clone(sketch, here.vector_to(on_drawing))
        .unwrap();
    // A step between the two placements, so no two history rows in a row carry one label.
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(page.origin.x + 5.0, page.origin.y + 5.0),
        Point::new(page.origin.x + 25.0, page.origin.y + 15.0),
    ));
    let on_laser = d
        .place_linked_clone(sketch, here.vector_to(centre(&laser)))
        .unwrap();
    let Some(ObjectSnapshot::Clone(clone)) = d.object(on_laser) else {
        panic!("not a clone");
    };
    let turn = Affine::rotation_about(centre(&laser), Angle::from_radians(12.0_f64.to_radians()));
    d.transform_objects(&[ObjectSnapshot::Clone(clone.mapped_by(turn))], false)
        .unwrap();
    let other = d.add_page_of(&PageKind::Sketch).unwrap();
    let converted = d
        .create_page_sketch(&other.id, &lines_and_circle(), &Style::default())
        .unwrap();
    d.convert_sketch_page(&other.id, &fresh_ids(&d, converted, 1000))
        .unwrap();
    d
}

fn read_golden() -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
    }
    std::fs::read(fixture_path(GOLDEN)).unwrap()
}

#[test]
fn the_sketch_pages_golden_reads_back_every_page_sketch_clone_and_path_identical() {
    let expected = golden_document();
    let bytes = read_golden();
    // `0099` made no bump (criterion 19); `0100` bumps to 38, `0101` to 39, `0110` to 42, and the v37 golden
    // still opens.
    // Format 41 since `specs/0103-fixture-jobs` (ADR 0052 §6).
    // Format 42 since `specs/0110-pen-modes-variable-stroke` (ADR 0053).
    // Format 44 since `specs/0111-laser-operation-tools` (ADR 0056 §8).
    // Format 46 since `specs/0112-test-grid-and-materials` (ADR 0057 §7).
    assert_eq!(CURRENT_FORMAT_VERSION, 46);
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.pages(), expected.pages());
    let kinds: Vec<_> = opened.pages().iter().map(|p| p.kind.clone()).collect();
    assert!(matches!(
        kinds.as_slice(),
        [
            PageKind::Drawing,
            PageKind::Drawing,
            PageKind::Output { .. },
            PageKind::Sketch,
            PageKind::Drawing
        ]
    ));
    assert_eq!(
        opened.read_objects().objects,
        expected.read_objects().objects
    );
    let resaved = unpack(3, &pack(&opened, "0.1.0").unwrap()).unwrap();
    assert_eq!(resaved.pages(), opened.pages());
    assert_eq!(
        resaved.read_objects().objects,
        opened.read_objects().objects
    );
}
