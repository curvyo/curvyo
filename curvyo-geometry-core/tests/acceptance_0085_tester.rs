//! Independent tester tests for `specs/0085-stroke-to-path` at the kernel (criteria 4 to 10, 12,
//! 13, 26, 27), written from the specification before the implementation was read; only the
//! public names were looked up.
//!
//! The reference is built here, not with the code under test: the stroke area of a polyline is
//! the union of one rectangle per segment, one join piece per corner (miter quad within the
//! limit 4, else a bevel triangle; a round join is a disk) and the caps, and of the dash pieces
//! and markers (`0018` criterion 4: arrow 4 w long and 3 w wide centred on its anchor, dot 3 w
//! across). Curves are flattened densely; a flattening vertex on a smooth curve gets a bevel
//! wedge. Both regions are sampled on a grid; a sample may differ only within `NEAR_EDGE_MM` of
//! the reference edge, and the result's winding must be 0 or 1 everywhere (the union, no
//! overlapping pieces, criterion 9).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::similar_names,
    missing_docs
)]

mod common;

use std::f64::consts::{PI, TAU};

use common::{Anchors, KERNEL_TOLERANCE, circle, polygon};
use curvyo_document_core::{
    DashPattern, Length, LineCap, LineJoin, MarkerPlace, MarkerShape, Markers, Point, Tolerance,
    Vec2,
};
use curvyo_geometry_core::{
    OffsetOutlines, Outline, StrokeOutlineError, StrokeShape, stroke_outline,
};
use proptest::prelude::*;

/// A sample may differ from the reference only this close to the reference edge: the kernel's
/// 0.01 mm plus the 0.001 mm grid plus the sampling of the reference.
const NEAR_EDGE_MM: f64 = 0.03;

type P = (f64, f64);

// ----- the reference stroke -------------------------------------------------------------------

/// One piece of the reference region.
#[derive(Debug, Clone)]
enum Piece {
    /// A simple polygon (convex or not), even-odd.
    Poly(Vec<P>),
    /// A disk.
    Disk(P, f64),
}

impl Piece {
    fn contains(&self, p: P) -> bool {
        match self {
            Self::Disk(c, r) => (p.0 - c.0).hypot(p.1 - c.1) <= *r,
            Self::Poly(points) => {
                let n = points.len();
                let mut inside = false;
                for i in 0..n {
                    let (a, b) = (points[i], points[(i + 1) % n]);
                    if (a.1 > p.1) != (b.1 > p.1) {
                        let x = a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0);
                        if p.0 < x {
                            inside = !inside;
                        }
                    }
                }
                inside
            }
        }
    }

    fn bounds(&self) -> (P, P) {
        match self {
            Self::Disk(c, r) => ((c.0 - r, c.1 - r), (c.0 + r, c.1 + r)),
            Self::Poly(points) => points.iter().fold(
                ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)),
                |(lo, hi), q| {
                    (
                        (lo.0.min(q.0), lo.1.min(q.1)),
                        (hi.0.max(q.0), hi.1.max(q.1)),
                    )
                },
            ),
        }
    }
}

fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}
fn add(a: P, b: P) -> P {
    (a.0 + b.0, a.1 + b.1)
}
fn mul(a: P, k: f64) -> P {
    (a.0 * k, a.1 * k)
}
fn len(a: P) -> f64 {
    a.0.hypot(a.1)
}
fn unit(a: P) -> P {
    mul(a, 1.0 / len(a))
}
/// The left normal.
fn left(d: P) -> P {
    (-d.1, d.0)
}
fn cross(a: P, b: P) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

/// One vertex of a reference polyline; `sharp` vertices get the stroke's join, the others (the
/// flattening of a smooth curve) a bevel wedge.
#[derive(Debug, Clone, Copy)]
struct V {
    p: P,
    sharp: bool,
}

fn sharp(points: &[P]) -> Vec<V> {
    points.iter().map(|&p| V { p, sharp: true }).collect()
}

/// The join piece at `v` between the directions `d1` (in) and `d2` (out).
fn join_piece(v: P, d1: P, d2: P, h: f64, join: LineJoin, out: &mut Vec<Piece>) {
    let turn = cross(d1, d2);
    let dot = d1.0 * d2.0 + d1.1 * d2.1;
    if turn.abs() < 1e-12 && dot > 0.0 {
        return;
    }
    if join == LineJoin::Round {
        out.push(Piece::Disk(v, h));
        return;
    }
    // The outer side is opposite the turn.
    let s = if turn > 0.0 { -1.0 } else { 1.0 };
    let p1 = add(v, mul(left(d1), s * h));
    let p2 = add(v, mul(left(d2), s * h));
    // Interior angle theta between the segments; miter length / width = 1 / sin(theta / 2).
    let phi = turn.atan2(dot).abs();
    let half_theta = (PI - phi) / 2.0;
    let within = half_theta.sin() > 0.0 && 1.0 / half_theta.sin() <= 4.0;
    if join == LineJoin::Miter && within {
        let bisector = unit(add(left(d1), left(d2)));
        let tip = add(v, mul(bisector, s * h / (phi / 2.0).cos()));
        out.push(Piece::Poly(vec![v, p1, tip, p2]));
    } else {
        out.push(Piece::Poly(vec![v, p1, p2]));
    }
}

fn cap_piece(end: P, outward: P, h: f64, cap: LineCap, out: &mut Vec<Piece>) {
    match cap {
        LineCap::Butt => {}
        LineCap::Round => out.push(Piece::Disk(end, h)),
        LineCap::Square => {
            let n = left(outward);
            let far = add(end, mul(outward, h));
            out.push(Piece::Poly(vec![
                add(end, mul(n, h)),
                add(far, mul(n, h)),
                sub(far, mul(n, h)),
                sub(end, mul(n, h)),
            ]));
        }
    }
}

