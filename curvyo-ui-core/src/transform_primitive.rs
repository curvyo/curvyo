//! Resizes a primitive's frame by a handle drag or a typed size and keeps the
//! resize's anchor fixed in document space
//! (`specs/0005-object-transform/adrs.md`: "a resize rewrites geometry").
//! Split out of [`crate::transform_drag`], which resolves the gestures.

use curvyo_document_core::{
    Angle, Corner, EllipseFrame, Length, ObjectSnapshot, PathSnapshot, Point, PrimitiveSnapshot,
    RectBounds, Shape, shape_center, translate_shape,
};

use crate::ResizeDirection;
use crate::group_transform::{is_stretch, scale_primitive};
use crate::mirror::{Mirror, mirrored};
use crate::oriented_box::OrientedBox;
use crate::transform_drag::CornerRadiusScaling;
use crate::transform_math::{
    ResizedBox, is_polygon_or_star, resize_anchor_local_position, scaled_and_floored,
    stroke_or_radius_factor,
};

/// A primitive's resized frame and the stroke/radius factor that goes
/// with it. Rectangles and ellipses resize their local box (the corner
/// radius scales with the factor only under [`CornerRadiusScaling::Proportional`],
/// acceptance criterion 9 of slice 5, criterion 23 of `unified-object-editing`); a polygon or
/// star that is not stretched is a uniform scale about the resize's anchor, outer radius
/// times the factor (`0047` criterion 10; a stretch is [`polygon_star_stretch`]). A negative
/// factor is the resize by the magnitudes, mirrored about the anchor along the box's own axes
/// (`specs/0056-mirror-and-negative-scale/` criteria 12 and 18).
pub(crate) fn resize_primitive(
    primitive: &PrimitiveSnapshot,
    start_box: &OrientedBox,
    resized: &ResizedBox,
    (direction, shift): (ResizeDirection, bool),
    radius_scaling: CornerRadiusScaling,
) -> (PrimitiveSnapshot, f64) {
    let anchor_local = resize_anchor_local_position(start_box.min, start_box.max, direction, shift);
    let anchor = start_box.to_document(anchor_local);
    let mirror = Mirror::of_factors(anchor, start_box.angle, resized.sx, resized.sy);
    let factor = stroke_or_radius_factor(resized.sx, resized.sy);
    let (min, max) = magnitude_box(resized, anchor_local);
    let shape = match primitive.shape {
        Shape::Rect { corner_radii, .. } => {
            // `Keep` hands the stored radii back untouched, so no register is
            // rewritten and a concurrent radius edit is not beaten.
            // `Proportional` multiplies all four stored radii by the one factor.
            let radii = match radius_scaling {
                CornerRadiusScaling::Keep => corner_radii,
                CornerRadiusScaling::Proportional => {
                    Corner::ALL.iter().fold(corner_radii, |scaled, &corner| {
                        scaled.with(
                            corner,
                            scaled_and_floored(
                                corner_radii.get(corner),
                                factor,
                                Length::from_mm(0.0),
                            ),
                        )
                    })
                }
            };
            Shape::Rect {
                bounds: RectBounds {
                    origin: min,
                    width: Length::from_mm(max.x - min.x),
                    height: Length::from_mm(max.y - min.y),
                },
                corner_radii: radii,
            }
        }
        // Above 100 % the box is the drawn outline's: map the frame as an arc's.
        Shape::Ellipse {
            frame,
            arc: None,
            curve,
        } if curvyo_document_core::is_beyond_ellipse(&primitive.shape) => Shape::Ellipse {
            frame: arc_frame_in_box(frame, (start_box.min, start_box.max), (min, max)),
            arc: None,
            curve,
        },
        Shape::Ellipse {
            arc: None, curve, ..
        } => Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y)),
                rx: Length::from_mm((max.x - min.x) / 2.0),
                ry: Length::from_mm((max.y - min.y) / 2.0),
            },
            arc: None,
            curve,
        },
        // An arc's box is the box of the drawn part (`specs/0021-ellipse-arcs-and-shaping/`
        // criteria 9 and 10): the frame follows the map from the old box to the new one and
        // the parametric angles stay.
        Shape::Ellipse {
            frame,
            arc: Some(arc),
            curve,
        } => Shape::Ellipse {
            frame: arc_frame_in_box(frame, (start_box.min, start_box.max), (min, max)),
            arc: Some(arc),
            curve,
        },
        Shape::Polygon { .. } | Shape::Star { .. } => {
            let magnitude = resized.sx.abs();
            let scaled = scale_primitive(primitive, anchor, (magnitude, magnitude), radius_scaling);
            return (mirrored_primitive_of(&scaled, mirror), magnitude);
        }
    };
    let pinned = pin_resize_anchor(shape, start_box.pivot, start_box.angle, anchor_local);
    let scaled = PrimitiveSnapshot {
        shape: pinned,
        ..primitive.clone()
    };
    (mirrored_primitive_of(&scaled, mirror), factor)
}

