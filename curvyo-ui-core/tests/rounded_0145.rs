//! `specs/0145-shape-spiro-handles` slice 1 at the `curvyo-ui-core` level: Object to path of a
//! looped shape (criterion 9), the knob (criteria 1 and 2), the bar state (criterion 3) and
//! the Inkscape import and export of `inkscape:rounded` (criterion 18).
#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]

use curvyo_document_core::{
    Angle, Curve, DrawingNode, Length, NodeId, Point, PointCount, Rounded, Shape, StarFrame,
    outline_of,
};
use curvyo_ui_core::{
    AnchorIdMinter, ParamHandle, SvgPlacement, TransformHandleTolerances,
    build_primitive_conversions, import_svg_bytes, oriented_bounds, param_handles,
};

fn frame(radius: f64, angle: f64) -> StarFrame {
    StarFrame {
        center: Point::new(100.0, 80.0),
        radius: Length::from_mm(radius),
        angle: Angle::from_radians(angle),
    }
}

fn star(q: f64) -> Shape {
    Shape::Star {
        randomized: curvyo_document_core::Randomized::ZERO,
        frame: frame(40.0, 0.0),
        point_count: PointCount::new(5).unwrap(),
        inner_ratio: curvyo_document_core::InnerRatio::new(0.5).unwrap(),
        curve: Curve::POLYGON_DEFAULT,
        rounded: Rounded::new(q).unwrap(),
    }
}

/// Criterion 9: Object to path gives one path with a smooth node per vertex and the cubic
/// handles of the outline, loops kept; q = 0 gives straight segments.
#[test]
fn object_to_path_keeps_the_loops_and_q_zero_stays_straight() {
    for q in [2.5, -1.0, 0.0] {
        let document = curvyo_document_core::Document::new(1);
        let id = document.create_primitive(star(q), Angle::from_radians(0.0));
        let mut minter = AnchorIdMinter::new(7);
        let conversions = build_primitive_conversions(&document, &mut minter, &[id]);
        let anchors = &conversions[0].1;
        assert_eq!(anchors.len(), 10);
        let outline = outline_of(&star(q));
        for (a, o) in anchors.iter().zip(&outline.anchors) {
            assert_eq!(a.point, o.point);
            assert_eq!(a.handle_out, o.handle_out);
            assert_eq!(a.handle_in, o.handle_in);
            if q == 0.0 {
                assert_eq!(a.handle_out.length(), 0.0, "straight segments");
            }
        }
        document.convert_to_paths(&conversions).unwrap();
        assert!(document.primitive(id).is_none() && document.path(id).is_some());
    }
}

fn knobs(shape: Shape, scale: f64) -> Vec<(ParamHandle, Point)> {
    let document = curvyo_document_core::Document::new(1);
    let id: NodeId = document.create_primitive(shape, Angle::from_radians(0.0));
    let object = document.object(id).unwrap();
    param_handles(
        &object,
        &oriented_bounds(&object),
        &TransformHandleTolerances::at_scale(scale),
    )
}

/// Criterion 1: a polygon or star shows a Rounded knob at 15 px inside vertex 0 at q = 0 and at
/// the centre at q = 1; below 72 px there is none, and a drag into loops never makes it vanish
/// (the threshold is measured on the straight shape).
#[test]
fn the_knob_rests_inside_vertex_zero_and_never_vanishes_when_the_box_grows() {
    let rest = knobs(star(0.0), 1.0)
        .into_iter()
        .find(|(h, _)| *h == ParamHandle::Rounded)
        .unwrap()
        .1;
    assert!((rest.x - (100.0 + 40.0 - 15.0)).abs() < 1e-9 && (rest.y - 80.0).abs() < 1e-9);
    let one = knobs(star(1.0), 1.0)
        .into_iter()
        .find(|(h, _)| *h == ParamHandle::Rounded)
        .unwrap()
        .1;
    assert!(
        one.x < 100.0 + 15.0 + 1e-9,
        "q = 1 is displaced off the centre glyph at most"
    );
    // 40 mm radius at 0.9 px/mm is a 72 px box: still shown; at 0.85 below T: none.
    assert!(
        knobs(star(0.0), 0.85)
            .iter()
            .all(|(h, _)| *h != ParamHandle::Rounded),
        "below T the bar field is the way"
    );
    for q in [10.0, -10.0, 3.0] {
        assert!(
            knobs(star(q), 0.9)
                .iter()
                .any(|(h, _)| *h == ParamHandle::Rounded),
            "q {q} keeps its knob"
        );
    }
}

