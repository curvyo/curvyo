//! A stored trace mask as kernel regions in document millimetres (ADR 0061 §2): each region's
//! corners mapped by the image's placement, one operand per region, added ones united and
//! subtracted ones taken out.

use curvyo_document_core::{Affine, Point, TraceMaskPart, TraceMaskRegion, UnitPoint, Vec2};

use crate::trace_region::{RegionOperand, TraceRegion, TraceRegionError};

/// `p` mapped by `map` with a plain multiply and add per term (ADR 0061 §10: no fused operation,
/// so every target computes the same bits).
#[must_use]
pub fn unit_to_mm(map: Affine, p: UnitPoint) -> Point {
    Point::new(
        map.a * p.u + map.c * p.v + map.e,
        map.b * p.u + map.d * p.v + map.f,
    )
}

/// The region's outline in document millimetres: a rectangle's four mapped corners (a
/// parallelogram under skew), a lasso's mapped corners.
#[must_use]
pub fn region_outline_mm(region: &TraceMaskRegion, map: Affine) -> Vec<Point> {
    region
        .corners()
        .into_iter()
        .map(|p| unit_to_mm(map, p))
        .collect()
}

/// The kernel operand of one region: its closed outline with straight sides.
#[must_use]
pub fn region_operand(region: &TraceMaskRegion, map: Affine) -> RegionOperand {
    let anchors = region_outline_mm(region, map)
        .into_iter()
        .map(|p| (p, Vec2::ZERO, Vec2::ZERO))
        .collect();
    vec![(anchors, true)]
}

/// The operands of `part`'s added and of its subtracted regions, in stored order.
#[must_use]
pub fn part_operands(
    part: &TraceMaskPart,
    map: Affine,
) -> (Vec<RegionOperand>, Vec<RegionOperand>) {
    let mut added = Vec::new();
    let mut subtracted = Vec::new();
    for region in &part.regions {
        let operand = region_operand(region, map);
        if region.subtract {
            subtracted.push(operand);
        } else {
            added.push(operand);
        }
    }
    (added, subtracted)
}

