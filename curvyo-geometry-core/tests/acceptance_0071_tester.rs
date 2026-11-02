//! Independent tester tests for `specs/0071-pattern-fill` at the geometry kernel (ADR 0029 §3 to
//! §5, §10): every generator on a region with holes, a reflex corner and a margin, in both modes;
//! the generator semantics of criteria 5 to 12; Lines vs Shapes; determinism and seeds; the count
//! limits. Written from the specification and the ADR before the implementation was read; only
//! the public names (`pattern_pieces_of`, the model types and the parameter table) were looked up.
//! The checks sample the pieces' Bézier curves and measure them against my own flattening of the
//! region, so nothing here reuses the kernel's clipping.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::doc_markdown
)]
#![allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
#![allow(clippy::similar_names, clippy::needless_pass_by_value)]

use curvyo_document_core::{
    Angle, GeneratorKind, Length, PatternFill, PatternGenerator, PatternMode, PatternParam,
    PatternSeed, Point, Tolerance, Vec2, pattern_params,
};
use curvyo_geometry_core::{
    PATTERN_MAX_PIECES, PATTERN_NOTE_PIECES, PatternError, PatternOutline, PatternPiece,
    pattern_pieces_of,
};
use proptest::prelude::*;

const TOL: Tolerance = Tolerance::from_mm(0.01);
/// Kernel tolerance plus the 0.001 mm grid the cut pieces are snapped to, plus my flattening.
const SLACK: f64 = 0.013;
const W: f64 = 0.25;

type Triple = (Point, Vec2, Vec2);

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn poly(points: &[(f64, f64)]) -> Vec<Triple> {
    points
        .iter()
        .map(|&(x, y)| (p(x, y), Vec2::ZERO, Vec2::ZERO))
        .collect()
}

/// A circle of four cubic arcs; `ccw` in the y-down document (screen) sense flips the order.
fn circle(cx: f64, cy: f64, r: f64, reverse: bool) -> Vec<Triple> {
    ellipse(cx, cy, r, r, reverse)
}

fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64, reverse: bool) -> Vec<Triple> {
    let k = 0.552_284_749_830_793_6;
    let mut v = vec![
        (
            p(cx + rx, cy),
            Vec2::new(0.0, -k * ry),
            Vec2::new(0.0, k * ry),
        ),
        (
            p(cx, cy + ry),
            Vec2::new(k * rx, 0.0),
            Vec2::new(-k * rx, 0.0),
        ),
        (
            p(cx - rx, cy),
            Vec2::new(0.0, k * ry),
            Vec2::new(0.0, -k * ry),
        ),
        (
            p(cx, cy - ry),
            Vec2::new(-k * rx, 0.0),
            Vec2::new(k * rx, 0.0),
        ),
    ];
    if reverse {
        v.reverse();
        for t in &mut v {
            *t = (t.0, t.2, t.1);
        }
    }
    v
}

/// The test region: an L (reflex corner at (60, 30)) with a round and a square hole, both wound
/// against the outline so the nonzero rule makes them holes (the app's compound paths).
fn region() -> Vec<PatternOutline> {
    let outer = poly(&[
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 70.0),
        (60.0, 70.0),
        (60.0, 30.0),
        (0.0, 30.0),
    ]);
    let square_hole = poly(&[(75.0, 40.0), (75.0, 55.0), (90.0, 55.0), (90.0, 40.0)]);
    let a = signed_area(&flatten(&outer, true, 8));
    let b = signed_area(&flatten(&square_hole, true, 8));
    assert!(
        a * b < 0.0,
        "test set-up: the hole is wound against the outline"
    );
    let round = circle(25.0, 15.0, 8.0, false);
    let round = if signed_area(&flatten(&round, true, 8)) * a < 0.0 {
        round
    } else {
        circle(25.0, 15.0, 8.0, true)
    };
    vec![(outer, true), (round, true), (square_hole, true)]
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<PatternOutline> {
    vec![(poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]), true)]
}

fn cubic(a: Point, b: Point, c: Point, d: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    p(
        w0 * a.x + w1 * b.x + w2 * c.x + w3 * d.x,
        w0 * a.y + w1 * b.y + w2 * c.y + w3 * d.y,
    )
}

/// Samples of an outline: `n` per segment, the anchors included.
fn flatten(anchors: &[Triple], closed: bool, n: usize) -> Vec<Point> {
    let mut out = Vec::new();
    if anchors.is_empty() {
        return out;
    }
    let segs = if closed {
        anchors.len()
    } else {
        anchors.len() - 1
    };
    out.push(anchors[0].0);
    for i in 0..segs {
        let (a, _, ao) = anchors[i];
        let (b, bi, _) = anchors[(i + 1) % anchors.len()];
        let c1 = p(a.x + ao.x, a.y + ao.y);
        let c2 = p(b.x + bi.x, b.y + bi.y);
        let straight = ao == Vec2::ZERO && bi == Vec2::ZERO;
        let steps = if straight { 1 } else { n };
        for s in 1..=steps {
            out.push(cubic(a, c1, c2, b, s as f64 / steps as f64));
        }
    }
    out
}

fn signed_area(pts: &[Point]) -> f64 {
    let mut s = 0.0;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        s += a.x * b.y - b.x * a.y;
    }
    s / 2.0
}

fn region_polys(region: &[PatternOutline]) -> Vec<Vec<Point>> {
    region
        .iter()
        .map(|(a, c)| {
            let mut v = flatten(a, *c, 256);
            if v.first() == v.last() {
                v.pop();
            }
            v
        })
        .collect()
}

fn winding(polys: &[Vec<Point>], q: Point) -> i32 {
    let mut w = 0;
    for poly in polys {
        for i in 0..poly.len() {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            let cross = (b.x - a.x) * (q.y - a.y) - (q.x - a.x) * (b.y - a.y);
            if a.y <= q.y {
                if b.y > q.y && cross > 0.0 {
                    w += 1;
                }
            } else if b.y <= q.y && cross < 0.0 {
                w -= 1;
            }
        }
    }
    w
}

