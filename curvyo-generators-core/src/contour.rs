//! A closed contour of lines and circular arcs with the kerf offset of ADR 0082 §2.
//!
//! Screen coordinates (y grows downward). Outer contours run clockwise on screen, holes
//! counter-clockwise, so the offset is always half the kerf to the left of the walk: outer
//! contours move out, holes in. An arc's `sweep` is positive when its angle grows, which is
//! clockwise on screen.

use curvyo_document_core::Point;
use curvyo_extension_core::ClosedPath;

use crate::outline::{Builder, polar};

/// Two directions closer than this (radians) are tangent.
const TANGENT: f64 = 1e-9;
/// A miter longer than this many times the offset distance is cut square (ADR 0082 §2: 2 k, the
/// offset being k / 2).
const MITER_LIMIT: f64 = 4.0;
/// An arc whose offset radius would drop below this (mm) keeps this radius.
const MIN_RADIUS: f64 = 1e-6;

/// One piece of a contour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Seg {
    /// A straight piece.
    Line { from: Point, to: Point },
    /// A circular arc from the angle `from`, turning by `sweep`.
    Arc {
        centre: Point,
        radius: f64,
        from: f64,
        sweep: f64,
    },
}

impl Seg {
    pub(crate) fn start(&self) -> Point {
        match *self {
            Self::Line { from, .. } => from,
            Self::Arc {
                centre,
                radius,
                from,
                ..
            } => polar(centre, radius, from),
        }
    }

    pub(crate) fn end(&self) -> Point {
        match *self {
            Self::Line { to, .. } => to,
            Self::Arc {
                centre,
                radius,
                from,
                sweep,
            } => polar(centre, radius, from + sweep),
        }
    }

    /// The unit direction of travel at the start (`false`) or the end (`true`).
    fn direction(&self, at_end: bool) -> (f64, f64) {
        match *self {
            Self::Line { from, to } => unit(to.x - from.x, to.y - from.y),
            Self::Arc { from, sweep, .. } => {
                let angle = if at_end { from + sweep } else { from };
                let turn = sweep.signum();
                (-turn * libm::sin(angle), turn * libm::cos(angle))
            }
        }
    }

    /// The piece moved `distance` to the left of its walk.
    fn shifted(&self, distance: f64) -> Self {
        match *self {
            Self::Line { from, to } => {
                let (ux, uy) = unit(to.x - from.x, to.y - from.y);
                let (nx, ny) = (uy * distance, -ux * distance);
                Self::Line {
                    from: Point::new(from.x + nx, from.y + ny),
                    to: Point::new(to.x + nx, to.y + ny),
                }
            }
            Self::Arc {
                centre,
                radius,
                from,
                sweep,
            } => Self::Arc {
                centre,
                radius: (radius + sweep.signum() * distance).max(MIN_RADIUS),
                from,
                sweep,
            },
        }
    }

    /// The piece with new end points, an arc keeping its centre, radius and turning direction.
    fn with_ends(&self, start: Point, end: Point) -> Self {
        match *self {
            Self::Line { .. } => Self::Line {
                from: start,
                to: end,
            },
            Self::Arc {
                centre,
                radius,
                sweep,
                ..
            } => {
                let from = libm::atan2(start.y - centre.y, start.x - centre.x);
                let to = libm::atan2(end.y - centre.y, end.x - centre.x);
                let mut turn = to - from;
                let tau = std::f64::consts::TAU;
                while turn * sweep < 0.0 {
                    turn += tau * sweep.signum();
                }
                while turn.abs() > tau {
                    turn -= tau * sweep.signum();
                }
                Self::Arc {
                    centre,
                    radius,
                    from,
                    sweep: turn,
                }
            }
        }
    }
}

fn unit(dx: f64, dy: f64) -> (f64, f64) {
    let length = libm::hypot(dx, dy);
    if length == 0.0 {
        (1.0, 0.0)
    } else {
        (dx / length, dy / length)
    }
}

