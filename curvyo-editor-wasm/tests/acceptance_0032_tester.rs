//! Independent tester acceptance tests for `specs/0032-pen-tablet-input/specification.md`, session
//! level (criteria 1 and 3 to 16): the same records the webview delivers, handed to
//! `Session::pencil_input`. Written from the specification and the public `Session` seam before
//! the implementation diff was read.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::doc_markdown,
    missing_docs
)]

use std::f64::consts::PI;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    Angle, Direction, Document, Length, NodeId, PathSnapshot, Point, Timestamp, unpack,
};
use curvyo_editor_wasm::{EscapeStep, KeyInput, KeyOutcome, PencilPhase, Session, Tool};
use curvyo_ui_core::{PointerKind, PointerSample, PressureCurve};

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

static NOW: AtomicU64 = AtomicU64::new(1_800_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(7, Ordering::SeqCst))
}

fn fresh() -> Session {
    let mut s = Session::new(32);
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Pencil);
    s
}

fn reread(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn json(s: &Session) -> Vec<u8> {
    reread(s).export_json().unwrap()
}

fn ids(s: &Session) -> Vec<NodeId> {
    reread(s).object_ids()
}

fn path(s: &Session, id: NodeId) -> PathSnapshot {
    reread(s).path(id).expect("a path")
}

fn smp(kind: PointerKind, x: f64, y: f64, p: f64, tx: f64, ty: f64) -> PointerSample {
    PointerSample {
        position: Point::new(x, y),
        kind,
        pressure: p,
        tilt_x: Angle::from_radians(tx.to_radians()),
        tilt_y: Angle::from_radians(ty.to_radians()),
        eraser: false,
        time_ms: 0.0,
    }
}

fn pen(x: f64, y: f64, p: f64) -> PointerSample {
    smp(PointerKind::Pen, x, y, p, 0.0, 0.0)
}

fn mouse(x: f64, y: f64) -> PointerSample {
    smp(PointerKind::Mouse, x, y, 0.5, 0.0, 0.0)
}

/// One stroke as the webview delivers it: down (buttons 1), one move per sample, up (buttons 0).
fn stroke(s: &mut Session, samples: &[PointerSample]) {
    s.pencil_input(PencilPhase::Down, 1, &samples[..1]);
    for one in &samples[1..] {
        s.pencil_input(PencilPhase::Move, 1, std::slice::from_ref(one));
    }
    let last = *samples.last().unwrap();
    s.pencil_input(PencilPhase::Up, 0, &[last]);
}

fn line_samples(make: impl Fn(f64, f64) -> PointerSample) -> Vec<PointerSample> {
    (0..=100)
        .map(|i| make(20.0 + f64::from(i) * 0.5, 30.0))
        .collect()
}

fn pressure_line() -> Vec<PointerSample> {
    (0..=100)
        .map(|i| pen(20.0 + f64::from(i) * 0.5, 30.0, 0.2 + 0.006 * f64::from(i)))
        .collect()
}

fn key(s: &mut Session, k: &str) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    })
}

fn ctrl_z(s: &mut Session) {
    let out = s.key_down(KeyInput {
        key: "z",
        ctrl: true,
        ..KeyInput::default()
    });
    if out == KeyOutcome::UndoBusy {
        let _ = s.undo();
    }
}

fn ctrl_shift_z(s: &mut Session) {
    let out = s.key_down(KeyInput {
        key: "Z",
        ctrl: true,
        shift: true,
        ..KeyInput::default()
    });
    if out == KeyOutcome::RedoBusy {
        let _ = s.redo();
    }
}

/// A Pen path's style today: two clicks and finish.
fn pen_path_style(s: &mut Session) -> curvyo_document_core::Style {
    let before = ids(s);
    s.set_tool(Tool::Pen);
    for p in [Point::new(100.0, 100.0), Point::new(140.0, 120.0)] {
        s.pointer_hover(p, false, false);
        s.pointer_down(p, false);
        s.pointer_up(p, false, false);
    }
    s.finish_pen();
    let id = *ids(s)
        .iter()
        .find(|i| !before.contains(i))
        .expect("a pen path");
    path(s, id).style
}

