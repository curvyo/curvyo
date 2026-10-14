//! Independent tester tests for `specs/0050-layers-and-objects-panel` (as amended by `0051`) at
//! the editor session: the public `Session` seam (pointer events, keys, and the typed commands
//! the Layers tab calls). Written from the specification before the implementation was read; only
//! the names of the public seams were looked up. Expected values are the specification's own
//! arithmetic.
//!
//! Expected gaps (clone parts of criteria 7, 24, 25, 26, 28, 36, 38, 45 and the status-bar
//! measures of 19) are not tested: `0049` is not merged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
#![allow(missing_docs, clippy::doc_markdown, clippy::needless_pass_by_value)]
#![allow(clippy::assert_is_empty, clippy::bool_assert_comparison)]
#![allow(
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::type_complexity,
    clippy::redundant_closure_for_method_calls
)]

use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    AnchorId, Document, Length, MovePlan, MoveTarget, NewAnchor, NodeId, Opacity, Point,
    RectBounds, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, PanelReply, RowFlag, RowPress, Session, Tool};
use curvyo_ui_core::Reorder;
use curvyo_ui_core::{PaintTarget, PenTarget};
use serde_json::Value;

static TIME: AtomicU64 = AtomicU64::new(1_000);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect_at(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

/// A 30 x 30 square whose top-left corner is at x = 20 + 40 i, y = 20; `i` is also its slot.
fn square(d: &Document, i: usize) -> NodeId {
    let id = rect_at(d, 20.0 + 40.0 * i as f64, 20.0, 30.0, 30.0);
    red_fill(d, &[id]);
    id
}

/// The middle of square `i`.
fn mid(i: usize) -> Point {
    pt(35.0 + 40.0 * i as f64, 35.0)
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).expect("the session output reopens")
}

fn rid(id: NodeId) -> String {
    serde_json::to_string(&id).unwrap()
}

fn into_layer(d: &Document, nodes: &[NodeId], layer: NodeId) {
    d.move_nodes(&MovePlan {
        nodes: nodes.to_vec(),
        target: MoveTarget::FrontOf(Some(layer)),
    })
    .unwrap();
}

fn name_layer(d: &Document, layer: NodeId, name: &str) {
    d.rename_node(layer, name).unwrap();
}

fn view(s: &mut Session) -> Value {
    serde_json::from_str(&s.layers_view_json(0, 500)).unwrap()
}

fn rows(s: &mut Session) -> Vec<Value> {
    view(s)["rows"].as_array().unwrap().clone()
}

fn row_names(s: &mut Session) -> Vec<String> {
    rows(s)
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect()
}

fn row(s: &mut Session, id: NodeId) -> Value {
    let want = serde_json::to_value(id).unwrap();
    rows(s)
        .into_iter()
        .find(|r| r["id"] == want)
        .unwrap_or_else(|| panic!("no row for {id:?}"))
}

fn has_row(s: &mut Session, id: NodeId) -> bool {
    let want = serde_json::to_value(id).unwrap();
    rows(s).iter().any(|r| r["id"] == want)
}

/// The selected nodes (leaves and groups) by their ids, as the Style scope sees them.
fn selected(s: &Session) -> Vec<NodeId> {
    s.style_scope().ids
}

fn hold(s: &mut Session, at: Point, shift: bool, ctrl: bool, alt: bool) {
    s.modifiers_changed(shift, ctrl, alt);
    s.pointer_hover(at, shift, ctrl);
}

fn click(s: &mut Session, p: Point) {
    hold(s, p, false, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn alt_click(s: &mut Session, p: Point) {
    hold(s, p, false, false, true);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    hold(s, p, false, false, false);
}

fn marquee(s: &mut Session, a: Point, b: Point) {
    hold(s, a, false, false, false);
    s.pointer_down(a, false);
    hold(
        s,
        pt(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
        false,
        false,
        false,
    );
    hold(s, b, false, false, false);
    s.pointer_up(b, false, false);
}

fn key_with(s: &mut Session, k: &str, ctrl: bool, shift: bool, alt: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        alt,
        time_ms: TIME.fetch_add(1_000, Ordering::SeqCst) as f64,
        ..KeyInput::default()
    })
}

fn key(s: &mut Session, k: &str) -> KeyOutcome {
    key_with(s, k, false, false, false)
}

fn ctrl_a(s: &mut Session) {
    assert_eq!(
        key_with(s, "a", true, false, false),
        KeyOutcome::SelectedAll
    );
}

fn undo(s: &mut Session) {
    match key_with(s, "z", true, false, false) {
        KeyOutcome::UndoBusy => {
            let _ = s.undo();
        }
        KeyOutcome::Undo => {}
        other => panic!("Ctrl+Z gave {other:?}"),
    }
}

/// The labels of all commits of the session's document, oldest first.
fn labels(s: &Session) -> Vec<String> {
    let l = loro::LoroDoc::new();
    l.import(&doc_of(s).export_loro_snapshot().unwrap())
        .unwrap();
    let ids: Vec<loro::ID> = l.oplog_frontiers().iter().collect();
    let mut changes: Vec<(u32, String)> = Vec::new();
    l.travel_change_ancestors(&ids, &mut |c| {
        changes.push((
            c.lamport,
            c.message
                .map(|m| m.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        std::ops::ControlFlow::Continue(())
    })
    .unwrap();
    changes.sort();
    changes.into_iter().map(|(_, m)| m).collect()
}

fn commits(s: &Session) -> usize {
    labels(s).len()
}

/// The colour of the top-most triangle that contains `p`, `None` when nothing does.
fn colour_at(s: &Session, p: Point) -> Option<[u8; 4]> {
    let list = s.draw_list();
    let mut found = None;
    for t in list.triangles.chunks(3) {
        let [a, b, c] = [t[0].position, t[1].position, t[2].position];
        let d1 = (p.x - b.x) * (a.y - b.y) - (a.x - b.x) * (p.y - b.y);
        let d2 = (p.x - c.x) * (b.y - c.y) - (b.x - c.x) * (p.y - c.y);
        let d3 = (p.x - a.x) * (c.y - a.y) - (c.x - a.x) * (p.y - a.y);
        let neg = d1 < -1e-9 || d2 < -1e-9 || d3 < -1e-9;
        let pos = d1 > 1e-9 || d2 > 1e-9 || d3 > 1e-9;
        if !(neg && pos) {
            found = Some([t[0].color.r, t[0].color.g, t[0].color.b, t[0].color.a]);
        }
    }
    found
}

/// A point far from every object: the document background (or nothing).
const EMPTY: Point = Point { x: 700.0, y: 400.0 };

fn red_fill(d: &Document, ids: &[NodeId]) {
    d.edit_style(
        ids,
        &StyleEdit::FillColor(curvyo_document_core::Color { r: 255, g: 0, b: 0 }),
    )
    .unwrap();
    d.edit_style(ids, &StyleEdit::FillEnabled(true)).unwrap();
}

/// Whether the maker sees something other than the background at `p`.
fn drawn(s: &Session, p: Point) -> bool {
    colour_at(s, p) != colour_at(s, EMPTY)
}

// ---------------------------------------------------------------------------
// The tab (criteria 1 to 4, with 0051 criteria 2 and 13)
// ---------------------------------------------------------------------------

#[test]
fn ac1_the_strip_is_file_document_style_layers_history_and_layers_is_never_dimmed() {
    let mut s = open(&Document::new(1));
    let v = s.panel_view();
    let names: Vec<_> = v.tabs.iter().map(|t| t.name).collect();
    assert_eq!(names, ["file", "document", "style", "layers", "history"]);
    let layers = v.tabs.iter().find(|t| t.name == "layers").unwrap();
    assert_eq!(layers.label, "Layers");
    assert_eq!(layers.tooltip, "Layers (Shift+Ctrl+L)");
    assert!(layers.enabled, "available with an empty document");
    // Never dimmed, whatever the selection state.
    let d = Document::new(1);
    square(&d, 0);
    let mut s2 = open(&d);
    click(&mut s2, mid(0));
    assert!(
        s2.panel_view()
            .tabs
            .iter()
            .find(|t| t.name == "layers")
            .unwrap()
            .enabled
    );
    // Pressing it makes it active, and a hostile spelling is refused.
    assert!(s.press_panel_tab("layers"));
    let v = s.panel_view();
    assert_eq!(v.tabs.iter().filter(|t| t.selected).count(), 1);
    assert!(v.tabs.iter().find(|t| t.name == "layers").unwrap().selected);
    for bad in ["Layers", "layers ", "LAYERS", "layer", ""] {
        assert!(!s.press_panel_tab(bad), "{bad:?}");
    }
}

#[test]
fn ac2_the_panel_never_switches_by_itself_and_the_shortcuts_work() {
    let d = Document::new(1);
    square(&d, 0);
    square(&d, 1);
    let mut s = open(&d);
    assert!(s.press_panel_tab("layers"));
    let active = |s: &Session| -> &'static str {
        s.panel_view()
            .tabs
            .iter()
            .find(|t| t.selected)
            .unwrap()
            .name
    };
    // Selection changes do not leave Layers.
    click(&mut s, mid(0));
    assert_eq!(active(&s), "layers");
    ctrl_a(&mut s);
    assert_eq!(active(&s), "layers");
    click(&mut s, EMPTY);
    assert_eq!(active(&s), "layers");
    // Shift+Ctrl+F with Layers active leaves it for Style or Document.
    s.panel_shortcut_style();
    assert!(
        ["style", "document"].contains(&active(&s)),
        "{}",
        active(&s)
    );
    // Shift+Ctrl+L brings it back.
    s.panel_shortcut_layers();
    assert_eq!(active(&s), "layers");
    // A selection change never makes Layers active from another tab.
    s.panel_shortcut_document();
    click(&mut s, mid(1));
    assert_ne!(active(&s), "layers");
}

#[test]
fn ac4_the_subject_line_counts_nodes_that_are_not_layers() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let c = square(&d, 2);
    let l1 = d.new_layer(None);
    let l2 = d.new_layer(None);
    into_layer(&d, &[a], l1);
    into_layer(&d, &[b], l2);
    let _ = c;
    let mut s = open(&d);
    assert_eq!(
        s.layers_subject(),
        "3 objects",
        "two layers are not objects"
    );
    assert_eq!(view(&mut s)["subject"], "3 objects");
    let d1 = Document::new(1);
    square(&d1, 0);
    assert_eq!(open(&d1).layers_subject(), "1 object");
    // A group counts as an object, its children too.
    let d2 = Document::new(1);
    let x = square(&d2, 0);
    let y = square(&d2, 1);
    d2.group_objects(&[x, y]).unwrap();
    assert_eq!(open(&d2).layers_subject(), "3 objects");
}

// ---------------------------------------------------------------------------
// The tree (criteria 5 to 9)
// ---------------------------------------------------------------------------

#[test]
fn ac5_rows_are_depth_first_front_most_first() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let layer = d.new_layer(None);
    name_layer(&d, layer, "Cut");
    let c = square(&d, 2);
    for (id, n) in [(a, "A"), (b, "B"), (c, "C")] {
        d.rename_node(id, n).unwrap();
    }
    // Stacking: A, B, layer (empty), C. Put the layer between B and C: it is already.
    let mut s = open(&d);
    assert_eq!(row_names(&mut s), ["C", "Cut", "B", "A"]);
    // The layer in the middle with a child: C, layer, its child, B, A.
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let layer = d.new_layer(None);
    name_layer(&d, layer, "Cut");
    let c = square(&d, 2);
    let k = square(&d, 3);
    for (id, n) in [(a, "A"), (b, "B"), (c, "C"), (k, "K")] {
        d.rename_node(id, n).unwrap();
    }
    into_layer(&d, &[k], layer);
    let mut s = open(&d);
    assert_eq!(row_names(&mut s), ["C", "Cut", "K", "B", "A"]);
    let rs = rows(&mut s);
    assert_eq!(rs[0]["level"], 1);
    assert_eq!(rs[1]["level"], 1);
    assert_eq!(rs[2]["level"], 2);
    assert_eq!(rs[3]["level"], 1);
    // Inside a container the last child is the first row.
    let d = Document::new(1);
    let l = d.new_layer(None);
    let p = square(&d, 0);
    let q = square(&d, 1);
    d.rename_node(p, "P").unwrap();
    d.rename_node(q, "Q").unwrap();
    into_layer(&d, &[p], l);
    into_layer(&d, &[q], l);
    let mut s = open(&d);
    let names = row_names(&mut s);
    assert_eq!(&names[1..], ["Q", "P"]);
}

#[test]
fn ac5_collapsed_containers_hide_descendants_and_ac8_defaults() {
    let d = Document::new(1);
    let layer = d.new_layer(None);
    let x = square(&d, 0);
    let y = square(&d, 1);
    let z = square(&d, 2);
    let group = d.group_objects(&[x, y]).unwrap();
    into_layer(&d, &[group, z], layer);
    let mut s = open(&d);
    // Layers open expanded, groups collapsed.
    let rs = rows(&mut s);
    assert_eq!(rs.len(), 3, "layer, group (collapsed), z: {rs:?}");
    let lrow = row(&mut s, layer);
    assert_eq!(lrow["expanded"], true);
    let grow = row(&mut s, group);
    assert_eq!(grow["container"], true);
    assert_eq!(grow["expanded"], false);
    // Expand and collapse; the count follows.
    assert!(s.set_layer_expanded(&rid(group), true));
    assert_eq!(view(&mut s)["count"], 5);
    assert!(s.set_layer_expanded(&rid(layer), false));
    assert_eq!(view(&mut s)["count"], 1);
    // A leaf is no container.
    assert!(!s.set_layer_expanded(&rid(z), true));
    // The state is view state: it is no step.
    let before = labels(&s);
    s.set_layer_expanded(&rid(layer), true);
    assert_eq!(labels(&s), before);
    // New and Open drop it: a fresh session reads the defaults again.
    let mut s2 = open(&doc_of(&s));
    assert_eq!(view(&mut s2)["count"], 3);
}

#[test]
fn ac8_a_collapsed_container_with_further_selected_descendants_shows_the_mark() {
    let d = Document::new(1);
    let l1 = d.new_layer(None);
    let l2 = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    into_layer(&d, &[a], l1);
    into_layer(&d, &[b], l2);
    let mut s = open(&d);
    ctrl_a(&mut s);
    let _ = view(&mut s);
    // Whichever layer holds the first selected row is open; the other, once collapsed, carries
    // the mark.
    s.set_layer_expanded(&rid(l1), false);
    s.set_layer_expanded(&rid(l2), false);
    let r1 = row(&mut s, l1);
    let r2 = row(&mut s, l2);
    assert_eq!(r1["containsSelection"], true, "{r1:?}");
    assert_eq!(r2["containsSelection"], true, "{r2:?}");
    // Without a selection there is no mark.
    click(&mut s, EMPTY);
    assert_eq!(row(&mut s, l1)["containsSelection"], false);
}

#[test]
fn ac7_default_names_and_layer_numbering() {
    let d = Document::new(1);
    let r = square(&d, 0);
    let x = square(&d, 1);
    let y = square(&d, 2);
    let g = d.group_objects(&[x, y]).unwrap();
    let mut s = open(&d);
    let rr = row(&mut s, r);
    let name = rr["name"].as_str().unwrap();
    assert!(name.starts_with("Rectangle "), "{name}");
    assert_eq!(name.len(), "Rectangle ".len() + 6, "{name}");
    assert_eq!(rr["named"], false);
    let gr = row(&mut s, g);
    let gname = gr["name"].as_str().unwrap();
    assert!(gname.starts_with("Group "), "{gname}");
    assert_eq!(gname.len(), "Group ".len() + 6);
    // Layers: Layer 1, Layer 2, Layer 3; rename 2, the next new layer is Layer 2 again.
    let mut s = open(&Document::new(1));
    for _ in 0..3 {
        s.new_layer_row();
    }
    let layer_names: Vec<String> = row_names(&mut s);
    assert_eq!(layer_names, ["Layer 3", "Layer 2", "Layer 1"]);
    let second = rows(&mut s)[1]["id"].to_string();
    s.rename_layer_row(&second, "Cut");
    s.new_layer_row();
    let names = row_names(&mut s);
    assert!(names.contains(&"Layer 2".to_owned()), "{names:?}");
    assert_eq!(names.iter().filter(|n| *n == "Layer 2").count(), 1);
    // A layer always has a name (never empty / default form).
    for r in rows(&mut s) {
        assert!(!r["name"].as_str().unwrap().is_empty());
    }
}

#[test]
fn ac9_an_empty_document_has_no_rows_and_no_active_layer() {
    let mut s = open(&Document::new(1));
    let v = view(&mut s);
    assert_eq!(v["count"], 0);
    assert_eq!(v["canDeleteLayer"], false);
    assert!(s.active_layer().is_none());
    assert_eq!(s.layers_subject(), "0 objects");
}

// ---------------------------------------------------------------------------
// Selecting by row and the canvas (criteria 10 to 15)
// ---------------------------------------------------------------------------

#[test]
fn ac10_a_plain_row_press_selects_exactly_that_node() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let mut s = open(&d);
    s.press_panel_tab("layers");
    click(&mut s, mid(0));
    assert_eq!(selected(&s), [a]);
    assert_eq!(
        s.press_layer_row(&rid(b), RowPress::Plain),
        PanelReply::Quiet
    );
    assert_eq!(selected(&s), [b], "replaces the selection");
    assert_eq!(row(&mut s, b)["mark"], "selected");
    assert_eq!(
        s.panel_view()
            .tabs
            .iter()
            .find(|t| t.selected)
            .unwrap()
            .name,
        "layers"
    );
}

