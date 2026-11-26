//! Black-box acceptance tests for `0156-fused-lettering` slices 1 (Lettering, `fuse_cut_out`) and
//! 2 (Stencil, `fuse_stencil`), written from `specification.md` and the public signatures in
//! `adrs.md` before the implementation was read. Criteria about the panel, the Boolean card, the
//! group names, the undo step, the save and reopen and the budget are not here: they belong to
//! the UI and wasm layers and are reported separately.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use common::{Anchors, KERNEL_TOLERANCE, Operand, area, circle, ok, polygon, rect};
use curvyo_document_core::{Length, Point};
use curvyo_geometry_core::{BooleanOp, FuseError, Fused, Outline, fuse_cut_out, fuse_stencil};

const MAX_BRIDGES: usize = 2000;

fn with_operands<T>(operands: &[Operand], run: impl FnOnce(&[&[Outline<'_>]]) -> T) -> T {
    let outlines: Vec<Vec<Outline<'_>>> = operands
        .iter()
        .map(|o| o.iter().map(|a| Outline::new(a, true)).collect())
        .collect();
    let borrowed: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
    run(&borrowed)
}

fn cut(operands: &[Operand], width: f64, reach: f64) -> Result<Fused, FuseError> {
    with_operands(operands, |o| {
        fuse_cut_out(
            o,
            Length::from_mm(width),
            Some(Length::from_mm(reach)),
            MAX_BRIDGES,
            KERNEL_TOLERANCE,
        )
    })
}

fn cut_with_limit(operands: &[Operand], max: usize) -> Result<Fused, FuseError> {
    with_operands(operands, |o| {
        fuse_cut_out(
            o,
            Length::from_mm(1.0),
            Some(Length::from_mm(5.0)),
            max,
            KERNEL_TOLERANCE,
        )
    })
}

fn stencil(operands: &[Operand], width: f64, bridges: u8) -> Result<Fused, FuseError> {
    with_operands(operands, |o| {
        fuse_stencil(
            o,
            Length::from_mm(width),
            bridges,
            MAX_BRIDGES,
            KERNEL_TOLERANCE,
        )
    })
}

/// The hole of a shape: the same anchors walked backwards, so the nonzero rule paints it out.
fn reversed(a: &Anchors) -> Anchors {
    a.iter()
        .rev()
        .map(|&(p, hin, hout)| (p, hout, hin))
        .collect()
}

fn segments(points: &[Point]) -> Vec<(Point, Point)> {
    (0..points.len())
        .map(|i| (points[i], points[(i + 1) % points.len()]))
        .collect()
}

fn signed_area(points: &[Point]) -> f64 {
    segments(points)
        .iter()
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum::<f64>()
        / 2.0
}

fn perimeter(outlines: &[Vec<Point>]) -> f64 {
    outlines
        .iter()
        .flat_map(|o| segments(o))
        .map(|(a, b)| (b.x - a.x).hypot(b.y - a.y))
        .sum()
}

/// Area painted by a set of outlines: the largest outline's sign is taken as the outer one, so
/// holes (wound the other way) subtract.
fn region_area(outlines: &[Vec<Point>]) -> f64 {
    let reference = outlines
        .iter()
        .max_by(|a, b| signed_area(a).abs().total_cmp(&signed_area(b).abs()))
        .map_or(1.0, |o| signed_area(o).signum());
    outlines.iter().map(|o| reference * signed_area(o)).sum()
}

/// Where a horizontal line `y` crosses the outlines, as sorted x values.
fn crossings_at_y(outlines: &[Vec<Point>], y: f64) -> Vec<f64> {
    let mut xs = Vec::new();
    for o in outlines {
        for (a, b) in segments(o) {
            if (a.y - y) * (b.y - y) <= 0.0 && a.y != b.y {
                xs.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
            }
        }
    }
    xs.sort_by(f64::total_cmp);
    xs
}

/// Where a vertical line `x` crosses the outlines, as sorted y values.
fn crossings_at_x(outlines: &[Vec<Point>], x: f64) -> Vec<f64> {
    let mut ys = Vec::new();
    for o in outlines {
        for (a, b) in segments(o) {
            if (a.x - x) * (b.x - x) <= 0.0 && a.x != b.x {
                ys.push(a.y + (x - a.x) * (b.y - a.y) / (b.x - a.x));
            }
        }
    }
    ys.sort_by(f64::total_cmp);
    ys
}

/// Non-zero winding of the outlines around `p`.
fn winding_at(outlines: &[Vec<Point>], p: Point) -> i32 {
    let mut total = 0;
    for o in outlines {
        for (a, b) in segments(o) {
            let side = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
            if a.y <= p.y {
                if b.y > p.y && side > 0.0 {
                    total += 1;
                }
            } else if b.y <= p.y && side < 0.0 {
                total -= 1;
            }
        }
    }
    total
}

fn inside(outlines: &[Vec<Point>], p: Point) -> bool {
    winding_at(outlines, p) != 0
}

/// Whether one outline lies inside another (a vertex of one strictly inside the other): an
/// inner contour in the sense of criterion 24 and 25.
fn has_nested_outline(outlines: &[Vec<Point>]) -> bool {
    for (i, outer) in outlines.iter().enumerate() {
        for (j, inner) in outlines.iter().enumerate() {
            if i != j && !inner.is_empty() && inside(std::slice::from_ref(outer), inner[0]) {
                return true;
            }
        }
    }
    false
}

fn bounds(points: &[Vec<Point>]) -> (f64, f64, f64, f64) {
    let all: Vec<Point> = points.iter().flatten().copied().collect();
    let min_x = all.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = all.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = all.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = all.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    (min_x, min_y, max_x, max_y)
}

fn centre_of(points: &[Vec<Point>]) -> Point {
    let (x0, y0, x1, y1) = bounds(points);
    Point::new(f64::midpoint(x0, x1), f64::midpoint(y0, y1))
}

fn assert_near(actual: f64, expected: f64, tolerance: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: {actual} is not {expected} within {tolerance}"
    );
}

/// Criterion 8 and 12 operand: an i, a 2 x 10 mm stem and a radius 1 mm dot at (1, 13).
fn i_letter() -> Vec<Operand> {
    vec![
        vec![rect(0.0, 0.0, 2.0, 10.0)],
        vec![circle(1.0, 13.0, 1.0)],
    ]
}

/// Criterion 10 stand-in for a u with two dots (no font file yet): three overlapping bars make
/// the u, two dots of radius 0.8 mm sit above its stems.
fn u_with_dots() -> Vec<Operand> {
    vec![
        vec![rect(0.0, 4.0, 2.0, 12.0)],
        vec![rect(6.0, 4.0, 8.0, 12.0)],
        vec![rect(0.0, 10.0, 8.0, 12.0)],
        vec![circle(1.0, 1.5, 0.8)],
        vec![circle(7.0, 1.5, 0.8)],
    ]
}

/// The O of criterion 24: a disc of radius 10 mm with a concentric hole of radius 6 mm.
fn o_ring() -> Vec<Operand> {
    vec![vec![
        circle(0.0, 0.0, 10.0),
        reversed(&circle(0.0, 0.0, 6.0)),
    ]]
}

// ---------------------------------------------------------------------------------------------
// Criterion 8: the i. One connector from (1, 10) to (1, 12), neck 1.0 mm, one outline, area
// 1.5 to 4.0 mm² more than stem plus disc.
// ---------------------------------------------------------------------------------------------

#[test]
fn c08_i_dot_gets_one_connector_on_its_axis_with_neck_one_mm() {
    let fused = cut(&i_letter(), 1.0, 5.0).expect("the i must bridge");
    assert_eq!(fused.connectors.len(), 1, "one connector for the dot");
    let connector: Vec<Vec<Point>> = fused.connectors[0].clone();

    // Centre line from (1, 10) to (1, 12): the band spans exactly y 10 to 12 on the axis.
    let on_axis = crossings_at_x(&connector, 1.0);
    assert!(on_axis.len() >= 2, "axis crossings: {on_axis:?}");
    assert_near(on_axis[0], 10.0, 0.01, "connector starts at the stem top");
    assert_near(
        on_axis[on_axis.len() - 1],
        12.0,
        0.01,
        "connector ends at the dot bottom",
    );

    // Neck: at y = 11 (the middle, outside both fillets of radius 1) the width is 1.0 mm.
    let across = crossings_at_y(&connector, 11.0);
    assert!(across.len() >= 2, "neck crossings: {across:?}");
    let neck = across[across.len() - 1] - across[0];
    assert_near(neck, 1.0, 0.01, "neck width at the middle");
    assert_near(
        f64::midpoint(across[0], across[across.len() - 1]),
        1.0,
        0.01,
        "neck centre x",
    );

    // One closed outline for the whole i.
    assert_eq!(fused.outline.len(), 1, "the Outline is one closed outline");
    let stem_and_disc = area(&ok(BooleanOp::Union, &i_letter()));
    let total = region_area(&fused.outline);
    let added = total - stem_and_disc;
    assert!(
        (1.5..=4.0).contains(&added),
        "added area {added} mm² is outside 1.5 to 4.0 mm² (stem plus disc is {stem_and_disc})"
    );
}

// ---------------------------------------------------------------------------------------------
// Criteria 12 and 13: the Outline is the union of pieces and connectors, the connector does not
// overlap a piece by more than the tolerance, and the sums agree within 0.01 mm per mm of outline.
// ---------------------------------------------------------------------------------------------

#[test]
fn c12_c13_outline_equals_pieces_plus_connectors_within_tolerance() {
    let operands = i_letter();
    let fused = cut(&operands, 1.0, 5.0).expect("the i must bridge");
    let pieces = area(&ok(BooleanOp::Union, &operands));
    let connectors: f64 = fused.connectors.iter().map(|c| region_area(c).abs()).sum();
    let bound = 0.01 * perimeter(&fused.outline) + 0.01;
    let total = region_area(&fused.outline);
    assert!(
        (total - (pieces + connectors)).abs() <= bound,
        "outline {total} vs pieces {pieces} plus connectors {connectors}, bound {bound}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 9: two bars, narrowest gap 3.0 mm, bridge centred at y 6.5 (the middle of y 3 to 10).
// ---------------------------------------------------------------------------------------------

#[test]
fn c09_narrowest_gap_is_bridged_at_the_middle_of_the_tied_stretch() {
    let bars = vec![
        vec![rect(0.0, 0.0, 2.0, 10.0)],
        vec![rect(5.0, 3.0, 7.0, 13.0)],
    ];
    let fused = cut(&bars, 1.0, 5.0).expect("two bars within reach must join");
    assert_eq!(
        fused.connectors.len(),
        1,
        "one connector between the two bars"
    );
    let connector = &fused.connectors[0];

    // Across the gap at x = 3.5: the neck is 1.0 mm wide, centred at y = 6.5.
    let across = crossings_at_x(connector, 3.5);
    assert!(across.len() >= 2, "crossings: {across:?}");
    let (lo, hi) = (across[0], across[across.len() - 1]);
    assert_near(f64::midpoint(lo, hi), 6.5, 0.01, "bridge centre y");
    assert_near(hi - lo, 1.0, 0.01, "neck width across the gap");

    // The bridge sits in the gap: x runs from the first bar to the second.
    let (x0, _, x1, _) = bounds(connector);
    assert!(
        x0 >= 2.0 - 0.01 && x1 <= 5.0 + 0.01,
        "bridge spans x {x0} to {x1}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 10 (stand-in, see report): dots of a u are each joined to the u, never to each other.
// ---------------------------------------------------------------------------------------------

#[test]
fn c10_each_dot_gets_one_connector_to_the_u_and_dots_are_not_joined_to_each_other() {
    let fused = cut(&u_with_dots(), 1.0, 5.0).expect("the u with dots must bridge");
    assert_eq!(fused.connectors.len(), 2, "one connector per dot");

    let mut dot_side = [0usize; 2];
    for connector in &fused.connectors {
        let c = centre_of(connector);
        // Between the dot and the u: below the dot bottom (y 2.3) and above the u top (y 4).
        assert!(
            c.y > 2.3 - 0.01 && c.y < 4.0 + 0.01,
            "connector centre y {} is not between dot and u",
            c.y
        );
        if c.x < 4.0 {
            dot_side[0] += 1;
        } else {
            dot_side[1] += 1;
        }
    }
    assert_eq!(
        dot_side,
        [1, 1],
        "one connector at each dot, none between the dots"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 6 and 17: overlapping operands are one piece (nothing to join), far pieces are
// refused, and degenerate or non-finite operands are refused as 0016 does.
// ---------------------------------------------------------------------------------------------

#[test]
fn c06_17_overlapping_operands_are_one_piece_so_nothing_to_join() {
    let overlap = vec![
        vec![rect(0.0, 0.0, 2.0, 10.0)],
        vec![rect(1.0, 9.0, 3.0, 11.0)],
    ];
    assert_eq!(
        cut(&overlap, 1.0, 5.0).unwrap_err(),
        FuseError::NothingToJoin,
        "two overlapping operands are one piece"
    );
}

#[test]
fn c17_pieces_beyond_reach_are_refused_with_nothing_to_join() {
    let far = vec![
        vec![rect(0.0, 0.0, 2.0, 10.0)],
        vec![rect(30.0, 0.0, 32.0, 10.0)],
    ];
    assert!(matches!(cut(&far, 1.0, 5.0), Err(FuseError::NothingToJoin)));
}

#[test]
fn c17_non_finite_and_empty_operands_are_refused_as_operands_errors() {
    let nan = vec![
        vec![rect(f64::NAN, 0.0, 2.0, 10.0)],
        vec![rect(5.0, 0.0, 7.0, 10.0)],
    ];
    assert!(
        matches!(cut(&nan, 1.0, 5.0), Err(FuseError::Operands(_))),
        "NaN operand"
    );

    let empty = vec![
        vec![rect(0.0, 0.0, 2.0, 0.0)],
        vec![rect(5.0, 0.0, 7.0, 10.0)],
    ];
    assert!(
        matches!(cut(&empty, 1.0, 5.0), Err(FuseError::Operands(_))),
        "zero-area operand"
    );
}

#[test]
fn c17_more_bridges_than_allowed_is_refused_before_any_band_is_built() {
    let row = vec![
        vec![rect(0.0, 0.0, 2.0, 10.0)],
        vec![rect(4.0, 0.0, 6.0, 10.0)],
        vec![rect(8.0, 0.0, 10.0, 10.0)],
    ];
    assert!(matches!(
        cut_with_limit(&row, 1),
        Err(FuseError::TooManyBridges)
    ));
    assert!(
        cut_with_limit(&row, 2).is_ok(),
        "two bridges are allowed at the limit"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 15: three pieces in a row, the middle one narrow: two connectors, never one through
// the middle piece.
// ---------------------------------------------------------------------------------------------

/// Criterion 15 with a middle piece that is a dot (area 2 mm², below a quarter of the largest
/// piece within Reach, 20 / 4 = 5 mm²). Lead decision of 2026-11-26 (Proposal): criterion 15 is
/// meant with a non-dot middle piece; a dot is attached to its nearest non-dot (rule 7a) and never
/// forms a path between two letters. So the dot joins A, the A-C edge through it is obstructed,
/// and C stays apart: one connector, between A and the dot.
#[test]
fn c15_as_written_a_dot_in_the_middle_joins_one_letter_only() {
    let row = vec![
        vec![rect(0.0, 0.0, 2.0, 10.0)],
        vec![rect(4.0, 4.0, 5.0, 6.0)],
        vec![rect(7.0, 0.0, 9.0, 10.0)],
    ];
    let fused = cut(&row, 1.0, 5.0).expect("the dot joins A");
    assert_eq!(fused.connectors.len(), 1, "only the dot's own connector");
    let centre = centre_of(&fused.connectors[0]).x;
    assert!(
        centre > 2.0 && centre < 4.0,
        "between A and the dot: {centre}"
    );
}

/// Criterion 15 with a middle piece that is not a dot (area 7 mm² >= 5 mm²): rule 7b joins the
/// chain A-M and M-C, and the A-C edge through M is obstructed.
#[test]
fn c15_connectors_never_cross_a_third_piece() {
    let row = vec![
        vec![rect(0.0, 0.0, 2.0, 10.0)],
        vec![rect(4.0, 2.0, 5.0, 9.0)],
        vec![rect(7.0, 0.0, 9.0, 10.0)],
    ];
    let fused = cut(&row, 1.0, 5.0).expect("three pieces in a row must join");
    assert_eq!(
        fused.connectors.len(),
        2,
        "two connectors, the chain A-M and M-C"
    );
    let mut centres: Vec<f64> = fused.connectors.iter().map(|c| centre_of(c).x).collect();
    centres.sort_by(f64::total_cmp);
    assert!(
        centres[0] > 2.0 && centres[0] < 4.0,
        "first connector between A and M: {centres:?}"
    );
    assert!(
        centres[1] > 5.0 && centres[1] < 7.0,
        "second connector between M and C: {centres:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 18: determinism. The same input gives the same result, node for node.
// ---------------------------------------------------------------------------------------------

#[test]
fn c18_same_input_gives_identical_result_on_every_run() {
    let first = cut(&u_with_dots(), 1.0, 5.0).unwrap();
    let second = cut(&u_with_dots(), 1.0, 5.0).unwrap();
    assert_eq!(
        format!("{:?}", first.outline),
        format!("{:?}", second.outline)
    );
    assert_eq!(
        format!("{:?}", first.connectors),
        format!("{:?}", second.connectors)
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 12 and 13 as a property: 200 random sets of rectangles, fixed seed. Whenever a
// result exists, the Outline's area equals the union of the operands plus the connectors, within
// 0.01 mm per mm of outline.
// ---------------------------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn next_unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[test]
fn c12_property_outline_area_is_union_plus_connectors_on_200_fixed_seed_sets() {
    let mut rng = Lcg(0x0156_0156_0156_0156);
    let mut checked = 0;
    for _ in 0..200 {
        let count = 2 + (rng.next_unit() * 3.0) as usize;
        let operands: Vec<Operand> = (0..count)
            .map(|_| {
                let x = rng.next_unit() * 20.0;
                let y = rng.next_unit() * 20.0;
                let w = 1.0 + rng.next_unit() * 6.0;
                let h = 1.0 + rng.next_unit() * 6.0;
                vec![rect(x, y, x + w, y + h)]
            })
            .collect();
        let Ok(fused) = cut(&operands, 1.0, 5.0) else {
            continue;
        };
        checked += 1;
        let pieces = area(&ok(BooleanOp::Union, &operands));
        let connectors: f64 = fused.connectors.iter().map(|c| region_area(c).abs()).sum();
        let bound = 0.01 * perimeter(&fused.outline) + 0.02;
        let total = region_area(&fused.outline);
        assert!(
            (total - (pieces + connectors)).abs() <= bound,
            "outline {total} vs pieces {pieces} plus connectors {connectors}, bound {bound}, operands {operands:?}"
        );
    }
    assert!(
        checked >= 20,
        "too few sets joined to be a property: {checked}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 22: stencil with no island is refused.
// ---------------------------------------------------------------------------------------------

#[test]
fn c22_stencil_without_an_island_is_refused_with_no_islands() {
    let filled = vec![vec![rect(0.0, 0.0, 10.0, 10.0)]];
    assert!(matches!(
        stencil(&filled, 1.0, 2),
        Err(FuseError::NoIslands)
    ));
}

// ---------------------------------------------------------------------------------------------
// Criteria 21, 23 and 24: the O. Two bridges, at the top (0, -8) and the bottom (0, 8), width
// 1.0, the Outline is two C shapes with no inner contour and the area is the ring minus the
// connectors.
// ---------------------------------------------------------------------------------------------

#[test]
fn c24_o_gets_two_bridges_at_top_and_bottom_and_two_c_shapes() {
    let fused = stencil(&o_ring(), 1.0, 2).expect("the O must bridge");
    assert_eq!(fused.connectors.len(), 2, "two bridges for the one island");

    let mut centres: Vec<f64> = fused
        .connectors
        .iter()
        .map(|connector| {
            let ys = crossings_at_x(connector, 0.0);
            f64::midpoint(ys[0], ys[ys.len() - 1])
        })
        .collect();
    centres.sort_by(f64::total_cmp);
    assert_near(centres[0], -8.0, 0.01, "top bridge centre (y down)");
    assert_near(centres[1], 8.0, 0.01, "bottom bridge centre (y down)");

    for connector in &fused.connectors {
        let across = crossings_at_y(connector, centre_of(connector).y);
        assert_near(
            across[across.len() - 1] - across[0],
            1.0,
            0.01,
            "bridge width",
        );
    }

    assert_eq!(fused.outline.len(), 2, "the two C shapes");
    assert!(!has_nested_outline(&fused.outline), "no inner contour");
    assert_eq!(
        winding_at(&fused.outline, Point::new(0.0, 0.0)),
        0,
        "the island stays open"
    );
    assert_eq!(
        winding_at(&fused.outline, Point::new(0.0, -8.0)),
        0,
        "the top bridge is not in the Outline"
    );
    assert_eq!(
        winding_at(&fused.outline, Point::new(0.0, 8.0)),
        0,
        "the bottom bridge is not in the Outline"
    );
    assert!(
        inside(&fused.outline, Point::new(8.0, 0.0)),
        "the stroke is in the Outline"
    );

    let ring_area = std::f64::consts::PI * (100.0 - 36.0);
    let connectors: f64 = fused.connectors.iter().map(|c| region_area(c).abs()).sum();
    let bound = 0.01 * perimeter(&fused.outline) + 0.01;
    let total = region_area(&fused.outline).abs();
    assert!(
        (total - (ring_area - connectors)).abs() <= bound,
        "outline {total} vs ring minus connectors {}, bound {bound}",
        ring_area - connectors
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 21 and 23: bridges per island is the count of connectors, one to four.
// ---------------------------------------------------------------------------------------------

#[test]
fn c21_bridges_per_island_sets_the_number_of_connectors() {
    for count in 1u8..=4 {
        let fused = stencil(&o_ring(), 1.0, count).expect("the O must bridge");
        assert_eq!(
            fused.connectors.len(),
            usize::from(count),
            "{count} bridges per island"
        );
        assert!(
            !has_nested_outline(&fused.outline),
            "no inner contour with {count} bridges"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Criteria 25 and 28: letters with islands (geometric stand-ins, see report). A B has two
// islands, a one-hole A has one. Each island gets the chosen bridges, every island has at least
// one connector touching it, and no inner contour is left.
// ---------------------------------------------------------------------------------------------

#[test]
fn c25_b_has_two_islands_four_bridges_and_no_inner_contour() {
    let b = vec![vec![
        rect(0.0, 0.0, 10.0, 20.0),
        reversed(&rect(2.0, 2.0, 8.0, 8.0)),
        reversed(&rect(2.0, 12.0, 8.0, 18.0)),
    ]];
    let fused = stencil(&b, 1.0, 2).expect("the B must bridge");
    assert_eq!(fused.connectors.len(), 4, "2 bridges times 2 islands");
    assert!(
        !has_nested_outline(&fused.outline),
        "no inner contour after Bridge"
    );

    for island in [Point::new(5.0, 5.0), Point::new(5.0, 15.0)] {
        let touching = fused
            .connectors
            .iter()
            .filter(|c| {
                let (x0, y0, x1, y1) = bounds(c);
                x0 <= island.x + 6.0
                    && x1 >= island.x - 6.0
                    && y0 <= island.y + 6.0
                    && y1 >= island.y - 6.0
            })
            .count();
        assert!(touching >= 1, "island at {island:?} has no connector");
    }
}

#[test]
fn c25_a_hole_has_one_island_and_its_bridges_leave_no_inner_contour() {
    let a = vec![vec![
        rect(0.0, 0.0, 10.0, 10.0),
        reversed(&polygon(&[(3.0, 3.0), (7.0, 3.0), (5.0, 7.0)])),
    ]];
    let fused = stencil(&a, 1.0, 2).expect("the A must bridge");
    assert_eq!(fused.connectors.len(), 2, "2 bridges times 1 island");
    assert!(
        !has_nested_outline(&fused.outline),
        "no inner contour after Bridge"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 16 (a) and (b) on the bars: straight axis when the normals line up, neck 1.0 mm.
// Criterion 16 (c) and (d) need the golden in tests/fixtures and are not repeated here.
// ---------------------------------------------------------------------------------------------

#[test]
fn c16_straight_axis_for_aligned_normals_is_flat_across_the_gap() {
    let bars = vec![
        vec![rect(0.0, 0.0, 2.0, 10.0)],
        vec![rect(5.0, 3.0, 7.0, 13.0)],
    ];
    let fused = cut(&bars, 1.0, 5.0).unwrap();
    let connector = &fused.connectors[0];
    // The Blend runs 1 mm (radius min(Width, 0.45 x gap) = 1) from each bar, so the neck is
    // clean only between x = 3.0 and x = 4.0.
    for x in [3.2, 3.5, 3.8] {
        let across = crossings_at_x(connector, x);
        assert_near(
            across[across.len() - 1] - across[0],
            1.0,
            0.02,
            "neck width at x",
        );
        assert_near(
            f64::midpoint(across[0], across[across.len() - 1]),
            6.5,
            0.02,
            "axis at x",
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 7 (Reach): a piece beyond Reach stays apart and is still in the result.
// ---------------------------------------------------------------------------------------------

#[test]
fn c07_piece_beyond_reach_stays_apart_and_is_still_in_the_result() {
    let operands = vec![
        vec![rect(0.0, 0.0, 2.0, 10.0)],
        vec![rect(5.0, 3.0, 7.0, 13.0)],
        vec![rect(40.0, 0.0, 42.0, 10.0)],
    ];
    let fused = cut(&operands, 1.0, 5.0).expect("the first two join");
    assert_eq!(
        fused.connectors.len(),
        1,
        "only the pair within reach is bridged"
    );
    let (x0, _, x1, _) = bounds(&fused.outline);
    assert!(
        x1 >= 42.0 - 0.01 && x0 <= 0.01,
        "the far piece is part of the Outline: {x0}..{x1}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 7 (rule a): a dot whose nearest non-dot is beyond reach is a lone dot and joins by
// rule (b) like any piece (issue 7). Covered implicitly by criterion 17: no separate test here.
// ---------------------------------------------------------------------------------------------
