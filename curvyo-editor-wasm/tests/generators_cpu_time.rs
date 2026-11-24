//! The cost of a parameter edit of a generated object (`specs/0141-parametric-generators`
//! criterion 4, ADR 0080 §8): the session's edit (the generator, the one commit and the frame's
//! canvas view) of a gear of 200 teeth with spokes, windows, bore and keyway, and of a box with
//! about 100 fingers per edge and a finger lid, at most 100 ms of thread CPU time. Asserted in
//! release builds, printed in debug builds, as the other budget tests do:
//! `cargo test --release -p curvyo-editor-wasm --test generators_cpu_time -- --nocapture`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{Document, pack};
use curvyo_editor_wasm::{Session, Tool};

const BUDGET: Duration = Duration::from_millis(100);

fn session() -> Session {
    let mut session = Session::open(2, &pack(&Document::new(1), "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1600.0, 900.0);
    session
}

fn set(session: &mut Session, key: &str, value: &str) {
    let answer = session.set_generated_value(key, value);
    assert!(!answer.contains("error"), "{key}: {answer}");
}

fn check(label: &str, took: Duration) {
    println!("{label}: {took:?} (budget {BUDGET:?})");
    if !cfg!(debug_assertions) {
        assert!(took <= BUDGET, "{label}: {took:?} is over {BUDGET:?}");
    }
}

#[test]
fn a_gear_of_200_teeth_with_spokes_bore_and_keyway_edits_within_the_budget() {
    let mut s = session();
    s.set_generator_kind("curvyo.generators/gear");
    assert!(s.place_generated(200.0, 200.0));
    for (key, value) in [
        ("teeth", "200"),
        ("module", "1"),
        ("web", "\"spokes\""),
        ("bore", "10"),
        ("keyway", "true"),
        ("spokes", "6"),
    ] {
        set(&mut s, key, value);
    }
    let (_, took) = cpu_time_of(|| {
        set(&mut s, "module", "1.5");
        s.draw_list()
    });
    check("gear, 200 teeth, spokes, bore and keyway", took);
    let panel = s.generated_panel_json();
    assert!(panel.contains("\"known\":true"), "{panel}");
}

#[test]
fn a_box_with_a_hundred_fingers_per_edge_and_a_finger_lid_edits_within_the_budget() {
    let mut s = session();
    s.set_generator_kind("curvyo.generators/box");
    assert!(s.place_generated(500.0, 300.0));
    for (key, value) in [
        ("inner_width", "400"),
        ("inner_depth", "400"),
        ("inner_height", "400"),
        ("finger_width", "2"),
        ("lid", "\"fingers\""),
    ] {
        set(&mut s, key, value);
    }
    let (_, took) = cpu_time_of(|| {
        set(&mut s, "thickness", "3.2");
        s.draw_list()
    });
    check("box, 100 fingers per edge, finger lid", took);
}

#[test]
fn a_rounded_box_of_400_slots_edits_within_the_budget() {
    let mut s = session();
    s.set_generator_kind("curvyo.generators/rounded-box");
    assert!(s.place_generated(500.0, 300.0));
    for (key, value) in [
        ("inner_width", "400"),
        ("inner_depth", "300"),
        ("inner_height", "100"),
        ("wall_slits", "\"all\""),
        ("slit_spacing", "4"),
    ] {
        set(&mut s, key, value);
    }
    // 1,400 mm of wall at 4 mm columns: about 350 slots; the corner arcs add more.
    let (_, took) = cpu_time_of(|| {
        set(&mut s, "corner_radius", "30");
        // The panel read generates again for the values the generator used (`instance::view`).
        let panel = s.generated_panel_json();
        assert!(panel.contains("slit_width"));
        s.draw_list()
    });
    check("rounded box, about 400 slots, edit and panel read", took);
    let panel = s.generated_panel_json();
    assert!(panel.contains("\"known\":true"), "{panel}");
}
