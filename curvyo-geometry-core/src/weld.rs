//! Weld's kernel (`specs/0068-laserable-lettering`, `adrs.md` decisions 1, 2, 6): the union of
//! every operand's contribution, closed by the gap tolerance, as one region with its piece count.
//!
//! On the 0.001 mm grid: `U` is the n-ary union of Boolean Union; the gap closing grows `U` by
//! g/2 and shrinks it by g/2 with round joins (`i_overlay`'s integer offsetter, as Offset uses it)
//! and joins `U` back in, so `U` lies inside the result exactly and convex corners stay as they
//! are. Cleanup and the canonical form run once at the end. Integer arithmetic and `sqrt` only,
//! so the result is the same on every target.

use curvyo_document_core::{Length, Tolerance};
use i_overlay::core::overlay_rule::OverlayRule;

use crate::Outline;
use crate::boolean::{BooleanError, BooleanResult, grid_paths, outline_in_range, union_region};
use crate::boolean_cleanup::cleanup;
use crate::boolean_grid::{GRID_MM, Paths64, canonical_millimetres, normalize, run};
use crate::offset::OffsetJoin;
use crate::offset_polyline::offset_region;
use crate::stack_regions::connected_pieces;
use crate::stroke_band::{ARC_TOLERANCE_MM, StrokeBand, stroke_band};

/// One operand of a Weld: the outlines whose region it contributes and the open outlines whose
/// stroke band it contributes. The caller decides per outline (`adrs.md` decision 3).
#[derive(Debug, Clone, Copy)]
pub struct WeldOperand<'a> {
    /// Outlines whose painted region (nonzero rule) counts; an open one is closed by its straight
    /// closing line, as Union reads it.
    pub regions: &'a [Outline<'a>],
    /// Open outlines whose stroke band counts.
    pub banded: &'a [Outline<'a>],
    /// The stroke of `banded`; `None` contributes no band.
    pub band: Option<StrokeBand>,
}

/// The result of [`weld`]: one region in the canonical form of the boolean kernel, and how many
/// pieces it falls into (every outline of even depth with the holes directly inside it).
#[derive(Debug, Clone, PartialEq)]
pub struct Weld {
    /// The outlines of the region.
    pub result: BooleanResult,
    /// How many separate pieces the region has; an island in a hole is a piece of its own.
    pub pieces: usize,
}

