//! Independent tester tests for `specs/0070-measure-and-dimension` through `Session`, driven the
//! way the wasm host drives it (modifiers first, then the pointer event, keys through
//! `key_down`): the Measure tool's letter and gating, the readout, the kinds inferred from picks
//! and the pointer, Shift at the second pick, Alt's two phases, snapping, selection first and
//! Enter, live following, unlinking, the Node tool on kept points, conversion, history names,
//! the live region and the Layers row. Written from the specification and the UX notes; only
//! public names and the overlay JSON shape were looked up.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty
)]
#![allow(
    clippy::too_many_lines,
    missing_docs,
    clippy::doc_markdown,
    clippy::many_single_char_names
)]

use curvyo_document_core::{
    AnchorId, DisplayUnit, Document, EllipseFrame, Length, NewAnchor, NodeId, ObjectSnapshot,
    Point, RectBounds, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, RowPress, Session, Tool};
use serde_json::Value;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn line(d: &Document, n: u64, a: Point, b: Point) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, n), a),
            NewAnchor::corner(AnchorId::new(1, n + 1), b),
        ],
        false,
    )
}

fn circle(d: &Document, c: Point, r: f64) -> NodeId {
    d.create_ellipse(EllipseFrame {
        center: c,
        rx: Length::from_mm(r),
        ry: Length::from_mm(r),
    })
}

fn open(d: &Document, snapping: bool) -> Session {
    let mut s = Session::open(2, &pack(d, "t").unwrap()).unwrap();
    let mut sn = s.snapping();
    if sn.enabled != snapping {
        sn.enabled = snapping;
        s.set_snapping(sn);
    }
    s
}

#[derive(Clone, Copy, Default)]
struct M {
    shift: bool,
    ctrl: bool,
    alt: bool,
}
const NONE: M = M {
    shift: false,
    ctrl: false,
    alt: false,
};
const SHIFT: M = M {
    shift: true,
    ctrl: false,
    alt: false,
};
const ALT: M = M {
    shift: false,
    ctrl: false,
    alt: true,
};
const CTRL: M = M {
    shift: false,
    ctrl: true,
    alt: false,
};

fn hover(s: &mut Session, p: Point, m: M) {
    s.modifiers_changed(m.shift, m.ctrl, m.alt);
    s.pointer_hover(p, m.shift, m.ctrl);
}
fn down(s: &mut Session, p: Point, m: M) {
    s.modifiers_changed(m.shift, m.ctrl, m.alt);
    s.pointer_down(p, m.shift);
}
fn up(s: &mut Session, p: Point, m: M) {
    s.modifiers_changed(m.shift, m.ctrl, m.alt);
    s.pointer_up(p, m.shift, m.ctrl);
}
fn click(s: &mut Session, p: Point, m: M) {
    hover(s, p, m);
    down(s, p, m);
    up(s, p, m);
}
fn drag(s: &mut Session, a: Point, b: Point, m_down: M, m_up: M) {
    hover(s, a, m_down);
    down(s, a, m_down);
    let mid = pt(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y));
    hover(s, mid, m_down);
    hover(s, b, m_up);
    up(s, b, m_up);
}

fn key(s: &mut Session, k: &str, m: M) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        shift: m.shift,
        ctrl: m.ctrl,
        alt: m.alt,
        ..KeyInput::default()
    })
}

fn view(s: &Session) -> Value {
    let _ = s.draw_list();
    serde_json::from_str(&s.measure_view_json()).unwrap()
}

fn readout(s: &Session) -> Option<Vec<String>> {
    let v = view(s);
    v["readout"]["lines"]
        .as_array()
        .map(|l| l.iter().map(|x| x.as_str().unwrap().to_owned()).collect())
}

fn mode(s: &Session) -> Option<String> {
    view(s)["mode"]["lines"][0].as_str().map(str::to_owned)
}

fn labels(s: &Session) -> Vec<Value> {
    view(s)["dimensions"].as_array().unwrap().clone()
}

fn preview_labels(s: &Session) -> Vec<Value> {
    labels(s)
        .into_iter()
        .filter(|l| l["preview"].as_bool().unwrap())
        .collect()
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("t").unwrap()).unwrap()
}

fn dims(s: &Session) -> Vec<curvyo_document_core::DimensionSnapshot> {
    doc(s)
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Dimension(x) => Some((**x).clone()),
            _ => None,
        })
        .collect()
}

fn history_names(s: &mut Session) -> Vec<String> {
    let rows: Value = serde_json::from_str(&s.history_rows_json(0, 50)).unwrap();
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect()
}

fn newest_name(s: &mut Session) -> String {
    history_names(s).first().cloned().unwrap_or_default()
}

fn row_id(id: NodeId) -> String {
    serde_json::to_string(&id).unwrap()
}

fn mm(v: curvyo_document_core::DimensionValue) -> f64 {
    match v {
        curvyo_document_core::DimensionValue::Length(l) => l.as_mm(),
        curvyo_document_core::DimensionValue::Angle(a) => a.as_radians(),
    }
}

/// A 30-40-50 path on an otherwise empty page, the Measure tool active.
fn measuring(snapping: bool) -> (Session, NodeId) {
    let d = Document::new(1);
    let p = line(&d, 10, pt(10.0, 10.0), pt(40.0, 50.0));
    let mut s = open(&d, snapping);
    s.set_tool(Tool::Measure);
    (s, p)
}

/// Leaves an aligned 50 mm dimension on the 30-40-50 path (Shift at the second pick, placed
/// inside the box).
fn leave_fifty(s: &mut Session) -> NodeId {
    drag(s, pt(10.0, 10.0), pt(40.0, 50.0), NONE, SHIFT);
    hover(s, pt(22.0, 33.0), SHIFT);
    click(s, pt(22.0, 33.0), NONE);
    let d = dims(s);
    assert_eq!(d.len(), 1, "one dimension placed");
    d[0].id
}

// ---------------- criterion 2: rail letter, gating, no bar ----------------

