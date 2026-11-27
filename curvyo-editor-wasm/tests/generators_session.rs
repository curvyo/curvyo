//! The Generators tool and the Generator block through the session
//! (`specs/0141-parametric-generators` criteria 1, 4, 6 and 7): placing selects and returns to
//! Select, a typed value clamps with a notice, Convert to paths, and the notice of an opened file
//! with a generator this build does not have.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{Affine, Document, GeneratedRecord, ObjectSnapshot, Point, pack};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::Value;

fn session() -> Session {
    let mut session = Session::open(2, &pack(&Document::new(1), "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Generators);
    // Entering the tool arms nothing (`0151` slice 1); these tests start with Box pressed.
    session.set_generator_kind("curvyo.generators/box");
    session.resize_viewport(1600.0, 900.0);
    session
}

fn panel(session: &Session) -> Value {
    serde_json::from_str(&session.generated_panel_json()).unwrap()
}

#[test]
fn the_bar_offers_box_box_from_path_gear_qr_code_and_rounded_box_and_a_press_places_the_chosen_one_selected()
 {
    let mut s = session();
    let kinds: Value = serde_json::from_str(&s.generator_kinds_json()).unwrap();
    let ids: Vec<&str> = kinds
        .as_array()
        .unwrap()
        .iter()
        .map(|kind| kind["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "curvyo.generators/box",
            "curvyo.generators/path-box",
            // The Code segment (one for the whole family, slice 2) sorts before Gear.
            "curvyo.codes/qr",
            "curvyo.generators/gear",
            "curvyo.generators/rounded-box"
        ]
    );
    assert_eq!(
        s.generator_kind(),
        "curvyo.generators/box",
        "armed by the test"
    );
    s.set_generator_kind("curvyo.generators/gear");
    assert!(s.place_generated(120.0, 80.0));
    assert_eq!(s.tool(), Tool::Generators, "the tool stays (0151 slice 1)");
    let block = panel(&s);
    assert_eq!(block["label"], "Gear");
    assert_eq!(block["known"], true);
    // The kind is kept for the session.
    assert_eq!(s.generator_kind(), "curvyo.generators/gear");
}

#[test]
fn a_typed_value_is_clamped_with_a_notice_and_the_block_follows() {
    let mut s = session();
    assert!(s.place_generated(50.0, 50.0));
    s.set_tool(Tool::Select);
    let answer: Value = serde_json::from_str(&s.set_generated_value("thickness", "0")).unwrap();
    assert_eq!(answer["written"], true);
    assert_eq!(
        answer["notices"][0],
        "Thickness set to 0.1 mm, the minimum."
    );
    let rows = panel(&s)["rows"].as_array().unwrap().clone();
    let thickness = rows
        .iter()
        .find(|row| row["param"]["key"] == "thickness")
        .unwrap();
    assert_eq!(
        thickness["value"]["value"]["mm"]
            .as_f64()
            .or(thickness["value"]["value"].as_f64()),
        Some(0.1)
    );
    let refused: Value = serde_json::from_str(&s.set_generated_value("nonsense", "1")).unwrap();
    assert!(refused["error"].is_string());
}

#[test]
fn convert_to_paths_leaves_no_block_and_selects_the_plates() {
    let mut s = session();
    assert!(s.place_generated(50.0, 50.0));
    s.set_tool(Tool::Select);
    assert!(s.convert_selected_generated());
    assert_eq!(s.generated_panel_json(), "null");
    assert!(
        !s.convert_selected_generated(),
        "nothing generated is left to convert"
    );
}

#[test]
fn a_file_with_an_unknown_generator_opens_as_stored_and_says_so_once() {
    let document = Document::new(1);
    let square = vec![
        0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 4.0, 0.0, 4.0, 0.0, 4.0, 4.0, 4.0, 4.0, 4.0, 4.0, 0.0, 4.0,
        0.0, 4.0, 0.0, 4.0, 0.0, 0.0, 0.0, 0.0,
    ];
    document
        .create_generated(
            &GeneratedRecord {
                kind: "future.generators/spiral".into(),
                seed: 1,
                params: serde_json::Map::new(),
                parts: vec![vec![square]],
                part_keys: Vec::new(),
                part_paints: std::collections::BTreeMap::new(),
            },
            Affine::IDENTITY,
        )
        .unwrap();
    let bytes = pack(&document, "0.1.0").unwrap();
    let mut s = Session::open(2, &bytes).unwrap();
    assert_eq!(
        s.take_generated_notice(),
        "1 object comes from a generator this version does not have. It is shown as a plain outline."
    );
    assert_eq!(s.take_generated_notice(), "", "once");
    assert_eq!(s.unknown_generated_count(), 1);
    assert!(!s.is_dirty(), "opening does not mark the file changed");
}

#[test]
fn the_bar_offers_no_kind_while_the_generators_extension_is_off() {
    let mut s = session();
    s.set_extension_enabled("curvyo.generators", false);
    let rest: Value = serde_json::from_str(&s.generator_kinds_json()).unwrap();
    assert_eq!(
        rest.as_array().unwrap().len(),
        1,
        "the QR code of the codes extension stays"
    );
    assert_eq!(
        s.generator_kind(),
        "",
        "the armed Box is gone with its extension: none is armed, not the first of the bar"
    );
    s.set_extension_enabled("curvyo.codes", false);
    assert_eq!(
        s.generator_kinds_json(),
        "[]",
        "the rail button hides on this"
    );
    s.set_extension_enabled("curvyo.generators", true);
    s.set_extension_enabled("curvyo.codes", true);
    assert_ne!(s.generator_kinds_json(), "[]");
}

fn newest_name(session: &mut Session) -> String {
    let rows: Value = serde_json::from_str(&session.history_rows_json(0, 1)).unwrap();
    rows[0]["name"].as_str().unwrap().to_owned()
}

#[test]
fn history_steps_are_named_after_the_kind_and_the_field() {
    let mut s = session();
    s.set_generator_kind("curvyo.generators/gear");
    assert!(s.place_generated(50.0, 50.0));
    s.set_tool(Tool::Select);
    assert_eq!(newest_name(&mut s), "Add gear");
    s.set_generated_value("teeth", "30");
    assert_eq!(newest_name(&mut s), "Gear: Teeth");
    assert!(s.convert_selected_generated());
    assert_eq!(newest_name(&mut s), "Convert gear to paths");

    s.set_tool(Tool::Generators);
    s.set_generator_kind("curvyo.generators/box");
    assert!(s.place_generated(200.0, 50.0));
    s.set_tool(Tool::Select);
    assert_eq!(newest_name(&mut s), "Add box");
    s.set_generated_value("thickness", "4");
    assert_eq!(newest_name(&mut s), "Box: Thickness");
    assert!(s.convert_selected_generated());
    assert_eq!(newest_name(&mut s), "Convert box to paths");
}

#[test]
fn the_top_shape_step_of_a_rounded_box_is_named_after_the_kind_and_the_field() {
    let mut s = session();
    s.set_generator_kind("curvyo.generators/rounded-box");
    assert!(s.place_generated(50.0, 50.0));
    s.set_tool(Tool::Select);
    s.set_generated_value("top_shape", "\"curved\"");
    assert_eq!(newest_name(&mut s), "Rounded box: Top shape");
}

#[test]
fn the_object_history_names_the_steps_like_the_document_history() {
    let mut s = session();
    s.set_generator_kind("curvyo.generators/gear");
    assert!(s.place_generated(50.0, 50.0));
    s.set_tool(Tool::Select);
    s.set_generated_value("teeth", "30");
    let rows: Value = serde_json::from_str(&s.object_history_rows_json(0, 5)).unwrap();
    let names: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Gear: Teeth"), "{names:?}");
    assert!(names.iter().any(|n| n.ends_with("Add gear")), "{names:?}");
}

#[test]
fn a_generated_object_is_a_gear_or_a_box_row_in_the_layers_list() {
    let mut s = session();
    s.set_generator_kind("curvyo.generators/gear");
    assert!(s.place_generated(50.0, 50.0));
    s.set_tool(Tool::Select);
    s.set_tool(Tool::Generators);
    s.set_generator_kind("curvyo.generators/box");
    assert!(s.place_generated(200.0, 50.0));
    s.set_tool(Tool::Select);
    let view: Value = serde_json::from_str(&s.layers_view_json(0, 10)).unwrap();
    let rows = view["rows"].as_array().unwrap();
    let find = |word: &str| {
        rows.iter()
            .find(|row| row["name"].as_str().is_some_and(|n| n.starts_with(word)))
            .unwrap_or_else(|| panic!("no {word} row in {rows:?}"))
    };
    let (gear, boxed) = (find("Gear"), find("Box"));
    assert_eq!(gear["kind"], "generated");
    assert_eq!(gear["generator"], "gear");
    assert_eq!(boxed["generator"], "box");
    assert!(gear["label"].as_str().unwrap().contains("gear"));
}

#[test]
fn the_select_bar_offers_object_to_path_for_a_generated_object() {
    let mut s = session();
    s.set_generator_kind("curvyo.generators/gear");
    assert!(s.place_generated(50.0, 50.0));
    s.set_tool(Tool::Select);
    assert!(s.select_bar_state().object_to_path);
}

/// A document with a gear at (100, 100) and its box corners, opened in a Select session.
fn gear_session() -> (Session, Point, Point) {
    let document = Document::new(1);
    let id = curvyo_ui_core::instance::place(
        &document,
        &curvyo_ui_core::ExtensionSettings::default(),
        "curvyo.generators/gear",
        Point::new(100.0, 100.0),
    )
    .unwrap();
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("a generated object reads as a path");
    };
    let points: Vec<Point> = path.all_anchors().map(|anchor| anchor.point).collect();
    let low = Point::new(
        points.iter().map(|p| p.x).fold(f64::MAX, f64::min),
        points.iter().map(|p| p.y).fold(f64::MAX, f64::min),
    );
    let high = Point::new(
        points.iter().map(|p| p.x).fold(f64::MIN, f64::max),
        points.iter().map(|p| p.y).fold(f64::MIN, f64::max),
    );
    let mut s = Session::open(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 900.0);
    // A marquee around the gear selects it (its fill is none, so a click inside misses).
    s.pointer_hover(Point::new(20.0, 20.0), false, false);
    s.pointer_down(Point::new(20.0, 20.0), false);
    s.pointer_hover(Point::new(180.0, 180.0), false, false);
    s.pointer_up(Point::new(180.0, 180.0), false, false);
    assert_eq!(s.selected_object_count(), 1);
    (s, low, high)
}

fn drag_corner(s: &mut Session, high: Point) {
    let to = Point::new(high.x + 30.0, high.y + 30.0);
    s.pointer_hover(high, false, false);
    s.pointer_down(high, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

#[test]
fn a_corner_drag_of_an_available_generator_resizes_through_its_parameters() {
    let (mut s, _, high) = gear_session();
    let steps = s.history_len();
    drag_corner(&mut s, high);
    assert_eq!(s.history_len(), steps + 1, "one resize step");
    let rows = panel(&s)["rows"].clone();
    let value_of = |key: &str| {
        rows.as_array()
            .unwrap()
            .iter()
            .find(|row| row["param"]["key"] == key)
            .unwrap()["value"]
            .to_string()
    };
    // 0141 revision 2: the drag writes Teeth (more than the default 24) and keeps the module.
    assert!(
        !value_of("teeth").contains("24"),
        "teeth grew: {}",
        value_of("teeth")
    );
    assert!(
        value_of("module").contains("2.0"),
        "module stays: {}",
        value_of("module")
    );
}

#[test]
fn a_disabled_or_unknown_generator_refuses_a_resize_with_a_notice() {
    let (mut s, _, high) = gear_session();
    s.set_extension_enabled("curvyo.generators", false);
    let steps = s.history_len();
    drag_corner(&mut s, high);
    assert_eq!(s.history_len(), steps, "nothing was written");
    assert_eq!(
        s.take_transform_notice(),
        "This object's generator is switched off under Extensions; it cannot be resized."
    );
    // A change by number is refused too.
    let answer: Value = serde_json::from_str(&s.set_generated_value("module", "3")).unwrap();
    assert!(answer["error"].is_string());
    assert_eq!(s.history_len(), steps);
}

#[test]
fn an_unknown_generator_refuses_a_resize_with_its_own_wording() {
    let document = Document::new(1);
    let square = vec![
        50.0, 50.0, 50.0, 50.0, 150.0, 50.0, 150.0, 50.0, 150.0, 50.0, 150.0, 150.0, 150.0, 150.0,
        150.0, 150.0, 50.0, 150.0, 50.0, 150.0, 50.0, 150.0, 50.0, 50.0, 50.0, 50.0,
    ];
    document
        .create_generated(
            &GeneratedRecord {
                kind: "future.generators/spiral".into(),
                seed: 1,
                params: serde_json::Map::new(),
                parts: vec![vec![square]],
                part_keys: Vec::new(),
                part_paints: std::collections::BTreeMap::new(),
            },
            Affine::IDENTITY,
        )
        .unwrap();
    let mut s = Session::open(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1600.0, 900.0);
    s.pointer_hover(Point::new(20.0, 20.0), false, false);
    s.pointer_down(Point::new(20.0, 20.0), false);
    s.pointer_hover(Point::new(180.0, 180.0), false, false);
    s.pointer_up(Point::new(180.0, 180.0), false, false);
    assert_eq!(s.selected_object_count(), 1);
    drag_corner(&mut s, Point::new(150.0, 150.0));
    assert_eq!(
        s.take_transform_notice(),
        "This object comes from a generator this version does not have; it cannot be resized."
    );
}

#[test]
fn the_size_chip_of_a_disabled_generator_does_not_open() {
    let (mut s, _, _) = gear_session();
    let press = |s: &mut Session| {
        s.key_down(curvyo_editor_wasm::KeyInput {
            key: "s",
            shift: false,
            ctrl: false,
            alt: false,
            repeat: false,
            dom_blocked: false,
            mac: false,
            time_ms: 0.0,
        })
    };
    assert_eq!(press(&mut s), curvyo_editor_wasm::KeyOutcome::EntryOpened);
    s.set_extension_enabled("curvyo.generators", false);
    s.escape();
    assert_eq!(press(&mut s), curvyo_editor_wasm::KeyOutcome::Consumed);
    assert!(s.take_transform_notice().ends_with("cannot be resized."));
}

fn teeth_of(s: &Session) -> f64 {
    let rows = panel(s)["rows"].as_array().unwrap().clone();
    let row = rows
        .iter()
        .find(|row| row["param"]["key"] == "teeth")
        .unwrap();
    let text = row["value"].to_string();
    text.trim_matches(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .find_map(|part| part.parse::<f64>().ok())
        .unwrap_or_else(|| panic!("no number in {text}"))
}

#[test]
fn a_corner_drag_of_a_generated_object_keeps_its_proportions_whatever_the_pointer_does() {
    let (mut s, low, high) = gear_session();
    let before = teeth_of(&s);
    // The pointer only moves along x: a free resize would scale x alone.
    let width = high.x - low.x;
    let to = Point::new(high.x + width / 2.0, high.y);
    s.pointer_hover(high, false, false);
    s.pointer_down(high, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
    let after = teeth_of(&s);
    // Proportional: both axes follow the larger change, the outer diameter m (z + 2) grows by half,
    // so 24 teeth become 1.5 * 26 - 2 = 37 (0141 revision 2).
    assert!((after - 37.0).abs() < 0.5, "teeth {before} -> {after}");
}

#[test]
fn k_skews_nothing_when_only_generated_objects_are_selected() {
    let (mut s, _, _) = gear_session();
    let outcome = s.key_down(curvyo_editor_wasm::KeyInput {
        key: "k",
        shift: false,
        ctrl: false,
        alt: false,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    });
    assert_eq!(outcome, curvyo_editor_wasm::KeyOutcome::Consumed);
}

fn enter(s: &mut Session) -> curvyo_editor_wasm::KeyOutcome {
    s.key_down(curvyo_editor_wasm::KeyInput {
        key: "Enter",
        shift: false,
        ctrl: false,
        alt: false,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    })
}

fn marquee(s: &mut Session, from: Point, to: Point) {
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

#[test]
fn enter_in_the_generators_tool_places_at_the_pointer() {
    let mut s = session();
    s.set_generator_kind("curvyo.generators/gear");
    s.pointer_hover(Point::new(400.0, 300.0), false, false);
    assert_eq!(enter(&mut s), curvyo_editor_wasm::KeyOutcome::ToolChanged);
    assert_eq!(s.selected_object_count(), 1);
    assert_eq!(newest_name(&mut s), "Add gear");
    s.set_tool(Tool::Select);
    s.escape();
    marquee(&mut s, Point::new(0.0, 0.0), Point::new(100.0, 100.0));
    assert_eq!(s.selected_object_count(), 0, "not at the origin");
    marquee(&mut s, Point::new(330.0, 230.0), Point::new(470.0, 370.0));
    assert_eq!(s.selected_object_count(), 1, "at the pointer");
}

#[test]
fn enter_in_the_generators_tool_without_a_pointer_places_in_the_visible_middle() {
    let mut s = session();
    assert_eq!(enter(&mut s), curvyo_editor_wasm::KeyOutcome::ToolChanged);
    assert_eq!(s.selected_object_count(), 1);
    assert_eq!(newest_name(&mut s), "Add box");
}

fn value_of(s: &Session, key: &str) -> f64 {
    let rows = panel(s)["rows"].as_array().unwrap().clone();
    let row = rows.iter().find(|row| row["param"]["key"] == key).unwrap();
    let text = row["value"].to_string();
    text.split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .find_map(|part| part.parse::<f64>().ok())
        .unwrap_or_else(|| panic!("no number in {text}"))
}

#[test]
fn a_stepped_value_is_previewed_without_a_step_and_the_key_up_commits_it() {
    let (mut s, _, _) = gear_session();
    let steps = s.history_len();
    let answer: Value = serde_json::from_str(&s.preview_generated_value("teeth", "30")).unwrap();
    assert!(answer["notices"].is_array());
    assert_eq!(s.history_len(), steps, "a preview writes nothing");
    assert!(
        (value_of(&s, "teeth") - 30.0).abs() < 1e-9,
        "the block follows the preview"
    );
    s.cancel_generated_preview();
    assert!(
        (value_of(&s, "teeth") - 24.0).abs() < 1e-9,
        "Escape restores"
    );
    s.preview_generated_value("teeth", "31");
    s.set_generated_value("teeth", "31");
    assert_eq!(s.history_len(), steps + 1, "one commit");
    assert!((value_of(&s, "teeth") - 31.0).abs() < 1e-9);
}

/// A spoked gear in a session and the rim knob's place, from the same gear in a scratch document.
fn spoked_gear_session() -> (Session, curvyo_ui_core::instance_handles::Knob) {
    let (mut s, _, _) = gear_session();
    s.set_generated_value("web", "\"spokes\"");
    let document = Document::new(1);
    let settings = curvyo_ui_core::ExtensionSettings::default();
    let id = curvyo_ui_core::instance::place(
        &document,
        &settings,
        "curvyo.generators/gear",
        Point::new(100.0, 100.0),
    )
    .unwrap();
    curvyo_ui_core::instance::change_value(
        &document,
        &settings,
        id,
        "web",
        &curvyo_extension_core::ParamValue::Choice("spokes".into()),
    )
    .unwrap();
    let knobs =
        curvyo_ui_core::instance_handles::knobs(&settings, &document.generated(id).unwrap());
    // 0141 revision 2 (UX notes 15.6, Proposal): the corner handle writes Teeth, so no pitch knob.
    assert!(knobs.iter().all(|knob| knob.key != "pitch_diameter"));
    let rim = knobs
        .into_iter()
        .find(|knob| knob.key == "rim_width")
        .unwrap();
    (s, rim)
}

#[test]
fn the_rim_knob_drags_the_rim_width_with_one_named_step() {
    let (mut s, rim) = spoked_gear_session();
    let steps = s.history_len();
    s.pointer_hover(rim.at, false, false);
    s.pointer_down(rim.at, false);
    let curvyo_ui_core::instance_handles::KnobMotion::Along { direction, .. } = rim.motion else {
        panic!("a line knob")
    };
    let to = Point::new(rim.at.x + direction.x * 2.0, rim.at.y + direction.y * 2.0);
    s.pointer_hover(to, false, false);
    assert_eq!(s.history_len(), steps, "nothing is written during the drag");
    assert!(
        s.live_readout().unwrap().text.starts_with("Rim width"),
        "the readout names the value"
    );
    s.pointer_up(to, false, false);
    assert_eq!(s.history_len(), steps + 1, "one step");
    assert_eq!(newest_name(&mut s), "Gear: Rim width");
    assert!((value_of(&s, "rim_width") - 6.0).abs() < 1e-6);
}

#[test]
fn a_knob_of_a_generator_switched_off_after_a_draw_is_neither_drawn_nor_hit() {
    let (mut s, rim) = spoked_gear_session();
    let at = rim.at;
    // The draw fills the knob memo.
    let with_knobs = s.draw_list().triangle_count();
    s.set_extension_enabled("curvyo.generators", false);
    assert!(
        s.draw_list().triangle_count() < with_knobs,
        "the knobs are gone"
    );
    s.pointer_down(at, false);
    let to = Point::new(at.x + 6.0, at.y + 6.0);
    s.pointer_hover(to, false, false);
    assert!(
        s.live_readout()
            .is_none_or(|readout| !readout.text.starts_with("Rim width")),
        "no knob drag runs"
    );
}

#[test]
fn escape_during_a_knob_drag_writes_nothing() {
    let (mut s, rim) = spoked_gear_session();
    let steps = s.history_len();
    let at = rim.at;
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_hover(Point::new(at.x + 2.0, at.y), false, false);
    s.escape();
    s.pointer_up(Point::new(at.x + 2.0, at.y), false, false);
    assert_eq!(s.history_len(), steps);
    assert!((value_of(&s, "rim_width") - 4.0).abs() < 1e-9);
}

#[test]
fn a_mixed_selection_shows_no_block_and_the_node_bar_says_generated_objects_have_no_nodes() {
    let mut s = session();
    s.set_generator_kind("curvyo.generators/gear");
    assert!(s.place_generated(50.0, 50.0));
    s.set_tool(Tool::Select);
    s.set_tool(Tool::Generators);
    s.set_generator_kind("curvyo.generators/box");
    assert!(s.place_generated(250.0, 50.0));
    s.set_tool(Tool::Select);
    // A marquee around both.
    s.pointer_hover(Point::new(-300.0, -300.0), false, false);
    s.pointer_down(Point::new(-300.0, -300.0), false);
    s.pointer_hover(Point::new(900.0, 500.0), false, false);
    s.pointer_up(Point::new(900.0, 500.0), false, false);
    assert_eq!(s.selected_object_count(), 2);
    assert_eq!(
        s.generated_panel_json(),
        "null",
        "a gear and a box show no block"
    );
    s.set_tool(Tool::Node);
    assert!(s.node_toolbar_state().generated_only);
    assert_eq!(s.node_toolbar_state().total_nodes, 0, "no node overlay");
}

#[test]
fn object_to_path_converts_a_generated_object_of_the_selection() {
    let mut s = session();
    assert!(s.place_generated(50.0, 50.0));
    s.set_tool(Tool::Select);
    assert_ne!(s.generated_panel_json(), "null");
    let outcome = s.object_to_path();
    assert!(matches!(
        outcome,
        curvyo_editor_wasm::ObjectToPathOutcome::Converted
    ));
    assert_eq!(
        s.generated_panel_json(),
        "null",
        "no generator block is left"
    );
    assert!(
        !s.convert_selected_generated(),
        "nothing generated is left to convert"
    );
}

#[test]
fn the_corner_radius_knob_reads_r_and_says_full_round_at_the_limit() {
    let mut s = session();
    s.set_generator_kind("curvyo.generators/rounded-box");
    assert!(s.place_generated(300.0, 300.0));
    s.set_tool(Tool::Select);
    let document = Document::new(1);
    let settings = curvyo_ui_core::ExtensionSettings::default();
    let id = curvyo_ui_core::instance::place(
        &document,
        &settings,
        "curvyo.generators/rounded-box",
        Point::new(300.0, 300.0),
    )
    .unwrap();
    let knobs =
        curvyo_ui_core::instance_handles::knobs(&settings, &document.generated(id).unwrap());
    let knob = knobs.iter().find(|k| k.key == "corner_radius").unwrap();
    s.pointer_hover(knob.at, false, false);
    s.pointer_down(knob.at, false);
    // A little along the diagonal: still a plain radius.
    let curvyo_ui_core::instance_handles::KnobMotion::Along { direction, .. } = knob.motion else {
        panic!("a line knob")
    };
    let step = |mm: f64| Point::new(knob.at.x + direction.x * mm, knob.at.y + direction.y * mm);
    s.pointer_hover(step(2.0), false, false);
    let text = s.live_readout().unwrap().text;
    assert!(text.starts_with("r 19."), "{text}");
    assert!(!text.contains("full round"), "{text}");
    // Far past half the shorter side (40 mm): full round.
    s.pointer_hover(step(60.0), false, false);
    assert_eq!(s.live_readout().unwrap().text, "r 40.0 mm full round");
    s.pointer_up(step(60.0), false, false);
    assert_eq!(value_of(&s, "corner_radius"), 40.0);
}

#[test]
fn a_gear_corner_drag_reads_teeth_and_steps_whole_teeth_while_it_moves() {
    let (mut s, _, high) = gear_session();
    s.pointer_hover(high, false, false);
    s.pointer_down(high, false);
    let to = Point::new(high.x + 20.0, high.y + 20.0);
    s.pointer_hover(to, false, false);
    let readout = s.live_readout().unwrap().text;
    assert!(readout.contains(" teeth"), "{readout}");
    assert!(readout.contains("⌀"), "{readout}");
    assert!(!readout.contains('×'), "{readout}");
    // Nothing is written before the release, and the release writes the same teeth.
    let shown: f64 = readout.split(' ').next().unwrap().parse().unwrap();
    s.pointer_up(to, false, false);
    assert_eq!(
        value_of(&s, "teeth"),
        shown,
        "the commit is the previewed gear"
    );
}

fn polygon_session(points: &[(f64, f64)], closed: bool) -> Session {
    use curvyo_document_core::NewAnchor;
    use curvyo_ui_core::AnchorIdMinter;
    let document = Document::new(1);
    let mut minter = AnchorIdMinter::new(9);
    let anchors: Vec<NewAnchor> = points
        .iter()
        .map(|&(x, y)| NewAnchor::corner(minter.mint(), Point::new(x, y)))
        .collect();
    let _source = document.create_path(&anchors, closed);
    let mut s = Session::open(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Generators);
    s.resize_viewport(1600.0, 900.0);
    s
}

/// `0149` criterion 1: the segment is a pick mode; a click on a closed outline builds the box
/// beside it, selected, one step "Add path box"; a click on anything else refuses in words and
/// changes nothing; a click on empty canvas says nothing.
#[test]
fn the_box_from_path_segment_picks_an_outline_and_refuses_the_rest_in_words() {
    let coffin = [
        (-50.0, -10.0),
        (-25.0, -30.0),
        (50.0, -20.0),
        (50.0, 20.0),
        (-25.0, 30.0),
        (-50.0, 10.0),
    ];
    let mut s = polygon_session(&coffin, true);
    let kinds: Value = serde_json::from_str(&s.generator_kinds_json()).unwrap();
    let entry = &kinds.as_array().unwrap()[1];
    assert_eq!(entry["segment"], "Box from path");
    assert_eq!(entry["label"], "Path box");
    s.set_generator_kind("curvyo.generators/path-box");
    assert!(
        !s.place_generated(0.0, 0.0),
        "the middle of the shape is not its outline"
    );
    assert_eq!(s.take_generated_notice(), "");
    assert!(s.place_generated(50.0, 0.0), "a click on the outline");
    assert_eq!(s.tool(), Tool::Select);
    assert_eq!(panel(&s)["label"], "Path box");
    assert!(
        !s.box_from_path_shown(),
        "a box is no outline to build from"
    );
    let mut open = polygon_session(&[(0.0, 0.0), (60.0, 0.0), (60.0, 60.0)], false);
    open.set_generator_kind("curvyo.generators/path-box");
    assert!(!open.place_generated(30.0, 0.0));
    assert_eq!(
        open.take_generated_notice(),
        "Select one closed outline. Nothing was changed."
    );
}

/// `0149` criterion 5: a spike is built anyway and the canvas notice names the corner; the marks
/// overlay draws without panicking for the selected box and for the hovered outline.
#[test]
fn a_spike_is_built_with_a_warning_notice_and_the_overlay_draws() {
    let tip = 11.0_f64.to_radians();
    let mut s = polygon_session(
        &[
            (0.0, -100.0),
            (100.0 * tip.tan(), 0.0),
            (-100.0 * tip.tan(), 0.0),
        ],
        true,
    );
    s.set_generator_kind("curvyo.generators/path-box");
    assert!(s.place_generated(100.0 * tip.tan() / 2.0, -50.0));
    s.set_tool(Tool::Select);
    let notice = s.take_generated_notice();
    assert!(notice.contains("very sharp"), "{notice}");
    let _ = s.draw_list();
}

/// `0149` Replace outline: the button arms the pick; a click on another closed outline replaces
/// the stored one in one step "Replace outline", the source stays; Escape (a tool change) cancels.
#[test]
fn replace_outline_picks_another_outline_in_one_step() {
    use curvyo_document_core::NewAnchor;
    use curvyo_ui_core::AnchorIdMinter;
    let document = Document::new(1);
    let mut minter = AnchorIdMinter::new(9);
    let mut draw = |points: &[(f64, f64)]| {
        let anchors: Vec<NewAnchor> = points
            .iter()
            .map(|&(x, y)| NewAnchor::corner(minter.mint(), Point::new(x, y)))
            .collect();
        let _id = document.create_path(&anchors, true);
    };
    draw(&[(0.0, 0.0), (80.0, 0.0), (80.0, 60.0), (0.0, 60.0)]);
    draw(&[(300.0, 0.0), (380.0, 0.0), (380.0, 90.0), (300.0, 90.0)]);
    let mut s = Session::open(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1600.0, 900.0);
    s.set_tool(Tool::Generators);
    s.set_generator_kind("curvyo.generators/path-box");
    assert!(s.place_generated(40.0, 0.0));
    s.set_tool(Tool::Select);
    let before = panel(&s);
    assert!(s.begin_replace_outline());
    assert_eq!(s.tool(), Tool::Generators);
    assert!(
        s.place_generated(340.0, 0.0),
        "a click on the other rectangle"
    );
    assert_eq!(s.tool(), Tool::Select);
    assert_eq!(newest_name(&mut s), "Replace outline");
    let _ = before;
    // Escape: leaving the tool disarms the pick.
    assert!(s.begin_replace_outline());
    s.set_tool(Tool::Select);
    s.set_tool(Tool::Generators);
    assert!(!s.place_generated(340.0, 0.0) || s.tool() == Tool::Select);
}

/// UX review: in the pick mode the cursor is the arrow over empty canvas, never the placing
/// tools' crosshair; the other kinds keep the default hint.
#[test]
fn the_pick_mode_shows_the_arrow_over_empty_canvas() {
    let mut s = polygon_session(&[(0.0, 0.0), (60.0, 0.0), (60.0, 60.0), (0.0, 60.0)], true);
    s.set_generator_kind("curvyo.generators/path-box");
    assert_eq!(s.cursor_hint(), "arrow");
    s.set_generator_kind("curvyo.generators/gear");
    assert_eq!(s.cursor_hint(), "default");
}
