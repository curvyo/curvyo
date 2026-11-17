//! Tester acceptance for `specs/0117-centre-lines` criteria 5 to 14 and determinism at the
//! geometry kernel (`CentreSkeleton::build` + `finish`). Written from the specification and the
//! public names only, before the implementation was read. The truth is analytic: every subject
//! is a stroke around a known centre curve (built by `variable_stroke_outline` or `boolean`
//! union of exact shapes), and the result is measured against that curve, `T(w)` =
//! max(0.05 mm, 3 % of `w`).
#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::similar_names,
    missing_docs
)]

use std::f64::consts::PI;

use curvyo_document_core::{
    DashPattern, Length, LineCap, LineJoin, Markers, Point, Tolerance, Vec2,
};
use curvyo_geometry_core::{
    BooleanOp, CentreLines, CentreOptions, CentrePath, CentreRole, CentreSkeleton, CentreStages,
    DEFAULT_OPTIMISE, DEFAULT_THICK_PARTS, Outline, PathMeasure, StrokeShape, WidthPoint,
    WidthProfile, boolean, contains_point_in_outlines, signed_area_mm2, variable_stroke_outline,
};

type Triple = (Point, Vec2, Vec2);
type Rings = Vec<Vec<Triple>>;

/// `T(w)` of the specification.
fn tol(w: f64) -> f64 {
    (0.03 * w).max(0.05)
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn corner(x: f64, y: f64) -> Triple {
    (p(x, y), Vec2::ZERO, Vec2::ZERO)
}

fn dist(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn rot(pt: (f64, f64), deg: f64) -> (f64, f64) {
    let (s, c) = deg.to_radians().sin_cos();
    (pt.0 * c - pt.1 * s, pt.0 * s + pt.1 * c)
}

// ---------- analytic centre curves ----------

/// A centre curve: Bézier anchors for the stroke and a dense polyline for the truth.
struct Curve {
    anchors: Vec<Triple>,
    closed: bool,
    dense: Vec<Point>,
}

/// A polyline of straight segments.
fn polyline(points: &[(f64, f64)], deg: f64) -> Curve {
    let pts: Vec<(f64, f64)> = points.iter().map(|&q| rot(q, deg)).collect();
    let anchors = pts.iter().map(|&(x, y)| corner(x, y)).collect();
    let mut dense = Vec::new();
    for w in pts.windows(2) {
        for k in 0..2000 {
            let t = f64::from(k) / 2000.0;
            dense.push(p(
                w[0].0 + (w[1].0 - w[0].0) * t,
                w[0].1 + (w[1].1 - w[0].1) * t,
            ));
        }
    }
    let last = pts[pts.len() - 1];
    dense.push(p(last.0, last.1));
    Curve {
        anchors,
        closed: false,
        dense,
    }
}

/// Arcs around `centre` of radius `r`: a list of `(from_deg, to_deg)` pieces joined in order,
/// written as cubic pieces of at most 30 degrees.
fn arc_pieces(pieces: &[((f64, f64), f64, f64, f64)], deg: f64) -> Curve {
    let mut anchors: Vec<Triple> = Vec::new();
    let mut dense = Vec::new();
    for &((cx, cy), r, a0, a1) in pieces {
        let n = ((a1 - a0).abs() / 30.0).ceil().max(1.0) as usize;
        let step = (a1 - a0) / n as f64;
        let k = 4.0 / 3.0 * (step.to_radians() / 4.0).tan() * r;
        for i in 0..=n {
            let a = (a0 + step * i as f64).to_radians();
            let (s, c) = a.sin_cos();
            let pt = rot((cx + r * c, cy + r * s), deg);
            let tangent = rot((-s * k, c * k), deg);
            let handle_in = Vec2::new(-tangent.0, -tangent.1);
            let handle_out = Vec2::new(tangent.0, tangent.1);
            if i == 0 {
                if let Some(last) = anchors.last_mut() {
                    // Joining piece: keep its incoming handle, set the outgoing.
                    last.2 = handle_out;
                    continue;
                }
                anchors.push((p(pt.0, pt.1), Vec2::ZERO, handle_out));
            } else if i == n {
                anchors.push((p(pt.0, pt.1), handle_in, Vec2::ZERO));
            } else {
                anchors.push((p(pt.0, pt.1), handle_in, handle_out));
            }
        }
        for k in 0..=4000 {
            let a = (a0 + (a1 - a0) * f64::from(k) / 4000.0).to_radians();
            let pt = rot((cx + r * a.cos(), cy + r * a.sin()), deg);
            dense.push(p(pt.0, pt.1));
        }
    }
    Curve {
        anchors,
        closed: false,
        dense,
    }
}

fn arc(deg: f64) -> Curve {
    arc_pieces(&[((0.0, 0.0), 25.0, 200.0, 340.0)], deg)
}

fn s_curve(deg: f64) -> Curve {
    arc_pieces(
        &[
            ((0.0, 0.0), 15.0, 180.0, 360.0),
            ((30.0, 0.0), 15.0, 180.0, 0.0),
        ],
        deg,
    )
}

fn straight(deg: f64) -> Curve {
    polyline(&[(0.0, 0.0), (60.0, 0.0)], deg)
}

/// Distance from `q` to a dense polyline.
fn to_dense(q: Point, dense: &[Point]) -> f64 {
    dense
        .windows(2)
        .map(|w| seg_dist(q, w[0], w[1]))
        .fold(f64::INFINITY, f64::min)
}

fn seg_dist(q: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((q.x - a.x) * dx + (q.y - a.y) * dy) / len2).clamp(0.0, 1.0)
    };
    dist(q, p(a.x + dx * t, a.y + dy * t))
}

// ---------- subjects ----------

fn profile(points: &[(f64, f64)], closed: bool) -> WidthProfile {
    let points = points
        .iter()
        .enumerate()
        .map(|(i, &(at, m))| WidthPoint::new(i as u128 + 1, at, m, closed).unwrap())
        .collect();
    WidthProfile::new(points, closed).unwrap()
}

fn shape(width: f64, cap: LineCap, join: LineJoin) -> StrokeShape {
    StrokeShape {
        width: Length::from_mm(width),
        join,
        cap,
        dash: DashPattern::default(),
        markers: Markers::default(),
    }
}

/// The filled outline a stroke of `width` x `profile` paints along `curve`.
fn stroke(curve: &Curve, width: f64, cap: LineCap, prof: &WidthProfile) -> Rings {
    let out = variable_stroke_outline(
        Outline::new(&curve.anchors, curve.closed),
        &shape(width, cap, LineJoin::Round),
        prof,
        Tolerance::from_mm(0.002),
    )
    .unwrap();
    out.outlines()
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect()
        })
        .collect()
}

fn uniform(curve: &Curve, width: f64) -> Rings {
    stroke(curve, width, LineCap::Round, &profile(&[], curve.closed))
}

fn outlines(rings: &Rings) -> Vec<Outline<'_>> {
    rings.iter().map(|r| Outline::new(r, true)).collect()
}

