//! Combine, Split and Break apart on open, touching and crossing outlines at the size of the
//! budgets (`specs/0054-combine-open-paths` criteria 32 and 33): four generated inputs, each
//! command through the session in under 2 s with the golden counts, and the same result on a
//! second run. Budgets are asserted in release builds only; a debug build runs a tenth of the size
//! and prints the numbers.
//! Run: `cargo test --release -p curvyo-editor-wasm --test open_outlines_interactivity -- --nocapture`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]

use std::time::{Duration, Instant};

use curvyo_document_core::{AnchorId, Document, NewAnchor, ObjectSnapshot, Point, pack};
use curvyo_editor_wasm::{BreakApartOutcome, CombineOutcome, Session, SplitOutcome, Tool};
use curvyo_ui_core::SplitRefusal;

const SCALE: usize = if cfg!(debug_assertions) { 10 } else { 1 };
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

fn select_all(session: &mut Session, to: Point) {
    let from = pt(-200.0, -200.0);
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(to, false, false);
    session.pointer_up(to, false, false);
}

fn check(label: &str, took: Duration) {
    println!("{label}: {took:?} (budget {BUDGET:?})");
    if !cfg!(debug_assertions) {
        assert!(took <= BUDGET, "{label}: {took:?} is over {BUDGET:?}");
    }
}

/// Anchor ids from one counter, so no two paths share one.
struct Ids(u64);

impl Ids {
    fn next(&mut self) -> AnchorId {
        self.0 += 1;
        AnchorId::new(1, self.0)
    }
}

fn path(document: &Document, ids: &mut Ids, points: &[(f64, f64)], closed: bool) {
    let anchors: Vec<NewAnchor> = points
        .iter()
        .map(|&(x, y)| NewAnchor::corner(ids.next(), pt(x, y)))
        .collect();
    let _ = document.create_path(&anchors, closed);
}

fn square(document: &Document, ids: &mut Ids, x: f64, y: f64, side: f64) {
    path(
        document,
        ids,
        &[(x, y), (x + side, y), (x + side, y + side), (x, y + side)],
        true,
    );
}

fn unpacked(session: &Session) -> Document {
    curvyo_document_core::unpack(3, &session.pack("0.1.0").unwrap()).unwrap()
}

/// The outlines of the only object, as (node count, closed).
fn only_outlines(session: &Session) -> Vec<(usize, bool)> {
    let document = unpacked(session);
    let ids = document.object_ids();
    assert_eq!(ids.len(), 1);
    let Some(ObjectSnapshot::Path(path)) = document.object(ids[0]) else {
        panic!("a path");
    };
    path.subpaths()
        .map(|subpath| (subpath.anchors.len(), subpath.closed))
        .collect()
}

/// (a) A closed rectangle with `count` open lines of 2 nodes inside it, none touching.
fn session_a(count: usize) -> Session {
    let document = Document::new(1);
    let mut ids = Ids(0);
    square(&document, &mut ids, 0.0, 0.0, 500.0);
    for i in 0..count {
        let (x, y) = (10.0 + (i % 40) as f64 * 12.0, 10.0 + (i / 40) as f64 * 9.0);
        path(&document, &mut ids, &[(x, y), (x + 8.0, y)], false);
    }
    let mut session = session_of(&document);
    select_all(&mut session, pt(700.0, 700.0));
    assert_eq!(session.selected_object_count(), count + 1);
    session
}

/// (b) `half` horizontal and `half` vertical open lines of 2 nodes crossing in a grid.
fn session_b(half: usize) -> Session {
    let document = Document::new(1);
    let mut ids = Ids(0);
    for i in 0..half {
        let at = i as f64 * 0.5;
        path(&document, &mut ids, &[(0.0, at), (500.0, at)], false);
        path(&document, &mut ids, &[(at, 0.0), (at, 500.0)], false);
    }
    let mut session = session_of(&document);
    select_all(&mut session, pt(700.0, 700.0));
    assert_eq!(session.selected_object_count(), 2 * half);
    session
}

