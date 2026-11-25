//! What the shell is told about a path box: warnings, corner and bend marks and piece badges
//! (ADR 0083 §11).

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_extension_core::{Cubic, Mark, Warning};

use super::build::{Badge, Measures, Plan, warning};
use crate::footprint::Shape;
use crate::outline::Builder;

pub(crate) fn unit(a: Point, b: Point) -> (f64, f64) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = libm::hypot(dx, dy);
    (dx / length, dy / length)
}

/// The point at distance `at` along the shape, with the direction of travel.
pub(crate) fn point_at(shape: &Shape, at: f64) -> (Point, (f64, f64)) {
    match *shape {
        Shape::Straight { from, to } => {
            let along = unit(from, to);
            (
                Point::new(from.x + along.0 * at, from.y + along.1 * at),
                along,
            )
        }
        Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        } => {
            let angle = from + at / radius * sweep.signum();
            let side = sweep.signum();
            (
                crate::outline::polar(centre, radius, angle),
                (-side * libm::sin(angle), side * libm::cos(angle)),
            )
        }
    }
}

pub(crate) fn inner_length(shape: &Shape) -> f64 {
    match *shape {
        Shape::Straight { from, to } => libm::hypot(to.x - from.x, to.y - from.y),
        Shape::Bend { radius, sweep, .. } => radius * sweep.abs(),
    }
}

fn bend_cubics(shape: &Shape) -> Option<(Point, Vec<Cubic>)> {
    let Shape::Bend {
        centre,
        radius,
        from,
        sweep,
    } = *shape
    else {
        return None;
    };
    let start = crate::outline::polar(centre, radius, from);
    let mut builder = Builder::new(start);
    builder.arc(centre, radius, from, from + sweep);
    Some((start, builder.finish().segments))
}

/// Warnings and marks of the plan: sharp corners, tight bends, the corners' crosses, the bends'
/// hatching and the badges of the pieces.
pub(crate) fn describe(
    plan: &Plan,
    m: &Measures,
    count: usize,
) -> (Vec<Warning>, Vec<Mark>, Vec<Badge>) {
    let mut warnings = Vec::new();
    let mut marks = Vec::new();
    for (k, vertex) in plan.built.corners.iter().enumerate() {
        let sharp = vertex.turn.abs() > f64::to_radians(150.0);
        if sharp {
            let (before, after) = ((k + count - 1) % count, k);
            let mut w = warning(
                "sharp_corner",
                vec![
                    u32::try_from(before + 1).unwrap_or(0),
                    u32::try_from(after + 1).unwrap_or(0),
                ],
            );
            w.value = Some(curvyo_extension_core::ParamValue::Angle(
                curvyo_extension_core::Degrees(180.0 - vertex.turn.abs().to_degrees()),
            ));
            warnings.push(w);
        }
        marks.push(Mark::Corner {
            at: vertex.at,
            bisector: Vec2::new(0.0, 0.0),
            sharp,
        });
    }
    marks_with_bisectors(plan, &mut marks);
    for &(index, radius) in &plan.built.fillets {
        let Shape::Bend { sweep, .. } = plan.built.shapes[index] else {
            continue;
        };
        let limit = if sweep > 0.0 { 2.0 * m.wall } else { m.wall };
        let tight = radius < limit - 1e-9;
        if tight {
            let piece = u32::try_from(plan.built.piece_of[index] + 1).unwrap_or(0);
            let mut w = warning("tight_bend", vec![piece]);
            w.value = Some(curvyo_extension_core::ParamValue::Length(Length::from_mm(
                radius,
            )));
            w.limit = Some(curvyo_extension_core::ParamValue::Length(Length::from_mm(
                limit,
            )));
            warnings.push(w);
        }
        if let Some((start, segments)) = bend_cubics(&plan.built.shapes[index]) {
            marks.push(Mark::Bend {
                start,
                segments,
                tight,
            });
        }
    }
    (warnings, marks, badges(plan, count))
}

/// Fills in the inward bisector of every corner mark.
fn marks_with_bisectors(plan: &Plan, marks: &mut [Mark]) {
    let corners = &plan.built.corners;
    for (mark, vertex) in marks.iter_mut().zip(corners) {
        let Mark::Corner { at, bisector, .. } = mark else {
            continue;
        };
        // The walk's turn is read from the shapes next to the corner.
        let index = plan.built.shapes.iter().position(|s| match s {
            Shape::Straight { from, .. } => libm::hypot(from.x - at.x, from.y - at.y) < 1e-6,
            Shape::Bend { .. } => false,
        });
        let Some(i) = index else { continue };
        let before = (i + plan.built.shapes.len() - 1) % plan.built.shapes.len();
        let end = |s: &Shape| match *s {
            Shape::Straight { from, to } => unit(from, to),
            Shape::Bend { .. } => point_at(s, inner_length(s)).1,
        };
        let (d_in, d_out) = (end(&plan.built.shapes[before]), end(&plan.built.shapes[i]));
        let (x, y) = (d_out.0 - d_in.0, d_out.1 - d_in.1);
        let length = libm::hypot(x, y).max(1e-12);
        let sign = if vertex.turn > 0.0 { 1.0 } else { -1.0 };
        *bisector = Vec2::new(sign * x / length, sign * y / length);
    }
}

/// The number badge of every piece on the inner base (none for a single piece).
fn badges(plan: &Plan, count: usize) -> Vec<Badge> {
    if plan.corners.is_empty() || count < 2 {
        return Vec::new();
    }
    (0..count)
        .map(|piece| {
            let own: Vec<&Shape> = plan
                .built
                .shapes
                .iter()
                .zip(&plan.built.piece_of)
                .filter(|(_, p)| **p == piece)
                .map(|(s, _)| s)
                .collect();
            let total: f64 = own.iter().map(|s| inner_length(s)).sum();
            let mut remaining = total / 2.0;
            let (mut at, mut along) = (Point::new(0.0, 0.0), (1.0, 0.0));
            for shape in &own {
                let length = inner_length(shape);
                if remaining <= length || std::ptr::eq(*shape, *own.last().unwrap_or(shape)) {
                    (at, along) = point_at(shape, remaining.min(length));
                    break;
                }
                remaining -= length;
            }
            (
                u32::try_from(piece + 1).unwrap_or(0),
                at,
                Vec2::new(-along.1, along.0),
                total,
            )
        })
        .collect()
}
