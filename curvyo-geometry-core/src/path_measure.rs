//! Arc length, point, tangent and nearest arc length along one outline
//! (`specs/0080-text-on-path/adrs.md` decision 4).
//!
//! The outline is measured on its exact cubics: a table of cumulative segment lengths is built
//! once (`kurbo` arclen per segment), and a point at an arc length is found by a binary search
//! over the table and the inverse arclen of one segment, so tangents are exact. No `kurbo` type
//! crosses the public API (the rule of `segment.rs`).

use curvyo_document_core::{Length, Point, Vec2};
use kurbo::{
    CubicBez, ParamCurve, ParamCurveArclen, ParamCurveDeriv, ParamCurveExtrema, ParamCurveNearest,
    Point as KurboPoint,
};

use crate::interior::OutlineTriple;

/// The accuracy of every arc length (mm).
const ARCLEN_ACCURACY_MM: f64 = 1e-5;
/// A segment shorter than this (mm) carries no length and no tangent.
const ZERO_LENGTH_MM: f64 = 1e-9;
/// Two heights closer than this (mm) are the same height (the topmost point).
const SAME_HEIGHT_MM: f64 = 1e-6;
/// The accuracy of the nearest-point search (mm).
const NEAREST_ACCURACY_MM: f64 = 1e-6;

/// A point on the outline and the unit tangent there, in carrier direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathSample {
    /// The point, document mm.
    pub point: Point,
    /// The unit tangent in the direction arc length grows.
    pub tangent: Vec2,
}

/// One outline measured: its segments with nonzero length and their cumulative lengths.
#[derive(Debug, Clone)]
pub struct PathMeasure {
    segments: Vec<CubicBez>,
    /// The arc length at the end of each segment.
    ends: Vec<f64>,
    closed: bool,
    /// The first point, where an outline without length sits.
    start: Point,
}

fn kp(point: Point) -> KurboPoint {
    KurboPoint::new(point.x, point.y)
}

fn pt(point: KurboPoint) -> Point {
    Point::new(point.x, point.y)
}

fn unit(v: kurbo::Vec2) -> Option<Vec2> {
    let length = v.hypot();
    (length > 1e-12 && length.is_finite()).then(|| Vec2::new(v.x / length, v.y / length))
}

/// The unit tangent of `cubic` at `t`; at a zero derivative (a cusp or a collapsed handle) the
/// chord to the next distinct point, else the whole chord.
fn tangent_at(cubic: &CubicBez, t: f64) -> Vec2 {
    if let Some(tangent) = unit(cubic.deriv().eval(t).to_vec2()) {
        return tangent;
    }
    let here = cubic.eval(t);
    let ahead = cubic.eval((t + 1e-3).min(1.0));
    let behind = cubic.eval((t - 1e-3).max(0.0));
    unit(ahead - here)
        .or_else(|| unit(here - behind))
        .or_else(|| unit(cubic.p3 - cubic.p0))
        .unwrap_or(Vec2::new(1.0, 0.0))
}

impl PathMeasure {
    /// Measures the outline of `anchors` (point, handle in, handle out, handles relative), closed
    /// back to its first anchor when `closed`.
    #[must_use]
    pub fn new(anchors: &[OutlineTriple], closed: bool) -> Self {
        let count = anchors.len();
        let pairs = if closed && count > 1 {
            count
        } else {
            count.saturating_sub(1)
        };
        let mut segments = Vec::with_capacity(pairs);
        let mut ends = Vec::with_capacity(pairs);
        let mut total = 0.0;
        for i in 0..pairs {
            let (p0, _, out) = anchors[i];
            let (p3, handle_in, _) = anchors[(i + 1) % count];
            let cubic = CubicBez::new(
                kp(p0),
                kp(p0.translated(out)),
                kp(p3.translated(handle_in)),
                kp(p3),
            );
            let length = cubic.arclen(ARCLEN_ACCURACY_MM);
            if length.is_finite() && length > ZERO_LENGTH_MM {
                total += length;
                segments.push(cubic);
                ends.push(total);
            }
        }
        Self {
            segments,
            ends,
            closed,
            start: anchors.first().map_or(Point::new(0.0, 0.0), |a| a.0),
        }
    }

    /// The whole length.
    #[must_use]
    pub fn length(&self) -> Length {
        Length::from_mm(self.ends.last().copied().unwrap_or(0.0))
    }

    /// Whether the outline closes back to its start.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    /// How many segments with length it has.
    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// The arc length `s` held to the outline: modulo the length on a closed outline, as is on an
    /// open one (beyond its ends it continues straight).
    #[must_use]
    pub fn wrapped(&self, s: Length) -> Length {
        let length = self.length().as_mm();
        if self.closed && length > 0.0 {
            Length::from_mm(s.as_mm().rem_euclid(length))
        } else {
            s
        }
    }

