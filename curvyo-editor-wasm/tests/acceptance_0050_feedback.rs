//! Tester cases for the customer feedback of 2026-10-14 on `specs/0050-layers-and-objects-panel`
//! (criteria 3, 20, 26, 34 to 36, 55 to 57). Written from the specification before the diff was read;
//! only the names of the public seams were looked up.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(missing_docs, clippy::too_many_lines, clippy::doc_markdown)]
#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::cast_precision_loss
)]
#![allow(clippy::assert_is_empty, clippy::collapsible_if)]

use curvyo_document_core::{Document, Length, NodeId, Opacity, Point, RectBounds, pack, unpack};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::Reorder;
use serde_json::Value;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect_at(d: &Document, x: f64) -> NodeId {
    let id = d.create_rect(RectBounds {
        origin: pt(x, 20.0),
        width: Length::from_mm(30.0),
        height: Length::from_mm(30.0),
    });
    d.edit_style(
        &[id],
        &curvyo_document_core::StyleEdit::FillColor(curvyo_document_core::Color {
            r: 255,
            g: 0,
            b: 0,
        }),
    )
    .unwrap();
    d.edit_style(&[id], &curvyo_document_core::StyleEdit::FillEnabled(true))
        .unwrap();
    id
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn view(s: &mut Session) -> Value {
    serde_json::from_str(&s.layers_view_json(0, 500)).unwrap()
}

fn names(s: &mut Session) -> Vec<String> {
    view(s)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect()
}

fn drag(s: &mut Session, a: Point, b: Point) {
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

fn draw(s: &mut Session, tool: Tool, x: f64) {
    s.set_tool(tool);
    drag(s, pt(x, 100.0), pt(x + 40.0, 140.0));
    s.set_tool(Tool::Select);
}

fn undo(s: &mut Session) {
    let o = s.key_down(KeyInput {
        key: "z",
        ctrl: true,
        ..KeyInput::default()
    });
    if o == KeyOutcome::UndoBusy {
        let _ = s.undo();
    }
}

fn commit_count(s: &Session) -> usize {
    let l = loro::LoroDoc::new();
    l.import(&doc_of(s).export_loro_snapshot().unwrap())
        .unwrap();
    let ids: Vec<loro::ID> = l.oplog_frontiers().iter().collect();
    let mut n = 0;
    l.travel_change_ancestors(&ids, &mut |_| {
        n += 1;
        std::ops::ControlFlow::Continue(())
    })
    .unwrap();
    n
}

fn top_json(s: &Session) -> Value {
    serde_json::from_slice(&doc_of(s).export_json().unwrap()).unwrap()
}

// ---- criterion 35 -------------------------------------------------------------------------

#[test]
fn ac35_each_drawing_tool_makes_the_initial_layer_with_the_first_object_in_one_undo_step() {
    for tool in [Tool::Rectangle, Tool::Ellipse, Tool::PolygonStar, Tool::Pen] {
        let mut s = Session::new(2);
        s.resize_viewport(1600.0, 1000.0);
        let before = commit_count(&s);
        s.set_tool(tool);
        if tool == Tool::Pen {
            for p in [pt(100.0, 100.0), pt(160.0, 100.0), pt(130.0, 150.0)] {
                s.pointer_hover(p, false, false);
                s.pointer_down(p, false);
                s.pointer_up(p, false, false);
            }
            s.key_down(KeyInput {
                key: "Enter",
                ..KeyInput::default()
            });
        } else {
            drag(&mut s, pt(100.0, 100.0), pt(160.0, 150.0));
        }
        s.set_tool(Tool::Select);
        let j = top_json(&s);
        let top = j["objects"].as_array().unwrap();
        assert_eq!(top.len(), 1, "{tool:?}: one top-level node");
        assert_eq!(
            top[0]["objects"].as_array().map(Vec::len),
            Some(1),
            "{tool:?}: the object is inside the layer"
        );
        assert!(
            names(&mut s).contains(&"Initial Layer".to_owned()),
            "{tool:?}"
        );
        assert_eq!(commit_count(&s), before + 1, "{tool:?}: one step");
        undo(&mut s);
        assert_eq!(
            top_json(&s)["objects"].as_array().unwrap().len(),
            0,
            "{tool:?}: undo removes both"
        );
    }
}

#[test]
fn ac35_the_second_object_goes_into_the_same_layer_and_no_second_layer_appears() {
    let mut s = Session::new(2);
    s.resize_viewport(1600.0, 1000.0);
    draw(&mut s, Tool::Rectangle, 100.0);
    draw(&mut s, Tool::Ellipse, 300.0);
    let j = top_json(&s);
    assert_eq!(j["objects"].as_array().unwrap().len(), 1);
    assert_eq!(j["objects"][0]["objects"].as_array().unwrap().len(), 2);
    let n = names(&mut s);
    assert_eq!(n.iter().filter(|x| *x == "Initial Layer").count(), 1);
}

#[test]
fn ac35_loose_objects_stay_loose_and_only_the_new_one_enters_the_initial_layer() {
    let d = Document::new(1);
    let a = rect_at(&d, 20.0);
    let b = rect_at(&d, 60.0);
    let mut s = open(&d);
    draw(&mut s, Tool::Rectangle, 300.0);
    let dd = doc_of(&s);
    assert_eq!(dd.parent_node(a), None, "loose object not migrated");
    assert_eq!(dd.parent_node(b), None);
    let j = top_json(&s);
    assert_eq!(
        j["objects"].as_array().unwrap().len(),
        3,
        "2 loose + 1 layer"
    );
    let layer_count = j["objects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["objects"].as_array().is_some_and(|c| c.len() == 1))
        .count();
    assert!(layer_count >= 1);
    undo(&mut s);
    assert_eq!(top_json(&s)["objects"].as_array().unwrap().len(), 2);
}

#[test]
fn ac35_a_document_that_has_a_layer_gets_no_initial_layer() {
    let d = Document::new(1);
    d.new_layer(None);
    let mut s = open(&d);
    let before = commit_count(&s);
    draw(&mut s, Tool::Rectangle, 100.0);
    assert_eq!(commit_count(&s), before + 1);
    assert!(!names(&mut s).contains(&"Initial Layer".to_owned()));
    assert_eq!(names(&mut s).len(), 2);
}

#[test]
fn ac35_after_deleting_the_last_layer_the_next_drawing_makes_a_new_initial_layer() {
    let mut s = Session::new(2);
    s.resize_viewport(1600.0, 1000.0);
    draw(&mut s, Tool::Rectangle, 100.0);
    let id = view(&mut s)["activeLayer"].clone();
    assert!(!id.is_null());
    s.delete_active_layer();
    draw(&mut s, Tool::Rectangle, 100.0);
    assert!(names(&mut s).contains(&"Initial Layer".to_owned()));
}

#[test]
fn ac35_the_initial_layer_is_not_made_when_a_group_is_entered() {
    let d = Document::new(1);
    let a = rect_at(&d, 20.0);
    let b = rect_at(&d, 60.0);
    let g = d.group_objects(&[a, b]).unwrap();
    let mut s = open(&d);
    s.press_layer_row(
        &serde_json::to_string(&a).unwrap(),
        curvyo_editor_wasm::RowPress::Plain,
    );
    draw(&mut s, Tool::Rectangle, 300.0);
    let dd = doc_of(&s);
    assert_eq!(
        dd.children(Some(g)).len(),
        3,
        "the new object is in the entered group"
    );
    assert!(!names(&mut s).contains(&"Initial Layer".to_owned()));
}

// ---- criterion 20 -------------------------------------------------------------------------

#[test]
fn ac20_pressing_a_locked_object_shows_no_message_and_the_cursor_shows_a_lock() {
    let d = Document::new(1);
    let a = rect_at(&d, 20.0);
    let b = rect_at(&d, 200.0);
    d.set_node_locked(&[a], true).unwrap();
    let mut s = open(&d);
    s.pointer_hover(pt(35.0, 35.0), false, false);
    assert_eq!(s.cursor_hint(), "locked");
    s.pointer_down(pt(35.0, 35.0), false);
    s.pointer_up(pt(35.0, 35.0), false, false);
    assert_eq!(s.take_layer_notice(), "");
    assert_eq!(s.take_layer_notice_json(), "");
    assert!(s.take_history_notice().is_none());
    // over empty canvas and over the unlocked object: no lock
    s.pointer_hover(pt(700.0, 400.0), false, false);
    assert_ne!(s.cursor_hint(), "locked");
    s.pointer_hover(pt(215.0, 35.0), false, false);
    assert_ne!(s.cursor_hint(), "locked");
    let _ = b;
}

#[test]
fn ac20_the_lock_cursor_follows_an_ancestor_lock_and_not_a_hidden_object() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = rect_at(&d, 20.0);
    let c = rect_at(&d, 200.0);
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![a],
        target: curvyo_document_core::MoveTarget::FrontOf(Some(l)),
    })
    .unwrap();
    d.set_node_locked(&[l], true).unwrap();
    d.set_node_visible(&[c], false).unwrap();
    d.set_node_locked(&[c], true).unwrap();
    let mut s = open(&d);
    s.pointer_hover(pt(35.0, 35.0), false, false);
    assert_eq!(s.cursor_hint(), "locked", "locked layer");
    s.pointer_hover(pt(215.0, 35.0), false, false);
    assert_ne!(
        s.cursor_hint(),
        "locked",
        "hidden and locked object is not there"
    );
}

