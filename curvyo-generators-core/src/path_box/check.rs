//! Reads a stored outline as the clockwise closed path the box is built from, with its area from
//! the curve (ADR 0083 §2, §13.1).

use curvyo_document_core::{Point, Vec2};
use curvyo_extension_core::{ClosedPath, Cubic, Warning};
use curvyo_geometry_core::{Outline, OutlineTriple, cut_parameters, outline_area_mm2};

use super::classify::{classify, starts};
use crate::outline::reversed;

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
    /// The outline, clockwise on screen (ADR 0083 §13.1).
    pub(crate) path: ClosedPath,
    /// Whether any cubic stays off its chord by more than the line tolerance.
    pub(crate) curved: bool,
    /// The corner points of an outline of straight cubics, clockwise, no two neighbours equal.
    pub(crate) points: Vec<Point>,
    /// The enclosed area (mm²).
    pub(crate) area: f64,
    /// `outline_crosses`.
    pub(crate) warnings: Vec<Warning>,
}

/// The signed area the outline encloses, from the curve (geometry-core, flattened within its
/// tolerance): positive for a clockwise walk on screen, the sign of `signed_area`.
pub(crate) fn curve_area(path: &ClosedPath) -> f64 {
    let anchors = triples(path);
    outline_area_mm2(&Outline::new(&anchors, true))
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

/// The outline without zero-length cubics: an import may close with a `C` onto its start before
/// the `Z`, which must never read as a corner.
pub(crate) fn without_zero_segments(outline: &ClosedPath) -> ClosedPath {
    let near = |a: Point, b: Point| libm::hypot(a.x - b.x, a.y - b.y) <= SAME_POINT_MM;
    let mut from = outline.start;
    let mut segments = Vec::with_capacity(outline.segments.len());
    for c in &outline.segments {
        let zero = near(from, c.to) && near(from, c.c1) && near(from, c.c2);
        if !zero {
            segments.push(*c);
        }
        from = c.to;
    }
    ClosedPath {
        start: outline.start,
        segments,
    }
}

/// Checks `outline`: a crossing or degenerate outline reads as the default coffin; the winding is
/// made clockwise (ADR 0083 §13.1).
pub(crate) fn check(outline: &ClosedPath) -> Checked {
    let mut warnings = Vec::new();
    let mut path = without_zero_segments(outline);
    let mut curved = classify(&path, 0.0).curved_cubics() > 0;
    let mut points = polygon_of(&path);
    let degenerate = if curved {
        curve_area(&path).abs() < 1e-9
    } else {
        points.len() < 3 || signed_area(&points).abs() < 1e-9
    };
    if degenerate || crosses(&path) {
        warnings.push(warning("outline_crosses"));
        path = default_outline();
        points = polygon_of(&path);
        curved = false;
    }
    let (winding, area) = if curved {
        {
            let area = curve_area(&path);
            (area, area.abs())
        }
    } else {
        let area = signed_area(&points);
        (area, area.abs())
    };
    if winding < 0.0 {
        points.reverse();
        path = reversed(&path);
    }
    Checked {
        path,
        curved,
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
    fn a_curved_cubic_is_a_valid_curved_outline_without_warning() {
        let mut path = polygon(&[(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)]);
        path.segments[0].c1 = Point::new(10.0, -8.0);
        path.segments[0].c2 = Point::new(30.0, -8.0);
        let checked = check(&path);
        assert_eq!(checked.warnings, []);
        assert!(checked.curved);
        assert!(
            checked.area > 1600.0,
            "the bulge adds area: {}",
            checked.area
        );
    }

    #[test]
    fn the_area_comes_from_the_curve_a_circle_of_four_cubics_has_pi_r_squared() {
        let checked = check(&crate::path_box::fixtures::circle(30.0));
        assert!(
            (checked.area - std::f64::consts::PI * 900.0).abs() < 2.0,
            "{}",
            checked.area
        );
        assert_eq!(checked.warnings, []);
    }

    #[test]
    fn a_counter_clockwise_curved_outline_is_reversed_to_clockwise() {
        let heart = crate::path_box::fixtures::heart();
        let flipped = crate::outline::reversed(&heart);
        let (a, b) = (check(&heart), check(&flipped));
        assert!((a.area - b.area).abs() < 1e-6);
        assert!(curve_area(&a.path) > 0.0 && curve_area(&b.path) > 0.0);
    }

    #[test]
    fn a_d_and_a_lens_are_valid_and_a_zero_area_curve_is_not() {
        use crate::path_box::fixtures::{curve, p};
        let d = curve(
            p(0.0, 0.0),
            &[
                ((30.0, 0.0), (30.0, 40.0), (0.0, 40.0)),
                ((0.0, 40.0), (0.0, 0.0), (0.0, 0.0)),
            ],
        );
        assert_eq!(check(&d).warnings, []);
        let lens = curve(
            p(0.0, 0.0),
            &[
                ((10.0, -20.0), (30.0, -20.0), (40.0, 0.0)),
                ((30.0, 20.0), (10.0, 20.0), (0.0, 0.0)),
            ],
        );
        assert_eq!(check(&lens).warnings, []);
        let flat = curve(
            p(0.0, 0.0),
            &[
                ((10.0, 5.0), (30.0, 5.0), (40.0, 0.0)),
                ((30.0, 5.0), (10.0, 5.0), (0.0, 0.0)),
            ],
        );
        assert_eq!(check(&flat).warnings[0].id, "outline_crosses");
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
