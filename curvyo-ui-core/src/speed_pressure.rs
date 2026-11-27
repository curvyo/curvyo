//! Pressure simulated from drawing speed for a stroke without pen pressure (`specs/0158-pen-display`
//! slice 1 criterion 8, ADR 0095 decision 6): `p = clamp(1 − v / 300 mm/s, 0, 1)`.

use curvyo_document_core::Point;

/// The speed at which the simulated pressure reaches 0, in millimetres per second (Proposal).
pub const ZERO_PRESSURE_SPEED_MM_PER_S: f64 = 300.0;

/// The simulated pressure at sample `index` of a stroke's document points and source-clock
/// times (ms): from the speed of the step into it, the first sample from the step after it. A
/// step with no time takes the speed of the step before it; a single sample has pressure 1.
#[must_use]
pub fn speed_pressure_at(points: &[Point], times: &[f64], index: usize) -> f64 {
    let speed = speed_into(points, times, index.max(1));
    (1.0 - speed / ZERO_PRESSURE_SPEED_MM_PER_S).clamp(0.0, 1.0)
}

/// The speed in mm/s of the step into sample `index` (0 for none), falling back to earlier steps
/// where a step has no time.
fn speed_into(points: &[Point], times: &[f64], index: usize) -> f64 {
    let mut i = index.min(points.len().min(times.len()).saturating_sub(1));
    while i >= 1 {
        let dt = times[i] - times[i - 1];
        if dt > 0.0 && dt.is_finite() {
            let (a, b) = (points[i - 1], points[i]);
            return (b.x - a.x).hypot(b.y - a.y) / dt * 1000.0;
        }
        i -= 1;
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Samples along x at `step` mm every `ms` milliseconds.
    fn line(count: usize, step: f64, ms: f64) -> (Vec<Point>, Vec<f64>) {
        #[allow(clippy::cast_precision_loss)]
        let points = (0..count)
            .map(|i| Point::new(i as f64 * step, 0.0))
            .collect();
        #[allow(clippy::cast_precision_loss)]
        let times = (0..count).map(|i| i as f64 * ms).collect();
        (points, times)
    }

    #[test]
    fn constant_speed_gives_a_constant_pressure() {
        // 1 mm every 8 ms = 125 mm/s.
        let (points, times) = line(20, 1.0, 8.0);
        let all: Vec<f64> = (0..20)
            .map(|i| speed_pressure_at(&points, &times, i))
            .collect();
        for p in &all {
            assert!((p - (1.0 - 125.0 / 300.0)).abs() < 1e-9, "{all:?}");
        }
    }

    #[test]
    fn slower_is_more_pressure_and_the_ends_clamp() {
        let (fast_p, fast_t) = line(5, 1.0, 8.0);
        let (slow_p, slow_t) = line(5, 1.0, 16.0);
        assert!(speed_pressure_at(&slow_p, &slow_t, 3) > speed_pressure_at(&fast_p, &fast_t, 3));
        let (very_fast_p, very_fast_t) = line(5, 10.0, 8.0);
        assert_eq!(speed_pressure_at(&very_fast_p, &very_fast_t, 2), 0.0);
        let (still_p, still_t) = line(5, 0.0, 8.0);
        assert_eq!(speed_pressure_at(&still_p, &still_t, 2), 1.0);
    }

    #[test]
    fn a_step_without_time_takes_the_speed_before() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(2.0, 0.0),
        ];
        let times = vec![0.0, 8.0, 8.0];
        let p = speed_pressure_at(&points, &times, 2);
        assert!((p - (1.0 - 125.0 / 300.0)).abs() < 1e-9);
        assert_eq!(speed_pressure_at(&points[..1], &times[..1], 0), 1.0);
    }
}
