//! Session-level tests of the Layers panel (`specs/0050-layers-and-objects-panel` criteria 10 to
//! 14, 16, 18, 20, 26, 29 to 37, 44 and 53): presses on rows, the eye, the lock, rename, opacity,
//! New and Delete layer, moves, and the Pen gate.

use curvyo_document_core::{Length, MovePlan, MoveTarget, NodeId, Point, RectBounds};
use curvyo_ui_core::{FINISH_PEN_FIRST, Reorder};
use serde_json::Value;

use super::layers_edit::RowFlag;
use super::layers_panel::{PanelReply, RowPress};
use super::{Session, Tool};

fn rect(session: &Session, x: f64) -> NodeId {
    session.document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
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

fn key(id: NodeId) -> String {
    serde_json::to_string(&id).unwrap()
}

fn view(session: &mut Session) -> Value {
    serde_json::from_str(&session.layers_view_json(0, 60)).unwrap()
}

fn row(session: &mut Session, id: NodeId) -> Value {
    view(session)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == serde_json::to_value(id).unwrap())
        .cloned()
        .unwrap()
}

/// Layer "Cut" holding a, b, c (c in front), a loose d on top.
fn cut_layer() -> (Session, NodeId, [NodeId; 4]) {
    let mut session = Session::new(1);
    let layer = session.document.new_layer(None);
    session.document.rename_node(layer, "Cut").unwrap();
    let ids = [0.0, 20.0, 40.0].map(|x| rect(&session, x));
    into(&session, &ids, layer);
    let d = rect(&session, 60.0);
    session.selection.clear();
    session.sync_context();
    (session, layer, [ids[0], ids[1], ids[2], d])
}

#[test]
fn a_plain_click_selects_exactly_the_row_and_marks_it() {
    let (mut session, _, [a, b, ..]) = cut_layer();
    session.press_layer_row(&key(a), RowPress::Plain);
    assert_eq!(session.selection.ids(), &[a]);
    assert_eq!(row(&mut session, a)["mark"], "selected");
    session.press_layer_row(&key(b), RowPress::Plain);
    assert_eq!(session.selection.ids(), &[b]);
}

#[test]
fn ctrl_toggles_and_shift_takes_the_range_among_siblings() {
    let (mut session, _, [a, b, c, d]) = cut_layer();
    session.press_layer_row(&key(a), RowPress::Plain);
    session.press_layer_row(&key(c), RowPress::Toggle);
    assert_eq!(session.selection.ids(), &[a, c]);
    session.press_layer_row(&key(a), RowPress::Toggle);
    assert_eq!(session.selection.ids(), &[c]);
    session.press_layer_row(&key(c), RowPress::Range);
    assert_eq!(session.selection.ids(), &[a, b, c]);
    // d is no sibling of the anchor c: only d.
    session.press_layer_row(&key(d), RowPress::Range);
    assert_eq!(session.selection.ids(), &[d]);
}

#[test]
fn a_click_on_a_row_inside_a_group_enters_the_group() {
    let (mut session, _, [a, b, ..]) = cut_layer();
    let group = session.document.group_objects(&[a, b]).unwrap();
    session.sync_context();
    session.press_layer_row(&key(a), RowPress::Plain);
    assert_eq!(session.context_group(), Some(group));
    assert_eq!(session.selection.ids(), &[a]);
    session.press_layer_row(&key(group), RowPress::Plain);
    assert_eq!(
        session.context_group(),
        None,
        "the group's parent is a layer: the top level"
    );
    assert_eq!(session.selection.ids(), &[a, b]);
}

#[test]
fn a_layer_row_activates_the_layer_and_clears_the_selection() {
    let (mut session, layer, [a, .., d]) = cut_layer();
    let other = session.document.new_layer(None);
    session.press_layer_row(&key(d), RowPress::Plain);
    assert_eq!(session.active_layer(), None, "a loose object makes none");
    session.press_layer_row(&key(layer), RowPress::Plain);
    assert!(session.selection.is_empty());
    assert_eq!(session.active_layer(), Some(layer));
    assert_eq!(row(&mut session, layer)["mark"], "active");
    assert_eq!(view(&mut session)["opacity"]["caption"], "Layer Cut");
    session.press_layer_row(&key(a), RowPress::Plain);
    assert_eq!(session.active_layer(), Some(layer));
    let _ = other;
}

#[test]
fn hidden_and_locked_rows_ignore_a_click() {
    let (mut session, _, [a, b, ..]) = cut_layer();
    session.document.set_node_visible(&[a], false).unwrap();
    session.document.set_node_locked(&[b], true).unwrap();
    session.press_layer_row(&key(a), RowPress::Plain);
    session.press_layer_row(&key(b), RowPress::Plain);
    assert!(session.selection.is_empty());
    assert_eq!(row(&mut session, a)["refusal"], "Hidden");
    assert_eq!(row(&mut session, b)["refusal"], "Locked");
}

