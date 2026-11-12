//! `specs/0111-laser-operation-tools` criterion 26 in thread CPU time (`adrs.md` "Goldens and
//! budgets"): a kind press assigning 1,000 items, a kind press assigning a group of 1,000 leaves,
//! entering a group of 1,000 leaves, and unfolding a row on a page with 50 operations (the press
//! and the tab's JSON the host reads after it), each within 16 ms. Budgets are asserted in release
//! builds (the `performance-budgets` job); a debug build prints the times and checks the rest.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{
    Document, DocumentSize, NewOperation, NewOutputPage, NodeId, Point, RectBounds, pack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};

const BUDGET: Duration = Duration::from_millis(16);

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

/// A laser page of 400 x 300 mm with 1,000 squares, grouped into one group when `grouped`, and
/// `operations` empty Cut operations. Returns the file and the laser page id.
fn page_file(grouped: bool, operations: usize) -> (Vec<u8>, String) {
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
    let squares: Vec<NodeId> = (0..1000_u32)
        .map(|i| {
            let (col, row) = (f64::from(i % 40), f64::from(i / 40));
            let x = origin.x + 2.0 + col * 9.0;
            let y = origin.y + 2.0 + row * 9.0;
            d.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 6.0, y + 6.0),
            ))
        })
        .collect();
    if grouped {
        d.group_objects(&squares).unwrap();
    }
    let m = laser.kind.manufacturing().unwrap();
    for (k, square) in squares.iter().take(operations).enumerate() {
        let index = d.operations(m).unwrap().len();
        d.add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: &format!("Cut {}", k + 1),
            },
            index,
            &[vec![*square]],
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

/// The page's 1,000 squares selected (the group's unit when they are grouped).
fn select_squares(s: &mut Session) {
    assert!(s.select_operation_items("not_assigned"));
    assert_eq!(s.selected_object_count(), 1000);
}

#[test]
fn criterion_26_a_kind_press_assigning_a_thousand_items() {
    let (bytes, laser) = page_file(false, 0);
    let cost = cheapest(
        || {
            let mut s = open_on(&bytes, &laser);
            select_squares(&mut s);
            let _ = s.operations_view_json();
            s
        },
        |s| assert!(s.press_operation_kind("engrave", false).is_some()),
    );
    eprintln!("criterion 26: kind press on 1000 items {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_26_a_kind_press_assigning_a_group_of_a_thousand_leaves() {
    let (bytes, laser) = page_file(true, 0);
    let cost = cheapest(
        || {
            let mut s = open_on(&bytes, &laser);
            select_squares(&mut s);
            let _ = s.operations_view_json();
            s
        },
        |s| assert!(s.press_operation_kind("cut", false).is_some()),
    );
    eprintln!("criterion 26: kind press on a group of 1000 leaves {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_26_entering_a_group_of_a_thousand_leaves() {
    let (bytes, laser) = page_file(true, 0);
    let cost = cheapest(
        || {
            let mut s = open_on(&bytes, &laser);
            select_squares(&mut s);
            s
        },
        |s| {
            s.key_down(KeyInput {
                key: "Enter",
                ..KeyInput::default()
            });
            assert_eq!(s.context_depth(), 1);
        },
    );
    eprintln!("criterion 26: entering a group of 1000 leaves {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_26_unfolding_a_row_on_a_page_with_fifty_operations() {
    let (bytes, laser) = page_file(false, 50);
    let cost = cheapest(
        || {
            let s = open_on(&bytes, &laser);
            let id = s.operations_view().unwrap().rows[25].id;
            (s, id)
        },
        |(s, id)| {
            assert!(s.unfold_operation_row(*id));
            assert!(s.operations_view_json().contains("\"unfolded\":true"));
        },
    );
    eprintln!("criterion 26: unfolding a row among 50 operations {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}
