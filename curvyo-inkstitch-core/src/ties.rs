// SPDX-License-Identifier: GPL-3.0-or-later
// Ported from Ink/Stitch `lib/stitch_plan/lock_stitch.py` (`RelativeLock`, `half_stitch`;
// https://github.com/inkstitch/inkstitch); Copyright (c) 2010 Authors (see git history); changed
// for Curvyo (see `NOTICE.md`).

//! Ink/Stitch's default lock stitches (`specs/0160-embroidery` criterion 10): the half stitch,
//! back along the run half a lock length, a whole one, half and home again.

use kurbo::{Point, Vec2};

/// The half stitch's path in lock lengths (upstream `"0 0.5 1 0.5 0"`).
const HALF_STITCH: [f64; 5] = [0.0, 0.5, 1.0, 0.5, 0.0];
/// The shortest and longest lock length (upstream 0.5 and 1.5 mm).
const LOCK_MM: (f64, f64) = (0.5, 1.5);
/// How many stitches are looked at for a direction (upstream 15).
const LOOK_AHEAD: usize = 15;

/// The direction from the first stitch to the first one more than 0.5 mm away and that
/// distance clamped to the lock lengths; along X at 0.5 mm when there is none.
fn direction_and_length(stitches: &[Point]) -> (Vec2, f64) {
    for i in 1..=LOOK_AHEAD.min(stitches.len().saturating_sub(1)) {
        let to = stitches[i] - stitches[0];
        let length = to.hypot();
        if length > LOCK_MM.0 {
            return (to / length, length.clamp(LOCK_MM.0, LOCK_MM.1));
        }
    }
    (Vec2::new(1.0, 0.0), LOCK_MM.0)
}

fn lock(stitches: &[Point], path: &[f64]) -> Vec<Point> {
    let Some(&first) = stitches.first() else {
        return Vec::new();
    };
    let (direction, length) = direction_and_length(stitches);
    path.iter()
        .map(|delta| first + direction * (delta * length))
        .collect()
}

/// The tie-in before the run `stitches`: five points from its first stitch along the run and
/// back to it (three needle points away from the start).
#[must_use]
pub fn tie_in(stitches: &[Point]) -> Vec<Point> {
    lock(stitches, &HALF_STITCH)
}

/// The tie-off after the run `stitches`: from its last stitch back along the run and home, the
/// duplicate first point left out (upstream).
#[must_use]
pub fn tie_off(stitches: &[Point]) -> Vec<Point> {
    let reversed: Vec<Point> = stitches.iter().rev().copied().collect();
    lock(&reversed, &HALF_STITCH[1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_half_stitch_goes_half_whole_half_and_home() {
        let run = [
            Point::new(0.0, 0.0),
            Point::new(2.5, 0.0),
            Point::new(5.0, 0.0),
        ];
        let xs = |v: Vec<Point>| v.iter().map(|p| p.x).collect::<Vec<_>>();
        assert_eq!(
            xs(tie_in(&run)),
            [0.0, 0.75, 1.5, 0.75, 0.0],
            "1.5 mm at most"
        );
        assert_eq!(xs(tie_off(&run)), [4.25, 3.5, 4.25, 5.0]);
        let short = [
            Point::new(0.0, 0.0),
            Point::new(0.2, 0.0),
            Point::new(0.0, 0.8),
        ];
        let ys: Vec<f64> = tie_in(&short).iter().map(|p| p.y).collect();
        assert_eq!(
            ys,
            [0.0, 0.4, 0.8, 0.4, 0.0],
            "the first stitch over 0.5 mm away"
        );
    }
}
