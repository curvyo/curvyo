//! End-to-end session page commands and pointer context.
#![allow(clippy::unwrap_used)]
mod cpu_time;
use curvyo_document_core::{
    Document, DocumentSize, ObjectSnapshot, Point, RectBounds, Shape, pack, unpack,
};
use curvyo_editor_wasm::{DocumentSide, KeyInput, Session, Tool};
fn document(s: &Session) -> Document {
    unpack(99, &s.pack("test").unwrap()).unwrap()
}
fn rectangle(
    d: &Document,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> curvyo_document_core::NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + width, y + height),
    ))
}
fn origin(d: &Document, id: curvyo_document_core::NodeId) -> Point {
    match d.object(id).unwrap() {
        ObjectSnapshot::Primitive(p) => match p.shape {
            Shape::Rect { bounds, .. } => bounds.origin,
            _ => panic!(),
        },
        _ => panic!(),
    }
}
#[test]
fn active_resize_moves_only_exclusive_content_and_rulers_follow_origin() {
    let d = Document::new(1);
    let second = d.add_page();
    let inside = rectangle(&d, second.origin.x + 10.0, 10.0, 20.0, 20.0);
    let partial = rectangle(&d, second.origin.x - 10.0, 50.0, 20.0, 20.0);
    let first = rectangle(&d, 10.0, 10.0, 20.0, 20.0);
    let mut s = Session::open(2, &pack(&d, "test").unwrap()).unwrap();
    let undo = s.undo_len();
    s.activate_page(&second.id);
    assert_eq!(s.undo_len(), undo);
    s.set_document_side(DocumentSide::Width, "310");
    let changed = document(&s);
    assert_eq!(
        origin(&changed, inside),
        Point::new(second.origin.x + 60.0, 10.0)
    );
    assert_eq!(origin(&changed, partial), origin(&d, partial));
    assert_eq!(origin(&changed, first), origin(&d, first));
    assert_eq!(changed.pages()[0], d.pages()[0]);
    s.resize_viewport(1200.0, 800.0);
    s.activate_page("first");
    s.activate_page(&second.id);
    let (x, y) = s.view().document_to_screen(s.active_page().origin);
    s.begin_pan(x, y);
    s.pan_to(100.0, 100.0);
    s.end_pan();
    let layout = s.ruler_layout(curvyo_ui_core::RulerAxis::Horizontal, 1200.0, 7.0, 7.0);
    let expected = s.view().document_to_screen(s.active_page().origin).0;
    assert!((layout.origin_px.unwrap() - expected).abs() < 1e-9);
}
#[test]
fn frame_drag_carries_contained_objects_is_cancelable_and_undoes_once() {
    let d = Document::new(3);
    let second = d.add_page();
    let inside = rectangle(&d, second.origin.x + 10.0, 10.0, 20.0, 20.0);
    let partial = rectangle(&d, second.origin.x - 10.0, 50.0, 20.0, 20.0);
    let mut s = Session::open(3, &pack(&d, "test").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    let start = Point::new(second.origin.x - 0.5, 100.0);
    let end = Point::new(start.x + 30.0, 120.0);
    s.pointer_down(start, false);
    s.pointer_hover(end, false, false);
    s.escape();
    assert_eq!(document(&s).pages(), d.pages());
    let count = s.undo_len();
    s.pointer_down(start, false);
    s.pointer_hover(end, false, false);
    s.pointer_up(end, false, false);
    assert_eq!(s.undo_len(), count + 1);
    let moved = document(&s);
    assert_eq!(
        origin(&moved, inside),
        Point::new(second.origin.x + 40.0, 30.0)
    );
    assert_eq!(origin(&moved, partial), origin(&d, partial));
    s.undo();
    assert_eq!(document(&s).pages(), d.pages());
    assert_eq!(origin(&document(&s), inside), origin(&d, inside));
    s.set_page_frame_only(&second.id, true);
    s.move_page_to(&second.id, Point::new(600.0, 30.0));
    assert_eq!(origin(&document(&s), inside), origin(&d, inside));
}
#[test]
fn select_all_uses_last_canvas_page_context_or_the_whole_drawing() {
    let d = Document::new(4);
    let p = d.add_page();
    rectangle(&d, 10.0, 10.0, 20.0, 20.0);
    rectangle(&d, p.origin.x + 10.0, 10.0, 20.0, 20.0);
    let bytes = pack(&d, "test").unwrap();
    let mut s = Session::open(5, &bytes).unwrap();
    s.set_tool(Tool::Select);
    s.pointer_down(Point::new(100.0, 100.0), false);
    s.pointer_up(Point::new(100.0, 100.0), false, false);
    s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    s.delete_selected();
    assert_eq!(document(&s).object_ids().len(), 1);
    s.undo();
    s.pointer_down(Point::new(-100.0, -100.0), false);
    s.pointer_up(Point::new(-100.0, -100.0), false, false);
    s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    s.delete_selected();
    assert_eq!(document(&s).object_ids(), []);
}
#[test]
fn active_page_falls_back_and_does_not_persist() {
    let mut s = Session::new(6);
    let id = s.add_page();
    s.rename_page(&id, "Board");
    s.delete_page(&id);
    assert_eq!(s.active_page().id, "first");
    s.undo();
    s.activate_page(&id);
    assert_eq!(
        Session::open(7, &s.pack("test").unwrap())
            .unwrap()
            .active_page()
            .id,
        "first"
    );
}
#[test]
fn two_hundred_pages_cpu_budget() {
    let d = Document::new(8);
    for i in 1..200 {
        let p = d.add_page();
        d.resize_page(
            &p.id,
            DocumentSize::from_mm(100.0 + f64::from(i % 7), 80.0),
            &[],
        )
        .unwrap();
        rectangle(&d, p.origin.x + 10.0, p.origin.y + 10.0, 20.0, 20.0);
    }
    let mut s = Session::open(8, &pack(&d, "test").unwrap()).unwrap();
    s.resize_viewport(1000.0, 800.0);
    let (last, add) = cpu_time::cpu_time_of(|| s.add_page());
    let ((), switch) = cpu_time::cpu_time_of(|| {
        for _ in 0..20 {
            s.activate_page("first");
            s.activate_page(&last);
        }
    });
    let ((), draw) = cpu_time::cpu_time_of(|| {
        for _ in 0..20 {
            assert_ne!(s.frame_draw_list().triangles, []);
        }
    });
    eprintln!(
        "201 pages thread CPU: add {add:?}, switch average {:?}, draw average {:?}",
        switch / 40,
        draw / 20
    );
    assert!(add.as_millis() < 100);
    assert!((switch / 40).as_millis() < 20);
    assert!((draw / 20).as_millis() < 20);
}

#[test]
fn activating_offscreen_page_reveals_it_without_repeated_drift() {
    let mut s = Session::new(10);
    s.resize_viewport(1000.0, 800.0);
    let id = s.add_page();
    s.set_page_frame_only(&id, true);
    s.move_page_to(&id, Point::new(10000.0, -2000.0));
    let scale = s.view().scale();
    assert!(s.activate_page(&id));
    assert!(s.active_page().overlaps(
        s.view().screen_to_document(0.0, 0.0),
        s.view().screen_to_document(1000.0, 800.0)
    ));
    let origin = s.view().screen_to_document(0.0, 0.0);
    for _ in 0..10 {
        s.activate_page(&id);
    }
    assert_eq!(s.view().screen_to_document(0.0, 0.0), origin);
    assert!((s.view().scale() - scale).abs() < 1e-9);
}

#[test]
fn page_move_keeps_cross_page_clones_stationary_and_shared_layers_intact() {
    use curvyo_document_core::{AnchorId, MovePlan, MoveTarget, NewAnchor, Vec2};
    let d = Document::new(11);
    let second = d.add_page();
    let source = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(11, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(11, 2), Point::new(30.0, 10.0)),
            NewAnchor::corner(AnchorId::new(11, 3), Point::new(30.0, 30.0)),
            NewAnchor::corner(AnchorId::new(11, 4), Point::new(10.0, 30.0)),
        ],
        true,
    );
    let clone = d.linked_clone(&[source], &[]).unwrap()[0];
    d.translate_objects(&[clone], Vec2::new(second.origin.x, 0.0))
        .unwrap();
    let partial = d.linked_clone(&[source], &[]).unwrap()[0];
    d.translate_objects(&[partial], Vec2::new(190.0, 0.0))
        .unwrap();
    let partial_before = curvyo_ui_core::object_bounds(&d.object(partial).unwrap());
    let layer = d.new_layer(None);
    d.move_nodes(&MovePlan {
        nodes: vec![source, clone],
        target: MoveTarget::FrontOf(Some(layer)),
    })
    .unwrap();
    d.set_node_visible(&[layer], false).unwrap();
    d.set_node_locked(&[layer], true).unwrap();
    let before = curvyo_ui_core::object_bounds(&d.object(clone).unwrap());
    let mut s = Session::open(11, &pack(&d, "test").unwrap()).unwrap();
    assert!(s.move_page_to("first", Point::new(40.0, 60.0)));
    let moved = document(&s);
    assert_eq!(
        curvyo_ui_core::object_bounds(&moved.object(clone).unwrap()),
        before
    );
    assert_eq!(
        curvyo_ui_core::object_bounds(&moved.object(source).unwrap()).0,
        Point::new(50.0, 70.0)
    );
    assert_eq!(
        curvyo_ui_core::object_bounds(&moved.object(partial).unwrap()),
        partial_before
    );
    assert_eq!(moved.parent_node(source), Some(layer));
    assert_eq!(moved.parent_node(clone), Some(layer));
    assert!(s.delete_page("first"));
    let deleted = document(&s);
    assert!(matches!(
        deleted.object(clone),
        Some(ObjectSnapshot::Clone(_))
    ));
    assert_eq!(deleted.parent_node(source), Some(layer));
    s.undo();
    s.undo();
    assert_eq!(
        curvyo_ui_core::object_bounds(&document(&s).object(source).unwrap()).0,
        Point::new(10.0, 10.0)
    );
}
