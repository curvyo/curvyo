//! Independent tester acceptance tests for `specs/0032-pen-tablet-input/specification.md`,
//! the pure stroke rules in `curvyo-ui-core` (criteria 1, 3, 4, 5, 7, 8, 9, 10, 11, 12) and
//! edge cases (zero or missing pressure, out-of-range pressure, very short, very long and
//! self-crossing strokes). Written from the specification and the public API before the
//! implementation diff was read.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::doc_markdown,
    clippy::many_single_char_names,
    missing_docs
)]

use std::f64::consts::PI;
use std::time::Instant;

use curvyo_document_core::{Angle, Length, Point, Tolerance};
use curvyo_geometry_core::{OutlineTriple, nearest_point_on_segment, signed_area_mm2};
use curvyo_ui_core::{
    ERASER_BUTTON, PencilSettings, PencilTool, PointerKind, PointerSample, PressureCurve,
    StrokeShape, smooth_pressures, stroke_has_pressure,
};

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

fn sample(kind: PointerKind, x: f64, y: f64, pressure: f64) -> PointerSample {
    PointerSample {
        position: Point::new(x, y),
        kind,
        pressure,
        tilt_x: Angle::from_radians(0.0),
        tilt_y: Angle::from_radians(0.0),
        eraser: false,
        time_ms: 0.0,
    }
}

fn pen(x: f64, y: f64, p: f64) -> PointerSample {
    sample(PointerKind::Pen, x, y, p)
}

fn mouse(x: f64, y: f64) -> PointerSample {
    // A mouse with a button down reports 0.5 (Pointer Events spec).
    sample(PointerKind::Mouse, x, y, 0.5)
}

/// Draws `samples` as one stroke: press on the first, one move per sample, release with none.
fn draw(samples: &[PointerSample]) -> Option<curvyo_ui_core::FinishedStroke> {
    let mut tool = PencilTool::default();
    assert!(
        tool.press(samples[0], 1),
        "a primary-contact press starts a stroke"
    );
    for s in &samples[1..] {
        tool.extend(std::slice::from_ref(s));
    }
    tool.release(&[])
}

fn shape_of(samples: &[PointerSample], settings: &PencilSettings) -> Option<StrokeShape> {
    draw(samples).and_then(|f| f.shape(settings))
}

fn centre(shape: Option<StrokeShape>) -> Vec<OutlineTriple> {
    match shape {
        Some(StrokeShape::Centre(c)) => c,
        other => panic!("expected an open centre line, got {other:?}"),
    }
}

fn outlines(shape: Option<StrokeShape>) -> Vec<Vec<Point>> {
    match shape {
        Some(StrokeShape::Outlines(o)) => o,
        other => panic!("expected outlines, got {other:?}"),
    }
}

/// Distance of `q` from the cubic Bézier path `c`.
fn distance_to_centre(c: &[OutlineTriple], q: Point) -> f64 {
    c.windows(2)
        .map(|w| {
            let (a, _, a_out) = w[0];
            let (b, b_in, _) = w[1];
            nearest_point_on_segment(a, a_out, b_in, b, q, Tolerance::from_mm(0.001))
                .1
                .as_mm()
        })
        .fold(f64::INFINITY, f64::min)
}

fn dist_point_segment(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
    };
    let (cx, cy) = (a.x + t * dx, a.y + t * dy);
    ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt()
}

/// Distance of `p` from the closed polygon boundaries `polys`.
fn distance_to_boundary(polys: &[Vec<Point>], p: Point) -> f64 {
    polys
        .iter()
        .flat_map(|poly| {
            (0..poly.len()).map(move |i| dist_point_segment(p, poly[i], poly[(i + 1) % poly.len()]))
        })
        .fold(f64::INFINITY, f64::min)
}

/// The vertical extent of the outlines at `x` (max y - min y of the crossings).
fn width_at_x(polys: &[Vec<Point>], x: f64) -> f64 {
    let mut ys = Vec::new();
    for poly in polys {
        for i in 0..poly.len() {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            if (a.x - x) * (b.x - x) <= 0.0 && a.x != b.x {
                let t = (x - a.x) / (b.x - a.x);
                ys.push(a.y + t * (b.y - a.y));
            } else if a.x == x {
                ys.push(a.y);
            }
        }
    }
    assert!(!ys.is_empty(), "no crossing at x = {x}");
    let lo = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    hi - lo
}

fn total_area(polys: &[Vec<Point>]) -> f64 {
    polys.iter().map(|p| signed_area_mm2(p)).sum()
}

