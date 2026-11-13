//! Independent acceptance tests for `specs/0098-page-kinds` through the public session: Add page
//! of each kind (criteria 1 to 4), the kind chip data (6), the workbench and tabs derived from the
//! active page (7 to 9), the laser page as a normal page (10, 11, 13, 16, 17), placements from
//! older files (12), the profile pick and the work-area edit (14, 15), the extension switched
//! off and the unknown kind (18), opening a format-36 file (19) and the activation budget (21).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::case_sensitive_file_extension_comparisons,
    missing_docs
)]

mod cpu_time;
mod operation_shims;
use operation_shims::{OperationsViewMode, set_view};

use std::path::PathBuf;

use curvyo_document_core::{
    Document, DocumentSize, Length, NewOutputPage, PageKind, Point, RectBounds, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, SvgExportResult, Tool, Workbench};
use curvyo_laser_core::{
    MachineProfile, OriginCorner, new_profile_id, record_file_name, write_record,
};
use curvyo_ui_core::{FontBlock, PanelContent, SvgExportOptions, TextInput};
use serde_json::Value;

const LASER: &str = "curvyo.laser";

// ---- helpers ------------------------------------------------------------------------------------

fn core_fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn session() -> Session {
    let d = Document::new(1);
    let mut s = Session::open(7, &pack(&d, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1200.0, 800.0);
    s
}

fn open(bytes: &[u8]) -> Session {
    let mut s = Session::open(7, bytes).unwrap();
    s.resize_viewport(1200.0, 800.0);
    s
}

fn rows(s: &Session) -> Vec<Value> {
    serde_json::from_str(&s.pages_json()).unwrap()
}

fn row(s: &Session, id: &str) -> Value {
    rows(s).into_iter().find(|r| r["id"] == id).unwrap()
}

fn wb(s: &Session) -> Value {
    serde_json::from_str(&s.workbenches_json()).unwrap()
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.manufacturing_view_json()).unwrap()
}

fn tabs(s: &Session) -> Vec<&'static str> {
    s.panel_view().tabs.iter().map(|t| t.name).collect()
}

fn open_tab(s: &Session) -> &'static str {
    s.panel_view()
        .tabs
        .iter()
        .find(|t| t.selected)
        .map_or("", |t| t.name)
}

fn last_step_name(s: &mut Session) -> String {
    let rows: Vec<Value> = serde_json::from_str(&s.history_rows_json(0, 1)).unwrap();
    rows[0]["name"].as_str().unwrap().to_owned()
}

fn key(s: &mut Session, k: &str, ctrl: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    })
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn origin(r: &Value) -> Point {
    Point::new(r["x"].as_f64().unwrap(), r["y"].as_f64().unwrap())
}

