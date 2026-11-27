//! Cubic Béziers through one corner-free run of points within an accuracy (ADR 0095 decision 5,
//! moved unchanged out of `centre_line::fit`): the piece fit traced and drawn centre lines share.

mod safe_fit;

use curvyo_document_core::{Point as DocPoint, Vec2 as DocVec};
use kurbo::{BezPath, PathEl, Point};

pub(crate) use safe_fit::fit_to_cubic;
use safe_fit::{STRAY, simplify, strays};

use crate::OutlineTriple;

/// A turn between two steps of a piece whose cosine is below this (120 degrees) cuts the piece.
const HAIRPIN_COS: f64 = -0.5;
/// Points of a piece closer than this are one point, in millimetres.
const COINCIDENT_MM: f64 = 1e-6;

/// Cubic Béziers through one corner-free piece. The simplifier runs in coordinates local to
/// the piece's first point (its moment integrals cancel catastrophically for a short piece far
/// from the origin) and behind [`safe_fit`] (its unbounded candidates overflow or stall
/// kurbo 0.13.1's root solver). A piece of fewer than three distinct points, or one within the
/// accuracy of its chord, is a line; a point at a local hairpin is dropped.
pub(crate) fn fit_piece(piece: &[Point], accuracy: f64) -> BezPath {
    fit_kept(&cleaned(piece, accuracy), accuracy)
}

/// The points of a piece the fit uses: steps shorter than half the accuracy dropped (noise to
/// the fit; their hairpins go with them), points at local hairpins dropped, coincident points
/// merged.
pub(crate) fn cleaned(piece: &[Point], accuracy: f64) -> Vec<Point> {
    // Steps shorter than half the accuracy are noise to the fit; their hairpins go with them.
    let mut kept: Vec<Point> = Vec::with_capacity(piece.len());
    for (i, &p) in piece.iter().enumerate() {
        let last = i + 1 == piece.len();
        match kept.last() {
            Some(&q) if !last && (p - q).hypot() < 0.5 * accuracy => {}
            _ => kept.push(p),
        }
    }
    if kept.len() >= 2 && kept.len() < piece.len() {
        let n = kept.len();
        if (kept[n - 1] - kept[n - 2]).hypot() < 0.5 * accuracy && n > 2 {
            kept.remove(n - 2);
        }
    } else {
        kept = piece.to_vec();
    }
    // A local hairpin (a turn sharper than 120 degrees between two steps) is a glitch of the
    // samples: the simplifier's candidates degenerate at it, so its point is dropped.
    while let Some(at) = (1..kept.len().saturating_sub(1)).find(|&i| {
        let (a, b) = (kept[i] - kept[i - 1], kept[i + 1] - kept[i]);
        a.dot(b) < HAIRPIN_COS * a.hypot() * b.hypot()
    }) {
        kept.remove(at);
    }
    // Near-coincident points give the Catmull-Rom curve zero-length pieces.
    kept.dedup_by(|a, b| (*a - *b).hypot() < COINCIDENT_MM);
    bridged(kept)
}

/// A step this many times longer than the piece's median step is a gap of the axis.
const GAP_STEPS: f64 = 3.0;

/// `points` with each gap (where the axis passes a junction cluster, its samples are missing)
/// filled by points on its chord at the median step: next to short steps a long one makes the
/// Catmull-Rom curve overshoot into a kink, which the simplifier answers with a cluster of
/// anchors (`adrs.md` Round A, hypothesis 5).
fn bridged(points: Vec<Point>) -> Vec<Point> {
    if points.len() < 4 {
        return points;
    }
    let mut steps: Vec<f64> = points.windows(2).map(|w| (w[1] - w[0]).hypot()).collect();
    steps.sort_by(f64::total_cmp);
    let median = steps[steps.len() / 2];
    if median <= 0.0 || steps[steps.len() - 1] <= GAP_STEPS * median {
        return points;
    }
    let count = points.len();
    // The direction at a point, from its neighbours, as a unit vector.
    let direction = |i: usize| {
        let along = points[(i + 1).min(count - 1)] - points[i.saturating_sub(1)];
        let length = along.hypot();
        if length > 0.0 {
            along / length
        } else {
            kurbo::Vec2::ZERO
        }
    };
    let mut out = Vec::with_capacity(count);
    for i in 0..count - 1 {
        let (from, to) = (points[i], points[i + 1]);
        out.push(from);
        let step = (to - from).hypot();
        if step > GAP_STEPS * median {
            // A cubic Hermite across the gap, its tangents the neighbours' directions at the
            // gap's length: it leaves and meets the axis as the axis runs.
            let (leave, meet) = (direction(i) * step, direction(i + 1) * step);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let parts = (step / median).ceil() as usize;
            for k in 1..parts {
                #[allow(clippy::cast_precision_loss)]
                let t = k as f64 / parts as f64;
                let (t2, t3) = (t * t, t * t * t);
                let point = from.to_vec2() * (2.0 * t3 - 3.0 * t2 + 1.0)
                    + leave * (t3 - 2.0 * t2 + t)
                    + to.to_vec2() * (-2.0 * t3 + 3.0 * t2)
                    + meet * (t3 - t2);
                out.push(point.to_point());
            }
        }
    }
    out.push(points[points.len() - 1]);
    out
}