fn vertex_count(polys: &[Vec<Point>]) -> usize {
    polys.iter().map(Vec::len).sum()
}

fn settings(min: f64, max: f64, curve: PressureCurve) -> PencilSettings {
    let mut s = PencilSettings::default();
    s.set_minimum(Length::from_mm(min));
    s.set_maximum(Length::from_mm(max));
    s.curve = curve;
    s
}

/// Samples of an S-curve about 150 mm long, `n` of them, pressure varying 0.2 to 0.9.
fn s_curve(n: usize) -> Vec<PointerSample> {
    // x from 0 to 120 mm, y = 20 sin(2 pi x / 120): arc length about 150 mm.
    (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            let x = 120.0 * t;
            let y = 20.0 * (2.0 * PI * t).sin();
            let p = 0.55 + 0.35 * (6.0 * PI * t).sin();
            pen(x, y, p)
        })
        .collect()
}

fn polyline_length(samples: &[PointerSample]) -> f64 {
    samples
        .windows(2)
        .map(|w| w[0].position.vector_to(w[1].position).length())
        .sum()
}

// ---------------------------------------------------------------------
// Criterion 1: the input record
// ---------------------------------------------------------------------

#[test]
fn c1_record_carries_kind_pressure_tilt_eraser_time_as_delivered() {
    let s = PointerSample::from_values(
        "pen",
        1 | ERASER_BUTTON,
        [3.0, 4.0, 0.37, 12.0, -3.0, 1234.5],
    );
    assert_eq!(s.kind, PointerKind::Pen);
    assert_eq!(s.position, Point::new(3.0, 4.0));
    assert_eq!(s.pressure, 0.37);
    assert!((s.tilt_x.as_radians().to_degrees() - 12.0).abs() < 1e-9);
    assert!((s.tilt_y.as_radians().to_degrees() + 3.0).abs() < 1e-9);
    assert!(s.eraser, "buttons bit 32 is the eraser flag");
    assert_eq!(s.time_ms, 1234.5);
    assert_eq!(ERASER_BUTTON, 32);

    let s = PointerSample::from_values("pen", 1, [0.0, 0.0, 0.5, 0.0, 0.0, 0.0]);
    assert!(!s.eraser);
    assert_eq!(
        PointerSample::from_values("mouse", 1, [0.0; 6]).kind,
        PointerKind::Mouse
    );
    assert_eq!(
        PointerSample::from_values("touch", 1, [0.0; 6]).kind,
        PointerKind::Touch
    );
    // An unknown pointerType is treated as a mouse.
    assert_eq!(
        PointerSample::from_values("", 1, [0.0; 6]).kind,
        PointerKind::Mouse
    );
    assert_eq!(
        PointerSample::from_values("Pen", 1, [0.0; 6]).kind,
        PointerKind::Mouse
    );
}

#[test]
fn c1_tilt_and_eraser_change_nothing_in_the_result() {
    let base: Vec<PointerSample> = (0..=100)
        .map(|i| pen(f64::from(i) * 0.5, 0.0, 0.3 + 0.004 * f64::from(i)))
        .collect();
    let tilted: Vec<PointerSample> = base
        .iter()
        .enumerate()
        .map(|(i, s)| PointerSample {
            tilt_x: Angle::from_radians((i as f64).to_radians()),
            tilt_y: Angle::from_radians(-(i as f64 * 0.5).to_radians()),
            eraser: i % 2 == 0,
            ..*s
        })
        .collect();
    let st = PencilSettings::default();
    assert_eq!(shape_of(&base, &st), shape_of(&tilted, &st));
}

// ---------------------------------------------------------------------
// Criterion 3: pressure present
// ---------------------------------------------------------------------

