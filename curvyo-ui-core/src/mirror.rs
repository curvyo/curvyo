//! The reflection of one object about the lines through a point along a frame's axes
//! (`specs/0056-mirror-and-negative-scale/` criteria 18 to 22).
//!
//! A scale with a negative factor is the scale by the magnitudes followed by this
//! reflection about the same point (`adrs.md`: "magnitudes first, then reflect"). Every
//! kind stays the kind it was: a path maps its anchors and handle vectors and keeps its
//! anchor order (the direction is not reversed, criterion 19); a rectangle, an ellipse, a
//! polygon and a star keep positive lengths and take the centre, the `rotation` and (a
//! rectangle) the radii or (a polygon or star) the first-vertex angle of their mirror
//! image.
//!
//! For a reflection that negates the component along the frame axis at angle φ (or the
//! one across it), an orientation θ becomes 2φ − θ, and the object's own frame is
//! reflected along its local x axis (or its local y axis): a rectangle's local left and
//! right radii (or top and bottom radii) swap, so each stays at its physical corner.

use curvyo_document_core::{
    Angle, CornerRadii, EllipseAxes, Linear, ObjectSnapshot, PathSnapshot, Point,
    PrimitiveSnapshot, RectBounds, Shape, StarFrame, arc_after_linear_map,
};

/// A reflection about `pivot` along the axes turned by `axes`: `x` negates the component
/// along the frame's x axis (the mirror line is the frame's y axis through `pivot`), `y`
/// the component along its y axis. Both is a half turn about `pivot`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Mirror {
    pub(crate) pivot: Point,
    pub(crate) axes: Angle,
    pub(crate) x: bool,
    pub(crate) y: bool,
}

impl Mirror {
    /// The reflection that turns a scale by the magnitudes of `(sx, sy)` into the scale by
    /// `(sx, sy)`: one per negative factor.
    pub(crate) fn of_factors(pivot: Point, axes: Angle, sx: f64, sy: f64) -> Self {
        Self {
            pivot,
            axes,
            x: sx < 0.0,
            y: sy < 0.0,
        }
    }

    const fn is_identity(self) -> bool {
        !self.x && !self.y
    }

    const fn factors(self) -> (f64, f64) {
        (
            if self.x { -1.0 } else { 1.0 },
            if self.y { -1.0 } else { 1.0 },
        )
    }

    /// The reflection's linear part, `R(φ) · diag(fx, fy) · R(−φ)`.
    fn linear(self) -> Linear {
        let (fx, fy) = self.factors();
        let (sin, cos) = self.axes.as_radians().sin_cos();
        [
            [fx * cos * cos + fy * sin * sin, (fx - fy) * cos * sin],
            [(fx - fy) * cos * sin, fx * sin * sin + fy * cos * cos],
        ]
    }

    /// `point` reflected.
    fn point(self, point: Point) -> Point {
        let (fx, fy) = self.factors();
        let pivot = self.pivot;
        if self.axes.as_radians() == 0.0 {
            // The document axes: no rotation, so no rounding beyond the subtraction.
            return Point::new(
                pivot.x + (point.x - pivot.x) * fx,
                pivot.y + (point.y - pivot.y) * fy,
            );
        }
        let local = point.rotated_around(pivot, Angle::from_radians(-self.axes.as_radians()));
        Point::new(
            pivot.x + (local.x - pivot.x) * fx,
            pivot.y + (local.y - pivot.y) * fy,
        )
        .rotated_around(pivot, self.axes)
    }

    /// The orientation `theta` of a frame after the reflection: `2φ − θ` for one
    /// reflection, `θ` for a half turn. Written as `φ + (φ − θ)` so that a frame along
    /// the reflection's own axes (`θ = φ`) keeps its exact value.
    fn orientation(self, theta: Angle) -> Angle {
        if self.x == self.y {
            return theta;
        }
        let phi = self.axes.as_radians();
        Angle::from_radians(phi + (phi - theta.as_radians())).normalized()
    }
}

