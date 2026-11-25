//! The plates of a path box: cover, lid, inner base and ring for the outline (ADR 0083 §7).

use curvyo_document_core::{Length, Point};
use curvyo_extension_core::{ClosedPath, Cubic, ParamValue, Warning};
use curvyo_geometry_core::{OffsetError, OffsetJoin, Outline, Tolerance, cut_parameters, offset};

use super::build::{Measures, Plan, warning};
use super::check::{signed_area, triples};
use super::hollow::lid_with_hollow;
use crate::contour::{Contour, Seg};
use crate::footprint::{Footprint, Shape, Tab};
use crate::outline::reversed;
use crate::rounded_box::layout::Piece;
use crate::wall_plates::{Hollow, plain, plain_at, tabbed};

pub(crate) fn contour_shapes(contour: &Contour) -> Vec<Shape> {
    contour
        .segs
        .iter()
        .map(|seg| match *seg {
            Seg::Line { from, to } => Shape::Straight { from, to },
            Seg::Arc {
                centre,
                radius,
                from,
                sweep,
            } => Shape::Bend {
                centre,
                radius,
                from,
                sweep,
            },
        })
        .collect()
}

/// The largest outline of a boolean result as a closed path.
fn largest_outline(
    result: &curvyo_geometry_core::OffsetOutlines,
) -> Result<ClosedPath, OffsetError> {
    let largest = result
        .outlines()
        .iter()
        .max_by(|a, b| {
            let area = |o: &Vec<curvyo_geometry_core::OffsetAnchor>| {
                signed_area(&o.iter().map(|n| n.point).collect::<Vec<_>>()).abs()
            };
            area(a).total_cmp(&area(b))
        })
        .ok_or(OffsetError::Empty)?;
    let count = largest.len();
    Ok(ClosedPath {
        start: largest[0].point,
        segments: (0..count)
            .map(|i| {
                let (a, b) = (largest[i], largest[(i + 1) % count]);
                Cubic {
                    c1: Point::new(a.point.x + a.handle_out.x, a.point.y + a.handle_out.y),
                    c2: Point::new(b.point.x + b.handle_in.x, b.point.y + b.handle_in.y),
                    to: b.point,
                }
            })
            .collect(),
    })
}

/// The footprint moved by `distance` (negative: inwards) through the boolean offset with round
/// corners.
fn boolean_offset(footprint: &Footprint, distance: f64) -> Result<ClosedPath, OffsetError> {
    let path = plain(footprint, None).to_path();
    let anchors = triples(&path);
    let result = offset(
        &[Outline::new(&anchors, true)],
        Length::from_mm(distance),
        OffsetJoin::Round,
        Tolerance::from_mm(0.01),
    )?;
    largest_outline(&result)
}

/// The ring's opening: the footprint inset by `distance` through the boolean offset, wound as a
/// hole; `None` when nothing is left (ADR 0083 §7).
fn ring_opening(footprint: &Footprint, distance: f64) -> Result<ClosedPath, OffsetError> {
    let closed = boolean_offset(footprint, -distance)?;
    let points: Vec<Point> = std::iter::once(closed.start)
        .chain(closed.segments.iter().map(|c| c.to))
        .collect();
    // A hole winds against its surrounding outline: counter-clockwise on screen.
    Ok(if signed_area(&points) > 0.0 {
        reversed(&closed)
    } else {
        closed
    })
}

/// The most a chord of the round offset's polyline may stand off its cubic (mm).
const FLATTEN_SAG_MM: f64 = 0.01;

#[allow(clippy::many_single_char_names)]
/// Appends the cubic `from`, `c`, flattened until no chord sags more than `FLATTEN_SAG_MM`
/// (the curve stays within three quarters of its control points' distance from the chord).
fn flatten(contour: &mut Contour, from: Point, c: &Cubic, depth: u32) {
    let (dx, dy) = (c.to.x - from.x, c.to.y - from.y);
    let chord = libm::hypot(dx, dy).max(1e-12);
    let off = |p: Point| ((p.x - from.x) * dy - (p.y - from.y) * dx).abs() / chord;
    if depth >= 12 || 0.75 * off(c.c1).max(off(c.c2)) <= FLATTEN_SAG_MM {
        contour.line_to(from, c.to);
        return;
    }
    let mid = |a: Point, b: Point| Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y));
    let (a, b, d) = (mid(from, c.c1), mid(c.c1, c.c2), mid(c.c2, c.to));
    let (e, f) = (mid(a, b), mid(b, d));
    let m = mid(e, f);
    flatten(
        contour,
        from,
        &Cubic {
            c1: a,
            c2: e,
            to: m,
        },
        depth + 1,
    );
    flatten(
        contour,
        m,
        &Cubic {
            c1: f,
            c2: d,
            to: c.to,
        },
        depth + 1,
    );
}

/// A closed path as a contour of short lines (the round offset has no arcs to keep).
fn polyline(path: &ClosedPath) -> Contour {
    let mut contour = Contour::new();
    let mut from = path.start;
    for c in &path.segments {
        flatten(&mut contour, from, c, 0);
        from = c.to;
    }
    contour
}

/// Whether the exact offset by the wall cannot be built: a concave arc below the wall thickness
/// in radius, or an offset that crosses itself (ADR 0083 §13.9).
fn exact_cover_fails(footprint: &Footprint, exact: &Contour, wall: f64) -> bool {
    let narrow = footprint.spans.iter().any(|span| {
        matches!(span.shape, Shape::Bend { radius, sweep, .. } if sweep < 0.0 && radius < wall)
    });
    narrow || {
        let anchors = triples(&exact.to_path());
        cut_parameters(&[Outline::new(&anchors, true)])
            .iter()
            .any(|cuts| !cuts.is_empty())
    }
}