fn import(svg: &str) -> Vec<DrawingNode> {
    import_svg_bytes(svg.as_bytes(), SvgPlacement::File)
        .unwrap()
        .nodes
}

/// Criterion 18 and 4: the real Inkscape 1.4.4 star with `inkscape:rounded="0.3"` (the fixture
/// `inkscape_shapes.svg`, `s_round`) imports as a live star with that value, its outline equal to
/// the file's `d` within 0.001 mm (the import compares them).
#[test]
fn an_inkscape_star_with_rounded_imports_as_a_live_star() {
    let svg = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/svg/inkscape_shapes.svg"),
    )
    .unwrap();
    let nodes = import(&svg);
    let rounded: Vec<f64> = nodes
        .iter()
        .flat_map(flatten)
        .filter_map(|n| match n {
            DrawingNode::Primitive {
                shape: Shape::Star { rounded, .. } | Shape::Polygon { rounded, .. },
                ..
            } if rounded.get() != 0.0 => Some(rounded.get()),
            _ => None,
        })
        .collect();
    assert_eq!(rounded, [0.3], "s_round is a live star with q = 0.3");
}

fn flatten(node: &DrawingNode) -> Vec<&DrawingNode> {
    match node {
        DrawingNode::Group { children, .. } => children.iter().flat_map(flatten).collect(),
        other => vec![other],
    }
}

const NS: &str = r#"xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd""#;

/// Criteria 4 and 18, provisional (`adrs.md` decision 7): a hexagon (R = 10, q = 0.3) whose `d`
/// was computed by hand from decision 1, not exported from Inkscape. A wrong `d` stays a path.
/// TODO(ISSUE-NEEDED): replace by Inkscape 1.4's hexagon export for q -1, -0.3, 0.3, 1, 3.
#[test]
fn a_hand_computed_hexagon_imports_live_and_a_wrong_d_stays_a_path() {
    let d = "M 110.0000000,100.0000000 C 110.0000000,103.0000000 107.5980762,107.1602540 105.0000000,108.6602540 C 102.4019238,110.1602540 97.5980762,110.1602540 95.0000000,108.6602540 C 92.4019238,107.1602540 90.0000000,103.0000000 90.0000000,100.0000000 C 90.0000000,97.0000000 92.4019238,92.8397460 95.0000000,91.3397460 C 97.5980762,89.8397460 102.4019238,89.8397460 105.0000000,91.3397460 C 107.5980762,92.8397460 110.0000000,97.0000000 110.0000000,100.0000000 Z";
    let element = |q: &str, d: &str| {
        format!(
            r#"<svg {NS} width="210mm" height="297mm" viewBox="0 0 210 297"><path sodipodi:type="star" sodipodi:sides="6" sodipodi:cx="100" sodipodi:cy="100" sodipodi:r1="10" sodipodi:r2="10" sodipodi:arg1="0" sodipodi:arg2="0.5235987756" inkscape:flatsided="true" inkscape:rounded="{q}" inkscape:randomized="0" d="{d}"/></svg>"#
        )
    };
    let live = import(&element("0.3", d));
    let DrawingNode::Primitive {
        shape:
            Shape::Polygon {
                rounded,
                point_count,
                ..
            },
        ..
    } = &live[0]
    else {
        panic!("a live polygon: {live:?}")
    };
    assert_eq!((rounded.get(), point_count.get()), (0.3, 6));
    let wrong = import(&element("0.6", d));
    assert!(
        matches!(wrong[0], DrawingNode::Path { .. }),
        "a value that does not draw the file's d stays a path"
    );
}

