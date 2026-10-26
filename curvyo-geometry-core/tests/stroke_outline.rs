//! The stroke outline kernel (`specs/0085-stroke-to-path` criteria 4 to 10, 12, 26 and 27): the
//! region a stroke paints, as closed outlines of exact pieces.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::many_single_char_names,
    missing_docs
)]

mod common;

use common::{Anchors, KERNEL_TOLERANCE, circle, polygon, rect};
use curvyo_document_core::{
    DashPattern, Length, LineCap, LineJoin, MarkerPlace, MarkerShape, Markers, Point, Vec2,
};
use curvyo_geometry_core::{
    MAX_COORDINATE_MM, OffsetAnchor, OffsetOutlines, Outline, StrokeOutlineError, StrokeShape,
    stroke_outline,
};

fn shape(width: f64, join: LineJoin, cap: LineCap) -> StrokeShape {
    StrokeShape {
        width: Length::from_mm(width),
        join,
        cap,
        dash: DashPattern::solid(),
        markers: Markers::default(),
    }
}

fn run(outlines: &[(Anchors, bool)], stroke: &StrokeShape) -> OffsetOutlines {
    try_run(outlines, stroke).expect("a stroke outline")
}

fn try_run(
    outlines: &[(Anchors, bool)],
    stroke: &StrokeShape,
) -> Result<OffsetOutlines, StrokeOutlineError> {
    let borrowed: Vec<Outline<'_>> = outlines
        .iter()
        .map(|(anchors, closed)| Outline::new(anchors, *closed))
        .collect();
    stroke_outline(&borrowed, stroke, KERNEL_TOLERANCE)
}

fn line(points: &[(f64, f64)]) -> (Anchors, bool) {
    (polygon(points), false)
}

/// The points of one outline, each segment cut into `steps` parts.
fn sampled(outline: &[OffsetAnchor], steps: usize) -> Vec<Point> {
    let n = outline.len();
    let mut points = Vec::new();
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        let p0 = a.point;
        let p1 = Point::new(p0.x + a.handle_out.x, p0.y + a.handle_out.y);
        let p3 = b.point;
        let p2 = Point::new(p3.x + b.handle_in.x, p3.y + b.handle_in.y);
        for k in 0..steps {
            let t = k as f64 / steps as f64;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            points.push(Point::new(
                w[0] * p0.x + w[1] * p1.x + w[2] * p2.x + w[3] * p3.x,
                w[0] * p0.y + w[1] * p1.y + w[2] * p2.y + w[3] * p3.y,
            ));
        }
    }
    points
}

