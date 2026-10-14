//! The output form of a rebuilt offset outline (`specs/0038-path-offset` criterion 33): nodes
//! closer than 0.001 mm merged, straight runs joined, smooth nodes where the tangents agree, each
//! outline started at its smallest node and the outlines sorted.

use curvyo_document_core::{Point as DocPoint, Vec2 as DocVec};
use kurbo::{CubicBez, Line, ParamCurve, PathSeg, Point, Vec2};

use crate::boolean_grid::{Point64, to_millimetres};
use crate::offset::OffsetAnchor;

/// A grid polygon as closed lines in millimetres.
pub(crate) fn lines_of_mm(path: &[Point64]) -> Vec<PathSeg> {
    let mm = |p: Point64| {
        let point = to_millimetres(&p);
        Point::new(point.x, point.y)
    };
    (0..path.len())
        .map(|i| PathSeg::Line(Line::new(mm(path[i]), mm(path[(i + 1) % path.len()]))))
        .collect()
}

/// The outlines as nodes, each started at its smallest node, sorted by that node and then by
/// area (larger first).
pub(crate) fn canonical(outlines: &[Vec<PathSeg>]) -> Vec<Vec<OffsetAnchor>> {
    let mut keyed: Vec<(f64, Vec<OffsetAnchor>)> = outlines
        .iter()
        .filter(|pieces| !pieces.is_empty())
        .map(|pieces| {
            let n = pieces.len();
            let start = (0..n)
                .min_by(|&a, &b| {
                    let (pa, pb) = (pieces[a].eval(0.0), pieces[b].eval(0.0));
                    pa.x.total_cmp(&pb.x).then(pa.y.total_cmp(&pb.y))
                })
                .unwrap_or(0);
            let anchors = (0..n)
                .map(|k| {
                    let index = (start + k) % n;
                    let (before, piece) = (&pieces[(index + n - 1) % n], &pieces[index]);
                    let point = piece.eval(0.0);
                    let handle_in = match *before {
                        PathSeg::Cubic(c) => c.p2 - c.p3,
                        _ => Vec2::ZERO,
                    };
                    let handle_out = match *piece {
                        PathSeg::Cubic(c) => c.p1 - c.p0,
                        _ => Vec2::ZERO,
                    };
                    OffsetAnchor {
                        point: DocPoint::new(point.x, point.y),
                        handle_in: DocVec::new(handle_in.x, handle_in.y),
                        handle_out: DocVec::new(handle_out.x, handle_out.y),
                        smooth: is_smooth(before, piece),
                    }
                })
                .collect();
            (double_area_mm2(pieces), anchors)
        })
        .collect();
    keyed.sort_by(|(area_a, a), (area_b, b)| {
        a[0].point
            .x
            .total_cmp(&b[0].point.x)
            .then(a[0].point.y.total_cmp(&b[0].point.y))
            .then(area_b.total_cmp(area_a))
    });
    keyed.into_iter().map(|(_, anchors)| anchors).collect()
}

/// Nodes closer than this, in millimetres, are merged (criterion 33).
const MIN_NODE_SPACING_MM: f64 = 0.001;

/// A node this close to the line between its straight neighbours is dropped, in millimetres.
const COLLINEAR_MM: f64 = 1e-6;

/// The sine of the largest tangent difference of a smooth node: 0.1° (criterion 33).
const SMOOTH_SINE: f64 = 0.001_745_328_365_898_309;

pub(crate) fn set_start(segment: &mut PathSeg, point: Point) {
    match segment {
        PathSeg::Line(line) => line.p0 = point,
        PathSeg::Cubic(cubic) => {
            let shift = point - cubic.p0;
            cubic.p0 = point;
            cubic.p1 += shift;
        }
        PathSeg::Quad(quad) => quad.p0 = point,
    }
}

pub(crate) fn set_end(segment: &mut PathSeg, point: Point) {
    match segment {
        PathSeg::Line(line) => line.p1 = point,
        PathSeg::Cubic(cubic) => {
            let shift = point - cubic.p3;
            cubic.p3 = point;
            cubic.p2 += shift;
        }
        PathSeg::Quad(quad) => quad.p2 = point,
    }
}

pub(crate) fn start_point(segment: &PathSeg) -> Point {
    segment.eval(0.0)
}

pub(crate) fn end_point(segment: &PathSeg) -> Point {
    segment.eval(1.0)
}

/// The size of a segment: the largest distance of a control point from its start.
fn extent(segment: &PathSeg) -> f64 {
    match *segment {
        PathSeg::Line(line) => (line.p1 - line.p0).hypot(),
        PathSeg::Cubic(c) => (c.p1 - c.p0)
            .hypot()
            .max((c.p2 - c.p0).hypot())
            .max((c.p3 - c.p0).hypot()),
        PathSeg::Quad(q) => (q.p1 - q.p0).hypot().max((q.p2 - q.p0).hypot()),
    }
}

