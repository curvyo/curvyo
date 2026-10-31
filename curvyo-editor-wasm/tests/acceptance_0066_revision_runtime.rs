//! Independent actual-runtime axis and endpoint-normal equations with dimension edits.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, Style, pack, unpack};
use curvyo_editor_wasm::{Session, Workbench};
use curvyo_sketch_core as sk;
const ROOT: &str = "../target/0066-runtime-revision";
fn point(g: &mut sk::Sketch, id: &str, x: f64, y: f64, fixed: bool) {
    g.entities.insert(
        id.into(),
        sk::Entity::Point {
            position: sk::Position::mm(x, y),
            fixed,
        },
    );
}
fn quantity(value: f64) -> sk::Expression {
    sk::Expression::Literal(sk::Quantity::length(value).unwrap())
}
fn fixture(case: &str) -> sk::Sketch {
    let mut g = sk::Sketch::new("origin".into());
    if case == "axis" {
        point(&mut g, "p", 10.0, 0.0, false);
        g.constraints.insert(
            "axis".into(),
            sk::Constraint::driving(sk::Relation::OnAxis {
                point: "p".into(),
                axis: sk::Axis::X,
            }),
        );
        g.constraints.insert(
            "size".into(),
            sk::Constraint::driving(sk::Relation::Distance {
                a: "origin".into(),
                b: "p".into(),
                value: quantity(10.0),
            }),
        );
        return g;
    }
    point(&mut g, "b", 0.0, 0.0, true);
    if case == "arc-arc" {
        point(&mut g, "a", -10.0, 10.0, true);
        point(&mut g, "center", 0.0, 10.0, true);
        g.entities.insert(
            "previous".into(),
            sk::Entity::Arc {
                center: "center".into(),
                start: "a".into(),
                end: "b".into(),
                radius: sk::Length(10.0),
                start_angle: sk::Angle(std::f64::consts::PI),
                end_angle: sk::Angle(1.5 * std::f64::consts::PI),
                fixed_shape: true,
            },
        );
    } else {
        point(&mut g, "a", -10.0, 0.0, true);
        g.entities.insert(
            "previous".into(),
            sk::Entity::Line {
                start: "a".into(),
                end: "b".into(),
            },
        );
    }
    let sign = if case == "right" { -1.0 } else { 1.0 };
    let mode = if sign < 0.0 {
        sk::ContinuationMode::PerpendicularArcRight
    } else {
        sk::ContinuationMode::PerpendicularArcLeft
    };
    let construction = sk::construct(
        &g,
        "curve",
        &sk::Construction::Continue {
            previous: "previous".into(),
            start: "b".into(),
            end: "c".into(),
            position: sk::Position::mm(6.0, sign * 6.0),
            mode,
        },
    )
    .unwrap();
    g = construction.intent;
    point(&mut g, "ref0", 6.0, -30.0, true);
    point(&mut g, "ref1", 6.0, 30.0, true);
    g.entities.insert(
        "ref".into(),
        sk::Entity::Line {
            start: "ref0".into(),
            end: "ref1".into(),
        },
    );
    g.constraints.insert(
        "endpoint-x".into(),
        sk::Constraint::driving(sk::Relation::On {
            point: "c".into(),
            curve: "ref".into(),
        }),
    );
    g.constraints.insert(
        "size".into(),
        sk::Constraint::driving(sk::Relation::Radius {
            curve: "curve:arc".into(),
            value: quantity(6.0),
        }),
    );
    g
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-6, "{actual} != {expected}");
}
fn witness(case: &str, g: &sk::Sketch) {
    near(g.point(&"origin".into()).unwrap().x.0, 0.0);
    near(g.point(&"origin".into()).unwrap().y.0, 0.0);
    if case == "axis" {
        let p = g.point(&"p".into()).unwrap();
        near(p.x.0, 20.0);
        near(p.y.0, 0.0);
        assert!(matches!(
            g.constraints[&"axis".into()].relation,
            sk::Relation::OnAxis { .. }
        ));
        return;
    }
    let p = g.point(&"c".into()).unwrap();
    near(p.x.0, 6.0);
    near(
        p.y.0,
        if case == "right" {
            -84.0_f64.sqrt()
        } else {
            84.0_f64.sqrt()
        },
    );
    let sk::Entity::Arc { center, radius, .. } = &g.entities[&"curve:arc".into()] else {
        panic!("arc")
    };
    let center = g.point(center).unwrap();
    near(center.x.0, 10.0);
    near(center.y.0, 0.0);
    near(radius.0, 10.0);
    near(
        g.point(&"b".into())
            .unwrap()
            .distance(sk::Position::mm(0.0, 0.0))
            .0,
        0.0,
    );
    let a = sk::endpoint_tangent(g, &"previous".into(), &"b".into()).unwrap();
    let b = sk::endpoint_tangent(g, &"curve:arc".into(), &"b".into()).unwrap();
    near(a.0 * b.0 + a.1 * b.1, 0.0);
    assert!(
        g.constraints
            .values()
            .any(|c| matches!(c.relation, sk::Relation::PerpendicularAt { .. }))
    );
}
fn solution(bytes: &[u8]) -> sk::Sketch {
    let d = unpack(991, bytes).unwrap();
    let id = d.page_sketch(&d.pages()[0].id).unwrap();
    let snap = d.sketch(id).unwrap();
    assert!(!snap.stale, "{:?}", snap.diagnosis);
    snap.solution
}
#[test]
fn revised_relations_native_known_answers_and_fixtures() {
    std::fs::create_dir_all(ROOT).unwrap();
    for case in ["axis", "left", "right", "arc-arc"] {
        let input = fixture(case);
        let d = Document::new(990);
        let id = d
            .create_page_sketch(&d.pages()[0].id, &input, &Style::default())
            .unwrap();
        let bytes = pack(&d, "relation runtime").unwrap();
        std::fs::write(format!("{ROOT}/{case}.curvyo"), &bytes).unwrap();
        std::fs::write(format!("{ROOT}/{case}-context.json"),serde_json::to_vec(&serde_json::json!({"owner":id.full_id(),"value":if case=="axis" {"20 mm"} else {"10 mm"}})).unwrap()).unwrap();
        let mut s = Session::open(992, &bytes).unwrap();
        s.set_workbench(Workbench::Sketch);
        assert!(s.enter_sketch(id));
        s.sketch_dimension_text(
            &"size".into(),
            if case == "axis" { "20 mm" } else { "10 mm" },
            false,
        )
        .unwrap();
        let output = s.pack("native relations").unwrap();
        let solved = solution(&output);
        witness(case, &solved);
        for (id, c) in &input.constraints {
            if id != &"size".into() {
                assert_eq!(&solved.constraints[id], c);
            }
        }
        for (id, e) in &input.entities {
            if matches!(e, sk::Entity::Point { fixed: true, .. }) {
                assert_eq!(&solved.entities[id], e);
            }
        }
        std::fs::write(format!("{ROOT}/{case}-native.curvyo"), output).unwrap();
    }
}
#[test]
#[ignore = "requires actual Node/WASM sketch-relations.cjs execution"]
fn revised_relations_actual_node_wasm_parity() {
    for case in ["axis", "left", "right", "arc-arc"] {
        let native = solution(&std::fs::read(format!("{ROOT}/{case}-native.curvyo")).unwrap());
        let wasm = solution(&std::fs::read(format!("{ROOT}/{case}-wasm.curvyo")).unwrap());
        witness(case, &native);
        witness(case, &wasm);
        assert_eq!(native.constraints, wasm.constraints);
        assert_eq!(
            native.entities.keys().collect::<Vec<_>>(),
            wasm.entities.keys().collect::<Vec<_>>()
        );
        for (id, e) in &native.entities {
            match e {
                sk::Entity::Point { position, .. } => {
                    assert!(position.distance(wasm.point(id).unwrap()).0 <= 1e-6);
                }
                sk::Entity::Arc { radius, .. } => {
                    let sk::Entity::Arc { radius: actual, .. } = &wasm.entities[id] else {
                        panic!("arc")
                    };
                    near(actual.0, radius.0);
                }
                _ => {}
            }
        }
        println!(
            "{case}: actual native/Node-WASM points/radii <=1e-6, independent hard equations and relation IDs retained"
        );
    }
}

