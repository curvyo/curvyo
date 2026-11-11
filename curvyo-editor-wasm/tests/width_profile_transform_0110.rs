//! An uneven resize or a skew of a path with width points is refused with a notice and changes
//! nothing; an even resize and a path without width points still transform (ADR 0053, "Width
//! points are geometry"; `specs/0110-pen-modes-variable-stroke` criterion 6).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    AnchorId, Document, NewAnchor, NodeId, PathSnapshot, Point, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};

/// A closed triangle with box (10, 10)-(50, 40); with a width point when `profiled`.
fn triangle(profiled: bool) -> (Document, NodeId) {
    let doc = Document::new(98);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(98, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(98, 2), Point::new(50.0, 10.0)),
            NewAnchor::corner(AnchorId::new(98, 3), Point::new(30.0, 40.0)),
        ],
        true,
    );
    if profiled {
        doc.set_width_point(id, AnchorId::new(98, 4), 0.1, 3.0)
            .unwrap();
    }
    (doc, id)
}

fn selected(doc: &Document) -> Session {
    let mut s = Session::open(777, &pack(doc, "test").unwrap()).unwrap();
    let _ = s.draw_list();
    s.set_tool(Tool::Select);
    let at = Point::new(30.0, 10.0);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    assert_eq!(s.selected_object_count(), 1, "setup: the path is selected");
    s
}

fn snap(session: &Session, id: NodeId) -> PathSnapshot {
    unpack(1, &session.pack("test").unwrap())
        .unwrap()
        .path(id)
        .unwrap()
}

fn drag(session: &mut Session, from: Point, to: Point) {
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    for i in 1..=6 {
        let t = f64::from(i) / 6.0;
        session.pointer_hover(
            Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t),
            false,
            false,
        );
        let _ = session.draw_list();
    }
    session.pointer_up(to, false, false);
}

/// The skew handle 16 px below the S side's midpoint.
fn south_skew(session: &Session) -> Point {
    Point::new(30.0, 40.0 + 16.0 / session.view().scale())
}

const SE: Point = Point::new(50.0, 40.0);

#[test]
fn an_uneven_resize_of_a_profiled_path_is_refused_with_a_notice() {
    let (doc, id) = triangle(true);
    let mut s = selected(&doc);
    let before = snap(&s, id);
    drag(&mut s, SE, Point::new(70.0, 40.0));
    assert_eq!(snap(&s, id), before, "nothing changes");
    assert!(
        s.take_transform_notice().contains("scaled evenly"),
        "the refusal says why"
    );
}

#[test]
fn an_even_resize_of_a_profiled_path_keeps_its_width_points() {
    let (doc, id) = triangle(true);
    let mut s = selected(&doc);
    let before = snap(&s, id);
    // Twice the box (40 x 30) along its diagonal from the fixed NW corner.
    drag(&mut s, SE, Point::new(90.0, 70.0));
    let after = snap(&s, id);
    assert_ne!(after.anchors, before.anchors, "the path is resized");
    assert_eq!(after.width_points, before.width_points);
    assert_eq!(s.take_transform_notice(), "");
}

#[test]
fn a_skew_of_a_profiled_path_is_refused() {
    let (doc, id) = triangle(true);
    let mut s = selected(&doc);
    let before = snap(&s, id);
    let handle = south_skew(&s);
    drag(&mut s, handle, Point::new(45.0, handle.y));
    assert_eq!(snap(&s, id), before, "nothing changes");
    assert!(s.take_transform_notice().contains("scaled evenly"));

    let (doc, id) = triangle(false);
    let mut s = selected(&doc);
    let before = snap(&s, id);
    let handle = south_skew(&s);
    drag(&mut s, handle, Point::new(45.0, handle.y));
    assert_ne!(
        snap(&s, id).anchors,
        before.anchors,
        "a plain path is skewed"
    );
}

