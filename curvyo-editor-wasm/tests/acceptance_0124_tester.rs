//! Independent tester tests for `specs/0124-gradient-fill` through `Session`, driven as the wasm
//! host drives it: the Fill switch and its first gradient, the Gradient block JSON (Kind, Angle,
//! Radius, stop rows, Add, Reverse, Mixed), the canvas handles (drag, Shift, Ctrl, Escape,
//! double-click adds a stop, Delete, entry boxes), the hit priority against the shape's resize
//! handles (8 px rule, 24 px rule), undo of every edit with its History name, transforms and
//! operations that keep the picture (criteria 15, 19), copies and clones, the draw list (open
//! paths, laser grey, thumbnails) and the SVG import notice. Written from the specification
//! and the ADRs; only public names and JSON field names were looked up.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
#![allow(clippy::many_single_char_names, clippy::items_after_statements)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, missing_docs, clippy::doc_markdown)]

use curvyo_document_core::{
    AnchorId, BoxPoint, Color, Document, DocumentSize, FillPaint, GradientGeometry, GradientStop,
    GradientStops, Length, NewAnchor, NewOutputPage, NodeId, ObjectSnapshot, PlacedGradient, Point,
    PremultipliedRgba8, RectBounds, StopOffset, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};
use curvyo_ui_core::{BooleanOp, FlipAxis};
use serde_json::Value;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn anchors(first: u64, points: &[(f64, f64)]) -> Vec<NewAnchor> {
    points
        .iter()
        .zip(first..)
        .map(|((x, y), k)| NewAnchor::corner(AnchorId::new(1, k), pt(*x, *y)))
        .collect()
}

/// An irregular closed quad (so a fitted affine is unique).
fn quad(d: &Document, first: u64, dx: f64, dy: f64) -> NodeId {
    d.create_path(
        &anchors(
            first,
            &[
                (dx, dy),
                (dx + 100.0, dy + 5.0),
                (dx + 90.0, dy + 40.0),
                (dx + 8.0, dy + 35.0),
            ],
        ),
        true,
    )
}

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

fn stop(r: u8, g: u8, b: u8, a: u8, offset: f64) -> GradientStop {
    GradientStop {
        colour: Color { r, g, b },
        alpha: a,
        offset: StopOffset::new(offset).unwrap(),
    }
}

fn shade(d: &Document, id: NodeId, geometry: GradientGeometry, stops: Vec<GradientStop>) {
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::GradientGeometry(geometry))
        .unwrap();
    d.edit_style(
        &[id],
        &StyleEdit::GradientStops(GradientStops::new(stops).unwrap()),
    )
    .unwrap();
}

fn rainbow() -> Vec<GradientStop> {
    vec![
        stop(0, 0, 0, 255, 0.0),
        stop(255, 0, 0, 255, 0.3),
        stop(0, 255, 0, 200, 0.6),
        stop(255, 255, 255, 255, 1.0),
    ]
}

fn slanted() -> GradientGeometry {
    GradientGeometry::Linear {
        start: BoxPoint::new(0.1, 0.2).unwrap(),
        end: BoxPoint::new(0.8, 0.9).unwrap(),
    }
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "t").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    let mut sn = s.snapping();
    sn.enabled = false;
    s.set_snapping(sn);
    s.set_tool(Tool::Select);
    s.wipe_history();
    s
}

fn hold(s: &mut Session, at: Point, shift: bool, ctrl: bool) {
    s.modifiers_changed(shift, ctrl, false);
    s.pointer_hover(at, shift, ctrl);
}

fn click(s: &mut Session, p: Point) {
    hold(s, p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn drag_mod(s: &mut Session, from: Point, to: Point, shift: bool, ctrl: bool) {
    hold(s, from, shift, ctrl);
    s.pointer_down(from, shift);
    hold(
        s,
        pt(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y)),
        shift,
        ctrl,
    );
    hold(s, to, shift, ctrl);
    s.pointer_up(to, shift, ctrl);
    hold(s, to, false, false);
}

fn drag(s: &mut Session, from: Point, to: Point) {
    drag_mod(s, from, to, false, false);
}

fn key(s: &mut Session, k: &str) {
    s.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    });
}

fn ctrl_key(s: &mut Session, k: &str) {
    s.key_down(KeyInput {
        key: k,
        ctrl: true,
        ..KeyInput::default()
    });
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("t").unwrap()).unwrap()
}

fn obj(s: &Session, id: NodeId) -> ObjectSnapshot {
    doc(s).object(id).expect("object exists")
}

fn gradient_of(o: &ObjectSnapshot) -> Option<curvyo_document_core::GradientFill> {
    o.style().fill.painted_gradient().cloned()
}

fn placed(o: &ObjectSnapshot) -> PlacedGradient {
    PlacedGradient::new(
        o.style().fill.painted_gradient().expect("a gradient"),
        &o.gradient_box().expect("a box"),
    )
}

fn view(s: &Session) -> Value {
    serde_json::to_value(s.gradient_view()).unwrap()
}

fn handles(s: &Session) -> Value {
    serde_json::to_value(s.gradient_handles()).unwrap()
}

/// Document point of handle `i`.
fn handle_doc(s: &Session, i: usize) -> Point {
    let h = s.gradient_handles();
    let hd = &h.handles[i];
    s.screen_to_document(hd.x, hd.y)
}

