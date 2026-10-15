//! One document-axes map applied to one object, kind by kind
//! (`specs/0019-multi-object-transform/`, criteria 20, 26 and 41): the result a
//! scale, a rotate or a skew of a selection gives each of its objects. The group
//! gesture computes its map once and calls [`apply`] for every object, in the
//! preview and on release (`adrs.md` decision 1), so the two cannot disagree.
//! It is a function of one object and one map, so a later group object can call
//! it with its own frame (`adrs.md`, forward note).

use curvyo_document_core::{
    Angle, Corner, Document, EllipseFrame, Length, ObjectEditError, ObjectSnapshot, PathSnapshot,
    Point, PrimitiveSnapshot, RectBounds, Shape, StarFrame,
};

use crate::ResizeDirection;
use crate::conversion::is_circle;
use crate::mirror::{Mirror, mirrored};
use crate::oriented_box::OrientedBox;
use crate::skew_math::{skew_angle, skew_factor, skew_frame};
use crate::transform_commit::is_sane;
use crate::transform_drag::{CornerRadiusScaling, ScaleModes, StrokeScaling, scale_stroke};
use crate::transform_handle_layout::EditHandle;
use crate::transform_math::{
    ExtentSigns, ResizedBox, resize_anchor_local_position, resize_local_box_with_prior,
    rotate_delta_angle, rotate_pivot, scaled_and_floored, stroke_or_radius_factor,
};

/// Two scale magnitudes whose difference is below this fraction of the larger are one
/// factor (`specs/0056-mirror-and-negative-scale/` Terms, "Stretch").
const UNIFORM_EPSILON: f64 = 1e-9;

/// Whether a scale by `(sx, sy)` is a stretch: the magnitudes differ. A scale with
/// equal magnitudes (a uniform scale, a mirror, a half turn, the identity) is not, and
/// converts nothing (`specs/0056-mirror-and-negative-scale/` criterion 18).
pub(crate) fn is_stretch(sx: f64, sy: f64) -> bool {
    (sx.abs() - sy.abs()).abs() > UNIFORM_EPSILON * sx.abs().max(sy.abs())
}

/// What a group gesture does to every selected object, in the document axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum GroupMap {
    /// A scale by `(sx, sy)` about `pivot`.
    Scale { pivot: Point, sx: f64, sy: f64 },
    /// A turn by `delta` about `pivot`.
    Rotate { pivot: Point, delta: Angle },
    /// A shear about the line through `fixed`: `x += ku * y`, `y += kv * x`, both
    /// measured from `fixed`.
    Shear { fixed: Point, ku: f64, kv: f64 },
}

/// `object` after `map`; `modes` decide whether a scale also scales the stroke
/// width and the corner radii. A move, a rotate and a uniform scale never change
/// the kind; a stretch (unequal factors) turns an object that has a `converted`
/// path (a polygon, a star, a rectangle or ellipse off the axes) into that path,
/// scaled, and a skew turns every primitive into its `converted` path, sheared
/// (`specs/0047-transform-polish/` criteria 2 and 7).
pub(crate) fn apply(
    object: &ObjectSnapshot,
    map: &GroupMap,
    modes: ScaleModes,
    converted: Option<&PathSnapshot>,
) -> ObjectSnapshot {
    if let ObjectSnapshot::Clone(clone) = object {
        return crate::clone_gesture::map_clone(clone, map_affine(clone.id(), map), converted);
    }
    match *map {
        GroupMap::Scale { pivot, sx, sy } => {
            scale_object(object, pivot, (sx, sy), modes, converted)
        }
        GroupMap::Rotate { pivot, delta } => object.rotated(pivot, delta),
        GroupMap::Shear { fixed, ku, kv } => {
            let axes = Angle::from_radians(0.0);
            match (object, converted) {
                (ObjectSnapshot::Path(path), _) | (ObjectSnapshot::Primitive(_), Some(path)) => {
                    ObjectSnapshot::Path(path.sheared_along(fixed, ku, kv, axes))
                }
                (ObjectSnapshot::Primitive(_), None) | (ObjectSnapshot::Clone(_), _) => {
                    object.clone()
                }
            }
        }
    }
}

