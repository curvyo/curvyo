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
    check_against(label, took, BUDGET);
}

fn check_against(label: &str, took: Duration, budget: Duration) {
    println!("{label}: {took:?} (budget {budget:?})");
    if !cfg!(debug_assertions) {
        assert!(took <= budget, "{label}: {took:?} is over {budget:?}");
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

/// `0149` criterion 11 (ADR 0083 §10): a path box of the default coffin edits within 100 ms.
#[test]
fn a_path_box_of_the_coffin_edits_within_the_budget() {
    use curvyo_document_core::{NewAnchor, Point};
    use curvyo_ui_core::AnchorIdMinter;
    let document = Document::new(1);
    let mut minter = AnchorIdMinter::new(9);
    let anchors: Vec<NewAnchor> = [
        (-50.0, -10.0),
        (-25.0, -30.0),
        (50.0, -20.0),
        (50.0, 20.0),
        (-25.0, 30.0),
        (-50.0, 10.0),
    ]
    .iter()
    .map(|&(x, y)| NewAnchor::corner(minter.mint(), Point::new(x, y)))
    .collect();
    let _source = document.create_path(&anchors, true);
    let mut s = Session::open(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1600.0, 900.0);
    s.set_tool(Tool::Generators);
    s.set_generator_kind("curvyo.generators/path-box");
    assert!(s.place_generated(50.0, 0.0), "pick the outline");
    let (_, took) = cpu_time_of(|| {
        set(&mut s, "inner_height", "60");
        s.draw_list()
    });
    check("path box, coffin", took);
}

/// A heart of four cubics at `scale`: the tip and the notch are corners, the lobes bend (the
/// fixture of `0149` slice 2, criterion 11).
fn heart_anchors(
    scale: f64,
    minter: &mut curvyo_ui_core::AnchorIdMinter,
) -> Vec<curvyo_document_core::NewAnchor> {
    use curvyo_document_core::{NewAnchor, Point, Vec2};
    // (point, handle in, handle out) of the cubics (0,40) > (-40,-10) > (0,-22) > (40,-10) > (0,40).
    let nodes = [
        ((0.0, 40.0), (10.0, -15.0), (-10.0, -15.0)),
        ((-40.0, -10.0), (0.0, 20.0), (0.0, -25.0)),
        ((0.0, -22.0), (-12.0, -10.0), (12.0, -10.0)),
        ((40.0, -10.0), (0.0, -25.0), (0.0, 20.0)),
    ];
    nodes
        .iter()
        .map(|&((x, y), hin, hout)| {
            let mut a = NewAnchor::corner(minter.mint(), Point::new(x * scale, y * scale));
            a.handle_in = Vec2::new(hin.0 * scale, hin.1 * scale);
            a.handle_out = Vec2::new(hout.0 * scale, hout.1 * scale);
            a
        })
        .collect()
}

/// The slots of the wall pieces: a rectangular slot has two long vertical edges.
fn slots_of(generated: &curvyo_extension_core::Generated) -> usize {
    let mut edges = 0;
    for part in &generated.parts {
        let height = part.paths[0]
            .segments
            .iter()
            .map(|c| c.to.y)
            .fold(f64::MIN, f64::max)
            - part.paths[0]
                .segments
                .iter()
                .map(|c| c.to.y)
                .fold(f64::MAX, f64::min);
        if (height - 56.0).abs() > 1e-6 {
            continue;
        }
        for path in &part.paths {
            let mut from = path.start;
            for c in &path.segments {
                if (c.to.x - from.x).abs() < 1e-9 && (c.to.y - from.y).abs() > 20.0 {
                    edges += 1;
                }
                from = c.to;
            }
        }
    }
    edges / 2
}

/// `0149` criterion 11 (ADR 0083 §13.13), slice 2: a heart box with at least 400 slots edits within
/// 300 ms, edit and commit included, and the pick hover (a draft) takes at most 100 ms.
#[test]
fn a_path_box_of_the_heart_with_400_slots_edits_within_300_ms_and_its_hover_within_100_ms() {
    use curvyo_extension_core::{GenerateRequest, ParamValue, Seed, StoredParams};
    use curvyo_ui_core::{ExtensionSettings, instance_source};
    let document = Document::new(1);
    let mut minter = curvyo_ui_core::AnchorIdMinter::new(9);
    let source = document.create_path(&heart_anchors(6.0, &mut minter), true);
    let mut s = Session::open(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1600.0, 900.0);
    s.set_tool(Tool::Generators);
    s.set_generator_kind("curvyo.generators/path-box");
    assert!(s.place_generated(240.0, -60.0), "pick the outline");
    set(&mut s, "slit_spacing", "1.4");
    let (_, took) = cpu_time_of(|| {
        set(&mut s, "inner_height", "60");
        s.draw_list()
    });
    check_against("path box, heart, edit", took, Duration::from_millis(300));

    // The slot count of the same box through the generator: at least 400.
    let mut anchors = Vec::new();
    let mut ids = curvyo_ui_core::AnchorIdMinter::new(11);
    anchors.extend(heart_anchors(6.0, &mut ids));
    let scratch = Document::new(1);
    let outline = scratch.create_path(&anchors, true);
    let path = instance_source::outline_of(&scratch.object(outline).expect("the object"))
        .expect("a closed outline");
    let mut params = StoredParams::new();
    params.insert("outline".to_owned(), ParamValue::Path(path).to_stored());
    params.insert(
        "slit_spacing".to_owned(),
        ParamValue::Length(curvyo_document_core::Length::from_mm(1.4)).to_stored(),
    );
    let generated = curvyo_ui_core::generate_object(
        "curvyo.generators",
        &GenerateRequest {
            kind: "path-box",
            params: &params,
            seed: Seed(1),
            edit: None,
        },
    )
    .expect("the generators extension");
    let slots = slots_of(&generated);
    assert!(slots >= 400, "at least 400 slots: {slots}");

    let settings = ExtensionSettings::default();
    let (marks, hover) = cpu_time_of(|| {
        instance_source::hover_marks(&document, &settings, "curvyo.generators/path-box", source)
    });
    assert!(marks.is_ok(), "the heart hovers");
    check_against(
        "path box, heart, pick hover",
        hover,
        Duration::from_millis(100),
    );
}