/// The union of several subjects as one region.
fn union(parts: &[Rings]) -> Rings {
    let ops: Vec<Vec<Outline<'_>>> = parts.iter().map(outlines).collect();
    let refs: Vec<&[Outline<'_>]> = ops.iter().map(Vec::as_slice).collect();
    let res = boolean(BooleanOp::Union, &refs, Tolerance::from_mm(0.003)).unwrap();
    res.outlines()
        .iter()
        .map(|ring| ring.iter().map(|q| corner(q.x, q.y)).collect())
        .collect()
}

/// A circle of four cubic arcs (radius error below 0.03 %).
fn circle(cx: f64, cy: f64, r: f64, reverse: bool) -> Vec<Triple> {
    let k = 0.552_284_749_830_793_4 * r;
    let mut a = vec![
        (p(cx + r, cy), Vec2::new(0.0, -k), Vec2::new(0.0, k)),
        (p(cx, cy + r), Vec2::new(k, 0.0), Vec2::new(-k, 0.0)),
        (p(cx - r, cy), Vec2::new(0.0, k), Vec2::new(0.0, -k)),
        (p(cx, cy - r), Vec2::new(-k, 0.0), Vec2::new(k, 0.0)),
    ];
    if reverse {
        a.reverse();
        for t in &mut a {
            std::mem::swap(&mut t.1, &mut t.2);
        }
    }
    a
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64, reverse: bool) -> Vec<Triple> {
    let mut a = vec![
        corner(x0, y0),
        corner(x1, y0),
        corner(x1, y1),
        corner(x0, y1),
    ];
    if reverse {
        a.reverse();
    }
    a
}

/// An annulus around `(cx, cy)` with centre radius `r` and width `w`.
fn annulus(cx: f64, cy: f64, r: f64, w: f64) -> Rings {
    vec![
        circle(cx, cy, r + w / 2.0, false),
        circle(cx, cy, r - w / 2.0, true),
    ]
}

// ---------- running and measuring ----------

fn run_with(rings: &Rings, options: &CentreOptions, cutouts: &[Rings]) -> CentreLines {
    let cut: Vec<Outline<'_>> = cutouts.iter().flat_map(outlines).collect();
    CentreSkeleton::build(&outlines(rings))
        .unwrap()
        .finish(options, &cut)
}

/// The defaults without a Thick parts limit: the strokes here reach 8 mm, wider than the 6 mm
/// default (customer 2026-11-18); criterion 12's tests set Thick parts themselves.
fn run(rings: &Rings) -> CentreLines {
    run_with(
        rings,
        &CentreOptions {
            thick_parts: None,
            ..CentreOptions::default()
        },
        &[],
    )
}

fn with_overlap(rings: &Rings, overlap: f64) -> CentreLines {
    // Without a Thick parts limit: the 8 mm T is wider than its 6 mm default (customer
    // 2026-11-18), and these tests are about the branch start.
    run_with(
        rings,
        &CentreOptions {
            overlap,
            thick_parts: None,
            ..CentreOptions::default()
        },
        &[],
    )
}

fn measure(path: &CentrePath) -> PathMeasure {
    PathMeasure::new(&path.anchors, path.closed)
}

/// The width the result paints at arc length `s`.
fn width_at(path: &CentrePath, m: &PathMeasure, s: f64) -> f64 {
    let len = m.length().as_mm();
    path.width.as_mm() * path.profile.sample((s / len).clamp(0.0, 1.0)).unwrap()
}

/// `n + 1` evenly spaced points on the result with their widths.
fn samples(path: &CentrePath, n: usize) -> Vec<(Point, f64)> {
    let m = measure(path);
    let len = m.length().as_mm();
    (0..=n)
        .map(|i| {
            let s = len * i as f64 / n as f64;
            (m.at(Length::from_mm(s)).point, width_at(path, &m, s))
        })
        .collect()
}

fn ends(path: &CentrePath) -> (Point, Point) {
    let m = measure(path);
    (m.at(Length::from_mm(0.0)).point, m.at(m.length()).point)
}

fn mains(lines: &CentreLines) -> Vec<&CentrePath> {
    lines
        .paths
        .iter()
        .filter(|q| q.role == CentreRole::Main)
        .collect()
}

fn branches(lines: &CentreLines) -> Vec<&CentrePath> {
    lines
        .paths
        .iter()
        .filter(|q| q.role == CentreRole::Branch)
        .collect()
}

/// The single main path of a stroke; fails if there are branches or several paths.
fn single(lines: &CentreLines, what: &str) -> CentrePath {
    assert_eq!(
        lines.paths.len(),
        1,
        "{what}: one path expected, got {} ({} filled)",
        lines.paths.len(),
        lines.filled.len()
    );
    assert!(lines.filled.is_empty(), "{what}: nothing kept filled");
    let path = lines.paths[0].clone();
    assert_eq!(path.role, CentreRole::Main, "{what}");
    path
}

/// The largest distance from the result to the truth and from the truth to the result.
fn axis_errors(path: &CentrePath, truth: &[Point]) -> (f64, f64) {
    let pts: Vec<Point> = samples(path, 800).into_iter().map(|s| s.0).collect();
    let out = pts.iter().map(|&q| to_dense(q, truth)).fold(0.0, f64::max);
    let step = (truth.len() / 400).max(1);
    let back = truth
        .iter()
        .step_by(step)
        .map(|&q| to_dense(q, &pts))
        .fold(0.0, f64::max);
    (out, back)
}

/// The widths at 20 points `t = (i + 0.5) / 20` of the truth: `(truth point, measured width)`.
fn widths_at_20(path: &CentrePath, truth: &[Point]) -> Vec<(usize, f64)> {
    let m = measure(path);
    (0..20)
        .map(|i| {
            let idx = ((f64::from(i) + 0.5) / 20.0 * (truth.len() - 1) as f64) as usize;
            let s = m.nearest(truth[idx]).as_mm();
            (idx, width_at(path, &m, s))
        })
        .collect()
}

/// The painted outline of one result path, with the cap of its role.
fn paint(path: &CentrePath) -> Rings {
    let cap = match path.role {
        CentreRole::Main => LineCap::Round,
        CentreRole::Branch => LineCap::Butt,
    };
    let out = variable_stroke_outline(
        Outline::new(&path.anchors, path.closed),
        &shape(path.width.as_mm(), cap, LineJoin::Round),
        &path.profile,
        Tolerance::from_mm(0.002),
    )
    .unwrap();
    out.outlines()
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect()
        })
        .collect()
}

fn area(rings: &Rings) -> f64 {
    let ops = [outlines(rings)];
    let refs: Vec<&[Outline<'_>]> = ops.iter().map(Vec::as_slice).collect();
    let res = boolean(BooleanOp::Union, &refs, Tolerance::from_mm(0.003)).unwrap();
    res.outlines()
        .iter()
        .map(|r| signed_area_mm2(r))
        .sum::<f64>()
        .abs()
}

/// The area painted by exactly one of `a`, `b`.
fn xor_area(a: &Rings, b: &Rings) -> f64 {
    let (oa, ob) = (outlines(a), outlines(b));
    let refs: Vec<&[Outline<'_>]> = vec![&oa, &ob];
    match boolean(BooleanOp::Exclusion, &refs, Tolerance::from_mm(0.003)) {
        Ok(res) => res
            .outlines()
            .iter()
            .map(|r| signed_area_mm2(r))
            .sum::<f64>()
            .abs(),
        Err(_) => 0.0,
    }
}

// ---------- criterion 5: constant strokes ----------

fn check_constant(name: &str, curve: &Curve, w: f64) -> (f64, f64) {
    let subject = uniform(curve, w);
    let lines = run(&subject);
    let path = single(&lines, name);
    let t = tol(w);
    let (out, back) = axis_errors(&path, &curve.dense);
    assert!(
        out <= t,
        "{name} w {w}: result off the axis by {out:.4} > {t}"
    );
    assert!(back <= t, "{name} w {w}: axis not covered, {back:.4} > {t}");
    let mut worst_w: f64 = 0.0;
    for (idx, got) in widths_at_20(&path, &curve.dense) {
        let err = (got - w).abs();
        worst_w = worst_w.max(err);
        assert!(
            err <= t,
            "{name} w {w}: width {got:.4} at truth #{idx}, err {err:.4} > {t}"
        );
    }
    // Round ends: the main path ends at the cap centres (w/2 before the tips).
    let (a, b) = ends(&path);
    let (c0, c1) = (curve.dense[0], curve.dense[curve.dense.len() - 1]);
    let e = (dist(a, c0) + dist(b, c1)).min(dist(a, c1) + dist(b, c0));
    assert!(
        e <= 2.0 * t,
        "{name} w {w}: ends {a:?} {b:?} not at {c0:?} {c1:?}"
    );
    // Painted result vs subject: at most 4 % of the area (CENTRE-DEFAULT A, 2026-11-17).
    let painted = paint(&path);
    let x = xor_area(&painted, &subject);
    let a_sub = area(&subject);
    assert!(
        x <= 0.04 * a_sub,
        "{name} w {w}: painted differs by {x:.3} mm² of {a_sub:.3} ({:.2} %)",
        100.0 * x / a_sub
    );
    (out.max(back), worst_w)
}

#[test]
fn c5_straight_arc_s_at_2mm_axis_width_and_area() {
    for (name, curve) in [
        ("straight", straight(0.0)),
        ("arc", arc(0.0)),
        ("s", s_curve(0.0)),
    ] {
        let (axis, width) = check_constant(name, &curve, 2.0);
        eprintln!(
            "c5 {name} 2mm: axis {axis:.4} width {width:.4} (T {:.3})",
            tol(2.0)
        );
    }
}

#[test]
fn c5_wide_strokes_5_to_8mm_at_rotations() {
    let mut worst = (0.0f64, 0.0f64);
    for w in [5.0, 6.5, 8.0] {
        for deg in [0.0, 17.0, 45.0, 90.0, 133.0] {
            for (name, curve) in [
                ("straight", straight(deg)),
                ("arc", arc(deg)),
                ("s", s_curve(deg)),
            ] {
                let (axis, width) = check_constant(&format!("{name}@{deg}"), &curve, w);
                worst = (worst.0.max(axis / tol(w)), worst.1.max(width / tol(w)));
            }
        }
    }
    eprintln!(
        "c5 wide: worst axis {:.2} T, worst width {:.2} T (lead asks <= 0.5 T for width)",
        worst.0, worst.1
    );
}

// ---------- criterion 6: taper ----------

fn linear_taper(from: f64, to: f64, n: usize) -> Vec<(f64, f64)> {
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            (t, (from + (to - from) * t) / from)
        })
        .collect()
}

#[test]
fn c6_linear_taper_3_to_half_mm() {
    for deg in [0.0, 30.0, 90.0] {
        let curve = straight(deg);
        let subject = stroke(
            &curve,
            3.0,
            LineCap::Round,
            &profile(&linear_taper(3.0, 0.5, 60), false),
        );
        let lines = run(&subject);
        let path = single(&lines, "taper");
        let (out, back) = axis_errors(&path, &curve.dense);
        assert!(
            out <= tol(3.0) && back <= tol(3.0),
            "taper axis {out} {back}"
        );
        assert!(
            (path.width.as_mm() - 3.0).abs() <= tol(3.0),
            "base width = largest"
        );
        let mut worst: f64 = 0.0;
        for (idx, got) in widths_at_20(&path, &curve.dense) {
            let t = idx as f64 / (curve.dense.len() - 1) as f64;
            let truth = 3.0 - 2.5 * t;
            let err = (got - truth).abs();
            worst = worst.max(err / tol(truth));
            assert!(
                err <= tol(truth),
                "taper@{deg} t {t:.3}: width {got:.4} vs {truth:.4}"
            );
        }
        let points = path.profile.positions().count();
        eprintln!("c6 taper@{deg}: {points} profile points, worst width {worst:.2} T");
    }
}

// ---------- criterion 7: ring ----------

#[test]
fn c7_ring_is_one_closed_path_with_periodic_profile() {
    for (r, w) in [(10.0, 2.0), (15.0, 5.0), (20.0, 8.0)] {
        let lines = run(&annulus(3.0, -4.0, r, w));
        let path = single(&lines, "ring");
        assert!(path.closed, "ring w {w} gives a closed path");
        assert!(path.profile.is_closed(), "periodic profile");
        let t = tol(w);
        for (q, got) in samples(&path, 400) {
            let radial = (dist(q, p(3.0, -4.0)) - r).abs();
            assert!(radial <= t, "ring w {w}: axis off by {radial}");
            assert!((got - w).abs() <= t, "ring w {w}: width {got}");
        }
        let m = measure(&path);
        let len = m.length().as_mm();
        let step = (width_at(&path, &m, 0.0) - width_at(&path, &m, len * 0.9999)).abs();
        assert!(step <= t, "seam step {step}");
        let (a, b) = ends(&path);
        assert!(dist(a, b) < 1e-6, "closed path meets itself");
    }
}

