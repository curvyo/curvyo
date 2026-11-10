//! `specs/0109-page-autolayout` criteria 14 to 20: what each page owns by the centre of the
//! top-level object's outline box (ADR 0051 §4).
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{Document, NodeId, PageKind, Point, RectBounds};
use curvyo_ui_core::{PageMembership, page_contents};

fn rect(d: &Document, x: f64, y: f64, w: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + w, y + 10.0),
    ))
}

fn contents(d: &Document) -> curvyo_document_core::PageContents {
    let pages = d.pages();
    let read = d.read_objects();
    page_contents(d, &pages, &PageMembership::of(d, &pages), &read.objects)
}

#[test]
fn c15_c16_an_object_goes_with_the_page_of_its_centre_and_a_group_as_a_whole() {
    let d = Document::new(1);
    let second = d.add_page().id;
    // Centre on page 1, hanging over its right edge.
    let hanging = rect(&d, 180.0, 20.0, 40.0);
    // Centre in the gap between the pages: the pasteboard.
    let gap = rect(&d, 215.0, 20.0, 20.0);
    // Two leaves on different pages, grouped: the group's centre (x 240) is on the second page.
    let left = rect(&d, 150.0, 50.0, 10.0);
    let right = rect(&d, 320.0, 50.0, 10.0);
    let group = d.group_objects(&[left, right]).unwrap();
    let owned = contents(&d);
    assert_eq!(owned.get("first"), Some(&vec![hanging]));
    let on_second = owned.get(&second).unwrap();
    assert!(
        on_second.contains(&left) && on_second.contains(&right),
        "{group:?}"
    );
    assert!(owned.values().all(|ids| !ids.contains(&gap)));
}

#[test]
fn c19_a_sketch_page_owns_its_sketch_and_a_drawing_object_over_it_belongs_to_no_page() {
    let d = Document::new(1);
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    let sketch = d
        .create_page_sketch(
            &sp.id,
            &{
                use curvyo_sketch_core as sk;
                let pts = [sk::Position::mm(-5.0, 0.0), sk::Position::mm(5.0, 0.0)];
                sk::create_polyline(&sk::Sketch::new("origin".into()), "edge", &pts, false).unwrap()
            },
            &curvyo_document_core::Style::default(),
        )
        .unwrap();
    let over = rect(&d, sp.origin.x + 20.0, sp.origin.y + 20.0, 10.0);
    let owned = contents(&d);
    assert_eq!(owned.get(&sp.id), Some(&vec![sketch]));
    assert!(owned.values().all(|ids| !ids.contains(&over)));
}