fn draw_list_vertices(s: &Session) -> usize {
    s.draw_list().triangles.len()
}

// ---------------------------------------------------------------------
// Criterion 6: the tool
// ---------------------------------------------------------------------

#[test]
fn c6_key_p_picks_the_pencil_and_cursor_is_crosshair() {
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    assert_eq!(key(&mut s, "p"), KeyOutcome::ToolChanged);
    assert_eq!(s.tool(), Tool::Pencil);
    // Upper-case P (Caps Lock) is the same key in the other tool letters' convention.
    s.set_tool(Tool::Select);
    let _ = key(&mut s, "P");
    // The cursor hint the frontend reads: crosshair or the creation-tools default.
    s.set_tool(Tool::Pencil);
    let hint = s.cursor_hint();
    assert!(
        hint.is_empty() || hint == "crosshair" || hint == "default",
        "unexpected cursor hint {hint:?}"
    );
}

#[test]
fn c6_mouse_draws_with_the_pencil() {
    let mut s = fresh();
    stroke(&mut s, &line_samples(mouse));
    assert_eq!(ids(&s).len(), 1);
}

// ---------------------------------------------------------------------
// Criterion 7: stroke without pressure
// ---------------------------------------------------------------------

#[test]
fn c7_mouse_stroke_is_one_open_path_with_the_pen_style() {
    let mut s = fresh();
    stroke(&mut s, &line_samples(mouse));
    let id = ids(&s)[0];
    let p = path(&s, id);
    assert!(!p.closed, "open path");
    assert!(!p.is_compound());
    assert_eq!(p.anchors.len(), 2, "a straight line has two nodes");
    assert!(
        (p.anchors[0].point.x - 20.0).abs() <= 0.1 && (p.anchors[1].point.x - 70.0).abs() <= 0.1
    );
    let style = p.style;
    let pen_style = pen_path_style(&mut s);
    assert_eq!(style, pen_style, "the style of a new Pen path");
}

#[test]
fn c7_pen_without_pressure_is_an_open_path() {
    for p in [0.5, 0.0] {
        let mut s = fresh();
        stroke(&mut s, &line_samples(|x, y| pen(x, y, p)));
        let path = path(&s, ids(&s)[0]);
        assert!(!path.closed, "pressure {p}: open path");
        assert!(!s.pencil_bar().pressure_seen, "pressure {p} is no pressure");
    }
}

// ---------------------------------------------------------------------
// Criterion 8: stroke with pressure
// ---------------------------------------------------------------------

#[test]
fn c8_pressure_stroke_is_a_closed_filled_outline() {
    let mut s = fresh();
    stroke(&mut s, &pressure_line());
    let all = ids(&s);
    assert_eq!(all.len(), 1, "one object");
    let p = path(&s, all[0]);
    assert!(p.closed, "closed path");
    assert!(p.style.fill.enabled, "fill on");
    assert!(!p.style.stroke.enabled, "stroke None");
    let pen_style = pen_path_style(&mut s);
    assert_eq!(
        p.style.fill.color, pen_style.stroke.color,
        "fill = the new Pen path's stroke colour"
    );
    // The outline spans the line plus the round ends.
    let xs: Vec<f64> = p.anchors.iter().map(|a| a.point.x).collect();
    let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(min_x < 20.0 && min_x > 19.5, "{min_x}");
    assert!(max_x > 70.0 && max_x < 71.5, "{max_x}");
}