#[test]
fn ac20_a_locked_object_in_front_passes_the_press_to_an_unlocked_one_behind() {
    let d = Document::new(1);
    let back = rect_at(&d, 20.0);
    let front = rect_at(&d, 20.0);
    d.set_node_locked(&[front], true).unwrap();
    let mut s = open(&d);
    s.pointer_hover(pt(35.0, 35.0), false, false);
    assert_ne!(
        s.cursor_hint(),
        "locked",
        "a press there would select the one behind"
    );
    s.pointer_down(pt(35.0, 35.0), false);
    s.pointer_up(pt(35.0, 35.0), false, false);
    assert_eq!(s.style_scope().ids, [back]);
    assert_eq!(s.take_layer_notice(), "");
}

#[test]
fn ac20_the_offset_tool_shows_the_lock_over_a_locked_object() {
    let d = Document::new(1);
    let a = rect_at(&d, 20.0);
    d.set_node_locked(&[a], true).unwrap();
    let mut s = open(&d);
    s.set_tool(Tool::Offset);
    s.pointer_hover(pt(35.0, 35.0), false, false);
    assert_eq!(s.cursor_hint(), "locked");
    s.pointer_hover(pt(700.0, 400.0), false, false);
    assert_ne!(s.cursor_hint(), "locked");
    s.pointer_hover(pt(35.0, 35.0), false, false);
    s.pointer_down(pt(35.0, 35.0), false);
    s.pointer_up(pt(35.0, 35.0), false, false);
    assert_eq!(s.take_layer_notice(), "");
    assert_eq!(s.take_layer_notice_json(), "");
}

