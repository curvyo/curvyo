//! Resolves a dimension's slots against object snapshots and measures it (ADR 0027 §3): pure, so
//! the one read pass and the editor's preview snapshots share it.

use std::collections::HashMap;

use crate::dimension_model::{
    CurveSlot, DimensionKind, DimensionSlots, DimensionSnapshot, DimensionValue, Measured,
    PointSlot, ShapeFeature,
};
use crate::path_model::{AnchorId, AnchorSnapshot, NodeId, PathSnapshot};
use crate::primitive_model::{ObjectSnapshot, PrimitiveSnapshot, Shape, shape_center};
use crate::units::{Angle, Length, Point, Tolerance, Vec2};

/// How far a curve may stray from a circle and still be measured as one (ADR 0027 §3).
pub const CIRCLE_TOLERANCE: Tolerance = Tolerance::from_mm(0.01);
/// Below this length a direction or a radius is undefined.
const DEGENERATE_MM: f64 = 1e-9;

/// The value of a dimension of `kind` over `points` (as [`Measured::points`]), with the angle
/// `sector` of [`crate::DimensionPlace::sector`]; `None` when there are too few points or the
/// geometry is degenerate.
#[must_use]
pub fn measure(kind: DimensionKind, points: &[Point], sector: u8) -> Option<DimensionValue> {
    let length = |v: f64| Some(DimensionValue::Length(Length::from_mm(v)));
    match (kind, points) {
        (DimensionKind::Aligned, [a, b, ..]) => length(a.vector_to(*b).length()),
        (DimensionKind::Horizontal, [a, b, ..]) => length((b.x - a.x).abs()),
        (DimensionKind::Vertical, [a, b, ..]) => length((b.y - a.y).abs()),
        (DimensionKind::Radius, [c, rim, ..]) => length(c.vector_to(*rim).length()),
        (DimensionKind::Diameter, [c, rim, ..]) => length(2.0 * c.vector_to(*rim).length()),
        (DimensionKind::Angle, [vertex, first, second, ..]) => {
            let (r1, r2) = angle_rays(*vertex, *first, *second, sector)?;
            let cross = r1.x * r2.y - r1.y * r2.x;
            let dot = r1.x * r2.x + r1.y * r2.y;
            Some(DimensionValue::Angle(Angle::from_radians(
                cross.abs().atan2(dot),
            )))
        }
        _ => None,
    }
}

/// The two rays of an angle dimension from `vertex`, each turned round by its bit of `sector`;
/// `None` when a ray has no direction.
#[must_use]
pub fn angle_rays(vertex: Point, first: Point, second: Point, sector: u8) -> Option<(Vec2, Vec2)> {
    let r1 = vertex.vector_to(first);
    let r2 = vertex.vector_to(second);
    if r1.length() <= DEGENERATE_MM || r2.length() <= DEGENERATE_MM {
        return None;
    }
    let flip = |v: Vec2, bit: u8| if sector & bit == 0 { v } else { v.negated() };
    Some((flip(r1, 1), flip(r2, 2)))
}

/// What `dimension` measures on the geometry `lookup` hands out, or `None` when a slot does not
/// resolve (the dimension is unlinked and shows its kept values).
#[must_use]
pub fn resolve<'a>(
    dimension: &DimensionSnapshot,
    lookup: impl Fn(NodeId) -> Option<&'a ObjectSnapshot>,
) -> Option<Measured> {
    let points = match &dimension.slots {
        DimensionSlots::Points(slots) => slots
            .iter()
            .map(|slot| point_of(slot, &lookup))
            .collect::<Option<Vec<_>>>()?,
        DimensionSlots::Curve(curve) => {
            let (center, radius) = circle_of(curve, &lookup)?;
            vec![center, Point::new(center.x + radius, center.y)]
        }
    };
    let value = measure(dimension.kind, &points, dimension.place.sector)?;
    let measured = Measured { points, value };
    measured.is_finite().then_some(measured)
}

