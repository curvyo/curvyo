//! Independent budget tests for `specs/0111-laser-operation-tools` criterion 26, thread CPU time
//! (never wall time, CLAUDE.md §5): a kind press assigning 1,000 items, a press assigning a group
//! of 1,000 leaves, entering a group of 1,000 leaves, and unfolding a row on a page with 50
//! operations, each within 16 ms. Assertions only run in a release build; a debug run executes
//! the scenarios.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    missing_docs
)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{
    Document, DocumentSize, NewOperation, NewOutputPage, NodeId, Page, Point, RectBounds, pack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};

const SIXTEEN_MS: Duration = Duration::from_millis(16);
const KINDS: [&str; 5] = ["cut", "score", "engrave", "engrave_outline", "cut_out"];

/// A laser page with 1,000 rectangles (grouped into one group when `grouped`) and 50 operations
/// cycling the five kinds, none holding anything.
fn scene(grouped: bool) -> (Session, Page, Vec<NodeId>, Vec<NodeId>, Option<NodeId>) {
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
    let group = grouped.then(|| d.group_objects(&ids).unwrap());
    let m = made.kind.manufacturing().unwrap();
    let ops = (0..50)
        .map(|i| {
            let kind = KINDS[i % 5];
            let name = format!("{kind} {}", i + 1);
            d.add_operation(m, &NewOperation { kind, name: &name }, i, &[])
                .unwrap()
        })
        .collect();
    let mut s = Session::open(7, &pack(&d, "0111-budget").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.activate_page(&l.id);
    s.wipe_history();
    (s, l, ids, ops, group)
}

fn select_all(s: &mut Session) {
    s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}

fn check(name: &str, took: Duration) {
    println!("criterion 26 {name}: {took:?}");
    if !cfg!(debug_assertions) {
        assert!(took <= SIXTEEN_MS, "{name}: {took:?} > 16 ms");
    }
}

#[test]
fn c26_a_kind_press_reusing_an_operation_for_1000_items() {
    let (mut s, _l, _ids, _ops, _) = scene(false);
    select_all(&mut s);
    assert_eq!(s.selected_object_count(), 1000);
    let (id, took) = cpu_time_of(|| s.press_operation_kind("engrave", false));
    assert!(id.is_some());
    check("kind press over 1000 items", took);
}

#[test]
fn c26_a_press_assigning_a_group_of_1000_leaves() {
    let (mut s, l, _ids, _ops, _group) = scene(true);
    let at = Point::new(l.origin.x + 5.0 + 6.0, l.origin.y + 5.0);
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    assert!(
        s.selected_object_count() > 0,
        "the group (counted by its leaves)"
    );
    let (id, took) = cpu_time_of(|| s.press_operation_kind("cut", false));
    assert!(id.is_some());
    check("group press over 1000 leaves", took);
}

#[test]
fn c26_entering_a_group_of_1000_leaves() {
    let (mut s, l, _ids, _ops, _) = scene(true);
    let at = Point::new(l.origin.x + 5.0 + 6.0, l.origin.y + 5.0);
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    let (_, took) = cpu_time_of(|| {
        s.double_click(at, false, false);
        s.operations_view_json()
    });
    let v: serde_json::Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    assert!(!v["entered_group"].is_null(), "entered");
    check("entering a group of 1000 leaves", took);
}

#[test]
fn c26_unfolding_a_row_among_50_operations() {
    let (mut s, _l, _ids, ops, _) = scene(false);
    let (ok, took) = cpu_time_of(|| {
        let ok = s.unfold_operation_row(ops[25]);
        let _ = s.operations_view_json();
        ok
    });
    assert!(ok);
    check("unfolding a row among 50", took);
}
