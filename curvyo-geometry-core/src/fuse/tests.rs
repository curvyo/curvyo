//! `fuse_cut_out` and `fuse_stencil` against `specs/0156-fused-lettering` criteria 7 to 10, 12,
//! 13, 15 to 18 and 22 to 27.

#![allow(
    clippy::many_single_char_names,
    clippy::manual_midpoint,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use curvyo_document_core::{Length, Point, Tolerance, Vec2};

use super::*;
use crate::{BooleanOp, OutlineTriple, boolean, signed_area_mm2};

const TOLERANCE: Tolerance = Tolerance::from_mm(0.01);
const WIDTH: Length = Length::from_mm(1.0);
const REACH: Length = Length::from_mm(5.0);

type Shape = Vec<(Vec<OutlineTriple>, bool)>;

fn corner(x: f64, y: f64) -> OutlineTriple {
    (Point::new(x, y), Vec2::ZERO, Vec2::ZERO)
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<OutlineTriple> {
    vec![
        corner(x0, y0),
        corner(x1, y0),
        corner(x1, y1),
        corner(x0, y1),
    ]
}

/// A circle of four cubics, clockwise on screen.
fn circle(cx: f64, cy: f64, r: f64) -> Vec<OutlineTriple> {
    let k = 0.552_284_749_830_793_4 * r;
    vec![
        (
            Point::new(cx + r, cy),
            Vec2::new(0.0, -k),
            Vec2::new(0.0, k),
        ),
        (
            Point::new(cx, cy + r),
            Vec2::new(k, 0.0),
            Vec2::new(-k, 0.0),
        ),
        (
            Point::new(cx - r, cy),
            Vec2::new(0.0, k),
            Vec2::new(0.0, -k),
        ),
        (
            Point::new(cx, cy - r),
            Vec2::new(-k, 0.0),
            Vec2::new(k, 0.0),
        ),
    ]
}

fn reversed(mut outline: Vec<OutlineTriple>) -> Vec<OutlineTriple> {
    outline.reverse();
    for (_, a, b) in &mut outline {
        std::mem::swap(a, b);
    }
    outline
}

fn shape(outlines: Vec<Vec<OutlineTriple>>) -> Shape {
    outlines.into_iter().map(|o| (o, true)).collect()
}

fn views(shapes: &[Shape]) -> Vec<Vec<Outline<'_>>> {
    shapes
        .iter()
        .map(|s| s.iter().map(|(a, c)| Outline::new(a, *c)).collect())
        .collect()
}

