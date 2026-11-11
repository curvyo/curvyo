//! The variable-width outline (`specs/0110-pen-modes-variable-stroke` criteria 6 and 9, ADR 0053
//! amendment of 2026-11-10): what it draws for profiles, caps, joins, dashes and markers, what it
//! refuses, and that a uniform profile is the ordinary stroke.

#![allow(clippy::unwrap_used)]

use curvyo_document_core::{
    DashPattern, Length, LineCap, LineJoin, MarkerShape, Markers, Point, Tolerance, Vec2,
};
use curvyo_geometry_core::{
    OffsetOutlines, Outline, StrokeOutlineError, StrokeShape, VariableStrokeError, WidthPoint,
    WidthProfile, contains_point_in_outlines, signed_area_mm2, stroke_outline,
    variable_stroke_outline,
};

type Triple = (Point, Vec2, Vec2);

const TOLERANCE: f64 = 0.01;

fn corner(x: f64, y: f64) -> Triple {
    (Point::new(x, y), Vec2::ZERO, Vec2::ZERO)
}

fn style(width: f64, cap: LineCap, join: LineJoin) -> StrokeShape {
    StrokeShape {
        width: Length::from_mm(width),
        join,
        cap,
        dash: DashPattern::default(),
        markers: Markers::default(),
    }
}

fn profile(points: &[(f64, f64)], closed: bool) -> WidthProfile {
    let points = points
        .iter()
        .enumerate()
        .map(|(i, &(at, m))| WidthPoint::new(i as u128 + 1, at, m, closed).unwrap())
        .collect();
    WidthProfile::new(points, closed).unwrap()
}

fn outline(
    anchors: &[Triple],
    closed: bool,
    style: &StrokeShape,
    profile: &WidthProfile,
) -> Result<OffsetOutlines, VariableStrokeError> {
    variable_stroke_outline(
        Outline::new(anchors, closed),
        style,
        profile,
        Tolerance::from_mm(TOLERANCE),
    )
}

fn triples(result: &OffsetOutlines) -> Vec<Vec<Triple>> {
    result
        .outlines()
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect()
        })
        .collect()
}

fn inside(result: &OffsetOutlines, x: f64, y: f64) -> bool {
    let rings = triples(result);
    let outlines: Vec<Outline<'_>> = rings.iter().map(|r| Outline::new(r, true)).collect();
    contains_point_in_outlines(&outlines, Point::new(x, y))
}

fn areas(result: &OffsetOutlines) -> Vec<f64> {
    result
        .outlines()
        .iter()
        .map(|ring| signed_area_mm2(&ring.iter().map(|a| a.point).collect::<Vec<_>>()))
        .collect()
}

fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

#[test]
fn a_uniform_profile_is_the_ordinary_stroke() {
    let anchors = [
        (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::new(10.0, 10.0)),
        (Point::new(40.0, 0.0), Vec2::new(-10.0, 10.0), Vec2::ZERO),
    ];
    let shape = style(2.0, LineCap::Round, LineJoin::Round);
    let ordinary = stroke_outline(
        &[Outline::new(&anchors, false)],
        &shape,
        Tolerance::from_mm(TOLERANCE),
    )
    .unwrap();
    for uniform in [
        profile(&[], false),
        profile(&[(0.2, 1.0), (0.8, 1.0)], false),
    ] {
        assert_eq!(
            outline(&anchors, false, &shape, &uniform).unwrap(),
            ordinary
        );
    }
    let doubled = stroke_outline(
        &[Outline::new(&anchors, false)],
        &style(4.0, LineCap::Round, LineJoin::Round),
        Tolerance::from_mm(TOLERANCE),
    )
    .unwrap();
    let twice = profile(&[(0.5, 2.0)], false);
    assert_eq!(outline(&anchors, false, &shape, &twice).unwrap(), doubled);
}