/// A closed contour: each piece starts where the one before it ends, the last one at the first's
/// start.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Contour {
    pub(crate) segs: Vec<Seg>,
}

impl Contour {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A line to `to` from the current point. A zero-length line is dropped.
    pub(crate) fn line_to(&mut self, from: Point, to: Point) {
        if libm::hypot(to.x - from.x, to.y - from.y) > 1e-9 {
            self.segs.push(Seg::Line { from, to });
        }
    }

    /// An arc of `centre`, `radius` from the angle `from`, turning by `sweep`. A full circle is
    /// split into quarter turns, so no piece is ambiguous.
    pub(crate) fn arc(&mut self, centre: Point, radius: f64, from: f64, sweep: f64) {
        let quarter = std::f64::consts::FRAC_PI_2;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let parts = ((sweep.abs() / (quarter + 1e-9)).ceil() as usize).max(1);
        #[allow(clippy::cast_precision_loss)]
        let step = sweep / parts as f64;
        for index in 0..parts {
            #[allow(clippy::cast_precision_loss)]
            let start = from + step * index as f64;
            self.segs.push(Seg::Arc {
                centre,
                radius,
                from: start,
                sweep: step,
            });
        }
    }

    /// The smallest rectangle round the contour: `(min x, min y, max x, max y)`.
    pub(crate) fn bounds(&self) -> (f64, f64, f64, f64) {
        let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        let mut add = |point: Point| {
            b = (
                b.0.min(point.x),
                b.1.min(point.y),
                b.2.max(point.x),
                b.3.max(point.y),
            );
        };
        for seg in &self.segs {
            add(seg.start());
            add(seg.end());
            if let Seg::Arc {
                centre,
                radius,
                from,
                sweep,
            } = *seg
            {
                // The extremes of the circle that the arc passes.
                for quarter in 0..4 {
                    let angle = f64::from(quarter) * std::f64::consts::FRAC_PI_2;
                    let (lo, hi) = (from.min(from + sweep), from.max(from + sweep));
                    let turns = ((lo - angle) / std::f64::consts::TAU).ceil();
                    if angle + turns * std::f64::consts::TAU <= hi {
                        add(polar(centre, radius, angle));
                    }
                }
            }
        }
        b
    }

    /// Appends a piece that starts where the walk is.
    pub(crate) fn push(&mut self, seg: Seg) {
        self.segs.push(seg);
    }

    /// The same contour walked the other way (a hole's winding).
    pub(crate) fn reversed(&self) -> Self {
        let segs = self.segs.iter().rev().map(Seg::reversed).collect();
        Self { segs }
    }

    /// The contour with every piece moved `distance` to the left of the walk and neighbouring
    /// pieces joined: tangent joins stay tangent, any other corner is the intersection of the two
    /// moved pieces, or cut square when the miter is longer than `MITER_LIMIT` offsets.
    pub(crate) fn offset(&self, distance: f64) -> Self {
        let count = self.segs.len();
        if distance == 0.0 || count < 2 {
            return self.clone();
        }
        let moved: Vec<Seg> = self.segs.iter().map(|seg| seg.shifted(distance)).collect();
        // joints[i] joins piece i to piece i + 1: the end of i and the start of i + 1.
        let joints: Vec<(Point, Point)> = (0..count)
            .map(|i| {
                let next = (i + 1) % count;
                join(
                    &self.segs[i],
                    &self.segs[next],
                    &moved[i],
                    &moved[next],
                    distance,
                )
            })
            .collect();
        let mut segs = Vec::with_capacity(count * 2);
        for i in 0..count {
            let before = (i + count - 1) % count;
            let (start, end) = (joints[before].1, joints[i].0);
            segs.push(moved[i].with_ends(start, end));
            let next_start = joints[i].1;
            if libm::hypot(next_start.x - end.x, next_start.y - end.y) > 1e-9 {
                segs.push(Seg::Line {
                    from: end,
                    to: next_start,
                });
            }
        }
        Self { segs }
    }

