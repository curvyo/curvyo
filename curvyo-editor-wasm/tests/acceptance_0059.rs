//! Independent specification 0059 session checks derived from the acceptance plan.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, DocumentSize, NodeId, Point, RectBounds, Vec2, pack, unpack};
use curvyo_editor_wasm::{DocumentSide, KeyInput, Session, Tool};

fn rect(d: &Document, x: f64, y: f64, width: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + width, y + 10.0),
    ))
}
fn opened(s: &Session) -> Document {
    unpack(599, &s.pack("acceptance").unwrap()).unwrap()
}
fn bounds(d: &Document, id: NodeId) -> (Point, Point) {
    curvyo_ui_core::object_bounds(&d.object(id).unwrap())
}
fn select_all(s: &mut Session) {
    s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}
fn press(s: &mut Session, point: Point) {
    s.pointer_down(point, false);
    s.pointer_up(point, false, false);
}

#[test]
fn ac4_overlapping_frame_resize_centers_exclusive_content_but_keeps_shared_and_partial() {
    let d = Document::new(600);
    let p = d.add_page();
    d.move_page(&p.id, Vec2::new(100.0 - p.origin.x, 0.0), &[])
        .unwrap();
    let shared = rect(&d, 120.0, 30.0, 20.0);
    let exclusive = rect(&d, 250.0, 30.0, 20.0);
    let partial = rect(&d, 300.0, 60.0, 30.0);
    let mut s = Session::open(601, &pack(&d, "acceptance").unwrap()).unwrap();
    s.activate_page(&p.id);
    let steps = s.undo_len();
    s.set_document_side(DocumentSide::Width, "230");
    let changed = opened(&s);
    assert_eq!(changed.pages()[0], d.pages()[0]);
    assert_eq!(changed.pages()[1].origin, Point::new(100.0, 0.0));
    assert_eq!(bounds(&changed, exclusive).0, Point::new(260.0, 30.0));
    assert_eq!(bounds(&changed, shared), bounds(&d, shared));
    assert_eq!(bounds(&changed, partial), bounds(&d, partial));
    assert_eq!(s.undo_len(), steps + 1);
    s.undo();
    assert_eq!(opened(&s).pages(), d.pages());
    assert_eq!(bounds(&opened(&s), exclusive), bounds(&d, exclusive));
}

#[test]
fn ac9_ac13_frame_only_state_does_not_persist_and_content_moves_once_after_reopen() {
    let d = Document::new(602);
    let p = d.add_page();
    let object = rect(&d, p.origin.x + 10.0, 10.0, 20.0);
    let mut s = Session::open(603, &pack(&d, "acceptance").unwrap()).unwrap();
    s.set_page_frame_only(&p.id, true);
    s.activate_page(&p.id);
    let mut reopened = Session::open(604, &s.pack("acceptance").unwrap()).unwrap();
    assert_eq!(reopened.active_page().id, "first");
    assert!(reopened.move_page_to(&p.id, Point::new(p.origin.x + 30.0, 20.0)));
    assert_eq!(
        bounds(&opened(&reopened), object).0,
        Point::new(p.origin.x + 40.0, 30.0)
    );
    reopened.undo();
    assert_eq!(bounds(&opened(&reopened), object), bounds(&d, object));
}

#[test]
fn ac12_unset_and_stale_context_select_all_but_hover_does_not_replace_pressed_context() {
    let d = Document::new(605);
    let p = d.add_page();
    let first = rect(&d, 10.0, 10.0, 20.0);
    let second = rect(&d, p.origin.x + 10.0, 10.0, 20.0);
    let mut s = Session::open(606, &pack(&d, "acceptance").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    select_all(&mut s);
    s.delete_selected();
    assert_eq!(opened(&s).object_ids(), []);
    s.undo();
    press(&mut s, Point::new(100.0, 100.0));
    s.pointer_hover(Point::new(p.origin.x + 100.0, 100.0), false, false);
    select_all(&mut s);
    s.delete_selected();
    assert_eq!(opened(&s).object_ids(), vec![second]);
    assert!(opened(&s).object(first).is_none());
    s.undo();
    s.activate_page(&p.id);
    select_all(&mut s);
    s.delete_selected();
    assert_eq!(opened(&s).object_ids(), []);
}

#[test]
fn ac2_ac3_ac6_active_row_changes_do_not_enter_history_and_add_uses_last_page() {
    let mut s = Session::new(607);
    let second = s.add_page();
    s.set_document_side(DocumentSide::Width, "100");
    s.set_document_side(DocumentSide::Height, "80");
    s.activate_page("first");
    let steps = s.undo_len();
    let third = s.add_page();
    assert_eq!(s.active_page().id, third);
    assert_eq!(s.active_page().size, DocumentSize::from_mm(100.0, 80.0));
    assert_eq!(s.undo_len(), steps + 1);
    s.activate_page(&second);
    assert_eq!(s.undo_len(), steps + 1);
    s.delete_page(&second);
    assert_eq!(s.active_page().id, third);
    s.delete_page(&third);
    assert_eq!(s.active_page().id, "first");
    assert!(!s.delete_page("first"));
}

#[test]
fn ac10_ac11_cross_page_clones_share_a_layer_and_still_follow_source_style_after_frame_move() {
    use curvyo_document_core::{
        AnchorId, Color, MovePlan, MoveTarget, NewAnchor, ObjectSnapshot, StyleEdit,
    };
    let d = Document::new(608);
    let p = d.add_page();
    let source = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(608, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(608, 2), Point::new(30.0, 10.0)),
            NewAnchor::corner(AnchorId::new(608, 3), Point::new(30.0, 30.0)),
        ],
        true,
    );
    let clone = d.linked_clone(&[source], &[]).unwrap()[0];
    d.translate_objects(&[clone], Vec2::new(p.origin.x, 0.0))
        .unwrap();
    let layer = d.new_layer(None);
    d.move_nodes(&MovePlan {
        nodes: vec![source, clone],
        target: MoveTarget::FrontOf(Some(layer)),
    })
    .unwrap();
    let mut s = Session::open(609, &pack(&d, "acceptance").unwrap()).unwrap();
    assert!(s.move_page_to("first", Point::new(-40.0, 20.0)));
    let moved = opened(&s);
    assert_eq!(bounds(&moved, clone), bounds(&d, clone));
    assert_eq!(moved.parent_node(source), Some(layer));
    assert_eq!(moved.parent_node(clone), Some(layer));
    let red = Color {
        r: 210,
        g: 12,
        b: 34,
    };
    moved
        .edit_style(&[source], &StyleEdit::StrokeColor(red))
        .unwrap();
    let Some(ObjectSnapshot::Clone(resolved)) = moved.object(clone) else {
        panic!("clone link lost")
    };
    assert_eq!(resolved.resolved.style.stroke.color, red);
    moved.set_node_visible(&[layer], false).unwrap();
    let reads = moved.read_nodes();
    assert!(
        reads
            .iter()
            .filter(|node| node.id == source || node.id == clone)
            .all(|node| !node.visible)
    );
    assert_eq!(moved.pages().len(), 2);
}