/// `map` as one affine map of the document (`id` only names the probe).
pub(crate) fn map_affine(
    id: curvyo_document_core::NodeId,
    map: &GroupMap,
) -> curvyo_document_core::Affine {
    let axes = Angle::from_radians(0.0);
    let map = *map;
    crate::clone_gesture::affine_of(id, &move |path: &PathSnapshot| match map {
        GroupMap::Scale { pivot, sx, sy } => path.scaled_along(pivot, sx, sy, axes),
        GroupMap::Rotate { pivot, delta } => path.rotated(pivot, delta),
        GroupMap::Shear { fixed, ku, kv } => path.sheared_along(fixed, ku, kv, axes),
    })
}

/// `object` scaled by `(sx, sy)` about `pivot` in the document axes: by the magnitudes
/// with the rules of `0019` criterion 20 and `0047`, then mirrored once per negative
/// factor about the same point (`specs/0056-mirror-and-negative-scale/` criterion 18).
fn scale_object(
    object: &ObjectSnapshot,
    pivot: Point,
    (sx, sy): (f64, f64),
    modes: ScaleModes,
    converted: Option<&PathSnapshot>,
) -> ObjectSnapshot {
    let scaled = scale_by_magnitudes(object, pivot, (sx.abs(), sy.abs()), modes, converted);
    mirrored(
        &scaled,
        Mirror::of_factors(pivot, Angle::from_radians(0.0), sx, sy),
    )
}

/// [`scale_object`] for factors that are not negative.
fn scale_by_magnitudes(
    object: &ObjectSnapshot,
    pivot: Point,
    (sx, sy): (f64, f64),
    modes: ScaleModes,
    converted: Option<&PathSnapshot>,
) -> ObjectSnapshot {
    let factor = stroke_or_radius_factor(sx, sy);
    let axes = Angle::from_radians(0.0);
    let mut scaled = match (object, converted) {
        (ObjectSnapshot::Path(path), _) => {
            ObjectSnapshot::Path(path.scaled_along(pivot, sx, sy, axes))
        }
        (ObjectSnapshot::Clone(_), _) => object.clone(),
        (ObjectSnapshot::Primitive(_), Some(path)) if is_stretch(sx, sy) => {
            ObjectSnapshot::Path(path.scaled_along(pivot, sx, sy, axes))
        }
        (ObjectSnapshot::Primitive(primitive), _) => {
            ObjectSnapshot::Primitive(scale_primitive(primitive, pivot, (sx, sy), modes.radius))
        }
    };
    if modes.stroke == StrokeScaling::Proportional {
        scale_stroke(&mut scaled, factor);
    }
    scaled
}

/// Whether `rotation` turns the local x axis onto the document y axis (an odd
/// number of quarter turns). Meaningful for an aligned primitive.
fn swaps_axes(rotation: Angle) -> bool {
    let quarters = (rotation.as_radians() / std::f64::consts::FRAC_PI_2).round();
    // A finite rotation is small enough for the cast; `rem_euclid` keeps -1 odd.
    #[allow(clippy::cast_possible_truncation)]
    let quarters = quarters as i64;
    quarters.rem_euclid(2) == 1
}

/// The multipliers of a rectangle's or ellipse's width and height (rx and ry),
/// and the rotation it ends with (criterion 20, second and third rows).
fn extent_factors(primitive: &PrimitiveSnapshot, (sx, sy): (f64, f64)) -> ((f64, f64), Angle) {
    let rotation = primitive.rotation;
    if !is_stretch(sx, sy) {
        return ((sx, sx), rotation);
    }
    if let Shape::Ellipse { frame } = &primitive.shape
        && is_circle(frame)
    {
        // A circle becomes an ellipse along the document axes, whatever its
        // rotation: the one write beyond a single object's resize (criterion 31).
        return ((sx, sy), Angle::from_radians(0.0));
    }
    if swaps_axes(rotation) {
        ((sy, sx), rotation)
    } else {
        ((sx, sy), rotation)
    }
}