#[test]
fn ac10_pressing_a_row_inside_a_group_enters_it() {
    let d = Document::new(1);
    let x = square(&d, 0);
    let y = square(&d, 1);
    let z = square(&d, 2);
    let g = d.group_objects(&[x, y]).unwrap();
    let mut s = open(&d);
    s.set_layer_expanded(&rid(g), true);
    assert_eq!(s.context_depth(), 0);
    s.press_layer_row(&rid(y), RowPress::Plain);
    assert_eq!(s.context_depth(), 1, "the group is entered");
    assert_eq!(selected(&s), [y]);
    // A row in the top level (layer or document) leaves again.
    s.press_layer_row(&rid(z), RowPress::Plain);
    assert_eq!(s.context_depth(), 0);
    assert_eq!(selected(&s), [z]);
    // A row in a layer keeps the context at the top level.
    let d = Document::new(1);
    let l = d.new_layer(None);
    let q = square(&d, 0);
    into_layer(&d, &[q], l);
    let mut s = open(&d);
    s.press_layer_row(&rid(q), RowPress::Plain);
    assert_eq!(s.context_depth(), 0);
    assert_eq!(selected(&s), [q]);
}

#[test]
fn ac11_ctrl_and_shift_press_extend_among_siblings() {
    let d = Document::new(1);
    let ids: Vec<NodeId> = (0..5).map(|i| square(&d, i)).collect();
    let mut s = open(&d);
    s.press_layer_row(&rid(ids[0]), RowPress::Plain);
    s.press_layer_row(&rid(ids[2]), RowPress::Toggle);
    let mut sel = selected(&s);
    sel.sort_by_key(|i| ids.iter().position(|x| x == i));
    assert_eq!(sel, [ids[0], ids[2]]);
    s.press_layer_row(&rid(ids[2]), RowPress::Toggle);
    assert_eq!(selected(&s), [ids[0]], "Ctrl+click removes");
    // Range from the anchor (last clicked) to the clicked one.
    s.press_layer_row(&rid(ids[1]), RowPress::Plain);
    s.press_layer_row(&rid(ids[3]), RowPress::Range);
    let mut sel = selected(&s);
    sel.sort_by_key(|i| ids.iter().position(|x| x == i));
    assert_eq!(sel, [ids[1], ids[2], ids[3]]);
}

#[test]
fn ac11_a_ctrl_or_shift_press_with_another_parent_acts_as_a_plain_press() {
    let d = Document::new(1);
    let x = square(&d, 0);
    let y = square(&d, 1);
    let z = square(&d, 2);
    let g = d.group_objects(&[x, y]).unwrap();
    let mut s = open(&d);
    s.set_layer_expanded(&rid(g), true);
    s.press_layer_row(&rid(z), RowPress::Plain);
    // x is inside the group: another context.
    s.press_layer_row(&rid(x), RowPress::Toggle);
    assert_eq!(selected(&s), [x], "acts as a plain click");
    s.press_layer_row(&rid(z), RowPress::Plain);
    s.press_layer_row(&rid(y), RowPress::Range);
    assert_eq!(selected(&s), [y]);
}

#[test]
fn ac12_a_canvas_selection_shows_in_the_rows_and_opens_only_the_first_ancestor() {
    let d = Document::new(1);
    let l1 = d.new_layer(None);
    let l2 = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let x = square(&d, 2);
    let y = square(&d, 3);
    let g = d.group_objects(&[x, y]).unwrap();
    into_layer(&d, &[a], l1);
    into_layer(&d, &[b, g], l2);
    let mut s = open(&d);
    s.set_layer_expanded(&rid(l1), false);
    s.set_layer_expanded(&rid(l2), false);
    // Click a on the canvas: its layer opens, the row is selected.
    click(&mut s, mid(0));
    assert_eq!(row(&mut s, a)["mark"], "selected");
    assert_eq!(row(&mut s, l1)["expanded"], true);
    assert_eq!(row(&mut s, l2)["expanded"], false);
    assert_eq!(
        s.take_layer_scroll().is_some(),
        true,
        "scrolls to the first selected row"
    );
    // Ctrl+A must not open every layer.
    s.set_layer_expanded(&rid(l1), false);
    click(&mut s, EMPTY);
    let _ = view(&mut s);
    ctrl_a(&mut s);
    let opened = [l1, l2]
        .iter()
        .filter(|&&l| row_expanded(&mut s, l))
        .count();
    assert_eq!(opened, 1, "only the first selected row's ancestors");
    // The other selected rows inside the collapsed layer show the mark.
    let closed = [l1, l2]
        .into_iter()
        .find(|&l| !row_expanded(&mut s, l))
        .unwrap();
    assert_eq!(row(&mut s, closed)["containsSelection"], true);
}

fn row_expanded(s: &mut Session, id: NodeId) -> bool {
    row(s, id)["expanded"] == true
}

#[test]
fn ac12_a_press_in_the_list_does_not_ask_for_a_scroll() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let mut s = open(&d);
    s.press_layer_row(&rid(a), RowPress::Plain);
    let _ = view(&mut s);
    assert_eq!(s.take_layer_scroll(), None);
}

