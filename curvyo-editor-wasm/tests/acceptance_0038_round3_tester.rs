//! Tester's acceptance tests for the third round of `specs/0038-path-offset` (customer decisions
//! of 2026-10-13): the mode table (criteria 1, 15, 19), preview visibility (52, 54), the chip
//! opened by a double-click with modifiers (57) and undo of a Shift write (16).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    dead_code,
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

fn node_ids(s: &Session) -> Vec<NodeId> {
    doc(s).object_ids()
}

/// Width of the rectangle `id` in millimetres.
fn width_of(s: &Session, id: NodeId) -> f64 {
    rect_size(&doc(s).object(id).unwrap()).unwrap().0
}

/// Widths of all objects in stacking order, bottom first.
fn widths(s: &Session) -> Vec<f64> {
    objects(s).iter().map(|o| rect_size(o).unwrap().0).collect()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.02
}

fn assert_widths(s: &Session, want: &[f64], what: &str) {
    let got = widths(s);
    assert!(
        got.len() == want.len() && got.iter().zip(want).all(|(a, b)| near(*a, *b)),
        "{what}: widths {got:?}, wanted {want:?}"
    );
}

/// One 20 x 20 rectangle; returns its id.
fn rect_with_id() -> (Session, NodeId) {
    let d = Document::new(1);
    let id = rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    select_and_offset(&mut s, pt(20.0, 10.0));
    (s, id)
}

// ------------------------------------------------------------------ AC 19 the mode table ------

#[test]
fn ac19_no_modifier_changes_the_original_outward_and_inward() {
    for (delta, want) in [(3.0, 26.0), (-3.0, 14.0)] {
        let (mut s, id) = rect_with_id();
        let steps = s.undo_len();
        drag_br(&mut s, (delta, delta), false, false);
        assert_eq!(take_applied(&mut s), (0, 1));
        assert_eq!(node_ids(&s), vec![id], "same object, no new one");
        assert!(near(width_of(&s, id), want), "{}", width_of(&s, id));
        assert_eq!(s.undo_len(), steps + 1);
    }
}

#[test]
fn ac19_ctrl_makes_a_new_object_and_keeps_the_original() {
    let (mut s, id) = rect_with_id();
    drag_br(&mut s, (3.0, 3.0), false, true);
    assert_eq!(take_applied(&mut s), (1, 0));
    assert!(near(width_of(&s, id), 20.0));
    assert_widths(&s, &[26.0, 20.0], "outset copy is below");
    assert_eq!(node_ids(&s)[1], id);

    let (mut s, id) = rect_with_id();
    drag_br(&mut s, (-3.0, -3.0), false, true);
    assert_eq!(take_applied(&mut s), (1, 0));
    assert!(near(width_of(&s, id), 20.0));
    assert_widths(&s, &[20.0, 14.0], "inset copy is above");
    assert_eq!(node_ids(&s)[0], id);
}

#[test]
fn ac19_shift_original_takes_the_outset_side_and_one_inner_copy_sits_directly_above() {
    // Independent of the drag direction.
    for delta in [3.0, -3.0] {
        let d = Document::new(1);
        let below = rect_at(&d, 100.0, 100.0, 5.0, 5.0);
        let id = rect_at(&d, 0.0, 0.0, 20.0, 20.0);
        let above = rect_at(&d, 100.0, 120.0, 7.0, 7.0);
        let mut s = open(&d);
        select_and_offset(&mut s, pt(20.0, 10.0));
        assert_eq!(s.selected_object_count(), 1);
        let steps = s.undo_len();
        let h = corner(&s, &[doc(&s).object(id).unwrap()], BR);
        s.pointer_hover(h, false, false);
        s.pointer_down(h, false);
        s.pointer_hover(pt(h.x + delta, h.y + delta), true, false);
        s.pointer_up(pt(h.x + delta, h.y + delta), true, false);
        assert_eq!(take_applied(&mut s), (1, 1), "drag {delta}");
        assert_eq!(s.undo_len(), steps + 1);
        let ids = node_ids(&s);
        assert_eq!(ids.len(), 4);
        assert_eq!(ids[0], below);
        assert_eq!(ids[1], id, "the original keeps its id and place");
        assert_eq!(ids[3], above);
        assert!(near(width_of(&s, id), 26.0), "original is the outset");
        assert!(
            near(rect_size(&doc(&s).object(ids[2]).unwrap()).unwrap().0, 14.0),
            "the new copy is the inset, directly above the original"
        );
        // Selection: the changed original and its inner copy.
        assert_eq!(s.selected_object_count(), 2);
    }
}

