//! The image of a gradient under an affine map (ADR 0066 §5, §6): the exact picture after a
//! scale, mirror, skew or a change of box, re-expressed as fractions of the new box.
//!
//! Only `+ - * /` and `sqrt` are used, so native and wasm agree bit for bit.

use crate::affine::Affine;
use crate::gradient_eval::{GradientBox, radial_axes};
use crate::gradient_model::{AxisRatio, BoxFraction, BoxPoint, GradientFill, GradientGeometry};
use crate::units::Point;

/// A fraction that moved by no more than this keeps its old value, so a map that changes
/// nothing writes nothing.
const SAME_FRACTION: f64 = 1e-12;

/// `value` as a fraction of `[lo, lo + extent]`; a box side of no length, or a value within
/// [`SAME_FRACTION`] of `old`, keeps `old`.
fn fraction(value: f64, lo: f64, extent: f64, old: BoxFraction) -> BoxFraction {
    if extent.is_nan() || extent <= 0.0 {
        return old;
    }
    // Rounded to the grid of `SAME_FRACTION`, so the float dust of a map does not reach a file.
    let raw = (value - lo) / extent;
    match BoxFraction::new((raw / SAME_FRACTION).round() * SAME_FRACTION) {
        Ok(new) if (new.get() - old.get()).abs() > SAME_FRACTION => new,
        _ => old,
    }
}

fn box_point(frame: &GradientBox, p: Point, old: BoxPoint) -> BoxPoint {
    let (w, h) = frame.size();
    BoxPoint {
        x: fraction(p.x, frame.min.x, w, old.x),
        y: fraction(p.y, frame.min.y, h, old.y),
    }
}

fn frame_point(frame: &GradientBox, p: BoxPoint) -> Point {
    frame.frame_point(p.x.get(), p.y.get())
}

/// The new end of a linear gradient from `s` to `e` under `map`, so that the lines of one
/// colour are the images of the old ones (also under skew); `None` for a line of no length or a
/// map of no area.
fn mapped_end(map: Affine, s: Point, e: Point) -> Option<Point> {
    let (dx, dy) = (e.x - s.x, e.y - s.y);
    let len2 = dx * dx + dy * dy;
    let det = map.determinant();
    if len2.is_nan() || len2 <= 0.0 || det == 0.0 || !det.is_finite() {
        return None;
    }
    // `t` changes fastest along `g`: the old direction through the inverse transpose.
    let gx = (map.d * dx - map.b * dy) / (det * len2);
    let gy = (-map.c * dx + map.a * dy) / (det * len2);
    let g2 = gx * gx + gy * gy;
    let start = map.apply_point(s);
    (g2 > 0.0 && g2.is_finite()).then(|| Point::new(start.x + gx / g2, start.y + gy / g2))
}

/// The axes of the ellipse with columns `m1`, `m2` as a radius vector near `m1` and the ratio of
/// the other axis to it; `None` when the ellipse has no area.
fn ellipse_axes(m1: Point, m2: Point) -> Option<(Point, f64)> {
    let p = m1.x * m1.x + m2.x * m2.x;
    let q = m1.x * m1.y + m2.x * m2.y;
    let r = m1.y * m1.y + m2.y * m2.y;
    let half_diff = (p - r) / 2.0;
    let disc = (half_diff * half_diff + q * q).sqrt();
    let mean = f64::midpoint(p, r);
    let (l1, l2) = (mean + disc, (mean - disc).max(0.0));
    let (s1, s2) = (l1.sqrt(), l2.sqrt());
    if s2.is_nan() || s2 <= 0.0 || !s1.is_finite() {
        return None;
    }
    // The unit eigenvector of the larger value; any direction for a circle (within rounding),
    // so take `m1`'s and the handle stays on the image of the old one.
    let (vx, vy) = if disc > 1e-12 * mean {
        let (e1, e2) = ((l1 - r, q), (q, l1 - p));
        if e1.0 * e1.0 + e1.1 * e1.1 >= e2.0 * e2.0 + e2.1 * e2.1 {
            e1
        } else {
            e2
        }
    } else {
        (m1.x, m1.y)
    };
    let norm = (vx * vx + vy * vy).sqrt();
    if norm.is_nan() || norm <= 0.0 {
        return None;
    }
    let u1 = Point::new(vx / norm, vy / norm);
    let u2 = Point::new(-u1.y, u1.x);
    let along = |u: Point| m1.x * u.x + m1.y * u.y;
    // Keep the radius handle on the axis nearest the image of the old one.
    let (axis, length, other) = if along(u1).abs() >= along(u2).abs() {
        (u1, s1, s2)
    } else {
        (u2, s2, s1)
    };
    let sign = if along(axis) < 0.0 { -1.0 } else { 1.0 };
    Some((
        Point::new(axis.x * length * sign, axis.y * length * sign),
        other / length,
    ))
}

