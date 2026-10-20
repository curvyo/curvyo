//! Weld's kernel against `specs/0068-laserable-lettering` criteria 4 to 10, 12 and 14 to 16.

use super::*;
use crate::boolean_grid::{normalize, to_millimetres};
use crate::visible_region::tests::disc;
use crate::{OutlineTriple, signed_area_mm2};
use curvyo_document_core::Vec2;

const TOLERANCE: Tolerance = Tolerance::from_mm(0.01);

fn corners(points: &[(f64, f64)]) -> Vec<OutlineTriple> {
    points
        .iter()
        .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
        .collect()
}

fn square(x: f64, y: f64, side: f64) -> Vec<OutlineTriple> {
    rectangle(x, y, side, side)
}

fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Vec<OutlineTriple> {
    corners(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)])
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

/// Welds closed shapes, bottom first, each one operand of one or more outlines.
fn weld_shapes(shapes: &[Vec<Vec<OutlineTriple>>]) -> Welded {
    let outlines: Vec<Vec<Outline<'_>>> = shapes
        .iter()
        .map(|shape| shape.iter().map(|a| Outline::new(a, true)).collect())
        .collect();
    let operands: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
    weld(&operands, TOLERANCE).unwrap()
}

fn closed(welded: &Welded) -> Vec<&Vec<Point>> {
    welded
        .outlines
        .iter()
        .filter(|o| o.1)
        .map(|o| &o.0)
        .collect()
}

fn open(welded: &Welded) -> Vec<&Vec<Point>> {
    welded
        .outlines
        .iter()
        .filter(|o| !o.1)
        .map(|o| &o.0)
        .collect()
}

/// The total line length, closing segments of closed outlines included.
fn length(welded: &Welded) -> f64 {
    welded
        .outlines
        .iter()
        .map(|(points, is_closed)| {
            let mut sum: f64 = points
                .windows(2)
                .map(|p| (p[1].x - p[0].x).hypot(p[1].y - p[0].y))
                .sum();
            if *is_closed {
                let (first, last) = (points[0], points[points.len() - 1]);
                sum += (first.x - last.x).hypot(first.y - last.y);
            }
            sum
        })
        .sum()
}

/// Criterion 4, and the `ClipRule` semantics the design rests on (`adrs.md` decision 2).
#[test]
fn ac04_two_squares_give_the_l_outline_and_the_trim_edge_once() {
    let welded = weld_shapes(&[vec![square(0.0, 0.0, 20.0)], vec![square(10.0, 10.0, 20.0)]]);
    let closed = closed(&welded);
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].len(), 8);
    assert!((signed_area_mm2(closed[0]) - 700.0).abs() < 1e-6);
    assert_eq!(
        open(&welded),
        vec![&vec![
            Point::new(10.0, 20.0),
            Point::new(10.0, 10.0),
            Point::new(20.0, 10.0)
        ]]
    );
    assert!((length(&welded) - 140.0).abs() < 1e-6);
    assert_eq!(welded.hidden, Vec::<usize>::new());
}

/// Criterion 5: a disc on a larger disc is a second closed outline wound as a shape; under it,
/// it is hidden.
#[test]
fn ac05_a_disc_on_a_disc_is_a_closed_shape_and_under_it_hidden() {
    let on_top = weld_shapes(&[vec![disc(20.0)], vec![disc(10.0)]]);
    let closed = closed(&on_top);
    assert_eq!((closed.len(), open(&on_top).len()), (2, 0));
    let areas: Vec<f64> = closed.iter().map(|o| signed_area_mm2(o)).collect();
    assert!(
        areas.iter().all(|&a| a > 0.0),
        "both wound as shapes: {areas:?}"
    );
    let under = weld_shapes(&[vec![disc(10.0)], vec![disc(20.0)]]);
    assert_eq!(under.outlines.len(), 1);
    assert_eq!(under.hidden, vec![0]);
}

/// Criterion 6: a shared edge is one open line; a shared corner adds none.
#[test]
fn ac06_a_shared_edge_is_one_open_line_and_a_corner_none() {
    let edge = weld_shapes(&[vec![square(0.0, 0.0, 20.0)], vec![square(20.0, 0.0, 20.0)]]);
    assert_eq!(closed(&edge).len(), 1);
    assert_eq!(closed(&edge)[0].len(), 4);
    assert_eq!(
        open(&edge),
        vec![&vec![Point::new(20.0, 0.0), Point::new(20.0, 20.0)]]
    );
    assert!((length(&edge) - 140.0).abs() < 1e-6);
    let corner = weld_shapes(&[vec![square(0.0, 0.0, 20.0)], vec![square(20.0, 20.0, 20.0)]]);
    assert_eq!((closed(&corner).len(), open(&corner).len()), (2, 0));
}

/// Criterion 7: identical squares give one 4-node outline.
#[test]
fn ac07_identical_squares_give_one_square() {
    let welded = weld_shapes(&[vec![square(0.0, 0.0, 20.0)], vec![square(0.0, 0.0, 20.0)]]);
    assert_eq!(welded.outlines.len(), 1);
    assert_eq!(welded.outlines[0].0.len(), 4);
}