pub(crate) fn scale_primitive(
    primitive: &PrimitiveSnapshot,
    pivot: Point,
    (sx, sy): (f64, f64),
    radius_scaling: CornerRadiusScaling,
) -> PrimitiveSnapshot {
    let map_point = |c: Point| {
        Point::new(
            pivot.x + (c.x - pivot.x) * sx,
            pivot.y + (c.y - pivot.y) * sy,
        )
    };
    let factor = stroke_or_radius_factor(sx, sy);
    let ((kx, ky), rotation) = extent_factors(primitive, (sx, sy));
    let shape = match primitive.shape {
        Shape::Rect {
            bounds,
            corner_radii,
        } => {
            let centre = map_point(Point::new(
                bounds.origin.x + bounds.width.as_mm() / 2.0,
                bounds.origin.y + bounds.height.as_mm() / 2.0,
            ));
            let (width, height) = (bounds.width.as_mm() * kx, bounds.height.as_mm() * ky);
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
                    origin: Point::new(centre.x - width / 2.0, centre.y - height / 2.0),
                    width: Length::from_mm(width),
                    height: Length::from_mm(height),
                },
                corner_radii: radii,
            }
        }
        Shape::Ellipse { frame } => Shape::Ellipse {
            frame: EllipseFrame {
                center: map_point(frame.center),
                rx: Length::from_mm(frame.rx.as_mm() * kx),
                ry: Length::from_mm(frame.ry.as_mm() * ky),
            },
        },
        Shape::Polygon { frame, point_count } => Shape::Polygon {
            frame: scaled_star_frame(frame, map_point(frame.center), factor),
            point_count,
        },
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
        } => Shape::Star {
            frame: scaled_star_frame(frame, map_point(frame.center), factor),
            point_count,
            inner_ratio,
        },
    };
    PrimitiveSnapshot {
        shape,
        rotation,
        ..primitive.clone()
    }
}

/// `frame` with its centre moved to `centre` and its outer radius times `factor`;
/// the angle and the inner ratio (a star's) are unchanged.
fn scaled_star_frame(frame: StarFrame, centre: Point, factor: f64) -> StarFrame {
    StarFrame {
        center: centre,
        radius: Length::from_mm(frame.radius.as_mm() * factor),
        angle: frame.angle,
    }
}

/// The resized group box of a resize of `direction` with the pointer going from `down_at`
/// to `current` and the `(shift, ctrl)` modifiers; `prior` holds the signs of the drag's
/// last frame whose extent was not 0 (`specs/0056-mirror-and-negative-scale/` criterion
/// 15). `None` for a non-finite pointer.
pub(crate) fn group_resized_box(
    start_box: &OrientedBox,
    direction: ResizeDirection,
    (down_at, current): (Point, Point),
    modifiers: (bool, bool),
    prior: ExtentSigns,
) -> Option<ResizedBox> {
    let delta = down_at.vector_to(current);
    // A non-finite pointer is refused before any arithmetic.
    if !(delta.x.is_finite() && delta.y.is_finite()) {
        return None;
    }
    Some(resize_local_box_with_prior(
        start_box.min,
        start_box.max,
        direction,
        delta,
        modifiers,
        prior,
    ))
}

/// The map of a group gesture on `handle` of `start_box`, with the pointer going
/// from `down_at` to `current` (a pair), the `(shift, ctrl)` modifiers and, for a
/// resize, the `prior` signs of [`group_resized_box`].
pub(crate) fn group_map(
    start_box: &OrientedBox,
    handle: EditHandle,
    (down_at, current): (Point, Point),
    (shift, ctrl): (bool, bool),
    prior: ExtentSigns,
) -> Option<GroupMap> {
    match handle {
        EditHandle::Resize(direction) => {
            let resized = group_resized_box(
                start_box,
                direction,
                (down_at, current),
                (shift, ctrl),
                prior,
            )?;
            let anchor =
                resize_anchor_local_position(start_box.min, start_box.max, direction, shift);
            Some(GroupMap::Scale {
                pivot: start_box.to_document(anchor),
                sx: resized.sx,
                sy: resized.sy,
            })
        }
        EditHandle::Rotate(direction) => {
            let pivot = rotate_pivot(start_box, direction, shift);
            Some(GroupMap::Rotate {
                pivot,
                delta: rotate_delta_angle(pivot, down_at, current, ctrl),
            })
        }
        EditHandle::Skew(side) => {
            let frame = skew_frame(start_box, side, shift);
            let k = skew_factor(
                &frame,
                skew_angle(start_box, side, down_at, current, shift, ctrl),
            );
            if k.abs() <= 0.0 {
                return None;
            }
            let (ku, kv) = if frame.along_u { (k, 0.0) } else { (0.0, k) };
            Some(GroupMap::Shear {
                fixed: frame.fixed_point,
                ku,
                kv,
            })
        }
        EditHandle::Move | EditHandle::Param(_) => None,
    }
}