fn history_names(s: &mut Session) -> Vec<String> {
    let rows: Value = serde_json::from_str(&s.history_rows_json(0, 500)).unwrap();
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn k(s: &Session) -> f64 {
    s.view().scale()
}

fn ids(s: &Session) -> Vec<NodeId> {
    doc(s).object_ids()
}

fn same_colour(a: PremultipliedRgba8, b: PremultipliedRgba8, tol: i32) -> bool {
    [(a.r, b.r), (a.g, b.g), (a.b, b.b), (a.a, b.a)]
        .iter()
        .all(|(x, y)| (i32::from(*x) - i32::from(*y)).abs() <= tol)
}

/// A session with one 100 x 40 rectangle at (10, 10), selected, Colour fill on.
fn one_rect() -> (Session, NodeId) {
    let d = Document::new(1);
    let id = rect(&d, 10.0, 10.0, 100.0, 40.0);
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(
        &[id],
        &StyleEdit::FillColor(Color {
            r: 200,
            g: 40,
            b: 10,
        }),
    )
    .unwrap();
    let mut s = open(&d);
    click(&mut s, pt(60.0, 10.0));
    assert_eq!(s.selected_object_count(), 1);
    (s, id)
}

fn shaded_rect() -> (Session, NodeId) {
    let (mut s, id) = one_rect();
    assert!(s.set_fill_kind("gradient"));
    (s, id)
}

fn style_bytes(s: &Session, id: NodeId) -> String {
    format!("{:?}", obj(s, id).style())
}

// ---------- criteria 1 to 3: the switch, the first gradient, the block ----------

#[test]
fn c2_choosing_gradient_starts_linear_from_the_colour_to_white_in_one_step() {
    let (mut s, id) = one_rect();
    let before = s.undo_len();
    assert!(
        view(&s)["offered"].as_bool().unwrap(),
        "the Gradient item is offered"
    );
    assert!(s.set_fill_kind("gradient"));
    assert_eq!(s.undo_len(), before + 1, "one undo step");
    assert!(
        history_names(&mut s).iter().any(|n| n == "Fill: Gradient"),
        "{:?}",
        history_names(&mut s)
    );
    let g = gradient_of(&obj(&s, id)).unwrap();
    assert_eq!(g.geometry, GradientGeometry::DEFAULT_LINEAR);
    assert_eq!(
        g.stops.as_slice()[0].colour,
        Color {
            r: 200,
            g: 40,
            b: 10
        }
    );
    assert_eq!(
        g.stops.as_slice()[1].colour,
        Color {
            r: 255,
            g: 255,
            b: 255
        }
    );
    assert_eq!(g.stops.as_slice()[1].alpha, 255);
    let v = view(&s);
    assert!(v["shown"].as_bool().unwrap());
    assert_eq!(v["kind"], "linear");
    assert_eq!(v["stops"].as_array().unwrap().len(), 2);
    assert!(v["stops"][0]["fixed"].as_bool().unwrap() && v["stops"][1]["fixed"].as_bool().unwrap());
    assert!(
        v["angleShown"]
            .as_bool()
            .unwrap_or(v["angle_shown"].as_bool().unwrap_or(false)),
        "{v}"
    );
    // Away and back keeps it with no undo needed.
    s.add_gradient_stop();
    let edited = gradient_of(&obj(&s, id)).unwrap();
    for away in ["solid", "pattern", "none"] {
        assert!(s.set_fill_kind(away) || away == "none");
        assert!(s.set_fill_kind("gradient"));
        assert_eq!(
            gradient_of(&obj(&s, id)).unwrap(),
            edited,
            "back from {away}"
        );
    }
}

#[test]
fn c3_angle_wraps_radius_must_be_positive_positions_clamp_and_reverse_mirrors() {
    let (mut s, id) = shaded_rect();
    assert!(s.set_gradient_angle_text("190").is_ok());
    let a = view(&s)["angle"].as_str().unwrap().to_owned();
    assert!(a.starts_with("-170"), "190 wraps to -170: {a}");
    assert!(s.set_gradient_angle_text("-540").is_ok());
    let a = view(&s)["angle"].as_str().unwrap().to_owned();
    assert!(a.starts_with("180") || a.starts_with("-180"), "{a}");
    // Angle turns about the midpoint and keeps the length.
    assert!(s.set_gradient_angle_text("90").is_ok());
    let g = gradient_of(&obj(&s, id)).unwrap();
    let GradientGeometry::Linear { start, end } = g.geometry else {
        panic!()
    };
    let (sx, sy) = (10.0 + start.x.get() * 100.0, 10.0 + start.y.get() * 40.0);
    let (ex, ey) = (10.0 + end.x.get() * 100.0, 10.0 + end.y.get() * 40.0);
    assert!(
        (f64::midpoint(sx, ex) - 60.0).abs() < 1e-6 && (f64::midpoint(sy, ey) - 30.0).abs() < 1e-6
    );
    assert!(
        ((ex - sx).hypot(ey - sy) - 100.0).abs() < 1e-6,
        "length kept"
    );
    assert!((ex - sx).abs() < 1e-6, "vertical");
    // Positive counter-clockwise on the page: 90 points up (y decreases from start to end).
    assert!(ey < sy, "90° points up the page: start y {sy}, end y {ey}");
    assert!(s.set_gradient_kind("radial"));
    assert!(
        view(&s)["radiusShown"].as_bool().unwrap_or(false),
        "{}",
        view(&s)
    );
    assert!(s.set_gradient_radius_text("0").is_err(), "radius above 0");
    assert!(s.set_gradient_radius_text("-3").is_err());
    assert!(s.set_gradient_radius_text("25").is_ok());
    let p = placed(&obj(&s, id));
    assert!(
        (p.t_at(pt(60.0 + 25.0, 30.0)) - 1.0).abs() < 1e-6,
        "radius 25 mm"
    );
    assert!(s.add_gradient_stop());
    let _ = s.set_gradient_stop_position_text(1, "500");
    let st = gradient_of(&obj(&s, id)).unwrap().stops;
    assert!(st.as_slice()[1].offset.get() <= 1.0);
    assert!(s.set_gradient_stop_position_text(1, "30").is_ok());
    assert!(s.set_gradient_stop_hex(1, "#11223344").is_ok());
    assert!(
        s.set_gradient_stop_hex(1, "#1122334").is_err(),
        "7 hex digits are refused"
    );
    assert!(
        s.set_gradient_stop_hex(1, "#112233").is_ok(),
        "6 hex digits"
    );
    assert!(s.set_gradient_stop_hex(1, "#11223344").is_ok());
    s.reverse_gradient();
    let st = gradient_of(&obj(&s, id)).unwrap().stops;
    assert!(
        (st.as_slice()[1].offset.get() - 0.7).abs() < 1e-9,
        "{:?}",
        st
    );
    assert_eq!(
        st.as_slice()[1].colour,
        Color {
            r: 0x11,
            g: 0x22,
            b: 0x33
        }
    );
    assert_eq!(st.as_slice()[1].alpha, 0x44);
    // Remove only inner stops.
    assert!(!s.remove_gradient_stop(0));
    assert!(!s.remove_gradient_stop(2));
    assert!(s.remove_gradient_stop(1));
    assert!(!s.remove_gradient_stop(1), "down to two");
}

#[test]
fn c8_mixed_selection_shows_mixed_and_sets_on_all() {
    let d = Document::new(1);
    let a = rect(&d, 0.0, 0.0, 50.0, 30.0);
    let b = rect(&d, 100.0, 0.0, 50.0, 30.0);
    shade(&d, a, GradientGeometry::DEFAULT_LINEAR, rainbow());
    shade(
        &d,
        b,
        GradientGeometry::DEFAULT_RADIAL,
        vec![stop(0, 0, 0, 255, 0.0), stop(9, 9, 9, 255, 1.0)],
    );
    let mut s = open(&d);
    ctrl_key(&mut s, "a");
    assert_eq!(s.selected_object_count(), 2);
    let v = view(&s);
    assert!(v["mixedGradients"].as_bool().unwrap_or(false), "{v}");
    assert!(s.reverse_gradient(), "Reverse is offered");
    let ga = gradient_of(&obj(&s, a)).unwrap();
    let gb = gradient_of(&obj(&s, b)).unwrap();
    assert_eq!(
        ga.stops.as_slice()[0].colour,
        Color {
            r: 255,
            g: 255,
            b: 255
        },
        "each its own"
    );
    assert_eq!(gb.stops.as_slice()[0].colour, Color { r: 9, g: 9, b: 9 });
    assert!(
        !s.gradient_handles().handles.is_empty(),
        "handles for the first only"
    );
}

// ---------- criterion 18: undo of each edit with its name ----------

#[test]
fn c18_each_edit_is_one_named_step_and_undo_restores_exactly() {
    let (mut s, id) = one_rect();
    let mut states = vec![style_bytes(&s, id)];
    type Edit = Box<dyn Fn(&mut Session) -> bool>;
    let edits: Vec<(&str, Edit)> = vec![
        ("Fill: Gradient", Box::new(|s| s.set_fill_kind("gradient"))),
        (
            "Gradient: Radial",
            Box::new(|s| s.set_gradient_kind("radial")),
        ),
        (
            "Gradient: Linear",
            Box::new(|s| s.set_gradient_kind("linear")),
        ),
        (
            "Edit gradient",
            Box::new(|s| s.set_gradient_angle_text("33").is_ok()),
        ),
        ("Add gradient stop", Box::new(Session::add_gradient_stop)),
        (
            "Edit gradient",
            Box::new(|s| s.set_gradient_stop_hex(1, "#00FF0080").is_ok()),
        ),
        (
            "Edit gradient",
            Box::new(|s| s.set_gradient_stop_position_text(1, "20").is_ok()),
        ),
        ("Reverse gradient", Box::new(Session::reverse_gradient)),
        (
            "Remove gradient stop",
            Box::new(|s| s.remove_gradient_stop(1)),
        ),
    ];
    for (name, edit) in &edits {
        let n = s.undo_len();
        assert!(edit(&mut s), "{name} applies");
        assert_eq!(s.undo_len(), n + 1, "{name} is one step");
        let names = history_names(&mut s);
        assert!(names.iter().any(|x| x == name), "{name} in {names:?}");
        states.push(style_bytes(&s, id));
    }
    for i in (0..edits.len()).rev() {
        s.undo();
        assert_eq!(
            style_bytes(&s, id),
            states[i],
            "undo of {} restores",
            edits[i].0
        );
    }
    for i in 0..edits.len() {
        s.redo();
        assert_eq!(style_bytes(&s, id), states[i + 1], "redo of {}", edits[i].0);
    }
}

// ---------- criteria 6 and 7: handles ----------

#[test]
fn c6_handles_sit_on_the_line_and_an_end_drag_is_one_edit() {
    let (mut s, id) = shaded_rect();
    let h = handles(&s);
    assert!(h["shown"].as_bool().unwrap());
    let hs = h["handles"].as_array().unwrap();
    assert_eq!(hs.len(), 2);
    assert_eq!(hs[0]["kind"], "start");
    assert_eq!(hs[1]["kind"], "end");
    let a = handle_doc(&s, 0);
    let b = handle_doc(&s, 1);
    assert!(
        (a.x - 10.0).abs() < 1e-6 && (a.y - 30.0).abs() < 1e-6,
        "{a:?}"
    );
    assert!(
        (b.x - 110.0).abs() < 1e-6 && (b.y - 30.0).abs() < 1e-6,
        "{b:?}"
    );
    let n = s.undo_len();
    let rect_before = format!("{:?}", obj(&s, id).gradient_box());
    drag(&mut s, b, pt(90.0, 50.0));
    assert_eq!(s.undo_len(), n + 1);
    assert!(history_names(&mut s).iter().any(|x| x == "Edit gradient"));
    assert_eq!(
        format!("{:?}", obj(&s, id).gradient_box()),
        rect_before,
        "the shape did not resize"
    );
    let p = placed(&obj(&s, id));
    assert!((p.t_at(pt(90.0, 50.0)) - 1.0).abs() < 1e-6);
    assert!(p.t_at(pt(10.0, 30.0)).abs() < 1e-6);
}

#[test]
fn c6_escape_during_a_drag_restores() {
    let (mut s, id) = shaded_rect();
    let before = style_bytes(&s, id);
    let n = s.undo_len();
    let b = handle_doc(&s, 1);
    hold(&mut s, b, false, false);
    s.pointer_down(b, false);
    hold(&mut s, pt(70.0, 70.0), false, false);
    key(&mut s, "Escape");
    s.pointer_up(pt(70.0, 70.0), false, false);
    assert_eq!(style_bytes(&s, id), before);
    assert_eq!(s.undo_len(), n);
}

#[test]
fn c6_shift_locks_to_angle_stops_and_ctrl_moves_the_whole_line() {
    let (mut s, id) = shaded_rect();
    let b = handle_doc(&s, 1);
    // Aim at about 37 degrees from the Start at (10, 30) with Shift.
    let aim = pt(
        10.0 + 80.0 * 37f64.to_radians().cos(),
        30.0 - 80.0 * 37f64.to_radians().sin(),
    );
    drag_mod(&mut s, b, aim, true, false);
    let v = view(&s);
    let a: f64 = v["angle"]
        .as_str()
        .unwrap()
        .trim_end_matches('°')
        .trim()
        .parse()
        .unwrap();
    let ok = [0.0, 15.0, 22.5, 30.0, 45.0, 60.0, 67.5, 75.0, 90.0];
    assert!(ok.iter().any(|x| (a - x).abs() < 0.05), "Shift snaps {a}");
    // Ctrl drag of the Start moves both ends by the same amount.
    let g0 = placed(&obj(&s, id));
    let a0 = handle_doc(&s, 0);
    let b0 = handle_doc(&s, 1);
    drag_mod(&mut s, a0, pt(a0.x + 5.0, a0.y + 3.0), false, true);
    let a1 = handle_doc(&s, 0);
    let b1 = handle_doc(&s, 1);
    assert!(
        (a1.x - a0.x - 5.0).abs() < 1e-6 && (a1.y - a0.y - 3.0).abs() < 1e-6,
        "{a0:?} {a1:?}"
    );
    assert!(
        (b1.x - b0.x - 5.0).abs() < 1e-6 && (b1.y - b0.y - 3.0).abs() < 1e-6,
        "{b0:?} {b1:?}"
    );
    let g1 = placed(&obj(&s, id));
    assert!((g1.t_at(pt(a0.x + 5.0, a0.y + 3.0)) - g0.t_at(a0)).abs() < 1e-6);
}

#[test]
fn c6_radial_centre_drag_moves_the_circle() {
    let (mut s, id) = shaded_rect();
    assert!(s.set_gradient_kind("radial"));
    let c = handle_doc(&s, 0);
    let r = handle_doc(&s, 1);
    assert!(
        (c.x - 60.0).abs() < 1e-6 && (c.y - 30.0).abs() < 1e-6,
        "{c:?}"
    );
    assert!((r.x - 110.0).abs() < 1e-6, "{r:?}");
    drag(&mut s, c, pt(50.0, 25.0));
    let r2 = handle_doc(&s, 1);
    assert!(
        (r2.x - (r.x - 10.0)).abs() < 1e-6 && (r2.y - (r.y - 5.0)).abs() < 1e-6,
        "{r2:?}"
    );
    let p = placed(&obj(&s, id));
    assert!(p.t_at(pt(50.0, 25.0)).abs() < 1e-6);
}

#[test]
fn c6_double_click_on_the_line_adds_a_stop_and_delete_removes_it_never_the_shape() {
    let (mut s, id) = shaded_rect();
    let n = s.undo_len();
    s.double_click(pt(35.0, 30.0), false, false);
    let g = gradient_of(&obj(&s, id)).unwrap();
    assert_eq!(g.stops.len(), 3, "a stop added");
    assert!(
        (g.stops.as_slice()[1].offset.get() - 0.25).abs() < 0.02,
        "{:?}",
        g.stops
    );
    assert_eq!(s.undo_len(), n + 1);
    assert!(
        history_names(&mut s)
            .iter()
            .any(|x| x == "Add gradient stop")
    );
    // Shade colour at that place: a quarter from the Colour to white.
    let want_r = 200.0 + (255.0 - 200.0) * 0.25;
    assert!((f64::from(g.stops.as_slice()[1].colour.r) - want_r).abs() <= 3.0);
    // Delete with an end handle selected: nothing, the shape stays.
    let a = handle_doc(&s, 0);
    click(&mut s, a);
    key(&mut s, "Delete");
    assert!(
        doc(&s).object(id).is_some(),
        "Delete on an end never deletes the shape"
    );
    assert_eq!(gradient_of(&obj(&s, id)).unwrap().stops.len(), 3);
    // Press the inner stop dot, then Delete removes it.
    let h = s.gradient_handles();
    let inner = h
        .handles
        .iter()
        .position(|x| x.kind == "stop")
        .expect("an inner stop dot");
    let p = handle_doc(&s, inner);
    click(&mut s, p);
    assert_eq!(
        view(&s)["activeStop"].as_u64().unwrap_or(99),
        1,
        "pressing makes it active"
    );
    key(&mut s, "Delete");
    assert!(doc(&s).object(id).is_some());
    assert_eq!(gradient_of(&obj(&s, id)).unwrap().stops.len(), 2);
    assert!(
        history_names(&mut s)
            .iter()
            .any(|x| x == "Remove gradient stop")
    );
}

#[test]
fn c6_dragging_an_inner_stop_moves_its_position_clamped() {
    let (mut s, id) = shaded_rect();
    s.add_gradient_stop();
    s.add_gradient_stop();
    let g = gradient_of(&obj(&s, id)).unwrap();
    assert_eq!(g.stops.len(), 4);
    let h = s.gradient_handles();
    let dot = h
        .handles
        .iter()
        .position(|x| x.kind == "stop" && x.stop == 1)
        .unwrap();
    let p = handle_doc(&s, dot);
    drag(&mut s, p, pt(200.0, p.y));
    let g = gradient_of(&obj(&s, id)).unwrap();
    let o = g.stops.as_slice()[1].offset.get();
    let next = g.stops.as_slice()[2].offset.get();
    assert!(o <= next, "clamped by its neighbour: {o} {next}");
    assert!(GradientStops::new(g.stops.as_slice().to_vec()).is_ok());
}

#[test]
fn c7_entry_boxes_commit_once_and_escape_writes_nothing() {
    let (mut s, id) = shaded_rect();
    let b = handle_doc(&s, 1);
    s.double_click(b, false, false);
    let h = handles(&s);
    let e = &h["entry"];
    assert_eq!(e["kind"], "end", "{h}");
    assert_eq!(e["field"], "angle");
    let n = s.undo_len();
    let _ = s.commit_gradient_entry("45", "#0000FFFF");
    assert_eq!(s.undo_len(), n + 1, "one step");
    let v = view(&s);
    assert!(v["angle"].as_str().unwrap().starts_with("45"), "{v}");
    assert_eq!(
        gradient_of(&obj(&s, id)).unwrap().stops.as_slice()[1].colour,
        Color { r: 0, g: 0, b: 255 }
    );
    // Escape closes and writes nothing.
    let before = style_bytes(&s, id);
    let b = handle_doc(&s, 1);
    s.double_click(b, false, false);
    assert!(!handles(&s)["entry"].is_null());
    s.close_gradient_entry();
    assert!(handles(&s)["entry"].is_null());
    assert_eq!(style_bytes(&s, id), before);
    // Radial radius handle opens with Radius.
    assert!(s.set_gradient_kind("radial"));
    let r = handle_doc(&s, 1);
    s.double_click(r, false, false);
    assert_eq!(handles(&s)["entry"]["field"], "radius");
    s.close_gradient_entry();
    let c = handle_doc(&s, 0);
    s.double_click(c, false, false);
    assert_eq!(handles(&s)["entry"]["kind"], "stop");
    assert_eq!(handles(&s)["entry"]["field"], "");
}

// ---------- criterion 6: hit priority ----------

/// Width of the rect in the document (via its box).
fn width(s: &Session, id: NodeId) -> f64 {
    let b = obj(s, id).gradient_box().unwrap();
    b.max.x - b.min.x
}

#[test]
fn c6_gradient_handles_win_over_the_shape_handles_at_the_defaults() {
    // Control: with Colour, a drag at the right edge midpoint resizes.
    let (mut s, id) = one_rect();
    drag(&mut s, pt(110.0, 30.0), pt(130.0, 30.0));
    assert!(
        (width(&s, id) - 120.0).abs() < 1e-6,
        "a Colour shape resizes: {}",
        width(&s, id)
    );
    // With Gradient the same press moves the End handle and the shape keeps its size.
    let (mut s, id) = shaded_rect();
    drag(&mut s, pt(110.0, 30.0), pt(130.0, 30.0));
    assert!(
        (width(&s, id) - 100.0).abs() < 1e-6,
        "no resize: {}",
        width(&s, id)
    );
    let b = handle_doc(&s, 1);
    assert!((b.x - 130.0).abs() < 1e-6, "{b:?}");
    // The other handles still resize (a corner).
    drag(&mut s, pt(110.0, 50.0), pt(120.0, 50.0));
    assert!(
        (width(&s, id) - 110.0).abs() < 1e-6,
        "corner resizes: {}",
        width(&s, id)
    );
}

#[test]
fn c6_a_shape_handle_within_8_px_of_a_gradient_handle_is_not_hit_beyond_it_is() {
    for (offset_px, resizes) in [(6.0, false), (14.0, true)] {
        let (mut s, id) = shaded_rect();
        let kk = k(&s);
        // Move the End handle `offset_px` screen pixels up from the right edge midpoint.
        let e = handle_doc(&s, 1);
        drag(&mut s, e, pt(e.x, e.y - offset_px / kk));
        let w0 = width(&s, id);
        // Press exactly at the shape's right-mid resize handle and drag right.
        let g0 = gradient_of(&obj(&s, id)).unwrap();
        drag(&mut s, pt(110.0, 30.0), pt(110.0 + 30.0 / kk, 30.0));
        let w1 = width(&s, id);
        if resizes {
            assert!(
                w1 > w0 + 1e-6,
                "{offset_px} px away the shape handle resizes: {w0} -> {w1}"
            );
        } else {
            assert!(
                (w1 - w0).abs() < 1e-6,
                "{offset_px} px away the shape handle is not hit: {w0} -> {w1}"
            );
            let _ = g0;
        }
    }
}

#[test]
fn c6_handles_hidden_under_24_px_in_the_node_tool_and_when_not_gradient() {
    let (mut s, _) = shaded_rect();
    assert!(s.gradient_handles().shown);
    assert!(s.set_fill_kind("solid"));
    assert!(
        !s.gradient_handles().shown,
        "only while the switch is Gradient"
    );
    assert!(s.set_fill_kind("gradient"));
    s.set_tool(Tool::Node);
    assert!(!s.gradient_handles().shown, "not in the Node tool");
    s.set_tool(Tool::Select);
    // A short line: a rect whose width is under 24 px on screen.
    let d = Document::new(1);
    let probe = open(&d);
    let kk = k(&probe);
    let w = 20.0 / kk;
    let id = rect(&d, 10.0, 10.0, w, w * 3.0);
    shade(&d, id, GradientGeometry::DEFAULT_LINEAR, rainbow());
    let mut s = open(&d);
    click(&mut s, pt(10.0 + w / 2.0, 10.0));
    assert_eq!(s.selected_object_count(), 1);
    assert!(
        !s.gradient_handles().shown,
        "a line under 24 px hides the handles"
    );
}

// ---------- criterion 15: transforms keep the picture ----------

/// Fits the affine that maps the anchors of `a` onto the anchors of `b` (paths with the same
/// anchor order), as `[a b c d e f]` with x' = a x + c y + e, y' = b x + d y + f.
fn fit(a: &ObjectSnapshot, b: &ObjectSnapshot) -> [f64; 6] {
    let (ObjectSnapshot::Path(pa), ObjectSnapshot::Path(pb)) = (a, b) else {
        panic!("paths only")
    };
    let p: Vec<Point> = pa.anchors.iter().map(|x| x.point).collect();
    let q: Vec<Point> = pb.anchors.iter().map(|x| x.point).collect();
    // Solve from the first three anchors.
    let (x0, y0, x1, y1, x2, y2) = (p[0].x, p[0].y, p[1].x, p[1].y, p[2].x, p[2].y);
    let det = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0);
    let solve = |u0: f64, u1: f64, u2: f64| {
        let a = ((u1 - u0) * (y2 - y0) - (u2 - u0) * (y1 - y0)) / det;
        let c = ((x1 - x0) * (u2 - u0) - (x2 - x0) * (u1 - u0)) / det;
        (a, c, u0 - a * x0 - c * y0)
    };
    let (ma, mc, me) = solve(q[0].x, q[1].x, q[2].x);
    let (mb, md, mf) = solve(q[0].y, q[1].y, q[2].y);
    let m = [ma, mb, mc, md, me, mf];
    // The fourth anchor confirms it is affine.
    let r = apply(m, p[3]);
    assert!(
        (r.x - q[3].x).abs() < 1e-6 && (r.y - q[3].y).abs() < 1e-6,
        "not affine"
    );
    m
}

