//! The Laser workbench's opening budget (`specs/0093-laser-workbench-shell` criterion 14,
//! `adrs.md` decision 5): with 1,000 objects on the drawing page, adding the laser manufacturing
//! and drawing its first frame costs at most 1.0 s of thread CPU (`tests/cpu_time/`), and a
//! drawing edit shows on the output page in the very next frame. Like the other budgets, the time
//! is asserted in a release build (CI's `performance-budgets` job); a debug build only prints.
//!
//! The roles of `specs/0027-manufacturing-roles` criterion 21 (`adrs.md` note 6): with 1,000
//! rectangles in 10 layers and the Laser tab open, a role set plus the next frame costs at most
//! 50 ms; with 200 layers, the Laser tab's view including the Layers block at most 100 ms.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use curvyo_document_core::{Document, Point, RectBounds, pack};
use curvyo_editor_wasm::{KeyInput, Session, Workbench};

const BUDGET_S: f64 = 1.0;
const ROLE_SET_BUDGET_S: f64 = 0.050;
const LAYERS_VIEW_BUDGET_S: f64 = 0.100;

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

/// `layers` layers holding `per_layer` 3 x 3 mm rectangles each, opened in a session with the
/// Laser workbench added and its first frame drawn.
fn layered_scene(layers: u32, per_layer: u32) -> Session {
    let document = Document::new(9);
    for layer_index in 0..layers {
        let layer = document.new_layer(None);
        document.set_draw_parent(Some(layer));
        document.batch("scene", |doc| {
            for index in 0..per_layer {
                let n = layer_index * per_layer + index;
                let x = 5.0 + f64::from(n % 40) * 5.0;
                let y = 5.0 + f64::from(n / 40 % 25) * 10.0;
                let _ = doc.create_rect(RectBounds::from_corners(
                    Point::new(x, y),
                    Point::new(x + 3.0, y + 3.0),
                ));
            }
        });
    }
    let mut session = Session::open(10, &pack(&document, "budget").unwrap()).unwrap();
    session.resize_viewport(1920.0, 1080.0);
    session.add_manufacturing("curvyo.laser").unwrap();
    let _ = session.frame_draw_list();
    session
}

fn first_row_key(session: &Session) -> String {
    let view: serde_json::Value = serde_json::from_str(&session.manufacturing_view_json()).unwrap();
    view["layers"]["rows"][0]["key"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn a_role_set_and_the_next_frame_with_1000_objects_in_10_layers_stay_within_50_ms() {
    let mut session = layered_scene(10, 100);
    let key = first_row_key(&session);
    let (frame, time) = cpu_time::cpu_time_of(|| {
        assert!(session.set_layer_role(&key, Some(curvyo_laser_core::Role::Cut)));
        session.frame_draw_list()
    });
    let seconds = time.as_secs_f64();
    eprintln!("a role set and the next Laser frame: {seconds:.3} s thread CPU");
    assert!(
        frame
            .triangles
            .iter()
            .any(|v| v.color == curvyo_render_core::ROLE_CUT),
        "the new role is drawn"
    );
    if !cfg!(debug_assertions) {
        assert!(
            seconds <= ROLE_SET_BUDGET_S,
            "{seconds:.3} s > {ROLE_SET_BUDGET_S} s"
        );
    }
}

#[test]
fn the_laser_view_with_200_layers_stays_within_100_ms() {
    let session = layered_scene(200, 1);
    let (json, time) = cpu_time::cpu_time_of(|| session.manufacturing_view_json());
    let seconds = time.as_secs_f64();
    eprintln!("the Laser view with 200 layers: {seconds:.3} s thread CPU");
    let view: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(view["layers"]["rows"].as_array().unwrap().len(), 200);
    if !cfg!(debug_assertions) {
        assert!(
            seconds <= LAYERS_VIEW_BUDGET_S,
            "{seconds:.3} s > {LAYERS_VIEW_BUDGET_S} s"
        );
    }
}

fn key(key: &str, ctrl: bool) -> KeyInput<'_> {
    KeyInput {
        key,
        ctrl,
        ..KeyInput::default()
    }
}