/// `object` reflected by `mirror`. The identity returns a copy.
pub(crate) fn mirrored(object: &ObjectSnapshot, mirror: Mirror) -> ObjectSnapshot {
    if mirror.is_identity() {
        return object.clone();
    }
    match object {
        ObjectSnapshot::Sketch(_) => object.clone(),
        ObjectSnapshot::Path(path) => ObjectSnapshot::Path(mirrored_path(path, mirror)),
        ObjectSnapshot::Primitive(primitive) => {
            ObjectSnapshot::Primitive(mirrored_primitive(primitive, mirror))
        }
        ObjectSnapshot::Clone(clone) => {
            let (fx, fy) = mirror.factors();
            let g = crate::clone_gesture::affine_of(clone.id(), &|path: &PathSnapshot| {
                path.scaled_along(mirror.pivot, fx, fy, mirror.axes)
            });
            ObjectSnapshot::Clone(clone.mapped_by(g))
        }
        ObjectSnapshot::Image(image) => {
            let (fx, fy) = mirror.factors();
            let g = crate::clone_gesture::affine_of(image.id, &|path: &PathSnapshot| {
                path.scaled_along(mirror.pivot, fx, fy, mirror.axes)
            });
            ObjectSnapshot::Image(image.mapped_by(g))
        }
        ObjectSnapshot::Text(text) => {
            let (fx, fy) = mirror.factors();
            let g = crate::clone_gesture::affine_of(text.id, &|path: &PathSnapshot| {
                path.scaled_along(mirror.pivot, fx, fy, mirror.axes)
            });
            ObjectSnapshot::Text(Box::new(text.mapped_by(g)))
        }
    }
}

/// A path reflected (criterion 19): every anchor mapped, every handle vector reflected,
/// ids, kinds, order and outlines kept; the `rotation` register that orients its box
/// becomes the mirror image.
fn mirrored_path(path: &PathSnapshot, mirror: Mirror) -> PathSnapshot {
    let (fx, fy) = mirror.factors();
    let mut mirrored = path.scaled_along(mirror.pivot, fx, fy, mirror.axes);
    mirrored.rotation = mirror.orientation(path.rotation);
    mirrored
}

/// A primitive reflected (criteria 20 to 22): the same kind and lengths, the centre
/// mapped, the `rotation` mirrored; a rectangle's radii stay at their physical corners, a
/// polygon's or star's first vertex points where the mirror puts it (its `rotation`
/// register is kept and the frame angle carries the change, so the commit of a single
/// object writes the frame only).
fn mirrored_primitive(primitive: &PrimitiveSnapshot, mirror: Mirror) -> PrimitiveSnapshot {
    let rotation = primitive.rotation;
    let (shape, rotation) = match primitive.shape {
        Shape::Rect {
            bounds,
            corner_radii,
        } => {
            let (width, height) = (bounds.width.as_mm(), bounds.height.as_mm());
            let centre = mirror.point(Point::new(
                bounds.origin.x + width / 2.0,
                bounds.origin.y + height / 2.0,
            ));
            let shape = Shape::Rect {
                bounds: RectBounds {
                    origin: Point::new(centre.x - width / 2.0, centre.y - height / 2.0),
                    ..bounds
                },
                corner_radii: swapped_radii(corner_radii, mirror),
            };
            (shape, mirror.orientation(rotation))
        }
        Shape::Ellipse {
            mut frame,
            arc,
            curve,
        } => {
            frame.center = mirror.point(frame.center);
            let mirrored_rotation = mirror.orientation(rotation);
            // The arc is the mirror image: start and end swap (criterion 54).
            let arc = arc.map(|arc| {
                let axes = |rotation: Angle| EllipseAxes {
                    rx: frame.rx.as_mm(),
                    ry: frame.ry.as_mm(),
                    rotation: rotation.as_radians(),
                };
                arc_after_linear_map(
                    arc,
                    axes(rotation),
                    axes(mirrored_rotation),
                    mirror.linear(),
                )
            });
            (Shape::Ellipse { frame, arc, curve }, mirrored_rotation)
        }
        // The ring is symmetric, so the curve is unchanged (criterion 54).
        Shape::Polygon {
            frame,
            point_count,
            curve,
        } => (
            Shape::Polygon {
                frame: mirrored_star_frame(frame, rotation, mirror),
                point_count,
                curve,
            },
            rotation,
        ),
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
            curve,
        } => (
            Shape::Star {
                frame: mirrored_star_frame(frame, rotation, mirror),
                point_count,
                inner_ratio,
                curve,
            },
            rotation,
        ),
    };
    PrimitiveSnapshot {
        shape,
        rotation,
        ..primitive.clone()
    }
}

