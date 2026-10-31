//! Samples a Bézier outline into points at a fixed spacing in basic operations.
//!
//! Uniform parameter steps per segment, their number from the control polygon's length, so no
//! `kurbo` arc length or flattening (they use `cbrt`, `acos` or adaptive recursion; `adrs.md`
//! decision 4 and 7).

use kurbo::{BezPath, PathEl, Point};

/// One closed outline as points in order; the last point joins the first.
pub(crate) type Ring = Vec<Point>;

/// Samples every subpath of `path` into a ring with points about `spacing` apart (at most that
/// far apart along the control polygon). `None` when a subpath is open, when the rings would hold
/// more than `max_points` points together, or when a coordinate is not finite.
pub(crate) fn sample_closed(path: &BezPath, spacing: f64, max_points: usize) -> Option<Vec<Ring>> {
    let mut rings: Vec<Ring> = Vec::new();
    let mut ring: Ring = Vec::new();
    let mut start = Point::ZERO;
    let mut current = Point::ZERO;
    let mut open = false;
    let mut total = 0_usize;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => {
                if open {
                    return None;
                }
                start = p;
                current = p;
                ring.clear();
                open = true;
            }
            PathEl::LineTo(p) => {
                push_line(&mut ring, current, p, spacing);
                current = p;
            }
            PathEl::QuadTo(a, b) => {
                let c1 = current + (a - current) * (2.0 / 3.0);
                let c2 = b + (a - b) * (2.0 / 3.0);
                push_cubic(&mut ring, [current, c1, c2, b], spacing);
                current = b;
            }
            PathEl::CurveTo(a, b, c) => {
                push_cubic(&mut ring, [current, a, b, c], spacing);
                current = c;
            }
            PathEl::ClosePath => {
                if !open {
                    continue;
                }
                if current.distance_squared(start) > 0.0 {
                    push_line(&mut ring, current, start, spacing);
                }
                current = start;
                open = false;
                if ring.len() >= 3 {
                    total += ring.len();
                    if total > max_points {
                        return None;
                    }
                    rings.push(std::mem::take(&mut ring));
                } else {
                    ring.clear();
                }
            }
        }
        if ring.len() > max_points {
            return None;
        }
    }
    let finite = rings
        .iter()
        .flatten()
        .all(|p| p.x.is_finite() && p.y.is_finite());
    (!open && finite && !rings.is_empty()).then_some(rings)
}

/// The number of steps a stretch of `length` needs at `spacing`, at least 1.
fn steps(length: f64, spacing: f64) -> usize {
    let n = (length / spacing).ceil();
    if n.is_finite() && n >= 1.0 {
        // invariant: clamped to a small positive whole number before the conversion.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = n.min(100_000.0) as usize;
        n
    } else {
        1
    }
}

/// The distance of two points by `sqrt` (no `hypot`, `adrs.md` decision 7).
pub(crate) fn dist(a: Point, b: Point) -> f64 {
    let (dx, dy) = (a.x - b.x, a.y - b.y);
    (dx * dx + dy * dy).sqrt()
}

/// `n` as a float.
pub(crate) fn f(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Appends the points of the line from `a` to `b`, `a` included and `b` left out.
fn push_line(ring: &mut Ring, a: Point, b: Point, spacing: f64) {
    let n = steps(dist(a, b), spacing);
    for k in 0..n {
        ring.push(a.lerp(b, f(k) / f(n)));
    }
}

/// Appends the points of the cubic `c`, its start included and its end left out.
fn push_cubic(ring: &mut Ring, c: [Point; 4], spacing: f64) {
    let polygon = dist(c[0], c[1]) + dist(c[1], c[2]) + dist(c[2], c[3]);
    let n = steps(polygon, spacing);
    for k in 0..n {
        let t = f(k) / f(n);
        let u = 1.0 - t;
        let (b0, b1, b2, b3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        ring.push(Point::new(
            b0 * c[0].x + b1 * c[1].x + b2 * c[2].x + b3 * c[3].x,
            b0 * c[0].y + b1 * c[1].y + b2 * c[2].y + b3 * c[3].y,
        ));
    }
}

/// The signed area of a ring (positive when it turns clockwise on screen, y down).
pub(crate) fn signed_area(ring: &[Point]) -> f64 {
    let mut sum = 0.0;
    for (i, p) in ring.iter().enumerate() {
        let q = ring[(i + 1) % ring.len()];
        sum += p.x * q.y - q.x * p.y;
    }
    sum / 2.0
}

/// Whether `p` lies inside `ring` by the even-odd rule (crossing count).
#[allow(clippy::many_single_char_names)]
pub(crate) fn inside(ring: &[Point], p: Point) -> bool {
    let mut odd = false;
    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y);
            if x > p.x {
                odd = !odd;
            }
        }
    }
    odd
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Shape;

    #[test]
    fn a_square_is_sampled_at_the_spacing() {
        let path = kurbo::Rect::new(0.0, 0.0, 1.0, 1.0).to_path(0.1);
        let rings = sample_closed(&path, 0.05, 10_000).unwrap_or_default();
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].len(), 80);
        assert!((signed_area(&rings[0]).abs() - 1.0).abs() < 1e-9);
        assert!(inside(&rings[0], Point::new(0.5, 0.5)));
        assert!(!inside(&rings[0], Point::new(1.5, 0.5)));
    }

    #[test]
    fn open_or_too_long_outlines_are_refused() {
        let mut open = BezPath::new();
        open.move_to((0.0, 0.0));
        open.line_to((1.0, 0.0));
        assert!(sample_closed(&open, 0.05, 10_000).is_none());
        let square = kurbo::Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.1);
        assert!(sample_closed(&square, 0.05, 100).is_none());
    }
}