fn seg_dist(q: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 {
        0.0
    } else {
        (((q.x - a.x) * dx + (q.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    };
    let (x, y) = (a.x + t * dx - q.x, a.y + t * dy - q.y);
    (x * x + y * y).sqrt()
}

fn boundary_dist(polys: &[Vec<Point>], q: Point) -> f64 {
    let mut d = f64::INFINITY;
    for poly in polys {
        for i in 0..poly.len() {
            d = d.min(seg_dist(q, poly[i], poly[(i + 1) % poly.len()]));
        }
    }
    d
}

fn fill(
    kind: GeneratorKind,
    params: &[(PatternParam, f64)],
    margin: f64,
    mode: PatternMode,
) -> PatternFill {
    let mut g = PatternGenerator::defaults(kind);
    for &(param, v) in params {
        g = g.with(param, v).unwrap();
    }
    PatternFill {
        generator: g,
        margin: Length::from_mm(margin),
        mode,
        seed: PatternSeed::new(12345),
        whole_pieces: false,
    }
}

fn pieces(region: &[PatternOutline], f: &PatternFill) -> Vec<PatternPiece> {
    pattern_pieces_of(region, Angle::from_radians(0.0), f, Length::from_mm(W), TOL).unwrap()
}

fn piece_points(piece: &PatternPiece) -> Vec<Point> {
    piece
        .outlines
        .iter()
        .flat_map(|(a, c)| flatten(a, *c, 24))
        .collect()
}

/// Every sample of every piece lies inside the region, not in a hole, and not closer than
/// `margin` to any boundary (criteria 13 to 15). Returns the smallest distance seen.
fn assert_inside(region: &[PatternOutline], ps: &[PatternPiece], margin: f64, what: &str) -> f64 {
    let polys = region_polys(region);
    let mut min_d = f64::INFINITY;
    for (i, piece) in ps.iter().enumerate() {
        for q in piece_points(piece) {
            assert!(
                q.x.is_finite() && q.y.is_finite(),
                "{what}: piece {i} not finite"
            );
            let d = boundary_dist(&polys, q);
            min_d = min_d.min(d);
            if d > SLACK {
                assert!(
                    winding(&polys, q) != 0,
                    "{what}: piece {i} has ({:.4}, {:.4}) outside the region or in a hole",
                    q.x,
                    q.y
                );
            }
            assert!(
                d >= margin - SLACK,
                "{what}: piece {i} has ({:.4}, {:.4}) at {d:.4} mm from the boundary, margin {margin}",
                q.x,
                q.y
            );
        }
    }
    min_d
}

fn all_open(ps: &[PatternPiece]) -> bool {
    ps.iter().all(|pc| pc.outlines.iter().all(|(_, c)| !c))
}

fn all_closed(ps: &[PatternPiece]) -> bool {
    ps.iter().all(|pc| pc.outlines.iter().all(|(_, c)| *c))
}

// ---------- criteria 13, 14, 15: every generator, holes, margin, both modes ----------

#[test]
fn c13_c14_c15_every_generator_in_both_modes_stays_in_the_inset_region_with_holes() {
    let r = region();
    for kind in GeneratorKind::ALL {
        for mode in [PatternMode::Lines, PatternMode::Shapes] {
            for margin in [0.0, 1.5, 4.0] {
                let what = format!("{kind:?} {mode:?} margin {margin}");
                let ps = pieces(&r, &fill(kind, &[], margin, mode));
                assert!(!ps.is_empty(), "{what}: the region gets pieces");
                let min_d = assert_inside(&r, &ps, margin, &what);
                // The pieces are cut at the inset boundary, not stopped well short of it.
                if !matches!(
                    kind,
                    GeneratorKind::LeafForest | GeneratorKind::InterpolationWaves
                ) {
                    assert!(
                        min_d <= margin + 0.6,
                        "{what}: nothing comes near the inset boundary (closest {min_d:.3})"
                    );
                }
            }
        }
    }
}

#[test]
fn c13_nothing_lies_inside_either_hole() {
    let r = region();
    for kind in GeneratorKind::ALL {
        let ps = pieces(&r, &fill(kind, &[], 0.0, PatternMode::Shapes));
        for piece in &ps {
            for q in piece_points(piece) {
                let in_round = ((q.x - 25.0).powi(2) + (q.y - 15.0).powi(2)).sqrt() < 8.0 - SLACK;
                let in_square = q.x > 75.0 + SLACK
                    && q.x < 90.0 - SLACK
                    && q.y > 40.0 + SLACK
                    && q.y < 55.0 - SLACK;
                assert!(
                    !in_round && !in_square,
                    "{kind:?}: ({}, {}) in a hole",
                    q.x,
                    q.y
                );
            }
        }
    }
}

#[test]
fn c14_hatch_lines_end_on_the_inset_boundary() {
    let r = region();
    for margin in [0.0, 2.0] {
        let ps = pieces(
            &r,
            &fill(GeneratorKind::Hatch, &[], margin, PatternMode::Lines),
        );
        let polys = region_polys(&r);
        assert!(all_open(&ps));
        for piece in &ps {
            for (a, _) in &piece.outlines {
                for end in [a.first().unwrap().0, a.last().unwrap().0] {
                    let d = boundary_dist(&polys, end);
                    assert!(
                        (d - margin).abs() <= 0.02,
                        "margin {margin}: a hatch line ends at {d:.4} mm from the boundary"
                    );
                }
            }
        }
    }
}

#[test]
fn c23_an_open_outline_gives_no_pieces_and_no_error() {
    let open = vec![(poly(&[(0.0, 0.0), (50.0, 0.0), (50.0, 50.0)]), false)];
    for kind in GeneratorKind::ALL {
        let r = pattern_pieces_of(
            &open,
            Angle::from_radians(0.0),
            &fill(kind, &[], 0.0, PatternMode::Lines),
            Length::from_mm(W),
            TOL,
        );
        assert_eq!(r, Ok(Vec::new()), "{kind:?}");
    }
}

#[test]
fn margin_larger_than_the_region_gives_nothing() {
    let r = rect(0.0, 0.0, 10.0, 10.0);
    for kind in GeneratorKind::ALL {
        let ps = pieces(&r, &fill(kind, &[], 6.0, PatternMode::Lines));
        assert!(
            ps.is_empty(),
            "{kind:?}: a margin past the middle leaves no region"
        );
    }
}

// ---------- criterion 5: hatch, crosshatch, grid ----------

/// Directions (degrees in [0, 180)) and perpendicular offsets of the straight segments of the
/// open pieces, segment by segment (a piece may run along several lines of a crossing set).
fn line_dirs(ps: &[PatternPiece]) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for piece in ps {
        for (a, _) in &piece.outlines {
            for w in a.windows(2) {
                let (s, e) = (w[0].0, w[1].0);
                let len = ((e.x - s.x).powi(2) + (e.y - s.y).powi(2)).sqrt();
                if len < 3.0 {
                    continue;
                }
                let mut deg = (e.y - s.y).atan2(e.x - s.x).to_degrees();
                deg = deg.rem_euclid(180.0);
                if deg > 179.95 {
                    deg -= 180.0;
                }
                // Offsets at the nominal angle (cut ends are snapped to the 0.001 mm grid).
                let th = ((deg * 2.0).round() / 2.0).to_radians(); // test angles are multiples of 0.5
                let (mx, my) = (f64::midpoint(s.x, e.x), f64::midpoint(s.y, e.y));
                let offset = -mx * th.sin() + my * th.cos();
                out.push((deg, offset));
            }
        }
    }
    out
}

fn assert_lattice(offsets: &mut [f64], spacing: f64, what: &str) {
    offsets.sort_by(f64::total_cmp);
    let mut uniq: Vec<f64> = Vec::new();
    for &o in offsets.iter() {
        if uniq.last().is_none_or(|l| (o - l).abs() > 0.01) {
            uniq.push(o);
        }
    }
    assert!(uniq.len() > 5, "{what}: several lines");
    for w in uniq.windows(2) {
        let gap = w[1] - w[0];
        assert!(
            (gap - spacing).abs() < 1e-3,
            "{what}: line gap {gap} != {spacing}"
        );
    }
}

fn dir_matches(deg: f64, want: f64) -> bool {
    let d = (deg - want.rem_euclid(180.0)).rem_euclid(180.0);
    d < 0.05 || d > 180.0 - 0.05
}

#[test]
fn c5_hatch_is_parallel_lines_at_the_spacing_and_angle() {
    let r = rect(0.0, 0.0, 100.0, 80.0);
    for (spacing, angle) in [(2.0, 0.0), (3.5, 30.0), (1.25, 90.0), (5.0, -60.0)] {
        let f = fill(
            GeneratorKind::Hatch,
            &[
                (PatternParam::Spacing, spacing),
                (PatternParam::Angle, angle),
            ],
            0.0,
            PatternMode::Lines,
        );
        let ps = pieces(&r, &f);
        assert!(all_open(&ps));
        let dirs = line_dirs(&ps);
        let first = dirs[0].0;
        // The spec does not fix the sign convention in a y-down document; accept ±angle but it
        // must be the same for every line.
        assert!(
            dir_matches(first, angle) || dir_matches(first, -angle),
            "hatch {angle}: direction {first}"
        );
        assert!(dirs.iter().all(|d| dir_matches(d.0, first)), "parallel");
        let mut offs: Vec<f64> = dirs.iter().map(|d| d.1).collect();
        assert_lattice(&mut offs, spacing, &format!("hatch {spacing}/{angle}"));
    }
}

#[test]
fn c5_crosshatch_has_both_angles_each_at_the_spacing() {
    let r = rect(0.0, 0.0, 100.0, 80.0);
    for (a1, a2) in [(45.0, 135.0), (20.0, 80.0), (0.0, 60.0)] {
        let f = fill(
            GeneratorKind::Crosshatch,
            &[
                (PatternParam::Spacing, 3.0),
                (PatternParam::Angle, a1),
                (PatternParam::SecondAngle, a2),
            ],
            0.0,
            PatternMode::Lines,
        );
        let dirs = line_dirs(&pieces(&r, &f));
        let mut set: Vec<f64> = Vec::new();
        for d in &dirs {
            if !set.iter().any(|s| dir_matches(d.0, *s)) {
                set.push(d.0);
            }
        }
        assert_eq!(
            set.len(),
            2,
            "crosshatch {a1}/{a2}: exactly two directions, got {set:?}"
        );
        let diff = (set[0] - set[1]).abs();
        let want = (a2 - a1).abs().rem_euclid(180.0);
        assert!(
            (diff - want).abs() < 0.1 || (diff - (180.0 - want)).abs() < 0.1,
            "crosshatch {a1}/{a2}: angle between the sets {diff}"
        );
        for s in set {
            let mut offs: Vec<f64> = dirs
                .iter()
                .filter(|d| dir_matches(d.0, s))
                .map(|d| d.1)
                .collect();
            assert_lattice(&mut offs, 3.0, "crosshatch set");
        }
    }
}

#[test]
fn c5_grid_is_two_perpendicular_line_sets() {
    let r = rect(0.0, 0.0, 100.0, 80.0);
    for angle in [0.0, 15.0] {
        let f = fill(
            GeneratorKind::Grid,
            &[(PatternParam::Spacing, 4.0), (PatternParam::Angle, angle)],
            0.0,
            PatternMode::Lines,
        );
        let dirs = line_dirs(&pieces(&r, &f));
        let mut set: Vec<f64> = Vec::new();
        for d in &dirs {
            if !set.iter().any(|s| dir_matches(d.0, *s)) {
                set.push(d.0);
            }
        }
        assert_eq!(set.len(), 2, "grid {angle}: {set:?}");
        assert!(
            ((set[0] - set[1]).abs() - 90.0).abs() < 1e-6,
            "perpendicular"
        );
        for s in set {
            let mut offs: Vec<f64> = dirs
                .iter()
                .filter(|d| dir_matches(d.0, s))
                .map(|d| d.1)
                .collect();
            assert_lattice(&mut offs, 4.0, "grid set");
        }
    }
}

// ---------- criterion 6: dots ----------

fn bbox(points: &[Point]) -> (f64, f64, f64, f64) {
    let mut b = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for q in points {
        b = (b.0.min(q.x), b.1.min(q.y), b.2.max(q.x), b.3.max(q.y));
    }
    b
}

/// Groups `(key, value)` into rows by `key` within `eps`.
fn rows(mut items: Vec<(f64, f64)>, eps: f64) -> Vec<(f64, Vec<f64>)> {
    items.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, Vec<f64>)> = Vec::new();
    for (k, v) in items {
        match out.last_mut() {
            Some(row) if (k - row.0).abs() < eps => row.1.push(v),
            _ => out.push((k, vec![v])),
        }
    }
    for row in &mut out {
        row.1.sort_by(f64::total_cmp);
    }
    out
}

#[test]
fn c6_dots_are_round_of_the_diameter_on_a_staggered_lattice() {
    let r = rect(0.0, 0.0, 100.0, 80.0);
    let (spacing, d) = (4.0, 1.5);
    for mode in [PatternMode::Lines, PatternMode::Shapes] {
        let f = fill(
            GeneratorKind::Dots,
            &[
                (PatternParam::Spacing, spacing),
                (PatternParam::Diameter, d),
            ],
            0.0,
            mode,
        );
        let ps = pieces(&r, &f);
        assert!(all_closed(&ps), "dots are closed in {mode:?}");
        let mut centres = Vec::new();
        for piece in &ps {
            let pts = piece_points(piece);
            let b = bbox(&pts);
            if b.0 < 0.5 || b.1 < 0.5 || b.2 > 99.5 || b.3 > 79.5 {
                continue; // cut at the edge
            }
            let (cx, cy) = (f64::midpoint(b.0, b.2), f64::midpoint(b.1, b.3));
            for q in &pts {
                let rr = ((q.x - cx).powi(2) + (q.y - cy).powi(2)).sqrt();
                assert!((rr - d / 2.0).abs() < 0.002, "dot radius {rr}");
            }
            centres.push((cy, cx));
        }
        let rs = rows(centres, 1e-3);
        assert!(rs.len() > 4);
        for row in &rs {
            for w in row.1.windows(2) {
                assert!((w[1] - w[0] - spacing).abs() < 1e-3, "dot step in a row");
            }
        }
        for w in rs.windows(2) {
            let shift = (w[1].1[0] - w[0].1[0]).rem_euclid(spacing);
            assert!(
                (shift - spacing / 2.0).abs() < 1e-3,
                "every second row is offset by half the spacing, got {shift}"
            );
        }
    }
}

// ---------- criterion 7: honeycomb ----------

#[test]
fn c7_honeycomb_cells_are_regular_hexagons_with_the_wall_between() {
    let r = rect(0.0, 0.0, 100.0, 80.0);
    let (cell, wall) = (8.0, 1.0);
    for mode in [PatternMode::Lines, PatternMode::Shapes] {
        let f = fill(
            GeneratorKind::Honeycomb,
            &[(PatternParam::CellSize, cell), (PatternParam::Wall, wall)],
            0.0,
            mode,
        );
        let ps = pieces(&r, &f);
        assert!(all_closed(&ps), "honeycomb cells are closed in {mode:?}");
        let mut centres = Vec::new();
        for piece in &ps {
            let (a, _) = &piece.outlines[0];
            let b = bbox(&a.iter().map(|t| t.0).collect::<Vec<_>>());
            if piece.outlines.len() != 1
                || a.len() != 6
                || b.0 < 0.01
                || b.1 < 0.01
                || b.2 > 99.99
                || b.3 > 79.99
            {
                continue;
            }
            let v: Vec<Point> = a.iter().map(|t| t.0).collect();
            let edges: Vec<f64> = (0..6)
                .map(|i| {
                    let (s, e) = (v[i], v[(i + 1) % 6]);
                    ((e.x - s.x).powi(2) + (e.y - s.y).powi(2)).sqrt()
                })
                .collect();
            for e in &edges {
                assert!((e - edges[0]).abs() < 2e-3, "regular: edges {edges:?}");
            }
            let flat = 3f64.sqrt() * edges[0];
            assert!(
                (flat - (cell - wall)).abs() < 2e-3,
                "flat-to-flat {flat} = cell size {cell} - wall {wall}"
            );
            let cx = v.iter().map(|q| q.x).sum::<f64>() / 6.0;
            let cy = v.iter().map(|q| q.y).sum::<f64>() / 6.0;
            centres.push(p(cx, cy));
        }
        assert!(centres.len() > 20, "{mode:?}: many whole cells");
        // Neighbouring centres are one cell size apart.
        let c0 = centres[centres.len() / 2];
        let nearest = centres
            .iter()
            .filter(|c| **c != c0)
            .map(|c| ((c.x - c0.x).powi(2) + (c.y - c0.y).powi(2)).sqrt())
            .fold(f64::INFINITY, f64::min);
        assert!((nearest - cell).abs() < 2e-3, "centre distance {nearest}");
    }
}

// ---------- criterion 8: brick wall ----------

#[test]
fn c8_brick_rows_shift_by_half_a_brick() {
    let r = rect(0.0, 0.0, 120.0, 80.0);
    let f = fill(
        GeneratorKind::BrickWall,
        &[
            (PatternParam::BrickWidth, 20.0),
            (PatternParam::BrickHeight, 10.0),
            (PatternParam::Joint, 1.0),
        ],
        0.0,
        PatternMode::Shapes,
    );
    let ps = pieces(&r, &f);
    assert!(all_closed(&ps));
    let mut bricks = Vec::new();
    for piece in &ps {
        let pts = piece_points(piece);
        let b = bbox(&pts);
        if b.0 < 0.01 || b.1 < 0.01 || b.2 > 119.99 || b.3 > 79.99 {
            continue;
        }
        bricks.push(b);
    }
    assert!(bricks.len() > 10);
    for b in &bricks {
        let (w, h) = (b.2 - b.0, b.3 - b.1);
        // Width/height are the pitch (width 20 with a joint 1 leaves a 19 mm brick); the spec's
        // check is the shift, the size is reported if it differs.
        assert!(
            ((w - 19.0).abs() < 1e-3 || (w - 20.0).abs() < 1e-3)
                && ((h - 9.0).abs() < 1e-3 || (h - 10.0).abs() < 1e-3),
            "brick {w} x {h}"
        );
    }
    let rs = rows(bricks.iter().map(|b| (b.1, b.0)).collect(), 1e-3);
    assert!(rs.len() >= 3);
    for row in &rs {
        for w in row.1.windows(2) {
            assert!((w[1] - w[0] - 20.0).abs() < 1e-3, "brick pitch along a row");
        }
    }
    for w in rs.windows(2) {
        assert!((w[1].0 - w[0].0 - 10.0).abs() < 1e-3, "row pitch 10");
        let shift = (w[1].1[0] - w[0].1[0]).rem_euclid(20.0);
        assert!(
            (shift - 10.0).abs() < 1e-3,
            "row n+1 shifted by 10 mm, got {shift}"
        );
    }
}

// ---------- criterion 9: leaf forest ----------

fn segs_cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    let o = |p1: Point, p2: Point, p3: Point| {
        (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x)
    };
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

fn polys_overlap(p: &[Point], q: &[Point]) -> bool {
    for i in 0..p.len() {
        for j in 0..q.len() {
            if segs_cross(p[i], p[(i + 1) % p.len()], q[j], q[(j + 1) % q.len()]) {
                return true;
            }
        }
    }
    winding(&[q.to_vec()], p[0]) != 0 || winding(&[p.to_vec()], q[0]) != 0
}

fn leaves(ps: &[PatternPiece]) -> Vec<Vec<Point>> {
    ps.iter()
        .map(|pc| flatten(&pc.outlines[0].0, true, 24))
        .collect()
}

#[test]
fn c9_leaves_never_overlap_and_stay_in_the_region() {
    let r = region();
    for (density, jitter) in [(0.5, 0.3), (1.0, 0.0), (1.0, 1.0)] {
        for seed in [1_u32, 2, 99, u32::MAX] {
            let mut f = fill(
                GeneratorKind::LeafForest,
                &[
                    (PatternParam::Density, density),
                    (PatternParam::SizeJitter, jitter),
                ],
                1.0,
                PatternMode::Shapes,
            );
            f.seed = PatternSeed::new(seed);
            let ps = pieces(&r, &f);
            assert!(
                ps.len() > 20,
                "density {density}: some leaves ({})",
                ps.len()
            );
            assert!(all_closed(&ps));
            assert_inside(&r, &ps, 1.0, "leaves");
            let ls = leaves(&ps);
            let boxes: Vec<_> = ls.iter().map(|l| bbox(l)).collect();
            for i in 0..ls.len() {
                for j in i + 1..ls.len() {
                    let (a, b) = (boxes[i], boxes[j]);
                    if a.2 < b.0 || b.2 < a.0 || a.3 < b.1 || b.3 < a.1 {
                        continue;
                    }
                    assert!(
                        !polys_overlap(&ls[i], &ls[j]),
                        "seed {seed} density {density}: leaves {i} and {j} overlap"
                    );
                }
            }
        }
    }
}

#[test]
fn c9_leaf_size_and_rotation_jitter_zero_give_equal_leaves() {
    let r = rect(0.0, 0.0, 100.0, 100.0);
    let f = fill(
        GeneratorKind::LeafForest,
        &[
            (PatternParam::LeafSize, 6.0),
            (PatternParam::SizeJitter, 0.0),
            (PatternParam::RotationJitter, 0.0),
        ],
        0.0,
        PatternMode::Shapes,
    );
    let ps = pieces(&r, &f);
    let mut sizes = Vec::new();
    for l in leaves(&ps) {
        let b = bbox(&l);
        if b.0 < 0.01 || b.1 < 0.01 || b.2 > 99.99 || b.3 > 99.99 {
            continue;
        }
        sizes.push((b.2 - b.0, b.3 - b.1));
    }
    assert!(sizes.len() > 10);
    for s in &sizes {
        assert!(
            (s.0 - sizes[0].0).abs() < 0.02 && (s.1 - sizes[0].1).abs() < 0.02,
            "same size and rotation: {s:?} vs {:?}",
            sizes[0]
        );
    }
    let long = sizes[0].0.max(sizes[0].1);
    assert!(
        (long - 6.0).abs() < 0.1,
        "leaf length about the leaf size, got {long}"
    );
}

// ---------- criterion 10: sea waves ----------

fn circumcircle(a: Point, b: Point, c: Point) -> Option<(Point, f64)> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-9 {
        return None;
    }
    let a2 = a.x * a.x + a.y * a.y;
    let b2 = b.x * b.x + b.y * b.y;
    let c2 = c.x * c.x + c.y * c.y;
    let ux = (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d;
    let uy = (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d;
    Some((p(ux, uy), ((a.x - ux).powi(2) + (a.y - uy).powi(2)).sqrt()))
}

#[test]
fn c10_sea_waves_are_concentric_arcs_of_the_ring_radii_in_offset_rows() {
    let r = rect(0.0, 0.0, 100.0, 80.0);
    let (radius, rings, ring_spacing) = (6.0, 3.0, 1.5);
    let f = fill(
        GeneratorKind::SeaWaves,
        &[
            (PatternParam::ScaleRadius, radius),
            (PatternParam::Rings, rings),
            (PatternParam::RingSpacing, ring_spacing),
        ],
        0.0,
        PatternMode::Lines,
    );
    let ps = pieces(&r, &f);
    assert!(all_open(&ps), "Lines mode: arcs are open");
    let mut centres_by_ring: Vec<(f64, f64)> = Vec::new();
    let mut exact = 0;
    for piece in &ps {
        for (a, _) in &piece.outlines {
            // Exact (uncut) arcs keep their cubic handles: each such segment is part of a circle.
            for w in a.windows(2) {
                let ((s0, _, so), (e0, ei, _)) = (w[0], w[1]);
                if so == Vec2::ZERO && ei == Vec2::ZERO {
                    continue;
                }
                let c1 = p(s0.x + so.x, s0.y + so.y);
                let c2 = p(e0.x + ei.x, e0.y + ei.y);
                let m = cubic(s0, c1, c2, e0, 0.5);
                let Some((c, rr)) = circumcircle(s0, m, e0) else {
                    continue;
                };
                exact += 1;
                let k = (radius - rr) / ring_spacing;
                assert!(
                    (k - k.round()).abs() < 0.01 && k.round() >= 0.0 && k.round() < rings,
                    "arc radius {rr} is not one of {radius} - k * {ring_spacing}, k < {rings}"
                );
                for t in [0.25, 0.75] {
                    let q = cubic(s0, c1, c2, e0, t);
                    let d = ((q.x - c.x).powi(2) + (q.y - c.y).powi(2)).sqrt();
                    assert!((d - rr).abs() < 0.01, "a segment on one circle");
                }
                if k.round() == 0.0 {
                    centres_by_ring.push((c.y, c.x));
                }
            }
        }
    }
    assert!(exact > 20, "uncut arcs keep exact curves ({exact})");
    centres_by_ring.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    centres_by_ring.dedup_by(|a, b| (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01);
    let rs = rows(centres_by_ring, 0.05);
    assert!(rs.len() >= 3, "several rows of scales");
    for w in rs.windows(2) {
        let shift = (w[1].1[0] - w[0].1[0]).rem_euclid(2.0 * radius);
        assert!(
            (shift - radius).abs() < 0.05 || (shift - radius).abs() > 2.0 * radius - 0.05,
            "rows offset by half a scale width ({radius}), got {shift}"
        );
    }
}

// ---------- criterion 11: waves ----------

#[test]
fn c11_waves_with_amplitude_zero_equal_hatch() {
    for r in [region(), rect(0.0, 0.0, 50.0, 30.0)] {
        for (spacing, angle, margin) in [(2.0, 0.0, 0.0), (3.0, 37.0, 1.5), (1.0, 90.0, 0.5)] {
            for mode in [PatternMode::Lines, PatternMode::Shapes] {
                let waves = fill(
                    GeneratorKind::Waves,
                    &[
                        (PatternParam::Amplitude, 0.0),
                        (PatternParam::Spacing, spacing),
                        (PatternParam::Angle, angle),
                    ],
                    margin,
                    mode,
                );
                let hatch = fill(
                    GeneratorKind::Hatch,
                    &[
                        (PatternParam::Spacing, spacing),
                        (PatternParam::Angle, angle),
                    ],
                    margin,
                    mode,
                );
                assert_eq!(
                    pieces(&r, &waves),
                    pieces(&r, &hatch),
                    "{spacing}/{angle}/{margin}/{mode:?}"
                );
            }
        }
    }
}

#[test]
fn c11_waves_are_sines_of_the_amplitude_and_wavelength() {
    let r = rect(0.0, 0.0, 100.0, 60.0);
    let (wl, amp) = (10.0, 2.0);
    let f = fill(
        GeneratorKind::Waves,
        &[
            (PatternParam::Wavelength, wl),
            (PatternParam::Amplitude, amp),
            (PatternParam::Spacing, 8.0),
            (PatternParam::Angle, 0.0),
        ],
        0.0,
        PatternMode::Lines,
    );
    let ps = pieces(&r, &f);
    assert!(all_open(&ps));
    let mut checked = 0;
    for piece in &ps {
        let pts = flatten(&piece.outlines[0].0, false, 32);
        let b = bbox(&pts);
        if b.1 < 0.01 || b.3 > 59.99 || b.2 - b.0 < 99.0 {
            continue;
        }
        checked += 1;
        assert!(
            ((b.3 - b.1) - 2.0 * amp).abs() < 0.05,
            "peak to peak {}",
            b.3 - b.1
        );
        // Peaks one wavelength apart.
        let mut peaks: Vec<f64> = Vec::new();
        for w in pts.windows(3) {
            if w[1].y < w[0].y && w[1].y <= w[2].y && (w[1].y - b.1) < 0.01 {
                peaks.push(w[1].x);
            }
        }
        peaks.dedup_by(|a, b| (*a - *b).abs() < 0.5);
        for w in peaks.windows(2) {
            assert!(
                (w[1] - w[0] - wl).abs() < 0.2,
                "peak distance {}",
                w[1] - w[0]
            );
        }
    }
    assert!(checked >= 3, "whole wave lines were checked");
}

// ---------- criterion 12: interpolation waves ----------

fn covered_near(ps: &[PatternPiece], target: &[Point], within: f64) -> f64 {
    // Length of pieces whose every sample lies within `within` of the target polyline.
    let mut total = 0.0;
    for piece in ps {
        for (a, c) in &piece.outlines {
            let pts = flatten(a, *c, 24);
            let near = pts.iter().all(|q| {
                (0..target.len() - 1).any(|i| seg_dist(*q, target[i], target[i + 1]) <= within)
            });
            if near {
                total += pts
                    .windows(2)
                    .map(|w| ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt())
                    .sum::<f64>();
            }
        }
    }
    total
}

fn arc_len(pts: &[Point]) -> f64 {
    pts.windows(2)
        .map(|w| ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt())
        .sum()
}

#[test]
fn c12_interpolation_first_and_last_lines_follow_the_top_and_bottom_boundary() {
    // A hexagon: leftmost (0, 20) and rightmost (100, 20) are single points.
    let outline = poly(&[
        (0.0, 20.0),
        (20.0, 0.0),
        (80.0, 0.0),
        (100.0, 20.0),
        (80.0, 50.0),
        (20.0, 50.0),
    ]);
    let r = vec![(outline, true)];
    let f = fill(
        GeneratorKind::InterpolationWaves,
        &[(PatternParam::Lines, 5.0)],
        0.0,
        PatternMode::Lines,
    );
    let ps = pieces(&r, &f);
    assert!(all_open(&ps));
    let top = [p(0.0, 20.0), p(20.0, 0.0), p(80.0, 0.0), p(100.0, 20.0)];
    let bottom = [p(0.0, 20.0), p(20.0, 50.0), p(80.0, 50.0), p(100.0, 20.0)];
    let near_top = covered_near(&ps, &top, 0.02);
    let near_bottom = covered_near(&ps, &bottom, 0.02);
    assert!(
        near_top >= 0.9 * arc_len(&top),
        "the first line follows the top boundary: {near_top:.2} of {:.2} mm",
        arc_len(&top)
    );
    assert!(
        near_bottom >= 0.9 * arc_len(&bottom),
        "the last line follows the bottom boundary: {near_bottom:.2} of {:.2} mm",
        arc_len(&bottom)
    );
}

#[test]
fn c12_interpolation_lines_in_an_ellipse_are_concentric_half_ellipses() {
    // By symmetry, blending the top and bottom halves point by point (by arc length) gives line
    // t on the ellipse with ry' = |1 - 2t| * ry; the middle line is the horizontal axis.
    let (cx, cy, rx, ry) = (50.0, 30.0, 50.0, 25.0);
    let r = vec![(ellipse(cx, cy, rx, ry, false), true)];
    let f = fill(
        GeneratorKind::InterpolationWaves,
        &[(PatternParam::Lines, 5.0)],
        0.0,
        PatternMode::Lines,
    );
    let ps = pieces(&r, &f);
    let mut by_t = [0.0_f64; 5];
    for piece in &ps {
        for (a, c) in &piece.outlines {
            let pts = flatten(a, *c, 24);
            let len = arc_len(&pts);
            // Which t does this line fit?
            let mut fit = None;
            for (k, slot) in by_t.iter_mut().enumerate() {
                let t = k as f64 / 4.0;
                let ry2 = (1.0 - 2.0 * t).abs() * ry;
                let side = if t < 0.5 { -1.0 } else { 1.0 };
                let curve: Vec<Point> = (0..=2000)
                    .map(|i| {
                        let th = std::f64::consts::PI * f64::from(i) / 2000.0;
                        p(cx - rx * th.cos(), cy + side * ry2 * th.sin())
                    })
                    .collect();
                let ok = pts
                    .iter()
                    .all(|q| curve.windows(2).any(|w| seg_dist(*q, w[0], w[1]) < 0.05));
                if ok {
                    fit = Some(k);
                    *slot += len;
                    break;
                }
            }
            assert!(
                fit.is_some(),
                "a line that is no blend of top and bottom: {:?}",
                &pts[..3]
            );
        }
    }
    for (k, len) in by_t.iter().enumerate() {
        assert!(
            *len > 80.0,
            "line {k} of 5 is there and spans the ellipse ({len:.1} mm)"
        );
    }
}

#[test]
fn c12_interpolation_respects_holes_and_margin() {
    let r = region();
    let ps = pieces(
        &r,
        &fill(
            GeneratorKind::InterpolationWaves,
            &[(PatternParam::Lines, 20.0)],
            2.0,
            PatternMode::Lines,
        ),
    );
    assert!(ps.len() > 5, "interpolation lines are drawn");
    assert_inside(&r, &ps, 2.0, "interpolation with holes");
}

// ---------- criterion 16: Lines vs Shapes ----------

#[test]
fn c16_lines_mode_gives_open_lines_and_shapes_mode_closed_outlines() {
    let r = region();
    for kind in [
        GeneratorKind::Hatch,
        GeneratorKind::Crosshatch,
        GeneratorKind::Grid,
        GeneratorKind::Waves,
        GeneratorKind::SeaWaves,
        GeneratorKind::InterpolationWaves,
    ] {
        let lines = pieces(&r, &fill(kind, &[], 0.0, PatternMode::Lines));
        let shapes = pieces(&r, &fill(kind, &[], 0.0, PatternMode::Shapes));
        assert!(all_open(&lines), "{kind:?} Lines: open paths");
        assert!(
            !shapes.is_empty() && all_closed(&shapes),
            "{kind:?} Shapes: closed outlines"
        );
    }
    for kind in [
        GeneratorKind::Dots,
        GeneratorKind::Honeycomb,
        GeneratorKind::LeafForest,
        GeneratorKind::BrickWall,
    ] {
        for mode in [PatternMode::Lines, PatternMode::Shapes] {
            let ps = pieces(&r, &fill(kind, &[], 0.0, mode));
            assert!(all_closed(&ps), "{kind:?} {mode:?}: naturally closed");
        }
        assert_eq!(
            pieces(&r, &fill(kind, &[], 0.0, PatternMode::Lines)),
            pieces(&r, &fill(kind, &[], 0.0, PatternMode::Shapes)),
            "{kind:?}: the same closed outlines in both modes"
        );
    }
}

#[test]
fn c16_shapes_mode_hatch_strips_are_the_line_width_wide() {
    let r = rect(0.0, 0.0, 60.0, 40.0);
    let f = fill(
        GeneratorKind::Hatch,
        &[(PatternParam::Angle, 0.0), (PatternParam::Spacing, 4.0)],
        0.0,
        PatternMode::Shapes,
    );
    for width in [0.25, 1.0] {
        let ps = pattern_pieces_of(
            &r,
            Angle::from_radians(0.0),
            &f,
            Length::from_mm(width),
            TOL,
        )
        .unwrap();
        for piece in &ps {
            let b = bbox(&piece_points(piece));
            if b.1 < 0.01 || b.3 > 39.99 {
                continue;
            }
            assert!(
                (b.3 - b.1 - width).abs() < 2e-3,
                "strip height {} = width {width}",
                b.3 - b.1
            );
        }
    }
}

// ---------- criterion 17: determinism and seeds ----------

#[test]
fn c17_same_input_same_pieces_and_the_seed_only_moves_leaves() {
    let r = region();
    for kind in GeneratorKind::ALL {
        for mode in [PatternMode::Lines, PatternMode::Shapes] {
            let mut a = fill(kind, &[], 1.0, mode);
            let first = pieces(&r, &a);
            assert_eq!(first, pieces(&r, &a), "{kind:?}: same run, same pieces");
            a.seed = PatternSeed::new(54321);
            let reseeded = pieces(&r, &a);
            if kind == GeneratorKind::LeafForest {
                assert_ne!(first, reseeded, "a new seed moves the leaves");
            } else {
                assert_eq!(first, reseeded, "{kind:?} is unseeded");
            }
            assert_eq!(kind.is_seeded(), kind == GeneratorKind::LeafForest);
        }
    }
}

#[test]
fn c17_moving_the_shape_moves_the_pattern_with_it() {
    let r = region();
    let moved: Vec<PatternOutline> = r
        .iter()
        .map(|(a, c)| {
            (
                a.iter()
                    .map(|t| (p(t.0.x + 13.25, t.0.y - 7.5), t.1, t.2))
                    .collect(),
                *c,
            )
        })
        .collect();
    for kind in GeneratorKind::ALL {
        let f = fill(kind, &[], 1.0, PatternMode::Lines);
        let a = pieces(&r, &f);
        let b = pieces(&moved, &f);
        assert_eq!(a.len(), b.len(), "{kind:?}: same count after a move");
        for (pa, pb) in a.iter().zip(&b) {
            for ((oa, _), (ob, _)) in pa.outlines.iter().zip(&pb.outlines) {
                for (ta, tb) in oa.iter().zip(ob) {
                    assert!(
                        (tb.0.x - ta.0.x - 13.25).abs() < 2e-3
                            && (tb.0.y - ta.0.y + 7.5).abs() < 2e-3,
                        "{kind:?}: pieces moved with the shape"
                    );
                }
            }
        }
    }
}

// ---------- criterion 18: count limits ----------

#[test]
fn c18_over_the_hard_limit_nothing_is_generated() {
    let r = rect(0.0, 0.0, 1000.0, 1000.0);
    let f = fill(
        GeneratorKind::Dots,
        &[
            (PatternParam::Spacing, 0.05),
            (PatternParam::Diameter, 0.05),
        ],
        0.0,
        PatternMode::Lines,
    );
    match pattern_pieces_of(&r, Angle::from_radians(0.0), &f, Length::from_mm(W), TOL) {
        Err(PatternError::TooMany { estimate }) => assert!(estimate > PATTERN_MAX_PIECES),
        other => panic!("expected TooMany, got {:?}", other.map(|v| v.len())),
    }
}

#[test]
fn c18_hard_limit_holds_for_every_area_generator_on_a_large_region() {
    // Line generators cannot pass 500,000 pieces at the minimum spacing on a sheet-sized shape,
    // so only the lattice and scatter generators (and the line count) are driven over it.
    let r = rect(0.0, 0.0, 1000.0, 1000.0);
    let small: [(GeneratorKind, &[(PatternParam, f64)]); 6] = [
        (
            GeneratorKind::Dots,
            &[
                (PatternParam::Spacing, 0.05),
                (PatternParam::Diameter, 0.05),
            ],
        ),
        (
            GeneratorKind::Honeycomb,
            &[(PatternParam::Wall, 0.05), (PatternParam::CellSize, 0.2)],
        ),
        (
            GeneratorKind::BrickWall,
            &[
                (PatternParam::Joint, 0.05),
                (PatternParam::BrickWidth, 0.2),
                (PatternParam::BrickHeight, 0.2),
            ],
        ),
        (
            GeneratorKind::LeafForest,
            &[(PatternParam::LeafSize, 0.05), (PatternParam::Density, 1.0)],
        ),
        (
            GeneratorKind::SeaWaves,
            &[
                (PatternParam::ScaleRadius, 0.2),
                (PatternParam::RingSpacing, 0.05),
            ],
        ),
        (
            GeneratorKind::InterpolationWaves,
            &[(PatternParam::Lines, 1_000_000.0)],
        ),
    ];
    for (kind, params) in small {
        let f = fill(kind, params, 0.0, PatternMode::Lines);
        let res = pattern_pieces_of(&r, Angle::from_radians(0.0), &f, Length::from_mm(W), TOL);
        assert!(
            matches!(res, Err(PatternError::TooMany { .. })),
            "{kind:?}: refused before generating, got {:?}",
            res.map(|v| v.len())
        );
    }
}

#[test]
fn c18_between_the_limits_the_pattern_is_generated() {
    // ~70,000 dots: above the note limit, below the hard one.
    let r = rect(0.0, 0.0, 60.0, 60.0);
    let f = fill(
        GeneratorKind::Dots,
        &[(PatternParam::Spacing, 0.24), (PatternParam::Diameter, 0.1)],
        0.0,
        PatternMode::Lines,
    );
    let ps = pieces(&r, &f);
    assert!(
        ps.len() > PATTERN_NOTE_PIECES && ps.len() <= PATTERN_MAX_PIECES,
        "{} pieces",
        ps.len()
    );
}

// ---------- white-box edges ----------

#[test]
fn edge_sea_waves_with_more_rings_than_fit_stay_finite_and_inside() {
    let r = rect(0.0, 0.0, 40.0, 30.0);
    let f = fill(
        GeneratorKind::SeaWaves,
        &[
            (PatternParam::ScaleRadius, 5.0),
            (PatternParam::Rings, 1000.0),
            (PatternParam::RingSpacing, 1.0),
        ],
        0.0,
        PatternMode::Lines,
    );
    let ps = pattern_pieces_of(&r, Angle::from_radians(0.0), &f, Length::from_mm(W), TOL);
    if let Ok(ps) = ps {
        assert_inside(&r, &ps, 0.0, "1000 rings");
    }
}

#[test]
fn edge_huge_and_extreme_angles_are_fine() {
    let r = region();
    for angle in [1.0e6, -1.0e9, 359.999_999, 1.0e-12] {
        let f = fill(
            GeneratorKind::Hatch,
            &[(PatternParam::Angle, angle)],
            0.5,
            PatternMode::Lines,
        );
        let ps = pieces(&r, &f);
        assert!(!ps.is_empty(), "angle {angle}");
        assert_inside(&r, &ps, 0.5, "extreme angle");
    }
}

#[test]
fn edge_rotated_shape_rotates_the_pattern_frame() {
    let r = rect(0.0, 0.0, 60.0, 40.0);
    let f = fill(
        GeneratorKind::Hatch,
        &[(PatternParam::Angle, 0.0), (PatternParam::Spacing, 3.0)],
        0.0,
        PatternMode::Lines,
    );
    let ps = pattern_pieces_of(
        &r,
        Angle::from_radians(30f64.to_radians()),
        &f,
        Length::from_mm(W),
        TOL,
    )
    .unwrap();
    let dirs = line_dirs(&ps);
    assert!(
        dirs.iter()
            .all(|d| dir_matches(d.0, 30.0) || dir_matches(d.0, -30.0)),
        "the base angle is the object's rotation register"
    );
    assert_inside(&r, &ps, 0.0, "rotated frame");
}

#[test]
fn edge_tiny_and_degenerate_regions() {
    for r in [
        rect(0.0, 0.0, 0.01, 0.01),
        rect(0.0, 0.0, 100.0, 0.0),
        vec![(poly(&[(0.0, 0.0), (10.0, 0.0)]), true)],
        vec![(Vec::new(), true)],
        Vec::new(),
    ] {
        for kind in GeneratorKind::ALL {
            let f = fill(kind, &[], 0.0, PatternMode::Shapes);
            let res = pattern_pieces_of(&r, Angle::from_radians(0.0), &f, Length::from_mm(W), TOL);
            let ps = res.expect("no error on a degenerate region");
            assert_inside(&r, &ps, 0.0, "degenerate");
        }
    }
}

#[test]
fn edge_non_finite_region_gives_no_pieces() {
    let r = vec![(poly(&[(0.0, 0.0), (f64::NAN, 0.0), (10.0, 10.0)]), true)];
    for kind in GeneratorKind::ALL {
        let res = pattern_pieces_of(
            &r,
            Angle::from_radians(0.0),
            &fill(kind, &[], 0.0, PatternMode::Lines),
            Length::from_mm(W),
            TOL,
        );
        assert_eq!(res, Ok(Vec::new()));
    }
}

#[test]
fn schema_every_default_is_accepted_and_every_minimum_is_positive_for_lengths() {
    for kind in GeneratorKind::ALL {
        let g = PatternGenerator::defaults(kind);
        assert_eq!(g.kind(), kind);
        for spec in pattern_params(kind) {
            assert!(
                spec.accepts(spec.default),
                "{kind:?} {}: default accepted",
                spec.name
            );
            assert!(
                !spec.accepts(f64::NAN),
                "{kind:?} {}: NaN refused",
                spec.name
            );
            assert!(g.with(spec.param, spec.min - 1.0).is_err() || spec.min == f64::MIN);
        }
    }
}

// ---------- property: any rectangle with any hole and margin keeps the pieces inside ----------

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, .. ProptestConfig::default() })]

    #[test]
    fn prop_pieces_stay_inside_the_inset_region(
        kind_ix in 0usize..10,
        w in 8.0f64..60.0,
        h in 8.0f64..60.0,
        hole in 0.0f64..0.4,
        margin in 0.0f64..2.5,
        angle in -180.0f64..180.0,
        rot in -3.2f64..3.2,
        shapes in any::<bool>(),
        seed in any::<u32>(),
    ) {
        let kind = GeneratorKind::ALL[kind_ix];
        let mut region = rect(0.0, 0.0, w, h);
        if hole > 0.05 {
            let r = hole * w.min(h) / 2.0;
            region.push((circle(w / 2.0, h / 2.0, r, true), true));
            let a = signed_area(&flatten(&region[0].0, true, 8));
            let b = signed_area(&flatten(&region[1].0, true, 8));
            if a * b > 0.0 {
                region[1] = (circle(w / 2.0, h / 2.0, r, false), true);
            }
        }
        let mut params: Vec<(PatternParam, f64)> = Vec::new();
        if pattern_params(kind).iter().any(|s| s.param == PatternParam::Angle) {
            params.push((PatternParam::Angle, angle));
        }
        let mut f = fill(kind, &params, margin, if shapes { PatternMode::Shapes } else { PatternMode::Lines });
        f.seed = PatternSeed::new(seed);
        let ps = pattern_pieces_of(&region, Angle::from_radians(rot), &f, Length::from_mm(W), TOL).unwrap();
        assert_inside(&region, &ps, margin, &format!("{kind:?}"));
    }

    #[test]
    fn prop_area_of_shapes_pieces_never_exceeds_the_region(
        kind_ix in 0usize..10,
        w in 10.0f64..50.0,
        h in 10.0f64..50.0,
    ) {
        let kind = GeneratorKind::ALL[kind_ix];
        let region = rect(0.0, 0.0, w, h);
        let ps = pieces(&region, &fill(kind, &[], 0.0, PatternMode::Shapes));
        let mut area = 0.0;
        for piece in &ps {
            for (i, (a, c)) in piece.outlines.iter().enumerate() {
                let sign = if i == 0 { 1.0 } else { -1.0 };
                area += signed_area(&flatten(a, *c, 24)).abs() * sign;
            }
        }
        prop_assert!(area <= w * h + 0.05, "{kind:?}: pieces area {area} > region {}", w * h);
    }
}
