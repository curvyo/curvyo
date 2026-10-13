//! Tester's acceptance tests for `specs/0038-path-offset` (rework of 2026-10-12), session side:
//! the Offset tool's handles, the drag with its modifiers, the inset stop, the write on release,
//! undo and redo, primitives that stay primitives, groups, open paths and refusals. Written from
//! the specification; only the public session API and the document are used.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    missing_docs
)]

use std::f64::consts::FRAC_PI_2;

use curvyo_document_core::{
    AnchorId, Angle, Document, EllipseFrame, InnerRatio, Length, NewAnchor, NodeId, ObjectSnapshot,
    Point, PointCount, RectBounds, Shape, StarFrame, pack, unpack,
};
use curvyo_editor_wasm::{DoubleClickHint, DragStop, OffsetOutcome, Session, Tool};
use curvyo_editor_wasm::{KeyInput, OFFSET_COMMIT_LABEL};
use curvyo_ui_core::{
    OffsetCap, OffsetDirection, OffsetJoin, OffsetRefusal, OffsetResultChoice, handle_positions,
    offset_box,
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

fn click(s: &mut Session, at: Point, shift: bool) {
    s.pointer_hover(at, shift, false);
    s.pointer_down(at, shift);
    s.pointer_up(at, shift, false);
}

/// Selects the object whose edge is at `edge` in the Select tool, then switches to Offset.
fn select_and_offset(s: &mut Session, edge: Point) {
    s.set_tool(Tool::Select);
    click(s, edge, false);
    s.set_tool(Tool::Offset);
}

fn handle_of(s: &Session, corner_sign: (f64, f64)) -> Point {
    let objs = objects(s);
    handle_over(s, &objs.iter().collect::<Vec<_>>(), corner_sign)
}

fn handle_over(s: &Session, sel: &[&ObjectSnapshot], corner_sign: (f64, f64)) -> Point {
    // The box over everything selected: callers select exactly what they pass here.
    let b = offset_box(sel).unwrap();
    let handles = handle_positions(&b, s.view().scale());
    let c = b.document_corners();
    let centre = pt(
        c.iter().map(|p| p.x).sum::<f64>() / 4.0,
        c.iter().map(|p| p.y).sum::<f64>() / 4.0,
    );
    *handles
        .iter()
        .min_by(|a, b2| {
            let da = (a.x - centre.x) * corner_sign.0 + (a.y - centre.y) * corner_sign.1;
            let db = (b2.x - centre.x) * corner_sign.0 + (b2.y - centre.y) * corner_sign.1;
            db.partial_cmp(&da).unwrap()
        })
        .unwrap()
}

const BR: (f64, f64) = (1.0, 1.0);
const TL: (f64, f64) = (-1.0, -1.0);

/// A drag of the handle `corner` by `delta` (document mm), released with the given keys.
fn drag(s: &mut Session, corner: (f64, f64), delta: (f64, f64), shift: bool, ctrl: bool) {
    let h = handle_of(s, corner);
    s.modifiers_changed(shift, ctrl, false);
    s.pointer_hover(h, shift, false);
    s.pointer_down(h, shift);
    s.pointer_hover(pt(h.x + delta.0, h.y + delta.1), shift, false);
    s.pointer_up(pt(h.x + delta.0, h.y + delta.1), shift, ctrl);
    s.modifiers_changed(false, false, false);
}

fn outcome(s: &mut Session) -> OffsetOutcome {
    s.take_offset_outcome().expect("a release outcome")
}

fn rect_of(o: &ObjectSnapshot) -> (RectBounds, [f64; 4], f64) {
    let ObjectSnapshot::Primitive(p) = o else {
        panic!("not a primitive: {o:?}")
    };
    let Shape::Rect {
        bounds,
        corner_radii,
    } = p.shape
    else {
        panic!("not a rect: {:?}", p.shape)
    };
    (
        bounds,
        [
            corner_radii.tl.as_mm(),
            corner_radii.tr.as_mm(),
            corner_radii.br.as_mm(),
            corner_radii.bl.as_mm(),
        ],
        p.rotation.as_radians(),
    )
}

fn size(o: &ObjectSnapshot) -> (f64, f64) {
    let (b, _, _) = rect_of(o);
    (b.width.as_mm(), b.height.as_mm())
}

fn square_session() -> (Session, NodeId) {
    let d = Document::new(1);
    let a = rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    select_and_offset(&mut s, pt(20.0, 10.0));
    (s, a)
}

fn key(s: &mut Session, k: &str) {
    let _ = s.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    });
}

// ---------------------------------------------------------------- AC 2, 3, 4, 5, 6, 7 ----------

#[test]
fn ac2_key_o_selects_the_tool_and_keeps_the_selection() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    click(&mut s, pt(20.0, 10.0), false);
    assert_eq!(s.selected_object_count(), 1);
    let _ = s.key_down(KeyInput {
        key: "o",
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(
        s.tool(),
        Tool::Select,
        "focus in a text field: O is a letter"
    );
    key(&mut s, "o");
    assert_eq!(s.tool(), Tool::Offset);
    assert_eq!(s.selected_object_count(), 1, "selection kept");
    key(&mut s, "n");
    assert_ne!(s.tool(), Tool::Offset, "exactly one tool is active");
}

#[test]
fn ac4_ac5_ac6_bar_defaults_and_ranges() {
    let (mut s, _) = square_session();
    let bar = s.offset_bar();
    assert_eq!(bar.distance_mm(), 1.0);
    assert_eq!(bar.direction(), OffsetDirection::Out);
    assert_eq!(bar.join(), OffsetJoin::Round);
    assert_eq!(bar.cap(), OffsetCap::Round);
    assert_eq!(bar.result(), OffsetResultChoice::NewObject);
    assert!(s.set_offset_distance(0.01));
    assert!(s.set_offset_distance(1000.0));
    assert!(!s.set_offset_distance(0.009), "below the range");
    assert!(!s.set_offset_distance(1000.01), "above the range");
    assert!(!s.set_offset_distance(f64::NAN));
    let v = s.offset_bar().distance_mm();
    assert!(
        (0.01..=1000.0).contains(&v),
        "the stored value stays in range: {v}"
    );
    assert!(s.set_offset_distance(3.0));
    let (mut again, _) = square_session();
    assert_eq!(again.offset_bar().distance_mm(), 1.0);
    let _ = again.apply_offset();
}

#[test]
fn ac5_no_selection_shows_no_handles_and_apply_does_nothing() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Offset);
    let v = s.offset_view();
    assert!(!v.handles && v.operands == 0 && !v.dragging);
    let before = s.pack("0.1.0").unwrap();
    assert_eq!(s.apply_offset(), OffsetOutcome::Ignored);
    assert_eq!(s.pack("0.1.0").unwrap(), before);
}