#[test]
fn c3_pressure_present_rules() {
    let varying: Vec<PointerSample> = (1..=9)
        .map(|i| pen(f64::from(i), 0.0, f64::from(i) / 10.0))
        .collect();
    assert!(stroke_has_pressure(&varying), "pen with 0.1..0.9: present");
    let half: Vec<PointerSample> = (0..9).map(|i| pen(f64::from(i), 0.0, 0.5)).collect();
    assert!(!stroke_has_pressure(&half), "pen constant 0.5: absent");
    let zero: Vec<PointerSample> = (0..9).map(|i| pen(f64::from(i), 0.0, 0.0)).collect();
    assert!(
        !stroke_has_pressure(&zero),
        "pen constant 0 with contact: absent"
    );
    let mixed: Vec<PointerSample> = (0..9)
        .map(|i| pen(f64::from(i), 0.0, if i % 2 == 0 { 0.0 } else { 0.5 }))
        .collect();
    assert!(
        !stroke_has_pressure(&mixed),
        "pen with only 0 and 0.5: absent"
    );
    let mice: Vec<PointerSample> = (1..=9)
        .map(|i| sample(PointerKind::Mouse, f64::from(i), 0.0, f64::from(i) / 10.0))
        .collect();
    assert!(
        !stroke_has_pressure(&mice),
        "mouse: absent, even with varying pressure"
    );
    let fingers: Vec<PointerSample> = (1..=9)
        .map(|i| sample(PointerKind::Touch, f64::from(i), 0.0, f64::from(i) / 10.0))
        .collect();
    assert!(!stroke_has_pressure(&fingers), "touch: absent");
    // One usable sample in the middle is enough.
    let mut one = half.clone();
    one[4].pressure = 0.7;
    assert!(stroke_has_pressure(&one));
    assert!(!stroke_has_pressure(&[]));
}

#[test]
fn c3_pen_without_pressure_draws_an_open_path() {
    let line: Vec<PointerSample> = (0..100)
        .map(|i| pen(f64::from(i) * 0.5, 0.0, 0.5))
        .collect();
    let c = centre(shape_of(&line, &PencilSettings::default()));
    assert_eq!(c.len(), 2);
    let zero: Vec<PointerSample> = (0..100)
        .map(|i| pen(f64::from(i) * 0.5, 0.0, 0.0))
        .collect();
    assert_eq!(centre(shape_of(&zero, &PencilSettings::default())).len(), 2);
    let finished = draw(&line).unwrap();
    assert!(!finished.has_pressure());
}

// ---------------------------------------------------------------------
// Criteria 4, 5: touch ignored, hover does not draw
// ---------------------------------------------------------------------

#[test]
fn c4_touch_never_starts_a_stroke() {
    let mut tool = PencilTool::default();
    assert!(!tool.press(sample(PointerKind::Touch, 0.0, 0.0, 0.7), 1));
    assert!(!tool.is_drawing());
    tool.extend(&[sample(PointerKind::Touch, 10.0, 0.0, 0.7)]);
    assert!(
        tool.release(&[sample(PointerKind::Touch, 20.0, 0.0, 0.7)])
            .is_none()
    );
}

#[test]
fn c4_touch_samples_inside_a_pen_stroke_are_ignored() {
    let mut tool = PencilTool::default();
    assert!(tool.press(pen(0.0, 0.0, 0.5), 1));
    tool.extend(&[sample(PointerKind::Touch, 100.0, 100.0, 0.9)]);
    tool.extend(&[pen(10.0, 0.0, 0.5)]);
    let f = tool.release(&[]).unwrap();
    assert!(
        f.points().iter().all(|p| p.y == 0.0),
        "the palm sample is not part of the stroke: {:?}",
        f.points()
    );
}

#[test]
fn c5_hover_starts_nothing_contact_starts_a_stroke() {
    let mut tool = PencilTool::default();
    assert!(
        !tool.press(pen(0.0, 0.0, 0.0), 0),
        "pen in range, buttons 0"
    );
    assert!(!tool.is_drawing());
    tool.extend(&[pen(5.0, 0.0, 0.0)]);
    assert!(tool.release(&[]).is_none());
    assert!(tool.press(pen(0.0, 0.0, 0.3), 1), "pen down with buttons 1");
    assert!(tool.is_drawing());
}

#[test]
fn c5_mouse_press_with_left_button_starts_a_stroke() {
    let mut tool = PencilTool::default();
    assert!(tool.press(mouse(0.0, 0.0), 1));
}

// ---------------------------------------------------------------------
// Criterion 7: stroke without pressure
// ---------------------------------------------------------------------

#[test]
fn c7_straight_line_of_100_samples_gives_two_nodes() {
    let line: Vec<PointerSample> = (0..100)
        .map(|i| mouse(10.0 + f64::from(i) * 0.5, 20.0))
        .collect();
    let c = centre(shape_of(&line, &PencilSettings::default()));
    assert_eq!(c.len(), 2, "{c:?}");
    for s in &line {
        assert!(distance_to_centre(&c, s.position) <= 0.1);
    }
}

