//! The outline against the same normal-offset construction sampled 64 times as densely, with
//! the same caps and joins (ADR 0053, amendment of 2026-11-10, item 3): the boundaries lie within
//! 0.01 mm of each other (every part of their symmetric difference is at most 0.01 mm thick)
//! and the areas agree.

use super::*;
use crate::WidthPoint;
use curvyo_document_core::{DashPattern, LineCap, LineJoin, Markers};

const TOLERANCE_MM: f64 = 0.01;

fn style(width: f64, cap: LineCap, join: LineJoin) -> StrokeShape {
    StrokeShape {
        width: Length::from_mm(width),
        join,
        cap,
        dash: DashPattern::default(),
        markers: Markers::default(),
    }
}

fn profile(points: &[(f64, f64)]) -> WidthProfile {
    let points = points
        .iter()
        .enumerate()
        .map(|(i, &(at, m))| WidthPoint::new(i as u128 + 1, at, m, false).unwrap())
        .collect();
    WidthProfile::new(points, false).unwrap()
}

fn rings(outlines: &OffsetOutlines) -> Vec<Vec<DocPoint>> {
    outlines
        .outlines()
        .iter()
        .map(|ring| ring.iter().map(|anchor| anchor.point).collect())
        .collect()
}

fn area(rings: &[Vec<DocPoint>]) -> f64 {
    rings.iter().map(|ring| crate::signed_area_mm2(ring)).sum()
}

fn perimeter(rings: &[Vec<DocPoint>]) -> f64 {
    rings
        .iter()
        .map(|ring| {
            (0..ring.len())
                .map(|i| ring[i].vector_to(ring[(i + 1) % ring.len()]).length())
                .sum::<f64>()
        })
        .sum()
}

/// The thickest component of the symmetric difference of `a` and `b`, as twice its area over
/// its perimeter (the width of a strip, the radius of a disc), and the difference's total area.
fn difference(a: &[Vec<DocPoint>], b: &[Vec<DocPoint>]) -> (f64, f64) {
    use crate::boolean_grid::{canonical_millimetres, run, snap_polygon};
    let grid = |rings: &[Vec<DocPoint>]| rings.iter().map(|r| snap_polygon(r)).collect::<Vec<_>>();
    let xor = canonical_millimetres(&run(OverlayRule::Xor, &grid(a), &grid(b)));
    let thickest = xor
        .iter()
        .map(|ring| {
            2.0 * crate::signed_area_mm2(ring).abs() / perimeter(std::slice::from_ref(ring))
        })
        .fold(0.0, f64::max);
    (thickest, area(&xor).abs())
}

fn assert_matches_reference(
    anchors: &[(DocPoint, DocVec, DocVec)],
    style: &StrokeShape,
    profile: &WidthProfile,
) {
    let outline = Outline::new(anchors, false);
    let tolerance = Tolerance::from_mm(TOLERANCE_MM);
    let fast = rings(&outline_with_density(outline, style, profile, tolerance, 1).unwrap());
    let dense = rings(&outline_with_density(outline, style, profile, tolerance, 64).unwrap());
    let (thickest, apart) = difference(&fast, &dense);
    assert!(
        thickest <= TOLERANCE_MM,
        "the boundaries are {thickest} mm apart"
    );
    let allowed = TOLERANCE_MM * perimeter(&dense);
    assert!(
        apart <= allowed,
        "the areas differ by {apart} mm², more than {allowed}"
    );
    assert!(area(&fast) > 0.0);
}

fn corner(x: f64, y: f64) -> (DocPoint, DocVec, DocVec) {
    (DocPoint::new(x, y), DocVec::ZERO, DocVec::ZERO)
}

#[test]
fn a_tight_two_node_arc_matches_the_dense_reference() {
    // A half circle of radius 5 drawn 8 mm wide in the middle: the inner side folds.
    let k = 5.0 * 4.0 / 3.0;
    let anchors = [
        (DocPoint::new(0.0, 0.0), DocVec::ZERO, DocVec::new(0.0, k)),
        (DocPoint::new(10.0, 0.0), DocVec::new(0.0, k), DocVec::ZERO),
    ];
    let profile = profile(&[(0.0, 1.0), (0.5, 4.0), (1.0, 1.0)]);
    assert_matches_reference(
        &anchors,
        &style(3.0, LineCap::Round, LineJoin::Round),
        &profile,
    );
}

#[test]
fn a_kinked_twelve_node_path_matches_the_dense_reference() {
    let anchors: Vec<_> = (0..12)
        .map(|i| {
            let x = 8.0 * f64::from(i);
            let y = if i % 2 == 0 { 0.0 } else { 6.0 };
            (
                DocPoint::new(x, y),
                DocVec::new(-2.0, 2.0),
                DocVec::new(2.0, 2.0),
            )
        })
        .collect();
    let profile = profile(&[(0.0, 0.5), (0.3, 2.0), (0.6, 0.2), (1.0, 1.5)]);
    for join in [LineJoin::Miter, LineJoin::Round, LineJoin::Bevel] {
        assert_matches_reference(&anchors, &style(2.0, LineCap::Square, join), &profile);
    }
}

#[test]
fn a_taper_preset_on_an_s_curve_matches_the_dense_reference() {
    let anchors = [
        (
            DocPoint::new(0.0, 0.0),
            DocVec::ZERO,
            DocVec::new(10.0, 0.0),
        ),
        (
            DocPoint::new(30.0, 20.0),
            DocVec::new(-10.0, 0.0),
            DocVec::new(10.0, 0.0),
        ),
        (
            DocPoint::new(60.0, -20.0),
            DocVec::new(-10.0, 0.0),
            DocVec::new(10.0, 0.0),
        ),
        (
            DocPoint::new(90.0, 0.0),
            DocVec::new(-10.0, 0.0),
            DocVec::ZERO,
        ),
    ];
    let taper_both = profile(&[(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)]);
    assert_matches_reference(
        &anchors,
        &style(4.0, LineCap::Butt, LineJoin::Miter),
        &taper_both,
    );
    let polyline = [corner(0.0, 0.0), corner(20.0, 0.0), corner(20.0, 20.0)];
    assert_matches_reference(
        &polyline,
        &style(4.0, LineCap::Round, LineJoin::Miter),
        &taper_both,
    );
}