/// The outline of a round eye drawn as a stroke along a closed circle.
#[test]
fn c7_stroked_circle_outline_is_one_closed_path() {
    let c = Curve {
        anchors: circle(0.0, 0.0, 12.0, false),
        closed: true,
        dense: (0..=4000)
            .map(|k| {
                let a = 2.0 * PI * f64::from(k) / 4000.0;
                p(12.0 * a.cos(), 12.0 * a.sin())
            })
            .collect(),
    };
    let lines = run(&uniform(&c, 2.5));
    let path = single(&lines, "eye outline");
    assert!(path.closed);
    let (out, back) = axis_errors(&path, &c.dense);
    assert!(out <= tol(2.5) && back <= tol(2.5), "{out} {back}");
}

// ---------- criterion 8: ends ----------

fn t_shape(w: f64, deg: f64) -> (Rings, Curve, Curve) {
    let bar = polyline(&[(-19.0, 0.0), (19.0, 0.0)], deg);
    let stem = polyline(&[(0.0, 0.0), (0.0, 20.0)], deg);
    (union(&[uniform(&bar, w), uniform(&stem, w)]), bar, stem)
}

#[test]
fn c8_main_round_end_and_branch_taper_to_the_tip() {
    for w in [2.0, 5.0, 8.0] {
        let (subject, bar, _) = t_shape(w, 0.0);
        let lines = run(&subject);
        let main = mains(&lines);
        let br = branches(&lines);
        assert_eq!(
            (main.len(), br.len()),
            (1, 1),
            "T w {w}: one main, one branch"
        );
        let t = tol(w);
        // Main: the round ends are w/2 before the tips (x = +-19).
        let (a, b) = ends(main[0]);
        let xs = [a.x.min(b.x), a.x.max(b.x)];
        assert!(
            (xs[0] + 19.0).abs() <= t && (xs[1] - 19.0).abs() <= t,
            "T w {w}: main ends {xs:?}"
        );
        assert!(a.y.abs() <= t && b.y.abs() <= t);
        let _ = bar;
        // Branch: the free end reaches the tip (y = 20 + w/2) with width 0, and over the last
        // w/2 the width follows 2 sqrt(r^2 - s^2).
        let branch = br[0];
        let m = measure(branch);
        let len = m.length().as_mm();
        let (s0, s1) = ends(branch);
        let (tip_s, tip) = if s0.y > s1.y { (0.0, s0) } else { (len, s1) };
        let r = w / 2.0;
        assert!(
            dist(tip, p(0.0, 20.0 + r)) <= t,
            "T w {w}: branch tip {tip:?} not at the shape's tip"
        );
        let w_tip = width_at(branch, &m, tip_s);
        assert!(w_tip <= tol(0.0), "T w {w}: width at tip {w_tip}");
        for s in [0.0, 0.5 * r, 0.8 * r] {
            let q = p(0.0, 20.0 + s);
            let at = m.nearest(q).as_mm();
            let got = width_at(branch, &m, at);
            let truth = 2.0 * (r * r - s * s).sqrt();
            // Criterion 8 as amended (CENTRE-TIP A, 2026-11-17): the four-point round end
            // stays within 0.08 r of the semicircle on each side, on top of T(w).
            assert!(
                (got - truth).abs() <= t + 2.0 * 0.08 * r,
                "T w {w}: round end width at s {s}: {got:.4} vs {truth:.4}"
            );
        }
    }
}

#[test]
fn c8_pointed_taper_reaches_the_tip_with_zero_width() {
    let curve = straight(20.0);
    let subject = stroke(
        &curve,
        3.0,
        LineCap::Butt,
        &profile(&linear_taper(3.0, 0.0, 60), false),
    );
    let lines = run(&subject);
    let path = single(&lines, "pointed taper");
    let tip = curve.dense[curve.dense.len() - 1];
    let m = measure(&path);
    let (a, b) = ends(&path);
    let (s_tip, end) = if dist(a, tip) < dist(b, tip) {
        (0.0, a)
    } else {
        (m.length().as_mm(), b)
    };
    assert!(
        dist(end, tip) <= tol(3.0),
        "ends {:.4} from the tip",
        dist(end, tip)
    );
    let w_tip = width_at(&path, &m, s_tip);
    assert!(w_tip <= tol(0.0), "width at the tip {w_tip}");
}

/// "A flat end counts as rounded": a Butt-capped 60 mm stroke ends w/2 before each flat end.
#[test]
fn c8_flat_end_counts_as_rounded() {
    let mut bad = Vec::new();
    for w in [2.0, 4.0, 5.0, 6.0, 8.0] {
        let curve = straight(0.0);
        let subject = stroke(&curve, w, LineCap::Butt, &profile(&[], false));
        let lines = run(&subject);
        let path = single(&lines, "flat");
        let (a, b) = ends(&path);
        let xs = [a.x.min(b.x), a.x.max(b.x)];
        let t = tol(w);
        if (xs[0] - w / 2.0).abs() > t || (xs[1] - (60.0 - w / 2.0)).abs() > t {
            bad.push(format!(
                "w {w}: ends {:.3} {:.3}, want {} {} (T {t:.3})",
                xs[0],
                xs[1],
                w / 2.0,
                60.0 - w / 2.0
            ));
        }
    }
    assert!(bad.is_empty(), "flat ends: {bad:?}");
}

/// The flat-ended T polygon (bar 40 x 2 mm): the main path ends w/2 before each flat end, on the
/// bar's centre line, with no corner fork left (criterion 8 Proposal, `adrs.md` amendment).
#[test]
fn c8_flat_ends_of_a_t_polygon() {
    let tee = vec![
        [
            (0.0, 0.0),
            (40.0, 0.0),
            (40.0, 2.0),
            (21.0, 2.0),
            (21.0, 22.0),
            (19.0, 22.0),
            (19.0, 2.0),
            (0.0, 2.0),
        ]
        .iter()
        .map(|&(x, y)| corner(x, y))
        .collect::<Vec<_>>(),
    ];
    let lines = run(&tee);
    let main = mains(&lines);
    assert_eq!(main.len(), 1);
    let (a, b) = ends(main[0]);
    let (l, r) = if a.x < b.x { (a, b) } else { (b, a) };
    let (el, er) = (dist(l, p(1.0, 1.0)), dist(r, p(39.0, 1.0)));
    assert!(
        el <= tol(2.0) && er <= tol(2.0),
        "main ends {l:?} {r:?}: {el:.3} and {er:.3} from (1, 1) and (39, 1)"
    );
}

// ---------- criterion 9: main path choice ----------

#[test]
fn c9_t_main_is_the_bar_at_rotations_and_widths() {
    for w in [2.0, 5.0, 8.0] {
        for deg in [0.0, 17.0, 45.0, 90.0, 133.0] {
            let (subject, bar, stem) = t_shape(w, deg);
            let lines = run(&subject);
            let main = mains(&lines);
            assert_eq!(main.len(), 1, "T w {w} @ {deg}: one main");
            assert_eq!(branches(&lines).len(), 1, "T w {w} @ {deg}: one branch");
            let t = tol(w);
            for (q, _) in samples(main[0], 200) {
                assert!(
                    to_dense(q, &bar.dense) <= t,
                    "T w {w} @ {deg}: main leaves the bar"
                );
            }
            // The branch runs on the stem's axis to the tip, w/2 past the cap centre.
            let tip = polyline(&[(0.0, 0.0), (0.0, 20.0 + w / 2.0)], deg);
            let _ = stem;
            for (q, _) in samples(branches(&lines)[0], 200) {
                assert!(
                    to_dense(q, &tip.dense) <= t,
                    "T w {w} @ {deg}: branch leaves the stem"
                );
            }
        }
    }
}

#[test]
fn c9_y_main_takes_the_straight_wide_arm() {
    let stem_arm = polyline(&[(0.0, -20.0), (0.0, 20.0)], 0.0);
    let (s, c) = 60f64.to_radians().sin_cos();
    let side = polyline(&[(0.0, 0.0), (18.0 * s, 18.0 * c)], 0.0);
    let subject = union(&[uniform(&stem_arm, 2.0), uniform(&side, 1.0)]);
    let lines = run(&subject);
    let main = mains(&lines);
    assert_eq!(main.len(), 1);
    let (a, b) = ends(main[0]);
    let ys = [a.y.min(b.y), a.y.max(b.y)];
    assert!(
        (ys[0] + 20.0).abs() <= tol(2.0) && (ys[1] - 20.0).abs() <= tol(2.0),
        "Y main ends {a:?} {b:?}"
    );
    assert!(a.x.abs() <= tol(2.0) && b.x.abs() <= tol(2.0));
    assert_eq!(branches(&lines).len(), 1, "the 60 degree arm is a branch");
}

fn fingerprint(lines: &CentreLines) -> String {
    format!(
        "{:?}|{:?}|{:?}",
        lines.paths, lines.filled, lines.inner_areas
    )
}