fn apply(m: [f64; 6], p: Point) -> Point {
    pt(
        m[0] * p.x + m[2] * p.y + m[4],
        m[1] * p.x + m[3] * p.y + m[5],
    )
}

fn assert_picture_follows(before: &ObjectSnapshot, after: &ObjectSnapshot, what: &str) {
    let m = fit(before, after);
    let (pb, pa) = (placed(before), placed(after));
    let bx = before.gradient_box().unwrap();
    for i in 0..9 {
        for j in 0..9 {
            let p = bx.frame_point(
                -0.2 + 1.4 * f64::from(i) / 8.0,
                -0.2 + 1.4 * f64::from(j) / 8.0,
            );
            let q = apply(m, p);
            let (tb, ta) = (pb.t_at(p), pa.t_at(q));
            assert!((tb - ta).abs() < 1e-6, "{what}: t at {p:?} {tb} -> {ta}");
            assert!(
                same_colour(pb.colour_at(p), pa.colour_at(q), 1),
                "{what}: colour at {p:?}"
            );
        }
    }
}

fn shaded_quad(geometry: GradientGeometry) -> (Session, NodeId) {
    let d = Document::new(1);
    let id = quad(&d, 1, 20.0, 20.0);
    shade(&d, id, geometry, rainbow());
    let mut s = open(&d);
    click(&mut s, pt(70.0, 22.5));
    assert_eq!(s.selected_object_count(), 1);
    (s, id)
}