fn opened(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn profile(name: &str, origin: OriginCorner, w: f64, h: f64, seed: u8) -> MachineProfile {
    let mut p = MachineProfile::generic_laser();
    p.id = new_profile_id([seed; 16]);
    name.clone_into(&mut p.values.name);
    p.values.origin = origin;
    p.work_area = DocumentSize::from_mm(w, h);
    p
}

fn library(profiles: &[MachineProfile]) -> String {
    let entries: Vec<Value> = profiles
        .iter()
        .map(|p| serde_json::json!({"name": record_file_name(&p.id), "text": write_record(p)}))
        .collect();
    Value::from(entries).to_string()
}

const VECTOR_TABS: [&str; 9] = [
    "file", "document", "style", "layers", "history", "fonts", "snapping", "grid", "share",
];

// ---- criteria 1 to 4: Add page ---------------------------------------------------------------

#[test]
fn ac1_the_kind_list_is_drawing_sketch_then_laser_with_glyph_and_word() {
    let s = session();
    let kinds = wb(&s)["kinds"].clone();
    let got: Vec<(String, String)> = kinds
        .as_array()
        .unwrap()
        .iter()
        .map(|k| {
            (
                k["label"].as_str().unwrap().into(),
                k["icon"].as_str().unwrap().into(),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            ("Drawing".into(), "PenTool".into()),
            ("Sketch".into(), "DraftingCompass".into()),
            ("Laser".into(), "Zap".into()),
        ]
    );
}

#[test]
fn ac2_to_4_add_page_of_each_kind_activates_it_and_is_one_named_undo_step() {
    let mut s = session();
    for (kind, name, step, word) in [
        ("drawing", "Page 2", "Add page", "Drawing"),
        ("sketch", "Sketch 1", "Add sketch page", "Sketch"),
        (LASER, "Laser 1", "Add laser page", "Laser"),
    ] {
        let before = rows(&s).len();
        let steps = s.undo_len();
        let id = s.add_page_of_kind(kind, None).unwrap();
        assert_eq!(s.undo_len(), steps + 1, "{kind}");
        assert_eq!(last_step_name(&mut s), step);
        assert_eq!(s.active_page().id, id, "{kind} is active");
        let r = row(&s, &id);
        assert_eq!(r["name"], name);
        assert_eq!(r["kindWord"], word);
        assert_eq!(r["active"], true);
        let _ = s.undo();
        assert_eq!(rows(&s).len(), before, "undo removes the {kind} page");
        let id = s.add_page_of_kind(kind, None).unwrap();
        assert_eq!(row(&s, &id)["name"], name);
    }
}

#[test]
fn ac4_laser_with_a_picked_profile_takes_its_work_area_and_generic_is_400_by_300() {
    let mut s = session();
    let big = profile("Big bed", OriginCorner::BottomLeft, 600.0, 400.0, 3);
    s.load_machine_profiles_json(&library(std::slice::from_ref(&big)));
    let g = s.add_page_of_kind(LASER, None).unwrap();
    assert_eq!(row(&s, &g)["size"], "400.0 × 300.0 mm");
    assert_eq!(row(&s, &g)["machine"], "Generic laser");
    let b = s.add_page_of_kind(LASER, Some(&big.id)).unwrap();
    assert_eq!(row(&s, &b)["size"], "600.0 × 400.0 mm");
    assert_eq!(row(&s, &b)["machine"], "Big bed");
    assert_eq!(view(&s)["machine"]["copy"]["width"], 600.0);
    assert!(
        view(&s)["fixtures"].as_array().unwrap().is_empty(),
        "no fixture"
    );
    let steps = s.undo_len();
    let _ = s.undo();
    assert_eq!(s.undo_len(), steps - 1);
    assert!(
        opened(&s).manufacturings().len() == 1,
        "one undo removes page and machine data"
    );
}

#[test]
fn ac2_a_refused_kind_adds_nothing() {
    let mut s = session();
    let steps = s.undo_len();
    assert!(s.add_page_of_kind("template", None).is_none());
    assert!(s.add_page_of_kind("", None).is_none());
    assert_eq!(s.undo_len(), steps);
    assert_eq!(rows(&s).len(), 1);
}

// ---- criterion 6: the kind is visible --------------------------------------------------------

#[test]
fn ac6_rows_page_chip_and_status_carry_the_kind() {
    let mut s = session();
    assert_eq!(row(&s, "first")["kindWord"], "Drawing");
    assert_eq!(row(&s, "first")["kindIcon"], "PenTool");
    assert_eq!(wb(&s)["page"]["word"], "Drawing");
    let sk = s.add_page_of_kind("sketch", None).unwrap();
    assert_eq!(row(&s, &sk)["kindIcon"], "DraftingCompass");
    assert_eq!(wb(&s)["page"]["word"], "Sketch");
    let l = s.add_page_of_kind(LASER, None).unwrap();
    assert_eq!(row(&s, &l)["kindIcon"], "Zap");
    assert_eq!(row(&s, &l)["kindWord"], "Laser");
    assert_eq!(wb(&s)["page"]["word"], "Laser");
    assert_eq!(view(&s)["status"], "Generic laser · 400.0 × 300.0 mm");
    assert_eq!(s.size_text(), "400.0 × 300.0 mm");
    assert!(s.activate_page("first"));
    assert_eq!(s.size_text(), "210.0 × 297.0 mm");
}

// ---- criteria 7 to 9: the tools follow the page ------------------------------------------------

#[test]
fn ac8_tools_and_tabs_follow_the_kind_and_activation_is_no_step() {
    let mut s = session();
    let sk = s.add_page_of_kind("sketch", None).unwrap();
    let l = s.add_page_of_kind(LASER, None).unwrap();
    let steps = s.undo_len();
    let dirty = s.is_dirty();
    assert!(s.activate_page("first"));
    assert_eq!(s.workbench(), Workbench::Vector);
    assert_eq!(tabs(&s), VECTOR_TABS);
    assert!(s.activate_page(&l));
    assert!(matches!(s.workbench(), Workbench::Manufacturing(_)));
    // `specs/0101` criterion 1: a laser page with its extension on adds the Operations tab after
    // Document; the other tabs are the drawing tabs.
    let laser_tabs: Vec<_> = tabs(&s)
        .into_iter()
        .filter(|n| *n != "operations" && *n != "materials")
        .collect();
    assert_eq!(
        laser_tabs, VECTOR_TABS,
        "a laser page has the drawing tabs, no Laser tab"
    );
    assert!(s.activate_page(&sk));
    assert_eq!(s.workbench(), Workbench::Sketch);
    let t = tabs(&s);
    assert!(
        t.contains(&"sketches") && t.contains(&"parameters"),
        "{t:?}"
    );
    assert_eq!(s.undo_len(), steps, "activation is not a history step");
    assert_eq!(s.is_dirty(), dirty);
}

#[test]
fn ac8_a_press_inside_a_frame_activates_that_page() {
    let mut s = session();
    let sk = s.add_page_of_kind("sketch", None).unwrap();
    assert!(s.activate_page("first"));
    s.set_tool(Tool::Select);
    let o = origin(&row(&s, &sk));
    let steps = s.undo_len();
    click(&mut s, Point::new(o.x + 20.0, o.y + 20.0));
    assert_eq!(s.active_page().id, sk);
    assert_eq!(s.workbench(), Workbench::Sketch);
    assert_eq!(s.undo_len(), steps, "the press draws nothing");
}

#[test]
fn ac8_ctrl_page_down_and_up_walk_the_list_and_stop_at_the_ends() {
    let mut s = session();
    let sk = s.add_page_of_kind("sketch", None).unwrap();
    let l = s.add_page_of_kind(LASER, None).unwrap();
    assert!(s.reorder_page(&l, 0));
    // List order: laser, first, sketch.
    assert!(s.activate_page(&l));
    let steps = s.undo_len();
    assert_eq!(key(&mut s, "PageUp", true), KeyOutcome::Consumed);
    assert_eq!(s.active_page().id, l, "stops at the start, no wrap");
    key(&mut s, "PageDown", true);
    assert_eq!(s.active_page().id, "first");
    assert_eq!(s.workbench(), Workbench::Vector);
    key(&mut s, "PageDown", true);
    assert_eq!(s.active_page().id, sk);
    assert_eq!(s.workbench(), Workbench::Sketch);
    key(&mut s, "PageDown", true);
    assert_eq!(s.active_page().id, sk, "stops at the end");
    key(&mut s, "PageUp", true);
    assert_eq!(s.active_page().id, "first");
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac8_plain_page_down_does_not_change_the_page() {
    let mut s = session();
    s.add_page_of_kind("drawing", None).unwrap();
    assert!(s.activate_page("first"));
    key(&mut s, "PageDown", false);
    assert_eq!(s.active_page().id, "first");
}

#[test]
fn ac8_the_chord_does_nothing_in_a_text_field() {
    let mut s = session();
    s.add_page_of_kind("drawing", None).unwrap();
    assert!(s.activate_page("first"));
    s.key_down(KeyInput {
        key: "PageDown",
        ctrl: true,
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(s.active_page().id, "first");
}

#[test]
fn ac9_a_missing_tab_falls_back_to_the_last_tab_of_that_kind_else_document() {
    let mut s = session();
    let sk = s.add_page_of_kind("sketch", None).unwrap();
    let l = s.add_page_of_kind(LASER, None).unwrap();
    assert!(s.activate_page("first"));
    assert!(s.press_panel_tab("grid"));
    assert!(s.activate_page(&sk));
    assert!(s.press_panel_tab("sketches"));
    assert_eq!(s.panel_view().content, PanelContent::Sketches);
    assert!(s.activate_page("first"));
    assert_eq!(open_tab(&s), "grid", "last tab used on a drawing page");
    assert!(s.activate_page(&sk));
    assert!(s.press_panel_tab("parameters"));
    assert!(s.activate_page(&l));
    assert_eq!(open_tab(&s), "document", "no tab used on a laser page yet");
}

#[test]
fn ac9_sketches_falls_back_to_the_last_drawing_tab() {
    let mut s = session();
    let sk = s.add_page_of_kind("sketch", None).unwrap();
    assert!(s.activate_page("first"));
    assert!(s.press_panel_tab("grid"));
    assert!(s.activate_page(&sk));
    assert!(s.press_panel_tab("sketches"));
    assert!(s.activate_page("first"));
    assert_eq!(open_tab(&s), "grid");
}

#[test]
fn ac9_sketches_falls_back_to_document_on_a_fresh_drawing_kind() {
    let mut s = session();
    let sk = s.add_page_of_kind("sketch", None).unwrap();
    assert!(s.activate_page(&sk));
    assert!(s.press_panel_tab("sketches"));
    assert!(s.activate_page("first"));
    assert_eq!(open_tab(&s), "document");
}

#[test]
fn ac9_a_tab_both_kinds_have_stays_open() {
    let mut s = session();
    let sk = s.add_page_of_kind("sketch", None).unwrap();
    assert!(s.activate_page("first"));
    assert!(s.press_panel_tab("layers"));
    assert!(s.activate_page(&sk));
    assert_eq!(open_tab(&s), "layers");
}

// ---- criteria 10, 11: the laser page is a normal page ----------------------------------------

#[test]
fn ac11_a_rectangle_drawn_on_a_laser_page_is_created_selected_and_undone_as_on_a_drawing_page() {
    let mut s = session();
    let l = s.add_page_of_kind(LASER, None).unwrap();
    let o = origin(&row(&s, &l));
    let steps = s.undo_len();
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(o.x + 10.0, o.y + 10.0), false);
    s.pointer_up(Point::new(o.x + 50.0, o.y + 40.0), false, false);
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(s.selected_object_count(), 1, "selected");
    assert_eq!(s.active_page().id, l, "still on the laser page");
    let d = opened(&s);
    assert_eq!(d.read_objects().objects.len(), 1);
    assert!(
        d.manufacturings()[0].placements.is_empty(),
        "an own item, not machine data"
    );
    // Moving the laser page carries it (membership by position).
    assert!(s.move_page_to(&l, Point::new(o.x, o.y + 500.0)));
    let moved = opened(&s);
    let RectBoundsOf(min) = rect_min(&moved);
    assert!((min.y - (o.y + 510.0)).abs() < 1e-6, "{min:?}");
    let _ = s.undo();
    let _ = s.undo();
    assert!(opened(&s).read_objects().objects.is_empty());
}

struct RectBoundsOf(Point);

fn rect_min(d: &Document) -> RectBoundsOf {
    let o = &d.read_objects().objects[0];
    let curvyo_document_core::ObjectSnapshot::Primitive(p) = o else {
        panic!("a rectangle: {o:?}")
    };
    let anchors = curvyo_document_core::outline_of(&p.shape).anchors;
    let x = anchors.iter().map(|a| a.point.x).fold(f64::MAX, f64::min);
    let y = anchors.iter().map(|a| a.point.y).fold(f64::MAX, f64::min);
    RectBoundsOf(Point::new(x, y))
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn load_fonts(s: &mut Session) {
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes), "{file} loads");
    }
}

fn included_faces() -> Vec<(String, String, String, u32, curvyo_document_core::FontKind)> {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let kind = if f["kind"] == "single_line" {
                curvyo_document_core::FontKind::SingleLine
            } else {
                curvyo_document_core::FontKind::Outline
            };
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
                kind,
            )
        })
        .collect()
}