/// `starts` after `map`, or `None` if any result is not finite and sane
/// (criterion 22: all or nothing).
pub(crate) fn map_all_checked(
    starts: &[ObjectSnapshot],
    converted: &[Option<PathSnapshot>],
    map: &GroupMap,
    modes: ScaleModes,
) -> Option<Vec<ObjectSnapshot>> {
    let mapped: Vec<ObjectSnapshot> = starts
        .iter()
        .enumerate()
        .map(|(index, object)| {
            apply(
                object,
                map,
                modes,
                converted.get(index).and_then(Option::as_ref),
            )
        })
        .collect();
    mapped.iter().all(is_sane).then_some(mapped)
}

/// [`map_all_checked`], or `starts` unchanged if a result is not sane.
pub(crate) fn map_all(
    starts: &[ObjectSnapshot],
    converted: &[Option<PathSnapshot>],
    map: &GroupMap,
    modes: ScaleModes,
) -> Vec<ObjectSnapshot> {
    map_all_checked(starts, converted, map, modes).unwrap_or_else(|| starts.to_vec())
}

/// The point a group gesture on `handle` holds fixed (the pivot marker): the
/// opposite corner or side, the centre under Shift for a resize; the rotate
/// pivot; the fixed line's point for a skew. `None` for the centre handle.
pub(crate) fn group_pivot(box_: &OrientedBox, handle: EditHandle, shift: bool) -> Option<Point> {
    match handle {
        EditHandle::Resize(direction) => Some(box_.to_document(resize_anchor_local_position(
            box_.min, box_.max, direction, shift,
        ))),
        EditHandle::Rotate(direction) => Some(rotate_pivot(box_, direction, shift)),
        EditHandle::Skew(side) => Some(skew_frame(box_, side, shift).fixed_point),
        EditHandle::Move | EditHandle::Param(_) => None,
    }
}

/// Writes a resolved group gesture with `transform_objects`; a stroke width the
/// document refuses is left out and the geometry still lands. `true` if the
/// document took the write (an object that is gone refuses it all).
///
/// `map` is the gesture's one map of the document, when it moved a set: the matrices of the
/// clones among `results` are settled with it (`specs/0049-linked-clone` criterion 16). A single
/// result never moves its own chain and passes `None`.
pub(crate) fn commit_group(
    document: &Document,
    results: &[ObjectSnapshot],
    stroke: StrokeScaling,
    label: &'static str,
    map: Option<&GroupMap>,
) -> bool {
    let with_width = stroke == StrokeScaling::Proportional;
    let mut results = results.to_vec();
    if let (Some(map), Some(first)) = (map, results.first())
        && results
            .iter()
            .any(|r| matches!(r, ObjectSnapshot::Clone(_)))
    {
        let g = map_affine(first.id(), map);
        crate::clone_gesture::settle_clone_matrices(&document.clone_links(), &mut results, g);
    }
    let results = &results;
    document.batch(label, |document| {
        let outcome = document.transform_objects(results, with_width);
        if with_width && outcome == Err(ObjectEditError::InvalidStrokeWidth) {
            // Anything else means an object is gone: nothing is left to write.
            return document.transform_objects(results, false).is_ok();
        }
        outcome.is_ok()
    })
}

const TRANSFORM_RESIZE_COMMIT_LABEL: &str = "transform_resize";
const TRANSFORM_ROTATE_COMMIT_LABEL: &str = "transform_rotate";
const TRANSFORM_SKEW_COMMIT_LABEL: &str = "transform_skew";
const TRANSFORM_OBJECTS_COMMIT_LABEL: &str = "transform_objects";