/// The output rules of criterion 33 on a closed list of pieces: no two nodes closer than
/// 0.001 mm, no node on the straight line between two straight neighbours, every joint closed.
pub(crate) fn finish(pieces: &[PathSeg]) -> Vec<PathSeg> {
    let mut out: Vec<PathSeg> = Vec::with_capacity(pieces.len());
    for piece in pieces {
        if extent(piece) < MIN_NODE_SPACING_MM {
            continue;
        }
        if let Some(previous) = out.last() {
            let joint = end_point(previous);
            let mut piece = *piece;
            set_start(&mut piece, joint);
            out.push(piece);
        } else {
            out.push(*piece);
        }
    }
    // Merge straight runs and close the outline.
    let mut merged: Vec<PathSeg> = Vec::with_capacity(out.len());
    for piece in out {
        if let (Some(PathSeg::Line(previous)), PathSeg::Line(line)) = (merged.last(), piece)
            && is_straight_through(previous.p0, previous.p1, line.p1)
        {
            let start = previous.p0;
            merged.pop();
            merged.push(PathSeg::Line(Line::new(start, line.p1)));
            continue;
        }
        merged.push(piece);
    }
    while merged.len() > 2 {
        let (PathSeg::Line(last), PathSeg::Line(first)) = (merged[merged.len() - 1], merged[0])
        else {
            break;
        };
        if !is_straight_through(last.p0, first.p0, first.p1) {
            break;
        }
        merged.pop();
        merged[0] = PathSeg::Line(Line::new(last.p0, first.p1));
    }
    if let (Some(first), Some(last)) = (merged.first().copied(), merged.last_mut()) {
        let joint = start_point(&first);
        if (end_point(last) - joint).hypot() < MIN_NODE_SPACING_MM {
            set_end(last, joint);
        } else {
            let from = end_point(last);
            merged.push(PathSeg::Line(Line::new(from, joint)));
        }
    }
    merged
}

/// Whether `middle` lies on the straight line from `a` to `b`, between them.
fn is_straight_through(a: Point, middle: Point, b: Point) -> bool {
    let chord = b - a;
    let length = chord.hypot();
    if length <= COLLINEAR_MM {
        return false;
    }
    let off = (middle - a).cross(chord).abs() / length;
    let along = (middle - a).dot(chord);
    off <= COLLINEAR_MM && along > 0.0 && along < chord.hypot2()
}

/// Whether the joint from `before` into `after` is smooth: the tangents differ by at most 0.1°.
pub(crate) fn is_smooth(before: &PathSeg, after: &PathSeg) -> bool {
    let (u1, u2) = (
        crate::offset_pieces::end_tangent(before),
        crate::offset_pieces::start_tangent(after),
    );
    u1.dot(u2) > 0.0 && u1.cross(u2).abs() <= SMOOTH_SINE
}

/// Twice the area of a closed list of pieces in square millimetres, positive for an outer
/// outline: the polygon of the end points plus each cubic's area against its chord.
pub(crate) fn double_area_mm2(pieces: &[PathSeg]) -> f64 {
    let mut sum = 0.0;
    for piece in pieces {
        let (a, b) = (start_point(piece), end_point(piece));
        sum += a.x * b.y - b.x * a.y;
        if let PathSeg::Cubic(c) = *piece {
            sum += 2.0 * cubic_chord_area(&c);
        }
    }
    sum
}

/// The signed area between a cubic and its chord (positive for a bulge to the left).
fn cubic_chord_area(c: &CubicBez) -> f64 {
    // Green's theorem on the cubic minus the chord's own term, in closed form.
    let (p0, p1, p2, p3) = (
        c.p0.to_vec2(),
        c.p1.to_vec2(),
        c.p2.to_vec2(),
        c.p3.to_vec2(),
    );
    let whole = (p0.cross(p1) * 6.0
        + p0.cross(p2) * 3.0
        + p0.cross(p3)
        + p1.cross(p2) * 3.0
        + p1.cross(p3) * 3.0
        + p2.cross(p3) * 6.0)
        / 20.0;
    whole - p0.cross(p3) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_area_of_a_cubic_circle_is_near_pi_r_squared() {
        let k = 0.552_284_749_830_793_4 * 10.0;
        let p = |x: f64, y: f64| Point::new(x, y);
        let quarter = [
            CubicBez::new(p(10.0, 0.0), p(10.0, k), p(k, 10.0), p(0.0, 10.0)),
            CubicBez::new(p(0.0, 10.0), p(-k, 10.0), p(-10.0, k), p(-10.0, 0.0)),
            CubicBez::new(p(-10.0, 0.0), p(-10.0, -k), p(-k, -10.0), p(0.0, -10.0)),
            CubicBez::new(p(0.0, -10.0), p(k, -10.0), p(10.0, -k), p(10.0, 0.0)),
        ];
        let pieces: Vec<PathSeg> = quarter.into_iter().map(PathSeg::Cubic).collect();
        let area = double_area_mm2(&pieces) / 2.0;
        assert!((area - std::f64::consts::PI * 100.0).abs() < 0.1, "{area}");
    }

    #[test]
    fn straight_runs_merge_and_tiny_pieces_go() {
        let p = |x: f64, y: f64| Point::new(x, y);
        let pieces = [
            PathSeg::Line(Line::new(p(0.0, 0.0), p(5.0, 0.0))),
            PathSeg::Line(Line::new(p(5.0, 0.0), p(10.0, 0.0))),
            PathSeg::Line(Line::new(p(10.0, 0.0), p(10.0, 0.0005))),
            PathSeg::Line(Line::new(p(10.0, 0.0005), p(10.0, 10.0))),
            PathSeg::Line(Line::new(p(10.0, 10.0), p(0.0, 10.0))),
            PathSeg::Line(Line::new(p(0.0, 10.0), p(0.0, 0.0))),
        ];
        let finished = finish(&pieces);
        assert_eq!(finished.len(), 4, "{finished:?}");
    }
}