/// Equal candidates (a symmetric cross) are broken the same way on every run, also when the
/// outline starts at another node, and the same way after a translation.
#[test]
fn c9_tie_break_is_stable() {
    let h = polyline(&[(-20.0, 0.0), (20.0, 0.0)], 0.0);
    let v = polyline(&[(0.0, -20.0), (0.0, 20.0)], 0.0);
    let subject = union(&[uniform(&h, 2.0), uniform(&v, 2.0)]);
    let first = run(&subject);
    for _ in 0..3 {
        assert_eq!(
            fingerprint(&first),
            fingerprint(&run(&subject)),
            "same run, same result"
        );
    }
    let main_ends = |l: &CentreLines| {
        let m = mains(l);
        assert_eq!(m.len(), 1);
        let (a, b) = ends(m[0]);
        if (a.x, a.y) < (b.x, b.y) {
            (a, b)
        } else {
            (b, a)
        }
    };
    let (a0, b0) = main_ends(&first);
    // Shift the start node of every ring.
    for shift in [1, 7, 13] {
        let shifted: Rings = subject
            .iter()
            .map(|r| {
                let k = shift % r.len();
                r[k..].iter().chain(r[..k].iter()).copied().collect()
            })
            .collect();
        let (a, b) = main_ends(&run(&shifted));
        assert!(
            dist(a, a0) <= tol(2.0) && dist(b, b0) <= tol(2.0),
            "start shift {shift}: main {a:?}-{b:?} vs {a0:?}-{b0:?}"
        );
    }
    // Translate.
    let moved: Rings = subject
        .iter()
        .map(|r| {
            r.iter()
                .map(|&(q, i, o)| (p(q.x + 100.0, q.y + 50.0), i, o))
                .collect()
        })
        .collect();
    let (a, b) = main_ends(&run(&moved));
    assert!(
        dist(a, p(a0.x + 100.0, a0.y + 50.0)) <= tol(2.0)
            && dist(b, p(b0.x + 100.0, b0.y + 50.0)) <= tol(2.0),
        "translated main {a:?}-{b:?} vs {a0:?}-{b0:?}"
    );
}

/// An L (one stroke with a corner) at 2, 5 and 8 mm is one path along the corner.
#[test]
fn c9_l_is_one_path() {
    let mut bad = Vec::new();
    for w in [2.0, 5.0, 8.0] {
        for deg in [0.0, 60.0] {
            let curve = polyline(&[(0.0, 0.0), (30.0, 0.0), (30.0, 30.0)], deg);
            let lines = run(&uniform(&curve, w));
            let path = single(&lines, &format!("L w {w} @ {deg}"));
            let (out, back) = axis_errors(&path, &curve.dense);
            eprintln!(
                "c9 L w {w} @ {deg}: axis out {out:.4} back {back:.4} (T {:.3})",
                tol(w)
            );
            if out > tol(w) || back > tol(w) {
                bad.push(format!(
                    "w {w} @ {deg}: out {out:.3} back {back:.3} > T {:.3}",
                    tol(w)
                ));
            }
        }
    }
    assert!(bad.is_empty(), "L off the axis at the corner: {bad:?}");
}

// ---------- criterion 10: junctions ----------

/// Distance from a point to a result path's centre line.
fn to_path(q: Point, path: &CentrePath) -> f64 {
    let m = measure(path);
    dist(q, m.at(m.nearest(q)).point)
}

/// The branch end nearest to `parent` and its distance to the parent's centre line.
fn attached_end(branch: &CentrePath, parent: &CentrePath) -> (Point, f64, f64) {
    let m = measure(branch);
    let (a, b) = ends(branch);
    let (da, db) = (to_path(a, parent), to_path(b, parent));
    if da <= db {
        (a, da, width_at(branch, &m, 0.0))
    } else {
        (b, db, width_at(branch, &m, m.length().as_mm()))
    }
}

#[test]
fn c10_branch_start_follows_the_overlap() {
    for w in [2.0, 5.0, 8.0] {
        for deg in [0.0, 33.0] {
            let (subject, _, _) = t_shape(w, deg);
            for overlap in [0.2, 0.0, -0.2] {
                let lines = with_overlap(&subject, overlap);
                let main = mains(&lines);
                let br = branches(&lines);
                assert_eq!((main.len(), br.len()), (1, 1));
                let (_, d, w_start) = attached_end(br[0], main[0]);
                let want = (0.5 - overlap) * w;
                assert!(
                    (d - want).abs() <= 0.01,
                    "T w {w} @ {deg} p {overlap}: start {d:.4} from the bar axis, want {want:.4}"
                );
                assert!(
                    (w_start - w).abs() <= tol(w),
                    "T w {w} @ {deg} p {overlap}: start width {w_start:.4}"
                );
            }
        }
    }
}

/// A loop crossed by a bar (theta): every branch is attached at both of its ends.
#[test]
fn c10_loop_with_junctions_attaches_branches_at_both_ends() {
    let w = 2.0;
    let bar = polyline(&[(-15.0, 0.0), (15.0, 0.0)], 0.0);
    let subject = union(&[annulus(0.0, 0.0, 15.0, w), uniform(&bar, w)]);
    let lines = run(&subject);
    let br = branches(&lines);
    assert!(!br.is_empty(), "theta has a branch");
    for (i, b) in br.iter().enumerate() {
        let (a, z) = ends(b);
        for e in [a, z] {
            let d = lines
                .paths
                .iter()
                .filter(|q| !std::ptr::eq(*q, *b))
                .map(|q| to_path(e, q))
                .fold(f64::INFINITY, f64::min);
            assert!(
                (d - 0.6).abs() <= 0.01,
                "branch {i}: end {e:?} is {d:.4} from the other centre lines, want 0.6"
            );
        }
    }
}

/// A branch of a branch attaches to its parent branch the same way.
#[test]
fn c10_branch_of_a_branch() {
    let w = 2.0;
    let bar = polyline(&[(-25.0, 0.0), (25.0, 0.0)], 0.0);
    let stem = polyline(&[(0.0, 0.0), (0.0, 30.0)], 0.0);
    let twig = polyline(&[(0.0, 15.0), (12.0, 15.0)], 0.0);
    let subject = union(&[uniform(&bar, w), uniform(&stem, w), uniform(&twig, w)]);
    let lines = run(&subject);
    assert_eq!(mains(&lines).len(), 1);
    let br = branches(&lines);
    assert_eq!(br.len(), 2, "stem and twig are branches");
    let is_twig = |b: &CentrePath| {
        let (a, z) = ends(b);
        a.x.max(z.x) > 5.0
    };
    let twig_path = br.iter().find(|b| is_twig(b)).expect("a twig");
    let stem_path = br.iter().find(|b| !is_twig(b)).expect("a stem");
    let (_, d, _) = attached_end(twig_path, stem_path);
    assert!(
        (d - 0.6).abs() <= 0.01,
        "twig starts {d:.4} from the stem axis"
    );
}

/// A spur shorter than the local width is no branch.
#[test]
fn c10_short_spur_is_no_branch() {
    let w = 2.0;
    let bar = polyline(&[(-20.0, 0.0), (20.0, 0.0)], 0.0);
    let spur = polyline(&[(0.0, 0.0), (0.0, 1.5)], 0.0);
    let subject = union(&[uniform(&bar, w), uniform(&spur, w)]);
    let lines = run(&subject);
    single(&lines, "bar with a short spur");
}

// ---------- criterion 11: holes ----------

fn frame_two_holes() -> (Rings, Vec<Vec<Triple>>) {
    let holes = vec![
        rect(3.0, 3.0, 27.0, 27.0, true),
        rect(33.0, 3.0, 57.0, 27.0, true),
    ];
    let mut rings = vec![rect(0.0, 0.0, 60.0, 30.0, false)];
    rings.extend(holes.iter().cloned());
    (rings, holes)
}

#[test]
fn c11_no_path_crosses_a_hole_and_holes_stay_open() {
    let ring = annulus(0.0, 0.0, 10.0, 2.0);
    let (frame, holes) = frame_two_holes();
    for (name, subject, hole_rings) in [
        ("ring", ring.clone(), vec![ring[1].clone()]),
        ("frame", frame, holes),
    ] {
        let lines = run(&subject);
        assert!(!lines.paths.is_empty(), "{name}: converted");
        for (i, path) in lines.paths.iter().enumerate() {
            for (q, _) in samples(path, 600) {
                for h in &hole_rings {
                    assert!(
                        !contains_point_in_outlines(&[Outline::new(h, true)], q),
                        "{name}: path {i} crosses a hole at {q:?}"
                    );
                }
            }
        }
        // The hole is not painted over by the result.
        let painted: Rings = lines.paths.iter().flat_map(paint).collect();
        let po = outlines(&painted);
        for h in &hole_rings {
            let n = h.len() as f64;
            let c = p(
                h.iter().map(|t| t.0.x).sum::<f64>() / n,
                h.iter().map(|t| t.0.y).sum::<f64>() / n,
            );
            assert!(
                !contains_point_in_outlines(&po, c),
                "{name}: hole at {c:?} filled in"
            );
        }
    }
}

#[test]
fn c11_frame_loop_around_a_hole_is_closed_or_attached() {
    let (frame, _) = frame_two_holes();
    let lines = run(&frame);
    // Two loops sharing the middle bar: either closed paths, or branches attached at both
    // ends (no free ends anywhere: the shape has no tips).
    for (i, path) in lines.paths.iter().enumerate() {
        if path.closed {
            continue;
        }
        let (a, z) = ends(path);
        for e in [a, z] {
            let d = lines
                .paths
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, q)| to_path(e, q))
                .fold(f64::INFINITY, f64::min);
            assert!(
                d <= 3.0,
                "frame path {i} has a free end at {e:?} ({d:.3} from others)"
            );
        }
    }
}

// ---------- criterion 12: blobs ----------

