//! Acceptance tests of `specs/0021-ellipse-arcs-and-shaping/specification.md` for the
//! interaction layer (handles, pointer inverses, bar state, snapping, announcements),
//! written from the spec against the public API of `curvyo-ui-core`.

#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::float_cmp
)]

use curvyo_document_core::{
    Angle, ArcKind, Curve, EllipseArc, EllipseFrame, InnerRatio, Length, ObjectSnapshot, Point,
    PointCount, Shape, StarFrame, curve_handle_position, ellipse_point,
};
use curvyo_ui_core::{
    ArcEnd, ParamHandle, ParamValue, TransformHandleTolerances, apply_param,
    arc_angle_from_pointer, arc_announcement, arc_capable, arc_readout, curve_announcement,
    curve_capable, curve_from_pointer, curve_readout, oriented_bounds, param_handles,
};
use proptest::prelude::*;

const DEG: f64 = std::f64::consts::PI / 180.0;

fn ang(deg: f64) -> Angle {
    Angle::from_radians(deg * DEG)
}

fn frame(rx: f64, ry: f64) -> EllipseFrame {
    EllipseFrame {
        center: Point::new(100.0, 100.0),
        rx: Length::from_mm(rx),
        ry: Length::from_mm(ry),
    }
}

fn ell(rx: f64, ry: f64, arc: Option<EllipseArc>, curve: f64) -> Shape {
    Shape::Ellipse {
        frame: frame(rx, ry),
        arc,
        curve: Curve::new(curve).unwrap(),
    }
}

fn arc(s: f64, e: f64, k: ArcKind) -> EllipseArc {
    EllipseArc::new(ang(s), ang(e), k).unwrap()
}

fn sframe(r: f64) -> StarFrame {
    StarFrame {
        center: Point::new(100.0, 100.0),
        radius: Length::from_mm(r),
        angle: ang(0.0),
    }
}

fn poly(n: u32, r: f64, curve: f64) -> Shape {
    Shape::Polygon {
        frame: sframe(r),
        point_count: PointCount::new(n).unwrap(),
        curve: Curve::new(curve).unwrap(),
    }
}

fn star(n: u32, ratio: f64, r: f64, curve: f64) -> Shape {
    Shape::Star {
        frame: sframe(r),
        point_count: PointCount::new(n).unwrap(),
        inner_ratio: InnerRatio::new(ratio).unwrap(),
        curve: Curve::new(curve).unwrap(),
    }
}

fn obj(shape: Shape, rot: f64) -> ObjectSnapshot {
    let d = curvyo_document_core::Document::new(1);
    let id = d.create_primitive(shape, ang(rot));
    d.object(id).unwrap()
}

fn shape_of(o: &ObjectSnapshot) -> Shape {
    match o {
        ObjectSnapshot::Primitive(p) => p.shape,
        _ => unreachable!(),
    }
}

/// `shape` with `value` applied through the one rule drag, entry and bar share; `None`
/// when nothing changes.
fn applied(shape: &Shape, value: ParamValue) -> Option<Shape> {
    let before = obj(*shape, 0.0);
    let after = apply_param(&before, value);
    (after != before).then(|| shape_of(&after))
}

fn with_arc_angle(shape: &Shape, end: ArcEnd, angle: Angle) -> Option<Shape> {
    applied(shape, ParamValue::ArcAngle(end, angle))
}

fn with_arc_kind(shape: &Shape, kind: ArcKind) -> Option<Shape> {
    applied(shape, ParamValue::ArcKind(kind))
}

fn made_whole(shape: &Shape) -> Option<Shape> {
    applied(shape, ParamValue::WholeEllipse)
}

fn with_curve(shape: &Shape, fraction: f64) -> Option<Shape> {
    applied(shape, ParamValue::Curve(fraction))
}

const SCALE: f64 = 4.0; // px per mm

fn knobs(o: &ObjectSnapshot) -> Vec<(ParamHandle, Point)> {
    param_handles(
        o,
        &oriented_bounds(o),
        &TransformHandleTolerances::at_scale(SCALE),
    )
}

fn find(k: &[(ParamHandle, Point)], h: ParamHandle) -> Option<Point> {
    k.iter().find(|(x, _)| *x == h).map(|(_, p)| *p)
}