#[test]
fn c15_mirror_scale_rotate_skew_and_move_keep_the_picture() {
    let elliptic = GradientGeometry::Radial {
        centre: BoxPoint::new(0.4, 0.6).unwrap(),
        radius: BoxPoint::new(0.9, 0.3).unwrap(),
        ratio: curvyo_document_core::AxisRatio::new(0.45).unwrap(),
    };
    for geometry in [slanted(), GradientGeometry::DEFAULT_RADIAL, elliptic] {
        // Mirror both ways.
        for axis in [FlipAxis::Horizontal, FlipAxis::Vertical] {
            let (mut s, id) = shaded_quad(geometry);
            let before = obj(&s, id);
            let _ = s.flip_selection(axis);
            assert_picture_follows(
                &before,
                &obj(&s, id),
                &format!("flip {axis:?} {geometry:?}"),
            );
        }
        // Handle drags: resize corner, rotate, skew; the box is the quad's box (20..120, 20..60).
        let kk;
        {
            let (s, _) = shaded_quad(geometry);
            kk = k(&s);
        }
        let (min, max) = (pt(20.0, 20.0), pt(120.0, 60.0));
        let rot = pt(
            max.x + 32.0 / std::f64::consts::SQRT_2 / kk,
            max.y + 32.0 / std::f64::consts::SQRT_2 / kk,
        );
        let skew = pt(f64::midpoint(min.x, max.x), min.y - 16.0 / kk);
        let cases: [(&str, Point, Point); 4] = [
            ("resize corner", max, pt(150.0, 90.0)),
            ("rotate", rot, pt(60.0, 140.0)),
            (
                "skew",
                skew,
                pt(f64::midpoint(min.x, max.x) + 25.0, min.y - 16.0 / kk),
            ),
            ("move", pt(70.0, 22.5), pt(10.0, 2.5)),
        ];
        for (what, from, to) in cases {
            let (mut s, id) = shaded_quad(geometry);
            let before = obj(&s, id);
            drag(&mut s, from, to);
            let after = obj(&s, id);
            assert_ne!(
                format!("{before:?}"),
                format!("{after:?}"),
                "{what} changed the shape"
            );
            assert_picture_follows(&before, &after, &format!("{what} {geometry:?}"));
        }
    }
}

