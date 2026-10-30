//! Smooths a boundary chain with a binomial kernel before the fit (`specs/0089` adrs.md §5,
//! input smoothing): repeated `[1, 2, 1] / 4` passes, an open chain's end points pinned.

use kurbo::Point;

/// The passes of `[1, 2, 1] / 4` whose kernel has a standard deviation of `sigma` samples
/// (`passes / 2` is its variance).
pub(crate) fn passes(sigma: f64) -> usize {
    if sigma.is_nan() || sigma <= 0.0 {
        return 0;
    }
    // Clamped to a whole number in 1 to 1 000 first, so the conversion is exact.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = (2.0 * sigma * sigma).round().clamp(0.0, 1_000.0) as usize;
    count
}

/// One pass over `q` into `out`, `q[0]` and its last point kept.
fn pass_open(q: &[Point], out: &mut [Point]) {
    let n = q.len();
    out[0] = q[0];
    out[n - 1] = q[n - 1];
    for i in 1..n - 1 {
        out[i] = Point::new(
            (q[i - 1].x + 2.0 * q[i].x + q[i + 1].x) / 4.0,
            (q[i - 1].y + 2.0 * q[i].y + q[i + 1].y) / 4.0,
        );
    }
}

/// `q` smoothed by `passes` passes with its two end points pinned.
pub(crate) fn smooth_open(q: &mut [Point], passes: usize) {
    if q.len() < 3 {
        return;
    }
    let mut buffer = q.to_vec();
    for _ in 0..passes {
        pass_open(q, &mut buffer);
        q.copy_from_slice(&buffer);
    }
}

/// The closed chain `q` (its last point not repeated) smoothed by `passes` passes.
pub(crate) fn smooth_closed(q: &mut [Point], passes: usize) {
    let len = q.len();
    if len < 3 {
        return;
    }
    let mut buffer = q.to_vec();
    for _ in 0..passes {
        for i in 0..len {
            let (before, here, after) = (q[(i + len - 1) % len], q[i], q[(i + 1) % len]);
            buffer[i] = Point::new(
                (before.x + 2.0 * here.x + after.x) / 4.0,
                (before.y + 2.0 * here.y + after.y) / 4.0,
            );
        }
        q.copy_from_slice(&buffer);
    }
}

#[cfg(test)]
// Test shapes are small, named by their formulas (x, y, w, h).
#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
mod tests {
    use super::*;

    #[test]
    fn pinned_ends_stay_and_a_zigzag_calms() {
        let mut q: Vec<Point> = (0..20)
            .map(|i| Point::new(f64::from(i), f64::from(i % 2)))
            .collect();
        smooth_open(&mut q, passes(1.5));
        assert_eq!(q[0], Point::new(0.0, 0.0));
        assert_eq!(q[19], Point::new(19.0, 1.0));
        assert!(q[5..15].iter().all(|p| (p.y - 0.5).abs() < 0.05), "{q:?}");
    }

    #[test]
    fn a_closed_square_keeps_its_centre() {
        let mut q = vec![
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(2.0, 2.0),
            Point::new(0.0, 2.0),
        ];
        smooth_closed(&mut q, 3);
        let c = q.iter().fold(Point::ZERO, |s, p| s + p.to_vec2());
        assert_eq!(c, Point::new(4.0, 4.0));
        assert_eq!(passes(0.0), 0);
    }
}
