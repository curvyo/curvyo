//! Resizes a primitive's frame by a handle drag or a typed size and keeps the
//! resize's anchor fixed in document space
//! (`specs/0005-object-transform/adrs.md`: "a resize rewrites geometry").
//! Split out of [`crate::transform_drag`], which resolves the gestures.

use curvyo_document_core::{
    Angle, Corner, EllipseFrame, Length, ObjectSnapshot, Point, PrimitiveSnapshot, RectBounds,
    Shape, Vec2, shape_center, translate_shape,
};

use crate::ResizeDirection;
use crate::group_transform::{is_stretch, scale_primitive};
use crate::oriented_box::OrientedBox;
use crate::transform_drag::{CornerRadiusScaling, ResizeOptions};
use crate::transform_math::{
    is_polygon_or_star, resize_anchor_local_position, resize_local_box, scaled_and_floored,
    stroke_or_radius_factor,
};

/// A primitive's resized frame and the stroke/radius factor that goes
/// with it. Rectangles and ellipses resize their local box (the corner
/// radius scales with the factor only under [`CornerRadiusScaling::Proportional`],
/// acceptance criterion 9 of slice 5, criterion 23 of `unified-object-editing`); a polygon or
/// star that is not stretched is a uniform scale about the resize's anchor, outer radius
/// times the factor (`0047` criterion 10; a stretch is [`polygon_star_stretch`]).
pub(crate) fn resize_primitive(
    primitive: &PrimitiveSnapshot,
    start_box: &OrientedBox,
    direction: ResizeDirection,
    local_delta: Vec2,
    modifiers: (bool, bool),
    radius_scaling: CornerRadiusScaling,
) -> (PrimitiveSnapshot, f64) {
    let (shift, ctrl) = modifiers;
    let box_resize = || {
        resize_local_box(
            start_box.min,
            start_box.max,
            direction,
            local_delta,
            shift,
            ctrl,
        )
    };
    let (shape, factor) = match primitive.shape {
        Shape::Rect { corner_radii, .. } => {
            let resized = box_resize();
            let factor = stroke_or_radius_factor(resized.sx, resized.sy);
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
            let shape = Shape::Rect {
                bounds: RectBounds {
                    origin: resized.min,
                    width: Length::from_mm(resized.max.x - resized.min.x),
                    height: Length::from_mm(resized.max.y - resized.min.y),
                },
                corner_radii: radii,
            };
            (shape, factor)
        }
        Shape::Ellipse { .. } => {
            let resized = box_resize();
            let shape = Shape::Ellipse {
                frame: EllipseFrame {
                    center: Point::new(
                        f64::midpoint(resized.min.x, resized.max.x),
                        f64::midpoint(resized.min.y, resized.max.y),
                    ),
                    rx: Length::from_mm((resized.max.x - resized.min.x) / 2.0),
                    ry: Length::from_mm((resized.max.y - resized.min.y) / 2.0),
                },
            };
            (shape, stroke_or_radius_factor(resized.sx, resized.sy))
        }
        Shape::Polygon { .. } | Shape::Star { .. } => {
            let sx = box_resize().sx;
            let anchor =
                resize_anchor_local_position(start_box.min, start_box.max, direction, shift);
            let scaled = scale_primitive(
                primitive,
                start_box.to_document(anchor),
                (sx, sx),
                radius_scaling,
            );
            return (scaled, sx);
        }
    };
    let anchor_local = resize_anchor_local_position(start_box.min, start_box.max, direction, shift);
    let pinned = pin_resize_anchor(shape, start_box.pivot, start_box.angle, anchor_local);
    (
        PrimitiveSnapshot {
            shape: pinned,
            ..primitive.clone()
        },
        factor,
    )
}

/// A polygon's or star's stretch along its box axes (`specs/0047-transform-polish/`
/// criterion 12): any resize handle or a typed size, with the opposite corner or side
/// (or the centre under Shift) fixed. Unequal factors are a stretch: the shape becomes
/// its `converted` path, scaled along the box's axes. `None` for any other object and
/// for equal factors, which [`resize_primitive`] scales uniformly and keeps the shape.
pub(crate) fn polygon_star_stretch(
    start: &ObjectSnapshot,
    start_box: &OrientedBox,
    direction: ResizeDirection,
    local_delta: Vec2,
    options: &ResizeOptions,
) -> Option<(ObjectSnapshot, f64)> {
    if !is_polygon_or_star(start) {
        return None;
    }
    let resized = resize_local_box(
        start_box.min,
        start_box.max,
        direction,
        local_delta,
        options.shift,
        options.ctrl,
    );
    let (sx, sy) = (resized.sx, resized.sy);
    if !is_stretch(sx, sy) {
        return None;
    }
    let Some(path) = options.converted else {
        return Some((start.clone(), 1.0));
    };
    let pivot_local =
        resize_anchor_local_position(start_box.min, start_box.max, direction, options.shift);
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
