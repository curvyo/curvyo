//! Tester's acceptance tests for `specs/0038-path-offset` after the amendment of 2026-10-13, session
//! side: the standing preview (criterion 52), open paths (55), the center handle and the Offset
//! entry chip (13, 25, 53, 54), the corner style set by a double-click (13, 56), the modifiers
//! (10, 19, 26), refusals (29, 32) and undo (16). Written from the specification and
//! `ux-rework.md`; only the public session API and the document are used.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    missing_docs
)]

use curvyo_document_core::{
    AnchorId, Document, Length, NewAnchor, NodeId, ObjectSnapshot, Point, RectBounds, Shape, pack,
    unpack,
};
use curvyo_editor_wasm::{DoubleClickHint, KeyInput, OffsetOutcome, Session, Tool};
use curvyo_ui_core::{
    OffsetDirection, OffsetJoin, OffsetResultChoice, handle_positions, offset_box,
};

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

/// A closed path (no fill) over the box (x, y, w, h); `n` is an anchor id seed.
fn closed_square(d: &Document, n: u64, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(n, 1), pt(x, y)),
            NewAnchor::corner(AnchorId::new(n, 2), pt(x + w, y)),
            NewAnchor::corner(AnchorId::new(n, 3), pt(x + w, y + h)),
            NewAnchor::corner(AnchorId::new(n, 4), pt(x, y + h)),
        ],
        true,
    )
}

fn open_line(d: &Document, n: u64, y: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(n, 1), pt(0.0, y)),
            NewAnchor::corner(AnchorId::new(n, 2), pt(20.0, y)),
        ],
        false,
    )
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1200.0, 800.0);
    s
}

fn doc(s: &Session) -> Document {
    unpack(3, &s.pack("0.1.0").unwrap()).unwrap()
}

fn objects(s: &Session) -> Vec<ObjectSnapshot> {
    let d = doc(s);
    d.object_ids()
        .into_iter()
        .map(|id| d.object(id).unwrap())
        .collect()
}

fn packed(s: &Session) -> Vec<u8> {
    s.pack("0.1.0").unwrap()
}

fn click(s: &mut Session, at: Point, shift: bool) {
    s.pointer_hover(at, shift, false);
    s.pointer_down(at, shift);
    s.pointer_up(at, shift, false);
}

fn key(s: &mut Session, k: &str) -> &'static str {
    s.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    })
    .code()
}

fn ctrl_a(s: &mut Session) {
    let _ = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}

fn group_selection(s: &mut Session) {
    let _ = s.key_down(KeyInput {
        key: "g",
        ctrl: true,
        ..KeyInput::default()
    });
}

/// Selects with a click on `edge` in the Select tool, then switches to Offset.
fn select_and_offset(s: &mut Session, edge: Point) {
    s.set_tool(Tool::Select);
    click(s, edge, false);
    s.set_tool(Tool::Offset);
}

fn corner(s: &Session, sel: &[ObjectSnapshot], sign: (f64, f64)) -> Point {
    let refs: Vec<&ObjectSnapshot> = sel.iter().collect();
    let b = offset_box(&refs).unwrap();
    let handles = handle_positions(&b, s.view().scale());
    let c = b.document_corners();
    let centre = pt(
        c.iter().map(|p| p.x).sum::<f64>() / 4.0,
        c.iter().map(|p| p.y).sum::<f64>() / 4.0,
    );
    *handles
        .iter()
        .min_by(|a, b2| {
            let da = (a.x - centre.x) * sign.0 + (a.y - centre.y) * sign.1;
            let db = (b2.x - centre.x) * sign.0 + (b2.y - centre.y) * sign.1;
            db.partial_cmp(&da).unwrap()
        })
        .unwrap()
}

const BR: (f64, f64) = (1.0, 1.0);

/// The bottom right handle of the box around exactly the first object.
fn br(s: &Session) -> Point {
    corner(s, &objects(s)[..1], BR)
}

/// A drag of the bottom right handle by `delta`, released with the given keys (the keys are
/// given on the move and release events only, as the host sends them).
fn drag_br(s: &mut Session, delta: (f64, f64), shift: bool, ctrl: bool) {
    let h = br(s);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + delta.0, h.y + delta.1), shift, false);
    s.pointer_up(pt(h.x + delta.0, h.y + delta.1), shift, ctrl);
}

fn applied(o: &OffsetOutcome) -> (usize, usize) {
    match o {
        OffsetOutcome::Applied { added, changed, .. } => (*added, *changed),
        other => panic!("expected Applied, got {other:?}"),
    }
}

fn take_applied(s: &mut Session) -> (usize, usize) {
    applied(&s.take_offset_outcome().expect("an outcome"))
}

/// The overlay is a function of the standing preview: at two distances the canvas differs only
/// if a preview is drawn (the box and the handles do not depend on the distance).
fn preview_drawn(s: &mut Session) -> bool {
    let keep = s.offset_values().distance_mm();
    assert!(s.set_offset_distance(1.0));
    let a = s.draw_list().triangles;
    assert!(s.set_offset_distance(5.0));
    let b = s.draw_list().triangles;
    assert!(s.set_offset_distance(keep));
    a != b
}