    /// The point and tangent at arc length `s`. At a segment boundary the incoming segment's
    /// tangent (a corner takes the side before it); before the start and past the end of an open
    /// outline the straight extension of the end tangents; on a closed outline `s` modulo the
    /// length. An outline without length lies straight along +x at its first point.
    #[must_use]
    pub fn at(&self, s: Length) -> PathSample {
        let s = self.wrapped(s).as_mm();
        let total = self.length().as_mm();
        let (Some(first), Some(last)) = (self.segments.first(), self.segments.last()) else {
            return PathSample {
                point: self.start.translated(Vec2::new(s, 0.0)),
                tangent: Vec2::new(1.0, 0.0),
            };
        };
        if s <= 0.0 {
            let tangent = tangent_at(first, 0.0);
            return PathSample {
                point: pt(first.p0).translated(tangent.scaled(s)),
                tangent,
            };
        }
        if s >= total {
            let tangent = tangent_at(last, 1.0);
            return PathSample {
                point: pt(last.p3).translated(tangent.scaled(s - total)),
                tangent,
            };
        }
        // The first segment whose end is at or past `s`: a boundary belongs to the one before.
        let index = self.ends.partition_point(|end| *end < s);
        let index = index.min(self.segments.len() - 1);
        let before = if index == 0 {
            0.0
        } else {
            self.ends[index - 1]
        };
        let cubic = &self.segments[index];
        let t = cubic.inv_arclen(s - before, ARCLEN_ACCURACY_MM);
        PathSample {
            point: pt(cubic.eval(t)),
            tangent: tangent_at(cubic, t),
        }
    }

    /// The arc length of the outline point nearest to `query`, in `0 ..= length`.
    #[must_use]
    pub fn nearest(&self, query: Point) -> Length {
        let mut best = (f64::INFINITY, 0.0);
        let mut before = 0.0;
        for (cubic, end) in self.segments.iter().zip(&self.ends) {
            let nearest = cubic.nearest(kp(query), NEAREST_ACCURACY_MM);
            if nearest.distance_sq < best.0 {
                let along = cubic.subsegment(0.0..nearest.t).arclen(ARCLEN_ACCURACY_MM);
                best = (nearest.distance_sq, before + along);
            }
            before = *end;
        }
        Length::from_mm(best.1)
    }