/// A rectangle's radii in its reflected local frame: a reflection along the local x axis
/// swaps left and right, along the local y axis top and bottom, a half turn both.
const fn swapped_radii(radii: CornerRadii, mirror: Mirror) -> CornerRadii {
    let radii = if mirror.x {
        CornerRadii {
            tl: radii.tr,
            tr: radii.tl,
            br: radii.bl,
            bl: radii.br,
        }
    } else {
        radii
    };
    if mirror.y {
        CornerRadii {
            tl: radii.bl,
            tr: radii.br,
            br: radii.tr,
            bl: radii.tl,
        }
    } else {
        radii
    }
}

/// A polygon's or star's frame reflected, with the `rotation` register kept: the shown
/// orientation `a + θ` of the first vertex becomes its mirror image (`2φ + π − o` across
/// the frame's y axis, `2φ − o` across its x axis, `o + π` for both), so the outline is
/// the exact mirror image (the outline is symmetric about its first vertex's direction).
fn mirrored_star_frame(frame: StarFrame, rotation: Angle, mirror: Mirror) -> StarFrame {
    let phi = mirror.axes.as_radians();
    let shown = frame.angle.as_radians() + rotation.as_radians();
    let mirrored_shown = match (mirror.x, mirror.y) {
        (true, true) => shown + std::f64::consts::PI,
        (true, false) => phi + (phi - shown) + std::f64::consts::PI,
        (false, true) => phi + (phi - shown),
        (false, false) => shown,
    };
    StarFrame {
        center: mirror.point(frame.center),
        radius: frame.radius,
        angle: Angle::from_radians(mirrored_shown - rotation.as_radians()).normalized(),
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{
        AnchorId, AnchorKind, Document, InnerRatio, Length, NewAnchor, PointCount, Vec2,
        outline_of_rotated,
    };

    use super::*;

    const EPS: f64 = 1e-9;

    fn h_flip(pivot_x: f64) -> Mirror {
        Mirror {
            pivot: Point::new(pivot_x, 0.0),
            axes: Angle::from_radians(0.0),
            x: true,
            y: false,
        }
    }

    fn v_flip(pivot_y: f64) -> Mirror {
        Mirror {
            pivot: Point::new(0.0, pivot_y),
            axes: Angle::from_radians(0.0),
            x: false,
            y: true,
        }
    }

    fn rect_with_tl_radius(document: &Document) -> ObjectSnapshot {
        let id = document.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(30.0),
            height: Length::from_mm(10.0),
        });
        document
            .set_corner_radii(&[(
                id,
                CornerRadii {
                    tl: Length::from_mm(3.0),
                    tr: Length::from_mm(0.0),
                    br: Length::from_mm(0.0),
                    bl: Length::from_mm(0.0),
                },
            )])
            .expect("radii");
        document.object(id).expect("exists")
    }

    fn radii_of(object: &ObjectSnapshot) -> [f64; 4] {
        let ObjectSnapshot::Primitive(PrimitiveSnapshot {
            shape: Shape::Rect { corner_radii, .. },
            ..
        }) = object
        else {
            panic!("not a rectangle");
        };
        [
            corner_radii.tl,
            corner_radii.tr,
            corner_radii.br,
            corner_radii.bl,
        ]
        .map(Length::as_mm)
    }

    /// Criterion 20 example: the 3 mm radius at the top-left goes to the top-right on a
    /// horizontal flip, to the bottom-left on a vertical one, to the bottom-right on both;
    /// the bounds stay.
    #[test]
    fn a_rectangle_keeps_its_radius_at_the_physical_corner() {
        let document = Document::new(1);
        let rect = rect_with_tl_radius(&document);
        let h = mirrored(&rect, h_flip(15.0));
        assert_eq!(radii_of(&h), [0.0, 3.0, 0.0, 0.0]);
        let v = mirrored(&rect, v_flip(5.0));
        assert_eq!(radii_of(&v), [0.0, 0.0, 0.0, 3.0]);
        let both = mirrored(
            &rect,
            Mirror {
                pivot: Point::new(15.0, 5.0),
                axes: Angle::from_radians(0.0),
                x: true,
                y: true,
            },
        );
        assert_eq!(radii_of(&both), [0.0, 0.0, 3.0, 0.0]);
        for flipped in [&h, &v, &both] {
            let ObjectSnapshot::Primitive(PrimitiveSnapshot {
                shape: Shape::Rect { bounds, .. },
                ..
            }) = flipped
            else {
                panic!("still a rectangle");
            };
            assert!(bounds.origin.x.abs() < EPS && bounds.origin.y.abs() < EPS);
        }
    }

    /// Criterion 20: a document-axes flip negates the rotation; the outline is the exact
    /// mirror image at every rotation, the quarter turn included.
    #[test]
    fn a_rotated_rectangle_is_the_exact_mirror_image() {
        for degrees in [0.0, 30.0, 90.0, -90.0, 135.0] {
            let document = Document::new(1);
            let rect = rect_with_tl_radius(&document);
            let rect = rect.rotated(
                Point::new(15.0, 5.0),
                Angle::from_radians(f64::to_radians(degrees)),
            );
            let flipped = mirrored(&rect, h_flip(40.0));
            let sum =
                Angle::from_radians(flipped.rotation().as_radians() + rect.rotation().as_radians())
                    .normalized();
            assert!(sum.as_radians().abs() < 1e-12, "{degrees}");
            assert_outline_mirrored(&rect, &flipped, h_flip(40.0));
        }
    }

    /// The outline of `after` is the outline of `before` with every point reflected, as a
    /// set of sampled points (each sample of one has a sample of the other within 1e-6).
    fn assert_outline_mirrored(before: &ObjectSnapshot, after: &ObjectSnapshot, mirror: Mirror) {
        let expected: Vec<Point> = samples(before)
            .into_iter()
            .map(|p| mirror.point(p))
            .collect();
        let actual = samples(after);
        for point in &expected {
            assert!(
                actual
                    .iter()
                    .any(|q| (q.x - point.x).abs() < 1e-6 && (q.y - point.y).abs() < 1e-6),
                "{point:?} missing"
            );
        }
    }

    fn samples(object: &ObjectSnapshot) -> Vec<Point> {
        match object {
            ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
            ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
            ObjectSnapshot::Sketch(_) => unreachable!("no sketch in this test"),
            ObjectSnapshot::Primitive(p) => outline_of_rotated(&p.shape, p.rotation)
                .iter()
                .map(|a| a.point)
                .collect(),
            ObjectSnapshot::Path(path) => path.all_anchors().map(|a| a.point).collect(),
            ObjectSnapshot::Clone(_) => Vec::new(),
        }
    }

    /// Criteria 21, 22: an ellipse keeps rx and ry, a polygon and a star keep the point
    /// count, the ratio and the radius; each is the exact mirror image.
    #[test]
    fn ellipses_polygons_and_stars_stay_their_kind() {
        let document = Document::new(1);
        let ellipse = document.create_ellipse(curvyo_document_core::EllipseFrame {
            center: Point::new(10.0, 5.0),
            rx: Length::from_mm(8.0),
            ry: Length::from_mm(3.0),
        });
        let frame = StarFrame {
            center: Point::new(25.0, 5.0),
            radius: Length::from_mm(5.0),
            angle: Angle::from_radians(0.3),
        };
        let polygon = document.create_polygon(frame, PointCount::new(5).expect("count"));
        let star = document.create_star(
            frame,
            PointCount::new(6).expect("count"),
            InnerRatio::new(0.4).expect("ratio"),
        );
        for id in [ellipse, polygon, star] {
            let object = document
                .object(id)
                .expect("exists")
                .rotated(Point::new(3.0, 4.0), Angle::from_radians(0.7));
            for mirror in [
                h_flip(26.25),
                v_flip(2.0),
                Mirror {
                    pivot: Point::new(1.0, 2.0),
                    axes: Angle::from_radians(0.7),
                    x: true,
                    y: false,
                },
            ] {
                let flipped = mirrored(&object, mirror);
                assert_eq!(
                    std::mem::discriminant(&flipped),
                    std::mem::discriminant(&object)
                );
                assert_outline_mirrored(&object, &flipped, mirror);
            }
        }
    }

    /// Criterion 22 example: a triangle with its first vertex pointing right, centre
    /// (25, 5), flipped about x = 26.25: centre (27.5, 5), radius 5, three points.
    #[test]
    fn the_triangle_example_of_criterion_22() {
        let document = Document::new(1);
        let id = document.create_polygon(
            StarFrame {
                center: Point::new(25.0, 5.0),
                radius: Length::from_mm(5.0),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(3).expect("count"),
        );
        let flipped = mirrored(&document.object(id).expect("exists"), h_flip(26.25));
        let ObjectSnapshot::Primitive(PrimitiveSnapshot {
            shape: Shape::Polygon {
                frame, point_count, ..
            },
            ..
        }) = &flipped
        else {
            panic!("still a polygon");
        };
        assert_eq!(point_count.get(), 3);
        assert!((frame.center.x - 27.5).abs() < EPS && (frame.center.y - 5.0).abs() < EPS);
        assert!((frame.radius.as_mm() - 5.0).abs() < EPS);
        let vertices = samples(&flipped);
        for expected in [(22.5, 5.0), (30.0, 9.330_127), (30.0, 0.669_873)] {
            assert!(
                vertices
                    .iter()
                    .any(|v| (v.x - expected.0).abs() < 1e-5 && (v.y - expected.1).abs() < 1e-5),
                "{expected:?} in {vertices:?}"
            );
        }
    }

    /// Criterion 19: anchors mapped, handles reflected, kinds, ids and order kept; the
    /// register becomes −θ for a document-axes flip and stays for a half turn.
    #[test]
    fn a_path_keeps_its_order_kinds_and_ids() {
        let document = Document::new(1);
        let anchor = |k: u64, x: f64, y: f64, out: Vec2, kind| NewAnchor {
            id: AnchorId::new(1, k),
            point: Point::new(x, y),
            handle_in: Vec2::new(-out.x, -out.y),
            handle_out: out,
            kind,
        };
        let id = document.create_path(
            &[
                anchor(1, 0.0, 0.0, Vec2::new(2.0, 1.0), AnchorKind::Symmetric),
                anchor(2, 30.0, 0.0, Vec2::ZERO, AnchorKind::Corner),
                anchor(3, 0.0, 10.0, Vec2::new(1.0, -3.0), AnchorKind::Symmetric),
            ],
            true,
        );
        let path = document
            .object(id)
            .expect("exists")
            .rotated(Point::new(0.0, 0.0), Angle::from_radians(0.4));
        let ObjectSnapshot::Path(flipped) = mirrored(&path, h_flip(0.0)) else {
            panic!("still a path");
        };
        let ObjectSnapshot::Path(before) = &path else {
            panic!("a path");
        };
        for (a, b) in before.anchors.iter().zip(&flipped.anchors) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.kind, b.kind);
            assert!((b.point.x + a.point.x).abs() < EPS && (b.point.y - a.point.y).abs() < EPS);
            assert!((b.handle_out.x + a.handle_out.x).abs() < EPS);
            assert!((b.handle_out.y - a.handle_out.y).abs() < EPS);
        }
        assert!((flipped.rotation.as_radians() + 0.4).abs() < 1e-12);
        let ObjectSnapshot::Path(turned) = mirrored(
            &path,
            Mirror {
                x: true,
                y: true,
                ..h_flip(0.0)
            },
        ) else {
            panic!("still a path");
        };
        assert!((turned.rotation.as_radians() - 0.4).abs() < 1e-12);
    }

    /// A mirror along an object's own axes (θ = φ) keeps the register exactly.
    #[test]
    fn a_mirror_along_the_own_axes_keeps_the_rotation_exactly() {
        let theta = Angle::from_radians(0.523_598_775_598_298_8);
        let mirror = Mirror {
            pivot: Point::new(3.0, 1.0),
            axes: theta,
            x: true,
            y: false,
        };
        assert_eq!(mirror.orientation(theta), theta);
    }
}