#[test]
fn c12_disc_stays_filled_even_at_threshold_one() {
    let disc = vec![circle(5.0, 5.0, 5.0, false)];
    // Thick parts at the default and at the maximum (it replaced the blob ratio 0.5 and 1.0,
    // customer 2026-11-18).
    for ratio in [Some(Length::from_mm(6.0)), None] {
        let lines = run_with(
            &disc,
            &CentreOptions {
                thick_parts: ratio,
                ..CentreOptions::default()
            },
            &[],
        );
        assert!(lines.paths.is_empty(), "disc @ {ratio:?}: no centre path");
        assert!(lines.all_filled());
        assert_eq!(
            lines.filled.len(),
            1,
            "disc @ {ratio:?}: kept as one filled region"
        );
    }
}

#[test]
fn c12_eye_ring_iris_and_highlight() {
    // Ring: a centre line. Iris with a highlight hole: stays filled, the hole kept.
    let ring = run(&annulus(0.0, 0.0, 10.0, 2.0));
    assert_eq!(ring.paths.len(), 1);
    assert!(ring.paths[0].closed);
    let iris = vec![circle(0.0, 0.0, 4.0, false), circle(1.5, -1.5, 0.8, true)];
    let lines = run(&iris);
    assert!(lines.paths.is_empty(), "iris stays filled");
    assert_eq!(lines.filled.len(), 1);
    assert_eq!(
        lines.filled[0].len(),
        2,
        "the highlight hole survives in the filled region"
    );
    let highlight = run(&vec![circle(1.5, -1.5, 0.8, false)]);
    assert!(highlight.paths.is_empty() && highlight.filled.len() == 1);
}

#[test]
fn c12_thin_capsule_converts_and_square_stays() {
    // 2 mm x 10 mm line: ratio 0.2.
    let cap = uniform(&polyline(&[(0.0, 0.0), (8.0, 0.0)], 0.0), 2.0);
    let defaults = |rings: &Rings| run_with(rings, &CentreOptions::default(), &[]);
    assert_eq!(defaults(&cap).paths.len(), 1);
    let sq = vec![rect(0.0, 0.0, 10.0, 10.0, false)];
    // At the default Thick parts (6 mm; it replaced the ratio 0.5) the 10 mm square stays.
    assert!(
        defaults(&sq).paths.is_empty(),
        "square ratio 0.7 stays filled"
    );
}

// ---------- criteria 13, 14: inner areas ----------

fn inner(rings: &Rings, offset: Option<f64>, cutouts: &[Rings]) -> CentreLines {
    run_with(
        rings,
        &CentreOptions {
            inner_offset: offset.map(Length::from_mm),
            ..CentreOptions::default()
        },
        cutouts,
    )
}

fn radii(poly: &[Point], c: Point) -> (f64, f64) {
    poly.iter()
        .map(|&q| dist(q, c))
        .fold((f64::INFINITY, 0.0), |(lo, hi), r| (lo.min(r), hi.max(r)))
}

#[test]
fn c13_inner_area_off_on_and_offsets() {
    let subject = annulus(0.0, 0.0, 10.0, 2.0); // hole radius r - w/2 = 9
    let hole_r = 9.0;
    assert!(
        inner(&subject, None, &[]).inner_areas.is_empty(),
        "off: no inner area"
    );
    for off in [0.3, -0.3, 1.0] {
        let lines = inner(&subject, Some(off), &[]);
        assert_eq!(lines.inner_areas.len(), 1, "offset {off}: one inner area");
        let (lo, hi) = radii(&lines.inner_areas[0], p(0.0, 0.0));
        let want = hole_r - off;
        assert!(
            (lo - want).abs() <= 0.01 && (hi - want).abs() <= 0.01,
            "offset {off}: radii {lo:.4}..{hi:.4}, want {want}"
        );
        assert!(
            signed_area_mm2(&lines.inner_areas[0]) > 0.0,
            "outer outline positive"
        );
        assert!(!lines.paths.is_empty(), "the ring is still converted");
    }
}

#[test]
fn c13_hole_smaller_than_offset_gives_none() {
    let bar = vec![
        rect(0.0, 0.0, 40.0, 4.0, false),
        circle(20.0, 2.0, 0.2, true),
    ];
    let lines = inner(&bar, Some(0.3), &[]);
    assert!(
        lines.inner_areas.is_empty(),
        "a 0.2 mm hole with 0.3 mm offset gives none"
    );
}

#[test]
fn c14_shape_inside_a_hole_is_cut_out_grown_by_the_offset() {
    let eye = annulus(0.0, 0.0, 10.0, 2.0); // hole radius 9
    let pupil = vec![circle(0.0, 0.0, 3.0, false)];
    let lines = inner(&eye, Some(0.3), &[pupil]);
    let mut outer = 0;
    let mut holes = 0;
    for poly in &lines.inner_areas {
        let (lo, hi) = radii(poly, p(0.0, 0.0));
        let a = signed_area_mm2(poly);
        if a > 0.0 {
            outer += 1;
            assert!(
                (lo - 8.7).abs() <= 0.01 && (hi - 8.7).abs() <= 0.01,
                "outer {lo}..{hi}"
            );
        } else {
            holes += 1;
            assert!(
                (lo - 3.3).abs() <= 0.01 && (hi - 3.3).abs() <= 0.01,
                "hole {lo}..{hi}"
            );
        }
    }
    assert_eq!((outer, holes), (1, 1), "inner area with the pupil cut out");
}

// ---------- determinism ----------

#[test]
fn determinism_same_input_same_output() {
    let (t, _, _) = t_shape(5.0, 17.0);
    let (frame, _) = frame_two_holes();
    for subject in [t, frame, annulus(0.0, 0.0, 10.0, 2.0)] {
        let options = CentreOptions {
            inner_offset: Some(Length::from_mm(0.3)),
            ..CentreOptions::default()
        };
        let a = fingerprint(&run_with(&subject, &options, &[]));
        let skeleton = CentreSkeleton::build(&outlines(&subject)).unwrap();
        for _ in 0..3 {
            assert_eq!(a, fingerprint(&skeleton.finish(&options, &[])));
            assert_eq!(a, fingerprint(&run_with(&subject, &options, &[])));
        }
    }
}

/// Finishing one skeleton with other options and back gives the first result again.
#[test]
fn determinism_finish_is_pure_in_the_options() {
    let (t, _, _) = t_shape(2.0, 0.0);
    let skeleton = CentreSkeleton::build(&outlines(&t)).unwrap();
    let base = fingerprint(&skeleton.finish(&CentreOptions::default(), &[]));
    let _ = skeleton.finish(
        &CentreOptions {
            overlap: -0.4,
            thick_parts: Some(Length::from_mm(0.5)),
            inner_offset: Some(Length::from_mm(-1.0)),
            optimise: curvyo_geometry_core::DEFAULT_OPTIMISE,
        },
        &[],
    );
    assert_eq!(
        base,
        fingerprint(&skeleton.finish(&CentreOptions::default(), &[]))
    );
}

#[test]
fn degenerate_inputs_do_not_panic() {
    let nan = vec![vec![
        corner(0.0, 0.0),
        corner(f64::NAN, 1.0),
        corner(1.0, 1.0),
    ]];
    assert!(
        CentreSkeleton::build(&outlines(&nan)).is_err(),
        "nonfinite refused"
    );
    let far = vec![rect(0.0, 0.0, 1e12, 1.0, false)];
    assert!(
        CentreSkeleton::build(&outlines(&far)).is_err(),
        "out of range refused"
    );
    for rings in [
        vec![vec![corner(0.0, 0.0), corner(1.0, 0.0), corner(2.0, 0.0)]],
        vec![vec![corner(0.0, 0.0)]],
        Vec::new(),
        vec![rect(0.0, 0.0, 0.001, 50.0, false)],
    ] {
        if let Ok(s) = CentreSkeleton::build(&outlines(&rings)) {
            let _ = s.finish(&CentreOptions::default(), &[]);
        }
    }
}

// ---------- re-verification (2026-11-16) ----------

/// Criterion 13 with a negative offset: a hole smaller than 0.3 mm gives no inner area at +0.3
/// but one at -0.3, grown outward under the stroke (radius 0.2 + 0.3 = 0.5 within 0.01 mm).
#[test]
fn c13_negative_offset_grows_a_small_hole_outward() {
    let bar = vec![
        rect(0.0, 0.0, 40.0, 4.0, false),
        circle(20.0, 2.0, 0.2, true),
    ];
    assert!(
        inner(&bar, Some(0.3), &[]).inner_areas.is_empty(),
        "+0.3 mm: none"
    );
    let lines = inner(&bar, Some(-0.3), &[]);
    assert_eq!(lines.inner_areas.len(), 1, "-0.3 mm: one inner area");
    let (lo, hi) = radii(&lines.inner_areas[0], p(20.0, 2.0));
    assert!(
        (lo - 0.5).abs() <= 0.01 && (hi - 0.5).abs() <= 0.01,
        "radii {lo:.4}..{hi:.4}, want 0.5"
    );
}

/// Criterion 14 with a negative offset: the hole's area grows by 0.3 mm (radius 9.3) and the
/// pupil cut out of it grows by the same -0.3 mm, so it shrinks to radius 2.7.
#[test]
fn c14_negative_offset_moves_the_cut_out_the_same_way() {
    let eye = annulus(0.0, 0.0, 10.0, 2.0);
    let pupil = vec![circle(0.0, 0.0, 3.0, false)];
    let lines = inner(&eye, Some(-0.3), &[pupil]);
    let mut found = Vec::new();
    for poly in &lines.inner_areas {
        let (lo, hi) = radii(poly, p(0.0, 0.0));
        found.push((signed_area_mm2(poly) > 0.0, lo, hi));
    }
    let outer: Vec<_> = found.iter().filter(|f| f.0).collect();
    let holes: Vec<_> = found.iter().filter(|f| !f.0).collect();
    assert_eq!((outer.len(), holes.len()), (1, 1), "{found:?}");
    assert!(
        (outer[0].1 - 9.3).abs() <= 0.01 && (outer[0].2 - 9.3).abs() <= 0.01,
        "{found:?}"
    );
    assert!(
        (holes[0].1 - 2.7).abs() <= 0.01 && (holes[0].2 - 2.7).abs() <= 0.01,
        "{found:?}"
    );
}

