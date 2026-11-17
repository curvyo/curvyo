//! The unit vector of an angle in degrees, by `libm`'s `sin` and `cos`, so a direction is the
//! same on every target (ADR 0064 §2).

use curvyo_document_core::Vec2;

/// The unit vector at `degrees` counter-clockwise from +X in a Y-up frame: `(cos, sin)`. Whole
/// multiples of 90 degrees give exact axis vectors (no `6e-17` residue), so a scan at 0 degrees
/// runs exactly along X. A non-finite angle gives +X.
#[must_use]
pub fn direction_of_degrees(degrees: f64) -> Vec2 {
    if !degrees.is_finite() {
        return Vec2::new(1.0, 0.0);
    }
    let turn = degrees.rem_euclid(360.0);
    let quarters = [
        (0.0, Vec2::new(1.0, 0.0)),
        (90.0, Vec2::new(0.0, 1.0)),
        (180.0, Vec2::new(-1.0, 0.0)),
        (270.0, Vec2::new(0.0, -1.0)),
    ];
    #[allow(clippy::float_cmp)]
    // exact: whole quarter turns are exact in binary
    let quarter = quarters.into_iter().find(|(at, _)| turn == *at);
    quarter.map_or_else(
        || {
            let radians = turn.to_radians();
            Vec2::new(libm::cos(radians), libm::sin(radians))
        },
        |(_, axis)| axis,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn quarter_turns_are_exact_axes() {
        assert_eq!(direction_of_degrees(0.0), Vec2::new(1.0, 0.0));
        assert_eq!(direction_of_degrees(90.0), Vec2::new(0.0, 1.0));
        assert_eq!(direction_of_degrees(-90.0), Vec2::new(0.0, -1.0));
        assert_eq!(direction_of_degrees(180.0), Vec2::new(-1.0, 0.0));
        assert_eq!(direction_of_degrees(450.0), Vec2::new(0.0, 1.0));
        assert_eq!(direction_of_degrees(f64::NAN), Vec2::new(1.0, 0.0));
    }

    #[test]
    fn other_angles_are_unit_vectors() {
        let d = direction_of_degrees(45.0);
        assert!((d.x - d.y).abs() < 1e-15);
        assert!((d.x * d.x + d.y * d.y - 1.0).abs() < 1e-15);
        let back = direction_of_degrees(-30.0);
        assert!((back.y + 0.5).abs() < 1e-15);
    }
}