#[test]
fn c15_a_node_edit_refits_the_fractions_to_the_new_box() {
    let (mut s, id) = shaded_quad(slanted());
    let g0 = gradient_of(&obj(&s, id)).unwrap();
    s.set_tool(Tool::Node);
    click(&mut s, pt(70.0, 22.5));
    drag(&mut s, pt(120.0, 25.0), pt(160.0, 25.0));
    let after = obj(&s, id);
    let b = after.gradient_box().unwrap();
    assert!(b.max.x > 150.0, "the node moved: {b:?}");
    assert_eq!(gradient_of(&after).unwrap(), g0, "the fractions stay");
}

#[test]
fn c15_a_rotated_primitive_keeps_its_picture_through_rotate_and_object_to_path() {
    let d = Document::new(1);
    let id = rect(&d, 20.0, 20.0, 80.0, 30.0);
    shade(&d, id, slanted(), rainbow());
    let o = d.object(id).unwrap();
    let c = pt(60.0, 35.0);
    let before = placed(&o);
    d.rotate_object(&o.rotated(c, curvyo_document_core::Angle::from_radians(0.7)))
        .unwrap();
    let r = d.object(id).unwrap();
    let after = placed(&r);
    let (sn, cs) = 0.7f64.sin_cos();
    let rot = |p: Point| {
        pt(
            c.x + (p.x - c.x) * cs - (p.y - c.y) * sn,
            c.y + (p.x - c.x) * sn + (p.y - c.y) * cs,
        )
    };
    for p in [
        pt(25.0, 22.0),
        pt(60.0, 35.0),
        pt(95.0, 48.0),
        pt(40.0, 45.0),
    ] {
        assert!(
            (before.t_at(p) - after.t_at(rot(p))).abs() < 1e-6,
            "rotate at {p:?}"
        );
    }
    // Object to path keeps the shade where it was.
    let mut s = open(&d);
    click(&mut s, rot(pt(60.0, 20.0)));
    assert_eq!(s.selected_object_count(), 1);
    let _ = s.object_to_path();
    let paths: Vec<ObjectSnapshot> = doc(&s)
        .read_objects()
        .objects
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::Path(_)))
        .cloned()
        .collect();
    assert_eq!(paths.len(), 1, "converted");
    let pp = placed(&paths[0]);
    for p in [
        pt(25.0, 22.0),
        pt(60.0, 35.0),
        pt(95.0, 48.0),
        pt(40.0, 45.0),
    ] {
        assert!(
            (after.t_at(rot(p)) - pp.t_at(rot(p))).abs() < 1e-6,
            "object to path at {p:?}"
        );
    }
}