/// `dimension` with its shown values and link state resolved against `lookup`.
pub fn resolve_into<'a>(
    dimension: &mut DimensionSnapshot,
    lookup: impl Fn(NodeId) -> Option<&'a ObjectSnapshot>,
) {
    let measured = resolve(dimension, lookup);
    show(dimension, measured);
}

/// Shows `measured` on `dimension` (linked), or its kept values when nothing resolved.
fn show(dimension: &mut DimensionSnapshot, measured: Option<Measured>) {
    dimension.linked = measured.is_some();
    dimension.shown = measured.unwrap_or_else(|| dimension.kept.clone());
}

/// Resolves every dimension among `objects` against the other objects of the same list: the one
/// read pass (after the carriers, ADR 0027 §3) and the editor's preview snapshots of a drag.
pub fn resolve_dimensions_in(objects: &mut [ObjectSnapshot]) {
    let dimensions: Vec<usize> = objects
        .iter()
        .enumerate()
        .filter_map(|(i, object)| matches!(object, ObjectSnapshot::Dimension(_)).then_some(i))
        .collect();
    if dimensions.is_empty() {
        return;
    }
    let index: HashMap<NodeId, usize> = objects
        .iter()
        .enumerate()
        .filter(|(_, object)| !matches!(object, ObjectSnapshot::Dimension(_)))
        .map(|(i, object)| (object.id(), i))
        .collect();
    let resolved: Vec<(usize, Option<Measured>)> = dimensions
        .into_iter()
        .filter_map(|i| match &objects[i] {
            ObjectSnapshot::Dimension(dimension) => Some((
                i,
                resolve(dimension, |id| index.get(&id).map(|&at| &objects[at])),
            )),
            _ => None,
        })
        .collect();
    for (i, measured) in resolved {
        if let ObjectSnapshot::Dimension(dimension) = &mut objects[i] {
            show(dimension, measured);
        }
    }
}

/// The read pass's form of [`resolve_dimensions_in`]: leaves that did not resolve are `None`.
pub(crate) fn resolve_dimensions(leaves: &mut [Option<ObjectSnapshot>]) {
    if !leaves
        .iter()
        .any(|leaf| matches!(leaf, Some(ObjectSnapshot::Dimension(_))))
    {
        return;
    }
    let mut objects: Vec<ObjectSnapshot> = Vec::new();
    let mut at: Vec<usize> = Vec::new();
    for (i, leaf) in leaves.iter_mut().enumerate() {
        if let Some(object) = leaf.take() {
            objects.push(object);
            at.push(i);
        }
    }
    resolve_dimensions_in(&mut objects);
    for (object, i) in objects.into_iter().zip(at) {
        leaves[i] = Some(object);
    }
}

/// The path whose anchors a slot names: a path's own, a clone's resolved outline (its root's
/// anchor ids, ADR 0027 §2).
fn anchored_path(object: &ObjectSnapshot) -> Option<&PathSnapshot> {
    match object {
        ObjectSnapshot::Path(path) => Some(path),
        ObjectSnapshot::Clone(clone) => Some(&clone.resolved),
        _ => None,
    }
}

fn point_of<'a>(
    slot: &PointSlot,
    lookup: &impl Fn(NodeId) -> Option<&'a ObjectSnapshot>,
) -> Option<Point> {
    match *slot {
        PointSlot::Free { point } => Some(point),
        PointSlot::Anchor { node, anchor } => {
            Some(anchored_path(lookup(node)?)?.anchor(anchor)?.point)
        }
        PointSlot::SegmentMid { node, anchor } => {
            let (a, b) = segment(anchored_path(lookup(node)?)?, anchor)?;
            Some(cubic_at(a, b, 0.5))
        }
        PointSlot::ShapePoint { node, feature } => match lookup(node)? {
            ObjectSnapshot::Primitive(primitive) => shape_point(primitive, feature),
            _ => None,
        },
    }
}

