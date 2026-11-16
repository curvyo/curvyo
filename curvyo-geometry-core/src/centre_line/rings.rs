//! The subject's own outline as the rings of a region kept filled (ADR 0063 amendment
//! 2026-11-18, K).

use curvyo_document_core::{Point as DocPoint, Vec2 as DocVec};
use kurbo::{BezPath, CubicBez, ParamCurve, ParamCurveNearest, Point, Shape, Vec2};

use crate::OutlineTriple;

/// How far a ring of the region may lie from the subject's outline it came from: the
/// flattening, the grid and some slack, in millimetres.
const MATCH_MM: f64 = 0.05;
/// Ring points checked against a subject outline.
const MATCH_POINTS: usize = 8;
pub(super) use super::KeptRegion;

/// The subject's outlines as closed rings of cubic segments.
pub(super) fn source_rings(sources: &[Vec<OutlineTriple>]) -> Vec<Vec<CubicBez>> {
    sources
        .iter()
        .filter(|ring| ring.len() >= 2)
        .map(|ring| {
            (0..ring.len())
                .map(|i| {
                    let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                    CubicBez::new(
                        point(a.0),
                        point(a.0) + vec(a.2),
                        point(b.0) + vec(b.1),
                        point(b.0),
                    )
                })
                .collect()
        })
        .collect()
}

fn point(p: DocPoint) -> Point {
    Point::new(p.x, p.y)
}

fn vec(v: DocVec) -> Vec2 {
    Vec2::new(v.x, v.y)
}

/// The rings of a region as cubic segments: the subject's own outline for each ring when every
/// ring matches exactly one of them, else the flattened rings as lines (the fallback of ADR K).
pub(super) fn region_segments(
    rings: &[Vec<Point>],
    sources: &[Vec<CubicBez>],
) -> Vec<Vec<CubicBez>> {
    matched(rings, sources).unwrap_or_else(|| {
        rings
            .iter()
            .map(|ring| {
                (0..ring.len())
                    .map(|i| line(ring[i], ring[(i + 1) % ring.len()]))
                    .collect()
            })
            .collect()
    })
}

pub(super) fn line(a: Point, b: Point) -> CubicBez {
    CubicBez::new(a, a, b, b)
}

/// The subject outline of each ring, if every ring has exactly one.
fn matched(rings: &[Vec<Point>], sources: &[Vec<CubicBez>]) -> Option<Vec<Vec<CubicBez>>> {
    let boxes: Vec<kurbo::Rect> = sources.iter().map(|s| path_of(s).bounding_box()).collect();
    let mut used = vec![false; sources.len()];
    let mut out = Vec::with_capacity(rings.len());
    for ring in rings {
        let ring_box = ring
            .iter()
            .fold(kurbo::Rect::from_points(ring[0], ring[0]), |r, &p| {
                r.union_pt(p)
            });
        let close = |a: f64, b: f64| (a - b).abs() <= MATCH_MM;
        let found = (0..sources.len()).find(|&s| {
            let b = boxes[s];
            !used[s]
                && close(b.x0, ring_box.x0)
                && close(b.y0, ring_box.y0)
                && close(b.x1, ring_box.x1)
                && close(b.y1, ring_box.y1)
                && (0..MATCH_POINTS).all(|k| {
                    let p = ring[k * ring.len() / MATCH_POINTS];
                    nearest(&sources[s], p).2 <= MATCH_MM
                })
        })?;
        used[found] = true;
        out.push(sources[found].clone());
    }
    Some(out)
}

pub(super) fn path_of(segments: &[CubicBez]) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(segments[0].p0);
    for s in segments {
        path.curve_to(s.p1, s.p2, s.p3);
    }
    path.close_path();
    path
}

/// The segment, its parameter and the distance of the point of `ring` nearest to `p`.
pub(super) fn nearest(ring: &[CubicBez], p: Point) -> (usize, f64, f64) {
    ring.iter()
        .enumerate()
        .map(|(i, s)| {
            let hit = s.nearest(p, 1e-9);
            (i, hit.t, hit.distance_sq.sqrt())
        })
        .min_by(|a, b| a.2.total_cmp(&b.2).then(a.0.cmp(&b.0)))
        .unwrap_or((0, 0.0, f64::INFINITY))
}

