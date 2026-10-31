//! Independent nonlinear native/WASM runtime corpus with analytic witnesses.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, Style, pack, unpack};
use curvyo_editor_wasm::{Session, Workbench};
use curvyo_sketch_core as sk;
fn seed() -> sk::Sketch {
    let mut g = sk::Sketch::new("origin".into());
    let q = 2.0_f64.sqrt();
    for (id, x, y) in [
        ("center", 20.0, 20.0),
        ("focus", 20.0 + 4.0 / q, 20.0 + 4.0 / q),
        ("a", 20.0 - 13.0 / q, 20.0 - 7.0 / q),
        ("b", 20.0 + 7.0 / q, 20.0 + 13.0 / q),
    ] {
        g.entities.insert(
            id.into(),
            sk::Entity::Point {
                position: sk::Position::mm(x, y),
                fixed: true,
            },
        );
    }
    g.entities.insert(
        "ellipse".into(),
        sk::Entity::Ellipse {
            center: "center".into(),
            focus: "focus".into(),
            minor_radius: sk::Length(2.8),
            fixed_radius: false,
        },
    );
    g.entities.insert(
        "tangent".into(),
        sk::Entity::Line {
            start: "a".into(),
            end: "b".into(),
        },
    );
    g.entities.insert(
        "circle".into(),
        sk::Entity::Circle {
            center: "origin".into(),
            radius: sk::Length(2.0),
            fixed_radius: false,
        },
    );
    g
}
fn solution(bytes: &[u8]) -> sk::Sketch {
    let d = unpack(921, bytes).unwrap();
    let objects = d.read_objects();
    let object = objects
        .objects
        .iter()
        .find_map(|o| {
            if let curvyo_document_core::ObjectSnapshot::Sketch(s) = o {
                Some(s)
            } else {
                None
            }
        })
        .unwrap();
    assert!(!object.stale, "{:?}", object.diagnosis);
    object.solution.clone()
}
fn known(g: &sk::Sketch) {
    let sk::Entity::Ellipse { minor_radius, .. } = g.entities[&"ellipse".into()] else {
        panic!("ellipse missing")
    };
    let sk::Entity::Circle { radius, .. } = g.entities[&"circle".into()] else {
        panic!("circle missing")
    };
    assert!((minor_radius.0 - 3.0).abs() < 1e-6);
    assert!((radius.0 - 5.0).abs() < 1e-6);
    let c = g.point(&"center".into()).unwrap();
    assert!(c.distance(sk::Position::mm(20.0, 20.0)).0 < 1e-6);
}
#[test]
fn s16_native_nonlinear_edit_produces_known_conics_and_runtime_fixture() {
    let d = Document::new(920);
    let id = d
        .create_page_sketch(&d.pages()[0].id, &seed(), &Style::default())
        .unwrap();
    let bytes = pack(&d, "conic runtime acceptance").unwrap();
    std::fs::create_dir_all("../target/0066-runtime-revision").unwrap();
    std::fs::write("../target/0066-runtime-revision/conics.curvyo", &bytes).unwrap();
    std::fs::write("../target/0066-runtime-revision/conics-context.json", serde_json::to_vec(&serde_json::json!({"owner": id.full_id(), "page": d.pages()[0].id, "pageOrigin": [d.pages()[0].origin.x, d.pages()[0].origin.y], "pageSize": [d.pages()[0].size.width.as_mm(),d.pages()[0].size.height.as_mm()], "placement": d.sketch(id).unwrap().placement.to_array()})).unwrap()).unwrap();
    let mut s = Session::open(922, &bytes).unwrap();
    s.set_workbench(Workbench::Sketch);
    assert!(s.enter_sketch(id));
    assert!(s.sketch_select_entity(&"ellipse".into(), false));
    assert!(s.sketch_select_entity(&"tangent".into(), true));
    s.sketch_apply_action("tangent", "", false).unwrap();
    assert!(s.sketch_select_entity(&"circle".into(), false));
    s.sketch_apply_action("radius", "5 mm", false).unwrap();
    let output = s.pack("conic native").unwrap();
    known(&solution(&output));
    std::fs::write(
        "../target/0066-runtime-revision/conics-native.curvyo",
        output,
    )
    .unwrap();
}
#[test]
#[ignore = "requires executed tests/runtime/sketch-conics.cjs production WASM artifacts"]
fn s16_actual_wasm_conics_equal_native_unrounded_geometry() {
    let native =
        solution(&std::fs::read("../target/0066-runtime-revision/conics-native.curvyo").unwrap());
    let wasm =
        solution(&std::fs::read("../target/0066-runtime-revision/conics-wasm.curvyo").unwrap());
    known(&native);
    known(&wasm);
    for (id, e) in &native.entities {
        match e {
            sk::Entity::Point { position, .. } => {
                assert!(position.distance(wasm.point(id).unwrap()).0 <= 1e-6);
            }
            sk::Entity::Circle { radius, .. } => {
                let sk::Entity::Circle { radius: r, .. } = wasm.entities[id] else {
                    panic!("circle")
                };
                assert!((radius.0 - r.0).abs() <= 1e-6);
            }
            sk::Entity::Ellipse { minor_radius, .. } => {
                let sk::Entity::Ellipse {
                    minor_radius: r, ..
                } = wasm.entities[id]
                else {
                    panic!("ellipse")
                };
                assert!((minor_radius.0 - r.0).abs() <= 1e-6);
            }
            _ => {}
        }
    }
}