// ---------- criteria 16, 17: copies and clones ----------

#[test]
fn c16_duplicate_copies_the_gradient_and_editing_the_copy_leaves_the_original() {
    let (mut s, id) = shaded_quad(slanted());
    let _ = s.duplicate_layer_target();
    let all = ids(&s);
    assert_eq!(all.len(), 2);
    let copy = *all.iter().find(|x| **x != id).unwrap();
    assert_eq!(gradient_of(&obj(&s, copy)), gradient_of(&obj(&s, id)));
    // The copy is selected: edit it.
    assert!(s.reverse_gradient());
    assert_ne!(gradient_of(&obj(&s, copy)), gradient_of(&obj(&s, id)));
    assert_eq!(
        gradient_of(&obj(&s, id)).unwrap().stops.as_slice()[0].colour,
        Color { r: 0, g: 0, b: 0 }
    );
}

#[test]
fn c17_a_linked_clone_shows_and_follows_its_original() {
    let (mut s, id) = shaded_quad(slanted());
    let r = s.linked_clone();
    let _ = r;
    let all = ids(&s);
    assert_eq!(all.len(), 2, "a clone was made");
    let clone = *all.iter().find(|x| **x != id).unwrap();
    let list = s.draw_list();
    assert_eq!(list.gradients().len(), 2, "the clone draws a gradient too");
    let pc = curvyo_render_core::placed_gradient(&obj(&s, clone)).expect("the clone's gradient");
    let po = curvyo_render_core::placed_gradient(&obj(&s, id)).unwrap();
    assert_eq!(pc.stops(), po.stops());
    // Edit the original: the clone follows.
    click(&mut s, pt(1000.0, 1000.0));
    let d = doc(&s);
    d.edit_style(&[id], &StyleEdit::ReverseGradient).unwrap();
    let pc2 = curvyo_render_core::placed_gradient(&d.object(clone).unwrap()).unwrap();
    assert_ne!(
        pc2.stops(),
        pc.stops(),
        "the clone follows the original's stops"
    );
}

// ---------- criterion 19: operations keep the shade ----------

#[test]
fn c19_union_keeps_the_bottom_gradient_with_the_picture_in_place() {
    let d = Document::new(1);
    let a = quad(&d, 1, 20.0, 20.0);
    let b = rect(&d, 60.0, 30.0, 100.0, 50.0);
    shade(&d, a, slanted(), rainbow());
    let before = placed(&d.object(a).unwrap());
    let mut s = open(&d);
    ctrl_key(&mut s, "a");
    assert_eq!(s.selected_object_count(), 2);
    let _ = s.apply_boolean(BooleanOp::Union);
    let read = doc(&s).read_objects();
    let shaded: Vec<&ObjectSnapshot> = read
        .objects
        .iter()
        .filter(|o| gradient_of(o).is_some())
        .collect();
    assert_eq!(shaded.len(), 1, "one result with the gradient");
    let after = placed(shaded[0]);
    let _ = b;
    for p in [
        pt(25.0, 25.0),
        pt(70.0, 35.0),
        pt(100.0, 40.0),
        pt(150.0, 70.0),
    ] {
        assert!(
            (before.t_at(p) - after.t_at(p)).abs() < 1e-6,
            "union at {p:?}"
        );
    }
}