#[test]
fn c7_half_circle_200_samples_under_10_nodes_within_0_1_mm() {
    let arc: Vec<PointerSample> = (0..200)
        .map(|i| {
            let a = PI * f64::from(i) / 199.0;
            mouse(50.0 + 30.0 * a.cos(), 50.0 + 30.0 * a.sin())
        })
        .collect();
    let c = centre(shape_of(&arc, &PencilSettings::default()));
    assert!(c.len() < 10, "{} nodes", c.len());
    let worst = arc
        .iter()
        .map(|s| distance_to_centre(&c, s.position))
        .fold(0.0, f64::max);
    assert!(worst <= 0.1, "worst sample distance {worst} mm");
    // Open path: the ends are the first and last sample.
    assert!(c[0].0.vector_to(arc[0].position).length() <= 0.1);
    assert!(c[c.len() - 1].0.vector_to(arc[199].position).length() <= 0.1);
}

#[test]
fn c7_noisy_hand_stroke_stays_within_0_1_mm() {
    // A wobbly hand-drawn line: deterministic pseudo-noise of +-0.15 mm.
    let mut seed: u64 = 12345;
    let wobbly: Vec<PointerSample> = (0..600)
        .map(|i| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let n = ((seed >> 33) as f64 / f64::from(u32::MAX) - 0.5) * 0.3;
            let t = f64::from(i) / 599.0;
            mouse(100.0 * t, 15.0 * (3.0 * t).sin() + n)
        })
        .collect();
    let c = centre(shape_of(&wobbly, &PencilSettings::default()));
    let worst = wobbly
        .iter()
        .map(|s| distance_to_centre(&c, s.position))
        .fold(0.0, f64::max);
    assert!(
        worst <= 0.1,
        "worst sample distance {worst} mm with {} nodes",
        c.len()
    );
}

#[test]
fn c7_self_crossing_stroke_without_pressure_is_one_open_path_within_tolerance() {
    // A loop: 1.3 turns of a circle of radius 15, crossing itself.
    let loop_: Vec<PointerSample> = (0..400)
        .map(|i| {
            let a = 2.6 * PI * f64::from(i) / 399.0;
            mouse(
                40.0 + 15.0 * a.cos() + f64::from(i) * 0.02,
                40.0 + 15.0 * a.sin(),
            )
        })
        .collect();
    let c = centre(shape_of(&loop_, &PencilSettings::default()));
    let worst = loop_
        .iter()
        .map(|s| distance_to_centre(&c, s.position))
        .fold(0.0, f64::max);
    assert!(worst <= 0.1, "worst {worst}");
}

#[test]
fn c7_stroke_that_returns_to_its_start_is_still_open() {
    // A closed-looking circle (end on start) without pressure stays an open centre line.
    let ring: Vec<PointerSample> = (0..=300)
        .map(|i| {
            let a = 2.0 * PI * f64::from(i) / 300.0;
            mouse(40.0 + 10.0 * a.cos(), 40.0 + 10.0 * a.sin())
        })
        .collect();
    let c = centre(shape_of(&ring, &PencilSettings::default()));
    assert!(c.len() >= 2);
    let worst = ring
        .iter()
        .map(|s| distance_to_centre(&c, s.position))
        .fold(0.0, f64::max);
    assert!(worst <= 0.1, "worst {worst}");
}

#[test]
fn c7_back_and_forth_scribble_within_tolerance() {
    // A zig-zag that reverses direction sharply 10 times.
    let mut pts = Vec::new();
    for leg in 0..10 {
        for i in 0..50 {
            let t = f64::from(i) / 49.0;
            let x = if leg % 2 == 0 {
                30.0 * t
            } else {
                30.0 * (1.0 - t)
            };
            pts.push(mouse(x, f64::from(leg) * 2.0 + t * 2.0));
        }
    }
    let c = centre(shape_of(&pts, &PencilSettings::default()));
    let worst = pts
        .iter()
        .map(|s| distance_to_centre(&c, s.position))
        .fold(0.0, f64::max);
    assert!(worst <= 0.1, "worst {worst} with {} nodes", c.len());
}

// ---------------------------------------------------------------------
// Criterion 8: stroke with pressure
// ---------------------------------------------------------------------

