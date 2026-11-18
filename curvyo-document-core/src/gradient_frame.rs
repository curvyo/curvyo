//! The box a gradient fill is placed in (ADR 0066 §4): the tight box of a leaf's outlines in its
//! own frame, the frame being the `rotation` register (a polygon's or star's shown angle), a
//! text's line box or, for a linked clone, its root's box carried by the clone's matrix.
//!
//! The box is the one the Select tool's oriented box shows, so the default gradient ends sit on
//! the middle of the left and right edges (`specs/0124-gradient-fill` criterion 6).

use crate::affine::Affine;
use crate::gradient_eval::GradientBox;
use crate::path_model::{NewAnchor, PathSnapshot};
use crate::primitive_model::{ObjectSnapshot, Shape, shape_center, shape_frame_bounds};
use crate::text_model::TextSnapshot;
use crate::units::{Angle, Point, Vec2};

/// The extent `[lo, hi]` of one coordinate of a cubic Bézier with control values `p0..p3`.
#[allow(clippy::many_single_char_names)] // The coefficients of the derivative's quadratic.
fn cubic_extent(p0: f64, p1: f64, p2: f64, p3: f64) -> (f64, f64) {
    let (mut lo, mut hi) = (p0.min(p3), p0.max(p3));
    // The derivative over 3: a t² + b t + c.
    let a = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
    let b = 2.0 * (p0 - 2.0 * p1 + p2);
    let c = p1 - p0;
    let mut visit = |t: f64| {
        if t > 0.0 && t < 1.0 {
            let u = 1.0 - t;
            let v = u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3;
            lo = lo.min(v);
            hi = hi.max(v);
        }
    };
    if a.abs() < 1e-12 {
        if b.abs() > 1e-12 {
            visit(-c / b);
        }
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            let root = disc.sqrt();
            visit((-b + root) / (2.0 * a));
            visit((-b - root) / (2.0 * a));
        }
    }
    (lo, hi)
}

/// A growing box.
#[derive(Debug, Clone, Copy)]
struct Extent {
    min: Point,
    max: Point,
}

impl Extent {
    const EMPTY: Self = Self {
        min: Point::new(f64::INFINITY, f64::INFINITY),
        max: Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };

    fn add_point(&mut self, p: Point) {
        self.min = Point::new(self.min.x.min(p.x), self.min.y.min(p.y));
        self.max = Point::new(self.max.x.max(p.x), self.max.y.max(p.y));
    }

    /// Adds the segment from `a` to `b` with control points `a + out` and `b + in_`.
    fn add_segment(&mut self, a: Point, out: Vec2, in_: Vec2, b: Point) {
        let (c1, c2) = (a.translated(out), b.translated(in_));
        let (x0, x1) = cubic_extent(a.x, c1.x, c2.x, b.x);
        let (y0, y1) = cubic_extent(a.y, c1.y, c2.y, b.y);
        self.add_point(Point::new(x0, y0));
        self.add_point(Point::new(x1, y1));
    }

    fn is_empty(&self) -> bool {
        !self.min.x.is_finite()
    }
}

/// The tight box of `outlines` (point, handle in, handle out; closed) seen turned back by
/// `rotation` about the document origin, as the frame `to_document` turns it out again.
fn outlines_box<'a>(
    outlines: impl Iterator<Item = (&'a [NewAnchor], bool)>,
    rotation: Angle,
    spiro: Option<Affine>,
) -> Option<GradientBox> {
    let back = Angle::from_radians(-rotation.as_radians());
    let origin = Point::new(0.0, 0.0);
    let mut extent = Extent::EMPTY;
    for (anchors, closed) in outlines {
        let local: Vec<(Point, Vec2, Vec2)> = anchors
            .iter()
            .map(|anchor| {
                let (mut p, mut h_in, mut h_out) =
                    (anchor.point, anchor.handle_in, anchor.handle_out);
                if let Some(m) = spiro {
                    (p, h_in, h_out) = (
                        m.apply_point(p),
                        m.apply_vector(h_in),
                        m.apply_vector(h_out),
                    );
                }
                (
                    p.rotated_around(origin, back),
                    h_in.rotated(back),
                    h_out.rotated(back),
                )
            })
            .collect();
        let n = local.len();
        if let Some(&(only, _, _)) = local.first() {
            extent.add_point(only);
        }
        let pairs = if closed && n > 1 {
            n
        } else {
            n.saturating_sub(1)
        };
        for i in 0..pairs {
            let (a, _, out) = local[i];
            let (b, in_, _) = local[(i + 1) % n];
            extent.add_segment(a, out, in_, b);
        }
    }
    (!extent.is_empty()).then(|| GradientBox {
        min: extent.min,
        max: extent.max,
        to_document: Affine::rotation_about(origin, rotation),
    })
}

