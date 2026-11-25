//! A test helper: a closed outline is a simple polygon (no repeated vertex, no two edges that
//! touch) at its vertices, which suffices for plates whose arcs are short cubics.

use curvyo_document_core::Point;
use curvyo_extension_core::ClosedPath;

fn vertices(path: &ClosedPath) -> Vec<Point> {
    let mut points = vec![path.start];
    points.extend(path.segments.iter().map(|segment| segment.to));
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points
}

/// Whether the segments `p1`-`p2` and `q1`-`q2` share a point (touching counts).
fn segments_meet(p1: Point, p2: Point, q1: Point, q2: Point) -> bool {
    let cross = |from: Point, to: Point, at: Point| {
        (to.x - from.x) * (at.y - from.y) - (to.y - from.y) * (at.x - from.x)
    };
    let within = |from: Point, to: Point, at: Point| {
        at.x >= from.x.min(to.x) - 1e-9
            && at.x <= from.x.max(to.x) + 1e-9
            && at.y >= from.y.min(to.y) - 1e-9
            && at.y <= from.y.max(to.y) + 1e-9
    };
    let (s1, s2) = (cross(q1, q2, p1), cross(q1, q2, p2));
    let (t1, t2) = (cross(p1, p2, q1), cross(p1, p2, q2));
    if s1 * s2 < -1e-12 && t1 * t2 < -1e-12 {
        return true;
    }
    (s1.abs() < 1e-9 && within(q1, q2, p1))
        || (s2.abs() < 1e-9 && within(q1, q2, p2))
        || (t1.abs() < 1e-9 && within(p1, p2, q1))
        || (t2.abs() < 1e-9 && within(p1, p2, q2))
}

/// No repeated vertex and no two non-adjacent edges that touch.
pub(crate) fn assert_simple(path: &ClosedPath, what: &str) {
    let points = vertices(path);
    let count = points.len();
    for first in 0..count {
        for second in first + 1..count {
            let gap =
                (points[first].x - points[second].x).hypot(points[first].y - points[second].y);
            assert!(gap > 1e-6, "{what}: vertex {first} repeats at {second}");
        }
    }
    for first in 0..count {
        for second in first + 2..count {
            if first == 0 && second == count - 1 {
                continue;
            }
            assert!(
                !segments_meet(
                    points[first],
                    points[(first + 1) % count],
                    points[second],
                    points[(second + 1) % count]
                ),
                "{what}: edge {first} meets edge {second}"
            );
        }
    }
}