/// The cover's contour: the footprint offset outwards by the wall thickness exactly, else (not
/// in a draft) by the round offset of geometry-core; the flag says it fell back.
fn cover_contour(plan: &Plan, m: &Measures, warnings: &mut Vec<Warning>) -> (Contour, bool) {
    let exact = plain(&plan.footprint, None).offset(m.wall);
    let needed = m.cover_on || (m.lid_on && m.seat);
    if m.draft || !needed || !exact_cover_fails(&plan.footprint, &exact, m.wall) {
        return (exact, false);
    }
    if let Ok(path) = boolean_offset(&plan.footprint, m.wall) {
        return (polyline(&path), true);
    }
    // The kernel refused: the exact cover stands and the maker is told.
    warnings.push(warning("cover_offset_failed", Vec::new()));
    (exact, false)
}

/// The plates in the order of the rounded box: cover, lid, inner base, ring.
pub(crate) fn plates(
    plan: &Plan,
    inner: &Footprint,
    tabs: &[Tab],
    m: &Measures,
    warnings: &mut Vec<Warning>,
) -> Vec<Piece> {
    let (outer_contour, rounded) = cover_contour(plan, m, warnings);
    if rounded {
        warnings.push(Warning {
            id: "cover_rounded",
            pieces: Vec::new(),
            value: Some(ParamValue::Length(Length::from_mm(m.wall))),
            limit: None,
        });
    }
    let outer = Footprint::from_shapes(&contour_shapes(&outer_contour), m.wall, m.wall);
    let mut out = Vec::new();
    if m.cover_on {
        out.push(Piece::new(vec![plain(&outer, None)], m.cover, "cover"));
    }
    if m.lid_on {
        let lid = if m.seat {
            seat_lid(&outer, m, warnings, rounded)
        } else {
            tabbed(inner, tabs, m.lid, m.wall)
        };
        out.push(Piece::new(vec![lid], m.lid, "lid"));
    }
    out.push(Piece::new(
        vec![tabbed(inner, tabs, m.base, m.wall)],
        m.base,
        "base",
    ));
    if m.lid_on && m.seat {
        let mut ring = Piece::new(vec![tabbed(inner, tabs, m.ring, m.wall)], m.ring, "ring");
        let opening = if m.draft {
            Err(OffsetError::NoArea)
        } else {
            ring_opening(&plan.footprint, 2.0 * m.ring + m.kerf / 2.0)
        };
        match opening {
            Ok(hole) => ring.holes.push(hole),
            // Nothing is left of the opening: the ring is a full plate.
            Err(OffsetError::Empty | OffsetError::NoArea) => {
                if !m.draft {
                    warnings.push(warning("ring_closed", Vec::new()));
                }
            }
            // The kernel refused (a coordinate out of range): the ring stays full, said so.
            Err(_) => warnings.push(warning("ring_opening_failed", Vec::new())),
        }
        out.push(ring);
    }
    out
}

/// The seat lid: the outer footprint with the finger hollow on its longest straight that holds
/// the hollow's width and two wall thicknesses (ADR 0083 §7), else without it and a warning.
fn seat_lid(
    outer: &Footprint,
    m: &Measures,
    warnings: &mut Vec<Warning>,
    rounded: bool,
) -> Contour {
    let Some((width, depth)) = m.hollow.filter(|_| !rounded) else {
        return plain(outer, None);
    };
    let longest = outer
        .spans
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            matches!(s.shape, Shape::Straight { .. }) && s.length >= width + 2.0 * m.wall
        })
        .max_by(|(_, a), (_, b)| a.length.total_cmp(&b.length))
        .map(|(index, _)| index);
    if let Some(at) = longest {
        plain_at(outer, Hollow { width, depth }, at)
    } else if let Some(lid) = lid_with_hollow(outer, Hollow { width, depth }, m.wall) {
        lid
    } else {
        warnings.push(warning("hollow_left_out", Vec::new()));
        plain(outer, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contour::Seg;

    #[test]
    fn the_flattened_round_offset_stays_within_the_named_sag_of_its_cubic() {
        let from = Point::new(0.0, 0.0);
        let c = Cubic {
            c1: Point::new(0.0, 33.0),
            c2: Point::new(60.0, 33.0),
            to: Point::new(60.0, 0.0),
        };
        let mut contour = Contour::new();
        flatten(&mut contour, from, &c, 0);
        let at = |t: f64| {
            let u = 1.0 - t;
            let f = |a: f64, b: f64, c1: f64, d: f64| {
                u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c1 + t * t * t * d
            };
            (
                f(from.x, c.c1.x, c.c2.x, c.to.x),
                f(from.y, c.c1.y, c.c2.y, c.to.y),
            )
        };
        for k in 0..=2000 {
            let (x, y) = at(f64::from(k) / 2000.0);
            let near = contour
                .segs
                .iter()
                .map(|seg| match *seg {
                    Seg::Line { from, to } => {
                        let (dx, dy) = (to.x - from.x, to.y - from.y);
                        let l2 = dx * dx + dy * dy;
                        let t = (((x - from.x) * dx + (y - from.y) * dy) / l2).clamp(0.0, 1.0);
                        libm::hypot(x - from.x - t * dx, y - from.y - t * dy)
                    }
                    Seg::Arc { .. } => f64::MAX,
                })
                .fold(f64::MAX, f64::min);
            assert!(near <= FLATTEN_SAG_MM + 1e-9, "{near}");
        }
    }
}