/// The segment of `path` that starts at `anchor`: its two end anchors.
#[must_use]
pub fn segment(path: &PathSnapshot, anchor: AnchorId) -> Option<(AnchorSnapshot, AnchorSnapshot)> {
    let outline = path.outline_of(anchor)?;
    let at = outline.anchors.iter().position(|a| a.id == anchor)?;
    let next = if at + 1 < outline.anchors.len() {
        at + 1
    } else if outline.closed && outline.anchors.len() > 1 {
        0
    } else {
        return None;
    };
    Some((outline.anchors[at], outline.anchors[next]))
}

/// The point at `t` of the cubic segment from `a` to `b`.
#[must_use]
pub fn cubic_at(a: AnchorSnapshot, b: AnchorSnapshot, t: f64) -> Point {
    let p0 = a.point;
    let p1 = a.point.translated(a.handle_out);
    let p2 = b.point.translated(b.handle_in);
    let p3 = b.point;
    let u = 1.0 - t;
    let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Point::new(
        w0 * p0.x + w1 * p1.x + w2 * p2.x + w3 * p3.x,
        w0 * p0.y + w1 * p1.y + w2 * p2.y + w3 * p3.y,
    )
}

/// The four box corners of a rectangle, clockwise from the top-left, rotated with the shape.
fn rect_corners(primitive: &PrimitiveSnapshot) -> Option<[Point; 4]> {
    let Shape::Rect { bounds, .. } = primitive.shape else {
        return None;
    };
    let (x, y) = (bounds.origin.x, bounds.origin.y);
    let (w, h) = (bounds.width.as_mm(), bounds.height.as_mm());
    let center = shape_center(&primitive.shape);
    Some(
        [
            Point::new(x, y),
            Point::new(x + w, y),
            Point::new(x + w, y + h),
            Point::new(x, y + h),
        ]
        .map(|p| p.rotated_around(center, primitive.rotation)),
    )
}

/// The point `feature` of `primitive` in document millimetres, `None` when the shape has no such
/// point.
#[must_use]
pub fn shape_point(primitive: &PrimitiveSnapshot, feature: ShapeFeature) -> Option<Point> {
    let index = |i: u32| usize::try_from(i).ok();
    match feature {
        ShapeFeature::Center => Some(shape_center(&primitive.shape)),
        ShapeFeature::Corner(i) => rect_corners(primitive)?.get(index(i)?).copied(),
        ShapeFeature::Vertex(i) => match primitive.shape {
            Shape::Polygon { .. } | Shape::Star { .. } | Shape::Ellipse { arc: None, .. } => {
                let outline = crate::outline_of_rotated(&primitive.shape, primitive.rotation);
                outline.anchors.get(index(i)?).map(|a| a.point)
            }
            _ => None,
        },
        ShapeFeature::EdgeMid(i) => {
            if let Some(corners) = rect_corners(primitive) {
                let at = index(i)?;
                let (a, b) = (*corners.get(at)?, corners[(at + 1) % 4]);
                return Some(Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)));
            }
            let (Shape::Polygon { .. } | Shape::Star { .. }) = primitive.shape else {
                return None;
            };
            let outline = crate::outline_of_rotated(&primitive.shape, primitive.rotation);
            let anchors = &outline.anchors;
            let at = index(i)?;
            let (a, b) = (anchors.get(at)?, &anchors[(at + 1) % anchors.len()]);
            let corner = |o: &crate::OutlineAnchor| AnchorSnapshot {
                id: AnchorId::new(0, 0),
                point: o.point,
                handle_in: o.handle_in,
                handle_out: o.handle_out,
                kind: o.kind,
            };
            Some(cubic_at(corner(a), corner(b), 0.5))
        }
    }
}