#[test]
fn select_contents_takes_the_visible_unlocked_children_and_says_why_not() {
    let (mut session, layer, [a, b, c, _]) = cut_layer();
    session.document.set_node_locked(&[b], true).unwrap();
    assert_eq!(
        session.select_layer_contents(&key(layer)),
        PanelReply::Quiet
    );
    assert_eq!(session.selection.ids(), &[a, c]);
    let empty = session.document.new_layer(None);
    session.document.rename_node(empty, "Engrave").unwrap();
    assert_eq!(
        session.select_layer_contents(&key(empty)),
        PanelReply::Notice("Layer Engrave is empty.".into())
    );
    session.document.set_node_visible(&[layer], false).unwrap();
    assert_eq!(
        session.select_layer_contents(&key(layer)),
        PanelReply::Notice("Layer Cut is hidden.".into())
    );
}

#[test]
fn the_eye_on_a_selected_row_hides_every_selected_row_in_one_step() {
    let (mut session, _, [a, b, c, _]) = cut_layer();
    session.press_layer_row(&key(a), RowPress::Plain);
    session.press_layer_row(&key(b), RowPress::Toggle);
    let steps = session.document.own_step_count();
    session.toggle_layer_row_flag(&key(a), RowFlag::Visible);
    assert_eq!(session.document.own_step_count(), steps + 1);
    assert!(!session.document.node_registers(a).unwrap().visible);
    assert!(!session.document.node_registers(b).unwrap().visible);
    assert!(session.document.node_registers(c).unwrap().visible);
    assert!(
        session.selection.is_empty(),
        "hidden objects leave the selection"
    );
    session.toggle_layer_row_flag(&key(c), RowFlag::Locked);
    assert!(session.document.node_registers(c).unwrap().locked);
}

#[test]
fn new_layer_and_its_rename_are_one_step() {
    let (mut session, layer, _) = cut_layer();
    session.press_layer_row(&key(layer), RowPress::Plain);
    let steps = session.document.own_step_count();
    session.new_layer_row();
    let fresh = session.active_layer().unwrap();
    assert_ne!(fresh, layer);
    assert_eq!(
        session.document.children(None)[1],
        fresh,
        "directly in front of Cut"
    );
    session.rename_layer_row(&key(fresh), "Engrave");
    assert_eq!(session.document.own_step_count(), steps + 1);
    assert_eq!(
        session
            .document
            .node_registers(fresh)
            .unwrap()
            .name
            .as_deref(),
        Some("Engrave")
    );
    session.rename_layer_row(&key(fresh), "Mark");
    assert_eq!(
        session.document.own_step_count(),
        steps + 2,
        "a later rename is its own step"
    );
}

#[test]
fn delete_layer_takes_everything_says_so_and_activates_the_next_layer_back() {
    let (mut session, layer, [a, ..]) = cut_layer();
    let front = session.document.new_layer(None);
    session.press_layer_row(&key(front), RowPress::Plain);
    assert_eq!(
        session.delete_active_layer(),
        PanelReply::Notice(format!("Deleted layer {}.", "Layer 1"))
    );
    assert_eq!(
        session.active_layer(),
        Some(layer),
        "the next one toward the back"
    );
    session.document.set_node_locked(&[layer], true).unwrap();
    assert_eq!(
        session.delete_layer_row(&key(layer)),
        PanelReply::Refusal("Layer Cut is locked. Unlock it first.".into())
    );
    session.document.set_node_locked(&[layer], false).unwrap();
    assert_eq!(
        session.delete_layer_row(&key(layer)),
        PanelReply::Notice("Deleted layer Cut and 3 objects.".into())
    );
    assert!(!session.document.contains_node(a));
    assert_eq!(session.active_layer(), None);
}