/// The reference stroke pieces of one polyline.
fn stroke_pieces(vs: &[V], closed: bool, w: f64, join: LineJoin, cap: LineCap) -> Vec<Piece> {
    let h = w / 2.0;
    let mut out = Vec::new();
    // Drop zero-length segments.
    let mut pts: Vec<V> = Vec::new();
    for &v in vs {
        if pts.last().is_none_or(|l| len(sub(v.p, l.p)) > 1e-9) {
            pts.push(v);
        } else if v.sharp {
            pts.last_mut().unwrap().sharp = true;
        }
    }
    if closed && pts.len() > 1 && len(sub(pts[0].p, pts[pts.len() - 1].p)) < 1e-9 {
        pts.pop();
    }
    let n = pts.len();
    if n < 2 {
        // A single point: a dot with Round, a square with Square (horizontal, as SVG), else none.
        if n == 1 {
            cap_piece(pts[0].p, (1.0, 0.0), h, cap, &mut out);
            cap_piece(pts[0].p, (-1.0, 0.0), h, cap, &mut out);
        }
        return out;
    }
    let segs = if closed { n } else { n - 1 };
    let dir = |i: usize| unit(sub(pts[(i + 1) % n].p, pts[i].p));
    for i in 0..segs {
        let (a, b) = (pts[i].p, pts[(i + 1) % n].p);
        let nn = mul(left(dir(i)), h);
        out.push(Piece::Poly(vec![
            add(a, nn),
            add(b, nn),
            sub(b, nn),
            sub(a, nn),
        ]));
    }
    let corners: Vec<usize> = if closed {
        (0..n).collect()
    } else {
        (1..n - 1).collect()
    };
    for i in corners {
        let d1 = dir((i + n - 1) % n);
        let d2 = dir(i);
        let j = if pts[i].sharp { join } else { LineJoin::Bevel };
        join_piece(pts[i].p, d1, d2, h, j, &mut out);
    }
    if !closed {
        cap_piece(pts[0].p, mul(dir(0), -1.0), h, cap, &mut out);
        cap_piece(pts[n - 1].p, dir(n - 2), h, cap, &mut out);
    }
    out
}

/// The dash intervals `[from, to]` of a path of length `total` (pattern in mm, odd doubled).
fn dash_intervals(total: f64, pattern_mm: &[f64]) -> Vec<(f64, f64)> {
    let mut pattern = pattern_mm.to_vec();
    if pattern.len() % 2 == 1 {
        pattern.extend_from_slice(pattern_mm);
    }
    let mut out = Vec::new();
    let mut at = 0.0;
    let mut k = 0;
    while at < total - 1e-9 {
        let l = pattern[k % pattern.len()];
        if k % 2 == 0 {
            out.push((at, (at + l).min(total)));
        }
        at += l;
        k += 1;
    }
    out
}

/// The sub-polyline of `vs` between arc lengths `a` and `b`.
fn cut(vs: &[V], a: f64, b: f64) -> Vec<V> {
    let mut out = Vec::new();
    let mut s = 0.0;
    for i in 0..vs.len() - 1 {
        let (p, q) = (vs[i].p, vs[i + 1].p);
        let l = len(sub(q, p));
        let (s0, s1) = (s, s + l);
        let at = |t: f64| add(p, mul(sub(q, p), (t - s0) / l.max(1e-300)));
        if s1 >= a && s0 <= b {
            if out.is_empty() {
                out.push(V {
                    p: at(a.max(s0)),
                    sharp: true,
                });
            }
            if s1 <= b {
                out.push(vs[i + 1]);
            } else {
                out.push(V {
                    p: at(b),
                    sharp: true,
                });
                break;
            }
        }
        s = s1;
    }
    out
}

fn polyline_length(vs: &[V]) -> f64 {
    vs.windows(2).map(|w| len(sub(w[1].p, w[0].p))).sum()
}

/// Dense flattening of anchors `(point, handle_in, handle_out)`.
fn flatten(anchors: &Anchors, closed: bool, steps: usize) -> Vec<V> {
    let n = anchors.len();
    let segs = if closed { n } else { n - 1 };
    let mut out = Vec::new();
    for i in 0..segs {
        let (a, b) = (anchors[i], anchors[(i + 1) % n]);
        let p0 = (a.0.x, a.0.y);
        let p1 = (a.0.x + a.2.x, a.0.y + a.2.y);
        let p3 = (b.0.x, b.0.y);
        let p2 = (b.0.x + b.1.x, b.0.y + b.1.y);
        let straight = a.2 == Vec2::ZERO && b.1 == Vec2::ZERO;
        let k = if straight { 1 } else { steps };
        for j in 0..k {
            let t = j as f64 / k as f64;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            out.push(V {
                p: (
                    w[0] * p0.0 + w[1] * p1.0 + w[2] * p2.0 + w[3] * p3.0,
                    w[0] * p0.1 + w[1] * p1.1 + w[2] * p2.1 + w[3] * p3.1,
                ),
                sharp: j == 0 && !smooth_anchor(&a),
            });
        }
    }
    if !closed {
        let last = anchors[n - 1];
        out.push(V {
            p: (last.0.x, last.0.y),
            sharp: true,
        });
    }
    out
}

fn smooth_anchor(a: &(Point, Vec2, Vec2)) -> bool {
    let (i, o) = (a.1, a.2);
    if i == Vec2::ZERO || o == Vec2::ZERO {
        return false;
    }
    let c = i.x * o.y - i.y * o.x;
    let d = i.x * o.x + i.y * o.y;
    d < 0.0 && (c / (i.x.hypot(i.y) * o.x.hypot(o.y))).abs() < 1e-3
}

/// `0018` criterion 4 and 15: an arrow of `w` centred on `at`, tip along `dir`.
fn arrow(at: P, dir: P, w: f64) -> Piece {
    let d = unit(dir);
    let n = left(d);
    let tip = add(at, mul(d, 2.0 * w));
    let base = sub(at, mul(d, 2.0 * w));
    Piece::Poly(vec![
        tip,
        add(base, mul(n, 1.5 * w)),
        sub(base, mul(n, 1.5 * w)),
    ])
}

fn dot(at: P, w: f64) -> Piece {
    Piece::Disk(at, 1.5 * w)
}

// ----- the result -----------------------------------------------------------------------------

fn result_polygons(result: &OffsetOutlines) -> Vec<Vec<P>> {
    result
        .outlines()
        .iter()
        .map(|o| {
            let n = o.len();
            let mut pts = Vec::new();
            for i in 0..n {
                let (a, b) = (o[i], o[(i + 1) % n]);
                let p0 = (a.point.x, a.point.y);
                let p1 = (p0.0 + a.handle_out.x, p0.1 + a.handle_out.y);
                let p3 = (b.point.x, b.point.y);
                let p2 = (p3.0 + b.handle_in.x, p3.1 + b.handle_in.y);
                let k = if a.handle_out == Vec2::ZERO && b.handle_in == Vec2::ZERO {
                    1
                } else {
                    64
                };
                for j in 0..k {
                    let t = f64::from(j) / f64::from(k);
                    let u = 1.0 - t;
                    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                    pts.push((
                        w[0] * p0.0 + w[1] * p1.0 + w[2] * p2.0 + w[3] * p3.0,
                        w[0] * p0.1 + w[1] * p1.1 + w[2] * p2.1 + w[3] * p3.1,
                    ));
                }
            }
            pts
        })
        .collect()
}

fn polygon_area(points: &[P]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| cross(points[i], points[(i + 1) % n]))
        .sum::<f64>()
        / 2.0
}