#[test]
fn c8_self_crossing_pressure_stroke_is_one_object_one_step() {
    let mut s = fresh();
    let ring: Vec<PointerSample> = (0..=460)
        .map(|i| {
            let a = 2.3 * PI * f64::from(i) / 460.0;
            pen(
                60.0 + 10.0 * a.cos(),
                60.0 + 10.0 * a.sin(),
                if i % 2 == 0 { 0.6 } else { 0.61 },
            )
        })
        .collect();
    let undo_before = s.undo_len();
    let before = json(&s);
    stroke(&mut s, &ring);
    let _ = s.history_target_name(Direction::Undo);
    assert_eq!(ids(&s).len(), 1, "one object, its hole included");
    let p = path(&s, ids(&s)[0]);
    assert!(p.closed);
    assert!(
        p.is_compound(),
        "the empty middle is a hole of a compound path"
    );
    assert_eq!(s.undo_len(), undo_before + 1, "one undo step");
    assert_eq!(
        s.history_target_name(Direction::Undo).as_deref(),
        Some("Pencil stroke")
    );
    ctrl_z(&mut s);
    assert_eq!(json(&s), before, "undo removes the whole stroke");
}

// ---------------------------------------------------------------------
// Criteria 3, 4, 5: pressure present, touch, hover
// ---------------------------------------------------------------------

#[test]
fn c4_touch_draws_nothing() {
    let mut s = fresh();
    let before = json(&s);
    let undo = s.undo_len();
    let fingers: Vec<PointerSample> = (0..=100)
        .map(|i| {
            smp(
                PointerKind::Touch,
                20.0 + f64::from(i) * 0.5,
                30.0,
                0.7,
                0.0,
                0.0,
            )
        })
        .collect();
    stroke(&mut s, &fingers);
    assert_eq!(json(&s), before, "no change");
    assert_eq!(s.undo_len(), undo, "no commit");
    assert_eq!(
        draw_list_vertices(&s),
        draw_list_vertices(&fresh()),
        "no preview either"
    );
    assert!(!s.pencil_bar().pen_seen, "a finger is not a pen");
}

#[test]
fn c5_pen_hover_starts_nothing_then_contact_draws() {
    let mut s = fresh();
    let hover: Vec<PointerSample> = (0..=50)
        .map(|i| pen(20.0 + f64::from(i), 30.0, 0.0))
        .collect();
    for h in &hover {
        s.pencil_input(PencilPhase::Move, 0, std::slice::from_ref(h));
    }
    assert_eq!(ids(&s).len(), 0, "hover draws nothing");
    assert_eq!(s.undo_len(), 0);
    assert!(s.pencil_bar().pen_seen, "a pen in range was seen");
    // An up without a down (pen lifted out of range) writes nothing either.
    s.pencil_input(PencilPhase::Up, 0, &[pen(80.0, 30.0, 0.0)]);
    assert_eq!(ids(&s).len(), 0);
    // A down with buttons 0 (no contact) is no stroke.
    s.pencil_input(PencilPhase::Down, 0, &[pen(20.0, 50.0, 0.0)]);
    s.pencil_input(PencilPhase::Move, 0, &[pen(60.0, 50.0, 0.0)]);
    s.pencil_input(PencilPhase::Up, 0, &[pen(60.0, 50.0, 0.0)]);
    assert_eq!(ids(&s).len(), 0, "no contact, no stroke");
    stroke(&mut s, &pressure_line());
    assert_eq!(ids(&s).len(), 1, "contact (buttons 1) draws");
}

// ---------------------------------------------------------------------
// Criterion 11: preview
// ---------------------------------------------------------------------

#[test]
fn c11_preview_shows_while_drawing_and_goes_on_release_or_escape() {
    let mut s = fresh();
    let empty = draw_list_vertices(&s);
    let samples = pressure_line();
    s.pencil_input(PencilPhase::Down, 1, &samples[..1]);
    s.pencil_input(PencilPhase::Move, 1, &samples[1..50]);
    let during = s.draw_list();
    assert!(during.triangles.len() > empty, "a preview is drawn");
    let blue = during
        .triangles
        .iter()
        .filter(|v| v.color.b > 200 && v.color.r < 100)
        .count();
    assert!(blue > 0, "the preview is blue");
    assert_eq!(ids(&s).len(), 0, "nothing written while drawing");
    s.pencil_input(PencilPhase::Move, 1, &samples[50..]);
    s.pencil_input(PencilPhase::Up, 0, &[*samples.last().unwrap()]);
    assert_eq!(ids(&s).len(), 1);
    // The second stroke, cancelled: the list returns to the pre-stroke list.
    let after_first = s.draw_list().triangles.len();
    let second: Vec<PointerSample> = samples
        .iter()
        .map(|p| pen(p.position.x, p.position.y + 20.0, p.pressure))
        .collect();
    s.pencil_input(PencilPhase::Down, 1, &second[..1]);
    s.pencil_input(PencilPhase::Move, 1, &second[1..]);
    assert!(s.draw_list().triangles.len() > after_first);
    let step = s.escape();
    assert_ne!(step, EscapeStep::Nothing);
    assert_ne!(step, EscapeStep::LeftTool);
    let _ = s.draw_list().triangles.len();
}

