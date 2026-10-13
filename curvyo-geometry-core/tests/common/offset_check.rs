//! Queries on curve results of the offset kernel (`specs/0038-path-offset` criteria 33, 34): the
//! region, the nodes, the distance to the exact offset and the output rules.

use curvyo_document_core::{Point, Vec2};
use curvyo_geometry_core::{OffsetOutlines, signed_area_mm2};

use super::Anchors;

/// The flattening used to measure a result: far below the kernel tolerance.
pub const MEASURE_MM: f64 = 0.0005;

/// The result's outlines as fine polygons.
pub fn flat(result: &OffsetOutlines) -> Vec<Vec<Point>> {
    result.flattened(MEASURE_MM)
}

/// The area the result paints (holes negative).
pub fn area(result: &OffsetOutlines) -> f64 {
    flat(result).iter().map(|o| signed_area_mm2(o)).sum()
}

/// Winding number of the result around `p`.
pub fn winding(result: &OffsetOutlines, p: Point) -> i32 {
    let mut total = 0;
    for outline in flat(result) {
        for i in 0..outline.len() {
            let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
            let side = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
            if a.y <= p.y {
                if b.y > p.y && side > 0.0 {
                    total += 1;
                }
            } else if b.y <= p.y && side < 0.0 {
                total -= 1;
            }
        }
    }
    total
}

pub fn covers(result: &OffsetOutlines, p: Point) -> bool {
    winding(result, p) != 0
}

/// Every node of every outline.
pub fn nodes(result: &OffsetOutlines) -> Vec<Point> {
    result
        .outlines()
        .iter()
        .flatten()
        .map(|a| a.point)
        .collect()
}

pub fn has_node(result: &OffsetOutlines, x: f64, y: f64, tol: f64) -> bool {
    nodes(result)
        .iter()
        .any(|p| (p.x - x).hypot(p.y - y) <= tol)
}

/// `(min x, min y, max x, max y)` of the result.
pub fn bbox(result: &OffsetOutlines) -> (f64, f64, f64, f64) {
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in flat(result).iter().flatten() {
        b = (b.0.min(p.x), b.1.min(p.y), b.2.max(p.x), b.3.max(p.y));
    }
    b
}

/// How many segments of the result are cubics.
pub fn cubic_count(result: &OffsetOutlines) -> usize {
    result
        .outlines()
        .iter()
        .map(|outline| {
            (0..outline.len())
                .filter(|&i| {
                    outline[i].handle_out != Vec2::ZERO
                        || outline[(i + 1) % outline.len()].handle_in != Vec2::ZERO
                })
                .count()
        })
        .sum()
}

fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a.x + t * dx - p.x).hypot(a.y + t * dy - p.y)
}

/// The operand's outlines flattened finely: the reference for "the exact offset".
fn operand_polylines(operand: &[(Anchors, bool)]) -> Vec<(Vec<Point>, bool)> {
    operand
        .iter()
        .map(|(anchors, closed)| {
            let mut points = Vec::new();
            let count = if *closed {
                anchors.len()
            } else {
                anchors.len() - 1
            };
            for i in 0..count {
                let (p0, _, out) = anchors[i];
                let (p3, hin, _) = anchors[(i + 1) % anchors.len()];
                let (p1, p2) = (p0.translated(out), p3.translated(hin));
                for k in 0..64 {
                    let t = f64::from(k) / 64.0;
                    let mt = 1.0 - t;
                    let w = [mt * mt * mt, 3.0 * mt * mt * t, 3.0 * mt * t * t, t * t * t];
                    points.push(Point::new(
                        w[0] * p0.x + w[1] * p1.x + w[2] * p2.x + w[3] * p3.x,
                        w[0] * p0.y + w[1] * p1.y + w[2] * p2.y + w[3] * p3.y,
                    ));
                }
            }
            if !*closed {
                points.push(anchors[anchors.len() - 1].0);
            }
            (points, *closed)
        })
        .collect()
}

/// The distance from `p` to the operand's outlines (lines and curves, not the region).
pub fn distance_to_operand(polylines: &[(Vec<Point>, bool)], p: Point) -> f64 {
    let mut best = f64::MAX;
    for (points, closed) in polylines {
        let n = points.len();
        let edges = if *closed { n } else { n - 1 };
        for i in 0..edges {
            best = best.min(distance_to_segment(p, points[i], points[(i + 1) % n]));
        }
    }
    best
}

/// Criterion 34 for Round: every point of the result lies `|d|` from the operand's outlines,
/// within 0.01 mm. Returns the largest deviation.
pub fn largest_deviation(operand: &[(Anchors, bool)], result: &OffsetOutlines, d: f64) -> f64 {
    let polylines = operand_polylines(operand);
    result
        .flattened(0.002)
        .iter()
        .flatten()
        .map(|&p| (distance_to_operand(&polylines, p) - d.abs()).abs())
        .fold(0.0, f64::max)
}

/// Criterion 33: consecutive nodes at least 0.001 mm apart, no node on the straight line between
/// two straight neighbours, `smooth` exactly where the tangents agree within 0.1°.
#[allow(clippy::many_single_char_names)]
pub fn check_node_rules(result: &OffsetOutlines) -> Result<(), String> {
    for outline in result.outlines() {
        let n = outline.len();
        if n < 2 {
            return Err("an outline has fewer than 2 nodes".into());
        }
        for i in 0..n {
            let (a, b, c) = (outline[(i + n - 1) % n], outline[i], outline[(i + 1) % n]);
            let gap = (c.point.x - b.point.x).hypot(c.point.y - b.point.y);
            if gap < 0.001 - 1e-12 {
                return Err(format!(
                    "nodes {:?} and {:?} are {gap} apart",
                    b.point, c.point
                ));
            }
            let straight_in = a.handle_out == Vec2::ZERO && b.handle_in == Vec2::ZERO;
            let straight_out = b.handle_out == Vec2::ZERO && c.handle_in == Vec2::ZERO;
            if straight_in && straight_out {
                let off = distance_to_segment(b.point, a.point, c.point);
                if off < 1e-6 {
                    return Err(format!("node {:?} lies on a straight line", b.point));
                }
            }
            let tangent_in = if b.handle_in == Vec2::ZERO {
                if a.handle_out == Vec2::ZERO || straight_in {
                    a.point.vector_to(b.point)
                } else {
                    a.point.translated(a.handle_out).vector_to(b.point)
                }
            } else {
                b.handle_in.negated()
            };
            let tangent_out = if b.handle_out == Vec2::ZERO {
                if c.handle_in == Vec2::ZERO || straight_out {
                    b.point.vector_to(c.point)
                } else {
                    b.point.vector_to(c.point.translated(c.handle_in))
                }
            } else {
                b.handle_out
            };
            let (u, v) = (
                tangent_in.scaled(1.0 / tangent_in.length()),
                tangent_out.scaled(1.0 / tangent_out.length()),
            );
            let sine = u.x * v.y - u.y * v.x;
            let cosine = u.x * v.x + u.y * v.y;
            let tangent_equal = cosine > 0.0 && sine.abs() <= 0.1_f64.to_radians().sin() + 1e-9;
            if tangent_equal != b.smooth
                && sine.abs() > 1e-6
                && (sine.abs() - 0.1_f64.to_radians().sin()).abs() > 1e-6
            {
                return Err(format!(
                    "node {:?}: smooth {} but tangents differ by {} degrees",
                    b.point,
                    b.smooth,
                    sine.asin().to_degrees()
                ));
            }
        }
    }
    Ok(())
}