fn cut_out(shapes: &[Shape], reach: Option<Length>) -> Result<Fused, FuseError> {
    let v = views(shapes);
    let slices: Vec<&[Outline<'_>]> = v.iter().map(Vec::as_slice).collect();
    fuse_cut_out(&slices, WIDTH, reach, 2000, TOLERANCE)
}

fn stencil(shapes: &[Shape], per_island: u8) -> Result<Fused, FuseError> {
    let v = views(shapes);
    let slices: Vec<&[Outline<'_>]> = v.iter().map(Vec::as_slice).collect();
    fuse_stencil(&slices, WIDTH, per_island, 2000, TOLERANCE)
}

fn area(outlines: &[Vec<Point>]) -> f64 {
    outlines.iter().map(|o| signed_area_mm2(o)).sum()
}

fn perimeter(outlines: &[Vec<Point>]) -> f64 {
    outlines
        .iter()
        .map(|o| {
            (0..o.len())
                .map(|i| {
                    let (a, b) = (o[i], o[(i + 1) % o.len()]);
                    ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
                })
                .sum::<f64>()
        })
        .sum()
}

fn union_area(shapes: &[Shape]) -> f64 {
    let v = views(shapes);
    let slices: Vec<&[Outline<'_>]> = v.iter().map(Vec::as_slice).collect();
    area(
        boolean(BooleanOp::Union, &slices, TOLERANCE)
            .unwrap()
            .outlines(),
    )
}

fn bounds(outlines: &[Vec<Point>]) -> (f64, f64, f64, f64) {
    let points = outlines.iter().flatten();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in points {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    (x0, y0, x1, y1)
}

/// The x extent of `outlines` along the horizontal line at `y`.
fn extent_at_y(outlines: &[Vec<Point>], y: f64) -> (f64, f64) {
    let mut xs = Vec::new();
    for o in outlines {
        for i in 0..o.len() {
            let (a, b) = (o[i], o[(i + 1) % o.len()]);
            if (a.y - y) * (b.y - y) < 0.0 {
                xs.push(a.x + (b.x - a.x) * (y - a.y) / (b.y - a.y));
            }
        }
    }
    let low = xs.iter().copied().fold(f64::MAX, f64::min);
    let high = xs.iter().copied().fold(f64::MIN, f64::max);
    (low, high)
}

/// The y extent of `outlines` along the vertical line at `x`.
fn extent_at_x(outlines: &[Vec<Point>], x: f64) -> (f64, f64) {
    let swapped: Vec<Vec<Point>> = outlines
        .iter()
        .map(|o| o.iter().map(|p| Point::new(p.y, p.x)).collect())
        .collect();
    extent_at_y(&swapped, x)
}

fn letter_i() -> Vec<Shape> {
    vec![
        shape(vec![rect(0.0, 0.0, 2.0, 10.0)]),
        shape(vec![circle(1.0, 13.0, 1.0)]),
    ]
}

/// Criterion 8: the i gets one straight connector from (1, 10) to (1, 12), 1 mm wide, and one
/// closed outline 1.5 to 4 mm² larger than stem and disc.
#[test]
fn the_i_gets_one_connector_of_the_width_on_its_centre_line() {
    let fused = cut_out(&letter_i(), Some(REACH)).unwrap();
    assert_eq!(fused.connectors.len(), 1);
    assert_eq!(fused.outline.len(), 1);
    let connector = &fused.connectors[0];
    let (low, high) = extent_at_y(connector, 11.3);
    assert!((high - low - 1.0).abs() <= 0.01, "{low} {high}");
    assert!(((high + low) / 2.0 - 1.0).abs() <= 0.01);
    let (_, y0, _, y1) = bounds(connector);
    assert!(y0 >= 10.0 - 0.01 && y1 <= 12.5, "{y0} {y1}");
    let added = area(&fused.outline) - union_area(&letter_i());
    assert!((1.5..=4.0).contains(&added), "{added}");
}

/// Criterion 9: the bars' bridge is centred at y = 6.5.
#[test]
fn the_bars_bridge_is_centred_on_the_tied_stretch() {
    let bars = vec![
        shape(vec![rect(0.0, 0.0, 2.0, 10.0)]),
        shape(vec![rect(5.0, 3.0, 7.0, 13.0)]),
    ];
    let fused = cut_out(&bars, Some(REACH)).unwrap();
    assert_eq!(fused.connectors.len(), 1);
    let (_, y0, _, y1) = bounds(&fused.connectors[0]);
    assert!(((y0 + y1) / 2.0 - 6.5).abs() <= 0.01, "{y0} {y1}");
}

/// Criterion 10 (geometric stand-in for the font): a ü's two dots each go to the u.
#[test]
fn the_dots_of_a_u_umlaut_each_go_to_the_u() {
    let u = shape(vec![
        rect(0.0, 4.0, 2.0, 12.0),
        rect(6.0, 4.0, 8.0, 12.0),
        rect(0.0, 10.0, 8.0, 12.0),
    ]);
    let letters = vec![
        u,
        shape(vec![circle(1.0, 1.5, 1.0)]),
        shape(vec![circle(7.0, 1.5, 1.0)]),
    ];
    let fused = cut_out(&letters, Some(REACH)).unwrap();
    assert_eq!(fused.connectors.len(), 2);
    for connector in &fused.connectors {
        let (x0, _, x1, _) = bounds(connector);
        assert!(x1 - x0 < 4.0, "a dot-to-dot bridge: {x0} {x1}");
    }
    assert_eq!(fused.outline.len(), 1);
}

/// Criterion 15: three pieces in a row, the middle one narrower (and no dot), give two
/// connectors, none through the middle piece.
#[test]
fn three_pieces_in_a_row_are_joined_through_the_middle() {
    let row = vec![
        shape(vec![rect(0.0, 0.0, 4.0, 10.0)]),
        shape(vec![rect(5.0, 0.0, 6.0, 10.0)]),
        shape(vec![rect(7.0, 0.0, 11.0, 10.0)]),
    ];
    let fused = cut_out(&row, Some(REACH)).unwrap();
    assert_eq!(fused.connectors.len(), 2);
    for connector in &fused.connectors {
        let (x0, _, x1, _) = bounds(connector);
        assert!(x1 - x0 < 2.0, "{x0} {x1}");
    }
}

/// Criterion 7 and Frage 398 C: beyond Reach pieces stay apart; Join all joins them.
#[test]
fn reach_limits_the_bridges_and_join_all_lifts_the_limit() {
    let far = vec![
        shape(vec![rect(0.0, 0.0, 2.0, 2.0)]),
        shape(vec![rect(20.0, 0.0, 22.0, 2.0)]),
    ];
    assert_eq!(cut_out(&far, Some(REACH)), Err(FuseError::NothingToJoin));
    let joined = cut_out(&far, None).unwrap();
    assert_eq!(joined.connectors.len(), 1);
    assert_eq!(joined.outline.len(), 1);
}

/// Criterion 7: a piece in a hole of another stays apart and in the result.
#[test]
fn a_piece_in_a_hole_stays_apart() {
    let ring_and_dot = vec![
        shape(vec![
            rect(0.0, 0.0, 10.0, 10.0),
            reversed(rect(2.0, 2.0, 8.0, 8.0)),
        ]),
        shape(vec![circle(5.0, 5.0, 1.0)]),
        shape(vec![rect(12.0, 0.0, 14.0, 10.0)]),
    ];
    let fused = cut_out(&ring_and_dot, Some(REACH)).unwrap();
    assert_eq!(fused.connectors.len(), 1);
    // Outer, hole and the dot.
    assert_eq!(fused.outline.len(), 3);
}

/// Criterion 17: one piece, and the bridge limit.
#[test]
fn refusals() {
    let one = vec![shape(vec![rect(0.0, 0.0, 2.0, 2.0)])];
    assert_eq!(cut_out(&one, Some(REACH)), Err(FuseError::NothingToJoin));
    let i = letter_i();
    let v = views(&i);
    let slices: Vec<&[Outline<'_>]> = v.iter().map(Vec::as_slice).collect();
    assert_eq!(
        fuse_cut_out(&slices, WIDTH, Some(REACH), 0, TOLERANCE),
        Err(FuseError::TooManyBridges)
    );
    let flat = vec![shape(vec![vec![corner(0.0, 0.0), corner(5.0, 0.0)]])];
    assert_eq!(
        cut_out(&flat, Some(REACH)),
        Err(FuseError::Operands(BooleanError::EmptyOperands(vec![0])))
    );
}

/// Criterion 18: the same input gives the same result.
#[test]
fn bridging_is_deterministic() {
    assert_eq!(
        cut_out(&letter_i(), Some(REACH)),
        cut_out(&letter_i(), Some(REACH))
    );
}

/// Criterion 16c: where a connector meets a stroke the nodes it adds turn by at most 15 degrees.
#[test]
fn the_connector_blends_without_a_sharp_corner() {
    let bars = vec![
        shape(vec![rect(0.0, 0.0, 2.0, 10.0)]),
        shape(vec![rect(5.0, 0.0, 7.0, 10.0)]),
    ];
    let fused = cut_out(&bars, Some(REACH)).unwrap();
    let outline = &fused.outline[0];
    let n = outline.len();
    for i in 0..n {
        let (a, b, c) = (outline[(i + n - 1) % n], outline[i], outline[(i + 1) % n]);
        // Only the nodes in the gap, off the bars' own corners.
        if !(b.x > 2.0 + 1e-6 && b.x < 5.0 - 1e-6) {
            continue;
        }
        let (u, v) = ((b.x - a.x, b.y - a.y), (c.x - b.x, c.y - b.y));
        let cos = (u.0 * v.0 + u.1 * v.1) / ((u.0.hypot(u.1)) * (v.0.hypot(v.1)));
        assert!(cos >= 15.0_f64.to_radians().cos() - 1e-9, "turn at {b:?}");
    }
}

/// Criterion 24: the O gets two bridges at the top and the bottom and becomes two C shapes.
#[test]
fn the_stencil_o_gets_two_bridges_and_no_inner_contour() {
    let o = vec![shape(vec![
        circle(0.0, 0.0, 10.0),
        reversed(circle(0.0, 0.0, 6.0)),
    ])];
    let fused = stencil(&o, 2).unwrap();
    assert_eq!(fused.connectors.len(), 2);
    assert_eq!(fused.outline.len(), 2);
    assert!(fused.outline.iter().all(|o| signed_area_mm2(o) > 0.0));
    let mut centres: Vec<(f64, f64)> = fused
        .connectors
        .iter()
        .map(|c| {
            let (x0, _, x1, _) = bounds(c);
            let (y0, y1) = extent_at_x(c, 0.0005);
            ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
        })
        .collect();
    centres.sort_by(|a, b| a.1.total_cmp(&b.1));
    assert!(
        centres[0].0.abs() <= 0.01 && (centres[0].1 + 8.0).abs() <= 0.01,
        "{centres:?}"
    );
    assert!(
        centres[1].0.abs() <= 0.01 && (centres[1].1 - 8.0).abs() <= 0.01,
        "{centres:?}"
    );
    let (low, high) = extent_at_y(&fused.connectors[0], -8.0);
    assert!((high - low - 1.0).abs() <= 0.01, "{low} {high}");
    let ring = std::f64::consts::PI * 64.0;
    let bridges: f64 = fused.connectors.iter().map(|c| area(c)).sum();
    let bound = 0.01 * perimeter(&fused.outline);
    assert!((area(&fused.outline) - (ring - bridges)).abs() <= bound + 0.05);
}

/// Criteria 22, 23 and 25 (geometric stand-in for a B): two counters, two bridges each, no inner
/// contour left; a solid shape has no islands.
#[test]
fn a_letter_with_two_counters_gets_its_bridges_per_island() {
    let b = vec![shape(vec![
        rect(0.0, 0.0, 8.0, 14.0),
        reversed(rect(2.0, 2.0, 6.0, 6.0)),
        reversed(rect(2.0, 8.0, 6.0, 12.0)),
    ])];
    let fused = stencil(&b, 2).unwrap();
    assert_eq!(fused.connectors.len(), 4);
    assert!(fused.outline.iter().all(|o| signed_area_mm2(o) > 0.0));
    let solid = vec![shape(vec![rect(0.0, 0.0, 8.0, 14.0)])];
    assert_eq!(stencil(&solid, 2), Err(FuseError::NoIslands));
}

/// A small deterministic generator for the property test (fixed seed).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[allow(clippy::cast_precision_loss)]
        let value = (self.0 >> 11) as f64 / (1_u64 << 53) as f64;
        value
    }
}

/// Criteria 12 and 13 on 200 random shape sets: the Outline is the union of the pieces and the
/// connectors, and no connector overlaps the pieces beyond the tolerance.
#[test]
fn outline_and_connectors_hold_on_random_shape_sets() {
    let mut random = Lcg(156);
    let mut checked = 0;
    for _ in 0..200 {
        let count = 2 + (random.next() * 4.0) as usize;
        let shapes: Vec<Shape> = (0..count)
            .map(|_| {
                let (x, y) = (random.next() * 20.0, random.next() * 20.0);
                let size = 0.8 + random.next() * 4.0;
                if random.next() < 0.5 {
                    shape(vec![rect(x, y, x + size, y + size * 1.7)])
                } else {
                    shape(vec![circle(x, y, size / 2.0)])
                }
            })
            .collect();
        let Ok(fused) = cut_out(&shapes, Some(REACH)) else {
            continue;
        };
        checked += 1;
        let pieces = union_area(&shapes);
        let mut bridges = 0.0;
        for connector in &fused.connectors {
            let mut both = shapes.clone();
            both.push(
                connector
                    .iter()
                    .map(|o| (o.iter().map(|p| corner(p.x, p.y)).collect(), true))
                    .collect(),
            );
            let v = views(&both);
            let slices: Vec<&[Outline<'_>]> = v.iter().map(Vec::as_slice).collect();
            let rest: Vec<&[Outline<'_>]> = vec![slices[slices.len() - 1]];
            let overlap = {
                let union =
                    boolean(BooleanOp::Union, &slices[..slices.len() - 1], TOLERANCE).unwrap();
                let u: Vec<Vec<OutlineTriple>> = union
                    .outlines()
                    .iter()
                    .map(|o| o.iter().map(|p| corner(p.x, p.y)).collect())
                    .collect();
                let uv: Vec<Outline<'_>> = u.iter().map(|o| Outline::new(o, true)).collect();
                boolean(BooleanOp::Intersection, &[rest[0], &uv], TOLERANCE)
                    .map_or(0.0, |r| area(r.outlines()))
            };
            assert!(
                overlap <= 0.01 * perimeter(connector) + 1e-6,
                "overlap {overlap}"
            );
            bridges += area(connector);
        }
        let bound = 0.01 * perimeter(&fused.outline);
        assert!(
            (area(&fused.outline) - (pieces + bridges)).abs() <= bound,
            "{} {} {}",
            area(&fused.outline),
            pieces,
            bridges
        );
    }
    assert!(checked > 50, "{checked}");
}
