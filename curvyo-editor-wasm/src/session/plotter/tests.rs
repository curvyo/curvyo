//! Session tests of `specs/0159-cutting-plotter` slice 1: Add page, profiles, mats, Cut and Draw,
//! the cut-area check, the Toolpath view, Export and the library sync of cutter records.

use curvyo_cutter_core::{CRICUT_MAKER_3, GENERIC_HPGL_CUTTER, write_record};
use curvyo_document_core::{Direction, DocumentSize, NodeId, Point, RectBounds};
use serde_json::Value;

use super::super::Session;

const PLOTTER: &str = "curvyo.cutter";

fn plotter() -> Session {
    let mut s = Session::new(159_001);
    s.resize_viewport(1200.0, 800.0);
    s.add_page_of_kind(PLOTTER, None).unwrap();
    s
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.plotter_view_json()).unwrap()
}

fn origin(s: &Session) -> Point {
    s.active_plotter().unwrap().page.origin
}

/// A rectangle from `(x, y)` mm of the mat origin, `w` x `h`.
fn rect(s: &mut Session, left: f64, top: f64, width: f64, height: f64) -> NodeId {
    let at = origin(s);
    s.document.create_rect(RectBounds::from_corners(
        Point::new(at.x + left, at.y + top),
        Point::new(at.x + left + width, at.y + top + height),
    ))
}

fn assign(s: &mut Session, kind: &str, items: &[NodeId]) -> NodeId {
    s.selection.set(items);
    let id = s.press_plotter_kind(kind, false).unwrap();
    s.selection.set(&[]);
    id
}

fn page_size(s: &Session) -> DocumentSize {
    s.active_plotter().unwrap().page.size
}

fn undo_name(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

#[test]
fn criterion_1_add_page_plotter_is_a_12x12_mat_page_and_set_mat_resizes_in_one_step() {
    let mut s = Session::new(159_002);
    let steps = s.undo_len();
    s.add_page_of_kind(PLOTTER, None).unwrap();
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_name(&mut s), "Add plotter page");
    let m = s.active_output_page().unwrap();
    assert_eq!(m.process, PLOTTER);
    assert_eq!(page_size(&s), DocumentSize::from_mm(304.8, 304.8));
    let steps = s.undo_len();
    assert!(s.set_plotter_mat("12x24"));
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert_eq!(undo_name(&mut s), "Set mat");
    assert_eq!(page_size(&s), DocumentSize::from_mm(304.8, 609.6));
    assert_eq!(view(&s)["mat"]["size"], "12x24");
    s.undo();
    assert_eq!(page_size(&s), DocumentSize::from_mm(304.8, 304.8));
}