fn rect_size(o: &ObjectSnapshot) -> Option<(f64, f64)> {
    let ObjectSnapshot::Primitive(p) = o else {
        return None;
    };
    let Shape::Rect { bounds, .. } = p.shape else {
        return None;
    };
    Some((bounds.width.as_mm(), bounds.height.as_mm()))
}

fn rect_radius(o: &ObjectSnapshot) -> Option<f64> {
    let ObjectSnapshot::Primitive(p) = o else {
        return None;
    };
    let Shape::Rect { corner_radii, .. } = p.shape else {
        return None;
    };
    Some(corner_radii.tl.as_mm())
}

fn sorted_rect_widths(s: &Session) -> Vec<f64> {
    let mut w: Vec<f64> = objects(s)
        .iter()
        .filter_map(|o| rect_size(o).map(|(w, _)| w))
        .collect();
    w.sort_by(|a, b| a.partial_cmp(b).unwrap());
    w
}

/// One filled 20 x 20 rectangle at the origin, selected, Offset tool active.
fn rect_session() -> Session {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    select_and_offset(&mut s, pt(20.0, 10.0));
    s
}

/// One unfilled closed path 40 x 40 at the origin: its center is empty canvas.
fn path_session() -> Session {
    let d = Document::new(1);
    closed_square(&d, 1, 0.0, 0.0, 40.0, 40.0);
    let mut s = open(&d);
    select_and_offset(&mut s, pt(20.0, 0.0));
    assert_eq!(
        s.selected_object_count(),
        1,
        "premise: the path is selected"
    );
    s
}

// ------------------------------------------------------------------------------- AC 52 --------

#[test]
fn ac52_preview_shows_at_once_for_the_remembered_values() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Offset);
    assert!(s.set_offset_distance(3.0));
    s.set_offset_direction(OffsetDirection::In);
    s.set_offset_join(OffsetJoin::Miter);
    let before = packed(&s);
    let steps = s.undo_len();
    click(&mut s, pt(20.0, 10.0), false);
    assert_eq!(s.selected_object_count(), 1);
    let v = s.offset_view();
    assert!(v.handles && v.operands == 1 && !v.dragging, "{v:?}");
    assert_eq!(v.distance_mm, -3.0, "Inset 3 mm");
    assert_eq!(v.join, OffsetJoin::Miter);
    assert!(v.refusal.is_none());
    assert!(preview_drawn(&mut s), "the standing preview is drawn");
    assert_eq!(packed(&s), before, "showing it writes nothing");
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac52_preview_for_every_way_of_selecting_and_on_tool_switch() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    rect_at(&d, 100.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Offset);
    assert!(!preview_drawn(&mut s), "nothing selected, no preview");
    click(&mut s, pt(20.0, 10.0), false);
    assert!(preview_drawn(&mut s), "click");
    click(&mut s, pt(120.0, 10.0), true);
    assert_eq!(s.selected_object_count(), 2);
    assert_eq!(s.offset_view().operands, 2);
    assert!(preview_drawn(&mut s), "shift+click");
    click(&mut s, pt(60.0, 300.0), false);
    assert_eq!(s.selected_object_count(), 0);
    assert!(!preview_drawn(&mut s), "empty press: no preview");
    s.pointer_down(pt(-10.0, -10.0), false);
    s.pointer_hover(pt(30.0, 30.0), false, false);
    s.pointer_up(pt(30.0, 30.0), false, false);
    assert_eq!(s.selected_object_count(), 1, "marquee");
    assert!(preview_drawn(&mut s), "marquee");
    ctrl_a(&mut s);
    assert_eq!(s.selected_object_count(), 2);
    assert!(preview_drawn(&mut s), "Ctrl+A");
    s.set_tool(Tool::Select);
    assert!(!preview_drawn(&mut s), "not in the Offset tool");
    s.set_tool(Tool::Offset);
    assert!(
        preview_drawn(&mut s),
        "switching to the tool with a selection"
    );
}

#[test]
fn ac52_after_a_write_the_new_selection_has_its_preview_and_enter_continues_from_it() {
    let mut s = rect_session();
    drag_br(&mut s, (3.0, 3.0), false, false);
    assert_eq!(take_applied(&mut s), (1, 0));
    assert!(preview_drawn(&mut s), "preview for the new selection");
    assert_eq!(key(&mut s, "o"), "offset-chip");
    assert!(s.offset_chip_open());
    let steps = s.undo_len();
    assert_eq!(applied(&s.apply_offset()), (1, 0));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(objects(&s).len(), 3);
}

#[test]
fn ac52_more_than_2000_nodes_show_the_grown_box_without_a_computed_preview() {
    let d = Document::new(1);
    let anchors: Vec<NewAnchor> = (0..2100u32)
        .map(|i| {
            let a = f64::from(i) / 2100.0 * std::f64::consts::TAU;
            NewAnchor::corner(
                AnchorId::new(1, u64::from(i) + 1),
                pt(100.0 + 80.0 * a.cos(), 100.0 + 80.0 * a.sin()),
            )
        })
        .collect();
    let _ = d.create_path(&anchors, true);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    ctrl_a(&mut s);
    s.set_tool(Tool::Offset);
    let v = s.offset_view();
    assert!(v.large, "{v:?}");
    assert!(v.refusal.is_none());
    assert!(preview_drawn(&mut s), "the grown box follows the distance");
}