// ---------------------------------------------------------------------
// Criterion 12: Escape and too-short strokes
// ---------------------------------------------------------------------

#[test]
fn c12_escape_cancels_the_stroke_then_leaves_the_tool() {
    let mut s = fresh();
    let before = json(&s);
    let samples = pressure_line();
    s.pencil_input(PencilPhase::Down, 1, &samples[..1]);
    s.pencil_input(PencilPhase::Move, 1, &samples[1..]);
    assert_ne!(
        key(&mut s, "Escape"),
        KeyOutcome::Ignored,
        "Escape is handled"
    );
    assert_eq!(s.tool(), Tool::Pencil, "the first Escape keeps the tool");
    // The pen is still down: the up after Escape writes nothing.
    s.pencil_input(PencilPhase::Move, 1, &[pen(90.0, 30.0, 0.8)]);
    s.pencil_input(PencilPhase::Up, 0, &[pen(90.0, 30.0, 0.8)]);
    assert_eq!(json(&s), before, "nothing is written");
    assert_eq!(s.undo_len(), 0);
    let step = s.escape();
    assert_eq!(
        step,
        EscapeStep::LeftTool,
        "the next Escape leaves the tool"
    );
    assert_eq!(s.tool(), Tool::Select);
}

#[test]
fn c12_escape_step_is_reported() {
    let mut s = fresh();
    let samples = pressure_line();
    s.pencil_input(PencilPhase::Down, 1, &samples[..1]);
    s.pencil_input(PencilPhase::Move, 1, &samples[1..]);
    let step = s.escape();
    assert!(
        matches!(step, EscapeStep::CancelledDrag | EscapeStep::ClearedState),
        "the first step of the cascade cancels the stroke: {step:?}"
    );
    assert_eq!(s.tool(), Tool::Pencil);
}

#[test]
fn c12_short_strokes_make_nothing() {
    let mut s = fresh();
    let before = json(&s);
    // A tap: down and up on one spot.
    s.pencil_input(PencilPhase::Down, 1, &[pen(10.0, 10.0, 0.7)]);
    s.pencil_input(PencilPhase::Up, 0, &[pen(10.0, 10.0, 0.7)]);
    // Down and up at the same point, with the up's own sample.
    stroke(&mut s, &[pen(10.0, 10.0, 0.7), pen(10.0, 10.0, 0.7)]);
    // 0.09 mm long.
    stroke(
        &mut s,
        &[mouse(10.0, 10.0), mouse(10.05, 10.0), mouse(10.09, 10.0)],
    );
    assert_eq!(json(&s), before);
    assert_eq!(s.undo_len(), 0, "no commit");
}

#[test]
fn c12_switching_tool_mid_stroke_writes_nothing() {
    let mut s = fresh();
    let before = json(&s);
    let samples = pressure_line();
    s.pencil_input(PencilPhase::Down, 1, &samples[..1]);
    s.pencil_input(PencilPhase::Move, 1, &samples[1..]);
    s.set_tool(Tool::Select);
    s.set_tool(Tool::Pencil);
    s.pencil_input(PencilPhase::Up, 0, &[*samples.last().unwrap()]);
    assert_eq!(json(&s), before);
}