const WHALE: &str = include_str!("fixtures/centre_lines/whale_traced.txt");

/// The fixture's outlines (one compound subject per `subject` line).
fn whale_subjects() -> Vec<Rings> {
    let mut subjects: Vec<Rings> = Vec::new();
    for line in WHALE.lines().filter(|l| !l.starts_with('#')) {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first() {
            Some(&"subject") => subjects.push(Vec::new()),
            Some(&"outline") => {
                assert_eq!(words[1], "true");
                subjects.last_mut().unwrap().push(Vec::new());
            }
            Some(_) => {
                let v: Vec<f64> = words.iter().map(|w| w.parse().unwrap()).collect();
                subjects.last_mut().unwrap().last_mut().unwrap().push((
                    p(v[0], v[1]),
                    Vec2::new(v[2], v[3]),
                    Vec2::new(v[4], v[5]),
                ));
            }
            None => {}
        }
    }
    subjects
}

fn whale_result(subject: &Rings, options: &CentreOptions) -> String {
    format!("{:?}", run_with(subject, options, &[]))
}

/// ADR 0063 §9: the traced whale with its outlines in another order and every ring started at
/// another anchor (handles moving with their anchor) gives a bit-identical result, with and
/// without inner areas.
#[test]
fn determinism_whale_permuted_subpaths_and_starts_are_bit_identical() {
    let inner_on = CentreOptions {
        inner_offset: Some(Length::from_mm(0.3)),
        ..CentreOptions::default()
    };
    for subject in whale_subjects() {
        let n = subject.len();
        for (k, options) in [CentreOptions::default(), inner_on].iter().enumerate() {
            let base = whale_result(&subject, options);
            for shift in [1, 3, n.saturating_sub(1)] {
                let mut permuted: Rings = subject
                    .iter()
                    .enumerate()
                    .map(|(i, ring)| {
                        let mut ring = ring.clone();
                        let len = ring.len();
                        ring.rotate_left((i * 7 + shift) % len);
                        ring
                    })
                    .collect();
                permuted.rotate_left(shift % n);
                permuted.reverse();
                assert_eq!(
                    base,
                    whale_result(&permuted, options),
                    "options {k}, shift {shift}: permuted outlines differ"
                );
            }
        }
    }
}

/// Beyond ADR 0063 §9 (canonical input): reversing every ring's direction gives a bit-identical
/// result too.
#[test]
fn determinism_whale_reversed_rings_are_bit_identical() {
    for subject in whale_subjects() {
        let base = whale_result(&subject, &CentreOptions::default());
        let reversed: Rings = subject
            .iter()
            .map(|ring| {
                ring.iter()
                    .rev()
                    .map(|&(q, hin, hout)| (q, hout, hin))
                    .collect()
            })
            .collect();
        let other = whale_result(&reversed, &CentreOptions::default());
        assert_eq!(base, other, "reversed rings differ");
    }
}

/// Options that are not numbers never panic the kernel (the binding maps them to defaults).
#[test]
fn nonfinite_options_do_not_panic() {
    let subject = whale_subjects().remove(0);
    for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let options = CentreOptions {
            overlap: x,
            thick_parts: Some(Length::from_mm(x)),
            inner_offset: Some(Length::from_mm(x)),
            optimise: curvyo_geometry_core::DEFAULT_OPTIMISE,
        };
        let r = std::panic::catch_unwind(|| run_with(&subject, &options, &[]));
        assert!(r.is_ok(), "options {x} panicked");
    }
}

// ---------- criteria 15a and 8 (amended 2026-11-16 and 2026-11-17): Optimise, corners, tip ----------

fn run_opt(rings: &Rings, optimise: f64) -> CentreLines {
    run_with(
        rings,
        &CentreOptions {
            optimise,
            ..CentreOptions::default()
        },
        &[],
    )
}

/// Nodes plus width points of a result (what the file stores per path).
fn budget(lines: &CentreLines) -> usize {
    lines
        .paths
        .iter()
        .map(|q| q.anchors.len() + q.profile.positions().count())
        .sum()
}

#[test]
fn opt_default_is_75_percent() {
    assert!((CentreOptions::default().optimise - 0.75).abs() < 1e-12);
}

/// Criterion 15a: a higher Optimise never gives more nodes plus width points.
#[test]
fn opt_nodes_and_width_points_never_grow_as_optimise_rises() {
    let taper = stroke(
        &straight(0.0),
        3.0,
        LineCap::Round,
        &profile(&linear_taper(3.0, 0.5, 60), false),
    );
    let subjects: Vec<(&str, Rings)> = vec![
        ("straight", uniform(&straight(0.0), 2.0)),
        ("arc", uniform(&arc(0.0), 2.0)),
        ("s", uniform(&s_curve(0.0), 2.0)),
        ("taper", taper),
        ("ring", annulus(3.0, -4.0, 10.0, 2.0)),
        ("t", t_shape(2.0, 0.0).0),
    ];
    let mut named: Vec<(String, Rings)> = subjects
        .into_iter()
        .map(|(n, r)| (n.to_owned(), r))
        .collect();
    for (i, whale) in whale_subjects().into_iter().enumerate() {
        named.push((format!("whale {i}"), whale));
    }
    for (name, subject) in &named {
        let mut prev = usize::MAX;
        let mut counts = Vec::new();
        for step in 0..=10 {
            let o = f64::from(step) / 10.0;
            let n = budget(&run_opt(subject, o));
            counts.push((o, n));
            assert!(
                n <= prev,
                "{name}: Optimise {o} gives {n} nodes+width points, more than {prev} before: {counts:?}"
            );
            prev = n;
        }
    }
}

/// Criterion 15a and the ADR 0063 amendment: the accuracy bound rises with Optimise. Half of
/// `T(w)` up to 50 %, `T(w)` at the default and 3.5 `T(w)` at 100 % (the spec's smooth-stretch
/// bound), on the axis and on the width at 20 points.
#[test]
fn opt_accuracy_is_bounded_at_0_50_75_and_100_percent() {
    let cases = [(0.0, 0.5), (0.5, 0.5), (0.75, 1.0), (1.0, 3.5)];
    for w in [2.0, 5.0] {
        for (name, curve) in [
            ("straight", straight(0.0)),
            ("arc", arc(0.0)),
            ("s", s_curve(0.0)),
        ] {
            let subject = uniform(&curve, w);
            for (o, k) in cases {
                let lines = run_opt(&subject, o);
                let path = single(&lines, &format!("{name} w {w} o {o}"));
                let t = tol(w);
                let (out, back) = axis_errors(&path, &curve.dense);
                assert!(
                    out <= k * t && back <= k * t,
                    "{name} w {w} Optimise {o}: axis {out:.4}/{back:.4} > {k} T {:.4}",
                    k * t
                );
                for (idx, got) in widths_at_20(&path, &curve.dense) {
                    assert!(
                        (got - w).abs() <= k * t,
                        "{name} w {w} Optimise {o}: width {got:.4} at #{idx} > {k} T"
                    );
                }
            }
        }
    }
}

/// Criterion 15a: corners keep their detail at every setting. An anchor sits at each corner, and
/// within two local widths of a corner the curve keeps half of `T(w)`.
#[test]
fn opt_corners_keep_anchors_and_tight_fit_at_default_and_100_percent() {
    let bent = |turn: f64| {
        let t = turn.to_radians();
        polyline(
            &[
                (0.0, 0.0),
                (30.0, 0.0),
                (30.0 + 30.0 * t.cos(), 30.0 * t.sin()),
            ],
            0.0,
        )
    };
    let cases: Vec<(&str, Curve, (f64, f64))> = vec![
        (
            "L",
            polyline(&[(0.0, 0.0), (30.0, 0.0), (30.0, 30.0)], 0.0),
            (30.0, 0.0),
        ),
        ("bent 60", bent(60.0), (30.0, 0.0)),
        ("bent 120", bent(120.0), (30.0, 0.0)),
        (
            "L at 60",
            polyline(&[(0.0, 0.0), (30.0, 0.0), (30.0, 30.0)], 60.0),
            rot((30.0, 0.0), 60.0),
        ),
    ];
    for w in [2.0, 5.0] {
        for (name, curve, corner_at) in &cases {
            let subject = uniform(curve, w);
            let c = p(corner_at.0, corner_at.1);
            for o in [DEFAULT_OPTIMISE, 1.0] {
                let lines = run_opt(&subject, o);
                let path = single(&lines, &format!("{name} w {w} o {o}"));
                let near = path
                    .anchors
                    .iter()
                    .map(|a| dist(a.0, c))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    near <= tol(w),
                    "{name} w {w} o {o}: no anchor at the corner, nearest {near:.4}"
                );
                let t = tol(w);
                let (samples_near, truth_near) = (
                    samples(&path, 800)
                        .into_iter()
                        .map(|s| s.0)
                        .filter(|q| dist(*q, c) <= 2.0 * w)
                        .map(|q| to_dense(q, &curve.dense))
                        .fold(0.0, f64::max),
                    curve
                        .dense
                        .iter()
                        .filter(|q| dist(**q, c) <= 2.0 * w)
                        .map(|&q| {
                            let pts: Vec<Point> =
                                samples(&path, 800).into_iter().map(|s| s.0).collect();
                            to_dense(q, &pts)
                        })
                        .fold(0.0, f64::max),
                );
                assert!(
                    samples_near <= 0.5 * t && truth_near <= 0.5 * t,
                    "{name} w {w} o {o}: near the corner {samples_near:.4}/{truth_near:.4} > T/2 {:.4}",
                    0.5 * t
                );
            }
        }
    }
}