#[test]
fn ac11_a_text_made_with_the_text_tool_on_a_laser_page_is_created_and_undone() {
    let mut s = session();
    s.set_font_list_with_kinds(FontBlock::Included, included_faces(), Vec::new());
    load_fonts(&mut s);
    let l = s.add_page_of_kind(LASER, None).unwrap();
    let o = origin(&row(&s, &l));
    let steps = s.undo_len();
    s.set_tool(Tool::Text);
    click(&mut s, Point::new(o.x + 30.0, o.y + 30.0));
    assert!(s.text_editing(), "the Text tool works on a laser page");
    s.text_input(TextInput::Insert("Bed".to_owned()));
    load_fonts(&mut s);
    s.escape();
    assert!(s.undo_len() > steps);
    let d = opened(&s);
    assert!(
        d.read_objects()
            .objects
            .iter()
            .any(|o| matches!(o, curvyo_document_core::ObjectSnapshot::Text(_)))
    );
    while s.undo_len() > steps {
        let _ = s.undo();
    }
    assert!(opened(&s).read_objects().objects.is_empty());
}

#[test]
fn ac10_the_laser_page_can_be_renamed_and_reordered_from_the_list() {
    let mut s = session();
    let l = s.add_page_of_kind(LASER, None).unwrap();
    assert!(s.rename_page(&l, "Bed"));
    assert!(s.reorder_page(&l, 0));
    assert_eq!(rows(&s)[0]["name"], "Bed");
    assert_eq!(rows(&s)[0]["kindWord"], "Laser");
}

