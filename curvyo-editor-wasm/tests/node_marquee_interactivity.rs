//! The node marquee at the size of its budgets (`specs/0053-node-marquee-selection/` criterion
//! 25, `adrs.md` decision 8): 10 paths of 1,000 nodes, the box over half of them.
//! - `anchors_in_box` under 1 ms;
//! - `NodeSelection::apply` of 5,000 nodes to a selection of 5,000 under 5 ms;
//! - the marquee frame of `Session::draw_list` within 16.7 ms of the thread's CPU time, the
//!   cheapest of 41 frames (the ratio to the plain Node-tool frame is printed; see the test);
//! - from the release, or Ctrl+A, to the finished draw list of the new selection at most 100 ms.
//!
//! Budgets are asserted in release builds only; a debug build runs a tenth of the size and
//! prints the numbers.
//! Run: `cargo test --release -p curvyo-editor-wasm --test node_marquee_interactivity -- --nocapture`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]

use std::time::{Duration, Instant};

use curvyo_document_core::{AnchorId, Document, NewAnchor, Point, Tolerance, pack};
use curvyo_editor_wasm::{KeyInput, Session, Tool};
use curvyo_ui_core::{NodeSelection, SelectionCombine, anchors_in_box, editing_set_anchors};

const PATHS: u64 = 10;
const NODES: u64 = if cfg!(debug_assertions) { 100 } else { 1_000 };
const RUNS: usize = 15;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// Path `p` is a horizontal line at y = 10 p with one node per millimetre from x = 0.
fn document() -> Document {
    let document = Document::new(1);
    for p in 0..PATHS {
        let anchors: Vec<NewAnchor> = (0..NODES)
            .map(|i| NewAnchor::corner(AnchorId::new(p + 10, i), pt(i as f64, 10.0 * p as f64)))
            .collect();
        let _ = document.create_path(&anchors, false);
    }
    document
}

fn session() -> Session {
    let mut session = Session::open(2, &pack(&document(), "0.1.0").unwrap()).unwrap();
    session.resize_viewport(1200.0, 800.0);
    session.set_tool(Tool::Node);
    session
}

/// The far corner of a box over the first half of every path.
fn half_corner() -> Point {
    pt(NODES as f64 / 2.0 - 0.5, 10.0 * PATHS as f64)
}

fn median(mut runs: Vec<Duration>) -> Duration {
    runs.sort_unstable();
    runs[runs.len() / 2]
}

fn time(mut work: impl FnMut()) -> Duration {
    median(
        (0..RUNS)
            .map(|_| {
                let start = Instant::now();
                work();
                start.elapsed()
            })
            .collect(),
    )
}

fn check(name: &str, measured: Duration, budget: Duration) {
    println!("{name}: {measured:?} (budget {budget:?})");
    if !cfg!(debug_assertions) {
        assert!(measured <= budget, "{name}: {measured:?} over {budget:?}");
    }
}

#[test]
fn anchors_in_box_and_apply_stay_within_their_budgets() {
    let document = document();
    let paths: Vec<_> = document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.path(id))
        .collect();
    let tolerance = Tolerance::from_mm(1e-6);
    let mut inside = Vec::new();
    let boxed = time(|| inside = anchors_in_box(&paths, pt(-1.0, -1.0), half_corner(), tolerance));
    assert_eq!(inside.len() as u64, PATHS * NODES / 2);
    check("anchors_in_box", boxed, Duration::from_millis(1));

    // A selection of the other half, then the box's half added.
    let other: Vec<_> = editing_set_anchors(&paths)
        .map(|(p, a)| (p, a.id))
        .filter(|pair| !inside.contains(pair))
        .collect();
    let applied = time(|| {
        let mut selection = NodeSelection::new();
        selection.select_nodes(other.clone());
        selection.apply(SelectionCombine::Add, &inside);
    });
    check("apply 5,000 to 5,000", applied, Duration::from_millis(5));
}

/// Frames timed per session for the frame budget; only the cheapest counts.
const FRAMES: usize = 41;

/// The CPU time the calling thread has used so far. Time the thread spends preempted by other
/// processes is not in it.
#[cfg(unix)]
fn thread_time() -> Duration {
    let now = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
    Duration::new(
        u64::try_from(now.tv_sec).unwrap(),
        u32::try_from(now.tv_nsec).unwrap(),
    )
}

/// The CPU time of one `draw_list` call (wall time where no thread clock is available).
fn frame(session: &mut Session) -> Duration {
    #[cfg(unix)]
    {
        let start = thread_time();
        let _ = session.draw_list();
        thread_time().saturating_sub(start)
    }
    #[cfg(not(unix))]
    {
        let start = Instant::now();
        let _ = session.draw_list();
        start.elapsed()
    }
}

/// The cheapest of `FRAMES` frames of each session, drawn alternately, as (plain, marquee).
fn cheapest_frames(plain: &mut Session, marquee: &mut Session) -> (Duration, Duration) {
    (0..FRAMES).fold((Duration::MAX, Duration::MAX), |(p, m), _| {
        (p.min(frame(plain)), m.min(frame(marquee)))
    })
}

/// The frame budget of criterion 25, 16.7 ms, measured so that a busy machine does not fail it.
///
/// A median of wall-clock frames failed on a shared CI runner (PR #135: 17.99 ms, while this
/// machine measures 10 to 12 ms). Wall time counts every moment another process holds the core,
/// and a 10 ms frame spans several scheduler slices. So the frame is measured as:
/// - the thread's own CPU time (`CLOCK_THREAD_CPUTIME_ID`), which leaves out preemption;
/// - the cheapest of `FRAMES` frames, since what load still adds (cache and memory
///   contention) only ever makes a frame dearer;
/// - drawn alternately with the plain Node-tool frame from a second session of the same
///   document, so the printed ratio compares frames taken under the same conditions.
///
/// The budget stays the absolute 16.7 ms of criterion 25 and `adrs.md` decision 8; the ratio is
/// printed, not asserted (decision 8, note of 2026-10-14). Off Unix the frame falls back to
/// wall time; the budget job runs on Linux.
#[test]
fn the_marquee_frame_and_the_release_stay_within_their_budgets() {
    let mut plain = session();
    let mut session = session();
    session.pointer_down(pt(-5.0, -5.0), false);
    session.pointer_hover(half_corner(), false, false);
    let (plain_frame, marquee) = cheapest_frames(&mut plain, &mut session);
    println!(
        "plain Node frame {plain_frame:?}, marquee frame {marquee:?} (CPU time), ratio {:.2}",
        marquee.as_secs_f64() / plain_frame.as_secs_f64()
    );
    check("marquee frame", marquee, Duration::from_micros(16_700));

    let start = Instant::now();
    session.pointer_up(half_corner(), false, false);
    let _ = session.draw_list();
    check(
        "release to frame",
        start.elapsed(),
        Duration::from_millis(100),
    );

    let start = Instant::now();
    let _ = session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    let _ = session.draw_list();
    check(
        "Ctrl+A to frame",
        start.elapsed(),
        Duration::from_millis(100),
    );
    assert_eq!(
        session.selection_announcement(),
        format!("{} nodes selected.", PATHS * NODES)
    );
}