    /// The contour as a closed outline: lines and arcs through the shared builder.
    pub(crate) fn to_path(&self) -> ClosedPath {
        let Some(first) = self.segs.first() else {
            return ClosedPath {
                start: Point::new(0.0, 0.0),
                segments: Vec::new(),
            };
        };
        let mut builder = Builder::new(first.start());
        for seg in &self.segs {
            match *seg {
                Seg::Line { to, .. } => builder.line_to(to),
                Seg::Arc {
                    centre,
                    radius,
                    from,
                    sweep,
                } => builder.arc(centre, radius, from, from + sweep),
            }
        }
        builder.snap_end();
        builder.finish()
    }
}

impl Seg {
    /// The piece walked the other way.
    pub(crate) fn reversed(&self) -> Self {
        match *self {
            Self::Line { from, to } => Self::Line { from: to, to: from },
            Self::Arc {
                centre,
                radius,
                from,
                sweep,
            } => Self::Arc {
                centre,
                radius,
                from: from + sweep,
                sweep: -sweep,
            },
        }
    }

    /// For an arc: the angle within its range at which it crosses the horizontal line `y`
    /// (the line-arc intersection the wave slots need), `None` when it does not.
    pub(crate) fn angle_at_y(&self, y: f64) -> Option<f64> {
        let Self::Arc {
            centre,
            radius,
            from,
            sweep,
        } = *self
        else {
            return None;
        };
        let sine = (y - centre.y) / radius;
        if sine.abs() > 1.0 {
            return None;
        }
        let (low, high) = (from.min(from + sweep), from.max(from + sweep));
        let middle = f64::midpoint(low, high);
        let tau = std::f64::consts::TAU;
        [libm::asin(sine), std::f64::consts::PI - libm::asin(sine)]
            .into_iter()
            .map(|angle| angle + tau * ((middle - angle) / tau).round())
            .find(|angle| *angle >= low - 1e-9 && *angle <= high + 1e-9)
    }
}

/// Where the moved pieces `a` and `b` (originals `oa`, `ob`) meet: the end of `a` and the start
/// of `b`. Equal points for a tangent join or an intersection, two points for a square cut.
fn join(oa: &Seg, ob: &Seg, a: &Seg, b: &Seg, distance: f64) -> (Point, Point) {
    let corner = oa.end();
    let (da, db) = (oa.direction(true), ob.direction(false));
    let cross = da.0 * db.1 - da.1 * db.0;
    let dot = da.0 * db.0 + da.1 * db.1;
    if cross.abs() < TANGENT && dot > 0.0 {
        let point = a.end();
        return (point, point);
    }
    match intersect(a, b, corner) {
        Some(point)
            if libm::hypot(point.x - corner.x, point.y - corner.y)
                <= MITER_LIMIT * distance.abs() + 1e-9 =>
        {
            (point, point)
        }
        _ => (a.end(), b.start()),
    }
}

/// The intersection of the carriers of `a` and `b` nearest to `near`.
fn intersect(a: &Seg, b: &Seg, near: Point) -> Option<Point> {
    let candidates: Vec<Point> = match (*a, *b) {
        (
            Seg::Line { from, to },
            Seg::Line {
                from: other,
                to: other_end,
            },
        ) => line_line(from, to, other, other_end).into_iter().collect(),
        (Seg::Line { from, to }, Seg::Arc { centre, radius, .. })
        | (Seg::Arc { centre, radius, .. }, Seg::Line { from, to }) => {
            line_circle(from, to, centre, radius)
        }
        (
            Seg::Arc {
                centre: c1,
                radius: r1,
                ..
            },
            Seg::Arc {
                centre: c2,
                radius: r2,
                ..
            },
        ) => circle_circle(c1, r1, c2, r2),
    };
    candidates.into_iter().min_by(|x, y| {
        let dx = libm::hypot(x.x - near.x, x.y - near.y);
        let dy = libm::hypot(y.x - near.x, y.y - near.y);
        dx.total_cmp(&dy)
    })
}