/// Criterion 8 (CENTRE-TIP A): the free round end's half-width stays within 0.08 r of the true
/// semicircle, without any allowance for `T(w)`.
#[test]
fn c8_round_end_half_width_within_0_08_r_alone() {
    for w in [2.0, 5.0, 8.0] {
        let (subject, _, _) = t_shape(w, 0.0);
        let lines = run(&subject);
        let branch = branches(&lines)[0];
        let m = measure(branch);
        let r = w / 2.0;
        for s in [0.0, 0.25 * r, 0.5 * r, 0.8 * r, 0.95 * r] {
            let q = p(0.0, 20.0 + s);
            let at = m.nearest(q).as_mm();
            let half = width_at(branch, &m, at) / 2.0;
            let truth = (r * r - s * s).max(0.0).sqrt();
            assert!(
                (half - truth).abs() <= 0.08 * r,
                "T w {w}: half-width at s {s:.4} is {half:.4}, semicircle {truth:.4}, 0.08 r = {:.4}",
                0.08 * r
            );
        }
    }
}

// ---------- Round A and B (customer 2026-11-18): Thick parts in mm, stages, the dolphin ----------

fn thick(mm: Option<f64>) -> CentreOptions {
    CentreOptions {
        thick_parts: mm.map(Length::from_mm),
        ..CentreOptions::default()
    }
}

/// Criteria 5 to 7 at the true defaults (Thick parts 6 mm, Optimise 75 %): the strokes of at most
/// 5 mm give the very result the limit-free runs above are checked on, so every assertion there
/// holds at the defaults too (criterion 15a: "at the default the result meets criteria 5 to 7").
#[test]
fn c5_to_c7_hold_at_the_full_defaults() {
    assert_eq!(DEFAULT_THICK_PARTS.as_mm(), 6.0);
    assert_eq!(
        CentreOptions::default().thick_parts.map(Length::as_mm),
        Some(6.0)
    );
    let taper = stroke(
        &straight(30.0),
        3.0,
        LineCap::Round,
        &profile(&linear_taper(3.0, 0.5, 60), false),
    );
    let cases = [
        ("straight 2", uniform(&straight(0.0), 2.0)),
        ("arc 2", uniform(&arc(0.0), 2.0)),
        ("s 2", uniform(&s_curve(17.0), 2.0)),
        ("straight 5", uniform(&straight(45.0), 5.0)),
        ("taper 3 to 0.5", taper),
        ("ring 2", annulus(3.0, -4.0, 10.0, 2.0)),
        ("ring 5", annulus(3.0, -4.0, 15.0, 5.0)),
    ];
    for (name, subject) in cases {
        let at_defaults = run_with(&subject, &CentreOptions::default(), &[]);
        assert!(!at_defaults.paths.is_empty(), "{name}: converted");
        assert_eq!(
            format!("{at_defaults:?}"),
            format!("{:?}", run(&subject)),
            "{name}: the 6 mm default changes the result"
        );
    }
}

/// Criterion 12 (reworded 2026-11-18): Thick parts is a width in mm; a stroke narrower than it
/// converts, a wider one stays filled; `None` (the slider's maximum) converts every stroke.
#[test]
fn c12_thick_parts_in_mm_decides_by_the_local_width() {
    for (w, limit, converts) in [
        (4.0, Some(6.0), true),
        (4.0, Some(3.0), false),
        (5.8, Some(6.0), true),
        (6.5, Some(6.0), false),
        (8.0, Some(6.0), false),
        (8.0, Some(10.0), true),
        (8.0, None, true),
        (1.0, Some(0.5), false),
    ] {
        let subject = uniform(&s_curve(17.0), w);
        let lines = run_with(&subject, &thick(limit), &[]);
        if converts {
            assert_eq!(lines.paths.len(), 1, "w {w} at {limit:?}: one centre line");
            assert!(lines.filled.is_empty(), "w {w} at {limit:?}: nothing kept");
        } else {
            assert!(lines.paths.is_empty(), "w {w} at {limit:?}: no centre line");
            assert!(lines.all_filled(), "w {w} at {limit:?}: stays filled");
        }
    }
}

/// Points of a set of rings, sorted, for an order- and start-free comparison.
fn point_set(rings: &[Vec<Triple>]) -> Vec<(i64, i64)> {
    let mut v: Vec<(i64, i64)> = rings
        .iter()
        .flatten()
        .map(|a| ((a.0.x * 1e6).round() as i64, (a.0.y * 1e6).round() as i64))
        .collect();
    v.sort_unstable();
    v
}

/// Criterion 12, customer 2026-11-18 point 1: a shape kept filled keeps its nodes as they are,
/// handles included (here a wide S stroke and an annulus with its hole).
#[test]
fn c12_fully_kept_shape_keeps_its_own_nodes() {
    for (name, subject) in [
        ("wide s", uniform(&s_curve(17.0), 8.0)),
        ("wide ring", annulus(0.0, 0.0, 15.0, 9.0)),
    ] {
        let lines = run_with(&subject, &CentreOptions::default(), &[]);
        assert!(lines.all_filled(), "{name}: stays filled");
        assert_eq!(lines.filled.len(), 1, "{name}: one kept part");
        let kept = &lines.filled[0];
        assert_eq!(kept.len(), subject.len(), "{name}: same subpaths");
        assert_eq!(point_set(kept), point_set(&subject), "{name}: same nodes");
        let handles = |rings: &[Vec<Triple>]| {
            let mut v: Vec<(i64, i64, i64, i64, i64, i64)> = rings
                .iter()
                .flatten()
                .map(|a| {
                    let r = |x: f64| (x * 1e6).round() as i64;
                    (r(a.0.x), r(a.0.y), r(a.1.x), r(a.1.y), r(a.2.x), r(a.2.y))
                })
                .collect();
            v.sort_unstable();
            v
        };
        let (a, b) = (handles(kept), handles(&subject));
        // Handles may be reversed with the ring's direction: in and out swap.
        let swapped: Vec<_> = {
            let mut v: Vec<_> = b
                .iter()
                .map(|&(x, y, ix, iy, ox, oy)| (x, y, ox, oy, ix, iy))
                .collect();
            v.sort_unstable();
            v
        };
        assert!(a == b || a == swapped, "{name}: handles changed");
    }
}

/// A 2 mm line of 30 mm ending in a 10 mm disc: one region with a part wider than the default.
fn line_with_blob() -> Rings {
    let line = uniform(&polyline(&[(0.0, 0.0), (30.0, 0.0)], 0.0), 2.0);
    let blob = vec![circle(34.0, 0.0, 5.0, false)];
    union(&[line, blob])
}

/// Criterion 12 as built (ADR 0063 amendment 2026-11-18, K): only the part wider than Thick parts
/// stays filled, with the subject's own nodes on the blob; the thin line becomes a centre line.
#[test]
fn c12_thick_part_is_cut_off_the_thin_line_as_built() {
    let subject = line_with_blob();
    let lines = run_with(&subject, &CentreOptions::default(), &[]);
    eprintln!(
        "line + blob at 6 mm: {} paths, {} kept parts",
        lines.paths.len(),
        lines.filled.len()
    );
    assert_eq!(lines.filled.len(), 1, "the blob stays filled");
    assert!(!lines.paths.is_empty(), "the thin line gets a centre line");
    let blob_nodes: Vec<Point> = subject
        .iter()
        .flatten()
        .map(|a| a.0)
        .filter(|q| q.x > 31.5)
        .collect();
    let kept: Vec<Point> = lines.filled[0].iter().flatten().map(|a| a.0).collect();
    for q in blob_nodes.iter().filter(|q| q.x > 33.0) {
        assert!(
            kept.iter().any(|k| dist(*k, *q) < 1e-6),
            "blob node {q:?} kept as it is"
        );
    }
    let path = lines
        .paths
        .iter()
        .max_by(|a, b| {
            measure(a)
                .length()
                .as_mm()
                .total_cmp(&measure(b).length().as_mm())
        })
        .unwrap();
    for (q, wq) in samples(path, 200) {
        if q.x < 25.0 {
            assert!(q.y.abs() <= tol(2.0), "axis on the line: {q:?}");
            assert!((wq - 2.0).abs() <= tol(2.0), "width on the line: {wq}");
        }
    }
    // At no limit the whole region converts.
    let free = run_with(&subject, &thick(None), &[]);
    assert!(free.filled.is_empty() && !free.paths.is_empty());
}

/// Criterion 12 as worded in the spec's Terms, criterion 12 and UX notes Frage 1 (default A): a
/// shape with a part wider than Thick parts is left as it is, as a whole.
#[test]
#[ignore = "spec 0117 c12 (whole shape stays filled) conflicts with ADR 0063 K (only the part): lead decides"]
fn c12_spec_text_whole_shape_with_a_thick_part_stays_filled() {
    let lines = run_with(&line_with_blob(), &CentreOptions::default(), &[]);
    assert!(
        lines.all_filled() && lines.paths.is_empty(),
        "{} paths",
        lines.paths.len()
    );
}

const DOLPHIN: &str = include_str!("fixtures/centre_lines/dolphin_traced.txt");

/// The subjects of a traced fixture (one compound subject per `subject` line).
fn subjects_of(text: &str) -> Vec<Rings> {
    let mut subjects: Vec<Rings> = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first() {
            Some(&"subject") => subjects.push(Vec::new()),
            Some(&"outline") => subjects.last_mut().unwrap().push(Vec::new()),
            Some(_) => {
                let v: Vec<f64> = words.iter().map(|w| w.parse().unwrap()).collect();
                subjects.last_mut().unwrap().last_mut().unwrap().push((
                    p(v[0], v[1]),
                    Vec2::new(v[2], v[3]),
                    Vec2::new(v[4], v[5]),
                ));
            }
            None => {}
        }
    }
    subjects
}