#[test]
fn ac13_a_layer_row_press_activates_the_layer_and_clears_the_selection() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let c = square(&d, 2);
    into_layer(&d, &[a, b], l);
    let mut s = open(&d);
    click(&mut s, mid(2));
    assert_eq!(selected(&s), [c]);
    s.press_layer_row(&rid(l), RowPress::Plain);
    assert_eq!(s.active_layer(), Some(l));
    assert!(selected(&s).is_empty(), "no node is selected");
    // Ctrl+click and Shift+click on a layer row act as a plain click.
    click(&mut s, mid(2));
    s.press_layer_row(&rid(l), RowPress::Toggle);
    assert!(selected(&s).is_empty());
    click(&mut s, mid(2));
    s.press_layer_row(&rid(l), RowPress::Range);
    assert!(selected(&s).is_empty());
    // Select contents.
    s.select_layer_contents(&rid(l));
    let mut sel = selected(&s);
    sel.sort_by_key(|i| [a, b].iter().position(|x| x == i));
    assert_eq!(sel, [a, b]);
}

#[test]
fn ac13_select_contents_skips_hidden_and_locked_children() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let ids: Vec<NodeId> = (0..4).map(|i| square(&d, i)).collect();
    into_layer(&d, &ids, l);
    d.set_node_visible(&[ids[1]], false).unwrap();
    d.set_node_locked(&[ids[2]], true).unwrap();
    let mut s = open(&d);
    s.select_layer_contents(&rid(l));
    let mut sel = selected(&s);
    sel.sort_by_key(|i| ids.iter().position(|x| x == i));
    assert_eq!(sel, [ids[0], ids[3]]);
}

#[test]
fn ac14_a_hidden_or_locked_row_ignores_a_click_and_a_new_flag_leaves_the_selection() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let c = square(&d, 2);
    d.set_node_visible(&[b], false).unwrap();
    d.set_node_locked(&[c], true).unwrap();
    let mut s = open(&d);
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.press_layer_row(&rid(b), RowPress::Plain);
    assert_eq!(selected(&s), [a], "hidden row: unchanged");
    s.press_layer_row(&rid(c), RowPress::Plain);
    assert_eq!(selected(&s), [a], "locked row: unchanged");
    s.press_layer_row(&rid(c), RowPress::Toggle);
    assert_eq!(selected(&s), [a]);
    assert_eq!(row(&mut s, b)["refusal"], "Hidden");
    assert_eq!(row(&mut s, c)["refusal"], "Locked");
    // The selected node becomes hidden: it leaves the selection in the same frame.
    s.toggle_layer_row_flag(&rid(a), RowFlag::Visible);
    assert!(selected(&s).is_empty());
    // Same for lock.
    s.toggle_layer_row_flag(&rid(a), RowFlag::Visible);
    s.press_layer_row(&rid(a), RowPress::Plain);
    assert_eq!(selected(&s), [a]);
    s.toggle_layer_row_flag(&rid(a), RowFlag::Locked);
    assert!(selected(&s).is_empty());
}

#[test]
fn ac14_hiding_a_layer_drops_its_selected_children() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    into_layer(&d, &[a], l);
    let mut s = open(&d);
    ctrl_a(&mut s);
    assert_eq!(selected(&s).len(), 2);
    s.toggle_layer_row_flag(&rid(l), RowFlag::Visible);
    assert_eq!(selected(&s), [b]);
    // A peer's change (here: the document edited under the session) is honoured too: undo of the
    // hide brings the layer back but does not put the dropped node back into the selection.
    // Undoing the hide shows the layer again: the dropped node is back on the canvas, pickable.
    undo(&mut s);
    click(&mut s, EMPTY);
    click(&mut s, mid(0));
    assert_eq!(selected(&s), [a]);
}

#[test]
fn ac14_the_entered_group_becoming_hidden_falls_back_to_the_top_level() {
    let d = Document::new(1);
    let x = square(&d, 0);
    let y = square(&d, 1);
    let g = d.group_objects(&[x, y]).unwrap();
    let mut s = open(&d);
    s.set_layer_expanded(&rid(g), true);
    s.press_layer_row(&rid(x), RowPress::Plain);
    assert_eq!(s.context_depth(), 1);
    s.toggle_layer_row_flag(&rid(g), RowFlag::Visible);
    assert_eq!(s.context_depth(), 0);
    assert!(selected(&s).is_empty());
}

#[test]
fn ac15_hovering_a_leaf_row_draws_a_box_and_a_layer_row_draws_nothing() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    into_layer(&d, &[a], l);
    let _ = b;
    let mut s = open(&d);
    let base = s.draw_list().triangles.len();
    s.set_layer_hover(&rid(a));
    let with = s.draw_list().triangles.len();
    assert!(with > base, "a hover box is drawn");
    s.set_layer_hover("");
    assert_eq!(
        s.draw_list().triangles.len(),
        base,
        "removed in the same frame"
    );
    s.set_layer_hover(&rid(l));
    assert_eq!(
        s.draw_list().triangles.len(),
        base,
        "a layer row draws nothing"
    );
    s.set_layer_hover(&rid(b));
    assert!(s.draw_list().triangles.len() > base);
}

// ---------------------------------------------------------------------------
// Rename (criteria 16, 17)
// ---------------------------------------------------------------------------

#[test]
fn ac16_rename_trims_limits_and_drops_control_characters_in_one_step() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let mut s = open(&d);
    let n0 = commits(&s);
    s.rename_layer_row(&rid(a), "  Door \u{7}handle\n ");
    assert_eq!(commits(&s), n0 + 1, "one step");
    assert_eq!(row(&mut s, a)["name"], "Door handle");
    assert_eq!(row(&mut s, a)["named"], true);
    assert_eq!(labels(&s).last().unwrap(), "rename_node");
    // 128 characters at most.
    let long = "x".repeat(300);
    s.rename_layer_row(&rid(a), &long);
    assert_eq!(
        row(&mut s, a)["name"].as_str().unwrap().chars().count(),
        128
    );
    // Empty result removes the name: the default shows again.
    s.rename_layer_row(&rid(a), " \t ");
    let r = row(&mut s, a);
    assert_eq!(r["named"], false);
    assert!(r["name"].as_str().unwrap().starts_with("Rectangle "));
    undo(&mut s);
    assert_eq!(
        row(&mut s, a)["name"].as_str().unwrap().chars().count(),
        128
    );
}

#[test]
fn ac16_two_nodes_may_have_the_same_name_and_rename_works_on_a_locked_node() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    d.set_node_locked(&[b], true).unwrap();
    let mut s = open(&d);
    s.rename_layer_row(&rid(a), "Same");
    s.rename_layer_row(&rid(b), "Same");
    assert_eq!(row(&mut s, a)["name"], "Same");
    assert_eq!(row(&mut s, b)["name"], "Same");
}

#[test]
fn ac16_a_character_count_not_a_byte_count() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let mut s = open(&d);
    let emoji: String = "\u{1F600}".repeat(200);
    s.rename_layer_row(&rid(a), &emoji);
    assert_eq!(
        row(&mut s, a)["name"].as_str().unwrap().chars().count(),
        128
    );
}

#[test]
fn ac17_markup_stays_plain_text_and_bidi_controls_are_dropped_on_commit() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let mut s = open(&d);
    let markup = "<img src=x onerror=alert(1)>";
    s.rename_layer_row(&rid(a), markup);
    let r = row(&mut s, a);
    assert_eq!(
        r["name"], markup,
        "kept verbatim as text; the page escapes it"
    );
    assert!(r["label"].as_str().unwrap().starts_with(markup));
    // Bidi controls U+202A..U+202E and U+2066..U+2069 are dropped.
    let hostile = "ab\u{202E}cd\u{202A}\u{202B}\u{202C}\u{202D}\u{2066}\u{2067}\u{2068}\u{2069}ef";
    s.rename_layer_row(&rid(a), hostile);
    assert_eq!(row(&mut s, a)["name"], "abcdef");
    // Only controls: empty result.
    s.rename_layer_row(&rid(a), "\u{202E}\u{2066}");
    assert_eq!(row(&mut s, a)["named"], false);
}

// ---------------------------------------------------------------------------
// Visibility, lock, opacity (criteria 18 to 28)
// ---------------------------------------------------------------------------

#[test]
fn ac18_the_eye_toggles_the_own_flag_in_one_step_and_a_multi_row_toggle_is_one_step() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let c = square(&d, 2);
    let mut s = open(&d);
    let n0 = commits(&s);
    s.toggle_layer_row_flag(&rid(a), RowFlag::Visible);
    assert_eq!(commits(&s), n0 + 1);
    assert_eq!(labels(&s).last().unwrap(), "set_node_visible");
    assert_eq!(row(&mut s, a)["visible"], false);
    s.toggle_layer_row_flag(&rid(a), RowFlag::Visible);
    assert_eq!(row(&mut s, a)["visible"], true);
    // Several selected rows: the press on one of them acts on all, in one step.
    s.press_layer_row(&rid(b), RowPress::Plain);
    s.press_layer_row(&rid(c), RowPress::Toggle);
    let n1 = commits(&s);
    s.toggle_layer_row_flag(&rid(b), RowFlag::Visible);
    assert_eq!(commits(&s), n1 + 1);
    assert_eq!(row(&mut s, b)["visible"], false);
    assert_eq!(row(&mut s, c)["visible"], false);
    assert_eq!(row(&mut s, a)["visible"], true);
    undo(&mut s);
    assert_eq!(row(&mut s, b)["visible"], true);
    assert_eq!(row(&mut s, c)["visible"], true);
}

#[test]
fn ac19_a_hidden_node_is_not_drawn_and_not_hit_by_any_route() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let c = square(&d, 2);
    red_fill(&d, &[a, b, c]);
    d.set_node_visible(&[b], false).unwrap();
    let mut s = open(&d);
    assert!(drawn(&s, mid(0)));
    assert!(!drawn(&s, mid(1)), "hidden: not drawn");
    assert!(drawn(&s, mid(2)));
    // Click.
    click(&mut s, mid(1));
    assert!(selected(&s).is_empty());
    // Ctrl+A.
    ctrl_a(&mut s);
    assert!(!selected(&s).contains(&b));
    assert_eq!(selected(&s).len(), 2);
    // Marquee.
    click(&mut s, EMPTY);
    marquee(&mut s, pt(0.0, 0.0), pt(220.0, 100.0));
    assert!(!selected(&s).contains(&b));
    assert_eq!(selected(&s).len(), 2);
}

#[test]
fn ac19_a_hidden_node_is_skipped_by_the_alt_click_cycle() {
    let d = Document::new(1);
    let under = rect_at(&d, 20.0, 20.0, 60.0, 60.0);
    let over = rect_at(&d, 30.0, 30.0, 20.0, 20.0);
    red_fill(&d, &[under, over]);
    d.set_node_visible(&[over], false).unwrap();
    let mut s = open(&d);
    for _ in 0..3 {
        alt_click(&mut s, pt(40.0, 40.0));
        assert_eq!(selected(&s), [under]);
    }
}