#[test]
fn a_taper_follows_the_smoothstep_profile_along_a_line() {
    let anchors = [corner(0.0, 0.0), corner(100.0, 0.0)];
    let taper_end = profile(&[(0.0, 1.0), (1.0, 0.0)], false);
    let result = outline(
        &anchors,
        false,
        &style(4.0, LineCap::Butt, LineJoin::Miter),
        &taper_end,
    )
    .unwrap();
    for i in 1..20 {
        let x = 5.0 * f64::from(i);
        let half = 2.0 * (1.0 - smoothstep(x / 100.0));
        assert!(inside(&result, x, half - 0.02), "x={x} inside");
        assert!(inside(&result, x, -(half - 0.02)), "x={x} inside below");
        assert!(!inside(&result, x, half + 0.02), "x={x} outside");
    }
}

#[test]
fn the_benchmark_paths_are_drawn() {
    let s_curve = [
        (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::new(10.0, 0.0)),
        (
            Point::new(30.0, 20.0),
            Vec2::new(-10.0, 0.0),
            Vec2::new(10.0, 0.0),
        ),
        (
            Point::new(60.0, -20.0),
            Vec2::new(-10.0, 0.0),
            Vec2::new(10.0, 0.0),
        ),
        (Point::new(90.0, 0.0), Vec2::new(-10.0, 0.0), Vec2::ZERO),
    ];
    let kinked: Vec<Triple> = (0..12)
        .map(|i| {
            let y = if i % 2 == 0 { 0.0 } else { 6.0 };
            (
                Point::new(8.0 * f64::from(i), y),
                Vec2::new(-2.0, 2.0),
                Vec2::new(2.0, 2.0),
            )
        })
        .collect();
    let k = 5.0 * 4.0 / 3.0;
    let arc = [
        (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::new(0.0, k)),
        (Point::new(10.0, 0.0), Vec2::new(0.0, k), Vec2::ZERO),
    ];
    let taper = profile(&[(0.0, 0.0), (0.5, 3.0), (1.0, 0.0)], false);
    for anchors in [&s_curve[..], &kinked, &arc] {
        let result = outline(
            anchors,
            false,
            &style(3.0, LineCap::Round, LineJoin::Miter),
            &taper,
        )
        .unwrap();
        assert!(areas(&result).iter().sum::<f64>() > 1.0);
    }
}

#[test]
fn an_interior_zero_width_splits_the_stroke() {
    let anchors = [corner(0.0, 0.0), corner(50.0, 0.0)];
    let pinch = profile(&[(0.0, 1.0), (0.5, 0.0), (1.0, 1.0)], false);
    let result = outline(
        &anchors,
        false,
        &style(4.0, LineCap::Butt, LineJoin::Miter),
        &pinch,
    )
    .unwrap();
    assert_eq!(areas(&result).iter().filter(|a| **a > 0.0).count(), 2);
    assert!(inside(&result, 5.0, 1.5) && inside(&result, 45.0, -1.5));
}

#[test]
fn caps_reach_the_local_half_width() {
    let anchors = [corner(0.0, 0.0), corner(100.0, 0.0)];
    let growing = profile(&[(0.0, 1.0), (1.0, 3.0)], false);
    let round = outline(
        &anchors,
        false,
        &style(2.0, LineCap::Round, LineJoin::Miter),
        &growing,
    )
    .unwrap();
    assert!(inside(&round, 102.9, 0.0) && !inside(&round, 103.1, 0.0));
    assert!(inside(&round, -0.9, 0.0) && !inside(&round, -1.1, 0.0));
    let square = outline(
        &anchors,
        false,
        &style(2.0, LineCap::Square, LineJoin::Miter),
        &growing,
    )
    .unwrap();
    assert!(inside(&square, 102.9, 2.9) && !inside(&square, 103.1, 0.0));
    let butt = outline(
        &anchors,
        false,
        &style(2.0, LineCap::Butt, LineJoin::Miter),
        &growing,
    )
    .unwrap();
    assert!(inside(&butt, 99.9, 2.9) && !inside(&butt, 100.1, 0.0));
}