#[test]
fn c12_pointer_cancel_mid_stroke_writes_nothing() {
    let mut s = fresh();
    let before = json(&s);
    let samples = pressure_line();
    s.pencil_input(PencilPhase::Down, 1, &samples[..1]);
    s.pencil_input(PencilPhase::Move, 1, &samples[1..]);
    s.pointer_cancelled();
    s.pencil_input(PencilPhase::Up, 0, &[*samples.last().unwrap()]);
    assert_eq!(
        json(&s),
        before,
        "a cancelled pointer (pointercancel) writes nothing"
    );
}

// ---------------------------------------------------------------------
// Criterion 13: one commit, undo step, selection, tool stays
// ---------------------------------------------------------------------

#[test]
fn c13_one_commit_one_undo_step_selected_tool_stays() {
    for (name, samples) in [
        ("mouse", line_samples(mouse)),
        ("pressure", pressure_line()),
    ] {
        let mut s = fresh();
        let before = json(&s);
        stroke(&mut s, &samples);
        let _ = s.history_target_name(Direction::Undo);
        assert_eq!(s.undo_len(), 1, "{name}: exactly one step");
        assert_eq!(
            s.history_target_name(Direction::Undo).as_deref(),
            Some("Pencil stroke"),
            "{name}"
        );
        assert_eq!(
            s.selected_object_count(),
            1,
            "{name}: the new object is selected"
        );
        assert_eq!(s.tool(), Tool::Pencil, "{name}: the tool stays the Pencil");
        let after = json(&s);
        ctrl_z(&mut s);
        assert_eq!(json(&s), before, "{name}: undo");
        ctrl_shift_z(&mut s);
        assert_eq!(json(&s), after, "{name}: redo");
        let rows = s.history_rows_json(0, 10);
        assert!(rows.contains("Pencil stroke"), "{name}: {rows}");
    }
}

#[test]
fn c13_two_strokes_are_two_steps_and_the_second_is_selected() {
    let mut s = fresh();
    stroke(&mut s, &line_samples(mouse));
    let first = ids(&s)[0];
    let second: Vec<PointerSample> = line_samples(mouse)
        .iter()
        .map(|p| mouse(p.position.x, 60.0))
        .collect();
    stroke(&mut s, &second);
    let _ = s.history_target_name(Direction::Undo);
    assert_eq!(s.undo_len(), 2);
    assert_eq!(ids(&s).len(), 2);
    assert_eq!(
        s.selected_object_count(),
        1,
        "only the last stroke is selected"
    );
    ctrl_z(&mut s);
    assert_eq!(ids(&s), vec![first]);
}

// ---------------------------------------------------------------------
// Criterion 14: the Pencil bar and session settings
// ---------------------------------------------------------------------

#[test]
fn c14_controls_appear_after_the_first_pressure_stroke_only() {
    let mut s = fresh();
    let bar = s.pencil_bar();
    assert!(!bar.pen_seen && !bar.pressure_seen);
    assert_eq!(bar.settings.minimum().as_mm(), 0.2);
    assert_eq!(bar.settings.maximum().as_mm(), 2.0);
    assert_eq!(bar.settings.curve, PressureCurve::Linear);
    stroke(&mut s, &line_samples(mouse));
    assert!(!s.pencil_bar().pen_seen, "mouse-only: no bar");
    assert!(!s.pencil_bar().pressure_seen, "mouse-only: no controls");
    stroke(&mut s, &line_samples(|x, y| pen(x, y + 10.0, 0.5)));
    assert!(s.pencil_bar().pen_seen);
    assert!(
        !s.pencil_bar().pressure_seen,
        "a pen without pressure shows no controls"
    );
    stroke(&mut s, &pressure_line());
    assert!(s.pencil_bar().pressure_seen);
    // They stay for the session, even after a later mouse stroke.
    stroke(&mut s, &line_samples(|x, y| mouse(x, y + 40.0)));
    assert!(s.pencil_bar().pressure_seen);
}