/// Criterion 8: no line of the result lies inside the upper square's region.
#[test]
fn ac08_the_hidden_part_of_the_lower_outline_is_gone() {
    let welded = weld_shapes(&[vec![square(0.0, 0.0, 20.0)], vec![square(5.0, 5.0, 10.0)]]);
    let strictly_inside = |p: Point| p.x > 5.0 && p.x < 15.0 && p.y > 5.0 && p.y < 15.0;
    for (points, _) in &welded.outlines {
        for pair in points.windows(2) {
            let middle = Point::new(
                f64::midpoint(pair[0].x, pair[1].x),
                f64::midpoint(pair[0].y, pair[1].y),
            );
            assert!(!strictly_inside(middle), "{pair:?}");
        }
    }
    assert_eq!((closed(&welded).len(), length(&welded).round()), (2, 120.0));
}

/// Criterion 9: the ring keeps its hole, the disc in the hole stays an island, the bar's outline
/// over the ring is open lines.
#[test]
fn ac09_holes_and_islands_stay_and_the_bar_crosses_as_open_lines() {
    let ring = vec![disc(20.0), reversed(&disc(10.0))];
    let bar = vec![rectangle(-25.0, -18.0, 50.0, 4.0)];
    let island = vec![disc(5.0)];
    let welded = weld_shapes(&[ring, bar, island]);
    let areas: Vec<f64> = closed(&welded).iter().map(|o| signed_area_mm2(o)).collect();
    assert_eq!(areas.len(), 3, "outer, hole, island: {areas:?}");
    assert_eq!(areas.iter().filter(|&&a| a < 0.0).count(), 1, "one hole");
    assert_eq!(
        open(&welded).len(),
        2,
        "the bar's two long edges over the ring"
    );
}

/// Criterion 10: lines inside one operand's own region are not boundaries.
#[test]
fn ac10_lines_inside_one_operands_region_are_dropped() {
    let compound = vec![square(0.0, 0.0, 20.0), square(10.0, 10.0, 20.0)];
    let welded = weld_shapes(&[compound]);
    assert_eq!(welded.outlines.len(), 1);
    assert_eq!(welded.outlines[0].0.len(), 8);
}

/// Criterion 12: one shape that overlaps nothing is its own outline.
#[test]
fn ac12_one_shape_alone_is_its_outline() {
    let welded = weld_shapes(&[vec![square(3.0, 4.0, 5.0)]]);
    assert_eq!(welded.outlines.len(), 1);
    assert_eq!(welded.outlines[0], (corners_points(3.0, 4.0, 5.0), true));
}

fn corners_points(x: f64, y: f64, side: f64) -> Vec<Point> {
    vec![
        Point::new(x, y),
        Point::new(x + side, y),
        Point::new(x + side, y + side),
        Point::new(x, y + side),
    ]
}

/// Criteria 15 and 16: a stack of three makes junctions where lines meet; open lines have no
/// collinear node and run from their smaller end.
#[test]
fn ac15_lines_end_where_they_meet_another_line() {
    let welded = weld_shapes(&[
        vec![square(0.0, 0.0, 30.0)],
        vec![square(10.0, 10.0, 10.0)],
        vec![rectangle(15.0, 5.0, 2.0, 20.0)],
    ]);
    assert_eq!(closed(&welded).len(), 1);
    for line in open(&welded) {
        assert!((line[0].x, line[0].y) <= (line[line.len() - 1].x, line[line.len() - 1].y));
        for triple in line.windows(3) {
            let cross = (triple[1].x - triple[0].x) * (triple[2].y - triple[0].y)
                - (triple[1].y - triple[0].y) * (triple[2].x - triple[0].x);
            assert!(cross.abs() > 1e-9, "collinear node in {line:?}");
        }
    }
}

/// A seeded generator of shape sets: rectangles on whole millimetres (shared and identical
/// edges), triangles and discs.
fn random_sets(count: usize) -> Vec<Vec<Vec<Vec<OutlineTriple>>>> {
    let mut seed: u64 = 0x5eed_0068_0002;
    let mut next = || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[allow(clippy::cast_precision_loss)]
        let unit = (seed >> 11) as f64 / (1_u64 << 53) as f64;
        unit
    };
    (0..count)
        .map(|case| {
            (0..2 + case % 4)
                .map(|_| {
                    let (x, y, w, h) = (
                        next() * 30.0,
                        next() * 30.0,
                        1.0 + next() * 15.0,
                        1.0 + next() * 15.0,
                    );
                    let kind = next();
                    if kind < 0.4 {
                        vec![rectangle(x.round(), y.round(), w.round(), h.round())]
                    } else if kind < 0.7 {
                        vec![corners(&[(x, y), (x + w, y), (x + w * 0.5, y + h)])]
                    } else {
                        vec![moved(&disc(w * 0.5), x, y)]
                    }
                })
                .collect()
        })
        .collect()
}

