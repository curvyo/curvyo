//! Independent acceptance tests for the page membership read of `specs/0099-sketch-pages`
//! criterion 1 and ADR 0043 §1: a sketch page holds only its owning sketch, by ownership, and
//! every other object ignores sketch frames.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::assert_is_empty,
    missing_docs
)]

use curvyo_document_core::{
    AnchorId, Document, NodeId, ObjectSnapshot, Page, PageKind, Point, RectBounds, Style, Vec2,
    pack, unpack,
};
use curvyo_sketch_core as sk;
use curvyo_ui_core::{PageMembership, Reach};

const REACHES: [Reach; 2] = [Reach::Touches, Reach::SharesArea];

fn page(d: &Document, id: &str) -> Page {
    d.pages().into_iter().find(|p| p.id == id).unwrap()
}

fn rect(d: &Document, min: Point, max: Point) -> NodeId {
    d.create_rect(RectBounds::from_corners(min, max))
}

fn bounds(d: &Document, id: NodeId) -> (Point, Point) {
    let o = d.object(id).unwrap();
    let points: Vec<Point> = match &o {
        ObjectSnapshot::Sketch(s) => s.resolved.all_anchors().map(|a| a.point).collect(),
        ObjectSnapshot::Clone(c) => c.resolved.all_anchors().map(|a| a.point).collect(),
        ObjectSnapshot::Path(p) => p.all_anchors().map(|a| a.point).collect(),
        ObjectSnapshot::Primitive(p) => curvyo_document_core::outline_of(&p.shape)
            .anchors
            .iter()
            .map(|a| a.point)
            .collect(),
        other => panic!("{other:?}"),
    };
    let mut min = Point::new(f64::MAX, f64::MAX);
    let mut max = Point::new(f64::MIN, f64::MIN);
    for p in points {
        min = Point::new(min.x.min(p.x), min.y.min(p.y));
        max = Point::new(max.x.max(p.x), max.y.max(p.y));
    }
    (min, max)
}

fn pages_of(d: &Document, id: NodeId, reach: Reach) -> Vec<String> {
    let pages = d.pages();
    let m = PageMembership::of(d, &pages);
    m.pages_of(&pages, id, bounds(d, id), reach)
        .map(|p| p.id.clone())
        .collect()
}

/// The page that owns `id` by the centre of its bounds (ADR 0051 §4; replaces the "wholly
/// inside" reach of `0059` criterion 9 that a frame move used to carry by).
fn owner_of(d: &Document, id: NodeId) -> Vec<String> {
    let pages = d.pages();
    let m = PageMembership::of(d, &pages);
    let (min, max) = bounds(d, id);
    let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
    m.owner_by_centre(&pages, id, centre)
        .map(|p| p.id.clone())
        .into_iter()
        .collect()
}

fn polyline(points: &[(f64, f64)]) -> sk::Sketch {
    let pts: Vec<_> = points
        .iter()
        .map(|&(x, y)| sk::Position::mm(x, y))
        .collect();
    sk::create_polyline(&sk::Sketch::new("origin".into()), "edge", &pts, false).unwrap()
}

/// "first" drawing page at (0, 0) and a sketch page moved so that it overlaps nothing.
fn doc_with_sketch_page() -> (Document, Page) {
    let d = Document::new(1);
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    let sp = page(&d, &sp.id);
    (d, sp)
}

#[test]
fn ac1_a_rectangle_wholly_inside_a_sketch_frame_belongs_to_no_page() {
    let (d, sp) = doc_with_sketch_page();
    let r = rect(
        &d,
        Point::new(sp.origin.x + 10.0, sp.origin.y + 10.0),
        Point::new(sp.origin.x + 30.0, sp.origin.y + 20.0),
    );
    for reach in REACHES {
        assert!(pages_of(&d, r, reach).is_empty(), "{reach:?}: pasteboard");
    }
    let pages = d.pages();
    let m = PageMembership::of(&d, &pages);
    assert!(!m.is_member(&sp, r, bounds(&d, r), Reach::Touches));
    assert!(!m.is_fixed(r));
}