// ---- criterion 12: placements from older files ------------------------------------------------

#[test]
fn ac12_a_placement_of_an_older_file_can_be_selected_and_deleted_and_undo_restores_it() {
    let mut s = open(&core_fixture("laser_workbench_v36.curvyo"));
    let laser = rows(&s)
        .into_iter()
        .find(|r| r["kind"] == LASER)
        .expect("listed");
    let id = laser["id"].as_str().unwrap().to_owned();
    assert!(s.activate_page(&id));
    let m = opened(&s).manufacturings()[0].clone();
    let count = m.placements.len();
    assert!(count >= 1);
    // The placed page "first" holds a 10 mm square at (20, 0); press its inside on the laser page.
    let o = origin(&laser);
    s.set_tool(Tool::Select);
    click(&mut s, Point::new(o.x + 25.0, o.y + 5.0));
    let picked = s.selected_placement().expect("the placement is hit");
    // Amended by `specs/0100` criterion 17: the placed page is the one selection.
    assert_eq!(s.selected_object_count(), 1, "its inside is not selected");
    let steps = s.undo_len();
    assert!(s.delete_placement(picked));
    assert_eq!(s.undo_len(), steps + 1);
    let d = opened(&s);
    assert_eq!(d.manufacturings()[0].placements.len(), count - 1);
    assert!(
        d.pages().iter().any(|p| p.id == "first"),
        "only the reference goes"
    );
    assert_eq!(
        d.read_objects().objects.len(),
        opened(&open(&core_fixture("laser_workbench_v36.curvyo")))
            .read_objects()
            .objects
            .len()
    );
    let _ = s.undo();
    assert_eq!(opened(&s).manufacturings()[0].placements.len(), count);
}

// ---- criteria 13 to 15: Machine block and size from the profile ------------------------------

#[test]
fn ac13_the_laser_page_document_tab_has_the_machine_and_fixtures_and_size_is_fixed() {
    let mut s = session();
    let l = s.add_page_of_kind(LASER, None).unwrap();
    assert_eq!(row(&s, &l)["fixedSize"], true);
    assert_eq!(row(&s, "first")["fixedSize"], false);
    let v = view(&s);
    assert_eq!(v["machine"]["copy"]["width"], 400.0);
    assert!(v["fixtures"].is_array());
    assert!(!tabs(&s).contains(&"laser"), "no Laser tab");
}

#[test]
fn ac14_picking_a_bigger_profile_grows_the_page_away_from_the_origin_corner_in_one_step() {
    let mut s = session();
    let big = profile("Big bed", OriginCorner::BottomLeft, 600.0, 400.0, 3);
    s.load_machine_profiles_json(&library(std::slice::from_ref(&big)));
    let l = s.add_page_of_kind(LASER, None).unwrap();
    let o = origin(&row(&s, &l));
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(o.x + 10.0, o.y + 200.0), false);
    s.pointer_up(Point::new(o.x + 50.0, o.y + 240.0), false, false);
    let rect_before = rect_min(&opened(&s)).0;
    let steps = s.undo_len();
    assert!(s.pick_machine_profile(&big.id));
    assert_eq!(s.undo_len(), steps + 1, "one undo step");
    let r = row(&s, &l);
    assert_eq!(r["size"], "600.0 × 400.0 mm");
    let now = origin(&r);
    assert!((now.x - o.x).abs() < 1e-6, "left edge stays (bottom-left)");
    assert!((now.y - (o.y - 100.0)).abs() < 1e-6, "bottom edge stays");
    let rect_after = rect_min(&opened(&s)).0;
    assert_eq!(
        rect_after, rect_before,
        "own items keep their place in the plane"
    );
    // Distance to the origin corner (bottom-left) is unchanged.
    let corner_before = Point::new(o.x, o.y + 300.0);
    let corner_after = Point::new(now.x, now.y + 400.0);
    assert!((rect_after.x - corner_after.x - (rect_before.x - corner_before.x)).abs() < 1e-6);
    assert!((rect_after.y - corner_after.y - (rect_before.y - corner_before.y)).abs() < 1e-6);
    let _ = s.undo();
    assert_eq!(row(&s, &l)["size"], "400.0 × 300.0 mm");
    assert_eq!(origin(&row(&s, &l)), o);
}

#[test]
fn ac14_a_top_right_profile_keeps_the_top_right_corner() {
    let mut s = session();
    let tr = profile("Right bed", OriginCorner::TopRight, 600.0, 400.0, 4);
    s.load_machine_profiles_json(&library(std::slice::from_ref(&tr)));
    let l = s.add_page_of_kind(LASER, None).unwrap();
    let o = origin(&row(&s, &l));
    assert!(s.pick_machine_profile(&tr.id));
    let now = origin(&row(&s, &l));
    assert!((now.x - (o.x - 200.0)).abs() < 1e-6, "right edge stays");
    assert!((now.y - o.y).abs() < 1e-6, "top edge stays");
}