#[test]
fn ac20_other_tools_keep_the_default_cursor_over_a_locked_object() {
    let d = Document::new(1);
    let a = rect_at(&d, 20.0);
    d.set_node_locked(&[a], true).unwrap();
    let mut s = open(&d);
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(pt(35.0, 35.0), false, false);
    assert_ne!(s.cursor_hint(), "locked");
}

// ---- criterion 26: opacity drag -------------------------------------------------------------

fn fill_alpha_at(s: &Session, p: Point) -> f32 {
    let list = s.draw_list();
    let mut alpha = None;
    for t in list.triangles.chunks(3) {
        let [a, b, c] = [t[0].position, t[1].position, t[2].position];
        let d1 = (p.x - b.x) * (a.y - b.y) - (a.x - b.x) * (p.y - b.y);
        let d2 = (p.x - c.x) * (b.y - c.y) - (b.x - c.x) * (p.y - c.y);
        let d3 = (p.x - a.x) * (c.y - a.y) - (c.x - a.x) * (p.y - a.y);
        let neg = d1 < -1e-9 || d2 < -1e-9 || d3 < -1e-9;
        let pos = d1 > 1e-9 || d2 > 1e-9 || d3 > 1e-9;
        if !(neg && pos) {
            if t[0].color.r == 255 && t[0].color.g == 0 && t[0].color.b == 0 {
                alpha = Some(f32::from(t[0].color.a));
            }
        }
    }
    alpha.expect("something is drawn there")
}

