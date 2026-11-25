//! Reads the spans of an outline as straight or curved, with the kind of each join (ADR 0083 §3).

use curvyo_document_core::Point;
use curvyo_extension_core::{ClosedPath, Cubic};

/// A cubic whose control points stay this close to its chord (mm) is a line (criterion 3).
pub(crate) const LINE_TOLERANCE_MM: f64 = 0.1;
/// A turn of at most this many degrees is a tangent join (fitting noise).
pub(crate) const TANGENT_DEGREES: f64 = 1.0;

/// What a join between two neighbouring cubics is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JoinKind {
    /// The tangents agree within a degree.
    Tangent,
    /// A turn of at most the Corner angle: filleted into a bent span.
    Kink,
    /// A turn above the Corner angle: a finger joint.
    Corner,
}

/// The join at the end of one cubic, which is the start of the next.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Join {
    pub(crate) kind: JoinKind,
    /// The turn in radians, positive clockwise on screen (a convex corner of the clockwise walk).
    pub(crate) turn: f64,
}

/// An outline read as spans and joins.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Classified {
    /// Per cubic: whether it stays off its chord by more than the line tolerance.
    pub(crate) curved: Vec<bool>,
    /// Per cubic: the join at its end.
    pub(crate) joins: Vec<Join>,
}

fn sub(a: Point, b: Point) -> (f64, f64) {
    (a.x - b.x, a.y - b.y)
}

/// The distance of `p` from the line through `a` and `b`; from `a` when the two coincide.
fn off_chord(a: Point, b: Point, p: Point) -> f64 {
    let (dx, dy) = sub(b, a);
    let length = libm::hypot(dx, dy);
    if length < 1e-12 {
        let (px, py) = sub(p, a);
        return libm::hypot(px, py);
    }
    let (px, py) = sub(p, a);
    (dx * py - dy * px).abs() / length
}

fn is_curved(from: Point, cubic: &Cubic) -> bool {
    off_chord(from, cubic.to, cubic.c1) > LINE_TOLERANCE_MM
        || off_chord(from, cubic.to, cubic.c2) > LINE_TOLERANCE_MM
}

/// The direction of travel at the start (`false`) or the end (`true`) of `cubic`, which starts at
/// `from`; the first handle that is not on the node gives it.
fn tangent(from: Point, cubic: &Cubic, at_end: bool) -> (f64, f64) {
    let candidates = if at_end {
        [
            sub(cubic.to, cubic.c2),
            sub(cubic.to, cubic.c1),
            sub(cubic.to, from),
        ]
    } else {
        [
            sub(cubic.c1, from),
            sub(cubic.c2, from),
            sub(cubic.to, from),
        ]
    };
    candidates
        .into_iter()
        .find(|(x, y)| libm::hypot(*x, *y) > 1e-9)
        .unwrap_or((1.0, 0.0))
}

/// The starts of the cubics of `path`: the start point, then every end but the last.
pub(crate) fn starts(path: &ClosedPath) -> Vec<Point> {
    let mut starts = vec![path.start];
    starts.extend(path.segments.iter().map(|c| c.to));
    starts.pop();
    starts
}

/// Reads `path` for the Corner angle `corner_degrees`.
pub(crate) fn classify(path: &ClosedPath, corner_degrees: f64) -> Classified {
    let from = starts(path);
    let count = path.segments.len();
    let curved = (0..count)
        .map(|i| is_curved(from[i], &path.segments[i]))
        .collect();
    let joins = (0..count)
        .map(|i| {
            let next = (i + 1) % count;
            let a = tangent(from[i], &path.segments[i], true);
            let b = tangent(from[next], &path.segments[next], false);
            let turn = libm::atan2(a.0 * b.1 - a.1 * b.0, a.0 * b.0 + a.1 * b.1);
            let degrees = turn.abs().to_degrees();
            let kind = if degrees <= TANGENT_DEGREES {
                JoinKind::Tangent
            } else if degrees <= corner_degrees {
                JoinKind::Kink
            } else {
                JoinKind::Corner
            };
            Join { kind, turn }
        })
        .collect();
    Classified { curved, joins }
}

impl Classified {
    /// How many joins are corners.
    #[cfg(test)]
    pub(crate) fn corners(&self) -> usize {
        self.joins
            .iter()
            .filter(|join| join.kind == JoinKind::Corner)
            .count()
    }

    /// How many cubics are curved.
    pub(crate) fn curved_cubics(&self) -> usize {
        self.curved.iter().filter(|curved| **curved).count()
    }