#[test]
fn c2_m_is_the_measure_tool_except_in_select_with_a_selection() {
    let d = Document::new(1);
    let _ = d.create_rect(RectBounds {
        origin: pt(10.0, 10.0),
        width: Length::from_mm(20.0),
        height: Length::from_mm(20.0),
    });
    let mut s = open(&d, false);
    let before = s.pack("t").unwrap();
    s.set_tool(Tool::Select);
    assert_eq!(key(&mut s, "m", NONE), KeyOutcome::ToolChanged);
    assert_eq!(s.tool(), Tool::Measure);
    for t in [Tool::Rectangle, Tool::Ellipse, Tool::Pen, Tool::Node] {
        s.set_tool(t);
        key(&mut s, "m", NONE);
        assert_eq!(s.tool(), Tool::Measure, "from {t:?}");
    }
    // Select with a selection: M keeps opening the move entry.
    s.set_tool(Tool::Select);
    click(&mut s, pt(10.0, 20.0), NONE);
    assert_eq!(s.selected_object_count(), 1);
    assert_eq!(key(&mut s, "m", NONE), KeyOutcome::EntryOpened);
    assert_eq!(s.tool(), Tool::Select);
    // In a field (dom_blocked) M is ignored.
    let mut s2 = open(&d, false);
    s2.set_tool(Tool::Rectangle);
    s2.key_down(KeyInput {
        key: "m",
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(s2.tool(), Tool::Rectangle);
    // Nothing written by the letter.
    let mut s3 = open(&d, false);
    key(&mut s3, "m", NONE);
    assert_eq!(s3.pack("t").unwrap(), before);
}

#[test]
fn c2_no_mode_label_or_readout_before_a_pick() {
    // Customer feedback on PR #243 (2026-11-01): only the crosshair until the first pick; it
    // replaced the label "Measure" ("Dimension" with Shift) before a pick.
    let (mut s, _) = measuring(false);
    hover(&mut s, pt(100.0, 100.0), NONE);
    assert_eq!(mode(&s), None);
    hover(&mut s, pt(100.0, 100.0), SHIFT);
    assert_eq!(mode(&s), None);
    assert!(readout(&s).is_none(), "no readout before a pick");
}

// ---------------- criterion 1: readout, nothing written, pinned ----------------

#[test]
fn c1_drag_shows_length_angle_and_deltas_and_writes_nothing() {
    let (mut s, _) = measuring(false);
    let before = s.pack("t").unwrap();
    let hist = s.history_len();
    hover(&mut s, pt(10.0, 10.0), NONE);
    down(&mut s, pt(10.0, 10.0), NONE);
    hover(&mut s, pt(40.0, 50.0), NONE);
    let lines = readout(&s).expect("live readout while dragging");
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "50 mm, 53.13°");
    assert_eq!(lines[1], "Δ 30, 40 mm");
    up(&mut s, pt(40.0, 50.0), NONE);
    assert_eq!(readout(&s).unwrap(), lines, "pinned after the second pick");
    // Pan and zoom keep it.
    s.wheel(0.0, 40.0, 100.0, 100.0, false, false);
    s.wheel(0.0, -120.0, 100.0, 100.0, false, true);
    assert_eq!(readout(&s).unwrap(), lines, "pan and zoom keep it");
    assert_eq!(s.history_len(), hist, "history unchanged");
    assert_eq!(s.pack("t").unwrap(), before, "document unchanged");
    assert_eq!(s.speech_for_test_or_view(), "50 millimetres, 53.13 degrees");
}

trait Speech {
    fn speech_for_test_or_view(&self) -> String;
}
impl Speech for Session {
    fn speech_for_test_or_view(&self) -> String {
        view(self)["speech"].as_str().unwrap().to_owned()
    }
}

#[test]
fn c1_click_click_gives_the_same_measurement_as_a_drag() {
    let (mut s, _) = measuring(false);
    click(&mut s, pt(10.0, 10.0), NONE);
    hover(&mut s, pt(40.0, 50.0), NONE);
    assert_eq!(readout(&s).unwrap()[0], "50 mm, 53.13°");
    click(&mut s, pt(40.0, 50.0), NONE);
    assert_eq!(readout(&s).unwrap()[0], "50 mm, 53.13°");
    hover(&mut s, pt(90.0, 90.0), NONE);
    assert_eq!(
        readout(&s).unwrap()[0],
        "50 mm, 53.13°",
        "pinned: moving the pointer does not change it"
    );
}

#[test]
fn c1_negative_deltas_use_the_minus_sign_and_three_decimals_trimmed() {
    let (mut s, _) = measuring(false);
    drag(&mut s, pt(0.0, 0.0), pt(-1.0 / 3.0, -2.0), NONE, NONE);
    let lines = readout(&s).unwrap();
    assert!(lines[1].contains("\u{2212}0.333"), "{lines:?}");
    assert!(lines[1].contains("\u{2212}2 mm"), "{lines:?}");
    assert!(!lines[1].contains('-'), "U+2212, not a hyphen: {lines:?}");
    assert!(lines[0].starts_with("2.028 mm"), "{lines:?}");
}

#[test]
fn c1_readout_is_in_the_document_unit() {
    let d = Document::new(1);
    let _ = d.set_display_unit(DisplayUnit::In);
    let mut s = open(&d, false);
    s.set_tool(Tool::Measure);
    drag(&mut s, pt(0.0, 0.0), pt(30.0, 40.0), NONE, NONE);
    let lines = readout(&s).unwrap();
    assert!(lines[0].starts_with("1.969 in"), "{lines:?}");
    assert!(
        lines[1].contains("1.181") && lines[1].contains("1.575"),
        "{lines:?}"
    );
}

#[test]
fn c1_pinned_measurement_ends_on_press_escape_tool_change_undo_and_edit() {
    let pinned = |s: &mut Session| {
        drag(s, pt(10.0, 10.0), pt(40.0, 50.0), NONE, NONE);
        assert!(readout(s).is_some());
    };
    let (mut s, _) = measuring(false);
    pinned(&mut s);
    key(&mut s, "Escape", NONE);
    assert!(readout(&s).is_none(), "Escape");
    assert_eq!(s.tool(), Tool::Measure, "first Escape cancels only");
    key(&mut s, "Escape", NONE);
    assert_eq!(s.tool(), Tool::Select, "second Escape leaves to Select");

    let (mut s, _) = measuring(false);
    pinned(&mut s);
    hover(&mut s, pt(80.0, 80.0), NONE);
    down(&mut s, pt(80.0, 80.0), NONE);
    let after_press = readout(&s);
    assert!(
        after_press.is_none() || after_press.unwrap()[0] != "50 mm, 53.13°",
        "next press ends it"
    );

    let (mut s, _) = measuring(false);
    pinned(&mut s);
    s.set_tool(Tool::Select);
    s.set_tool(Tool::Measure);
    assert!(readout(&s).is_none(), "tool change");
}

fn pinned_after_a_dimension() -> Session {
    let (mut s, _) = measuring(false);
    leave_fifty(&mut s);
    drag(&mut s, pt(10.0, 10.0), pt(40.0, 50.0), NONE, NONE);
    assert!(readout(&s).is_some());
    s
}

#[test]
fn c1_pinned_measurement_ends_on_undo() {
    let mut s = pinned_after_a_dimension();
    let _ = s.undo();
    assert!(dims(&s).is_empty(), "the undo happened");
    // The host re-sends the pointer position after a change (the pointer did not move); the
    // session drops a stale pin on that event.
    hover(&mut s, pt(40.0, 50.0), NONE);
    assert!(readout(&s).is_none(), "undo ends the pinned measurement");
}

#[test]
fn c1_pinned_measurement_ends_on_redo() {
    let (mut s, _) = measuring(false);
    leave_fifty(&mut s);
    let _ = s.undo();
    drag(&mut s, pt(10.0, 10.0), pt(40.0, 50.0), NONE, NONE);
    assert!(readout(&s).is_some());
    let _ = s.redo();
    assert_eq!(dims(&s).len(), 1, "the redo happened");
    // The host re-sends the pointer position after a change (the pointer did not move); the
    // session drops a stale pin on that event.
    hover(&mut s, pt(40.0, 50.0), NONE);
    assert!(readout(&s).is_none(), "redo ends the pinned measurement");
}

#[test]
fn c1_pinned_measurement_ends_on_a_document_edit() {
    let mut s = pinned_after_a_dimension();
    assert!(s.set_display_unit(DisplayUnit::Cm));
    // The host re-sends the pointer position after a change (the pointer did not move); the
    // session drops a stale pin on that event.
    hover(&mut s, pt(40.0, 50.0), NONE);
    assert!(
        readout(&s).is_none(),
        "a document edit ends the pinned measurement"
    );
}

// ---------------- criterion 3: kinds and the eight regions ----------------

#[test]
fn c3_placement_region_chooses_horizontal_vertical_or_aligned() {
    // A=(10,10), B=(40,50): box x 10..40, y 10..50.
    let cases = [
        (pt(25.0, -10.0), "Dimension, horizontal", 30.0), // above
        (pt(25.0, 80.0), "Dimension, horizontal", 30.0),  // below
        (pt(-10.0, 30.0), "Dimension, vertical", 40.0),   // left
        (pt(70.0, 30.0), "Dimension, vertical", 40.0),    // right
        (pt(25.0, 30.0), "Dimension, aligned", 50.0),     // inside
        (pt(-10.0, -10.0), "Dimension, aligned", 50.0),   // corner quadrants
        (pt(70.0, -10.0), "Dimension, aligned", 50.0),
        (pt(-10.0, 80.0), "Dimension, aligned", 50.0),
        (pt(70.0, 80.0), "Dimension, aligned", 50.0),
    ];
    for (p, label, value) in cases {
        let (mut s, _) = measuring(false);
        drag(&mut s, pt(10.0, 10.0), pt(40.0, 50.0), NONE, SHIFT);
        hover(&mut s, p, NONE);
        assert_eq!(mode(&s).as_deref(), Some(label), "pointer at {p:?}");
        let n = s.history_len();
        click(&mut s, p, NONE);
        assert_eq!(s.history_len(), n + 1);
        let d = dims(&s);
        assert_eq!(d.len(), 1);
        assert!(
            (mm(d[0].shown.value) - value).abs() < 1e-9,
            "{label} at {p:?}: {}",
            mm(d[0].shown.value)
        );
        assert_eq!(
            d[0].kind.name().to_lowercase(),
            label.trim_start_matches("Dimension, ")
        );
    }
}

#[test]
fn c3_without_shift_the_mode_label_names_the_kind_alone() {
    let (mut s, _) = measuring(false);
    hover(&mut s, pt(10.0, 10.0), NONE);
    down(&mut s, pt(10.0, 10.0), NONE);
    hover(&mut s, pt(40.0, 50.0), NONE);
    let m = mode(&s).unwrap();
    assert!(!m.starts_with("Dimension"), "{m}");
    assert!(
        ["Aligned", "Horizontal", "Vertical"].contains(&m.as_str()),
        "{m}"
    );
}

#[test]
fn c3_a_curve_pick_is_radius_and_a_segment_pick_gives_its_length() {
    let d = Document::new(1);
    let _c = circle(&d, pt(100.0, 100.0), 25.0);
    let _p = line(&d, 10, pt(0.0, 0.0), pt(30.0, 40.0));
    let mut s = open(&d, false);
    s.set_tool(Tool::Measure);
    // Curve pick on the outline away from the quadrant nodes, Shift held at the pick.
    let on = pt(
        100.0 + 25.0 * (0.4_f64).cos(),
        100.0 + 25.0 * (0.4_f64).sin(),
    );
    hover(&mut s, on, NONE);
    assert_eq!(
        mode(&s),
        None,
        "no label before a pick (customer feedback on PR #243)"
    );
    click(&mut s, on, SHIFT);
    hover(&mut s, pt(150.0, 150.0), NONE);
    assert_eq!(mode(&s).as_deref(), Some("Dimension, radius"));
    let pv = preview_labels(&s);
    assert_eq!(pv.len(), 1);
    assert_eq!(pv[0]["text"], "R 25 mm");
    click(&mut s, pt(150.0, 150.0), NONE);
    let d1 = dims(&s);
    assert_eq!(d1.len(), 1);
    assert_eq!(d1[0].kind.name(), "Radius");
    assert!((mm(d1[0].shown.value) - 25.0).abs() < 1e-9);
    assert!(d1[0].linked);

    // Segment pick (middle of the 30-40-50 segment, away from its midpoint snap) then
    // placement gives the aligned length.
    let mid = pt(15.0 * 0.6, 20.0 * 0.6);
    click(&mut s, mid, SHIFT);
    hover(&mut s, pt(30.0, 0.0), NONE);
    let m = mode(&s).unwrap();
    assert_eq!(m, "Dimension, aligned");
    click(&mut s, pt(30.0, 0.0), NONE);
    let all = dims(&s);
    assert_eq!(all.len(), 2);
    let seg = all.iter().find(|x| x.kind.name() == "Aligned").unwrap();
    assert!((mm(seg.shown.value) - 50.0).abs() < 1e-9);
    assert!(seg.linked);
}

#[test]
fn c3_two_non_parallel_segment_picks_give_an_angle() {
    let d = Document::new(1);
    let _ = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(60.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 3), pt(60.0, -60.0)),
        ],
        false,
    );
    let mut s = open(&d, false);
    s.set_tool(Tool::Measure);
    click(&mut s, pt(37.0, 0.0), NONE);
    click(&mut s, pt(37.0, -37.0), SHIFT);
    hover(&mut s, pt(30.0, -10.0), NONE);
    assert_eq!(mode(&s).as_deref(), Some("Dimension, angle"));
    let pv = preview_labels(&s);
    assert_eq!(pv.len(), 1, "{pv:?}");
    assert_eq!(pv[0]["text"], "45°");
    click(&mut s, pt(30.0, -10.0), NONE);
    let d = dims(&s);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].kind.name(), "Angle");
    assert!((mm(d[0].shown.value).abs() - std::f64::consts::FRAC_PI_4).abs() < 1e-9);
}