/// Welds `operands`: the union of their contributions, gap-closed by `gap`.
///
/// `tolerance` is the kernel tolerance of [`crate::boolean`]. A negative or non-finite gap counts
/// as zero; a gap under one grid pitch (0.001 mm) skips the closing, so the result is the Union region.
/// An operand that contributes no area is ignored; only an empty union is refused.
///
/// # Errors
/// [`BooleanError::NoOperands`], [`BooleanError::ToleranceTooSmall`],
/// [`BooleanError::OutOfRange`] with every offending operand, or [`BooleanError::EmptyResult`]
/// when the contributions enclose no area together.
pub fn weld(
    operands: &[WeldOperand<'_>],
    gap: Length,
    tolerance: Tolerance,
) -> Result<Weld, BooleanError> {
    if operands.is_empty() {
        return Err(BooleanError::NoOperands);
    }
    let flatten_tolerance_mm = tolerance.as_mm() - 2.0 * GRID_MM;
    if !(flatten_tolerance_mm > 0.0 && flatten_tolerance_mm.is_finite()) {
        return Err(BooleanError::ToleranceTooSmall);
    }
    let united = union_region(&contributions(operands, flatten_tolerance_mm)?);
    if united.is_empty() {
        return Err(BooleanError::EmptyResult);
    }
    let units = gap_units(gap);
    let region = if units > 0 {
        close_gaps(&united, units)
            .ok_or_else(|| BooleanError::OutOfRange((0..operands.len()).collect()))?
    } else {
        united
    };
    let outlines = canonical_millimetres(&cleanup(&region));
    if outlines.is_empty() {
        return Err(BooleanError::EmptyResult);
    }
    let pieces = connected_pieces(outlines.clone()).len();
    Ok(Weld {
        result: BooleanResult { outlines },
        pieces,
    })
}

/// Every operand's region and band on the grid, the empty ones left out.
///
/// # Errors
/// [`BooleanError::OutOfRange`] with every operand that has an outline out of range, or else
/// with every operand whose band leaves the engine's range.
fn contributions(
    operands: &[WeldOperand<'_>],
    flatten_tolerance_mm: f64,
) -> Result<Vec<Paths64>, BooleanError> {
    let out_of_range: Vec<usize> = operands
        .iter()
        .enumerate()
        .filter(|(_, operand)| {
            operand
                .regions
                .iter()
                .chain(operand.banded)
                .any(|outline| !outline_in_range(outline))
        })
        .map(|(index, _)| index)
        .collect();
    if !out_of_range.is_empty() {
        return Err(BooleanError::OutOfRange(out_of_range));
    }
    let mut bad_bands = Vec::new();
    let mut contributions: Vec<Paths64> = Vec::with_capacity(operands.len());
    for (index, operand) in operands.iter().enumerate() {
        let region = normalize(&grid_paths(operand.regions, flatten_tolerance_mm));
        if !region.is_empty() {
            contributions.push(region);
        }
        if let Some(band) = operand.band {
            match stroke_band(operand.banded, band, flatten_tolerance_mm) {
                Ok(paths) if !paths.is_empty() => contributions.push(paths),
                Ok(_) => {}
                Err(_) => bad_bands.push(index),
            }
        }
    }
    if bad_bands.is_empty() {
        Ok(contributions)
    } else {
        Err(BooleanError::OutOfRange(bad_bands))
    }
}

/// Half the gap in whole grid units: the radius of the closing; 0 skips it.
fn gap_units(gap: Length) -> i64 {
    let half_units = gap.as_mm() / 2.0 / GRID_MM;
    if half_units.is_finite() && half_units >= 0.5 {
        // Below 10^10 for any gap the editor allows; a larger one saturates and is refused by the
        // offsetter's bounds check.
        #[allow(clippy::cast_possible_truncation)]
        let units = half_units.round() as i64;
        units
    } else {
        0
    }
}

/// The morphological closing of `region` by `units`, joined with `region`; `None` when an offset
/// leaves the engine's range.
fn close_gaps(region: &Paths64, units: i64) -> Option<Paths64> {
    let grown = offset_region(region, units, OffsetJoin::Round, ARC_TOLERANCE_MM).ok()?;
    let closed = offset_region(
        &normalize(&grown),
        -units,
        OffsetJoin::Round,
        ARC_TOLERANCE_MM,
    )
    .ok()?;
    Some(run(OverlayRule::Union, &closed, region))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boolean_grid::snap_polygon;
    use crate::signed_area_mm2;
    use crate::visible_region::tests::disc;
    use curvyo_document_core::{LineCap, LineJoin, Point, Vec2};

    use crate::OutlineTriple;

    const TOLERANCE: Tolerance = Tolerance::from_mm(0.01);

    fn corners(points: &[(f64, f64)]) -> Vec<OutlineTriple> {
        points
            .iter()
            .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
            .collect()
    }

    fn square(x: f64, y: f64, side: f64) -> Vec<OutlineTriple> {
        corners(&[(x, y), (x + side, y), (x + side, y + side), (x, y + side)])
    }

    fn moved(anchors: &[OutlineTriple], dx: f64, dy: f64) -> Vec<OutlineTriple> {
        anchors
            .iter()
            .map(|&(p, a, b)| (Point::new(p.x + dx, p.y + dy), a, b))
            .collect()
    }

    fn reversed(anchors: &[OutlineTriple]) -> Vec<OutlineTriple> {
        anchors.iter().rev().map(|&(p, a, b)| (p, b, a)).collect()
    }

    /// Welds closed shapes, each one operand of one or more outlines.
    fn weld_shapes(shapes: &[Vec<Vec<OutlineTriple>>], gap_mm: f64) -> Weld {
        let outlines: Vec<Vec<Outline<'_>>> = shapes
            .iter()
            .map(|shape| shape.iter().map(|a| Outline::new(a, true)).collect())
            .collect();
        let operands: Vec<WeldOperand<'_>> = outlines
            .iter()
            .map(|regions| WeldOperand {
                regions,
                banded: &[],
                band: None,
            })
            .collect();
        weld(&operands, Length::from_mm(gap_mm), TOLERANCE).unwrap()
    }

    fn area(weld: &Weld) -> f64 {
        weld.result
            .outlines()
            .iter()
            .map(|o| signed_area_mm2(o))
            .sum()
    }

    /// Criterion 4: two overlapping squares at g = 0 give Union's 8-node path of 700 mm².
    #[test]
    fn ac04_two_squares_weld_to_unions_result() {
        let weld = weld_shapes(
            &[vec![square(0.0, 0.0, 20.0)], vec![square(10.0, 10.0, 20.0)]],
            0.0,
        );
        assert_eq!(weld.result.outlines().len(), 1);
        assert_eq!(weld.result.outlines()[0].len(), 8);
        assert!((area(&weld) - 700.0).abs() < 1e-6);
        assert_eq!(weld.pieces, 1);
    }

    /// Criterion 5: a ring and a bar keep the ring's hole; a disc inside the hole stays an island,
    /// which is a piece of its own.
    #[test]
    fn ac05_the_hole_is_kept_and_an_island_stays() {
        let ring = vec![disc(20.0), reversed(&disc(10.0))];
        let bar = vec![
            square(0.0, 0.0, 4.0)
                .iter()
                .map(|&(p, a, b)| (Point::new(p.x * 12.5 - 25.0, p.y - 18.0), a, b))
                .collect(),
        ];
        let island = vec![disc(5.0)];
        let two = weld_shapes(&[ring.clone(), bar.clone()], 0.1);
        assert_eq!(two.result.outlines().len(), 2);
        let hole = two
            .result
            .outlines()
            .iter()
            .map(|o| signed_area_mm2(o))
            .fold(0.0, f64::min);
        assert!(
            (hole + std::f64::consts::PI * 100.0).abs() < 0.5,
            "hole {hole}"
        );
        let three = weld_shapes(&[ring, bar, island], 0.1);
        assert_eq!(three.result.outlines().len(), 3);
        assert_eq!(three.pieces, 2);
    }

    /// Criterion 6: four bars laid as a frame enclose a 20 x 20 mm hole.
    #[test]
    fn ac06_an_area_enclosed_between_shapes_is_a_hole() {
        let bar = |x: f64, y: f64, w: f64, h: f64| {
            vec![corners(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)])]
        };
        let weld = weld_shapes(
            &[
                bar(0.0, 10.0, 40.0, 4.0),
                bar(0.0, 34.0, 40.0, 4.0),
                bar(6.0, 0.0, 4.0, 40.0),
                bar(30.0, 0.0, 4.0, 40.0),
            ],
            0.0,
        );
        assert_eq!(weld.result.outlines().len(), 2);
        let hole = weld
            .result
            .outlines()
            .iter()
            .map(|o| signed_area_mm2(o))
            .fold(0.0, f64::min);
        assert!((hole + 400.0).abs() < 1e-6, "hole {hole}");
        assert_eq!(weld.pieces, 1);
    }

    /// Criterion 7: a shared edge merges into a 4-node rectangle; a shared corner is two pieces at
    /// g = 0 and one at g = 0.1 mm.
    #[test]
    fn ac07_shared_edges_merge_and_corners_join_with_a_gap() {
        let edge = weld_shapes(
            &[vec![square(0.0, 0.0, 20.0)], vec![square(20.0, 0.0, 20.0)]],
            0.0,
        );
        assert_eq!(edge.result.outlines().len(), 1);
        assert_eq!(edge.result.outlines()[0].len(), 4);
        let corner = [vec![square(0.0, 0.0, 20.0)], vec![square(20.0, 20.0, 20.0)]];
        assert_eq!(weld_shapes(&corner, 0.0).pieces, 2);
        assert_eq!(weld_shapes(&corner, 0.1).pieces, 1);
    }

    /// Criterion 8: a gap of 0.05 mm closes at g = 0.1 mm; one of 0.15 mm does not; g = 0 closes
    /// neither.
    #[test]
    fn ac08_gaps_narrower_than_g_close() {
        let pair = |gap: f64| {
            [
                vec![square(0.0, 0.0, 20.0)],
                vec![square(20.0 + gap, 0.0, 20.0)],
            ]
        };
        assert_eq!(weld_shapes(&pair(0.05), 0.1).pieces, 1);
        assert_eq!(weld_shapes(&pair(0.15), 0.1).pieces, 2);
        assert_eq!(weld_shapes(&pair(0.05), 0.0).pieces, 2);
        assert_eq!(weld_shapes(&pair(0.15), 0.0).pieces, 2);
    }

    /// Criterion 9: a 2 mm round-capped line welds with a 10 mm square into one outline of the
    /// band's and the square's area.
    #[test]
    fn ac09_an_open_line_contributes_its_stroke_band() {
        let line = corners(&[(0.0, 0.0), (40.0, 0.0)]);
        let square = square(15.0, -5.0, 10.0);
        let open = [Outline::new(&line, false)];
        let closed = [Outline::new(&square, true)];
        let operands = [
            WeldOperand {
                regions: &[],
                banded: &open,
                band: Some(StrokeBand {
                    width: Length::from_mm(2.0),
                    join: LineJoin::Miter,
                    cap: LineCap::Round,
                }),
            },
            WeldOperand {
                regions: &closed,
                banded: &[],
                band: None,
            },
        ];
        let weld = weld(&operands, Length::from_mm(0.0), TOLERANCE).unwrap();
        assert_eq!(weld.result.outlines().len(), 1);
        let expected = 40.0 * 2.0 + std::f64::consts::PI + 100.0 - 10.0 * 2.0;
        assert!(
            (area(&weld) - expected).abs() < 0.05,
            "{} vs {expected}",
            area(&weld)
        );
    }

    /// Criterion 16: a convex operand alone keeps its outline within 0.01 mm for every g.
    #[test]
    fn ac16_a_convex_operand_keeps_its_outline_for_every_gap() {
        for gap in [0.0, 0.05, 0.1, 0.5, 1.0] {
            let weld = weld_shapes(&[vec![square(0.0, 0.0, 20.0)]], gap);
            assert!(
                (area(&weld) - 400.0).abs() < 0.01,
                "g {gap}: {}",
                area(&weld)
            );
            for point in weld.result.outlines().iter().flatten() {
                let inside =
                    (-0.01..=20.01).contains(&point.x) && (-0.01..=20.01).contains(&point.y);
                assert!(inside, "g {gap}: {point:?}");
            }
            let disc = weld_shapes(&[vec![disc(10.0)]], gap);
            for point in disc.result.outlines().iter().flatten() {
                let r = (point.x * point.x + point.y * point.y).sqrt();
                assert!((r - 10.0).abs() <= 0.01, "g {gap}: r {r}");
            }
        }
    }

    /// Criterion 17: a slot narrower than g is filled, a wider one stays.
    #[test]
    fn ac17_narrow_slots_fill_and_wide_ones_stay() {
        let slotted = |w: f64| {
            vec![corners(&[
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 5.0),
                (10.0 + w, 5.0),
                (10.0 + w, 0.0),
                (20.0, 0.0),
                (20.0, 20.0),
                (0.0, 20.0),
            ])]
        };
        assert!((area(&weld_shapes(&[slotted(0.08)], 0.1)) - 400.0).abs() < 0.01);
        assert!((area(&weld_shapes(&[slotted(0.2)], 0.1)) - 399.0).abs() < 0.01);
    }

    /// Criterion 15 (property, 200 random sets, fixed seed): the Union region lies inside the
    /// result, and the result within g/2 + 0.01 mm of the Union region.
    #[test]
    fn ac15_the_result_holds_the_union_and_stays_within_half_the_gap() {
        let mut seed: u64 = 0x5eed_0068;
        let mut next = || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            #[allow(clippy::cast_precision_loss)]
            let unit = (seed >> 11) as f64 / (1_u64 << 53) as f64;
            unit
        };
        for case in 0..200 {
            let gap = [0.0, 0.05, 0.1, 0.5, 1.0][case % 5];
            let count = 2 + (case % 4);
            let shapes: Vec<Vec<Vec<OutlineTriple>>> = (0..count)
                .map(|_| {
                    let (x, y, w, h) = (
                        next() * 30.0,
                        next() * 30.0,
                        1.0 + next() * 15.0,
                        1.0 + next() * 15.0,
                    );
                    if next() < 0.5 {
                        vec![corners(&[(x, y), (x + w, y), (x + w * 0.5, y + h)])]
                    } else {
                        vec![moved(&disc(w * 0.5), x, y)]
                    }
                })
                .collect();
            let weld = weld_shapes(&shapes, gap);
            let grid = |outlines: &[Vec<Point>]| -> Paths64 {
                normalize(&outlines.iter().map(|o| snap_polygon(o)).collect())
            };
            let result = grid(weld.result.outlines());
            let union = weld_shapes(&shapes, 0.0);
            let union = grid(union.result.outlines());
            let outside =
                crate::stack_regions::has_area(&run(OverlayRule::Difference, &union, &result));
            assert!(!outside, "case {case}: the union is not inside the result");
            #[allow(clippy::cast_possible_truncation)]
            let reach = ((gap / 2.0 + 0.01) / GRID_MM).round() as i64;
            let bound = normalize(&offset_region(&union, reach, OffsetJoin::Round, 0.001).unwrap());
            let beyond =
                crate::stack_regions::has_area(&run(OverlayRule::Difference, &result, &bound));
            assert!(
                !beyond,
                "case {case}: the result reaches beyond g/2 + 0.01 mm"
            );
        }
    }

    /// The anchors are used by the tests above; an empty selection of contributions is refused.
    #[test]
    fn ac28_no_area_is_refused_and_vec2_is_plain() {
        let line = vec![
            (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::ZERO),
            (Point::new(5.0, 0.0), Vec2::ZERO, Vec2::ZERO),
        ];
        let open = [Outline::new(&line, false)];
        let operands = [WeldOperand {
            regions: &open,
            banded: &[],
            band: None,
        }];
        assert_eq!(
            weld(&operands, Length::from_mm(0.1), TOLERANCE),
            Err(BooleanError::EmptyResult)
        );
    }
}
