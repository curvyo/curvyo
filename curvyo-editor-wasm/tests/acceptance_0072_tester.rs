//! Tester's acceptance tests for `specs/0072-pen-angle-snap-and-typed-values`: only what the
//! implementer's tests leave open. Black box: 11 (Ctrl and Tab outside the Pen), 35 (every
//! number carries its unit), 10 (the snap's added cost, release benchmark). White box: the
//! continuation from the first node (7, 31), a refused frozen press (20, 26), degenerate texts
//! and the coordinate limit (24, 26, 28).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use curvyo_document_core::{AnchorId, DisplayUnit, Document, NewAnchor, Point, pack, unpack};
use curvyo_editor_wasm::{KeyInput, Session, Tool};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn session_with(document: &Document, tool: Tool) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.resize_viewport(1200.0, 800.0);
    session.set_tool(tool);
    session
}

fn tab(session: &mut Session, ctrl: bool) {
    let _ = session.key_down(KeyInput {
        key: "Tab",
        ctrl,
        ..KeyInput::default()
    });
}

/// A Pen session in `unit` with one node placed at L = (100, 50).
fn pen_with_last_node(unit: DisplayUnit) -> Session {
    let mut session = session_with(&Document::new(1), Tool::Pen);
    session.set_display_unit(unit);
    let l = pt(100.0, 50.0);
    session.pointer_hover(l, false, false);
    session.pointer_down(l, false);
    session.pointer_up(l, false, false);
    session
}

/// Criterion 11: outside the Pen, Tab (with or without Ctrl) opens no Pen entry, and Ctrl gives
/// a Node-tool node drag no angle lock: the node lands on the raw pointer.
#[test]
fn ctrl_and_tab_keep_their_meaning_outside_the_pen() {
    let document = Document::new(1);
    let path = document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(100.0, 50.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(300.0, 50.0)),
        ],
        false,
    );

    for tool in [Tool::Select, Tool::Node, Tool::Rectangle, Tool::Ellipse] {
        let mut session = session_with(&document, tool);
        session.modifiers_changed(false, true, false);
        tab(&mut session, false);
        tab(&mut session, true);
        assert!(
            session.pen_entry_chip().is_none(),
            "{tool:?}: Tab opened a Pen entry"
        );
    }

    // Node tool: drag the first node 30 mm right and 2 mm down (3.8 degrees) with Ctrl held.
    let mut session = session_with(&document, Tool::Node);
    session.modifiers_changed(false, true, false);
    let from = pt(100.0, 50.0);
    let to = pt(130.0, 52.0);
    session.pointer_hover(from, false, true);
    session.pointer_down(from, false);
    session.pointer_hover(pt(115.0, 51.0), false, true);
    session.pointer_hover(to, false, true);
    session.pointer_up(to, false, true);
    let reread = unpack(9, &session.pack("0.1.0").unwrap()).unwrap();
    let moved = reread.path(path).unwrap().anchors[0].point;
    assert!(
        (moved.x - 130.0).abs() < 1e-6 && (moved.y - 52.0).abs() < 1e-6,
        "the Node tool got an angle lock: node at ({}, {})",
        moved.x,
        moved.y
    );
}