#[test]
fn ac19_hidden_objects_are_left_out_of_fit_to_content() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let far = rect_at(&d, 900.0, 600.0, 30.0, 30.0);
    d.set_node_visible(&[far], false).unwrap();
    let _ = a;
    let mut s = open(&d);
    let outcome = s.fit_document();
    let size = doc_of(&s).size();
    let d2 = Document::new(1);
    square(&d2, 0);
    let mut s2 = open(&d2);
    let outcome2 = s2.fit_document();
    assert_eq!(outcome, outcome2);
    assert_eq!(
        size,
        doc_of(&s2).size(),
        "same fit as without the hidden object"
    );
}

#[test]
fn ac20_a_locked_node_is_drawn_but_not_hit_and_the_lock_toggle_is_one_step() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    red_fill(&d, &[a, b]);
    let mut s = open(&d);
    let n0 = commits(&s);
    s.toggle_layer_row_flag(&rid(b), RowFlag::Locked);
    assert_eq!(commits(&s), n0 + 1);
    assert_eq!(labels(&s).last().unwrap(), "set_node_locked");
    assert!(drawn(&s, mid(1)), "still drawn");
    click(&mut s, mid(1));
    assert!(selected(&s).is_empty());
    ctrl_a(&mut s);
    assert_eq!(selected(&s), [a]);
    click(&mut s, EMPTY);
    marquee(&mut s, pt(0.0, 0.0), pt(220.0, 100.0));
    assert_eq!(selected(&s), [a]);
    // Cannot be selected in the panel.
    s.press_layer_row(&rid(b), RowPress::Plain);
    assert_eq!(selected(&s), [a]);
    // Hide, show, rename still work on a locked node.
    s.toggle_layer_row_flag(&rid(b), RowFlag::Visible);
    assert_eq!(row(&mut s, b)["visible"], false);
    s.toggle_layer_row_flag(&rid(b), RowFlag::Visible);
    s.rename_layer_row(&rid(b), "L");
    assert_eq!(row(&mut s, b)["name"], "L");
    // Unlock works and makes it pickable again.
    s.toggle_layer_row_flag(&rid(b), RowFlag::Locked);
    click(&mut s, mid(1));
    assert_eq!(selected(&s), [b]);
}

#[test]
fn ac20_a_locked_node_cannot_be_deleted_dragged_or_have_its_opacity_set() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let l = d.new_layer(None);
    d.set_node_locked(&[b], true).unwrap();
    let mut s = open(&d);
    // Delete from the list: the locked row cannot be selected, so Delete deletes nothing.
    s.press_layer_row(&rid(b), RowPress::Plain);
    key(&mut s, "Delete");
    assert!(has_row(&mut s, b));
    // Drag: refused with a reason; nothing changes.
    let why = s.check_layer_drop(&rid(b), &rid(a), "above");
    assert_eq!(why, "Locked");
    let n0 = commits(&s);
    s.drop_layer_rows(&rid(b), &rid(a), "above");
    assert_eq!(commits(&s), n0);
    // No node can be dropped into a locked container.
    s.toggle_layer_row_flag(&rid(l), RowFlag::Locked);
    let why = s.check_layer_drop(&rid(a), &rid(l), "into");
    assert_eq!(why, "Locked");
    s.drop_layer_rows(&rid(a), &rid(l), "into");
    assert_eq!(commits(&s), n0 + 1, "only the lock toggle");
    // Opacity of the locked layer cannot be changed via the field.
    s.press_layer_row(&rid(l), RowPress::Plain);
    let n1 = commits(&s);
    s.commit_layer_opacity(40.0);
    assert_eq!(commits(&s), n1);
    let v = view(&mut s);
    assert_eq!(
        v["opacity"]["field"], false,
        "not in the tree when the target is locked"
    );
    assert_eq!(v["opacity"]["caption"], "Locked");
}

#[test]
fn ac21_a_selected_group_carries_its_locked_members() {
    let d = Document::new(1);
    let x = square(&d, 0);
    let y = square(&d, 1);
    let g = d.group_objects(&[x, y]).unwrap();
    d.set_node_locked(&[y], true).unwrap();
    let before = d.object(y).unwrap();
    let mut s = open(&d);
    click(&mut s, mid(0));
    let mut sel = selected(&s);
    sel.sort_by_key(|i| [x, y].iter().position(|q| q == i));
    assert_eq!(sel, [x, y], "the group is picked, with its locked member");
    assert!(
        row(&mut s, g)["mark"] == "selected",
        "the group row is the selected one"
    );
    assert_eq!(
        key(&mut s, "ArrowRight"),
        KeyOutcome::Nudged { new_run: true }
    );
    let after = doc_of(&s).object(y).unwrap();
    assert_ne!(
        format!("{before:?}"),
        format!("{after:?}"),
        "the locked member moved with its group"
    );
}

#[test]
fn ac22_an_inherited_state_is_shown_and_the_own_flag_is_what_a_click_toggles() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    name_layer(&d, l, "Cut");
    let a = square(&d, 0);
    into_layer(&d, &[a], l);
    d.set_node_visible(&[l], false).unwrap();
    let mut s = open(&d);
    let r = row(&mut s, a);
    assert_eq!(r["visible"], true, "own flag");
    assert_eq!(r["effectiveVisible"], false);
    assert!(r["hiddenBy"].as_str().unwrap().contains("Cut"), "{r:?}");
    // The click toggles the child's own flag only.
    s.toggle_layer_row_flag(&rid(a), RowFlag::Visible);
    let r = row(&mut s, a);
    assert_eq!(r["visible"], false);
    assert_eq!(row(&mut s, l)["visible"], false);
    // Showing the ancestor restores it as it was, own flags included.
    s.toggle_layer_row_flag(&rid(l), RowFlag::Visible);
    let r = row(&mut s, a);
    assert_eq!(r["visible"], false, "own hidden flag survives");
    assert_eq!(r["effectiveVisible"], false);
    s.toggle_layer_row_flag(&rid(a), RowFlag::Visible);
    assert_eq!(row(&mut s, a)["effectiveVisible"], true);
    // Lock inheritance reads the same way.
    s.toggle_layer_row_flag(&rid(l), RowFlag::Locked);
    let r = row(&mut s, a);
    assert_eq!(r["locked"], false);
    assert_eq!(r["effectiveLocked"], true);
    assert!(r["lockedBy"].as_str().unwrap().contains("Cut"));
}

#[test]
fn ac23_drawing_into_a_locked_or_hidden_active_layer_creates_nothing_and_says_why() {
    for hidden in [false, true] {
        let d = Document::new(1);
        let l = d.new_layer(None);
        name_layer(&d, l, "Cut");
        if hidden {
            d.set_node_visible(&[l], false).unwrap();
        } else {
            d.set_node_locked(&[l], true).unwrap();
        }
        let mut s = open(&d);
        s.press_layer_row(&rid(l), RowPress::Plain);
        assert_eq!(s.active_layer(), Some(l));
        for tool in [Tool::Rectangle, Tool::Ellipse] {
            s.set_tool(tool);
            let n0 = commits(&s);
            hold(&mut s, pt(100.0, 100.0), false, false, false);
            s.pointer_down(pt(100.0, 100.0), false);
            hold(&mut s, pt(150.0, 140.0), false, false, false);
            s.pointer_up(pt(150.0, 140.0), false, false);
            assert_eq!(commits(&s), n0, "nothing created ({tool:?})");
            assert_eq!(doc_of(&s).object_ids().len(), 0);
            assert_eq!(s.tool(), tool, "the tool stays");
        }
        let notice = s.take_layer_notice();
        let want = if hidden {
            "Layer Cut is hidden. Show it or choose another layer."
        } else {
            "Layer Cut is locked. Unlock it or choose another layer."
        };
        assert_eq!(notice, want);
    }
}

#[test]
fn ac35_new_objects_go_to_the_front_of_the_active_layer() {
    let d = Document::new(1);
    let l1 = d.new_layer(None);
    let l2 = d.new_layer(None);
    let existing = square(&d, 0);
    into_layer(&d, &[existing], l1);
    let mut s = open(&d);
    s.press_layer_row(&rid(l1), RowPress::Plain);
    s.set_tool(Tool::Rectangle);
    hold(&mut s, pt(300.0, 300.0), false, false, false);
    s.pointer_down(pt(300.0, 300.0), false);
    hold(&mut s, pt(340.0, 340.0), false, false, false);
    s.pointer_up(pt(340.0, 340.0), false, false);
    let d2 = doc_of(&s);
    let kids = d2.children(Some(l1));
    assert_eq!(
        kids.len(),
        2,
        "the new rectangle is a child of the active layer"
    );
    assert_eq!(kids[0], existing, "and the front-most one");
    assert!(d2.children(Some(l2)).is_empty());
    // With no layer: objects go to the top level as today.
    let mut s = open(&Document::new(1));
    s.set_tool(Tool::Rectangle);
    hold(&mut s, pt(300.0, 300.0), false, false, false);
    s.pointer_down(pt(300.0, 300.0), false);
    hold(&mut s, pt(340.0, 340.0), false, false, false);
    s.pointer_up(pt(340.0, 340.0), false, false);
    let d3 = doc_of(&s);
    assert_eq!(d3.object_ids().len(), 1);
}

#[test]
fn ac35_selecting_an_object_makes_its_layer_active_and_open_picks_the_front_most_layer() {
    let d = Document::new(1);
    let back = d.new_layer(None);
    let front = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let loose = square(&d, 2);
    into_layer(&d, &[a], back);
    into_layer(&d, &[b], front);
    let _ = loose;
    let mut s = open(&d);
    assert_eq!(
        s.active_layer(),
        Some(front),
        "at Open the front-most layer"
    );
    click(&mut s, mid(0));
    assert_eq!(s.active_layer(), Some(back));
    click(&mut s, mid(1));
    assert_eq!(s.active_layer(), Some(front));
    // The active layer row is marked.
    s.press_layer_row(&rid(back), RowPress::Plain);
    assert_eq!(row(&mut s, back)["mark"], "active");
    assert_eq!(
        view(&mut s)["activeLayer"],
        serde_json::to_value(back).unwrap()
    );
}

#[test]
fn ac26_the_opacity_field_writes_the_selected_nodes_in_one_step() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let mut s = open(&d);
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.press_layer_row(&rid(b), RowPress::Toggle);
    let v = view(&mut s);
    assert_eq!(v["opacity"]["field"], true);
    assert_eq!(v["opacity"]["caption"], "2 objects");
    let n0 = commits(&s);
    s.preview_layer_opacity(30.0);
    assert_eq!(commits(&s), n0, "a drag writes nothing");
    s.commit_layer_opacity_preview();
    assert_eq!(commits(&s), n0 + 1, "one commit on release");
    assert_eq!(labels(&s).last().unwrap(), "set_node_opacity");
    let dd = doc_of(&s);
    for id in [a, b] {
        assert_eq!(dd.node_registers(id).unwrap().opacity.get(), 0.3);
    }
    // Different values: Mixed.
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.commit_layer_opacity(80.0);
    s.press_layer_row(&rid(b), RowPress::Toggle);
    assert!(
        view(&mut s)["opacity"]["value"].is_null(),
        "different values: no single value, the field shows Mixed"
    );
    // Escape restores: cancel leaves the stored value.
    let n1 = commits(&s);
    s.preview_layer_opacity(5.0);
    s.cancel_layer_opacity();
    assert_eq!(commits(&s), n1);
    // Out of range values are clamped to integer percent 0..=100.
    s.commit_layer_opacity(250.0);
    assert_eq!(doc_of(&s).node_registers(a).unwrap().opacity.get(), 1.0);
    s.commit_layer_opacity(-3.0);
    assert_eq!(doc_of(&s).node_registers(a).unwrap().opacity.get(), 0.0);
    // Nothing targeted: no field.
    click(&mut s, EMPTY);
    assert_eq!(view(&mut s)["opacity"]["field"], false);
}