/// Criterion 8: a looped shape is hit on its outline (an unfilled one), whatever the box says.
#[test]
fn a_looped_shape_is_hit_on_its_drawn_outline() {
    use curvyo_document_core::{Tolerance, drawing_outline_of};
    use curvyo_ui_core::hit_test_object;
    let document = curvyo_document_core::Document::new(1);
    let id = document.create_primitive(star(3.0), Angle::from_radians(0.0));
    let object = document.object(id).unwrap();
    let outline = drawing_outline_of(&star(3.0));
    let (a, b) = (outline.anchors[0], outline.anchors[1]);
    let (p1, p2) = (
        a.point.translated(a.handle_out),
        b.point.translated(b.handle_in),
    );
    let at = |f: fn(Point) -> f64| {
        0.125 * f(a.point) + 0.375 * f(p1) + 0.375 * f(p2) + 0.125 * f(b.point)
    };
    let mid = Point::new(at(|p| p.x), at(|p| p.y));
    assert_eq!(
        hit_test_object(&[object], mid, Tolerance::from_mm(0.5)),
        Some(id)
    );
}

fn outline_samples(shape: &Shape, per_cubic: u32) -> Vec<Point> {
    let outline = curvyo_document_core::drawing_outline_of(shape);
    let n = outline.anchors.len();
    let mut points = Vec::new();
    for i in 0..n {
        let (a, b) = (outline.anchors[i], outline.anchors[(i + 1) % n]);
        let (p1, p2) = (
            a.point.translated(a.handle_out),
            b.point.translated(b.handle_in),
        );
        for k in 0..per_cubic {
            let t = f64::from(k) / f64::from(per_cubic);
            let u = 1.0 - t;
            let at = |f: fn(Point) -> f64| {
                u * u * u * f(a.point)
                    + 3.0 * u * u * t * f(p1)
                    + 3.0 * u * t * t * f(p2)
                    + t * t * t * f(b.point)
            };
            points.push(Point::new(at(|p| p.x), at(|p| p.y)));
        }
    }
    points
}

/// Criterion 7: the selection box the maker sees (`oriented_bounds`) contains 4096 sampled points
/// of a looped outline and touches the extreme ones, for q = 3 and a rotated star.
#[test]
fn the_selection_box_contains_the_loops() {
    for (q, rotation) in [(3.0, 0.0), (-2.0, 0.5)] {
        let document = curvyo_document_core::Document::new(1);
        let id = document.create_primitive(star(q), Angle::from_radians(rotation));
        let object = document.object(id).unwrap();
        let box_ = oriented_bounds(&object);
        let samples = outline_samples(&star(q), 410);
        assert!(samples.len() >= 4096);
        let mut grew = false;
        for p in samples {
            let turned = p.rotated_around(Point::new(100.0, 80.0), Angle::from_radians(rotation));
            let local = box_.to_local(turned);
            assert!(
                local.x >= box_.min.x - 1e-6
                    && local.x <= box_.max.x + 1e-6
                    && local.y >= box_.min.y - 1e-6
                    && local.y <= box_.max.y + 1e-6,
                "q {q}: {turned:?} lies outside the box {box_:?}"
            );
            grew |= local.x > 100.0 + 30.0 + 1e-3 || local.x < 100.0 - 30.0 - 1e-3;
        }
        assert!(grew, "q {q}: the outline leaves the circumscribed box");
    }
}