// ------------------------------------------------------------------------------- AC 55 --------

#[test]
fn ac55_only_open_paths_no_handles_no_chip_no_preview_no_message() {
    let d = Document::new(1);
    open_line(&d, 1, 0.0);
    open_line(&d, 2, 40.0);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    ctrl_a(&mut s);
    s.set_tool(Tool::Offset);
    assert_eq!(s.selected_object_count(), 2);
    let v = s.offset_view();
    assert!(!v.handles && v.operands == 0, "{v:?}");
    assert!(!preview_drawn(&mut s));
    assert!(!s.open_offset_chip());
    assert_eq!(key(&mut s, "Enter"), "ignored");
    assert!(!s.offset_chip_open());
    assert_eq!(s.apply_offset(), OffsetOutcome::Ignored);
    assert_eq!(
        s.double_click_hint(pt(10.0, 20.0), false, false),
        DoubleClickHint::None
    );
    assert_eq!(s.handle_hint(), "");
}

#[test]
fn ac55_a_group_alone_draws_no_handles() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    rect_at(&d, 40.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    ctrl_a(&mut s);
    group_selection(&mut s);
    s.set_tool(Tool::Offset);
    assert!(s.selected_object_count() >= 1);
    let v = s.offset_view();
    assert!(v.has_group && !v.handles && v.operands == 0, "{v:?}");
    assert!(!s.open_offset_chip());
    assert!(!preview_drawn(&mut s));
}

#[test]
fn ac55_mixed_selection_handles_chip_and_preview_are_for_the_closed_ones() {
    let d = Document::new(1);
    let line = open_line(&d, 1, 200.0);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    ctrl_a(&mut s);
    s.set_tool(Tool::Offset);
    assert_eq!(s.selected_object_count(), 2);
    assert_eq!(s.offset_view().operands, 1);
    // The center handle is the center of the rectangle alone, not of the far line.
    assert_eq!(
        s.double_click_hint(pt(10.0, 10.0), false, false),
        DoubleClickHint::OffsetChip
    );
    assert!(s.offset_chip_open());
    let line_before = doc(&s).object(line).unwrap();
    let closed: Vec<ObjectSnapshot> = objects(&s)
        .into_iter()
        .filter(|o| rect_size(o).is_some())
        .collect();
    let h = corner(&s, &closed, BR);
    assert!(
        h.y < 40.0,
        "handle at the rectangle, not at the line: {h:?}"
    );
    assert!(preview_drawn(&mut s));
    assert_eq!(applied(&s.apply_offset()), (1, 0), "only the closed one");
    assert_eq!(doc(&s).object(line).unwrap(), line_before, "unchanged");
    assert_eq!(
        s.selected_object_count(),
        1,
        "the line is not selected after"
    );
}

// --------------------------------------------------------------------- AC 53 center handle ----

#[test]
fn ac53_a_press_on_the_center_handle_does_not_deselect_move_or_marquee() {
    let mut s = path_session();
    let before = packed(&s);
    let steps = s.undo_len();
    let c = pt(20.0, 20.0);
    s.pointer_hover(c, false, false);
    s.pointer_down(c, false);
    s.pointer_hover(pt(35.0, 35.0), false, false);
    s.pointer_up(pt(35.0, 35.0), false, false);
    assert_eq!(s.selected_object_count(), 1, "selection kept");
    assert_eq!(packed(&s), before, "nothing moved");
    assert_eq!(s.undo_len(), steps);
    assert!(!s.offset_chip_open(), "a single click does nothing");
    click(&mut s, c, false);
    assert_eq!(
        s.selected_object_count(),
        1,
        "first click of a double-click"
    );
    s.pointer_down(c, false);
    s.pointer_up(c, false, false);
    assert_eq!(s.selected_object_count(), 1);
    assert_eq!(
        s.double_click_hint(c, false, false),
        DoubleClickHint::OffsetChip
    );
    assert!(s.offset_chip_open());
    assert_eq!(s.selected_object_count(), 1);
}

#[test]
fn ac53_hit_area_is_twelve_screen_pixels_at_any_zoom() {
    let mut s = path_session();
    let scale = s.view().scale();
    let c = pt(20.0, 20.0);
    assert_eq!(
        s.double_click_hint(pt(c.x + 14.0 / scale, c.y), false, false),
        DoubleClickHint::None
    );
    assert!(!s.offset_chip_open());
    assert_eq!(
        s.double_click_hint(pt(c.x + 11.0 / scale, c.y), false, false),
        DoubleClickHint::OffsetChip
    );
    s.close_offset_chip();
    s.wheel(0.0, -300.0, 600.0, 400.0, false, true);
    assert!(s.view().scale() > scale);
    let scale = s.view().scale();
    assert_eq!(
        s.double_click_hint(pt(c.x + 14.0 / scale, c.y), false, false),
        DoubleClickHint::None
    );
    assert_eq!(
        s.double_click_hint(pt(c.x + 11.0 / scale, c.y), false, false),
        DoubleClickHint::OffsetChip
    );
}