// ---------------- criterion 4: Shift ----------------

#[test]
fn c4_shift_at_the_second_pick_leaves_one_dimension_in_one_undo_step() {
    let (mut s, p) = measuring(false);
    let n = s.history_len();
    let id = leave_fifty(&mut s);
    assert_eq!(s.history_len(), n + 1);
    assert_eq!(newest_name(&mut s), "Dimension");
    let d = dims(&s);
    assert!(d[0].linked);
    assert_eq!(d[0].slots.nodes(), vec![p], "linked to the measured path");
    let _ = s.undo();
    assert!(dims(&s).is_empty(), "undo removes it in one step");
    let _ = id;
}

#[test]
fn c4_shift_is_read_live_and_swaps_readout_and_preview_in_the_same_frame() {
    let (mut s, _) = measuring(false);
    hover(&mut s, pt(10.0, 10.0), NONE);
    down(&mut s, pt(10.0, 10.0), NONE);
    hover(&mut s, pt(40.0, 50.0), NONE);
    assert!(readout(&s).is_some());
    assert!(preview_labels(&s).is_empty());
    hover(&mut s, pt(40.0, 50.0), SHIFT);
    assert!(readout(&s).is_none(), "chip hidden with Shift");
    assert_eq!(preview_labels(&s).len(), 1, "green preview with Shift");
    assert!(mode(&s).unwrap().starts_with("Dimension, "));
    // Only the modifier changes, no pointer move: the next frame still swaps.
    s.modifiers_changed(false, false, false);
    assert!(readout(&s).is_some(), "released Shift: readout back");
    assert!(preview_labels(&s).is_empty());
    up(&mut s, pt(40.0, 50.0), NONE);
    assert!(
        dims(&s).is_empty() && readout(&s).is_some(),
        "no Shift at the pick: a measurement"
    );
}