/// (d) A chain of `count` closed squares, each crossing the next.
fn session_d(count: usize) -> Session {
    let document = Document::new(1);
    let mut ids = Ids(0);
    for i in 0..count {
        let (x, y) = ((i % 100) as f64 * 5.0, (i / 100) as f64 * 5.0);
        square(&document, &mut ids, x, y, 10.0);
    }
    let mut session = session_of(&document);
    select_all(&mut session, pt(700.0, 700.0));
    assert_eq!(session.selected_object_count(), count);
    session
}

/// (a) Combine gives 1 compound path of 2,001 outlines.
#[test]
fn a_rectangle_with_two_thousand_lines_combines() {
    let count = 2000 / SCALE;
    let mut session = session_a(count);
    let started = Instant::now();
    let outcome = session.apply_combine();
    check("combine (a)", started.elapsed());
    assert!(matches!(
        outcome,
        CombineOutcome::Applied { holes: 0, open, touching: 0, .. } if open == count
    ));
    let outlines = only_outlines(&session);
    assert_eq!(outlines.len(), count + 1);
    assert_eq!(outlines.iter().filter(|(_, closed)| !closed).count(), count);
}

/// (b) Combine gives 1 compound path of 2,000 outlines; open outlines are not counted as
/// touching.
#[test]
fn a_crossing_grid_of_two_thousand_lines_combines() {
    let half = 1000 / SCALE;
    let mut session = session_b(half);
    let started = Instant::now();
    let outcome = session.apply_combine();
    check("combine (b)", started.elapsed());
    assert!(matches!(
        outcome,
        CombineOutcome::Applied { touching: 0, open, .. } if open == 2 * half
    ));
    assert_eq!(only_outlines(&session).len(), 2 * half);
}

/// (c) Split of the compound path of (a) is refused as one piece; Break apart gives 1 closed path
/// and 2,000 open paths.
#[test]
fn the_compound_path_of_a_rectangle_and_lines_splits_and_breaks_apart() {
    let count = 2000 / SCALE;
    let mut session = session_a(count);
    let _ = session.apply_combine();

    let started = Instant::now();
    let outcome = session.apply_split();
    check("split (c)", started.elapsed());
    assert_eq!(
        outcome,
        SplitOutcome::Refused(SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: false
        })
    );

    let started = Instant::now();
    let outcome = session.apply_break_apart();
    check("break apart (c)", started.elapsed());
    assert!(matches!(
        outcome,
        BreakApartOutcome::Applied { pieces, .. } if pieces == count + 1
    ));
    let document = unpacked(&session);
    let closed: Vec<bool> = document
        .object_ids()
        .into_iter()
        .filter_map(|id| match document.object(id) {
            Some(ObjectSnapshot::Path(path)) => Some(path.closed),
            _ => None,
        })
        .collect();
    assert_eq!(closed.iter().filter(|&&c| c).count(), 1);
    assert_eq!(closed.iter().filter(|&&c| !c).count(), count);
}

/// (d) Combine gives 1 compound path of 1,000 outlines, no hole, all counted as touching.
#[test]
fn a_chain_of_a_thousand_crossing_squares_combines() {
    let count = 1000 / SCALE;
    let mut session = session_d(count);
    let started = Instant::now();
    let outcome = session.apply_combine();
    check("combine (d)", started.elapsed());
    assert!(matches!(
        outcome,
        CombineOutcome::Applied { holes: 0, touching, .. } if touching == count
    ));
    assert_eq!(only_outlines(&session).len(), count);
}

/// Criterion 33: the same objects give the same result node for node.
#[test]
fn combine_and_split_are_deterministic() {
    let run = || {
        let mut session = session_a(200);
        let _ = session.apply_combine();
        let _ = session.apply_break_apart();
        session.pack("0.1.0").map(|_| ()).unwrap();
        let document = unpacked(&session);
        document
            .object_ids()
            .into_iter()
            .filter_map(|id| document.path(id))
            .map(|path| {
                path.subpaths()
                    .flat_map(|s| s.anchors.iter().map(|a| (a.point.x, a.point.y)))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}