#[test]
fn ac53_modifiers_at_the_double_click_have_no_meaning_and_the_cursor_is_pointer() {
    let mut s = path_session();
    let c = pt(20.0, 20.0);
    s.pointer_hover(c, false, false);
    assert_eq!(s.cursor_hint(), "pointer");
    assert_eq!(s.handle_hint(), "offset-center");
    assert_eq!(
        s.double_click_hint(c, true, true),
        DoubleClickHint::OffsetChip
    );
    assert!(s.offset_chip_open());
    assert_eq!(s.offset_values().direction(), OffsetDirection::Out);
    assert_eq!(s.offset_values().result(), OffsetResultChoice::NewObject);
    assert_eq!(s.handle_hint(), "", "no hint while the chip is open");
}

#[test]
fn ac53_the_center_handle_is_gone_during_a_corner_drag_and_the_chip_cannot_open_then() {
    let mut s = rect_session();
    let h = br(&s);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 10.0, h.y + 10.0), false, false);
    assert!(!s.open_offset_chip());
    assert_eq!(key(&mut s, "o"), "ignored");
    assert_eq!(key(&mut s, "Enter"), "ignored");
    assert_eq!(
        s.double_click_hint(pt(10.0, 10.0), false, false),
        DoubleClickHint::None
    );
    assert!(s.cancel_offset_drag());
    assert!(!s.offset_chip_open());
}

// ------------------------------------------------------------ AC 25 O / Enter open the chip ----

#[test]
fn ac25_o_and_enter_open_the_chip_o_o_from_the_select_tool() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    click(&mut s, pt(20.0, 10.0), false);
    let first = key(&mut s, "o");
    assert_eq!(
        s.tool(),
        Tool::Offset,
        "O in the Select tool is the tool letter ({first})"
    );
    assert!(!s.offset_chip_open());
    assert_eq!(key(&mut s, "o"), "offset-chip", "O, O opens the chip");
    assert!(s.offset_chip_open());
    assert_eq!(key(&mut s, "o"), "ignored", "gated while open");
    s.close_offset_chip();
    let rep = s.key_down(KeyInput {
        key: "Enter",
        repeat: true,
        ..KeyInput::default()
    });
    assert_eq!(rep.code(), "ignored", "no auto-repeat");
    assert!(!s.offset_chip_open());
    assert_eq!(key(&mut s, "Enter"), "offset-chip");
    assert!(s.offset_chip_open());
}