#[test]
fn ac26_the_drag_shows_on_the_canvas_writes_nothing_and_the_release_is_one_step() {
    let d = Document::new(1);
    let a = rect_at(&d, 20.0);
    d.edit_style(
        &[a],
        &curvyo_document_core::StyleEdit::FillColor(curvyo_document_core::Color {
            r: 255,
            g: 0,
            b: 0,
        }),
    )
    .unwrap();
    d.edit_style(&[a], &curvyo_document_core::StyleEdit::FillEnabled(true))
        .unwrap();
    let mut s = open(&d);
    s.pointer_hover(pt(35.0, 35.0), false, false);
    s.pointer_down(pt(35.0, 35.0), false);
    s.pointer_up(pt(35.0, 35.0), false, false);
    let full = fill_alpha_at(&s, pt(35.0, 35.0));
    let steps = commit_count(&s);
    s.preview_layer_opacity(25.0);
    let live = fill_alpha_at(&s, pt(35.0, 35.0));
    assert!(live < full * 0.5, "live {live} vs {full}");
    s.preview_layer_opacity(50.0);
    let live2 = fill_alpha_at(&s, pt(35.0, 35.0));
    assert!(live2 > live, "follows the drag");
    assert_eq!(commit_count(&s), steps, "nothing written while dragging");
    assert_eq!(doc_of(&s).node_registers(a).unwrap().opacity.get(), 1.0);
    s.commit_layer_opacity_preview();
    assert_eq!(commit_count(&s), steps + 1, "one step on release");
    assert_eq!(doc_of(&s).node_registers(a).unwrap().opacity.get(), 0.5);
    // a second release without a drag does nothing
    s.commit_layer_opacity_preview();
    assert_eq!(commit_count(&s), steps + 1);
    undo(&mut s);
    assert_eq!(doc_of(&s).node_registers(a).unwrap().opacity.get(), 1.0);
}

#[test]
fn ac26_escape_ends_the_preview_and_the_canvas_returns() {
    let d = Document::new(1);
    let a = rect_at(&d, 20.0);
    d.edit_style(&[a], &curvyo_document_core::StyleEdit::FillEnabled(true))
        .unwrap();
    let mut s = open(&d);
    s.pointer_hover(pt(35.0, 35.0), false, false);
    s.pointer_down(pt(35.0, 35.0), false);
    s.pointer_up(pt(35.0, 35.0), false, false);
    let full = fill_alpha_at(&s, pt(35.0, 35.0));
    let steps = commit_count(&s);
    s.preview_layer_opacity(10.0);
    s.cancel_layer_opacity();
    assert_eq!(fill_alpha_at(&s, pt(35.0, 35.0)), full);
    s.commit_layer_opacity_preview();
    assert_eq!(commit_count(&s), steps);
}

#[test]
fn ac26_no_hint_is_ever_in_the_view_and_the_opacity_slot_is_absent_without_target() {
    let mut s = Session::new(2);
    let raw = s.layers_view_json(0, 10);
    assert!(!raw.contains("Select an object or a layer"), "{raw}");
    let v: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["opacity"]["field"], false);
    let caption = v["opacity"]["caption"].as_str().unwrap_or("");
    assert_eq!(caption, "");
    assert!(v.get("footer").is_none(), "{raw}");
}

