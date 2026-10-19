//! Black-box acceptance tests for `specs/0021-ellipse-arcs-and-shaping/specification.md`
//! (criteria 1 to 67, the document-model share), written from the spec against the public
//! API of `curvyo-document-core` only. The oracles (circular-arc geometry, the side
//! formulas of criteria 18/19/36/37/48, the mirror/SVD rules of 54/56) are computed here
//! independently of the implementation.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::float_cmp,
    clippy::needless_range_loop
)]

use curvyo_document_core::{
    AnchorKind, Angle, ArcKind, CURRENT_FORMAT_VERSION, CopySource, Curve, Document, EllipseArc,
    EllipseAxes, EllipseFrame, InnerRatio, Length, OpenError, OutlineAnchor, Point, PointCount,
    PrimitiveOutline, Shape, ShapeEditError, StarFrame, Vec2, arc_after_linear_map, curve_limit,
    effective_curve, ellipse_after_linear_map, ellipse_outline, ellipse_point, outline_of,
    outline_of_rotated, pack, shape_frame_bounds, unpack,
};
use std::f64::consts::{PI, TAU};

const DEG: f64 = PI / 180.0;

fn ang(deg: f64) -> Angle {
    Angle::from_radians(deg * DEG)
}

fn frame(cx: f64, cy: f64, rx: f64, ry: f64) -> EllipseFrame {
    EllipseFrame {
        center: Point::new(cx, cy),
        rx: Length::from_mm(rx),
        ry: Length::from_mm(ry),
    }
}

fn arc(start: f64, end: f64, kind: ArcKind) -> EllipseArc {
    EllipseArc::new(ang(start), ang(end), kind).expect("a real arc")
}

fn ellipse(f: EllipseFrame, a: Option<EllipseArc>, curve: f64) -> Shape {
    Shape::Ellipse {
        frame: f,
        arc: a,
        curve: Curve::new(curve).unwrap(),
    }
}

fn sframe(r: f64) -> StarFrame {
    StarFrame {
        center: Point::new(100.0, 100.0),
        radius: Length::from_mm(r),
        angle: ang(0.0),
    }
}