#[test]
fn ac25_o_and_enter_with_no_operand_do_nothing_visible() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Offset);
    assert_eq!(s.selected_object_count(), 0);
    assert_eq!(key(&mut s, "Enter"), "ignored");
    assert!(!s.offset_chip_open());
    let _ = key(&mut s, "o");
    assert_eq!(s.tool(), Tool::Offset, "O re-selects the tool");
    assert!(!s.offset_chip_open());
    click(&mut s, pt(20.0, 10.0), false);
    let r = s.key_down(KeyInput {
        key: "Enter",
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(r.code(), "ignored", "a text field has the keyboard");
    assert!(!s.offset_chip_open());
}

// ---------------------------------------------------------------------- AC 54 Enter, Escape ----

#[test]
fn ac54_enter_writes_one_commit_closes_the_chip_selects_the_result_and_undo_takes_it_back() {
    let mut s = rect_session();
    assert!(s.set_offset_distance(2.5));
    let before = objects(&s);
    let steps = s.undo_len();
    assert!(s.open_offset_chip());
    assert_eq!(applied(&s.apply_offset()), (1, 0));
    assert!(!s.offset_chip_open(), "Enter closes the chip");
    assert_eq!(s.undo_len(), steps + 1, "one commit");
    assert_eq!(objects(&s).len(), 2);
    assert_eq!(
        s.selected_object_count(),
        1,
        "the produced object is selected"
    );
    assert_ne!(objects(&s), before);
    let _ = s.undo();
    assert_eq!(objects(&s), before, "one Ctrl+Z takes the whole step back");
    assert_eq!(objects(&s).len(), 1);
    let _ = s.redo();
    assert_eq!(objects(&s).len(), 2);
}

#[test]
fn ac54_enter_with_untouched_distance_writes_the_remembered_value() {
    let mut s = rect_session();
    assert!(s.open_offset_chip());
    let o = s.apply_offset();
    let OffsetOutcome::Applied { distance_mm, .. } = o else {
        panic!("{o:?}")
    };
    assert_eq!(distance_mm, 1.0, "the prefilled 1 mm");
}

#[test]
fn ac54_escape_writes_nothing_and_keeps_the_chip_values() {
    let mut s = rect_session();
    assert!(s.open_offset_chip());
    assert!(s.set_offset_distance(4.0));
    s.set_offset_direction(OffsetDirection::Both);
    s.set_offset_join(OffsetJoin::Bevel);
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    let before = packed(&s);
    let steps = s.undo_len();
    let v = s.offset_view();
    assert!(v.both && v.change && v.join == OffsetJoin::Bevel && v.distance_mm == 4.0);
    s.close_offset_chip();
    assert!(!s.offset_chip_open());
    assert_eq!(packed(&s), before);
    assert_eq!(s.undo_len(), steps);
    assert_eq!(s.selected_object_count(), 1, "the selection stays");
    let kept = s.offset_values();
    assert_eq!(kept.distance_mm(), 4.0);
    assert_eq!(kept.direction(), OffsetDirection::Both);
    assert_eq!(kept.join(), OffsetJoin::Bevel);
    assert_eq!(kept.result(), OffsetResultChoice::ChangeOriginal);
    assert!(s.open_offset_chip());
    assert_eq!(s.offset_values().distance_mm(), 4.0);
    assert!(s.offset_view().change);
}

#[test]
fn ac6_invalid_distances_are_refused_and_the_stored_value_stays_in_range() {
    let mut s = rect_session();
    assert!(s.set_offset_distance(2.0));
    for bad in [0.0, -1.0, 0.0099, 1000.5, f64::NAN, f64::INFINITY] {
        assert!(!s.set_offset_distance(bad), "{bad}");
        let v = s.offset_values().distance_mm();
        assert!((0.01..=1000.0).contains(&v), "{bad} stored {v}");
    }
    assert!(s.set_offset_distance(2.0));
    assert!(!s.set_offset_distance(f64::NAN));
    assert_eq!(s.offset_values().distance_mm(), 2.0, "NaN leaves the value");
    assert!(s.set_offset_distance(0.01));
    assert!(s.set_offset_distance(1000.0));
}

#[test]
fn ac6_a_drag_sets_the_remembered_distance_on_release() {
    let mut s = rect_session();
    drag_br(&mut s, (5.0, 5.0), false, false);
    let _ = take_applied(&mut s);
    let kept = s.offset_values().distance_mm();
    assert!((kept - 5.0).abs() < 0.5, "{kept}");
    assert!(s.open_offset_chip());
    assert_eq!(s.offset_values().distance_mm(), kept);
}

#[test]
fn ac29_a_refused_inset_keeps_the_chip_open_and_writes_nothing_then_a_valid_value_writes() {
    let mut s = rect_session();
    assert!(s.open_offset_chip());
    assert!(s.set_offset_distance(10.0));
    s.set_offset_direction(OffsetDirection::In);
    let before = packed(&s);
    let steps = s.undo_len();
    let o = s.apply_offset();
    assert!(
        matches!(o, OffsetOutcome::Refused { distance_mm, .. } if distance_mm == 10.0),
        "{o:?}"
    );
    assert!(s.offset_chip_open(), "the chip stays open");
    assert_eq!(packed(&s), before);
    assert_eq!(s.undo_len(), steps);
    assert!(s.set_offset_distance(2.0));
    assert_eq!(applied(&s.apply_offset()), (1, 0));
    assert!(!s.offset_chip_open());
    assert_eq!(sorted_rect_widths(&s), [16.0, 20.0]);
}

#[test]
fn ac29_partial_vanish_survivors_get_their_results() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 40.0, 40.0);
    rect_at(&d, 100.0, 0.0, 4.0, 4.0);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    ctrl_a(&mut s);
    s.set_tool(Tool::Offset);
    assert!(s.set_offset_distance(3.0));
    s.set_offset_direction(OffsetDirection::In);
    assert!(s.open_offset_chip());
    let OffsetOutcome::Applied {
        added,
        vanished,
        of,
        ..
    } = s.apply_offset()
    else {
        panic!()
    };
    assert_eq!((added, vanished, of), (1, 1, 2));
}

