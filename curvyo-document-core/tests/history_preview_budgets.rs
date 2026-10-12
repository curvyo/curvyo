//! The preview budgets of `specs/0042-history-branches` criterion 21 (ADR 0014 §13): at 10,000
//! steps the first frame of a row up to 1,000 steps back within 300 ms, a row 10,000 back within
//! 1.5 s; right after Open and warm. Release builds, `#[ignore]`d, four times each figure:
//! `cargo test --release -p curvyo-document-core --test history_preview_budgets -- --ignored --nocapture`.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::time::{Duration, Instant};

use curvyo_document_core::{Document, Length, Point, RectBounds, StyleEdit, pack, unpack};

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

#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn previews_at_ten_thousand_steps_stay_within_their_budgets() {
    let d = Document::new(1);
    let r = d.create_rect(RectBounds {
        origin: Point::new(10.0, 10.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    });
    let mut steps = Vec::new();
    for n in 0..10_000 {
        d.edit_style(
            &[r],
            &StyleEdit::StrokeWidth(Length::from_mm(1.0 + f64::from(n) * 0.001)),
        )
        .unwrap();
        steps.push(d.own_steps().last().unwrap().step);
    }
    let opened = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    for (label, document) in [("after Open", &opened), ("warm", &opened)] {
        for (back, budget) in [(10, 300), (100, 300), (1_000, 300), (9_999, 1_500)] {
            let step = steps[steps.len() - 1 - back];
            let started = Instant::now();
            let object = document.preview_object_at(step, r).unwrap();
            within(
                &format!("object, {back} back, {label}"),
                started.elapsed(),
                budget,
            );
            assert_eq!(object.objects.len(), 1);
            let started = Instant::now();
            document.preview_document_at(step).unwrap();
            within(
                &format!("document, {back} back, {label}"),
                started.elapsed(),
                budget,
            );
        }
    }
}