/// The ellipse frame whose arc box `old` becomes `new` under the per-axis map between the
/// two boxes (both in the frame's local, unrotated coordinates). A box side of zero
/// length keeps that axis's radius.
fn arc_frame_in_box(frame: EllipseFrame, old: (Point, Point), new: (Point, Point)) -> EllipseFrame {
    let axis = |centre: f64, radius: f64, old: (f64, f64), new: (f64, f64)| {
        let span = old.1 - old.0;
        if span.abs() <= f64::EPSILON {
            return (centre - old.0 + new.0, radius);
        }
        let factor = (new.1 - new.0) / span;
        (new.0 + (centre - old.0) * factor, radius * factor)
    };
    let (cx, rx) = axis(
        frame.center.x,
        frame.rx.as_mm(),
        (old.0.x, old.1.x),
        (new.0.x, new.1.x),
    );
    let (cy, ry) = axis(
        frame.center.y,
        frame.ry.as_mm(),
        (old.0.y, old.1.y),
        (new.0.y, new.1.y),
    );
    EllipseFrame {
        center: Point::new(cx, cy),
        rx: Length::from_mm(rx),
        ry: Length::from_mm(ry),
    }
}

/// `primitive` reflected by `mirror` (a primitive stays a primitive).
fn mirrored_primitive_of(primitive: &PrimitiveSnapshot, mirror: Mirror) -> PrimitiveSnapshot {
    match mirrored(&ObjectSnapshot::Primitive(primitive.clone()), mirror) {
        ObjectSnapshot::Primitive(mirrored) => mirrored,
        // invariant: `mirrored` keeps the kind of its object.
        ObjectSnapshot::Path(_)
        | ObjectSnapshot::Clone(_)
        | ObjectSnapshot::Text(_)
        | ObjectSnapshot::Image(_) => primitive.clone(),
    }
}

/// The local box the resize by the magnitudes of `resized`'s factors gives: `resized`'s
/// normalised box with each axis whose factor is negative reflected back across the
/// anchor (the mirror is applied after, criterion 18).
fn magnitude_box(resized: &ResizedBox, anchor: Point) -> (Point, Point) {
    let axis = |min: f64, max: f64, anchor: f64, factor: f64| {
        if factor < 0.0 {
            (2.0 * anchor - max, 2.0 * anchor - min)
        } else {
            (min, max)
        }
    };
    let (min_x, max_x) = axis(resized.min.x, resized.max.x, anchor.x, resized.sx);
    let (min_y, max_y) = axis(resized.min.y, resized.max.y, anchor.y, resized.sy);
    (Point::new(min_x, min_y), Point::new(max_x, max_y))
}

/// A polygon's or star's stretch along its box axes (`specs/0047-transform-polish/`
/// criterion 12): any resize handle or a typed size, with the opposite corner or side
/// (or the centre under Shift) fixed. Unequal magnitudes are a stretch: the shape becomes
/// its `converted` path, scaled along the box's axes with the signed factors. `None` for
/// any other object and for equal magnitudes, which [`resize_primitive`] scales uniformly
/// and keeps the shape.
pub(crate) fn polygon_star_stretch(
    start: &ObjectSnapshot,
    start_box: &OrientedBox,
    resized: &ResizedBox,
    (direction, shift): (ResizeDirection, bool),
    converted: Option<&PathSnapshot>,
) -> Option<(ObjectSnapshot, f64)> {
    if !is_polygon_or_star(start) {
        return None;
    }
    let (sx, sy) = (resized.sx, resized.sy);
    if !is_stretch(sx, sy) {
        return None;
    }
    let Some(path) = converted else {
        return Some((start.clone(), 1.0));
    };
    let pivot_local = resize_anchor_local_position(start_box.min, start_box.max, direction, shift);
    let scaled = path.scaled_along(start_box.to_document(pivot_local), sx, sy, start_box.angle);
    Some((
        ObjectSnapshot::Path(scaled),
        stroke_or_radius_factor(sx, sy),
    ))
}

/// Keeps a resize's anchor fixed in document space.
///
/// A primitive rotates about its *own* frame center, which a resize just
/// moved — so on a rotated object the anchor (the opposite corner or
/// edge, or the center under Shift) would swing across the canvas.
/// `after` is the resized shape in its local frame, `before_pivot` the
/// frame center before the resize, `anchor_local` the anchor in local
/// coordinates (unchanged by the resize); the whole frame is translated
/// by however far the anchor moved. A no-op at zero rotation, where the
/// local and document frames agree, and for a polygon/star, whose center
/// does not move. The Select tool's resize and typed size both call this.
pub(crate) fn pin_resize_anchor(
    after: Shape,
    before_pivot: Point,
    rotation: Angle,
    anchor_local: Point,
) -> Shape {
    let before = anchor_local.rotated_around(before_pivot, rotation);
    let after_anchor = anchor_local.rotated_around(shape_center(&after), rotation);
    translate_shape(after, after_anchor.vector_to(before))
}