/// A whole region kept: its rings as they are.
pub(super) fn whole(segments: &[Vec<CubicBez>]) -> KeptRegion {
    canonical(
        segments
            .iter()
            .enumerate()
            .map(|(k, ring)| triples(&oriented(ring, k == 0)))
            .collect(),
    )
}

/// The ring turned to positive (`positive`) or negative signed area.
fn oriented(ring: &[CubicBez], positive: bool) -> Vec<CubicBez> {
    if (signed_area(ring) > 0.0) == positive {
        ring.to_vec()
    } else {
        reversed(ring)
    }
}

/// A kept region in canonical order (ADR 0063 §9, the same result for permuted subpaths and
/// start anchors): every ring starts at its smallest anchor (x, then y), holes sorted by it.
pub(super) fn canonical(mut region: KeptRegion) -> KeptRegion {
    let key = |a: &OutlineTriple| (a.0.x, a.0.y);
    for ring in &mut region {
        let first = (0..ring.len())
            .min_by(|&i, &j| {
                key(&ring[i])
                    .partial_cmp(&key(&ring[j]))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(0);
        ring.rotate_left(first);
    }
    if region.len() > 2 {
        region[1..].sort_by(|a, b| {
            key(&a[0])
                .partial_cmp(&key(&b[0]))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    region
}

/// A ring of segments as anchors.
pub(super) fn triples(ring: &[CubicBez]) -> Vec<OutlineTriple> {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (previous, here) = (ring[(i + n - 1) % n], ring[i]);
            (
                DocPoint::new(here.p0.x, here.p0.y),
                DocVec::new(previous.p2.x - previous.p3.x, previous.p2.y - previous.p3.y),
                DocVec::new(here.p1.x - here.p0.x, here.p1.y - here.p0.y),
            )
        })
        .collect()
}

/// The position (segment index plus parameter) of the point of `ring` nearest to `p`.
pub(super) fn position(ring: &[CubicBez], p: Point) -> f64 {
    let (segment, t, _) = nearest(ring, p);
    #[allow(clippy::cast_precision_loss)]
    let at = segment as f64 + t;
    at
}

/// The segments of `ring` from position `from` ahead to position `to` (wrapping).
pub(super) fn arc_of(ring: &[CubicBez], from: f64, to: f64) -> Vec<CubicBez> {
    let n = ring.len();
    #[allow(clippy::cast_precision_loss)]
    let to = if to <= from { to + n as f64 } else { to };
    let mut out = Vec::new();
    let mut at = from;
    while at < to {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let index = at.floor() as usize;
        #[allow(clippy::cast_precision_loss)]
        let base = index as f64;
        let (t0, t1) = (at - base, (to - base).min(1.0));
        let segment = ring[index % n];
        if t1 > t0 {
            // invariant: a whole segment gives exactly 0 and 1 (an index and the next).
            #[allow(clippy::float_cmp)]
            let whole = t0 == 0.0 && t1 == 1.0;
            out.push(if whole {
                segment
            } else if segment.p1 == segment.p0 && segment.p2 == segment.p3 {
                // A line stays a line: no handles.
                line(segment.eval(t0), segment.eval(t1))
            } else {
                segment.subsegment(t0..t1)
            });
        }
        at = base + 1.0;
    }
    out
}

pub(super) fn reversed(arc: &[CubicBez]) -> Vec<CubicBez> {
    arc.iter()
        .rev()
        .map(|s| CubicBez::new(s.p3, s.p2, s.p1, s.p0))
        .collect()
}

/// The signed area of a ring, as `signed_area_mm2` counts it.
pub(super) fn signed_area(ring: &[CubicBez]) -> f64 {
    let mut points: Vec<DocPoint> = Vec::new();
    kurbo::flatten(path_of(ring), 0.01, |el| {
        if let kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) = el {
            points.push(DocPoint::new(p.x, p.y));
        }
    });
    crate::signed_area_mm2(&points)
}