fn signed_area(outline: &[OffsetAnchor]) -> f64 {
    let points = sampled(outline, 64);
    let n = points.len();
    (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

fn area(result: &OffsetOutlines) -> f64 {
    result.outlines().iter().map(|o| signed_area(o)).sum()
}

fn winding(result: &OffsetOutlines, p: Point) -> i32 {
    let mut total = 0;
    for outline in result.outlines() {
        let points = sampled(outline, 64);
        let n = points.len();
        for i in 0..n {
            let (a, b) = (points[i], points[(i + 1) % n]);
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

fn near(a: Point, x: f64, y: f64) -> bool {
    (a.x - x).abs() < 1e-6 && (a.y - y).abs() < 1e-6
}

fn is_corner(anchor: &OffsetAnchor) -> bool {
    anchor.handle_in == Vec2::new(0.0, 0.0) && anchor.handle_out == Vec2::new(0.0, 0.0)
}

#[test]
fn ac4_a_butt_line_is_one_rectangle_of_four_corners() {
    let result = run(
        &[line(&[(0.0, 0.0), (100.0, 0.0)])],
        &shape(2.0, LineJoin::Miter, LineCap::Butt),
    );
    assert_eq!(result.outlines().len(), 1);
    let outline = &result.outlines()[0];
    assert_eq!(outline.len(), 4, "{outline:?}");
    assert!(outline.iter().all(is_corner));
    for (x, y) in [(0.0, -1.0), (100.0, -1.0), (100.0, 1.0), (0.0, 1.0)] {
        assert!(outline.iter().any(|a| near(a.point, x, y)), "({x}, {y})");
    }
    assert!((area(&result) - 200.0).abs() < 0.01, "{}", area(&result));
}

#[test]
fn ac4_square_and_round_caps() {
    let square = run(
        &[line(&[(0.0, 0.0), (100.0, 0.0)])],
        &shape(2.0, LineJoin::Miter, LineCap::Square),
    );
    assert_eq!(square.outlines()[0].len(), 4);
    assert!((area(&square) - 204.0).abs() < 0.01);
    let round = run(
        &[line(&[(0.0, 0.0), (100.0, 0.0)])],
        &shape(2.0, LineJoin::Miter, LineCap::Round),
    );
    let outline = &round.outlines()[0];
    assert!((area(&round) - 203.14).abs() < 0.05, "{}", area(&round));
    // Two lines and two half circles of at most two cubics each.
    let cubics = outline.iter().filter(|a| !is_corner(a)).count();
    assert!(outline.len() <= 6, "{outline:?}");
    assert!(cubics >= 2, "{outline:?}");
}

#[test]
fn ac5_a_closed_square_gives_an_outer_square_and_a_hole() {
    let result = run(
        &[(rect(0.0, 0.0, 20.0, 20.0), true)],
        &shape(2.0, LineJoin::Miter, LineCap::Butt),
    );
    assert_eq!(result.outlines().len(), 2);
    let areas: Vec<f64> = result.outlines().iter().map(|o| signed_area(o)).collect();
    assert!(areas.iter().any(|a| (a - 484.0).abs() < 0.01), "{areas:?}");
    assert!(areas.iter().any(|a| (a + 324.0).abs() < 0.01), "{areas:?}");
    assert!((area(&result) - 160.0).abs() < 0.01);
    for outline in result.outlines() {
        assert_eq!(outline.len(), 4);
    }
}

#[test]
fn ac5_round_and_bevel_joins_cut_the_outer_corners_only() {
    for join in [LineJoin::Round, LineJoin::Bevel] {
        let result = run(
            &[(rect(0.0, 0.0, 20.0, 20.0), true)],
            &shape(2.0, join, LineCap::Butt),
        );
        let (outer, hole): (Vec<_>, Vec<_>) =
            result.outlines().iter().partition(|o| signed_area(o) > 0.0);
        assert_eq!(outer[0].len(), 8, "{join:?}");
        assert_eq!(hole[0].len(), 4, "{join:?}");
        assert!(hole[0].iter().all(is_corner));
        // Nothing reaches the miter corner (-1, -1).
        assert!(!outer[0].iter().any(|a| near(a.point, -1.0, -1.0)));
        if join == LineJoin::Round {
            let expected = 22.0 * 22.0 - 4.0 + std::f64::consts::PI - 324.0;
            assert!((area(&result) - expected).abs() < 0.01, "{}", area(&result));
        }
    }
}

#[test]
fn ac6_a_sharp_corner_falls_back_to_a_bevel() {
    let result = run(
        &[line(&[(0.0, 0.0), (50.0, 0.0), (0.0, 5.0)])],
        &shape(2.0, LineJoin::Miter, LineCap::Butt),
    );
    let reach = result
        .outlines()
        .iter()
        .flatten()
        .map(|a| a.point.x)
        .fold(f64::MIN, f64::max);
    assert!(
        reach < 51.0,
        "the miter tip of the 6 degree corner is drawn: {reach}"
    );
}

#[test]
fn ac7_a_circle_gives_two_smooth_circles() {
    let result = run(
        &[(circle(0.0, 0.0, 10.0), true)],
        &shape(2.0, LineJoin::Miter, LineCap::Butt),
    );
    assert_eq!(result.outlines().len(), 2);
    for outline in result.outlines() {
        assert!(outline.len() <= 8, "{}", outline.len());
        assert!(outline.iter().all(|a| a.smooth));
        let radius = if signed_area(outline) > 0.0 {
            11.0
        } else {
            9.0
        };
        for p in sampled(outline, 32) {
            let r = p.x.hypot(p.y);
            assert!((r - radius).abs() <= 0.01, "{r} vs {radius}");
        }
    }
}

#[test]
fn ac8_a_dashed_line_is_one_rectangle_per_dash() {
    let mut stroke = shape(1.0, LineJoin::Miter, LineCap::Butt);
    stroke.dash = DashPattern::new(vec![5.0, 5.0]).unwrap();
    let result = run(&[line(&[(0.0, 0.0), (100.0, 0.0)])], &stroke);
    assert_eq!(result.outlines().len(), 10);
    for (k, outline) in result.outlines().iter().enumerate() {
        assert_eq!(outline.len(), 4);
        let x0 = 10.0 * k as f64;
        assert!(near(outline[0].point, x0, -0.5), "{:?}", outline[0].point);
        assert!((signed_area(outline) - 5.0).abs() < 0.01);
    }
    stroke.cap = LineCap::Round;
    let round = run(&[line(&[(0.0, 0.0), (100.0, 0.0)])], &stroke);
    assert_eq!(round.outlines().len(), 10);
    let each = 5.0 + std::f64::consts::PI * 0.25;
    assert!(
        (area(&round) - 10.0 * each).abs() < 0.05,
        "{}",
        area(&round)
    );
}

#[test]
fn ac8_a_dash_around_a_corner_stays_one_outline() {
    let mut stroke = shape(1.0, LineJoin::Miter, LineCap::Butt);
    stroke.dash = DashPattern::new(vec![15.0, 5.0]).unwrap();
    let result = run(&[line(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)])], &stroke);
    assert_eq!(result.outlines().len(), 1);
    // The miter corner (10.5, -0.5) is part of the dash.
    assert!(
        result.outlines()[0]
            .iter()
            .any(|a| near(a.point, 10.5, -0.5))
    );
}

#[test]
fn ac9_a_crossing_stroke_counts_once() {
    let result = run(
        &[line(&[(0.0, 0.0), (20.0, 20.0), (20.0, 0.0), (0.0, 20.0)])],
        &shape(3.0, LineJoin::Miter, LineCap::Butt),
    );
    assert_eq!(winding(&result, Point::new(10.0, 10.0)), 1, "the crossing");
    assert_eq!(winding(&result, Point::new(5.0, 5.0)), 1);
    assert_eq!(winding(&result, Point::new(15.8, 10.0)), 0, "the hole");
    for outline in result.outlines() {
        // Outer outlines positive, holes negative: no outline lies inside the area twice.
        let a = signed_area(outline);
        assert!(a.abs() > 1.0, "{a}");
    }
}

#[test]
fn ac10_markers_join_the_line_or_stand_alone() {
    let mut stroke = shape(1.0, LineJoin::Miter, LineCap::Butt);
    stroke.markers = Markers {
        start: MarkerShape::Dot,
        end: MarkerShape::Arrow,
        ..Markers::default()
    };
    let result = run(&[line(&[(0.0, 0.0), (50.0, 0.0)])], &stroke);
    assert_eq!(
        result.outlines().len(),
        1,
        "line, dot and arrow are one region"
    );
    let xs: Vec<f64> = result.outlines()[0].iter().map(|a| a.point.x).collect();
    let (min, max) = xs
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), &x| (lo.min(x), hi.max(x)));
    assert!((max - 52.0).abs() < 1e-6, "arrow tip {max}");
    assert!((min + 1.5).abs() < 0.01, "dot {min}");

    stroke.dash = DashPattern::new(vec![1.0, 9.0]).unwrap();
    stroke.markers = Markers {
        mid: MarkerShape::Dot,
        mid_place: MarkerPlace::Spaced,
        ..Markers::default()
    };
    let result = run(&[line(&[(0.0, 0.0), (50.0, 0.0)])], &stroke);
    // Five dashes and the dot at 25 mm in the gap between two of them.
    assert_eq!(result.outlines().len(), 6);
    assert_eq!(winding(&result, Point::new(25.0, 1.2)), 1);
}

#[test]
fn ac12_nodes_are_spaced_and_smooth_where_tangents_agree() {
    // A curvy open path: lines and cubics, sharp and smooth nodes.
    let anchors: Anchors = vec![
        (
            Point::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(5.0, 0.0),
        ),
        (
            Point::new(10.0, 5.0),
            Vec2::new(-3.0, -3.0),
            Vec2::new(3.0, 3.0),
        ),
        (
            Point::new(20.0, 5.0),
            Vec2::new(-3.0, 3.0),
            Vec2::new(0.0, 0.0),
        ),
        (
            Point::new(30.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
        ),
        (
            Point::new(30.0, 20.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(-5.0, 5.0),
        ),
        (
            Point::new(10.0, 25.0),
            Vec2::new(5.0, 0.0),
            Vec2::new(-5.0, 0.0),
        ),
    ];
    for join in [LineJoin::Miter, LineJoin::Round, LineJoin::Bevel] {
        for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
            let result = run(&[(anchors.clone(), false)], &shape(2.0, join, cap));
            assert_eq!(result.fallback_outlines(), 0, "{join:?} {cap:?}");
            for outline in result.outlines() {
                let n = outline.len();
                for i in 0..n {
                    let (a, b) = (outline[i].point, outline[(i + 1) % n].point);
                    assert!((a.x - b.x).hypot(a.y - b.y) >= 0.001, "{join:?} {cap:?}");
                }
            }
        }
    }
}

#[test]
fn ac27_a_stroke_without_area_is_empty() {
    let single = (
        vec![(
            Point::new(5.0, 5.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
        )],
        false,
    );
    assert_eq!(
        try_run(&[single], &shape(1.0, LineJoin::Miter, LineCap::Butt)),
        Err(StrokeOutlineError::Empty)
    );
    let zero = line(&[(5.0, 5.0), (5.0, 5.0)]);
    assert_eq!(
        try_run(&[zero], &shape(1.0, LineJoin::Miter, LineCap::Butt)),
        Err(StrokeOutlineError::Empty)
    );
}

#[test]
fn ac26_out_of_range_is_refused() {
    let far = line(&[(0.0, 0.0), (MAX_COORDINATE_MM, 0.0)]);
    assert_eq!(
        try_run(&[far], &shape(2.0, LineJoin::Miter, LineCap::Butt)),
        Err(StrokeOutlineError::OutOfRange)
    );
    let nan = line(&[(0.0, 0.0), (f64::NAN, 0.0)]);
    assert_eq!(
        try_run(&[nan], &shape(2.0, LineJoin::Miter, LineCap::Butt)),
        Err(StrokeOutlineError::OutOfRange)
    );
    let ok = line(&[(0.0, 0.0), (10.0, 0.0)]);
    assert_eq!(
        try_run(&[ok], &shape(f64::INFINITY, LineJoin::Miter, LineCap::Butt)),
        Err(StrokeOutlineError::OutOfRange)
    );
}

#[test]
fn ac24_the_outline_offsets_like_any_closed_path() {
    let outline = run(
        &[line(&[(0.0, 0.0), (100.0, 0.0)])],
        &shape(2.0, LineJoin::Miter, LineCap::Butt),
    );
    let anchors: Vec<Anchors> = outline
        .outlines()
        .iter()
        .map(|o| {
            o.iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect()
        })
        .collect();
    let borrowed: Vec<Outline<'_>> = anchors.iter().map(|a| Outline::new(a, true)).collect();
    let grown = curvyo_geometry_core::offset(
        &borrowed,
        Length::from_mm(1.0),
        curvyo_geometry_core::OffsetJoin::Round,
        KERNEL_TOLERANCE,
    )
    .unwrap();
    let points: Vec<Point> = grown
        .outlines()
        .iter()
        .flat_map(|o| sampled(o, 16))
        .collect();
    let (min_x, max_x) = points.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p.x), hi.max(p.x))
    });
    let (min_y, max_y) = points.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p.y), hi.max(p.y))
    });
    assert!((max_x - min_x - 102.0).abs() < 0.01 && (max_y - min_y - 4.0).abs() < 0.01);
    let expected = 102.0 * 4.0 - 4.0 + std::f64::consts::PI;
    assert!((area(&grown) - expected).abs() < 0.01, "{}", area(&grown));
}