fn result_area(result: &OffsetOutlines) -> f64 {
    result_polygons(result)
        .iter()
        .map(|p| polygon_area(p))
        .sum()
}

/// The nonzero winding of the result at each sample of a row: the crossings of the row.
fn row_windings(polys: &[Vec<P>], y: f64) -> Vec<(f64, i32)> {
    let mut crossings = Vec::new();
    for poly in polys {
        let n = poly.len();
        for i in 0..n {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            if (a.1 <= y) != (b.1 <= y) {
                let x = a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0);
                crossings.push((x, if b.1 > a.1 { -1 } else { 1 }));
            }
        }
    }
    crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
    crossings
}

/// The pieces bucketed by 0.5 mm cells, so dense curves stay fast.
struct Index<'a> {
    pieces: &'a [Piece],
    lo: P,
    cols: usize,
    rows: usize,
    cells: Vec<Vec<usize>>,
}

const CELL_MM: f64 = 0.5;

impl<'a> Index<'a> {
    fn new(pieces: &'a [Piece], lo: P, hi: P) -> Self {
        let cols = ((hi.0 - lo.0) / CELL_MM).ceil() as usize + 1;
        let rows = ((hi.1 - lo.1) / CELL_MM).ceil() as usize + 1;
        let mut cells = vec![Vec::new(); cols * rows];
        let clamp = |v: f64, n: usize| (v.max(0.0) as usize).min(n - 1);
        for (k, piece) in pieces.iter().enumerate() {
            let (a, b) = piece.bounds();
            let (c0, c1) = (
                clamp((a.0 - lo.0) / CELL_MM, cols),
                clamp((b.0 - lo.0) / CELL_MM, cols),
            );
            let (r0, r1) = (
                clamp((a.1 - lo.1) / CELL_MM, rows),
                clamp((b.1 - lo.1) / CELL_MM, rows),
            );
            for r in r0..=r1 {
                for c in c0..=c1 {
                    cells[r * cols + c].push(k);
                }
            }
        }
        Self {
            pieces,
            lo,
            cols,
            rows,
            cells,
        }
    }

    fn contains(&self, p: P) -> bool {
        let c = (p.0 - self.lo.0) / CELL_MM;
        let r = (p.1 - self.lo.1) / CELL_MM;
        if c < 0.0 || r < 0.0 || c as usize >= self.cols || r as usize >= self.rows {
            return false;
        }
        self.cells[r as usize * self.cols + c as usize]
            .iter()
            .any(|&k| self.pieces[k].contains(p))
    }
}

struct Comparison {
    result_area: f64,
    reference_area: f64,
    far_mismatches: Vec<P>,
    bad_winding: Vec<(P, i32)>,
}

fn compare(result: &OffsetOutlines, pieces: &[Piece], step: f64) -> Comparison {
    let polys = result_polygons(result);
    let (mut lo, mut hi) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
    for piece in pieces {
        let (a, b) = piece.bounds();
        lo = (lo.0.min(a.0), lo.1.min(a.1));
        hi = (hi.0.max(b.0), hi.1.max(b.1));
    }
    for p in polys.iter().flatten() {
        lo = (lo.0.min(p.0), lo.1.min(p.1));
        hi = (hi.0.max(p.0), hi.1.max(p.1));
    }
    let (lo, hi) = ((lo.0 - 0.5, lo.1 - 0.5), (hi.0 + 0.5, hi.1 + 0.5));
    let index = Index::new(pieces, lo, hi);
    let inside = |p: P| index.contains(p);
    let mut reference_count = 0usize;
    let mut far_mismatches = Vec::new();
    let mut bad_winding = Vec::new();
    let rows = ((hi.1 - lo.1) / step).ceil() as usize;
    let cols = ((hi.0 - lo.0) / step).ceil() as usize;
    for r in 0..rows {
        let y = lo.1 + (r as f64 + 0.37) * step;
        let crossings = row_windings(&polys, y);
        let mut k = 0;
        let mut winding = 0;
        for c in 0..cols {
            let x = lo.0 + (c as f64 + 0.53) * step;
            while k < crossings.len() && crossings[k].0 <= x {
                winding += crossings[k].1;
                k += 1;
            }
            let ours = winding != 0;
            if !(0..=1).contains(&winding) && bad_winding.len() < 5 {
                bad_winding.push(((x, y), winding));
            }
            let theirs = inside((x, y));
            reference_count += usize::from(theirs);
            if ours != theirs {
                let near_edge = (0..16).any(|i| {
                    let a = TAU * f64::from(i) / 16.0;
                    inside((x + NEAR_EDGE_MM * a.cos(), y + NEAR_EDGE_MM * a.sin())) != theirs
                });
                if !near_edge && far_mismatches.len() < 10 {
                    far_mismatches.push((x, y));
                }
            }
        }
    }
    Comparison {
        result_area: result_area(result),
        reference_area: reference_count as f64 * step * step,
        far_mismatches,
        bad_winding,
    }
}

/// Asserts the result is the reference region (samples, winding) and its area is within
/// `area_tol` of the reference's sampled area.
fn assert_matches(name: &str, result: &OffsetOutlines, pieces: &[Piece], step: f64, tol: f64) {
    let c = compare(result, pieces, step);
    assert!(
        c.far_mismatches.is_empty(),
        "{name}: result and reference differ away from the edge at {:?}",
        c.far_mismatches
    );
    assert!(
        c.bad_winding.is_empty(),
        "{name}: winding not 0 or 1 (overlapping pieces) at {:?}",
        c.bad_winding
    );
    assert!(
        (c.result_area - c.reference_area).abs() <= tol,
        "{name}: area {} vs reference {} (tol {tol})",
        c.result_area,
        c.reference_area
    );
}

// ----- running the kernel ---------------------------------------------------------------------

fn shape(w: f64, join: LineJoin, cap: LineCap) -> StrokeShape {
    StrokeShape {
        width: Length::from_mm(w),
        join,
        cap,
        dash: DashPattern::solid(),
        markers: Markers::default(),
    }
}

fn dashed(mut s: StrokeShape, pattern: &[f64]) -> StrokeShape {
    s.dash = DashPattern::new(pattern.to_vec()).unwrap();
    s
}

fn try_stroke(
    outlines: &[(Anchors, bool)],
    s: &StrokeShape,
) -> Result<OffsetOutlines, StrokeOutlineError> {
    let borrowed: Vec<Outline<'_>> = outlines.iter().map(|(a, c)| Outline::new(a, *c)).collect();
    stroke_outline(&borrowed, s, KERNEL_TOLERANCE)
}

fn stroke(outlines: &[(Anchors, bool)], s: &StrokeShape) -> OffsetOutlines {
    try_stroke(outlines, s).expect("a stroke outline")
}

fn open(points: &[P]) -> (Anchors, bool) {
    (polygon(points), false)
}