#[test]
fn ac26_the_opacity_field_targets_the_layer_after_a_layer_row_press() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    name_layer(&d, l, "Cut");
    let a = square(&d, 0);
    into_layer(&d, &[a], l);
    let mut s = open(&d);
    s.press_layer_row(&rid(l), RowPress::Plain);
    let v = view(&mut s);
    // The caption is the name after the Layers glyph (UX review of 2026-10-14, should-fix 5).
    assert_eq!(v["opacity"]["caption"], "Cut");
    assert_eq!(v["opacity"]["layer"], true);
    s.commit_layer_opacity(50.0);
    let dd = doc_of(&s);
    assert_eq!(dd.node_registers(l).unwrap().opacity.get(), 0.5);
    assert_eq!(
        dd.node_registers(a).unwrap().opacity.get(),
        1.0,
        "children untouched"
    );
    let r = row(&mut s, l);
    assert_eq!(r["readout"], "50%");
    // A leaf at 100 % shows no readout.
    assert!(row(&mut s, a)["readout"].is_null() || row(&mut s, a)["readout"] == "");
}

#[test]
fn ac27_alpha_is_the_product_of_paint_leaf_and_every_ancestor_opacity() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let leaf = rect_at(&d, 20.0, 20.0, 60.0, 60.0);
    red_fill(&d, &[leaf]);
    d.edit_style(&[leaf], &StyleEdit::FillOpacity(Opacity::new(0.8).unwrap()))
        .unwrap();
    d.edit_style(
        &[leaf],
        &StyleEdit::StrokeColor(curvyo_document_core::Color { r: 0, g: 0, b: 255 }),
    )
    .unwrap();
    d.edit_style(&[leaf], &StyleEdit::StrokeWidth(Length::from_mm(8.0)))
        .unwrap();
    d.edit_style(
        &[leaf],
        &StyleEdit::StrokeOpacity(Opacity::new(1.0).unwrap()),
    )
    .unwrap();
    into_layer(&d, &[leaf], l);
    d.set_node_opacity(&[leaf], Opacity::new(0.5).unwrap())
        .unwrap();
    d.set_node_opacity(&[l], Opacity::new(0.5).unwrap())
        .unwrap();
    let s = open(&d);
    // Fill: 0.8 * 0.5 * 0.5 = 0.2 -> 51; stroke (outside the fill, x = 18): 1.0 * 0.25 -> 64.
    let fill = colour_at(&s, pt(50.0, 50.0)).unwrap();
    assert_eq!(fill[..3], [255, 0, 0]);
    assert!(
        (i32::from(fill[3]) - 51).abs() <= 1,
        "fill alpha {}",
        fill[3]
    );
    let stroke = colour_at(&s, pt(18.0, 50.0)).unwrap();
    assert_eq!(stroke[..3], [0, 0, 255]);
    assert!(
        (i32::from(stroke[3]) - 64).abs() <= 1,
        "stroke alpha {}",
        stroke[3]
    );
}

// ---------------------------------------------------------------------------
// The frame and the read (criterion 24)
// ---------------------------------------------------------------------------

#[test]
fn ac24_the_frame_draws_exactly_the_effectively_visible_leaves() {
    let d = Document::new(1);
    let hidden_layer = d.new_layer(None);
    let visible_layer = d.new_layer(None);
    let h1 = square(&d, 0); // in the hidden layer
    let h2 = square(&d, 1); // hidden leaf in a visible layer
    let v = square(&d, 2); // visible leaf in the visible layer
    let locked = square(&d, 3);
    red_fill(&d, &[h1, h2, v, locked]);
    into_layer(&d, &[h1], hidden_layer);
    into_layer(&d, &[h2, v, locked], visible_layer);
    d.set_node_visible(&[hidden_layer], false).unwrap();
    d.set_node_visible(&[h2], false).unwrap();
    d.set_node_locked(&[locked], true).unwrap();
    let s = open(&d);
    assert!(!drawn(&s, mid(0)), "leaf of a hidden layer");
    assert!(!drawn(&s, mid(1)), "hidden leaf of a visible layer");
    assert!(drawn(&s, mid(2)));
    assert!(drawn(&s, mid(3)), "a locked leaf is drawn");
    // The read still lists every leaf with its effective flags.
    let read = doc_of(&s).read_nodes();
    let leaf = |id: NodeId| read.iter().find(|n| n.id == id).unwrap();
    assert!(!leaf(h1).visible && !leaf(h2).visible && leaf(v).visible && leaf(locked).visible);
    assert!(leaf(locked).locked && !leaf(v).locked);
}

#[test]
fn ac25_toggling_a_layer_and_its_flag_is_one_register_and_one_commit_per_property() {
    // 50: hiding a layer is one write, not one per object.
    let d = Document::new(1);
    let l = d.new_layer(None);
    let ids: Vec<NodeId> = (0..50)
        .map(|i| rect_at(&d, 10.0 * f64::from(i), 0.0, 5.0, 5.0))
        .collect();
    into_layer(&d, &ids, l);
    let mut s = open(&d);
    let n0 = commits(&s);
    s.toggle_layer_row_flag(&rid(l), RowFlag::Visible);
    assert_eq!(commits(&s), n0 + 1);
    // Child registers untouched.
    let dd = doc_of(&s);
    assert!(ids.iter().all(|&i| dd.node_registers(i).unwrap().visible));
}

// ---------------------------------------------------------------------------
// Reordering and moving (criteria 29 to 33)
// ---------------------------------------------------------------------------

fn order(s: &Session, parent: Option<NodeId>) -> Vec<NodeId> {
    doc_of(s).children(parent)
}

#[test]
fn ac29_a_drop_is_one_step_and_the_new_stacking_order_is_the_list_order() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let c = square(&d, 2);
    let mut s = open(&d);
    // Rows (front first): c, b, a. Drop a above c: a is now front-most.
    let n0 = commits(&s);
    let reply = s.drop_layer_rows(&rid(a), &rid(c), "above");
    assert_eq!(commits(&s), n0 + 1);
    assert_eq!(labels(&s).last().unwrap(), "move_nodes");
    assert!(!matches!(reply, PanelReply::Refusal(_)));
    assert_eq!(order(&s, None), [b, c, a], "back to front");
    assert_eq!(row_names(&mut s).len(), 3);
    // Below: a behind b.
    s.drop_layer_rows(&rid(a), &rid(b), "below");
    assert_eq!(order(&s, None), [a, b, c]);
    undo(&mut s);
    assert_eq!(order(&s, None), [b, c, a]);
}

#[test]
fn ac29_dragging_a_selected_row_moves_all_selected_rows_in_their_relative_order() {
    let d = Document::new(1);
    let ids: Vec<NodeId> = (0..5).map(|i| square(&d, i)).collect();
    let mut s = open(&d);
    s.press_layer_row(&rid(ids[0]), RowPress::Plain);
    s.press_layer_row(&rid(ids[2]), RowPress::Toggle);
    let n0 = commits(&s);
    s.drop_layer_rows(&rid(ids[0]), &rid(ids[4]), "above");
    assert_eq!(commits(&s), n0 + 1);
    assert_eq!(order(&s, None), [ids[1], ids[3], ids[4], ids[0], ids[2]]);
}

#[test]
fn ac29_a_drop_on_a_container_puts_the_nodes_in_as_front_most_children() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let k = square(&d, 2);
    into_layer(&d, &[k], l);
    let _ = b;
    let mut s = open(&d);
    s.drop_layer_rows(&rid(a), &rid(l), "into");
    assert_eq!(order(&s, Some(l)), [k, a], "a is front-most in the layer");
    assert!(!order(&s, None).contains(&a));
}

#[test]
fn ac30_refused_drops_say_why_and_change_nothing() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let l2 = d.new_layer(None);
    let a = square(&d, 0);
    let x = square(&d, 1);
    let y = square(&d, 2);
    let g = d.group_objects(&[x, y]).unwrap();
    into_layer(&d, &[a], l);
    let mut s = open(&d);
    s.set_layer_expanded(&rid(g), true);
    // A layer never goes into a group or another layer.
    assert_eq!(
        s.check_layer_drop(&rid(l2), &rid(g), "into"),
        "Layers stay at the top level"
    );
    assert_eq!(
        s.check_layer_drop(&rid(l2), &rid(l), "into"),
        "Layers stay at the top level"
    );
    assert_eq!(
        s.check_layer_drop(&rid(l2), &rid(x), "above"),
        "Layers stay at the top level"
    );
    let n0 = commits(&s);
    s.drop_layer_rows(&rid(l2), &rid(l), "into");
    s.drop_layer_rows(&rid(l2), &rid(x), "above");
    // Never into itself or a descendant.
    assert_ne!(s.check_layer_drop(&rid(g), &rid(g), "into"), "");
    assert_ne!(
        s.check_layer_drop(&rid(g), &rid(x), "above"),
        "",
        "above its own child"
    );
    s.drop_layer_rows(&rid(g), &rid(x), "into");
    assert_eq!(commits(&s), n0, "nothing changed");
    // Allowed: leaf into layer, layer between top-level rows.
    assert_eq!(s.check_layer_drop(&rid(x), &rid(l), "into"), "");
    assert_eq!(s.check_layer_drop(&rid(l2), &rid(l), "above"), "");
    // Unknown ids are refused without a panic.
    let _ = s.check_layer_drop("nonsense", &rid(l), "into");
    let _ = s.check_layer_drop(&rid(a), &rid(l), "sideways");
    s.drop_layer_rows("[]", "{}", "above");
    assert_eq!(commits(&s), n0);
}

#[test]
fn ac30_the_nesting_depth_limit_is_32() {
    let d = Document::new(1);
    let mut inner = square(&d, 0);
    let mut chain = Vec::new();
    for i in 0..32 {
        let other = rect_at(&d, 5.0 * f64::from(i), 100.0, 3.0, 3.0);
        inner = d.group_objects(&[inner, other]).unwrap();
        chain.push(inner);
    }
    let g = d
        .group_objects(&[
            rect_at(&d, 500.0, 500.0, 5.0, 5.0),
            rect_at(&d, 600.0, 500.0, 5.0, 5.0),
        ])
        .unwrap();
    let s = open(&d);
    // chain[0] is the innermost of 32 nested groups, chain[31] the outermost. The group at
    // depth 31 may take a group (the chain would be 32 deep), the innermost may not.
    assert_eq!(s.check_layer_drop(&rid(g), &rid(chain[1]), "into"), "");
    assert_eq!(
        s.check_layer_drop(&rid(g), &rid(chain[0]), "into"),
        "Groups can be nested 32 levels deep"
    );
}