/// The centre and radius (mm) of the circle a curve slot names, `None` when it is gone or no
/// longer a circle within [`CIRCLE_TOLERANCE`].
fn circle_of<'a>(
    slot: &CurveSlot,
    lookup: &impl Fn(NodeId) -> Option<&'a ObjectSnapshot>,
) -> Option<(Point, f64)> {
    match *slot {
        CurveSlot::Circle { node } => match lookup(node)? {
            ObjectSnapshot::Primitive(PrimitiveSnapshot {
                shape: Shape::Ellipse { frame, .. },
                ..
            }) => {
                let (rx, ry) = (frame.rx.as_mm(), frame.ry.as_mm());
                ((rx - ry).abs() <= CIRCLE_TOLERANCE.as_mm() && rx > DEGENERATE_MM)
                    .then_some((frame.center, rx))
            }
            _ => None,
        },
        CurveSlot::Arc { node, anchor } => {
            let (a, b) = segment(anchored_path(lookup(node)?)?, anchor)?;
            arc_circle(a, b).map(|(center, radius)| (center, radius.as_mm()))
        }
        CurveSlot::RoundCorner { node, corner } => match lookup(node)? {
            ObjectSnapshot::Primitive(primitive) => round_corner(primitive, corner),
            _ => None,
        },
        CurveSlot::FreeCircle { center, radius } => Some((center, radius.as_mm())),
    }
}

/// The circle through the cubic segment from `a` to `b`, when the segment is a circular arc
/// within [`CIRCLE_TOLERANCE`].
#[must_use]
pub fn arc_circle(a: AnchorSnapshot, b: AnchorSnapshot) -> Option<(Point, Length)> {
    let (center, radius) = circumcircle(a.point, cubic_at(a, b, 0.5), b.point)?;
    [0.25, 0.75, 0.125, 0.875]
        .into_iter()
        .all(|t| {
            (center.vector_to(cubic_at(a, b, t)).length() - radius).abs()
                <= CIRCLE_TOLERANCE.as_mm()
        })
        .then_some((center, Length::from_mm(radius)))
}

/// The circle through three points, `None` when they are (nearly) on one line.
fn circumcircle(first: Point, middle: Point, last: Point) -> Option<(Point, f64)> {
    let det = 2.0
        * (first.x * (middle.y - last.y)
            + middle.x * (last.y - first.y)
            + last.x * (first.y - middle.y));
    if det.abs() <= DEGENERATE_MM {
        return None;
    }
    let square = |p: Point| p.x * p.x + p.y * p.y;
    let (s1, s2, s3) = (square(first), square(middle), square(last));
    let center = Point::new(
        (s1 * (middle.y - last.y) + s2 * (last.y - first.y) + s3 * (first.y - middle.y)) / det,
        (s1 * (last.x - middle.x) + s2 * (first.x - last.x) + s3 * (middle.x - first.x)) / det,
    );
    let radius = center.vector_to(first).length();
    radius.is_finite().then_some((center, radius))
}

/// The circle of a rectangle's rounded corner `corner` (clockwise from the top-left).
fn round_corner(primitive: &PrimitiveSnapshot, corner: u32) -> Option<(Point, f64)> {
    let Shape::Rect {
        bounds,
        corner_radii,
    } = primitive.shape
    else {
        return None;
    };
    let radii = crate::effective_corner_radii(bounds, corner_radii);
    let (radius, sx, sy) = match corner {
        0 => (radii.tl, 1.0, 1.0),
        1 => (radii.tr, -1.0, 1.0),
        2 => (radii.br, -1.0, -1.0),
        3 => (radii.bl, 1.0, -1.0),
        _ => return None,
    };
    let r = radius.as_mm();
    if r <= DEGENERATE_MM {
        return None;
    }
    let corners = rect_corners(&PrimitiveSnapshot {
        rotation: Angle::from_radians(0.0),
        ..primitive.clone()
    })?;
    let base = corners[usize::try_from(corner).ok()?];
    let local = Point::new(base.x + sx * r, base.y + sy * r);
    Some((
        local.rotated_around(shape_center(&primitive.shape), primitive.rotation),
        r,
    ))
}

#[cfg(test)]
#[path = "dimension_read_tests.rs"]
mod tests;