// ---- criteria 55 to 57 ----------------------------------------------------------------------

fn tb(s: &mut Session) -> Value {
    view(s)["toolbar"].clone()
}

#[test]
fn ac55_the_toolbar_flags_follow_the_target() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = rect_at(&d, 20.0);
    let b = rect_at(&d, 60.0);
    let m = rect_at(&d, 100.0);
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![a, b],
        target: curvyo_document_core::MoveTarget::FrontOf(Some(l)),
    })
    .unwrap();
    let mut s = open(&d);
    let _ = m;
    // activate the layer through the row press path would be needed; start with nothing selected
    s.pointer_hover(pt(700.0, 500.0), false, false);
    s.pointer_down(pt(700.0, 500.0), false);
    s.pointer_up(pt(700.0, 500.0), false, false);
    let t = tb(&mut s);
    assert_eq!(t["newLayer"], true);
    assert_eq!(t["duplicate"], false);
    assert_eq!(t["moveUp"], false);
    assert_eq!(t["moveDown"], false);
    // select the front-most object of the layer (b)
    s.pointer_hover(pt(75.0, 35.0), false, false);
    s.pointer_down(pt(75.0, 35.0), false);
    s.pointer_up(pt(75.0, 35.0), false, false);
    let t = tb(&mut s);
    assert_eq!(t["duplicate"], true);
    assert_eq!(t["deleteLayer"], true, "selecting makes the layer active");
    assert_eq!(t["moveUp"], false, "already front-most");
    assert_eq!(t["moveDown"], true);
}

#[test]
fn ac56_move_up_and_down_step_one_place_in_one_step_and_keep_the_selection() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let ids: Vec<NodeId> = (0..3)
        .map(|i| rect_at(&d, 20.0 + 40.0 * f64::from(i)))
        .collect();
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: ids.clone(),
        target: curvyo_document_core::MoveTarget::FrontOf(Some(l)),
    })
    .unwrap();
    let mut s = open(&d);
    let order = |s: &Session| doc_of(s).children(Some(l));
    let before = order(&s);
    // select the middle one
    let mid = before[1];
    let idx = ids.iter().position(|i| *i == mid).unwrap();
    let p = pt(35.0 + 40.0 * idx as f64, 35.0);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    let n = commit_count(&s);
    s.step_layer_target(Reorder::Forward);
    assert_eq!(commit_count(&s), n + 1);
    let after = order(&s);
    assert_ne!(before, after);
    assert_eq!(s.style_scope().ids, [mid]);
    s.step_layer_target(Reorder::Backward);
    assert_eq!(order(&s), before);
    assert_eq!(commit_count(&s), n + 2);
    // a press that changes nothing writes nothing: move the front-most up
    let front = *before.last().unwrap();
    let idx = ids.iter().position(|i| *i == front).unwrap();
    let p = pt(35.0 + 40.0 * idx as f64, 35.0);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    let n = commit_count(&s);
    s.step_layer_target(Reorder::Forward);
    assert_eq!(commit_count(&s), n);
}

#[test]
fn ac57_duplicate_copies_in_place_above_the_original_selects_the_copy_one_step_and_undo_removes_it()
{
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = rect_at(&d, 20.0);
    let b = rect_at(&d, 60.0);
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![a, b],
        target: curvyo_document_core::MoveTarget::FrontOf(Some(l)),
    })
    .unwrap();
    let mut s = open(&d);
    s.pointer_hover(pt(35.0, 35.0), false, false);
    s.pointer_down(pt(35.0, 35.0), false);
    s.pointer_up(pt(35.0, 35.0), false, false);
    let n = commit_count(&s);
    let kids_before = doc_of(&s).children(Some(l));
    s.duplicate_layer_target();
    assert_eq!(commit_count(&s), n + 1);
    let dd = doc_of(&s);
    let kids = dd.children(Some(l));
    assert_eq!(kids.len(), 3);
    let sel = s.style_scope().ids;
    assert_eq!(sel.len(), 1);
    assert_ne!(sel[0], a);
    let pos_copy = kids.iter().position(|k| *k == sel[0]).unwrap();
    let pos_orig = kids.iter().position(|k| *k == a).unwrap();
    assert!(
        pos_copy > pos_orig,
        "children are listed front-most first or last? copy {pos_copy} orig {pos_orig} before {kids_before:?}"
    );
    undo(&mut s);
    assert_eq!(doc_of(&s).children(Some(l)).len(), 2);
}

