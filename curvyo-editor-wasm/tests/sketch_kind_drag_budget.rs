//! AC29: dragging each entity kind in a 100-entity sketch stays inside AC6's 16 ms of thread CPU.
#![allow(clippy::unwrap_used)]
mod cpu_time;
use curvyo_document_core::{Document, Point, Style, pack};
use curvyo_editor_wasm::{Session, SketchTool, Workbench};
use curvyo_sketch_core as sk;

/// 30 constrained point/line groups (90 entities) far from the dragged object.
fn background() -> sk::Sketch {
    let mut graph = sk::Sketch::new("origin".into());
    for n in 0..30 {
        let x = f64::from(n) * 40.0 + 200.0;
        let (center, point, line) = (format!("c{n}"), format!("p{n}"), format!("l{n}"));
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
            format!("d{n}").into(),
            sk::Constraint::driving(sk::Relation::Distance {
                a: center.into(),
                b: point.into(),
                value: sk::Expression::Literal(sk::Quantity::length(10.0).unwrap()),
            }),
        );
    }
    graph
}

fn kinds() -> Vec<(&'static str, sk::Construction, (f64, f64))> {
    vec![
        (
            "point",
            sk::Construction::Point(sk::Position::mm(30.0, 30.0)),
            (30.0, 30.0),
        ),
        (
            "line",
            sk::Construction::Polyline(vec![
                sk::Position::mm(20.0, 20.0),
                sk::Position::mm(40.0, 20.0),
            ]),
            (25.0, 20.0),
        ),
        (
            "circle",
            sk::Construction::Circle {
                center: sk::Position::mm(30.0, 30.0),
                radius: sk::Length(10.0),
            },
            (40.0, 30.0),
        ),
        (
            "arc",
            sk::Construction::ArcCenter {
                center: sk::Position::mm(30.0, 30.0),
                start: sk::Position::mm(40.0, 30.0),
                end: sk::Position::mm(30.0, 40.0),
            },
            (30.0 + 10.0 * 0.866_025_403_784_438_6, 35.0),
        ),
        (
            "ellipse",
            sk::Construction::Ellipse {
                center: sk::Position::mm(30.0, 30.0),
                axis_end: sk::Position::mm(42.0, 30.0),
                second_axis: sk::Length(6.0),
            },
            (30.0, 36.0),
        ),
        (
            "rectangle member",
            sk::Construction::Rectangle(sk::Position::mm(20.0, 20.0), sk::Position::mm(40.0, 32.0)),
            (25.0, 20.0),
        ),
        (
            "slot member",
            sk::Construction::Slot {
                start: sk::Position::mm(20.0, 30.0),
                end: sk::Position::mm(40.0, 30.0),
                width: sk::Length(10.0),
            },
            (25.0, 35.0),
        ),
    ]
}

#[test]
fn each_kind_drags_inside_the_frame_budget() {
    for (name, command, edge) in kinds() {
        let graph = sk::construct(&background(), "dragged", &command)
            .unwrap()
            .candidate;
        assert!(graph.entities.len() >= 92, "{name}");
        let d = Document::new(66_600);
        let id = d
            .create_page_sketch(&d.pages()[0].id, &graph, &Style::default())
            .unwrap();
        let placement = d.sketch(id).unwrap().placement;
        let mut s = Session::open(66_601, &pack(&d, "budget").unwrap()).unwrap();
        assert!(s.enter_sketch(id));
        s.set_workbench(Workbench::Sketch);
        s.set_sketch_tool(SketchTool::Select);
        s.pointer_down(placement.apply_point(Point::new(edge.0, edge.1)), false);
        let mut times = Vec::new();
        for frame in 0..60 {
            let offset = 2.0 + f64::from(frame) * 0.2;
            let target = Point::new(edge.0 + offset, edge.1 + offset * 0.5);
            let ((), elapsed) = cpu_time::cpu_time_of(|| {
                s.pointer_hover(placement.apply_point(target), false, false);
                std::hint::black_box(s.sketch_view_json());
                std::hint::black_box(s.draw_list());
            });
            assert!(s.sketch_preview().is_some(), "{name}: frame {frame}");
            if frame >= 10 {
                times.push(elapsed.as_secs_f64() * 1000.0);
            }
        }
        let max = times.iter().copied().fold(0.0, f64::max);
        println!(
            "{name}: thread CPU max {max:.3}ms, debug_assertions={}",
            cfg!(debug_assertions)
        );
        if !cfg!(debug_assertions) {
            assert!(max < 16.0, "{name}: AC6 maximum {max}ms");
        }
    }
}