    #[cfg(test)]
    /// How many bent spans the outline has: runs of curved cubics and kinks between two corners
    /// (a run is broken by a straight cubic or a corner; a loop without corner joins its ends).
    pub(crate) fn bent_spans(&self) -> usize {
        // The items in walk order: cubic 0, join 0, cubic 1, join 1, ...
        let items: Vec<Option<bool>> = (0..self.curved.len())
            .flat_map(|i| {
                let join = self.joins[i];
                [
                    Some(self.curved[i]),
                    match join.kind {
                        JoinKind::Kink => Some(true),
                        JoinKind::Tangent => None,
                        JoinKind::Corner => Some(false),
                    },
                ]
            })
            .collect();
        // `None` is neutral: it neither bends nor breaks a run.
        let kept: Vec<bool> = items.into_iter().flatten().collect();
        if kept.iter().all(|bent| *bent) {
            return usize::from(!kept.is_empty());
        }
        let first_break = kept.iter().position(|bent| !bent).unwrap_or(0);
        let mut runs = 0;
        let mut inside = false;
        for step in 0..kept.len() {
            let bent = kept[(first_break + step) % kept.len()];
            if bent && !inside {
                runs += 1;
            }
            inside = bent;
        }
        runs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Triple = ((f64, f64), (f64, f64), (f64, f64));

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn line(from: Point, to: Point) -> Cubic {
        Cubic {
            c1: from,
            c2: to,
            to,
        }
    }

    fn polygon(points: &[(f64, f64)]) -> ClosedPath {
        let pts: Vec<Point> = points.iter().map(|&(x, y)| p(x, y)).collect();
        ClosedPath {
            start: pts[0],
            segments: (0..pts.len())
                .map(|i| line(pts[i], pts[(i + 1) % pts.len()]))
                .collect(),
        }
    }

    pub(crate) fn coffin() -> ClosedPath {
        polygon(&[
            (-50.0, -10.0),
            (-25.0, -30.0),
            (50.0, -20.0),
            (50.0, 20.0),
            (-25.0, 30.0),
            (-50.0, 10.0),
        ])
    }

    /// A closed outline from `(c1, c2, to)` triples after `start`.
    fn curve(start: Point, cubics: &[Triple]) -> ClosedPath {
        ClosedPath {
            start,
            segments: cubics
                .iter()
                .map(|&(c1, c2, to)| Cubic {
                    c1: p(c1.0, c1.1),
                    c2: p(c2.0, c2.1),
                    to: p(to.0, to.1),
                })
                .collect(),
        }
    }

    #[test]
    fn a_coffin_has_six_corners_and_no_curved_span() {
        let c = classify(&coffin(), 25.0);
        assert_eq!(c.corners(), 6);
        assert_eq!(c.curved_cubics(), 0);
        assert_eq!(c.bent_spans(), 0);
        assert!(
            c.joins.iter().all(|j| j.turn > 0.0),
            "clockwise: all convex"
        );
    }

    #[test]
    fn at_a_corner_angle_of_ninety_degrees_the_corners_of_the_coffin_become_kinks() {
        let c = classify(&coffin(), 90.0);
        assert_eq!(c.corners(), 0);
        assert_eq!(
            c.joins.iter().filter(|j| j.kind == JoinKind::Kink).count(),
            6
        );
        assert_eq!(
            c.bent_spans(),
            6,
            "straights between the kinks keep six runs apart"
        );
    }

    #[test]
    fn a_shield_has_its_two_top_corners_and_one_curved_run() {
        // Flat top, two straight sides, a bottom of two cubics that meets the sides tangentially.
        let shield = curve(
            p(-30.0, -40.0),
            &[
                ((-30.0, -40.0), (30.0, -40.0), (30.0, -40.0)),
                ((30.0, -40.0), (30.0, 10.0), (30.0, 10.0)),
                ((30.0, 25.0), (10.0, 40.0), (0.0, 40.0)),
                ((-10.0, 40.0), (-30.0, 25.0), (-30.0, 10.0)),
                ((-30.0, 10.0), (-30.0, -40.0), (-30.0, -40.0)),
            ],
        );
        let c = classify(&shield, 25.0);
        assert_eq!(c.corners(), 2);
        assert_eq!(c.curved_cubics(), 2);
        assert_eq!(c.bent_spans(), 1);
    }

    #[test]
    fn a_heart_has_two_corners_and_four_curved_cubics() {
        // Bottom tip, two lobes meeting in the top notch.
        let heart = curve(
            p(0.0, 40.0),
            &[
                ((-10.0, 25.0), (-40.0, 10.0), (-40.0, -10.0)),
                ((-40.0, -35.0), (-5.0, -40.0), (0.0, -20.0)),
                ((5.0, -40.0), (40.0, -35.0), (40.0, -10.0)),
                ((40.0, 10.0), (10.0, 25.0), (0.0, 40.0)),
            ],
        );
        let c = classify(&heart, 25.0);
        assert_eq!(c.corners(), 2);
        assert_eq!(c.curved_cubics(), 4);
        assert_eq!(
            c.bent_spans(),
            2,
            "two merged bent spans between the corners"
        );
    }

    #[test]
    fn a_circle_of_four_cubics_has_no_corner_and_one_bent_span() {
        let k = 0.552_284_75 * 30.0;
        let circle = curve(
            p(30.0, 0.0),
            &[
                ((30.0, k), (k, 30.0), (0.0, 30.0)),
                ((-k, 30.0), (-30.0, k), (-30.0, 0.0)),
                ((-30.0, -k), (-k, -30.0), (0.0, -30.0)),
                ((k, -30.0), (30.0, -k), (30.0, 0.0)),
            ],
        );
        let c = classify(&circle, 25.0);
        assert_eq!(c.corners(), 0);
        assert_eq!(c.curved_cubics(), 4);
        assert_eq!(c.bent_spans(), 1);
    }

    #[test]
    fn a_cubic_within_a_tenth_of_a_millimetre_of_its_chord_is_a_line() {
        let near = Cubic {
            c1: p(3.0, 0.09),
            c2: p(7.0, -0.09),
            to: p(10.0, 0.0),
        };
        assert!(!is_curved(p(0.0, 0.0), &near));
        let off = Cubic {
            c1: p(3.0, 0.2),
            ..near
        };
        assert!(is_curved(p(0.0, 0.0), &off));
    }
}
