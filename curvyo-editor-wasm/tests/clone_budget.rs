//! The session costs of linked clones (`specs/0049-linked-clone` criteria 43 to 45, 61): Linked
//! clone and Unlink clone of 10,000 objects within 5 s, a frame after the original of 10,000
//! clones moved within 1.2 times the frame of 10,000 ordinary copies (its 150 ms is reported), and
//! Ctrl+A over 10,000 clones drawn with the marks capped. Budgets are asserted in release builds
//! only; a debug build runs a tenth of the size and prints the numbers.
//! Run: `cargo test --release -p curvyo-editor-wasm --test clone_budget -- --nocapture`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]

use std::time::{Duration, Instant};

use curvyo_document_core::{AnchorId, Document, NewAnchor, NodeId, Point, Vec2, pack};
use curvyo_editor_wasm::{KeyInput, Session, Tool};

const COUNT: usize = if cfg!(debug_assertions) {
    1_000
} else {
    10_000
};
const COMMAND_BUDGET: Duration = Duration::from_secs(5);
const FRAME_AFTER_COMMIT_BUDGET: Duration = Duration::from_millis(150);

fn check(label: &str, took: Duration, budget: Duration) {
    println!("{label}: {took:?} (budget {budget:?})");
    if !cfg!(debug_assertions) {
        assert!(took <= budget, "{label}: {took:?} is over {budget:?}");
    }
}

fn path_at(document: &Document, k: usize, nodes: u64, anchor: &mut u64) -> NodeId {
    let (x, y) = ((k % 100) as f64 * 25.0, (k / 100) as f64 * 25.0);
    let anchors: Vec<NewAnchor> = (0..nodes)
        .map(|n| {
            *anchor += 1;
            let angle = n as f64 / nodes as f64 * std::f64::consts::TAU;
            NewAnchor::corner(
                AnchorId::new(1, *anchor),
                Point::new(x + 10.0 + 8.0 * angle.cos(), y + 10.0 + 8.0 * angle.sin()),
            )
        })
        .collect();
    document.create_path(&anchors, true)
}

fn session_of(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1600.0, 900.0);
    session
}

fn select_all(session: &mut Session) {
    let _ = session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}

/// The document of `COUNT` clones of one path of 100 nodes, or of `COUNT` ordinary copies.
fn scene(clones: bool) -> Document {
    let document = Document::new(1);
    let mut anchor = 0;
    let original = path_at(&document, 0, 100, &mut anchor);
    for k in 1..=COUNT {
        if clones {
            let clone = document.linked_clone(&[original], &[]).unwrap()[0];
            let (x, y) = ((k % 100) as f64 * 25.0, (k / 100) as f64 * 25.0);
            document
                .translate_objects(&[clone], Vec2::new(x, y))
                .unwrap();
        } else {
            path_at(&document, k, 100, &mut anchor);
        }
    }
    document
}

#[test]
fn ac45_linked_clone_and_unlink_of_ten_thousand() {
    let document = Document::new(1);
    let mut anchor = 0;
    for k in 0..COUNT {
        path_at(&document, k, 8, &mut anchor);
    }
    let mut session = session_of(&document);
    select_all(&mut session);
    assert_eq!(session.selected_object_count(), COUNT);
    let started = Instant::now();
    let made = session.linked_clone();
    check(
        &format!("Linked clone of {COUNT}"),
        started.elapsed(),
        COMMAND_BUDGET,
    );
    assert_eq!(made.notice, format!("Created {COUNT} linked clones."));
    let started = Instant::now();
    let unlinked = session.unlink_clones();
    check(
        &format!("Unlink clone of {COUNT}"),
        started.elapsed(),
        COMMAND_BUDGET,
    );
    assert_eq!(unlinked.notice, format!("Unlinked {COUNT} clones."));
}

/// Moves the first object (the original) one nudge and times the frame after the commit.
fn frame_after_moving_the_first(document: &Document) -> Duration {
    let mut session = session_of(document);
    let _ = session.draw_list();
    let at = Point::new(18.0, 10.0);
    session.pointer_hover(at, false, false);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
    assert_eq!(session.selected_object_count(), 1);
    let _ = session.key_down(KeyInput {
        key: "ArrowRight",
        ..KeyInput::default()
    });
    let started = Instant::now();
    let _ = session.draw_list();
    started.elapsed()
}

/// Where the frame goes: the read of the document and the artwork of the read.
fn breakdown(document: &Document) {
    let started = Instant::now();
    let read = document.read_objects();
    let read_took = started.elapsed();
    let started = Instant::now();
    let _ = curvyo_render_core::build_artwork(
        &read.objects,
        curvyo_document_core::ViewTransform::new(1.0, Point::new(0.0, 0.0)),
    );
    println!("  read {read_took:?}, artwork {:?}", started.elapsed());
}