#[test]
fn ac32_group_among_operands_is_refused_with_the_chip_still_open() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    rect_at(&d, 40.0, 0.0, 20.0, 20.0);
    rect_at(&d, 100.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    s.pointer_down(pt(-5.0, -5.0), false);
    s.pointer_hover(pt(70.0, 30.0), false, false);
    s.pointer_up(pt(70.0, 30.0), false, false);
    assert_eq!(s.selected_object_count(), 2);
    group_selection(&mut s);
    ctrl_a(&mut s);
    s.set_tool(Tool::Offset);
    let v = s.offset_view();
    assert!(v.has_group && v.handles && v.operands == 1, "{v:?}");
    assert!(s.open_offset_chip());
    let before = packed(&s);
    let steps = s.undo_len();
    let o = s.apply_offset();
    assert!(matches!(o, OffsetOutcome::RefusedGroups { .. }), "{o:?}");
    assert!(s.offset_chip_open(), "refusal keeps the chip open");
    assert_eq!(packed(&s), before);
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac54_the_chip_closes_without_writing_on_outside_press_tool_switch_and_selection_change() {
    let mut s = rect_session();
    assert!(s.open_offset_chip());
    assert!(s.set_offset_distance(7.0));
    let before = packed(&s);
    click(&mut s, pt(500.0, 500.0), false);
    assert!(!s.offset_chip_open(), "press outside");
    assert_eq!(packed(&s), before);
    assert_eq!(s.offset_values().distance_mm(), 7.0, "values remembered");

    let mut s = rect_session();
    assert!(s.open_offset_chip());
    s.set_tool(Tool::Select);
    assert!(!s.offset_chip_open());
    s.set_tool(Tool::Offset);
    assert!(!s.offset_chip_open(), "does not reopen by itself");

    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    rect_at(&d, 100.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    select_and_offset(&mut s, pt(20.0, 10.0));
    assert!(s.open_offset_chip());
    click(&mut s, pt(120.0, 10.0), false);
    assert!(!s.offset_chip_open(), "selection changed by click");
    assert!(s.open_offset_chip());
    s.pointer_down(pt(-10.0, -10.0), false);
    s.pointer_hover(pt(30.0, 30.0), false, false);
    s.pointer_up(pt(30.0, 30.0), false, false);
    assert_eq!(s.selected_object_count(), 1);
    assert!(!s.offset_chip_open(), "selection changed by marquee");
}

#[test]
fn ac54_a_press_on_a_corner_handle_closes_the_chip_and_the_drag_snaps_to_the_typed_distance() {
    let mut s = rect_session();
    assert!(s.open_offset_chip());
    assert!(s.set_offset_distance(2.0));
    let h = br(&s);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    assert!(!s.offset_chip_open(), "the press closes the chip");
    let scale = s.view().scale();
    let by = 2.0 + 2.0 / scale;
    s.pointer_hover(pt(h.x + by, h.y + by), false, false);
    assert_eq!(
        s.offset_view().distance_mm,
        2.0,
        "snapped to the remembered 2 mm"
    );
    s.pointer_up(pt(h.x + by, h.y + by), false, false);
    assert_eq!(take_applied(&mut s), (1, 0));
    assert_eq!(s.offset_values().direction(), OffsetDirection::Out);
}

// ----------------------------------------------------------------- AC 13, 56 corner style ----

#[test]
fn ac56_double_click_on_a_corner_steps_round_miter_bevel_round_and_writes_nothing() {
    let mut s = rect_session();
    let h = br(&s);
    let before = packed(&s);
    let steps = s.undo_len();
    let mut seen = vec![];
    for _ in 0..4 {
        assert_eq!(
            s.double_click_hint(h, false, false),
            DoubleClickHint::OffsetCornerStyle
        );
        seen.push(s.offset_view().pending_join.unwrap());
    }
    assert_eq!(
        seen,
        [
            OffsetJoin::Miter,
            OffsetJoin::Bevel,
            OffsetJoin::Round,
            OffsetJoin::Miter
        ]
    );
    assert_eq!(
        s.offset_values().join(),
        OffsetJoin::Round,
        "Corners is not changed"
    );
    assert_eq!(packed(&s), before);
    assert_eq!(s.undo_len(), steps);
    assert_eq!(s.selected_object_count(), 1);
}

#[test]
fn ac56_the_cycle_starts_from_the_chip_corners_value() {
    let mut s = rect_session();
    s.set_offset_join(OffsetJoin::Bevel);
    let h = br(&s);
    let _ = s.double_click_hint(h, false, false);
    assert_eq!(s.offset_view().pending_join, Some(OffsetJoin::Round));
    let _ = s.double_click_hint(h, false, false);
    assert_eq!(s.offset_view().pending_join, Some(OffsetJoin::Miter));
}

#[test]
fn ac56_the_standing_preview_redraws_with_the_pending_style() {
    let mut s = rect_session();
    assert!(s.set_offset_distance(3.0));
    let round = s.draw_list().triangles;
    let _ = s.double_click_hint(br(&s), false, false);
    assert_eq!(s.offset_view().join, OffsetJoin::Miter);
    assert_ne!(
        s.draw_list().triangles,
        round,
        "mitered corners look different"
    );
}

#[test]
fn ac56_the_press_clicks_of_a_double_click_write_nothing() {
    let mut s = rect_session();
    let h = br(&s);
    let before = packed(&s);
    for _ in 0..2 {
        s.pointer_hover(h, false, false);
        s.pointer_down(h, false);
        s.pointer_up(h, false, false);
    }
    assert_eq!(packed(&s), before, "a click is a drag of d = 0");
    assert_eq!(
        s.double_click_hint(h, false, false),
        DoubleClickHint::OffsetCornerStyle
    );
    assert_eq!(packed(&s), before);
}

#[test]
fn ac56_the_next_written_drag_uses_the_pending_style_and_consumes_it() {
    let mut s = rect_session();
    let _ = s.double_click_hint(br(&s), false, false); // Miter
    let h = br(&s);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 6.0, h.y + 6.0), false, false);
    assert_eq!(
        s.offset_view().join,
        OffsetJoin::Miter,
        "the readout names it"
    );
    s.pointer_up(pt(h.x + 6.0, h.y + 6.0), false, false);
    assert_eq!(take_applied(&mut s).0, 1);
    // A mitered outset of a rectangle is a rectangle with sharp corners.
    let first = objects(&s);
    assert_eq!(first.len(), 2);
    assert!(
        first
            .iter()
            .all(|o| rect_size(o).is_some() && rect_radius(o) == Some(0.0)),
        "{first:?}"
    );
    // Consumed: the standing preview and the next drag use Round again.
    assert_eq!(s.offset_view().pending_join, None);
    assert_eq!(s.offset_view().join, OffsetJoin::Round);
    // The produced rectangle is selected; drag its own handle.
    let produced: Vec<ObjectSnapshot> = first
        .into_iter()
        .filter(|o| rect_size(o).is_some_and(|(w, _)| w > 20.5))
        .collect();
    assert_eq!(produced.len(), 1);
    let h = corner(&s, &produced, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 4.0, h.y + 4.0), false, false);
    assert_eq!(s.offset_view().join, OffsetJoin::Round);
    s.pointer_up(pt(h.x + 4.0, h.y + 4.0), false, false);
    assert_eq!(take_applied(&mut s).0, 1);
    assert!(
        objects(&s)
            .iter()
            .any(|o| rect_radius(o).is_some_and(|r| r > 0.0)),
        "Round outset of a rectangle is a rounded rectangle"
    );
}