#[test]
fn c4_shift_released_before_the_placing_click_still_places_and_escape_cancels() {
    let (mut s, _) = measuring(false);
    drag(&mut s, pt(10.0, 10.0), pt(40.0, 50.0), NONE, SHIFT);
    hover(&mut s, pt(25.0, 30.0), NONE);
    assert_eq!(preview_labels(&s).len(), 1, "follows the pointer");
    let n = s.history_len();
    key(&mut s, "Escape", NONE);
    assert!(preview_labels(&s).is_empty());
    click(&mut s, pt(25.0, 30.0), NONE);
    assert!(dims(&s).is_empty(), "Escape cancelled the gesture");
    assert_eq!(s.history_len(), n);
}

#[test]
fn c4_ctrl_locks_the_direction_and_shift_never_locks_an_axis() {
    let (mut s, _) = measuring(false);
    hover(&mut s, pt(0.0, 0.0), NONE);
    down(&mut s, pt(0.0, 0.0), NONE);
    hover(&mut s, pt(30.0, 2.0), CTRL);
    let lines = readout(&s).unwrap();
    assert!(lines[0].ends_with(", 0°"), "{lines:?}");
    assert!(lines[1].ends_with(", 0 mm"), "{lines:?}");
    assert!(s.measure_angle_lock_badge(), "Lock badge while Ctrl locks");
    hover(&mut s, pt(30.0, 2.0), SHIFT);
    // With Shift the chip hides; the preview text is the free length.
    let pv = preview_labels(&s);
    assert_eq!(pv.len(), 1);
    assert!(!s.measure_angle_lock_badge());
}

// ---------------- criterion 6: snapping and Alt ----------------

