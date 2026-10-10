//! The memory budget of `specs/0020-undo-redo` criterion 29: the undo data of 500 steps stays
//! under 50 MB. The stacks keep step references, labels and the selection around each step, not
//! inverses (ADR 0014 consequences), so the data of 500 steps of any kind, a Boolean union of two
//! 2,000-anchor paths included, is a few hundred kilobytes. A run of steps on one large selection
//! holds that selection once. The test sizes the stack after 500 steps on a selection of 4,000
//! objects, after 500 steps on a different selection of 4,000 objects each time (the worst case),
//! and after 500 unions of two objects.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{Document, NodeId, Point, RectBounds, StepRef};
use curvyo_ui_core::{SelectionSnapshot, UndoStack};

const BUDGET_BYTES: usize = 50 * 1024 * 1024;

fn objects(count: u32) -> Vec<NodeId> {
    let document = Document::new(1);
    (0..count)
        .map(|n| {
            let x = f64::from(n);
            document.create_rect(RectBounds::from_corners(
                Point::new(x, 0.0),
                Point::new(x + 1.0, 1.0),
            ))
        })
        .collect()
}

fn selection(ids: &[NodeId]) -> SelectionSnapshot {
    SelectionSnapshot {
        objects: ids.to_vec(),
        nodes: Vec::new(),
    }
}

#[test]
fn a_run_of_steps_on_one_large_selection_holds_the_selection_once() {
    let ids = objects(4_000);
    let mut stack = UndoStack::new();
    for n in 0..500_i32 {
        stack.record(
            StepRef::new(1, n),
            "translate_objects",
            selection(&ids),
            selection(&ids),
        );
    }
    assert_eq!(stack.undo_len(), 500);
    eprintln!(
        "500 steps on one selection hold {} KiB",
        stack.approx_bytes() / 1024
    );
    assert!(stack.approx_bytes() < BUDGET_BYTES);
}

#[test]
fn five_hundred_unions_hold_a_few_hundred_kilobytes() {
    let ids = objects(2);
    let mut stack = UndoStack::new();
    for n in 0..500_i32 {
        stack.record(
            StepRef::new(1, n),
            "boolean_union",
            selection(&ids),
            selection(&ids[..1]),
        );
    }
    eprintln!("500 unions hold {} KiB", stack.approx_bytes() / 1024);
    assert!(stack.approx_bytes() < 1024 * 1024);
}

#[test]
fn even_five_hundred_different_large_selections_stay_under_the_budget() {
    let ids = objects(4_000);
    let mut stack = UndoStack::new();
    for n in 0..500_i32 {
        let end = 2_000 + usize::try_from(n).unwrap() * 4;
        stack.record(
            StepRef::new(1, n),
            "translate_objects",
            selection(&ids[..end]),
            selection(&ids[..end]),
        );
    }
    eprintln!(
        "500 different selections hold {} KiB",
        stack.approx_bytes() / 1024
    );
    assert!(stack.approx_bytes() < BUDGET_BYTES);
}