#[test]
fn an_uneven_typed_size_of_a_profiled_path_is_refused() {
    let (doc, id) = triangle(true);
    let mut s = selected(&doc);
    let before = snap(&s, id);
    s.pointer_hover(SE, false, false);
    s.pointer_down(SE, false);
    s.pointer_up(SE, false, false);
    s.pointer_hover(SE, false, false);
    s.double_click(SE, false, false);
    let view = s.transform_entry().expect("size entry on the SE handle");
    assert_eq!(view.kind, "size");
    let w0: f64 = view.fields[0].prefill.parse().unwrap();
    let h0: f64 = view.fields[1].prefill.parse().unwrap();
    let _ = s.commit_transform_entry(&format!("{}", w0 * 2.0), &format!("{h0}"), 0);
    assert_eq!(snap(&s, id), before, "nothing changes");
    assert!(s.take_transform_notice().contains("scaled evenly"));
}

#[test]
fn an_uneven_resize_of_a_plain_path_still_applies() {
    let (doc, id) = triangle(false);
    let mut s = selected(&doc);
    let before = snap(&s, id);
    drag(&mut s, SE, Point::new(70.0, 40.0));
    assert_ne!(snap(&s, id).anchors, before.anchors);
}

/// The profiled triangle, its linked clone 100 mm to the right (box (110, 10)-(150, 40)), and a
/// session with the clone selected; `group` clones a group of the triangle instead.
fn clone_selected(group: bool) -> (Session, NodeId) {
    let (doc, id) = triangle(true);
    let original = if group {
        let plain = doc.create_path(
            &[
                NewAnchor::corner(AnchorId::new(98, 7), Point::new(20.0, 30.0)),
                NewAnchor::corner(AnchorId::new(98, 8), Point::new(40.0, 30.0)),
            ],
            false,
        );
        doc.group_objects(&[id, plain]).unwrap()
    } else {
        id
    };
    let clone = doc
        .place_linked_clones(&[original], curvyo_document_core::Vec2::new(100.0, 0.0))
        .unwrap()[0];
    let mut s = Session::open(777, &pack(&doc, "test").unwrap()).unwrap();
    let _ = s.draw_list();
    s.set_tool(Tool::Select);
    let at = Point::new(130.0, 10.0);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    assert_eq!(s.selected_object_count(), 1, "setup: the clone is selected");
    (s, clone)
}

fn packed(session: &Session, id: NodeId) -> curvyo_document_core::ObjectSnapshot {
    unpack(1, &session.pack("test").unwrap())
        .unwrap()
        .object(id)
        .unwrap()
}

#[test]
fn an_uneven_resize_of_a_plain_clone_of_a_profiled_path_is_refused() {
    let (mut s, clone) = clone_selected(false);
    let before = packed(&s, clone);
    drag(&mut s, Point::new(150.0, 40.0), Point::new(170.0, 40.0));
    assert_eq!(packed(&s, clone), before, "nothing changes");
    assert!(s.take_transform_notice().contains("scaled evenly"));
}

#[test]
fn an_uneven_resize_of_a_clone_of_a_group_with_a_profiled_member_is_refused() {
    let (mut s, clone) = clone_selected(true);
    let before = packed(&s, clone);
    drag(&mut s, Point::new(150.0, 40.0), Point::new(170.0, 40.0));
    assert_eq!(packed(&s, clone), before, "nothing changes");
    assert!(s.take_transform_notice().contains("scaled evenly"));
}

/// UX review U8: the refusal is flagged so the host shows it beside the pointer.
#[test]
fn the_uneven_resize_refusal_is_flagged_as_a_refusal() {
    let (doc, _) = triangle(true);
    let mut s = selected(&doc);
    drag(&mut s, SE, Point::new(70.0, 40.0));
    assert!(s.transform_notice_is_refusal());
    assert!(s.take_transform_notice().contains("scaled evenly"));
    assert!(!s.transform_notice_is_refusal(), "taken");
}