impl GradientGeometry {
    /// This geometry, placed in `old`, carried by `map` (document to document) and re-expressed
    /// in `new`; a map of no area, or a gradient of no length, keeps the fractions as they are.
    #[must_use]
    pub fn mapped(self, old: &GradientBox, map: Affine, new: &GradientBox) -> Self {
        let Some(from_new) = new.to_document.inverse() else {
            return self;
        };
        let a = from_new.compose(map.compose(old.to_document));
        match self {
            Self::Linear { start, end } => {
                let (s, e) = (frame_point(old, start), frame_point(old, end));
                mapped_end(a, s, e).map_or(self, |e2| Self::Linear {
                    start: box_point(new, a.apply_point(s), start),
                    end: box_point(new, e2, end),
                })
            }
            Self::Radial {
                centre,
                radius,
                ratio,
            } => {
                let c = frame_point(old, centre);
                let (a1, a2) = radial_axes(c, frame_point(old, radius), ratio.get());
                let image = |v: Point| {
                    let w = a.apply_vector(crate::units::Vec2::new(v.x, v.y));
                    Point::new(w.x, w.y)
                };
                let c2 = a.apply_point(c);
                let Some((axis, new_ratio)) = ellipse_axes(image(a1), image(a2)) else {
                    return self;
                };
                let Ok(new_ratio) = AxisRatio::new(new_ratio) else {
                    return self;
                };
                let ratio = if (new_ratio.get() - ratio.get()).abs() > SAME_FRACTION {
                    new_ratio
                } else {
                    ratio
                };
                Self::Radial {
                    centre: box_point(new, c2, centre),
                    radius: box_point(new, Point::new(c2.x + axis.x, c2.y + axis.y), radius),
                    ratio,
                }
            }
        }
    }
}

impl GradientFill {
    /// The fill with its geometry carried by `map` from `old` into `new` (ADR 0066 §5); the
    /// stops are unchanged.
    #[must_use]
    pub fn mapped(&self, old: &GradientBox, map: Affine, new: &GradientBox) -> Self {
        Self {
            geometry: self.geometry.mapped(old, map, new),
            stops: self.stops.clone(),
        }
    }