#[test]
fn ac15_a_work_area_edit_writes_the_library_not_the_project_and_reset_resizes() {
    let mut s = session();
    let user = profile("Garage", OriginCorner::BottomLeft, 500.0, 300.0, 5);
    s.load_machine_profiles_json(&library(std::slice::from_ref(&user)));
    let l = s.add_page_of_kind(LASER, Some(&user.id)).unwrap();
    let _ = s.take_machine_writes_json();
    let steps = s.undo_len();
    let before = opened(&s).pages();
    assert!(
        s.set_profile_work_area(&user.id, Length::from_mm(700.0), Length::from_mm(450.0), 1)
            .is_some()
    );
    assert_eq!(s.undo_len(), steps, "no history step");
    let w: Vec<Value> = serde_json::from_str(&s.take_machine_writes_json()).unwrap();
    assert!(!w.is_empty(), "a library write");
    assert_eq!(opened(&s).pages(), before, "the project is not written");
    assert!(
        view(&s)["machine"]["state"] == "differs",
        "{}",
        view(&s)["machine"]
    );
    let o = origin(&row(&s, &l));
    assert!(s.reset_machine_profile());
    assert_eq!(s.undo_len(), steps + 1, "Reset is one step");
    let r = row(&s, &l);
    assert_eq!(r["size"], "700.0 × 450.0 mm");
    assert!(
        (origin(&r).y - (o.y - 150.0)).abs() < 1e-6,
        "about the origin corner"
    );
}

#[test]
fn ac15_generic_laser_has_no_work_area_edit() {
    let mut s = session();
    s.add_page_of_kind(LASER, None).unwrap();
    assert!(
        s.set_profile_work_area(
            "generic-laser",
            Length::from_mm(700.0),
            Length::from_mm(450.0),
            1
        )
        .is_none()
    );
    let w: Vec<Value> = serde_json::from_str(&s.take_machine_writes_json()).unwrap();
    assert!(w.iter().all(|w| !w.to_string().contains("700")));
    let rows = view(&s)["machine"]["profiles"].clone();
    let generic = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "generic-laser")
        .unwrap();
    assert_eq!(generic["builtin"], true);
}

#[test]
fn ac15_a_work_area_edit_clamps_to_the_profile_range() {
    let mut s = session();
    let user = profile("Garage", OriginCorner::BottomLeft, 500.0, 300.0, 5);
    s.load_machine_profiles_json(&library(std::slice::from_ref(&user)));
    s.add_page_of_kind(LASER, Some(&user.id)).unwrap();
    let notice = s
        .set_profile_work_area(&user.id, Length::from_mm(500.0), Length::from_mm(1.0e9), 1)
        .unwrap();
    assert!(notice.is_some(), "a cut value tells the maker");
}

// ---- criterion 16: Delete page on a laser page -----------------------------------------------

#[test]
fn ac16_delete_page_on_a_laser_page_and_undo() {
    let mut s = session();
    let l = s.add_page_of_kind(LASER, None).unwrap();
    s.add_fixture("engraved_grid").unwrap();
    let o = origin(&row(&s, &l));
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(o.x + 10.0, o.y + 10.0), false);
    s.pointer_up(Point::new(o.x + 50.0, o.y + 40.0), false, false);
    let rect = rect_min(&opened(&s)).0;
    let steps = s.undo_len();
    assert!(s.delete_page(&l));
    assert_eq!(s.undo_len(), steps + 1);
    let d = opened(&s);
    assert_eq!(d.pages().len(), 1);
    assert!(d.manufacturings().is_empty());
    assert_eq!(
        rect_min(&d).0,
        rect,
        "the rectangle stays on the pasteboard"
    );
    assert_eq!(s.workbench(), Workbench::Vector);
    let _ = s.undo();
    let d = opened(&s);
    assert_eq!(d.pages().len(), 2);
    assert_eq!(d.manufacturings()[0].fixtures.len(), 1);
}

// ---- criterion 17: export ----------------------------------------------------------------------

#[test]
fn ac17_the_laser_page_is_a_page_of_the_svg_export_without_fixture_or_marker() {
    let mut s = session();
    let l = s.add_page_of_kind(LASER, None).unwrap();
    s.add_fixture("engraved_grid").unwrap();
    let o = origin(&row(&s, &l));
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(o.x + 10.0, o.y + 10.0), false);
    s.pointer_up(Point::new(o.x + 50.0, o.y + 40.0), false, false);
    let SvgExportResult::Written { text, objects, .. } = s.export_svg(SvgExportOptions::default())
    else {
        panic!("written")
    };
    assert_eq!(objects, 1, "only the rectangle: {text}");
    assert_eq!(text.matches("<inkscape:page ").count(), 2, "{text}");
    assert!(text.contains("inkscape:label=\"Laser 1\""), "{text}");
    assert!(
        text.contains("width=\"400\"") && text.contains("height=\"300\""),
        "{text}"
    );
    assert!(!text.contains("Work area"), "{text}");
    assert_eq!(
        text.matches("<line").count() + text.matches("<path").count(),
        0,
        "no grid or marker: {text}"
    );
}

#[test]
fn ac17_active_page_export_of_a_laser_page_has_its_rectangle() {
    let mut s = session();
    let l = s.add_page_of_kind(LASER, None).unwrap();
    let o = origin(&row(&s, &l));
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(o.x + 10.0, o.y + 10.0), false);
    s.pointer_up(Point::new(o.x + 50.0, o.y + 40.0), false, false);
    let SvgExportResult::Written { objects, text, .. } = s.export_svg(SvgExportOptions {
        active_page_only: true,
        ..SvgExportOptions::default()
    }) else {
        panic!("written")
    };
    assert_eq!(objects, 1, "{text}");
}

// ---- criterion 18: extension off, unknown kind -----------------------------------------------