/// The mask of `part` over an image placed by `map`, Inside or Outside as stored.
///
/// # Errors
/// [`TraceRegionError::NoArea`] when the added regions minus the subtracted ones enclose nothing;
/// [`TraceRegionError::Kernel`] when the kernel refuses for another reason.
pub fn part_region(part: &TraceMaskPart, map: Affine) -> Result<TraceRegion, TraceRegionError> {
    let (added, subtracted) = part_operands(part, map);
    TraceRegion::of_signed_operands(&added, &subtracted, part.outside)
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::TraceRegionKind;

    use super::*;

    fn rect(id: &str, order: i64, subtract: bool, a: (f64, f64), b: (f64, f64)) -> TraceMaskRegion {
        TraceMaskRegion {
            id: id.into(),
            order,
            kind: TraceRegionKind::Rect,
            subtract,
            points: vec![UnitPoint::new(a.0, a.1), UnitPoint::new(b.0, b.1)],
        }
    }

    fn part(regions: Vec<TraceMaskRegion>) -> TraceMaskPart {
        TraceMaskPart {
            id: "0".into(),
            order: 0,
            outside: false,
            settings: curvyo_document_core::TracePartSettings::new(),
            regions,
        }
    }

    fn area(region: &TraceRegion) -> f64 {
        region
            .polygons()
            .iter()
            .map(|p| curvyo_geometry_core::signed_area_mm2(p))
            .sum::<f64>()
            .abs()
    }

    const M1: Affine = Affine {
        a: 100.0,
        b: 0.0,
        c: 0.0,
        d: 100.0,
        e: 0.0,
        f: 0.0,
    };

    /// Spec Terms and criterion 8: the mask is the added union minus the subtracted union,
    /// whichever was drawn first; overlaps count once.
    #[test]
    fn subtracted_regions_are_taken_out_whatever_the_order() {
        let r1 = rect("a", 0, false, (0.0, 0.0), (0.5, 1.0));
        let r2 = rect("b", 1, false, (0.25, 0.0), (0.75, 1.0));
        let cut = rect("c", 2, true, (0.0, 0.0), (1.0, 0.5));
        let one = part_region(&part(vec![r1.clone(), r2.clone(), cut.clone()]), M1).unwrap();
        let two = part_region(&part(vec![cut.clone(), r2.clone(), r1.clone()]), M1).unwrap();
        assert!((area(&one) - 75.0 * 50.0).abs() < 0.01, "{}", area(&one));
        assert_eq!(one.polygons().len(), two.polygons().len());
        assert!((area(&one) - area(&two)).abs() < 1e-9);
        assert!(one.keeps(Point::new(10.0, 80.0)) && !one.keeps(Point::new(10.0, 20.0)));
        assert_eq!(
            part_region(&part(vec![cut]), M1),
            Err(TraceRegionError::NoArea),
            "only subtracted regions make an empty mask"
        );
    }

    /// Criterion 13's premise and ADR 0061 §2: a rectangle follows a rotated and skewed placement
    /// as a parallelogram.
    #[test]
    fn a_rectangle_follows_a_turned_and_skewed_placement() {
        let map = Affine::new(86.6, 50.0, -40.0, 86.6, 20.0, 10.0);
        let r = rect("a", 0, false, (0.0, 0.0), (1.0, 1.0));
        let outline = region_outline_mm(&r, map);
        assert_eq!(outline[0], Point::new(20.0, 10.0));
        assert_eq!(
            outline[2],
            Point::new(86.6 - 40.0 + 20.0, 50.0 + 86.6 + 10.0)
        );
        let region = part_region(&part(vec![r]), map).unwrap();
        let det = 86.6f64.mul_add(86.6, 40.0 * 50.0);
        assert!((area(&region) - det).abs() < 0.05, "{}", area(&region));
    }

    fn lasso(id: &str, order: i64, subtract: bool, corners: &[(f64, f64)]) -> TraceMaskRegion {
        TraceMaskRegion {
            id: id.into(),
            order,
            kind: TraceRegionKind::Lasso,
            subtract,
            points: corners.iter().map(|&(u, v)| UnitPoint::new(u, v)).collect(),
        }
    }

    /// Criterion 10's premise: L1 (corners (0, 0), (60, 0), (0, 100) mm on M1) is the triangle
    /// closed by its slanted side; Inside keeps its area and Outside the rest.
    #[test]
    fn a_lasso_is_its_corners_closed_by_a_straight_side() {
        let l1 = lasso("a", 0, false, &[(0.0, 0.0), (0.6, 0.0), (0.0, 1.0)]);
        let region = part_region(&part(vec![l1]), M1).unwrap();
        assert!(
            (area(&region) - 60.0 * 100.0 / 2.0).abs() < 0.01,
            "{}",
            area(&region)
        );
        assert!(region.keeps(Point::new(10.0, 50.0)));
        assert!(
            !region.keeps(Point::new(40.0, 50.0)),
            "beyond the slanted side"
        );
        let mut outside = part(vec![lasso(
            "a",
            0,
            false,
            &[(0.0, 0.0), (0.6, 0.0), (0.0, 1.0)],
        )]);
        outside.outside = true;
        let out = part_region(&outside, M1).unwrap();
        assert!(out.outside() && !out.keeps(Point::new(10.0, 50.0)));
        assert!(out.keeps(Point::new(40.0, 50.0)));
    }

    /// Criterion 23: a lasso that crosses itself paints by non-zero winding: a figure 8 covers
    /// both lobes, a pentagram its centre too (wound twice), and the result is the same bits each
    /// time.
    #[test]
    fn a_self_crossing_lasso_paints_by_non_zero_winding() {
        let eight = lasso(
            "a",
            0,
            false,
            &[(0.0, 0.0), (1.0, 1.0), (1.0, 0.0), (0.0, 1.0)],
        );
        let region = part_region(&part(vec![eight.clone()]), M1).unwrap();
        assert!(region.keeps(Point::new(10.0, 50.0)) && region.keeps(Point::new(90.0, 50.0)));
        assert!(!region.keeps(Point::new(50.0, 10.0)));
        assert!(
            (area(&region) - 2.0 * 2500.0).abs() < 0.01,
            "{}",
            area(&region)
        );
        let star = lasso(
            "b",
            1,
            false,
            &[(0.5, 0.0), (0.8, 1.0), (0.0, 0.35), (1.0, 0.35), (0.2, 1.0)],
        );
        let filled = part_region(&part(vec![star.clone()]), M1).unwrap();
        assert!(
            filled.keeps(Point::new(50.0, 50.0)),
            "non-zero, not even-odd"
        );
        let again = part_region(&part(vec![star]), M1).unwrap();
        assert_eq!(filled.to_numbers(), again.to_numbers());
        // A subtracted figure 8 takes out both lobes and nothing above the crossing.
        let whole = rect("c", 2, false, (0.0, 0.0), (1.0, 1.0));
        let cut = TraceMaskRegion {
            subtract: true,
            ..eight
        };
        let left = part_region(&part(vec![whole, cut]), M1).unwrap();
        assert!(left.keeps(Point::new(50.0, 10.0)) && !left.keeps(Point::new(10.0, 50.0)));
    }

    /// ADR 0061 §2: a lasso whose corners lie on one line has no area and is dropped; nothing
    /// left is an empty mask.
    #[test]
    fn a_flat_lasso_is_dropped() {
        let flat = lasso("a", 0, false, &[(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)]);
        assert_eq!(
            part_region(&part(vec![flat.clone()]), M1),
            Err(TraceRegionError::NoArea)
        );
        let r = rect("b", 1, false, (0.0, 0.0), (0.5, 1.0));
        let kept = part_region(&part(vec![flat, r]), M1).unwrap();
        assert!((area(&kept) - 5000.0).abs() < 0.01);
    }

    /// Criterion 11: the same stored regions give the same polygons, bit for bit.
    #[test]
    fn the_same_regions_give_the_same_bits() {
        let regions = vec![
            rect("a", 0, false, (0.1, 0.1), (0.6, 0.7)),
            rect("b", 1, true, (0.3, 0.3), (0.4, 0.9)),
        ];
        let one = part_region(&part(regions.clone()), M1).unwrap();
        let two = part_region(&part(regions), M1).unwrap();
        assert_eq!(one.to_numbers(), two.to_numbers());
    }
}