/// The box of a primitive of `shape` with the `rotation` register: its frame box turned by its
/// shown angle (a polygon's or star's frame angle plus the register).
#[must_use]
pub fn primitive_gradient_box(shape: &Shape, rotation: Angle) -> GradientBox {
    let (min, max) = shape_frame_bounds(shape);
    let shown = match shape {
        Shape::Polygon { frame, .. } | Shape::Star { frame, .. } => {
            Angle::from_radians(frame.angle.as_radians() + rotation.as_radians())
        }
        Shape::Rect { .. } | Shape::Ellipse { .. } => rotation,
    };
    GradientBox {
        min,
        max,
        to_document: Affine::rotation_about(shape_center(shape), shown),
    }
}

/// The box of a new, unrotated path with `outlines` (an operation's result,
/// `specs/0124-gradient-fill` criterion 19); `None` without anchors.
#[must_use]
pub fn outlines_gradient_box(outlines: &[(Vec<NewAnchor>, bool)]) -> Option<GradientBox> {
    outlines_box(
        outlines
            .iter()
            .map(|(anchors, closed)| (anchors.as_slice(), *closed)),
        Angle::from_radians(0.0),
        None,
    )
}

/// The box of a path with `outlines` and the `rotation` register (a primitive converted in place,
/// which keeps its shown angle).
#[must_use]
pub(crate) fn rotated_outlines_box(
    outlines: &[(Vec<NewAnchor>, bool)],
    rotation: Angle,
) -> Option<GradientBox> {
    outlines_box(
        outlines
            .iter()
            .map(|(anchors, closed)| (anchors.as_slice(), *closed)),
        rotation,
        None,
    )
}

impl PathSnapshot {
    /// The box its gradient is placed in: its outlines' tight box in the frame of its `rotation`.
    #[must_use]
    pub fn gradient_box(&self) -> Option<GradientBox> {
        outlines_box(
            self.subpaths()
                .map(|subpath| (subpath.anchors, subpath.closed)),
            self.rotation,
            self.spiro_transform.as_deref().copied(),
        )
    }
}

impl TextSnapshot {
    /// The box its gradient is placed in: its line box, in the frame of its sides (a turned or
    /// skewed text's box turns and skews with it); `None` before it is laid out.
    #[must_use]
    pub fn gradient_box(&self) -> Option<GradientBox> {
        let [c0, c1, _, c3] = self.outline.as_ref()?.line_box;
        let (across, down) = (c0.vector_to(c1), c0.vector_to(c3));
        let (w, h) = (across.length(), down.length());
        if !(w > 0.0 && h > 0.0 && w.is_finite() && h.is_finite()) {
            return None;
        }
        Some(GradientBox {
            min: Point::new(0.0, 0.0),
            max: Point::new(w, h),
            to_document: Affine::new(
                across.x / w,
                across.y / w,
                down.x / h,
                down.y / h,
                c0.x,
                c0.y,
            ),
        })
    }
}

impl crate::clone_model::CloneSnapshot {
    /// The box its gradient is placed in: its root's box carried by its composed matrix; `None`
    /// for a clone of a picture.
    #[must_use]
    pub fn gradient_box(&self) -> Option<GradientBox> {
        if self.chain.picture.is_some() {
            return None;
        }
        let root = self.chain.root_outline.gradient_box()?;
        Some(GradientBox {
            to_document: self.chain.composed.compose(root.to_document),
            ..root
        })
    }
}

