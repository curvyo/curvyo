//! `specs/0101-laser-operations` criterion 29 in thread CPU time: one press assigning a selection of
//! 1,000 items within 16 ms; the summary recomputed after a change on a page with 1,000 items and 50
//! operations within 16 ms; the Operations tab with 50 operations and 1,000 items showing its rows
//! within 100 ms; and one move of an operation among 50 within 16 ms (the drag's pointer moves cost
//! no document write; the one commit on drop is what is timed). Budgets are asserted in release
//! builds (the `performance-budgets` job); a debug build prints the times and checks the rest.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

mod cpu_time;
mod operation_shims;
use operation_shims::add_operation;

use std::time::Duration;

use curvyo_document_core::{
    Document, DocumentSize, NewOperation, NewOutputPage, NodeId, Point, RectBounds, pack,
};
use curvyo_editor_wasm::{Session, Tool};

const BUDGET: Duration = Duration::from_millis(16);
const TABLE_BUDGET: Duration = Duration::from_millis(100);

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

/// A laser page of 400 x 300 mm with `items` squares on it; `operations` operations, the square
/// `k` in operation `k % operations` (none when `operations` is 0). Returns the file and the laser
/// page id.
fn page_file(items: usize, operations: usize) -> (Vec<u8>, String) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let origin = laser.origin;
    let squares: Vec<NodeId> = (0..items)
        .map(|i| {
            let (col, row) = ((i % 40) as f64, (i / 40) as f64);
            let x = origin.x + 2.0 + col * 9.0;
            let y = origin.y + 2.0 + row * 9.0;
            d.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 6.0, y + 6.0),
            ))
        })
        .collect();
    let m = laser.kind.manufacturing().unwrap();
    for k in 0..operations {
        let members: Vec<Vec<NodeId>> = squares
            .iter()
            .enumerate()
            .filter(|(i, _)| i % operations == k)
            .map(|(_, id)| vec![*id])
            .collect();
        let index = d.operations(m).unwrap().len();
        d.add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: &format!("Cut {}", k + 1),
            },
            index,
            &members,
        )
        .unwrap();
    }
    (pack(&d, "budget").unwrap(), laser.id)
}

fn open_on(bytes: &[u8], laser: &str) -> Session {
    let mut s = Session::open(2, bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    assert!(s.activate_page(laser));
    s
}

/// A marquee from far above-left to far below-right: every object on the page.
fn select_everything(s: &mut Session) {
    let from = Point::new(-5000.0, -5000.0);
    let to = Point::new(50_000.0, 50_000.0);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

/// The cheapest of five runs of `work` on fresh sessions made by `make`.
fn cheapest<S>(mut make: impl FnMut() -> S, mut work: impl FnMut(&mut S)) -> Duration {
    (0..5)
        .map(|_| {
            let mut s = make();
            cpu_time::cpu_time_of(|| work(&mut s)).1
        })
        .min()
        .unwrap()
}

#[test]
fn criterion_29_a_press_assigning_a_thousand_selected_items() {
    let (bytes, laser) = page_file(1000, 0);
    let cost = cheapest(
        || {
            let mut s = open_on(&bytes, &laser);
            select_everything(&mut s);
            assert_eq!(s.selected_object_count(), 1000);
            s
        },
        |s| assert!(add_operation(s, "cut").is_some()),
    );
    eprintln!("criterion 29 a: press of 1000 items {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_29_b_the_summary_after_a_change_on_1000_items_and_50_operations() {
    let (bytes, laser) = page_file(1000, 50);
    let cost = cheapest(
        || {
            let s = open_on(&bytes, &laser);
            let _ = s.operations_view_json();
            s
        },
        |s| {
            let id = s.operations_view().unwrap().rows[0].id;
            s.set_operation_switch(id, curvyo_document_core::OperationSwitch::Show, false);
            let json = s.operations_view_json();
            assert!(json.contains("summary"));
        },
    );
    eprintln!("criterion 29 b: change and summary {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_29_c_the_tab_with_50_operations_and_1000_items_shows_its_rows() {
    let (bytes, laser) = page_file(1000, 50);
    let cost = cheapest(
        || open_on(&bytes, &laser),
        |s| {
            let json = s.operations_view_json();
            assert!(json.contains("\"rows\""));
        },
    );
    eprintln!("criterion 29 c: tab first rows {cost:?} (CPU time, cold index)");
    assert!(!TIMED || cost < TABLE_BUDGET, "{cost:?}");
}

#[test]
fn criterion_29_d_one_operation_move_among_50() {
    let (bytes, laser) = page_file(1000, 50);
    let cost = cheapest(
        || {
            let s = open_on(&bytes, &laser);
            let id = s.operations_view().unwrap().rows[0].id;
            (s, id)
        },
        |(s, id)| assert!(s.move_operation(*id, 49)),
    );
    eprintln!("criterion 29 d: one move among 50 {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_29_e_the_output_page_draw_list_with_50_operations() {
    let (bytes, laser) = page_file(1000, 50);
    let cost = cheapest(
        || open_on(&bytes, &laser),
        |s| {
            let list = s.draw_list();
            assert_ne!(list.triangle_count(), 0);
        },
    );
    eprintln!("criterion 29 e: output page draw list, operations look {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

/// Criterion 29 with the hatch (customer 2026-11-10, ux-notes 13.4): 1,000 squares, all in one
/// Engrave + Outline operation, so every item is hatched and carries a symbol chip.
fn engraved_file(items: usize) -> (Vec<u8>, String) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let origin = laser.origin;
    let squares: Vec<NodeId> = (0..items)
        .map(|i| {
            let (col, row) = ((i % 40) as f64, (i / 40) as f64);
            let x = origin.x + 2.0 + col * 9.0;
            let y = origin.y + 2.0 + row * 9.0;
            d.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 6.0, y + 6.0),
            ))
        })
        .collect();
    let m = laser.kind.manufacturing().unwrap();
    let members: Vec<Vec<NodeId>> = squares.iter().map(|id| vec![*id]).collect();
    d.add_operation(
        m,
        &NewOperation {
            kind: "engrave_outline",
            name: "Engrave + Outline 1",
        },
        0,
        &members,
    )
    .unwrap();
    (pack(&d, "engraved").unwrap(), laser.id)
}

#[test]
fn criterion_29_f_the_hatched_draw_of_a_thousand_items_with_chips() {
    let (bytes, laser) = engraved_file(1000);
    let cost = cheapest(
        || open_on(&bytes, &laser),
        |s| {
            let list = s.draw_list();
            assert_ne!(list.triangle_count(), 0);
        },
    );
    eprintln!("criterion 29 f: hatched draw, 1000 items with chips {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}
