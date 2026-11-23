//! The thread-CPU budget of the part of a recovery write that runs on the canvas thread
//! (`specs/0136-autosave-and-restore` criterion 2, ADR 0079 decision 4, `adrs.md` "Criterion 1's
//! timing test"): after a step on a project of 10,000 objects, the delta export and the hand-over
//! of the assets the host lacks take less than a frame (16.7 ms), asserted at four times the
//! figure in an optimised build as the other budget tests do; a debug build only smoke-tests. The
//! worker's pack runs off this thread and has no frame budget. The first hand-over (a snapshot, a
//! reseed) runs at an idle moment and is measured and printed only. Run:
//! `cargo test --release -p curvyo-editor-wasm --test recovery_export_budget -- --nocapture`.

#![cfg(unix)]
#![allow(clippy::unwrap_used, missing_docs)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{Document, Length, Point, RectBounds, pack};
use curvyo_editor_wasm::{KeyInput, RecoveryWriter, Session, Tool};
use curvyo_ui_core::MoveEntryMode;

const OBJECTS: usize = 10_000;
const FRAME: Duration = Duration::from_micros(16_700);

fn big_session() -> Session {
    let document = Document::new(1);
    for n in 0..OBJECTS {
        let (column, row) = (n % 100, n / 100);
        let _ = document.create_rect(RectBounds {
            origin: Point::new(
                f64::from(u32::try_from(column).unwrap()) * 12.0,
                f64::from(u32::try_from(row).unwrap()) * 12.0,
            ),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        });
    }
    let mut session = Session::open(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    session.resize_viewport(1200.0, 800.0);
    session.set_tool(Tool::Select);
    session
}

/// One step: everything selected and moved 5 mm, 10,000 objects changed at once (the worst case).
fn big_step(session: &mut Session) {
    session.pointer_down(Point::new(-100.0, -100.0), false);
    session.pointer_up(Point::new(100_000.0, 100_000.0), false, false);
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

#[test]
fn a_step_on_10_000_objects_costs_the_canvas_thread_less_than_a_frame() {
    let mut session = big_session();
    assert!(!session.recovery_changed(), "an opened project is no step");

    let (seed, seed_time) = cpu_time_of(|| session.recovery_export().unwrap());
    assert!(seed.full);
    eprintln!(
        "reseed snapshot of {OBJECTS} objects: {seed_time:?} ({} bytes, idle moment)",
        seed.bytes.len()
    );
    let mut writer = RecoveryWriter::new();
    writer.apply(7, seed.full, &seed.bytes).unwrap();

    big_step(&mut session);
    assert!(session.recovery_changed(), "the step was seen");
    let ((delta, assets), took) = cpu_time_of(|| {
        (
            session.recovery_export().unwrap(),
            session.recovery_new_assets(),
        )
    });
    assert!(!delta.full);
    assert!(assets.is_empty());
    eprintln!(
        "main-thread part of a write after a step of {OBJECTS} objects: {took:?} (budget {FRAME:?}, asserted at four times; {} bytes)",
        delta.bytes.len()
    );
    if !cfg!(debug_assertions) {
        assert!(took <= FRAME * 4, "{took:?} over 4 x {FRAME:?}");
    }

    // The worker's side is off this thread: it takes the delta and packs, and the copy is whole.
    writer.apply(7, delta.full, &delta.bytes).unwrap();
    let packed = writer.pack().unwrap().unwrap();
    let (_, pack_time) = cpu_time_of(|| writer.pack().unwrap().unwrap());
    eprintln!(
        "worker pack of {OBJECTS} objects: {pack_time:?} ({} bytes, off the canvas thread)",
        packed.bytes.len()
    );
    assert_eq!(packed.pages, 1);
}