/// The commit label of a group gesture on `handle`: one step for all objects, named after the
/// gesture (`specs/0020-undo-redo` criteria 10 and 58).
pub(crate) const fn group_label(handle: EditHandle) -> &'static str {
    match handle {
        EditHandle::Resize(_) => TRANSFORM_RESIZE_COMMIT_LABEL,
        EditHandle::Rotate(_) => TRANSFORM_ROTATE_COMMIT_LABEL,
        EditHandle::Skew(_) => TRANSFORM_SKEW_COMMIT_LABEL,
        EditHandle::Move | EditHandle::Param(_) => TRANSFORM_OBJECTS_COMMIT_LABEL,
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{
        AnchorId, CornerRadii, Document, InnerRatio, NewAnchor, NodeId, PointCount,
    };

    use super::*;

    fn rect(document: &Document, x: f64, y: f64, w: f64, h: f64) -> ObjectSnapshot {
        let id = document.create_rect(RectBounds {
            origin: Point::new(x, y),
            width: Length::from_mm(w),
            height: Length::from_mm(h),
        });
        document.object(id).expect("exists")
    }

    fn turned(document: &Document, id: NodeId, degrees: f64) -> ObjectSnapshot {
        let object = document.object(id).expect("exists");
        let rotated = object.rotated(
            Point::new(0.0, 0.0),
            Angle::from_radians(degrees.to_radians()),
        );
        document.rotate_object(&rotated).expect("rotates");
        document.object(id).expect("exists")
    }

    fn scale(sx: f64, sy: f64) -> GroupMap {
        GroupMap::Scale {
            pivot: Point::new(0.0, 0.0),
            sx,
            sy,
        }
    }

    fn bounds_of(object: &ObjectSnapshot) -> RectBounds {
        let ObjectSnapshot::Primitive(PrimitiveSnapshot {
            shape: Shape::Rect { bounds, .. },
            ..
        }) = object
        else {
            panic!("a rectangle");
        };
        *bounds
    }

    /// Criterion 18, worked example: a 10 x 10 rectangle at (20, 0) scaled by
    /// (2, 3) about the origin is (40, 0) to (60, 30).
    #[test]
    fn a_rectangle_is_stretched_along_the_document_axes() {
        let document = Document::new(1);
        let object = rect(&document, 20.0, 0.0, 10.0, 10.0);
        let result = apply(&object, &scale(2.0, 3.0), ScaleModes::default(), None);
        let b = bounds_of(&result);
        assert_eq!((b.origin.x, b.origin.y), (40.0, 0.0));
        assert_eq!((b.width.as_mm(), b.height.as_mm()), (20.0, 30.0));
        assert_eq!(result.rotation(), object.rotation());
    }

    /// Criterion 20: an aligned rectangle turned 90 degrees swaps the factors
    /// onto its width and height; its rotation stays.
    #[test]
    fn a_quarter_turned_rectangle_takes_the_factors_swapped() {
        let document = Document::new(1);
        let id = document.create_rect(RectBounds {
            origin: Point::new(-10.0, -5.0),
            width: Length::from_mm(20.0),
            height: Length::from_mm(10.0),
        });
        let object = turned(&document, id, 90.0);
        let result = apply(&object, &scale(2.0, 3.0), ScaleModes::default(), None);
        let b = bounds_of(&result);
        assert!((b.width.as_mm() - 60.0).abs() < 1e-9, "width times sy");
        assert!((b.height.as_mm() - 20.0).abs() < 1e-9, "height times sx");
        assert!((result.rotation().as_radians() - object.rotation().as_radians()).abs() < 1e-12);
    }

    /// Criterion 20 and D3: a circle stretched becomes an ellipse along the
    /// document axes with rotation 0, whatever its rotation was.
    #[test]
    fn a_stretched_circle_becomes_an_ellipse_with_rotation_zero() {
        let document = Document::new(1);
        let id = document.create_ellipse(EllipseFrame {
            center: Point::new(10.0, 0.0),
            rx: Length::from_mm(5.0),
            ry: Length::from_mm(5.0),
        });
        let object = turned(&document, id, 30.0);
        let result = apply(&object, &scale(2.0, 3.0), ScaleModes::default(), None);
        let ObjectSnapshot::Primitive(PrimitiveSnapshot {
            shape: Shape::Ellipse { frame },
            rotation,
            ..
        }) = result
        else {
            panic!("an ellipse");
        };
        assert!((frame.rx.as_mm() - 10.0).abs() < 1e-9 && (frame.ry.as_mm() - 15.0).abs() < 1e-9);
        assert_eq!(rotation.as_radians(), 0.0);
        // The same factor twice keeps the rotation: a circle stays a circle.
        let kept = apply(&object, &scale(2.0, 2.0), ScaleModes::default(), None);
        assert_eq!(kept.rotation(), object.rotation());
    }

    /// Criterion 20: a polygon or star scales its outer radius by the one factor
    /// and moves its centre; count, ratio and angle are unchanged.
    #[test]
    fn a_star_scales_its_outer_radius() {
        let document = Document::new(1);
        let id = document.create_star(
            StarFrame {
                center: Point::new(10.0, 10.0),
                radius: Length::from_mm(4.0),
                angle: Angle::from_radians(0.5),
            },
            PointCount::new(5).expect("count"),
            InnerRatio::new(0.4).expect("ratio"),
        );
        let object = document.object(id).expect("exists");
        let result = apply(&object, &scale(2.0, 2.0), ScaleModes::default(), None);
        let ObjectSnapshot::Primitive(PrimitiveSnapshot {
            shape:
                Shape::Star {
                    frame,
                    point_count,
                    inner_ratio,
                },
            ..
        }) = result
        else {
            panic!("a star");
        };
        assert_eq!((frame.center.x, frame.center.y), (20.0, 20.0));
        assert_eq!(frame.radius.as_mm(), 8.0);
        assert_eq!(frame.angle.as_radians(), 0.5);
        assert_eq!(point_count.get(), 5);
        assert!((inner_ratio.get() - 0.4).abs() < 1e-12);
    }

    /// Criterion 24: the stroke width and the corner radii scale by the square
    /// root of the factors only when their switch is on.
    #[test]
    fn the_switches_decide_stroke_width_and_corner_radius() {
        let document = Document::new(1);
        let id = document.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        });
        document
            .set_corner_radius(&[id], Length::from_mm(2.0))
            .expect("radius");
        let mut object = document.object(id).expect("exists");
        object.style_mut().stroke.width = Length::from_mm(1.0);
        let off = apply(&object, &scale(4.0, 1.0), ScaleModes::default(), None);
        assert_eq!(off.style().stroke.width.as_mm(), 1.0);
        let ObjectSnapshot::Primitive(PrimitiveSnapshot {
            shape: Shape::Rect { corner_radii, .. },
            ..
        }) = &off
        else {
            panic!("a rectangle");
        };
        assert_eq!(*corner_radii, CornerRadii::uniform(Length::from_mm(2.0)));
        let on = apply(
            &object,
            &scale(4.0, 1.0),
            ScaleModes {
                stroke: StrokeScaling::Proportional,
                radius: CornerRadiusScaling::Proportional,
            },
            None,
        );
        assert!((on.style().stroke.width.as_mm() - 2.0).abs() < 1e-9);
        let ObjectSnapshot::Primitive(PrimitiveSnapshot {
            shape: Shape::Rect { corner_radii, .. },
            ..
        }) = &on
        else {
            panic!("a rectangle");
        };
        assert!((corner_radii.tl.as_mm() - 4.0).abs() < 1e-9);
    }

    /// Criterion 20: a path's anchors map and its handle vectors scale in the
    /// document axes, whatever the path's own rotation; `rotation` is unchanged.
    #[test]
    fn a_path_scales_in_the_document_axes_whatever_its_rotation() {
        let document = Document::new(1);
        let id = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(1.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(2.0, 1.0)),
            ],
            false,
        );
        let object = turned(&document, id, 40.0);
        let ObjectSnapshot::Path(before) = &object else {
            panic!("a path");
        };
        let ObjectSnapshot::Path(after) =
            apply(&object, &scale(2.0, 3.0), ScaleModes::default(), None)
        else {
            panic!("a path");
        };
        for (a, b) in before.anchors.iter().zip(&after.anchors) {
            assert!((b.point.x - a.point.x * 2.0).abs() < 1e-9);
            assert!((b.point.y - a.point.y * 3.0).abs() < 1e-9);
        }
        assert_eq!(after.rotation, before.rotation);
    }

    /// Criterion 26: a rotate turns every kind about the pivot and advances its
    /// `rotation`; turning back restores the geometry.
    #[test]
    fn rotating_and_rotating_back_restores_the_object() {
        let document = Document::new(1);
        let object = rect(&document, 20.0, 5.0, 10.0, 10.0);
        let pivot = Point::new(7.0, 3.0);
        let there = apply(
            &object,
            &GroupMap::Rotate {
                pivot,
                delta: Angle::from_radians(0.7),
            },
            ScaleModes::default(),
            None,
        );
        assert!((there.rotation().as_radians() - 0.7).abs() < 1e-12);
        let back = apply(
            &there,
            &GroupMap::Rotate {
                pivot,
                delta: Angle::from_radians(-0.7),
            },
            ScaleModes::default(),
            None,
        );
        let (a, b) = (bounds_of(&object), bounds_of(&back));
        assert!((a.origin.x - b.origin.x).abs() < 1e-9 && (a.origin.y - b.origin.y).abs() < 1e-9);
        assert!(back.rotation().as_radians().abs() < 1e-12);
    }

    /// Criterion 41, worked example: the top handle of two paths with the bottom
    /// side fixed, 45 degrees: every point at y = 0 moves 10 mm in x, every point
    /// at y = 10 stays.
    #[test]
    fn a_shear_moves_points_in_proportion_to_their_distance_from_the_fixed_line() {
        let document = Document::new(1);
        let id = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 10.0)),
            ],
            false,
        );
        let object = document.object(id).expect("exists");
        // The bottom side (y = 10) is fixed; the top moves: ku = -tan(45) for a
        // point above the line moving right means x += ku * (y - 10).
        let result = apply(
            &object,
            &GroupMap::Shear {
                fixed: Point::new(5.0, 10.0),
                ku: -1.0,
                kv: 0.0,
            },
            ScaleModes::default(),
            None,
        );
        let ObjectSnapshot::Path(path) = result else {
            panic!("a path");
        };
        assert!(
            (path.anchors[0].point.x - 10.0).abs() < 1e-9,
            "top moved 10"
        );
        assert!(
            (path.anchors[1].point.x - 10.0).abs() < 1e-9,
            "bottom stays"
        );
    }

    fn square_polygon(document: &Document) -> ObjectSnapshot {
        let id = document.create_polygon(
            StarFrame {
                center: Point::new(25.0, 5.0),
                radius: Length::from_mm(5.0),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(4).expect("count"),
        );
        document.object(id).expect("exists")
    }

    #[allow(clippy::cast_possible_truncation)] // test coordinates are small whole numbers
    fn to_whole(value: f64) -> i64 {
        value.round() as i64
    }

    fn rounded_corners(path: &PathSnapshot) -> Vec<(i64, i64)> {
        let mut corners: Vec<(i64, i64)> = path
            .anchors
            .iter()
            .map(|a| (to_whole(a.point.x), to_whole(a.point.y)))
            .collect();
        corners.sort_unstable();
        corners
    }

    /// `0047` criterion 7, worked example: a polygon with the vertices (25, 0), (30, 5),
    /// (25, 10) and (20, 5) stretched by (2, 3) about the origin is a path with the corners
    /// (50, 0), (60, 15), (50, 30) and (40, 15); a uniform scale keeps the polygon.
    #[test]
    fn a_stretched_polygon_is_the_scaled_object_to_path_outline() {
        let document = Document::new(1);
        let polygon = square_polygon(&document);
        let converted = {
            let ObjectSnapshot::Primitive(primitive) = &polygon else {
                panic!("a polygon");
            };
            crate::conversion::primitive_as_path(primitive, &mut crate::AnchorIdMinter::new(0))
        };
        assert_eq!(
            rounded_corners(&converted),
            [(20, 5), (25, 0), (25, 10), (30, 5)],
            "the vertices Object to path gives"
        );
        let stretched = apply(
            &polygon,
            &scale(2.0, 3.0),
            ScaleModes::default(),
            Some(&converted),
        );
        let ObjectSnapshot::Path(path) = stretched else {
            panic!("a path");
        };
        assert_eq!(
            rounded_corners(&path),
            [(40, 15), (50, 0), (50, 30), (60, 15)]
        );
        let uniform = apply(
            &polygon,
            &scale(2.0, 2.0),
            ScaleModes::default(),
            Some(&converted),
        );
        assert!(matches!(uniform, ObjectSnapshot::Primitive(_)));
    }

    /// `0047` criterion 2: a shear turns a primitive into its sheared outline, exactly the
    /// image of the path; without a converted path (a rotate never has one) it stays.
    #[test]
    fn a_shear_converts_a_primitive_with_a_converted_path() {
        let document = Document::new(1);
        let object = rect(&document, 0.0, 0.0, 10.0, 10.0);
        let ObjectSnapshot::Primitive(primitive) = &object else {
            panic!("a rectangle");
        };
        let converted =
            crate::conversion::primitive_as_path(primitive, &mut crate::AnchorIdMinter::new(0));
        let shear = GroupMap::Shear {
            fixed: Point::new(5.0, 10.0),
            ku: -1.0,
            kv: 0.0,
        };
        let ObjectSnapshot::Path(path) =
            apply(&object, &shear, ScaleModes::default(), Some(&converted))
        else {
            panic!("a path");
        };
        assert_eq!(
            rounded_corners(&path),
            [(0, 10), (10, 0), (10, 10), (20, 0)]
        );
        assert_eq!(path.id, primitive.id, "same object");
        assert!(matches!(
            apply(&object, &shear, ScaleModes::default(), None),
            ObjectSnapshot::Primitive(_)
        ));
    }

    /// `specs/0056-mirror-and-negative-scale/` criterion 28 d: a scale by (-s, -s) about
    /// a point is a half turn about it followed by a scale by (s, s), outline for outline.
    #[test]
    fn a_negative_uniform_scale_is_a_half_turn_and_a_scale() {
        use curvyo_document_core::{CornerRadii, StarFrame, outline_of_rotated};
        let document = Document::new(1);
        let r = rect(&document, 3.0, 4.0, 30.0, 10.0);
        document
            .set_corner_radii(&[(
                r.id(),
                CornerRadii {
                    tl: Length::from_mm(1.0),
                    tr: Length::from_mm(2.0),
                    br: Length::from_mm(3.0),
                    bl: Length::from_mm(4.0),
                },
            )])
            .unwrap();
        let r = turned(&document, r.id(), 25.0);
        let star = document.create_star(
            StarFrame {
                center: Point::new(40.0, 20.0),
                radius: Length::from_mm(6.0),
                angle: Angle::from_radians(0.2),
            },
            PointCount::new(5).unwrap(),
            InnerRatio::new(0.5).unwrap(),
        );
        let star = turned(&document, star, 10.0);
        let pivot = Point::new(7.0, -2.0);
        let points = |object: &ObjectSnapshot| -> Vec<Point> {
            match object {
                ObjectSnapshot::Primitive(p) => outline_of_rotated(&p.shape, p.rotation)
                    .iter()
                    .map(|a| a.point)
                    .collect(),
                ObjectSnapshot::Path(p) => p.all_anchors().map(|a| a.point).collect(),
                ObjectSnapshot::Clone(_) => Vec::new(),
            }
        };
        for object in [r, star] {
            let s = 1.7;
            let negative = apply(
                &object,
                &GroupMap::Scale {
                    pivot,
                    sx: -s,
                    sy: -s,
                },
                ScaleModes::default(),
                None,
            );
            let turned_then_scaled = apply(
                &apply(
                    &object,
                    &GroupMap::Rotate {
                        pivot,
                        delta: Angle::from_radians(std::f64::consts::PI),
                    },
                    ScaleModes::default(),
                    None,
                ),
                &GroupMap::Scale {
                    pivot,
                    sx: s,
                    sy: s,
                },
                ScaleModes::default(),
                None,
            );
            let (a, b) = (points(&negative), points(&turned_then_scaled));
            assert_eq!(a.len(), b.len());
            for p in &a {
                assert!(
                    b.iter()
                        .any(|q| (p.x - q.x).abs() < 1e-6 && (p.y - q.y).abs() < 1e-6),
                    "{p:?} not in {b:?}"
                );
            }
        }
    }
}