#[test]
fn c6_anchors_snap_to_centres_only_with_snapping_and_alt_held_at_the_pick_turns_it_off() {
    // Picks near a circle centre, far from any outline or segment.
    let d = Document::new(1);
    let _ = circle(&d, pt(50.0, 50.0), 25.0);
    let _ = circle(&d, pt(150.0, 50.0), 25.0);
    let a = pt(50.4, 50.3);
    let b = pt(149.7, 50.2);
    let mut s = open(&d, true);
    s.set_tool(Tool::Measure);
    drag(&mut s, a, b, NONE, NONE);
    assert_eq!(
        readout(&s).unwrap()[1],
        "Δ 100, 0 mm",
        "snapped to both centres"
    );
    key(&mut s, "Escape", NONE);
    drag(&mut s, a, b, ALT, ALT);
    assert_ne!(
        readout(&s).unwrap()[1],
        "Δ 100, 0 mm",
        "Alt held at the picks: snapping off"
    );
    let mut s = open(&d, false);
    s.set_tool(Tool::Measure);
    drag(&mut s, a, b, NONE, NONE);
    assert_ne!(
        readout(&s).unwrap()[1],
        "Δ 100, 0 mm",
        "snapping off: free points"
    );
}

#[test]
fn c6_snaps_to_centres_and_midpoints() {
    let d = Document::new(1);
    let _ = circle(&d, pt(50.0, 50.0), 10.0);
    let _ = line(&d, 10, pt(0.0, 0.0), pt(20.0, 0.0));
    let mut s = open(&d, true);
    s.set_tool(Tool::Measure);
    drag(&mut s, pt(10.2, 0.3), pt(49.7, 50.2), NONE, NONE);
    let lines = readout(&s).unwrap();
    assert_eq!(lines[1], "Δ 40, 50 mm", "midpoint to centre: {lines:?}");
}

#[test]
fn c6_alt_cycles_radius_and_diameter_only_while_placing_and_not_when_held_at_the_pick() {
    let d = Document::new(1);
    let _ = circle(&d, pt(100.0, 100.0), 25.0);
    let on = pt(
        100.0 + 25.0 * (0.4_f64).cos(),
        100.0 + 25.0 * (0.4_f64).sin(),
    );
    let mut s = open(&d, true);
    s.set_tool(Tool::Measure);
    click(&mut s, on, SHIFT);
    hover(&mut s, pt(150.0, 150.0), NONE);
    assert_eq!(
        mode(&s).as_deref(),
        Some("Dimension, radius"),
        "default Radius"
    );
    key(&mut s, "Alt", ALT);
    hover(&mut s, pt(150.0, 150.0), NONE);
    assert_eq!(mode(&s).as_deref(), Some("Dimension, diameter"));
    assert_eq!(preview_labels(&s)[0]["text"], "⌀ 50 mm");
    // A repeat does nothing.
    s.key_down(KeyInput {
        key: "Alt",
        alt: true,
        repeat: true,
        ..KeyInput::default()
    });
    assert_eq!(mode(&s).as_deref(), Some("Dimension, diameter"));
    // With Shift or Ctrl down it does nothing.
    key(
        &mut s,
        "Alt",
        M {
            shift: true,
            ctrl: false,
            alt: true,
        },
    );
    key(
        &mut s,
        "Alt",
        M {
            shift: false,
            ctrl: true,
            alt: true,
        },
    );
    hover(&mut s, pt(150.0, 150.0), NONE);
    assert_eq!(mode(&s).as_deref(), Some("Dimension, diameter"));
    key(&mut s, "Alt", ALT);
    hover(&mut s, pt(150.0, 150.0), NONE);
    assert_eq!(
        mode(&s).as_deref(),
        Some("Dimension, radius"),
        "cycles back"
    );
    key(&mut s, "Alt", ALT);
    click(&mut s, pt(150.0, 150.0), NONE);
    let x = dims(&s);
    assert_eq!(x[0].kind.name(), "Diameter");
    assert!((mm(x[0].shown.value) - 50.0).abs() < 1e-9);

    // Alt held at the pick (snapping off for it) does not cycle.
    let mut s = open(&d, true);
    s.set_tool(Tool::Measure);
    hover(
        &mut s,
        on,
        M {
            shift: true,
            ctrl: false,
            alt: true,
        },
    );
    down(
        &mut s,
        on,
        M {
            shift: true,
            ctrl: false,
            alt: true,
        },
    );
    up(
        &mut s,
        on,
        M {
            shift: true,
            ctrl: false,
            alt: true,
        },
    );
    hover(&mut s, pt(150.0, 150.0), ALT);
    assert_eq!(mode(&s).as_deref(), Some("Dimension, radius"));
}

#[test]
fn c6_alt_does_nothing_in_a_two_point_placement_and_placement_is_never_snapped() {
    let (mut s, _) = measuring(true);
    drag(&mut s, pt(10.0, 10.0), pt(40.0, 50.0), NONE, SHIFT);
    hover(&mut s, pt(25.0, 30.0), NONE);
    let before = mode(&s);
    key(&mut s, "Alt", ALT);
    hover(&mut s, pt(25.0, 30.0), NONE);
    assert_eq!(mode(&s), before);
    // Place near a node: the place is not pulled onto it (offset from the pointer, not the node).
    let pv1 = preview_labels(&s)[0].clone();
    hover(&mut s, pt(25.0, 30.6), NONE);
    let pv2 = preview_labels(&s)[0].clone();
    assert_ne!(
        pv1["y"], pv2["y"],
        "the line follows the pointer continuously"
    );
}

// ---------------- criterion 5: selection first and Enter ----------------