fn closed(points: &[P]) -> (Anchors, bool) {
    (polygon(points), true)
}

const JOINS: [LineJoin; 3] = [LineJoin::Miter, LineJoin::Round, LineJoin::Bevel];
const CAPS: [LineCap; 3] = [LineCap::Butt, LineCap::Square, LineCap::Round];

// ----- criterion 4: caps ----------------------------------------------------------------------

/// Criterion 4: the three caps on a straight line, and on a diagonal line (no axis luck),
/// region and exact area.
#[test]
fn t04_caps_on_straight_and_diagonal_lines_match_the_reference() {
    for cap in CAPS {
        let exact = match cap {
            LineCap::Butt => 200.0,
            LineCap::Square => 204.0,
            LineCap::Round => 200.0 + PI,
        };
        let r = stroke(
            &[open(&[(0.0, 0.0), (100.0, 0.0)])],
            &shape(2.0, LineJoin::Miter, cap),
        );
        assert!(
            (result_area(&r) - exact).abs() < 0.01,
            "{cap:?}: {}",
            result_area(&r)
        );
        let refp = stroke_pieces(
            &sharp(&[(0.0, 0.0), (100.0, 0.0)]),
            false,
            2.0,
            LineJoin::Miter,
            cap,
        );
        assert_matches(&format!("line {cap:?}"), &r, &refp, 0.02, 0.1);

        let (a, b) = ((3.0, 7.0), (3.0 + 60.0 * 0.6, 7.0 + 60.0 * 0.8));
        let r = stroke(&[open(&[a, b])], &shape(1.5, LineJoin::Miter, cap));
        let exact = 60.0 * 1.5
            + match cap {
                LineCap::Butt => 0.0,
                LineCap::Square => 1.5 * 1.5,
                LineCap::Round => PI * 0.75 * 0.75,
            };
        assert!((result_area(&r) - exact).abs() < 0.01, "diagonal {cap:?}");
        let refp = stroke_pieces(&sharp(&[a, b]), false, 1.5, LineJoin::Miter, cap);
        assert_matches(&format!("diagonal {cap:?}"), &r, &refp, 0.02, 0.1);
    }
}

// ----- criteria 5 and 6: joins, miter limit ---------------------------------------------------

/// Criterion 5: the closed square with each join; analytic areas (bevel cuts a 1 x 1 / 2
/// triangle at each outer corner, round a 1 - pi/4 sliver) and the hole stays.
#[test]
fn t05_closed_square_each_join_area_and_hole() {
    let sq = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)];
    for join in JOINS {
        let r = stroke(&[closed(&sq)], &shape(2.0, join, LineCap::Butt));
        let exact = match join {
            LineJoin::Miter => 160.0,
            LineJoin::Bevel => 160.0 - 4.0 * 0.5,
            LineJoin::Round => 160.0 - 4.0 * (1.0 - PI / 4.0),
        };
        assert!(
            (result_area(&r) - exact).abs() < 0.01,
            "{join:?}: {} vs {exact}",
            result_area(&r)
        );
        assert_eq!(r.outlines().len(), 2, "{join:?}: outer and hole");
        let areas: Vec<f64> = result_polygons(&r)
            .iter()
            .map(|p| polygon_area(p))
            .collect();
        assert!(
            areas.iter().any(|&a| a > 0.0) && areas.iter().any(|&a| a < 0.0),
            "{areas:?}"
        );
        let refp = stroke_pieces(&sharp(&sq), true, 2.0, join, LineCap::Butt);
        assert_matches(&format!("square {join:?}"), &r, &refp, 0.02, 0.1);
    }
}

/// Criterion 6 at the limit: miter limit 4 means a corner of interior angle above
/// 2 asin(1/4) = 28.955 degrees is mitred, below it bevelled. 30 and 28 degrees, each join.
#[test]
fn t06_miter_limit_four_just_inside_and_just_outside() {
    for (degrees, mitred) in [
        (30.0_f64, true),
        (28.0_f64, false),
        (29.5, true),
        (28.5, false),
    ] {
        let a = degrees.to_radians();
        let pts = [
            (0.0, 0.0),
            (40.0, 0.0),
            (40.0 - 40.0 * a.cos(), 40.0 * a.sin()),
        ];
        for join in JOINS {
            let r = stroke(&[open(&pts)], &shape(2.0, join, LineCap::Butt));
            let refp = stroke_pieces(&sharp(&pts), false, 2.0, join, LineCap::Butt);
            assert_matches(&format!("{degrees} deg {join:?}"), &r, &refp, 0.01, 0.05);
            if join == LineJoin::Miter {
                // The miter tip lies (w/2) / sin(theta/2) from the corner, beyond x = 40.
                let tip = 1.0 / (a / 2.0).sin();
                let reach = result_polygons(&r)
                    .iter()
                    .flatten()
                    // The outer side of the corner (the inner corner lies as far on the other).
                    .filter(|p| p.0 > 40.0 + 1e-9)
                    .map(|p| len(sub(*p, (40.0, 0.0))))
                    .filter(|&d| d < 6.0)
                    .fold(0.0, f64::max);
                if mitred {
                    assert!(
                        (reach - tip).abs() < 0.01,
                        "{degrees}: reach {reach} tip {tip}"
                    );
                } else {
                    assert!(reach < 1.01, "{degrees}: bevel expected, reach {reach}");
                }
            }
        }
    }
}

/// Criteria 5 and 6 on an open zigzag with obtuse, right and acute corners, each join and cap.
#[test]
fn t06_zigzag_every_join_and_cap() {
    let pts = [
        (0.0, 0.0),
        (15.0, 0.0),
        (25.0, 12.0),
        (32.0, 0.0),
        (45.0, 3.0),
        (45.0, 20.0),
        (20.0, 22.0),
    ];
    for join in JOINS {
        for cap in CAPS {
            let r = stroke(&[open(&pts)], &shape(2.5, join, cap));
            let refp = stroke_pieces(&sharp(&pts), false, 2.5, join, cap);
            assert_matches(&format!("zigzag {join:?} {cap:?}"), &r, &refp, 0.025, 0.2);
        }
    }
}

// ----- criterion 7: curves --------------------------------------------------------------------