#[test]
fn c19_combine_then_break_apart_continue_one_shade_over_the_pieces() {
    let d = Document::new(1);
    let a = quad(&d, 1, 20.0, 20.0);
    let b = quad(&d, 11, 200.0, 20.0);
    shade(&d, a, slanted(), rainbow());
    shade(&d, b, GradientGeometry::DEFAULT_RADIAL, rainbow());
    let mut s = open(&d);
    ctrl_key(&mut s, "a");
    let _ = s.apply_combine();
    let read = doc(&s).read_objects();
    assert_eq!(read.objects.len(), 1, "combined");
    let combined = placed(&read.objects[0]);
    let first = placed(&d.object(a).unwrap());
    for p in [pt(25.0, 25.0), pt(70.0, 35.0), pt(110.0, 30.0)] {
        assert!(
            (first.t_at(p) - combined.t_at(p)).abs() < 1e-6,
            "combine keeps the bottom's picture at {p:?}"
        );
    }
    let _ = s.apply_break_apart();
    let read = doc(&s).read_objects();
    assert_eq!(read.objects.len(), 2, "broken apart");
    for o in read.objects.iter() {
        let piece = placed(o);
        let bx = o.gradient_box().unwrap();
        for (u, v) in [(0.2, 0.3), (0.5, 0.5), (0.8, 0.7)] {
            let p = bx.frame_point(u, v);
            assert!(
                (piece.t_at(p) - combined.t_at(p)).abs() < 1e-6,
                "break apart continues at {p:?}"
            );
        }
    }
}

// ---------- criteria 9 to 11, 20: the draw list ----------

#[test]
fn c9_c20_closed_open_and_compound_shapes_draw_gradient_spans() {
    let d = Document::new(1);
    let a = quad(&d, 1, 0.0, 0.0);
    let b = d.create_path(
        &anchors(11, &[(200.0, 0.0), (300.0, 50.0), (200.0, 80.0)]),
        false,
    );
    shade(&d, a, slanted(), rainbow());
    shade(&d, b, GradientGeometry::DEFAULT_LINEAR, rainbow());
    let s = open(&d);
    let list = s.draw_list();
    assert_eq!(
        list.gradients().len(),
        2,
        "the open path's gradient is drawn too"
    );
    for span in list.gradients() {
        assert!(!span.vertices().is_empty());
    }
    let attrs = list.gradient_attributes();
    assert!(
        attrs.iter().any(|a| a[3] != 0.0),
        "gradient vertices are marked"
    );
    assert!(
        attrs.iter().any(|a| a[3] == 0.0),
        "solid vertices (the strokes) carry none"
    );
}

#[test]
fn c10_c11_a_laser_page_shows_grey_stops_on_canvas_and_thumbnail() {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let page = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let id = rect(&d, page.origin.x + 10.0, page.origin.y + 10.0, 100.0, 40.0);
    shade(&d, id, GradientGeometry::DEFAULT_LINEAR, rainbow());
    let drawing = rect(&d, 10.0, 10.0, 100.0, 40.0);
    shade(&d, drawing, GradientGeometry::DEFAULT_LINEAR, rainbow());
    let mut s = open(&d);
    assert_eq!(s.draw_list().gradients().len(), 2);
    assert!(s.activate_page(&page.id));
    // Observation: in the laser page's default "View: Operations" the unassigned gradient
    // shape draws no gradient span at all; the grey of criterion 10 shows in "View: Drawn".
    let in_operations_view = s.draw_list().gradients().len();
    let ops: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    if ops["operations_view"].as_bool().unwrap_or(true) {
        assert!(s.toggle_operations_view(&page.id), "switch to View: Drawn");
    }
    assert_eq!(
        s.draw_list().gradients().len(),
        2,
        "Drawn view draws both gradients (Operations view drew {in_operations_view})"
    );
    let grey = |stops: &[curvyo_render_core::StopTexel]| {
        stops.iter().all(|t| {
            let c = t.colour;
            (c[0] - c[1]).abs() < 1e-6 && (c[1] - c[2]).abs() < 1e-6
        })
    };
    let (thumb, _) = s.thumbnail_frame(200.0, 150.0);
    assert!(
        !thumb.gradients().is_empty(),
        "the thumbnail draws the gradient"
    );
    // The thumbnail list also carries the drawing page's artwork (off the laser page's area);
    // exactly one span, the laser page's shape, is grey.
    let spans: Vec<Vec<curvyo_render_core::StopTexel>> = thumb
        .gradients()
        .iter()
        .map(|g| {
            let r = g.stops();
            thumb.gradient_stops()[r.start as usize..r.end as usize].to_vec()
        })
        .collect();
    let grey_spans: Vec<_> = spans.iter().filter(|s| grey(s)).collect();
    assert_eq!(
        grey_spans.len(),
        1,
        "laser page thumbnail is grey: {spans:?}"
    );
    let laser_stops = grey_spans[0].clone();
    // Grey of red (0.3): 0.2126 * 255 = 54.2.
    assert!(
        laser_stops
            .iter()
            .any(|t| (t.colour[0] * 255.0 - 54.2).abs() < 1.5),
        "{laser_stops:?}"
    );
    let canvas = s.draw_list();
    let mut saw_grey = false;
    let mut saw_colour = false;
    for g in canvas.gradients() {
        let r = g.stops();
        let st = &canvas.gradient_stops()[r.start as usize..r.end as usize];
        if grey(st) {
            saw_grey = true;
        } else {
            saw_colour = true;
        }
    }
    let dump: Vec<_> = canvas
        .gradients()
        .iter()
        .map(|g| {
            let r = g.stops();
            canvas.gradient_stops()[r.start as usize..r.end as usize].to_vec()
        })
        .collect();
    assert!(
        saw_grey,
        "the laser page's gradient is grey on the canvas: {dump:?}"
    );
    assert!(saw_colour, "the drawing page's gradient keeps its colours");
    // The panel still shows the real colours.
    click(&mut s, pt(page.origin.x + 60.0, page.origin.y + 10.0));
    if s.selected_object_count() == 1 {
        let v = view(&s);
        assert_eq!(
            v["stops"][1]["hex"].as_str().unwrap().to_uppercase(),
            "#FF0000FF",
            "{v}"
        );
    }
}