fn suffix_seed() -> sk::Sketch {
    let mut g = sk::Sketch::new("origin".into());
    for (i, (x, y)) in [(0.0, 0.0), (4.0, 3.0), (8.0, 0.0), (11.0, 4.0)]
        .into_iter()
        .enumerate()
    {
        point(&mut g, &format!("p{i}"), x, y, i == 0);
        if i > 0 {
            g.entities.insert(
                format!("l{i}").into(),
                sk::Entity::Line {
                    start: format!("p{}", i - 1).into(),
                    end: format!("p{i}").into(),
                },
            );
            g.constraints.insert(
                format!("d{i}").into(),
                sk::Constraint::driving(sk::Relation::Distance {
                    a: format!("p{}", i - 1).into(),
                    b: format!("p{i}").into(),
                    value: quantity(5.0),
                }),
            );
        }
    }
    g
}
fn suffix_witness(g: &sk::Sketch) {
    for (i, (x, y)) in [(0.0, 0.0), (3.0, 4.0), (6.0, 0.0), (9.0, 4.0)]
        .into_iter()
        .enumerate()
    {
        near(
            g.point(&format!("p{i}").into())
                .unwrap()
                .distance(sk::Position::mm(x, y))
                .0,
            0.0,
        );
    }
    for i in 1..4 {
        near(
            g.point(&format!("p{i}").into())
                .unwrap()
                .distance(g.point(&format!("p{}", i - 1).into()).unwrap())
                .0,
            5.0,
        );
    }
}
#[test]
fn suffix_native_body_drag_full_target_vector_and_fixture() {
    use curvyo_document_core::Point;
    let d = Document::new(995);
    let input = suffix_seed();
    let id = d
        .create_page_sketch(&d.pages()[0].id, &input, &Style::default())
        .unwrap();
    let placement = d.sketch(id).unwrap().placement;
    let bytes = pack(&d, "suffix runtime").unwrap();
    std::fs::create_dir_all(ROOT).unwrap();
    std::fs::write(format!("{ROOT}/suffix.curvyo"), &bytes).unwrap();
    std::fs::write(format!("{ROOT}/suffix-context.json"),serde_json::to_vec(&serde_json::json!({"owner":id.full_id(),"placement":placement.to_array(),"start":[9.5,2.0],"target":[7.5,2.0]})).unwrap()).unwrap();
    let mut s = Session::open(996, &bytes).unwrap();
    s.set_workbench(Workbench::Sketch);
    assert!(s.enter_sketch(id));
    s.set_sketch_tool(curvyo_editor_wasm::SketchTool::Select);
    s.pointer_down(placement.apply_point(Point::new(9.5, 2.0)), false);
    s.pointer_hover(placement.apply_point(Point::new(7.5, 2.0)), false, false);
    suffix_witness(s.sketch_preview().unwrap());
    s.pointer_up(placement.apply_point(Point::new(7.5, 2.0)), false, false);
    let output = s.pack("suffix native").unwrap();
    let g = solution(&output);
    suffix_witness(&g);
    assert_eq!(g.constraints, input.constraints);
    std::fs::write(format!("{ROOT}/suffix-native.curvyo"), output).unwrap();
}
#[test]
#[ignore = "requires actual Node/WASM sketch-suffix.cjs execution"]
fn suffix_actual_node_wasm_full_target_vector_parity() {
    let native = solution(&std::fs::read(format!("{ROOT}/suffix-native.curvyo")).unwrap());
    let wasm = solution(&std::fs::read(format!("{ROOT}/suffix-wasm.curvyo")).unwrap());
    suffix_witness(&native);
    suffix_witness(&wasm);
    assert_eq!(native.constraints, wasm.constraints);
    for (id, e) in &native.entities {
        if let sk::Entity::Point { position, .. } = e {
            assert!(position.distance(wasm.point(id).unwrap()).0 <= 1e-6);
        }
    }
    println!(
        "actual suffix native/Node-WASM: both raw goals(6,0),(9,4), all3lengths5, prefixjoint(3,4), all points <=1e-6"
    );
}
fn tangent_witness(g: &sk::Sketch) {
    let end = g.point(&"p2".into()).unwrap();
    let joint = g.point(&"p1".into()).unwrap();
    near(end.distance(sk::Position::mm(10.0, 0.0)).0, 0.0);
    near(joint.distance(sk::Position::mm(0.0, 0.0)).0, 5.0);
    near(joint.distance(end).0, 5.0);
    let d = end.x.0.hypot(end.y.0);
    let h = (25.0 - d * d / 4.0).max(0.0).sqrt();
    assert!(joint.y.0 >= -1e-6);
    near(
        joint
            .distance(sk::Position::mm(
                end.x.0 / 2.0 - h * end.y.0 / d,
                end.y.0 / 2.0 + h * end.x.0 / d,
            ))
            .0,
        0.0,
    );
}
#[test]
fn tangent_native_conditioned_endpoint_fixture() {
    use curvyo_document_core::Point;
    let mut g = suffix_seed();
    g.entities.remove(&"p3".into());
    g.entities.remove(&"l3".into());
    g.constraints.remove(&"d3".into());
    point(&mut g, "p1", 3.0, 4.0, false);
    point(&mut g, "p2", 6.0, 0.0, false);
    let d = Document::new(997);
    let id = d
        .create_page_sketch(&d.pages()[0].id, &g, &Style::default())
        .unwrap();
    let placement = d.sketch(id).unwrap().placement;
    let bytes = pack(&d, "tangent runtime").unwrap();
    std::fs::create_dir_all(ROOT).unwrap();
    std::fs::write(format!("{ROOT}/tangent.curvyo"), &bytes).unwrap();
    std::fs::write(format!("{ROOT}/tangent-context.json"),serde_json::to_vec(&serde_json::json!({"owner":id.full_id(),"placement":placement.to_array(),"start":[6.0,0.0],"target":[10.0,0.0]})).unwrap()).unwrap();
    let mut s = Session::open(998, &bytes).unwrap();
    s.set_workbench(Workbench::Sketch);
    assert!(s.enter_sketch(id));
    s.set_sketch_tool(curvyo_editor_wasm::SketchTool::Select);
    s.pointer_down(placement.apply_point(Point::new(6.0, 0.0)), false);
    s.pointer_hover(placement.apply_point(Point::new(10.0, 0.0)), false, false);
    tangent_witness(s.sketch_preview().unwrap());
    s.pointer_up(placement.apply_point(Point::new(10.0, 0.0)), false, false);
    let out = s.pack("tangent native").unwrap();
    tangent_witness(&solution(&out));
    std::fs::write(format!("{ROOT}/tangent-native.curvyo"), out).unwrap();
}
#[test]
#[ignore = "requires actual Node/WASM sketch-suffix.cjs tangent execution"]
fn tangent_actual_node_wasm_all_coordinate_parity() {
    let a = solution(&std::fs::read(format!("{ROOT}/tangent-native.curvyo")).unwrap());
    let b = solution(&std::fs::read(format!("{ROOT}/tangent-wasm.curvyo")).unwrap());
    tangent_witness(&a);
    tangent_witness(&b);
    assert_eq!(a.constraints, b.constraints);
    for (id, e) in &a.entities {
        if let sk::Entity::Point { position, .. } = e {
            assert!(
                position.distance(b.point(id).unwrap()).0 <= 1e-6,
                "singular runtime point {id:?}"
            );
        }
    }
}
#[test]
#[ignore = "requires actual sketch-revision-final-host.cjs contexts saved artifact"]
fn host_dimension_doubleclick_save_resolves_eight_mm_radius() {
    let path = std::env::var("CURVYO_0066_DIMENSION_ARTIFACT").unwrap_or_else(|_| {
        "../target/0066-browser/revision-final-host/doubleclick-dimension.curvyo".into()
    });
    let graph = solution(&std::fs::read(path).unwrap());
    let circles: Vec<_> = graph
        .entities
        .values()
        .filter_map(|e| {
            if let sk::Entity::Circle { radius, .. } = e {
                Some(radius.0)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(circles.len(), 1);
    near(circles[0], 8.0);
}
