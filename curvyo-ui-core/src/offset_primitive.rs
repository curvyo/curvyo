//! Whether the offset of a rectangle, ellipse, polygon or star is again that primitive, and which
//! one (`specs/0038-path-offset` criteria 40, 41, 43, 50, 51): computed from the primitive's own
//! fields, exact, in its own unrotated frame, so the caller keeps its rotation.

use curvyo_document_core::{
    CornerRadii, EllipseFrame, Length, Point, RectBounds, Shape, StarFrame, effective_corner_radii,
};

use crate::offset_entry::OffsetJoin;

/// The kernel tolerance: an ellipse stays an ellipse only within it (criterion 40).
const STAYS_TOLERANCE_MM: f64 = 0.01;

/// Radii and distances below this, in millimetres, are zero.
const ZERO_MM: f64 = 1e-9;

/// The miter limit 4 as `1 + cos(turn)` (see `offset_pieces` in the kernel).
const MITER_LIMIT_ONE_PLUS_COS: f64 = 0.125;

/// How many points per quarter of an ellipse the deviation check samples.
const ELLIPSE_SAMPLES: u32 = 64;

/// What the offset of a primitive is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrimitiveOffset {
    /// The same kind of primitive with these fields (in the primitive's own frame).
    Stays(Shape),
    /// Nothing is left (criterion 41).
    Vanishes,
    /// Not exactly a primitive: the curve kernel computes a path.
    Path,
}

/// The offset of `shape` by `d` (positive grows) with `join` at the corners where a gap opens.
#[must_use]
pub fn offset_primitive(shape: &Shape, d: f64, join: OffsetJoin) -> PrimitiveOffset {
    match *shape {
        Shape::Rect {
            bounds,
            corner_radii,
        } => offset_rect(bounds, corner_radii, d, join),
        Shape::Ellipse { frame } => offset_ellipse(frame, d),
        Shape::Polygon { frame, point_count } => {
            let n = f64::from(point_count.get());
            match scaled_frame(frame, &regular_vertices(n, 1.0), d, join) {
                Scaled::Stays(frame) => {
                    PrimitiveOffset::Stays(Shape::Polygon { frame, point_count })
                }
                Scaled::Vanishes => PrimitiveOffset::Vanishes,
                Scaled::Path => PrimitiveOffset::Path,
            }
        }
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
        } => {
            let n = f64::from(point_count.get());
            match scaled_frame(frame, &star_vertices(n, inner_ratio.get()), d, join) {
                Scaled::Stays(frame) => PrimitiveOffset::Stays(Shape::Star {
                    frame,
                    point_count,
                    inner_ratio,
                }),
                Scaled::Vanishes => PrimitiveOffset::Vanishes,
                Scaled::Path => PrimitiveOffset::Path,
            }
        }
    }
}

/// A rectangle: every side moves by `d`; a rounded corner gets `r + d` (at least 0); a sharp
/// corner of an outset follows the join (Miter sharp, Round radius `d`, Bevel not a rectangle).
fn offset_rect(
    bounds: RectBounds,
    radii: CornerRadii,
    d: f64,
    join: OffsetJoin,
) -> PrimitiveOffset {
    let (w, h) = (bounds.width.as_mm(), bounds.height.as_mm());
    if w + 2.0 * d <= ZERO_MM || h + 2.0 * d <= ZERO_MM {
        return PrimitiveOffset::Vanishes;
    }
    let effective = effective_corner_radii(bounds, radii);
    let mut grown = [effective.tl, effective.tr, effective.br, effective.bl];
    for radius in &mut grown {
        let r = radius.as_mm();
        let new = if r > ZERO_MM {
            (r + d).max(0.0)
        } else if d < 0.0 {
            0.0
        } else {
            match join {
                OffsetJoin::Miter => 0.0,
                OffsetJoin::Round => d,
                OffsetJoin::Bevel => return PrimitiveOffset::Path,
            }
        };
        *radius = Length::from_mm(new);
    }
    let [tl, tr, br, bl] = grown;
    PrimitiveOffset::Stays(Shape::Rect {
        bounds: RectBounds {
            origin: Point::new(bounds.origin.x - d, bounds.origin.y - d),
            width: Length::from_mm(w + 2.0 * d),
            height: Length::from_mm(h + 2.0 * d),
        },
        corner_radii: CornerRadii { tl, tr, br, bl },
    })
}