#[test]
fn c5_selection_first_places_on_one_click_and_enter_places_at_the_default_position() {
    let d = Document::new(1);
    let c = circle(&d, pt(100.0, 100.0), 25.0);
    let mut s = open(&d, false);
    s.set_tool(Tool::Select);
    click(&mut s, pt(125.0, 100.0), NONE);
    assert_eq!(s.selected_object_count(), 1);
    s.set_tool(Tool::Measure);
    hover(&mut s, pt(160.0, 60.0), NONE);
    let pv = preview_labels(&s);
    assert_eq!(pv.len(), 1, "preview follows the pointer at once");
    assert_eq!(pv[0]["text"], "R 25 mm");
    let n = s.history_len();
    click(&mut s, pt(160.0, 60.0), NONE);
    assert_eq!(s.history_len(), n + 1, "one click places");
    assert_eq!(dims(&s).len(), 1);
    assert_eq!(dims(&s)[0].slots.nodes(), vec![c]);
    assert_eq!(
        s.selected_object_count(),
        1,
        "the selection does not change"
    );
    // Enter places at the default position.
    let _ = key(&mut s, "Enter", NONE);
    assert_eq!(dims(&s).len(), 2, "Enter placed one more");
}

#[test]
fn c5_enter_places_for_a_selected_two_node_path_and_nothing_without_an_eligible_selection() {
    let d = Document::new(1);
    let p = line(&d, 10, pt(0.0, 0.0), pt(30.0, 40.0));
    let r = d.create_rect(RectBounds {
        origin: pt(100.0, 100.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    });
    let mut s = open(&d, false);
    s.press_layer_row(&row_id(p), RowPress::Plain);
    s.set_tool(Tool::Measure);
    let n = s.history_len();
    key(&mut s, "Enter", NONE);
    assert_eq!(s.history_len(), n + 1);
    let x = dims(&s);
    assert_eq!(x.len(), 1);
    assert!((mm(x[0].shown.value) - 50.0).abs() < 1e-9);
    assert_eq!(x[0].kind.name(), "Aligned");
    // A rectangle is not eligible: Enter does nothing.
    s.set_tool(Tool::Select);
    s.press_layer_row(&row_id(r), RowPress::Plain);
    s.set_tool(Tool::Measure);
    let n = s.history_len();
    key(&mut s, "Enter", NONE);
    assert_eq!(s.history_len(), n);
    // Two objects selected: not eligible.
    s.set_tool(Tool::Select);
    s.press_layer_row(&row_id(p), RowPress::Plain);
    s.press_layer_row(&row_id(r), RowPress::Toggle);
    s.set_tool(Tool::Measure);
    key(&mut s, "Enter", NONE);
    assert_eq!(s.history_len(), n);
}

#[test]
fn c5_a_press_on_a_pick_source_starts_a_normal_pick_with_a_selection() {
    let d = Document::new(1);
    let _c = circle(&d, pt(100.0, 100.0), 25.0);
    let _p = line(&d, 10, pt(0.0, 0.0), pt(30.0, 40.0));
    let mut s = open(&d, true);
    s.set_tool(Tool::Select);
    click(&mut s, pt(125.0, 100.0), NONE);
    s.set_tool(Tool::Measure);
    // Press on the path's node: a measurement starts, nothing placed.
    drag(&mut s, pt(0.0, 0.0), pt(30.0, 40.0), NONE, NONE);
    assert!(dims(&s).is_empty());
    assert_eq!(readout(&s).unwrap()[0], "50 mm, 53.13°");
}

// ---------------- criterion 7 / 12: look, unlink, labels ----------------

#[test]
fn c12_deleting_the_measured_path_unlinks_in_the_same_step_and_undo_relinks() {
    let (mut s, p) = measuring(false);
    let id = leave_fifty(&mut s);
    s.set_tool(Tool::Select);
    s.press_layer_row(&row_id(p), RowPress::Plain);
    let n = s.history_len();
    s.delete_selected();
    assert_eq!(s.history_len(), n + 1);
    assert_eq!(newest_name(&mut s), "Delete");
    let l = labels(&s);
    let me = l.iter().find(|x| x["id"] == row_id(id)).unwrap();
    assert_eq!(me["linked"], false);
    assert_eq!(me["text"], "50 mm", "no parentheses");
    assert!(
        me["name"].as_str().unwrap().ends_with("not linked"),
        "{}",
        me["name"]
    );
    let rows: Value = serde_json::from_str(&s.layers_view_json(0, 20)).unwrap();
    let row = rows["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "dimension")
        .unwrap();
    let name = row["name"].as_str().unwrap();
    assert!(
        name.starts_with("Dimension 50 mm") && name.ends_with("not linked"),
        "{name}"
    );
    let _ = s.undo();
    let l = labels(&s);
    assert_eq!(l[0]["linked"], true, "one undo restores the link");
    assert_eq!(dims(&s)[0].slots.nodes(), vec![p]);
}

#[test]
fn c17_layers_row_announcer_and_delete() {
    let (mut s, _) = measuring(false);
    let id = leave_fifty(&mut s);
    let rows: Value = serde_json::from_str(&s.layers_view_json(0, 20)).unwrap();
    let row = rows["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == serde_json::to_value(id).unwrap())
        .unwrap()
        .clone();
    assert_eq!(row["name"], "Dimension 50 mm");
    s.set_tool(Tool::Select);
    s.press_layer_row(&row_id(id), RowPress::Plain);
    assert_eq!(s.selected_object_count(), 1);
    let ann = s.selection_announcement();
    assert!(ann.contains("Dimension 50 mm"), "announcer: {ann}");
    let n = s.history_len();
    key(&mut s, "Delete", NONE);
    assert_eq!(s.history_len(), n + 1);
    assert_eq!(newest_name(&mut s), "Delete");
    assert!(dims(&s).is_empty());
}

#[test]
fn c17_layers_row_uses_the_display_unit() {
    let (mut s, _) = measuring(false);
    let _ = leave_fifty(&mut s);
    s.set_display_unit(DisplayUnit::Cm);
    let rows: Value = serde_json::from_str(&s.layers_view_json(0, 20)).unwrap();
    let row = rows["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "dimension")
        .unwrap()
        .clone();
    assert_eq!(row["name"], "Dimension 5 cm");
    let l = labels(&s);
    assert_eq!(l[0]["text"], "5 cm");
}

#[test]
fn c7_vector_dimension_text_is_never_hidden_even_for_a_tiny_span() {
    let d = Document::new(1);
    let _ = line(&d, 10, pt(0.0, 0.0), pt(0.5, 0.0));
    let mut s = open(&d, false);
    s.set_tool(Tool::Measure);
    click(&mut s, pt(0.0, 0.0), NONE);
    click(&mut s, pt(0.5, 0.0), SHIFT);
    click(&mut s, pt(0.25, -3.0), NONE);
    let l = labels(&s);
    assert_eq!(l.len(), 1, "drawn though the span is under 16 px");
    // The axis glyph of the shared 0066 text ("↔ ") may lead.
    assert!(
        l[0]["text"].as_str().unwrap().ends_with("0.5 mm"),
        "{}",
        l[0]["text"]
    );
}

// ---------------- criterion 11: live follow ----------------

#[test]
fn c11_a_node_tool_drag_updates_the_number_live_in_one_history_step() {
    let (mut s, p) = measuring(false);
    let _ = leave_fifty(&mut s);
    s.set_tool(Tool::Select);
    s.press_layer_row(&row_id(p), RowPress::Plain);
    s.set_tool(Tool::Node);
    let n = s.history_len();
    hover(&mut s, pt(40.0, 50.0), NONE);
    down(&mut s, pt(40.0, 50.0), NONE);
    hover(&mut s, pt(70.0, 90.0), NONE);
    assert_eq!(labels(&s)[0]["text"], "100 mm", "live during the drag");
    up(&mut s, pt(70.0, 90.0), NONE);
    assert_eq!(s.history_len(), n + 1, "one step");
    assert_eq!(labels(&s)[0]["text"], "100 mm");
    let x = dims(&s);
    assert!((mm(x[0].kept.value) - 100.0).abs() < 1e-9, "kept refreshed");
}

#[test]
fn c11_a_select_move_moves_the_dimension_live_without_changing_its_number() {
    let (mut s, p) = measuring(false);
    let _ = leave_fifty(&mut s);
    s.set_tool(Tool::Select);
    let before = labels(&s)[0].clone();
    s.press_layer_row(&row_id(p), RowPress::Plain);
    let n = s.history_len();
    hover(&mut s, pt(25.0, 30.0), NONE);
    down(&mut s, pt(25.0, 30.0), NONE);
    hover(&mut s, pt(35.0, 30.0), NONE);
    let during = labels(&s)[0].clone();
    assert_eq!(during["text"], "50 mm");
    assert!(
        during["x"].as_f64().unwrap() > before["x"].as_f64().unwrap() + 30.0,
        "label moved with the drag: {before} -> {during}"
    );
    up(&mut s, pt(35.0, 30.0), NONE);
    assert_eq!(s.history_len(), n + 1);
}

// ---------------- criterion 8: Move dimension ----------------

fn select_dimension_by_click(s: &mut Session) -> Value {
    s.set_tool(Tool::Select);
    let l = labels(s)[0].clone();
    let at = s.screen_to_document(l["x"].as_f64().unwrap(), l["y"].as_f64().unwrap());
    click(s, at, NONE);
    l
}

#[test]
fn c8_dragging_the_text_is_one_step_move_dimension() {
    let (mut s, _) = measuring(false);
    let id = leave_fifty(&mut s);
    let l = select_dimension_by_click(&mut s);
    assert_eq!(s.selected_object_count(), 1, "clicking the text selects it");
    assert_eq!(labels(&s)[0]["selected"], true);
    let at = s.screen_to_document(l["x"].as_f64().unwrap(), l["y"].as_f64().unwrap());
    let before = dims(&s)[0].place;
    let n = s.history_len();
    drag(&mut s, at, pt(at.x + 5.0, at.y + 5.0), NONE, NONE);
    assert_eq!(s.history_len(), n + 1);
    assert_eq!(newest_name(&mut s), "Move dimension");
    let after = dims(&s)[0].clone();
    assert_ne!(after.place, before);
    assert_eq!(after.id, id);
    assert!(
        (mm(after.shown.value) - 50.0).abs() < 1e-9,
        "the number stays"
    );
}

// ---------------- criterion 10: Guide button converts ----------------

fn fonts_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

/// Loads the included faces the way the host does, so a text object can be made.
fn with_fonts(s: &mut Session) {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect();
    s.set_font_list(curvyo_ui_core::FontBlock::Included, faces, Vec::new());
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
}

#[test]
fn c10_guide_button_on_a_dimension_converts_in_one_step_and_back_to_guides() {
    let (mut s, _) = measuring(false);
    with_fonts(&mut s);
    let id = leave_fifty(&mut s);
    s.set_tool(Tool::Select);
    s.press_layer_row(&row_id(id), RowPress::Plain);
    let _ = view(&s); // the host draws a frame after every change
    let n = s.history_len();
    assert!(s.toggle_guide(), "Guide pressed on a dimension converts it");
    assert_eq!(s.history_len(), n + 1);
    assert_eq!(newest_name(&mut s), "Convert to path");
    assert!(dims(&s).is_empty());
    let d = doc(&s);
    let objects = d.read_objects();
    let texts = objects
        .objects
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Text(_)))
        .count();
    let paths = objects
        .objects
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Path(_)))
        .count();
    assert_eq!(texts, 1, "the text becomes an editable text object");
    assert!(
        paths > 3,
        "measured path plus lines and arrowheads: {paths}"
    );
    assert!(
        s.selected_object_count() >= 1,
        "the new objects are selected"
    );
    let nodes = d.read_nodes();
    let guides = nodes.iter().filter(|n| n.guide).count();
    assert_eq!(guides, 0, "ordinary objects after the convert");
    // Guide again: plain guides.
    let _ = view(&s);
    assert!(s.toggle_guide());
    assert_eq!(newest_name(&mut s), "Guide");
    assert!(dims(&s).is_empty(), "no dimension rebuilt");
    assert!(doc(&s).read_nodes().iter().any(|n| n.guide));
    // Undo twice: the dimension is back.
    let _ = s.undo();
    let _ = s.undo();
    assert_eq!(dims(&s).len(), 1);
}