#[test]
fn corners_get_the_join_of_the_style_at_the_local_width() {
    let anchors = [corner(0.0, 0.0), corner(20.0, 0.0), corner(20.0, 20.0)];
    let growing = profile(&[(0.0, 1.0), (1.0, 1.5)], false);
    // Half width 2.5 at the corner: the miter tip is at (22.5, -2.5).
    let miter = outline(
        &anchors,
        false,
        &style(4.0, LineCap::Butt, LineJoin::Miter),
        &growing,
    )
    .unwrap();
    assert!(inside(&miter, 22.4, -2.4));
    let bevel = outline(
        &anchors,
        false,
        &style(4.0, LineCap::Butt, LineJoin::Bevel),
        &growing,
    )
    .unwrap();
    assert!(!inside(&bevel, 22.3, -2.3) && inside(&bevel, 21.1, -1.1));
    let round = outline(
        &anchors,
        false,
        &style(4.0, LineCap::Butt, LineJoin::Round),
        &growing,
    )
    .unwrap();
    let d = 2.5 * std::f64::consts::FRAC_1_SQRT_2;
    assert!(inside(&round, 20.0 + d * 0.98, -d * 0.98) && !inside(&round, 22.3, -2.3));
}

#[test]
fn a_closed_path_has_an_outer_ring_and_a_hole_with_a_periodic_profile() {
    let square = [
        corner(0.0, 0.0),
        corner(40.0, 0.0),
        corner(40.0, 40.0),
        corner(0.0, 40.0),
    ];
    let periodic = profile(&[(0.0, 1.0), (0.5, 3.0)], true);
    let result = outline(
        &square,
        true,
        &style(2.0, LineCap::Butt, LineJoin::Miter),
        &periodic,
    )
    .unwrap();
    let signs: Vec<bool> = areas(&result).iter().map(|a| *a > 0.0).collect();
    assert_eq!(signs.iter().filter(|s| **s).count(), 1);
    assert_eq!(signs.iter().filter(|s| !**s).count(), 1);
    assert!(inside(&result, 20.0, 0.0) && !inside(&result, 20.0, 20.0));
    // (40, 20) lies at position 0.375: multiplier 1 + 2 smoothstep(0.75) = 2.6875.
    assert!(inside(&result, 42.6, 20.0) && !inside(&result, 42.8, 20.0));
    // The seam takes the value across it from the last point to the first: multiplier 1.
    assert!(inside(&result, 0.0, -0.9) && !inside(&result, 0.0, -1.1));
    assert_eq!(
        variable_stroke_outline(
            Outline::new(&square, true),
            &style(2.0, LineCap::Butt, LineJoin::Miter),
            &profile(&[(0.5, 2.0)], false),
            Tolerance::from_mm(TOLERANCE)
        ),
        Err(VariableStrokeError::Unsupported)
    );
}

#[test]
fn dashes_take_the_global_profile_at_their_ends() {
    let anchors = [corner(0.0, 0.0), corner(40.0, 0.0)];
    let mut shape = style(1.0, LineCap::Butt, LineJoin::Miter);
    shape.dash = DashPattern::new(vec![5.0, 5.0]).unwrap();
    let growing = profile(&[(0.0, 1.0), (1.0, 3.0)], false);
    let result = outline(&anchors, false, &shape, &growing).unwrap();
    assert_eq!(areas(&result).iter().filter(|a| **a > 0.0).count(), 4);
    let half_at = |x: f64| f64::midpoint(1.0, 2.0 * smoothstep(x / 40.0));
    assert!(inside(&result, 34.9, half_at(34.9) - 0.02));
    assert!(!inside(&result, 35.1, 0.0) && !inside(&result, 7.5, 0.0));
    let zero = profile(&[(0.0, 0.0), (1.0, 0.0)], false);
    assert_eq!(
        outline(&anchors, false, &shape, &zero),
        Err(VariableStrokeError::Outline(StrokeOutlineError::Empty))
    );
}

