//! Independent Session acceptance from the pre-implementation matrix.
#![allow(clippy::unwrap_used)]
mod cpu_time;
use curvyo_document_core::{Affine, Document, Point, Style, pack, unpack};
use curvyo_editor_wasm::{Session, SketchTool};
use curvyo_sketch_core as sk;
fn workload() -> sk::Sketch {
    let mut graph = sk::Sketch::new("origin".into());
    for n in 0..33 {
        let x = f64::from(n) * 40.0;
        let center = format!("center{n}");
        let point = format!("point{n}");
        let line = format!("line{n}");
        graph.entities.insert(
            center.clone().into(),
            sk::Entity::Point {
                position: sk::Position::mm(x, 0.0),
                fixed: true,
            },
        );
        graph.entities.insert(
            point.clone().into(),
            sk::Entity::Point {
                position: sk::Position::mm(x + 10.0, 0.0),
                fixed: false,
            },
        );
        graph.entities.insert(
            line.into(),
            sk::Entity::Line {
                start: center.clone().into(),
                end: point.clone().into(),
            },
        );
        graph.constraints.insert(
            format!("distance{n}").into(),
            sk::Constraint::driving(sk::Relation::Distance {
                a: center.into(),
                b: point.into(),
                value: sk::Expression::Literal(sk::Quantity::length(10.0).unwrap()),
            }),
        );
    }
    assert_eq!(graph.entities.len(), 100);
    graph
}
fn setup() -> (Session, curvyo_document_core::NodeId, Vec<u8>) {
    let d = Document::new(701);
    let id = d
        .create_sketch(&workload(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let bytes = pack(&d, "acceptance").unwrap();
    let mut s = Session::open(702, &bytes).unwrap();
    assert!(s.enter_sketch(id));
    s.set_workbench(curvyo_editor_wasm::Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Select);
    let mut snap = s.snapping();
    snap.enabled = false;
    s.set_snapping(snap);
    (s, id, bytes)
}
fn residuals(graph: &sk::Sketch) {
    for n in 0..33 {
        let center = graph.point(&format!("center{n}").into()).unwrap();
        let point = graph.point(&format!("point{n}").into()).unwrap();
        assert!((center.distance(point).0 - 10.0).abs() < 1e-6);
        assert!((center.x.0 - f64::from(n) * 40.0).abs() < 1e-6);
        assert!(center.y.0.abs() < 1e-6);
        if n > 0 {
            assert!(
                (point.x.0 - center.x.0 - 10.0).abs() < 1e-6 && point.y.0.abs() < 1e-6,
                "unrelated component {n} moved"
            );
        }
    }
}
#[test]
fn s10_closest_feasible_release_does_not_resurrect_raw_goal() {
    let (mut s, id, _) = setup();
    s.pointer_down(Point::new(10.0, 0.0), false);
    s.pointer_hover(Point::new(20.0, 20.0), false, false);
    let preview = s
        .sketch_preview()
        .unwrap_or_else(|| panic!("no preview: {}", s.sketch_view_json()));
    residuals(preview);
    let point = preview.point(&"point0".into()).unwrap();
    let expected = 10.0 / 2.0_f64.sqrt();
    assert!(
        (point.x.0 - expected).abs() < 1e-4 && (point.y.0 - expected).abs() < 1e-4,
        "{point:?}"
    );
    s.pointer_up(Point::new(20.0, 20.0), false, false);
    s.sketch_remove_constraint(&"distance0".into()).unwrap();
    let d = unpack(703, &s.pack("acceptance").unwrap()).unwrap();
    let actual = d
        .sketch(id)
        .unwrap()
        .solution
        .point(&"point0".into())
        .unwrap();
    assert!(
        actual.distance(point).0 < 1e-6,
        "release {point:?}, after removal {actual:?}"
    );
}
#[test]
fn s15_actual_100_entity_pointer_view_draw_thread_cpu() {
    let (mut s, _, bytes) = setup();
    std::fs::create_dir_all("../target/0066-runtime").unwrap();
    std::fs::write("../target/0066-runtime/input.curvyo", bytes).unwrap();
    s.pointer_down(Point::new(10.0, 0.0), false);
    let mut times = Vec::new();
    for frame in 0..100 {
        let angle = 0.2 + f64::from(frame) * 0.009;
        let radius = if frame % 2 == 0 { 10.0 } else { 20.0 };
        let target = Point::new(radius * angle.cos(), radius * angle.sin());
        let ((), elapsed) = cpu_time::cpu_time_of(|| {
            s.pointer_hover(target, false, false);
            std::hint::black_box(s.sketch_view_json());
            std::hint::black_box(s.draw_list());
        });
        residuals(s.sketch_preview().unwrap());
        if frame >= 10 {
            times.push(elapsed.as_secs_f64() * 1000.0);
        }
    }
    let max = times.iter().copied().fold(0.0, f64::max);
    times.sort_by(f64::total_cmp);
    println!(
        "100 entities (67 points,33 lines),33 driving distances; Linux thread CPU pointer+view+draw n={} median={}ms max={}ms; debug_assertions={}",
        times.len(),
        times[times.len() / 2],
        max,
        cfg!(debug_assertions)
    );
    if !cfg!(debug_assertions) {
        assert!(max < 16.0, "AC6 maximum {max}ms");
    }
}
