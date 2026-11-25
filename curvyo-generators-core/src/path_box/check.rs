//! Reads a stored outline as the clockwise polygon the box is built from (ADR 0083 §2).

use curvyo_document_core::{Point, Vec2};
use curvyo_extension_core::{ClosedPath, Cubic, Warning};
use curvyo_geometry_core::{Outline, OutlineTriple, cut_parameters};

use super::classify::starts;

/// Two points closer than this (mm) are one.
const SAME_POINT_MM: f64 = 1e-6;

/// The default outline: a coffin of 100 x 60 mm, six straight edges (ADR 0083 §2, Proposal).
pub(crate) fn default_outline() -> ClosedPath {
    let corners = [
        (-50.0, -10.0),
        (-25.0, -30.0),
        (50.0, -20.0),
        (50.0, 20.0),
        (-25.0, 30.0),
        (-50.0, 10.0),
    ];
    let points: Vec<Point> = corners.iter().map(|&(x, y)| Point::new(x, y)).collect();
    ClosedPath {
        start: points[0],
        segments: (0..points.len())
            .map(|i| {
                let to = points[(i + 1) % points.len()];
                Cubic {
                    c1: points[i],
                    c2: to,
                    to,
                }
            })
            .collect(),
    }
}

/// What the generator builds from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Checked {
    /// The corner points, clockwise on screen, no two neighbours equal.
    pub(crate) points: Vec<Point>,
    /// The enclosed area (mm²).
    pub(crate) area: f64,
    /// `outline_crosses` or `outline_curved`.
    pub(crate) warnings: Vec<Warning>,
}

/// One anchor per cubic start, handles relative to the node.
pub(crate) fn triples(path: &ClosedPath) -> Vec<OutlineTriple> {
    let from = starts(path);
    let count = path.segments.len();
    (0..count)
        .map(|i| {
            let before = &path.segments[(i + count - 1) % count];
            let point = from[i];
            let towards = |q: Point| Vec2::new(q.x - point.x, q.y - point.y);
            (point, towards(before.c2), towards(path.segments[i].c1))
        })
        .collect()
}

/// The signed area of a polygon (positive: clockwise on screen, y down).
pub(crate) fn signed_area(points: &[Point]) -> f64 {
    let count = points.len();
    (0..count)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % count]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

fn crosses(path: &ClosedPath) -> bool {
    let anchors = triples(path);
    cut_parameters(&[Outline::new(&anchors, true)])
        .iter()
        .any(|cuts| !cuts.is_empty())
}

fn warning(id: &'static str) -> Warning {
    Warning {
        id,
        pieces: Vec::new(),
        value: None,
        limit: None,
    }
}

fn polygon_of(path: &ClosedPath) -> Vec<Point> {
    let mut points: Vec<Point> = Vec::new();
    for point in starts(path) {
        if points
            .last()
            .is_none_or(|last| libm::hypot(last.x - point.x, last.y - point.y) > SAME_POINT_MM)
        {
            points.push(point);
        }
    }
    if points.len() > 1
        && libm::hypot(
            points[0].x - points[points.len() - 1].x,
            points[0].y - points[points.len() - 1].y,
        ) <= SAME_POINT_MM
    {
        points.pop();
    }
    points
}

/// Checks `outline`: crossing, degenerate and curved outlines, then the polygon and its area.
pub(crate) fn check(outline: &ClosedPath) -> Checked {
    let mut warnings = Vec::new();
    let mut path = outline.clone();
    let mut points = polygon_of(&path);
    if points.len() < 3 || signed_area(&points).abs() < 1e-9 || crosses(&path) {
        warnings.push(warning("outline_crosses"));
        path = default_outline();
        points = polygon_of(&path);
    } else if super::classify::classify(&path, 0.0).curved_cubics() > 0 {
        warnings.push(warning("outline_curved"));
    }
    let mut area = signed_area(&points);
    if area < 0.0 {
        points.reverse();
        area = -area;
    }
    Checked {
        points,
        area,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn polygon(points: &[(f64, f64)]) -> ClosedPath {
        let pts: Vec<Point> = points.iter().map(|&(x, y)| Point::new(x, y)).collect();
        ClosedPath {
            start: pts[0],
            segments: (0..pts.len())
                .map(|i| {
                    let to = pts[(i + 1) % pts.len()];
                    Cubic {
                        c1: pts[i],
                        c2: to,
                        to,
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn a_square_has_no_cut_and_a_bow_tie_has_some() {
        assert!(!crosses(&polygon(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0)
        ])));
        assert!(crosses(&polygon(&[
            (0.0, 0.0),
            (10.0, 10.0),
            (10.0, 0.0),
            (0.0, 10.0)
        ])));
    }

    #[test]
    fn a_counter_clockwise_outline_is_reversed_to_clockwise() {
        let ccw = polygon(&[(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)]);
        let checked = check(&ccw);
        assert!(signed_area(&checked.points) > 0.0);
        assert!((checked.area - 100.0).abs() < 1e-9);
        assert_eq!(checked.warnings, []);
    }

    #[test]
    fn a_crossing_outline_reads_as_the_default_coffin_with_a_warning() {
        let bow = polygon(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]);
        let checked = check(&bow);
        assert_eq!(checked.warnings[0].id, "outline_crosses");
        assert_eq!(checked.points.len(), 6);
    }

    #[test]
    fn a_curved_cubic_is_built_as_its_chord_with_a_warning() {
        let mut path = polygon(&[(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)]);
        path.segments[0].c1 = Point::new(10.0, -8.0);
        path.segments[0].c2 = Point::new(30.0, -8.0);
        let checked = check(&path);
        assert_eq!(checked.warnings[0].id, "outline_curved");
        assert_eq!(checked.points.len(), 4);
    }

    #[test]
    fn the_default_coffin_is_valid_100_by_60() {
        let checked = check(&default_outline());
        assert_eq!(checked.warnings, []);
        let xs: Vec<f64> = checked.points.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = checked.points.iter().map(|p| p.y).collect();
        let span = |v: &[f64]| {
            v.iter().copied().fold(f64::MIN, f64::max) - v.iter().copied().fold(f64::MAX, f64::min)
        };
        assert!((span(&xs) - 100.0).abs() < 1e-9 && (span(&ys) - 60.0).abs() < 1e-9);
    }
}