/// A circle stays a circle; an ellipse stays one only when the ellipse `(rx + d, ry + d)` is
/// within the tolerance of the exact offset, sampled deterministically.
fn offset_ellipse(frame: EllipseFrame, d: f64) -> PrimitiveOffset {
    let (rx, ry) = (frame.rx.as_mm(), frame.ry.as_mm());
    if rx.min(ry) + d <= ZERO_MM {
        return PrimitiveOffset::Vanishes;
    }
    let grown = EllipseFrame {
        center: frame.center,
        rx: Length::from_mm(rx + d),
        ry: Length::from_mm(ry + d),
    };
    if (rx - ry).abs() <= ZERO_MM || ellipse_deviation(rx, ry, d) <= STAYS_TOLERANCE_MM {
        PrimitiveOffset::Stays(Shape::Ellipse { frame: grown })
    } else {
        PrimitiveOffset::Path
    }
}

/// The largest distance between the exact offset of the ellipse `(a, b)` by `d` and the ellipse
/// `(a + d, b + d)`, over points of one quarter taken on the rational parametrisation (so only
/// `+ - * /` and `sqrt` take part), each measured to first order as `|F| / |grad F|`.
#[allow(clippy::many_single_char_names)] // the textbook names of the ellipse
fn ellipse_deviation(a: f64, b: f64, d: f64) -> f64 {
    let (big_a, big_b) = (a + d, b + d);
    let mut largest = 0.0_f64;
    for k in 0..=ELLIPSE_SAMPLES {
        let t = f64::from(k) / f64::from(ELLIPSE_SAMPLES);
        let (cos, sin) = ((1.0 - t * t) / (1.0 + t * t), 2.0 * t / (1.0 + t * t));
        let (x, y) = (a * cos, b * sin);
        let (nx, ny) = (x / (a * a), y / (b * b));
        let length = (nx * nx + ny * ny).sqrt();
        let (qx, qy) = (x + d * nx / length, y + d * ny / length);
        let f = qx * qx / (big_a * big_a) + qy * qy / (big_b * big_b) - 1.0;
        let (gx, gy) = (2.0 * qx / (big_a * big_a), 2.0 * qy / (big_b * big_b));
        largest = largest.max(f.abs() / (gx * gx + gy * gy).sqrt());
    }
    largest
}

/// The vertices of a regular polygon of `n` corners on the unit circle, first at angle 0.
fn regular_vertices(n: f64, radius: f64) -> Vec<(f64, f64)> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 3..=1024
    let count = n as usize;
    (0..count)
        .map(|k| {
            #[allow(clippy::cast_precision_loss)]
            let theta = std::f64::consts::TAU * k as f64 / n;
            (radius * theta.cos(), radius * theta.sin())
        })
        .collect()
}

/// The vertices of a star of `n` points and inner `ratio`, outer radius 1, alternating outer and
/// inner.
fn star_vertices(n: f64, ratio: f64) -> Vec<(f64, f64)> {
    let outer = regular_vertices(n, 1.0);
    let step = std::f64::consts::PI / n;
    let mut vertices = Vec::with_capacity(2 * outer.len());
    for (k, vertex) in outer.into_iter().enumerate() {
        vertices.push(vertex);
        #[allow(clippy::cast_precision_loss)]
        let theta = std::f64::consts::TAU * k as f64 / n + step;
        vertices.push((ratio * theta.cos(), ratio * theta.sin()));
    }
    vertices
}

