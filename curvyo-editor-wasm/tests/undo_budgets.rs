//! The time budgets of undo and redo (`specs/0020-undo-redo` criterion 30; ADR 0014 §13), for a
//! release build on the host CPU: a small step (at most 2,000 operations) is taken back within
//! 100 ms; a larger step within 250 ms for 100 paths of 100 anchors and within 1 s for 5,000
//! objects; the first undo after Open, before the history was warmed, is a large step whatever its
//! size. Each test asserts four times the figure, as the other budget tests do (machine noise);
//! the measured time is printed. `#[ignore]`d, run on purpose:
//! `cargo test --release -p curvyo-editor-wasm --test undo_budgets -- --ignored --nocapture`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

use std::time::{Duration, Instant};

use curvyo_document_core::{
    AnchorId, Direction, Document, NewAnchor, Point, RectBounds, SMALL_STEP_OPS, Vec2, pack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};
use curvyo_ui_core::MoveEntryMode;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn open(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.resize_viewport(1200.0, 800.0);
    session.set_tool(Tool::Select);
    session
}

fn select_all(session: &mut Session) {
    session.pointer_down(pt(-100.0, -100.0), false);
    session.pointer_up(pt(100_000.0, 100_000.0), false, false);
}

fn typed_move(session: &mut Session) {
    session.key_down(KeyInput {
        key: "m",
        ..KeyInput::default()
    });
    session.commit_move_entry(
        "5",
        "5",
        MoveEntryMode {
            absolute: false,
            copy: false,
        },
    );
}

/// Undoes the newest step, draws the frame and returns the time of both.
fn timed_undo(session: &mut Session) -> Duration {
    let started = Instant::now();
    let result = session.undo();
    let _ = session.draw_list();
    let elapsed = started.elapsed();
    assert!(result.changed, "{}", result.notice);
    elapsed
}

fn within(name: &str, measured: Duration, budget_ms: u64) {
    eprintln!(
        "{name}: {measured:?} (budget {budget_ms} ms, asserted at {} ms)",
        budget_ms * 4
    );
    assert!(
        measured <= Duration::from_millis(budget_ms * 4),
        "{name} took {measured:?}, budget {budget_ms} ms (asserted at four times)"
    );
}

/// 5,000 objects (70 % rectangles, 30 % closed paths of 4 anchors) and 20,000 older steps.
fn big_document() -> Document {
    let document = Document::new(1);
    let mut ids = Vec::new();
    let mut anchor = 0_u64;
    for n in 0..5_000_u32 {
        let (x, y) = (f64::from(n % 100) * 12.0, f64::from(n / 100) * 12.0);
        if n % 10 < 7 {
            ids.push(
                document.create_rect(RectBounds::from_corners(pt(x, y), pt(x + 8.0, y + 8.0))),
            );
        } else {
            let anchors: Vec<NewAnchor> = [(0.0, 0.0), (8.0, 0.0), (8.0, 8.0), (0.0, 8.0)]
                .iter()
                .map(|(dx, dy)| {
                    anchor += 1;
                    NewAnchor::corner(AnchorId::new(1, anchor), pt(x + dx, y + dy))
                })
                .collect();
            ids.push(document.create_path(&anchors, true));
        }
    }
    for n in 0..20_000_usize {
        let id = ids[(n * 7919) % ids.len()];
        document
            .translate_objects(&[id], Vec2::new(1.0, 0.5))
            .unwrap();
    }
    document
}

#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn a_small_step_in_a_big_drawing_is_undone_within_100_ms_and_shows_no_busy_state() {
    let mut session = open(&big_document());
    session.warm_history();
    // Move three objects by a typed entry: a few hundred operations at most.
    session.pointer_down(pt(-1.0, -1.0), false);
    session.pointer_up(pt(30.0, 5.0), false, false);
    assert!(session.selected_object_count() > 0);
    typed_move(&mut session);
    assert!(!session.history_needs_busy_state(Direction::Undo));
    let measured = timed_undo(&mut session);
    within(
        "undo of a small step in a drawing of 5,000 objects and 25,000 steps",
        measured,
        100,
    );
    let started = Instant::now();
    session.redo();
    let _ = session.draw_list();
    within("redo of a small step", started.elapsed(), 100);
}

#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn a_hundred_paths_of_a_hundred_anchors_are_undone_within_250_ms_behind_the_busy_state() {
    let document = Document::new(1);
    let mut anchor = 0_u64;
    for p in 0..100_u32 {
        let anchors: Vec<NewAnchor> = (0..100_u32)
            .map(|n| {
                anchor += 1;
                let a = f64::from(n) / 100.0 * std::f64::consts::TAU;
                NewAnchor::corner(
                    AnchorId::new(1, anchor),
                    pt(
                        f64::from(p % 10) * 50.0 + 20.0 * a.cos(),
                        f64::from(p / 10) * 50.0 + 20.0 * a.sin(),
                    ),
                )
            })
            .collect();
        let _ = document.create_path(&anchors, true);
    }
    let mut session = open(&document);
    session.warm_history();
    select_all(&mut session);
    assert_eq!(session.selected_object_count(), 100);
    typed_move(&mut session);
    assert!(
        session.history_needs_busy_state(Direction::Undo),
        "a step of more than {SMALL_STEP_OPS} operations announces the busy state"
    );
    let measured = timed_undo(&mut session);
    within("undo of 100 paths of 100 anchors", measured, 250);
}

#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn five_thousand_objects_are_undone_within_one_second_behind_the_busy_state() {
    let mut session = open(&big_document());
    session.warm_history();
    select_all(&mut session);
    assert_eq!(session.selected_object_count(), 5_000);
    typed_move(&mut session);
    assert!(session.history_needs_busy_state(Direction::Undo));
    let measured = timed_undo(&mut session);
    within("undo of 5,000 objects", measured, 1_000);
}

#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn the_first_undo_after_open_is_a_large_step_until_the_history_was_warmed() {
    let mut session = open(&big_document());
    session.pointer_down(pt(-1.0, -1.0), false);
    session.pointer_up(pt(30.0, 5.0), false, false);
    typed_move(&mut session);
    assert!(
        session.history_needs_busy_state(Direction::Undo),
        "before the warm-up even a small step shows the busy state"
    );
    let measured = timed_undo(&mut session);
    within("the first undo after Open, cold", measured, 250);
    session.warm_history();
    session.redo();
    assert!(!session.history_needs_busy_state(Direction::Undo));
}