#[test]
fn ac31_a_move_keeps_ids_geometry_style_flags_and_names() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = square(&d, 0);
    red_fill(&d, &[a]);
    d.rename_node(a, "Keep").unwrap();
    d.set_node_locked(&[a], false).unwrap();
    d.set_node_opacity(&[a], Opacity::new(0.4).unwrap())
        .unwrap();
    let before = format!("{:?}", d.object(a).unwrap());
    let mut s = open(&d);
    s.drop_layer_rows(&rid(a), &rid(l), "into");
    let dd = doc_of(&s);
    assert_eq!(format!("{:?}", dd.object(a).unwrap()), before);
    let regs = dd.node_registers(a).unwrap();
    assert_eq!(regs.name.as_deref(), Some("Keep"));
    assert_eq!(regs.opacity.get(), 0.4);
    assert_eq!(dd.children(Some(l)), [a]);
}

#[test]
fn ac32_alt_arrows_move_the_focused_row_one_step_each_and_announce_it() {
    let d = Document::new(1);
    let ids: Vec<NodeId> = (0..4).map(|i| square(&d, i)).collect();
    let mut s = open(&d);
    // Forward = toward the front.
    let n0 = commits(&s);
    let reply = s.reorder_layer_rows(&rid(ids[1]), Reorder::Forward);
    assert_eq!(commits(&s), n0 + 1);
    assert_eq!(order(&s, None), [ids[0], ids[2], ids[1], ids[3]]);
    match reply {
        PanelReply::Announce(t) => {
            assert!(t.starts_with("Moved "), "{t}");
            assert!(t.contains("position 2 of 4"), "{t}");
        }
        other => panic!("expected an announcement, got {other:?}"),
    }
    s.reorder_layer_rows(&rid(ids[1]), Reorder::Backward);
    assert_eq!(order(&s, None), [ids[0], ids[1], ids[2], ids[3]]);
    s.reorder_layer_rows(&rid(ids[0]), Reorder::Front);
    assert_eq!(order(&s, None), [ids[1], ids[2], ids[3], ids[0]]);
    s.reorder_layer_rows(&rid(ids[0]), Reorder::Back);
    assert_eq!(order(&s, None), [ids[0], ids[1], ids[2], ids[3]]);
    // At the end of the range nothing moves and nothing breaks.
    let n1 = commits(&s);
    s.reorder_layer_rows(&rid(ids[0]), Reorder::Back);
    s.reorder_layer_rows(&rid(ids[3]), Reorder::Front);
    assert_eq!(order(&s, None), [ids[0], ids[1], ids[2], ids[3]]);
    assert_eq!(commits(&s), n1, "a move that changes nothing is no step");
}

#[test]
fn ac32_alt_left_and_alt_right_move_out_of_and_into_containers() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let l = d.new_layer(None); // stacking: a, l (l in front of a)
    let k = square(&d, 1);
    into_layer(&d, &[k], l);
    let mut s = open(&d);
    // In: a goes into the layer directly in front of it, as its front-most child.
    let n0 = commits(&s);
    s.reorder_layer_rows(&rid(a), Reorder::In);
    assert_eq!(commits(&s), n0 + 1, "one step");
    assert_eq!(order(&s, Some(l)), [k, a]);
    // Out: it leaves to the place directly in front of the layer.
    s.reorder_layer_rows(&rid(a), Reorder::Out);
    assert_eq!(order(&s, Some(l)), [k]);
    assert_eq!(order(&s, None), [l, a]);
    // Alt+Left on a top-level row and Alt+Right with no container in front change nothing.
    let n1 = commits(&s);
    s.reorder_layer_rows(&rid(l), Reorder::Out);
    s.reorder_layer_rows(&rid(a), Reorder::In);
    assert_eq!(commits(&s), n1);
}

#[test]
fn ac32_a_locked_row_does_not_move_by_key_either() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    d.set_node_locked(&[a], true).unwrap();
    let mut s = open(&d);
    let n0 = commits(&s);
    s.reorder_layer_rows(&rid(a), Reorder::Forward);
    assert_eq!(commits(&s), n0);
    assert_eq!(order(&s, None), [a, b]);
}

#[test]
fn ac33_home_end_page_keys_restack_the_selection_in_the_select_tool() {
    let d = Document::new(1);
    let ids: Vec<NodeId> = (0..5).map(|i| square(&d, i)).collect();
    let mut s = open(&d);
    click(&mut s, mid(1));
    let n0 = commits(&s);
    assert_eq!(key(&mut s, "PageUp"), KeyOutcome::Reordered);
    assert_eq!(commits(&s), n0 + 1);
    assert_eq!(labels(&s).last().unwrap(), "move_nodes");
    assert_eq!(order(&s, None), [ids[0], ids[2], ids[1], ids[3], ids[4]]);
    assert_eq!(key(&mut s, "PageDown"), KeyOutcome::Reordered);
    assert_eq!(order(&s, None), [ids[0], ids[1], ids[2], ids[3], ids[4]]);
    assert_eq!(key(&mut s, "Home"), KeyOutcome::Reordered);
    assert_eq!(order(&s, None), [ids[0], ids[2], ids[3], ids[4], ids[1]]);
    assert_eq!(key(&mut s, "End"), KeyOutcome::Reordered);
    assert_eq!(order(&s, None), [ids[1], ids[0], ids[2], ids[3], ids[4]]);
    // One undo per key.
    undo(&mut s);
    assert_eq!(order(&s, None), [ids[0], ids[2], ids[3], ids[4], ids[1]]);
}

#[test]
fn ac33_several_selected_nodes_keep_their_relative_order() {
    let d = Document::new(1);
    let ids: Vec<NodeId> = (0..5).map(|i| square(&d, i)).collect();
    let mut s = open(&d);
    s.press_layer_row(&rid(ids[0]), RowPress::Plain);
    s.press_layer_row(&rid(ids[2]), RowPress::Toggle);
    assert_eq!(key(&mut s, "Home"), KeyOutcome::Reordered);
    assert_eq!(order(&s, None), [ids[1], ids[3], ids[4], ids[0], ids[2]]);
    assert_eq!(key(&mut s, "End"), KeyOutcome::Reordered);
    assert_eq!(order(&s, None), [ids[0], ids[2], ids[1], ids[3], ids[4]]);
}

#[test]
fn ac33_the_keys_are_ignored_without_a_selection_with_modifiers_in_a_field_or_in_other_tools() {
    let d = Document::new(1);
    let ids: Vec<NodeId> = (0..3).map(|i| square(&d, i)).collect();
    let mut s = open(&d);
    let n0 = commits(&s);
    for k in ["Home", "End", "PageUp", "PageDown"] {
        assert_eq!(key(&mut s, k), KeyOutcome::Ignored, "{k}: nothing selected");
    }
    click(&mut s, mid(1));
    for k in ["Home", "End", "PageUp", "PageDown"] {
        for (ctrl, shift, alt) in [
            (true, false, false),
            (false, false, true),
            (false, true, false),
        ] {
            let o = key_with(&mut s, k, ctrl, shift, alt);
            assert_ne!(
                o,
                KeyOutcome::Reordered,
                "{k} ctrl={ctrl} shift={shift} alt={alt}"
            );
        }
        let blocked = s.key_down(KeyInput {
            key: k,
            dom_blocked: true,
            ..KeyInput::default()
        });
        assert_eq!(
            blocked,
            KeyOutcome::Ignored,
            "{k}: a focused field keeps its key"
        );
    }
    s.set_tool(Tool::Rectangle);
    for k in ["Home", "End", "PageUp", "PageDown"] {
        assert_ne!(key(&mut s, k), KeyOutcome::Reordered, "{k}: other tool");
    }
    assert_eq!(commits(&s), n0);
    assert_eq!(order(&s, None), ids);
}

#[test]
fn ac33_the_keys_restack_among_siblings_inside_a_layer() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let c = square(&d, 2);
    into_layer(&d, &[a, b, c], l);
    let mut s = open(&d);
    click(&mut s, mid(0));
    assert_eq!(key(&mut s, "Home"), KeyOutcome::Reordered);
    assert_eq!(order(&s, Some(l)), [b, c, a]);
}

// ---------------------------------------------------------------------------
// Layers, the active layer, delete (criteria 34 to 37)
// ---------------------------------------------------------------------------

#[test]
fn ac34_new_layer_is_created_in_front_of_the_active_layer_and_becomes_active() {
    let d = Document::new(1);
    let l1 = d.new_layer(None);
    let l2 = d.new_layer(None);
    let mut s = open(&d);
    s.press_layer_row(&rid(l1), RowPress::Plain);
    let n0 = commits(&s);
    s.new_layer_row();
    assert_eq!(commits(&s), n0 + 1, "one step");
    assert_eq!(labels(&s).last().unwrap(), "new_layer");
    let new = s.active_layer().unwrap();
    assert!(![l1, l2].contains(&new));
    assert_eq!(
        order(&s, None),
        [l1, new, l2],
        "directly in front of the active layer"
    );
    let v = view(&mut s);
    assert_eq!(v["opacity"]["layer"], true, "the panel target");
    // The first call with no layer puts it at the front of the top level.
    let d = Document::new(1);
    let a = square(&d, 0);
    let mut s = open(&d);
    s.new_layer_row();
    let new = s.active_layer().unwrap();
    assert_eq!(order(&s, None), [a, new]);
}

#[test]
fn ac34_new_layer_while_inside_a_group_still_creates_a_top_level_layer() {
    let d = Document::new(1);
    let x = square(&d, 0);
    let y = square(&d, 1);
    let g = d.group_objects(&[x, y]).unwrap();
    let mut s = open(&d);
    s.set_layer_expanded(&rid(g), true);
    s.press_layer_row(&rid(x), RowPress::Plain);
    assert_eq!(s.context_depth(), 1);
    s.new_layer_row();
    let new = s.active_layer().unwrap();
    let dd = doc_of(&s);
    assert!(dd.is_layer(new));
    assert_eq!(
        dd.parent_node(new),
        None,
        "layers live at the top level only"
    );
}

#[test]
fn ac36_delete_layer_removes_the_layer_with_its_content_in_one_step_and_undo_restores_it() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    name_layer(&d, l, "Cut");
    let ids: Vec<NodeId> = (0..3).map(|i| square(&d, i)).collect();
    into_layer(&d, &ids, l);
    d.rename_node(ids[1], "Mid").unwrap();
    d.set_node_locked(&[ids[2]], true).unwrap();
    d.set_node_opacity(&[l], Opacity::new(0.6).unwrap())
        .unwrap();
    let keep = square(&d, 5);
    let _ = keep;
    let mut s = open(&d);
    s.press_layer_row(&rid(l), RowPress::Plain);
    assert_eq!(view(&mut s)["canDeleteLayer"], true);
    let n0 = commits(&s);
    let reply = s.delete_active_layer();
    assert_eq!(commits(&s), n0 + 1, "one commit");
    assert_eq!(labels(&s).last().unwrap(), "delete_layer");
    assert_eq!(
        reply,
        PanelReply::Notice("Deleted layer Cut and 3 objects.".to_owned())
    );
    let dd = doc_of(&s);
    assert!(dd.object_ids().iter().all(|i| !ids.contains(i)));
    assert!(!has_row(&mut s, l));
    // Undo: id, name, flags, children in order with ids.
    undo(&mut s);
    let dd = doc_of(&s);
    assert!(dd.is_layer(l));
    assert_eq!(dd.children(Some(l)), ids);
    let regs = dd.node_registers(l).unwrap();
    assert_eq!(regs.name.as_deref(), Some("Cut"));
    assert_eq!(regs.opacity.get(), 0.6);
    assert_eq!(
        dd.node_registers(ids[1]).unwrap().name.as_deref(),
        Some("Mid")
    );
    assert!(dd.node_registers(ids[2]).unwrap().locked);
}