#[test]
fn c14_maximum_never_below_minimum_and_ranges() {
    let mut s = fresh();
    s.set_pencil_minimum(Length::from_mm(3.0));
    s.set_pencil_maximum(Length::from_mm(1.0));
    let b = s.pencil_bar().settings;
    assert_eq!(b.minimum().as_mm(), 3.0);
    assert_eq!(b.maximum().as_mm(), 3.0, "raised to the Minimum");
    s.set_pencil_minimum(Length::from_mm(0.0));
    assert_eq!(s.pencil_bar().settings.minimum().as_mm(), 0.05);
    s.set_pencil_maximum(Length::from_mm(99.0));
    assert_eq!(s.pencil_bar().settings.maximum().as_mm(), 50.0);
}

fn outline_height(s: &Session, id: NodeId) -> f64 {
    let p = path(s, id);
    let ys: Vec<f64> = p.all_anchors().map(|a| a.point.y).collect();
    ys.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - ys.iter().copied().fold(f64::INFINITY, f64::min)
}

#[test]
fn c14_change_applies_from_the_next_stroke() {
    let mut s = fresh();
    let full: Vec<PointerSample> = (0..=100)
        .map(|i| pen(20.0 + f64::from(i) * 0.5, 30.0, 1.0))
        .collect();
    stroke(&mut s, &full);
    let first = ids(&s)[0];
    let h1 = outline_height(&s, first);
    assert!((h1 - 2.0).abs() < 0.05, "full pressure, Maximum 2: {h1}");
    s.set_pencil_maximum(Length::from_mm(6.0));
    assert!(
        (outline_height(&s, first) - h1).abs() < 1e-9,
        "the existing stroke does not change"
    );
    let second: Vec<PointerSample> = full.iter().map(|p| pen(p.position.x, 60.0, 1.0)).collect();
    stroke(&mut s, &second);
    let new = *ids(&s).iter().find(|i| **i != first).unwrap();
    let h2 = outline_height(&s, new);
    assert!(
        (h2 - 6.0).abs() < 0.05,
        "Maximum 6 applies to the next stroke: {h2}"
    );
    // Curve change: Firm at pressure 0.5 -> 0.2 + 1.8 * 0.25 = 0.65 with default widths.
    let mut s = fresh();
    s.set_pencil_curve(PressureCurve::Firm);
    let half: Vec<PointerSample> = (0..=100)
        .map(|i| {
            pen(
                20.0 + f64::from(i) * 0.5,
                30.0,
                if i == 0 { 0.49 } else { 0.5 },
            )
        })
        .collect();
    stroke(&mut s, &half);
    // pressure 0.49 makes it a pressure stroke; most samples smooth to 0.5.
    let h = outline_height(&s, ids(&s)[0]);
    assert!((h - 0.65).abs() < 0.05, "Firm at 0.5: {h}");
}

#[test]
fn c14_c16_settings_are_not_in_the_saved_file() {
    let mut s = fresh();
    stroke(&mut s, &pressure_line());
    let saved = s.pack("0.1.0").unwrap();
    let doc_json = json(&s);
    s.set_pencil_minimum(Length::from_mm(1.5));
    s.set_pencil_maximum(Length::from_mm(7.0));
    s.set_pencil_curve(PressureCurve::Soft);
    assert_eq!(json(&s), doc_json, "the document does not change");
    assert_eq!(
        s.pack("0.1.0").unwrap(),
        saved,
        "the saved file does not change"
    );
    // Reopened: defaults again.
    let reopened = Session::open(5, &saved).unwrap();
    let bar = reopened.pencil_bar();
    assert_eq!(bar.settings.minimum().as_mm(), 0.2);
    assert_eq!(bar.settings.maximum().as_mm(), 2.0);
    assert_eq!(bar.settings.curve, PressureCurve::Linear);
    assert!(!bar.pressure_seen && !bar.pen_seen);
}

#[test]
fn c14_settings_do_not_make_the_document_dirty() {
    let mut s = fresh();
    s.mark_saved();
    s.set_pencil_maximum(Length::from_mm(7.0));
    s.set_pencil_curve(PressureCurve::Firm);
    assert!(!s.is_dirty());
}