pub(crate) fn line_line(
    first: Point,
    first_end: Point,
    second: Point,
    second_end: Point,
) -> Option<Point> {
    let (ux, uy) = (first_end.x - first.x, first_end.y - first.y);
    let (vx, vy) = (second_end.x - second.x, second_end.y - second.y);
    let det = ux * vy - uy * vx;
    if det.abs() < 1e-12 {
        return None;
    }
    let along = ((second.x - first.x) * vy - (second.y - first.y) * vx) / det;
    Some(Point::new(first.x + along * ux, first.y + along * uy))
}

pub(crate) fn line_circle(from: Point, to: Point, centre: Point, radius: f64) -> Vec<Point> {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let (fx, fy) = (from.x - centre.x, from.y - centre.y);
    let quad = dx * dx + dy * dy;
    let linear = 2.0 * (fx * dx + fy * dy);
    let constant = fx * fx + fy * fy - radius * radius;
    let disc = linear * linear - 4.0 * quad * constant;
    if quad < 1e-18 || disc < 0.0 {
        return Vec::new();
    }
    let root = libm::sqrt(disc);
    [
        (-linear - root) / (2.0 * quad),
        (-linear + root) / (2.0 * quad),
    ]
    .into_iter()
    .map(|t| Point::new(from.x + t * dx, from.y + t * dy))
    .collect()
}