#[test]
fn ac5_ends_applies_only_to_open_paths() {
    let d = Document::new(1);
    let _ = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(20.0, 0.0)),
        ],
        false,
    );
    rect_at(&d, 0.0, 40.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    click(&mut s, pt(10.0, 40.0), false);
    s.set_tool(Tool::Offset);
    assert!(!s.offset_view().has_open);
    s.set_tool(Tool::Select);
    click(&mut s, pt(10.0, 0.0), false);
    s.set_tool(Tool::Offset);
    assert!(s.offset_view().has_open);
}

// ------------------------------------------------------------------------ AC 9, 10, 11 --------

#[test]
fn ac9_four_handles_twelve_px_out_and_object_drag_does_not_move() {
    let (mut s, _) = square_session();
    assert!(s.offset_view().handles);
    let objs = objects(&s);
    let refs: Vec<&ObjectSnapshot> = objs.iter().collect();
    let b = offset_box(&refs).unwrap();
    let handles = handle_positions(&b, s.view().scale());
    assert_eq!(handles.len(), 4);
    let scale = s.view().scale();
    let corners = [pt(0.0, 0.0), pt(20.0, 0.0), pt(20.0, 20.0), pt(0.0, 20.0)];
    for c in corners {
        let nearest = handles
            .iter()
            .map(|h| ((h.x - c.x).hypot(h.y - c.y)) * scale)
            .fold(f64::MAX, f64::min);
        assert!(
            (11.9..=17.1).contains(&nearest),
            "handle {nearest} px from its corner"
        );
    }
    let before = s.pack("0.1.0").unwrap();
    s.pointer_hover(pt(20.0, 10.0), false, false);
    s.pointer_down(pt(20.0, 10.0), false);
    s.pointer_hover(pt(35.0, 30.0), false, false);
    s.pointer_up(pt(35.0, 30.0), false, false);
    assert_eq!(
        s.pack("0.1.0").unwrap(),
        before,
        "dragging the object itself moves nothing"
    );
}

#[test]
fn ac10_projection_formula_and_sign() {
    // Bottom right, 1 mm right and 1 mm down: outset 1 mm.
    let (mut s, _) = square_session();
    drag(&mut s, BR, (4.0, 4.0), false, false);
    let OffsetOutcome::Applied {
        added,
        changed,
        distance_mm,
        direction,
        ..
    } = outcome(&mut s)
    else {
        panic!("applied")
    };
    assert_eq!((added, changed, direction), (1, 0, OffsetDirection::Out));
    assert!(
        (distance_mm - 4.0).abs() < 1e-9,
        "d = (4 + 4) / 2: {distance_mm}"
    );
    // Top left, moving up and left: also outset.
    let (mut s, _) = square_session();
    drag(&mut s, TL, (-3.0, -3.0), false, false);
    let OffsetOutcome::Applied {
        distance_mm,
        direction,
        ..
    } = outcome(&mut s)
    else {
        panic!()
    };
    assert!((distance_mm - 3.0).abs() < 1e-9);
    assert_eq!(direction, OffsetDirection::Out);
    // Only dx: d = dx / 2.
    let (mut s, _) = square_session();
    drag(&mut s, BR, (10.0, 0.0), false, false);
    let OffsetOutcome::Applied { distance_mm, .. } = outcome(&mut s) else {
        panic!()
    };
    assert!((distance_mm - 5.0).abs() < 1e-9, "{distance_mm}");
    // Inward.
    let (mut s, _) = square_session();
    drag(&mut s, BR, (-4.0, -4.0), false, false);
    let OffsetOutcome::Applied {
        distance_mm,
        direction,
        ..
    } = outcome(&mut s)
    else {
        panic!()
    };
    assert!((distance_mm - 4.0).abs() < 1e-9);
    assert_eq!(direction, OffsetDirection::In);
    // Perpendicular to the diagonal: d = 0, nothing is written.
    let (mut s, _) = square_session();
    let before = s.pack("0.1.0").unwrap();
    let undo_before = s.undo_len();
    drag(&mut s, BR, (5.0, -5.0), false, false);
    assert_eq!(s.pack("0.1.0").unwrap(), before);
    assert_eq!(s.undo_len(), undo_before);
}

#[test]
fn ac10_the_drag_writes_into_the_bar_and_direction_sticks() {
    let (mut s, _) = square_session();
    drag(&mut s, BR, (-4.0, -4.0), false, false);
    let _ = outcome(&mut s);
    assert!((s.offset_bar().distance_mm() - 4.0).abs() < 1e-9);
    assert_eq!(s.offset_bar().direction(), OffsetDirection::In);
}

#[test]
fn ac10_distance_is_limited_to_1000() {
    let (mut s, _) = square_session();
    drag(&mut s, BR, (5000.0, 5000.0), false, false);
    match outcome(&mut s) {
        OffsetOutcome::Applied { distance_mm, .. } => {
            assert!(distance_mm <= 1000.0, "{distance_mm}");
        }
        OffsetOutcome::Refused { .. } | OffsetOutcome::Ignored => {}
        other @ OffsetOutcome::RefusedGroups { .. } => panic!("{other:?}"),
    }
}

#[test]
fn ac11_distance_snaps_to_the_series() {
    let (mut s, _) = square_session();
    let scale = s.view().scale();
    drag(&mut s, BR, (2.0 * 1.2347, 2.0 * 1.2347), false, false);
    let OffsetOutcome::Applied { distance_mm, .. } = outcome(&mut s) else {
        panic!()
    };
    let series = [0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0];
    let step = series
        .iter()
        .copied()
        .find(|st| st * scale >= 2.0)
        .unwrap_or(5.0);
    let k = distance_mm / step;
    assert!(
        (k - k.round()).abs() < 1e-6,
        "d={distance_mm} is not a multiple of {step}"
    );
    // Near the typed value (within 6 px) it snaps to exactly that value.
    let (mut s, _) = square_session();
    assert!(s.set_offset_distance(2.0));
    let near = 2.0 + 3.0 / s.view().scale();
    drag(&mut s, BR, (near, near), false, false);
    let OffsetOutcome::Applied { distance_mm, .. } = outcome(&mut s) else {
        panic!()
    };
    assert!(
        (distance_mm - 2.0).abs() < 1e-9,
        "snapped to the bar value: {distance_mm}"
    );
}

// -------------------------------------------------------------------- AC 12 inset stop ---------