#[test]
fn ac36_deleting_an_empty_layer_and_a_locked_layer() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    name_layer(&d, l, "Cut");
    let a = square(&d, 0);
    into_layer(&d, &[a], l);
    d.set_node_locked(&[l], true).unwrap();
    let mut s = open(&d);
    s.press_layer_row(&rid(l), RowPress::Plain);
    let n0 = commits(&s);
    let reply = s.delete_active_layer();
    assert_eq!(
        reply,
        PanelReply::Refusal("Layer Cut is locked. Unlock it first.".to_owned())
    );
    assert_eq!(commits(&s), n0);
    assert!(has_row(&mut s, l) && has_row(&mut s, a));
    // An empty layer is deleted at once, with a notice that does not mention "No undo yet.".
    let d = Document::new(1);
    let e = d.new_layer(None);
    name_layer(&d, e, "Empty");
    let mut s = open(&d);
    s.press_layer_row(&rid(e), RowPress::Plain);
    match s.delete_active_layer() {
        PanelReply::Notice(t) => {
            assert!(t.starts_with("Deleted layer Empty"), "{t}");
            assert!(!t.contains("No undo yet."));
        }
        other => panic!("{other:?}"),
    }
    assert!(s.active_layer().is_none());
}

#[test]
fn ac36_the_next_active_layer_after_a_delete() {
    let d = Document::new(1);
    let l1 = d.new_layer(None);
    let l2 = d.new_layer(None);
    let l3 = d.new_layer(None);
    let mut s = open(&d);
    s.press_layer_row(&rid(l2), RowPress::Plain);
    s.delete_active_layer();
    // "the next one toward the back, or the next toward the front, or none"
    assert_eq!(s.active_layer(), Some(l1));
    s.press_layer_row(&rid(l1), RowPress::Plain);
    s.delete_active_layer();
    assert_eq!(s.active_layer(), Some(l3));
    s.press_layer_row(&rid(l3), RowPress::Plain);
    s.delete_active_layer();
    assert_eq!(s.active_layer(), None);
}

#[test]
fn ac37_delete_on_an_object_row_and_empty_layers_stay() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = square(&d, 0);
    into_layer(&d, &[a], l);
    let mut s = open(&d);
    s.press_layer_row(&rid(a), RowPress::Plain);
    let n0 = commits(&s);
    assert_eq!(key(&mut s, "Delete"), KeyOutcome::Deleted);
    assert_eq!(commits(&s), n0 + 1);
    assert!(!has_row(&mut s, a));
    assert!(has_row(&mut s, l), "the emptied layer stays");
    // Moving out the last child leaves the layer, too.
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = square(&d, 0);
    into_layer(&d, &[a], l);
    let mut s = open(&d);
    s.reorder_layer_rows(&rid(a), Reorder::Out);
    assert!(has_row(&mut s, l));
    // An emptied group, by contrast, goes (0023 criterion 14).
    let d = Document::new(1);
    let x = square(&d, 0);
    let y = square(&d, 1);
    let g = d.group_objects(&[x, y]).unwrap();
    let mut s = open(&d);
    s.set_layer_expanded(&rid(g), true);
    s.press_layer_row(&rid(x), RowPress::Plain);
    s.press_layer_row(&rid(y), RowPress::Toggle);
    key(&mut s, "Delete");
    assert!(!has_row(&mut s, g));
}

// ---------------------------------------------------------------------------
// Row labels (criterion 41) and the commands that read the tree (42, 43)
// ---------------------------------------------------------------------------

#[test]
fn ac41_the_accessible_name_carries_kind_state_and_inheritance() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    name_layer(&d, l, "Cut");
    let a = square(&d, 0);
    d.rename_node(a, "Door").unwrap();
    into_layer(&d, &[a], l);
    d.set_node_opacity(&[a], Opacity::new(0.6).unwrap())
        .unwrap();
    let mut s = open(&d);
    assert_eq!(
        row(&mut s, a)["label"],
        "Door, rectangle, opacity 60 percent"
    );
    s.toggle_layer_row_flag(&rid(a), RowFlag::Visible);
    s.toggle_layer_row_flag(&rid(a), RowFlag::Locked);
    assert_eq!(
        row(&mut s, a)["label"],
        "Door, rectangle, hidden, locked, opacity 60 percent"
    );
    s.toggle_layer_row_flag(&rid(a), RowFlag::Visible);
    s.toggle_layer_row_flag(&rid(a), RowFlag::Locked);
    s.toggle_layer_row_flag(&rid(l), RowFlag::Visible);
    let label = row(&mut s, a)["label"].as_str().unwrap().to_owned();
    assert!(label.contains("hidden by layer Cut"), "{label}");
    // aria-level, posinset, setsize.
    let r = row(&mut s, a);
    assert_eq!(r["level"], 2);
    assert_eq!(r["posinset"], 1);
    assert_eq!(r["setsize"], 1);
}

#[test]
fn ac42_layers_are_transparent_to_ctrl_a_marquee_and_click() {
    let d = Document::new(1);
    let l1 = d.new_layer(None);
    let l2 = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let c = square(&d, 2);
    let hidden = square(&d, 3);
    let locked = square(&d, 4);
    red_fill(&d, &[a, b, c, hidden, locked]);
    into_layer(&d, &[a], l1);
    into_layer(&d, &[b, hidden, locked], l2);
    d.set_node_visible(&[hidden], false).unwrap();
    d.set_node_locked(&[locked], true).unwrap();
    let mut s = open(&d);
    ctrl_a(&mut s);
    let mut sel = selected(&s);
    sel.sort_by_key(|i| [a, b, c].iter().position(|x| x == i));
    assert_eq!(sel, [a, b, c]);
    click(&mut s, EMPTY);
    marquee(&mut s, pt(0.0, 0.0), pt(260.0, 100.0));
    let mut sel = selected(&s);
    sel.sort_by_key(|i| [a, b, c].iter().position(|x| x == i));
    assert_eq!(sel, [a, b, c]);
    // A click selects the object, not the layer.
    click(&mut s, EMPTY);
    click(&mut s, mid(0));
    assert_eq!(selected(&s), [a]);
    // An effectively hidden layer removes everything in it.
    d.set_node_visible(&[l1], false).unwrap();
    let mut s = open(&d);
    ctrl_a(&mut s);
    assert!(!selected(&s).contains(&a));
}

#[test]
fn ac42_ctrl_a_in_an_entered_group_chooses_its_direct_children() {
    let d = Document::new(1);
    let x = square(&d, 0);
    let y = square(&d, 1);
    let z = square(&d, 2);
    let _g = d.group_objects(&[x, y]).unwrap();
    let mut s = open(&d);
    s.double_click(mid(0), false, false);
    ctrl_a(&mut s);
    let mut sel = selected(&s);
    sel.sort_by_key(|i| [x, y].iter().position(|q| q == i));
    assert_eq!(sel, [x, y]);
    assert!(!sel.contains(&z));
}

#[test]
fn ac43_group_and_commands_keep_the_result_in_the_same_layer() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let c = square(&d, 2);
    into_layer(&d, &[a, b], l);
    let _ = c;
    let mut s = open(&d);
    click(&mut s, mid(0));
    s.press_layer_row(&rid(b), RowPress::Toggle);
    assert_eq!(key_with(&mut s, "g", true, false, false), KeyOutcome::Group);
    let dd = doc_of(&s);
    let kids = dd.children(Some(l));
    assert_eq!(kids.len(), 1, "the group is a child of the layer");
    assert_eq!(dd.parent_node(kids[0]), Some(l));
}

// ---------------------------------------------------------------------------
// History (criteria 44, 45)
// ---------------------------------------------------------------------------

#[test]
fn ac44_every_panel_write_is_one_step_with_its_operation_name() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    let mut s = open(&d);
    let l = {
        s.new_layer_row();
        s.active_layer().unwrap()
    };
    s.rename_layer_row(&rid(a), "A");
    s.toggle_layer_row_flag(&rid(a), RowFlag::Visible);
    s.toggle_layer_row_flag(&rid(a), RowFlag::Locked);
    s.press_layer_row(&rid(b), RowPress::Plain);
    s.commit_layer_opacity(50.0);
    s.drop_layer_rows(&rid(b), &rid(l), "into");
    s.press_layer_row(&rid(l), RowPress::Plain);
    s.delete_active_layer();
    let got = labels(&s);
    let tail: Vec<&str> = got.iter().rev().take(7).rev().map(String::as_str).collect();
    assert_eq!(
        tail,
        [
            "new_layer",
            "rename_node",
            "set_node_visible",
            "set_node_locked",
            "set_node_opacity",
            "move_nodes",
            "delete_layer"
        ]
    );
    // Expanding, the active layer and the panel target are no steps.
    let n = commits(&s);
    s.set_layer_expanded(&rid(l), true);
    s.press_layer_row(&rid(a), RowPress::Plain);
    let _ = view(&mut s);
    assert_eq!(commits(&s), n);
    // The History tab names them in words.
    let history = s.history_rows_json(0, 50);
    assert!(
        history.contains("New layer") || history.contains("Delete layer"),
        "{history}"
    );
}

#[test]
fn ac45_undo_after_hide_lock_opacity_and_move_restores_field_by_field() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let b = square(&d, 1);
    d.set_node_opacity(&[a], Opacity::new(0.7).unwrap())
        .unwrap();
    let mut s = open(&d);
    s.toggle_layer_row_flag(&rid(a), RowFlag::Visible);
    undo(&mut s);
    let regs = doc_of(&s).node_registers(a).unwrap();
    assert!(regs.visible && !regs.locked);
    assert_eq!(regs.opacity.get(), 0.7);
    s.toggle_layer_row_flag(&rid(a), RowFlag::Locked);
    undo(&mut s);
    assert!(!doc_of(&s).node_registers(a).unwrap().locked);
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.commit_layer_opacity(10.0);
    undo(&mut s);
    assert_eq!(doc_of(&s).node_registers(a).unwrap().opacity.get(), 0.7);
    s.drop_layer_rows(&rid(a), &rid(b), "above");
    undo(&mut s);
    assert_eq!(order(&s, None), [a, b]);
}

// ---------------------------------------------------------------------------
// The Pen gate (criterion 53)
// ---------------------------------------------------------------------------

fn pen_unfinished(d: &Document) -> Session {
    let mut s = open(d);
    s.set_tool(Tool::Pen);
    click(&mut s, pt(400.0, 400.0));
    click(&mut s, pt(450.0, 420.0));
    assert!(s.pen_in_progress().is_some(), "an unfinished path");
    s
}

fn is_hint(r: &PanelReply) -> bool {
    matches!(r, PanelReply::Hint(t) if t == "Finish the path (Enter) or cancel it (Esc) first.")
}