// ---------------- criterion 13: Node tool on unlinked kept points ----------------

#[test]
fn c13_node_tool_moves_kept_points_of_an_unlinked_dimension_without_changing_the_number() {
    let (mut s, p) = measuring(false);
    let id = leave_fifty(&mut s);
    s.set_tool(Tool::Select);
    s.press_layer_row(&row_id(p), RowPress::Plain);
    s.delete_selected();
    assert!(!dims(&s)[0].linked);
    s.press_layer_row(&row_id(id), RowPress::Plain);
    s.set_tool(Tool::Node);
    let n = s.history_len();
    drag(&mut s, pt(40.0, 50.0), pt(80.0, 50.0), NONE, NONE);
    assert_eq!(s.history_len(), n + 1, "one step");
    assert_eq!(newest_name(&mut s), "Edit nodes");
    let x = dims(&s);
    assert!((mm(x[0].shown.value) - 50.0).abs() < 1e-9, "number is kept");
    assert!(
        (x[0].shown.points[1].x - 80.0).abs() < 1e-6,
        "{:?}",
        x[0].shown.points
    );
}

// ---------------- criterion 15: hidden, locked, existing dimensions ----------------

#[test]
fn c15_hidden_objects_are_not_picked_locked_ones_are() {
    let d = Document::new(1);
    let hidden = line(&d, 10, pt(0.0, 0.0), pt(30.0, 40.0));
    let locked = line(&d, 20, pt(100.0, 0.0), pt(130.0, 40.0));
    d.set_node_visible(&[hidden], false).unwrap();
    d.set_node_locked(&[locked], true).unwrap();
    let mut s = open(&d, true);
    s.set_tool(Tool::Measure);
    drag(&mut s, pt(100.3, 0.2), pt(129.8, 39.7), NONE, SHIFT);
    click(&mut s, pt(115.0, 20.0), NONE);
    let x = dims(&s);
    assert_eq!(x.len(), 1);
    assert!(
        x[0].linked && x[0].slots.nodes() == vec![locked],
        "locked objects can be measured"
    );
    // A hidden path's nodes are not snap targets and a dimension does not link to it.
    drag(&mut s, pt(0.3, 0.2), pt(29.8, 39.7), NONE, SHIFT);
    click(&mut s, pt(15.0, 20.0), NONE);
    let x = dims(&s);
    let newest = x
        .iter()
        .find(|d| !d.slots.nodes().contains(&locked))
        .unwrap();
    assert!(
        newest.slots.nodes().is_empty(),
        "free points, not the hidden path"
    );
    assert!((mm(newest.shown.value) - 50.0).abs() > 1e-6);
}