#[test]
fn ac18b_laser_switched_off_keeps_pages_shown_and_data_unchanged() {
    let mut s = session();
    let l = s.add_page_of_kind(LASER, None).unwrap();
    s.add_fixture("engraved_grid").unwrap();
    let before = opened(&s).manufacturings();
    s.set_extension_enabled(LASER, false);
    let kinds: Vec<String> = wb(&s)["kinds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["label"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        kinds,
        ["Drawing", "Sketch"],
        "Laser absent from the Kind switch"
    );
    assert!(s.add_page_of_kind(LASER, None).is_none());
    let r = row(&s, &l);
    assert_eq!(r["kindWord"], "Laser (off)");
    assert_eq!(r["kindOff"], true);
    assert!(s.activate_page(&l));
    assert_eq!(s.workbench(), Workbench::Vector, "the drawing rail");
    assert_eq!(tabs(&s), VECTOR_TABS);
    let steps = s.undo_len();
    assert!(
        s.add_fixture("engraved_grid").is_none(),
        "no manufacturing write"
    );
    assert!(!s.reset_machine_profile());
    assert_eq!(s.undo_len(), steps);
    let after = opened(&s).manufacturings();
    assert_eq!(after, before, "saved back with the data unchanged");
    assert!(s.rename_page(&l, "Old bed"));
    assert!(s.reorder_page(&l, 0));
    assert_eq!(row(&s, &l)["fixedSize"], true, "cannot resize");
    assert!(s.delete_page(&l), "can delete");
}

fn unknown_kind_file() -> (Vec<u8>, String) {
    let d = Document::new(1);
    let p = d.add_page_of(&PageKind::Drawing).unwrap();
    let loro = loro::LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro.get_map("root")
        .insert(&format!("page.{}.kind", p.id), "template")
        .unwrap();
    loro.commit();
    let fresh = unpack(1, &pack(&d, "x").unwrap()).unwrap();
    drop(fresh);
    (
        repack(&loro, curvyo_document_core::CURRENT_FORMAT_VERSION),
        p.id,
    )
}

fn repack(loro: &loro::LoroDoc, version: u32) -> Vec<u8> {
    use std::io::Write;
    let manifest = serde_json::json!({
        "format_version": version,
        "loro_snapshot_version": curvyo_document_core::CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "0.1.0",
    });
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("manifest.json", options).unwrap();
    writer.write_all(manifest.to_string().as_bytes()).unwrap();
    writer.start_file("document.loro", options).unwrap();
    writer
        .write_all(&loro.export(loro::ExportMode::Snapshot).unwrap())
        .unwrap();
    writer.start_file("document.json", options).unwrap();
    writer.write_all(b"{}").unwrap();
    writer.finish().unwrap().into_inner()
}

#[test]
fn ac18c_d_an_unknown_kind_page_is_listed_with_the_vector_tools_and_kept() {
    let (bytes, id) = unknown_kind_file();
    let mut s = open(&bytes);
    let r = row(&s, &id);
    assert_eq!(r["kindWord"], "Unknown kind");
    assert_ne!(r["kindIcon"], "PenTool", "a neutral glyph");
    assert_ne!(r["kindIcon"], "Zap");
    assert_eq!(r["fixedSize"], true);
    assert!(s.activate_page(&id));
    assert_eq!(s.workbench(), Workbench::Vector);
    assert_eq!(tabs(&s), VECTOR_TABS);
    assert_eq!(wb(&s)["page"]["word"], "Unknown kind");
    assert!(s.rename_page(&id, "Kept"));
    assert!(s.reorder_page(&id, 0));
    let d = opened(&s);
    assert_eq!(
        d.pages().into_iter().find(|p| p.id == id).unwrap().kind,
        PageKind::Unknown("template".into())
    );
    assert!(s.delete_page(&id));
}

#[test]
fn ac18c_unknown_golden_opens_in_the_session() {
    let s = open(&core_fixture("page_kind_unknown_v37.curvyo"));
    assert!(rows(&s).iter().any(|r| r["kindWord"] == "Unknown kind"));
}

#[test]
fn ac20_damaged_golden_shows_a_drawing_page() {
    let s = open(&core_fixture("page_kind_damaged_v37.curvyo"));
    assert!(rows(&s).iter().all(|r| r["kindWord"] != "Unknown kind"));
    assert!(!s.is_dirty());
}

// ---- criterion 19: opening a format-36 file --------------------------------------------------

#[test]
fn ac19_opening_a_format_36_file_is_not_a_change_and_no_step() {
    for name in [
        "page_kinds_migration_v36.curvyo",
        "laser_workbench_v36.curvyo",
        "machine_profile_v36.curvyo",
        "sketch_page_owner_v29.curvyo",
    ] {
        let mut s = open(&core_fixture(name));
        assert!(!s.is_dirty(), "{name}: not marked changed");
        assert_eq!(s.undo_len(), 0, "{name}: no undo step");
        let history: Vec<Value> = serde_json::from_str(&s.history_rows_json(0, 20)).unwrap();
        assert!(
            history.iter().all(|r| r["label"] != "migrate_format"),
            "{name}: not listed in History: {history:?}"
        );
        assert_eq!(s.active_page().id, "first", "{name}: the active page stays");
    }
}

#[test]
fn ac19_the_converted_sketch_page_shows_the_sketch_tools() {
    let mut s = open(&core_fixture("page_kinds_migration_v36.curvyo"));
    let sk = rows(&s)
        .into_iter()
        .find(|r| r["kind"] == "sketch")
        .expect("a sketch page");
    assert_eq!(sk["name"], "Page 1 sketch");
    assert!(s.activate_page(sk["id"].as_str().unwrap()));
    assert_eq!(s.workbench(), Workbench::Sketch);
    assert!(s.activate_page("first"));
    assert_eq!(
        s.workbench(),
        Workbench::Vector,
        "the drawing page has no sketch tools"
    );
}