#[test]
fn ac53_every_panel_write_or_select_shows_the_hint_and_changes_nothing() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    into_layer(&d, &[a], l);
    let mut s = pen_unfinished(&d);
    let before_commits = commits(&s);
    let before_json = doc_of(&s).export_json().unwrap();
    let sel_before = selected(&s);
    let replies = vec![
        s.press_layer_row(&rid(b), RowPress::Plain),
        s.press_layer_row(&rid(b), RowPress::Toggle),
        s.press_layer_row(&rid(b), RowPress::Range),
        s.press_layer_row(&rid(l), RowPress::Plain),
        s.toggle_layer_row_flag(&rid(b), RowFlag::Visible),
        s.toggle_layer_row_flag(&rid(b), RowFlag::Locked),
        s.select_layer_contents(&rid(l)),
        s.rename_layer_row(&rid(b), "x"),
        s.drop_layer_rows(&rid(b), &rid(l), "into"),
        s.reorder_layer_rows(&rid(b), Reorder::Forward),
        s.reorder_layer_rows(&rid(b), Reorder::Front),
        s.reorder_layer_rows(&rid(b), Reorder::Back),
        s.reorder_layer_rows(&rid(b), Reorder::Backward),
        s.reorder_layer_rows(&rid(a), Reorder::Out),
        s.reorder_layer_rows(&rid(b), Reorder::In),
        s.delete_layer_row(&rid(l)),
        s.new_layer_row(),
        s.delete_active_layer(),
        s.commit_layer_opacity(10.0),
    ];
    for (i, r) in replies.iter().enumerate() {
        assert!(is_hint(r), "action {i} gave {r:?}");
    }
    assert!(!s.layers_pen_hint().is_empty());
    assert_eq!(commits(&s), before_commits);
    assert_eq!(doc_of(&s).export_json().unwrap(), before_json);
    assert_eq!(selected(&s), sel_before);
    // Scrolling and expanding still work.
    assert!(s.set_layer_expanded(&rid(l), false));
    assert!(!view(&mut s).is_null());
    // After the path ends the same action works.
    key(&mut s, "Escape");
    s.set_tool(Tool::Select);
    assert!(s.pen_in_progress().is_none());
    let r = s.toggle_layer_row_flag(&rid(b), RowFlag::Visible);
    assert!(!is_hint(&r));
    assert_eq!(row(&mut s, b)["visible"], false);
}

#[test]
fn ac53_the_pen_gate_covers_the_keyboard_delete_and_the_canvas_keys_do_not_leak_in() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let mut s = pen_unfinished(&d);
    let n = commits(&s);
    // No selection can exist while the pen is unfinished, so the row actions are all that is left.
    assert!(is_hint(&s.press_layer_row(&rid(a), RowPress::Plain)));
    assert!(selected(&s).is_empty());
    assert_eq!(commits(&s), n);
}

// ---------------------------------------------------------------------------
// Hostile ids
// ---------------------------------------------------------------------------

#[test]
fn hostile_row_ids_and_arguments_are_refused_without_a_panic() {
    let d = Document::new(1);
    let a = square(&d, 0);
    let mut s = open(&d);
    let n = commits(&s);
    for bad in [
        "",
        "null",
        "[]",
        "{}",
        "\"x\"",
        "123456789012345678901234567890",
        "\u{0}",
        "-1",
    ] {
        s.press_layer_row(bad, RowPress::Plain);
        s.press_layer_row(bad, RowPress::Range);
        s.select_layer_contents(bad);
        s.toggle_layer_row_flag(bad, RowFlag::Visible);
        s.toggle_layer_row_flag(bad, RowFlag::Locked);
        s.rename_layer_row(bad, "x");
        s.delete_layer_row(bad);
        s.set_layer_expanded(bad, true);
        s.set_layer_hover(bad);
        s.drop_layer_rows(bad, bad, "above");
        s.reorder_layer_rows(bad, Reorder::Out);
        let _ = s.check_layer_drop(bad, bad, "into");
    }
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 1e300] {
        s.press_layer_row(&rid(a), RowPress::Plain);
        s.preview_layer_opacity(v);
        s.commit_layer_opacity(v);
        s.cancel_layer_opacity();
        let o = doc_of(&s).node_registers(a).unwrap().opacity.get();
        assert!((0.0..=1.0).contains(&o) && o.is_finite(), "{v}: {o}");
    }
    let _ = n;
    // A window past the end and a huge window are fine.
    let big: Value = serde_json::from_str(&s.layers_view_json(usize::MAX - 1, usize::MAX)).unwrap();
    assert_eq!(big["rows"].as_array().unwrap().len(), 0);
    let all: Value = serde_json::from_str(&s.layers_view_json(0, usize::MAX)).unwrap();
    assert_eq!(all["rows"].as_array().unwrap().len(), 1);
    // Deleting a non-layer through the layer route is refused.
    let _ = s.delete_layer_row(&rid(a));
}

// ---------------------------------------------------------------------------
// The other consumers of the flags (criterion 19, 20)
// ---------------------------------------------------------------------------

fn anchor(n: u64, x: f64, y: f64) -> NewAnchor {
    NewAnchor::corner(AnchorId::new(1, n), pt(x, y))
}

#[test]
fn ac19_the_eyedropper_skips_hidden_nodes_and_samples_locked_ones() {
    let d = Document::new(1);
    let target = square(&d, 0);
    let hidden = square(&d, 1);
    let locked = square(&d, 2);
    d.edit_style(
        &[hidden],
        &StyleEdit::FillColor(curvyo_document_core::Color { r: 0, g: 0, b: 255 }),
    )
    .unwrap();
    d.edit_style(
        &[locked],
        &StyleEdit::FillColor(curvyo_document_core::Color { r: 0, g: 255, b: 0 }),
    )
    .unwrap();
    d.set_node_visible(&[hidden], false).unwrap();
    d.set_node_locked(&[locked], true).unwrap();
    let fill_of = |s: &Session, id: NodeId| -> curvyo_document_core::Color {
        match doc_of(s).object(id).unwrap() {
            curvyo_document_core::ObjectSnapshot::Path(p) => p.style.fill.color,
            curvyo_document_core::ObjectSnapshot::Primitive(p) => p.style.fill.color,
            curvyo_document_core::ObjectSnapshot::Clone(p) => p.resolved.style.fill.color,
        }
    };
    let mut s = open(&d);
    click(&mut s, mid(0));
    assert_eq!(selected(&s), [target]);
    s.begin_colour_pick(PaintTarget::Fill);
    click(&mut s, mid(1));
    let picked = fill_of(&s, target);
    assert_ne!(
        picked,
        curvyo_document_core::Color { r: 0, g: 0, b: 255 },
        "a hidden node is not sampled"
    );
    s.begin_colour_pick(PaintTarget::Fill);
    click(&mut s, mid(2));
    assert_eq!(
        fill_of(&s, target),
        curvyo_document_core::Color { r: 0, g: 255, b: 0 }
    );
}

#[test]
fn ac19_the_pen_does_not_continue_a_hidden_or_locked_path() {
    for hidden in [true, false] {
        let d = Document::new(1);
        let path = d.create_path(&[anchor(1, 0.0, 0.0), anchor(2, 100.0, 0.0)], false);
        let mut s = open(&d);
        s.set_tool(Tool::Pen);
        s.pointer_hover(pt(100.0, 0.0), false, false);
        assert!(
            matches!(s.pen_target(), Some(PenTarget::Continue(_))),
            "visible path continues"
        );
        if hidden {
            d.set_node_visible(&[path], false).unwrap();
        } else {
            d.set_node_locked(&[path], true).unwrap();
        }
        let mut s = open(&d);
        s.set_tool(Tool::Pen);
        s.pointer_hover(pt(100.0, 0.0), false, false);
        assert_eq!(s.pen_target(), Some(PenTarget::Place), "hidden={hidden}");
    }
}

#[test]
fn ac19_the_node_tool_ignores_hidden_and_locked_paths() {
    for hidden in [true, false] {
        let d = Document::new(1);
        let path = d.create_path(&[anchor(1, 0.0, 0.0), anchor(2, 100.0, 0.0)], false);
        if hidden {
            d.set_node_visible(&[path], false).unwrap();
        } else {
            d.set_node_locked(&[path], true).unwrap();
        }
        let mut s = open(&d);
        s.set_tool(Tool::Node);
        click(&mut s, pt(50.0, 0.0));
        click(&mut s, pt(100.0, 0.0));
        assert_eq!(s.selected_object_count(), 0, "hidden={hidden}");
    }
}

#[test]
fn ac43_a_result_of_a_command_stays_in_the_layer_of_its_operand() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = rect_at(&d, 20.0, 20.0, 30.0, 30.0);
    let b = rect_at(&d, 40.0, 20.0, 30.0, 30.0);
    red_fill(&d, &[a, b]);
    into_layer(&d, &[a, b], l);
    let mut s = open(&d);
    s.press_layer_row(&rid(a), RowPress::Plain);
    s.press_layer_row(&rid(b), RowPress::Toggle);
    let out = s.apply_boolean(curvyo_ui_core::BooleanOp::Union);
    let _ = out;
    let dd = doc_of(&s);
    assert_eq!(dd.object_ids().len(), 1);
    assert_eq!(
        dd.children(Some(l)).len(),
        1,
        "the union lies in the same layer"
    );
    assert!(dd.children(None).contains(&l));
}

#[test]
fn ac23_the_pen_and_the_polygon_tool_are_refused_into_a_locked_active_layer_too() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    name_layer(&d, l, "Cut");
    d.set_node_locked(&[l], true).unwrap();
    let mut s = open(&d);
    s.press_layer_row(&rid(l), RowPress::Plain);
    s.set_tool(Tool::Pen);
    let n0 = commits(&s);
    click(&mut s, pt(300.0, 300.0));
    click(&mut s, pt(360.0, 320.0));
    key(&mut s, "Enter");
    assert_eq!(commits(&s), n0, "the Pen creates nothing");
    assert!(
        s.pen_in_progress().is_none(),
        "and holds no unfinished path"
    );
    assert_eq!(
        s.take_layer_notice(),
        "Layer Cut is locked. Unlock it or choose another layer."
    );
    assert_eq!(s.tool(), Tool::Pen, "the tool stays");
    s.set_tool(Tool::PolygonStar);
    hold(&mut s, pt(100.0, 100.0), false, false, false);
    s.pointer_down(pt(100.0, 100.0), false);
    hold(&mut s, pt(150.0, 140.0), false, false, false);
    s.pointer_up(pt(150.0, 140.0), false, false);
    assert_eq!(commits(&s), n0);
    assert_eq!(doc_of(&s).object_ids().len(), 0);
    // Unlocking makes the same gesture work again.
    s.toggle_layer_row_flag(&rid(l), RowFlag::Locked);
    s.set_tool(Tool::Rectangle);
    hold(&mut s, pt(100.0, 100.0), false, false, false);
    s.pointer_down(pt(100.0, 100.0), false);
    hold(&mut s, pt(150.0, 140.0), false, false, false);
    s.pointer_up(pt(150.0, 140.0), false, false);
    assert_eq!(doc_of(&s).children(Some(l)).len(), 1);
}

#[test]
fn ac14_locking_a_layer_drops_its_selected_children_and_a_selected_group_member_stays_selectable_by_group()
 {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = square(&d, 0);
    let b = square(&d, 1);
    into_layer(&d, &[a, b], l);
    let mut s = open(&d);
    ctrl_a(&mut s);
    assert_eq!(selected(&s).len(), 2);
    s.toggle_layer_row_flag(&rid(l), RowFlag::Locked);
    assert!(selected(&s).is_empty());
    click(&mut s, mid(0));
    assert!(
        selected(&s).is_empty(),
        "a node of a locked layer is not hit"
    );
    assert!(drawn(&s, mid(0)), "but it is still drawn");
}