    /// The fill re-expressed in another box with the picture unchanged (ADR 0066 §6): an
    /// operation's result continues the shade of its input.
    #[must_use]
    pub fn rebased(&self, old: &GradientBox, new: &GradientBox) -> Self {
        self.mapped(old, Affine::IDENTITY, new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gradient_eval::PlacedGradient;
    use crate::gradient_model::{GradientStop, GradientStops, StopOffset};
    use crate::path_model::Color;

    fn fill(geometry: GradientGeometry) -> GradientFill {
        let stop = |offset| GradientStop {
            colour: Color::BLACK,
            alpha: 255,
            offset,
        };
        GradientFill {
            geometry,
            stops: GradientStops::new(vec![stop(StopOffset::START), stop(StopOffset::END)])
                .unwrap(),
        }
    }

    fn unit_box(min: (f64, f64), max: (f64, f64)) -> GradientBox {
        GradientBox::axis_aligned(Point::new(min.0, min.1), Point::new(max.0, max.1))
    }

    /// The box of `old`'s corners under `map`, axis-aligned (exact for the maps used here).
    fn image_box(old: &GradientBox, map: Affine) -> GradientBox {
        let corners = [
            old.min,
            old.max,
            Point::new(old.min.x, old.max.y),
            Point::new(old.max.x, old.min.y),
        ]
        .map(|p| map.apply_point(p));
        let lo = corners.iter().fold(Point::new(f64::MAX, f64::MAX), |a, p| {
            Point::new(a.x.min(p.x), a.y.min(p.y))
        });
        let hi = corners.iter().fold(Point::new(f64::MIN, f64::MIN), |a, p| {
            Point::new(a.x.max(p.x), a.y.max(p.y))
        });
        GradientBox::axis_aligned(lo, hi)
    }

    /// The picture is carried exactly: `t` at `map(p)` after equals `t` at `p` before.
    fn assert_carried(geometry: GradientGeometry, map: Affine) {
        let old = unit_box((10.0, 5.0), (70.0, 45.0));
        let new = image_box(&old, map);
        let before = fill(geometry);
        let after = before.mapped(&old, map, &new);
        let (pb, pa) = (
            PlacedGradient::new(&before, &old),
            PlacedGradient::new(&after, &new),
        );
        for (x, y) in [
            (10.0, 5.0),
            (33.0, 17.0),
            (70.0, 45.0),
            (50.0, 9.0),
            (-4.0, 60.0),
        ] {
            let p = Point::new(x, y);
            let (tb, ta) = (pb.t_at(p), pa.t_at(map.apply_point(p)));
            assert!(
                (tb - ta).abs() < 1e-9,
                "{geometry:?} {map:?} at {p:?}: {tb} {ta}"
            );
        }
    }

    fn geometries() -> [GradientGeometry; 3] {
        [
            GradientGeometry::DEFAULT_LINEAR,
            GradientGeometry::Linear {
                start: BoxPoint::new(0.1, 0.2).unwrap(),
                end: BoxPoint::new(0.8, 1.3).unwrap(),
            },
            GradientGeometry::Radial {
                centre: BoxPoint::new(0.4, 0.6).unwrap(),
                radius: BoxPoint::new(0.9, 0.3).unwrap(),
                ratio: AxisRatio::new(0.6).unwrap(),
            },
        ]
    }

    #[test]
    fn scale_mirror_and_skew_carry_the_picture() {
        let maps = [
            Affine::scale(2.0, 0.5),
            Affine::scale(-1.0, 1.0),
            Affine::new(1.0, 0.0, 0.7, 1.0, 3.0, -2.0),
            Affine::new(1.0, 0.3, 0.0, 1.0, 0.0, 0.0),
            Affine::new(0.0, 1.0, -1.0, 0.0, 5.0, 5.0),
        ];
        for geometry in geometries() {
            for map in maps {
                assert_carried(geometry, map);
            }
        }
        assert_carried(GradientGeometry::DEFAULT_RADIAL, Affine::scale(3.0, 1.0));
    }

    #[test]
    fn a_mirror_swaps_the_default_ends() {
        let old = unit_box((0.0, 0.0), (40.0, 20.0));
        let map = Affine::new(-1.0, 0.0, 0.0, 1.0, 40.0, 0.0);
        let mirrored = GradientGeometry::DEFAULT_LINEAR.mapped(&old, map, &old);
        assert_eq!(
            mirrored,
            GradientGeometry::Linear {
                start: BoxPoint::new(1.0, 0.5).unwrap(),
                end: BoxPoint::new(0.0, 0.5).unwrap(),
            }
        );
    }

    #[test]
    fn a_map_that_changes_nothing_keeps_the_fractions_exactly() {
        let frame = unit_box((3.3, 1.7), (91.1, 23.9));
        for geometry in geometries() {
            assert_eq!(geometry.mapped(&frame, Affine::IDENTITY, &frame), geometry);
        }
        let moved = Affine::translation(crate::units::Vec2::new(7.0, -2.0));
        let new = image_box(&frame, moved);
        for geometry in geometries() {
            assert_eq!(geometry.mapped(&frame, moved, &new), geometry, "a move");
        }
    }

    #[test]
    fn a_map_of_no_area_keeps_the_gradient() {
        let frame = unit_box((0.0, 0.0), (10.0, 10.0));
        for geometry in geometries() {
            assert_eq!(
                geometry.mapped(&frame, Affine::scale(0.0, 1.0), &frame),
                geometry
            );
        }
    }

    #[test]
    fn rebasing_keeps_the_shade_where_it_was() {
        let old = unit_box((0.0, 0.0), (100.0, 40.0));
        let piece = unit_box((50.0, 0.0), (100.0, 40.0));
        let rebased = fill(GradientGeometry::DEFAULT_LINEAR).rebased(&old, &piece);
        assert_eq!(
            rebased.geometry,
            GradientGeometry::Linear {
                start: BoxPoint::new(-1.0, 0.5).unwrap(),
                end: BoxPoint::new(1.0, 0.5).unwrap(),
            }
        );
    }

    #[test]
    fn a_circle_under_a_rotation_keeps_its_handle_on_the_image() {
        let frame = unit_box((0.0, 0.0), (20.0, 20.0));
        let quarter = Affine::new(0.0, 1.0, -1.0, 0.0, 20.0, 0.0);
        let turned = GradientGeometry::DEFAULT_RADIAL.mapped(&frame, quarter, &frame);
        let GradientGeometry::Radial { radius, ratio, .. } = turned else {
            panic!("radial")
        };
        // The radius handle (20, 10) goes to (10, 20): the middle of the bottom edge.
        assert!((radius.x.get() - 0.5).abs() < 1e-12 && (radius.y.get() - 1.0).abs() < 1e-12);
        assert_eq!(ratio, AxisRatio::CIRCLE);
    }
}
