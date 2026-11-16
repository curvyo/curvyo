//! The axis point of a chord: the point on it farthest from the outline, found by a
//! golden-section search to 0.001 mm.

use kurbo::Point;

use super::AxisSample;
use super::boundary::Boundary;

/// The search stops once the bracket is this short, in millimetres.
const PRECISION_MM: f64 = 0.001;
/// The golden ratio's inverse, `(sqrt(5) - 1) / 2`.
const INVERSE_PHI: f64 = 0.618_033_988_749_894_8;

/// The point of the chord from `a` to `b` with the largest clearance, and that clearance.
pub(super) fn axis_point(boundary: &Boundary, a: Point, b: Point) -> AxisSample {
    let length = (b - a).hypot();
    let at = |t: f64| a.lerp(b, t);
    let clearance = |t: f64| boundary.distance(at(t));
    let (mut low, mut high) = (0.0_f64, 1.0_f64);
    let mut left = high - INVERSE_PHI * (high - low);
    let mut right = low + INVERSE_PHI * (high - low);
    let (mut left_value, mut right_value) = (clearance(left), clearance(right));
    while (high - low) * length > PRECISION_MM {
        if left_value < right_value {
            low = left;
            left = right;
            left_value = right_value;
            right = low + INVERSE_PHI * (high - low);
            right_value = clearance(right);
        } else {
            high = right;
            right = left;
            right_value = left_value;
            left = high - INVERSE_PHI * (high - low);
            left_value = clearance(left);
        }
    }
    let t = f64::midpoint(low, high);
    let point = at(t);
    AxisSample {
        point,
        radius: boundary.distance(point),
    }
}
