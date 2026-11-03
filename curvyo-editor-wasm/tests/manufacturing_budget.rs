//! The Laser workbench's opening budget (`specs/0093-laser-workbench-shell` criterion 14,
//! `adrs.md` decision 5): with 1,000 objects on the drawing page, adding the laser manufacturing
//! and drawing its first frame costs at most 1.0 s of thread CPU (`tests/cpu_time/`), and a
//! drawing edit shows on the output page in the very next frame. Like the other budgets, the time
//! is asserted in a release build (CI's `performance-budgets` job); a debug build only prints.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use curvyo_document_core::{Document, Point, RectBounds, pack};
use curvyo_editor_wasm::{KeyInput, Session, Workbench};

const BUDGET_S: f64 = 1.0;

fn scene() -> Session {
    let document = Document::new(9);
    document.batch("scene", |doc| {
        for index in 0..1_000 {
            let x = 5.0 + f64::from(index % 40) * 5.0;
            let y = 5.0 + f64::from(index / 40) * 10.0;
            let _ = doc.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 3.0, y + 3.0),
            ));
        }
    });
    let mut session = Session::open(10, &pack(&document, "budget").unwrap()).unwrap();
    session.resize_viewport(1920.0, 1080.0);
    session
}

#[test]
fn opening_the_laser_workbench_with_1000_objects_stays_within_one_second() {
    let mut session = scene();
    // The drawing has been read once, as it is when the maker has worked on it.
    let _ = session.frame_draw_list();
    let (frame, time) = cpu_time::cpu_time_of(|| {
        let id = session.add_manufacturing("curvyo.laser").unwrap();
        assert_eq!(session.workbench(), Workbench::Manufacturing(id));
        session.frame_draw_list()
    });
    let seconds = time.as_secs_f64();
    eprintln!("opening the Laser workbench: {seconds:.3} s thread CPU");
    assert!(frame.triangles.len() > 1_000 * 6, "every object is placed");
    if !cfg!(debug_assertions) {
        assert!(seconds <= BUDGET_S, "{seconds:.3} s > {BUDGET_S} s");
    }
}

#[test]
fn a_drawing_edit_shows_on_the_output_page_in_the_next_frame() {
    let mut session = scene();
    let id = session.add_manufacturing("curvyo.laser").unwrap();
    let before = session.frame_draw_list().triangles.len();
    let (frame, time) = cpu_time::cpu_time_of(|| {
        session.set_workbench(Workbench::Vector);
        // One drawing edit in Vector: Select all, Delete.
        let _ = session.key_down(key("a", true));
        let _ = session.key_down(key("Delete", false));
        session.set_workbench(Workbench::Manufacturing(id));
        session.frame_draw_list()
    });
    let seconds = time.as_secs_f64();
    eprintln!("an edit and the next Laser frame: {seconds:.3} s thread CPU");
    assert!(
        frame.triangles.len() < before,
        "the deleted objects are gone from the output page"
    );
    if !cfg!(debug_assertions) {
        assert!(seconds <= BUDGET_S, "{seconds:.3} s > {BUDGET_S} s");
    }
}

fn key(key: &str, ctrl: bool) -> KeyInput<'_> {
    KeyInput {
        key,
        ctrl,
        ..KeyInput::default()
    }
}
