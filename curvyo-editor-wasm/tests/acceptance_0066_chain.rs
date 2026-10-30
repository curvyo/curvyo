//! Independent connected 100-entity production Session workload and runtime parity corpus.
#![allow(clippy::unwrap_used)]
mod cpu_time;
use curvyo_document_core::{Affine, Document, Point, Style, pack, unpack};
use curvyo_editor_wasm::{Session, SketchTool, Workbench};
use curvyo_sketch_core as sk;
fn chain() -> sk::Sketch {
    let mut g = sk::Sketch::new("origin".into());
    for n in 0..50 {
        g.entities.insert(
            format!("p{n:02}").into(),
            sk::Entity::Point {
                position: sk::Position::mm(f64::from(n) * 5.0, if n % 2 == 0 { 0.0 } else { 3.0 }),
                fixed: n == 0,
            },
        );
        if n > 0 {
            g.entities.insert(
                format!("l{n:02}").into(),
                sk::Entity::Line {
                    start: format!("p{:02}", n - 1).into(),
                    end: format!("p{n:02}").into(),
                },
            );
            g.constraints.insert(
                format!("d{n:02}").into(),
                sk::Constraint::driving(sk::Relation::Distance {
                    a: format!("p{:02}", n - 1).into(),
                    b: format!("p{n:02}").into(),
                    value: sk::Expression::Literal(sk::Quantity::length(34.0_f64.sqrt()).unwrap()),
                }),
            );
        }
    }
    assert_eq!(g.entities.len(), 100);
    g
}
fn valid(g: &sk::Sketch) {
    let origin = g.point(&"p00".into()).unwrap();
    assert!(origin.distance(sk::Position::mm(0.0, 0.0)).0 < 1e-6);
    for n in 1..50 {
        let a = g.point(&format!("p{:02}", n - 1).into()).unwrap();
        let b = g.point(&format!("p{n:02}").into()).unwrap();
        assert!((a.distance(b).0 - 34.0_f64.sqrt()).abs() < 1e-6, "link {n}");
    }
}
#[test]
fn s15_connected_chain_pointer_view_draw_and_parity_fixture() {
    let d = Document::new(720);
    let id = d
        .create_sketch(&chain(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let bytes = pack(&d, "acceptance").unwrap();
    std::fs::create_dir_all("../target/0066-runtime").unwrap();
    std::fs::write("../target/0066-runtime/chain.curvyo", &bytes).unwrap();
    let mut s = Session::open(721, &bytes).unwrap();
    s.set_workbench(Workbench::Sketch);
    assert!(s.enter_sketch(id));
    s.set_sketch_tool(SketchTool::Select);
    let mut snapping = s.snapping();
    snapping.enabled = false;
    s.set_snapping(snapping);
    let mut start = Point::new(245.0, 3.0);
    let mut records = Vec::new();
    let mut cpu = Vec::new();
    for frame in 0..30 {
        let target = if frame < 20 {
            Point::new(245.0 - f64::from(frame) * 0.3, 10.0 + f64::from(frame))
        } else {
            Point::new(500.0, 200.0)
        };
        s.pointer_down(start, false);
        let ((), elapsed) = cpu_time::cpu_time_of(|| {
            s.pointer_hover(target, false, false);
            std::hint::black_box(s.sketch_view_json());
            std::hint::black_box(s.draw_list());
        });
        let candidate = s
            .sketch_preview()
            .unwrap_or_else(|| panic!("frame {frame}: {}", s.sketch_view_json()))
            .clone();
        valid(&candidate);
        let endpoint = candidate.point(&"p49".into()).unwrap();
        if frame >= 20 {
            let reach = 49.0 * 34.0_f64.sqrt();
            let norm = target.x.hypot(target.y);
            assert!(
                endpoint
                    .distance(sk::Position::mm(
                        reach * target.x / norm,
                        reach * target.y / norm
                    ))
                    .0
                    < 1e-6,
                "triangle bound: {endpoint:?}"
            );
        }
        if frame == 0 || frame == 20 {
            assert!(
                endpoint.distance(sk::Position::mm(target.x, target.y)).0
                    < sk::Position::mm(start.x, start.y)
                        .distance(sk::Position::mm(target.x, target.y))
                        .0,
                "frame {frame} must move toward target"
            );
        }
        s.pointer_up(target, false, false);
        let committed = unpack(722, &s.pack("acceptance").unwrap())
            .unwrap()
            .sketch(id)
            .unwrap()
            .solution;
        valid(&committed);
        records.push(serde_json::json!({"target":[target.x,target.y],"points":(0..50).map(|n|{let p=committed.point(&format!("p{n:02}").into()).unwrap();[p.x.0,p.y.0]}).collect::<Vec<_>>() }));
        start = Point::new(endpoint.x.0, endpoint.y.0);
        if frame >= 3 {
            cpu.push(elapsed.as_secs_f64() * 1000.0);
        }
    }
    std::fs::write(
        "../target/0066-runtime/chain-native.json",
        serde_json::to_vec_pretty(&records).unwrap(),
    )
    .unwrap();
    let max = cpu.iter().copied().fold(0.0, f64::max);
    cpu.sort_by(f64::total_cmp);
    println!(
        "connected100entities=origin+50points+49lines,49distances; Linux threadCPU pointer+view+draw n={} median={}ms max={}ms debug={}",
        cpu.len(),
        cpu[cpu.len() / 2],
        max,
        cfg!(debug_assertions)
    );
    if !cfg!(debug_assertions) {
        assert!(max < 16.0, "AC6 max {max}ms");
    }
}

#[test]
#[ignore = "requires executed Node/WASM artifacts from tests/runtime/sketch-chain.cjs"]
fn s16_actual_node_wasm_unrounded_geometry_matches_native() {
    let records: serde_json::Value =
        serde_json::from_slice(&std::fs::read("../target/0066-runtime/chain-native.json").unwrap())
            .unwrap();
    let mut worst: f64 = 0.0;
    for (index, record) in records.as_array().unwrap().iter().enumerate() {
        let bytes =
            std::fs::read(format!("../target/0066-runtime/chain-wasm-{index}.curvyo")).unwrap();
        let document = unpack(799, &bytes).unwrap();
        let object = document
            .read_objects()
            .objects
            .iter()
            .find_map(|object| {
                if let curvyo_document_core::ObjectSnapshot::Sketch(sketch) = object {
                    Some(sketch.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert!(!object.stale, "frame {index}: {:?}", object.diagnosis);
        assert_eq!(object.solution.entities.len(), 100);
        assert_eq!(object.solution.constraints.len(), 49);
        valid(&object.solution);
        for point in 0..50 {
            let actual = object
                .solution
                .point(&format!("p{point:02}").into())
                .unwrap();
            let expected = &record["points"][point];
            let error = actual
                .distance(sk::Position::mm(
                    expected[0].as_f64().unwrap(),
                    expected[1].as_f64().unwrap(),
                ))
                .0;
            worst = worst.max(error);
            assert!(
                error <= 1e-6,
                "frame {index}, p{point:02}, deviation {error} mm"
            );
        }
    }
    println!(
        "actual Node/WASM vs native {} frames ×50 unrounded points: maximum {worst}mm",
        records.as_array().unwrap().len()
    );
}

#[test]
#[ignore = "requires frontend/tests/browser/sketch-host-cpu.cjs saved artifact"]
fn s15_actual_host_drag_changed_the_connected_chain() {
    let artifact = std::env::var("CURVYO_0066_HOST_ARTIFACT")
        .unwrap_or_else(|_| "../target/0066-browser/chain-host.curvyo".into());
    let d = unpack(950, &std::fs::read(artifact).unwrap()).unwrap();
    let objects = d.read_objects();
    let snapshot = objects
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
    assert!(!snapshot.stale);
    valid(&snapshot.solution);
    let p = snapshot.solution.point(&"p49".into()).unwrap();
    println!("actual host committed endpoint: {p:?}");
    assert!(
        p.distance(sk::Position::mm(245.0, 3.0)).0 > 5.0,
        "host measurement must execute a real drag"
    );
    // Browser pixels/page-frame CSS rounding can differ submillimetres; the exact
    // native/WASM parity test above is the strict unrounded coordinate oracle.
    assert!(p.distance(sk::Position::mm(241.15, 13.8)).0 < 0.1);
}