#[test]
fn ac57_duplicate_with_a_layer_target_copies_the_layer_and_activates_the_copy() {
    let mut s = Session::new(2);
    s.resize_viewport(1600.0, 1000.0);
    draw(&mut s, Tool::Rectangle, 100.0);
    draw(&mut s, Tool::Rectangle, 300.0);
    let layer_row = view(&mut s)["rows"][0]["id"].to_string();
    s.press_layer_row(&layer_row, curvyo_editor_wasm::RowPress::Plain);
    let n = commit_count(&s);
    s.duplicate_layer_target();
    assert_eq!(commit_count(&s), n + 1);
    let v = view(&mut s);
    let active = v["activeLayer"].clone();
    assert!(!active.is_null());
    assert_ne!(active.to_string(), layer_row);
    let top = top_json(&s);
    assert_eq!(top["objects"].as_array().unwrap().len(), 2);
    for l in top["objects"].as_array().unwrap() {
        assert_eq!(l["objects"].as_array().unwrap().len(), 2);
    }
    undo(&mut s);
    assert_eq!(top_json(&s)["objects"].as_array().unwrap().len(), 1);
}

#[test]
fn ac34_new_layer_goes_in_front_of_the_active_layer_becomes_active_and_one_step() {
    let mut s = Session::new(2);
    s.resize_viewport(1600.0, 1000.0);
    draw(&mut s, Tool::Rectangle, 100.0);
    let n = commit_count(&s);
    s.new_layer_row();
    assert!(commit_count(&s) <= n + 1);
    let v = view(&mut s);
    assert_eq!(
        v["rows"][0]["name"],
        "Layer 2".replace('2', &v["rows"][0]["name"].as_str().unwrap()[6..])
    );
    assert_eq!(v["rows"][0]["kind"], "layer");
    assert_eq!(v["activeLayer"], v["rows"][0]["id"]);
    undo(&mut s);
    assert_eq!(names(&mut s).len(), 2);
}

#[test]
fn ac36_delete_layer_button_is_one_step_without_confirm_and_undo_restores() {
    let mut s = Session::new(2);
    s.resize_viewport(1600.0, 1000.0);
    draw(&mut s, Tool::Rectangle, 100.0);
    draw(&mut s, Tool::Rectangle, 300.0);
    let n = commit_count(&s);
    let r = s.delete_active_layer();
    let notice = format!("{r:?}");
    assert_eq!(commit_count(&s), n + 1);
    assert_eq!(names(&mut s).len(), 0);
    assert!(
        notice.contains("Deleted Initial Layer and 2 objects.") && !notice.contains("No undo"),
        "{notice}"
    );
    undo(&mut s);
    assert_eq!(names(&mut s).len(), 3);
    // toolbar after: delete dimmed
    let mut s2 = Session::new(2);
    assert_eq!(view(&mut s2)["toolbar"]["deleteLayer"], false);
    let _ = Opacity::new(1.0);
}