#[test]
fn ac19_ctrl_shift_makes_two_new_objects_the_original_stays_between() {
    for delta in [3.0, -3.0] {
        let (mut s, id) = rect_with_id();
        drag_br(&mut s, (delta, delta), true, true);
        assert_eq!(take_applied(&mut s), (2, 0), "drag {delta}");
        assert_widths(&s, &[26.0, 20.0, 14.0], "outer below, inner above");
        assert_eq!(node_ids(&s)[1], id);
    }
}

#[test]
fn ac19_remembered_new_object_inverts_ctrl() {
    let (mut s, id) = rect_with_id();
    s.set_offset_result(OffsetResultChoice::NewObject);
    drag_br(&mut s, (3.0, 3.0), false, false);
    assert_eq!(take_applied(&mut s), (1, 0), "no modifier: New object");
    let (mut s, id2) = rect_with_id();
    s.set_offset_result(OffsetResultChoice::NewObject);
    drag_br(&mut s, (3.0, 3.0), false, true);
    assert_eq!(take_applied(&mut s), (0, 1), "Ctrl: Change original");
    assert_eq!(node_ids(&s), vec![id2]);
    let _ = id;
}

#[test]
fn ac19_keys_may_be_pressed_during_the_drag_and_the_release_counts() {
    let (mut s, _id) = rect_with_id();
    let h = br(&s);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 3.0, h.y + 3.0), true, false);
    // Shift released again before the release: no modifier counts.
    s.pointer_up(pt(h.x + 3.0, h.y + 3.0), false, false);
    assert_eq!(take_applied(&mut s), (0, 1));
}

#[test]
fn ac19_chip_enter_follows_the_same_table() {
    // Direction Both + Change original = Shift result; Both + New object = Ctrl+Shift result.
    let (mut s, id) = rect_with_id();
    s.set_offset_distance(3.0);
    s.set_offset_direction(OffsetDirection::Both);
    assert_eq!(key(&mut s, "o"), "offset-chip");
    assert_eq!(applied(&s.apply_offset()), (1, 1));
    assert_widths(&s, &[26.0, 14.0], "Both + Change original");
    assert_eq!(node_ids(&s)[0], id);

    let (mut s, id) = rect_with_id();
    s.set_offset_distance(3.0);
    s.set_offset_direction(OffsetDirection::Both);
    s.set_offset_result(OffsetResultChoice::NewObject);
    assert_eq!(key(&mut s, "o"), "offset-chip");
    assert_eq!(applied(&s.apply_offset()), (2, 0));
    assert_widths(&s, &[26.0, 20.0, 14.0], "Both + New object");
    assert_eq!(node_ids(&s)[1], id);
}

#[test]
fn ac54_initial_chip_state_is_change_original() {
    let s = Session::open(2, &pack(&Document::new(1), "0.1.0").unwrap()).unwrap();
    let v = s.offset_values();
    assert_eq!(v.result(), OffsetResultChoice::ChangeOriginal);
    assert_eq!(v.direction(), OffsetDirection::Out);
    assert_eq!(v.join(), OffsetJoin::Round);
    assert!(near(v.distance_mm(), 1.0));
}

// ------------------------------------------------- AC 52, 54 preview visibility ---------------

/// A preview is drawn if switching the remembered Corners changes the canvas while a drag past
/// the dead zone or the open chip applies them (the box and handles do not depend on Corners).
fn preview_by_join(s: &mut Session) -> bool {
    let keep = s.offset_values().join();
    s.set_offset_join(OffsetJoin::Round);
    let a = s.draw_list().triangles;
    s.set_offset_join(OffsetJoin::Bevel);
    let b = s.draw_list().triangles;
    s.set_offset_join(keep);
    a != b
}