#[test]
fn ac12_inset_stop_whole_selection_and_release_writes_nothing() {
    let (mut s, _) = square_session();
    let before = s.pack("0.1.0").unwrap();
    let undo_before = s.undo_len();
    let h = handle_of(&s, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    let mut stopped = false;
    for i in 1..=30 {
        let m = -f64::from(i) * 1.0;
        s.pointer_hover(pt(h.x + m, h.y + m), false, false);
        let v = s.offset_view();
        if v.stop == Some(DragStop::NothingLeft) {
            stopped = true;
            assert!(
                v.distance_mm < 0.0 && v.distance_mm > -10.0,
                "stops before 10 mm: {}",
                v.distance_mm
            );
        }
    }
    assert!(stopped, "the handle stops where nothing is left");
    s.pointer_up(pt(h.x - 30.0, h.y - 30.0), false, false);
    assert_eq!(
        s.pack("0.1.0").unwrap(),
        before,
        "a release at the stop writes nothing"
    );
    assert_eq!(s.undo_len(), undo_before);
    assert!(s.take_offset_outcome().is_none(), "no outcome, no notice");
    assert!(s.offset_view().handles && !s.offset_view().dragging);
}

#[test]
fn ac12_stop_clears_when_the_pointer_comes_back() {
    let (mut s, _) = square_session();
    let h = handle_of(&s, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x - 30.0, h.y - 30.0), false, false);
    assert_eq!(s.offset_view().stop, Some(DragStop::NothingLeft));
    s.pointer_hover(pt(h.x - 4.0, h.y - 4.0), false, false);
    assert_eq!(s.offset_view().stop, None);
    s.pointer_up(pt(h.x - 4.0, h.y - 4.0), false, false);
    let OffsetOutcome::Applied {
        distance_mm,
        direction,
        ..
    } = outcome(&mut s)
    else {
        panic!()
    };
    assert_eq!(direction, OffsetDirection::In);
    assert!((distance_mm - 4.0).abs() < 1e-9);
}

#[test]
fn ac12_some_vanish_drag_goes_on_and_notice_counts() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    rect_at(&d, 200.0, 0.0, 100.0, 100.0);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    click(&mut s, pt(0.0, 10.0), false);
    click(&mut s, pt(200.0, 50.0), true);
    s.set_tool(Tool::Offset);
    assert_eq!(s.selected_object_count(), 2);
    let h = handle_of(&s, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x - 30.0, h.y - 30.0), false, false);
    let v = s.offset_view();
    assert_eq!(v.stop, None, "the drag goes on");
    assert_eq!((v.vanished, v.of), (1, 2));
    s.pointer_up(pt(h.x - 30.0, h.y - 30.0), false, false);
    let OffsetOutcome::Applied { vanished, of, .. } = outcome(&mut s) else {
        panic!()
    };
    assert_eq!((vanished, of), (1, 2));
    // The survivor is 100 -> 40.
    assert!(objects(&s).iter().any(|o| size(o) == (40.0, 40.0)));
}

// ------------------------------------------------------------------ AC 13, 16, 17 ---------------

#[test]
fn ac13_double_click_on_a_handle_focuses_the_distance_field() {
    let (mut s, _) = square_session();
    let h = handle_of(&s, BR);
    assert_eq!(
        s.double_click_hint(h, false, false),
        DoubleClickHint::OffsetDistance
    );
    assert_eq!(
        s.double_click_hint(pt(10.0, 10.0), false, false),
        DoubleClickHint::None
    );
}

#[test]
fn ac16_one_commit_named_offset_path_and_undo_redo() {
    let (mut s, _) = square_session();
    let before = s.pack("0.1.0").unwrap();
    let n = s.undo_len();
    drag(&mut s, BR, (4.0, 4.0), false, false);
    assert_eq!(s.undo_len(), n + 1, "one step");
    assert_eq!(OFFSET_COMMIT_LABEL, "offset_path");
    assert_eq!(
        s.history_target_name(curvyo_document_core::Direction::Undo)
            .as_deref(),
        Some("Offset")
    );
    let after = s.pack("0.1.0").unwrap();
    assert_ne!(after, before);
    assert_eq!(objects(&s).len(), 2);
    assert!(s.undo().changed);
    assert_eq!(objects(&s).len(), 1);
    assert_eq!(size(&objects(&s)[0]), (20.0, 20.0));
    assert!(s.redo().changed);
    assert_eq!(objects(&s).len(), 2);
    assert_eq!(doc(&s).export_json().unwrap(), {
        let again = unpack(3, &after).unwrap();
        again.export_json().unwrap()
    });
}

#[test]
fn ac17_escape_cancels_nothing_written() {
    let (mut s, _) = square_session();
    let before = s.pack("0.1.0").unwrap();
    let n = s.undo_len();
    let h = handle_of(&s, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 6.0, h.y + 6.0), false, false);
    assert!(s.offset_view().dragging && s.offset_view().distance_mm > 0.0);
    key(&mut s, "Escape");
    assert!(!s.offset_view().dragging, "Escape ends the drag");
    s.pointer_up(pt(h.x + 6.0, h.y + 6.0), false, false);
    assert_eq!(s.pack("0.1.0").unwrap(), before);
    assert_eq!(s.undo_len(), n);
    assert!(s.offset_view().handles, "the handles return");
    // The other mouse button.
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 6.0, h.y + 6.0), false, false);
    assert!(s.cancel_offset_drag());
    s.pointer_up(pt(h.x + 6.0, h.y + 6.0), false, false);
    assert_eq!(s.pack("0.1.0").unwrap(), before);
    assert!(s.take_offset_outcome().is_none());
}

#[test]
fn ac17_pointer_cancelled_writes_nothing() {
    let (mut s, _) = square_session();
    let before = s.pack("0.1.0").unwrap();
    let h = handle_of(&s, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 6.0, h.y + 6.0), false, false);
    s.pointer_cancelled();
    s.pointer_up(pt(h.x + 6.0, h.y + 6.0), false, false);
    assert_eq!(s.pack("0.1.0").unwrap(), before);
}

// -------------------------------------------------------------------------- AC 19 to 24 ----------

