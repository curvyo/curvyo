//! The affine map that takes one drawn outline to another, when one exists: how a transform
//! writer that is handed results, not a gesture map, learns the change T of each outline for the
//! held-back rule (ADR 0016 §2).

use crate::affine::Affine;
use crate::path_model::PathSnapshot;
use crate::units::Point;

/// How far (millimetres, scaled by the outline's extent where it is larger than 1 mm) a mapped
/// point may lie from its result for the change to count as one affine map.
const FIT_TOLERANCE: f64 = 1e-6;

/// The affine map T with `T(before) = after` for every anchor point and handle end of the two
/// outlines (matched in order), or `None` when the outlines do not match one by one or no affine
/// map takes one to the other (a node edit, not a transform). An outline whose points lie on one
/// line determines T only along that line; across it T is extended conformally, which leaves the
/// drawn outline the same whatever the true map was. A collapse to a point or a line gives a
/// singular T.
#[must_use]
pub(crate) fn outline_map(before: &PathSnapshot, after: &PathSnapshot) -> Option<Affine> {
    let mut pairs = Vec::new();
    let (mut old, mut new) = (before.all_anchors(), after.all_anchors());
    loop {
        match (old.next(), new.next()) {
            (Some(a), Some(b)) => {
                pairs.push((a.point, b.point));
                let end = |p: Point, h: crate::units::Vec2| Point::new(p.x + h.x, p.y + h.y);
                pairs.push((end(a.point, a.handle_in), end(b.point, b.handle_in)));
                pairs.push((end(a.point, a.handle_out), end(b.point, b.handle_out)));
            }
            (None, None) => break,
            _ => return None,
        }
    }
    fit(&pairs)
}

