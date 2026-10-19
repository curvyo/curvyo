//! Node editing of a compound path at the size of its budget (`specs/0055-compound-path-node-
//! editing` criterion 11): a closed rectangle with 2000 open lines of 2 nodes inside (4004 nodes,
//! `0054` criterion 32 c) in the Node tool.
//! - from entering the Node tool to the finished draw list with the node overlay: at most 500 ms;
//! - one frame of a drag of one node (the move and the draw list): at most 33 ms, 30 frames per
//!   second, the median of 15 frames;
//! - Delete of 2000 selected nodes (both nodes of 1000 lines, taken by a node marquee): at most
//!   1 s.
//!
//! Budgets are asserted in release builds only; a debug build runs a tenth of the size and
//! prints the numbers.
//! Run: `cargo test --release -p curvyo-editor-wasm --test compound_node_editing_budget -- --nocapture`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]

use std::time::{Duration, Instant};

use curvyo_document_core::{AnchorId, Document, NewAnchor, NodeId, Point, pack};
use curvyo_editor_wasm::{Session, Tool};

const LINES: u64 = if cfg!(debug_assertions) { 200 } else { 2000 };
const FRAMES: usize = 15;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// The plate: a 1000 by 1100 mm rectangle with `LINES` short horizontal lines inside, 40 per row
/// of 19 mm.
fn plate() -> (Document, NodeId) {
    let document = Document::new(1);
    let mut next = 0;
    let mut corner = |x: f64, y: f64| {
        next += 1;
        NewAnchor::corner(AnchorId::new(1, next), pt(x, y))
    };
    let mut outlines = vec![(
        vec![
            corner(0.0, -100.0),
            corner(0.0, 1000.0),
            corner(1000.0, 1000.0),
            corner(1000.0, -100.0),
        ],
        true,
    )];
    for line in 0..LINES {
        let (x, y) = (
            10.0 + (line % 40) as f64 * 24.0,
            10.0 + (line / 40) as f64 * 19.0,
        );
        outlines.push((vec![corner(x, y), corner(x + 10.0, y)], false));
    }
    let seed = document.create_path(&[corner(2000.0, 0.0), corner(2001.0, 0.0)], false);
    let node = document
        .replace_with_path(&[seed], seed, &outlines, "combine_paths")
        .unwrap();
    (document, node)
}

fn session_of(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.resize_viewport(1200.0, 800.0);
    session.set_tool(Tool::Select);
    session
}

fn check(name: &str, measured: Duration, budget: Duration) {
    println!("{name}: {measured:?} (budget {budget:?})");
    if !cfg!(debug_assertions) {
        assert!(
            measured <= budget,
            "{name}: {measured:?} is over {budget:?}"
        );
    }
}

#[test]
fn the_node_tool_on_a_plate_of_4004_nodes_stays_within_its_budgets() {
    let (document, node) = plate();
    let mut session = session_of(&document);
    let _ = session.draw_list();

    // Entering the Node tool to the drawn overlay.
    let start = Instant::now();
    session.set_tool(Tool::Node);
    let _ = session.draw_list();
    check("overlay", start.elapsed(), Duration::from_millis(500));
    assert_eq!(
        session.node_toolbar_state().total_nodes as u64,
        4 + 2 * LINES
    );

    // A drag of the first line's first node, one frame per pointer move.
    let from = pt(10.0, 10.0);
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    let mut frames = Vec::with_capacity(FRAMES);
    for step in 1..=FRAMES {
        let start = Instant::now();
        session.pointer_hover(pt(10.0 + step as f64, 12.0), false, false);
        let _ = session.draw_list();
        frames.push(start.elapsed());
    }
    session.pointer_up(pt(10.0 + FRAMES as f64, 12.0), false, false);
    frames.sort_unstable();
    check("drag frame", frames[FRAMES / 2], Duration::from_millis(33));

    // A node marquee over the first half of the rows, from outside the rectangle (no hit there),
    // then Delete of its nodes: 2000 nodes, both of every line in those rows.
    let half_rows = LINES / 40 / 2;
    let bottom = 10.0 + (half_rows - 1) as f64 * 19.0 + 9.5;
    session.pointer_hover(pt(-50.0, -50.0), false, false);
    session.pointer_down(pt(-50.0, -50.0), false);
    session.pointer_hover(pt(1050.0, bottom), false, false);
    session.pointer_up(pt(1050.0, bottom), false, false);
    assert_eq!(
        session.node_toolbar_state().selected_nodes as u64,
        half_rows * 40 * 2
    );
    let start = Instant::now();
    session.delete_selected();
    check("delete", start.elapsed(), Duration::from_secs(1));
    let after = session_path(&session, node);
    assert_eq!(
        after.extra_subpaths.len() as u64,
        LINES - half_rows * 40,
        "the lines of the box are removed"
    );
}

fn session_path(session: &Session, node: NodeId) -> curvyo_document_core::PathSnapshot {
    curvyo_document_core::unpack(3, &session.pack("0.1.0").unwrap())
        .unwrap()
        .path(node)
        .unwrap()
}
