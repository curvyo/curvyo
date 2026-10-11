//! Time budgets of the object history (`specs/0041-object-history` criteria 17 and 35; ADR 0014
//! §13): the Object scope opens within 100 ms for an object with 1,000 steps in a document with
//! 20,000, the first build of the per-object index takes up to 300 ms, a commit extends it within
//! 1 ms, and an object undo of a step 1,000 steps back stays within the budget of `0020`.
//! Release builds, `#[ignore]`d, four times each figure:
//! `cargo test --release -p curvyo-document-core --test object_history_budgets -- --ignored --nocapture`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::time::{Duration, Instant};

use curvyo_document_core::{Document, NodeId, Point, RectBounds, Vec2, pack, unpack};

fn within(name: &str, measured: Duration, budget_ms: u64) {
    eprintln!(
        "{name}: {measured:?} (budget {budget_ms} ms, asserted at {} ms)",
        budget_ms * 4
    );
    assert!(
        measured <= Duration::from_millis(budget_ms * 4),
        "{name}: {measured:?}"
    );
}

/// 200 rectangles, then `total` single-object moves of which every `every`-th is of the first.
fn drawing(total: usize, every: usize) -> (Document, NodeId) {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..200_u32)
        .map(|n| {
            let x = f64::from(n % 20) * 12.0;
            document.create_rect(RectBounds::from_corners(
                Point::new(x, f64::from(n / 20) * 12.0),
                Point::new(x + 8.0, f64::from(n / 20) * 12.0 + 8.0),
            ))
        })
        .collect();
    for n in 0..total {
        let id = if n % every == 0 {
            ids[0]
        } else {
            ids[1 + n % 199]
        };
        document
            .translate_objects(&[id], Vec2::new(0.01, 0.0))
            .unwrap();
    }
    (document, ids[0])
}

#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn the_object_scope_opens_within_a_hundred_milliseconds_in_a_big_log() {
    let (document, id) = drawing(20_000, 20);
    let reopened = unpack(9, &pack(&document, "t").unwrap()).unwrap();
    let started = Instant::now();
    let len = reopened.object_history_len(id);
    within("first build of the object index", started.elapsed(), 300);
    assert_eq!(len, 1_001);

    let started = Instant::now();
    let len = reopened.object_history_len(id);
    let rows = reopened.object_history_page(id, 0, 50);
    within("the Object scope, index built", started.elapsed(), 100);
    assert_eq!((len, rows.len()), (1_001, 50));

    reopened
        .translate_objects(&[id], Vec2::new(1.0, 0.0))
        .unwrap();
    let started = Instant::now();
    let len = reopened.object_history_len(id);
    within("one commit extends the index", started.elapsed(), 100);
    assert_eq!(len, 1_002);
}

#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn an_object_undo_of_a_step_a_thousand_steps_back_stays_within_the_budget() {
    let (document, id) = drawing(1_001, 1_001);
    // The object's last step lies 1,000 steps behind the latest version.
    let reopened = unpack(9, &pack(&document, "t").unwrap()).unwrap();
    reopened.warm_history();
    let started = Instant::now();
    let done = reopened.undo_object_step(id).unwrap().unwrap();
    within(
        "object undo, a step 1,000 steps back",
        started.elapsed(),
        100,
    );
    assert!(done.report.wrote);
}