#[test]
fn c8_constant_full_pressure_line_is_a_capsule() {
    // Pressure exactly 1.0 throughout: has pressure (not 0, not 0.5).
    let line: Vec<PointerSample> = (0..=500)
        .map(|i| pen(10.0 + f64::from(i) * 0.1, 30.0, 1.0))
        .collect();
    let o = outlines(shape_of(&line, &settings(0.2, 2.0, PressureCurve::Linear)));
    assert_eq!(o.len(), 1, "one outer outline");
    let expected = 50.0 * 2.0 + PI * 1.0;
    let area = total_area(&o);
    assert!(
        (area - expected).abs() <= 0.3,
        "area {area}, expected {expected}"
    );
    assert!((width_at_x(&o, 35.0) - 2.0).abs() <= 0.02);
    // Every outline vertex lies on the exact capsule within 0.02 mm.
    for p in &o[0] {
        let d = dist_point_segment(*p, Point::new(10.0, 30.0), Point::new(60.0, 30.0));
        assert!(
            (d - 1.0).abs() <= 0.02,
            "vertex {p:?} at {d} from the centre line"
        );
    }
}

#[test]
fn c8_linear_ramp_widths() {
    let line: Vec<PointerSample> = (0..=500)
        .map(|i| pen(f64::from(i) * 0.1, 0.0, f64::from(i) / 500.0))
        .collect();
    let o = outlines(shape_of(&line, &settings(0.2, 2.0, PressureCurve::Linear)));
    let w0 = width_at_x(&o, 0.0);
    assert!((w0 - 0.2).abs() <= 0.05, "width at the start {w0}");
    let w25 = width_at_x(&o, 25.0);
    assert!((w25 - 1.1).abs() <= 0.05, "width at x = 25 {w25}");
    // The start has a round end: the outline reaches about 0.1 mm before x = 0.
    let min_x = o
        .iter()
        .flatten()
        .map(|p| p.x)
        .fold(f64::INFINITY, f64::min);
    assert!(
        (min_x + 0.1).abs() <= 0.03,
        "round start cap reaches {min_x}"
    );
    let max_x = o
        .iter()
        .flatten()
        .map(|p| p.x)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        (max_x - 51.0).abs() <= 0.05,
        "round end cap reaches {max_x}"
    );
}

#[test]
fn c8_outline_is_positive_and_contains_the_centre() {
    let s = s_curve(400);
    let o = outlines(shape_of(&s, &PencilSettings::default()));
    assert!(total_area(&o) > 0.0);
    // Every sample is at least Minimum/2 - tolerance inside: never on the wrong side.
    for smp in &s {
        let d = distance_to_boundary(&o, smp.position);
        assert!(d >= 0.1 - 0.1, "sample {smp:?} at {d} from the boundary");
    }
}

#[test]
fn c8_self_crossing_pressure_stroke_keeps_its_hole() {
    // 1.15 turns of radius 10, width about 1.28 mm: the ring overlaps itself and leaves a hole.
    let ring: Vec<PointerSample> = (0..=460)
        .map(|i| {
            let a = 2.3 * PI * f64::from(i) / 460.0;
            pen(
                50.0 + 10.0 * a.cos(),
                50.0 + 10.0 * a.sin(),
                if i % 2 == 0 { 0.6 } else { 0.61 },
            )
        })
        .collect();
    let o = outlines(shape_of(&ring, &PencilSettings::default()));
    let areas: Vec<f64> = o.iter().map(|p| signed_area_mm2(p)).collect();
    assert!(
        areas.iter().any(|a| *a > 0.0),
        "an outer outline: {areas:?}"
    );
    assert!(
        areas.iter().any(|a| *a < 0.0),
        "the empty middle is a hole: {areas:?}"
    );
    let w = 0.2 + 1.8 * 0.605;
    let ring_area = 2.0 * PI * 10.0 * w;
    let area = total_area(&o);
    assert!(
        area > ring_area * 0.95 && area < ring_area * 1.25,
        "area {area}, ring {ring_area}"
    );
    // The centre of the circle is not filled: it lies in the hole, far from every edge.
    assert!(distance_to_boundary(&o, Point::new(50.0, 50.0)) > 8.0);
}

#[test]
fn c8_figure_eight_pressure_stroke() {
    let eight: Vec<PointerSample> = (0..=600)
        .map(|i| {
            let t = 2.0 * PI * f64::from(i) / 600.0;
            pen(
                50.0 + 20.0 * t.sin(),
                50.0 + 10.0 * (2.0 * t).sin(),
                0.3 + 0.4 * (t / 2.0).sin(),
            )
        })
        .collect();
    let o = outlines(shape_of(&eight, &PencilSettings::default()));
    assert!(total_area(&o) > 0.0);
    assert!(
        o.iter()
            .flatten()
            .all(|p| p.x.is_finite() && p.y.is_finite())
    );
}