#[test]
fn c15_existing_dimensions_are_not_pick_targets() {
    let (mut s, _) = measuring(false);
    let _ = leave_fifty(&mut s);
    let l = labels(&s)[0].clone();
    let at = s.screen_to_document(l["x"].as_f64().unwrap(), l["y"].as_f64().unwrap());
    let n = dims(&s).len();
    // Shift-click on the dimension's text, then place: no dimension of a dimension.
    click(&mut s, at, NONE);
    click(&mut s, pt(at.x + 10.0, at.y), SHIFT);
    click(&mut s, pt(at.x + 5.0, at.y + 10.0), NONE);
    for x in dims(&s) {
        let nodes = x.slots.nodes();
        assert!(
            nodes.iter().all(|n| dims(&s).iter().all(|d| d.id != *n)),
            "a dimension refers to another dimension"
        );
    }
    assert!(dims(&s).len() <= n + 1);
}

// ---------------- live region ----------------

#[test]
fn live_region_speaks_on_pin_and_on_placement_not_on_every_move() {
    let (mut s, _) = measuring(false);
    hover(&mut s, pt(10.0, 10.0), NONE);
    down(&mut s, pt(10.0, 10.0), NONE);
    hover(&mut s, pt(40.0, 50.0), NONE);
    assert_eq!(s.speech_for_test_or_view(), "", "nothing while dragging");
    up(&mut s, pt(40.0, 50.0), NONE);
    assert_eq!(s.speech_for_test_or_view(), "50 millimetres, 53.13 degrees");
    key(&mut s, "Escape", NONE);
    drag(&mut s, pt(10.0, 10.0), pt(40.0, 50.0), NONE, SHIFT);
    click(&mut s, pt(25.0, 30.0), NONE);
    assert_eq!(s.speech_for_test_or_view(), "Dimension 50 millimetres");
}

// ---------------- white box: degenerate input ----------------

#[test]
fn edge_a_zero_length_measurement_and_non_finite_pointer_do_not_write_or_panic() {
    let (mut s, _) = measuring(false);
    let before = s.pack("t").unwrap();
    drag(&mut s, pt(5.0, 5.0), pt(5.0, 5.0), NONE, NONE);
    let _ = readout(&s);
    hover(&mut s, pt(f64::NAN, 1.0), NONE);
    down(&mut s, pt(f64::INFINITY, 1.0), SHIFT);
    up(&mut s, pt(1.0, f64::NAN), SHIFT);
    let _ = view(&s);
    assert_eq!(s.pack("t").unwrap(), before);
    // Shift at a zero-length second pick: no zero dimension with a NaN.
    drag(&mut s, pt(5.0, 5.0), pt(5.0, 5.0), NONE, SHIFT);
    click(&mut s, pt(8.0, 8.0), NONE);
    for x in dims(&s) {
        assert!(x.shown.is_finite());
    }
}

#[test]
fn edge_huge_coordinates_stay_finite_in_the_readout() {
    let (mut s, _) = measuring(false);
    drag(&mut s, pt(-1e7, -1e7), pt(1e7, 1e7), NONE, NONE);
    if let Some(lines) = readout(&s) {
        assert!(
            !lines.iter().any(|l| l.contains("NaN") || l.contains("inf")),
            "{lines:?}"
        );
    }
}