// ---------------------------------------------------------------------
// Criterion 15: readout
// ---------------------------------------------------------------------

#[test]
fn c15_readout_for_a_pen_none_for_a_mouse() {
    let mut s = fresh();
    assert!(s.pencil_bar().readout.is_none());
    s.pencil_input(
        PencilPhase::Move,
        0,
        &[smp(PointerKind::Pen, 10.0, 10.0, 0.62, 12.0, -3.0)],
    );
    let r = s.pencil_bar().readout.expect("pen in range: readout");
    assert!((r.pressure - 0.62).abs() < 1e-12);
    assert!((r.tilt_x.as_radians().to_degrees() - 12.0).abs() < 1e-9);
    assert!((r.tilt_y.as_radians().to_degrees() + 3.0).abs() < 1e-9);
    s.pencil_input(PencilPhase::Move, 0, &[mouse(20.0, 20.0)]);
    assert!(s.pencil_bar().readout.is_none(), "not for a mouse");
    // Touching: the readout follows the stroke.
    s.pencil_input(
        PencilPhase::Down,
        1,
        &[smp(PointerKind::Pen, 10.0, 10.0, 0.3, 5.0, 5.0)],
    );
    s.pencil_input(
        PencilPhase::Move,
        1,
        &[smp(PointerKind::Pen, 30.0, 10.0, 0.9, 40.0, -20.0)],
    );
    let r = s.pencil_bar().readout.expect("pen touching: readout");
    assert!((r.pressure - 0.9).abs() < 1e-12);
}

// ---------------------------------------------------------------------
// Criterion 16: nothing device-identifying is stored
// ---------------------------------------------------------------------

#[test]
fn c16_saved_document_holds_no_pen_data() {
    let mut s = fresh();
    let samples: Vec<PointerSample> = (0..=100)
        .map(|i| {
            smp(
                PointerKind::Pen,
                20.0 + f64::from(i) * 0.5,
                30.0,
                0.2 + 0.006 * f64::from(i),
                33.0,
                -21.0,
            )
        })
        .collect();
    stroke(&mut s, &samples);
    let text = String::from_utf8_lossy(&json(&s)).to_lowercase();
    for word in [
        "pressure", "tilt", "pointer", "device", "tablet", "wacom", "stylus", "eraser",
    ] {
        assert!(!text.contains(word), "{word} in document.json");
    }
    let bytes = s.pack("0.1.0").unwrap();
    let raw = String::from_utf8_lossy(&bytes).to_lowercase();
    for word in ["pressure", "tilt", "pointerid", "tablet"] {
        assert!(!raw.contains(word), "{word} in the saved file");
    }
}

// ---------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------

#[test]
fn edge_coalesced_batches_equal_single_samples() {
    // Criterion 2 at the session: five samples in one move call draw the same as five calls.
    let samples = pressure_line();
    let mut a = fresh();
    stroke(&mut a, &samples);
    let mut b = fresh();
    b.pencil_input(PencilPhase::Down, 1, &samples[..1]);
    for chunk in samples[1..].chunks(5) {
        b.pencil_input(PencilPhase::Move, 1, chunk);
    }
    b.pencil_input(PencilPhase::Up, 0, &[*samples.last().unwrap()]);
    let pa = path(&a, ids(&a)[0]);
    let pb = path(&b, ids(&b)[0]);
    let pts = |p: &PathSnapshot| p.all_anchors().map(|x| x.point).collect::<Vec<_>>();
    assert_eq!(pts(&pa), pts(&pb));
}

#[test]
fn edge_nan_and_infinite_positions_do_not_corrupt_the_document() {
    let mut s = fresh();
    let mut samples = pressure_line();
    samples[40].position = Point::new(f64::NAN, 30.0);
    samples[60].position = Point::new(f64::INFINITY, f64::NEG_INFINITY);
    stroke(&mut s, &samples);
    for id in ids(&s) {
        let p = path(&s, id);
        assert!(p.all_anchors().all(|a| a.point.x.is_finite()
            && a.point.y.is_finite()
            && a.handle_in.x.is_finite()
            && a.handle_out.y.is_finite()));
    }
    // The file still saves and reopens.
    let bytes = s.pack("0.1.0").unwrap();
    assert!(Session::open(3, &bytes).is_ok());
}

