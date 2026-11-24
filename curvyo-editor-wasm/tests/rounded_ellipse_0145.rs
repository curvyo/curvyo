//! Session-level tests of `specs/0145-shape-spiro-handles/specification.md` slice 2 (the Rounded
//! handle of the ellipse, criteria 14, 15, 16 and 19), against the public API of
//! `curvyo-editor-wasm`.

#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

mod cpu_time;

use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    Angle, Curve, Document, EllipseFrame, Length, Point, Q_C, Rounded, Shape, Timestamp,
    drawing_outline_of, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::{
    EntryOutcome, ParamHandle, TransformHandleTolerances, oriented_bounds, param_handles,
};

use cpu_time::cpu_time_of;

/// Budgets are asserted in the release profile only, like the other budget tests.
const TIMED: bool = !cfg!(debug_assertions);

static NOW: AtomicU64 = AtomicU64::new(1_800_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(7, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn ellipse(curve: f64, rounded: f64) -> Shape {
    Shape::Ellipse {
        frame: EllipseFrame {
            center: pt(100.0, 80.0),
            rx: Length::from_mm(60.0),
            ry: Length::from_mm(45.0),
        },
        arc: None,
        curve: Curve::new(curve).unwrap(),
        rounded: Rounded::new(rounded).unwrap(),
    }
}

fn open(shape: Shape) -> Session {
    let d = Document::new(1);
    let _ = d.create_primitive(shape, Angle::from_radians(0.0));
    let mut s = Session::open(2, &curvyo_document_core::pack(&d, "0.1.0").unwrap()).unwrap();
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    s
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn shape(s: &Session) -> Shape {
    let d = doc(s);
    d.primitive(d.object_ids()[0]).unwrap().shape
}

fn rounded_and_curve(s: &Session) -> (f64, f64) {
    match shape(s) {
        Shape::Ellipse { rounded, curve, .. } => (rounded.get(), curve.get()),
        o => panic!("{o:?}"),
    }
}

fn handles(s: &Session) -> Vec<(ParamHandle, Point)> {
    let d = doc(s);
    let o = d.object(d.object_ids()[0]).unwrap();
    param_handles(
        &o,
        &oriented_bounds(&o),
        &TransformHandleTolerances::at_scale(s.view().scale()),
    )
}

fn knob(s: &Session) -> Point {
    handles(s)
        .iter()
        .find(|(h, _)| *h == ParamHandle::Rounded)
        .unwrap_or_else(|| panic!("no Rounded knob"))
        .1
}

fn select(s: &mut Session) {
    let p = pt(
        100.0 - 60.0 * std::f64::consts::FRAC_1_SQRT_2,
        80.0 + 45.0 * std::f64::consts::FRAC_1_SQRT_2,
    );
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    assert_eq!(s.selected_object_count(), 1);
}

/// The document point `q` units along the Rounded ray from vertex E.
fn at_q(s: &Session, q: f64) -> Point {
    let scale = s.view().scale();
    let unit = 60.0 - 15.0 / scale;
    pt(100.0 + 60.0 - 15.0 / scale - q * unit, 80.0)
}

fn drag(s: &mut Session, from: Point, to: Point) {
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

/// Criteria 14 and 15: an untouched ellipse rests its knob at `q_c`, a release there writes
/// nothing, a drag past it writes `q` and Curve 0 in one step, a drag near `q_c` snaps to it
/// exactly, and one undo restores the ellipse.
#[test]
fn the_untouched_ellipse_rests_at_q_c_and_a_drag_writes_one_step() {
    let mut s = open(ellipse(1.0, 0.0));
    select(&mut s);
    let steps = s.undo_len();
    let rest = knob(&s);
    assert!((rest.x - at_q(&s, Q_C).x).abs() < 1e-6, "rests at q_c");
    drag(&mut s, rest, pt(rest.x - 0.5, rest.y));
    assert_eq!(s.undo_len(), steps, "under 3 px writes nothing");
    s.pointer_hover(rest, false, false);
    s.pointer_down(rest, false);
    s.pointer_hover(pt(rest.x - 40.0, rest.y), false, false);
    s.pointer_hover(rest, false, false);
    s.pointer_up(rest, false, false);
    assert_eq!(s.undo_len(), steps, "a release at q_c writes nothing");
    let target = at_q(&s, 1.7);
    drag(&mut s, rest, target);
    let (q, curve) = rounded_and_curve(&s);
    assert!((q - 1.7).abs() < 1e-6, "{q}");
    assert_eq!(curve, 0.0, "Curve is held at 0");
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(
        s.history_target_name(curvyo_document_core::Direction::Undo)
            .as_deref(),
        Some("Change rounding")
    );
    // The arc knobs exist only at Curve 100 %: they are gone while Rounded holds it at 0.
    assert!(
        handles(&s)
            .iter()
            .all(|(h, _)| !matches!(h, ParamHandle::ArcStart | ParamHandle::ArcEnd))
    );
    // Dragging the knob from 1.7 to within 2 percent of q_c writes q_c exactly.
    let rest = knob(&s);
    let near = at_q(&s, Q_C + 0.008);
    drag(&mut s, rest, near);
    assert_eq!(rounded_and_curve(&s).0, Q_C);
    s.undo();
    assert!((rounded_and_curve(&s).0 - 1.7).abs() < 1e-6);
    s.undo();
    assert_eq!(rounded_and_curve(&s), (0.0, 1.0));
}

/// Criteria 14 and 19: the bar field takes a typed value, clears Curve in the same commit, and
/// is greyed with an arc.
#[test]
fn the_bar_field_writes_and_is_greyed_with_an_arc() {
    let mut s = open(ellipse(1.0, 0.0));
    select(&mut s);
    let field = s.select_bar_state().rounded.expect("the field is shown");
    assert_eq!(field.grey, None);
    assert_eq!(s.set_selected_rounded_text("0.39"), EntryOutcome::Committed);
    assert_eq!(rounded_and_curve(&s), (0.39, 0.0));
    assert_eq!(s.set_selected_curve_text("100"), EntryOutcome::Committed);
    assert_eq!(rounded_and_curve(&s), (0.0, 1.0), "Curve clears Rounded");
    let arc = curvyo_document_core::EllipseArc::new(
        Angle::from_radians(0.2),
        Angle::from_radians(3.5),
        curvyo_document_core::ArcKind::Pie,
    );
    let Shape::Ellipse { frame, .. } = ellipse(1.0, 0.0) else {
        unreachable!()
    };
    let mut s = open(Shape::Ellipse {
        frame,
        arc,
        curve: Curve::ELLIPSE_DEFAULT,
        rounded: Rounded::ZERO,
    });
    select(&mut s);
    let field = s.select_bar_state().rounded.expect("shown, greyed");
    assert_eq!(field.grey, Some(curvyo_ui_core::GreyReason::HasArc));
    assert!(handles(&s).iter().all(|(h, _)| *h != ParamHandle::Rounded));
}

/// Criterion 12 for the ellipse (through criterion 16): a pointer event of a Rounded drag and
/// the outline of the ellipse at q = 10 and q = -10 stay within 2 ms of thread CPU time (the
/// fastest of five runs).
#[test]
fn a_drag_event_stays_within_the_budget() {
    for q in [10.0, -10.0] {
        let shape = ellipse(0.0, q);
        let best = (0..5)
            .map(|_| cpu_time_of(|| drawing_outline_of(&shape)).1)
            .min()
            .unwrap();
        eprintln!("ellipse at q {q}: outline {best:?}");
        assert!(!TIMED || best.as_secs_f64() < 0.002, "outline {best:?}");
        let mut s = open(shape);
        // Vertex E lies on the outline at every q (the loops cover the rest).
        let vertex = pt(160.0, 80.0);
        s.pointer_hover(vertex, false, false);
        s.pointer_down(vertex, false);
        s.pointer_up(vertex, false, false);
        assert_eq!(s.selected_object_count(), 1);
        let rest = knob(&s);
        s.pointer_hover(rest, false, false);
        s.pointer_down(rest, false);
        s.pointer_hover(pt(rest.x - 20.0, rest.y), false, false);
        let event = (0..5)
            .map(|i| {
                cpu_time_of(|| {
                    s.pointer_hover(pt(rest.x - 30.0 - f64::from(i), rest.y), false, false)
                })
                .1
            })
            .min()
            .unwrap();
        eprintln!("ellipse at q {q}: drag event {event:?}");
        assert!(
            !TIMED || event.as_secs_f64() < 0.002,
            "a drag event took {event:?}"
        );
        s.escape();
    }
}