#[test]
fn ac52_none_on_tool_choice_selection_release_escape_close_and_corner_double_click() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Offset);
    assert!(!preview_by_join(&mut s), "tool choice");
    assert!(!preview_drawn(&mut s), "tool choice");
    click(&mut s, pt(20.0, 10.0), false);
    assert!(s.offset_view().handles);
    assert!(!preview_by_join(&mut s), "selection");

    // After a Ctrl release (keeps a rectangle selected with handles).
    drag_br(&mut s, (3.0, 3.0), false, true);
    let _ = s.take_offset_outcome();
    assert!(!preview_by_join(&mut s), "after release");
    assert!(!preview_drawn(&mut s), "after release");

    // Escape of a drag (fresh square corners: a rounded result has nothing for Corners to change).
    let (mut s, _) = rect_with_id();
    let h = br(&s);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 4.0, h.y + 4.0), false, false);
    assert!(preview_by_join(&mut s), "live during the drag");
    let _ = s.escape();
    s.pointer_up(pt(h.x + 4.0, h.y + 4.0), false, false);
    let _ = s.take_offset_outcome();
    assert!(!preview_by_join(&mut s), "after Escape of a drag");
    assert!(!preview_drawn(&mut s), "after Escape of a drag");

    // Chip open then Escape.
    assert_eq!(key(&mut s, "o"), "offset-chip");
    assert!(preview_by_join(&mut s), "chip open");
    let _ = s.escape();
    assert!(!s.offset_chip_open());
    assert!(!preview_by_join(&mut s), "Escape closed the chip");
    assert!(!preview_drawn(&mut s), "Escape closed the chip");

    // Chip closed by close_offset_chip.
    assert!(s.open_offset_chip());
    assert!(preview_by_join(&mut s));
    s.close_offset_chip();
    assert!(!preview_by_join(&mut s), "chip closed");

    // Corner style double-click.
    let h = br(&s);
    assert_eq!(
        s.double_click_hint(h, false, false),
        DoubleClickHint::OffsetCornerStyle
    );
    assert!(!preview_by_join(&mut s), "after corner double-click");
    assert!(!preview_drawn(&mut s), "after corner double-click");
}

#[test]
fn ac52_drag_inside_the_dead_zone_shows_no_preview_past_it_shows_one() {
    let (mut s, _id) = rect_with_id();
    let h = br(&s);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    assert!(!preview_by_join(&mut s), "press");
    s.pointer_hover(pt(h.x + 0.3, h.y + 0.3), false, false);
    assert!(!s.offset_view().dragging);
    assert!(!preview_by_join(&mut s), "inside the dead zone");
    s.pointer_hover(pt(h.x + 6.0, h.y + 6.0), false, false);
    assert!(s.offset_view().dragging);
    assert!(preview_by_join(&mut s), "past the dead zone");
    s.pointer_up(pt(h.x + 6.0, h.y + 6.0), false, false);
    let _ = s.take_offset_outcome();
    assert!(!preview_by_join(&mut s), "released");
}

