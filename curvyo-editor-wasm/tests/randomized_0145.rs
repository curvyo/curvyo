//! Session-level tests of `specs/0145-shape-spiro-handles/specification.md` slice 3 (the
//! Randomized field and the Re-roll seed button, their history names, and the pointer-event
//! budget), against the public API of `curvyo-editor-wasm`.

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
    Angle, Curve, Direction, Document, InnerRatio, Length, Point, PointCount, Randomized, Rounded,
    Seed, Shape, StarFrame, Timestamp, drawing_outline_of, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::{
    EntryOutcome, ParamHandle, TransformHandleTolerances, oriented_bounds, param_handles,
};

use cpu_time::cpu_time_of;

/// The budgets are asserted in the release profile only, like the other budget tests; the debug
/// profile reports the times.
const TIMED: bool = !cfg!(debug_assertions);

static NOW: AtomicU64 = AtomicU64::new(1_800_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(7, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn star_of(points: u32, q: f64, r: f64) -> Shape {
    Shape::Star {
        frame: StarFrame {
            center: pt(100.0, 80.0),
            radius: Length::from_mm(60.0),
            angle: Angle::from_radians(0.0),
        },
        point_count: PointCount::new(points).unwrap(),
        inner_ratio: InnerRatio::new(0.5).unwrap(),
        curve: Curve::POLYGON_DEFAULT,
        rounded: Rounded::new(q).unwrap(),
        randomized: Randomized::new(r, Seed::new(if r == 0.0 { 0 } else { 77 })).unwrap(),
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

fn shape(s: &Session) -> Shape {
    let d = unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    d.primitive(d.object_ids()[0]).unwrap().shape
}

fn random(s: &Session) -> Randomized {
    match shape(s) {
        Shape::Polygon { randomized, .. } | Shape::Star { randomized, .. } => randomized,
        o => panic!("{o:?}"),
    }
}

/// Selects the star by a click on its vertex 0 (an unrandomized star has it at (160, 80)).
fn select(s: &mut Session) {
    let vertex_0 = pt(160.0, 80.0);
    s.pointer_hover(vertex_0, false, false);
    s.pointer_down(vertex_0, false);
    s.pointer_up(vertex_0, false, false);
    assert_eq!(s.selected_object_count(), 1);
}

fn last_step(s: &mut Session) -> Option<String> {
    s.history_target_name(Direction::Undo)
}

/// Criteria 21, 22: a typed value is limited and written as one step "Change randomizing", draws
/// a seed in the same commit, text that is no number is refused, an arrow-key preview writes
/// nothing until it is committed; Re-roll seed is one step "New seed" and one undo restores it.
#[test]
fn the_field_and_the_button_write_one_step_each() {
    let mut s = open(star_of(5, 0.0, 0.0));
    select(&mut s);
    let steps = s.undo_len();
    assert_eq!(
        s.set_selected_randomized_text("25"),
        EntryOutcome::Committed
    );
    assert_eq!(random(&s).amount(), 10.0, "limited, and written");
    assert_ne!(random(&s).seed(), Seed::new(0), "a seed in the same commit");
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(last_step(&mut s).as_deref(), Some("Change randomizing"));
    assert_eq!(s.take_transform_announcement(), "Randomized 10.00");
    assert_eq!(
        s.set_selected_randomized_text("25"),
        EntryOutcome::Unchanged
    );
    assert!(matches!(
        s.set_selected_randomized_text("x"),
        EntryOutcome::Invalid { .. }
    ));
    let before = s.undo_len();
    s.preview_selected_randomized(0.25);
    s.preview_selected_randomized(0.26);
    assert_eq!(s.undo_len(), before, "a preview writes nothing");
    s.commit_selected_curve();
    assert_eq!(random(&s).amount(), 0.26);
    assert_eq!(s.undo_len(), before + 1);

    let seed = random(&s).seed();
    let outline = drawing_outline_of(&shape(&s)).anchors[0].point;
    assert_eq!(s.reroll_selected_seed(), EntryOutcome::Committed);
    assert_ne!(random(&s).seed(), seed);
    assert_eq!(last_step(&mut s).as_deref(), Some("New seed"));
    assert_ne!(drawing_outline_of(&shape(&s)).anchors[0].point, outline);
    s.undo();
    assert_eq!(random(&s).seed(), seed, "one Ctrl+Z restores the old seed");
    assert_eq!(drawing_outline_of(&shape(&s)).anchors[0].point, outline);
}

/// Criterion 22: the button does nothing at 0 (nothing committed), and the bar says it is greyed.
#[test]
fn the_button_waits_for_a_value() {
    let mut s = open(star_of(5, 0.0, 0.0));
    select(&mut s);
    let steps = s.undo_len();
    assert_eq!(s.reroll_selected_seed(), EntryOutcome::Unchanged);
    assert_eq!(s.undo_len(), steps);
    let bar = s.select_bar_state().randomized.unwrap();
    assert!(!bar.reroll_enabled);
    s.set_selected_randomized_text("0.5");
    let bar = s.select_bar_state().randomized.unwrap();
    assert!(bar.reroll_enabled);
    assert_eq!(bar.value, curvyo_ui_core::BarValue::Uniform(0.5));
}

fn flatten_count(star: &Shape, scale: f64) -> (std::time::Duration, std::time::Duration, usize) {
    let (outline, t_outline) = cpu_time_of(|| drawing_outline_of(star));
    let anchors = &outline.anchors;
    let ((count, _), t_flatten) = cpu_time_of(|| {
        let mut path = kurbo::BezPath::new();
        path.move_to((anchors[0].point.x, anchors[0].point.y));
        for i in 0..anchors.len() {
            let (a, b) = (anchors[i], anchors[(i + 1) % anchors.len()]);
            path.curve_to(
                (a.point.x + a.handle_out.x, a.point.y + a.handle_out.y),
                (b.point.x + b.handle_in.x, b.point.y + b.handle_in.y),
                (b.point.x, b.point.y),
            );
        }
        let mut count = 0_usize;
        kurbo::flatten(path, 0.25 / scale, |_| count += 1);
        (count, ())
    });
    (t_outline, t_flatten, count)
}

/// Criterion 30: the outline of a 1024-point star at r = 10 and q = 10 (and a 5-point star at
/// r = 10, q = -10) is recomputed within 2 ms of thread CPU time (the fastest of five runs); its
/// flattening at the display tolerance is measured and bounded at 10 ms, as the Rounded budget of
/// slice 1 (`docs/technical-debt.md`). The bounds are asserted in the release profile only.
#[test]
fn a_pointer_event_stays_within_the_budget() {
    for (points, q) in [(1024_u32, 10.0), (5, -10.0)] {
        let star = star_of(points, q, 10.0);
        let s = open(star);
        let scale = s.view().scale();
        let (t_outline, t_flatten, segments) = (0..5)
            .map(|_| flatten_count(&star, scale))
            .min_by_key(|&(outline, flatten, _)| outline + flatten)
            .unwrap();
        eprintln!(
            "{points} points at r 10, q {q}: outline {t_outline:?}, {segments} segments flattened in {t_flatten:?}"
        );
        assert!(
            !TIMED || t_outline.as_secs_f64() < 0.002,
            "{points}: outline {t_outline:?}"
        );
        let flatten_limit = if points == 1024 { 0.010 } else { 0.002 };
        assert!(
            !TIMED || t_flatten.as_secs_f64() < flatten_limit,
            "{points} points: {segments} segments flattened in {t_flatten:?}"
        );
    }
    // Straight sides, the common case of the field: the outline of a 1024-point star at r = 10.
    let star = star_of(1024, 0.0, 10.0);
    let best = (0..5)
        .map(|_| cpu_time_of(|| drawing_outline_of(&star)).1)
        .min()
        .unwrap();
    eprintln!("1024 points at r 10, q 0: outline {best:?}");
    assert!(!TIMED || best.as_secs_f64() < 0.002, "{best:?}");
}

fn knob(s: &Session) -> Point {
    let d = unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    let o = d.object(d.object_ids()[0]).unwrap();
    param_handles(
        &o,
        &oriented_bounds(&o),
        &TransformHandleTolerances::at_scale(s.view().scale()),
    )
    .iter()
    .find(|(h, _)| *h == ParamHandle::Rounded)
    .unwrap_or_else(|| panic!("no Rounded knob"))
    .1
}

fn centre(s: &Session) -> Point {
    match shape(s) {
        Shape::Polygon { frame, .. } | Shape::Star { frame, .. } => frame.center,
        o => panic!("{o:?}"),
    }
}

/// A randomized shape moves like any other: the seed is a 32-bit number, not a coordinate, so the
/// "Too large" guard of a transform never refuses it, and the value and the seed stay.
#[test]
fn a_randomized_star_with_a_fresh_seed_can_be_moved() {
    let mut s = open(star_of(5, 0.0, 0.0));
    select(&mut s);
    assert_eq!(
        s.set_selected_randomized_text("0.5"),
        EntryOutcome::Committed
    );
    let kept = random(&s);
    assert!(
        kept.seed().get() > 1_000_000_000,
        "a fresh seed is 32 bits: {kept:?}"
    );
    // The middle of the first straight side, clear of every handle.
    let outline = drawing_outline_of(&shape(&s));
    let (a, b) = (outline.anchors[0].point, outline.anchors[1].point);
    let from = pt(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y));
    let before = centre(&s);
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(pt(from.x + 30.0, from.y + 20.0), false, false);
    s.pointer_up(pt(from.x + 30.0, from.y + 20.0), false, false);
    let after = centre(&s);
    assert!(
        (after.x - before.x - 30.0).abs() < 1e-6 && (after.y - before.y - 20.0).abs() < 1e-6,
        "moved by the drag: {before:?} to {after:?}"
    );
    assert_eq!(random(&s), kept, "the value and the seed stay");
}

/// Criterion 30: one pointer event of a Rounded knob drag on a 1024-point star with Randomized 10
/// and Rounded 10 (the knob sits on the unshifted ring) stays within 2 ms of thread CPU time, the
/// fastest of five events; asserted in the release profile only, the frame is measured.
#[test]
fn a_rounded_drag_event_on_a_randomized_star_stays_within_the_budget() {
    let mut s = open(star_of(1024, 0.0, 0.0));
    select(&mut s);
    s.set_selected_randomized_text("10");
    s.set_selected_rounded_text("10");
    let rest = knob(&s);
    s.pointer_hover(rest, false, false);
    s.pointer_down(rest, false);
    s.pointer_hover(pt(rest.x - 20.0, rest.y), false, false);
    let best = (0..5)
        .map(|i| {
            cpu_time_of(|| {
                s.pointer_hover(pt(rest.x - 40.0 - f64::from(i), rest.y), false, false);
            })
            .1
        })
        .min()
        .unwrap();
    eprintln!("1024 points, r 10, q 10: a drag event {best:?}");
    assert!(
        !TIMED || best.as_secs_f64() < 0.002,
        "a drag event took {best:?}"
    );
    let (_, frame) = cpu_time_of(|| s.frame_draw_list());
    eprintln!("frame_draw_list during a drag: {frame:?}");
    s.escape();
}