/// A polygon or star whose edge lines all touch one circle around its centre (radius `h`): its
/// offset is the shape scaled by `(h + d) / h`, unless a corner where a gap opens is not a miter
/// within the limit. `unit` are the shape's corners at outer radius 1, in order.
#[allow(clippy::many_single_char_names)] // corners a, b, c and the vectors between them
fn scaled_frame(frame: StarFrame, unit: &[(f64, f64)], d: f64, join: OffsetJoin) -> Scaled {
    let n = unit.len();
    let area: f64 = (0..n)
        .map(|i| {
            let (a, b) = (unit[i], unit[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    let orientation = area.signum();
    // The distance of the first edge line from the centre, at outer radius 1.
    let (p, q) = (unit[0], unit[1]);
    let (ex, ey) = (q.0 - p.0, q.1 - p.1);
    let apothem = (p.0 * q.1 - p.1 * q.0).abs() / (ex * ex + ey * ey).sqrt();
    let h = apothem * frame.radius.as_mm();
    if h + d <= ZERO_MM {
        return Scaled::Vanishes;
    }
    // Corners of a polygon are alike, a star has two kinds: checking the first two covers all.
    for corner in 1..=2.min(n - 1) {
        let (a, b, c) = (unit[corner - 1], unit[corner], unit[(corner + 1) % n]);
        let u1 = unit_vector(b.0 - a.0, b.1 - a.1);
        let u2 = unit_vector(c.0 - b.0, c.1 - b.1);
        let cross = (u1.0 * u2.1 - u1.1 * u2.0) * orientation;
        let dot = u1.0 * u2.0 + u1.1 * u2.1;
        let gap = cross * d > ZERO_MM * ZERO_MM;
        if gap && (join != OffsetJoin::Miter || 1.0 + dot < MITER_LIMIT_ONE_PLUS_COS) {
            return Scaled::Path;
        }
    }
    Scaled::Stays(StarFrame {
        center: frame.center,
        radius: Length::from_mm(frame.radius.as_mm() * (h + d) / h),
        angle: frame.angle,
    })
}

/// The offset of a polygon or star as its frame.
enum Scaled {
    Stays(StarFrame),
    Vanishes,
    Path,
}

fn unit_vector(x: f64, y: f64) -> (f64, f64) {
    let length = (x * x + y * y).sqrt();
    (x / length, y / length)
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{Angle, InnerRatio, PointCount};

    fn rect(w: f64, h: f64, r: f64) -> Shape {
        Shape::Rect {
            bounds: RectBounds {
                origin: Point::new(0.0, 0.0),
                width: Length::from_mm(w),
                height: Length::from_mm(h),
            },
            corner_radii: CornerRadii::uniform(Length::from_mm(r)),
        }
    }

    fn rect_fields(result: PrimitiveOffset) -> (f64, f64, f64, f64, [f64; 4]) {
        let PrimitiveOffset::Stays(Shape::Rect {
            bounds,
            corner_radii: r,
        }) = result
        else {
            panic!("a rectangle: {result:?}");
        };
        (
            bounds.origin.x,
            bounds.origin.y,
            bounds.width.as_mm(),
            bounds.height.as_mm(),
            [r.tl.as_mm(), r.tr.as_mm(), r.br.as_mm(), r.bl.as_mm()],
        )
    }

    #[test]
    fn ac50_a_rounded_rectangle_grows_its_radius_with_every_join() {
        for join in [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Bevel] {
            let (x, y, w, h, radii) =
                rect_fields(offset_primitive(&rect(100.0, 50.0, 5.0), 2.0, join));
            assert_eq!((x, y, w, h), (-2.0, -2.0, 104.0, 54.0));
            assert_eq!(radii, [7.0; 4]);
        }
    }

    #[test]
    fn ac50_a_sharp_rectangle_follows_the_join() {
        let (_, _, w, _, radii) = rect_fields(offset_primitive(
            &rect(20.0, 20.0, 0.0),
            2.0,
            OffsetJoin::Round,
        ));
        assert_eq!((w, radii), (24.0, [2.0; 4]));
        let (_, _, w, _, radii) = rect_fields(offset_primitive(
            &rect(20.0, 20.0, 0.0),
            2.0,
            OffsetJoin::Miter,
        ));
        assert_eq!((w, radii), (24.0, [0.0; 4]));
        assert_eq!(
            offset_primitive(&rect(20.0, 20.0, 0.0), 2.0, OffsetJoin::Bevel),
            PrimitiveOffset::Path
        );
    }

    #[test]
    fn ac40_an_inset_rectangle_shrinks_its_radii_and_vanishes_at_half_the_side() {
        for join in [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Bevel] {
            let (x, _, w, h, radii) =
                rect_fields(offset_primitive(&rect(100.0, 50.0, 5.0), -2.0, join));
            assert_eq!((x, w, h), (2.0, 96.0, 46.0));
            assert_eq!(radii, [3.0; 4]);
            let (_, _, _, _, radii) =
                rect_fields(offset_primitive(&rect(100.0, 50.0, 5.0), -7.0, join));
            assert_eq!(radii, [0.0; 4]);
            assert_eq!(
                offset_primitive(&rect(100.0, 50.0, 5.0), -25.0, join),
                PrimitiveOffset::Vanishes
            );
        }
    }

    #[test]
    fn ac50_a_circle_stays_a_circle() {
        let circle = Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(5.0, 5.0),
                rx: Length::from_mm(20.0),
                ry: Length::from_mm(20.0),
            },
        };
        for join in [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Bevel] {
            let PrimitiveOffset::Stays(Shape::Ellipse { frame }) =
                offset_primitive(&circle, 2.0, join)
            else {
                panic!("a circle");
            };
            assert_eq!((frame.rx.as_mm(), frame.ry.as_mm()), (22.0, 22.0));
            assert_eq!(frame.center, Point::new(5.0, 5.0));
        }
        let PrimitiveOffset::Stays(Shape::Ellipse { frame }) =
            offset_primitive(&circle, -2.0, OffsetJoin::Round)
        else {
            panic!("a circle");
        };
        assert_eq!(frame.rx.as_mm(), 18.0);
        assert_eq!(
            offset_primitive(&circle, -20.0, OffsetJoin::Round),
            PrimitiveOffset::Vanishes
        );
    }

    #[test]
    fn ac51_an_oval_ellipse_becomes_a_path_and_a_nearly_round_one_stays() {
        let ellipse = |rx: f64, ry: f64| Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(0.0, 0.0),
                rx: Length::from_mm(rx),
                ry: Length::from_mm(ry),
            },
        };
        assert_eq!(
            offset_primitive(&ellipse(20.0, 10.0), 2.0, OffsetJoin::Round),
            PrimitiveOffset::Path
        );
        assert!(ellipse_deviation(20.0, 10.0, 2.0) > 0.05);
        assert!(matches!(
            offset_primitive(&ellipse(20.0, 19.999), 2.0, OffsetJoin::Round),
            PrimitiveOffset::Stays(_)
        ));
    }

    #[test]
    fn ac50_a_hexagon_grows_by_d_over_cos_30_with_miter_only() {
        let hexagon = Shape::Polygon {
            frame: StarFrame {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(10.0),
                angle: Angle::from_radians(0.3),
            },
            point_count: PointCount::new(6).unwrap(),
        };
        let PrimitiveOffset::Stays(Shape::Polygon { frame, .. }) =
            offset_primitive(&hexagon, 2.0, OffsetJoin::Miter)
        else {
            panic!("a polygon");
        };
        assert!(
            (frame.radius.as_mm() - 12.309).abs() < 0.001,
            "{}",
            frame.radius.as_mm()
        );
        assert_eq!(frame.angle, Angle::from_radians(0.3));
        assert_eq!(
            offset_primitive(&hexagon, 2.0, OffsetJoin::Round),
            PrimitiveOffset::Path
        );
        assert_eq!(
            offset_primitive(&hexagon, 2.0, OffsetJoin::Bevel),
            PrimitiveOffset::Path
        );
        let PrimitiveOffset::Stays(Shape::Polygon { frame, .. }) =
            offset_primitive(&hexagon, -2.0, OffsetJoin::Round)
        else {
            panic!("an inset polygon stays one");
        };
        assert!((frame.radius.as_mm() - (10.0 - 2.0 / 30f64.to_radians().cos())).abs() < 1e-9);
        assert_eq!(
            offset_primitive(&hexagon, -9.0, OffsetJoin::Round),
            PrimitiveOffset::Vanishes
        );
    }

    #[test]
    fn ac40_a_star_scales_while_its_miters_stay_within_the_limit() {
        let star = |ratio: f64| Shape::Star {
            frame: StarFrame {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(20.0),
                angle: Angle::from_radians(0.0),
            },
            point_count: PointCount::new(5).unwrap(),
            inner_ratio: InnerRatio::new(ratio).unwrap(),
        };
        // A plump star: tips of about 108°, inside the limit.
        let PrimitiveOffset::Stays(Shape::Star {
            frame, inner_ratio, ..
        }) = offset_primitive(&star(0.7), 1.0, OffsetJoin::Miter)
        else {
            panic!("a star");
        };
        assert!(frame.radius.as_mm() > 20.0);
        assert!((inner_ratio.get() - 0.7).abs() < 1e-12);
        // A thin star: tips sharper than 29°, clipped by the limit.
        assert_eq!(
            offset_primitive(&star(0.1), 1.0, OffsetJoin::Miter),
            PrimitiveOffset::Path
        );
        assert_eq!(
            offset_primitive(&star(0.7), 1.0, OffsetJoin::Round),
            PrimitiveOffset::Path
        );
        // An inset opens gaps at the inner corners: Miter scales, Round is a path.
        assert!(matches!(
            offset_primitive(&star(0.5), -1.0, OffsetJoin::Miter),
            PrimitiveOffset::Stays(_)
        ));
        assert_eq!(
            offset_primitive(&star(0.5), -1.0, OffsetJoin::Round),
            PrimitiveOffset::Path
        );
    }
}