#[test]
fn ac52_open_chip_preview_follows_its_current_values() {
    let (mut s, _id) = rect_with_id();
    assert!(s.open_offset_chip());
    assert!(preview_drawn(&mut s), "distance");
    s.set_offset_direction(OffsetDirection::In);
    let a = s.draw_list().triangles;
    s.set_offset_direction(OffsetDirection::Both);
    let b = s.draw_list().triangles;
    assert_ne!(a, b, "direction changes the preview");
    s.set_offset_direction(OffsetDirection::Out);
    assert!(preview_by_join(&mut s), "corners change the preview");
    // Showing it writes nothing.
    let before = packed(&s);
    let steps = s.undo_len();
    let _ = s.draw_list();
    assert_eq!(packed(&s), before);
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac52_chip_opened_by_double_click_on_the_center_has_a_preview_and_closing_removes_it() {
    let (mut s, _id) = rect_with_id();
    assert!(!preview_drawn(&mut s));
    assert_eq!(
        s.double_click_hint(pt(10.0, 10.0), false, false),
        DoubleClickHint::OffsetChip
    );
    assert!(preview_drawn(&mut s));
    s.close_offset_chip();
    assert!(!preview_drawn(&mut s));
}

// ----------------------------------------------------------------------------- AC 57 ---------

fn chip_values(s: &Session) -> (OffsetDirection, OffsetResultChoice) {
    let v = s.offset_values();
    (v.direction(), v.result())
}

fn dbl_center(s: &mut Session, shift: bool, ctrl: bool) {
    assert_eq!(
        s.double_click_hint(pt(10.0, 10.0), shift, ctrl),
        DoubleClickHint::OffsetChip
    );
    assert!(s.offset_chip_open());
}

#[test]
fn ac57_modifiers_preselect_the_chip() {
    use OffsetDirection::{Both, Out};
    use OffsetResultChoice::{ChangeOriginal, NewObject};
    for (shift, ctrl, want) in [
        (false, false, (Out, ChangeOriginal)),
        (true, false, (Both, ChangeOriginal)),
        (false, true, (Out, NewObject)),
        (true, true, (Both, NewObject)),
    ] {
        let (mut s, _) = rect_with_id();
        dbl_center(&mut s, shift, ctrl);
        assert_eq!(chip_values(&s), want, "shift={shift} ctrl={ctrl}");
        let v = s.offset_view();
        assert_eq!(v.both, shift);
        assert_eq!(v.change, !ctrl);
        assert!(preview_drawn(&mut s));
    }
}

#[test]
fn ac57_unnamed_options_show_the_remembered_values() {
    use OffsetDirection::{Both, In};
    use OffsetResultChoice::{ChangeOriginal, NewObject};
    let (mut s, _) = rect_with_id();
    s.set_offset_direction(In);
    s.set_offset_result(NewObject);
    s.set_offset_join(OffsetJoin::Miter);
    s.set_offset_distance(2.5);
    dbl_center(&mut s, false, false);
    assert_eq!(chip_values(&s), (In, NewObject), "no modifier");
    s.close_offset_chip();
    dbl_center(&mut s, true, false);
    assert_eq!(
        chip_values(&s),
        (Both, NewObject),
        "Shift keeps remembered Mode"
    );
    s.close_offset_chip();

    let (mut s, _) = rect_with_id();
    s.set_offset_direction(In);
    s.set_offset_join(OffsetJoin::Bevel);
    s.set_offset_distance(2.5);
    dbl_center(&mut s, false, true);
    assert_eq!(
        chip_values(&s),
        (In, NewObject),
        "Ctrl keeps remembered Direction"
    );
    assert_eq!(s.offset_values().join(), OffsetJoin::Bevel);
    assert!(near(s.offset_values().distance_mm(), 2.5));
    let _ = ChangeOriginal;
}

#[test]
fn ac57_ctrl_sets_new_object_even_when_that_is_already_remembered() {
    let (mut s, _) = rect_with_id();
    s.set_offset_result(OffsetResultChoice::NewObject);
    dbl_center(&mut s, false, true);
    assert_eq!(s.offset_values().result(), OffsetResultChoice::NewObject);
}

#[test]
fn ac57_o_and_enter_use_remembered_values_only() {
    let (mut s, _) = rect_with_id();
    s.set_offset_direction(OffsetDirection::In);
    assert_eq!(key(&mut s, "o"), "offset-chip");
    assert_eq!(chip_values(&s).0, OffsetDirection::In);
    s.close_offset_chip();
    assert_eq!(key(&mut s, "Enter"), "offset-chip");
    assert_eq!(
        chip_values(&s),
        (OffsetDirection::In, OffsetResultChoice::ChangeOriginal)
    );
}

#[test]
fn ac57_shift_double_click_leaves_both_for_next_time_and_enter_writes_the_shift_result() {
    let (mut s, id) = rect_with_id();
    s.set_offset_distance(3.0);
    dbl_center(&mut s, true, false);
    s.close_offset_chip();
    assert_eq!(
        s.offset_values().direction(),
        OffsetDirection::Both,
        "remembered"
    );
    assert_eq!(key(&mut s, "o"), "offset-chip");
    assert_eq!(chip_values(&s).0, OffsetDirection::Both);
    s.close_offset_chip();

    let (mut s, id2) = rect_with_id();
    s.set_offset_distance(3.0);
    dbl_center(&mut s, true, false);
    assert_eq!(applied(&s.apply_offset()), (1, 1));
    assert_widths(&s, &[26.0, 14.0], "Shift dbl-click Enter");
    assert_eq!(node_ids(&s)[0], id2);
    let _ = id;
    let (mut s, id3) = rect_with_id();
    s.set_offset_distance(3.0);
    dbl_center(&mut s, true, true);
    assert_eq!(applied(&s.apply_offset()), (2, 0));
    assert_widths(&s, &[26.0, 20.0, 14.0], "Ctrl+Shift dbl-click Enter");
    assert_eq!(node_ids(&s)[1], id3);
}

// ------------------------------------------------------------------------------ undo ----------

#[test]
fn undo_of_a_shift_write_is_one_step_and_redo_restores_it() {
    let (mut s, id) = rect_with_id();
    let steps = s.undo_len();
    drag_br(&mut s, (3.0, 3.0), true, false);
    assert_eq!(take_applied(&mut s), (1, 1));
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert!(s.undo().changed);
    assert_eq!(s.undo_len(), steps);
    assert_eq!(node_ids(&s), vec![id]);
    assert!(
        near(width_of(&s, id), 20.0),
        "original and copy both undone"
    );
    assert_widths(&s, &[20.0], "undone");
    assert!(s.redo().changed);
    assert_widths(&s, &[26.0, 14.0], "redo");
}

#[test]
fn undo_of_a_chip_both_write_is_one_step() {
    let (mut s, id) = rect_with_id();
    let steps = s.undo_len();
    dbl_center(&mut s, true, true);
    assert_eq!(applied(&s.apply_offset()), (2, 0));
    assert_eq!(s.undo_len(), steps + 1);
    assert!(s.undo().changed);
    assert_eq!(node_ids(&s), vec![id]);
    assert_widths(&s, &[20.0], "undone");
}
