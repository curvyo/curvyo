//! The geometry each sketch dimension measures, in document coordinates.
use curvyo_document_core::{Affine, Point, Vec2};
use curvyo_sketch_core::{Entity, EntityId, Relation, Sketch};
use curvyo_ui_core::DimensionGeometry;

fn point(solution: &Sketch, id: &EntityId, placement: Affine) -> Option<Point> {
    let p = solution.point(id).ok()?;
    Some(placement.apply_point(Point::new(p.x.0, p.y.0)))
}

/// A round curve's centre, document radius and default leader direction.
fn round(solution: &Sketch, id: &EntityId, placement: Affine) -> Option<(Point, f64, Vec2)> {
    let (center, radius, toward) = match solution.entities.get(id)? {
        Entity::Circle { center, radius, .. } => (center, radius.0, Vec2::new(1.0, -1.0)),
        Entity::Arc {
            center,
            radius,
            start_angle,
            end_angle,
            ..
        } => {
            let mut sweep = end_angle.0 - start_angle.0;
            if sweep < 0.0 {
                sweep += std::f64::consts::TAU;
            }
            // Three quarters along the arc: its middle carries the arc's symbols.
            let leader = start_angle.0 + sweep * 0.75;
            (center, radius.0, Vec2::new(leader.cos(), leader.sin()))
        }
        _ => return None,
    };
    let c = solution.point(center).ok()?;
    let local = Point::new(c.x.0, c.y.0);
    let doc_center = placement.apply_point(local);
    let rim = placement.apply_point(Point::new(local.x + radius, local.y));
    Some((
        doc_center,
        doc_center.vector_to(rim).length(),
        placement.apply_vector(toward),
    ))
}

fn line(solution: &Sketch, id: &EntityId, placement: Affine) -> Option<(Point, Point)> {
    match solution.entities.get(id)? {
        Entity::Line { start, end } => Some((
            point(solution, start, placement)?,
            point(solution, end, placement)?,
        )),
        _ => None,
    }
}

/// The geometry a dimension measures, in document coordinates, and its size for the
/// small-span rule; `None` for relations that are not dimensions.
pub(super) fn dimension_geometry(
    solution: &Sketch,
    relation: &Relation,
    placement: Affine,
) -> Option<(DimensionGeometry, f64)> {
    match relation {
        Relation::Length { curve, .. } => {
            if let Some((from, to)) = line(solution, curve, placement) {
                return Some(linear(from, to));
            }
            let (center, radius, toward) = round(solution, curve, placement)?;
            Some((radius_geometry(center, radius, toward), radius * 2.0))
        }
        Relation::Distance { a, b, .. } => distance_geometry(solution, a, b, placement),
        Relation::HorizontalDistance { a, b, .. } | Relation::VerticalDistance { a, b, .. } => {
            let horizontal = matches!(relation, Relation::HorizontalDistance { .. });
            axis_distance_geometry(solution, a, b, horizontal, placement)
        }
        Relation::Radius { curve, .. } => {
            let (center, radius, toward) = round(solution, curve, placement)?;
            Some((radius_geometry(center, radius, toward), radius * 2.0))
        }
        Relation::Diameter { curve, .. } => diameter_geometry(solution, curve, placement),
        Relation::Angle { a, b, .. } => {
            let (a0, a1) = line(solution, a, placement)?;
            let (b0, b1) = line(solution, b, placement)?;
            angle_geometry(a0, a1, b0, b1)
        }
        Relation::AxisAngle { line: id, .. } => axis_angle_geometry(solution, id, placement),
        Relation::EllipseRadius { curve, axis, .. } => {
            let (center, radius, toward) = semi_axis(solution, curve, *axis, placement)?;
            Some((radius_geometry(center, radius, toward), radius * 2.0))
        }
        // A sweep is the structural half turn of a slot cap or arc, not a maker's dimension.
        _ => None,
    }
}

/// An ellipse's centre, one semi-axis's document length and that semi-axis's direction.
fn semi_axis(
    solution: &Sketch,
    id: &EntityId,
    axis: curvyo_sketch_core::EllipseAxis,
    placement: Affine,
) -> Option<(Point, f64, Vec2)> {
    let Entity::Ellipse {
        center,
        focus,
        minor_radius,
        ..
    } = solution.entities.get(id)?
    else {
        return None;
    };
    let (c, f) = (solution.point(center).ok()?, solution.point(focus).ok()?);
    let focal = c.distance(f).0;
    let major = if focal > 0.0 {
        Vec2::new((f.x.0 - c.x.0) / focal, (f.y.0 - c.y.0) / focal)
    } else {
        Vec2::new(1.0, 0.0)
    };
    let (direction, radius) = match axis {
        curvyo_sketch_core::EllipseAxis::Major => (major, focal.hypot(minor_radius.0)),
        curvyo_sketch_core::EllipseAxis::Minor => (Vec2::new(-major.y, major.x), minor_radius.0),
    };
    let local = Point::new(c.x.0, c.y.0);
    let doc_center = placement.apply_point(local);
    let rim = placement.apply_point(local.translated(direction.scaled(radius)));
    let toward = doc_center.vector_to(rim);
    Some((doc_center, toward.length(), toward))
}