    /// The arc length of the topmost point (least y on screen); of several, the first in carrier
    /// direction, and on a flat top edge its middle.
    #[must_use]
    pub fn topmost(&self) -> Length {
        let mut top = f64::INFINITY;
        for cubic in &self.segments {
            top = top.min(cubic.bounding_box().y0);
        }
        let mut before = 0.0;
        // A flat run at the top (its start and end arc lengths), or a top point at a segment's
        // end, which the next segment may continue flat.
        let mut run: Option<(f64, f64)> = None;
        let mut at_end: Option<f64> = None;
        for (cubic, end) in self.segments.iter().zip(&self.ends) {
            let bounds = cubic.bounding_box();
            if bounds.y1 - top <= SAME_HEIGHT_MM {
                run = Some(run.map_or((before, *end), |(start, _)| (start, *end)));
            } else if let Some((start, finish)) = run {
                return Length::from_mm(f64::midpoint(start, finish));
            } else if let Some(s) = at_end {
                return Length::from_mm(s);
            } else if bounds.y0 - top <= SAME_HEIGHT_MM {
                let t = std::iter::once(0.0)
                    .chain(cubic.extrema())
                    .chain(std::iter::once(1.0))
                    .find(|t| cubic.eval(*t).y - top <= SAME_HEIGHT_MM)
                    .unwrap_or(0.0);
                let s = before + cubic.subsegment(0.0..t).arclen(ARCLEN_ACCURACY_MM);
                if t < 1.0 {
                    return Length::from_mm(s);
                }
                at_end = Some(s);
            }
            before = *end;
        }
        Length::from_mm(
            run.map(|(start, finish)| f64::midpoint(start, finish))
                .or(at_end)
                .unwrap_or(0.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn corner(x: f64, y: f64) -> OutlineTriple {
        (Point::new(x, y), Vec2::ZERO, Vec2::ZERO)
    }

    /// A circle of radius `r` about `(cx, cy)`, four cubics, clockwise on screen from 3 o'clock
    /// (the ellipse outline's start and direction).
    fn circle(cx: f64, cy: f64, r: f64) -> Vec<OutlineTriple> {
        let k = 0.552_284_749_830_793_4 * r;
        vec![
            (
                Point::new(cx + r, cy),
                Vec2::new(0.0, -k),
                Vec2::new(0.0, k),
            ),
            (
                Point::new(cx, cy + r),
                Vec2::new(k, 0.0),
                Vec2::new(-k, 0.0),
            ),
            (
                Point::new(cx - r, cy),
                Vec2::new(0.0, k),
                Vec2::new(0.0, -k),
            ),
            (
                Point::new(cx, cy - r),
                Vec2::new(-k, 0.0),
                Vec2::new(k, 0.0),
            ),
        ]
    }

    fn close(a: Point, b: Point) -> bool {
        a.vector_to(b).length() < 1e-4
    }

    #[test]
    fn a_line_measures_its_length_and_points() {
        let measure = PathMeasure::new(&[corner(0.0, 0.0), corner(100.0, 0.0)], false);
        assert!((measure.length().as_mm() - 100.0).abs() < 1e-9);
        let sample = measure.at(Length::from_mm(25.0));
        assert!(close(sample.point, Point::new(25.0, 0.0)));
        assert!(close(
            Point::new(sample.tangent.x, sample.tangent.y),
            Point::new(1.0, 0.0)
        ));
    }

    /// Criterion 26: past the ends of an open carrier the text continues straight.
    #[test]
    fn an_open_outline_continues_straight_beyond_its_ends() {
        let measure = PathMeasure::new(&[corner(0.0, 0.0), corner(10.0, 0.0)], false);
        assert!(close(
            measure.at(Length::from_mm(15.0)).point,
            Point::new(15.0, 0.0)
        ));
        assert!(close(
            measure.at(Length::from_mm(-5.0)).point,
            Point::new(-5.0, 0.0)
        ));
    }

    /// Criterion 26: a closed carrier wraps.
    #[test]
    fn a_closed_outline_wraps() {
        let measure = PathMeasure::new(
            &[
                corner(0.0, 0.0),
                corner(10.0, 0.0),
                corner(10.0, 10.0),
                corner(0.0, 10.0),
            ],
            true,
        );
        assert!((measure.length().as_mm() - 40.0).abs() < 1e-9);
        assert!(close(
            measure.at(Length::from_mm(45.0)).point,
            Point::new(5.0, 0.0)
        ));
        assert!(close(
            measure.at(Length::from_mm(-5.0)).point,
            Point::new(0.0, 5.0)
        ));
    }

    /// Criterion 11: at a corner the incoming side's tangent.
    #[test]
    fn a_corner_takes_the_incoming_tangent() {
        let measure = PathMeasure::new(
            &[corner(0.0, 0.0), corner(10.0, 0.0), corner(10.0, 10.0)],
            false,
        );
        let at_corner = measure.at(Length::from_mm(10.0));
        assert!(close(at_corner.point, Point::new(10.0, 0.0)));
        assert!((at_corner.tangent.x - 1.0).abs() < 1e-9);
        let after = measure.at(Length::from_mm(10.5));
        assert!((after.tangent.y - 1.0).abs() < 1e-9);
    }

    /// The four-cubic circle is exact at its anchors; a quarter of its length is 6 o'clock.
    #[test]
    fn a_circle_is_exact_at_its_anchors() {
        let measure = PathMeasure::new(&circle(100.0, 100.0, 50.0), true);
        let length = measure.length().as_mm();
        assert!((length - 2.0 * PI * 50.0).abs() < 0.05, "{length}");
        let bottom = measure.at(Length::from_mm(length / 4.0));
        assert!(close(bottom.point, Point::new(100.0, 150.0)));
        assert!(
            (bottom.tangent.x + 1.0).abs() < 1e-6,
            "clockwise: leftwards at the bottom"
        );
        let top = measure.topmost().as_mm();
        assert!((top - 0.75 * length).abs() < 1e-4, "{top}");
        let near = measure.nearest(Point::new(100.0, 20.0)).as_mm();
        assert!((near - 0.75 * length).abs() < 1e-4);
    }

    /// The topmost point of a rectangle is the middle of its top edge; of a triangle its apex.
    #[test]
    fn the_topmost_point_is_the_middle_of_a_flat_top() {
        let rect = PathMeasure::new(
            &[
                corner(0.0, 0.0),
                corner(40.0, 0.0),
                corner(40.0, 10.0),
                corner(0.0, 10.0),
            ],
            true,
        );
        assert!((rect.topmost().as_mm() - 20.0).abs() < 1e-9);
        let triangle = PathMeasure::new(
            &[corner(0.0, 10.0), corner(5.0, 0.0), corner(10.0, 10.0)],
            true,
        );
        let apex = triangle.at(triangle.topmost()).point;
        assert!(close(apex, Point::new(5.0, 0.0)));
    }

    /// A collapsed handle or a lone point never gives a NaN tangent; no length lies along +x.
    #[test]
    fn degenerate_outlines_are_measured_without_nan() {
        let cusp = PathMeasure::new(
            &[
                (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::ZERO),
                (Point::new(10.0, 0.0), Vec2::ZERO, Vec2::ZERO),
            ],
            false,
        );
        assert!(cusp.at(Length::from_mm(0.0)).tangent.x.is_finite());
        let dot = PathMeasure::new(&[corner(3.0, 4.0)], false);
        assert_eq!(dot.length(), Length::from_mm(0.0));
        let sample = dot.at(Length::from_mm(2.0));
        assert!(close(sample.point, Point::new(5.0, 4.0)));
        let empty = PathMeasure::new(&[], true);
        assert!(empty.at(Length::from_mm(1.0)).point.x.is_finite());
    }
}
