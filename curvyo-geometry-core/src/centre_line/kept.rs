//! The parts kept filled at Thick parts (ADR 0063 amendment 2026-11-18, K): the region less the
//! painted strokes, outlined by the subject's own Béziers wherever it runs along them.

use curvyo_document_core::{Point as DocPoint, Vec2 as DocVec};
use kurbo::{CubicBez, Point, Vec2};

use super::rings::{KeptRegion, arc_of, canonical, line, position, reversed, signed_area, triples};
use crate::OutlineTriple;
use crate::medial_axis::AxisSample;
use crate::medial_axis::boundary::Boundary;

/// The discs reach this far past their clearance, so they cross the outline where they touch
/// it, in millimetres.
const REACH_MM: f64 = 0.05;
/// Corners of the polygon standing in for a disc.
const DISC_CORNERS: u32 = 24;
/// A vertex of the kept part this close to the outline lies on it, in millimetres: the grid of
/// the booleans plus rounding, far below the discs' reach.
const ON_OUTLINE_MM: f64 = 0.003;
/// How far a stretch of a kept outline across the region may be straightened, in mm.
const ACROSS_TOLERANCE_MM: f64 = 0.02;
/// Kept rings smaller than this are crumbs of the booleans, in square millimetres.
const CRUMB_MM2: f64 = 0.01;
/// The tolerance of the intersection.
const TOLERANCE_MM: f64 = 0.01;
/// A piece of an outline shorter than this is dropped, in millimetres.
const DEGENERATE_MM: f64 = 1e-6;

/// The parts of a region kept filled: the region less the strokes that became paths (each
/// piece's samples swept by their discs), the parts that hold a `thick` point, each outline
/// restored to `segments` (the same rings as `rings`, in mm) where it runs along them; `None`
/// when the difference fails.
pub(super) fn parts(
    rings: &[Vec<Point>],
    segments: &[Vec<CubicBez>],
    strokes: &[&[AxisSample]],
    thick: &[Point],
) -> Option<Vec<KeptRegion>> {
    let mut sweeps: Vec<Vec<OutlineTriple>> = Vec::new();
    for stroke in strokes {
        for (i, sample) in stroke.iter().enumerate() {
            sweeps.push(disc_polygon(sample));
            if let Some(next) = stroke.get(i + 1) {
                sweeps.extend(link_polygon(sample, next));
            }
        }
    }
    let region: Vec<Vec<OutlineTriple>> = rings.iter().map(|r| corners(r)).collect();
    let region: Vec<crate::Outline<'_>> = region
        .iter()
        .map(|r| crate::Outline::new(r, true))
        .collect();
    let sweeps: Vec<crate::Outline<'_>> = sweeps
        .iter()
        .map(|r| crate::Outline::new(r, true))
        .collect();
    let kept = crate::boolean(
        crate::BooleanOp::Difference,
        &[&region, &sweeps],
        curvyo_document_core::Tolerance::from_mm(TOLERANCE_MM),
    )
    .ok()?
    .into_outlines();
    let areas: Vec<f64> = kept.iter().map(|r| crate::signed_area_mm2(r)).collect();
    // Each outer ring with the holes directly inside it.
    let mut groups: Vec<(usize, Vec<usize>)> = (0..kept.len())
        .filter(|&i| areas[i] > CRUMB_MM2)
        .map(|i| (i, Vec::new()))
        .collect();
    for i in (0..kept.len()).filter(|&i| areas[i] < -CRUMB_MM2) {
        let owner = (0..groups.len())
            .filter(|&g| contains(&kept[groups[g].0], kept[i][0]))
            .min_by(|&a, &b| areas[groups[a].0].total_cmp(&areas[groups[b].0]));
        if let Some(g) = owner {
            groups[g].1.push(i);
        }
    }
    let boundary = Boundary::new(rings);
    let doc = |p: &Point| DocPoint::new(p.x, p.y);
    Some(
        groups
            .into_iter()
            .filter(|(outer, holes)| {
                thick.iter().any(|p| {
                    contains(&kept[*outer], doc(p))
                        && !holes.iter().any(|&h| contains(&kept[h], doc(p)))
                })
            })
            .map(|(outer, holes)| {
                canonical(
                    std::iter::once(outer)
                        .chain(holes)
                        .map(|i| restored(&kept[i], segments, &boundary))
                        .collect(),
                )
            })
            .collect(),
    )
}