// ---- criterion 21: the activation budget -----------------------------------------------------

#[test]
fn ac21_activating_pages_of_alternating_kinds_stays_within_50_ms_cpu() {
    let d = Document::new(31);
    for index in 1..20 {
        match index % 3 {
            0 => {
                d.add_page_of(&PageKind::Drawing).unwrap();
            }
            1 => {
                d.add_page_of(&PageKind::Sketch).unwrap();
            }
            _ => {
                d.add_output_page(NewOutputPage {
                    process: LASER,
                    label: "Laser",
                    profile: None,
                    work_area: DocumentSize::from_mm(400.0, 300.0),
                })
                .unwrap();
            }
        }
    }
    let pages = d.pages();
    d.batch("scene", |doc| {
        for i in 0..1_000_u32 {
            let p = &pages[i as usize % pages.len()];
            let x = p.origin.x + 2.0 + f64::from(i % 25) * 6.0;
            let y = p.origin.y + 2.0 + f64::from(i / 25 % 40) * 6.0;
            let _ = doc.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 3.0, y + 3.0),
            ));
        }
    });
    let mut s = open(&pack(&d, "budget").unwrap());
    let _ = s.frame_draw_list();
    let by_kind = |k: fn(&PageKind) -> bool| -> Vec<String> {
        pages
            .iter()
            .filter(|p| k(&p.kind))
            .map(|p| p.id.clone())
            .collect()
    };
    let drawing = by_kind(|k| *k == PageKind::Drawing);
    let sketch = by_kind(|k| *k == PageKind::Sketch);
    let laser = by_kind(|k| matches!(k, PageKind::Output { .. }));
    let mut worst = std::time::Duration::ZERO;
    for round in 0..50 {
        let set = [&drawing, &sketch, &laser][round % 3];
        let id = set[round % set.len()].clone();
        let ((), cost) = cpu_time::cpu_time_of(|| {
            assert!(s.activate_page(&id));
            let _ = s.panel_view();
            let _ = s.pages_json();
            let _ = s.workbenches_json();
            let _ = s.size_text();
        });
        worst = worst.max(cost);
    }
    eprintln!("worst activation: {worst:?}");
    if !cfg!(debug_assertions) {
        assert!(worst.as_secs_f64() <= 0.050, "worst {worst:?}");
    }
}

#[test]
// Amended by `specs/0100-bring-content-in` criterion 17: a drag moves a placed page now.
fn ac12_a_placement_is_moved_by_a_drag_since_0100() {
    let mut s = open(&core_fixture("laser_workbench_v36.curvyo"));
    let laser = rows(&s).into_iter().find(|r| r["kind"] == LASER).unwrap();
    assert!(s.activate_page(laser["id"].as_str().unwrap()));
    let before = opened(&s).manufacturings()[0].placements.clone();
    let o = origin(&laser);
    s.set_tool(Tool::Select);
    let a = Point::new(o.x + 25.0, o.y + 5.0);
    let b = Point::new(o.x + 125.0, o.y + 105.0);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
    let after = opened(&s).manufacturings()[0].placements.clone();
    assert_eq!(after.len(), before.len());
    assert_ne!(after, before, "the pressed placement moved");
}

// ---- accepted `0093` criteria 6, 8, 9, 13 kept by criterion 12: placements of older files ------

type Box4 = (f64, f64, f64, f64);

fn is_red(v: &curvyo_render_core::Vertex) -> bool {
    v.color.r > 200 && v.color.g < 60 && v.color.b < 60 && v.color.a > 0
}

/// The box of the red vertices right of `min_x` (the laser page's side of the plane).
fn red_box_right_of(s: &Session, min_x: f64) -> Option<Box4> {
    s.frame_draw_list()
        .triangles
        .iter()
        .filter(|v| is_red(v) && v.position.x >= min_x - 0.5)
        .fold(None, |acc, v| {
            let (x, y) = (v.position.x, v.position.y);
            Some(match acc {
                None => (x, y, x, y),
                Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
            })
        })
}

fn close4(a: Box4, b: Box4, what: &str) {
    for (x, y) in [(a.0, b.0), (a.1, b.1), (a.2, b.2), (a.3, b.3)] {
        assert!((x - y).abs() < 0.05, "{what}: {a:?} vs {b:?}");
    }
}

/// A red rectangle `a`..`b` on page 1, a second drawing page, and a laser page that places page 1
/// with the identity matrix (as a format-36 file holds it); the laser page's origin.
fn placed_file(a: (f64, f64), b: (f64, f64)) -> (Vec<u8>, String, Point) {
    use curvyo_document_core::{Color, StyleEdit};
    let d = Document::new(1);
    let id = d.create_rect(RectBounds::from_corners(
        Point::new(a.0, a.1),
        Point::new(b.0, b.1),
    ));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[id], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::StrokeEnabled(false))
        .unwrap();
    d.add_page_of(&PageKind::Drawing).unwrap();
    let page = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let loro = loro::LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = loro.get_tree("paths");
    let node = tree
        .children(loro::TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
        .unwrap();
    let child = tree.create(loro::TreeParentId::Node(node)).unwrap();
    let meta = tree.get_meta(child).unwrap();
    meta.insert("shape", "placement").unwrap();
    let matrix: Vec<loro::LoroValue> = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        .into_iter()
        .map(loro::LoroValue::Double)
        .collect();
    let link: std::collections::HashMap<String, loro::LoroValue> = [
        ("page".to_owned(), loro::LoroValue::from("first")),
        ("matrix".to_owned(), loro::LoroValue::from(matrix)),
    ]
    .into();
    meta.insert("placement_link", loro::LoroValue::from(link))
        .unwrap();
    loro.commit();
    (
        repack(&loro, curvyo_document_core::CURRENT_FORMAT_VERSION),
        page.id,
        page.origin,
    )
}

