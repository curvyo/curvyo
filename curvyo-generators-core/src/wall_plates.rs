//! The plates that sit inside or on a wall footprint: a tabbed plate (inner base, tabbed lid, seat
//! ring), a plain plate (cover, seat lid) and the ring's opening (ADR 0082 §4, §5).
//!
//! Screen coordinates, outlines clockwise, holes counter-clockwise. Millimetres.

use curvyo_document_core::Point;

use crate::contour::Contour;
use crate::footprint::{Footprint, Shape, Span, Tab};

/// The notch of the finger hollow: width across the edge, depth into the plate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Hollow {
    pub(crate) width: f64,
    pub(crate) depth: f64,
}

/// The footprint's outline with a tab of half width `half_width` and length `protrusion` at each
/// of `tabs` (flush with the outside of the wall).
pub(crate) fn tabbed(
    footprint: &Footprint,
    tabs: &[Tab],
    half_width: f64,
    protrusion: f64,
) -> Contour {
    walk(footprint, tabs, (half_width, protrusion), None, None)
}

/// The footprint's outline as a plain plate, with the finger hollow in its front straight.
pub(crate) fn plain(footprint: &Footprint, hollow: Option<Hollow>) -> Contour {
    walk(footprint, &[], (0.0, 0.0), hollow, None)
}

/// A plain plate with the finger hollow in the straight span `at` (a path box puts it on its
/// longest straight, ADR 0083 §7).
pub(crate) fn plain_at(footprint: &Footprint, hollow: Hollow, at: usize) -> Contour {
    walk(footprint, &[], (0.0, 0.0), Some(hollow), Some(at))
}

/// The opening of a ring: the footprint, walked the other way as a hole.
pub(crate) fn opening(footprint: &Footprint) -> Contour {
    plain(footprint, None).reversed()
}

/// The span of the front (bottom of the plan) straight, if the footprint has one.
fn front_straight(footprint: &Footprint) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (index, span) in footprint.spans.iter().enumerate() {
        if let Shape::Straight { from, to } = span.shape
            && to.x < from.x
            && (to.y - from.y).abs() < 1e-9
            && best.is_none_or(|(_, y)| from.y > y)
        {
            best = Some((index, from.y));
        }
    }
    best.map(|(index, _)| index)
}

fn walk(
    footprint: &Footprint,
    tabs: &[Tab],
    (half_width, protrusion): (f64, f64),
    hollow: Option<Hollow>,
    at: Option<usize>,
) -> Contour {
    let front = hollow.and_then(|_| at.or_else(|| front_straight(footprint)));
    let mut contour = Contour::new();
    for (index, span) in footprint.spans.iter().enumerate() {
        match span.shape {
            Shape::Bend {
                centre,
                radius,
                from,
                sweep,
            } => {
                let cut = hollow.filter(|_| front.is_none());
                if !cut.is_some_and(|h| {
                    bend_with_hollow(&mut contour, (centre, radius), (from, sweep), h)
                }) {
                    contour.arc(centre, radius, from, sweep);
                }
            }
            Shape::Straight { from, to } => {
                let notch = if front == Some(index) { hollow } else { None };
                straight(
                    &mut contour,
                    span,
                    (from, to),
                    tabs,
                    (half_width, protrusion),
                    notch,
                );
            }
        }
    }
    contour
}

/// One straight with the tabs that stand on it and the hollow, if any.
fn straight(
    contour: &mut Contour,
    span: &Span,
    (from, to): (Point, Point),
    tabs: &[Tab],
    (half_width, protrusion): (f64, f64),
    hollow: Option<Hollow>,
) {
    let length = libm::hypot(to.x - from.x, to.y - from.y);
    let along = ((to.x - from.x) / length, (to.y - from.y) / length);
    let outward = (along.1, -along.0);
    let at = |u: f64, out: f64| {
        Point::new(
            from.x + along.0 * u + outward.0 * out,
            from.y + along.1 * u + outward.1 * out,
        )
    };
    // Each feature is (centre along the span, builder); features never overlap.
    let mut here = from;
    let mut features: Vec<(f64, Option<Hollow>)> = tabs
        .iter()
        .filter(|tab| tab.s > span.s0 - 1e-9 && tab.s < span.s0 + span.length + 1e-9)
        .map(|tab| {
            // Where the tab stands on this chord, measured in the plan.
            let u = (tab.centre.x - from.x) * along.0 + (tab.centre.y - from.y) * along.1;
            (u, None)
        })
        .collect();
    if let Some(hollow) = hollow {
        features.push((length / 2.0, Some(hollow)));
    }
    features.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (centre, notch) in features {
        match notch {
            None => {
                let points = [
                    at(centre - half_width, 0.0),
                    at(centre - half_width, protrusion),
                    at(centre + half_width, protrusion),
                    at(centre + half_width, 0.0),
                ];
                for point in points {
                    contour.line_to(here, point);
                    here = point;
                }
            }
            Some(hollow) => {
                // Into the plate (against the outward direction): up the first side, a half
                // circle over the end, down the second side.
                let radius = hollow.width / 2.0;
                let straight_part = (hollow.depth - radius).max(0.0);
                let first = at(centre - radius, 0.0);
                contour.line_to(here, first);
                let first_in = at(centre - radius, -straight_part);
                contour.line_to(first, first_in);
                let middle = at(centre, -straight_part);
                let start_angle = libm::atan2(first_in.y - middle.y, first_in.x - middle.x);
                // A notch turns the walk to the left (counter-clockwise on screen) round its end.
                contour.arc(middle, radius, start_angle, -std::f64::consts::PI);
                let second_in = at(centre + radius, -straight_part);
                let second = at(centre + radius, 0.0);
                contour.line_to(second_in, second);
                here = second;
            }
        }
    }
    contour.line_to(here, to);
}

/// A bend that holds the front point (the lowest point of the plan) with the finger hollow cut
/// into it: two sides parallel to the front direction and a half circle at the end (ADR 0082
/// §5, a footprint without a front straight). `false` when the bend does not hold that point or
/// the notch does not fit inside it.
fn bend_with_hollow(
    contour: &mut Contour,
    (centre, radius): (Point, f64),
    (from, sweep): (f64, f64),
    hollow: Hollow,
) -> bool {
    let half = hollow.width / 2.0;
    if sweep <= 0.0 || half >= radius {
        return false;
    }
    let front = std::f64::consts::FRAC_PI_2;
    let reach = libm::asin(half / radius);
    let tau = std::f64::consts::TAU;
    let front = front + tau * ((from + sweep / 2.0 - front) / tau).round();
    let (east, west) = (front - reach, front + reach);
    if east <= from + 1e-9 || west >= from + sweep - 1e-9 {
        return false;
    }
    let edge_y = centre.y + libm::sqrt(radius * radius - half * half);
    let end_y = (centre.y + radius - hollow.depth + half).min(edge_y);
    let at = |x: f64, y: f64| Point::new(centre.x + x, y);
    contour.arc(centre, radius, from, east - from);
    let (first, first_in) = (at(half, edge_y), at(half, end_y));
    contour.line_to(first, first_in);
    contour.arc(at(0.0, end_y), half, 0.0, -std::f64::consts::PI);
    let (second_in, second) = (at(-half, end_y), at(-half, edge_y));
    contour.line_to(second_in, second);
    contour.arc(centre, radius, west, from + sweep - west);
    true
}
