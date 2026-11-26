//! The fillets where a bridge meets the material it joins (`specs/0156-fused-lettering`
//! criterion 16c, `adrs.md` decision 3 step 8): a morphological closing of the material and the
//! band on a local box, with round joins of at most 10 degrees per arc step, kept near the band.

use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::i_float::int::angle::Angle;
use i_overlay::mesh::int::arc::ArcOptions;
use i_overlay::mesh::int::outline::offset::IntOutlineOffset;
use i_overlay::mesh::int::style::{IntLineJoin, IntOutlineStyle};
use i_overlay::mesh::math::MathMode;

use crate::boolean_grid::{Paths64, normalize, normalize_shapes, run};
use crate::stack_regions::{GridBox, has_area};

/// 10 degrees in the library's binary angle units, rounded down (2³² / 36).
const TEN_DEGREES: Angle = Angle::from_bits(119_304_647);

/// `region` moved outward by `units` (inward when negative) with round joins of at most 10
/// degrees per step. Empty when the region vanishes.
pub(crate) fn grown(region: &Paths64, units: i64) -> Paths64 {
    if region.is_empty() {
        return Paths64::new();
    }
    let style = IntOutlineStyle::new(units)
        .line_join(IntLineJoin::Round(ArcOptions {
            max_step: TEN_DEGREES,
            ..ArcOptions::default()
        }))
        .math(MathMode::Integer);
    region
        .outline(&style)
        .map(|shapes| shapes.into_iter().flatten().collect())
        .unwrap_or_default()
}

/// The closing of `region` by `radius` units: what a disc of that radius cannot reach from
/// outside.
fn closed(region: &Paths64, radius: i64) -> Paths64 {
    grown(&grown(region, radius), -radius)
}

/// The components of `raw` that share area with `band`, as one set of contours.
fn touching(raw: &Paths64, band: &Paths64) -> Paths64 {
    normalize_shapes(raw)
        .into_iter()
        .filter(|component| has_area(&run(OverlayRule::Intersect, component, band)))
        .flatten()
        .collect()
}

/// The local box of a band: its box grown by four radii, as a polygon.
fn local_box(band: &Paths64, radius: i64) -> Paths64 {
    vec![GridBox::of(band).grown(radius.saturating_mul(4)).polygon()]
}

/// The material a Lettering bridge adds: the closing of the two pieces and the band, near the
/// band, minus the union `near` (every piece the closing could reach).
pub(crate) fn added(band: &Paths64, joined: &Paths64, near: &Paths64, radius: i64) -> Paths64 {
    let mut material = joined.clone();
    material.extend(band.iter().cloned());
    let local = run(
        OverlayRule::Intersect,
        &normalize(&material),
        &local_box(band, radius),
    );
    let reach = grown(band, radius.saturating_mul(2));
    let kept = run(OverlayRule::Intersect, &closed(&local, radius), &reach);
    let raw = run(OverlayRule::Difference, &kept, near);
    touching(&raw, band)
}

/// The material a Stencil bridge leaves standing: the closing of the sheet and the band, near
/// the band, inside the lettering `near`.
pub(crate) fn standing(band: &Paths64, near: &Paths64, radius: i64) -> Paths64 {
    let frame = local_box(band, radius);
    let mut sheet = run(OverlayRule::Difference, &frame, near);
    sheet.extend(band.iter().cloned());
    let reach = grown(band, radius.saturating_mul(2));
    let kept = run(
        OverlayRule::Intersect,
        &closed(&normalize(&sheet), radius),
        &reach,
    );
    let raw = run(OverlayRule::Intersect, &kept, near);
    touching(&raw, band)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boolean_grid::{Point64, double_area};

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<Point64> {
        vec![
            Point64::new(x0, y0),
            Point64::new(x1, y0),
            Point64::new(x1, y1),
            Point64::new(x0, y1),
        ]
    }

    fn area(paths: &Paths64) -> i128 {
        paths.iter().map(|p| double_area(p)).sum::<i128>() / 2
    }

    /// A neck of 1 mm across a 3 mm gap between two blocks gets fillets: more than the neck, less than the neck
    /// plus four unrounded corners.
    #[test]
    #[allow(clippy::cast_precision_loss)] // small areas
    fn a_neck_between_two_blocks_gets_fillets() {
        let pieces = vec![rect(0, 0, 10_000, 10_000), rect(0, 13_000, 10_000, 23_000)];
        let band = vec![rect(4_500, 9_000, 5_500, 14_000)];
        let added = added(&band, &pieces, &normalize(&pieces), 1_000);
        let neck = 1_000 * 3_000;
        let square_corners = 4 * 1_000 * 1_000;
        let a = area(&added);
        assert!(a > neck && a < neck + square_corners, "{a}");
        // A quarter disc's complement per corner: 1 - pi/4.
        let expected = neck as f64 + 4.0 * (1.0 - std::f64::consts::FRAC_PI_4) * 1e6;
        assert!(
            (a as f64 - expected).abs() < 0.01 * expected,
            "{a} {expected}"
        );
    }
}
