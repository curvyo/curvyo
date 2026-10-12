//! The time budget of the History list with branches (`specs/0042-history-branches` criterion 8):
//! a list with 50 branches and 10,000 steps opens within 200 ms. Release build, `#[ignore]`d, four
//! times the figure:
//! `cargo test --release -p curvyo-editor-wasm --test history_branch_budgets -- --ignored --nocapture`.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::time::{Duration, Instant};

use curvyo_document_core::{Document, Length, Point, RectBounds, StyleEdit, pack};
use curvyo_editor_wasm::Session;

#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn fifty_branches_in_ten_thousand_steps_open_within_200_ms() {
    let d = Document::new(1);
    let r = d.create_rect(RectBounds {
        origin: Point::new(10.0, 10.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    });
    let mut width = 1.0;
    let mut step = |d: &Document| {
        width += 0.001;
        d.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(width)))
            .unwrap();
        d.own_steps().last().unwrap().step
    };
    for n in 0..10_000 {
        let s = step(&d);
        // Every 200th step is taken back and left by the next one: 50 branches.
        if n % 200 == 100 {
            d.undo_step(s).unwrap();
        }
    }
    let bytes = pack(&d, "t").unwrap();
    let mut session = Session::open(2, &bytes).unwrap();
    let started = Instant::now();
    let shape = session.history_layout_json(false);
    let items = session.history_items_json(false, 0, 50);
    let measured = started.elapsed();
    eprintln!("open the list: {measured:?} (budget 200 ms, asserted at 800 ms)");
    let shape: serde_json::Value = serde_json::from_str(&shape).unwrap();
    let special = shape["special"].as_array().unwrap();
    assert_eq!(
        special.iter().filter(|s| s[1] == "header").count(),
        3,
        "three headers, the rest held by the more row"
    );
    assert_eq!(special.iter().filter(|s| s[1] == "more").count(), 1);
    assert!(items.starts_with('['));
    assert!(measured <= Duration::from_millis(800), "{measured:?}");
}
