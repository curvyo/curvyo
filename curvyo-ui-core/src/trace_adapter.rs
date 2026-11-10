//! Turns the shape pass's plain primitives into the document's: the `Shape` of a `Primitive`, the
//! path it draws, and the readback step that replaces traced outlines by them
//! (`specs/0088-trace-recognition` Part B).

use curvyo_document_core::{
    Angle as DocAngle, CornerRadii, Curve, EllipseFrame, InnerRatio, Length, LineJoin,
    Point as DocPoint, PointCount, RectBounds, Shape, StarFrame, drawing_outline_of_rotated,
};
use curvyo_vectorize_core::Angle;
use kurbo::BezPath;

use crate::svg_import_match::outline_path;
use crate::trace_shapes::{Join, Primitive, ShapeRegion};
use crate::trace_transfer::{TraceWire, read_wire};

/// The document `Shape` of `primitive` and the rotation as the document's angle.
///
/// # Panics
/// Never for a primitive the shape pass makes or the wire reader accepts: their point counts
/// lie in `POINTS` and their inner ratios in (0, 1), the document's own bounds (a test asserts
/// it).
#[must_use]
#[allow(clippy::expect_used)] // invariant: counts and ratios were checked against these bounds.
pub(crate) fn shape_of(primitive: &Primitive, rotation: Angle) -> (Shape, DocAngle) {
    let point = |p: kurbo::Point| DocPoint::new(p.x, p.y);
    let star_frame = |centre, radius: curvyo_vectorize_core::Mm, angle: Angle| StarFrame {
        center: point(centre),
        radius: Length::from_mm(radius.as_mm()),
        angle: DocAngle::from_radians(angle.as_radians()),
    };
    let shape = match *primitive {
        Primitive::Rect {
            origin,
            width,
            height,
            corner_radius,
        } => Shape::Rect {
            bounds: RectBounds {
                origin: point(origin),
                width: Length::from_mm(width.as_mm()),
                height: Length::from_mm(height.as_mm()),
            },
            corner_radii: CornerRadii::uniform(Length::from_mm(corner_radius.as_mm())),
        },
        Primitive::Ellipse { centre, rx, ry } => Shape::Ellipse {
            frame: EllipseFrame {
                center: point(centre),
                rx: Length::from_mm(rx.as_mm()),
                ry: Length::from_mm(ry.as_mm()),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        },
        Primitive::Polygon {
            centre,
            radius,
            angle,
            points,
        } => Shape::Polygon {
            frame: star_frame(centre, radius, angle),
            point_count: PointCount::new(points).expect("point count in range"),
            curve: Curve::POLYGON_DEFAULT,
        },
        Primitive::Star {
            centre,
            radius,
            angle,
            points,
            inner_ratio,
        } => Shape::Star {
            frame: star_frame(centre, radius, angle),
            point_count: PointCount::new(points).expect("point count in range"),
            inner_ratio: InnerRatio::new(inner_ratio).expect("inner ratio in range"),
            curve: Curve::POLYGON_DEFAULT,
        },
    };
    (shape, DocAngle::from_radians(rotation.as_radians()))
}

/// The numbers of a trace job read back (`None` when they are not a well-formed result), each
/// recognised outline replaced by its primitive's, as the session keeps the result.
#[must_use]
pub fn read_trace_wire(wire: &[f64]) -> Option<TraceWire> {
    let mut read = read_wire(wire)?;
    let (outlines, shapes) = with_primitives(read.outlines, read.shapes);
    read.outlines = outlines;
    read.shapes = shapes;
    Some(read)
}

/// The document's line join for `join`.
#[must_use]
pub(crate) const fn line_join(join: Join) -> LineJoin {
    match join {
        Join::Miter => LineJoin::Miter,
        Join::Round => LineJoin::Round,
        Join::Bevel => LineJoin::Bevel,
    }
}

/// The outline a primitive draws, as a path.
#[must_use]
pub(crate) fn primitive_path(primitive: &Primitive, rotation: Angle) -> BezPath {
    let (shape, rotation) = shape_of(primitive, rotation);
    outline_path(&drawing_outline_of_rotated(&shape, rotation))
}

/// [`primitive_path`] wound as `traced` is, so a hole stays a hole under the nonzero fill of a
/// compound path (`specs/0088` Part B: a primitive's outline always winds one way).
#[must_use]
pub(crate) fn primitive_path_like(
    primitive: &Primitive,
    rotation: Angle,
    traced: &BezPath,
) -> BezPath {
    let path = primitive_path(primitive, rotation);
    if kurbo::Shape::area(&path) * kurbo::Shape::area(traced) < 0.0 {
        path.reverse_subpaths()
    } else {
        path
    }
}

/// `outlines` with each recognised outline of `regions` replaced by its primitive's, wound as the
/// traced one, and the regions kept. A region whose fill would change by more than its outlines'
/// own accepted area changes (a hole that no longer cuts, for any reason) keeps its traced paths
/// and is dropped: a hole is never lost.
pub(crate) fn with_primitives(
    mut outlines: Vec<BezPath>,
    regions: Vec<ShapeRegion>,
) -> (Vec<BezPath>, Vec<ShapeRegion>) {
    let mut kept = Vec::with_capacity(regions.len());
    for region in regions {
        let traced: Vec<&BezPath> = region.outlines.iter().map(|&i| &outlines[i]).collect();
        let drawn: Vec<BezPath> = region
            .outlines
            .iter()
            .zip(&region.shapes)
            .map(|(&i, shape)| match shape {
                Some((shape, rotation)) => primitive_path_like(shape, *rotation, &outlines[i]),
                None => outlines[i].clone(),
            })
            .collect();
        if !keeps_fill(&traced, &drawn) {
            continue;
        }
        for (&i, path) in region.outlines.iter().zip(drawn) {
            outlines[i] = path;
        }
        kept.push(region);
    }
    (outlines, kept)
}

/// Whether `drawn` fills what `traced` fills (outline for outline, nonzero): the net signed area
/// changes by no more than the outlines' own area changes add up to.
pub(crate) fn keeps_fill(traced: &[&BezPath], drawn: &[BezPath]) -> bool {
    let area = |p: &BezPath| kurbo::Shape::area(p);
    let before: f64 = traced.iter().map(|p| area(p)).sum();
    let after: f64 = drawn.iter().map(area).sum();
    let own: f64 = traced
        .iter()
        .zip(drawn)
        .map(|(t, d)| (area(t).abs() - area(d).abs()).abs())
        .sum();
    (after - before).abs() <= own + 1e-9 * before.abs().max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace_shapes::POINTS;

    fn square(x: f64, side: f64, clockwise: bool) -> BezPath {
        let mut p = BezPath::new();
        let c = [(x, x), (x + side, x), (x + side, x + side), (x, x + side)];
        let order: Vec<_> = if clockwise {
            c.to_vec()
        } else {
            c.iter().rev().copied().collect()
        };
        p.move_to(order[0]);
        for q in &order[1..] {
            p.line_to(*q);
        }
        p.close_path();
        p
    }

    /// A hole drawn the outer's way round would fill the frame: that is not the same fill.
    #[test]
    fn a_hole_that_no_longer_cuts_is_not_the_same_fill() {
        let (outer, hole) = (square(0.0, 10.0, true), square(2.0, 6.0, false));
        let traced = [&outer, &hole];
        assert!(keeps_fill(&traced, &[outer.clone(), hole.clone()]));
        assert!(keeps_fill(
            &traced,
            &[square(0.0, 10.01, true), square(2.0, 5.99, false)]
        ));
        assert!(!keeps_fill(
            &traced,
            &[outer.clone(), square(2.0, 6.0, true)]
        ));
    }

    /// The point counts the shape pass allows are the document's.
    #[test]
    fn the_point_bounds_are_the_documents() {
        assert_eq!(*POINTS.start(), PointCount::MIN);
        assert_eq!(*POINTS.end(), PointCount::MAX);
        assert!(PointCount::new(POINTS.start() - 1).is_err());
        assert!(PointCount::new(POINTS.end() + 1).is_err());
        for ratio in [0.0, 1.0, f64::NAN, -0.5, 1.5] {
            assert!(crate::trace_shapes::valid_ratio(ratio).is_none());
            assert!(InnerRatio::new(ratio).is_err());
        }
        for ratio in [f64::MIN_POSITIVE, 0.5, 0.999_999] {
            assert_eq!(crate::trace_shapes::valid_ratio(ratio), Some(ratio));
            assert!(InnerRatio::new(ratio).is_ok());
        }
    }
}