pub(crate) fn circle_circle(c1: Point, r1: f64, c2: Point, r2: f64) -> Vec<Point> {
    let (dx, dy) = (c2.x - c1.x, c2.y - c1.y);
    let d = libm::hypot(dx, dy);
    if d < 1e-12 || d > r1 + r2 || d < (r1 - r2).abs() {
        return Vec::new();
    }
    let along = (d * d + r1 * r1 - r2 * r2) / (2.0 * d);
    let height = libm::sqrt((r1 * r1 - along * along).max(0.0));
    let (mx, my) = (c1.x + along * dx / d, c1.y + along * dy / d);
    vec![
        Point::new(mx + height * dy / d, my - height * dx / d),
        Point::new(mx - height * dy / d, my + height * dx / d),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// A clockwise (on screen) rectangle with rounded corners of radius `r`.
    fn rounded(w: f64, h: f64, r: f64) -> Contour {
        let mut c = Contour::new();
        c.line_to(p(r, 0.0), p(w - r, 0.0));
        c.arc(p(w - r, r), r, -FRAC_PI_2, FRAC_PI_2);
        c.line_to(p(w, r), p(w, h - r));
        c.arc(p(w - r, h - r), r, 0.0, FRAC_PI_2);
        c.line_to(p(w - r, h), p(r, h));
        c.arc(p(r, h - r), r, FRAC_PI_2, FRAC_PI_2);
        c.line_to(p(0.0, h - r), p(0.0, r));
        c.arc(p(r, r), r, std::f64::consts::PI, FRAC_PI_2);
        c
    }

    fn assert_closed(c: &Contour) {
        let n = c.segs.len();
        for i in 0..n {
            let (a, b) = (c.segs[i].end(), c.segs[(i + 1) % n].start());
            assert!(
                libm::hypot(a.x - b.x, a.y - b.y) < 1e-9,
                "gap at {i}: {a:?} {b:?}"
            );
        }
    }

    #[test]
    fn a_rounded_rectangle_offset_outward_is_the_exact_larger_rounded_rectangle() {
        let offset = rounded(100.0, 60.0, 10.0).offset(0.1);
        assert_closed(&offset);
        let expected = [
            Seg::Line {
                from: p(10.0, -0.1),
                to: p(90.0, -0.1),
            },
            Seg::Arc {
                centre: p(90.0, 10.0),
                radius: 10.1,
                from: -FRAC_PI_2,
                sweep: FRAC_PI_2,
            },
        ];
        for (seg, want) in offset.segs.iter().zip(expected) {
            match (seg, want) {
                (Seg::Line { from, to }, Seg::Line { from: f, to: t }) => {
                    assert!(libm::hypot(from.x - f.x, from.y - f.y) < 1e-9);
                    assert!(libm::hypot(to.x - t.x, to.y - t.y) < 1e-9);
                }
                (
                    Seg::Arc { radius, centre, .. },
                    Seg::Arc {
                        radius: r,
                        centre: c,
                        ..
                    },
                ) => {
                    assert!(
                        (radius - r).abs() < 1e-9
                            && libm::hypot(centre.x - c.x, centre.y - c.y) < 1e-12
                    );
                }
                _ => panic!("pieces differ"),
            }
        }
        assert_eq!(offset.segs.len(), 8, "tangent joins add no piece");
    }

    #[test]
    fn a_hole_walked_the_other_way_shrinks() {
        let hole = rounded(20.0, 10.0, 2.0).reversed();
        let offset = hole.offset(0.1);
        assert_closed(&offset);
        let arc = offset
            .segs
            .iter()
            .find_map(|seg| match seg {
                Seg::Arc { radius, .. } => Some(*radius),
                Seg::Line { .. } => None,
            })
            .expect("an arc");
        assert!(
            (arc - 1.9).abs() < 1e-9,
            "a hole's corner radius grows smaller: {arc}"
        );
    }

    #[test]
    fn a_right_angle_corner_is_the_intersection_of_the_moved_lines() {
        let mut square = Contour::new();
        square.line_to(p(0.0, 0.0), p(10.0, 0.0));
        square.line_to(p(10.0, 0.0), p(10.0, 10.0));
        square.line_to(p(10.0, 10.0), p(0.0, 10.0));
        square.line_to(p(0.0, 10.0), p(0.0, 0.0));
        let offset = square.offset(0.5);
        assert_closed(&offset);
        assert_eq!(offset.segs.len(), 4);
        let start = offset.segs[0].start();
        assert!(
            (start.x + 0.5).abs() < 1e-9 && (start.y + 0.5).abs() < 1e-9,
            "{start:?}"
        );
    }

    #[test]
    fn a_line_meeting_an_arc_at_a_kink_joins_at_their_intersection() {
        // A chord from the arc's point at -45 degrees to the one at +45 degrees, then the arc back.
        let centre = p(0.0, 0.0);
        let a = polar(centre, 10.0, -std::f64::consts::FRAC_PI_4);
        let b = polar(centre, 10.0, std::f64::consts::FRAC_PI_4);
        let mut contour = Contour::new();
        contour.line_to(a, b);
        contour.arc(centre, 10.0, std::f64::consts::FRAC_PI_4, -FRAC_PI_2);
        // The chord goes down on screen with the arc turning back: a counter-clockwise lens.
        let offset = contour.offset(0.1);
        assert_closed(&offset);
        for seg in &offset.segs {
            if let Seg::Arc { radius, .. } = seg {
                assert!((radius - 9.9).abs() < 1e-9, "{radius}");
            }
        }
    }

    #[test]
    fn a_full_circle_is_four_quarter_arcs_and_converts_to_a_closed_path() {
        let mut circle = Contour::new();
        circle.arc(p(5.0, 5.0), 5.0, 0.0, std::f64::consts::TAU);
        assert_eq!(circle.segs.len(), 4);
        let path = circle.offset(0.2).to_path();
        let start = path.start;
        let last = path.segments.last().expect("segments").to;
        assert!(libm::hypot(start.x - last.x, start.y - last.y) < 1e-12);
        let radius = libm::hypot(start.x - 5.0, start.y - 5.0);
        assert!((radius - 5.2).abs() < 1e-9, "{radius}");
    }
}