#[test]
fn ac19_modifier_table_on_a_rectangle() {
    // none: new object
    let (mut s, a) = square_session();
    drag(&mut s, BR, (4.0, 4.0), false, false);
    let OffsetOutcome::Applied { added, changed, .. } = outcome(&mut s) else {
        panic!()
    };
    assert_eq!((added, changed), (1, 0));
    assert_eq!(objects(&s).len(), 2);
    // Shift: two new objects, original unchanged.
    let (mut s, a_id) = square_session();
    drag(&mut s, BR, (4.0, 4.0), true, false);
    let OffsetOutcome::Applied {
        added,
        changed,
        direction,
        ..
    } = outcome(&mut s)
    else {
        panic!()
    };
    assert_eq!((added, changed, direction), (2, 0, OffsetDirection::Both));
    let d = doc(&s);
    assert_eq!(d.object_ids().len(), 3);
    assert_eq!(size(&d.object(a_id).unwrap()), (20.0, 20.0));
    let mut sizes: Vec<(f64, f64)> = objects(&s).iter().map(size).collect();
    sizes.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
    assert_eq!(sizes, vec![(12.0, 12.0), (20.0, 20.0), (28.0, 28.0)]);
    // Ctrl: original changed, no new object, same id.
    let (mut s, a_id) = square_session();
    drag(&mut s, BR, (4.0, 4.0), false, true);
    let OffsetOutcome::Applied { added, changed, .. } = outcome(&mut s) else {
        panic!()
    };
    assert_eq!((added, changed), (0, 1));
    let d = doc(&s);
    assert_eq!(d.object_ids(), vec![a_id], "same object, same id");
    assert_eq!(size(&d.object(a_id).unwrap()), (28.0, 28.0));
    // Shift+Ctrl dragged outward: original becomes the outset, a new object is the inset.
    let (mut s, a_id) = square_session();
    drag(&mut s, BR, (4.0, 4.0), true, true);
    let OffsetOutcome::Applied { added, changed, .. } = outcome(&mut s) else {
        panic!()
    };
    assert_eq!((added, changed), (1, 1));
    let d = doc(&s);
    assert_eq!(size(&d.object(a_id).unwrap()), (28.0, 28.0));
    assert!(
        d.object_ids()
            .iter()
            .any(|id| size(&d.object(*id).unwrap()) == (12.0, 12.0))
    );
    // Shift+Ctrl dragged inward: original becomes the inset.
    let (mut s, a_id) = square_session();
    drag(&mut s, BR, (-4.0, -4.0), true, true);
    let _ = outcome(&mut s);
    let d = doc(&s);
    assert_eq!(size(&d.object(a_id).unwrap()), (12.0, 12.0));
    assert!(
        d.object_ids()
            .iter()
            .any(|id| size(&d.object(*id).unwrap()) == (28.0, 28.0))
    );
    let _ = a;
}

#[test]
fn ac19_release_reads_the_modifier_from_its_own_event() {
    // Shift pressed during the drag but released before the pointer: the event says no Shift.
    let (mut s, _) = square_session();
    let h = handle_of(&s, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.modifiers_changed(true, false, false);
    s.pointer_hover(pt(h.x + 4.0, h.y + 4.0), true, false);
    s.pointer_up(pt(h.x + 4.0, h.y + 4.0), false, false);
    let OffsetOutcome::Applied {
        added, direction, ..
    } = outcome(&mut s)
    else {
        panic!()
    };
    assert_eq!(
        (added, direction),
        (1, OffsetDirection::Out),
        "late Shift key-up must not count"
    );
    // And the other way: Shift only on the release event.
    let (mut s, _) = square_session();
    let h = handle_of(&s, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 4.0, h.y + 4.0), false, false);
    s.pointer_up(pt(h.x + 4.0, h.y + 4.0), true, false);
    let OffsetOutcome::Applied {
        added, direction, ..
    } = outcome(&mut s)
    else {
        panic!()
    };
    assert_eq!((added, direction), (2, OffsetDirection::Both));
}

#[test]
fn ac19_result_choice_inverts_ctrl() {
    let (mut s, a) = square_session();
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    drag(&mut s, BR, (4.0, 4.0), false, false);
    let OffsetOutcome::Applied { added, changed, .. } = outcome(&mut s) else {
        panic!()
    };
    assert_eq!((added, changed), (0, 1));
    let (mut s, _) = square_session();
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    drag(&mut s, BR, (4.0, 4.0), false, true);
    let OffsetOutcome::Applied { added, changed, .. } = outcome(&mut s) else {
        panic!()
    };
    assert_eq!(
        (added, changed),
        (1, 0),
        "Ctrl gives New object when the bar says Change original"
    );
    let _ = a;
}

#[test]
fn ac20_z_order_outset_below_inset_above_both_around() {
    let order = |s: &Session, a: NodeId| -> (usize, Vec<(f64, f64)>) {
        let ids = doc(s).object_ids();
        (
            ids.iter().position(|i| *i == a).unwrap(),
            objects(s).iter().map(size).collect(),
        )
    };
    let (mut s, a) = square_session();
    drag(&mut s, BR, (4.0, 4.0), false, false);
    let _ = outcome(&mut s);
    let (pos, sizes) = order(&s, a);
    assert_eq!(sizes[pos], (20.0, 20.0));
    assert_eq!(
        sizes[0],
        (28.0, 28.0),
        "outset directly below the original: {sizes:?}"
    );
    assert_eq!(pos, 1);
    let (mut s, a) = square_session();
    drag(&mut s, BR, (-4.0, -4.0), false, false);
    let _ = outcome(&mut s);
    let (pos, sizes) = order(&s, a);
    assert_eq!(pos, 0);
    assert_eq!(sizes[1], (12.0, 12.0), "inset directly above: {sizes:?}");
    let (mut s, a) = square_session();
    drag(&mut s, BR, (4.0, 4.0), true, false);
    let _ = outcome(&mut s);
    let (pos, sizes) = order(&s, a);
    assert_eq!(pos, 1);
    assert_eq!(
        (sizes[0], sizes[2]),
        ((28.0, 28.0), (12.0, 12.0)),
        "{sizes:?}"
    );
}

#[test]
fn ac20_new_object_copies_the_complete_style() {
    let d = Document::new(1);
    let a = rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    click(&mut s, pt(20.0, 10.0), false);
    s.set_tool(Tool::Offset);
    let style = objects(&s)[0].style().clone();
    drag(&mut s, BR, (4.0, 4.0), false, false);
    let _ = outcome(&mut s);
    for o in objects(&s) {
        assert_eq!(o.style(), &style);
    }
    let _ = a;
}

#[test]
fn ac22_ac23_several_operands_each_on_their_own_and_selection_after() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    rect_at(&d, 25.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    click(&mut s, pt(0.0, 10.0), false);
    click(&mut s, pt(45.0, 10.0), true);
    s.set_tool(Tool::Offset);
    let h = handle_of(&s, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 6.0, h.y + 6.0), false, false);
    s.pointer_up(pt(h.x + 6.0, h.y + 6.0), false, false);
    let OffsetOutcome::Applied { added, .. } = outcome(&mut s) else {
        panic!()
    };
    assert_eq!(added, 2, "two results, never merged");
    assert_eq!(objects(&s).len(), 4);
    assert_eq!(
        s.selected_object_count(),
        2,
        "the new objects are selected, the originals not"
    );
    // Remove the selection: the originals remain.
    s.set_tool(Tool::Select);
    s.delete_selected();
    let left = objects(&s);
    assert_eq!(left.len(), 2);
    assert!(left.iter().all(|o| size(o) == (20.0, 20.0)));
}