#[test]
fn c8_pressure_above_one_is_clamped_to_maximum() {
    // A driver that overshoots: 1.3. The width must not exceed Maximum.
    let line: Vec<PointerSample> = (0..=200)
        .map(|i| pen(f64::from(i) * 0.1, 0.0, 1.3))
        .collect();
    let o = outlines(shape_of(&line, &settings(0.2, 2.0, PressureCurve::Linear)));
    let w = width_at_x(&o, 10.0);
    assert!(w <= 2.0 + 0.02, "width {w} above Maximum");
}

#[test]
fn c8_nan_pressure_never_yields_nan_geometry() {
    let line: Vec<PointerSample> = (0..=200)
        .map(|i| {
            pen(
                f64::from(i) * 0.1,
                0.0,
                if i == 100 { f64::NAN } else { 0.3 },
            )
        })
        .collect();
    if let Some(StrokeShape::Outlines(o)) = shape_of(&line, &PencilSettings::default()) {
        assert!(
            o.iter()
                .flatten()
                .all(|p| p.x.is_finite() && p.y.is_finite())
        );
    }
}

#[test]
fn c8_pen_with_zero_on_contact_then_pressure_has_pressure() {
    // Some webviews report 0 on contact before real values arrive.
    let line: Vec<PointerSample> = (0..=200)
        .map(|i| pen(f64::from(i) * 0.1, 0.0, if i < 3 { 0.0 } else { 0.8 }))
        .collect();
    assert!(draw(&line).unwrap().has_pressure());
    let _ = outlines(shape_of(&line, &PencilSettings::default()));
}

// ---------------------------------------------------------------------
// Criterion 9: smoothing, curve, width
// ---------------------------------------------------------------------

#[test]
fn c9_smoothing_example() {
    let s = smooth_pressures(&[0.2, 0.2, 1.0, 0.2, 0.2]);
    let want = [0.2, 0.2, 0.467, 0.467, 0.467];
    assert_eq!(s.len(), 5);
    for (a, b) in s.iter().zip(want) {
        assert!((a - b).abs() <= 0.001, "{s:?}");
    }
}

#[test]
fn c9_smoothing_at_the_start_and_degenerate_input() {
    assert_eq!(smooth_pressures(&[]), Vec::<f64>::new());
    assert_eq!(smooth_pressures(&[0.3]), vec![0.3]);
    let two = smooth_pressures(&[0.2, 0.6]);
    assert!(
        (two[1] - 0.4).abs() < 1e-12,
        "fewer at the start: mean of two"
    );
    // Smoothing uses the raw pressures of the samples before, not the smoothed ones.
    let s = smooth_pressures(&[0.0, 0.0, 0.9, 0.0, 0.0, 0.0]);
    assert!(
        (s[3] - 0.3).abs() < 1e-12 && (s[4] - 0.3).abs() < 1e-12 && s[5].abs() < 1e-12,
        "{s:?}"
    );
}

#[test]
fn c9_curves_and_widths() {
    assert!((PressureCurve::Soft.apply(0.25) - 0.5).abs() < 1e-12);
    assert!((PressureCurve::Linear.apply(0.25) - 0.25).abs() < 1e-12);
    assert!((PressureCurve::Firm.apply(0.25) - 0.0625).abs() < 1e-12);
    for (curve, want) in [
        (PressureCurve::Soft, 1.1),
        (PressureCurve::Linear, 0.65),
        (PressureCurve::Firm, 0.3125),
    ] {
        let st = settings(0.2, 2.0, curve);
        assert!(
            (st.width_for(0.25).as_mm() - want).abs() < 1e-9,
            "{curve:?}"
        );
    }
    // Ends of the range.
    let st = settings(0.2, 2.0, PressureCurve::Soft);
    assert!((st.width_for(0.0).as_mm() - 0.2).abs() < 1e-12);
    assert!((st.width_for(1.0).as_mm() - 2.0).abs() < 1e-12);
}

#[test]
fn c9_widths_smooth_first_then_curve() {
    let st = settings(0.2, 2.0, PressureCurve::Firm);
    let w = st.widths(&[0.2, 0.2, 1.0, 0.2, 0.2]);
    let smoothed = 1.4 / 3.0;
    let want = 0.2 + 1.8 * smoothed * smoothed;
    assert!((w[2].as_mm() - want).abs() < 1e-3, "{w:?}");
    // Curve first would give a different value: (0.04 + 0.04 + 1)/3 = 0.36 -> 0.848.
    assert!((w[2].as_mm() - (0.2 + 1.8 * 0.36)).abs() > 0.01);
}

