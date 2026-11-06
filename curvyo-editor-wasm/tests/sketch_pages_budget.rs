//! The sketch page budgets (`specs/0099-sketch-pages` criterion 20, `adrs.md` note 2): with the
//! 100-entity sketch of `0066` criterion 6 and 20 linked clones on three other pages, each pointer
//! move of (a) the sketch page's frame drag and (b) the page-row drag ghost, with the canvas view
//! built for that frame, costs at most 16 ms of thread CPU (`tests/cpu_time/`). Neither drag
//! re-solves the sketch. Like the other budgets, the time is asserted in a release build; a debug
//! build only prints.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use curvyo_document_core::{
    Document, DocumentSize, NewOutputPage, PageKind, Point, Style, Vec2, pack,
};
use curvyo_editor_wasm::{DropBadge, Session, Tool};
use curvyo_sketch_core as sk;

const BUDGET_S: f64 = 0.016;

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

/// A drawing page, a second drawing page, a laser page and the sketch page; 20 clones spread over
/// the three others.
fn scene() -> (Session, curvyo_document_core::Page) {
    let d = Document::new(9920);
    let second = d.add_page();
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let page = d.add_page_of(&PageKind::Sketch).unwrap();
    let sketch = d
        .create_page_sketch(&page.id, &workload(), &Style::default())
        .unwrap();
    let targets = [d.pages()[0].clone(), second, laser];
    for n in 0..20_u32 {
        let target = &targets[n as usize % 3];
        let to = Vec2::new(
            target.origin.x - page.origin.x + f64::from(n / 3) * 3.0,
            target.origin.y - page.origin.y + f64::from(n / 3) * 3.0,
        );
        d.place_linked_clone(sketch, to).unwrap();
    }
    let mut s = Session::open(9921, &pack(&d, "budget").unwrap()).unwrap();
    s.resize_viewport(1920.0, 1080.0);
    (s, page)
}

fn report(name: &str, worst: f64) {
    eprintln!("{name}: worst {worst:.4} s thread CPU");
    if !cfg!(debug_assertions) {
        assert!(worst <= BUDGET_S, "{name}: {worst:.4} s > {BUDGET_S} s");
    }
}

#[test]
fn a_sketch_frame_drag_moves_within_16_ms_each() {
    let (mut s, frame) = scene();
    s.activate_page(curvyo_document_core::FIRST_PAGE_ID);
    s.set_tool(Tool::Select);
    // Just outside the left edge, in the 6 px band of the frame-edge drag.
    let mm = 2.0 / s.view().scale();
    let start = Point::new(
        frame.origin.x - mm,
        frame.origin.y + frame.size.height.as_mm() / 2.0,
    );
    s.pointer_down(start, false);
    let _ = s.frame_draw_list();
    let mut worst = 0.0_f64;
    for n in 1..=60 {
        let at = Point::new(start.x + f64::from(n) * 0.5, start.y + f64::from(n) * 0.25);
        let ((), time) = cpu_time::cpu_time_of(|| {
            s.pointer_hover(at, false, false);
            let _ = s.frame_draw_list();
        });
        worst = worst.max(time.as_secs_f64());
    }
    let end = Point::new(start.x + 30.0, start.y + 15.0);
    s.pointer_up(end, false, false);
    let rows: Vec<serde_json::Value> = serde_json::from_str(&s.pages_json()).unwrap();
    let moved = rows.iter().find(|r| r["id"] == frame.id.as_str()).unwrap();
    assert!(
        (moved["x"].as_f64().unwrap() - frame.origin.x - 30.0).abs() < 1e-6,
        "the frame moved"
    );
    assert_eq!(s.active_page().id, frame.id, "the dragged page is active");
    report("sketch frame drag", worst);
}

#[test]
fn a_row_drag_ghost_moves_within_16_ms_each() {
    let (mut s, page) = scene();
    s.activate_page(&page.id);
    assert!(s.row_drag_begin(&page.id));
    let _ = s.frame_draw_list();
    let mut worst = 0.0_f64;
    for n in 0..60 {
        let at = Point::new(f64::from(n) * 2.0, 100.0 + f64::from(n));
        let (answer, time) = cpu_time::cpu_time_of(|| {
            let answer = s.row_drag_move(Some(at));
            let _ = s.frame_draw_list();
            answer
        });
        assert_ne!(answer.badge, DropBadge::None);
        worst = worst.max(time.as_secs_f64());
    }
    s.row_drag_cancel();
    report("row drag ghost", worst);
}