/// Criterion 8: a click inside a loop of a filled shape selects it (nonzero fill).
#[test]
fn a_click_inside_a_loop_of_a_filled_shape_selects_it() {
    let document = curvyo_document_core::Document::new(1);
    let id = document.create_primitive(star(3.0), Angle::from_radians(0.0));
    document
        .edit_style(&[id], &curvyo_document_core::StyleEdit::FillEnabled(true))
        .unwrap();
    let object = document.object(id).unwrap();
    let outline = curvyo_document_core::drawing_outline_of(&star(3.0));
    let n = outline.anchors.len();
    let mut path = kurbo::BezPath::new();
    path.move_to((outline.anchors[0].point.x, outline.anchors[0].point.y));
    for i in 0..n {
        let (a, b) = (outline.anchors[i], outline.anchors[(i + 1) % n]);
        path.curve_to(
            (a.point.x + a.handle_out.x, a.point.y + a.handle_out.y),
            (b.point.x + b.handle_in.x, b.point.y + b.handle_in.y),
            (b.point.x, b.point.y),
        );
    }
    let outline_points = outline_samples(&star(3.0), 40);
    let mut inside = None;
    'grid: for iy in 0..120 {
        for ix in 0..120 {
            let p = Point::new(40.0 + f64::from(ix), 20.0 + f64::from(iy));
            let far = outline_points
                .iter()
                .all(|q| (q.x - p.x).hypot(q.y - p.y) > 2.0);
            if far && kurbo::Shape::winding(&path, kurbo::Point::new(p.x, p.y)) != 0 {
                inside = Some(p);
                break 'grid;
            }
        }
    }
    let p = inside.unwrap();
    assert_eq!(
        curvyo_ui_core::hit_test_object(
            &[object],
            p,
            curvyo_document_core::Tolerance::from_mm(0.5)
        ),
        Some(id),
        "inside at {p:?}"
    );
}

// Slice 2: the ellipse (criteria 14, 15, 19).

fn ellipse(q: f64, curve: f64) -> Shape {
    Shape::Ellipse {
        frame: curvyo_document_core::EllipseFrame {
            center: Point::new(100.0, 80.0),
            rx: Length::from_mm(50.0),
            ry: Length::from_mm(40.0),
        },
        arc: None,
        curve: Curve::new(curve).unwrap(),
        rounded: Rounded::new(q).unwrap(),
    }
}

fn rounded_knob(shape: Shape) -> Option<Point> {
    knobs(shape, 1.0)
        .into_iter()
        .find(|(h, _)| *h == ParamHandle::Rounded)
        .map(|(_, at)| at)
}

/// Criterion 14: the knob lies on the ray through E, 15 px inside at q = 0 (Curve 0), and an
/// untouched ellipse (q = 0, Curve 100 %) rests at `q_c`, where its look already is.
#[test]
fn the_ellipse_knob_rests_at_q_c_while_untouched() {
    let at_zero = rounded_knob(ellipse(0.0, 0.0)).unwrap();
    assert!((at_zero.x - (100.0 + 50.0 - 15.0)).abs() < 1e-9 && (at_zero.y - 80.0).abs() < 1e-9);
    let untouched = rounded_knob(ellipse(0.0, 1.0)).unwrap();
    let expected = 100.0 + 50.0 - (15.0 + curvyo_document_core::Q_C * (50.0 - 15.0));
    assert!((untouched.x - expected).abs() < 1e-9, "{untouched:?}");
    let one = rounded_knob(ellipse(1.0, 0.0)).unwrap();
    assert!(one.x < 100.0 + 15.0 + 1e-9);
}

/// Criterion 19: an ellipse with an arc has no Rounded knob and takes no value.
#[test]
fn an_ellipse_with_an_arc_has_no_rounded_knob() {
    let Shape::Ellipse { frame, curve, .. } = ellipse(0.0, 1.0) else {
        unreachable!()
    };
    let arc = curvyo_document_core::EllipseArc::new(
        Angle::from_radians(0.2),
        Angle::from_radians(3.5),
        curvyo_document_core::ArcKind::Pie,
    );
    let shape = Shape::Ellipse {
        frame,
        arc,
        curve,
        rounded: Rounded::ZERO,
    };
    assert!(rounded_knob(shape).is_none());
    assert!(!curvyo_ui_core::rounded_capable(&shape));
    assert!(curvyo_ui_core::with_rounded(&shape, 1.0).is_none());
}