#[test]
fn ac43_44_a_frame_after_the_original_moved() {
    let copies = frame_after_moving_the_first(&scene(false));
    println!("the frame after the first of {COUNT} ordinary copies moved: {copies:?}");
    let clones = scene(true);
    breakdown(&clones);
    let with_clones = frame_after_moving_the_first(&clones);
    check(
        &format!("frame after the original of {COUNT} clones moved"),
        with_clones,
        FRAME_AFTER_COMMIT_BUDGET,
    );
    if !cfg!(debug_assertions) {
        assert!(
            with_clones.as_secs_f64() <= 1.2 * copies.as_secs_f64() + 0.005,
            "{with_clones:?} against {copies:?}"
        );
    }
}

/// Criterion 43: the first frame after opening a file of 10,000 clones within 1 s (the Open and
/// the first draw list).
#[test]
fn ac43_the_first_frame_after_opening() {
    let bytes = pack(&scene(true), "0.1.0").unwrap();
    let started = Instant::now();
    let mut session = Session::open(2, &bytes).unwrap();
    session.resize_viewport(1600.0, 900.0);
    let _ = session.draw_list();
    check(
        &format!("open and first frame of {COUNT} clones"),
        started.elapsed(),
        Duration::from_secs(1),
    );
}

#[test]
fn ac61_ctrl_a_over_ten_thousand_clones_draws_the_capped_marks() {
    let mut session = session_of(&scene(true));
    select_all(&mut session);
    let marks = session.clone_marks();
    assert_eq!(marks.tags.len(), 0, "no tags with so many marks");
    // The Style tab's counts stay exact and cheap (review F4).
    let _ = session.clone_panel();
    let started = Instant::now();
    let panel = session.clone_panel();
    check(
        &format!("the clone panel over {COUNT} selected clones"),
        started.elapsed(),
        Duration::from_millis(5),
    );
    assert_eq!(
        panel.followers, COUNT,
        "the original is selected too: exact count"
    );
    let started = Instant::now();
    let _ = session.draw_list();
    let with_marks = started.elapsed();
    let mut plain = session_of(&scene(false));
    select_all(&mut plain);
    let started = Instant::now();
    let _ = plain.draw_list();
    let without = started.elapsed();
    println!("Ctrl+A over {COUNT} clones: {with_marks:?}; over {COUNT} paths: {without:?}");
    if !cfg!(debug_assertions) {
        assert!(
            with_marks.as_secs_f64() <= 1.2 * without.as_secs_f64() + 0.005,
            "{with_marks:?} against {without:?}"
        );
    }
}

/// `specs/0057-clone-overrides` criterion 41: 10,000 clones with own styles that differ from the
/// root's only in fill and stroke colour keep the frame after the original moved within 150 ms
/// (the root tessellation stays shared); with own placement too, the move holds them all back.
#[test]
fn ac41_a_frame_with_ten_thousand_own_colours() {
    let document = scene(true);
    let clones: Vec<NodeId> = document
        .read_objects()
        .objects
        .iter()
        .skip(1)
        .map(curvyo_document_core::ObjectSnapshot::id)
        .collect();
    let root = document.read_objects().objects[0].id();
    document
        .edit_style(&[root], &curvyo_document_core::StyleEdit::FillEnabled(true))
        .unwrap();
    document.set_own_style(&clones, true).unwrap();
    for hue in 0..12_u8 {
        let batch: Vec<NodeId> = clones
            .iter()
            .copied()
            .skip(usize::from(hue))
            .step_by(12)
            .collect();
        let colour = curvyo_document_core::Color {
            r: hue * 20,
            g: 255 - hue * 20,
            b: 40,
        };
        document
            .edit_style(&batch, &curvyo_document_core::StyleEdit::FillColor(colour))
            .unwrap();
        document
            .edit_style(
                &batch,
                &curvyo_document_core::StyleEdit::StrokeColor(colour),
            )
            .unwrap();
    }
    breakdown(&document);
    check(
        &format!("frame after the original of {COUNT} clones with own colours moved"),
        frame_after_moving_the_first(&document),
        FRAME_AFTER_COMMIT_BUDGET,
    );
    document.set_own_placement(&clones, true).unwrap();
    let frame = frame_after_moving_the_first(&document);
    // Not gated before the first measurement (criterion 41): printed for the PR.
    println!("frame after the original of {COUNT} held-back clones moved: {frame:?}");
}
