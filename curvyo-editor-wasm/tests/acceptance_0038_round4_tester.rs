//! Tester's acceptance tests for the fourth round of `specs/0038-path-offset` (customer decision
//! of 2026-10-13): the corner style is stepped by an Alt keydown during a corner drag
//! (criterion 13), remembered by a written drag (56, 7) and shared with the chip's Corners (54);
//! the double-click on a corner handle changes nothing any more.

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
use curvyo_ui_core::{OffsetJoin, handle_positions, offset_box};

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

use curvyo_ui_core::OffsetJoin::{Bevel, Miter, Round};

const KEY_ALT: &str = "offset-corner-style";

fn alt_key(s: &mut Session, repeat: bool, shift: bool, ctrl: bool) -> &'static str {
    s.key_down(KeyInput {
        key: "Alt",
        alt: true,
        repeat,
        shift,
        ctrl,
        ..KeyInput::default()
    })
    .code()
}

fn alt(s: &mut Session) -> &'static str {
    alt_key(s, false, false, false)
}

fn press(s: &mut Session) -> Point {
    let h = br(s);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    h
}

fn press_moved(s: &mut Session, by: f64) -> Point {
    let h = press(s);
    s.pointer_hover(pt(h.x + by, h.y + by), false, false);
    h
}

fn join(s: &Session) -> OffsetJoin {
    s.offset_view().join
}

fn release(s: &mut Session, h: Point, by: f64) {
    s.pointer_up(pt(h.x + by, h.y + by), false, false);
}

#[test]
fn ac13_alt_steps_round_miter_bevel_round_and_view_follows_at_once() {
    let mut s = rect_session();
    let before = packed(&s);
    press_moved(&mut s, 6.0);
    assert_eq!(join(&s), Round, "the first drag uses Round");
    let mut canvases = vec![s.draw_list().triangles];
    let mut seen = vec![];
    for _ in 0..3 {
        assert_eq!(alt(&mut s), KEY_ALT);
        seen.push(join(&s));
        canvases.push(s.draw_list().triangles);
    }
    assert_eq!(seen, [Miter, Bevel, Round]);
    assert_ne!(canvases[0], canvases[1], "Miter redraws the preview");
    assert_ne!(canvases[1], canvases[2], "Bevel redraws the preview");
    assert_eq!(canvases[0], canvases[3], "back to Round: same preview");
    assert_eq!(packed(&s), before, "stepping writes nothing");
}

#[test]
fn ac13_auto_repeat_is_ignored() {
    let mut s = rect_session();
    press_moved(&mut s, 6.0);
    assert_eq!(alt(&mut s), KEY_ALT);
    for _ in 0..5 {
        assert_ne!(alt_key(&mut s, true, false, false), KEY_ALT);
    }
    assert_eq!(join(&s), Miter, "held Alt steps once");
    assert_eq!(alt(&mut s), KEY_ALT, "a fresh keydown steps again");
    assert_eq!(join(&s), Bevel);
}

#[test]
fn ac13_alt_outside_a_corner_drag_is_not_used() {
    let mut s = rect_session();
    assert_ne!(alt(&mut s), KEY_ALT, "handles shown, no drag");
    assert_eq!(s.offset_values().join(), Round);
    let h = press_moved(&mut s, 6.0);
    s.pointer_up(pt(h.x + 6.0, h.y + 6.0), false, false);
    let _ = s.take_offset_outcome();
    assert_ne!(alt(&mut s), KEY_ALT, "after the release");
    // Select tool with a drag of a different kind.
    let mut s = rect_session();
    s.set_tool(Tool::Select);
    assert_ne!(alt(&mut s), KEY_ALT);
}

#[test]
fn ac13_alt_held_at_the_press_is_not_used_and_needs_a_fresh_keydown() {
    let mut s = rect_session();
    s.modifiers_changed(false, false, true);
    let h = press(&mut s);
    s.pointer_hover(pt(h.x + 6.0, h.y + 6.0), false, false);
    assert_eq!(join(&s), Round, "Alt at the press does not step");
    assert!(s.offset_view().dragging, "the drag starts as before");
    assert_ne!(alt_key(&mut s, true, false, false), KEY_ALT);
    assert_eq!(join(&s), Round);
    assert_eq!(alt(&mut s), KEY_ALT);
    assert_eq!(join(&s), Miter);
}

#[test]
fn ac13_alt_works_in_the_dead_zone() {
    let mut s = rect_session();
    let h = press(&mut s);
    s.pointer_hover(pt(h.x + 0.05, h.y + 0.05), false, false);
    assert!(!s.offset_view().dragging, "premise: still in the dead zone");
    assert_eq!(alt(&mut s), KEY_ALT);
    assert_eq!(join(&s), Miter);
    s.pointer_hover(pt(h.x + 6.0, h.y + 6.0), false, false);
    assert_eq!(join(&s), Miter, "the step survives leaving the dead zone");
}

#[test]
fn ac13_alt_steps_with_shift_or_ctrl_and_changes_nothing_else() {
    for (shift, ctrl) in [(true, false), (false, true), (true, true)] {
        let mut s = rect_session();
        let h = press_moved(&mut s, 6.0);
        let dir = s.offset_values().direction();
        let res = s.offset_values().result();
        assert_eq!(
            alt_key(&mut s, false, shift, ctrl),
            KEY_ALT,
            "{shift} {ctrl}"
        );
        assert_eq!(join(&s), Miter);
        assert_eq!(s.offset_values().direction(), dir);
        assert_eq!(s.offset_values().result(), res);
        s.pointer_up(pt(h.x + 6.0, h.y + 6.0), shift, ctrl);
        assert!(s.take_offset_outcome().is_some());
    }
}

