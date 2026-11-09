//! The budgets of `specs/0100-bring-content-in` criterion 22 (a) and (b) (`adrs.md` note 2): in
//! a document of 1,000 objects, each pointer move of (a) the cloning drag of a group of 200
//! paths over a laser page and (b) the row-drag ghost of a drawing page of 1,000 objects, with
//! the canvas view built for that frame, costs at most 16 ms of thread CPU (`tests/cpu_time/`).
//! Asserted in a release build; a debug build only prints.

#![allow(
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::cast_precision_loss
)]

mod cpu_time;

use curvyo_document_core::{
    AnchorId, Document, DocumentSize, NewAnchor, NewOutputPage, NodeId, Point, pack,
};
use curvyo_editor_wasm::{DropBadge, Session, Tool};

const BUDGET_S: f64 = 0.016;

fn report(name: &str, worst: f64) {
    eprintln!("{name}: worst {worst:.4} s thread CPU");
    if !cfg!(debug_assertions) {
        assert!(worst <= BUDGET_S, "{name}: {worst:.4} s > {BUDGET_S} s");
    }
}

/// A small closed triangle at `x`, `y`.
fn triangle(d: &Document, n: u64, x: f64, y: f64) -> NodeId {
    let c = |k: u64, dx: f64, dy: f64| {
        NewAnchor::corner(AnchorId::new(7, n * 4 + k), Point::new(x + dx, y + dy))
    };
    d.create_path(&[c(0, 0.0, 0.0), c(1, 4.0, 0.0), c(2, 2.0, 3.0)], true)
}

/// 1,000 objects on the drawing page, the first 200 grouped, and a laser page: the session, the
/// group's leaves, the laser page's centre.
fn scene() -> (Session, Point, Point) {
    let d = Document::new(22);
    let page = d.pages()[0].clone();
    let mut ids = Vec::new();
    for n in 0..1000_u64 {
        let (col, row) = ((n % 40) as f64, (n / 40) as f64);
        ids.push(triangle(
            &d,
            n,
            page.origin.x + 2.0 + col * 5.0,
            page.origin.y + 2.0 + row * 5.0,
        ));
    }
    d.group_objects(&ids[..200]).unwrap();
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let centre = Point::new(
        laser.origin.x + laser.size.width.as_mm() / 2.0,
        laser.origin.y + laser.size.height.as_mm() / 2.0,
    );
    let mut s = Session::open(23, &pack(&d, "budget").unwrap()).unwrap();
    s.resize_viewport(1200.0, 800.0);
    // On the first triangle's base edge: a press selects the whole group.
    let press = Point::new(page.origin.x + 4.0, page.origin.y + 2.0);
    (s, press, centre)
}

#[test]
fn a_the_cloning_drag_of_a_group_of_200_paths_moves_within_16_ms_each() {
    let (mut s, from, centre) = scene();
    s.set_tool(Tool::Select);
    s.pointer_down(from, false);
    s.pointer_hover(Point::new(from.x + 5.0, from.y + 5.0), false, false);
    let _ = s.frame_draw_list();
    let mut worst = 0.0_f64;
    for n in 0..60 {
        let at = Point::new(centre.x + f64::from(n), centre.y + f64::from(n) * 0.5);
        let (badge, time) = cpu_time::cpu_time_of(|| {
            s.pointer_hover(at, false, false);
            let _ = s.frame_draw_list();
            s.clone_drop_answer().map(|a| a.badge)
        });
        assert_eq!(badge, Some(DropBadge::Clone));
        worst = worst.max(time.as_secs_f64());
    }
    s.escape();
    report("cloning drag of 200 paths", worst);
}

#[test]
fn b_the_row_drag_ghost_of_a_page_of_1000_objects_moves_within_16_ms_each() {
    let (mut s, _, centre) = scene();
    let page = curvyo_document_core::FIRST_PAGE_ID;
    assert!(s.row_drag_begin(page));
    let _ = s.frame_draw_list();
    let mut worst = 0.0_f64;
    for n in 0..60 {
        let at = Point::new(centre.x + f64::from(n), centre.y);
        let (answer, time) = cpu_time::cpu_time_of(|| {
            let answer = s.row_drag_move(Some(at));
            let _ = s.frame_draw_list();
            answer
        });
        assert_eq!(answer.badge, DropBadge::PlacedPage);
        worst = worst.max(time.as_secs_f64());
    }
    s.row_drag_cancel();
    report("page row ghost of 1,000 objects", worst);
}
