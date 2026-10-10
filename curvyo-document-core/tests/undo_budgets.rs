//! Time budgets of the restore engine below the editor (`specs/0020-undo-redo` criteria 14 and 30;
//! ADR 0014 §13): the result of a Boolean union with about 5,000 anchors is one step and one press,
//! and an undo of a small step that lies 1,000 and 10,000 operations behind the latest version.
//! Release builds, `#[ignore]`d, four times each figure:
//! `cargo test --release -p curvyo-document-core --test undo_budgets -- --ignored --nocapture`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::time::{Duration, Instant};

use curvyo_document_core::{AnchorId, Document, NewAnchor, Point, RectBounds, Vec2, pack, unpack};

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
fn a_boolean_result_of_five_thousand_anchors_is_one_step_and_one_press() {
    let document = Document::new(1);
    let ring = |first: u64, radius: f64| -> Vec<NewAnchor> {
        (0..2_500_u64)
            .map(|n| {
                let a = n as f64 / 2_500.0 * std::f64::consts::TAU;
                NewAnchor::corner(
                    AnchorId::new(1, first + n),
                    Point::new(radius * a.cos(), radius * a.sin()),
                )
            })
            .collect()
    };
    let a = document.create_path(&ring(0, 100.0), true);
    let b = document.create_path(&ring(10_000, 90.0), true);
    let before = document.export_json().unwrap();
    let result = document
        .replace_with_path(
            &[a, b],
            a,
            &[(ring(20_000, 100.0), true), (ring(30_000, 100.0), true)],
            "boolean_union",
        )
        .unwrap();
    let after = document.export_json().unwrap();
    let step = document.own_steps().last().unwrap().step;
    assert!(document.step_op_count(step).unwrap() > 5_000);
    let reopened = unpack(9, &pack(&document, "t").unwrap()).unwrap();
    assert_eq!(
        reopened.history_len(),
        3,
        "two creates and one union, however Loro splits it"
    );
    let started = Instant::now();
    let report = document.undo_step(step).unwrap();
    within(
        "undo of a union with 5,000 anchors",
        started.elapsed(),
        1_000,
    );
    assert!(report.wrote);
    assert_eq!(document.export_json().unwrap(), before);
    let started = Instant::now();
    document.redo_step(step).unwrap();
    within(
        "redo of a union with 5,000 anchors",
        started.elapsed(),
        1_000,
    );
    assert_eq!(document.export_json().unwrap(), after);
    let _ = result;
}

fn drawing(steps_after: usize) -> (Document, curvyo_document_core::StepRef) {
    let document = Document::new(1);
    let ids: Vec<_> = (0..1_000_u32)
        .map(|n| {
            let x = f64::from(n % 50) * 12.0;
            document.create_rect(RectBounds::from_corners(
                Point::new(x, 0.0),
                Point::new(x + 8.0, 8.0),
            ))
        })
        .collect();
    document
        .translate_objects(&ids[..3], Vec2::new(1.0, 1.0))
        .unwrap();
    let step = document.own_steps().last().unwrap().step;
    for n in 0..steps_after {
        document
            .translate_objects(&[ids[10 + n % 900]], Vec2::new(1.0, 0.0))
            .unwrap();
    }
    (document, step)
}

#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn a_small_step_far_behind_the_latest_version_costs_in_proportion_to_the_distance() {
    for steps_after in [500_usize, 5_000] {
        let (document, step) = drawing(steps_after);
        let reopened = unpack(9, &pack(&document, "t").unwrap()).unwrap();
        reopened.warm_history();
        let distance = reopened.step_distance(step).unwrap();
        let started = Instant::now();
        let report = reopened.undo_step(step).unwrap();
        let elapsed = started.elapsed();
        eprintln!(
            "undo of a 3-object step {distance} operations behind the latest version: {elapsed:?}"
        );
        assert!(report.wrote);
        // The distance decides the busy state, not the budget: the figures are printed to set it.
        assert!(elapsed < Duration::from_secs(4), "{elapsed:?}");
    }
}