#[test]
fn the_opacity_field_writes_its_target_once_and_shows_mixed() {
    let (mut session, _, [a, b, ..]) = cut_layer();
    session
        .document
        .set_node_opacity(&[a], curvyo_document_core::Opacity::new(0.5).unwrap())
        .unwrap();
    session.press_layer_row(&key(a), RowPress::Plain);
    session.press_layer_row(&key(b), RowPress::Toggle);
    let opacity = &view(&mut session)["opacity"];
    assert_eq!(opacity["value"], Value::Null, "Mixed");
    assert_eq!(opacity["caption"], "2 objects");
    session.preview_layer_opacity(30.0);
    assert!(session.previewed_drawn().is_some());
    let steps = session.document.own_step_count();
    session.commit_layer_opacity(30.0);
    assert_eq!(session.document.own_step_count(), steps + 1);
    assert_eq!(view(&mut session)["opacity"]["value"], 30);
    assert_eq!(row(&mut session, a)["readout"], "30%");
    assert!(session.previewed_drawn().is_none());
    session.preview_layer_opacity(70.0);
    session.commit_layer_opacity_preview();
    assert_eq!(
        view(&mut session)["opacity"]["value"],
        70,
        "the release commits the preview"
    );
    session.preview_layer_opacity(10.0);
    session.cancel_layer_opacity();
    assert_eq!(
        view(&mut session)["opacity"]["value"],
        70,
        "Escape restores"
    );
}

#[test]
fn a_drop_moves_the_selected_rows_and_a_refused_drop_says_why() {
    let (mut session, layer, [a, b, c, d]) = cut_layer();
    session.press_layer_row(&key(d), RowPress::Plain);
    session.drop_layer_rows(&key(d), &key(a), "below");
    assert_eq!(session.document.children(Some(layer)), vec![d, a, b, c]);
    let other = session.document.new_layer(None);
    assert_eq!(
        session.check_layer_drop(&key(other), &key(layer), "into"),
        "Layers stay at the top level"
    );
    session.document.set_node_locked(&[other], true).unwrap();
    assert_eq!(
        session.check_layer_drop(&key(a), &key(other), "into"),
        "Locked"
    );
    assert_eq!(session.check_layer_drop(&key(a), &key(b), "above"), "");
}

#[test]
fn alt_arrows_move_the_focused_row_and_announce_where() {
    let (mut session, layer, [a, b, c, _]) = cut_layer();
    let reply = session.reorder_layer_rows(&key(a), Reorder::Forward);
    assert_eq!(session.document.children(Some(layer)), vec![b, a, c]);
    assert_eq!(
        reply,
        PanelReply::Announce(format!(
            "Moved Rectangle {} to position 2 of 3",
            a.short_id()
        ))
    );
    session.reorder_layer_rows(&key(a), Reorder::Out);
    assert_eq!(
        session.document.children(None)[1],
        a,
        "in front of the layer"
    );
}

#[test]
fn the_list_follows_a_canvas_selection_and_opens_only_the_first_rows_ancestors() {
    let (mut session, layer, [a, b, ..]) = cut_layer();
    let group = session.document.group_objects(&[a, b]).unwrap();
    session.set_layer_expanded(&key(layer), false);
    let _ = view(&mut session);
    session.selection.set(&[a, b]);
    let shown = view(&mut session);
    let first = shown["firstSelected"]
        .as_u64()
        .map(|row| usize::try_from(row).unwrap());
    assert_eq!(session.take_layer_scroll(), first);
    assert_eq!(session.take_layer_scroll(), None, "once");
    assert_eq!(
        row(&mut session, layer)["expanded"],
        true,
        "the layer opened"
    );
    assert_eq!(row(&mut session, group)["mark"], "selected");
}

/// One panel action of the table of criterion 53.
type Action = Box<dyn Fn(&mut Session) -> PanelReply>;

/// Criterion 53 as a table: every panel action that writes or selects shows the hint and
/// changes nothing while the Pen has an unfinished path.
#[test]
fn every_writing_or_selecting_action_waits_for_the_pen() {
    let (mut session, layer, [a, b, ..]) = cut_layer();
    session.set_tool(Tool::Pen);
    session.layers.active = super::layers::ActiveLayer::Chosen(None);
    for x in [200.0, 260.0] {
        session.pointer_down(Point::new(x, 200.0), false);
        session.pointer_up(Point::new(x, 200.0), false, false);
    }
    assert!(session.pen_in_progress().is_some());
    let before = session.document.version();
    let hint = PanelReply::Hint(FINISH_PEN_FIRST.to_owned());
    let actions: Vec<(&str, Action)> = vec![
        (
            "click",
            Box::new(move |s| s.press_layer_row(&key(a), RowPress::Plain)),
        ),
        (
            "ctrl",
            Box::new(move |s| s.press_layer_row(&key(a), RowPress::Toggle)),
        ),
        (
            "shift",
            Box::new(move |s| s.press_layer_row(&key(a), RowPress::Range)),
        ),
        (
            "eye",
            Box::new(move |s| s.toggle_layer_row_flag(&key(a), RowFlag::Visible)),
        ),
        (
            "lock",
            Box::new(move |s| s.toggle_layer_row_flag(&key(a), RowFlag::Locked)),
        ),
        (
            "contents",
            Box::new(move |s| s.select_layer_contents(&key(layer))),
        ),
        (
            "rename",
            Box::new(move |s| s.rename_layer_row(&key(a), "X")),
        ),
        (
            "drop",
            Box::new(move |s| s.drop_layer_rows(&key(a), &key(b), "above")),
        ),
        (
            "alt",
            Box::new(move |s| s.reorder_layer_rows(&key(a), Reorder::Forward)),
        ),
        ("delete row", Box::new(move |s| s.delete_layer_row(&key(a)))),
        ("new layer", Box::new(Session::new_layer_row)),
        (
            "delete layer",
            Box::new(move |s| s.delete_layer_row(&key(layer))),
        ),
        ("opacity", Box::new(move |s| s.commit_layer_opacity(10.0))),
    ];
    for (name, action) in actions {
        assert_eq!(action(&mut session), hint, "{name}");
        assert_eq!(session.document.version(), before, "{name}");
        assert!(session.selection.is_empty(), "{name}");
    }
    assert!(
        session.set_layer_expanded(&key(layer), false),
        "collapse still works"
    );
    session.finish_pen();
    assert_eq!(
        session.press_layer_row(&key(a), RowPress::Plain),
        PanelReply::Quiet
    );
    assert_eq!(session.selection.ids(), &[a]);
}