/// The band between two neighbouring samples' discs grown by the reach: the quadrilateral of
/// their offsets to both sides of the link.
fn link_polygon(a: &AxisSample, b: &AxisSample) -> Option<Vec<OutlineTriple>> {
    let d = b.point - a.point;
    let length = d.hypot();
    if length <= DEGENERATE_MM {
        return None;
    }
    let n = Vec2::new(-d.y, d.x) / length;
    let (ra, rb) = (a.radius + REACH_MM, b.radius + REACH_MM);
    let doc = |p: Point| (DocPoint::new(p.x, p.y), DocVec::ZERO, DocVec::ZERO);
    Some(vec![
        doc(a.point + n * ra),
        doc(a.point - n * ra),
        doc(b.point - n * rb),
        doc(b.point + n * rb),
    ])
}

/// A polygon around the disc of `sample` grown by the reach.
fn disc_polygon(sample: &AxisSample) -> Vec<OutlineTriple> {
    let n = f64::from(DISC_CORNERS);
    let r = (sample.radius + REACH_MM) / (std::f64::consts::PI / n).cos();
    (0..DISC_CORNERS)
        .map(|i| {
            let a = f64::from(i) * std::f64::consts::TAU / n;
            (
                DocPoint::new(sample.point.x + r * a.cos(), sample.point.y + r * a.sin()),
                DocVec::ZERO,
                DocVec::ZERO,
            )
        })
        .collect()
}

fn corners(ring: &[Point]) -> Vec<OutlineTriple> {
    ring.iter()
        .map(|p| (DocPoint::new(p.x, p.y), DocVec::ZERO, DocVec::ZERO))
        .collect()
}

/// Whether `p` lies inside the polygon `ring` (even-odd).
fn contains(ring: &[DocPoint], p: DocPoint) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    inside
}

/// A kept ring with its stretches along the outline replaced by the subject's own segments
/// between the same ends, and straight chords elsewhere.
fn restored(
    ring: &[DocPoint],
    segments: &[Vec<CubicBez>],
    boundary: &Boundary,
) -> Vec<OutlineTriple> {
    let points: Vec<Point> = ring.iter().map(|p| Point::new(p.x, p.y)).collect();
    let on: Vec<Option<usize>> = points
        .iter()
        .map(|&p| {
            boundary
                .nearest_where(p, |_, _| true)
                .and_then(|(d, _, k)| (d <= ON_OUTLINE_MM).then_some(k))
        })
        .collect();
    let n = points.len();
    let Some(off) = on.iter().position(Option::is_none) else {
        // The whole ring runs along one outline ring: that ring as it is.
        return whole_ring(&segments[on[0].unwrap_or(0)], &points);
    };
    // From an off-outline vertex: the runs along the outline (each with its outline ring and
    // the indices of its points), and between them the stretches across the region.
    let mut runs: Vec<(usize, Vec<usize>)> = Vec::new();
    for i in 0..n {
        let at = (off + i) % n;
        match (on[at], runs.last_mut()) {
            (Some(k), Some((last, run))) if *last == k && on[(at + n - 1) % n] == Some(k) => {
                run.push(at);
            }
            (Some(k), _) => runs.push((k, vec![at])),
            (None, _) => {}
        }
    }
    let inside_sign = crate::signed_area_mm2(ring).signum();
    let mut out: Vec<CubicBez> = Vec::new();
    for (r, (k, run)) in runs.iter().enumerate() {
        let along: Vec<Point> = run.iter().map(|&j| points[j]).collect();
        out.extend(arc_along(&segments[*k], &along));
        // Across to the next run: a straight chord where the stretch dents the kept part (the
        // chord only adds stroke to it), else the stretch itself, simplified.
        let from = run[run.len() - 1];
        let to = runs[(r + 1) % runs.len()].1[0];
        // A do-while: one run of one vertex (`from == to`) goes round the whole ring.
        let mut across: Vec<Point> = vec![points[from]];
        let mut j = from;
        loop {
            j = (j + 1) % n;
            across.push(points[j]);
            if j == to {
                break;
            }
        }
        let (a, b) = (across[0], across[across.len() - 1]);
        let dents = from != to
            && across[1..across.len() - 1]
                .iter()
                .all(|&v| (b - a).cross(v - a) * inside_sign >= 0.0);
        if dents {
            out.push(line(a, b));
        } else {
            let kept = simplified(&across, ACROSS_TOLERANCE_MM);
            out.extend(kept.windows(2).map(|w| line(w[0], w[1])));
        }
    }
    out.retain(|s| {
        (s.p3 - s.p0).hypot() > DEGENERATE_MM
            || (s.p1 - s.p0).hypot() > DEGENERATE_MM
            || (s.p2 - s.p3).hypot() > DEGENERATE_MM
    });
    if out.len() < 2 {
        return corners(&points);
    }
    triples(&out)
}