/// Criterion 7 plus 12: a circle of radius 10 gives two outlines whose every sampled point is
/// within 0.01 mm of radius 11 and 9, at most 8 nodes each, all Smooth, no straight segments.
#[test]
fn t07_circle_two_exact_circles_of_smooth_curves() {
    let r = stroke(
        &[(circle(5.0, -3.0, 10.0), true)],
        &shape(2.0, LineJoin::Miter, LineCap::Butt),
    );
    assert_eq!(r.outlines().len(), 2);
    for (outline, poly) in r.outlines().iter().zip(result_polygons(&r)) {
        assert!(outline.len() <= 8, "{} nodes", outline.len());
        assert!(outline.iter().all(|a| a.smooth), "all nodes smooth");
        assert!(
            outline
                .iter()
                .all(|a| a.handle_in != Vec2::ZERO && a.handle_out != Vec2::ZERO),
            "curves, no polyline"
        );
        let radii: Vec<f64> = poly.iter().map(|p| len(sub(*p, (5.0, -3.0)))).collect();
        let target = if radii[0] > 10.0 { 11.0 } else { 9.0 };
        let worst = radii.iter().map(|q| (q - target).abs()).fold(0.0, f64::max);
        assert!(worst <= 0.01, "radius {target}: off by {worst}");
    }
}

/// A 12-node Bezier path with sharp and smooth nodes (criterion 11's test shape), each join and
/// cap: the region matches the densely flattened reference, the output is made of curves with
/// a bounded node count, and criterion 12 holds (no near-duplicate nodes, no collinear corner
/// node between two lines, smooth flags where tangents agree).
fn bezier12() -> Anchors {
    let c = |x: f64, y: f64| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO);
    let s = |x: f64, y: f64, hx: f64, hy: f64| {
        (Point::new(x, y), Vec2::new(-hx, -hy), Vec2::new(hx, hy))
    };
    let a = |x: f64, y: f64, ix: f64, iy: f64, ox: f64, oy: f64| {
        (Point::new(x, y), Vec2::new(ix, iy), Vec2::new(ox, oy))
    };
    vec![
        c(0.0, 0.0),
        s(10.0, 6.0, 4.0, 0.0),
        a(20.0, 0.0, -3.0, 2.0, 2.0, 5.0),
        s(26.0, 12.0, 0.0, 4.0),
        c(18.0, 20.0),
        s(8.0, 22.0, -3.0, 1.0),
        a(0.0, 30.0, 2.0, -3.0, 6.0, 0.0),
        s(14.0, 34.0, 4.0, 1.0),
        c(30.0, 30.0),
        s(38.0, 20.0, 1.0, -4.0),
        a(40.0, 6.0, -2.0, 3.0, 4.0, 1.0),
        c(50.0, 10.0),
    ]
}

#[test]
fn t11_t12_bezier_path_every_join_and_cap_is_clean_curves() {
    let path = bezier12();
    let flat = flatten(&path, false, 200);
    for join in JOINS {
        for cap in CAPS {
            let r = stroke(&[(path.clone(), false)], &shape(2.0, join, cap));
            let refp = stroke_pieces(&flat, false, 2.0, join, cap);
            assert_matches(&format!("bezier {join:?} {cap:?}"), &r, &refp, 0.04, 0.4);
            let nodes: usize = r.outlines().iter().map(Vec::len).sum();
            let curves = r
                .outlines()
                .iter()
                .flat_map(|o| {
                    (0..o.len()).filter(move |&i| {
                        o[i].handle_out != Vec2::ZERO
                            || o[(i + 1) % o.len()].handle_in != Vec2::ZERO
                    })
                })
                .count();
            // 11 input segments, two sides each, plus joins and caps: well under 150 nodes, and
            // most segments are curves (no polyline fallback).
            assert!(nodes <= 150, "{join:?} {cap:?}: {nodes} nodes");
            assert!(
                curves * 2 >= nodes,
                "{join:?} {cap:?}: {curves} curves of {nodes}"
            );
            criterion_12(&format!("bezier {join:?} {cap:?}"), &r);
        }
    }
}

/// Criterion 12: no near-duplicate consecutive nodes, no corner node between two collinear
/// straight segments, smooth exactly where the tangents agree within 0.1 degree.
fn criterion_12(name: &str, r: &OffsetOutlines) {
    for o in r.outlines() {
        let n = o.len();
        for i in 0..n {
            let (prev, a, next) = (o[(i + n - 1) % n], o[i], o[(i + 1) % n]);
            let gap = len(sub((next.point.x, next.point.y), (a.point.x, a.point.y)));
            assert!(gap > 0.001, "{name}: near-duplicate nodes at {:?}", a.point);
            let t_in = if a.handle_in == Vec2::ZERO {
                sub(
                    (a.point.x, a.point.y),
                    (
                        prev.point.x + prev.handle_out.x,
                        prev.point.y + prev.handle_out.y,
                    ),
                )
            } else {
                (-a.handle_in.x, -a.handle_in.y)
            };
            let t_out = if a.handle_out == Vec2::ZERO {
                sub(
                    (
                        next.point.x + next.handle_in.x,
                        next.point.y + next.handle_in.y,
                    ),
                    (a.point.x, a.point.y),
                )
            } else {
                (a.handle_out.x, a.handle_out.y)
            };
            let angle = cross(unit(t_in), unit(t_out))
                .atan2(unit(t_in).0 * unit(t_out).0 + unit(t_in).1 * unit(t_out).1);
            let agree = angle.abs().to_degrees() < 0.1;
            let lines = a.handle_in == Vec2::ZERO
                && a.handle_out == Vec2::ZERO
                && prev.handle_out == Vec2::ZERO
                && next.handle_in == Vec2::ZERO;
            if lines {
                assert!(!agree, "{name}: collinear node at {:?}", a.point);
            } else {
                assert_eq!(
                    a.smooth, agree,
                    "{name}: smooth flag at {:?} (angle {angle})",
                    a.point
                );
            }
        }
    }
}

// ----- criterion 8: dashes --------------------------------------------------------------------

/// Criterion 8: `5 5` on a 100 mm line, width 1, each cap: 10 pieces; with Square the 6 mm
/// pieces still stand apart; region and area against the reference.
#[test]
fn t08_dashes_each_cap_match_the_reference() {
    let line = sharp(&[(0.0, 0.0), (100.0, 0.0)]);
    for cap in CAPS {
        let r = stroke(
            &[open(&[(0.0, 0.0), (100.0, 0.0)])],
            &dashed(shape(1.0, LineJoin::Miter, cap), &[5.0, 5.0]),
        );
        assert_eq!(r.outlines().len(), 10, "{cap:?}");
        let mut refp = Vec::new();
        for (a, b) in dash_intervals(100.0, &[5.0, 5.0]) {
            refp.extend(stroke_pieces(
                &cut(&line, a, b),
                false,
                1.0,
                LineJoin::Miter,
                cap,
            ));
        }
        let exact = 10.0
            * (5.0
                + match cap {
                    LineCap::Butt => 0.0,
                    LineCap::Square => 1.0,
                    LineCap::Round => PI / 4.0,
                });
        assert!(
            (result_area(&r) - exact).abs() < 0.02,
            "{cap:?}: {}",
            result_area(&r)
        );
        assert_matches(&format!("dash {cap:?}"), &r, &refp, 0.02, 0.1);
    }
}