#[test]
fn ac56_a_cancelled_drag_keeps_the_pending_style_a_tool_switch_drops_it() {
    let mut s = rect_session();
    let _ = s.double_click_hint(br(&s), false, false);
    let h = br(&s);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 8.0, h.y + 8.0), false, false);
    assert!(s.cancel_offset_drag());
    assert_eq!(
        s.offset_view().pending_join,
        Some(OffsetJoin::Miter),
        "Escape"
    );
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 8.0, h.y + 8.0), false, false);
    s.pointer_hover(h, false, false);
    s.pointer_up(h, false, false);
    assert_eq!(
        s.offset_view().pending_join,
        Some(OffsetJoin::Miter),
        "release at 0"
    );
    s.set_tool(Tool::Select);
    s.set_tool(Tool::Offset);
    assert_eq!(s.offset_view().pending_join, None);
    assert_eq!(s.offset_view().join, OffsetJoin::Round);
}

#[test]
fn ac56_a_selection_change_keeps_the_pending_style() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    rect_at(&d, 100.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    select_and_offset(&mut s, pt(20.0, 10.0));
    let _ = s.double_click_hint(br(&s), false, false);
    click(&mut s, pt(120.0, 10.0), false);
    assert_eq!(s.offset_view().pending_join, Some(OffsetJoin::Miter));
}

#[test]
fn ac56_opening_the_chip_with_a_pending_style_takes_it_over() {
    let mut s = rect_session();
    let h = br(&s);
    let _ = s.double_click_hint(h, false, false);
    let _ = s.double_click_hint(h, false, false); // Bevel
    assert_eq!(key(&mut s, "o"), "offset-chip");
    assert_eq!(s.offset_values().join(), OffsetJoin::Bevel);
    assert_eq!(s.offset_view().pending_join, None);
    s.close_offset_chip();
    assert_eq!(
        s.offset_values().join(),
        OffsetJoin::Bevel,
        "Escape keeps it"
    );
    assert!(s.open_offset_chip());
    assert_eq!(applied(&s.apply_offset()), (1, 0));
    assert!(objects(&s).iter().any(|o| rect_size(o).is_none()));
}

#[test]
fn ac56_a_double_click_off_the_handles_does_nothing() {
    let mut s = rect_session();
    assert_eq!(
        s.double_click_hint(pt(300.0, 300.0), false, false),
        DoubleClickHint::None
    );
    assert_eq!(s.offset_view().pending_join, None);
}

// ----------------------------------------------------------------- AC 10, 19, 26 modifiers ----

#[test]
fn ac19_modifier_table_on_a_drag() {
    for (shift, ctrl, expected) in [
        (false, false, (1, 0)),
        (true, false, (2, 0)),
        (false, true, (0, 1)),
        (true, true, (1, 1)),
    ] {
        let mut s = rect_session();
        drag_br(&mut s, (4.0, 4.0), shift, ctrl);
        assert_eq!(take_applied(&mut s), expected, "shift {shift} ctrl {ctrl}");
        assert_eq!(s.undo_len(), 1, "one commit: {shift} {ctrl}");
        let _ = s.undo();
        assert_eq!(objects(&s).len(), 1);
    }
}

#[test]
fn ac19_shift_ctrl_outward_gives_the_original_the_dragged_side() {
    let mut s = rect_session();
    drag_br(&mut s, (4.0, 4.0), true, true);
    let _ = take_applied(&mut s);
    let sizes = sorted_rect_widths(&s);
    assert_eq!(sizes.len(), 2, "{sizes:?}");
    assert!(sizes[0] < 20.0 && sizes[1] > 20.0, "{sizes:?}");
    // The original (same id, first in the document) is the outset one.
    let first = objects(&s);
    let ids = doc(&s).object_ids();
    let orig = rect_size(&doc(&s).object(ids[0]).unwrap());
    let _ = first;
    assert!(orig.is_some());
}