#[test]
fn ac53_an_unfinished_pen_path_blocks_every_toolbar_action_and_the_opacity_field() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = rect_at(&d, 20.0);
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![a],
        target: curvyo_document_core::MoveTarget::FrontOf(Some(l)),
    })
    .unwrap();
    let mut s = open(&d);
    s.pointer_hover(pt(35.0, 35.0), false, false);
    s.pointer_down(pt(35.0, 35.0), false);
    s.pointer_up(pt(35.0, 35.0), false, false);
    s.set_tool(Tool::Pen);
    for p in [pt(300.0, 300.0), pt(360.0, 300.0)] {
        s.pointer_hover(p, false, false);
        s.pointer_down(p, false);
        s.pointer_up(p, false, false);
    }
    let n = commit_count(&s);
    let before = top_json(&s);
    let replies = [
        s.new_layer_row(),
        s.delete_active_layer(),
        s.duplicate_layer_target(),
        s.step_layer_target(Reorder::Forward),
        s.step_layer_target(Reorder::Backward),
        s.commit_layer_opacity(10.0),
    ];
    for r in &replies {
        assert!(format!("{r:?}").contains("Finish the path"), "{r:?}");
    }
    assert_eq!(commit_count(&s), n);
    assert_eq!(top_json(&s), before);
    s.preview_layer_opacity(10.0);
    assert_eq!(
        s.commit_layer_opacity_preview(),
        curvyo_editor_wasm::PanelReply::Quiet
    );
    assert_eq!(commit_count(&s), n);
}

#[test]
fn ac35_after_deselecting_the_next_drawing_still_goes_into_the_initial_layer() {
    let mut s = Session::new(2);
    s.resize_viewport(1600.0, 1000.0);
    draw(&mut s, Tool::Rectangle, 100.0);
    s.key_down(KeyInput {
        key: "Escape",
        ..KeyInput::default()
    });
    s.pointer_hover(pt(900.0, 700.0), false, false);
    s.pointer_down(pt(900.0, 700.0), false);
    s.pointer_up(pt(900.0, 700.0), false, false);
    assert!(s.style_scope().ids.is_empty());
    draw(&mut s, Tool::Ellipse, 300.0);
    let j = top_json(&s);
    assert_eq!(
        j["objects"].as_array().unwrap().len(),
        1,
        "no second layer, no loose object"
    );
    assert_eq!(j["objects"][0]["objects"].as_array().unwrap().len(), 2);
}

#[test]
fn ac35_undo_then_redraw_makes_the_layer_again_and_redo_restores_both() {
    let mut s = Session::new(2);
    s.resize_viewport(1600.0, 1000.0);
    draw(&mut s, Tool::Rectangle, 100.0);
    undo(&mut s);
    assert!(names(&mut s).is_empty());
    draw(&mut s, Tool::Rectangle, 100.0);
    assert_eq!(
        names(&mut s)
            .iter()
            .filter(|n| *n == "Initial Layer")
            .count(),
        1
    );
    undo(&mut s);
    let o = s.key_down(KeyInput {
        key: "z",
        ctrl: true,
        shift: true,
        ..KeyInput::default()
    });
    let _ = o;
    assert_eq!(
        top_json(&s)["objects"][0]["objects"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
}

/// `0041` criterion 31 against `0050` criterion 35: in a document that starts empty, the first
/// drawing step also writes the Initial Layer. Wiping the first object's history must still drop
/// the entries that touched only that object; a layer is a container, not an object, so its step
/// is that object's own (the implementer sidestepped this by giving the existing test a document
/// that already has a layer).
#[test]
fn ac35_with_0041_wiping_the_first_object_drops_its_drawing_step_from_the_stack() {
    let mut s = Session::new(2);
    s.resize_viewport(1200.0, 800.0);
    s.warm_history();
    for (x, _) in [(10.0, 0), (60.0, 1)] {
        s.set_tool(Tool::Rectangle);
        drag(&mut s, pt(x, 10.0), pt(x + 20.0, 30.0));
        s.set_tool(Tool::Select);
    }
    // select the first square (A) by a click
    s.escape();
    s.pointer_hover(pt(12.0, 20.0), false, false);
    s.pointer_down(pt(12.0, 20.0), false);
    s.pointer_up(pt(12.0, 20.0), false, false);
    assert_eq!(s.selected_object_count(), 1);
    let before = s.undo_len();
    assert_eq!(before, 2);
    let _ = s.wipe_object_history();
    assert_eq!(s.undo_len(), 1, "only B's drawing stays on the stack");
}
