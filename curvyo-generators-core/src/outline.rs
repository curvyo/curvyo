//! Closed outlines built from lines, circular arcs and cubic Hermite fits (`Builder`), and the
//! helpers the gear and the box share.

use curvyo_document_core::Point;
use curvyo_extension_core::{ClosedPath, Cubic};

/// The largest sweep of one cubic arc: a 45 degree arc deviates from its circle by about 4e-6 of
/// the radius, well under the 0.01 mm of ADR 0080's tolerance for the radii this crate allows.
const MAX_ARC_SWEEP: f64 = std::f64::consts::FRAC_PI_4;

/// `centre + radius * (cos angle, sin angle)`.
pub(crate) fn polar(centre: Point, radius: f64, angle: f64) -> Point {
    Point::new(
        centre.x + radius * libm::cos(angle),
        centre.y + radius * libm::sin(angle),
    )
}

/// Builds one closed outline segment by segment.
pub(crate) struct Builder {
    start: Point,
    current: Point,
    segments: Vec<Cubic>,
}

impl Builder {
    pub(crate) fn new(start: Point) -> Self {
        Self {
            start,
            current: start,
            segments: Vec::new(),
        }
    }

    /// A straight segment, written as a cubic with its control points on the ends.
    pub(crate) fn line_to(&mut self, to: Point) {
        self.segments.push(Cubic {
            c1: self.current,
            c2: to,
            to,
        });
        self.current = to;
    }

    pub(crate) fn cubic_to(&mut self, c1: Point, c2: Point, to: Point) {
        self.segments.push(Cubic { c1, c2, to });
        self.current = to;
    }

    /// An arc of the circle `centre`, `radius` from the angle `from` to `to` (counter-clockwise in
    /// the maths sense when `to > from`), starting at the current point, which must be the circle's
    /// point at `from`; the end is the circle's point at `to` exactly.
    pub(crate) fn arc(&mut self, centre: Point, radius: f64, from: f64, to: f64) {
        let sweep = to - from;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let count = ((sweep.abs() / MAX_ARC_SWEEP).ceil() as usize).max(1);
        #[allow(clippy::cast_precision_loss)]
        let step = sweep / count as f64;
        let k = 4.0 / 3.0 * libm::tan(step / 4.0);
        for index in 0..count {
            #[allow(clippy::cast_precision_loss)]
            let a0 = from + step * index as f64;
            let a1 = a0 + step;
            let p0 = polar(centre, radius, a0);
            let p1 = if index + 1 == count {
                polar(centre, radius, to)
            } else {
                polar(centre, radius, a1)
            };
            let c1 = Point::new(
                p0.x - k * radius * libm::sin(a0),
                p0.y + k * radius * libm::cos(a0),
            );
            let c2 = Point::new(
                p1.x + k * radius * libm::sin(a1),
                p1.y - k * radius * libm::cos(a1),
            );
            self.cubic_to(c1, c2, p1);
        }
    }

    /// Ends the outline exactly on its start: the last segment's end point is replaced by the
    /// start, for a walk that came round to it up to rounding.
    pub(crate) fn snap_end(&mut self) {
        if let Some(last) = self.segments.last_mut() {
            last.to = self.start;
        }
        self.current = self.start;
    }

    /// The outline, closed with a straight segment when the current point is not the start.
    pub(crate) fn finish(mut self) -> ClosedPath {
        if self.current != self.start {
            let start = self.start;
            self.line_to(start);
        }
        ClosedPath {
            start: self.start,
            segments: self.segments,
        }
    }
}

/// `path` traversed the other way round (holes wind against their surrounding outline).
pub(crate) fn reversed(path: &ClosedPath) -> ClosedPath {
    let mut points = vec![path.start];
    points.extend(path.segments.iter().map(|segment| segment.to));
    let mut segments = Vec::with_capacity(path.segments.len());
    for (index, segment) in path.segments.iter().enumerate().rev() {
        segments.push(Cubic {
            c1: segment.c2,
            c2: segment.c1,
            to: points[index],
        });
    }
    ClosedPath {
        start: *points.last().unwrap_or(&path.start),
        segments,
    }
}

/// The polygon through `points` as a closed outline of line segments.
#[cfg(test)]
pub(crate) fn polygon(points: &[Point]) -> ClosedPath {
    let mut builder = Builder::new(points[0]);
    for point in &points[1..] {
        builder.line_to(*point);
    }
    builder.finish()
}

/// `path` moved by `(dx, dy)`.
pub(crate) fn translated(path: &ClosedPath, dx: f64, dy: f64) -> ClosedPath {
    let shift = |p: Point| Point::new(p.x + dx, p.y + dy);
    ClosedPath {
        start: shift(path.start),
        segments: path
            .segments
            .iter()
            .map(|s| Cubic {
                c1: shift(s.c1),
                c2: shift(s.c2),
                to: shift(s.to),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_arc_ends_exactly_on_the_circle_and_stays_within_a_millionth_of_it() {
        let centre = Point::new(1.0, 2.0);
        let mut builder = Builder::new(polar(centre, 100.0, 0.0));
        builder.arc(centre, 100.0, 0.0, std::f64::consts::PI);
        let path = builder.finish();
        assert_eq!(
            path.segments.len(),
            5,
            "4 arcs of 45 degrees plus the closing line"
        );
        for cubic in &path.segments[..4] {
            let d = (cubic.to.x - centre.x).hypot(cubic.to.y - centre.y);
            assert!((d - 100.0).abs() < 1e-9);
        }
    }

    #[test]
    fn reversing_twice_gives_the_same_outline_and_flips_the_walk() {
        let path = polygon(&[
            Point::new(0.0, 0.0),
            Point::new(4.0, 0.0),
            Point::new(4.0, 3.0),
        ]);
        let back = reversed(&reversed(&path));
        assert_eq!(back.start, path.start);
        assert_eq!(back.segments, path.segments);
        assert_eq!(reversed(&path).segments[0].to, Point::new(4.0, 3.0));
    }
}
