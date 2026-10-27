//! Slice 3 of `specs/0024-svg-import-export` (criteria 111 to 124): Inkscape's arcs, polygons
//! and stars as primitives when their outline is the drawn `d` within 0.001 mm, spirals and 3D
//! boxes as named paths.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::fmt::Write as _;

use curvyo_document_core::{ArcKind, DrawingNode, Shape};
use curvyo_ui_core::{SvgPlacement, import_svg_bytes};

const NS: &str = r#"xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd""#;

fn import(body: &str) -> Vec<DrawingNode> {
    let text =
        format!(r#"<svg {NS} width="210mm" height="297mm" viewBox="0 0 210 297">{body}</svg>"#);
    import_svg_bytes(text.as_bytes(), SvgPlacement::File)
        .unwrap()
        .nodes
}

fn one(body: &str) -> DrawingNode {
    let mut nodes = import(body);
    assert_eq!(nodes.len(), 1, "{nodes:?}");
    nodes.remove(0)
}

fn name(node: &DrawingNode) -> Option<&str> {
    match node {
        DrawingNode::Group { registers, .. }
        | DrawingNode::Path { registers, .. }
        | DrawingNode::Primitive { registers, .. } => registers.name.as_deref(),
        _ => None,
    }
}

/// An Inkscape arc element; `extra` holds the angle and kind attributes.
fn arc(cx: f64, cy: f64, rx: f64, ry: f64, extra: &str, d: &str) -> String {
    format!(
        r#"<path sodipodi:type="arc" sodipodi:cx="{cx}" sodipodi:cy="{cy}" sodipodi:rx="{rx}" sodipodi:ry="{ry}" {extra} d="{d}"/>"#
    )
}

const QUARTER: &str = r#"sodipodi:start="0" sodipodi:end="1.5708""#;

/// The arc's kind and angles in degrees, its name, rotation in degrees and frame.
fn arc_of(node: &DrawingNode) -> (ArcKind, f64, f64) {
    let DrawingNode::Primitive {
        shape: Shape::Ellipse { arc: Some(arc), .. },
        ..
    } = node
    else {
        panic!("not an arc: {node:?}")
    };
    (
        arc.kind,
        arc.start().as_radians().to_degrees(),
        arc.end().as_radians().to_degrees(),
    )
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn ac112_slice_arc_and_chord_of_the_example() {
    let pie = one(&arc(
        50.0,
        50.0,
        20.0,
        20.0,
        &format!(r#"{QUARTER} sodipodi:arc-type="slice""#),
        "M 70,50 A 20,20 0 0 1 50,70 L 50,50 Z",
    ));
    let (kind, start, end) = arc_of(&pie);
    assert_eq!(kind, ArcKind::Pie);
    assert!(
        near(start, 0.0, 1e-9) && near(end, 90.0, 1e-3),
        "{start} {end}"
    );
    assert_eq!(name(&pie), Some("Arc"));
    let DrawingNode::Primitive {
        shape: Shape::Ellipse { frame, .. },
        rotation,
        ..
    } = &pie
    else {
        panic!()
    };
    assert_eq!((frame.center.x, frame.center.y), (50.0, 50.0));
    assert_eq!((frame.rx.as_mm(), frame.ry.as_mm()), (20.0, 20.0));
    assert_eq!(rotation.as_radians(), 0.0);

    let chord = one(&arc(
        50.0,
        50.0,
        20.0,
        20.0,
        &format!(r#"{QUARTER} sodipodi:arc-type="chord" sodipodi:open="true""#),
        "M 70,50 A 20,20 0 0 1 50,70 Z",
    ));
    assert_eq!(arc_of(&chord).0, ArcKind::Chord);
    let open = one(&arc(
        50.0,
        50.0,
        20.0,
        20.0,
        &format!(r#"{QUARTER} sodipodi:arc-type="arc" sodipodi:open="true""#),
        "M 70,50 A 20,20 0 0 1 50,70",
    ));
    assert_eq!(arc_of(&open).0, ArcKind::Arc);
}

#[test]
fn ac112_kind_without_arc_type_and_end_before_start() {
    let open = one(&arc(
        50.0,
        50.0,
        20.0,
        20.0,
        &format!(r#"{QUARTER} sodipodi:open="true""#),
        "M 70,50 A 20,20 0 0 1 50,70",
    ));
    assert_eq!(arc_of(&open).0, ArcKind::Arc, "0.92: open is an arc");
    let pie = one(&arc(
        50.0,
        50.0,
        20.0,
        20.0,
        QUARTER,
        "M 70,50 A 20,20 0 0 1 50,70 L 50,50 Z",
    ));
    assert_eq!(arc_of(&pie).0, ArcKind::Pie, "no open: a pie");
    // From 270° over 0° to 90°: the end is raised by 2π.
    let past = one(&arc(
        50.0,
        50.0,
        20.0,
        20.0,
        r#"sodipodi:start="4.712389" sodipodi:end="1.5707963" sodipodi:arc-type="slice""#,
        "M 50,30 A 20,20 0 0 1 70,50 20,20 0 0 1 50,70 L 50,50 Z",
    ));
    let (_, start, end) = arc_of(&past);
    assert!(near(start, 270.0, 1e-4) && near(end, 90.0, 1e-4));
    let unknown = one(&arc(
        50.0,
        50.0,
        20.0,
        20.0,
        &format!(r#"{QUARTER} sodipodi:arc-type="wedge""#),
        "M 70,50 A 20,20 0 0 1 50,70 L 50,50 Z",
    ));
    assert!(
        matches!(unknown, DrawingNode::Path { .. }),
        "unreadable kind"
    );
    assert_eq!(name(&unknown), None);
}

#[test]
fn ac113_whole_ellipses_from_the_arc_tool() {
    let whole = one(&arc(
        30.0,
        20.0,
        10.0,
        5.0,
        r#"sodipodi:open="true" sodipodi:arc-type="arc""#,
        "M 40,20 A 10,5 0 1 1 20,20 10,5 0 1 1 40,20 Z",
    ));
    let DrawingNode::Primitive {
        shape:
            Shape::Ellipse {
                frame,
                arc: whole_arc,
                ..
            },
        ..
    } = &whole
    else {
        panic!("{whole:?}")
    };
    assert!(whole_arc.is_none());
    assert_eq!(
        (
            frame.center.x,
            frame.center.y,
            frame.rx.as_mm(),
            frame.ry.as_mm()
        ),
        (30.0, 20.0, 10.0, 5.0)
    );
    assert_eq!(name(&whole), None, "a whole ellipse takes the default name");
    let swept = one(&arc(
        30.0,
        20.0,
        10.0,
        10.0,
        &format!(r#"sodipodi:start="0" sodipodi:end="{}""#, TAU - 0.0005),
        "M 40,20 A 10,10 0 1 1 20,20 10,10 0 1 1 40,20 Z",
    ));
    assert!(matches!(
        swept,
        DrawingNode::Primitive {
            shape: Shape::Ellipse { arc: None, .. },
            ..
        }
    ));
}

const PIE: &str = r#"sodipodi:start="0" sodipodi:end="1.5707963" sodipodi:arc-type="slice""#;
const PIE_D: &str = "M 70,50 A 20,20 0 0 1 50,70 L 50,50 Z";

fn transformed_pie(transform: &str) -> DrawingNode {
    one(&arc(
        50.0,
        50.0,
        20.0,
        20.0,
        &format!(r#"{PIE} transform="{transform}""#),
        PIE_D,
    ))
}

#[test]
fn ac114_transforms_of_an_arc() {
    let rotated = transformed_pie("rotate(30)");
    let (_, start, end) = arc_of(&rotated);
    let DrawingNode::Primitive { rotation, .. } = &rotated else {
        panic!()
    };
    assert!(near(rotation.as_radians().to_degrees(), 30.0, 1e-9));
    assert!(near(start, 0.0, 1e-9) && near(end, 90.0, 1e-4));

    let mirrored = transformed_pie("translate(200 0) scale(-1 1)");
    let (kind, start, end) = arc_of(&mirrored);
    assert_eq!(kind, ArcKind::Pie);
    assert!(near((end - start).rem_euclid(360.0), 90.0, 1e-4));

    let stretched = transformed_pie("scale(2 1)");
    let DrawingNode::Primitive {
        shape: Shape::Ellipse { frame, .. },
        ..
    } = &stretched
    else {
        panic!("{stretched:?}")
    };
    assert_eq!((frame.rx.as_mm(), frame.ry.as_mm()), (40.0, 20.0));

    for other in ["scale(2 1) rotate(30)", "skewX(20)"] {
        let node = transformed_pie(other);
        assert!(matches!(node, DrawingNode::Path { .. }), "{other}");
        assert_eq!(name(&node), None);
    }
}

#[test]
fn ac115_rx_ne_ry_rectangle_stays_a_path() {
    let node = one(r#"<rect x="10" y="10" width="40" height="20" rx="10" ry="5"/>"#);
    assert!(matches!(node, DrawingNode::Path { .. }));
}

/// A star or polygon element with Inkscape's `d` for its numbers (straight sides).
fn star(sides: u32, r1: f64, r2: f64, arg1: f64, arg2: f64, flat: bool, extra: &str) -> String {
    let (cx, cy) = (100.0, 100.0);
    let mut d = String::from("M");
    for k in 0..sides {
        let turn = TAU * f64::from(k) / f64::from(sides);
        let (s, c) = (arg1 + turn).sin_cos();
        write!(d, " {:.6},{:.6}", cx + r1 * c, cy + r1 * s).unwrap();
        if !flat {
            let (s, c) = (arg2 + turn).sin_cos();
            write!(d, " {:.6},{:.6}", cx + r2 * c, cy + r2 * s).unwrap();
        }
    }
    d.push_str(" Z");
    format!(
        r#"<path sodipodi:type="star" sodipodi:sides="{sides}" sodipodi:cx="{cx}" sodipodi:cy="{cy}" sodipodi:r1="{r1}" sodipodi:r2="{r2}" sodipodi:arg1="{arg1}" sodipodi:arg2="{arg2}" inkscape:flatsided="{flat}" {extra} d="{d}"/>"#
    )
}

#[test]
fn ac116_hexagon_is_a_polygon() {
    let node = one(&star(6, 10.0, 8.660_254, 0.0, PI / 6.0, true, ""));
    let DrawingNode::Primitive {
        shape: Shape::Polygon {
            frame, point_count, ..
        },
        ..
    } = &node
    else {
        panic!("{node:?}")
    };
    assert_eq!(point_count.get(), 6);
    assert!(near(frame.radius.as_mm(), 10.0, 1e-9) && near(frame.angle.as_radians(), 0.0, 1e-9));
    assert_eq!(name(&node), None);
}

#[test]
fn ac117_star_and_twisted_star() {
    let node = one(&star(
        5,
        10.0,
        4.0,
        -FRAC_PI_2,
        -FRAC_PI_2 + PI / 5.0,
        false,
        "",
    ));
    let DrawingNode::Primitive {
        shape: Shape::Star {
            inner_ratio, frame, ..
        },
        ..
    } = &node
    else {
        panic!("{node:?}")
    };
    assert!(near(inner_ratio.get(), 0.4, 1e-9));
    assert!(near(frame.angle.as_radians(), -FRAC_PI_2, 1e-9));
    let twisted = one(&star(5, 10.0, 4.0, -FRAC_PI_2, -FRAC_PI_2 + 0.7, false, ""));
    assert!(matches!(twisted, DrawingNode::Path { .. }));
}

#[test]
fn ac118_rounded_and_randomized_stars_are_paths() {
    for extra in [r#"inkscape:rounded="0.3""#, r#"inkscape:randomized="0.2""#] {
        let node = one(&star(
            5,
            10.0,
            4.0,
            -FRAC_PI_2,
            -FRAC_PI_2 + PI / 5.0,
            false,
            extra,
        ));
        let DrawingNode::Path { outlines, .. } = &node else {
            panic!("{extra}")
        };
        assert_eq!(outlines[0].anchors.len(), 10, "{extra}: the points of d");
    }
}

#[test]
fn ac119_ac121_spiral_is_one_path_named_spiral() {
    let node = one(
        r#"<path sodipodi:type="spiral" sodipodi:cx="40" sodipodi:cy="160" sodipodi:expansion="1" sodipodi:revolution="2" sodipodi:radius="12" sodipodi:argument="-12" sodipodi:t0="0" d="m 40,160 c 0.6,0.4 -0.3,1.3 -1,1.2 -1.6,-0.1 -2,-2.1 -1.4,-3.3"/>"#,
    );
    let DrawingNode::Path { outlines, .. } = &node else {
        panic!()
    };
    assert_eq!(outlines.len(), 1);
    assert!(!outlines[0].closed);
    assert_eq!(name(&node), Some("Spiral"));
    let labelled =
        one(r#"<path sodipodi:type="spiral" inkscape:label="Coil" d="m 40,160 c 1,1 2,2 3,0"/>"#);
    assert_eq!(name(&labelled), Some("Coil"), "a label wins");
}

fn face(d: &str) -> String {
    format!(r#"<path sodipodi:type="inkscape:box3dside" inkscape:box3dsidetype="6" d="{d}"/>"#)
}

#[test]
fn ac120_ac121_ac122_box_is_a_group_of_named_faces() {
    let faces: String = (0..6)
        .map(|i| face(&format!("M {i},0 L {},0 {i},10 Z", i + 5)))
        .collect();
    let node = one(&format!(
        r##"<g sodipodi:type="inkscape:box3d" inkscape:perspectiveID="#perspective1" transform="translate(10 20)">{faces}</g>"##
    ));
    let DrawingNode::Group { children, .. } = &node else {
        panic!("{node:?}")
    };
    assert_eq!(name(&node), Some("3D box"));
    let names: Vec<_> = children.iter().map(name).collect();
    assert_eq!(
        names,
        ["Face 1", "Face 2", "Face 3", "Face 4", "Face 5", "Face 6"].map(Some)
    );
    let DrawingNode::Path { outlines, .. } = &children[2] else {
        panic!()
    };
    let first = outlines[0].anchors[0].point;
    assert_eq!((first.x, first.y), (12.0, 20.0), "the transform is baked");
    let labelled = one(&format!(
        r#"<g sodipodi:type="inkscape:box3d" inkscape:label="Crate">{}</g>"#,
        face("M 0,0 L 5,0 0,10 Z")
    ));
    assert_eq!(name(&labelled), Some("Crate"));
}

#[test]
fn ac123_a_moved_node_makes_the_arc_a_path() {
    let moved = |dx: f64| {
        one(&arc(
            50.0,
            50.0,
            20.0,
            20.0,
            PIE,
            &format!("M 70,50 A 20,20 0 0 1 50,70 L {},50 Z", 50.0 + dx),
        ))
    };
    assert!(matches!(moved(0.002), DrawingNode::Path { .. }));
    arc_of(&moved(0.0005));
}

#[test]
fn ac111_large_arcs_are_checked_against_the_true_arc() {
    // A 900 mm radius: the eighth-turn cubics of `d` are 0.004 mm off the true arc.
    let text = format!(
        r#"<svg {NS} width="2000mm" height="2000mm" viewBox="0 0 2000 2000">{}</svg>"#,
        arc(
            1000.0,
            1000.0,
            900.0,
            900.0,
            PIE,
            "M 1900,1000 A 900,900 0 0 1 1000,1900 L 1000,1000 Z"
        )
    );
    let nodes = import_svg_bytes(text.as_bytes(), SvgPlacement::File)
        .unwrap()
        .nodes;
    arc_of(&nodes[0]);
}

#[test]
fn ac124_hostile_numbers_give_paths() {
    let d = "M 70,50 A 20,20 0 0 1 50,70 L 50,50 Z";
    let arcs = [
        r#"sodipodi:cx="50" sodipodi:cy="50" sodipodi:rx="0" sodipodi:ry="20""#,
        r#"sodipodi:cx="50" sodipodi:cy="50" sodipodi:rx="-20" sodipodi:ry="20""#,
        r#"sodipodi:cx="50" sodipodi:cy="50" sodipodi:rx="20" sodipodi:ry="20" sodipodi:start="1e7" sodipodi:end="1e7""#,
        r#"sodipodi:cx="50" sodipodi:cy="50" sodipodi:rx="20" sodipodi:ry="20" sodipodi:start="0" sodipodi:end="inf""#,
        r#"sodipodi:cx="50" sodipodi:cy="50" sodipodi:rx="1e300" sodipodi:ry="20""#,
    ];
    for numbers in arcs {
        let node = one(&format!(r#"<path sodipodi:type="arc" {numbers} d="{d}"/>"#));
        assert!(matches!(node, DrawingNode::Path { .. }), "{numbers}");
    }
    let stars = [
        r#"sodipodi:sides="0" sodipodi:r1="10""#,
        r#"sodipodi:sides="2" sodipodi:r1="10""#,
        r#"sodipodi:sides="2.5" sodipodi:r1="10""#,
        r#"sodipodi:sides="100000" sodipodi:r1="10""#,
        r#"sodipodi:sides="5" sodipodi:r1="-10""#,
        r#"sodipodi:sides="5" sodipodi:r1="NaN""#,
    ];
    for numbers in stars {
        let node = one(&format!(
            r#"<path sodipodi:type="star" {numbers} sodipodi:cx="0" sodipodi:cy="0" sodipodi:r2="-4" sodipodi:arg1="0" sodipodi:arg2="0.6" d="M 10,0 L 0,10 -10,0 Z"/>"#
        ));
        assert!(matches!(node, DrawingNode::Path { .. }), "{numbers}");
    }
    assert!(
        import(r#"<path sodipodi:type="arc" sodipodi:cx="0" sodipodi:cy="0" sodipodi:rx="1" sodipodi:ry="1" d="M 0"/>"#)
            .is_empty(),
        "an unusable d is skipped"
    );
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/svg/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// The kind of every node of the fixture's layer.
fn kinds(name: &str) -> Vec<&'static str> {
    let nodes = import_svg_bytes(&fixture(name), SvgPlacement::File)
        .unwrap()
        .nodes;
    let DrawingNode::Group { children, .. } = &nodes[0] else {
        panic!()
    };
    children
        .iter()
        .map(|node| match node {
            DrawingNode::Primitive { shape, .. } => match shape {
                Shape::Ellipse { arc: Some(_), .. } => "arc",
                Shape::Ellipse { .. } => "ellipse",
                Shape::Polygon { .. } => "polygon",
                Shape::Star { .. } => "star",
                Shape::Rect { .. } => "rect",
            },
            DrawingNode::Path { .. } => "path",
            DrawingNode::Group { .. } => "group",
            _ => "other",
        })
        .collect()
}

#[test]
fn ac125_inkscape_shapes_fixture() {
    assert_eq!(
        kinds("inkscape_shapes.svg"),
        // The tester's real Inkscape 1.4.4 save (`inkscape_shapes_src/`): five arcs, the arc
        // tool's whole ellipse (an `<ellipse>`) and a 359.99° arc, three arcs, the rx ≠ ry
        // rectangle, four polygons, a star, the rounded, randomized and twisted stars, two
        // stars, the spiral, then the skew, stretch and rotate groups and the 3D box.
        [
            "arc", "arc", "arc", "arc", "arc", "ellipse", "ellipse", "arc", "arc", "arc", "arc",
            "path", "polygon", "polygon", "polygon", "polygon", "star", "path", "path", "path",
            "star", "star", "path", "group", "group", "group", "group"
        ]
    );
}

#[test]
fn ac125_inkscape_shapes_edited_fixture() {
    let mut expected = vec!["path", "arc", "path", "star", "path", "arc", "arc", "arc"];
    expected.extend(["path"; 10]);
    assert_eq!(kinds("inkscape_shapes_edited.svg"), expected);
}

/// 1,000 mixed arcs, polygons and stars.
fn mixed() -> String {
    let mut body = String::new();
    for i in 0..1000_u32 {
        let x = f64::from(i % 40) * 5.0 + 3.0;
        let y = f64::from(i / 40) * 5.0 + 3.0;
        body.push_str(&match i % 3 {
            0 => arc(
                x,
                y,
                2.0,
                2.0,
                r#"sodipodi:start="0" sodipodi:end="1.5707963" sodipodi:arc-type="slice""#,
                &format!("M {},{y} A 2,2 0 0 1 {x},{} L {x},{y} Z", x + 2.0, y + 2.0),
            ),
            1 => star(6, 2.0, 1.732_050_8, 0.0, PI / 6.0, true, ""),
            _ => star(5, 2.0, 0.8, -FRAC_PI_2, -FRAC_PI_2 + PI / 5.0, false, ""),
        });
    }
    format!(r#"<svg {NS} width="210mm" height="297mm" viewBox="0 0 210 297">{body}</svg>"#)
}

#[test]
fn ac126_same_bytes_give_the_same_document() {
    let text = mixed();
    let a = import_svg_bytes(text.as_bytes(), SvgPlacement::File).unwrap();
    let b = import_svg_bytes(text.as_bytes(), SvgPlacement::File).unwrap();
    assert_eq!(a.nodes, b.nodes);
    assert_eq!(a.leaves, 1000);
}

#[test]
#[ignore = "budget: run with --release -- --ignored"]
fn ac126_budget_1000_shapes_under_half_a_second() {
    let text = mixed();
    let start = std::time::Instant::now();
    let import = import_svg_bytes(text.as_bytes(), SvgPlacement::File).unwrap();
    let took = start.elapsed();
    let primitives = import
        .nodes
        .iter()
        .filter(|n| matches!(n, DrawingNode::Primitive { .. }))
        .count();
    assert_eq!(primitives, 1000);
    assert!(took.as_secs_f64() < 0.5, "{took:?}");
}