fn dist(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn polar_deg(p: Point, c: Point) -> f64 {
    (p.y - c.y).atan2(p.x - c.x).to_degrees().rem_euclid(360.0)
}

// ---- 1, 22, 40, 31, 52: which knobs exist ------------------------------------------

#[test]
fn c1_c22_a_whole_ellipse_shows_both_arc_knobs_and_the_curve_knob() {
    let k = knobs(&obj(ell(20.0, 20.0, None, 1.0), 0.0));
    assert!(find(&k, ParamHandle::ArcStart).is_some());
    assert!(find(&k, ParamHandle::ArcEnd).is_some());
    assert!(find(&k, ParamHandle::Curve).is_some());
    assert_eq!(k.len(), 3);
}

#[test]
fn c1_resting_arc_knobs_sit_at_the_45_degree_seam_14_px_apart_end_first() {
    let c = Point::new(100.0, 100.0);
    let k = knobs(&obj(ell(20.0, 20.0, None, 1.0), 0.0));
    let (s, e) = (
        find(&k, ParamHandle::ArcStart).unwrap(),
        find(&k, ParamHandle::ArcEnd).unwrap(),
    );
    // on the outline
    assert!((dist(s, c) - 20.0).abs() < 0.05 && (dist(e, c) - 20.0).abs() < 0.05);
    // 14 px apart along the outline (chord within a hair of the arc)
    assert!(
        (dist(s, e) * SCALE - 14.0).abs() < 1.0,
        "{} px",
        dist(s, e) * SCALE
    );
    // end first, start second, clockwise: end < 45 < start (screen angle grows clockwise)
    let (ps, pe) = (polar_deg(s, c), polar_deg(e, c));
    assert!(pe < 45.0 && 45.0 < ps, "end {pe} start {ps}");
    assert!(
        (((ps + pe) / 2.0) - 45.0).abs() < 0.5,
        "centred on the seam"
    );
}

#[test]
fn c1_c53_knobs_follow_the_arc_angles_and_the_rotation() {
    for rot in [0.0, 30.0, 90.0, -45.0] {
        let a = arc(20.0, 250.0, ArcKind::Pie);
        let o = obj(ell(40.0, 25.0, Some(a), 1.0), rot);
        let k = knobs(&o);
        let c = Point::new(100.0, 100.0);
        for (h, t) in [(ParamHandle::ArcStart, 20.0), (ParamHandle::ArcEnd, 250.0)] {
            let at = find(&k, h).unwrap_or_else(|| panic!("{h:?} at rot {rot}"));
            let want = ellipse_point(frame(40.0, 25.0), t * DEG).rotated_around(c, ang(rot));
            // the true position, or displaced along the outline by the displacement rule:
            // it must lie on the (rotated) outline either way
            let local = at.rotated_around(c, ang(-rot));
            let (u, v) = ((local.x - 100.0) / 40.0, (local.y - 100.0) / 25.0);
            assert!(
                (u.hypot(v) - 1.0).abs() < 0.01,
                "{h:?} rot {rot} is on the outline"
            );
            assert!(
                dist(at, want) < 14.0 / SCALE + 0.1,
                "{h:?} rot {rot}: moved at most 14 px"
            );
        }
        assert!(
            find(&k, ParamHandle::Curve).is_none(),
            "an arc has no curve knob"
        );
    }
}

#[test]
fn c31_a_curved_ellipse_has_no_arc_knobs_and_an_arc_has_no_curve_knob() {
    let k = knobs(&obj(ell(20.0, 20.0, None, 0.4), 0.0));
    assert!(find(&k, ParamHandle::Curve).is_some());
    assert!(find(&k, ParamHandle::ArcStart).is_none() && find(&k, ParamHandle::ArcEnd).is_none());
    let k = knobs(&obj(
        ell(20.0, 20.0, Some(arc(10.0, 250.0, ArcKind::Arc)), 1.0),
        0.0,
    ));
    assert!(find(&k, ParamHandle::Curve).is_none());
    assert_eq!(k.len(), 2);
}

#[test]
fn c40_polygon_has_only_the_curve_knob_a_star_adds_its_inner_radius_rect_none_of_ours() {
    let k = knobs(&obj(poly(6, 20.0, 0.0), 0.0));
    assert_eq!(k.len(), 1);
    assert!(find(&k, ParamHandle::Curve).is_some());
    let k = knobs(&obj(star(5, 0.4, 20.0, 0.0), 0.0));
    assert!(find(&k, ParamHandle::Curve).is_some() && find(&k, ParamHandle::InnerRadius).is_some());
    assert_eq!(k.len(), 2);
    let rect = obj(
        Shape::Rect {
            bounds: curvyo_document_core::RectBounds::from_corners(
                Point::new(0.0, 0.0),
                Point::new(40.0, 40.0),
            ),
            corner_radii: curvyo_document_core::CornerRadii::uniform(Length::from_mm(0.0)),
        },
        0.0,
    );
    assert!(
        find(&knobs(&rect), ParamHandle::Curve).is_none(),
        "criterion 52"
    );
    assert!(!curve_capable(&match rect {
        ObjectSnapshot::Primitive(p) => p.shape,
        _ => unreachable!(),
    }));
}

#[test]
fn c1_c22_no_knobs_below_72_px_of_shorter_side() {
    // 17 mm * 4 = 68 px; 18.5 mm * 4 = 74 px (box side = 2r)
    let small = obj(ell(8.5, 8.5, None, 1.0), 0.0);
    assert!(knobs(&small).is_empty(), "68 px");
    let ok = obj(ell(9.25, 9.25, None, 1.0), 0.0);
    assert_eq!(knobs(&ok).len(), 3, "74 px");
    // an arc's threshold is measured on the drawn part (criteria 1 and 9): a half circle of
    // radius 10 mm is 20 x 10 mm = 40 px high -> no knobs; of radius 20 mm it is 80 px
    // high -> both arc knobs
    let small = obj(
        ell(10.0, 10.0, Some(arc(0.0, 180.0, ArcKind::Arc)), 1.0),
        0.0,
    );
    assert!(knobs(&small).is_empty(), "half circle r=10: 40 px");
    let half = obj(
        ell(20.0, 20.0, Some(arc(0.0, 180.0, ArcKind::Arc)), 1.0),
        0.0,
    );
    assert_eq!(knobs(&half).len(), 2, "half circle r=20: 80 px");
}

#[test]
fn c22_curve_knob_position_follows_criterion_19_and_rotation() {
    let c = Point::new(100.0, 100.0);
    for rot in [0.0, 30.0] {
        for k in [1.0, 0.5, 0.0, -0.5, -1.0] {
            let o = obj(ell(20.0, 20.0, None, k), rot);
            let at = find(&knobs(&o), ParamHandle::Curve).unwrap();
            let d = 0.5 * (1.0 + (22.5 * DEG * k).tan());
            // upper right of the unrotated ellipse: x positive, y negative (screen)
            let want = Point::new(100.0 + 20.0 * d, 100.0 - 20.0 * d).rotated_around(c, ang(rot));
            // may be displaced by the 14 px rule; if not displaced it must be exact
            if dist(at, want) > 1e-6 {
                assert!(dist(at, want) <= 14.0 / SCALE + 1e-6, "k={k} rot={rot}");
            }
        }
    }
    // at 100% the knob is on the outline at 315 degrees
    let at = find(
        &knobs(&obj(ell(20.0, 20.0, None, 1.0), 0.0)),
        ParamHandle::Curve,
    )
    .unwrap();
    assert!((polar_deg(at, c) - 315.0).abs() < 0.2 && (dist(at, c) - 20.0).abs() < 0.05);
}

#[test]
fn c1_c40_no_two_knobs_closer_than_14_px_pitch_unless_a_sliver() {
    // star: inner-radius knob and curve knob on opposite sides of vertex 0
    for k in [-0.5, 0.0, 0.5, 1.0] {
        let o = obj(star(5, 0.4, 20.0, k), 0.0);
        let kn = knobs(&o);
        let a = find(&kn, ParamHandle::Curve).unwrap();
        let b = find(&kn, ParamHandle::InnerRadius).unwrap();
        assert!(
            dist(a, b) * SCALE >= 4.0 + 10.0 - 0.01,
            "k={k}: {} px",
            dist(a, b) * SCALE
        );
    }
}

// ---- 2, 7, 8: arc angle from the pointer ------------------------------------------------

#[test]
fn c2_angle_is_atan2_in_the_ellipse_frame_inside_or_outside() {
    let s = ell(30.0, 10.0, None, 1.0);
    for t in [0.0, 33.0, 90.0, 180.0, 271.0] {
        let p = ellipse_point(frame(30.0, 10.0), t * DEG);
        // on the outline, inside (0.4x) and far outside (3x): the same parametric angle
        for scale in [1.0, 0.4, 3.0] {
            let q = Point::new(100.0 + (p.x - 100.0) * scale, 100.0 + (p.y - 100.0) * scale);
            let got = arc_angle_from_pointer(&s, q, false)
                .unwrap()
                .as_radians()
                .rem_euclid(std::f64::consts::TAU);
            assert!((got - t * DEG).abs() < 1e-9, "t={t} scale={scale}: {got}");
        }
    }
    assert!(
        arc_angle_from_pointer(&s, Point::new(100.0, 100.0), false).is_none(),
        "the centre itself"
    );
}

#[test]
fn c7_ctrl_snaps_to_15_and_22_5_stops() {
    let s = ell(20.0, 20.0, None, 1.0);
    let at = |deg: f64| {
        let q = Point::new(
            100.0 + 30.0 * (deg * DEG).cos(),
            100.0 + 30.0 * (deg * DEG).sin(),
        );
        arc_angle_from_pointer(&s, q, true)
            .unwrap()
            .as_radians()
            .to_degrees()
            .rem_euclid(360.0)
    };
    // stops: multiples of 15 and of 22.5 (0, 15, 22.5, 30, 45, 60, 67.5, 75, 90, ...)
    for (input, want) in [
        (1.0, 0.0),
        (14.0, 15.0),
        (21.0, 22.5),
        (31.0, 30.0),
        (44.0, 45.0),
        (68.0, 67.5),
        (89.0, 90.0),
        (-3.0, 0.0),
        (-20.0, -22.5_f64.rem_euclid(360.0)),
    ] {
        let got = at(input);
        let want = f64::rem_euclid(want, 360.0);
        assert!(
            (got - want).abs() < 1e-6 || (got - want).abs() > 359.99999,
            "{input} -> {got}, want {want}"
        );
    }
    // without ctrl: free
    let q = Point::new(
        100.0 + 30.0 * (13.0 * DEG).cos(),
        100.0 + 30.0 * (13.0 * DEG).sin(),
    );
    let free = arc_angle_from_pointer(&s, q, false)
        .unwrap()
        .as_radians()
        .to_degrees();
    assert!((free - 13.0).abs() < 1e-9);
}

#[test]
fn c2_c3_c5_c64_first_drag_of_a_handle_on_a_whole_ellipse() {
    let s = ell(20.0, 20.0, None, 1.0);
    // dragging the start handle: the end rests at 45 degrees, the kind is Arc
    let out = with_arc_angle(&s, ArcEnd::Start, ang(30.0)).unwrap();
    match out {
        Shape::Ellipse { arc: Some(a), .. } => {
            assert_eq!(a.kind, ArcKind::Arc);
            assert!((a.start().as_radians() - 30.0 * DEG).abs() < 1e-12);
            assert!((a.end().as_radians() - 45.0 * DEG).abs() < 1e-12);
        }
        o => panic!("{o:?}"),
    }
    let out = with_arc_angle(&s, ArcEnd::End, ang(-90.0)).unwrap();
    match out {
        Shape::Ellipse { arc: Some(a), .. } => {
            assert!(
                (a.end().as_radians() - 270.0 * DEG).abs() < 1e-12,
                "-90 becomes 270"
            );
            assert!((a.start().as_radians() - 45.0 * DEG).abs() < 1e-12);
        }
        o => panic!("{o:?}"),
    }
    // within 0.1 degrees of the other angle: whole
    assert!(
        with_arc_angle(&s, ArcEnd::Start, ang(45.0)).is_none(),
        "equal -> whole, nothing to write"
    );
    let arced = ell(20.0, 20.0, Some(arc(10.0, 100.0, ArcKind::Pie)), 1.0);
    match with_arc_angle(&arced, ArcEnd::End, ang(10.05)).unwrap() {
        Shape::Ellipse { arc: None, .. } => {}
        o => panic!("equal within 0.1 degrees is whole: {o:?}"),
    }
    // typed values: -90 -> 270, 360 -> 0, 450 -> 90
    for (typed, want) in [(-90.0, 270.0), (360.0, 0.0), (450.0, 90.0)] {
        match with_arc_angle(&arced, ArcEnd::End, ang(typed)).unwrap() {
            Shape::Ellipse { arc: Some(a), .. } => {
                assert!((a.end().as_radians() / DEG - want).abs() < 1e-9, "{typed}");
                assert_eq!(a.kind, ArcKind::Pie, "the kind does not change");
                assert!(
                    (a.start().as_radians() / DEG - 10.0).abs() < 1e-9,
                    "the other angle does not change"
                );
            }
            o => panic!("{o:?}"),
        }
    }
    // not applicable on a curved ellipse or a polygon
    assert!(with_arc_angle(&ell(20.0, 20.0, None, 0.5), ArcEnd::Start, ang(30.0)).is_none());
    assert!(with_arc_angle(&poly(5, 20.0, 0.0), ArcEnd::Start, ang(30.0)).is_none());
    assert!(arc_capable(&ell(20.0, 20.0, None, 0.5)).is_none());
}

#[test]
fn c3_kind_and_whole_ellipse() {
    let a = ell(20.0, 20.0, Some(arc(10.0, 100.0, ArcKind::Arc)), 1.0);
    match with_arc_kind(&a, ArcKind::Chord).unwrap() {
        Shape::Ellipse { arc: Some(x), .. } => {
            assert_eq!(x.kind, ArcKind::Chord);
            assert_eq!(x, arc(10.0, 100.0, ArcKind::Chord), "angles kept");
        }
        o => panic!("{o:?}"),
    }
    assert!(
        with_arc_kind(&a, ArcKind::Arc).is_none(),
        "no change, nothing to write"
    );
    assert!(
        with_arc_kind(&ell(20.0, 20.0, None, 1.0), ArcKind::Pie).is_none(),
        "a whole ellipse has no kind"
    );
    assert!(matches!(
        made_whole(&a),
        Some(Shape::Ellipse { arc: None, .. })
    ));
    assert!(made_whole(&ell(20.0, 20.0, None, 1.0)).is_none());
}

// ---- 23, 41: curve from the pointer --------------------------------------------------------

#[test]
fn c23_curve_pointer_is_the_exact_inverse_of_the_handle_position() {
    for (rx, ry) in [(20.0, 20.0), (30.0, 10.0)] {
        for k in [-0.9, -0.62, -0.3, 0.2, 0.4, 0.77, 0.93] {
            let s = ell(rx, ry, None, k);
            let at = curve_handle_position(&s).unwrap();
            let back = curve_from_pointer(&s, at, false).unwrap();
            assert!((back - k).abs() < 1e-9, "rx={rx} k={k}: {back}");
        }
    }
    // criterion 23's own formula: u+v-1 = tan(22.5 k)
    let s = ell(20.0, 10.0, None, 0.5);
    for &(u, v) in &[(0.6, 0.5), (0.3, 0.3), (0.9, 0.9)] {
        let p = Point::new(100.0 + 20.0 * u, 100.0 - 10.0 * v);
        let want = ((u + v - 1.0).atan().to_degrees() / 22.5).clamp(-1.0, 1.0);
        let got = curve_from_pointer(&s, p, false).unwrap();
        let snapped = [1.0, 0.0, -1.0]
            .iter()
            .any(|st| (want - st).abs() <= 0.0101);
        if !snapped {
            assert!((got - want).abs() < 1e-9, "({u},{v}): {got} vs {want}");
        }
    }
}

#[test]
fn c23_snaps_within_one_point_of_100_0_and_the_limit_and_ctrl_to_25() {
    let s = ell(20.0, 20.0, None, 0.5);
    let at = |k: f64| curve_handle_position(&ell(20.0, 20.0, None, k)).unwrap();
    assert_eq!(curve_from_pointer(&s, at(0.995), false), Some(1.0));
    assert_eq!(curve_from_pointer(&s, at(0.005), false), Some(0.0));
    assert_eq!(curve_from_pointer(&s, at(-0.995), false), Some(-1.0));
    let v = curve_from_pointer(&s, at(0.97), false).unwrap();
    assert!((v - 0.97).abs() < 1e-9, "outside the 1 point zone: {v}");
    // ctrl: multiples of 25 points
    for (k, want) in [(0.37, 0.25), (0.63, 0.75), (-0.12, 0.0), (-0.4, -0.5)] {
        let v = curve_from_pointer(&s, at(k), true).unwrap();
        assert!((v - want).abs() < 1e-9, "{k} -> {v}, want {want}");
    }
    // limited to [-1, 1]
    let far = Point::new(100.0 + 200.0, 100.0 - 200.0);
    assert_eq!(curve_from_pointer(&s, far, false), Some(1.0));
    let tri = poly(3, 20.0, 0.0);
    let deep = Point::new(100.0, 100.0);
    let v = curve_from_pointer(&tri, deep, false).unwrap();
    assert!(v >= -0.5 - 1e-12, "a triangle stops at -50%: {v}");
}

#[test]
fn c41_polygon_and_star_pointer_inverse() {
    for shape_of in [
        (|k| poly(6, 20.0, k)) as fn(f64) -> Shape,
        |k| poly(3, 20.0, k),
        |k| poly(12, 20.0, k),
        |k| star(5, 0.4, 20.0, k),
        |k| star(7, 0.6, 20.0, k),
    ] {
        for k in [-0.4, -0.2, 0.15, 0.5, 0.8] {
            let s = shape_of(k);
            let at = curve_handle_position(&s).unwrap();
            let back = curve_from_pointer(&s, at, false).unwrap();
            assert!((back - k).abs() < 1e-9, "{s:?} k={k}: {back}");
        }
    }
}

proptest! {
    #[test]
    fn c41_inverse_holds_for_random_shapes(n in 3u32..60, ratio in 0.05f64..0.95, frac in 0.0f64..1.0, is_star in any::<bool>()) {
        let proto = if is_star { star(n, ratio, 20.0, 0.0) } else { poly(n, 20.0, 0.0) };
        let limit = curvyo_document_core::curve_limit(&proto).unwrap().get();
        let k = limit + (1.0 - limit) * frac;
        // keep clear of the 1-point snap zones
        prop_assume!((k - 1.0).abs() > 0.02 && k.abs() > 0.02 && (k - limit).abs() > 0.02);
        let s = if is_star { star(n, ratio, 20.0, k) } else { poly(n, 20.0, k) };
        let at = curve_handle_position(&s).unwrap();
        let back = curve_from_pointer(&s, at, false).unwrap();
        prop_assert!((back - k).abs() < 1e-7, "{back} vs {k}");
    }

    #[test]
    fn c2_pointer_angle_inverse_on_ellipses(rx in 5.0f64..60.0, ry in 5.0f64..60.0, t in 0.0f64..360.0, far in 0.2f64..4.0) {
        let f = frame(rx, ry);
        let p = ellipse_point(f, t * DEG);
        let q = Point::new(100.0 + (p.x - 100.0) * far, 100.0 + (p.y - 100.0) * far);
        let s = ell(rx, ry, None, 1.0);
        let got = arc_angle_from_pointer(&s, q, false).unwrap().as_radians().rem_euclid(std::f64::consts::TAU);
        let d = (got - t * DEG).abs();
        prop_assert!(d < 1e-9 || (std::f64::consts::TAU - d) < 1e-9);
    }
}

// ---- 26, 38, 42, 44: limits through `with_curve` and `apply_param` -------------------------

#[test]
fn c38_c42_c44_typed_values_are_limited_per_shape() {
    let tri = poly(3, 20.0, 0.0);
    match with_curve(&tri, -1.0).unwrap() {
        Shape::Polygon { curve, .. } => {
            assert!((curve.get() + 0.5).abs() < 1e-9, "triangle stops at -50")
        }
        o => panic!("{o:?}"),
    }
    let hex = poly(6, 20.0, 0.0);
    match with_curve(&hex, -1.0).unwrap() {
        Shape::Polygon { curve, .. } => assert!((curve.get() + 1.0).abs() < 1e-9),
        o => panic!("{o:?}"),
    }
    match with_curve(&hex, 5.0).unwrap() {
        Shape::Polygon { curve, .. } => {
            assert!((curve.get() - 1.0).abs() < 1e-9, "above 100 limited")
        }
        o => panic!("{o:?}"),
    }
    // pentagram star ratio 0.2 limit about -44.4%
    match with_curve(&star(5, 0.2, 20.0, 0.0), -1.0).unwrap() {
        Shape::Star { curve, .. } => assert!((curve.get() + 0.444).abs() < 0.002),
        o => panic!("{o:?}"),
    }
    assert!(with_curve(&hex, f64::NAN).is_none());
    assert!(
        with_curve(&hex, 0.0).is_none(),
        "no change, nothing written"
    );
    // ellipse: typing 100 gives the ellipse back; with an arc the curve cannot be set
    assert!(
        matches!(with_curve(&ell(20.0, 20.0, None, 0.3), 1.0), Some(Shape::Ellipse { curve, .. }) if curve == Curve::ELLIPSE_DEFAULT)
    );
    assert!(
        with_curve(
            &ell(20.0, 20.0, Some(arc(1.0, 90.0, ArcKind::Arc)), 1.0),
            0.5
        )
        .is_none()
    );
    assert!(!curve_capable(&ell(
        20.0,
        20.0,
        Some(arc(1.0, 90.0, ArcKind::Arc)),
        1.0
    )));
}

#[test]
fn c26_the_bar_edit_goes_through_apply_param_and_leaves_other_parameters_alone() {
    let o = obj(ell(30.0, 10.0, None, 1.0), 25.0);
    let out = apply_param(&o, ParamValue::Curve(-0.35));
    let (ObjectSnapshot::Primitive(a), ObjectSnapshot::Primitive(b)) = (&o, &out) else {
        panic!()
    };
    assert_eq!(a.rotation, b.rotation);
    match (a.shape, b.shape) {
        (
            Shape::Ellipse { frame: f1, .. },
            Shape::Ellipse {
                frame: f2, curve, ..
            },
        ) => {
            assert_eq!(f1, f2);
            assert!((curve.get() + 0.35).abs() < 1e-12);
        }
        _ => panic!(),
    }
    // ArcAngle through the bar
    let out = apply_param(&o, ParamValue::ArcAngle(ArcEnd::End, ang(450.0)));
    match out {
        ObjectSnapshot::Primitive(p) => match p.shape {
            Shape::Ellipse { arc: Some(a), .. } => {
                assert!((a.end().as_radians() / DEG - 90.0).abs() < 1e-9)
            }
            s => panic!("{s:?}"),
        },
        _ => panic!(),
    }
    // WholeEllipse on a whole ellipse is a no-op
    assert_eq!(apply_param(&o, ParamValue::WholeEllipse), o);
}

// ---- 6, 25, 59, 65: readouts and announcements ------------------------------------------------

#[test]
fn c6_c25_c65_readout_and_announcement_texts() {
    let s = ell(20.0, 20.0, Some(arc(30.0, 150.0, ArcKind::Pie)), 1.0);
    assert_eq!(
        arc_readout(&s, ArcEnd::Start).as_deref(),
        Some("start 30.0\u{b0} \u{b7} sweep 120.0\u{b0}")
    );
    let a = arc_announcement(&s).unwrap();
    assert_eq!(a, "Arc start 30.0\u{b0}, end 150.0\u{b0}, pie");
    let c = ell(20.0, 20.0, None, -0.35);
    assert_eq!(curve_readout(&c).as_deref(), Some("curve -35.0 %"));
    let ann = curve_announcement(&c).unwrap();
    assert!(
        ann == "Curve \u{2212}35.0%" || ann == "Curve -35.0%",
        "{ann}"
    );
    assert!(
        curve_announcement(&poly(6, 20.0, 0.5))
            .unwrap()
            .contains("50.0%")
    );
    // the end readout names the end angle
    let e = arc_readout(&s, ArcEnd::End).unwrap();
    assert!(e.contains("150.0") && e.contains("120.0"), "{e}");
}

// ---- 9: box of the drawn part is the oriented box --------------------------------------------

#[test]
fn c9_oriented_box_is_the_drawn_part_in_the_rotated_frame() {
    let o = obj(
        ell(20.0, 20.0, Some(arc(0.0, 180.0, ArcKind::Arc)), 1.0),
        30.0,
    );
    let b = oriented_bounds(&o);
    assert!(
        (b.width() - 40.0).abs() < 1e-9 && (b.height() - 20.0).abs() < 1e-9,
        "{} x {}",
        b.width(),
        b.height()
    );
    assert!((b.angle.as_radians() - 30.0 * DEG).abs() < 1e-9);
}