/// Criteria 14 and 15: a release at `q_c` on an untouched ellipse writes nothing, a drag past
/// it writes the value and sets Curve to 0, and the drag snaps to `q_c` within 2 percent and
/// writes it exactly.
#[test]
fn a_release_at_q_c_writes_nothing_and_the_drag_snaps_to_it() {
    use curvyo_document_core::Q_C;
    let untouched = ellipse(0.0, 1.0);
    assert!(curvyo_ui_core::with_rounded(&untouched, Q_C).is_none());
    let Some(Shape::Ellipse { curve, rounded, .. }) = curvyo_ui_core::with_rounded(&untouched, 0.8)
    else {
        panic!("a value past q_c is written")
    };
    assert_eq!((curve, rounded.get()), (Curve::POLYGON_DEFAULT, 0.8));
    // Press at q_c, move 0.5 percent of the unit: snapped to q_c exactly; far away: free.
    let unit = 35.0;
    let near = curvyo_ui_core::rounded_from_drag_to(
        Q_C,
        -0.005 * unit,
        unit,
        0.02,
        false,
        &[0.0, 1.0, Q_C],
    );
    assert_eq!(near, Q_C);
    let far =
        curvyo_ui_core::rounded_from_drag_to(Q_C, -0.2 * unit, unit, 0.02, false, &[0.0, 1.0, Q_C]);
    assert!((far - (Q_C + 0.2)).abs() < 1e-12);
    // A typed 0.39 is written (it differs from q_c by less than 0.05 % of the radius).
    assert!(curvyo_ui_core::with_rounded(&untouched, 0.39).is_some());
}

/// Criterion 16: a rounded ellipse is exported as a path with the shadow and comes back as a
/// live ellipse with the same value (`svg_round_trip.rs` pins the bytes); Object to path keeps
/// four smooth nodes with the loops.
#[test]
fn a_rounded_ellipse_converts_to_four_nodes_with_the_loops() {
    let document = curvyo_document_core::Document::new(1);
    let id = document.create_primitive(ellipse(2.5, 0.0), Angle::from_radians(0.0));
    let mut minter = AnchorIdMinter::new(7);
    let conversions = build_primitive_conversions(&document, &mut minter, &[id]);
    let anchors = &conversions[0].1;
    assert_eq!(anchors.len(), 4);
    let outline = outline_of(&ellipse(2.5, 0.0));
    for (a, o) in anchors.iter().zip(&outline.anchors) {
        assert_eq!(a.handle_out, o.handle_out);
    }
    // The handle of E is `q * sqrt(2) * ry` long, along +y.
    assert!((anchors[0].handle_out.y - 2.5 * 2f64.sqrt() * 40.0).abs() < 1e-9);
}

/// Criterion 16: Object to path of an ellipse gives four nodes with the cubic handles
/// of the outline, loops kept at q 3; q 0 (Curve 0) stays straight and `q_c` is today's ellipse.
#[test]
fn object_to_path_of_an_ellipse_keeps_loops_and_q_zero_and_q_c_are_unchanged() {
    use curvyo_document_core::{Q_C, ellipse_outline};
    for q in [3.0, 0.0, Q_C] {
        let shape = ellipse(q, 0.0);
        let document = curvyo_document_core::Document::new(1);
        let id = document.create_primitive(shape, Angle::from_radians(0.0));
        let mut minter = AnchorIdMinter::new(7);
        let conversions = build_primitive_conversions(&document, &mut minter, &[id]);
        let anchors = &conversions[0].1;
        assert_eq!(anchors.len(), 4);
        let outline = outline_of(&shape);
        for (a, o) in anchors.iter().zip(&outline.anchors) {
            assert_eq!(
                (a.point, a.handle_in, a.handle_out),
                (o.point, o.handle_in, o.handle_out)
            );
        }
        match q {
            0.0 => assert!(anchors.iter().all(|a| a.handle_out.length() == 0.0)),
            Q_C => {
                let Shape::Ellipse { frame, .. } = shape else {
                    unreachable!()
                };
                for (a, o) in anchors.iter().zip(&ellipse_outline(frame)) {
                    assert!((a.handle_out - o.handle_out).length() < 1e-9);
                }
            }
            // q 3: the handle of E is 3 * sqrt(2) * ry long, longer than the ellipse is high.
            _ => assert!((anchors[0].handle_out.y - 3.0 * 2f64.sqrt() * 40.0).abs() < 1e-9),
        }
        document.convert_to_paths(&conversions).unwrap();
        assert!(document.primitive(id).is_none() && document.path(id).is_some());
    }
}