impl ObjectSnapshot {
    /// The box its gradient is placed in (ADR 0066 §4); `None` for a kind that has no fill of its
    /// own (an image, a dimension, a sketch), a text not laid out yet, or a clone of a picture.
    #[must_use]
    pub fn gradient_box(&self) -> Option<GradientBox> {
        match self {
            Self::Primitive(primitive) => {
                Some(primitive_gradient_box(&primitive.shape, primitive.rotation))
            }
            Self::Path(path) => path.gradient_box(),
            Self::Clone(clone) => clone.gradient_box(),
            Self::Text(text) => text.gradient_box(),
            Self::Sketch(_) | Self::Image(_) | Self::Dimension(_) => None,
        }
    }
}

/// `result`'s gradient after a command took `start` to `result` by the document map `map`
/// (ADR 0066 §5): the exact image of `start`'s gradient, re-expressed in `result`'s box. A move
/// or a turn the frame absorbs gives the fractions back unchanged. Only a path or a primitive is
/// carried: a clone's matrix, a text's placement and line box already take the whole map. Nothing
/// changes for a start without a gradient or either side without a box.
pub fn carry_gradient(start: &ObjectSnapshot, result: &mut ObjectSnapshot, map: Affine) {
    let carried = matches!(
        start,
        ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_)
    ) && matches!(
        result,
        ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_)
    );
    let Some(gradient) = start.style().fill.gradient.as_ref().filter(|_| carried) else {
        return;
    };
    let (Some(old), Some(new)) = (start.gradient_box(), result.gradient_box()) else {
        return;
    };
    result.style_mut().fill.gradient = Some(gradient.mapped(&old, map, &new));
}

/// The style of a piece made from `source` (an operation's result, ADR 0066 §6): `style` with
/// its gradient re-based from `source`'s box into the piece's, so the shade stays where it was.
#[must_use]
pub(crate) fn rebased_for_piece(
    style: &crate::style_model::Style,
    source: Option<&GradientBox>,
    outlines: &[(Vec<NewAnchor>, bool)],
) -> crate::style_model::Style {
    let mut style = style.clone();
    if let (Some(gradient), Some(old), Some(new)) = (
        style.fill.gradient.as_ref(),
        source,
        outlines_gradient_box(outlines),
    ) {
        style.fill.gradient = Some(gradient.rebased(old, &new));
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_model::AnchorId;

    fn corner(k: u64, x: f64, y: f64) -> NewAnchor {
        NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y))
    }

    #[test]
    fn a_curve_bulging_past_its_anchors_widens_the_box() {
        let mut a = corner(0, 0.0, 0.0);
        a.handle_out = Vec2::new(0.0, -10.0);
        let mut b = corner(1, 10.0, 0.0);
        b.handle_in = Vec2::new(0.0, -10.0);
        let frame = outlines_gradient_box(&[(vec![a, b], false)]).unwrap();
        assert!((frame.min.y + 7.5).abs() < 1e-12, "{frame:?}");
        assert_eq!(frame.max, Point::new(10.0, 0.0));
    }

    #[test]
    fn a_rotated_path_has_its_box_in_its_own_frame() {
        let square = vec![
            corner(0, 0.0, 0.0),
            corner(1, 20.0, 0.0),
            corner(2, 20.0, 10.0),
            corner(3, 0.0, 10.0),
        ];
        let turn = Angle::from_radians(0.6);
        let m = Affine::rotation_about(Point::new(3.0, 4.0), turn);
        let path = PathSnapshot {
            id: crate::NodeId::from_parts(1, 1),
            closed: true,
            style: crate::Style::default(),
            anchors: square
                .into_iter()
                .map(|mut a| {
                    a.point = m.apply_point(a.point);
                    a
                })
                .collect(),
            extra_subpaths: Vec::new(),
            curve_kind: crate::CurveKind::Bezier,
            spiro_transform: None,
            width_points: Vec::new(),
            rotation: turn,
        };
        let frame = path.gradient_box().unwrap();
        let (w, h) = frame.size();
        assert!(
            (w - 20.0).abs() < 1e-9 && (h - 10.0).abs() < 1e-9,
            "{w} {h}"
        );
        let (corner, expected) = (
            frame.to_document.apply_point(frame.min),
            m.apply_point(Point::new(0.0, 0.0)),
        );
        assert!((corner.x - expected.x).abs() < 1e-9 && (corner.y - expected.y).abs() < 1e-9);
    }
}