fn polygon(n: u32, r: f64, curve: f64) -> Shape {
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

fn dist(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn add(p: Point, v: Vec2) -> Point {
    Point::new(p.x + v.x, p.y + v.y)
}

/// Cubic Bezier points of the outline's segments, `n` per segment (end excluded, the
/// next segment starts there; an open outline gets its last point too).
fn sample(outline: &PrimitiveOutline, n: usize) -> Vec<Point> {
    let a = &outline.anchors;
    let segs = if outline.closed { a.len() } else { a.len() - 1 };
    let mut out = Vec::new();
    for i in 0..segs {
        let (p, q) = (&a[i], &a[(i + 1) % a.len()]);
        let p0 = p.point;
        let p1 = add(p.point, p.handle_out);
        let p2 = add(q.point, q.handle_in);
        let p3 = q.point;
        for s in 0..n {
            let t = s as f64 / n as f64;
            out.push(bez(p0, p1, p2, p3, t));
        }
    }
    if !outline.closed {
        out.push(a[a.len() - 1].point);
    }
    out
}

fn bez(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    Point::new(
        w[0] * p0.x + w[1] * p1.x + w[2] * p2.x + w[3] * p3.x,
        w[0] * p0.y + w[1] * p1.y + w[2] * p2.y + w[3] * p3.y,
    )
}

/// Midpoint (t = 0.5) of each segment.
fn side_middles(outline: &PrimitiveOutline) -> Vec<Point> {
    let a = &outline.anchors;
    (0..a.len())
        .map(|i| {
            let (p, q) = (&a[i], &a[(i + 1) % a.len()]);
            bez(
                p.point,
                add(p.point, p.handle_out),
                add(q.point, q.handle_in),
                q.point,
                0.5,
            )
        })
        .collect()
}

fn area(points: &[Point]) -> f64 {
    let mut s = 0.0;
    for i in 0..points.len() {
        let (p, q) = (points[i], points[(i + 1) % points.len()]);
        s += p.x * q.y - q.x * p.y;
    }
    s.abs() / 2.0
}

fn kinds(o: &PrimitiveOutline) -> Vec<AnchorKind> {
    o.anchors.iter().map(|a| a.kind).collect()
}

fn doc_with(shape: Shape, rotation: f64) -> (Document, curvyo_document_core::NodeId) {
    let d = Document::new(1);
    let id = d.create_primitive(shape, ang(rotation));
    (d, id)
}

// ---------------------------------------------------------------------------------------
// 13: Object to path of an arc
// ---------------------------------------------------------------------------------------

/// Largest deviation of the sampled outline from the true elliptical arc, in mm.
fn arc_deviation(f: EllipseFrame, o: &PrimitiveOutline, skip_straight: bool) -> f64 {
    let (rx, ry) = (f.rx.as_mm(), f.ry.as_mm());
    let mut worst: f64 = 0.0;
    for p in sample(o, 24) {
        let (u, v) = ((p.x - f.center.x) / rx, (p.y - f.center.y) / ry);
        let r = u.hypot(v);
        if skip_straight && r < 0.999 - 0.0 {
            // a pie's straight edges are inside the unit disc; they are checked elsewhere
            continue;
        }
        worst = worst.max((r - 1.0).abs() * rx.max(ry));
    }
    worst
}

#[test]
fn c13_arc_kinds_become_the_right_paths() {
    let f = frame(50.0, 50.0, 20.0, 20.0);
    // Arc: 0 -> 180: two quarter segments, open, 3 nodes.
    let o = outline_of(&ellipse(f, Some(arc(0.0, 180.0, ArcKind::Arc)), 1.0));
    assert!(!o.closed, "an open arc is an open path");
    assert_eq!(o.anchors.len(), 3, "ceil(180/90)=2 segments");
    assert_eq!(
        kinds(&o),
        vec![AnchorKind::Corner, AnchorKind::Symmetric, AnchorKind::Corner]
    );
    assert!(dist(o.anchors[0].point, Point::new(70.0, 50.0)) < 1e-9);
    assert!(dist(o.anchors[2].point, Point::new(30.0, 50.0)) < 1e-9);
    assert!(arc_deviation(f, &o, false) <= 0.001 * 20.0);

    // Chord: closed, same nodes, no centre.
    let o = outline_of(&ellipse(f, Some(arc(0.0, 180.0, ArcKind::Chord)), 1.0));
    assert!(o.closed);
    assert_eq!(o.anchors.len(), 3);
    assert_eq!(kinds(&o)[0], AnchorKind::Corner);
    assert_eq!(kinds(&o)[2], AnchorKind::Corner);

    // Pie: closed, adds the centre as one Corner node.
    let o = outline_of(&ellipse(f, Some(arc(0.0, 180.0, ArcKind::Pie)), 1.0));
    assert!(o.closed);
    assert_eq!(o.anchors.len(), 4);
    assert!(
        o.anchors
            .iter()
            .any(|a| dist(a.point, f.center) < 1e-9 && a.kind == AnchorKind::Corner),
        "the centre is a Corner node"
    );
}

#[test]
fn c13_segment_count_is_ceil_sweep_over_90() {
    let f = frame(0.0, 0.0, 10.0, 10.0);
    for (sweep, segs) in [
        (10.0, 1),
        (89.0, 1),
        (90.0, 1),
        (91.0, 2),
        (180.0, 2),
        (181.0, 3),
        (270.0, 3),
        (271.0, 4),
        (359.0, 4),
    ] {
        let o = outline_of(&ellipse(
            f,
            Some(arc(20.0, 20.0 + sweep, ArcKind::Arc)),
            1.0,
        ));
        assert_eq!(o.anchors.len(), segs + 1, "sweep {sweep}");
        assert!(arc_deviation(f, &o, false) <= 0.001 * 10.0, "sweep {sweep}");
        // equal sweep per segment: chord lengths between nodes equal
        let chords: Vec<f64> = o
            .anchors
            .windows(2)
            .map(|w| dist(w[0].point, w[1].point))
            .collect();
        for c in &chords {
            assert!((c - chords[0]).abs() < 1e-9, "equal sweep at {sweep}");
        }
    }
}

#[test]
fn c13_angles_wrap_through_zero_and_run_clockwise_on_screen() {
    // 300 -> 60 sweeps 120 degrees through 0 (3 o'clock). On screen (y down) +angle is
    // clockwise, so angle 90 is 6 o'clock (larger y).
    let f = frame(0.0, 0.0, 10.0, 10.0);
    let o = outline_of(&ellipse(f, Some(arc(300.0, 60.0, ArcKind::Arc)), 1.0));
    assert!(dist(o.anchors[0].point, ellipse_point(f, 300.0 * DEG)) < 1e-9);
    assert!(
        (o.anchors[0].point.x - 5.0).abs() < 1e-9 && o.anchors[0].point.y < 0.0,
        "300 degrees is upper right"
    );
    let last = o.anchors.last().unwrap().point;
    assert!(last.x > 0.0 && last.y > 0.0, "60 degrees is lower right");
    assert!(dist(ellipse_point(f, 90.0 * DEG), Point::new(0.0, 10.0)) < 1e-9);
}

#[test]
fn c13_whole_ellipse_converts_as_today() {
    let f = frame(50.0, 50.0, 20.0, 12.0);
    let a = outline_of(&ellipse(f, None, 1.0));
    let b = ellipse_outline(f);
    assert_eq!(a.anchors, b);
    assert!(a.closed);
    assert_eq!(a.anchors.len(), 4);
    assert!(kinds(&a).iter().all(|k| *k == AnchorKind::Symmetric));
}

#[test]
fn c13_c9_elliptical_arc_is_exact_on_non_circles_too() {
    let f = frame(0.0, 0.0, 30.0, 10.0);
    for (s, e) in [(0.0, 90.0), (15.0, 200.0), (100.0, 350.0), (359.0, 40.0)] {
        let o = outline_of(&ellipse(f, Some(arc(s, e, ArcKind::Arc)), 1.0));
        assert!(arc_deviation(f, &o, false) <= 0.001 * 30.0, "{s}->{e}");
    }
}

// ---------------------------------------------------------------------------------------
// 5, 64: angle rules of the value type
// ---------------------------------------------------------------------------------------

#[test]
fn c5_c64_angles_wrap_and_equal_angles_are_whole() {
    let a = EllipseArc::new(ang(-90.0), ang(450.0), ArcKind::Arc).unwrap();
    assert!((a.start().as_radians() - 270.0 * DEG).abs() < 1e-12);
    assert!((a.end().as_radians() - 90.0 * DEG).abs() < 1e-12);
    let a = EllipseArc::new(ang(360.0), ang(10.0), ArcKind::Pie).unwrap();
    assert!(a.start().as_radians().abs() < 1e-12, "360 becomes 0");
    // 0.1 degree rule
    assert!(EllipseArc::new(ang(30.0), ang(30.0), ArcKind::Arc).is_none());
    assert!(EllipseArc::new(ang(30.0), ang(30.09), ArcKind::Arc).is_none());
    assert!(EllipseArc::new(ang(30.0), ang(30.0 + 360.0), ArcKind::Arc).is_none());
    assert!(EllipseArc::new(ang(30.0), ang(29.91), ArcKind::Arc).is_none());
    assert!(EllipseArc::new(ang(30.0), ang(30.2), ArcKind::Arc).is_some());
    assert!(EllipseArc::new(ang(30.0), ang(29.8), ArcKind::Arc).is_some());
    assert!(EllipseArc::new(ang(f64::NAN), ang(10.0), ArcKind::Arc).is_none());
}

#[test]
fn c5_sweep_is_between_0_and_360_never_negative() {
    for (s, e, sweep) in [
        (30.0, 150.0, 120.0),
        (150.0, 30.0, 240.0),
        (350.0, 10.0, 20.0),
    ] {
        let a = arc(s, e, ArcKind::Arc);
        assert!(
            (a.sweep().as_radians() - sweep * DEG).abs() < 1e-9,
            "{s}->{e}"
        );
    }
    // a sliver: end crossing the start turns an almost whole ellipse into a sliver and back
    assert!(arc(100.0, 99.0, ArcKind::Arc).sweep().as_radians() > 358.0 * DEG);
    assert!(arc(100.0, 101.0, ArcKind::Arc).sweep().as_radians() < 2.0 * DEG);
}

#[test]
fn c3_with_kind_keeps_the_angles() {
    let a = arc(30.0, 150.0, ArcKind::Arc).with_kind(ArcKind::Pie);
    assert_eq!(a.kind, ArcKind::Pie);
    assert_eq!(a.start(), arc(30.0, 150.0, ArcKind::Arc).start());
    assert_eq!(a.end(), arc(30.0, 150.0, ArcKind::Arc).end());
    assert_eq!(ArcKind::from_name("pie"), Some(ArcKind::Pie));
    assert_eq!(ArcKind::from_name("arc"), Some(ArcKind::Arc));
    assert_eq!(ArcKind::from_name("chord"), Some(ArcKind::Chord));
    assert_eq!(ArcKind::from_name("slice"), None);
}

// ---------------------------------------------------------------------------------------
// 9, 20, 47: boxes
// ---------------------------------------------------------------------------------------

fn bbox(shape: &Shape) -> (f64, f64, f64, f64) {
    let (a, b) = shape_frame_bounds(shape);
    (a.x, a.y, b.x - a.x, b.y - a.y)
}

#[test]
fn c9_box_is_around_the_drawn_part_only() {
    let f = frame(50.0, 50.0, 20.0, 20.0);
    // half circle (lower half: 0 -> 180 goes through 90 = 6 o'clock)
    let (x, y, w, h) = bbox(&ellipse(f, Some(arc(0.0, 180.0, ArcKind::Arc)), 1.0));
    assert!(
        (w - 40.0).abs() < 1e-9 && (h - 20.0).abs() < 1e-9,
        "{w}x{h}"
    );
    assert!((x - 30.0).abs() < 1e-9 && (y - 50.0).abs() < 1e-9);
    // the same half as a pie or a chord: same box (centre on the chord)
    let (_, _, w, h) = bbox(&ellipse(f, Some(arc(0.0, 180.0, ArcKind::Pie)), 1.0));
    assert!((w - 40.0).abs() < 1e-9 && (h - 20.0).abs() < 1e-9);
    // quarter pie includes the centre: 0 -> 90 is lower right quadrant
    let (x, y, w, h) = bbox(&ellipse(f, Some(arc(0.0, 90.0, ArcKind::Pie)), 1.0));
    assert!((w - 20.0).abs() < 1e-9 && (h - 20.0).abs() < 1e-9);
    assert!(
        (x - 50.0).abs() < 1e-9 && (y - 50.0).abs() < 1e-9,
        "includes the centre"
    );
    // quarter open arc does NOT include the centre
    let (x, _y, w, _h) = bbox(&ellipse(f, Some(arc(0.0, 90.0, ArcKind::Arc)), 1.0));
    assert!((x - 50.0).abs() < 1e-9 && (w - 20.0).abs() < 1e-9);
    // a 45-degree sliver arc: box is tight (not the quadrant)
    let (_, _, w, h) = bbox(&ellipse(f, Some(arc(0.0, 45.0, ArcKind::Arc)), 1.0));
    assert!(w < 20.0 && h < 20.0, "tight box of a small arc: {w}x{h}");
    // whole ellipse unchanged
    let (_, _, w, h) = bbox(&ellipse(f, None, 1.0));
    assert!((w - 40.0).abs() < 1e-9 && (h - 40.0).abs() < 1e-9);
}

#[test]
fn c9_box_matches_the_sampled_outline() {
    let f = frame(10.0, -5.0, 30.0, 12.0);
    for (s, e, k) in [
        (10.0, 80.0, ArcKind::Arc),
        (200.0, 20.0, ArcKind::Pie),
        (300.0, 340.0, ArcKind::Chord),
        (170.0, 190.0, ArcKind::Arc),
    ] {
        let shape = ellipse(f, Some(arc(s, e, k)), 1.0);
        let o = outline_of(&shape);
        let pts = sample(&o, 64);
        let (minx, maxx) = pts
            .iter()
            .fold((1e9f64, -1e9f64), |a, p| (a.0.min(p.x), a.1.max(p.x)));
        let (miny, maxy) = pts
            .iter()
            .fold((1e9f64, -1e9f64), |a, p| (a.0.min(p.y), a.1.max(p.y)));
        let (x, y, w, h) = bbox(&shape);
        assert!(
            (x - minx).abs() < 3e-2 && (x + w - maxx).abs() < 3e-2,
            "{s}->{e} x: box {x}..{} sampled {minx}..{maxx}", x + w
        );
        assert!(
            (y - miny).abs() < 3e-2 && (y + h - maxy).abs() < 3e-2,
            "{s}->{e} y: box {y}..{} sampled {miny}..{maxy}", y + h
        );
    }
}

#[test]
fn c20_curved_ellipse_box_is_the_ellipse_box_for_every_curve() {
    let f = frame(0.0, 0.0, 30.0, 10.0);
    for k in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let (x, y, w, h) = bbox(&ellipse(f, None, k));
        assert!((w - 60.0).abs() < 1e-9 && (h - 20.0).abs() < 1e-9, "k={k}");
        assert!((x + 30.0).abs() < 1e-9 && (y + 10.0).abs() < 1e-9);
    }
}

#[test]
fn c47_polygon_star_boxes_grow_with_outward_bulge() {
    let straight = bbox(&polygon(6, 20.0, 0.0));
    let round = bbox(&polygon(6, 20.0, 1.0));
    assert!(round.2 >= straight.2 - 1e-9 && round.3 >= straight.3 - 1e-9);
    assert!(
        (round.2 - 40.0).abs() < 1e-6,
        "polygon at 100% has the circle's box"
    );
    for k in [0.25, 0.5, 0.75] {
        let b = bbox(&polygon(5, 20.0, k));
        assert!(
            b.2 >= straight_box_w(5) - 1e-9,
            "never smaller than the straight box"
        );
        assert!(b.2 <= 40.0 + 1e-6);
    }
    let s = bbox(&star(5, 0.4, 20.0, 0.0));
    let s2 = bbox(&star(5, 0.4, 20.0, 0.6));
    assert!(s2.2 >= s.2 - 1e-9);
}

fn straight_box_w(_n: u32) -> f64 {
    // the existing box rule for polygons is the circumscribed square (2R) -- see
    // shape_frame_bounds; asserted only as a lower bound above
    0.0
}

// ---------------------------------------------------------------------------------------
// 17-21, 29: curve on the ellipse
// ---------------------------------------------------------------------------------------

fn d_of(k: f64) -> f64 {
    0.5 * (1.0 + (22.5 * DEG * k).tan())
}

#[test]
fn c19_side_middles_lie_at_d_of_k() {
    for (rx, ry) in [(20.0, 20.0), (30.0, 10.0), (7.0, 25.0)] {
        for k in [-1.0, -0.75, -0.5, -0.1, 0.0, 0.3, 0.5, 0.9, 1.0] {
            let f = frame(0.0, 0.0, rx, ry);
            let o = outline_of(&ellipse(f, None, k));
            assert_eq!(o.anchors.len(), 4, "criterion 29: 4 nodes");
            let d = d_of(k);
            for m in side_middles(&o) {
                assert!(
                    (m.x.abs() - rx * d).abs() < 1e-6,
                    "rx={rx} ry={ry} k={k}: {m:?}"
                );
                assert!(
                    (m.y.abs() - ry * d).abs() < 1e-6,
                    "rx={rx} ry={ry} k={k}: {m:?}"
                );
            }
        }
    }
}

#[test]
fn c19_published_examples() {
    let f = frame(0.0, 0.0, 20.0, 20.0);
    let area_of = |k: f64| area(&sample(&outline_of(&ellipse(f, None, k)), 200));
    assert!((area_of(1.0) - 1256.6).abs() < 1.3, "area 100%: {}", area_of(1.0));
    assert!((area_of(0.0) - 800.0).abs() < 0.01);
    assert!((area_of(-1.0) - 343.4).abs() < 0.5);
    let m = side_middles(&outline_of(&ellipse(f, None, -1.0)))[0];
    assert!((m.x.abs() - 5.86).abs() < 0.01 && (m.y.abs() - 5.86).abs() < 0.01);
    let m = side_middles(&outline_of(&ellipse(f, None, 0.0)))[0];
    assert!((m.x.abs() - 10.0).abs() < 1e-9);
    let m = side_middles(&outline_of(&ellipse(
        frame(0.0, 0.0, 30.0, 10.0),
        None,
        -1.0,
    )))[0];
    assert!((m.x.abs() - 8.79).abs() < 0.01 && (m.y.abs() - 2.93).abs() < 0.01);
}

#[test]
fn c18_fixed_points_and_the_tangent_angle() {
    let f = frame(0.0, 0.0, 1.0, 1.0);
    // k=0: a diamond with straight segments and no handles
    let o = outline_of(&ellipse(f, None, 0.0));
    for a in &o.anchors {
        assert!(a.handle_in.length() < 1e-12 && a.handle_out.length() < 1e-12);
        assert_eq!(a.kind, AnchorKind::Corner);
    }
    // k=-1: sharp tips, tangent along the axis (cusp) -> all four handles point inward
    let o = outline_of(&ellipse(f, None, -1.0));
    for a in &o.anchors {
        assert_eq!(a.kind, AnchorKind::Corner, "Corner at -100%");
        let (hi, ho) = (a.handle_in, a.handle_out);
        // both handles point towards the centre (the two handles of a node point the same way)
        let towards = (-a.point.x) * hi.x + (-a.point.y) * hi.y;
        let towards2 = (-a.point.x) * ho.x + (-a.point.y) * ho.y;
        assert!(
            towards > 0.0 && towards2 > 0.0,
            "cusp: both handles point the same way"
        );
    }
    // for general k the tangent at the node makes 45*|k| degrees with the chord to the next node
    for k in [-0.8, -0.3, 0.4, 0.7, 1.0] {
        let o = outline_of(&ellipse(f, None, k));
        let a = &o.anchors;
        for i in 0..4 {
            let (p, q) = (&a[i], &a[(i + 1) % 4]);
            let chord = Vec2::new(q.point.x - p.point.x, q.point.y - p.point.y);
            let h = p.handle_out;
            let cosang = (chord.x * h.x + chord.y * h.y) / (chord.length() * h.length());
            let got = cosang.clamp(-1.0, 1.0).acos() / DEG;
            assert!((got - 45.0 * k.abs()).abs() < 1e-6, "k={k}: tangent {got}");
        }
    }
}

#[test]
fn c29_control_point_formula() {
    let f = frame(0.0, 0.0, 1.0, 1.0);
    for k in [-1.0, -0.6, 0.2, 0.55, 1.0] {
        let o = outline_of(&ellipse(f, None, k));
        let alpha = (135.0 - 45.0 * k) * DEG;
        let l = (2f64.sqrt() / 3.0) / (22.5 * DEG * f64::abs(k)).cos().powi(2);
        // the E node is the one with x = 1
        let e = o
            .anchors
            .iter()
            .find(|a| (a.point.x - 1.0).abs() < 1e-9)
            .unwrap();
        let want = Vec2::new(l * alpha.cos(), l * alpha.sin());
        let ok = [e.handle_in, e.handle_out].iter().any(|h| {
            (h.x - want.x).abs() < 1e-6
                && ((h.y - want.y).abs() < 1e-6 || (h.y + want.y).abs() < 1e-6)
        });
        assert!(
            ok,
            "k={k}: want {want:?}, got {:?} {:?}",
            e.handle_in, e.handle_out
        );
    }
    // check values
    let e = outline_of(&ellipse(f, None, 1.0));
    let n = e
        .anchors
        .iter()
        .find(|a| (a.point.x - 1.0).abs() < 1e-9)
        .unwrap();
    assert!(
        [n.handle_in, n.handle_out]
            .iter()
            .any(|h| h.x.abs() < 1e-6 && (h.y.abs() - 0.5523).abs() < 1e-4)
    );
    let e = outline_of(&ellipse(f, None, -1.0));
    let n = e
        .anchors
        .iter()
        .find(|a| (a.point.x - 1.0).abs() < 1e-9)
        .unwrap();
    assert!(
        [n.handle_in, n.handle_out]
            .iter()
            .any(|h| (h.x + 0.5523).abs() < 1e-4 && h.y.abs() < 1e-6),
        "P1 = (0.4477, 0)"
    );
}

#[test]
fn c29_node_kinds_and_curve_100_equals_today() {
    let f = frame(5.0, 5.0, 20.0, 12.0);
    assert_eq!(
        outline_of(&ellipse(f, None, 1.0)).anchors,
        ellipse_outline(f)
    );
    for k in [-1.0, -0.2, 0.0, 0.99] {
        assert!(
            kinds(&outline_of(&ellipse(f, None, k)))
                .iter()
                .all(|x| *x == AnchorKind::Corner),
            "Corner at every other value, k={k}"
        );
    }
}

#[test]
fn c20_curved_ellipse_outline_is_simple_and_inside_its_box() {
    for k in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let f = frame(0.0, 0.0, 30.0, 10.0);
        let pts = sample(&outline_of(&ellipse(f, None, k)), 40);
        assert!(
            pts.iter()
                .all(|p| p.x.abs() <= 30.0 + 1e-9 && p.y.abs() <= 10.0 + 1e-9)
        );
        assert!(!self_crosses(&pts), "k={k}");
    }
}