/// Criterion 37 (amended 2026-10-12): Delete on a focused row that is not selected deletes that
/// row only; on a selected row it deletes the selection; a locked row refuses.
#[test]
fn delete_on_a_row_acts_on_what_the_focus_shows() {
    let (mut session, layer, [a, b, c, _]) = cut_layer();
    session.press_layer_row(&key(a), RowPress::Plain);
    let steps = session.document.own_step_count();
    assert_eq!(session.delete_layer_row(&key(b)), PanelReply::Quiet);
    assert_eq!(session.document.own_step_count(), steps + 1);
    assert!(!session.document.contains_node(b));
    assert_eq!(session.selection.ids(), &[a], "the selection stays");
    session.delete_layer_row(&key(a));
    assert!(!session.document.contains_node(a));
    session.document.set_node_locked(&[c], true).unwrap();
    assert_eq!(
        session.delete_layer_row(&key(c)),
        PanelReply::Refusal(format!(
            "Rectangle {} is locked. Unlock it first.",
            c.short_id()
        ))
    );
    assert!(session.document.contains_node(c));
    assert!(session.document.is_layer(layer));
}

/// Criteria 32 and 33 (amended 2026-10-12): a step that changes nothing is said, and no step is
/// written.
#[test]
fn a_move_that_changes_nothing_is_announced() {
    let (mut session, _, [_, _, c, _]) = cut_layer();
    let steps = session.document.own_step_count();
    assert_eq!(
        session.reorder_layer_rows(&key(c), Reorder::Forward),
        PanelReply::Announce("Already at the front".into())
    );
    assert_eq!(session.document.own_step_count(), steps);
    session.press_layer_row(&key(c), RowPress::Plain);
    let outcome = session.key_down(crate::KeyInput {
        key: "Home",
        ..crate::KeyInput::default()
    });
    assert_eq!(outcome, super::KeyOutcome::Reordered);
    assert_eq!(
        session.take_layer_notice_json(),
        r#"{"kind":"status","text":"Already at the front"}"#
    );
    assert_eq!(session.document.own_step_count(), steps);
}

/// Criterion 32: the host finds the moved row to keep the focus on it.
#[test]
fn the_moved_row_can_be_found_after_a_move() {
    let (mut session, _, [a, ..]) = cut_layer();
    session.reorder_layer_rows(&key(a), Reorder::Forward);
    let shown = view(&mut session);
    let index = session.layer_row_index(&key(a)).unwrap();
    assert_eq!(shown["rows"][index]["id"], serde_json::to_value(a).unwrap());
}

/// Shift+Ctrl+L: with the first selected object inside a collapsed group, the first selected row
/// is the group's row, which shows the mark (UX review item 11).
#[test]
fn the_first_selected_row_falls_back_to_the_shown_ancestor() {
    let (mut session, layer, [a, b, ..]) = cut_layer();
    let group = session.document.group_objects(&[a, b]).unwrap();
    session.sync_context();
    // A click on a's row enters the group; the group's row stays collapsed (from the list).
    session.press_layer_row(&key(a), RowPress::Plain);
    assert_eq!(session.context_group(), Some(group));
    let _ = layer;
    let shown = view(&mut session);
    let index = shown["firstSelected"].as_u64().unwrap();
    assert_eq!(
        shown["rows"][usize::try_from(index).unwrap()]["id"],
        serde_json::to_value(group).unwrap()
    );
}
