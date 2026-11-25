//! The plates of a path box: cover, lid, inner base and ring for the outline (ADR 0083 §7).

use curvyo_document_core::{Length, Point};
use curvyo_extension_core::{ClosedPath, Cubic, Warning};
use curvyo_geometry_core::{OffsetError, OffsetJoin, Outline, Tolerance, offset};

use super::build::{Measures, Plan, warning};
use super::check::{signed_area, triples};
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

/// The ring's opening: the footprint inset by `distance` through the boolean offset, wound as a
/// hole; `None` when nothing is left (ADR 0083 §7).
fn ring_opening(footprint: &Footprint, distance: f64) -> Result<ClosedPath, OffsetError> {
    let path = plain(footprint, None).to_path();
    let anchors = triples(&path);
    let result = offset(
        &[Outline::new(&anchors, true)],
        Length::from_mm(-distance),
        OffsetJoin::Round,
        Tolerance::from_mm(0.01),
    )?;
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
    let closed = ClosedPath {
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
    };
    let points: Vec<Point> = largest.iter().map(|n| n.point).collect();
    // A hole winds against its surrounding outline: counter-clockwise on screen.
    Ok(if signed_area(&points) > 0.0 {
        reversed(&closed)
    } else {
        closed
    })
}

/// The plates in the order of the rounded box: cover, lid, inner base, ring.
pub(crate) fn plates(
    plan: &Plan,
    inner: &Footprint,
    tabs: &[Tab],
    m: &Measures,
    warnings: &mut Vec<Warning>,
) -> Vec<Piece> {
    let outer_contour = plain(&plan.footprint, None).offset(m.wall);
    let outer = Footprint::from_shapes(&contour_shapes(&outer_contour), m.wall, m.wall);
    let mut out = Vec::new();
    if m.cover_on {
        out.push(Piece::new(vec![plain(&outer, None)], m.cover, "cover"));
    }
    if m.lid_on {
        let lid = if m.seat {
            seat_lid(&outer, m, warnings)
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
        match ring_opening(&plan.footprint, 2.0 * m.ring + m.kerf / 2.0) {
            Ok(hole) => ring.holes.push(hole),
            // Nothing is left of the opening: the ring is a full plate.
            Err(OffsetError::Empty | OffsetError::NoArea) => {
                warnings.push(warning("ring_closed", Vec::new()));
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
fn seat_lid(outer: &Footprint, m: &Measures, warnings: &mut Vec<Warning>) -> Contour {
    let Some((width, depth)) = m.hollow else {
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
    } else {
        warnings.push(warning("hollow_left_out", Vec::new()));
        plain(outer, None)
    }
}