/// Criteria 35 and 8: in every display unit the drag readout carries its length unit and "°",
/// and both chip fields keep their unit suffix, also when the field is emptied.
#[test]
fn every_number_on_screen_has_its_unit() {
    for (unit, symbol) in [
        (DisplayUnit::Mm, "mm"),
        (DisplayUnit::Cm, "cm"),
        (DisplayUnit::In, "in"),
    ] {
        // Drag readout while a node is dragged out, with and without Ctrl.
        for ctrl in [false, true] {
            let mut session = pen_with_last_node(unit);
            session.modifiers_changed(false, ctrl, false);
            let n = pt(130.0, 50.0);
            session.pointer_hover(n, false, ctrl);
            session.pointer_down(n, false);
            session.pointer_hover(pt(150.0, 52.0), false, ctrl);
            let readout = session
                .pen_drag_readout()
                .expect("a readout while the button is held");
            assert!(
                readout.text.contains(symbol) && readout.text.contains('°'),
                "{symbol}, ctrl {ctrl}: readout {:?} lacks a unit",
                readout.text
            );
            session.pointer_up(pt(150.0, 52.0), false, ctrl);
            assert!(
                session.pen_drag_readout().is_none(),
                "the readout outlives the release"
            );
        }

        // The chip: both suffixes, before and after emptying each field.
        let mut session = pen_with_last_node(unit);
        session.pointer_hover(pt(130.0, 40.0), false, false);
        tab(&mut session, false);
        let chip = session.pen_entry_chip().expect("Tab opens the chip");
        assert_eq!(chip.fields.len(), 2);
        assert_eq!(chip.fields[0].suffix, symbol);
        assert_eq!(chip.fields[1].suffix, "°");
        session.pen_entry_edit(0, "");
        session.pen_entry_edit(1, "");
        let chip = session.pen_entry_chip().expect("still open after edits");
        assert_eq!(
            chip.fields[0].suffix, symbol,
            "emptied Length lost its unit"
        );
        assert_eq!(chip.fields[1].suffix, "°", "emptied Angle lost its unit");
    }
}

fn key(session: &mut Session, name: &str) {
    let _ = session.key_down(KeyInput {
        key: name,
        ..KeyInput::default()
    });
}

fn click(session: &mut Session, at: Point, ctrl: bool) {
    session.modifiers_changed(false, ctrl, false);
    session.pointer_hover(at, false, ctrl);
    session.pointer_down(at, false);
    session.pointer_up(at, false, ctrl);
}

fn typed_node(session: &mut Session, length: &str, angle: &str) -> bool {
    key(session, "Tab");
    session.pen_entry_edit(0, length);
    session.pen_entry_edit(1, angle);
    session.pen_entry_place()
}

/// Criteria 7 and 31 (white box): a continuation from the first node of a path takes E as L for
/// both the typed node and the Ctrl snap, and the new nodes are prepended in stored order.
#[test]
fn a_continuation_from_the_first_node_types_and_snaps_from_e() {
    let document = Document::new(1);
    let path = document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(100.0, 50.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(200.0, 50.0)),
        ],
        false,
    );
    let mut session = session_with(&document, Tool::Pen);
    click(&mut session, pt(100.0, 50.0), false);
    assert!(
        typed_node(&mut session, "40", "180"),
        "Tab from E typed nothing"
    );
    click(&mut session, pt(59.0, 80.0), true);
    key(&mut session, "Enter");
    let reread = unpack(9, &session.pack("0.1.0").unwrap()).unwrap();
    let got: Vec<(f64, f64)> = reread
        .path(path)
        .unwrap()
        .anchors
        .iter()
        .map(|a| (a.point.x, a.point.y))
        .collect();
    assert_eq!(got.len(), 4, "{got:?}");
    let expected = [(60.0, 80.017), (60.0, 50.0), (100.0, 50.0), (200.0, 50.0)];
    for ((x, y), (ex, ey)) in got.iter().zip(expected) {
        assert!((x - ex).abs() < 0.005 && (y - ey).abs() < 0.005, "{got:?}");
    }
}

/// Criteria 20 and 26 (white box): a frozen press with a zero length adds no node, neither at
/// the press nor at its release, and keeps the chip open with the notice.
#[test]
fn a_frozen_press_with_a_zero_length_adds_nothing() {
    let mut session = pen_with_last_node(DisplayUnit::Mm);
    key(&mut session, "Tab");
    session.pen_entry_edit(0, "0");
    click(&mut session, pt(300.0, 300.0), false);
    assert_eq!(session.pen_in_progress().unwrap().len(), 1);
    assert!(
        session.pen_entry_chip().is_some(),
        "zero length closed the chip"
    );
    assert_eq!(
        session.take_pen_entry_notice().as_deref(),
        Some("Length must be more than 0.")
    );
}