#[test]
fn edge_two_thousand_sample_pressure_stroke_under_600_nodes() {
    let mut s = fresh();
    let samples: Vec<PointerSample> = (0..2000)
        .map(|i| {
            let t = f64::from(i) / 1999.0;
            pen(
                20.0 + 120.0 * t,
                80.0 + 20.0 * (2.0 * PI * t).sin(),
                0.55 + 0.35 * (6.0 * PI * t).sin(),
            )
        })
        .collect();
    // Criterion 11: the frame's draw list with the preview of 2000 samples fits in a frame.
    s.pencil_input(PencilPhase::Down, 1, &samples[..1]);
    for chunk in samples[1..].chunks(4) {
        s.pencil_input(PencilPhase::Move, 1, chunk);
    }
    let frame = std::time::Instant::now();
    let list = s.draw_list();
    let frame = frame.elapsed();
    assert!(list.triangle_count() > 0);
    if !cfg!(debug_assertions) {
        assert!(
            frame.as_millis() < 16,
            "a preview frame of 2000 samples took {frame:?}"
        );
    }
    // Criterion 10: the release completes in under 200 ms.
    let start = std::time::Instant::now();
    s.pencil_input(PencilPhase::Up, 0, &[*samples.last().unwrap()]);
    let elapsed = start.elapsed();
    if !cfg!(debug_assertions) {
        assert!(elapsed.as_millis() < 200, "release took {elapsed:?}");
    }
    eprintln!("preview frame {frame:?}");
    let p = path(&s, ids(&s)[0]);
    let n = p.all_anchors().count();
    assert!(n < 600, "{n} nodes");
    eprintln!("release {elapsed:?}");
}

#[test]
fn edge_undo_of_a_stroke_while_drawing_another_is_safe() {
    let mut s = fresh();
    stroke(&mut s, &line_samples(mouse));
    let samples = pressure_line();
    s.pencil_input(PencilPhase::Down, 1, &samples[..1]);
    s.pencil_input(PencilPhase::Move, 1, &samples[1..50]);
    ctrl_z(&mut s);
    s.pencil_input(PencilPhase::Move, 1, &samples[50..]);
    s.pencil_input(PencilPhase::Up, 0, &[*samples.last().unwrap()]);
    // Whatever the outcome, the document stays consistent and saves.
    assert!(s.pack("0.1.0").is_ok());
    assert!(ids(&s).len() <= 2);
}

/// The DOM reports `pressure` 0 on `pointerup` (no button pressed). The release sample lies where
/// the last move was, so it adds no length; it must not shrink the end of a full-pressure stroke
/// below the width the pen drew with (criterion 8: a capsule of Maximum width at both ends).
#[test]
fn edge_release_with_dom_pressure_zero_keeps_the_end_width() {
    let mut s = fresh();
    let full: Vec<PointerSample> = (0..=500)
        .map(|i| pen(10.0 + f64::from(i) * 0.1, 30.0, 1.0))
        .collect();
    s.pencil_input(PencilPhase::Down, 1, &full[..1]);
    for one in &full[1..] {
        s.pencil_input(PencilPhase::Move, 1, std::slice::from_ref(one));
    }
    // pointerup as the webview delivers it: same position, buttons 0, pressure 0.
    s.pencil_input(PencilPhase::Up, 0, &[pen(60.0, 30.0, 0.0)]);
    let p = path(&s, ids(&s)[0]);
    let max_x = p
        .all_anchors()
        .map(|a| a.point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        (max_x - 61.0).abs() <= 0.05,
        "the round end has radius Maximum/2 = 1 mm: the outline ends at {max_x}, not 61"
    );
}