/// The least-squares affine map of `pairs`, if it takes every point to its partner within
/// [`FIT_TOLERANCE`].
#[allow(clippy::cast_precision_loss)] // a point count far below 2^52
#[allow(clippy::many_single_char_names)] // the entries a..f of the affine map, as in `Affine`
fn fit(pairs: &[(Point, Point)]) -> Option<Affine> {
    if pairs.is_empty() {
        return None;
    }
    let n = pairs.len() as f64;
    let mean = |pick: fn(&(Point, Point)) -> Point| {
        let (x, y) = pairs
            .iter()
            .map(pick)
            .fold((0.0, 0.0), |(x, y), p| (x + p.x, y + p.y));
        (x / n, y / n)
    };
    let (px, py) = mean(|pair| pair.0);
    let (qx, qy) = mean(|pair| pair.1);
    // The principal axes of the centred points first, then every sum in that frame: a thin
    // outline turned by any angle is fitted as accurately as one along the axes (no cancellation
    // in the determinant).
    let (mut a11, mut a12, mut a22) = (0.0, 0.0, 0.0);
    let mut extent: f64 = 1.0;
    for (p, q) in pairs {
        let (dx, dy) = (p.x - px, p.y - py);
        a11 += dx * dx;
        a12 += dx * dy;
        a22 += dy * dy;
        extent = extent
            .max(dx.abs())
            .max(dy.abs())
            .max((q.x - qx).abs())
            .max((q.y - qy).abs());
    }
    let (sin, cos) = (0.5 * (2.0 * a12).atan2(a11 - a22)).sin_cos();
    // Sums in the frame (u along the main axis, v across it): s = Σ (u, v)·(u, v)ᵀ, and the
    // products of the results with u and v.
    let (mut suu, mut suv, mut svv) = (0.0, 0.0, 0.0);
    let (mut xu, mut xv, mut yu, mut yv) = (0.0, 0.0, 0.0, 0.0);
    for (p, q) in pairs {
        let (dx, dy, ex, ey) = (p.x - px, p.y - py, q.x - qx, q.y - qy);
        let u = cos.mul_add(dx, sin * dy);
        let v = cos.mul_add(dy, -(sin * dx));
        suu += u * u;
        suv += u * v;
        svv += v * v;
        xu += ex * u;
        xv += ex * v;
        yu += ey * u;
        yv += ey * v;
    }
    let trace = suu + svv;
    let det = suu.mul_add(svv, -(suv * suv));
    let tolerance = FIT_TOLERANCE * extent;
    let accept = |(a, b, c, d): (f64, f64, f64, f64)| {
        let e = qx - a.mul_add(px, c * py);
        let f = qy - b.mul_add(px, d * py);
        let map = Affine::new(a, b, c, d, e, f);
        pairs
            .iter()
            .all(|&(p, q)| {
                let m = map.apply_point(p);
                (m.x - q.x).abs() <= tolerance && (m.y - q.y).abs() <= tolerance
            })
            .then_some(map)
    };
    if trace <= f64::EPSILON * extent * extent {
        // One point: a translation.
        return accept((1.0, 0.0, 0.0, 1.0));
    }
    // The linear part in document axes from its columns in the frame: A = A'·R with R taking
    // (dx, dy) to (u, v).
    let from_frame = |(au_x, au_y): (f64, f64), (av_x, av_y): (f64, f64)| {
        (
            au_x.mul_add(cos, -(av_x * sin)),
            au_y.mul_add(cos, -(av_y * sin)),
            au_x.mul_add(sin, av_x * cos),
            au_y.mul_add(sin, av_y * cos),
        )
    };
    // The full fit where the spread is two-dimensional beyond rounding (a test against trace²,
    // so it does not depend on how the outline is turned); when it does not hold, or the spread
    // is a line, the collinear fit.
    let full = (det > 1e3 * f64::EPSILON * trace * trace).then(|| {
        let (i11, i12, i22) = (svv / det, -suv / det, suu / det);
        from_frame(
            (xu.mul_add(i11, xv * i12), yu.mul_add(i11, yv * i12)),
            (xu.mul_add(i12, xv * i22), yu.mul_add(i12, yv * i22)),
        )
    });
    full.and_then(accept).or_else(|| {
        // Collinear: the main axis goes to v; the axis across it to v turned by 90°, so the map
        // is conformal across the line.
        let (vx, vy) = (xu / suu, yu / suu);
        accept(from_frame((vx, vy), (-vy, vx)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_model::{AnchorId, NewAnchor};
    use crate::style_model::Style;
    use crate::units::{Angle, Vec2};

    fn path(points: &[(f64, f64)]) -> PathSnapshot {
        PathSnapshot {
            id: crate::path_model::NodeId::from_parts(1, 1),
            closed: true,
            style: Style::default(),
            anchors: points
                .iter()
                .enumerate()
                .map(|(k, &(x, y))| {
                    let mut a = NewAnchor::corner(AnchorId::new(1, k as u64), Point::new(x, y));
                    a.handle_out = Vec2::new(1.0, 0.5);
                    a
                })
                .collect(),
            extra_subpaths: Vec::new(),
            rotation: Angle::from_radians(0.0),
        }
    }

    fn close(a: Affine, b: Affine) -> bool {
        a.to_array()
            .iter()
            .zip(b.to_array())
            .all(|(x, y)| (x - y).abs() < 1e-9)
    }

    #[test]
    fn a_mapped_square_gives_back_its_map() {
        let square = path(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        for g in [
            Affine::translation(Vec2::new(5.0, -3.0)),
            Affine::rotation_about(Point::new(5.0, 5.0), Angle::from_radians(1.2)),
            Affine::new(2.0, 0.3, -0.7, 0.5, 11.0, 4.0),
            Affine::scale(-1.0, 1.0),
        ] {
            let fitted = outline_map(&square, &square.mapped(g)).unwrap();
            assert!(close(fitted, g), "{fitted:?} {g:?}");
        }
    }

    #[test]
    fn a_node_edit_is_no_map() {
        let square = path(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        let mut edited = square.clone();
        edited.anchors[2].point = Point::new(15.0, 15.0);
        assert_eq!(outline_map(&square, &edited), None);
        let mut shorter = square.clone();
        shorter.anchors.pop();
        assert_eq!(outline_map(&square, &shorter), None);
    }

    #[test]
    fn a_collapse_to_a_line_is_singular() {
        let square = path(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        let flat = square.mapped(Affine::scale(0.0, 1.0));
        let fitted = outline_map(&square, &flat).unwrap();
        assert!(fitted.determinant().abs() < 1e-9);
    }

    #[test]
    fn a_line_moves_its_points_exactly() {
        let mut line = path(&[(0.0, 0.0), (10.0, 0.0)]);
        for a in &mut line.anchors {
            a.handle_out = Vec2::new(0.0, 0.0);
        }
        let g = Affine::rotation_about(Point::new(0.0, 0.0), Angle::from_radians(0.5));
        let moved = line.mapped(g);
        let fitted = outline_map(&line, &moved).unwrap();
        for (a, b) in line.anchors.iter().zip(&moved.anchors) {
            let p = fitted.apply_point(a.point);
            assert!((p.x - b.point.x).abs() < 1e-9 && (p.y - b.point.y).abs() < 1e-9);
        }
        assert!(fitted.determinant() > 0.5);
    }

    fn max_error(fitted: Affine, before: &PathSnapshot, after: &PathSnapshot) -> f64 {
        before
            .all_anchors()
            .zip(after.all_anchors())
            .map(|(a, b)| {
                let p = fitted.apply_point(a.point);
                (p.x - b.point.x).abs().max((p.y - b.point.y).abs())
            })
            .fold(0.0, f64::max)
    }

    #[test]
    fn review_f1_one_anchor_gives_a_translation() {
        let mut dot = path(&[(3.0, 4.0)]);
        dot.anchors[0].handle_out = Vec2::new(0.0, 0.0);
        let moved = dot.mapped(Affine::translation(Vec2::new(5.0, -2.0)));
        let fitted = outline_map(&dot, &moved).unwrap();
        assert!(
            close(fitted, Affine::translation(Vec2::new(5.0, -2.0))),
            "{fitted:?}"
        );
    }

    #[test]
    fn review_f1_a_square_far_out_fits_a_rotation() {
        let at = 1e6;
        let square = path(&[
            (at, at),
            (at + 10.0, at),
            (at + 10.0, at + 10.0),
            (at, at + 10.0),
        ]);
        let g = Affine::rotation_about(Point::new(at + 5.0, at + 5.0), Angle::from_radians(0.9));
        let moved = square.mapped(g);
        let fitted = outline_map(&square, &moved).unwrap();
        assert!(max_error(fitted, &square, &moved) < 1e-6);
    }

    #[test]
    fn review_f1_a_thin_sheared_rectangle_fits_and_inverts_within_the_limit() {
        let mut thin = path(&[(0.0, 0.0), (100.0, 0.0), (100.0, 1e-4), (0.0, 1e-4)]);
        for a in &mut thin.anchors {
            a.handle_out = Vec2::new(0.0, 0.0);
        }
        let shear = Affine::new(1.0, 0.0, 0.7, 1.0, 2.0, 3.0);
        let moved = thin.mapped(shear);
        let fitted = outline_map(&thin, &moved).unwrap();
        assert!(max_error(fitted, &thin, &moved) < 1e-6);
        let inverse = fitted.inverse().unwrap();
        assert!(inverse.is_within_limit(), "{inverse:?}");
    }

    #[test]
    fn review_f1_a_thin_rectangle_turned_45_degrees_and_sheared_fits() {
        let mut thin = path(&[(0.0, 0.0), (100.0, 0.0), (100.0, 1e-4), (0.0, 1e-4)]);
        for a in &mut thin.anchors {
            a.handle_out = Vec2::new(0.0, 0.0);
        }
        let turned = thin.mapped(Affine::rotation_about(
            Point::new(0.0, 0.0),
            Angle::from_radians(std::f64::consts::FRAC_PI_4),
        ));
        let shear = Affine::new(1.0, 0.0, 0.7, 1.0, 2.0, 3.0);
        let moved = turned.mapped(shear);
        let fitted = outline_map(&turned, &moved).unwrap();
        let error = max_error(fitted, &turned, &moved);
        assert!(error < 1e-6, "{error} {fitted:?}");
        assert!(fitted.inverse().unwrap().is_within_limit());
    }
}