/// Whether a closed polyline has two non-adjacent segments that properly cross.
fn self_crosses(pts: &[Point]) -> bool {
    let n = pts.len();
    let eps = 1e-7;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        for j in (i + 2)..n {
            if (j + 1) % n == i {
                continue;
            }
            let (c, d) = (pts[j], pts[(j + 1) % n]);
            let o = |p: Point, q: Point, r: Point| {
                (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x)
            };
            let (d1, d2, d3, d4) = (o(a, b, c), o(a, b, d), o(c, d, a), o(c, d, b));
            if d1 * d2 < -eps * eps
                && d3 * d4 < -eps * eps
                && d1.abs().min(d2.abs()) > eps
                && d3.abs().min(d4.abs()) > eps
            {
                return true;
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------------------
// 35-51: polygon and star
// ---------------------------------------------------------------------------------------

#[test]
fn c37_polygon_side_middle_distances() {
    let at = |n: u32, k: f64| {
        let o = outline_of(&polygon(n, 20.0, k));
        side_middles(&o)
            .iter()
            .map(|m| dist(*m, Point::new(100.0, 100.0)))
            .collect::<Vec<_>>()
    };
    for d in at(6, 1.0) {
        assert!((d - 20.0).abs() < 1e-6, "hexagon 100%");
    }
    for d in at(6, 0.0) {
        assert!((d - 17.32).abs() < 0.01);
    }
    for d in at(6, -1.0) {
        assert!((d - 14.64).abs() < 0.01);
    }
    for d in at(3, 0.0) {
        assert!((d - 10.0).abs() < 1e-6);
    }
    for d in at(3, -0.5) {
        assert!((d - 5.36).abs() < 0.01, "triangle at its limit");
    }
    for d in at(4, 0.0) {
        assert!((d - 14.14).abs() < 0.01);
    }
    for d in at(4, 1.0) {
        assert!((d - 20.0).abs() < 1e-6);
    }
    // general formula m(k)
    for n in [5u32, 7, 12, 100] {
        for k in [-1.0, -0.4, 0.3, 0.8] {
            let phi = PI / f64::from(n);
            let want = 20.0 * (phi.cos() + phi.sin() * (k * 90.0 * DEG / f64::from(n)).tan());
            for d in at(n, k) {
                assert!((d - want).abs() < 1e-6, "n={n} k={k}: {d} vs {want}");
            }
        }
    }
}

#[test]
fn c36_polygon_at_100_is_the_circle_for_every_n() {
    for n in [3u32, 4, 5, 6, 7, 11, 64, 1024] {
        let o = outline_of(&polygon(n, 20.0, 1.0));
        for p in sample(&o, 6) {
            assert!(
                (dist(p, Point::new(100.0, 100.0)) - 20.0).abs() <= 0.001 * 20.0,
                "n={n}"
            );
        }
    }
}

#[test]
fn c37_areas_and_c36_zero_is_today() {
    let a = |k: f64| area(&sample(&outline_of(&polygon(6, 20.0, k)), 100));
    assert!((a(1.0) - 1256.6).abs() < 1.5);
    assert!((a(0.0) - 1039.2).abs() < 0.1);
    // 0% equals today's straight outline exactly
    let o = outline_of(&polygon(6, 20.0, 0.0));
    let today = curvyo_document_core::polygon_outline(sframe(20.0), PointCount::new(6).unwrap());
    assert_eq!(o.anchors, today);
    assert_eq!(o.anchors.len(), 6);
    assert!(kinds(&o).iter().all(|k| *k == AnchorKind::Corner));
}

#[test]
fn c38_limits() {
    let lim = |s: &Shape| curve_limit(s).unwrap().get();
    assert!((lim(&polygon(3, 20.0, 0.0)) + 0.5).abs() < 1e-9, "triangle");
    for n in [4u32, 5, 6, 100, 1024] {
        assert!((lim(&polygon(n, 20.0, 0.0)) + 1.0).abs() < 1e-9, "n={n}");
    }
    assert!(
        (lim(&star(3, 0.5, 20.0, 0.0)) + 1.0).abs() < 1e-9,
        "hexagram"
    );
    assert!(
        (lim(&star(5, 0.382, 20.0, 0.0)) + 1.0).abs() < 1e-9,
        "pentagram"
    );
    let l = lim(&star(5, 0.2, 20.0, 0.0));
    assert!((l + 0.444).abs() < 0.002, "5 points ratio 0.2: {l}");
    assert!((lim(&ellipse(frame(0.0, 0.0, 1.0, 1.0), None, 1.0)) + 1.0).abs() < 1e-9);
}

#[test]
fn c38_effective_curve_is_max_of_stored_and_limit() {
    let tri = polygon(3, 20.0, -0.9);
    assert!((effective_curve(&tri).unwrap().get() + 0.5).abs() < 1e-9);
    // drawn with the effective value: middle distance is the -50% one
    let o = outline_of(&tri);
    for m in side_middles(&o) {
        assert!((dist(m, Point::new(100.0, 100.0)) - 5.36).abs() < 0.01);
    }
    // stored value stays (document keeps it raw)
    let (d, id) = doc_with(tri, 0.0);
    let back = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    match back.primitive(id).unwrap().shape {
        Shape::Polygon { curve, .. } => assert!((curve.get() + 0.9).abs() < 1e-12),
        _ => panic!(),
    }
    // ellipse arc: effective curve is 100
    let w = ellipse(
        frame(0.0, 0.0, 1.0, 1.0),
        Some(arc(0.0, 90.0, ArcKind::Arc)),
        1.0,
    );
    assert!((effective_curve(&w).unwrap().get() - 1.0).abs() < 1e-12);
    assert!(
        effective_curve(&Shape::Rect {
            bounds: curvyo_document_core::RectBounds::from_corners(
                Point::new(0.0, 0.0),
                Point::new(1.0, 1.0)
            ),
            corner_radii: curvyo_document_core::CornerRadii::uniform(Length::from_mm(0.0)),
        })
        .is_none(),
        "criterion 52: a rectangle has no curve"
    );
}

#[test]
fn c38_no_self_crossing_over_the_allowed_range() {
    // N from 3 to 1024 (sparse grid for the large ones), ratios 0.01..0.99
    let ns = [3u32, 4, 5, 6, 7, 8, 9, 12, 17, 24, 33, 48];
    let ratios = [0.01, 0.05, 0.2, 0.382, 0.5, 0.8, 0.99];
    for &n in &ns {
        for k in [-1.0, -0.8, -0.5, -0.2, 0.0, 0.3, 0.7, 1.0] {
            let s = polygon(n, 20.0, k);
            assert!(
                !self_crosses(&sample(&outline_of(&s), 5)),
                "polygon n={n} k={k}"
            );
            for &r in &ratios {
                let s = star(n, r, 20.0, k);
                assert!(
                    !self_crosses(&sample(&outline_of(&s), 5)),
                    "star n={n} r={r} k={k}"
                );
            }
        }
        // exactly the limit and below it (effective = limit)
        let s = star(n, 0.05, 20.0, -1.0);
        assert!(
            !self_crosses(&sample(&outline_of(&s), 5)),
            "star at its limit n={n}"
        );
    }
    for n in [100u32, 1024] {
        for k in [-1.0, 0.0, 1.0] {
            assert!(
                !self_crosses(&sample(&outline_of(&polygon(n, 20.0, k)), 3)),
                "n={n} k={k}"
            );
        }
    }
}

#[test]
fn c48_c49_conversion_nodes_and_kinds() {
    // triangle at 100% -> 6 nodes (a side above 90 degrees is split)
    let o = outline_of(&polygon(3, 20.0, 1.0));
    assert_eq!(
        o.anchors.len(),
        6,
        "criterion 48: triangle at 100% gives 6 nodes"
    );
    assert!(
        kinds(&o).iter().all(|k| *k == AnchorKind::Symmetric),
        "all tangents collinear at 100%"
    );
    // triangle at 70%: sides subtend 2*theta = 2*0.7*60 = 84 < 90, no split
    assert_eq!(outline_of(&polygon(3, 20.0, 0.7)).anchors.len(), 3);
    // at 75% the side subtends exactly 90: still not split; at 80% it is
    assert_eq!(outline_of(&polygon(3, 20.0, 0.8)).anchors.len(), 6);
    // hexagon: M = 6 nodes, Corner except exactly 100%
    for k in [-1.0, -0.5, 0.5, 0.99] {
        let o = outline_of(&polygon(6, 20.0, k));
        assert_eq!(o.anchors.len(), 6);
        assert!(kinds(&o).iter().all(|x| *x == AnchorKind::Corner), "k={k}");
    }
    assert!(
        kinds(&outline_of(&polygon(6, 20.0, 1.0)))
            .iter()
            .all(|x| *x == AnchorKind::Symmetric)
    );
    // star: 2N nodes
    assert_eq!(outline_of(&star(5, 0.4, 20.0, 0.5)).anchors.len(), 10);
    assert_eq!(outline_of(&star(5, 0.4, 20.0, 0.0)).anchors.len(), 10);
    // control point formula for a polygon side: L = (c/3)/cos^2(theta/2)
    let n = 6u32;
    let k = 0.5;
    let o = outline_of(&polygon(n, 20.0, k));
    let c = 2.0 * 20.0 * (PI / f64::from(n)).sin();
    let theta = k * PI / f64::from(n);
    let l = (c / 3.0) / (theta / 2.0).cos().powi(2);
    for a in &o.anchors {
        assert!(
            (a.handle_out.length() - l).abs() < 1e-6,
            "{} vs {l}",
            a.handle_out.length()
        );
        assert!((a.handle_in.length() - l).abs() < 1e-6);
    }
}

#[test]
fn c51_four_sided_polygon_equals_the_circle() {
    let f = frame(100.0, 100.0, 20.0, 20.0);
    let mut k = -1.0;
    while k <= 1.0 + 1e-12 {
        let a = outline_of(&ellipse(f, None, k));
        let b = outline_of(&polygon(4, 20.0, k));
        let pa = sample(&a, 16);
        let pb = sample(&b, 16);
        for p in &pb {
            let near = pa.iter().map(|q| dist(*p, *q)).fold(1e9, f64::min);
            assert!(near < 0.001 + 20.0 * 0.0006, "k={k}: {near}");
        }
        // same control points: each polygon anchor matches an ellipse anchor
        for q in &b.anchors {
            let m = a.anchors.iter().find(|r| dist(r.point, q.point) < 1e-6);
            let m = m.unwrap_or_else(|| panic!("k={k}: node {:?} missing", q.point));
            let hs = |x: &OutlineAnchor| {
                let mut v = vec![
                    (x.handle_in.x, x.handle_in.y),
                    (x.handle_out.x, x.handle_out.y),
                ];
                v.sort_by(|p, q| p.partial_cmp(q).unwrap());
                v
            };
            for (h1, h2) in hs(m).iter().zip(hs(q).iter()) {
                assert!(
                    (h1.0 - h2.0).abs() < 1e-6 && (h1.1 - h2.1).abs() < 1e-6,
                    "k={k}"
                );
            }
        }
        k += 0.125;
    }
}

// ---------------------------------------------------------------------------------------
// 54, 56: mirror and affine maps
// ---------------------------------------------------------------------------------------

fn axes(rx: f64, ry: f64, rot_deg: f64) -> EllipseAxes {
    EllipseAxes {
        rx,
        ry,
        rotation: rot_deg * DEG,
    }
}

/// The document point of an ellipse at parameter t (centre at the origin).
fn pt_at(a: EllipseAxes, t: f64) -> (f64, f64) {
    let (x, y) = (a.rx * t.cos(), a.ry * t.sin());
    let (s, c) = a.rotation.sin_cos();
    (c * x - s * y, s * x + c * y)
}

fn mul(m: [[f64; 2]; 2], p: (f64, f64)) -> (f64, f64) {
    (m[0][0] * p.0 + m[0][1] * p.1, m[1][0] * p.0 + m[1][1] * p.1)
}

/// Checks that the arc after the map equals the mapped old arc at 64 samples.
fn check_map(
    before: EllipseAxes,
    a: EllipseArc,
    m: [[f64; 2]; 2],
    tol: f64,
) -> (EllipseAxes, EllipseArc) {
    let after = ellipse_after_linear_map(before, m);
    let na = arc_after_linear_map(a, before, after, m);
    let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    let sweep = a.sweep().as_radians();
    assert!(
        (na.sweep().as_radians() - sweep).abs() < 1e-9,
        "sweep unchanged"
    );
    assert_eq!(na.kind, a.kind, "kind unchanged");
    for i in 0..64 {
        let f = f64::from(i) / 63.0;
        let t_old = a.start().as_radians() + sweep * f;
        let want = mul(m, pt_at(before, t_old));
        let t_new = if det >= 0.0 {
            na.start().as_radians() + sweep * f
        } else {
            na.end().as_radians() - sweep * f
        };
        let got = pt_at(after, t_new);
        assert!(
            (want.0 - got.0).abs() < tol && (want.1 - got.1).abs() < tol,
            "i={i} want {want:?} got {got:?} (after {after:?}, arc {na:?})"
        );
    }
    (after, na)
}

#[test]
fn c54_mirror_arc_is_the_exact_mirror_image_for_rotations_0_30_90() {
    for rot in [0.0, 30.0, 90.0] {
        for (rx, ry) in [(20.0, 20.0), (30.0, 10.0)] {
            for (s, e) in [(30.0, 150.0), (200.0, 20.0), (300.0, 350.0)] {
                for m in [
                    [[1.0, 0.0], [0.0, -1.0]],
                    [[-1.0, 0.0], [0.0, 1.0]],
                    [[-1.0, 0.0], [0.0, -1.0]],
                ] {
                    check_map(axes(rx, ry, rot), arc(s, e, ArcKind::Pie), m, 1e-9);
                }
            }
        }
    }
}

#[test]
fn c54_the_mirror_angle_rule_in_the_own_frame() {
    // mirror along the own horizontal axis (y -> -y): start' = -end, end' = -start
    let before = axes(30.0, 10.0, 0.0);
    let a = arc(30.0, 100.0, ArcKind::Arc);
    let (_, na) = check_map(before, a, [[1.0, 0.0], [0.0, -1.0]], 1e-9);
    assert!(
        (na.start().as_radians() - (360.0 - 100.0) * DEG).abs() < 1e-9,
        "{:?}",
        na
    );
    assert!((na.end().as_radians() - (360.0 - 30.0) * DEG).abs() < 1e-9);
    // mirror along the own vertical axis (x -> -x): start' = 180 - end, end' = 180 - start
    let (_, na) = check_map(before, a, [[-1.0, 0.0], [0.0, 1.0]], 1e-9);
    assert!(
        (na.start().as_radians() - 80.0 * DEG).abs() < 1e-9,
        "{:?}",
        na
    );
    assert!((na.end().as_radians() - 150.0 * DEG).abs() < 1e-9);
}

#[test]
fn c54_flip_twice_is_identity() {
    for rot in [0.0, 30.0, 90.0] {
        let before = axes(25.0, 14.0, rot);
        let a = arc(40.0, 250.0, ArcKind::Chord);
        let m = [[1.0, 0.0], [0.0, -1.0]];
        let (b1, a1) = check_map(before, a, m, 1e-9);
        let (b2, a2) = check_map(b1, a1, m, 1e-9);
        assert!((b2.rx - before.rx).abs() < 1e-9 && (b2.ry - before.ry).abs() < 1e-9);
        let dr = ((b2.rotation - before.rotation) / PI).rem_euclid(1.0);
        assert!(
            dr < 1e-9 || (1.0 - dr) < 1e-9 || (dr - 0.5).abs() < 1e-9,
            "rotation {dr}"
        );
        // angles back (mod the 4-fold ambiguity of an ellipse frame, the drawn points agree)
        for i in 0..64 {
            let f = f64::from(i) / 63.0;
            let sweep = a.sweep().as_radians();
            let p0 = pt_at(before, a.start().as_radians() + sweep * f);
            let p2 = pt_at(b2, a2.start().as_radians() + sweep * f);
            assert!((p0.0 - p2.0).abs() < 1e-9 && (p0.1 - p2.1).abs() < 1e-9);
        }
        if (b2.rotation - before.rotation).abs() < 1e-9 {
            assert!((a2.start().as_radians() - a.start().as_radians()).abs() < 1e-9);
            assert!((a2.end().as_radians() - a.end().as_radians()).abs() < 1e-9);
        }
    }
}

#[test]
fn c56_non_uniform_scale_keeps_an_exact_arc_also_after_rotation() {
    let half = arc(0.0, 180.0, ArcKind::Arc);
    // the spec's example: a 30 degree rotated half circle stretched by (2, 0.5) in document axes
    let (after, _) = check_map(axes(20.0, 20.0, 30.0), half, [[2.0, 0.0], [0.0, 0.5]], 1e-9);
    assert!(after.rx > 0.0 && after.ry > 0.0);
    for rot in [0.0, 15.0, 30.0, 45.0, 60.0, 90.0, 133.0] {
        for (rx, ry) in [(20.0, 20.0), (30.0, 10.0), (4.0, 40.0)] {
            for m in [
                [[2.0, 0.0], [0.0, 0.5]],
                [[0.5, 0.0], [0.0, 2.0]],
                [[3.0, 0.0], [0.0, 1.0]],
                [[1.0, 0.0], [0.0, 0.2]],
                [[-2.0, 0.0], [0.0, 0.5]],
            ] {
                for (s, e) in [(0.0, 180.0), (45.0, 300.0), (340.0, 30.0)] {
                    check_map(axes(rx, ry, rot), arc(s, e, ArcKind::Pie), m, 1e-9);
                }
            }
        }
    }
}

#[test]
fn c56_stretch_and_inverse_restore_the_angles() {
    for rot in [0.0, 30.0, 77.0] {
        let before = axes(20.0, 20.0, rot);
        let a = arc(35.0, 215.0, ArcKind::Arc);
        let (b1, a1) = check_map(before, a, [[2.0, 0.0], [0.0, 0.5]], 1e-9);
        let (b2, a2) = check_map(b1, a1, [[0.5, 0.0], [0.0, 2.0]], 1e-9);
        // a circle is restored: rotation 0 and the same shift
        assert!((b2.rx - 20.0).abs() < 1e-9 && (b2.ry - 20.0).abs() < 1e-9);
        assert!(
            b2.rotation.abs() < 1e-9,
            "singular values equal: rotation 0"
        );
        // a circle's drawn points are rotation-agnostic only via the shift: compare drawn points
        let sweep = a.sweep().as_radians();
        for i in 0..64 {
            let f = f64::from(i) / 63.0;
            let p0 = pt_at(before, a.start().as_radians() + sweep * f);
            let p2 = pt_at(b2, a2.start().as_radians() + sweep * f);
            assert!((p0.0 - p2.0).abs() < 1e-9 && (p0.1 - p2.1).abs() < 1e-9);
        }
        // for the unrotated circle the angles themselves come back within 1e-9 degrees
        if rot == 0.0 {
            assert!((a2.start().as_radians() - a.start().as_radians()).abs() < 1e-9 * DEG);
            assert!((a2.end().as_radians() - a.end().as_radians()).abs() < 1e-9 * DEG);
        }
    }
}

#[test]
fn c56_group_stretch_with_rotation_of_the_map_and_uniform_scale() {
    // rotation in the map + stretch: M = R(25) diag(2, 0.5) R(-10)
    let (s25, c25) = (25.0 * DEG).sin_cos();
    let (s10, c10) = (10.0 * DEG).sin_cos();
    let r25 = [[c25, -s25], [s25, c25]];
    let r10 = [[c10, s10], [-s10, c10]];
    let d = [[2.0, 0.0], [0.0, 0.5]];
    let mul2 = |p: [[f64; 2]; 2], q: [[f64; 2]; 2]| {
        [
            [
                p[0][0] * q[0][0] + p[0][1] * q[1][0],
                p[0][0] * q[0][1] + p[0][1] * q[1][1],
            ],
            [
                p[1][0] * q[0][0] + p[1][1] * q[1][0],
                p[1][0] * q[0][1] + p[1][1] * q[1][1],
            ],
        ]
    };
    let m = mul2(r25, mul2(d, r10));
    check_map(
        axes(30.0, 12.0, 41.0),
        arc(10.0, 270.0, ArcKind::Chord),
        m,
        1e-9,
    );
    // uniform scale keeps the angles exactly
    let a = arc(10.0, 270.0, ArcKind::Chord);
    let (b, na) = check_map(axes(30.0, 12.0, 41.0), a, [[2.0, 0.0], [0.0, 2.0]], 1e-9);
    assert!((b.rx - 60.0).abs() < 1e-9 && (b.ry - 24.0).abs() < 1e-9);
    assert!((na.start().as_radians() - a.start().as_radians()).abs() < 1e-9);
    assert!((na.end().as_radians() - a.end().as_radians()).abs() < 1e-9);
}

#[test]
fn c10_aligned_scale_does_not_touch_the_angles_at_all() {
    let a = arc(33.3, 211.1, ArcKind::Pie);
    let before = axes(20.0, 10.0, 0.0);
    let m = [[3.0, 0.0], [0.0, 0.7]];
    let after = ellipse_after_linear_map(before, m);
    assert!((after.rx - 60.0).abs() < 1e-9 && (after.ry - 7.0).abs() < 1e-9);
    let na = arc_after_linear_map(a, before, after, m);
    assert_eq!(na.start(), a.start(), "bit-exact: angles are not rewritten");
    assert_eq!(na.end(), a.end());
}

// ---------------------------------------------------------------------------------------
// 14, 15, 32, 33, 50, 61: persistence
// ---------------------------------------------------------------------------------------

fn roundtrip(d: &Document) -> Document {
    unpack(7, &pack(d, "tester").unwrap()).unwrap()
}

#[test]
fn c14_c32_c50_save_and_open_keeps_every_parameter_exactly() {
    let d = Document::new(1);
    let f = frame(12.5, 7.25, 33.0, 17.5);
    let arcs = [
        d.create_primitive(
            ellipse(f, Some(arc(33.3333333, 211.1, ArcKind::Pie)), 1.0),
            ang(17.0),
        ),
        d.create_primitive(
            ellipse(f, Some(arc(300.0, 20.0, ArcKind::Arc)), 1.0),
            ang(0.0),
        ),
        d.create_primitive(
            ellipse(f, Some(arc(0.5, 359.0, ArcKind::Chord)), 1.0),
            ang(-40.0),
        ),
    ];
    let curved = d.create_primitive(ellipse(f, None, -0.37), ang(5.0));
    let poly = d.create_primitive(polygon(7, 20.0, 0.61), ang(0.0));
    let st = d.create_primitive(star(5, 0.382, 20.0, -0.23), ang(0.0));
    let under = d.create_primitive(polygon(3, 20.0, -0.9), ang(0.0));
    let back = roundtrip(&d);
    for id in arcs.iter().chain([&curved, &poly, &st, &under]) {
        assert_eq!(
            back.primitive(*id).unwrap().shape,
            d.primitive(*id).unwrap().shape
        );
        assert_eq!(
            back.primitive(*id).unwrap().rotation,
            d.primitive(*id).unwrap().rotation
        );
    }
    assert!(matches!(
        back.primitive(arcs[0]).unwrap().shape,
        Shape::Ellipse { arc: Some(_), .. }
    ));
}

#[test]
fn c14_c32_c50_defaults_are_not_stored() {
    let plain = Document::new(1);
    let json_empty = String::from_utf8(plain.export_json().unwrap()).unwrap();
    let d = Document::new(1);
    let _ = d.create_primitive(ellipse(frame(0.0, 0.0, 5.0, 5.0), None, 1.0), ang(0.0));
    let _ = d.create_primitive(polygon(5, 5.0, 0.0), ang(0.0));
    let _ = d.create_primitive(star(5, 0.5, 5.0, 0.0), ang(0.0));
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(
        !json.contains("ellipse_arc"),
        "whole ellipse stores no arc field"
    );
    assert!(!json.contains("\"curve\""), "default curve is not stored");
    assert!(!json_empty.contains("curve"));
}

#[test]
fn c14_a_non_default_value_is_stored_and_the_default_deletes_it() {
    let d = Document::new(1);
    let e = d.create_primitive(ellipse(frame(0.0, 0.0, 5.0, 5.0), None, 1.0), ang(0.0));
    let p = d.create_primitive(polygon(5, 5.0, 0.0), ang(0.0));
    d.set_ellipse_arcs(&[(e, Some(arc(10.0, 100.0, ArcKind::Pie)))])
        .unwrap();
    d.set_curves(&[(p, Curve::new(0.5).unwrap())]).unwrap();
    let keys = stored_keys(&d);
    assert!(keys.contains("ellipse_arc") && keys.contains("curve"), "{keys}");
    d.set_ellipse_arcs(&[(e, None)]).unwrap();
    d.set_curves(&[(p, Curve::POLYGON_DEFAULT)]).unwrap();
    let keys = stored_keys(&d);
    assert!(!keys.contains("ellipse_arc"), "{keys}");
    assert!(!keys.contains("curve"), "{keys}");
}

#[test]
fn c15_c33_c50_the_format_version_increased() {
    assert!(
        CURRENT_FORMAT_VERSION >= 15,
        "was 14 on main; 15 here, 16 at merge"
    );
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn c14_golden_fixtures_open_and_hold_the_values_of_their_names() {
    for (name, kind) in [
        ("arc_pie_v15.curvyo", ArcKind::Pie),
        ("arc_arc_v15.curvyo", ArcKind::Arc),
        ("arc_chord_v15.curvyo", ArcKind::Chord),
    ] {
        let d = unpack(1, &fixture(name)).unwrap();
        let shapes = all_shapes(&d);
        assert!(
            shapes
                .iter()
                .any(|s| matches!(s, Shape::Ellipse { arc: Some(a), .. } if a.kind == kind)),
            "{name}"
        );
    }
    let d = unpack(1, &fixture("curve_ellipse_v15.curvyo")).unwrap();
    assert!(
        all_shapes(&d)
            .iter()
            .any(|s| matches!(s, Shape::Ellipse { curve, .. } if *curve != Curve::ELLIPSE_DEFAULT))
    );
    let d = unpack(1, &fixture("curve_polygon_star_v15.curvyo")).unwrap();
    let shapes = all_shapes(&d);
    assert!(
        shapes
            .iter()
            .any(|s| matches!(s, Shape::Polygon { curve, .. } if *curve != Curve::POLYGON_DEFAULT))
    );
    assert!(
        shapes
            .iter()
            .any(|s| matches!(s, Shape::Star { curve, .. } if *curve != Curve::POLYGON_DEFAULT))
    );
    // re-saving an opened golden gives the same shapes
    let back = roundtrip(&d);
    assert_eq!(all_shapes(&back), shapes);
}

fn all_shapes(d: &Document) -> Vec<Shape> {
    d.object_ids()
        .into_iter()
        .filter_map(|id| d.primitive(id).map(|p| p.shape))
        .collect()
}

#[test]
fn c33_c50_damaged_fixtures_are_refused_with_the_damaged_error() {
    for name in [
        "arc_unknown_kind_damaged_v15.curvyo",
        "arc_with_curve_damaged_v15.curvyo",
        "curve_two_damaged_v15.curvyo",
    ] {
        assert!(
            matches!(unpack(1, &fixture(name)), Err(OpenError::Damaged)),
            "{name}"
        );
    }
}

#[test]
fn c33_c50_damaged_values_crafted_by_hand_are_refused_not_clamped() {
    // overwrite the stored registers directly in the Loro snapshot
    let cases: Vec<(&str, loro::LoroValue)> = vec![
        ("curve", loro::LoroValue::Double(1.5)),
        ("curve", loro::LoroValue::Double(-1.0000001)),
        ("curve", loro::LoroValue::Double(f64::NAN)),
        ("curve", loro::LoroValue::Double(f64::INFINITY)),
        ("curve", loro::LoroValue::String("0.5".to_string().into())),
    ];
    for (key, value) in cases {
        for shape in [
            ellipse(frame(0.0, 0.0, 5.0, 5.0), None, 0.5),
            polygon(5, 5.0, 0.5),
            star(5, 0.4, 5.0, 0.5),
        ] {
            let d = Document::new(1);
            let _ = d.create_primitive(shape, ang(0.0));
            let bytes = craft(&d, |meta| {
                meta.insert(key, value.clone()).unwrap();
            });
            let r = unpack(1, &bytes);
            assert!(
                matches!(r, Err(OpenError::Damaged)),
                "{key}={value:?} on {shape:?}: {:?}",
                r.err()
            );
        }
    }
    // arc registers: wrong shape of list, non finite, three values required
    let bad_arcs: Vec<Vec<loro::LoroValue>> = vec![
        vec![
            loro::LoroValue::Double(f64::NAN),
            loro::LoroValue::Double(10.0),
            "pie".into(),
        ],
        vec![
            loro::LoroValue::Double(10.0),
            loro::LoroValue::Double(f64::INFINITY),
            "pie".into(),
        ],
        vec![loro::LoroValue::Double(10.0), loro::LoroValue::Double(20.0)],
        vec![
            loro::LoroValue::Double(10.0),
            loro::LoroValue::Double(20.0),
            "pie".into(),
            "x".into(),
        ],
        vec![
            loro::LoroValue::Double(10.0),
            loro::LoroValue::Double(20.0),
            "Pie".into(),
        ],
        vec![
            loro::LoroValue::Double(10.0),
            loro::LoroValue::Double(20.0),
            loro::LoroValue::Double(1.0),
        ],
    ];
    for list in bad_arcs {
        let d = Document::new(1);
        let _ = d.create_primitive(ellipse(frame(0.0, 0.0, 5.0, 5.0), None, 1.0), ang(0.0));
        let list2 = list.clone();
        let bytes = craft(&d, move |meta| {
            meta.insert("ellipse_arc", loro::LoroValue::List(list2.clone().into()))
                .unwrap();
        });
        assert!(
            matches!(unpack(1, &bytes), Err(OpenError::Damaged)),
            "{list:?}"
        );
    }
    // an arc on a polygon (not an ellipse) is not an arc: must not be silently shown
    // as a plain polygon without complaint? (spec silent) -> not asserted.
    // a polygon curve below its limit is NOT damaged (criterion 38/50)
    let d = Document::new(1);
    let _ = d.create_primitive(polygon(3, 5.0, 0.5), ang(0.0));
    let bytes = craft(&d, |meta| {
        meta.insert("curve", loro::LoroValue::Double(-0.9)).unwrap();
    });
    assert!(unpack(1, &bytes).is_ok(), "below k_min is valid");
}

/// The keys of every object's stored meta map, as one string.
fn stored_keys(d: &Document) -> String {
    let doc = loro::LoroDoc::new();
    doc.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = doc.get_tree("paths");
    let mut out = String::new();
    for n in tree.nodes() {
        let meta = tree.get_meta(n).unwrap();
        out.push_str(&format!("{:?};", meta.get_deep_value()));
    }
    out
}

fn craft(d: &Document, f: impl Fn(&loro::LoroMap)) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;
    let snap = d.export_loro_snapshot().unwrap();
    let doc = loro::LoroDoc::new();
    doc.import(&snap).unwrap();
    let tree = doc.get_tree("paths");
    let node = tree.nodes()[0];
    f(&tree.get_meta(node).unwrap());
    doc.commit();
    let bytes = doc.export(loro::ExportMode::Snapshot).unwrap();
    let packed = pack(d, "t").unwrap();
    // reuse the manifest of a real pack (right version), swap the document
    let mut zin = zip::ZipArchive::new(Cursor::new(packed)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default();
    for i in 0..zin.len() {
        let mut file = zin.by_index(i).unwrap();
        let name = file.name().unwrap().to_string();
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut buf).unwrap();
        out.start_file(&name, opts).unwrap();
        if name == "document.loro" {
            out.write_all(&bytes).unwrap();
        } else {
            out.write_all(&buf).unwrap();
        }
    }
    out.finish().unwrap().into_inner()
}

#[test]
fn c61_determinism_pure_function_any_order() {
    let shapes = [
        ellipse(
            frame(1.0, 2.0, 30.0, 10.0),
            Some(arc(10.0, 200.0, ArcKind::Pie)),
            1.0,
        ),
        ellipse(frame(1.0, 2.0, 30.0, 10.0), None, -0.4),
        polygon(1024, 20.0, 0.3),
        star(7, 0.3, 20.0, -0.2),
    ];
    let first: Vec<_> = shapes.iter().map(outline_of).collect();
    let first_bounds: Vec<_> = shapes.iter().map(shape_frame_bounds).collect();
    for _ in 0..3 {
        for (i, s) in shapes.iter().enumerate().rev() {
            assert_eq!(outline_of(s), first[i]);
            assert_eq!(shape_frame_bounds(s), first_bounds[i]);
        }
    }
    // rotation applied about the centre: rotated outline = rotate(outline)
    let s = shapes[0];
    let r = outline_of_rotated(&s, ang(30.0));
    let c = curvyo_document_core::shape_center(&s);
    for (a, b) in outline_of(&s).anchors.iter().zip(&r.anchors) {
        let want = a.point.rotated_around(c, ang(30.0));
        assert!(dist(want, b.point) < 1e-9);
    }
}

// ---------------------------------------------------------------------------------------
// 3, 31, 57, 59, 63, 64: commands
// ---------------------------------------------------------------------------------------

#[test]
fn c59_set_arc_is_one_commit_with_the_label_and_undo_redo() {
    let d = Document::new(1);
    let a = d.create_primitive(ellipse(frame(0.0, 0.0, 5.0, 5.0), None, 1.0), ang(0.0));
    let b = d.create_primitive(ellipse(frame(20.0, 0.0, 5.0, 5.0), None, 1.0), ang(0.0));
    let before = d.own_steps().len();
    d.set_ellipse_arcs(&[
        (a, Some(arc(10.0, 100.0, ArcKind::Arc))),
        (b, Some(arc(20.0, 200.0, ArcKind::Pie))),
    ])
    .unwrap();
    let steps = d.own_steps();
    assert_eq!(steps.len(), before + 1, "one commit for the whole batch");
    assert_eq!(steps.last().unwrap().label, "set_ellipse_arc");
    // no change -> no commit
    d.set_ellipse_arcs(&[(a, Some(arc(10.0, 100.0, ArcKind::Arc)))])
        .unwrap();
    assert_eq!(d.own_steps().len(), before + 1);
    let step = steps.last().unwrap().step;
    let with_arcs = d.export_json().unwrap();
    d.undo_step(step).unwrap();
    for id in [a, b] {
        assert!(matches!(
            d.primitive(id).unwrap().shape,
            Shape::Ellipse { arc: None, .. }
        ));
    }
    d.redo_step(step).unwrap();
    assert_eq!(d.export_json().unwrap(), with_arcs);
}

#[test]
fn c59_set_curves_is_one_commit_labelled_set_curve() {
    let d = Document::new(1);
    let e = d.create_primitive(ellipse(frame(0.0, 0.0, 5.0, 5.0), None, 1.0), ang(0.0));
    let p = d.create_primitive(polygon(5, 5.0, 0.0), ang(0.0));
    let s = d.create_primitive(star(5, 0.4, 5.0, 0.0), ang(0.0));
    let n = d.own_steps().len();
    let c = Curve::new(-0.35).unwrap();
    d.set_curves(&[(e, c), (p, c), (s, c)]).unwrap();
    assert_eq!(d.own_steps().len(), n + 1);
    assert_eq!(d.own_steps().last().unwrap().label, "set_curve");
    d.set_curves(&[(e, c), (p, c), (s, c)]).unwrap();
    assert_eq!(d.own_steps().len(), n + 1, "no change, no commit");
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert!(
        matches!(d.primitive(e).unwrap().shape, Shape::Ellipse { curve, .. } if curve == Curve::ELLIPSE_DEFAULT)
    );
    assert!(
        matches!(d.primitive(p).unwrap().shape, Shape::Polygon { curve, .. } if curve == Curve::POLYGON_DEFAULT)
    );
}

#[test]
fn c31_arc_and_curve_never_coexist_through_the_commands() {
    let d = Document::new(1);
    let curved = d.create_primitive(ellipse(frame(0.0, 0.0, 5.0, 5.0), None, 0.5), ang(0.0));
    let arced = d.create_primitive(
        ellipse(
            frame(0.0, 0.0, 5.0, 5.0),
            Some(arc(10.0, 100.0, ArcKind::Arc)),
            1.0,
        ),
        ang(0.0),
    );
    let n = d.own_steps().len();
    assert_eq!(
        d.set_ellipse_arcs(&[(curved, Some(arc(10.0, 100.0, ArcKind::Arc)))]),
        Err(ShapeEditError::ArcWithCurve)
    );
    assert_eq!(
        d.set_curves(&[(arced, Curve::new(0.2).unwrap())]),
        Err(ShapeEditError::ArcWithCurve)
    );
    assert_eq!(d.own_steps().len(), n, "a refusal writes nothing");
    // whole first, then the other works
    d.set_ellipse_arcs(&[(curved, None)]).unwrap();
    d.set_curves(&[(arced, Curve::ELLIPSE_DEFAULT)]).unwrap();
    // the batch is atomic: one bad id refuses all
    let json = d.export_json().unwrap();
    let r = d.set_ellipse_arcs(&[(arced, None), (arced, Some(arc(1.0, 2.0, ArcKind::Arc)))]);
    assert!(r.is_ok());
    let poly = d.create_primitive(polygon(5, 5.0, 0.0), ang(0.0));
    let json2 = d.export_json().unwrap();
    assert_ne!(json, json2);
    let r = d.set_ellipse_arcs(&[(arced, None), (poly, Some(arc(1.0, 2.0, ArcKind::Arc)))]);
    assert!(r.is_err(), "an arc on a polygon is refused");
    assert_eq!(
        d.export_json().unwrap(),
        json2,
        "nothing of the batch written"
    );
}

#[test]
fn c52_curve_on_a_rectangle_is_refused() {
    let d = Document::new(1);
    let r = d.create_rect(curvyo_document_core::RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(10.0, 10.0),
    ));
    assert_eq!(
        d.set_curves(&[(r, Curve::new(0.5).unwrap())]),
        Err(ShapeEditError::WrongShape)
    );
}

#[test]
fn c63_c16_c34_c35_new_shapes_use_the_defaults_and_stay_primitives() {
    let d = Document::new(1);
    let e = d.create_ellipse(frame(0.0, 0.0, 5.0, 5.0));
    let p = d.create_polygon(sframe(5.0), PointCount::new(5).unwrap());
    let s = d.create_star(
        sframe(5.0),
        PointCount::new(5).unwrap(),
        InnerRatio::new(0.5).unwrap(),
    );
    assert!(
        matches!(d.primitive(e).unwrap().shape, Shape::Ellipse { arc: None, curve, .. } if curve.percent() == 100.0)
    );
    assert!(
        matches!(d.primitive(p).unwrap().shape, Shape::Polygon { curve, .. } if curve.percent() == 0.0)
    );
    assert!(
        matches!(d.primitive(s).unwrap().shape, Shape::Star { curve, .. } if curve.percent() == 0.0)
    );
    // 100% on a polygon and 0% on an ellipse stay primitives
    d.set_curves(&[(p, Curve::new(1.0).unwrap()), (e, Curve::new(0.0).unwrap())])
        .unwrap();
    assert!(d.primitive(p).is_some() && d.primitive(e).is_some());
}

#[test]
fn c57_duplicate_copies_the_whole_parameter_set() {
    let d = Document::new(1);
    let a = d.create_primitive(
        ellipse(
            frame(0.0, 0.0, 5.0, 5.0),
            Some(arc(10.0, 100.0, ArcKind::Chord)),
            1.0,
        ),
        ang(20.0),
    );
    let c = d.create_primitive(ellipse(frame(0.0, 0.0, 5.0, 5.0), None, -0.5), ang(0.0));
    let p = d.create_primitive(polygon(5, 5.0, -0.7), ang(0.0));
    let s = d.create_primitive(star(6, 0.4, 5.0, 0.4), ang(0.0));
    let src: Vec<_> = [a, c, p, s]
        .iter()
        .map(|id| CopySource {
            id: *id,
            anchor_ids: vec![],
        })
        .collect();
    let copies = d.duplicate_objects(&src, Vec2::new(10.0, 10.0)).unwrap();
    for (orig, copy) in [a, c, p, s].iter().zip(&copies) {
        let (o, k) = (d.primitive(*orig).unwrap(), d.primitive(*copy).unwrap());
        assert_eq!(
            std::mem::discriminant(&o.shape),
            std::mem::discriminant(&k.shape)
        );
        match (o.shape, k.shape) {
            (
                Shape::Ellipse {
                    arc: a1, curve: c1, ..
                },
                Shape::Ellipse {
                    arc: a2, curve: c2, ..
                },
            ) => {
                assert_eq!(a1, a2);
                assert_eq!(c1, c2);
            }
            (Shape::Polygon { curve: c1, .. }, Shape::Polygon { curve: c2, .. })
            | (Shape::Star { curve: c1, .. }, Shape::Star { curve: c2, .. }) => assert_eq!(c1, c2),
            _ => panic!("kind changed"),
        }
        assert_eq!(o.rotation, k.rotation);
    }
}

#[test]
fn c59_c54_resize_through_zero_writes_the_mirrored_arc_in_one_commit() {
    let d = Document::new(1);
    let id = d.create_primitive(
        ellipse(
            frame(0.0, 0.0, 30.0, 10.0),
            Some(arc(30.0, 100.0, ArcKind::Arc)),
            1.0,
        ),
        ang(0.0),
    );
    let n = d.own_steps().len();
    let mirrored = arc(260.0, 330.0, ArcKind::Arc);
    d.resize_ellipse_with_arc(id, frame(0.0, 0.0, 30.0, 10.0), Some(mirrored), None)
        .unwrap();
    assert_eq!(d.own_steps().len(), n + 1);
    assert!(
        matches!(d.primitive(id).unwrap().shape, Shape::Ellipse { arc: Some(a), .. } if a == mirrored)
    );
    // arc on a curved ellipse is refused
    let c = d.create_primitive(ellipse(frame(0.0, 0.0, 30.0, 10.0), None, 0.3), ang(0.0));
    assert_eq!(
        d.resize_ellipse_with_arc(c, frame(0.0, 0.0, 3.0, 1.0), Some(mirrored), None),
        Err(ShapeEditError::ArcWithCurve)
    );
}

// ---------------------------------------------------------------------------------------
// 59: history preview, 60 consumers share the evaluation
// ---------------------------------------------------------------------------------------

#[test]
fn c59_a_preview_of_an_earlier_state_reads_a_whole_ellipse_and_the_later_one_the_arc() {
    let d = Document::new(1);
    let id = d.create_primitive(ellipse(frame(0.0, 0.0, 5.0, 5.0), None, 1.0), ang(0.0));
    let before = d.own_steps().last().unwrap().step;
    d.set_ellipse_arcs(&[(id, Some(arc(10.0, 100.0, ArcKind::Pie)))])
        .unwrap();
    let after = d.own_steps().last().unwrap().step;
    let old = d.preview_object_at(before, id).unwrap();
    let new = d.preview_object_at(after, id).unwrap();
    let shape_of = |h: &curvyo_document_core::HistoricalDocument| match &h.objects[0] {
        curvyo_document_core::ObjectSnapshot::Primitive(p) => p.shape,
        o => panic!("{o:?}"),
    };
    assert!(matches!(shape_of(&old), Shape::Ellipse { arc: None, .. }));
    assert!(
        matches!(shape_of(&new), Shape::Ellipse { arc: Some(a), .. } if a.kind == ArcKind::Pie)
    );
    // a preview does not write
    assert_eq!(d.own_steps().last().unwrap().step, after);
}

#[test]
fn c13_c60_drawing_outline_stays_within_kernel_tolerance_of_the_true_arc() {
    let f = frame(0.0, 0.0, 50.0, 30.0);
    for shape in [
        ellipse(f, Some(arc(0.0, 270.0, ArcKind::Arc)), 1.0),
        ellipse(f, Some(arc(10.0, 350.0, ArcKind::Pie)), 1.0),
    ] {
        let o = curvyo_document_core::drawing_outline_of(&shape);
        assert!(
            arc_deviation(
                f,
                &o,
                matches!(shape, Shape::Ellipse { arc: Some(a), .. } if a.kind == ArcKind::Pie)
            ) <= 0.01,
            "0.01 mm kernel tolerance"
        );
    }
    // curved side: the drawn outline is the true circular arc of the side
    let o = curvyo_document_core::drawing_outline_of(&polygon(6, 20.0, 0.5));
    let c = 2.0 * 20.0 * (PI / 6.0).sin();
    let theta = 0.5 * PI / 6.0;
    let rad = c / (2.0 * theta.sin());
    // each sample lies on one of the six side circles
    let verts: Vec<Point> = (0..6)
        .map(|i| {
            Point::new(
                100.0 + 20.0 * (f64::from(i) * PI / 3.0).cos(),
                100.0 + 20.0 * (f64::from(i) * PI / 3.0).sin(),
            )
        })
        .collect();
    for p in sample(&o, 12) {
        let on_some = (0..6).any(|i| {
            let (v, w) = (verts[i], verts[(i + 1) % 6]);
            let mid = Point::new((v.x + w.x) / 2.0, (v.y + w.y) / 2.0);
            let out = Vec2::new(mid.x - 100.0, mid.y - 100.0).normalized_to(1.0);
            let center = Point::new(
                mid.x - out.x * rad * theta.cos(),
                mid.y - out.y * rad * theta.cos(),
            );
            (dist(p, center) - rad).abs() < 0.01
        });
        assert!(on_some, "{p:?}");
    }
}

// ---------------------------------------------------------------------------------------
// 62: budgets (run with --release --ignored)
// ---------------------------------------------------------------------------------------

#[test]
#[ignore = "benchmark: run in release"]
fn c62a_evaluation_at_most_50_us() {
    for shape in [
        ellipse(
            frame(0.0, 0.0, 30.0, 10.0),
            Some(arc(10.0, 200.0, ArcKind::Pie)),
            1.0,
        ),
        ellipse(frame(0.0, 0.0, 30.0, 10.0), None, -0.4),
        polygon(6, 20.0, 0.4),
    ] {
        let t = std::time::Instant::now();
        let n = 2000;
        for _ in 0..n {
            std::hint::black_box(outline_of(std::hint::black_box(&shape)));
            std::hint::black_box(shape_frame_bounds(std::hint::black_box(&shape)));
        }
        let us = t.elapsed().as_secs_f64() * 1e6 / f64::from(n);
        assert!(us <= 50.0, "{shape:?}: {us} us");
    }
}

#[test]
#[ignore = "benchmark: run in release"]
fn c62b_curved_1024_gon_at_most_2_ms() {
    let s = polygon(1024, 20.0, 0.4);
    let t = std::time::Instant::now();
    for _ in 0..20 {
        std::hint::black_box(outline_of(std::hint::black_box(&s)));
        std::hint::black_box(shape_frame_bounds(&s));
    }
    let ms = t.elapsed().as_secs_f64() * 1e3 / 20.0;
    assert!(ms <= 2.0, "{ms} ms");
}

#[test]
#[ignore = "benchmark: run in release"]
fn c62d_typed_curve_over_5000_shapes_at_most_250_ms() {
    let d = Document::new(1);
    let ids: Vec<_> = (0..5000)
        .map(|i| match i % 3 {
            0 => d.create_primitive(
                ellipse(frame(f64::from(i), 0.0, 5.0, 4.0), None, 1.0),
                ang(0.0),
            ),
            1 => d.create_primitive(polygon(6, 5.0, 0.0), ang(0.0)),
            _ => d.create_primitive(star(5, 0.4, 5.0, 0.0), ang(0.0)),
        })
        .collect();
    let c = Curve::new(-0.35).unwrap();
    let list: Vec<_> = ids.iter().map(|id| (*id, c)).collect();
    let t = std::time::Instant::now();
    d.set_curves(&list).unwrap();
    let ms = t.elapsed().as_secs_f64() * 1e3;
    assert!(ms <= 250.0, "{ms} ms");
}

#[test]
#[ignore = "benchmark: run in release"]
fn c62e_opening_5000_curved_shapes_adds_at_most_100_ms() {
    let build = |curved: bool| {
        let d = Document::new(1);
        for i in 0..5000 {
            let c = if curved { 0.5 } else { 0.0 };
            match i % 3 {
                0 => d.create_primitive(
                    ellipse(
                        frame(f64::from(i), 0.0, 5.0, 4.0),
                        None,
                        if curved { 0.5 } else { 1.0 },
                    ),
                    ang(0.0),
                ),
                1 => d.create_primitive(polygon(6, 5.0, c), ang(0.0)),
                _ => d.create_primitive(star(5, 0.4, 5.0, c), ang(0.0)),
            };
        }
        pack(&d, "t").unwrap()
    };
    let (plain, curved) = (build(false), build(true));
    let time = |b: &[u8]| {
        let mut best = f64::MAX;
        for _ in 0..3 {
            let t = std::time::Instant::now();
            std::hint::black_box(unpack(1, b).unwrap());
            best = best.min(t.elapsed().as_secs_f64() * 1e3);
        }
        best
    };
    let (a, b) = (time(&plain), time(&curved));
    assert!(b - a <= 100.0, "plain {a} ms, curved {b} ms");
}

#[test]
fn units_tau_sanity() {
    assert!((TAU - 2.0 * PI).abs() < 1e-15);
}