/// Criteria 15b and 18 (ADR 0063 amendment 2026-11-18, W; Round B): the staged memo reruns only
/// the stage a value feeds (and those after it) and its result is bit-identical to a fresh
/// `finish` after every step, back-and-forth moves included.
#[test]
fn c15b_staged_recompute_equals_a_fresh_finish_and_reruns_only_its_stage() {
    let base = CentreOptions::default();
    let keep = CentreOptions {
        inner_offset: Some(Length::from_mm(0.3)),
        ..base
    };
    // (options, which stage must rerun: "overlap" | "offset" | "optimise" | "thick" | "none" | "any")
    let seq: Vec<(CentreOptions, &str)> = vec![
        (base, "any"),
        (base, "none"),
        (
            CentreOptions {
                overlap: -0.2,
                ..base
            },
            "overlap",
        ),
        (base, "overlap"),
        (
            CentreOptions {
                optimise: 0.3,
                ..base
            },
            "optimise",
        ),
        (
            CentreOptions {
                optimise: 1.0,
                ..base
            },
            "optimise",
        ),
        (base, "optimise"),
        (thick(Some(3.0)), "thick"),
        (thick(None), "thick"),
        (base, "thick"),
        (keep, "any"),
        (
            CentreOptions {
                inner_offset: Some(Length::from_mm(-0.5)),
                ..base
            },
            "offset",
        ),
        (keep, "offset"),
        (base, "any"),
        (
            CentreOptions {
                overlap: 0.0,
                optimise: 0.5,
                ..keep
            },
            "any",
        ),
        (base, "any"),
    ];
    let mut fixtures: Vec<(String, Rings)> = subjects_of(WHALE)
        .into_iter()
        .enumerate()
        .map(|(i, s)| (format!("whale {i}"), s))
        .collect();
    fixtures.push(("dolphin".into(), subjects_of(DOLPHIN).remove(0)));
    fixtures.push(("T 8 mm".into(), t_shape(8.0, 0.0).0));
    fixtures.push(("ring".into(), annulus(0.0, 0.0, 10.0, 2.0)));
    for (name, subject) in &fixtures {
        let skeleton = CentreSkeleton::build(&outlines(subject)).unwrap();
        let mut stages = CentreStages::new(CentreSkeleton::build(&outlines(subject)).unwrap());
        for (k, (options, stage)) in seq.iter().enumerate() {
            let before = stages.runs();
            let staged = format!("{:?}", stages.finish(options, &[]));
            let fresh = format!("{:?}", skeleton.finish(options, &[]));
            assert_eq!(
                staged, fresh,
                "{name} step {k}: staged differs from a fresh finish"
            );
            let after = stages.runs();
            let d = |a: usize, b: usize| a != b;
            let (cut, through, branches, inner) = (
                d(before.cut, after.cut),
                d(before.through, after.through),
                d(before.branches, after.branches),
                d(before.inner, after.inner),
            );
            match *stage {
                "none" => assert!(
                    !cut && !through && !branches && !inner,
                    "{name} step {k}: same values rerun a stage"
                ),
                "overlap" => assert!(
                    !cut && !through && !inner,
                    "{name} step {k}: Overlap reran cut {cut} through {through} inner {inner}"
                ),
                "offset" => assert!(
                    !cut && !through && !branches,
                    "{name} step {k}: Offset reran cut {cut} through {through} branches {branches}"
                ),
                "optimise" => assert!(!cut, "{name} step {k}: Optimise reran the cut"),
                _ => {}
            }
        }
    }
}

/// Criterion 15a (customer 2026-11-18, point 2): on the dolphin, a higher Optimise in 5 % steps
/// never gives more nodes plus width points, and the default gives fewer than 50 % (where the
/// density merge starts).
#[test]
fn c15a_dolphin_optimise_never_adds_nodes_in_5_percent_steps() {
    let subject = subjects_of(DOLPHIN).remove(0);
    let mut stages = CentreStages::new(CentreSkeleton::build(&outlines(&subject)).unwrap());
    let mut last = usize::MAX;
    let mut at = Vec::new();
    for step in 0..=20 {
        let optimise = f64::from(step) * 0.05;
        let lines = stages.finish(
            &CentreOptions {
                optimise,
                ..CentreOptions::default()
            },
            &[],
        );
        let n = budget(&lines);
        assert!(
            n <= last,
            "dolphin: {n} at {optimise:.2} > {last} one step lower"
        );
        last = n;
        at.push((optimise, n));
    }
    eprintln!("dolphin nodes + width points by Optimise: {at:?}");
    // The lead's finding of the last round: 0 to 50 % did nothing; now every 5 % step counts.
    for w in at[..=10].windows(2) {
        assert!(
            w[1].1 < w[0].1,
            "Optimise {:.2} to {:.2} is dead: {}",
            w[0].0,
            w[1].0,
            w[1].1
        );
    }
    assert!(at[15].1 < at[10].1, "density merge: 75 % below 50 %");
}

/// One dolphin band: `(index, length mm, median width mm, nodes, width points, pointed ends)`.
type Band = (usize, f64, f64, usize, usize, usize);

/// The dolphin's bands at the defaults (ADR 0063 Round A: a path at least 10 mm long whose median
/// width is at least 1.5 mm) and the whole result.
fn dolphin_bands() -> (Vec<Band>, CentreLines, Rings) {
    let subject = subjects_of(DOLPHIN).remove(0);
    let lines = run_with(&subject, &CentreOptions::default(), &[]);
    let mut bands = Vec::new();
    for (i, path) in lines.paths.iter().enumerate() {
        let len = measure(path).length().as_mm();
        let mut w: Vec<f64> = samples(path, 200).into_iter().map(|s| s.1).collect();
        w.sort_by(f64::total_cmp);
        let median = w[w.len() / 2];
        if len >= 10.0 && median >= 1.5 {
            let base = path.width.as_mm();
            let pointed = if path.closed {
                0
            } else {
                [0.0, 1.0]
                    .iter()
                    .filter(|&&t| base * path.profile.sample(t).unwrap() < 0.05)
                    .count()
            };
            // A crest's tip inside the path (CENTRE-THROUGH A, ADR 0063 amendment 2026-11-20)
            // counts as two pointed ends: a taper into the tip and one out of it.
            let tips = path
                .profile
                .positions()
                .filter(|&t| t > 0.0 && t < 1.0 && base * path.profile.sample(t).unwrap() < 0.05)
                .count();
            let pointed = pointed + 2 * tips;
            bands.push((
                i,
                len,
                median,
                path.anchors.len(),
                path.profile.positions().count(),
                pointed,
            ));
        }
    }
    (bands, lines, subject)
}

/// Criterion 20 with ADR 0063's budget (amendment 2026-11-18, B): each band path has at most
/// `2 + ceil(0.12 L)` nodes and `6 + 4 p + ceil(0.12 L)` width points, L in mm, p its pointed
/// ends.
#[test]
fn c20_each_dolphin_band_within_its_own_budget() {
    let (bands, _, _) = dolphin_bands();
    let mut over = Vec::new();
    for &(i, len, median, nodes, points, pointed) in &bands {
        let extra = (0.12 * len).ceil() as usize;
        let (nb, wb, proposal) = (2 + extra, 6 + 4 * pointed + extra, 6 + 5 * pointed + extra);
        // Criterion 8 fixes four taper support points per pointed end.
        let (c8, c8_no_base) = (6 + 4 * pointed + extra, 4 * pointed + extra);
        eprintln!(
            "band {i}: L {len:.0} mm, w {median:.2}, nodes {nodes}/{nb}, width points {points}/{wb} \
             (implementer's proposal {proposal}, 6 + 4 per pointed end {c8}, without the 6 \
             {c8_no_base}), pointed ends {pointed}"
        );
        if nodes > nb || points > wb {
            over.push(i);
        }
    }
    eprintln!(
        "{} of {} bands over their own budget: {over:?}",
        over.len(),
        bands.len()
    );
    assert!(over.is_empty(), "bands over budget: {over:?}");
}

/// Criterion 20, the parts that hold: the bands become a few smooth paths (sum within the ADR
/// budget, no piece shorter than its width counted as a band) and the painted result with the
/// kept parts differs from the subject by at most 4 % of its area.
#[test]
fn c20_dolphin_bands_sum_and_painted_area() {
    let (bands, lines, subject) = dolphin_bands();
    assert!(
        bands.len() >= 10,
        "the dolphin has its bands: {}",
        bands.len()
    );
    let (mut nodes, mut points, mut nb, mut wb) = (0, 0, 0, 0);
    for &(_, len, _, n, w, _) in &bands {
        let extra = (0.12 * len).ceil() as usize;
        nodes += n;
        points += w;
        nb += 2 + extra;
        wb += 6 + extra;
    }
    eprintln!(
        "dolphin: {} paths, {} bands, nodes {nodes}/{nb}, width points {points}/{wb}, {} kept",
        lines.paths.len(),
        bands.len(),
        lines.filled.len()
    );
    assert!(nodes <= nb && points <= wb, "sum over budget");
    let mut painted: Vec<Rings> = lines.paths.iter().map(paint).collect();
    for kept in &lines.filled {
        painted.push(kept.clone());
    }
    let painted = union(&painted);
    let x = xor_area(&painted, &subject);
    let a = area(&subject);
    eprintln!(
        "dolphin painted: {x:.1} of {a:.1} mm² differ ({:.2} %)",
        100.0 * x / a
    );
    assert!(x <= 0.04 * a, "painted differs by {:.2} %", 100.0 * x / a);
}