// ---------- criterion 9: interior hit ----------

#[test]
fn c9_click_inside_selects_where_alpha_is_above_zero() {
    let d = Document::new(1);
    let id = rect(&d, 10.0, 10.0, 100.0, 40.0);
    shade(
        &d,
        id,
        GradientGeometry::DEFAULT_LINEAR,
        vec![
            stop(0, 0, 0, 255, 0.0),
            stop(0, 0, 0, 255, 0.5),
            stop(0, 0, 0, 0, 0.5),
            stop(0, 0, 0, 0, 1.0),
        ],
    );
    d.edit_style(&[id], &StyleEdit::StrokeEnabled(false))
        .unwrap();
    let mut s = open(&d);
    click(&mut s, pt(30.0, 30.0));
    assert_eq!(s.selected_object_count(), 1, "opaque half selects");
    click(&mut s, pt(500.0, 500.0));
    assert_eq!(s.selected_object_count(), 0);
    click(&mut s, pt(90.0, 30.0));
    assert_eq!(
        s.selected_object_count(),
        0,
        "transparent half does not select"
    );
}

// ---------- criterion 27: the SVG import notice ----------

#[test]
fn c27_one_reflect_and_two_focal_points_give_exactly_two_lines() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="200mm" height="100mm" viewBox="0 0 200 100">
<defs>
<linearGradient id="a" x1="0" y1="0" x2="1" y2="0" spreadMethod="reflect"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient>
<radialGradient id="b" cx="0.5" cy="0.5" r="0.5" fx="0.3" fy="0.4"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#000"/></radialGradient>
<radialGradient id="c" cx="0.5" cy="0.5" r="0.5" fx="0.6" fy="0.5"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#000"/></radialGradient>
</defs>
<rect x="0" y="0" width="50" height="50" fill="url(#a)"/>
<rect x="60" y="0" width="50" height="50" fill="url(#b)"/>
<rect x="120" y="0" width="50" height="50" fill="url(#c)"/>
</svg>"##;
    let d = Document::new(1);
    let mut s = open(&d);
    s.import_svg(svg, "t.svg").expect("imports");
    let notice = s.take_gradient_import_notice();
    let lines: Vec<&str> = notice.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines,
        vec![
            "1 gradient repeat was shown as a plain shade.",
            "2 focal points were read as the centre."
        ],
        "{notice:?}"
    );
    assert!(s.take_gradient_import_notice().is_empty(), "taken once");
    // Nothing lost: no notice.
    let plain = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><defs><linearGradient id="a"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs><rect width="50" height="50" fill="url(#a)"/></svg>"##;
    s.import_svg(plain, "p.svg").expect("imports");
    assert_eq!(s.take_gradient_import_notice(), "");
    let shaded = doc(&s)
        .read_objects()
        .objects
        .iter()
        .filter(|o| gradient_of(o).is_some())
        .count();
    assert_eq!(shaded, 4, "every gradient became a Gradient fill");
}

// ---------- criterion 21: text ----------

fn fonts_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn load_fonts(s: &mut Session) {
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes), "{file} loads");
    }
}

fn text_session() -> (Session, NodeId) {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces: Vec<(String, String, String, u32)> = index["faces"]
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
    let mut s = Session::new(21);
    s.resize_viewport(1600.0, 1000.0);
    s.set_font_list(curvyo_ui_core::FontBlock::Included, faces, Vec::new());
    load_fonts(&mut s);
    s.set_tool(Tool::Text);
    click(&mut s, pt(20.0, 40.0));
    assert!(s.text_editing());
    s.text_input(curvyo_ui_core::TextInput::Insert("Shade".to_owned()));
    load_fonts(&mut s);
    let _ = s.escape();
    load_fonts(&mut s);
    s.set_tool(Tool::Select);
    let id = ids(&s)[0];
    ctrl_key(&mut s, "a");
    assert_eq!(s.selected_object_count(), 1);
    (s, id)
}

#[test]
fn c21_a_text_takes_a_gradient_drawn_across_its_box_and_object_to_path_keeps_it() {
    let (mut s, id) = text_session();
    assert!(
        view(&s)["offered"].as_bool().unwrap(),
        "a text offers Gradient"
    );
    s.set_fill_paint(true);
    assert!(s.set_fill_kind("gradient"));
    let o = obj(&s, id);
    assert!(matches!(o, ObjectSnapshot::Text(_)));
    assert!(gradient_of(&o).is_some());
    let list = s.draw_list();
    assert_eq!(list.gradients().len(), 1, "the glyphs draw the gradient");
    let span = &list.gradients()[0];
    let attrs = list.gradient_attributes();
    // The text's box is laid out only in the session, so read the shade from the draw list: the
    // default Linear runs left to right across the text box, so u is affine in x alone.
    let verts: Vec<(Point, f64, f64)> = span
        .vertices()
        .map(|k| {
            (
                list.triangles[k].position,
                f64::from(attrs[k][0]),
                f64::from(attrs[k][1]),
            )
        })
        .collect();
    assert!(verts.len() > 30, "glyph triangles");
    let (lo, hi) = verts.iter().fold((f64::MAX, f64::MIN), |(a, b), v| {
        (a.min(v.0.x), b.max(v.0.x))
    });
    let (ulo, uhi) = verts
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(v.1), b.max(v.1)));
    assert!(
        ulo > -0.1 && uhi < 1.1 && uhi - ulo > 0.8,
        "one shade across the text: {ulo}..{uhi}"
    );
    let slope = (uhi - ulo) / (hi - lo);
    for v in &verts {
        let want = ulo + (v.0.x - lo) * slope;
        assert!(
            (v.1 - want).abs() < 1e-3,
            "u affine in x: {v:?} want {want}"
        );
    }
    let shade_t = |p: Point| ulo + (p.x - lo) * slope;
    let _ = s.object_to_path();
    let read = doc(&s).read_objects();
    let pieces: Vec<_> = read
        .objects
        .iter()
        .filter(|o| gradient_of(o).is_some())
        .cloned()
        .collect();
    assert!(
        pieces.len() >= 2,
        "the pieces keep the gradient: {}",
        pieces.len()
    );
    for piece in &pieces {
        let pp = placed(piece);
        let bx = piece.gradient_box().unwrap();
        for (u, v) in [(0.1, 0.5), (0.5, 0.5), (0.9, 0.2)] {
            let p = bx.frame_point(u, v);
            assert!(
                (pp.t_at(p) - shade_t(p)).abs() < 2e-3,
                "shade stays at {p:?}: {} vs {}",
                pp.t_at(p),
                shade_t(p)
            );
        }
    }
}
