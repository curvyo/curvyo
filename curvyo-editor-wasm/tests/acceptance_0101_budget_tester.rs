//! Independent budget tests for `specs/0101-laser-operations` criterion 29, thread CPU time in a
//! release build (never wall time, CLAUDE.md §5). Budgets (a) one press assigning 1,000 items
//! within 16 ms, (b) the summary after a change on 1,000 items and 50 operations within 16 ms,
//! (c) the Operations tab with 50 operations and 1,000 items within 100 ms. Budget (d), the drag
//! of an operation among 50, has no public session entry point to drive and is reported as a
//! missing seam. Assertions only run in a release build; a debug run executes the scenarios.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{
    Document, DocumentSize, NewOutputPage, NodeId, Page, Point, RectBounds, pack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};

use cpu_time::cpu_time_of;

const SIXTEEN_MS: Duration = Duration::from_millis(16);
const HUNDRED_MS: Duration = Duration::from_millis(100);

/// A laser page with 1,000 rectangles in a 25 x 40 grid.
fn scene() -> (Session, Page, Vec<NodeId>) {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(800.0, 800.0),
        })
        .unwrap();
    let l = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let mut ids = Vec::with_capacity(1000);
    for row in 0..40 {
        for col in 0..25 {
            let x = l.origin.x + 5.0 + f64::from(col) * 30.0;
            let y = l.origin.y + 5.0 + f64::from(row) * 19.0;
            ids.push(d.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 20.0, y + 10.0),
            )));
        }
    }
    let mut s = Session::open(7, &pack(&d, "0101-budget").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.activate_page(&l.id);
    s.wipe_history();
    (s, l, ids)
}

/// Fifty operations, cycling the five kinds, with no selection.
fn fifty_operations(s: &mut Session) -> Vec<NodeId> {
    let kinds = ["cut", "score", "engrave", "engrave_outline", "cut_out"];
    (0..50)
        .map(|i| s.press_operation_kind(kinds[i % 5]).unwrap())
        .collect()
}

fn select_all(s: &mut Session) {
    s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}

#[test]
fn c29a_one_press_assigning_a_selection_of_1000_items() {
    let (mut s, _l, _ids) = scene();
    let ops = fifty_operations(&mut s);
    select_all(&mut s);
    assert_eq!(
        s.selected_object_count(),
        1000,
        "the scene selects every rectangle"
    );

    let (ok, took) = cpu_time_of(|| s.set_selection_in_operation(ops[0], true));
    assert!(ok, "the tick assigns the selection");
    println!("criterion 29 (a) press over 1000 items: {took:?}");
    if !cfg!(debug_assertions) {
        assert!(took <= SIXTEEN_MS, "budget (a): {took:?} > 16 ms");
    }
}

#[test]
fn c29a_a_kind_press_on_a_1000_item_selection_creates_and_assigns() {
    let (mut s, _l, _ids) = scene();
    fifty_operations(&mut s);
    select_all(&mut s);
    let (id, took) = cpu_time_of(|| s.press_operation_kind("engrave"));
    assert!(id.is_some());
    println!("criterion 29 (a) kind press over 1000 items: {took:?}");
    if !cfg!(debug_assertions) {
        assert!(took <= SIXTEEN_MS, "budget (a): {took:?} > 16 ms");
    }
}

#[test]
fn c29b_the_summary_after_a_change_on_1000_items_and_50_operations() {
    let (mut s, _l, _ids) = scene();
    let ops = fifty_operations(&mut s);
    select_all(&mut s);
    s.set_selection_in_operation(ops[0], true);
    let (_, took) = cpu_time_of(|| {
        s.set_operation_switch(ops[1], curvyo_document_core::OperationSwitch::Output, false);
        s.operations_view_json()
    });
    println!("criterion 29 (b) change and summary: {took:?}");
    if !cfg!(debug_assertions) {
        assert!(took <= SIXTEEN_MS, "budget (b): {took:?} > 16 ms");
    }
}

#[test]
fn c29c_the_operations_tab_with_50_operations_and_1000_items() {
    let (mut s, _l, _ids) = scene();
    let ops = fifty_operations(&mut s);
    select_all(&mut s);
    s.set_selection_in_operation(ops[0], true);
    s.key_down(KeyInput {
        key: "Escape",
        ..KeyInput::default()
    });
    let (json, took) = cpu_time_of(|| s.operations_view_json());
    assert!(json.contains("rows"));
    println!("criterion 29 (c) tab with 50 operations and 1000 items: {took:?}");
    if !cfg!(debug_assertions) {
        assert!(took <= HUNDRED_MS, "budget (c): {took:?} > 100 ms");
    }
}