#[test]
fn ac1_a_sketch_crossing_its_frame_and_a_drawing_page_is_a_member_of_its_sketch_page_only() {
    let d = Document::new(2);
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    let first = page(&d, "first");
    // Put the sketch page right next to the drawing page, then draw from the sketch page's
    // centre across its left edge onto the drawing page.
    let gap = sp.origin.x - (first.origin.x + first.size.width.as_mm());
    d.move_page(
        &sp.id,
        Vec2::new(-gap, first.origin.y - sp.origin.y),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let sp = page(&d, &sp.id);
    let half = sp.size.width.as_mm() / 2.0;
    let owner = d
        .create_page_sketch(
            &sp.id,
            &polyline(&[(0.0, 0.0), (-(half + 40.0), 0.0), (-(half + 40.0), 10.0)]),
            &Style::default(),
        )
        .unwrap();
    let (min, max) = bounds(&d, owner);
    assert!(
        first.overlaps(min, max),
        "fixture: the sketch reaches the drawing page"
    );
    assert!(
        min.x < sp.origin.x,
        "fixture: the outline crosses the frame"
    );
    for reach in REACHES {
        assert_eq!(pages_of(&d, owner, reach), vec![sp.id.clone()], "{reach:?}");
    }
    let pages = d.pages();
    let m = PageMembership::of(&d, &pages);
    assert!(m.is_fixed(owner));
    assert!(!m.is_member(&first, owner, (min, max), Reach::Touches));
    assert_eq!(owner_of(&d, owner), vec![sp.id.clone()]);
}

#[test]
fn ac1_a_sketch_wholly_off_its_frame_still_belongs_to_it() {
    let (d, sp) = doc_with_sketch_page();
    let owner = d
        .create_page_sketch(
            &sp.id,
            &polyline(&[(5000.0, 5000.0), (5100.0, 5050.0)]),
            &Style::default(),
        )
        .unwrap();
    assert_eq!(owner_of(&d, owner), vec![sp.id.clone()]);
}

#[test]
fn ac1_an_object_over_a_sketch_page_and_a_drawing_page_belongs_to_the_drawing_page_only() {
    let d = Document::new(3);
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    let first = page(&d, "first");
    // Lay the sketch page over the drawing page's right half. A drop pushes the page it lands
    // on away (`specs/0109` criterion 22), so the overlap of an older file is made by moving the
    // drawing page out of the way and undoing only that move.
    let to = Vec2::new(
        first.origin.x + first.size.width.as_mm() / 2.0 - sp.origin.x,
        first.origin.y - sp.origin.y,
    );
    let none = curvyo_document_core::PageContents::new();
    d.move_page("first", Vec2::new(0.0, 5000.0), &none).unwrap();
    let away = d.own_steps().last().unwrap().step;
    d.move_page(&sp.id, to, &none).unwrap();
    d.undo_step(away).unwrap();
    assert_eq!(page(&d, "first"), first, "fixture: page 1 is back");
    let sp = page(&d, &sp.id);
    let r = rect(
        &d,
        Point::new(sp.origin.x + 5.0, sp.origin.y + 5.0),
        Point::new(sp.origin.x + 15.0, sp.origin.y + 15.0),
    );
    assert!(first.overlaps(bounds(&d, r).0, bounds(&d, r).1));
    assert_eq!(pages_of(&d, r, Reach::Touches), vec!["first".to_owned()]);
    assert_eq!(owner_of(&d, r), vec!["first".to_owned()]);
}

#[test]
fn ac1_a_clone_of_this_very_sketch_over_its_page_is_no_member() {
    let (d, sp) = doc_with_sketch_page();
    let owner = d
        .create_page_sketch(
            &sp.id,
            &polyline(&[(-10.0, -10.0), (10.0, 10.0)]),
            &Style::default(),
        )
        .unwrap();
    let c = d.linked_clone(&[owner], &[]).unwrap()[0];
    d.translate_objects(&[c], Vec2::new(3.0, 3.0)).unwrap();
    for reach in REACHES {
        assert!(pages_of(&d, c, reach).is_empty(), "{reach:?}");
    }
}

#[test]
fn ac1_any_drawing_object_kind_over_a_sketch_page_is_no_member() {
    let (d, sp) = doc_with_sketch_page();
    let a = Point::new(sp.origin.x + 10.0, sp.origin.y + 10.0);
    let b = Point::new(sp.origin.x + 40.0, sp.origin.y + 30.0);
    let r1 = rect(&d, a, b);
    let r2 = rect(&d, a, b);
    let group = d.group_objects(&[r1, r2]).unwrap();
    let ellipse = d.create_ellipse(curvyo_document_core::EllipseFrame::from_corners(a, b));
    let pages = d.pages();
    let m = PageMembership::of(&d, &pages);
    for id in [group, ellipse, r1] {
        assert!(!m.is_member(&sp, id, (a, b), Reach::Touches), "{id:?}");
        assert_eq!(m.pages_of(&pages, id, (a, b), Reach::Touches).count(), 0);
    }
}

#[test]
fn ac1_a_conflict_object_is_an_ordinary_object_under_the_position_rule() {
    let a = Document::new(4);
    let sp = a.add_page_of(&PageKind::Sketch).unwrap();
    let b = unpack(5, &pack(&a, "t").unwrap()).unwrap();
    let left = a
        .create_page_sketch(
            &sp.id,
            &polyline(&[(0.0, 0.0), (10.0, 0.0)]),
            &Style::default(),
        )
        .unwrap();
    let right = b
        .create_page_sketch(
            &sp.id,
            &polyline(&[(0.0, 0.0), (20.0, 0.0)]),
            &Style::default(),
        )
        .unwrap();
    let aa = a.export_updates(&b.version()).unwrap();
    let bb = b.export_updates(&a.version()).unwrap();
    a.import_updates(&bb).unwrap();
    b.import_updates(&aa).unwrap();
    let winner = a.page_sketch(&sp.id).unwrap();
    let loser = if winner == left { right } else { left };
    let pages = a.pages();
    let m = PageMembership::of(&a, &pages);
    assert!(!m.is_fixed(loser));
    let sp = page(&a, &sp.id);
    assert!(!m.is_member(&sp, loser, bounds(&a, loser), Reach::Touches));
    assert!(
        pages_of(&a, loser, Reach::Touches).is_empty(),
        "over the sketch page: pasteboard"
    );
}

#[test]
fn ac15_after_convert_objects_over_the_page_belong_to_it_by_position() {
    let (d, sp) = doc_with_sketch_page();
    let owner = d
        .create_page_sketch(
            &sp.id,
            &polyline(&[(-10.0, -10.0), (10.0, 10.0)]),
            &Style::default(),
        )
        .unwrap();
    let r = rect(
        &d,
        Point::new(sp.origin.x + 2.0, sp.origin.y + 2.0),
        Point::new(sp.origin.x + 8.0, sp.origin.y + 8.0),
    );
    assert!(owner_of(&d, r).is_empty());
    let n = d.sketch(owner).unwrap().resolved.all_anchors().count();
    let ids: Vec<_> = (0..n as u64).map(|i| AnchorId::new(55, i)).collect();
    d.convert_sketch_page(&sp.id, &ids).unwrap();
    assert_eq!(owner_of(&d, r), vec![sp.id.clone()]);
    assert_eq!(owner_of(&d, owner), vec![sp.id.clone()], "the paths too");
    let pages = d.pages();
    assert!(!PageMembership::of(&d, &pages).is_fixed(owner));
}

#[test]
fn ac17_after_deleting_the_sketch_page_objects_over_it_stay_on_the_pasteboard() {
    let (d, sp) = doc_with_sketch_page();
    d.create_page_sketch(
        &sp.id,
        &polyline(&[(0.0, 0.0), (5.0, 5.0)]),
        &Style::default(),
    )
    .unwrap();
    let r = rect(
        &d,
        Point::new(sp.origin.x + 2.0, sp.origin.y + 2.0),
        Point::new(sp.origin.x + 8.0, sp.origin.y + 8.0),
    );
    d.delete_page(&sp.id, &curvyo_document_core::PageContents::new())
        .unwrap();
    assert!(pages_of(&d, r, Reach::Touches).is_empty());
}

#[test]
fn adr0043_from_owners_and_of_agree() {
    let (d, sp) = doc_with_sketch_page();
    let owner = d
        .create_page_sketch(
            &sp.id,
            &polyline(&[(0.0, 0.0), (5.0, 5.0)]),
            &Style::default(),
        )
        .unwrap();
    let pages = d.pages();
    assert_eq!(
        PageMembership::of(&d, &pages),
        PageMembership::from_owners([(owner, sp.id.clone())])
    );
}
