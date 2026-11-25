//! The cost of a Parameters field drag (`specs/0099-sketch-pages` criterion 31, `adrs.md` A4):
//! 200 clones of the 100-entity sketch of `0066` criterion 6, each with a distinct own value; each
//! step of a field drag solves only the clone being changed and builds the canvas view of that
//! frame within 16 ms of thread CPU (`tests/cpu_time/`), and none of the other 199 clones is
//! touched. The file open (200 solves) and the release commit are measured and printed, not
//! budgeted. Like the other budgets, the time is asserted in a release build; a debug build only
//! prints: `cargo test --release -p curvyo-editor-wasm --test sketch_instances_cpu_time
//! -- --nocapture`.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use curvyo_document_core::{
    Document, ExposedKey, InstanceValue, InstanceWrite, NodeId, ObjectSnapshot, Point, Style,
    ValueSet, Vec2, pack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_sketch_core as sk;

const BUDGET_S: f64 = 0.016;
const TIMED: bool = !cfg!(debug_assertions);
const CLONES: u32 = 200;

/// 33 lines, each from a fixed point with a driving distance: 100 entities (`0066` criterion 6).
fn workload() -> sk::Sketch {
    let mut graph = sk::Sketch::new("origin".into());
    for n in 0..33 {
        let x = f64::from(n) * 4.0 - 64.0;
        let (center, point) = (format!("center{n}"), format!("point{n}"));
        graph.entities.insert(
            center.clone().into(),
            sk::Entity::Point {
                position: sk::Position::mm(x, -10.0),
                fixed: true,
            },
        );
        graph.entities.insert(
            point.clone().into(),
            sk::Entity::Point {
                position: sk::Position::mm(x, 10.0),
                fixed: false,
            },
        );
        graph.entities.insert(
            format!("line{n}").into(),
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
                value: sk::Expression::Literal(sk::Quantity::length(20.0).unwrap()),
            }),
        );
    }
    assert_eq!(graph.entities.len(), 100);
    graph
}

fn key() -> ExposedKey {
    ExposedKey::dimension(&"distance0".into())
}

/// The sketch page, its sketch exposing the first distance, and 200 clones on a grid of the
/// pasteboard, each with its own value of it.
fn document() -> (Document, Vec<NodeId>) {
    let d = Document::new(9930);
    let page = d
        .add_page_of(&curvyo_document_core::PageKind::Sketch)
        .unwrap();
    let sketch = d
        .create_page_sketch(&page.id, &workload(), &Style::default())
        .unwrap();
    d.expose_parameter(sketch, &key(), Some("Distance 1"))
        .unwrap();
    let mut clones = Vec::new();
    let mut writes = Vec::new();
    for n in 0..CLONES {
        let to = Vec2::new(
            -2000.0 + f64::from(n % 20) * 150.0,
            600.0 + f64::from(n / 20) * 60.0,
        );
        let clone = d.place_linked_clone(sketch, to).unwrap();
        let mut own = ValueSet::default();
        own.insert(key(), InstanceValue::from_mm(12.0 + f64::from(n) * 0.04));
        writes.push(InstanceWrite {
            clone,
            own,
            last_solvable: None,
        });
        clones.push(clone);
    }
    d.write_instance_values(&writes, "change_parameter", None)
        .unwrap();
    (d, clones)
}

fn report(name: &str, worst: f64) {
    eprintln!("{name}: worst {worst:.4} s thread CPU (budget {BUDGET_S} s, asserted: {TIMED})");
    if TIMED {
        assert!(worst <= BUDGET_S, "{name}: {worst:.4} s > {BUDGET_S} s");
    }
}

#[test]
fn a_field_drag_over_200_distinct_clones_solves_one_clone_per_step_within_16_ms() {
    let (d, clones) = document();
    let bytes = pack(&d, "budget").unwrap();
    let (mut s, opened) = {
        let (opened, time) = cpu_time::cpu_time_of(|| {
            let mut s = Session::open(9931, &bytes).unwrap();
            s.resize_viewport(1920.0, 1080.0);
            // The open itself reads once; the first frame builds every clone's outline.
            let _ = s.frame_draw_list();
            s
        });
        (opened, time)
    };
    eprintln!("open and first frame of 200 distinct clones: {opened:?} (not budgeted)");
    s.set_tool(Tool::Select);
    // Select the first clone by a press on its first anchor, and release.
    let read = d.read_objects();
    let ObjectSnapshot::Clone(first) = read.get(clones[0]).unwrap() else {
        panic!("clone")
    };
    let anchor = first.resolved.anchors[0].point;
    let at = Point::new(anchor.x, anchor.y + 1.0);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    assert!(
        s.sketch_instance_json().contains("\"Distance 1\""),
        "the clone is selected: {}",
        s.sketch_instance_json()
    );
    let outlines: Vec<usize> = clones
        .iter()
        .map(|id| s.object_outline_ptr(*id).unwrap_or_default())
        .collect();
    let solves = s.instance_solve_count();
    let mut worst = 0.0_f64;
    for step in 1..=40 {
        let value = 12.003 + f64::from(step) * 0.05;
        let (answer, time) = cpu_time::cpu_time_of(|| {
            let answer = s.preview_sketch_instance_value("d:distance0", value);
            let _ = s.frame_draw_list();
            answer
        });
        assert!(answer.contains("notices"), "{answer}");
        worst = worst.max(time.as_secs_f64());
    }
    assert_eq!(
        s.instance_solve_count() - solves,
        40,
        "one solve per step and none for the 199 others"
    );
    report("field drag step", worst);
    let after: Vec<_> = clones
        .iter()
        .map(|id| s.object_outline_ptr(*id).unwrap_or_default())
        .collect();
    assert_eq!(outlines, after, "the other clones' outlines are untouched");
    let before = s.instance_solve_count();
    let (written, release) =
        cpu_time::cpu_time_of(|| s.set_sketch_instance_value("d:distance0", 14.0));
    assert!(written.contains("\"written\":true"), "{written}");
    eprintln!(
        "release commit: {release:?} (not budgeted), {} further solves",
        s.instance_solve_count() - before
    );
}