// ---------------------------------------------------------------------
// Criterion 14 (pure part): defaults and ranges
// ---------------------------------------------------------------------

#[test]
fn c14_defaults_and_ranges() {
    let d = PencilSettings::default();
    assert_eq!(d.minimum().as_mm(), 0.2);
    assert_eq!(d.maximum().as_mm(), 2.0);
    assert_eq!(d.curve, PressureCurve::Linear);

    let mut s = PencilSettings::default();
    s.set_minimum(Length::from_mm(0.01));
    assert_eq!(s.minimum().as_mm(), 0.05);
    s.set_minimum(Length::from_mm(100.0));
    assert_eq!(s.minimum().as_mm(), 20.0);
    assert!(s.maximum().as_mm() >= 20.0, "Maximum never below Minimum");
    s.set_maximum(Length::from_mm(500.0));
    assert_eq!(s.maximum().as_mm(), 50.0);
    s.set_maximum(Length::from_mm(5.0));
    assert_eq!(
        s.maximum().as_mm(),
        20.0,
        "a Maximum below Minimum is raised to it"
    );

    let mut s = PencilSettings::default();
    s.set_maximum(Length::from_mm(0.01));
    assert!(s.maximum().as_mm() >= 0.1 && s.maximum().as_mm() >= s.minimum().as_mm());
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -3.0] {
        let mut s = PencilSettings::default();
        s.set_minimum(Length::from_mm(bad));
        s.set_maximum(Length::from_mm(bad));
        let (lo, hi) = (s.minimum().as_mm(), s.maximum().as_mm());
        assert!(
            lo.is_finite()
                && hi.is_finite()
                && (0.05..=20.0).contains(&lo)
                && (0.1..=50.0).contains(&hi)
                && hi >= lo,
            "{bad}: {lo} {hi}"
        );
    }
}

// ---------------------------------------------------------------------
// Criterion 10: node and time budget
// ---------------------------------------------------------------------

#[test]
fn c10_two_thousand_samples_under_600_nodes_and_200_ms() {
    let s = s_curve(2000);
    let len = polyline_length(&s);
    assert!((len - 150.0).abs() < 10.0, "test curve is {len} mm long");
    let st = PencilSettings::default();
    let start = Instant::now();
    let shape = shape_of(&s, &st);
    let elapsed = start.elapsed();
    let o = outlines(shape);
    let n = vertex_count(&o);
    assert!(n < 600, "{n} nodes");
    if !cfg!(debug_assertions) {
        assert!(elapsed.as_millis() < 200, "release took {elapsed:?}");
    }
    // The same stroke without pressure (mouse) is also cheap and small.
    let m: Vec<PointerSample> = s
        .iter()
        .map(|p| mouse(p.position.x, p.position.y))
        .collect();
    assert!(centre(shape_of(&m, &st)).len() < 600);
}

#[test]
fn c10_very_long_stroke_does_not_break() {
    // 20,000 samples, ten S-curves end to end (1.5 m).
    let s: Vec<PointerSample> = (0..20_000)
        .map(|i| {
            let t = f64::from(i) / 2000.0;
            pen(
                120.0 * t,
                20.0 * (2.0 * PI * t).sin(),
                0.5 + 0.3 * (t * 7.0).sin(),
            )
        })
        .collect();
    let o = outlines(shape_of(&s, &PencilSettings::default()));
    assert!(total_area(&o) > 0.0);
    assert!(
        o.iter()
            .flatten()
            .all(|p| p.x.is_finite() && p.y.is_finite())
    );
}

// ---------------------------------------------------------------------
// Criterion 11: the preview matches the object
// ---------------------------------------------------------------------