/// Criterion 8 and `0017` criterion 29: the pattern is in multiples of the width (width 2,
/// `2 1` is 4 mm on, 2 mm off) and an odd pattern repeats (`3` is 3 on, 3 off, as SVG).
#[test]
fn t08_dash_pattern_scales_with_width_and_odd_patterns_repeat() {
    let r = stroke(
        &[open(&[(0.0, 0.0), (60.0, 0.0)])],
        &dashed(shape(2.0, LineJoin::Miter, LineCap::Butt), &[2.0, 1.0]),
    );
    assert_eq!(r.outlines().len(), 10, "60 / 6 dashes of 4 mm");
    assert!((result_area(&r) - 10.0 * 4.0 * 2.0).abs() < 0.02);

    let r = stroke(
        &[open(&[(0.0, 0.0), (60.0, 0.0)])],
        &dashed(shape(1.0, LineJoin::Miter, LineCap::Butt), &[3.0]),
    );
    assert_eq!(r.outlines().len(), 10, "3 on, 3 off");
    assert!((result_area(&r) - 30.0).abs() < 0.02, "{}", result_area(&r));
}

/// Criterion 8: zero-length dashes with Round cap are dots (as the canvas draws them); with
/// Butt they paint nothing.
#[test]
fn t08_zero_length_dashes_are_dots_with_round_cap() {
    let r = stroke(
        &[open(&[(0.0, 0.0), (40.0, 0.0)])],
        &dashed(shape(1.0, LineJoin::Round, LineCap::Round), &[0.0, 4.0]),
    );
    // Dots at 0, 4, ..., 36 (and maybe 40, the canvas parity test decides): disks of r 0.5.
    let dots = r.outlines().len();
    assert!((10..=11).contains(&dots), "{dots}");
    assert!(
        (result_area(&r) - dots as f64 * PI * 0.25).abs() < 0.02,
        "{}",
        result_area(&r)
    );
    let butt = try_stroke(
        &[open(&[(0.0, 0.0), (40.0, 0.0)])],
        &dashed(shape(1.0, LineJoin::Round, LineCap::Butt), &[0.0, 4.0]),
    );
    assert_eq!(butt.err(), Some(StrokeOutlineError::Empty));
}

/// Criterion 8: dashes running around corners keep the join and are one piece each; a dashed
/// zigzag against the reference, each join.
#[test]
fn t08_dashes_around_corners_keep_the_join() {
    let pts = [
        (0.0, 0.0),
        (12.0, 0.0),
        (12.0, 9.0),
        (25.0, 2.0),
        (40.0, 14.0),
    ];
    let line = sharp(&pts);
    let total = polyline_length(&line);
    for join in JOINS {
        let r = stroke(
            &[open(&pts)],
            &dashed(shape(1.0, join, LineCap::Butt), &[7.0, 2.0]),
        );
        let mut refp = Vec::new();
        let intervals = dash_intervals(total, &[7.0, 2.0]);
        for &(a, b) in &intervals {
            if b > a {
                refp.extend(stroke_pieces(
                    &cut(&line, a, b),
                    false,
                    1.0,
                    join,
                    LineCap::Butt,
                ));
            }
        }
        assert_matches(&format!("dashed zigzag {join:?}"), &r, &refp, 0.01, 0.05);
    }
}

/// Criterion 8 on a curve: dashes on a circle follow the arc length.
#[test]
fn t08_dashes_on_a_circle_follow_arc_length() {
    let c = circle(0.0, 0.0, 10.0);
    let flat = flatten(&c, true, 600);
    let mut ring = flat.clone();
    ring.push(flat[0]);
    let total = polyline_length(&ring);
    let r = stroke(
        &[(c, true)],
        &dashed(shape(1.0, LineJoin::Miter, LineCap::Butt), &[3.0, 2.0]),
    );
    let mut refp = Vec::new();
    for (a, b) in dash_intervals(total, &[3.0, 2.0]) {
        if b > a {
            refp.extend(stroke_pieces(
                &cut(&ring, a, b),
                false,
                1.0,
                LineJoin::Miter,
                LineCap::Butt,
            ));
        }
    }
    // The circle is 62.83 mm: 12 full dashes and a 2.83 mm piece that may meet the first dash at
    // the start of the closed outline (one region then).
    assert!(
        (12..=13).contains(&r.outlines().len()),
        "{} outlines",
        r.outlines().len()
    );
    assert_matches("dashed circle", &r, &refp, 0.01, 0.1);
}

// ----- criterion 9: self-crossing -------------------------------------------------------------

/// Criterion 9: a figure eight (open, crossing itself) and a closed pentagram, each join and
/// cap: the union, winding 0 or 1 everywhere, the pentagram's centre is a hole of the stroke.
#[test]
fn t09_self_crossing_strokes_are_one_union() {
    let eight = [(0.0, 0.0), (20.0, 20.0), (20.0, 0.0), (0.0, 20.0)];
    let star: Vec<P> = (0..5)
        .map(|i| {
            let a = TAU * f64::from(i * 2) / 5.0 - PI / 2.0;
            (20.0 * a.cos(), 20.0 * a.sin())
        })
        .collect();
    for join in JOINS {
        for cap in CAPS {
            let r = stroke(&[open(&eight)], &shape(3.0, join, cap));
            let refp = stroke_pieces(&sharp(&eight), false, 3.0, join, cap);
            assert_matches(&format!("eight {join:?} {cap:?}"), &r, &refp, 0.02, 0.2);
        }
        let r = stroke(&[closed(&star)], &shape(3.0, join, LineCap::Butt));
        let refp = stroke_pieces(&sharp(&star), true, 3.0, join, LineCap::Butt);
        assert_matches(&format!("pentagram {join:?}"), &r, &refp, 0.02, 0.3);
        let holes = result_polygons(&r)
            .iter()
            .filter(|p| polygon_area(p) < 0.0)
            .count();
        assert_eq!(
            holes, 6,
            "pentagram {join:?}: centre pentagon and 5 tips' inner triangles"
        );
    }
}

/// Criterion 9: a path that runs back over itself (180 degree turn) counts once.
#[test]
fn t09_a_path_running_back_over_itself_counts_once() {
    let pts = [(0.0, 0.0), (30.0, 0.0), (10.0, 0.0)];
    for join in JOINS {
        let r = stroke(&[open(&pts)], &shape(2.0, join, LineCap::Butt));
        let refp = stroke_pieces(&sharp(&pts), false, 2.0, join, LineCap::Butt);
        assert_matches(&format!("back {join:?}"), &r, &refp, 0.02, 0.1);
        assert_eq!(r.outlines().len(), 1);
    }
}