#[test]
fn ac56_first_drag_round_written_drag_remembers_next_drag_starts_with_it() {
    let mut s = rect_session();
    assert_eq!(s.offset_values().join(), Round);
    let h = press_moved(&mut s, 6.0);
    alt(&mut s);
    alt(&mut s);
    assert_eq!(join(&s), Bevel);
    release(&mut s, h, 6.0);
    let _ = take_applied(&mut s);
    assert_eq!(s.offset_values().join(), Bevel, "stored at the release");
    let _ = press_moved(&mut s, 6.0);
    assert_eq!(join(&s), Bevel, "the next drag starts with Bevel");
    assert_eq!(alt(&mut s), KEY_ALT);
    assert_eq!(join(&s), Round, "Bevel steps to Round");
}

#[test]
fn ac56_a_style_stepped_back_to_round_is_stored_as_round() {
    let mut s = rect_session();
    s.set_offset_join(Bevel);
    let h = press_moved(&mut s, 6.0);
    assert_eq!(join(&s), Bevel);
    alt(&mut s);
    assert_eq!(join(&s), Round);
    release(&mut s, h, 6.0);
    let _ = take_applied(&mut s);
    assert_eq!(s.offset_values().join(), Round);
}

#[test]
fn ac56d_escape_other_button_zero_and_refused_release_do_not_store() {
    // Escape
    let mut s = rect_session();
    press_moved(&mut s, 6.0);
    alt(&mut s);
    assert_eq!(key(&mut s, "Escape"), "escape-drag");
    assert!(s.take_offset_outcome().is_none());
    assert_eq!(s.offset_values().join(), Round, "Escape");
    // Other button
    let mut s = rect_session();
    press_moved(&mut s, 6.0);
    alt(&mut s);
    assert!(s.cancel_offset_drag());
    assert_eq!(s.offset_values().join(), Round, "other button");
    // d = 0 release
    let mut s = rect_session();
    let h = press_moved(&mut s, 6.0);
    alt(&mut s);
    let before = packed(&s);
    s.pointer_hover(h, false, false);
    s.pointer_up(h, false, false);
    assert_eq!(packed(&s), before, "d = 0 writes nothing");
    assert_eq!(s.offset_values().join(), Round, "d = 0");
    // refused: inset that leaves nothing
    let mut s = rect_session();
    let h = press_moved(&mut s, -6.0);
    alt(&mut s);
    s.pointer_hover(pt(h.x - 30.0, h.y - 30.0), false, false);
    let before = packed(&s);
    s.pointer_up(pt(h.x - 30.0, h.y - 30.0), false, false);
    let wrote = packed(&s) != before;
    let _ = s.take_offset_outcome();
    let stored = s.offset_values().join();
    assert!(
        !wrote,
        "premise: an inset of 20 mm leaves nothing and writes nothing"
    );
    assert_eq!(stored, Round, "refused release");
}

#[test]
fn ac56e_chip_corners_shows_and_sets_the_remembered_style() {
    let mut s = rect_session();
    let h = press_moved(&mut s, 6.0);
    alt(&mut s);
    release(&mut s, h, 6.0);
    let _ = take_applied(&mut s);
    assert!(s.open_offset_chip());
    assert_eq!(
        s.offset_values().join(),
        Miter,
        "chip shows the drag's style"
    );
    assert_eq!(join(&s), Miter);
    s.set_offset_join(Bevel);
    s.close_offset_chip();
    let _ = press_moved(&mut s, 6.0);
    assert_eq!(
        join(&s),
        Bevel,
        "the next drag starts with the chip's style"
    );
}

#[test]
fn ac54_chip_style_survives_escape() {
    let mut s = rect_session();
    assert_eq!(key(&mut s, "o"), "offset-chip");
    s.set_offset_join(Miter);
    assert!(!key(&mut s, "Escape").is_empty());
    assert_eq!(s.offset_values().join(), Miter);
    let _ = press_moved(&mut s, 6.0);
    assert_eq!(join(&s), Miter);
}

#[test]
fn ac13_double_click_on_a_corner_handle_changes_no_style_center_still_opens_chip() {
    let mut s = rect_session();
    let h = br(&s);
    let before = packed(&s);
    for _ in 0..2 {
        s.pointer_hover(h, false, false);
        s.pointer_down(h, false);
        s.pointer_up(h, false, false);
    }
    assert_eq!(s.double_click_hint(h, false, false), DoubleClickHint::None);
    assert!(!s.double_click(h, false, false) || packed(&s) == before);
    assert_eq!(s.offset_values().join(), Round);
    assert_eq!(join(&s), Round);
    assert_eq!(packed(&s), before);
    assert!(!s.offset_chip_open());
    // center handle
    let mut s = rect_session();
    let objs = objects(&s);
    let refs: Vec<&ObjectSnapshot> = objs.iter().collect();
    let b = offset_box(&refs).unwrap();
    let cs = b.document_corners();
    let c = pt(
        cs.iter().map(|p| p.x).sum::<f64>() / 4.0,
        cs.iter().map(|p| p.y).sum::<f64>() / 4.0,
    );
    s.pointer_hover(c, false, false);
    s.pointer_down(c, false);
    s.pointer_up(c, false, false);
    s.pointer_down(c, false);
    s.pointer_up(c, false, false);
    let _ = s.double_click(c, false, false);
    assert!(s.offset_chip_open(), "center double-click opens the chip");
}