/// Criteria 24, 28 (white box, degenerate input): non-finite and huge texts never write a
/// non-finite or out-of-limit point.
#[test]
fn degenerate_texts_stay_finite_and_inside_the_limit() {
    for (length, angle) in [
        ("1e400", "0"),
        ("inf", "0"),
        ("NaN", "NaN"),
        ("1e308", "45"),
        ("30", "1e300"),
        ("-1e308", "-1e308"),
    ] {
        let mut session = pen_with_last_node(DisplayUnit::Mm);
        let _ = typed_node(&mut session, length, angle);
        for node in session.pen_in_progress().unwrap() {
            assert!(
                node.point.x.is_finite()
                    && node.point.y.is_finite()
                    && node.point.x.abs() <= 10_000_000.0 + 1e-6
                    && node.point.y.abs() <= 10_000_000.0 + 1e-6,
                "{length:?}/{angle:?}: {:?}",
                node.point
            );
        }
    }
}

/// Criteria 26 and 28 (white box): the Pen can place a node on the coordinate limit (the pointer
/// is clamped at 1e9 mm, not 1e7). A typed segment pointing outward from there is shortened to
/// zero: that is the zero-length case of criterion 26, not a second node on the same spot.
#[test]
fn a_segment_shortened_to_zero_at_the_limit_places_no_node() {
    let mut session = session_with(&Document::new(1), Tool::Pen);
    click(&mut session, pt(10_000_000.0, 50.0), false);
    let placed = typed_node(&mut session, "30", "0");
    let nodes: Vec<Point> = session
        .pen_in_progress()
        .unwrap()
        .iter()
        .map(|n| n.point)
        .collect();
    assert!(
        !placed && nodes.len() == 1,
        "a zero-length segment was placed: {nodes:?}"
    );
}

/// Criterion 10: with 5000 open paths and a path in progress, Ctrl adds at most 0.1 ms to a
/// pointer move (the hover and the frame that draws the rubber band). Release build only:
/// `cargo test --release -p curvyo-editor-wasm --test acceptance_0072_tester -- --ignored`.
#[test]
#[ignore = "release-build benchmark"]
fn the_snap_adds_at_most_a_tenth_of_a_millisecond_per_move() {
    // The 5000 open paths are built in the document, not drawn: drawing them one by one
    // is slow for reasons that are not this criterion's.
    let document = Document::new(1);
    for i in 0..5000_u64 {
        let y = 100.0 + f64::from(u32::try_from(i).unwrap()) * 2.0;
        let _ = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 2 * i + 1), pt(0.0, y)),
                NewAnchor::corner(AnchorId::new(1, 2 * i + 2), pt(1.0, y)),
            ],
            false,
        );
    }
    let mut session = session_with(&document, Tool::Pen);
    click(&mut session, pt(100.0, 50.0), false);
    let moves = 2000;
    let mut per_move = |ctrl: bool| {
        session.modifiers_changed(false, ctrl, false);
        let start = std::time::Instant::now();
        for i in 0..moves {
            session.pointer_hover(pt(130.0, 40.0 + f64::from(i) * 0.01), false, ctrl);
            let _ = session.draw_list();
        }
        start.elapsed().as_secs_f64() * 1000.0 / f64::from(moves)
    };
    // The best of five alternating runs each, so load on the machine does not decide.
    let (mut plain, mut snapped) = (f64::INFINITY, f64::INFINITY);
    for _ in 0..5 {
        plain = plain.min(per_move(false));
        snapped = snapped.min(per_move(true));
    }
    println!("per move with a frame: plain {plain:.4} ms, Ctrl {snapped:.4} ms");
    assert!(
        snapped - plain < 0.1,
        "the snap adds {} ms",
        snapped - plain
    );
}