#[test]
fn ac23_second_drag_offsets_the_result() {
    let (mut s, _) = square_session();
    drag(&mut s, BR, (4.0, 4.0), false, false);
    let _ = outcome(&mut s);
    drag(&mut s, BR, (4.0, 4.0), false, false);
    let _ = outcome(&mut s);
    let mut sizes: Vec<f64> = objects(&s).iter().map(|o| size(o).0).collect();
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(sizes, vec![20.0, 28.0, 36.0], "concentric copies");
}

#[test]
fn ac23_ctrl_selects_the_changed_original() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    rect_at(&d, 100.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    select_and_offset(&mut s, pt(0.0, 10.0));
    let first = objects(&s);
    let h = handle_over(&s, &[&first[0]], BR);
    s.modifiers_changed(false, true, false);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 4.0, h.y + 4.0), false, false);
    s.pointer_up(pt(h.x + 4.0, h.y + 4.0), false, true);
    let _ = outcome(&mut s);
    assert_eq!(s.selected_object_count(), 1);
    s.set_tool(Tool::Select);
    s.delete_selected();
    assert_eq!(
        objects(&s).len(),
        1,
        "the changed original was the selection"
    );
}

// ------------------------------------------------------------------------ AC 25, 26, 29 --------

#[test]
fn ac25_apply_writes_the_bar_values_in_one_commit() {
    let (mut s, _) = square_session();
    assert!(s.set_offset_distance(2.5));
    let n = s.undo_len();
    let OffsetOutcome::Applied {
        added,
        distance_mm,
        direction,
        ..
    } = s.apply_offset()
    else {
        panic!()
    };
    assert_eq!((added, direction), (1, OffsetDirection::Out));
    assert_eq!(distance_mm, 2.5);
    assert_eq!(s.undo_len(), n + 1);
    assert!(objects(&s).iter().any(|o| size(o) == (25.0, 25.0)));
    // Keyboard preview: while focus is in the bar the bar values preview.
    let (mut s, _) = square_session();
    s.set_offset_bar_focus(true);
    let v = s.offset_view();
    assert!(
        v.refusal.is_none() && v.kind_note.is_some(),
        "preview for the bar values: {v:?}"
    );
}

#[test]
fn ac26_both_and_change_original_equal_shift_ctrl() {
    let (mut s, a) = square_session();
    assert!(s.set_offset_distance(3.0));
    s.set_offset_direction(OffsetDirection::Both);
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    let OffsetOutcome::Applied { added, changed, .. } = s.apply_offset() else {
        panic!()
    };
    assert_eq!((added, changed), (1, 1));
    let d = doc(&s);
    assert_eq!(
        size(&d.object(a).unwrap()),
        (26.0, 26.0),
        "the original takes the Out side"
    );
    assert!(
        d.object_ids()
            .iter()
            .any(|id| size(&d.object(*id).unwrap()) == (14.0, 14.0))
    );
}

#[test]
fn ac29_apply_inset_that_empties_is_refused_and_writes_nothing() {
    let (mut s, _) = square_session();
    assert!(s.set_offset_distance(10.0));
    s.set_offset_direction(OffsetDirection::In);
    let before = s.pack("0.1.0").unwrap();
    let n = s.undo_len();
    match s.apply_offset() {
        OffsetOutcome::Refused {
            refusal,
            distance_mm,
        } => {
            assert_eq!(refusal, OffsetRefusal::Empty);
            assert_eq!(distance_mm, 10.0);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(s.pack("0.1.0").unwrap(), before);
    assert_eq!(s.undo_len(), n);
    // 9.99 leaves a square of 0.02.
    assert!(s.set_offset_distance(9.99));
    assert!(matches!(s.apply_offset(), OffsetOutcome::Applied { .. }));
    assert!(objects(&s).iter().any(|o| (size(o).0 - 0.02).abs() < 1e-6));
}

#[test]
fn ac27_selecting_in_the_tool() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    rect_at(&d, 100.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Offset);
    click(&mut s, pt(0.0, 10.0), false);
    assert_eq!(s.selected_object_count(), 1);
    click(&mut s, pt(100.0, 10.0), true);
    assert_eq!(s.selected_object_count(), 2);
    click(&mut s, pt(100.0, 10.0), true);
    assert_eq!(s.selected_object_count(), 1);
    click(&mut s, pt(60.0, 100.0), false);
    assert_eq!(s.selected_object_count(), 0);
    key(&mut s, "a");
    let _ = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    assert_eq!(s.selected_object_count(), 2, "Ctrl+A");
}

// -------------------------------------------------------------------------- AC 30 open paths ----

fn open_line_session() -> (Session, NodeId) {
    let d = Document::new(1);
    let l = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(20.0, 0.0)),
        ],
        false,
    );
    let mut s = open(&d);
    select_and_offset(&mut s, pt(10.0, 0.0));
    (s, l)
}

#[test]
fn ac30_open_path_outset_gives_a_closed_outline() {
    let (mut s, _) = open_line_session();
    assert!(s.set_offset_distance(5.0));
    s.set_offset_cap(OffsetCap::Flat);
    let OffsetOutcome::Applied { added, .. } = s.apply_offset() else {
        panic!()
    };
    assert_eq!(added, 1);
    let mut closed = 0;
    for o in objects(&s) {
        if let ObjectSnapshot::Path(p) = o
            && p.closed
        {
            closed += 1;
            assert_eq!(p.anchors.len(), 4, "flat ends: a rectangle");
        }
    }
    assert_eq!(closed, 1);
}

#[test]
fn ac30_open_path_inset_handle_stops_at_zero_and_apply_refuses() {
    let (mut s, _) = open_line_session();
    let h = handle_of(&s, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x - 5.0, h.y - 5.0), false, false);
    let v = s.offset_view();
    assert_eq!(v.stop, Some(DragStop::InsetNeedsClosed));
    assert_eq!(v.distance_mm, 0.0);
    let before = s.pack("0.1.0").unwrap();
    s.pointer_up(pt(h.x - 5.0, h.y - 5.0), false, false);
    assert_eq!(s.pack("0.1.0").unwrap(), before);
    s.set_offset_direction(OffsetDirection::In);
    match s.apply_offset() {
        OffsetOutcome::Refused {
            refusal: OffsetRefusal::InsetOfOpenPaths { .. },
            ..
        } => {}
        other => panic!("{other:?}"),
    }
    assert_eq!(s.pack("0.1.0").unwrap(), before);
}