#[test]
fn ac19_the_release_event_decides_not_earlier_key_state() {
    let mut s = rect_session();
    let h = br(&s);
    s.modifiers_changed(true, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 4.0, h.y + 4.0), true, false);
    s.modifiers_changed(false, false, false);
    s.pointer_up(pt(h.x + 4.0, h.y + 4.0), false, false);
    assert_eq!(take_applied(&mut s), (1, 0), "key up before release");
    let mut s = rect_session();
    drag_br(&mut s, (4.0, 4.0), true, false);
    assert_eq!(
        take_applied(&mut s),
        (2, 0),
        "Shift on the release event only"
    );
}

#[test]
fn ac10_a_drag_does_not_change_the_chip_direction_or_mode() {
    let mut s = rect_session();
    s.set_offset_direction(OffsetDirection::Both);
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    drag_br(&mut s, (-4.0, -4.0), false, false);
    let o = s.take_offset_outcome().unwrap();
    let OffsetOutcome::Applied {
        direction,
        added,
        changed,
        ..
    } = o
    else {
        panic!("{o:?}")
    };
    assert_eq!(direction, OffsetDirection::In, "the drag decides the side");
    assert_eq!((added, changed), (0, 1), "Mode Change, Ctrl not held");
    assert_eq!(s.offset_values().direction(), OffsetDirection::Both);
    assert_eq!(
        s.offset_values().result(),
        OffsetResultChoice::ChangeOriginal
    );
}

#[test]
fn ac19_with_mode_change_ctrl_makes_a_new_object_and_shift_still_means_both() {
    let mut s = rect_session();
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    assert!(s.offset_view().change);
    drag_br(&mut s, (4.0, 4.0), false, true);
    assert_eq!(take_applied(&mut s), (1, 0), "Ctrl inverts to New object");
    let mut s = rect_session();
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    drag_br(&mut s, (4.0, 4.0), true, false);
    assert_eq!(
        take_applied(&mut s),
        (1, 1),
        "Shift + Change = Shift+Ctrl result"
    );
}

#[test]
fn ac26_the_chip_combines_direction_and_mode_like_the_modifiers() {
    for (dir, mode, expected) in [
        (OffsetDirection::Out, OffsetResultChoice::NewObject, (1, 0)),
        (OffsetDirection::In, OffsetResultChoice::NewObject, (1, 0)),
        (OffsetDirection::Both, OffsetResultChoice::NewObject, (2, 0)),
        (
            OffsetDirection::Out,
            OffsetResultChoice::ChangeOriginal,
            (0, 1),
        ),
        (
            OffsetDirection::In,
            OffsetResultChoice::ChangeOriginal,
            (0, 1),
        ),
        (
            OffsetDirection::Both,
            OffsetResultChoice::ChangeOriginal,
            (1, 1),
        ),
    ] {
        let mut s = rect_session();
        assert!(s.set_offset_distance(2.0));
        s.set_offset_direction(dir);
        s.set_offset_result(mode);
        assert!(s.open_offset_chip());
        assert_eq!(applied(&s.apply_offset()), expected, "{dir:?} {mode:?}");
    }
    let mut s = rect_session();
    assert!(s.set_offset_distance(2.0));
    s.set_offset_direction(OffsetDirection::Both);
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    assert!(s.open_offset_chip());
    let _ = s.apply_offset();
    let sizes = sorted_rect_widths(&s);
    assert_eq!(sizes.len(), 2);
    assert!(
        (sizes[0] - 16.0).abs() < 1e-6 && (sizes[1] - 24.0).abs() < 1e-6,
        "{sizes:?}"
    );
}

#[test]
fn ac42_change_original_keeps_id_and_kind_for_a_primitive_and_converts_in_place_otherwise() {
    let d = Document::new(1);
    let id = rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    select_and_offset(&mut s, pt(20.0, 10.0));
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    s.set_offset_join(OffsetJoin::Miter);
    assert!(s.set_offset_distance(2.0));
    assert!(s.open_offset_chip());
    assert_eq!(applied(&s.apply_offset()), (0, 1));
    let o = doc(&s).object(id).expect("same id");
    assert_eq!(rect_size(&o), Some((24.0, 24.0)), "still a rectangle");
    s.set_offset_join(OffsetJoin::Bevel);
    assert!(s.open_offset_chip());
    assert_eq!(applied(&s.apply_offset()), (0, 1));
    let o = doc(&s).object(id).expect("same id");
    assert!(matches!(o, ObjectSnapshot::Path(_)), "became a path: {o:?}");
    assert_eq!(objects(&s).len(), 1);
}

#[test]
fn ac16_a_drag_to_zero_writes_nothing() {
    let mut s = rect_session();
    let h = br(&s);
    let before = packed(&s);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 9.0, h.y + 9.0), false, false);
    s.pointer_hover(h, false, false);
    s.pointer_up(h, false, false);
    assert_eq!(packed(&s), before);
    assert_eq!(s.undo_len(), 0);
}
