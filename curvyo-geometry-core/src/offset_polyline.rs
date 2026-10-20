//! The polyline route of the first build, kept as the fallback of the offset kernel
//! (`specs/0038-path-offset` criterion 39): the operand flattened onto the 0.001 mm grid and offset
//! by `i_overlay`'s own integer offsetter, with the join angles computed from `sqrt` only.

use i_overlay::i_float::int::angle::Angle;
use i_overlay::mesh::int::arc::ArcOptions;
use i_overlay::mesh::int::outline::offset::IntOutlineOffset;
use i_overlay::mesh::int::style::{IntLineJoin, IntOutlineStyle};
use i_overlay::mesh::math::MathMode;

use crate::boolean::{Outline, grid_paths};
use crate::boolean_cleanup::cleanup;
use crate::boolean_grid::{GRID_MM, Paths64, double_area, normalize};
use crate::offset::{OffsetError, OffsetJoin};

/// One turn in the library's binary angle units.
const UNITS_PER_TURN: f64 = 4_294_967_296.0;

/// The miter limit of 4 (SVG's) as the smallest interior angle that keeps a sharp miter:
/// `2 x asin(1/4)` = 28.955°, in binary angle units (`round(0.505360510284157 / 2π x 2³²)`).
/// A constant, so no platform's `asin` takes part.
const MITER_LIMIT_ANGLE: Angle = Angle::from_bits(345_446_896);

/// The library's default near-straight cutoff for miters, 5°.
const MITER_MIN_TURN_DEFAULT_RAD: f64 = 5.0 * std::f64::consts::PI / 180.0;

/// Largest miter error at a near-straight turn, squared over the distance (see `miter_min_turn`).
const MITER_STRAIGHT_ERROR_MM: f64 = 0.005;