// ----- criterion 10: markers ------------------------------------------------------------------

/// Criterion 10 with `0018` criteria 4 and 15: Start Dot and End Arrow on (0, 0)-(100, 0), w 1:
/// the dot (r 1.5 at 0, 0) and the arrow (tip 102, base 98, 3 wide) join the line in one region.
#[test]
fn t10_start_dot_and_end_arrow_join_the_line() {
    let mut s = shape(1.0, LineJoin::Miter, LineCap::Butt);
    s.markers = Markers {
        start: MarkerShape::Dot,
        end: MarkerShape::Arrow,
        ..Markers::default()
    };
    let r = stroke(&[open(&[(0.0, 0.0), (100.0, 0.0)])], &s);
    assert_eq!(r.outlines().len(), 1, "one region, no inner line");
    let mut refp = stroke_pieces(
        &sharp(&[(0.0, 0.0), (100.0, 0.0)]),
        false,
        1.0,
        LineJoin::Miter,
        LineCap::Butt,
    );
    refp.push(dot((0.0, 0.0), 1.0));
    refp.push(arrow((100.0, 0.0), (1.0, 0.0), 1.0));
    assert_matches("dot and arrow", &r, &refp, 0.01, 0.05);
    let xs: Vec<f64> = result_polygons(&r).iter().flatten().map(|p| p.0).collect();
    let max_x = xs.iter().copied().fold(f64::MIN, f64::max);
    let min_x = xs.iter().copied().fold(f64::MAX, f64::min);
    assert!(
        (max_x - 102.0).abs() < 0.01 && (min_x + 1.5).abs() < 0.01,
        "{min_x} {max_x}"
    );
}

/// Criterion 10: a Start arrow points outward (`0018` criterion 15), Middle arrows at nodes
/// point along the bisector (criterion 16).
#[test]
fn t10_start_arrow_outward_and_mid_arrows_at_nodes() {
    let pts = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)];
    let mut s = shape(0.5, LineJoin::Miter, LineCap::Butt);
    s.markers = Markers {
        start: MarkerShape::Arrow,
        mid: MarkerShape::Arrow,
        mid_place: MarkerPlace::AtNodes,
        ..Markers::default()
    };
    let r = stroke(&[open(&pts)], &s);
    let mut refp = stroke_pieces(&sharp(&pts), false, 0.5, LineJoin::Miter, LineCap::Butt);
    refp.push(arrow((0.0, 0.0), (-1.0, 0.0), 0.5));
    refp.push(arrow((10.0, 0.0), (1.0, 1.0), 0.5));
    assert_matches("start and mid arrows", &r, &refp, 0.01, 0.05);
    let min_x = result_polygons(&r)
        .iter()
        .flatten()
        .map(|p| p.0)
        .fold(f64::MAX, f64::min);
    assert!((min_x + 1.0).abs() < 0.01, "start arrow tip at -1: {min_x}");
}

/// Criterion 10: a marker that does not touch the stroke is its own outline (a Middle dot on a
/// node that lies in a dash gap).
#[test]
fn t10_a_marker_in_a_dash_gap_is_a_separate_outline() {
    let pts = [(0.0, 0.0), (15.0, 0.0), (100.0, 0.0)];
    let mut s = dashed(shape(1.0, LineJoin::Miter, LineCap::Butt), &[10.0, 10.0]);
    s.markers = Markers {
        mid: MarkerShape::Dot,
        mid_place: MarkerPlace::AtNodes,
        ..Markers::default()
    };
    let r = stroke(&[open(&pts)], &s);
    // Dashes on [0,10], [20,30], ..., [80,90]: 5, plus the dot on (15, 0).
    assert_eq!(r.outlines().len(), 6);
    let line = sharp(&pts);
    let mut refp = Vec::new();
    for (a, b) in dash_intervals(100.0, &[10.0, 10.0]) {
        if b > a {
            refp.extend(stroke_pieces(
                &cut(&line, a, b),
                false,
                1.0,
                LineJoin::Miter,
                LineCap::Butt,
            ));
        }
    }
    refp.push(dot((15.0, 0.0), 1.0));
    assert_matches("dot in a gap", &r, &refp, 0.01, 0.05);
}

/// Criterion 10: spaced Middle markers spread evenly by arc length (`0018`): 3 dots on a 100 mm
/// line. Spaced positions are taken as `0018` defines them: equal steps of length / (n + 1).
#[test]
fn t10_spaced_mid_dots_are_spread_evenly() {
    let mut s = shape(1.0, LineJoin::Miter, LineCap::Butt);
    s.markers = Markers {
        mid: MarkerShape::Dot,
        mid_place: MarkerPlace::Spaced,
        mid_count: curvyo_document_core::MarkerCount::new(3).unwrap(),
        ..Markers::default()
    };
    let r = stroke(&[open(&[(0.0, 0.0), (100.0, 0.0)])], &s);
    assert_eq!(r.outlines().len(), 1);
    // Each dot adds pi 1.5^2 minus its overlap with the 1 mm band.
    let band_overlap = {
        // integral over |y| <= 0.5 of 2 sqrt(2.25 - y^2)
        let f = |y: f64| y * (2.25 - y * y).sqrt() + 2.25 * (y / 1.5).asin();
        f(0.5) - f(-0.5)
    };
    let expected = 100.0 + 3.0 * (PI * 2.25 - band_overlap);
    assert!(
        (result_area(&r) - expected).abs() < 0.02,
        "{} vs {expected}",
        result_area(&r)
    );
}

// ----- criterion 13: compound paths -----------------------------------------------------------

/// Criterion 13: an open and a closed outline that cross: every outline is stroked, the open one
/// with caps, and the areas form one union.
#[test]
fn t13_compound_open_and_closed_outlines_union() {
    let sq = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)];
    let ln = [(-5.0, 10.0), (25.0, 12.0)];
    for cap in CAPS {
        let r = stroke(&[closed(&sq), open(&ln)], &shape(2.0, LineJoin::Round, cap));
        let mut refp = stroke_pieces(&sharp(&sq), true, 2.0, LineJoin::Round, cap);
        refp.extend(stroke_pieces(&sharp(&ln), false, 2.0, LineJoin::Round, cap));
        assert_matches(&format!("compound {cap:?}"), &r, &refp, 0.02, 0.2);
        // Two holes: the square's inside is split in two by the line.
        let holes = result_polygons(&r)
            .iter()
            .filter(|p| polygon_area(p) < 0.0)
            .count();
        assert_eq!(holes, 2, "{cap:?}");
    }
}

// ----- criteria 26, 27: degenerate input ------------------------------------------------------