#[test]
fn zero_length_dashes_are_dots_of_the_local_width_unless_butt() {
    let anchors = [corner(0.0, 0.0), corner(40.0, 0.0)];
    let mut shape = style(1.0, LineCap::Round, LineJoin::Miter);
    shape.dash = DashPattern::new(vec![0.0, 10.0]).unwrap();
    let taper = profile(&[(0.0, 1.0), (1.0, 3.0)], false);
    let dots = outline(&anchors, false, &shape, &taper).unwrap();
    assert_eq!(areas(&dots).iter().filter(|a| **a > 0.0).count(), 4);
    shape.cap = LineCap::Butt;
    assert_eq!(
        outline(&anchors, false, &shape, &taper),
        Err(VariableStrokeError::Outline(StrokeOutlineError::Empty))
    );
}

#[test]
fn markers_keep_the_base_width_at_a_zero_width_end() {
    let anchors = [corner(0.0, 0.0), corner(40.0, 0.0)];
    let mut shape = style(2.0, LineCap::Butt, LineJoin::Miter);
    shape.markers = Markers {
        end: MarkerShape::Dot,
        ..Markers::default()
    };
    let taper_end = profile(&[(0.0, 1.0), (1.0, 0.0)], false);
    let result = outline(&anchors, false, &shape, &taper_end).unwrap();
    // A dot marker has a radius of 1.5 base widths.
    assert!(inside(&result, 42.9, 0.0) && !inside(&result, 43.1, 0.0));
}

#[test]
fn crossings_and_self_overlaps_are_drawn() {
    let crossing = [
        corner(0.0, 0.0),
        corner(20.0, 20.0),
        corner(20.0, 0.0),
        corner(0.0, 20.0),
    ];
    let taper = profile(&[(0.0, 0.5), (1.0, 2.0)], false);
    let result = outline(
        &crossing,
        false,
        &style(2.0, LineCap::Round, LineJoin::Round),
        &taper,
    )
    .unwrap();
    assert!(inside(&result, 10.0, 10.0));
}

#[test]
fn only_degenerate_input_is_refused() {
    let taper = profile(&[(0.0, 1.0), (1.0, 2.0)], false);
    let shape = style(2.0, LineCap::Butt, LineJoin::Miter);
    assert_eq!(
        outline(&[corner(5.0, 5.0), corner(5.0, 5.0)], false, &shape, &taper),
        Err(VariableStrokeError::ZeroLength)
    );
    assert_eq!(
        outline(
            &[corner(f64::NAN, 0.0), corner(5.0, 5.0)],
            false,
            &shape,
            &taper
        ),
        Err(VariableStrokeError::Outline(StrokeOutlineError::OutOfRange))
    );
    assert_eq!(
        outline(
            &[corner(1.0e7, 0.0), corner(5.0, 5.0)],
            false,
            &shape,
            &taper
        ),
        Err(VariableStrokeError::Outline(StrokeOutlineError::OutOfRange))
    );
}

#[test]
fn a_finer_tolerance_is_met_by_magnifying() {
    let anchors = [corner(1000.0, 0.0), corner(1010.0, 0.0)];
    let taper = profile(&[(0.0, 1.0), (1.0, 2.0)], false);
    let fine = variable_stroke_outline(
        Outline::new(&anchors, false),
        &style(0.2, LineCap::Butt, LineJoin::Miter),
        &taper,
        Tolerance::from_mm(0.001),
    )
    .unwrap();
    assert!(inside(&fine, 1009.9, 0.19) && !inside(&fine, 1009.9, 0.21));
}

#[test]
fn a_closed_dash_across_the_seam_is_joined_without_caps() {
    let square = [
        corner(0.0, 0.0),
        corner(40.0, 0.0),
        corner(40.0, 40.0),
        corner(0.0, 40.0),
    ];
    let mut shape = style(1.0, LineCap::Butt, LineJoin::Miter);
    // 0–30, 50–80, 100–130 and 150–160: the last dash runs on into the first.
    shape.dash = DashPattern::new(vec![30.0, 20.0]).unwrap();
    let growing = profile(&[(0.0, 1.0), (0.5, 2.0)], true);
    let result = outline(&square, true, &shape, &growing).unwrap();
    assert!(inside(&result, -0.4, -0.4), "the seam corner is mitred");
    assert_eq!(areas(&result).iter().filter(|a| **a > 0.0).count(), 3);
}