/// The polyline route of the first build: flatten, offset with `i_overlay`'s own offsetter, clean
/// up. Used for operands whose outlines cross or touch.
pub(crate) fn polyline_offset(
    operand: &[Outline<'_>],
    units: i64,
    join: OffsetJoin,
    flatten_tolerance_mm: f64,
) -> Result<Paths64, OffsetError> {
    let region = normalize(&grid_paths(operand, flatten_tolerance_mm));
    let raw = offset_region(&region, units, join, flatten_tolerance_mm)?;
    Ok(without_flattening_noise(
        cleanup(&raw),
        flatten_tolerance_mm,
    ))
}

/// `i_overlay`'s integer outline offset of a normalized grid region by `units` grid units, with
/// round arcs whose chords stay within `arc_tolerance_mm`: the raw result, not cleaned up. Shared
/// with the gap closing of Weld (`specs/0068-laserable-lettering/adrs.md` decision 2).
pub(crate) fn offset_region(
    region: &Paths64,
    units: i64,
    join: OffsetJoin,
    arc_tolerance_mm: f64,
) -> Result<Paths64, OffsetError> {
    #[allow(clippy::cast_precision_loss)] // |units| <= 10^10
    let radius_mm = (units as f64 * GRID_MM).abs();
    let style = IntOutlineStyle::new(units)
        .line_join(int_join(join, radius_mm, arc_tolerance_mm))
        .miter_min_turn(miter_min_turn(radius_mm))
        .math(MathMode::Integer);
    region
        .validate_outline(&style)
        .map_err(|_| OffsetError::OutOfRange)?;
    let shapes = region
        .outline(&style)
        .map_err(|_| OffsetError::OutOfRange)?;
    Ok(shapes.into_iter().flatten().collect())
}

/// `paths` without the outlines that are on average narrower than the flattening tolerance.
///
/// Where an inset makes two curved outlines meet (a ring shrunk by half its width), the chords of
/// the two flattened curves do not coincide and leave slivers a few thousandths of a millimetre
/// wide (up to twice the tolerance at their thickest, as both chords may deviate by it). The
/// flattening is only exact to its tolerance, so a part that thin is not a part of the exact result: the spec counts that ring as vanished (criterion 16). The width estimate is twice
/// the area over the perimeter, which is half the side for a square: a square inset to a side of
/// 0.02 mm (criterion 15) is 0.01 mm wide by it and stays (the tolerance is 0.008 mm).
pub(crate) fn without_flattening_noise(mut paths: Paths64, flatten_tolerance_mm: f64) -> Paths64 {
    let threshold_units = flatten_tolerance_mm / GRID_MM;
    paths.retain(|path| {
        let mut perimeter = 0.0_f64;
        let Some(&last) = path.last() else {
            return false;
        };
        let mut previous = last;
        for point in path {
            // IEEE `sqrt` of a sum of squares, not `hypot` (criterion 12). Exact below 2^53.
            #[allow(clippy::cast_precision_loss)]
            let (dx, dy) = ((point.x - previous.x) as f64, (point.y - previous.y) as f64);
            perimeter += (dx * dx + dy * dy).sqrt();
            previous = *point;
        }
        #[allow(clippy::cast_precision_loss)]
        let double_area = double_area(path).unsigned_abs() as f64;
        double_area >= threshold_units * perimeter
    });
    paths
}

/// The library's join for `join` at radius `radius_mm`.
pub(crate) fn int_join(join: OffsetJoin, radius_mm: f64, flatten_tolerance_mm: f64) -> IntLineJoin {
    match join {
        OffsetJoin::Round => IntLineJoin::Round(arc_options(radius_mm, flatten_tolerance_mm)),
        OffsetJoin::Miter => IntLineJoin::Miter(MITER_LIMIT_ANGLE),
        OffsetJoin::Bevel => IntLineJoin::Bevel,
    }
}

/// Arc steps that keep every chord within `tolerance_mm` of an arc of radius `radius_mm`.
///
/// A chord over the angle θ deviates from its arc by `r (1 - cos(θ/2)) ≤ r θ² / 8`, so
/// `θ = 2 sqrt(2 t / r)` keeps it within `t`. That is within 12 % of the exact largest step for
/// the tolerances used, so the node count stays inside criterion 11's budget, and it needs only
/// `sqrt`. The library clamps the step to 0.35°..45°.
pub(crate) fn arc_options(radius_mm: f64, tolerance_mm: f64) -> ArcOptions {
    let step_rad = 2.0 * (2.0 * tolerance_mm / radius_mm).sqrt();
    ArcOptions {
        max_step: angle_of(step_rad, ArcOptions::MAX_STEP),
        ..ArcOptions::default()
    }
}

/// The near-straight cutoff of miters: below it the library closes a turn with a straight edge,
/// which misses the miter by up to `r θ² / 8`. `min(5°, sqrt(8 x 0.005 mm / r))` keeps that within
/// 0.005 mm at any distance (`adrs.md`, "near-straight miter cutoff").
pub(crate) fn miter_min_turn(radius_mm: f64) -> Angle {
    let limit_rad = (8.0 * MITER_STRAIGHT_ERROR_MM / radius_mm).sqrt();
    angle_of(
        limit_rad.min(MITER_MIN_TURN_DEFAULT_RAD),
        Angle::QUARTER_TURN,
    )
}

/// `radians` as a binary angle, at most `cap` (for an angle that is huge or not a number).
fn angle_of(radians: f64, cap: Angle) -> Angle {
    let units = radians / std::f64::consts::TAU * UNITS_PER_TURN;
    // Below `cap` (≤ 2³⁰) after the comparison, and not negative, so the cast is exact up to
    // rounding; NaN fails the comparison and takes the cap.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    if units >= 0.0 && units < f64::from(cap.bits()) {
        Angle::from_bits(units.round() as u32)
    } else {
        cap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_miter_limit_constant_is_two_arcsines_of_a_quarter() {
        let expected = 2.0 * 0.25_f64.asin() / std::f64::consts::TAU * UNITS_PER_TURN;
        assert!((f64::from(MITER_LIMIT_ANGLE.bits()) - expected).abs() <= 0.5);
    }

    #[test]
    fn arc_steps_keep_the_chord_within_the_tolerance() {
        for radius in [0.5, 2.0, 5.0, 50.0, 1000.0] {
            let step = f64::from(arc_options(radius, 0.008).max_step.bits()) / UNITS_PER_TURN
                * std::f64::consts::TAU;
            let sagitta = radius * (1.0 - (step / 2.0).cos());
            assert!(sagitta <= 0.008 + 1e-9, "radius {radius}: {sagitta}");
        }
    }

    #[test]
    fn the_miter_cutoff_shrinks_with_the_distance() {
        assert_eq!(
            miter_min_turn(1.0),
            angle_of(MITER_MIN_TURN_DEFAULT_RAD, Angle::QUARTER_TURN)
        );
        let at_fifty =
            f64::from(miter_min_turn(50.0).bits()) / UNITS_PER_TURN * std::f64::consts::TAU;
        assert!(50.0 * at_fifty * at_fifty / 8.0 <= 0.005 + 1e-9);
    }

    #[test]
    fn a_nan_angle_takes_the_cap_and_an_empty_ring_is_noise() {
        assert_eq!(angle_of(f64::NAN, Angle::QUARTER_TURN), Angle::QUARTER_TURN);
        assert_eq!(
            without_flattening_noise(vec![Vec::new()], 0.008),
            Paths64::new()
        );
    }
}