/// The points of `line` that keep it within `tolerance` (Douglas-Peucker), both ends kept.
fn simplified(line: &[Point], tolerance: f64) -> Vec<Point> {
    if line.len() <= 2 {
        return line.to_vec();
    }
    let (a, b) = (line[0], line[line.len() - 1]);
    let chord = b - a;
    let length = chord.hypot();
    let (far, distance) = line[1..line.len() - 1]
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let d = if length > 0.0 {
                chord.cross(p - a).abs() / length
            } else {
                (p - a).hypot()
            };
            (i + 1, d)
        })
        .fold((0, -1.0), |best, x| if x.1 > best.1 { x } else { best });
    if distance <= tolerance {
        return vec![a, b];
    }
    let mut out = simplified(&line[..=far], tolerance);
    out.pop();
    out.extend(simplified(&line[far..], tolerance));
    out
}

/// The outline `ring` as it is, turned the way of the kept ring `points`.
fn whole_ring(ring: &[CubicBez], points: &[Point]) -> Vec<OutlineTriple> {
    let forward = signed_area(ring) > 0.0;
    let outer = crate::signed_area_mm2(ring_points(points).as_slice()) > 0.0;
    triples(&if forward == outer {
        ring.to_vec()
    } else {
        reversed(ring)
    })
}

fn ring_points(points: &[Point]) -> Vec<DocPoint> {
    points.iter().map(|p| DocPoint::new(p.x, p.y)).collect()
}

/// The outline `ring`'s segments from the first of `run`'s points to its last, in the direction
/// the run goes.
fn arc_along(ring: &[CubicBez], run: &[Point]) -> Vec<CubicBez> {
    if run.len() < 2 {
        return Vec::new();
    }
    let from = position(ring, run[0]);
    let to = position(ring, run[run.len() - 1]);
    #[allow(clippy::cast_precision_loss)]
    let length = ring.len() as f64;
    let wrap = |d: f64| {
        if d > length / 2.0 {
            d - length
        } else if d < -length / 2.0 {
            d + length
        } else {
            d
        }
    };
    // The direction from the first point that moves along the ring.
    let forward = run[1..]
        .iter()
        .map(|&p| wrap(position(ring, p) - from))
        .find(|d| d.abs() > 1e-9)
        .is_none_or(|d| d > 0.0);
    if forward {
        arc_of(ring, from, to)
    } else {
        reversed(&arc_of(ring, to, from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A kept part that touches the outline at one vertex (a thick part reaching the outline
    /// at one crest point): its outline runs from that vertex round the ring back to it.
    #[test]
    fn a_kept_ring_touching_the_outline_at_one_vertex_is_restored() {
        let square = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        let segments: Vec<CubicBez> = (0..4)
            .map(|i| line(square[i], square[(i + 1) % 4]))
            .collect();
        let boundary = Boundary::new(std::slice::from_ref(&square));
        let ring = [
            DocPoint::new(0.0, 5.0),
            DocPoint::new(3.0, 4.0),
            DocPoint::new(4.0, 5.0),
            DocPoint::new(3.0, 6.0),
        ];
        let restored = restored(&ring, &[segments], &boundary);
        assert!(restored.len() >= 3, "{restored:?}");
        for corner in ring {
            assert!(
                restored
                    .iter()
                    .any(|t| (t.0.x - corner.x).hypot(t.0.y - corner.y) < 1e-6),
                "{corner:?} missing from {restored:?}"
            );
        }
    }
}