#[test]
fn ac12_0093_6_a_drawing_edit_shows_live_in_the_placed_page() {
    let (bytes, laser, o) = placed_file((10.0, 20.0), (50.0, 40.0));
    let mut s = open(&bytes);
    assert!(s.activate_page(&laser));
    // `0101` criterion 14, 18: unassigned placed content is in the Not assigned look under "View:
    // Operations"; these read its own red under "View: Drawn".
    set_view(&mut s, OperationsViewMode::Drawn);
    close4(
        red_box_right_of(&s, o.x).expect("the placed page is drawn"),
        (o.x + 10.0, o.y + 20.0, o.x + 50.0, o.y + 40.0),
        "placed at the identity",
    );
    assert!(s.activate_page("first"));
    s.set_tool(Tool::Select);
    let a = Point::new(30.0, 30.0);
    let b = Point::new(60.0, 45.0);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
    assert!(s.activate_page(&laser));
    close4(
        red_box_right_of(&s, o.x).unwrap(),
        (o.x + 40.0, o.y + 35.0, o.x + 80.0, o.y + 55.0),
        "follows the move without a command",
    );
    let _ = s.undo();
    close4(
        red_box_right_of(&s, o.x).unwrap(),
        (o.x + 10.0, o.y + 20.0, o.x + 50.0, o.y + 40.0),
        "an undo shows at once",
    );
}

#[test]
fn ac12_0093_8_content_past_the_work_area_is_drawn_whole_and_named() {
    let (bytes, laser, o) = placed_file((0.0, 0.0), (440.0, 312.0));
    let mut s = open(&bytes);
    assert!(s.activate_page(&laser));
    // `0101` criterion 14, 18: unassigned placed content is in the Not assigned look under "View:
    // Operations"; these read its own red under "View: Drawn".
    set_view(&mut s, OperationsViewMode::Drawn);
    close4(
        red_box_right_of(&s, o.x).unwrap(),
        (o.x, o.y, o.x + 440.0, o.y + 312.0),
        "not clipped",
    );
    assert_eq!(
        view(&s)["notices"]["overhang"],
        "Extends past the work area by 40 x 12 mm"
    );
}

#[test]
fn ac12_0093_8_within_the_work_area_no_notice() {
    let (bytes, laser, _) = placed_file((10.0, 20.0), (50.0, 40.0));
    let mut s = open(&bytes);
    assert!(s.activate_page(&laser));
    assert!(view(&s)["notices"].get("overhang").is_none());
}

#[test]
fn ac12_0093_9_grid_lines_are_drawn_below_the_placed_drawing() {
    let (bytes, laser, o) = placed_file((10.0, 20.0), (50.0, 40.0));
    let mut s = open(&bytes);
    assert!(s.activate_page(&laser));
    // `0101` criterion 14, 18: unassigned placed content is in the Not assigned look under "View:
    // Operations"; these read its own red under "View: Drawn".
    set_view(&mut s, OperationsViewMode::Drawn);
    let first_red = |s: &Session| {
        s.frame_draw_list()
            .triangles
            .iter()
            .position(|v| is_red(v) && v.position.x >= o.x - 0.5)
            .unwrap()
    };
    let plain = s.frame_draw_list().triangles.len();
    let red_plain = first_red(&s);
    s.add_fixture("engraved_grid").unwrap();
    // `0102` criterion 2: a new fixture is selected; its box is drawn over everything.
    s.escape();
    let extra = s.frame_draw_list().triangles.len() - plain;
    assert!(extra > 0, "the grid draws lines");
    assert!(
        first_red(&s) >= red_plain + extra,
        "grid vertices come before (below) the placed drawing"
    );
}

#[test]
// Amended by `specs/0100-bring-content-in` criterion 24: the placement goes with its page.
fn ac12_0093_13_deleting_the_placed_drawing_page_removes_the_placement_and_undo_restores() {
    let (bytes, laser, o) = placed_file((10.0, 20.0), (50.0, 40.0));
    let mut s = open(&bytes);
    assert!(s.delete_page("first"));
    assert!(s.activate_page(&laser));
    // `0101` criterion 14, 18: unassigned placed content is in the Not assigned look under "View:
    // Operations"; these read its own red under "View: Drawn".
    set_view(&mut s, OperationsViewMode::Drawn);
    assert!(red_box_right_of(&s, o.x).is_none(), "draws nothing");
    assert_eq!(
        opened(&s).manufacturings()[0].placements.len(),
        0,
        "and is removed with its page"
    );
    let _ = s.undo();
    assert!(red_box_right_of(&s, o.x).is_some());
}

#[test]
fn ac12_0093_7_the_inside_of_a_placed_page_stays_read_only() {
    let (bytes, laser, o) = placed_file((10.0, 20.0), (50.0, 40.0));
    let mut s = open(&bytes);
    assert!(s.activate_page(&laser));
    let before = opened(&s).read_objects().objects;
    s.set_tool(Tool::Select);
    let a = Point::new(o.x + 30.0, o.y + 30.0);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(Point::new(a.x + 20.0, a.y + 20.0), false, false);
    s.pointer_up(Point::new(a.x + 20.0, a.y + 20.0), false, false);
    key(&mut s, "ArrowRight", false);
    // Amended by `specs/0100` criterion 17: the placed page itself is the selection; no
    // drawing object inside it is.
    assert!(
        s.selected_placement().is_some(),
        "the placed page is selected"
    );
    assert_eq!(
        s.selected_object_count(),
        1,
        "the drawing object is not selected"
    );
    assert_eq!(
        opened(&s).read_objects().objects,
        before,
        "nothing inside changed"
    );
}