/// [`fit_piece`] on its cleaned points.
fn fit_kept(piece: &[Point], accuracy: f64) -> BezPath {
    let origin = piece[0].to_vec2();
    let local: Vec<Point> = piece.iter().map(|&p| p - origin).collect();
    let (first, last) = (local[0], local[local.len() - 1]);
    let chord = last - first;
    let straight = local.len() < 3
        || (chord.hypot() > COINCIDENT_MM
            && local
                .iter()
                .all(|&p| (p - first).cross(chord).abs() / chord.hypot() <= accuracy));
    if straight {
        let mut line = BezPath::new();
        line.move_to(piece[0]);
        line.line_to(piece[piece.len() - 1]);
        return line;
    }
    let fitted = simplify(&smooth(&local), accuracy);
    // kurbo's error measure casts rays from the source only, so it misses a candidate that
    // runs on past the piece's end along a straight run (a crest's leg into its tip): a fit
    // that strays from the piece's points by more than `STRAY` times the accuracy is split.
    if local.len() >= 4 && strays(&fitted, &local, STRAY * accuracy) {
        let mid = piece.len() / 2;
        let mut out = fit_kept(&piece[..=mid], accuracy);
        for element in fit_kept(&piece[mid..], accuracy).elements().iter().skip(1) {
            out.push(*element);
        }
        return out;
    }
    let shift = kurbo::Affine::translate(origin);
    let mut out = BezPath::new();
    for element in fitted.elements() {
        out.push(shift * *element);
    }
    out
}

/// A Catmull-Rom curve through `points`: tangent-continuous, so the simplifier can merge its
/// pieces (it keeps every corner of a polyline).
pub(crate) fn smooth(points: &[Point]) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(points[0]);
    let n = points.len();
    let tangent = |i: usize| {
        let (a, b) = (points[i.saturating_sub(1)], points[(i + 1).min(n - 1)]);
        (b - a) / 6.0
    };
    for i in 0..n - 1 {
        path.curve_to(
            points[i] + tangent(i),
            points[i + 1] - tangent(i + 1),
            points[i + 1],
        );
    }
    path
}

/// The path's anchors as `(point, handle_in, handle_out)`.
pub(crate) fn anchors_of(path: &BezPath, closed: bool) -> Vec<OutlineTriple> {
    let doc = |p: Point| DocPoint::new(p.x, p.y);
    let mut anchors: Vec<OutlineTriple> = Vec::new();
    let mut last = Point::ZERO;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => {
                anchors.push((doc(p), DocVec::ZERO, DocVec::ZERO));
                last = p;
            }
            PathEl::CurveTo(c1, c2, p) => {
                if let Some(previous) = anchors.last_mut() {
                    previous.2 = DocVec::new(c1.x - last.x, c1.y - last.y);
                }
                anchors.push((doc(p), DocVec::new(c2.x - p.x, c2.y - p.y), DocVec::ZERO));
                last = p;
            }
            PathEl::QuadTo(c, p) => {
                let c1 = last + (c - last) * (2.0 / 3.0);
                let c2 = p + (c - p) * (2.0 / 3.0);
                if let Some(previous) = anchors.last_mut() {
                    previous.2 = DocVec::new(c1.x - last.x, c1.y - last.y);
                }
                anchors.push((doc(p), DocVec::new(c2.x - p.x, c2.y - p.y), DocVec::ZERO));
                last = p;
            }
            PathEl::ClosePath => {}
        }
    }
    if closed
        && anchors.len() > 2
        && let Some(end) = anchors.pop()
    {
        anchors[0].1 = end.1;
    }
    anchors
}