#[test]
fn criterion_2_the_profile_pick_lists_exactly_the_plotter_profiles() {
    let mut s = plotter();
    let names: Vec<String> = view(&s)["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names, ["Cricut Maker 3", "Generic HPGL cutter"]);
    assert_eq!(view(&s)["profile"]["output"], "Design Space SVG");
    assert!(s.pick_plotter_profile(GENERIC_HPGL_CUTTER.id));
    assert_eq!(page_size(&s), DocumentSize::from_mm(600.0, 1000.0));
    assert_eq!(view(&s)["profile"]["output"], "HPGL");
    assert_eq!(view(&s)["mat"], Value::Null, "no mat by default");
    // A saved copy is a cutter record in the same machines list.
    let id = s.save_plotter_profile([7; 16], 1_000).unwrap();
    let writes: Value = serde_json::from_str(&s.take_machine_writes_json()).unwrap();
    let text = writes[0]["text"].as_str().unwrap();
    assert!(text.contains("\"kind\": \"cutter\""), "{text}");
    assert_eq!(writes[0]["id"], id.as_str());
    assert!(s.pick_plotter_profile(CRICUT_MAKER_3.id));
    assert_eq!(view(&s)["profiles"].as_array().unwrap().len(), 3);
}

#[test]
fn criterion_3_the_grip_changes_the_colour_only_and_the_mat_switch_the_cut_area() {
    let mut s = plotter();
    let a = rect(&mut s, 20.0, 20.0, 30.0, 30.0);
    assign(&mut s, "cut", &[a]);
    let before = s.plotter_export().unwrap().text;
    assert!(s.set_plotter_grip("strong"));
    assert_eq!(undo_name(&mut s), "Set grip");
    assert_eq!(view(&s)["mat"]["gripWord"], "StrongGrip");
    assert_eq!(
        s.plotter_export().unwrap().text,
        before,
        "no effect on the output"
    );
    assert_eq!(view(&s)["profile"]["area"], "292.1 × 292.1 mm");
    assert!(s.set_plotter_mat("12x24"));
    assert_eq!(view(&s)["profile"]["area"], "292.1 × 596.9 mm");
    assert_eq!(
        view(&s)["mat"]["gripWord"],
        "StrongGrip",
        "kept across the size change"
    );
}

#[test]
fn criterion_5_the_mat_grid_never_reaches_the_export() {
    let mut s = plotter();
    let a = rect(&mut s, 20.0, 20.0, 30.0, 30.0);
    assign(&mut s, "cut", &[a]);
    s.show_default_view();
    let on = s.plotter_export().unwrap().text;
    let drawn_on = s.frame_draw_list().triangles.len();
    assert!(!s.plotter_mat_labels().is_empty() || s.view().scale() < 1.0);
    assert!(s.set_plotter_mat_grid(false));
    assert_eq!(undo_name(&mut s), "Mat grid");
    assert_eq!(s.plotter_export().unwrap().text, on, "byte-identical");
    assert!(
        s.frame_draw_list().triangles.len() < drawn_on,
        "the printing goes"
    );
    assert_eq!(s.plotter_mat_labels(), []);
}

#[test]
fn criterion_6_an_item_past_the_cut_area_blocks_export() {
    let mut s = plotter();
    // The cut area ends at 6.35 + 292.1 = 298.45 mm; this square ends 1 mm past it.
    let a = rect(&mut s, 289.45, 20.0, 10.0, 10.0);
    assign(&mut s, "cut", &[a]);
    let v = view(&s);
    assert_eq!(
        v["checks"][0]["text"],
        "1 item lies outside the mat's cut area."
    );
    assert_eq!(v["exportAllowed"], false);
    assert!(s.plotter_export().is_none());
    assert!(s.export_job("Card.curvyo").is_none());
}

#[test]
fn criterion_7_8_9_cut_and_draw_with_their_numbers_draw_before_cut() {
    let mut s = plotter();
    assert!(s.pick_plotter_profile(GENERIC_HPGL_CUTTER.id));
    let a = rect(&mut s, 10.0, 10.0, 20.0, 20.0);
    let b = rect(&mut s, 50.0, 10.0, 20.0, 20.0);
    let cut = assign(&mut s, "cut", &[a]);
    assert_eq!(undo_name(&mut s), "Assign to Cut 1");
    let draw = assign(&mut s, "draw", &[b]);
    let rows = view(&s)["operations"].clone();
    assert_eq!(rows[0]["name"], "Draw 1", "Draw before Cut");
    assert_eq!(rows[1]["name"], "Cut 1");
    assert_eq!(rows[1]["line3"], "40 %, 200 mm/s, 1 ×");
    assert_eq!(rows[0]["line3"], "40 %, 200 mm/s");
    assert_eq!(
        s.edit_plotter_setting(cut, "passes", "11"),
        Err("Passes is 1 to 10.".to_owned())
    );
    assert_eq!(s.edit_plotter_setting(cut, "passes", "2"), Ok(true));
    assert_eq!(undo_name(&mut s), "Edit settings");
    assert!(s.edit_plotter_setting(draw, "passes", "2").is_err());
    let plt = s.plotter_export().unwrap().text;
    assert!(plt.starts_with("IN;\nVS20;\nFS32;\n"), "{plt}");
    assert_eq!(
        plt.lines().filter(|l| l.starts_with("PD")).count(),
        3,
        "draw once, cut twice"
    );
}

#[test]
fn criterion_8_a_design_space_page_hides_the_numbers() {
    let mut s = plotter();
    let a = rect(&mut s, 10.0, 10.0, 20.0, 20.0);
    assign(&mut s, "cut", &[a]);
    let v = view(&s);
    assert_eq!(v["operations"][0]["line3"], "Set in Design Space");
    assert_eq!(v["operations"][0]["unfolds"], false);
    assert_eq!(v["total"]["time"], Value::Null);
    assert_eq!(v["profile"]["setsNumbers"], true);
}

#[test]
fn criterion_10_the_toolpath_view_draws_the_moves_of_the_written_job() {
    let mut s = plotter();
    s.show_default_view();
    let a = rect(&mut s, 10.0, 10.0, 20.0, 20.0);
    assign(&mut s, "cut", &[a]);
    let page = s.active_page().id;
    assert!(s.plotter_toolpath(s.view()).is_none(), "not pressed");
    assert!(s.toggle_toolpath_view(&page));
    let page = s.active_plotter().unwrap();
    let look = s.plotter_look(&page);
    let job = s.plotter_job(&page).job;
    assert_eq!(look.segments.len(), job.moves().len(), "one builder");
    assert!(s.plotter_toolpath(s.view()).is_some());
    assert_eq!(view(&s)["toolpath"], true);
}

#[test]
fn criterion_11_16_export_is_the_design_space_svg_under_the_laser_name_rule() {
    let mut s = plotter();
    let a = rect(&mut s, 10.0, 20.0, 30.0, 40.0);
    assign(&mut s, "cut", &[a]);
    let steps = s.undo_len();
    let job = s.export_job("Card.curvyo").unwrap();
    assert_eq!(job.name, "Card - Plotter 1.svg");
    assert_eq!(job.file_type, "Design Space SVG");
    let text = String::from_utf8(job.bytes.clone()).unwrap();
    assert!(text.contains("width=\"12in\" height=\"12in\""));
    assert!(text.contains("M 10.000 20.000"), "{text}");
    assert!(s.export_job_done(job.token, &job.name, false, 0).is_some());
    assert_eq!(s.undo_len(), steps, "no step (UX C16)");
    let again = s.export_job("Card.curvyo").unwrap();
    assert_eq!(again.bytes, job.bytes, "criterion 16");
}

#[test]
fn criterion_17_a_plotter_page_survives_save_and_reopen() {
    let mut s = plotter();
    assert!(s.set_plotter_mat("12x24"));
    assert!(s.set_plotter_grip("light"));
    assert!(s.set_plotter_mat_grid(false));
    let a = rect(&mut s, 10.0, 10.0, 20.0, 20.0);
    assign(&mut s, "draw", &[a]);
    let before = view(&s);
    let mut r = Session::open(159_003, &s.pack("0.1.0").unwrap()).unwrap();
    let page = r
        .document
        .manufacturings()
        .into_iter()
        .find(|m| m.process == PLOTTER)
        .and_then(|m| m.page)
        .unwrap();
    assert!(r.activate_page(&page.id));
    let after = view(&r);
    assert_eq!(after["mat"], before["mat"]);
    assert_eq!(after["operations"], before["operations"]);
    assert_eq!(after["profile"], before["profile"]);
}

#[test]
fn criterion_18_a_synced_cutter_record_lands_in_the_plotter_profiles() {
    let mut s = plotter();
    let mut profile = CRICUT_MAKER_3.profile();
    profile.id = "0b5f6c2e-9d4a-4c1b-8e7f-1a2b3c4d5e6f".to_owned();
    profile.modified = 5;
    profile.values.name = "From the other device".to_owned();
    let text = write_record(&profile);
    s.load_machine_profiles(&[(format!("{}.json", profile.id), text)]);
    assert!(s.manufacturing.plotter.find(&profile.id).is_some());
    assert!(
        s.manufacturing.machine.profiles.find(&profile.id).is_none(),
        "never a laser profile"
    );
}

#[test]
fn laser_tools_are_absent_on_a_plotter_page() {
    let s = plotter();
    assert!(s.active_manufacturing().is_none());
    assert!(s.active_operations().is_none());
    let v: Value = serde_json::from_str(&s.manufacturing_view_json()).unwrap();
    assert_eq!(v["machine"], Value::Null);
    assert_eq!(
        v["plotter"]["machineLine"],
        "Cricut Maker 3 · Design Space SVG"
    );
    assert!(
        v["status"]
            .as_str()
            .unwrap()
            .starts_with("Cricut Maker 3 · 304.8 × 304.8")
    );
}

#[test]
fn criterion_18_cutter_profiles_go_out_on_first_sync_and_arrivals_take_the_cutter_merge() {
    let mut s = plotter();
    let id = s.save_plotter_profile([9; 16], 2_000).unwrap();
    let items = s.library_items();
    let item = items.iter().find(|(_, i, _)| *i == id).unwrap();
    assert!(item.2.contains("\"kind\": \"cutter\""));
    assert!(super::super::library_sync::is_cutter_record(&item.2));
    let laser = r#"{"format_version": 8, "kind": "laser"}"#;
    assert!(!super::super::library_sync::is_cutter_record(laser));
}