/// Segments of `outlines` in millimetres (closing segments of closed ones included).
fn segments_of(outlines: &[(Vec<Point>, bool)]) -> Vec<(Point, Point)> {
    let mut segments = Vec::new();
    for (points, is_closed) in outlines {
        for pair in points.windows(2) {
            segments.push((pair[0], pair[1]));
        }
        if *is_closed && points.len() > 2 {
            segments.push((points[points.len() - 1], points[0]));
        }
    }
    segments
}

fn distance_to_segment(p: Point, (a, b): (Point, Point)) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared == 0.0 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / length_squared).clamp(0.0, 1.0)
    };
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}

/// Segments bucketed by 1 mm cells, for nearest-segment queries.
struct Buckets(std::collections::HashMap<(i64, i64), Vec<(Point, Point)>>);

impl Buckets {
    #[allow(clippy::cast_possible_truncation)]
    fn new(segments: &[(Point, Point)]) -> Self {
        let mut map: std::collections::HashMap<(i64, i64), Vec<(Point, Point)>> =
            std::collections::HashMap::new();
        for &(a, b) in segments {
            let (x0, x1) = (
                a.x.min(b.x).floor() as i64 - 1,
                a.x.max(b.x).floor() as i64 + 1,
            );
            let (y0, y1) = (
                a.y.min(b.y).floor() as i64 - 1,
                a.y.max(b.y).floor() as i64 + 1,
            );
            for x in x0..=x1 {
                for y in y0..=y1 {
                    map.entry((x, y)).or_default().push((a, b));
                }
            }
        }
        Self(map)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn distance(&self, p: Point) -> f64 {
        self.0
            .get(&(p.x.floor() as i64, p.y.floor() as i64))
            .into_iter()
            .flatten()
            .map(|&segment| distance_to_segment(p, segment))
            .fold(f64::INFINITY, f64::min)
    }
}

/// Points along every segment, at most 0.05 mm apart, ends included.
fn samples(segments: &[(Point, Point)]) -> Vec<Point> {
    let mut points = Vec::new();
    for &(a, b) in segments {
        let length = (b.x - a.x).hypot(b.y - a.y);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = (length / 0.05).ceil().max(1.0) as usize;
        for k in 0..=steps {
            #[allow(clippy::cast_precision_loss)]
            let t = k as f64 / steps as f64;
            points.push(Point::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y)));
        }
    }
    points
}

/// Whether two segments run along each other within 0.001 mm over more than 0.001 mm.
fn overlap(s: (Point, Point), t: (Point, Point)) -> bool {
    let (dx, dy) = (s.1.x - s.0.x, s.1.y - s.0.y);
    let length = dx.hypot(dy);
    if length == 0.0 {
        return false;
    }
    let along = |p: Point| ((p.x - s.0.x) * dx + (p.y - s.0.y) * dy) / length;
    let (ta, tb) = (along(t.0), along(t.1));
    let (lo, hi) = (ta.min(tb).max(0.0), ta.max(tb).min(length));
    if hi - lo <= 0.001 || (ta - tb).abs() < f64::EPSILON {
        return false;
    }
    let at = |u: f64| {
        let r = (u - ta) / (tb - ta);
        Point::new(t.0.x + r * (t.1.x - t.0.x), t.0.y + r * (t.1.y - t.0.y))
    };
    distance_to_segment(at(lo), s) <= 0.001 && distance_to_segment(at(hi), s) <= 0.001
}

/// Criterion 14 (property, 200 random sets, fixed seed): no two segments overlap over more than
/// 0.001 mm, and the result and the visible-region boundaries lie within 0.002 mm of each other
/// (`adrs.md`, "Flagged to the PO" 2: one grid unit plus one of cleanup).
#[test]
fn ac14_every_visible_line_exists_exactly_once() {
    for (case, shapes) in random_sets(200).iter().enumerate() {
        let welded = weld_shapes(shapes);
        let result = segments_of(&welded.outlines);
        for (i, &s) in result.iter().enumerate() {
            for &t in &result[i + 1..] {
                assert!(!overlap(s, t), "case {case}: {s:?} overlaps {t:?}");
            }
        }
        let outlines: Vec<Vec<Outline<'_>>> = shapes
            .iter()
            .map(|shape| shape.iter().map(|a| Outline::new(a, true)).collect())
            .collect();
        let operands: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
        let regions = operand_regions(&operands, TOLERANCE).unwrap();
        let mut reference: Vec<(Vec<Point>, bool)> = Vec::new();
        let mut above = Paths64::new();
        for region in regions.iter().rev() {
            let visible = run(OverlayRule::Difference, region, &above);
            reference.extend(
                visible
                    .iter()
                    .map(|path| (path.iter().map(to_millimetres).collect(), true)),
            );
            above = normalize(&above.iter().chain(region).cloned().collect());
        }
        let reference = segments_of(&reference);
        let near_reference = Buckets::new(&reference);
        for p in samples(&result) {
            let d = near_reference.distance(p);
            assert!(d <= 0.002 + 1e-9, "case {case}: {p:?} is {d} mm off");
        }
        let near_result = Buckets::new(&result);
        for p in samples(&reference) {
            let d = near_result.distance(p);
            assert!(
                d <= 0.002 + 1e-9,
                "case {case}: boundary {p:?} is {d} mm from the result"
            );
        }
    }
}