/// Criterion 27: one node with Butt vanishes; zero-length segments and repeated nodes do not
/// break the stroke; width at the 0.002 mm grid edge.
#[test]
fn t27_degenerate_paths() {
    let one = try_stroke(
        &[open(&[(5.0, 5.0)])],
        &shape(2.0, LineJoin::Miter, LineCap::Butt),
    );
    assert_eq!(one.err(), Some(StrokeOutlineError::Empty));
    let two_same = try_stroke(
        &[open(&[(5.0, 5.0), (5.0, 5.0)])],
        &shape(2.0, LineJoin::Miter, LineCap::Butt),
    );
    assert_eq!(two_same.err(), Some(StrokeOutlineError::Empty));
    // Repeated middle nodes: the same as without them.
    let pts = [
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
    ];
    for join in JOINS {
        let r = stroke(&[open(&pts)], &shape(2.0, join, LineCap::Square));
        let refp = stroke_pieces(&sharp(&pts), false, 2.0, join, LineCap::Square);
        assert_matches(&format!("repeated nodes {join:?}"), &r, &refp, 0.02, 0.1);
    }
    // A closed outline of two nodes is a there-and-back line.
    let r = stroke(
        &[closed(&[(0.0, 0.0), (10.0, 0.0)])],
        &shape(1.0, LineJoin::Bevel, LineCap::Butt),
    );
    assert!((result_area(&r) - 10.0).abs() < 0.02, "{}", result_area(&r));
    // A tiny width: no panic, area or Empty.
    let tiny = try_stroke(
        &[open(&[(0.0, 0.0), (10.0, 0.0)])],
        &shape(0.0005, LineJoin::Miter, LineCap::Butt),
    );
    assert!(
        matches!(tiny, Err(StrokeOutlineError::Empty) | Ok(_)),
        "{tiny:?}"
    );
}

/// Criterion 26 at the kernel: non-finite coordinates, handles and widths are refused; a
/// tolerance below the grid is refused.
#[test]
fn t26_non_finite_input_is_refused() {
    let nan = vec![
        (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::new(f64::NAN, 0.0)),
        (Point::new(10.0, 0.0), Vec2::ZERO, Vec2::ZERO),
    ];
    assert_eq!(
        try_stroke(&[(nan, false)], &shape(1.0, LineJoin::Miter, LineCap::Butt)).err(),
        Some(StrokeOutlineError::OutOfRange)
    );
    for w in [f64::NAN, f64::INFINITY, 2.0e7] {
        assert_eq!(
            try_stroke(
                &[open(&[(0.0, 0.0), (10.0, 0.0)])],
                &shape(w, LineJoin::Miter, LineCap::Butt)
            )
            .err(),
            Some(StrokeOutlineError::OutOfRange),
            "{w}"
        );
    }
    let borrowed_anchors = polygon(&[(0.0, 0.0), (10.0, 0.0)]);
    let borrowed = [Outline::new(&borrowed_anchors, false)];
    assert_eq!(
        stroke_outline(
            &borrowed,
            &shape(1.0, LineJoin::Miter, LineCap::Butt),
            Tolerance::from_mm(0.001)
        )
        .err(),
        Some(StrokeOutlineError::ToleranceTooSmall)
    );
}

// ----- criterion 28: determinism --------------------------------------------------------------

/// Criterion 28: the same input gives the same result node for node, and the order of the
/// outlines of a compound path does not change it.
#[test]
fn t28_outline_order_does_not_change_the_result() {
    let sq = (
        polygon(&[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]),
        true,
    );
    let ln = (polygon(&[(-5.0, 10.0), (25.0, 12.0)]), false);
    let c = (circle(30.0, 10.0, 6.0), true);
    let s = dashed(shape(1.0, LineJoin::Round, LineCap::Round), &[4.0, 2.0]);
    let a = stroke(&[sq.clone(), ln.clone(), c.clone()], &s);
    let b = stroke(&[c, ln, sq], &s);
    assert_eq!(
        stroke(&[(bezier12(), false)], &s),
        stroke(&[(bezier12(), false)], &s)
    );
    // Dashes start at each outline's own start, so the order of outlines must not matter.
    assert_eq!(a.outlines().len(), b.outlines().len());
    for (x, y) in a.outlines().iter().zip(b.outlines()) {
        assert_eq!(x.len(), y.len());
        for (p, q) in x.iter().zip(y) {
            assert!(len(sub((p.point.x, p.point.y), (q.point.x, q.point.y))) < 1e-6);
        }
    }
}

// ----- property test --------------------------------------------------------------------------

fn join_of(i: u8) -> LineJoin {
    JOINS[usize::from(i % 3)]
}

fn cap_of(i: u8) -> LineCap {
    CAPS[usize::from(i % 3)]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 24, ..ProptestConfig::default() })]

    /// Any polyline of 2 to 6 nodes (segments of at least 1 mm), open or closed, each join and
    /// cap, width 0.3 to 4 mm: the result is the reference region, winding 0 or 1, and its area
    /// at least the band of the longest segment (area of A union B >= max of the parts).
    #[test]
    fn prop_any_polyline_matches_the_reference(
        raw in prop::collection::vec((0.0f64..40.0, 0.0f64..40.0), 2..7),
        is_closed in any::<bool>(),
        w in 0.3f64..4.0,
        j in any::<u8>(),
        c in any::<u8>(),
    ) {
        let pts: Vec<P> = raw.iter().map(|&(x, y)| ((x * 1000.0).round() / 1000.0, (y * 1000.0).round() / 1000.0)).collect();
        let n = pts.len();
        let segs = if is_closed && n > 2 { n } else { n - 1 };
        for i in 0..segs {
            prop_assume!(len(sub(pts[(i + 1) % n], pts[i])) >= 1.0);
        }
        let is_closed = is_closed && n > 2;
        let (join, cap) = (join_of(j), cap_of(c));
        let r = stroke(&[(polygon(&pts), is_closed)], &shape(w, join, cap));
        let refp = stroke_pieces(&sharp(&pts), is_closed, w, join, cap);
        let cmp = compare(&r, &refp, 0.04);
        prop_assert!(cmp.far_mismatches.is_empty(), "{:?}", cmp.far_mismatches);
        prop_assert!(cmp.bad_winding.is_empty(), "{:?}", cmp.bad_winding);
        let longest = (0..segs).map(|i| len(sub(pts[(i + 1) % n], pts[i]))).fold(0.0, f64::max);
        prop_assert!(cmp.result_area >= longest * w - 0.01);
        let perimeter: f64 = (0..segs).map(|i| len(sub(pts[(i + 1) % n], pts[i]))).sum::<f64>() * 2.0;
        prop_assert!((cmp.result_area - cmp.reference_area).abs() <= 0.02 * perimeter + 0.1,
            "area {} vs {}", cmp.result_area, cmp.reference_area);
    }
}