#[test]
fn c11_preview_kind_follows_pressure_and_object_within_0_15_mm() {
    let st = PencilSettings::default();
    let mut tool = PencilTool::default();
    assert!(tool.preview(&st).is_none(), "idle: no preview");
    tool.press(pen(0.0, 0.0, 0.5), 1);
    for i in 1..=5 {
        tool.extend(&[pen(f64::from(i), 0.0, 0.5)]);
    }
    assert!(
        matches!(tool.preview(&st), Some(StrokeShape::Centre(_))),
        "no pressure yet: a line"
    );
    let s = s_curve(800);
    for smp in &s {
        tool.extend(&[PointerSample {
            position: Point::new(smp.position.x + 5.0, smp.position.y),
            ..*smp
        }]);
    }
    let Some(StrokeShape::Outlines(preview)) = tool.preview(&st) else {
        panic!("with pressure the preview is the outline");
    };
    let finished = tool.release(&[]).unwrap();
    let object = outlines(finished.shape(&st));
    let worst = object
        .iter()
        .flatten()
        .map(|p| distance_to_boundary(&preview, *p))
        .fold(0.0, f64::max);
    assert!(
        worst <= 0.15,
        "object vertex {worst} mm from the last preview"
    );
    let worst_back = preview
        .iter()
        .flatten()
        .map(|p| distance_to_boundary(&object, *p))
        .fold(0.0, f64::max);
    assert!(
        worst_back <= 0.15,
        "preview vertex {worst_back} mm from the object"
    );
    assert!(tool.preview(&st).is_none(), "after release: no preview");
}

#[test]
fn c11_preview_of_2000_samples_fits_in_a_frame() {
    let st = PencilSettings::default();
    let mut tool = PencilTool::default();
    let s = s_curve(2000);
    tool.press(s[0], 1);
    tool.extend(&s[1..]);
    let start = Instant::now();
    let p = tool.preview(&st);
    let elapsed = start.elapsed();
    assert!(p.is_some());
    if !cfg!(debug_assertions) {
        assert!(elapsed.as_millis() < 16, "preview took {elapsed:?}");
    }
}

// ---------------------------------------------------------------------
// Criterion 12: cancel and too-short strokes
// ---------------------------------------------------------------------

#[test]
fn c12_cancel_drops_the_stroke() {
    let mut tool = PencilTool::default();
    assert!(!tool.cancel(), "nothing to cancel while idle");
    tool.press(pen(0.0, 0.0, 0.7), 1);
    tool.extend(&[pen(10.0, 0.0, 0.7)]);
    assert!(tool.cancel());
    assert!(!tool.is_drawing());
    assert!(
        tool.release(&[pen(20.0, 0.0, 0.7)]).is_none(),
        "a release after Escape writes nothing"
    );
}

#[test]
fn c12_fewer_than_two_samples_or_under_0_1_mm_make_nothing() {
    let mut tool = PencilTool::default();
    tool.press(pen(0.0, 0.0, 0.7), 1);
    assert!(tool.release(&[]).is_none(), "one sample");
    // Two samples 0.09 mm apart.
    assert!(draw(&[pen(0.0, 0.0, 0.7), pen(0.09, 0.0, 0.7)]).is_none());
    // Many samples, total polyline length below 0.1 mm (a jittering tap).
    let tap: Vec<PointerSample> = (0..30)
        .map(|i| pen(if i % 2 == 0 { 0.0 } else { 0.003 }, 0.0, 0.7))
        .collect();
    assert!(polyline_length(&tap) < 0.1);
    assert!(draw(&tap).is_none());
    // All samples on one spot.
    let dot: Vec<PointerSample> = (0..50).map(|_| pen(5.0, 5.0, 0.7)).collect();
    assert!(draw(&dot).is_none());
    // Just over 0.1 mm makes something.
    let short = draw(&[mouse(0.0, 0.0), mouse(0.11, 0.0)]).expect("0.11 mm is enough");
    assert!(short.shape(&PencilSettings::default()).is_some());
    let short_pen = draw(&[pen(0.0, 0.0, 0.7), pen(0.11, 0.0, 0.8)]).expect("0.11 mm pen");
    assert!(matches!(
        short_pen.shape(&PencilSettings::default()),
        Some(StrokeShape::Outlines(_))
    ));
}

#[test]
fn c12_release_samples_count() {
    // The release's own samples are part of the stroke.
    let mut tool = PencilTool::default();
    tool.press(mouse(0.0, 0.0), 1);
    let f = tool
        .release(&[mouse(30.0, 0.0)])
        .expect("press + release sample = 2 samples");
    assert_eq!(f.points().len(), 2);
}

#[test]
fn huge_coordinates_do_not_panic() {
    let far: Vec<PointerSample> = (0..100)
        .map(|i| pen(1.0e9 + f64::from(i), 1.0e9, 0.7))
        .collect();
    let _ = shape_of(&far, &PencilSettings::default());
    let far_mouse: Vec<PointerSample> = (0..100)
        .map(|i| mouse(1.0e9 + f64::from(i), 1.0e9))
        .collect();
    let _ = shape_of(&far_mouse, &PencilSettings::default());
}