fn radius_geometry(center: Point, radius: f64, toward: Vec2) -> DimensionGeometry {
    DimensionGeometry::Radius {
        center,
        radius: curvyo_document_core::Length::from_mm(radius),
        toward,
    }
}

/// The angle between two lines: at their intersection, each ray toward the line's far part.
fn angle_geometry(a0: Point, a1: Point, b0: Point, b1: Point) -> Option<(DimensionGeometry, f64)> {
    let (da, db) = (a0.vector_to(a1), b0.vector_to(b1));
    let cross = da.x * db.y - da.y * db.x;
    let size = da.length().min(db.length());
    if cross.abs() <= f64::EPSILON * da.length() * db.length() {
        return None;
    }
    let t = (a0.vector_to(b0).x * db.y - a0.vector_to(b0).y * db.x) / cross;
    let vertex = a0.translated(da.scaled(t));
    let ray = |s: Point, e: Point, d: Vec2| {
        let mid = Point::new(f64::midpoint(s.x, e.x), f64::midpoint(s.y, e.y));
        let toward = vertex.vector_to(mid);
        if toward.length() > size * 1e-6 {
            toward
        } else {
            d
        }
    };
    Some((
        DimensionGeometry::Angle {
            vertex,
            first: ray(a0, a1, da),
            second: ray(b0, b1, db),
        },
        size,
    ))
}

fn linear(from: Point, to: Point) -> (DimensionGeometry, f64) {
    (
        DimensionGeometry::Linear { from, to },
        from.vector_to(to).length(),
    )
}

/// A point to a point, or a point to its foot on a line.
fn distance_geometry(
    solution: &Sketch,
    a: &EntityId,
    b: &EntityId,
    placement: Affine,
) -> Option<(DimensionGeometry, f64)> {
    let from = point(solution, a, placement)?;
    if let Some((s, e)) = line(solution, b, placement) {
        let along = s.vector_to(e);
        let length = along.length();
        let t = if length > 0.0 {
            (s.vector_to(from).x * along.x + s.vector_to(from).y * along.y) / (length * length)
        } else {
            0.0
        };
        return Some(linear(from, s.translated(along.scaled(t))));
    }
    Some(linear(from, point(solution, b, placement)?))
}

/// Two points measured along the sketch's X (`horizontal`) or Y axis.
fn axis_distance_geometry(
    solution: &Sketch,
    a: &EntityId,
    b: &EntityId,
    horizontal: bool,
    placement: Affine,
) -> Option<(DimensionGeometry, f64)> {
    let (from, to) = (
        point(solution, a, placement)?,
        point(solution, b, placement)?,
    );
    let axis = if horizontal {
        Vec2::new(1.0, 0.0)
    } else {
        Vec2::new(0.0, 1.0)
    };
    let along = placement.apply_vector(axis);
    // The small-span rule looks at the points' distance: a zero distance between two aligned
    // points stays visible.
    let size = from.vector_to(to).length();
    Some((DimensionGeometry::Projected { from, to, along }, size))
}

fn diameter_geometry(
    solution: &Sketch,
    curve: &EntityId,
    placement: Affine,
) -> Option<(DimensionGeometry, f64)> {
    let (center, radius, toward) = round(solution, curve, placement)?;
    Some((
        DimensionGeometry::Diameter {
            center,
            radius: curvyo_document_core::Length::from_mm(radius),
            toward,
        },
        radius * 2.0,
    ))
}

/// A line's angle against the sketch's X axis, at the line's start.
fn axis_angle_geometry(
    solution: &Sketch,
    id: &EntityId,
    placement: Affine,
) -> Option<(DimensionGeometry, f64)> {
    let (start, end) = line(solution, id, placement)?;
    let along = start.vector_to(end);
    Some((
        DimensionGeometry::Angle {
            vertex: start,
            first: placement.apply_vector(Vec2::new(1.0, 0.0)),
            second: along,
        },
        along.length(),
    ))
}
