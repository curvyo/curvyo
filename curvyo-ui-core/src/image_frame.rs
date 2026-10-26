//! The frame of an image object as the Select tool reads it (`specs/0058-bitmap-import`
//! criteria 31 and 32): the mapped unit square is hit anywhere inside, transparent pixels
//! included, and boxed like a rectangle turned by the frame's angle.

use curvyo_document_core::{Angle, ImageSnapshot, Point, Vec2};
use curvyo_geometry_core::OutlineTriple;

use crate::oriented_box::OrientedBox;

/// Whether `point` lies inside the frame or within `margin` of it.
pub(crate) fn frame_contains(image: &ImageSnapshot, point: Point, margin: f64) -> bool {
    crate::text_box::quad_contains(&image.corners(), point, margin)
}

/// The axis-aligned bounds of the frame.
pub(crate) fn frame_bounds(image: &ImageSnapshot) -> (Point, Point) {
    image.corners().iter().fold(
        (
            Point::new(f64::INFINITY, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(min, max), p| {
            (
                Point::new(min.x.min(p.x), min.y.min(p.y)),
                Point::new(max.x.max(p.x), max.y.max(p.y)),
            )
        },
    )
}

/// The frame as one closed outline of four straight segments, for marquee and lasso.
pub(crate) fn frame_outline(image: &ImageSnapshot) -> Vec<(Vec<OutlineTriple>, bool)> {
    vec![(
        image
            .corners()
            .iter()
            .map(|corner| (*corner, Vec2::ZERO, Vec2::ZERO))
            .collect(),
        true,
    )]
}

/// The frame as an oriented box turned by the angle of the picture's top edge about its first
/// corner, so the box sits on the picture as it does on a turned rectangle.
pub(crate) fn frame_oriented(image: &ImageSnapshot) -> OrientedBox {
    let corners = image.corners();
    let angle = Angle::from_radians(image.placement.b.atan2(image.placement.a));
    let pivot = corners[0];
    let back = Angle::from_radians(-angle.as_radians());
    let (min, max) = corners.iter().map(|p| p.rotated_around(pivot, back)).fold(
        (
            Point::new(f64::INFINITY, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(min, max), p| {
            (
                Point::new(min.x.min(p.x), min.y.min(p.y)),
                Point::new(max.x.max(p.x), max.y.max(p.y)),
            )
        },
    );
    OrientedBox {
        min,
        max,
        angle,
        pivot,
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Affine, Document, NewImage, ObjectSnapshot};

    use super::*;

    fn image(placement: Affine) -> ImageSnapshot {
        let document = Document::new(1);
        let id = document
            .add_images(&[NewImage {
                bytes: std::sync::Arc::from(&b"x"[..]),
                placement,
                name: String::new(),
            }])
            .unwrap()[0];
        match document.object(id) {
            Some(ObjectSnapshot::Image(image)) => image,
            _ => unreachable!(),
        }
    }

    #[test]
    fn the_whole_frame_is_hit_and_nothing_outside() {
        let flipped = image(Affine::new(-40.0, 0.0, 0.0, 20.0, 50.0, 0.0));
        assert!(frame_contains(&flipped, Point::new(30.0, 10.0), 0.0));
        assert!(!frame_contains(&flipped, Point::new(5.0, 10.0), 0.0));
        assert_eq!(
            frame_bounds(&flipped),
            (Point::new(10.0, 0.0), Point::new(50.0, 20.0))
        );
    }

    #[test]
    fn a_turned_frame_has_its_own_size_in_the_oriented_box() {
        let turned = image(Affine::new(0.0, 40.0, -20.0, 0.0, 20.0, 0.0));
        let oriented = frame_oriented(&turned);
        assert!((oriented.width() - 40.0).abs() < 1e-9 && (oriented.height() - 20.0).abs() < 1e-9);
    }
}
