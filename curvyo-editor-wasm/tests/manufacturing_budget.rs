//! The laser page's drawing budget (`specs/0093-laser-workbench-shell` criterion 14,
//! `adrs.md` decision 5, with `0098`): with 1,000 objects on the drawing page placed on a laser
//! page (an older file's placement, `0098` criterion 12), activating the laser page and drawing
//! the frame costs at most 1.0 s of thread CPU (`tests/cpu_time/`), and a drawing edit shows on
//! the laser page in the very next frame. Like the other budgets, the time
//! is asserted in a release build (CI's `performance-budgets` job); a debug build only prints.

#![allow(clippy::unwrap_used)]

mod cpu_time;

mod page_kind_support;
use curvyo_document_core::{Document, DocumentSize, NewOutputPage, Point, RectBounds};
use curvyo_editor_wasm::{KeyInput, Session, Workbench};

const BUDGET_S: f64 = 1.0;

/// The session and the laser page's id.
fn scene() -> (Session, String) {
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
    let page = document
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let bytes = page_kind_support::with_placement(&document, "first");
    let mut session = Session::open(10, &bytes).unwrap();
    session.resize_viewport(1920.0, 1080.0);
    (session, page.id)
}

#[test]
fn opening_the_laser_workbench_with_1000_objects_stays_within_one_second() {
    let (mut session, page) = scene();
    // The drawing has been read once, as it is when the maker has worked on it.
    let before = session.frame_draw_list().triangles.len();
    let (frame, time) = cpu_time::cpu_time_of(|| {
        assert!(session.activate_page(&page));
        assert!(matches!(session.workbench(), Workbench::Manufacturing(_)));
        session.frame_draw_list()
    });
    let seconds = time.as_secs_f64();
    eprintln!("opening the Laser workbench: {seconds:.3} s thread CPU");
    assert!(frame.triangles.len() >= before, "every object is placed");
    assert!(
        before > 2 * 1_000 * 6,
        "drawn on page 1 and placed on the laser page"
    );
    if !cfg!(debug_assertions) {
        assert!(seconds <= BUDGET_S, "{seconds:.3} s > {BUDGET_S} s");
    }
}

#[test]
fn a_drawing_edit_shows_on_the_output_page_in_the_next_frame() {
    let (mut session, page) = scene();
    assert!(session.activate_page(&page));
    let before = session.frame_draw_list().triangles.len();
    let (frame, time) = cpu_time::cpu_time_of(|| {
        assert!(session.activate_page("first"));
        // One drawing edit: Select all, Delete.
        let _ = session.key_down(key("a", true));
        let _ = session.key_down(key("Delete", false));
        assert!(session.activate_page(&page));
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
