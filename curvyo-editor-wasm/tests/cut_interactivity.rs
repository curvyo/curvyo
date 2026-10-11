//! Cut at the size of the budget (`specs/0036-cut-at-crossings` criterion 13): a grid of 50
//! horizontal and 50 vertical lines (2,500 crossings) gives 5,100 pieces in under 2 s, and a
//! field of overlapping circles (curves, not lines) stays within the same budget. The budgets are
//! asserted in release builds only; a debug build prints the numbers.
//! Run: `cargo test --release -p curvyo-editor-wasm --test cut_interactivity -- --nocapture`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]

use std::time::{Duration, Instant};

use curvyo_document_core::{AnchorId, AnchorKind, Document, NewAnchor, Point, Vec2, pack};
use curvyo_editor_wasm::{CutOutcome, Session, Tool};

const BUDGET: Duration = Duration::from_secs(2);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn session_of(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1200.0, 800.0);
    session
}

fn select_all(session: &mut Session) {
    let from = pt(-200.0, -200.0);
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(pt(900.0, 900.0), false, false);
    session.pointer_up(pt(900.0, 900.0), false, false);
}

fn check(label: &str, took: Duration) {
    println!("{label}: {took:?} (budget {BUDGET:?})");
    if !cfg!(debug_assertions) {
        assert!(took <= BUDGET, "{label}: {took:?} is over {BUDGET:?}");
    }
}

struct Ids(u64);

impl Ids {
    fn next(&mut self) -> AnchorId {
        self.0 += 1;
        AnchorId::new(1, self.0)
    }
}

fn line(document: &Document, ids: &mut Ids, a: (f64, f64), b: (f64, f64)) {
    let anchors = [a, b].map(|(x, y)| NewAnchor::corner(ids.next(), pt(x, y)));
    let _ = document.create_path(&anchors, false);
}

fn circle(document: &Document, ids: &mut Ids, cx: f64, cy: f64, r: f64) {
    let k = r * 0.552_284_749_8;
    let node = |ids: &mut Ids, x: f64, y: f64, hin: Vec2, hout: Vec2| NewAnchor {
        id: ids.next(),
        point: pt(x, y),
        handle_in: hin,
        handle_out: hout,
        kind: AnchorKind::Symmetric,
    };
    let anchors = [
        node(ids, cx + r, cy, Vec2::new(0.0, -k), Vec2::new(0.0, k)),
        node(ids, cx, cy + r, Vec2::new(k, 0.0), Vec2::new(-k, 0.0)),
        node(ids, cx - r, cy, Vec2::new(0.0, k), Vec2::new(0.0, -k)),
        node(ids, cx, cy - r, Vec2::new(-k, 0.0), Vec2::new(k, 0.0)),
    ];
    let _ = document.create_path(&anchors, true);
}

/// 50 horizontal and 50 vertical lines: 2,500 crossings, 100 paths became 5,100 pieces.
#[test]
fn a_grid_of_a_hundred_lines() {
    let document = Document::new(1);
    let mut ids = Ids(0);
    for k in 0..50 {
        let y = f64::from(k) * 10.0 + 5.0;
        line(&document, &mut ids, (0.0, y), (500.0, y));
        let x = f64::from(k) * 10.0 + 5.0;
        line(&document, &mut ids, (x, 0.0), (x, 500.0));
    }
    let mut session = session_of(&document);
    select_all(&mut session);
    assert_eq!(session.selected_object_count(), 100);

    let started = Instant::now();
    let outcome = session.apply_cut();
    check("cut (grid)", started.elapsed());
    assert_eq!(
        outcome,
        CutOutcome::Applied {
            paths: 100,
            pieces: 5100
        }
    );
    assert_eq!(session.selected_object_count(), 5100);
}

/// 400 circles of radius 6 on a grid of 8: every circle crosses its eight neighbours twice (the
/// diagonal centres are 11.3 apart): 1,482 pairs, 2,964 crossings, 5,928 pieces.
#[test]
fn a_field_of_overlapping_circles() {
    let document = Document::new(1);
    let mut ids = Ids(0);
    for i in 0..20 {
        for j in 0..20 {
            circle(
                &document,
                &mut ids,
                f64::from(i) * 8.0 + 20.0,
                f64::from(j) * 8.0 + 20.0,
                6.0,
            );
        }
    }
    let mut session = session_of(&document);
    select_all(&mut session);
    assert_eq!(session.selected_object_count(), 400);

    let started = Instant::now();
    let outcome = session.apply_cut();
    check("cut (circles)", started.elapsed());
    // Each of the 2 * 20 * 19 straight and 2 * 19 * 19 diagonal neighbour pairs crosses twice.
    let crossings = (2 * 20 * 19 + 2 * 19 * 19) * 2;
    let CutOutcome::Applied { paths, pieces } = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(paths, 400);
    // Every crossing cuts two circles once: the cuts add up to 2 * crossings, and a closed
    // circle with m cuts makes m pieces.
    assert_eq!(pieces, 2 * crossings);
}
