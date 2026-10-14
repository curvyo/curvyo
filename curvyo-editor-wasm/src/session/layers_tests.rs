//! Session-level tests of `specs/0050-layers-and-objects-panel` criteria 19, 20, 24 and 42: every
//! consumer of the canvas reads the effective flags from the one document read. One table: a
//! hidden layer with a rectangle, a visible layer with a hidden, a locked and a plain rectangle,
//! and a loose rectangle. A new consumer that reads unfiltered objects fails here.

use curvyo_document_core::{
    Length, MovePlan, MoveTarget, NodeId, ObjectSnapshot, Point, RectBounds,
};
use curvyo_ui_core::content_bounds;

use super::{KeyOutcome, Session, Tool};
use crate::KeyInput;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect(session: &Session, x: f64) -> NodeId {
    session.document.create_rect(RectBounds {
        origin: pt(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn into(session: &Session, nodes: &[NodeId], layer: NodeId) {
    session
        .document
        .move_nodes(&MovePlan {
            nodes: nodes.to_vec(),
            target: MoveTarget::FrontOf(Some(layer)),
        })
        .unwrap();
}

/// The ids of the table: `in_hidden_layer` at x 0, `hidden` at 20, `locked` at 40, `plain` at
/// 60, `loose` at 80.
struct Table {
    session: Session,
    in_hidden_layer: NodeId,
    hidden: NodeId,
    locked: NodeId,
    plain: NodeId,
    loose: NodeId,
}

fn table() -> Table {
    let mut session = Session::new(1);
    let hidden_layer = session.document.new_layer(None);
    let in_hidden_layer = rect(&session, 0.0);
    into(&session, &[in_hidden_layer], hidden_layer);
    session
        .document
        .set_node_visible(&[hidden_layer], false)
        .unwrap();
    let layer = session.document.new_layer(None);
    let (hidden, locked, plain) = (
        rect(&session, 20.0),
        rect(&session, 40.0),
        rect(&session, 60.0),
    );
    into(&session, &[hidden, locked, plain], layer);
    session.document.set_node_visible(&[hidden], false).unwrap();
    session.document.set_node_locked(&[locked], true).unwrap();
    let loose = rect(&session, 80.0);
    session.selection.clear();
    session.sync_context();
    Table {
        session,
        in_hidden_layer,
        hidden,
        locked,
        plain,
        loose,
    }
}

fn click(session: &mut Session, at: Point, alt: bool) {
    session.held.alt = alt;
    session.pointer_hover(at, false, false);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
    session.held.alt = false;
}

fn ids(objects: &[ObjectSnapshot]) -> Vec<NodeId> {
    objects.iter().map(ObjectSnapshot::id).collect()
}

#[test]
fn the_frame_draws_the_visible_and_locked_leaves_in_stacking_order() {
    let t = table();
    assert_eq!(
        ids(&t.session.render_objects()),
        vec![t.locked, t.plain, t.loose]
    );
}

#[test]
fn a_click_never_selects_a_hidden_or_locked_object() {
    let mut t = table();
    for (x, name) in [(0.0, "in hidden layer"), (20.0, "hidden"), (40.0, "locked")] {
        click(&mut t.session, pt(x, 5.0), false);
        assert!(t.session.selection.is_empty(), "{name}");
    }
    click(&mut t.session, pt(60.0, 5.0), false);
    assert_eq!(
        t.session.selection.ids(),
        &[t.plain],
        "layers are transparent to a click"
    );
}

#[test]
fn the_alt_click_cycle_skips_hidden_and_locked_objects() {
    let mut t = table();
    click(&mut t.session, pt(40.0, 5.0), true);
    assert!(t.session.selection.is_empty());
    click(&mut t.session, pt(60.0, 5.0), true);
    assert_eq!(t.session.selection.ids(), &[t.plain]);
}

#[test]
fn a_marquee_selects_only_visible_unlocked_objects_across_layers() {
    let mut t = table();
    t.session.pointer_hover(pt(-5.0, -5.0), false, false);
    t.session.pointer_down(pt(-5.0, -5.0), false);
    t.session.pointer_hover(pt(100.0, 20.0), false, false);
    t.session.pointer_up(pt(100.0, 20.0), false, false);
    assert_eq!(t.session.selection.ids(), &[t.plain, t.loose]);
}

#[test]
fn ctrl_a_selects_across_layers_without_hidden_or_locked_objects() {
    let mut t = table();
    let outcome = t.session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    assert_eq!(outcome, KeyOutcome::SelectedAll);
    assert_eq!(t.session.selection.ids(), &[t.plain, t.loose]);
}

#[test]
fn the_eyedropper_samples_a_locked_object_and_not_a_hidden_one() {
    let t = table();
    let visible = ids(&t.session.document_read().visible);
    assert!(visible.contains(&t.locked));
    assert!(!visible.contains(&t.hidden) && !visible.contains(&t.in_hidden_layer));
}

#[test]
fn fit_to_content_measures_only_visible_objects() {
    let t = table();
    let (min, max) = content_bounds(&t.session.document).unwrap();
    assert!(
        (min.x - 40.0).abs() < 1e-9,
        "the hidden ones left of 40 do not count"
    );
    assert!((max.x - 90.0).abs() < 1e-9);
}

#[test]
fn the_pen_and_the_node_tool_see_no_hidden_or_locked_path() {
    let mut t = table();
    t.session.set_tool(Tool::Node);
    let paths: Vec<NodeId> = t
        .session
        .pickable_objects()
        .iter()
        .map(ObjectSnapshot::id)
        .collect();
    assert_eq!(
        paths,
        vec![t.plain, t.loose],
        "what the Node tool and the Pen read"
    );
}

/// Draws a rectangle with the Rectangle tool from `from` to `to`.
fn draw_rect(session: &mut Session, from: Point, to: Point) {
    session.set_tool(Tool::Rectangle);
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(to, false, false);
    session.pointer_up(to, false, false);
}

/// Customer feedback 2026-10-14 (criterion 35 amended): the first drawn object of a document
/// with no layer makes the layer "Initial Layer" and goes into it, in the object's one step.
#[test]
fn drawing_into_a_document_with_no_layer_makes_the_initial_layer_in_the_same_step() {
    let mut session = Session::new(1);
    let steps = session.document.own_step_count();
    draw_rect(&mut session, pt(10.0, 10.0), pt(40.0, 40.0));
    assert_eq!(session.document.own_step_count(), steps + 1);
    let roots = session.document.children(None);
    assert_eq!(roots.len(), 1);
    let layer = roots[0];
    assert!(session.document.is_layer(layer));
    assert_eq!(
        session
            .document
            .node_registers(layer)
            .unwrap()
            .name
            .as_deref(),
        Some("Initial Layer")
    );
    assert_eq!(session.document.children(Some(layer)).len(), 1);
    assert_eq!(session.active_layer(), Some(layer));
    // One Ctrl+Z removes both.
    session.undo();
    assert_eq!(session.document.children(None), Vec::<NodeId>::new());
}

/// A document that already has loose objects keeps them where they are (no migration); only the
/// new drawn object goes into the Initial Layer. An object made by another route (here a write
/// with the Select tool active) makes no layer.
#[test]
fn a_document_with_loose_objects_keeps_them_and_only_the_drawn_object_goes_in() {
    let mut session = Session::new(1);
    let loose = rect(&session, 0.0);
    assert_eq!(session.document.parent_node(loose), None);
    draw_rect(&mut session, pt(10.0, 10.0), pt(40.0, 40.0));
    let roots = session.document.children(None);
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0], loose, "the loose object stays at the top level");
    assert!(session.document.is_layer(roots[1]));
}

/// Criterion 20, customer feedback 2026-10-14: a press on a locked object shows no message, and
/// the cursor shows the lock while it hovers a locked object, its own or its layer's lock.
#[test]
fn a_locked_object_shows_the_lock_cursor_and_a_press_on_it_posts_no_notice() {
    let mut t = table();
    t.session.pointer_hover(pt(40.0, 5.0), false, false);
    assert_eq!(t.session.cursor_hint(), "locked");
    t.session.pointer_hover(pt(60.0, 5.0), false, false);
    assert_eq!(t.session.cursor_hint(), "default");
    click(&mut t.session, pt(40.0, 5.0), false);
    assert!(t.session.selection.is_empty());
    assert_eq!(t.session.take_layer_notice_json(), "", "no empty notice");
    assert_eq!(super::layers_panel::PanelReply::Quiet.to_json(), "");
    // Locked through its layer.
    let layer = t.session.document.parent_node(t.plain).unwrap();
    t.session.document.set_node_locked(&[layer], true).unwrap();
    t.session.pointer_hover(pt(60.0, 5.0), false, false);
    assert_eq!(t.session.cursor_hint(), "locked");
    // The Offset tool hit-tests for its press too.
    t.session.set_tool(Tool::Offset);
    t.session.pointer_hover(pt(40.0, 5.0), false, false);
    assert_eq!(t.session.cursor_hint(), "locked");
    // A hidden object is not there: no lock over it.
    t.session.set_tool(Tool::Select);
    t.session.pointer_hover(pt(20.0, 5.0), false, false);
    assert_eq!(t.session.cursor_hint(), "default");
}