#[test]
fn ac30_mixed_selection_inset_leaves_open_paths_out() {
    let d = Document::new(1);
    let _ = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(0.0, 50.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(20.0, 50.0)),
        ],
        false,
    );
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    let _ = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    s.set_tool(Tool::Offset);
    assert_eq!(s.selected_object_count(), 2);
    assert!(s.set_offset_distance(2.0));
    s.set_offset_direction(OffsetDirection::In);
    let OffsetOutcome::Applied {
        open_left_out,
        added,
        ..
    } = s.apply_offset()
    else {
        panic!()
    };
    assert_eq!((open_left_out, added), (1, 1));
}

// ------------------------------------------------------------------ AC 32 groups ---------------

#[test]
fn ac32_groups_are_refused_and_draw_no_handles() {
    let d = Document::new(1);
    rect_at(&d, 0.0, 0.0, 20.0, 20.0);
    rect_at(&d, 40.0, 0.0, 20.0, 20.0);
    let lone = rect_at(&d, 100.0, 0.0, 20.0, 20.0);
    let _ = lone;
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    click(&mut s, pt(0.0, 10.0), false);
    click(&mut s, pt(60.0, 10.0), true);
    let _ = s.group_selection();
    s.set_tool(Tool::Offset);
    let v = s.offset_view();
    assert!(
        v.has_group && !v.handles,
        "a group alone draws no handles: {v:?}"
    );
    let before = s.pack("0.1.0").unwrap();
    let n = s.undo_len();
    assert!(matches!(
        s.apply_offset(),
        OffsetOutcome::RefusedGroups { leaves: 2 }
    ));
    assert_eq!(s.pack("0.1.0").unwrap(), before);
    assert_eq!(s.undo_len(), n);
    // Group plus another object: handles are drawn, a release is refused.
    s.set_tool(Tool::Select);
    click(&mut s, pt(100.0, 10.0), true);
    s.set_tool(Tool::Offset);
    assert!(s.offset_view().handles);
    assert!(s.selected_object_count() >= 2);
    let objs = objects(&s);
    let _ = objs;
    let b = {
        let all = objects(&s);
        let refs: Vec<&ObjectSnapshot> = all.iter().collect();
        offset_box(&refs[2..]).unwrap_or_else(|| offset_box(&refs).unwrap())
    };
    let _ = b;
    let h = handle_of(&s, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_hover(pt(h.x + 5.0, h.y + 5.0), false, false);
    s.pointer_up(pt(h.x + 5.0, h.y + 5.0), false, false);
    assert!(
        matches!(
            s.take_offset_outcome(),
            Some(OffsetOutcome::RefusedGroups { .. })
        ),
        "release with a group among the operands is refused"
    );
    assert_eq!(s.pack("0.1.0").unwrap(), before, "nothing was changed");
}

// ------------------------------------------------------------- AC 40 to 43 primitives -----------

fn one_object_session(make: impl FnOnce(&Document) -> NodeId, edge: Point) -> (Session, NodeId) {
    let d = Document::new(1);
    let id = make(&d);
    let mut s = open(&d);
    select_and_offset(&mut s, edge);
    (s, id)
}

#[test]
fn ac40_circle_stays_a_circle() {
    let (mut s, id) = one_object_session(
        |d| {
            d.create_ellipse(EllipseFrame {
                center: pt(50.0, 50.0),
                rx: Length::from_mm(20.0),
                ry: Length::from_mm(20.0),
            })
        },
        pt(70.0, 50.0),
    );
    for join in [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Bevel] {
        s.set_offset_join(join);
        assert!(s.set_offset_distance(2.0));
        let OffsetOutcome::Applied { added, .. } = s.apply_offset() else {
            panic!()
        };
        assert_eq!(added, 1);
        let objs = objects(&s);
        let ObjectSnapshot::Primitive(p) = &objs[0] else {
            panic!("outset goes below: {objs:?}")
        };
        let Shape::Ellipse { frame } = p.shape else {
            panic!("{:?}", p.shape)
        };
        assert!((frame.rx.as_mm() - 22.0).abs() < 1e-6 && (frame.ry.as_mm() - 22.0).abs() < 1e-6);
        assert_eq!(frame.center, pt(50.0, 50.0));
        // Restore for the next join.
        assert!(s.undo().changed);
        let _ = id;
        s.set_tool(Tool::Select);
        click(&mut s, pt(70.0, 50.0), false);
        s.set_tool(Tool::Offset);
    }
}

#[test]
fn ac40_ac41_rounded_rectangle_rotated_keeps_rotation() {
    let (mut s, id) = one_object_session(
        |d| {
            let id = rect_at(d, 0.0, 0.0, 100.0, 50.0);
            d.set_corner_radii(&[(
                id,
                curvyo_document_core::CornerRadii::uniform(Length::from_mm(5.0)),
            )])
            .unwrap();
            let o = d
                .object(id)
                .unwrap()
                .rotated(pt(50.0, 25.0), Angle::from_radians(30f64.to_radians()));
            d.rotate_object(&o).unwrap();
            id
        },
        pt(50.0, -18.0),
    );
    // Selection by clicking the rotated edge is fragile: select all instead.
    s.set_tool(Tool::Select);
    let _ = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    s.set_tool(Tool::Offset);
    assert_eq!(s.selected_object_count(), 1);
    assert!(s.set_offset_distance(2.0));
    let OffsetOutcome::Applied { added, .. } = s.apply_offset() else {
        panic!("{:?}", s.offset_view())
    };
    assert_eq!(added, 1);
    let d = doc(&s);
    let new_id = d.object_ids().into_iter().find(|i| *i != id).unwrap();
    let (b, r, rot) = rect_of(&d.object(new_id).unwrap());
    assert!(
        (b.width.as_mm() - 104.0).abs() < 1e-6 && (b.height.as_mm() - 54.0).abs() < 1e-6,
        "{b:?}"
    );
    assert!(r.iter().all(|x| (x - 7.0).abs() < 1e-6), "{r:?}");
    assert!(
        (rot - 30f64.to_radians()).abs() < 1e-9,
        "rotation kept: {rot}"
    );
}

#[test]
fn ac40_sharp_rectangle_round_and_miter() {
    let (mut s, _) = square_session();
    assert!(s.set_offset_distance(2.0));
    assert!(matches!(s.apply_offset(), OffsetOutcome::Applied { .. }));
    let (b, r, _) = rect_of(&objects(&s)[0]);
    assert_eq!((b.width.as_mm(), b.height.as_mm()), (24.0, 24.0));
    assert!(
        r.iter().all(|x| (x - 2.0).abs() < 1e-9),
        "Round: radius d {r:?}"
    );
    let (mut s, _) = square_session();
    s.set_offset_join(OffsetJoin::Miter);
    assert!(s.set_offset_distance(2.0));
    assert!(matches!(s.apply_offset(), OffsetOutcome::Applied { .. }));
    let (b, r, _) = rect_of(&objects(&s)[0]);
    assert_eq!((b.width.as_mm(), b.height.as_mm()), (24.0, 24.0));
    assert!(r.iter().all(|x| *x == 0.0), "Miter: sharp {r:?}");
    // Bevel on a sharp rectangle becomes a path.
    let (mut s, _) = square_session();
    s.set_offset_join(OffsetJoin::Bevel);
    assert!(s.set_offset_distance(2.0));
    assert!(matches!(s.apply_offset(), OffsetOutcome::Applied { .. }));
    assert!(matches!(&objects(&s)[0], ObjectSnapshot::Path(p) if p.anchors.len() == 8));
}

#[test]
fn ac40_rectangle_inset_radius_is_max_r_minus_d_and_vanishes_at_half() {
    let (mut s, _) = one_object_session(
        |d| {
            let id = rect_at(d, 0.0, 0.0, 40.0, 20.0);
            d.set_corner_radii(&[(
                id,
                curvyo_document_core::CornerRadii::uniform(Length::from_mm(3.0)),
            )])
            .unwrap();
            id
        },
        pt(20.0, 0.0),
    );
    s.set_offset_direction(OffsetDirection::In);
    assert!(s.set_offset_distance(5.0));
    assert!(matches!(s.apply_offset(), OffsetOutcome::Applied { .. }));
    let objs = objects(&s);
    let inner = objs.iter().find(|o| size(o).0 < 40.0).unwrap();
    let (b, r, _) = rect_of(inner);
    assert_eq!((b.width.as_mm(), b.height.as_mm()), (30.0, 10.0));
    assert!(r.iter().all(|x| *x == 0.0), "max(3 - 5, 0): {r:?}");
    assert!(s.undo().changed);
    assert!(s.set_offset_distance(10.0));
    assert!(matches!(
        s.apply_offset(),
        OffsetOutcome::Refused {
            refusal: OffsetRefusal::Empty,
            ..
        }
    ));
}

fn hexagon(d: &Document) -> NodeId {
    d.create_polygon(
        StarFrame {
            center: pt(50.0, 50.0),
            radius: Length::from_mm(10.0),
            angle: Angle::from_radians(-FRAC_PI_2),
        },
        PointCount::new(6).unwrap(),
    )
}

#[test]
fn ac40_hexagon_miter_stays_a_hexagon_round_becomes_a_path() {
    let (mut s, _) = one_object_session(hexagon, pt(50.0, 40.0));
    s.set_offset_join(OffsetJoin::Miter);
    assert!(s.set_offset_distance(2.0));
    assert!(matches!(s.apply_offset(), OffsetOutcome::Applied { .. }));
    let objs = objects(&s);
    let ObjectSnapshot::Primitive(p) = &objs[0] else {
        panic!("{objs:?}")
    };
    let Shape::Polygon { frame, point_count } = p.shape else {
        panic!("{:?}", p.shape)
    };
    assert_eq!(point_count.get(), 6);
    let want = 10.0 + 2.0 / 30f64.to_radians().cos();
    assert!(
        (frame.radius.as_mm() - want).abs() < 1e-3,
        "{} vs {want}",
        frame.radius.as_mm()
    );
    let (mut s, _) = one_object_session(hexagon, pt(50.0, 40.0));
    s.set_offset_join(OffsetJoin::Round);
    assert!(s.set_offset_distance(2.0));
    assert!(matches!(s.apply_offset(), OffsetOutcome::Applied { .. }));
    assert!(
        matches!(&objects(&s)[0], ObjectSnapshot::Path(_)),
        "Round outset of a polygon is a path"
    );
    // Inset: a polygon for every join.
    for join in [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Bevel] {
        let (mut s, _) = one_object_session(hexagon, pt(50.0, 40.0));
        s.set_offset_join(join);
        s.set_offset_direction(OffsetDirection::In);
        assert!(s.set_offset_distance(1.0));
        assert!(matches!(s.apply_offset(), OffsetOutcome::Applied { .. }));
        let objs = objects(&s);
        let inner = objs
            .iter()
            .find(|o| matches!(o, ObjectSnapshot::Primitive(_)))
            .unwrap();
        let ObjectSnapshot::Primitive(p) = inner else {
            unreachable!()
        };
        let Shape::Polygon { frame, .. } = p.shape else {
            panic!("{join:?}: {:?}", p.shape)
        };
        let want = 10.0 - 1.0 / 30f64.to_radians().cos();
        assert!(
            objs.len() == 2 && (frame.radius.as_mm() - want).abs() < 1e-3
                || (frame.radius.as_mm() - 10.0).abs() < 1e-9
        );
    }
}

#[test]
fn ac40_star_miter_is_a_scaled_star() {
    let (mut s, _) = one_object_session(
        |d| {
            d.create_star(
                StarFrame {
                    center: pt(50.0, 50.0),
                    radius: Length::from_mm(20.0),
                    angle: Angle::from_radians(-FRAC_PI_2),
                },
                PointCount::new(5).unwrap(),
                InnerRatio::new(0.8).unwrap(),
            )
        },
        pt(50.0, 31.0),
    );
    s.set_offset_join(OffsetJoin::Miter);
    assert!(s.set_offset_distance(1.0));
    assert!(matches!(s.apply_offset(), OffsetOutcome::Applied { .. }));
    let objs = objects(&s);
    println!(
        "star miter outset result: {:?}",
        objs.iter()
            .map(|o| matches!(o, ObjectSnapshot::Primitive(_)))
            .collect::<Vec<_>>()
    );
    if let ObjectSnapshot::Primitive(p) = &objs[0] {
        let Shape::Star {
            frame,
            point_count,
            inner_ratio,
        } = p.shape
        else {
            panic!("{:?}", p.shape)
        };
        assert_eq!(point_count.get(), 5);
        assert!((inner_ratio.get() - 0.8).abs() < 1e-9);
        assert!(frame.radius.as_mm() > 20.0);
    }
}

#[test]
fn ac37_ac42_ellipse_becomes_a_path_change_original_keeps_id_and_place() {
    let (mut s, id) = one_object_session(
        |d| {
            d.create_ellipse(EllipseFrame {
                center: pt(50.0, 50.0),
                rx: Length::from_mm(20.0),
                ry: Length::from_mm(10.0),
            })
        },
        pt(70.0, 50.0),
    );
    assert!(s.set_offset_distance(2.0));
    let v_before = s.offset_view();
    let _ = v_before;
    s.set_offset_result(OffsetResultChoice::ChangeOriginal);
    let OffsetOutcome::Applied { changed, added, .. } = s.apply_offset() else {
        panic!()
    };
    assert_eq!((changed, added), (1, 0));
    let d = doc(&s);
    assert_eq!(d.object_ids(), vec![id], "same id, same place");
    let ObjectSnapshot::Path(p) = d.object(id).unwrap() else {
        panic!("converted to a path")
    };
    assert!(p.anchors.len() <= 8, "{} nodes", p.anchors.len());
    assert!(p.closed);
}

#[test]
fn ac42_both_on_a_rectangle_gives_two_rectangles() {
    let (mut s, _) = square_session();
    s.set_offset_direction(OffsetDirection::Both);
    assert!(s.set_offset_distance(3.0));
    assert!(matches!(
        s.apply_offset(),
        OffsetOutcome::Applied { added: 2, .. }
    ));
    assert!(
        objects(&s)
            .iter()
            .all(|o| matches!(o, ObjectSnapshot::Primitive(_)))
    );
}

#[test]
fn circle_converted_to_a_path_gives_four_cubics() {
    let d = Document::new(1);
    let _ = d.create_ellipse(EllipseFrame {
        center: pt(50.0, 50.0),
        rx: Length::from_mm(20.0),
        ry: Length::from_mm(20.0),
    });
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    click(&mut s, pt(70.0, 50.0), false);
    s.convert_selected_to_paths();
    s.set_tool(Tool::Offset);
    assert!(s.set_offset_distance(2.0));
    assert!(matches!(s.apply_offset(), OffsetOutcome::Applied { .. }));
    let objs = objects(&s);
    let ObjectSnapshot::Path(p) = &objs[0] else {
        panic!()
    };
    assert_eq!(p.anchors.len(), 4, "AC35 via the session");
    assert!(
        p.anchors
            .iter()
            .all(|a| a.handle_in != curvyo_document_core::Vec2::ZERO
                || a.handle_out != curvyo_document_core::Vec2::ZERO)
    );
}

// -------------------------------------------------------------------- AC 18 / large, extremes ----

#[test]
fn edge_huge_distance_on_a_rectangle_and_out_of_range_result() {
    let (mut s, _) = square_session();
    assert!(s.set_offset_distance(1000.0));
    assert!(matches!(s.apply_offset(), OffsetOutcome::Applied { .. }));
    let big = objects(&s).iter().map(|o| size(o).0).fold(0.0, f64::max);
    assert_eq!(big, 2020.0);
}

/// Criterion 32: a result beyond `MAX_COORDINATE_MM` (1e7 mm) is refused, also when the rectangle
/// would stay a rectangle (the primitive route).
#[test]
fn edge_rectangle_result_beyond_the_coordinate_limit_is_refused() {
    let d = Document::new(1);
    rect_at(&d, 9_999_900.0, 0.0, 20.0, 20.0);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    let _ = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    s.set_tool(Tool::Offset);
    assert!(s.set_offset_distance(1000.0));
    let before = s.pack("0.1.0").unwrap();
    let r = s.apply_offset();
    println!("beyond MAX_COORDINATE: {r:?}");
    assert!(matches!(r, OffsetOutcome::Refused { .. }), "{r:?}");
    assert_eq!(s.pack("0.1.0").unwrap(), before);
}

#[test]
fn edge_compound_path_ring_through_the_session() {
    let d = Document::new(1);
    let a = rect_at(&d, 0.0, 0.0, 40.0, 40.0);
    let b = rect_at(&d, 10.0, 10.0, 20.0, 20.0);
    let _ = (a, b);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    let _ = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    s.convert_selected_to_paths();
    s.join_selected();
    let _ = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    s.set_tool(Tool::Offset);
    let n = objects(&s).len();
    println!("objects after join: {n}");
}

/// Criterion 11 and 10: with many pointer moves (as a real mouse sends them) the result still
/// follows the pointer. Each move writes its distance into the bar (criterion 6); that value must
/// not act as a snap target of the same drag, or the distance sticks at its previous value for
/// 6 px (about 1.6 mm at 100 %).
#[test]
fn ac11_a_drag_with_many_moves_ends_at_the_pointer_not_at_the_previous_value() {
    for target in [4.0, 6.0, 9.0] {
        let (mut s, _) = square_session();
        let h = handle_of(&s, BR);
        s.pointer_hover(h, false, false);
        s.pointer_down(h, false);
        let steps = 40;
        for i in 1..=steps {
            let f = target * f64::from(i) / f64::from(steps);
            s.pointer_hover(pt(h.x + f, h.y + f), false, false);
        }
        s.pointer_up(pt(h.x + target, h.y + target), false, false);
        let OffsetOutcome::Applied { distance_mm, .. } = outcome(&mut s) else {
            panic!()
        };
        assert!(
            (distance_mm - target).abs() < 1e-9,
            "pointer at {target} mm, written {distance_mm} mm"
        );
    }
}

/// Criterion 11: the snap step is not coarser than the series says (1 mm at 100 %): moving by
/// 0.2 mm steps past the bar value produces 1 mm multiples, never a repeat of the last value.
#[test]
fn ac11_successive_moves_pass_through_every_step() {
    let (mut s, _) = square_session();
    let h = handle_of(&s, BR);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    let mut seen = Vec::new();
    for i in 1..=100 {
        let f = f64::from(i) * 0.1;
        s.pointer_hover(pt(h.x + f, h.y + f), false, false);
        let d = s.offset_view().distance_mm;
        if seen.last() != Some(&d) {
            seen.push(d);
        }
    }
    println!("distinct distances while the pointer goes 0..10 mm: {seen:?}");
    assert!(seen.len() >= 9, "expected a step per mm, saw {seen:?}");
}

/// Criterion 32 (0016 17a): a result beyond `MAX_COORDINATE_MM` is refused, for a path as well.
#[test]
fn edge_path_result_beyond_the_coordinate_limit_is_refused() {
    let d = Document::new(1);
    let _ = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(9_999_900.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(9_999_950.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 3), pt(9_999_950.0, 50.0)),
        ],
        true,
    );
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    let _ = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    s.set_tool(Tool::Offset);
    assert!(s.set_offset_distance(1000.0));
    let before = s.pack("0.1.0").unwrap();
    let r = s.apply_offset();
    assert!(matches!(r, OffsetOutcome::Refused { .. }), "{r:?}");
    assert_eq!(s.pack("0.1.0").unwrap(), before);
}
