//! Session-level acceptance tests of `specs/0021-ellipse-arcs-and-shaping/specification.md`
//! (handle drags, the Select bar commands, history, conversion, clones, mirror, resize),
//! written from the spec against the public API of `curvyo-editor-wasm`.

#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::similar_names,
    missing_docs
)]

use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    Angle, ArcKind, Curve, Document, EllipseArc, EllipseFrame, InnerRatio, Length, ObjectSnapshot,
    Point, PointCount, Shape, StarFrame, Timestamp, outline_of_rotated, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::{
    ArcEnd, EntryOutcome, FlipAxis, ParamHandle, TransformHandleTolerances, oriented_bounds,
    param_handles,
};

const DEG: f64 = std::f64::consts::PI / 180.0;

static NOW: AtomicU64 = AtomicU64::new(1_800_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(7, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn ang(d: f64) -> Angle {
    Angle::from_radians(d * DEG)
}

fn ell(rx: f64, ry: f64, arc: Option<EllipseArc>, curve: f64) -> Shape {
    Shape::Ellipse {
        frame: EllipseFrame {
            center: pt(100.0, 80.0),
            rx: Length::from_mm(rx),
            ry: Length::from_mm(ry),
        },
        arc,
        curve: Curve::new(curve).unwrap(),
    }
}

fn arc(s: f64, e: f64, k: ArcKind) -> EllipseArc {
    EllipseArc::new(ang(s), ang(e), k).unwrap()
}

fn sframe(r: f64) -> StarFrame {
    StarFrame {
        center: pt(100.0, 80.0),
        radius: Length::from_mm(r),
        angle: ang(-90.0),
    }
}

fn open(shapes: &[(Shape, f64)]) -> Session {
    let d = Document::new(1);
    for (shape, rot) in shapes {
        let _ = d.create_primitive(*shape, ang(*rot));
    }
    let mut s = Session::open(2, &curvyo_document_core::pack(&d, "0.1.0").unwrap()).unwrap();
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    s
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn prim(s: &Session, i: usize) -> curvyo_document_core::PrimitiveSnapshot {
    let d = doc(s);
    d.primitive(d.object_ids()[i]).unwrap()
}

fn shape(s: &Session, i: usize) -> Shape {
    prim(s, i).shape
}

fn click(s: &mut Session, p: Point) {
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

/// Selects object 0 by clicking at `p` (a point on its drawn outline).
fn select_at(s: &mut Session, p: Point) {
    click(s, p);
    assert_eq!(s.selected_object_count(), 1, "selected by a click at {p:?}");
}

/// A double-click: the first press and release, then the session's `double_click`.
fn dbl(s: &mut Session, p: Point) -> bool {
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    s.pointer_hover(p, false, false);
    let hint = s.double_click(p, false, false);
    s.pointer_hover(p, false, false);
    hint
}

fn dist(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Selects object 0 by clicking its outline at the 135 degree spot (clear of every knob),
/// for an unrotated ellipse of radii (rx, ry) at (100, 80).
fn select_ellipse(s: &mut Session, rx: f64, ry: f64) {
    let p = pt(
        100.0 - rx * std::f64::consts::FRAC_1_SQRT_2,
        80.0 + ry * std::f64::consts::FRAC_1_SQRT_2,
    );
    click(s, p);
    assert_eq!(s.selected_object_count(), 1, "the ellipse is selected");
}

fn knobs(s: &Session, i: usize) -> Vec<(ParamHandle, Point)> {
    let d = doc(s);
    let o = d.object(d.object_ids()[i]).unwrap();
    param_handles(
        &o,
        &oriented_bounds(&o),
        &TransformHandleTolerances::at_scale(s.view().scale()),
    )
}

fn knob(s: &Session, h: ParamHandle) -> Point {
    knobs(s, 0)
        .iter()
        .find(|(x, _)| *x == h)
        .unwrap_or_else(|| panic!("{h:?} not drawn"))
        .1
}

fn drag(s: &mut Session, from: Point, to: Point, ctrl: bool) {
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, ctrl);
    s.pointer_up(to, false, ctrl);
}

/// The pointer at parametric angle `deg` of the unrotated ellipse at (100, 80), scaled
/// out from the centre by `far`.
fn at_angle(rx: f64, ry: f64, deg: f64, far: f64) -> Point {
    pt(
        100.0 + far * rx * (deg * DEG).cos(),
        80.0 + far * ry * (deg * DEG).sin(),
    )
}

fn arc_of(s: &Session) -> Option<EllipseArc> {
    match shape(s, 0) {
        Shape::Ellipse { arc, .. } => arc,
        o => panic!("{o:?}"),
    }
}

fn deg(a: Angle) -> f64 {
    a.as_radians() / DEG
}

// ---- 2, 5, 59: dragging an arc handle -----------------------------------------------------------

#[test]
fn c2_dragging_an_arc_handle_on_a_whole_ellipse_makes_an_open_arc_in_one_step() {
    let mut s = open(&[(ell(30.0, 30.0, None, 1.0), 0.0)]);
    select_ellipse(&mut s, 30.0, 30.0);
    let steps = s.undo_len();
    let end = knob(&s, ParamHandle::ArcEnd);
    // pointer far outside the circle: the angle is still atan2 (criterion 2)
    drag(&mut s, end, at_angle(30.0, 30.0, 200.0, 2.5), false);
    let a = arc_of(&s).expect("an arc");
    assert_eq!(
        a.kind,
        ArcKind::Arc,
        "criterion 3: a new arc is an open arc"
    );
    assert!(
        (deg(a.end()) - 200.0).abs() < 1e-6,
        "end follows the pointer: {}",
        deg(a.end())
    );
    assert!(
        (deg(a.start()) - 45.0).abs() < 1e-6,
        "the other angle rests at the 45 degree seam"
    );
    assert_eq!(s.undo_len(), steps + 1, "one commit");
    assert_eq!(
        s.history_target_name(curvyo_document_core::Direction::Undo)
            .as_deref(),
        Some("Change arc")
    );
    s.undo();
    assert_eq!(arc_of(&s), None, "undo gives the whole ellipse back");
    s.redo();
    assert_eq!(deg(arc_of(&s).unwrap().end()).round(), 200.0);
}

#[test]
fn c2_start_handle_and_inside_the_ellipse_pointer() {
    let mut s = open(&[(ell(40.0, 20.0, None, 1.0), 0.0)]);
    select_ellipse(&mut s, 40.0, 20.0);
    let start = knob(&s, ParamHandle::ArcStart);
    // inside: 0.4 of the radii at parametric angle 300
    drag(&mut s, start, at_angle(40.0, 20.0, 300.0, 0.4), false);
    let a = arc_of(&s).unwrap();
    assert!((deg(a.start()) - 300.0).abs() < 1e-6);
    assert!((deg(a.end()) - 45.0).abs() < 1e-6);
}

#[test]
fn c2_a_press_and_release_without_movement_and_escape_write_nothing() {
    let mut s = open(&[(ell(30.0, 30.0, None, 1.0), 0.0)]);
    select_ellipse(&mut s, 30.0, 30.0);
    let steps = s.undo_len();
    let before = s.pack("0.1.0").unwrap().len();
    let end = knob(&s, ParamHandle::ArcEnd);
    s.pointer_hover(end, false, false);
    s.pointer_down(end, false);
    s.pointer_hover(pt(end.x + 0.3, end.y + 0.2), false, false); // under 3 px
    s.pointer_up(pt(end.x + 0.3, end.y + 0.2), false, false);
    assert_eq!(arc_of(&s), None);
    assert_eq!(s.undo_len(), steps);
    // Escape during a drag
    s.pointer_hover(end, false, false);
    s.pointer_down(end, false);
    s.pointer_hover(at_angle(30.0, 30.0, 200.0, 1.0), false, false);
    s.escape();
    s.pointer_up(at_angle(30.0, 30.0, 200.0, 1.0), false, false);
    assert_eq!(arc_of(&s), None, "escape cancels");
    assert_eq!(s.undo_len(), steps);
    assert_eq!(s.pack("0.1.0").unwrap().len(), before);
}

#[test]
fn c5_c7_ctrl_snaps_the_dragged_angle_and_the_sweep_is_never_negative() {
    let mut s = open(&[(
        ell(30.0, 30.0, Some(arc(10.0, 280.0, ArcKind::Pie)), 1.0),
        0.0,
    )]);
    select_at(&mut s, at_angle(30.0, 30.0, 135.0, 1.0));
    // drag the end handle past the start handle to 340 with Ctrl: snaps to 337.5
    let end = knob(&s, ParamHandle::ArcEnd);
    drag(&mut s, end, at_angle(30.0, 30.0, 340.0, 1.0), true);
    let a = arc_of(&s).unwrap();
    assert!((deg(a.end()) - 337.5).abs() < 1e-6, "{}", deg(a.end()));
    assert!(deg(a.sweep()) > 0.0 && deg(a.sweep()) < 360.0);
    assert!(
        (deg(a.sweep()) - 327.5).abs() < 1e-6,
        "sweep from 10 to 337.5"
    );
    assert_eq!(a.kind, ArcKind::Pie, "the kind does not change");
    // dragging onto the start handle (within 0.1 degrees) makes the ellipse whole
    let end = knob(&s, ParamHandle::ArcEnd);
    drag(&mut s, end, at_angle(30.0, 30.0, 10.02, 1.0), false);
    assert_eq!(arc_of(&s), None, "criterion 5: within 0.1 degrees is whole");
    assert!(
        matches!(shape(&s, 0), Shape::Ellipse { .. }),
        "still an ellipse (criterion 63)"
    );
}

#[test]
fn c6_the_drag_readout_names_start_and_sweep() {
    let mut s = open(&[(
        ell(30.0, 30.0, Some(arc(30.0, 250.0, ArcKind::Arc)), 1.0),
        0.0,
    )]);
    select_at(&mut s, at_angle(30.0, 30.0, 135.0, 1.0));
    let start = knob(&s, ParamHandle::ArcStart);
    s.pointer_hover(start, false, false);
    s.pointer_down(start, false);
    s.pointer_hover(at_angle(30.0, 30.0, 60.0, 1.0), false, false);
    let readout = s.live_readout().expect("a readout during the drag").text;
    assert_eq!(readout, "start 60.0\u{b0} \u{b7} sweep 190.0\u{b0}");
    s.escape();
    s.pointer_up(at_angle(30.0, 30.0, 60.0, 1.0), false, false);
}

#[test]
fn c53_rotated_ellipse_the_handle_stays_under_the_pointer() {
    for rot in [30.0, 90.0, -45.0] {
        let mut s = open(&[(
            ell(40.0, 25.0, Some(arc(20.0, 250.0, ArcKind::Chord)), 1.0),
            rot,
        )]);
        // select by clicking the knob-free outline: the arc's middle
        let mid = ang(135.0);
        let local = pt(40.0 * mid.as_radians().cos(), 25.0 * mid.as_radians().sin());
        let at = pt(100.0 + local.x, 80.0 + local.y).rotated_around(pt(100.0, 80.0), ang(rot));
        click(&mut s, at);
        assert_eq!(s.selected_object_count(), 1, "rot {rot}");
        let h = knob(&s, ParamHandle::ArcEnd);
        // pointer at the same rotated position of parametric angle 200 (own frame)
        let target = pt(
            100.0 + 40.0 * (200.0 * DEG).cos(),
            80.0 + 25.0 * (200.0 * DEG).sin(),
        )
        .rotated_around(pt(100.0, 80.0), ang(rot));
        drag(&mut s, h, target, false);
        let a = arc_of(&s).unwrap();
        assert!(
            (deg(a.end()) - 200.0).abs() < 1e-6,
            "rot {rot}: {}",
            deg(a.end())
        );
        assert_eq!(prim(&s, 0).rotation, ang(rot), "only the arc is written");
    }
}

// ---- 23, 25, 26: the curve handle ------------------------------------------------------------------

#[test]
fn c23_dragging_the_curve_handle_follows_the_pointer_in_one_step() {
    for (rx, ry) in [(30.0, 30.0), (40.0, 20.0)] {
        let mut s = open(&[(ell(rx, ry, None, 1.0), 0.0)]);
        select_ellipse(&mut s, rx, ry);
        let steps = s.undo_len();
        let h = knob(&s, ParamHandle::Curve);
        let want = -0.5;
        let d = 0.5 * (1.0 + (22.5 * DEG * want).tan());
        let target = pt(100.0 + rx * d, 80.0 - ry * d);
        s.pointer_hover(h, false, false);
        s.pointer_down(h, false);
        s.pointer_hover(target, false, false);
        let readout = s.live_readout().unwrap().text;
        assert_eq!(readout, "curve -50.0 %");
        s.pointer_up(target, false, false);
        match shape(&s, 0) {
            Shape::Ellipse { curve, .. } => {
                assert!((curve.get() - want).abs() < 1e-6, "{}", curve.get())
            }
            o => panic!("{o:?}"),
        }
        assert_eq!(s.undo_len(), steps + 1);
        assert_eq!(
            s.history_target_name(curvyo_document_core::Direction::Undo)
                .as_deref(),
            Some("Change curve")
        );
    }
}

#[test]
fn c23_curve_drag_snaps_ctrl_snaps_and_a_tiny_move_writes_nothing() {
    let mut s = open(&[(ell(30.0, 30.0, None, 0.5), 0.0)]);
    let d0 = 0.5 * (1.0 + (22.5 * DEG * 0.5).tan());
    select_at(&mut s, pt(100.0 - 30.0 * d0, 80.0 + 30.0 * d0));
    let steps = s.undo_len();
    let h = knob(&s, ParamHandle::Curve);
    // press/release under 3 px
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    s.pointer_up(pt(h.x + 0.2, h.y), false, false);
    assert_eq!(s.undo_len(), steps);
    // ctrl: pointer for k = 0.37 snaps to 0.25
    let d = 0.5 * (1.0 + (22.5 * DEG * 0.37).tan());
    drag(&mut s, h, pt(100.0 + 30.0 * d, 80.0 - 30.0 * d), true);
    match shape(&s, 0) {
        Shape::Ellipse { curve, .. } => {
            assert!((curve.get() - 0.25).abs() < 1e-9, "{}", curve.get())
        }
        o => panic!("{o:?}"),
    }
    // no modifier: within 1 point of 0% snaps to exactly 0
    let h = knob(&s, ParamHandle::Curve);
    let d = 0.5 * (1.0 + (22.5 * DEG * 0.006).tan());
    drag(&mut s, h, pt(100.0 + 30.0 * d, 80.0 - 30.0 * d), false);
    match shape(&s, 0) {
        Shape::Ellipse { curve, .. } => assert_eq!(curve.get(), 0.0),
        o => panic!("{o:?}"),
    }
    assert!(
        matches!(shape(&s, 0), Shape::Ellipse { .. }),
        "0% ellipse stays an ellipse (criterion 63)"
    );
}

#[test]
fn c40_c41_polygon_and_star_curve_handle_drag() {
    for (label, base) in [
        (
            "hexagon",
            Shape::Polygon {
                frame: sframe(30.0),
                point_count: PointCount::new(6).unwrap(),
                curve: Curve::POLYGON_DEFAULT,
            },
        ),
        (
            "star",
            Shape::Star {
                frame: sframe(30.0),
                point_count: PointCount::new(5).unwrap(),
                inner_ratio: InnerRatio::new(0.45).unwrap(),
                curve: Curve::POLYGON_DEFAULT,
            },
        ),
    ] {
        let mut s = open(&[(base, 0.0)]);
        let o = outline_of_rotated(&base, ang(0.0));
        let far_mid = pt(
            f64::midpoint(o[2].point.x, o[3].point.x),
            f64::midpoint(o[2].point.y, o[3].point.y),
        );
        click(&mut s, far_mid);
        assert_eq!(s.selected_object_count(), 1, "{label}");
        let h = knob(&s, ParamHandle::Curve);
        // push the handle outward and inward along the bisector: curve 0 -> positive -> negative
        let c = pt(100.0, 80.0);
        let out = pt(c.x + (h.x - c.x) * 1.3, c.y + (h.y - c.y) * 1.3);
        drag(&mut s, h, out, false);
        let k_out = match shape(&s, 0) {
            Shape::Polygon { curve, .. } | Shape::Star { curve, .. } => curve.get(),
            o => panic!("{o:?}"),
        };
        assert!(k_out > 0.0, "{label}: outward is positive: {k_out}");
        let h2 = knob(&s, ParamHandle::Curve);
        let inn = pt(c.x + (h2.x - c.x) * 0.5, c.y + (h2.y - c.y) * 0.5);
        drag(&mut s, h2, inn, false);
        let k_in = match shape(&s, 0) {
            Shape::Polygon { curve, .. } | Shape::Star { curve, .. } => curve.get(),
            o => panic!("{o:?}"),
        };
        assert!(k_in < k_out, "{label}: inward is lower: {k_in}");
        assert_eq!(
            s.history_target_name(curvyo_document_core::Direction::Undo)
                .as_deref(),
            Some("Change curve"),
            "{label}"
        );
    }
}

// ---- 3, 26, 44, 52, 64, 65, 66: the Select bar ---------------------------------------------------------

#[test]
fn c3_bar_state_is_always_present_greyed_when_not_applicable() {
    // whole ellipse: group present; kind choice and whole button greyed (no arc)
    let mut s = open(&[(ell(30.0, 30.0, None, 1.0), 0.0)]);
    select_ellipse(&mut s, 30.0, 30.0);
    let st = s.select_bar_state();
    let e = st
        .ellipse
        .expect("the ellipse group is shown for a whole ellipse");
    assert!(
        e.kind_grey.is_some(),
        "kind and Whole ellipse greyed while there is no arc"
    );
    assert!(
        e.angles_grey.is_none(),
        "Start and End are live on a whole ellipse"
    );
    assert!(st.curve.is_some() && st.curve.unwrap().grey.is_none());
    // typed start on a whole ellipse: other angle 45, kind Arc (customer 2026-10-17)
    assert_eq!(
        s.set_selected_arc_angle_text(ArcEnd::Start, "-90"),
        EntryOutcome::Committed
    );
    let a = arc_of(&s).unwrap();
    assert_eq!(a.kind, ArcKind::Arc);
    assert!((deg(a.start()) - 270.0).abs() < 1e-9 && (deg(a.end()) - 45.0).abs() < 1e-9);
    let e = s.select_bar_state().ellipse.unwrap();
    assert!(e.kind_grey.is_none(), "kind choice now live");
    // curve field greyed while an arc exists (criterion 26/31)
    let c = s.select_bar_state().curve.unwrap();
    assert!(c.grey.is_some(), "Curve greyed for an ellipse with an arc");
    assert_eq!(s.set_selected_curve_text("30"), EntryOutcome::Unchanged);
    assert!(
        matches!(shape(&s, 0), Shape::Ellipse { curve, .. } if curve == Curve::ELLIPSE_DEFAULT)
    );
    // kind and whole
    assert_eq!(
        s.set_selected_arc_kind(ArcKind::Pie),
        EntryOutcome::Committed
    );
    assert_eq!(arc_of(&s).unwrap().kind, ArcKind::Pie);
    assert_eq!(
        s.set_selected_arc_angle_text(ArcEnd::End, "450"),
        EntryOutcome::Committed
    );
    assert!(
        (deg(arc_of(&s).unwrap().end()) - 90.0).abs() < 1e-9,
        "450 becomes 90"
    );
    assert_eq!(
        s.set_selected_arc_angle_text(ArcEnd::End, "abc"),
        EntryOutcome::Invalid {
            field: 0,
            reason: curvyo_ui_core::InvalidReason::NotANumber
        }
    );
    assert_eq!(
        s.set_selected_arc_angle_text(ArcEnd::End, "90"),
        EntryOutcome::Unchanged,
        "unchanged value writes nothing"
    );
    assert_eq!(s.make_selected_ellipses_whole(), EntryOutcome::Committed);
    assert_eq!(arc_of(&s), None);
    assert_eq!(s.make_selected_ellipses_whole(), EntryOutcome::Unchanged);
}

#[test]
fn c3_equal_angles_typed_make_the_ellipse_whole_in_one_commit() {
    let mut s = open(&[(
        ell(30.0, 30.0, Some(arc(10.0, 280.0, ArcKind::Chord)), 1.0),
        0.0,
    )]);
    select_at(&mut s, at_angle(30.0, 30.0, 135.0, 1.0));
    let steps = s.undo_len();
    assert_eq!(
        s.set_selected_arc_angle_text(ArcEnd::End, "10.05"),
        EntryOutcome::Committed
    );
    assert_eq!(arc_of(&s), None);
    assert_eq!(s.undo_len(), steps + 1);
}

#[test]
fn c26_c38_c44_the_curve_field_limits_per_shape_and_mixed_selections() {
    let tri = Shape::Polygon {
        frame: sframe(30.0),
        point_count: PointCount::new(3).unwrap(),
        curve: Curve::POLYGON_DEFAULT,
    };
    let hex = Shape::Polygon {
        frame: StarFrame {
            center: pt(200.0, 80.0),
            ..sframe(30.0)
        },
        point_count: PointCount::new(6).unwrap(),
        curve: Curve::POLYGON_DEFAULT,
    };
    let mut s = open(&[(tri, 0.0), (hex, 0.0)]);
    assert_eq!(
        s.key_down(curvyo_editor_wasm::KeyInput {
            key: "a",
            ctrl: true,
            ..Default::default()
        }),
        curvyo_editor_wasm::KeyOutcome::SelectedAll
    );
    assert_eq!(s.set_selected_curve_text("-100"), EntryOutcome::Committed);
    let k = |i| match shape(&s, i) {
        Shape::Polygon { curve, .. } => curve.get(),
        o => panic!("{o:?}"),
    };
    let (a, b) = (k(0), k(1));
    let (tri_k, hex_k) = if a > b { (a, b) } else { (b, a) };
    assert!(
        (tri_k + 0.5).abs() < 1e-9 && (hex_k + 1.0).abs() < 1e-9,
        "triangle stops at -50, hexagon goes to -100: {a} {b}"
    );
    // the field shows Mixed for different effective values
    let c = s.select_bar_state().curve.unwrap();
    assert_eq!(c.value, Some(curvyo_ui_core::BarValue::Mixed));
    // 0 resets to straight sides, one step for the selection
    let steps = s.undo_len();
    assert_eq!(s.set_selected_curve_text("0"), EntryOutcome::Committed);
    assert_eq!(
        s.undo_len(),
        steps + 1,
        "one commit for the whole selection"
    );
    // Decided 2026-10-19 (±400 %): a typed value above a shape's own outward limit is
    // limited per shape, the triangle to 250 %, the hexagon to 400 %.
    assert_eq!(s.set_selected_curve_text("500"), EntryOutcome::Committed);
    let k = |i| match shape(&s, i) {
        Shape::Polygon { curve, .. } => curve.get(),
        o => panic!("{o:?}"),
    };
    let (a, b) = (k(0), k(1));
    let (tri_k, hex_k) = if a < b { (a, b) } else { (b, a) };
    assert!(
        (tri_k - 2.5).abs() < 1e-9 && (hex_k - 4.0).abs() < 1e-9,
        "triangle stops at 250, hexagon at 400: {a} {b}"
    );
    assert_eq!(
        s.set_selected_curve_text(""),
        EntryOutcome::Invalid {
            field: 0,
            reason: curvyo_ui_core::InvalidReason::NotANumber
        }
    );
}

#[test]
fn c52_a_rectangle_in_the_selection_hides_the_curve_field_and_has_no_handle() {
    let d = Document::new(1);
    let _ = d.create_rect(curvyo_document_core::RectBounds::from_corners(
        pt(0.0, 0.0),
        pt(60.0, 60.0),
    ));
    let _ = d.create_primitive(ell(30.0, 30.0, None, 1.0), ang(0.0));
    let mut s = Session::open(2, &curvyo_document_core::pack(&d, "0.1.0").unwrap()).unwrap();
    s.set_clock(clock);
    s.set_tool(Tool::Select);
    s.key_down(curvyo_editor_wasm::KeyInput {
        key: "a",
        ctrl: true,
        ..Default::default()
    });
    let st = s.select_bar_state();
    assert!(st.curve.is_none(), "criterion 52");
    assert!(
        st.ellipse.is_some(),
        "the ellipse group is for any selection holding an ellipse"
    );
}

#[test]
fn c26_the_polite_announcements_name_the_new_value() {
    let mut s = open(&[(ell(30.0, 30.0, None, 1.0), 0.0)]);
    select_ellipse(&mut s, 30.0, 30.0);
    s.set_selected_arc_angle_text(ArcEnd::Start, "30");
    s.set_selected_arc_angle_text(ArcEnd::End, "150");
    s.set_selected_arc_kind(ArcKind::Pie);
    assert_eq!(
        s.take_transform_announcement(),
        "Arc start 30.0\u{b0}, end 150.0\u{b0}, pie"
    );
    s.make_selected_ellipses_whole();
    s.set_selected_curve_text("-35");
    let said = s.take_transform_announcement();
    assert_eq!(
        said, "Curve \u{2212}35.0%",
        "spec criterion 65 writes a true minus sign: {said:?}"
    );
}

// ---- 8, 24, 42: double-click entries; 16, 34, 35: tools; 58: hidden and locked ---------------------------------------

#[test]
fn c8_c24_double_click_on_a_knob_opens_its_field_and_enter_writes_one_commit() {
    // arc handle
    let mut s = open(&[(
        ell(30.0, 30.0, Some(arc(30.0, 250.0, ArcKind::Pie)), 1.0),
        0.0,
    )]);
    select_at(&mut s, at_angle(30.0, 30.0, 135.0, 1.0));
    let h = knob(&s, ParamHandle::ArcEnd);
    let tool = s.tool();
    dbl(&mut s, h);
    assert_eq!(s.tool(), tool, "no double-click switches a tool");
    let view = s
        .transform_entry()
        .expect("a field opens next to the handle");
    assert_eq!(view.fields.len(), 1);
    assert_eq!(
        view.fields[0].prefill, "250.0",
        "pre-filled with the handle's angle, one decimal"
    );
    assert!(
        view.fields[0].accessible_name.contains("Arc end")
            || view.fields[0].accessible_name.contains("end"),
        "{}",
        view.fields[0].accessible_name
    );
    // an unedited Enter writes nothing
    let steps = s.undo_len();
    assert_eq!(
        s.commit_transform_entry("250.0", "", 0),
        EntryOutcome::Unchanged
    );
    assert_eq!(s.undo_len(), steps);
    // invalid keeps it open
    dbl(&mut s, h);
    assert!(matches!(
        s.commit_transform_entry("x", "", 0),
        EntryOutcome::Invalid { .. }
    ));
    assert!(
        s.transform_entry().is_some(),
        "the field stays open and marked invalid"
    );
    assert_eq!(
        s.commit_transform_entry("-90", "", 0),
        EntryOutcome::Committed
    );
    assert_eq!(s.undo_len(), steps + 1);
    assert!((deg(arc_of(&s).unwrap().end()) - 270.0).abs() < 1e-9);
    // Escape closes without writing
    let h = knob(&s, ParamHandle::ArcStart);
    dbl(&mut s, h);
    assert!(s.transform_entry().is_some());
    s.cancel_transform_entry();
    assert!(s.transform_entry().is_none());
    assert_eq!(s.undo_len(), steps + 1);
}

#[test]
fn c24_c42_double_click_on_the_curve_knob_limits_and_names_the_field() {
    let tri = Shape::Polygon {
        frame: sframe(30.0),
        point_count: PointCount::new(3).unwrap(),
        curve: Curve::POLYGON_DEFAULT,
    };
    let mut s = open(&[(tri, 0.0)]);
    let o = outline_of_rotated(&tri, ang(0.0));
    select_at(
        &mut s,
        pt(
            f64::midpoint(o[0].point.x, o[1].point.x),
            f64::midpoint(o[0].point.y, o[1].point.y),
        ),
    );
    let h = knob(&s, ParamHandle::Curve);
    dbl(&mut s, h);
    let view = s.transform_entry().expect("curve field");
    assert_eq!(view.fields[0].accessible_name, "Curve");
    assert_eq!(view.fields[0].prefill, "0.0");
    assert_eq!(
        s.commit_transform_entry("-300", "", 0),
        EntryOutcome::Committed
    );
    assert!(
        matches!(shape(&s, 0), Shape::Polygon { curve, .. } if (curve.get() + 0.5).abs() < 1e-9),
        "a triangle is limited to -50%"
    );
    let h = knob(&s, ParamHandle::Curve);
    dbl(&mut s, h);
    assert_eq!(
        s.commit_transform_entry("999", "", 0),
        EntryOutcome::Committed
    );
    assert!(
        matches!(shape(&s, 0), Shape::Polygon { curve, .. } if (curve.get() - 2.5).abs() < 1e-9),
        "limited to the triangle's 250% (decided 2026-10-19: ±400 %)"
    );
    assert!(
        matches!(shape(&s, 0), Shape::Polygon { .. }),
        "a curved polygon stays a polygon (criterion 63)"
    );
}

#[test]
fn c16_c34_c35_new_shapes_from_the_tools_use_the_defaults() {
    let mut s = Session::new(11);
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    for (tool, from, to) in [
        (Tool::Ellipse, pt(10.0, 10.0), pt(80.0, 60.0)),
        (Tool::PolygonStar, pt(150.0, 40.0), pt(190.0, 40.0)),
    ] {
        s.set_tool(tool);
        s.pointer_hover(from, false, false);
        s.pointer_down(from, false);
        s.pointer_hover(to, false, false);
        s.pointer_up(to, false, false);
    }
    // shift and ctrl variants of the ellipse tool
    s.set_tool(Tool::Ellipse);
    for (shift, ctrl) in [(true, false), (false, true), (true, true)] {
        let (a, b) = (pt(10.0, 100.0), pt(70.0, 150.0));
        s.modifiers_changed(shift, ctrl, false);
        s.pointer_hover(a, shift, ctrl);
        s.pointer_down(a, shift);
        s.pointer_hover(b, shift, ctrl);
        s.pointer_up(b, shift, ctrl);
        s.modifiers_changed(false, false, false);
    }
    let d = doc(&s);
    let mut seen = 0;
    for id in d.object_ids() {
        match d.primitive(id).unwrap().shape {
            Shape::Ellipse { arc, curve, .. } => {
                assert_eq!(arc, None, "criterion 16");
                assert_eq!(curve, Curve::ELLIPSE_DEFAULT, "criterion 34");
                seen += 1;
            }
            Shape::Polygon { curve, .. } | Shape::Star { curve, .. } => {
                assert_eq!(curve, Curve::POLYGON_DEFAULT, "criterion 35");
                seen += 1;
            }
            Shape::Rect { .. } => {}
        }
    }
    assert!(seen >= 3, "{seen} shapes were created");
}

#[test]
fn c58_hidden_and_locked_ellipses_are_not_selectable_and_the_bar_skips_them() {
    let d = Document::new(1);
    let hidden = d.create_primitive(
        ell(30.0, 30.0, Some(arc(0.0, 270.0, ArcKind::Pie)), 1.0),
        ang(0.0),
    );
    let locked = d.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: pt(200.0, 80.0),
                rx: Length::from_mm(30.0),
                ry: Length::from_mm(30.0),
            },
            arc: None,
            curve: Curve::new(0.3).unwrap(),
        },
        ang(0.0),
    );
    d.set_node_visible(&[hidden], false).unwrap();
    d.set_node_locked(&[locked], true).unwrap();
    let mut s = Session::open(2, &curvyo_document_core::pack(&d, "0.1.0").unwrap()).unwrap();
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    click(&mut s, at_angle(30.0, 30.0, 90.0, 1.0));
    assert_eq!(s.selected_object_count(), 0, "hidden: not selectable");
    click(&mut s, pt(200.0 - 30.0 * 0.5, 80.0));
    assert_eq!(s.selected_object_count(), 0, "locked: not selectable");
    s.key_down(curvyo_editor_wasm::KeyInput {
        key: "a",
        ctrl: true,
        ..Default::default()
    });
    let st = s.select_bar_state();
    assert!(
        st.ellipse.is_none() && st.curve.is_none(),
        "no bar control reaches them"
    );
    assert_eq!(s.set_selected_curve_text("50"), EntryOutcome::Unchanged);
    assert_eq!(
        s.set_selected_arc_angle_text(ArcEnd::Start, "10"),
        EntryOutcome::Unchanged
    );
}

// ---- 12: hit test ------------------------------------------------------------------------------------------------

#[test]
fn c12_only_the_drawn_outline_is_clickable() {
    // lower half circle (0 -> 180 passes 6 o'clock) as an open arc
    let mut s = open(&[(
        ell(30.0, 30.0, Some(arc(0.0, 180.0, ArcKind::Arc)), 1.0),
        0.0,
    )]);
    click(&mut s, at_angle(30.0, 30.0, 270.0, 1.0)); // the missing top
    assert_eq!(
        s.selected_object_count(),
        0,
        "the missing part is not clickable"
    );
    click(&mut s, at_angle(30.0, 30.0, 90.0, 1.0)); // the drawn bottom
    assert_eq!(s.selected_object_count(), 1);
    // a pie's straight edge to the centre is clickable
    let mut s = open(&[(
        ell(30.0, 30.0, Some(arc(0.0, 180.0, ArcKind::Pie)), 1.0),
        0.0,
    )]);
    click(&mut s, pt(100.0 + 15.0, 80.0 - 0.0));
    assert_eq!(s.selected_object_count(), 1, "a pie's radius line is hit");
}

// ---- 54: flip ------------------------------------------------------------------------------------------------------------

fn param_point(shape: Shape, rot: f64, t: f64) -> Point {
    let Shape::Ellipse { frame, .. } = shape else {
        panic!()
    };
    let p = curvyo_document_core::ellipse_point(frame, t);
    p.rotated_around(frame.center, ang(rot))
}

/// Flips `before_shape` (rotation `rot`) on `axis`; returns the box centre before, the
/// shape after, and the sampled old points with the new points they must equal.
fn flip_case(
    before_shape: Shape,
    rot: f64,
    axis: FlipAxis,
) -> (
    Point,
    curvyo_document_core::PrimitiveSnapshot,
    Vec<(Point, Point)>,
    Session,
) {
    let mut s = open(&[(before_shape, rot)]);
    let o = outline_of_rotated(&before_shape, ang(rot));
    select_at(&mut s, o[1].point);
    let d0 = doc(&s);
    let b = oriented_bounds(&d0.object(d0.object_ids()[0]).unwrap());
    let bc = b.to_document(pt(
        f64::midpoint(b.min.x, b.max.x),
        f64::midpoint(b.min.y, b.max.y),
    ));
    s.flip_selection(axis);
    let after = prim(&s, 0);
    let Shape::Ellipse { arc: Some(na), .. } = after.shape else {
        panic!("stays an ellipse with an arc")
    };
    assert_eq!(na.kind, ArcKind::Pie);
    assert!((deg(na.sweep()) - 170.0).abs() < 1e-6, "sweep unchanged");
    let sweep = 170.0 * DEG;
    let arot = after.rotation.as_radians() / DEG;
    // the image of the old start is the new end (a reflection reverses the direction)
    let pairs = (0..64)
        .map(|i| {
            let f = f64::from(i) / 63.0;
            let old = param_point(before_shape, rot, 30.0 * DEG + sweep * f);
            let new = param_point(after.shape, arot, na.end().as_radians() - sweep * f);
            (old, new)
        })
        .collect();
    (bc, after, pairs, s)
}

#[test]
fn c54_flip_mirrors_the_arc_exactly_and_twice_is_identity() {
    for rot in [0.0, 30.0, 90.0] {
        for axis in [FlipAxis::Horizontal, FlipAxis::Vertical] {
            let before_shape = ell(40.0, 20.0, Some(arc(30.0, 200.0, ArcKind::Pie)), 1.0);
            let (_, _, pairs, mut s) = flip_case(before_shape, rot, axis);
            // pivot-agnostic: all 64 samples are the mirror image about ONE axis-aligned line
            let line = |(o, n): &(Point, Point)| match axis {
                FlipAxis::Horizontal => (f64::midpoint(o.x, n.x), (o.y - n.y).abs()),
                FlipAxis::Vertical => (f64::midpoint(o.y, n.y), (o.x - n.x).abs()),
            };
            let first = line(&pairs[0]);
            for (i, pair) in pairs.iter().enumerate() {
                let l = line(pair);
                assert!(
                    (l.0 - first.0).abs() < 1e-9 && l.1 < 1e-9,
                    "rot {rot} {axis:?} sample {i}"
                );
            }
            // flip twice: the drawn points are back within 1e-9 mm
            s.flip_selection(axis);
            let back = prim(&s, 0);
            let Shape::Ellipse { arc: Some(ba), .. } = back.shape else {
                panic!()
            };
            let brot = back.rotation.as_radians() / DEG;
            let sweep = 170.0 * DEG;
            for i in 0..64 {
                let f = f64::from(i) / 63.0;
                let was = param_point(before_shape, rot, 30.0 * DEG + sweep * f);
                let now = param_point(back.shape, brot, ba.start().as_radians() + sweep * f);
                assert!(
                    dist(was, now) < 1e-9,
                    "rot {rot} {axis:?} twice, sample {i}: {was:?} {now:?}"
                );
            }
        }
    }
}

/// The mirror line is the line through the middle of the box around the drawn part
/// (`0056` criterion 21 with `0021` criterion 9): the flipped arc keeps its box.
#[test]
fn c54_c9_flip_is_about_the_exact_box_centre_of_the_drawn_part() {
    for rot in [0.0, 30.0, 90.0] {
        for axis in [FlipAxis::Horizontal, FlipAxis::Vertical] {
            let before_shape = ell(40.0, 20.0, Some(arc(30.0, 200.0, ArcKind::Pie)), 1.0);
            let (bc, _, pairs, _) = flip_case(before_shape, rot, axis);
            for (i, (old, new)) in pairs.iter().enumerate() {
                let image = match axis {
                    FlipAxis::Horizontal => Point::new(2.0 * bc.x - old.x, old.y),
                    FlipAxis::Vertical => Point::new(old.x, 2.0 * bc.y - old.y),
                };
                assert!(
                    dist(image, *new) < 1e-9,
                    "rot {rot} {axis:?} sample {i}: image {image:?} new {new:?}"
                );
            }
        }
    }
}

// ---- 10, 27: resize keeps arc and curve ------------------------------------------------------------------------------------

#[test]
fn c10_c27_a_resize_drag_keeps_the_angles_and_the_curve() {
    for (label, e) in [
        (
            "arc",
            ell(40.0, 40.0, Some(arc(20.0, 250.0, ArcKind::Pie)), 1.0),
        ),
        ("curve", ell(40.0, 40.0, None, -0.4)),
    ] {
        let mut s = open(&[(e, 0.0)]);
        let o = outline_of_rotated(&e, ang(0.0));
        let _ = o;
        // select by clicking the outline at 135 degrees (drawn for both)
        click(&mut s, at_angle(40.0, 40.0, 135.0, 1.0));
        if s.selected_object_count() == 0 {
            // a curved ellipse's outline is not the circle: click the diamond-ish side's middle
            let d = 0.5 * (1.0 + (22.5 * DEG * -0.4).tan());
            click(&mut s, pt(100.0 - 40.0 * d, 80.0 + 40.0 * d));
        }
        assert_eq!(s.selected_object_count(), 1, "{label}");
        let d = doc(&s);
        let obj = d.object(d.object_ids()[0]).unwrap();
        let b = oriented_bounds(&obj);
        // drag the bottom-right corner resize handle outwards (non-uniform)
        let corner = b.to_document(pt(b.max.x, b.max.y));
        drag(&mut s, corner, pt(corner.x + 25.0, corner.y + 5.0), false);
        match (shape(&s, 0), e) {
            (
                Shape::Ellipse {
                    arc: a1,
                    curve: c1,
                    frame: f1,
                },
                Shape::Ellipse {
                    arc: a0,
                    curve: c0,
                    frame: f0,
                },
            ) => {
                assert_eq!(
                    a1.map(|a| (a.start(), a.end(), a.kind)),
                    a0.map(|a| (a.start(), a.end(), a.kind)),
                    "{label}: angles unchanged"
                );
                assert_eq!(c1, c0, "{label}: curve unchanged");
                assert!(
                    f1.rx.as_mm() != f0.rx.as_mm() || f1.ry.as_mm() != f0.ry.as_mm(),
                    "{label}: it did resize"
                );
            }
            o => panic!("{o:?}"),
        }
    }
}

// ---- 13, 30, 55: Object to path and linked clone ----------------------------------------------------------------------------

#[test]
fn c13_object_to_path_of_an_arc_is_the_drawn_outline() {
    let mut s = open(&[(
        ell(30.0, 30.0, Some(arc(0.0, 180.0, ArcKind::Arc)), 1.0),
        0.0,
    )]);
    click(&mut s, at_angle(30.0, 30.0, 90.0, 1.0));
    assert_eq!(s.selected_object_count(), 1);
    s.convert_selected_to_paths();
    let d = doc(&s);
    let o = d.object(d.object_ids()[0]).unwrap();
    match o {
        ObjectSnapshot::Path(p) => {
            let subs: Vec<_> = p.subpaths().collect();
            assert_eq!(subs.len(), 1);
            assert!(!subs[0].closed, "an Arc becomes an open path");
            assert_eq!(subs[0].anchors.len(), 3);
        }
        o => panic!("{o:?}"),
    }
    assert!(
        s.history_target_name(curvyo_document_core::Direction::Undo)
            .is_some_and(|name| !name.is_empty()),
        "the conversion is a named step"
    );
}

#[test]
fn c55_linked_clone_of_an_arc_converts_the_original_and_shows_no_arc_controls() {
    let mut s = open(&[(
        ell(30.0, 30.0, Some(arc(0.0, 270.0, ArcKind::Pie)), 1.0),
        0.0,
    )]);
    click(&mut s, at_angle(30.0, 30.0, 90.0, 1.0));
    assert_eq!(s.selected_object_count(), 1);
    assert!(s.linked_clone_shown());
    s.linked_clone();
    let d = doc(&s);
    assert_eq!(d.object_ids().len(), 2);
    for id in d.object_ids() {
        assert!(
            d.primitive(id).is_none(),
            "the primitive was converted, the clone is a clone"
        );
    }
    // the selection is the clone: no ellipse group and no Curve field
    let st = s.select_bar_state();
    assert!(st.ellipse.is_none() && st.curve.is_none(), "criterion 55");
}

#[test]
fn c55_linked_clone_of_every_arc_kind_and_of_a_curved_shape_converts_without_notice() {
    let shapes = [
        ell(30.0, 30.0, Some(arc(0.0, 270.0, ArcKind::Arc)), 1.0),
        ell(30.0, 30.0, Some(arc(0.0, 270.0, ArcKind::Chord)), 1.0),
        ell(30.0, 30.0, None, -0.4),
        Shape::Polygon {
            frame: sframe(30.0),
            point_count: PointCount::new(5).unwrap(),
            curve: Curve::new(0.5).unwrap(),
        },
    ];
    for shape in shapes {
        let mut s = open(&[(shape, 0.0)]);
        let o = outline_of_rotated(&shape, ang(0.0));
        select_at(&mut s, o[1].point);
        assert!(s.linked_clone_shown(), "{shape:?}");
        s.linked_clone();
        let d = doc(&s);
        assert_eq!(d.object_ids().len(), 2, "{shape:?}: original and clone");
        assert!(
            d.object_ids()
                .iter()
                .all(|id| d.primitive(id.to_owned()).is_none()),
            "{shape:?}: both are paths/clones"
        );
        assert!(
            s.take_clone_repair_notice().is_empty() && s.take_delete_notice().is_empty(),
            "no notice (0049 criterion 2)"
        );
    }
}

// ---- 14, 59: history, save ---------------------------------------------------------------------------------------------------------

#[test]
fn c14_the_session_save_round_trips_arc_and_curve() {
    let mut s = open(&[
        (
            ell(30.0, 30.0, Some(arc(12.5, 301.25, ArcKind::Chord)), 1.0),
            20.0,
        ),
        (ell(30.0, 30.0, None, -0.37), 0.0),
    ]);
    let before: Vec<Shape> = (0..2).map(|i| shape(&s, i)).collect();
    let bytes = s.pack("0.1.0").unwrap();
    let s2 = Session::open(5, &bytes).unwrap();
    let after: Vec<Shape> = (0..2).map(|i| shape(&s2, i)).collect();
    assert_eq!(before, after);
    let _ = &mut s;
}

// ---- 56: group stretch and skew ---------------------------------------------------------------------------------------------------

/// A 40 x 40 mm rectangle at the origin beside an ellipse with an arc turned by `rot`; both
/// selected. The rectangle's corner is the group's top-left, so a corner drag scales about
/// (0, 0) by exactly (new width / 40, new height / 40).
fn rect_and_arc(rot: f64) -> (Session, Shape) {
    let d = Document::new(1);
    let _ = d.create_rect(curvyo_document_core::RectBounds::from_corners(
        pt(0.0, 0.0),
        pt(40.0, 40.0),
    ));
    let e = ell(30.0, 30.0, Some(arc(0.0, 180.0, ArcKind::Arc)), 1.0);
    let _ = d.create_primitive(e, ang(rot));
    let mut s = Session::open(2, &curvyo_document_core::pack(&d, "0.1.0").unwrap()).unwrap();
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    click(&mut s, pt(0.0, 20.0));
    let o = outline_of_rotated(&e, ang(rot));
    let on_arc = o[o.len() / 2].point;
    s.pointer_hover(on_arc, true, false);
    s.pointer_down(on_arc, true);
    s.pointer_up(on_arc, true, false);
    assert_eq!(s.selected_object_count(), 2);
    (s, e)
}

fn group_box(e: &Shape, rot: f64) -> (Point, Point) {
    let o = outline_of_rotated(e, ang(rot));
    let mut min = pt(0.0, 0.0);
    let mut max = pt(40.0, 40.0);
    for a in o.iter() {
        for q in [
            a.point,
            pt(a.point.x + a.handle_out.x, a.point.y + a.handle_out.y),
        ] {
            min = pt(min.x.min(q.x), min.y.min(q.y));
            max = pt(max.x.max(q.x), max.y.max(q.y));
        }
    }
    (min, max)
}

#[test]
fn c56_a_group_stretch_keeps_a_rotated_arc_an_arc_and_it_is_exact() {
    for rot in [0.0, 30.0] {
        let (mut s, e) = rect_and_arc(rot);
        let (min, max) = group_box(&e, rot);
        let corner = pt(max.x, max.y);
        let target = pt(
            max.x + 0.6 * (max.x - min.x),
            max.y - 0.35 * (max.y - min.y),
        );
        drag(&mut s, corner, target, false);
        let d = doc(&s);
        let ids = d.object_ids();
        let rect = match d.object(ids[0]).unwrap() {
            ObjectSnapshot::Primitive(p) => match p.shape {
                Shape::Rect { bounds, .. } => bounds,
                o => panic!("{o:?}"),
            },
            o => panic!("{o:?}"),
        };
        let (sx, sy) = (rect.width.as_mm() / 40.0, rect.height.as_mm() / 40.0);
        assert!(
            (sx - 1.0).abs() > 0.1 && (sy - 1.0).abs() > 0.1,
            "it did stretch: {sx} {sy}"
        );
        let after = d
            .primitive(ids[1])
            .expect("the ellipse stays a primitive (customer 2026-10-17)");
        let Shape::Ellipse { arc: Some(na), .. } = after.shape else {
            panic!("still an ellipse with an arc")
        };
        assert_eq!(na.kind, ArcKind::Arc);
        assert!(
            (deg(na.sweep()) - 180.0).abs() < 1e-6,
            "rot {rot}: sweep unchanged"
        );
        let arot = after.rotation.as_radians() / DEG;
        for i in 0..64 {
            let f = f64::from(i) / 63.0;
            let old = param_point(e, rot, 180.0 * DEG * f);
            let image = pt(old.x * sx, old.y * sy);
            let det = sx * sy;
            let t = if det > 0.0 {
                na.start().as_radians() + 180.0 * DEG * f
            } else {
                na.end().as_radians() - 180.0 * DEG * f
            };
            let new = param_point(after.shape, arot, t);
            assert!(
                dist(image, new) < 1e-9,
                "rot {rot} sample {i}: {image:?} vs {new:?}"
            );
        }
    }
}

fn skew_case(kind: ArcKind, via_chip: bool) {
    let d = Document::new(1);
    let _ = d.create_rect(curvyo_document_core::RectBounds::from_corners(
        pt(0.0, 0.0),
        pt(40.0, 40.0),
    ));
    let e = ell(30.0, 30.0, Some(arc(0.0, 180.0, kind)), 1.0);
    let _ = d.create_primitive(e, ang(0.0));
    let mut s = Session::open(2, &curvyo_document_core::pack(&d, "0.1.0").unwrap()).unwrap();
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    s.key_down(curvyo_editor_wasm::KeyInput {
        key: "a",
        ctrl: true,
        ..Default::default()
    });
    assert_eq!(s.selected_object_count(), 2);
    let steps = s.undo_len();
    if via_chip {
        s.key_down(curvyo_editor_wasm::KeyInput {
            key: "k",
            ..Default::default()
        });
        assert_eq!(
            s.commit_transform_entry("30", "", 0),
            EntryOutcome::Committed
        );
    } else {
        let (min, max) = group_box(&e, 0.0);
        let handle = pt(f64::midpoint(min.x, max.x), min.y - 16.0 / s.view().scale());
        s.pointer_hover(handle, false, false);
        assert!(
            s.cursor_hint().starts_with("skew"),
            "a skew handle: {}",
            s.cursor_hint()
        );
        s.pointer_down(handle, false);
        for i in 1..=6 {
            s.pointer_hover(pt(handle.x + 5.0 * f64::from(i), handle.y), false, false);
        }
        s.pointer_up(pt(handle.x + 30.0, handle.y), false, false);
    }
    let d = doc(&s);
    let still: Vec<_> = d
        .object_ids()
        .into_iter()
        .filter(|id| d.primitive(*id).is_some())
        .collect();
    assert!(
        still.is_empty(),
        "{kind:?} chip={via_chip}: a skew converts every primitive of the selection to a path (criterion 56, 0047); still primitives: {still:?} (undo steps {steps} -> {})",
        s.undo_len()
    );
    assert_eq!(s.undo_len(), steps + 1, "one commit");
    assert!(
        s.select_bar_state().ellipse.is_none(),
        "the bar's ellipse group no longer applies to a path"
    );
}

#[test]
fn c56_a_skew_converts_a_pie_ellipse_in_a_multi_selection_by_drag() {
    skew_case(ArcKind::Pie, false);
}

#[test]
fn c56_a_skew_converts_a_pie_ellipse_in_a_multi_selection_by_chip() {
    skew_case(ArcKind::Pie, true);
}

#[test]
fn c56_a_skew_converts_an_open_arc_ellipse_in_a_multi_selection_by_drag() {
    skew_case(ArcKind::Arc, false);
}

#[test]
fn c56_a_skew_converts_an_open_arc_ellipse_in_a_multi_selection_by_chip() {
    skew_case(ArcKind::Arc, true);
}

#[test]
fn c56_a_skew_converts_a_single_open_arc_ellipse() {
    for kind in [ArcKind::Pie, ArcKind::Chord, ArcKind::Arc] {
        let mut s = open(&[(ell(30.0, 30.0, Some(arc(0.0, 180.0, kind)), 1.0), 0.0)]);
        s.key_down(curvyo_editor_wasm::KeyInput {
            key: "a",
            ctrl: true,
            ..Default::default()
        });
        s.key_down(curvyo_editor_wasm::KeyInput {
            key: "k",
            ..Default::default()
        });
        let steps = s.undo_len();
        s.commit_transform_entry("30", "", 0);
        let d = doc(&s);
        assert!(
            d.primitive(d.object_ids()[0]).is_none() && s.undo_len() == steps + 1,
            "{kind:?}: the ellipse becomes a path on a skew"
        );
    }
}

// ---- 60: Boolean on arcs ----------------------------------------------------------------------------------------------------------------

fn path_area(p: &curvyo_document_core::PathSnapshot) -> f64 {
    let mut total = 0.0;
    for sub in p.subpaths() {
        let a = sub.anchors;
        let mut pts = Vec::new();
        for i in 0..a.len() {
            let (u, v) = (&a[i], &a[(i + 1) % a.len()]);
            for k in 0..24 {
                let t = f64::from(k) / 24.0;
                let w = [
                    (1.0 - t).powi(3),
                    3.0 * (1.0 - t).powi(2) * t,
                    3.0 * (1.0 - t) * t * t,
                    t.powi(3),
                ];
                pts.push((
                    w[0] * u.point.x
                        + w[1] * (u.point.x + u.handle_out.x)
                        + w[2] * (v.point.x + v.handle_in.x)
                        + w[3] * v.point.x,
                    w[0] * u.point.y
                        + w[1] * (u.point.y + u.handle_out.y)
                        + w[2] * (v.point.y + v.handle_in.y)
                        + w[3] * v.point.y,
                ));
            }
        }
        let mut sum = 0.0;
        for i in 0..pts.len() {
            let (q, r) = (pts[i], pts[(i + 1) % pts.len()]);
            sum += q.0 * r.1 - r.0 * q.1;
        }
        total += sum.abs() / 2.0;
    }
    total
}

#[test]
fn c60_boolean_union_of_a_pie_and_a_curved_hexagon_uses_the_drawn_outline() {
    let pie = ell(30.0, 30.0, Some(arc(0.0, 270.0, ArcKind::Pie)), 1.0);
    let hex = Shape::Polygon {
        frame: StarFrame {
            center: pt(130.0, 80.0),
            ..sframe(25.0)
        },
        point_count: PointCount::new(6).unwrap(),
        curve: Curve::new(-0.6).unwrap(),
    };
    let mut s = open(&[(pie, 0.0), (hex, 0.0)]);
    click(&mut s, at_angle(30.0, 30.0, 135.0, 1.0));
    let o = outline_of_rotated(&hex, ang(0.0));
    s.pointer_hover(o[1].point, true, false);
    s.pointer_down(o[1].point, true);
    s.pointer_up(o[1].point, true, false);
    assert_eq!(s.selected_object_count(), 2);
    let area_pie = std::f64::consts::PI * 900.0 * 0.75;
    let outcome = s.apply_boolean(curvyo_ui_core::BooleanOp::Union);
    assert!(
        matches!(outcome, curvyo_editor_wasm::BooleanOutcome::Applied { .. }),
        "{outcome:?}"
    );
    let d = doc(&s);
    assert_eq!(d.object_ids().len(), 1);
    let ObjectSnapshot::Path(p) = d.object(d.object_ids()[0]).unwrap() else {
        panic!()
    };
    let a = path_area(&p);
    assert!(
        a >= area_pie - 1.0,
        "A union B covers at least the pie: {a} vs {area_pie}"
    );
}

// ---- 62c: one frame of a curve drag on a 1024-gon (release build) -----------------------------------------------------------------

#[test]
#[ignore = "benchmark: run in release"]
fn c62c_a_curve_drag_frame_on_a_1024_gon_takes_at_most_4_ms() {
    let big = Shape::Polygon {
        frame: sframe(60.0),
        point_count: PointCount::new(1024).unwrap(),
        curve: Curve::new(0.3).unwrap(),
    };
    let mut s = open(&[(big, 0.0)]);
    let o = outline_of_rotated(&big, ang(0.0));
    click(&mut s, o[10].point);
    assert_eq!(s.selected_object_count(), 1);
    let h = knob(&s, ParamHandle::Curve);
    s.pointer_hover(h, false, false);
    s.pointer_down(h, false);
    let c = pt(100.0, 80.0);
    let mut worst: f64 = 0.0;
    let mut total = 0.0;
    let n = 60;
    for i in 0..n {
        let f = 1.0 + 0.2 * f64::from(i) / f64::from(n);
        let p = pt(c.x + (h.x - c.x) * f, c.y + (h.y - c.y) * f);
        let t = std::time::Instant::now();
        s.pointer_hover(p, false, false);
        let _ = s.live_readout();
        let ms = t.elapsed().as_secs_f64() * 1e3;
        worst = worst.max(ms);
        total += ms;
    }
    s.escape();
    assert!(
        total / f64::from(n) <= 4.0,
        "mean {} ms, worst {worst} ms",
        total / f64::from(n)
    );
}
