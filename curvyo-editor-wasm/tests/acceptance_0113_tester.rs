//! Independent acceptance tests for `specs/0113-fixture-sets-and-pins` through the public session:
//! machine sets and their On this page check (criteria 2 to 8), the profile copy (9, 10, 14),
//! job fixtures and armed placement (11 to 17), the pin and the pin and screw grids (18 to 22),
//! the role jobs Centre mark and Diameters (25 to 28), centre snapping (30), Lock (31, 32), Apply
//! fixtures from page (33 to 37), the session round trip, old files and damage (38 to 41), peer
//! merge and no export of fixtures (43). Written from the specification before the implementation
//! was read; the texts asserted are the words of the spec, the view keys are the session's
//! contract (`tests/fixtures/fixtures_view/`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::redundant_closure_for_method_calls,
    clippy::bool_assert_comparison,
    clippy::assertions_on_result_states,
    missing_docs
)]

use std::io::{Cursor, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Direction, Document, DocumentSize, DocumentSnapping, EllipseFrame,
    FixtureJobSwitch, FixtureSnapshot, Length, NewOutputPage, NodeId, Point, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, SvgExportResult, Tool};
use curvyo_ui_core::SvgExportOptions;
use loro::LoroDoc;
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";

// ---- helpers ------------------------------------------------------------------------------------

/// A drawing page plus laser pages of the given work areas (mm), each with the circles
/// `(x, y, r)` in work-area coordinates as own items; the first laser page is active.
fn pages(areas: &[(f64, f64)], circles: &[(f64, f64, f64)]) -> (Session, Vec<String>) {
    let d = Document::new(1);
    let mut ids = Vec::new();
    for (i, (w, h)) in areas.iter().enumerate() {
        let made = d
            .add_output_page(NewOutputPage {
                process: LASER,
                label: if i == 0 { "Front" } else { "Back" },
                profile: None,
                work_area: DocumentSize::from_mm(*w, *h),
            })
            .unwrap();
        if i == 0 {
            let o = d
                .pages()
                .into_iter()
                .find(|p| p.id == made.id)
                .unwrap()
                .origin;
            for (x, y, r) in circles {
                let _ = d.create_ellipse(EllipseFrame {
                    center: Point::new(o.x + x, o.y + y),
                    rx: Length::from_mm(*r),
                    ry: Length::from_mm(*r),
                });
            }
        }
        ids.push(made.id);
    }
    let mut s = Session::open(9113, &pack(&d, "0113-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.activate_page(&ids[0]);
    // Superseded by rework (2026-11-16): sets live in the machine profile, so a page on Generic
    // laser has no "+ Set" (criterion 3); the active page is saved as a profile first, so its
    // sets are editable. `generic()` keeps a Generic laser page.
    let _ = s.save_machine_profile([13; 16], 1_000);
    s.wipe_history();
    (s, ids)
}

fn one() -> Session {
    pages(&[(400.0, 300.0)], &[]).0
}

/// A laser page left on Generic laser (no profile): its sets are not editable (criterion 3).
fn generic() -> Session {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Front",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mut s = Session::open(9113, &pack(&d, "0113-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.activate_page(&made.id);
    s.wipe_history();
    s
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.fixtures_view_json()).unwrap()
}

fn sets(s: &Session) -> Vec<Value> {
    view(s)["sets"].as_array().cloned().unwrap_or_default()
}

fn set_row(s: &Session, name: &str) -> Value {
    sets(s)
        .into_iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no set {name}: {}", view(s)))
}

fn set_names(s: &Session) -> Vec<String> {
    sets(s)
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect()
}

fn jobfx(s: &Session) -> Vec<Value> {
    view(s)["job_fixtures"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn all_rows(s: &Session) -> Vec<Value> {
    let mut out: Vec<Value> = sets(s)
        .iter()
        .flat_map(|set| set["fixtures"].as_array().cloned().unwrap_or_default())
        .collect();
    out.extend(jobfx(s));
    out
}

fn row(s: &Session, id: NodeId) -> Value {
    all_rows(s)
        .into_iter()
        .find(|r| r["id"] == id.full_id())
        .unwrap_or_else(|| panic!("no row for {id:?}: {}", view(s)))
}

fn row_named(s: &Session, name: &str) -> Value {
    all_rows(s)
        .into_iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no row {name}: {}", view(s)))
}

fn node(s: &Session, row: &Value) -> NodeId {
    s.fixture_part_named(row["id"].as_str().unwrap())
        .expect("the row's node")
}

fn undo_label(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

fn jobs_view(s: &Session) -> Value {
    serde_json::from_str(&s.fixture_jobs_view_json()).unwrap()
}

fn jobs_of(s: &Session, fixture: NodeId) -> Vec<Value> {
    jobs_view(s)["groups"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|g| g["fixture"] == fixture.full_id())
        .map(|g| g["jobs"].as_array().cloned().unwrap_or_default())
        .unwrap_or_default()
}

fn role_job(s: &Session, fixture: NodeId, role: &str) -> (NodeId, Value) {
    let job = jobs_of(s, fixture)
        .into_iter()
        .find(|j| j["role"] == role)
        .unwrap_or_else(|| panic!("no {role} job: {}", jobs_view(s)));
    let id = s
        .fixture_job_named(job["id"].as_str().unwrap())
        .expect("the job node");
    (id, job)
}

fn reopened(s: &Session) -> Document {
    unpack(9114, &s.pack("0113-tester").unwrap()).unwrap()
}

/// The stored fixture `id` as the document reads it.
fn stored(s: &Session, id: NodeId) -> FixtureSnapshot {
    reopened(s)
        .manufacturings()
        .into_iter()
        .flat_map(|m| m.fixtures)
        .find(|f| f.id == id)
        .unwrap_or_else(|| panic!("no stored fixture {id:?}"))
}

fn centre(s: &Session, id: NodeId) -> (f64, f64) {
    let m = stored(s, id).matrix.expect("a matrix");
    (m.e, m.f)
}

/// A point of the active page in document coordinates, from work-area mm.
fn at(s: &Session, x: f64, y: f64) -> Point {
    let o = s.active_page().origin;
    Point::new(o.x + x, o.y + y)
}

fn key(s: &mut Session, k: &str) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    })
}

fn snapping_on(s: &mut Session, objects: bool) {
    assert!(
        s.set_snapping(DocumentSnapping {
            enabled: true,
            objects,
            ..DocumentSnapping::DEFAULT
        }) || s.snapping_settings().objects == objects
    );
}

/// Arms `kind` in the Job fixtures bar and drops it at the work area's centre (keyboard
/// placement), then ends placement.
fn place(s: &mut Session, kind: &str) -> NodeId {
    assert!(s.arm_job_fixture(kind), "{kind} arms");
    let id = s.drop_fixture_placement().expect("a dropped job fixture");
    s.end_fixture_placement();
    id
}

/// Places `kind` at work-area `(x, y)` with the keyboard (from the centre).
fn place_at(s: &mut Session, kind: &str, x: f64, y: f64) -> NodeId {
    assert!(s.arm_job_fixture(kind));
    let (w, h) = (
        s.active_page().size.width.as_mm(),
        s.active_page().size.height.as_mm(),
    );
    assert!(s.nudge_fixture_placement(x - w / 2.0, y - h / 2.0));
    let id = s.drop_fixture_placement().expect("dropped");
    s.end_fixture_placement();
    id
}

fn draw(s: &Session) -> String {
    format!("{:?}", s.frame_draw_list().triangles)
}

fn tri(s: &Session) -> usize {
    s.frame_draw_list().triangles.len()
}

fn click(s: &mut Session, p: Point) {
    s.set_tool(Tool::Select);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn drag(s: &mut Session, from: Point, to: Point) {
    s.pointer_down(from, false);
    for i in 1..=8 {
        let t = f64::from(i) / 8.0;
        s.pointer_hover(
            Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t),
            false,
            false,
        );
    }
    s.pointer_up(to, false, false);
}

fn machine(s: &Session) -> Value {
    serde_json::from_str::<Value>(&s.manufacturing_view_json()).unwrap()["machine"].clone()
}

fn record_texts(json: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    let mut out = Vec::new();
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) if s.contains("format_version") => out.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            Value::Object(o) => o.values().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    walk(&v, &mut out);
    out
}

/// The profile the active page edits. Superseded by rework (2026-11-16): every tester page is
/// on a saved profile, so the first user profile of the list is no longer the page's.
fn user_profile_id(s: &Session) -> String {
    let id = machine(s)["copy"]["id"].as_str().unwrap().to_owned();
    assert!(
        machine(s)["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == id && p["builtin"] == false),
        "the page is on a user profile"
    );
    id
}

/// The host's library listing `[{name, text}]` of the newest record text of every profile the
/// session wrote since the last call (the writes are taken).
fn library(s: &mut Session) -> String {
    let writes: Vec<Value> = serde_json::from_str(&s.take_machine_writes_json()).unwrap();
    let mut newest: Vec<(String, Value)> = Vec::new();
    for w in writes.iter().filter(|w| w["kind"] == "write") {
        let id = w["id"].as_str().unwrap().to_owned();
        newest.retain(|(i, _)| *i != id);
        newest.push((id, w["text"].clone()));
    }
    Value::Array(
        newest
            .into_iter()
            .map(|(id, text)| json!({ "name": curvyo_laser_core::record_file_name(&id), "text": text }))
            .collect(),
    )
    .to_string()
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name)
}

fn profile_golden(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-laser-core/tests/fixtures/machine_profiles")
            .join(name),
    )
    .unwrap()
}

/// Opens a golden and activates its first laser page with a Fixtures view.
fn open_golden(name: &str) -> Session {
    let bytes = std::fs::read(golden_path(name)).unwrap();
    let d = unpack(9120, &bytes).unwrap();
    let mut s = Session::open(9120, &bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.load_machine_profiles_json("[]");
    for page in d.pages() {
        if s.activate_page(&page.id) && view(&s) != Value::Null {
            return s;
        }
    }
    panic!("{name} has no laser page");
}

fn d_name(s: &Session, page: &str) -> String {
    unpack(1, &s.pack("t").unwrap())
        .unwrap()
        .pages()
        .into_iter()
        .find(|p| p.id == page)
        .unwrap()
        .name
}

fn notice_ok(n: &str) {
    assert!(n.chars().count() <= 70, "notice over 70 characters: {n}");
}

// ---- criterion 1: the Fixtures tab ---------------------------------------------------------------

#[test]
fn ac01_a_laser_page_has_a_fixtures_tab_after_operations_and_materials() {
    let s = one();
    let tabs: Vec<&str> = s.panel_view().tabs.iter().map(|t| t.name).collect();
    let f = tabs
        .iter()
        .position(|t| *t == "fixtures")
        .expect("a Fixtures tab");
    let o = tabs.iter().position(|t| *t == "operations").unwrap();
    assert!(o < f, "after Operations: {tabs:?}");
    if let Some(m) = tabs.iter().position(|t| *t == "materials") {
        assert!(m < f, "after Materials: {tabs:?}");
    }
    let label = s
        .panel_view()
        .tabs
        .iter()
        .find(|t| t.name == "fixtures")
        .unwrap()
        .label;
    assert_eq!(label, "Fixtures");
}

#[test]
fn ac01_a_drawing_page_has_no_fixtures_tab() {
    let (mut s, _) = pages(&[(400.0, 300.0)], &[]);
    let drawing = s.add_page_of_kind("drawing", None).unwrap();
    assert!(s.activate_page(&drawing));
    assert!(
        s.panel_view().tabs.iter().all(|t| t.name != "fixtures"),
        "no Fixtures tab on a drawing page"
    );
    assert!(
        !s.press_panel_tab("fixtures") || s.panel_view().tabs.iter().all(|t| t.name != "fixtures")
    );
}

// ---- criteria 2, 3, 11: set rows, "+ Set", the empty page ----------------------------------------

#[test]
fn ac11_an_empty_laser_page_shows_both_sections_and_no_apply() {
    let s = one();
    let v = view(&s);
    assert_eq!(v["sets"], json!([]));
    assert_eq!(v["job_fixtures"], json!([]));
    assert_eq!(v["can_add_set"], true, "+ Set is offered");
    assert_eq!(
        v["can_add_job_fixture"], true,
        "the Job fixtures add bar is offered"
    );
    assert_eq!(v["apply_candidates"], json!([]), "Apply is hidden");
}

#[test]
fn ac03_plus_set_adds_set_n_empty_on_unfolded_in_one_step() {
    let mut s = one();
    let before = s.undo_len();
    let a = s.add_fixture_set().expect("Set 1");
    assert_eq!(s.undo_len(), before + 1);
    assert_eq!(undo_label(&mut s), "Add fixture set");
    let r = set_row(&s, "Set 1");
    assert_eq!(r["on_page"], true);
    assert_eq!(r["unfolded"], true);
    assert_eq!(r["fixtures"], json!([]));
    s.add_fixture_set().unwrap();
    s.add_fixture_set().unwrap();
    assert_eq!(set_names(&s), ["Set 1", "Set 2", "Set 3"]);
    assert!(s.remove_fixture_set(a));
    s.add_fixture_set().unwrap();
    assert!(
        set_names(&s).contains(&"Set 1".to_owned()),
        "the lowest unused number: {:?}",
        set_names(&s)
    );
}

#[test]
fn ac02_set_row_line_two_counts_kinds_and_heading_counts_sets_on_page() {
    let mut s = one();
    let a = s.add_fixture_set().unwrap();
    s.add_set_fixture(a, "holder").unwrap();
    s.add_set_fixture(a, "screw").unwrap();
    let b = s.add_fixture_set().unwrap();
    s.add_set_fixture(b, "clamp").unwrap();
    assert_eq!(set_row(&s, "Set 1")["line2"], "Holder, Screw · 2 fixtures");
    assert_eq!(view(&s)["sets_on_page"], 2);
    assert!(s.set_fixture_set_on_page(b, false));
    assert_eq!(view(&s)["sets_on_page"], 1, "1 of 2 on this page");
    assert_eq!(sets(&s).len(), 2);
    let line = set_row(&s, "Set 2")["line2"].as_str().unwrap().to_owned();
    assert!(line.contains("not on this page"), "{line}");
}

#[test]
fn ac02_an_empty_set_that_is_off_reads_not_on_this_page() {
    let mut s = one();
    let a = s.add_fixture_set().unwrap();
    assert!(s.set_fixture_set_on_page(a, false));
    let line = set_row(&s, "Set 1")["line2"].as_str().unwrap().to_owned();
    assert!(line.contains("not on this page"), "{line}");
}

#[test]
fn ac02_fixtures_of_a_set_that_is_off_are_not_drawn_not_hit_and_run_no_job() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    let holder = s.add_set_fixture(set, "holder").unwrap();
    // The holder is 100 x 60 mm centred on the 400 x 300 work area: outline at x = 150.
    s.deselect_fixture_job();
    s.escape();
    let edge = at(&s, 150.0, 150.0);
    click(&mut s, edge);
    assert_eq!(s.selected_fixture(), Some(holder), "on: hit by its outline");
    s.add_slot(holder).expect("a slot");
    assert!(s.select_fixture(holder));
    let job = s.add_fixture_job("engrave").expect("a job on the holder");
    assert!(s.set_fixture_job_part(job, "slot:0", true));
    assert_eq!(jobs_view(&s)["running"], 0, "Output starts off");
    let jid = s
        .fixture_job_named(jobs_of(&s, holder)[0]["id"].as_str().unwrap())
        .unwrap();
    assert!(s.set_fixture_job_switch(jid, FixtureJobSwitch::Output, true));
    let running_on = jobs_view(&s)["running"].as_u64().unwrap();
    assert!(running_on >= 1, "the job runs while the set is on");
    s.escape();
    let drawn_on = draw(&s);
    assert!(s.set_fixture_set_on_page(set, false));
    assert_eq!(
        s.selected_fixture(),
        None,
        "a fixture of a set turned off is not selected"
    );
    assert_ne!(draw(&s), drawn_on, "not drawn");
    click(&mut s, edge);
    assert_eq!(s.selected_fixture(), None, "not hit");
    assert_eq!(jobs_view(&s)["running"], 0, "runs no job");
    let r = row(&s, holder);
    assert_eq!(r["on_bed"], false);
    assert_eq!(r["active"], true, "its own check is untouched");
}

// ---- criterion 4: rename --------------------------------------------------------------------------

#[test]
fn ac04_rename_cuts_to_40_need_not_be_unique_and_is_one_step() {
    let mut s = one();
    let a = s.add_fixture_set().unwrap();
    let b = s.add_fixture_set().unwrap();
    let long = "x".repeat(55);
    assert!(s.rename_fixture_set(a, &long));
    assert_eq!(undo_label(&mut s), "Rename fixture set");
    assert_eq!(sets(&s)[0]["name"].as_str().unwrap().chars().count(), 40);
    assert!(
        s.rename_fixture_set(b, &"x".repeat(40)),
        "a repeated name is allowed"
    );
    assert_eq!(sets(&s)[0]["name"], sets(&s)[1]["name"]);
    s.undo();
    assert_eq!(sets(&s)[1]["name"], "Set 2");
}

// ---- criterion 5: remove --------------------------------------------------------------------------

#[test]
fn ac05_remove_takes_the_set_its_fixtures_and_jobs_and_undo_restores_exactly() {
    let mut s = one();
    let keep = s.add_fixture_set().unwrap();
    s.add_set_fixture(keep, "clamp").unwrap();
    let gone = s.add_fixture_set().unwrap();
    let holder = s.add_set_fixture(gone, "holder").unwrap();
    s.add_slot(holder).unwrap();
    assert!(s.select_fixture(holder));
    s.add_fixture_job("engrave").unwrap();
    let screw = s.add_set_fixture(gone, "screw").unwrap();
    let _ = screw;
    let job_fixture = place(&mut s, "pin");
    s.escape();
    let before = view(&s);
    let before_jobs = jobs_view(&s)["groups"].clone();
    let steps = s.undo_len();
    assert!(s.remove_fixture_set(gone));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Remove fixture set");
    assert_eq!(set_names(&s), ["Set 1"]);
    assert_eq!(all_rows(&s).len(), 2, "the clamp and the job fixture stay");
    assert!(jobs_of(&s, holder).is_empty(), "the jobs went with the set");
    let _ = row(&s, job_fixture);
    s.undo();
    let mut after = view(&s);
    let mut b = before.clone();
    for v in [&mut after, &mut b] {
        v["selected"] = Value::Null;
        v["selection_announcement"] = Value::Null;
    }
    assert_eq!(after["sets"].as_array().unwrap().len(), 2);
    assert_eq!(
        after["sets"][1]["fixtures"], b["sets"][1]["fixtures"],
        "undo restores the set's fixtures exactly"
    );
    assert_eq!(after["sets"][1]["name"], b["sets"][1]["name"]);
    assert_eq!(jobs_view(&s)["groups"], before_jobs, "and their jobs");
}

// ---- criteria 6, 16: move and the listing order --------------------------------------------------

#[test]
fn ac06_16_moving_a_set_keeps_contents_and_orders_sets_then_job_fixtures() {
    let mut s = one();
    let a = s.add_fixture_set().unwrap();
    let ha = s.add_set_fixture(a, "holder").unwrap();
    let b = s.add_fixture_set().unwrap();
    let cb = s.add_set_fixture(b, "clamp").unwrap();
    let sb = s.add_set_fixture(b, "stop").unwrap();
    let pin = place(&mut s, "pin");
    let names = |s: &Session| -> Vec<String> {
        all_rows(s)
            .iter()
            .map(|r| r["name"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(names(&s), ["Holder 1", "Clamp 1", "Stop 1", "Pin 1"]);
    let steps = s.undo_len();
    assert!(s.move_fixture_set(b, 0));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Move fixture set");
    assert_eq!(set_names(&s), ["Set 2", "Set 1"]);
    assert_eq!(names(&s), ["Clamp 1", "Stop 1", "Holder 1", "Pin 1"]);
    // The Layers tree lists the fixtures in the same order.
    let tree: Value = serde_json::from_str(&s.layers_view_json(0, 100)).unwrap();
    let layer_order: Vec<String> = tree["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "fixture")
        .map(|r| r["fixture"]["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        layer_order,
        [cb, sb, ha, pin].map(|n| n.full_id()).to_vec(),
        "Layers: sets in order, then job fixtures"
    );
    s.undo();
    assert_eq!(set_names(&s), ["Set 1", "Set 2"]);
}

#[test]
fn ac16_the_fixture_jobs_groups_follow_sets_then_job_fixtures() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    let a = s.add_fixture_set().unwrap();
    let screw = s.add_set_fixture(a, "screw").unwrap();
    let groups: Vec<String> = jobs_view(&s)["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["fixture"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        groups,
        [screw.full_id(), pin.full_id()],
        "the set's screw comes first though the pin is older"
    );
}

// ---- criterion 7: the set add bar ----------------------------------------------------------------

#[test]
fn ac07_the_set_add_bar_adds_each_kind_active_selected_centred_one_step() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    for kind in ["grid", "holder", "clamp", "stop", "screw", "pin"] {
        let steps = s.undo_len();
        let id = s
            .add_set_fixture(set, kind)
            .unwrap_or_else(|| panic!("{kind} is added"));
        assert_eq!(s.undo_len(), steps + 1, "{kind}: one step");
        assert_eq!(undo_label(&mut s), "Add fixture", "{kind}");
        assert_eq!(s.selected_fixture(), Some(id), "{kind} is selected");
        let r = row(&s, id);
        assert_eq!(r["active"], true, "{kind} starts Active");
        // Superseded by rework (2026-11-16): a set fixture has a Lock (criterion 32), off.
        assert_eq!(r["locked"], false, "{kind}: a set fixture starts unlocked");
        assert_eq!(r["locked_by_set"], false, "{kind}");
    }
    let pin = node(&s, &row_named(&s, "Pin 1"));
    assert_eq!(centre(&s, pin), (200.0, 150.0), "centred on the work area");
}

#[test]
fn ac07_grid_is_hidden_while_the_set_has_one_but_another_set_may_have_one() {
    let mut s = one();
    let a = s.add_fixture_set().unwrap();
    assert_eq!(set_row(&s, "Set 1")["can_add_grid"], true);
    s.add_set_fixture(a, "grid").unwrap();
    assert_eq!(set_row(&s, "Set 1")["can_add_grid"], false);
    assert!(s.add_set_fixture(a, "grid").is_none());
    let b = s.add_fixture_set().unwrap();
    assert!(
        s.add_set_fixture(b, "grid").is_some(),
        "one engraved grid per set"
    );
}

#[test]
fn ac07_folding_a_set_is_not_an_undo_step() {
    let mut s = one();
    let a = s.add_fixture_set().unwrap();
    let steps = s.undo_len();
    assert!(s.set_fixture_set_unfolded(a, false));
    assert_eq!(set_row(&s, "Set 1")["unfolded"], false);
    assert_eq!(s.undo_len(), steps, "fold is no step");
}

// ---- criterion 8: own On the bed and the set's On this page (Frage 211 B) ------------------------

#[test]
fn ac08_on_the_bed_needs_the_set_on_and_the_own_check_on() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    let h = s.add_set_fixture(set, "holder").unwrap();
    let on_bed = |s: &Session| row(s, h)["on_bed"].as_bool().unwrap();
    assert!(on_bed(&s));
    assert!(s.set_fixture_active(h, false));
    assert_eq!(undo_label(&mut s), "Set fixture inactive");
    assert!(!on_bed(&s));
    assert!(s.set_fixture_set_on_page(set, false));
    assert_eq!(undo_label(&mut s), "Set fixture set off page");
    assert!(!on_bed(&s));
    assert!(s.set_fixture_active(h, true));
    assert_eq!(undo_label(&mut s), "Set fixture active");
    assert!(!on_bed(&s), "own check on, set off: not on the bed");
    assert!(s.set_fixture_set_on_page(set, true));
    assert_eq!(undo_label(&mut s), "Set fixture set on page");
    assert!(on_bed(&s));
}

// ---- criteria 9, 10, 14: the profile and its sets ------------------------------------------------

/// A page with two sets (the second off), a job fixture; saved as a profile. Returns the record.
fn saved_profile(s: &mut Session) -> (String, String) {
    let a = s.add_fixture_set().unwrap();
    s.add_set_fixture(a, "grid").unwrap();
    s.add_set_fixture(a, "holder").unwrap();
    let b = s.add_fixture_set().unwrap();
    assert!(s.rename_fixture_set(b, "Coaster jig"));
    s.add_set_fixture(b, "screw").unwrap();
    assert!(s.set_fixture_set_on_page(b, false));
    place(s, "pin");
    let _ = s.save_machine_profile([7; 16], 50);
    let id = user_profile_id(s);
    let record = record_texts(&s.take_machine_writes_json())
        .last()
        .expect("a record")
        .clone();
    (id, record)
}

#[test]
fn ac10_save_as_profile_writes_sets_without_on_page_or_job_fixtures() {
    let mut s = one();
    let (_, record) = saved_profile(&mut s);
    let r: Value = serde_json::from_str(&record).unwrap();
    // Record format 6 since `0029` (ADR 0065 §3), 7 since `0150`.
    assert_eq!(r["format_version"], 7);
    let sets_ = r["fixture_sets"].as_array().unwrap();
    assert_eq!(sets_.len(), 2);
    assert_eq!(sets_[0]["name"], "Set 1");
    assert_eq!(sets_[1]["name"], "Coaster jig");
    assert_eq!(sets_[0]["fixtures"].as_array().unwrap().len(), 2);
    assert_eq!(sets_[1]["fixtures"].as_array().unwrap().len(), 1);
    assert!(
        !record.contains("on_page"),
        "no On this page in the record: {record}"
    );
    assert!(
        !record.contains("\"Pin 1\""),
        "no job fixture in the record: {record}"
    );
    assert!(r.get("fixtures").is_none(), "no flat fixtures key");
    assert_eq!(
        machine(&s)["fixture_sets_in_profile"],
        2,
        "Fixture sets in profile: 2"
    );
}

#[test]
fn ac10_differs_ignores_on_this_page_and_job_fixtures_but_sees_set_changes() {
    let mut s = one();
    saved_profile(&mut s);
    assert_eq!(
        machine(&s)["state"],
        Value::Null,
        "freshly saved: not differing"
    );
    let b = s
        .fixture_set_named(sets(&s)[1]["id"].as_str().unwrap())
        .unwrap();
    assert!(s.set_fixture_set_on_page(b, true));
    place(&mut s, "screw");
    assert_eq!(
        machine(&s)["state"],
        Value::Null,
        "On this page and job fixtures are not compared"
    );
    // Superseded by rework (2026-11-16): Differs compares the machine fields only; the sets
    // are the profile's and cannot differ (criterion 10).
    assert!(s.rename_fixture_set(b, "Other"));
    assert_eq!(
        machine(&s)["state"],
        Value::Null,
        "a set name is not compared"
    );
    assert!(s.move_fixture_set(b, 0));
    assert_eq!(
        machine(&s)["state"],
        Value::Null,
        "the set order is not compared"
    );
    let h = node(&s, &row_named(&s, "Holder 1"));
    assert!(s.set_fixture_locked(h, true));
    assert_eq!(machine(&s)["state"], Value::Null, "a Lock is not compared");
    let laser_mode = machine(&s)["copy"]["laser_mode"].as_bool().unwrap();
    s.set_machine_laser_mode(!laser_mode);
    assert_eq!(
        machine(&s)["state"],
        "differs",
        "a machine field is compared"
    );
}

#[test]
fn ac09_a_new_page_from_the_profile_gets_every_set_on_in_order_and_no_job_fixture() {
    let mut a = one();
    let (id, record) = saved_profile(&mut a);
    let mut b = one();
    b.load_machine_profiles_json(
        &json!([{ "name": curvyo_laser_core::record_file_name(&id), "text": record }]).to_string(),
    );
    b.add_page_of_kind(LASER, Some(&id))
        .expect("a page from the profile");
    assert_eq!(set_names(&b), ["Set 1", "Coaster jig"]);
    assert!(
        sets(&b).iter().all(|r| r["on_page"] == true),
        "all sets On this page"
    );
    assert!(jobfx(&b).is_empty(), "no job fixture comes along");
    let src_ids: Vec<Value> = sets(&a).iter().map(|r| r["id"].clone()).collect();
    assert!(
        sets(&b).iter().all(|r| !src_ids.contains(&r["id"])),
        "fresh node ids"
    );
}

#[test]
fn ac09_14_pick_replaces_sets_only_in_one_step_and_reset_turns_every_set_on() {
    let mut a = one();
    let (id, record) = saved_profile(&mut a);
    // Superseded by rework (2026-11-16): sets need a saved profile (criterion 3), so page b is
    // saved as its own profile before it gets a set.
    let mut b = generic();
    b.load_machine_profiles_json(
        &json!([{ "name": curvyo_laser_core::record_file_name(&id), "text": record }]).to_string(),
    );
    let _ = b.save_machine_profile([21; 16], 70);
    let own = b.add_fixture_set().unwrap();
    b.add_set_fixture(own, "clamp").unwrap();
    let pin = place(&mut b, "pin");
    let steps = b.undo_len();
    assert!(b.pick_machine_profile(&id));
    assert_eq!(b.undo_len(), steps + 1, "one step");
    assert_eq!(
        set_names(&b),
        ["Set 1", "Coaster jig"],
        "the sets are replaced"
    );
    assert_eq!(jobfx(&b).len(), 1, "the job fixture is kept");
    let _ = row(&b, pin);
    assert!(
        sets(&b).iter().all(|r| r["on_page"] == true),
        "the pick turns every set on"
    );
    // Superseded by rework (2026-11-16): Reset from profile does not touch the sets
    // (criterion 9); it used to turn every set on.
    let jig = b
        .fixture_set_named(sets(&b)[1]["id"].as_str().unwrap())
        .unwrap();
    assert!(b.set_fixture_set_on_page(jig, false));
    b.set_machine_laser_mode(!machine(&b)["copy"]["laser_mode"].as_bool().unwrap());
    assert!(b.reset_machine_profile());
    assert_eq!(
        set_names(&b),
        ["Set 1", "Coaster jig"],
        "Reset keeps the sets"
    );
    assert_eq!(
        sets(&b)[1]["on_page"],
        false,
        "Reset keeps the page's choice"
    );
    assert_eq!(jobfx(&b).len(), 1, "Reset keeps the job fixtures");
    b.undo();
    b.undo();
    b.undo();
    b.undo();
    assert_eq!(
        set_names(&b),
        ["Set 1"],
        "undo of the pick restores the old sets"
    );
    assert!(b.pick_machine_profile("generic-laser"));
    assert!(sets(&b).is_empty(), "Generic laser has no set");
    assert_eq!(jobfx(&b).len(), 1, "the job fixture stays");
    // Superseded by rework (2026-11-16): no "Fixture sets removed" notice (criterion 9).
    assert_eq!(b.take_fixture_notice(), None, "no notice");
    assert!(
        b.machine_profiles_json().contains(&id),
        "the sets stay in their profile"
    );
}

#[test]
fn ac09_update_profile_keeps_the_on_this_page_choices() {
    let mut s = one();
    saved_profile(&mut s);
    let a = s
        .fixture_set_named(sets(&s)[0]["id"].as_str().unwrap())
        .unwrap();
    let _ = s.take_machine_writes_json();
    s.add_set_fixture(a, "clamp").unwrap();
    // Superseded by rework (2026-11-16): a set edit writes the profile at once and never makes
    // the page differ; Update profile writes the machine fields only and keeps the record's
    // sets (criteria 7, 10).
    assert_eq!(
        machine(&s)["state"],
        Value::Null,
        "a set edit does not differ"
    );
    let record = record_texts(&s.take_machine_writes_json())
        .last()
        .expect("the set edit writes the record")
        .clone();
    let r: Value = serde_json::from_str(&record).unwrap();
    assert_eq!(
        r["fixture_sets"][0]["fixtures"].as_array().unwrap().len(),
        3
    );
    s.set_machine_laser_mode(!machine(&s)["copy"]["laser_mode"].as_bool().unwrap());
    assert_eq!(machine(&s)["state"], "differs");
    let _ = s.update_machine_profile(60);
    assert_eq!(
        sets(&s)[1]["on_page"],
        false,
        "Update keeps the page's choice"
    );
    let record = record_texts(&s.take_machine_writes_json())
        .last()
        .unwrap()
        .clone();
    let u: Value = serde_json::from_str(&record).unwrap();
    assert_eq!(
        u["fixture_sets"], r["fixture_sets"],
        "Update keeps the record's sets"
    );
}

// ---- criteria 12, 13: Job fixtures add bar and armed placement -----------------------------------

#[test]
fn ac12_the_job_bar_offers_pin_screw_holder_clamp_stop_and_no_grid() {
    let mut s = one();
    for kind in ["pin", "screw", "holder", "clamp", "stop"] {
        assert!(s.arm_job_fixture(kind), "{kind} arms");
        assert!(s.end_fixture_placement());
    }
    assert!(!s.arm_job_fixture("grid"), "no Grid among job fixtures");
    assert_eq!(s.armed_job_fixture(), None);
}

#[test]
fn ac12_line_two_of_a_job_fixture_names_kind_count_size_locked_and_jobs() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    assert!(s.set_fixture_grid_value(pin, "columns", 2.0));
    assert!(s.set_fixture_grid_value(pin, "rows", 2.0));
    assert!(s.set_fixture_locked(pin, true));
    let (cm, _) = role_job(&s, pin, "centre_mark");
    assert!(s.set_fixture_job_switch(cm, FixtureJobSwitch::Output, true));
    assert_eq!(
        row(&s, pin)["line2"],
        "Pin · 2 × 2 · ⌀ 6 mm · locked · Centre mark"
    );
}

#[test]
fn ac13_placement_stays_armed_names_lowest_unused_and_each_drop_is_one_step() {
    let mut s = one();
    assert!(s.arm_job_fixture("pin"));
    let steps = s.undo_len();
    let p1 = s.drop_fixture_placement().unwrap();
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Add job fixture");
    assert_eq!(
        s.armed_job_fixture().is_some(),
        true,
        "stays armed after a drop"
    );
    let p2 = s.drop_fixture_placement().unwrap();
    assert_eq!(s.selected_fixture(), Some(p2), "the new one is selected");
    assert_eq!(row(&s, p1)["name"], "Pin 1");
    assert_eq!(row(&s, p2)["name"], "Pin 2");
    assert_eq!(row(&s, p2)["active"], true);
    assert!(s.end_fixture_placement());
    assert!(s.delete_fixture(p1));
    assert!(s.arm_job_fixture("screw"));
    let sc = s.drop_fixture_placement().unwrap();
    assert_eq!(row(&s, sc)["name"], "Screw 1", "numbered per kind");
    s.end_fixture_placement();
    assert!(s.arm_job_fixture("pin"));
    let p = s.drop_fixture_placement().unwrap();
    assert_eq!(row(&s, p)["name"], "Pin 1", "the lowest unused number");
}

#[test]
fn ac13_second_press_escape_and_another_tool_end_placement_without_adding() {
    let mut s = one();
    let count = |s: &Session| jobfx(s).len();
    assert!(s.arm_job_fixture("pin"));
    assert!(!s.arm_job_fixture("pin"), "a second press ends it");
    assert_eq!(s.armed_job_fixture(), None);
    assert!(s.arm_job_fixture("pin"));
    let _ = key(&mut s, "Escape");
    assert_eq!(s.armed_job_fixture(), None, "Escape ends it");
    assert!(s.arm_job_fixture("pin"));
    s.set_tool(Tool::Rectangle);
    assert_eq!(s.armed_job_fixture(), None, "another tool ends it");
    s.set_tool(Tool::Select);
    assert_eq!(count(&s), 0, "nothing added");
}

#[test]
fn ac13_a_click_drops_and_selects_nothing_else_and_the_pointer_outside_shows_nothing() {
    let (mut s, _) = pages(&[(400.0, 300.0)], &[(100.0, 100.0, 10.0)]);
    s.set_tool(Tool::Select);
    let empty = tri(&s);
    assert!(s.arm_job_fixture("clamp"));
    let p = at(&s, 300.0, 200.0);
    s.pointer_hover(p, false, false);
    let hanging = tri(&s);
    assert!(hanging > empty, "the fixture hangs at the pointer");
    s.pointer_leave();
    assert_eq!(
        tri(&s),
        empty,
        "outside the canvas the fixture is not shown"
    );
    // A click on the circle's outline drops a fixture and selects no drawing item.
    let edge = at(&s, 110.0, 100.0);
    s.pointer_hover(edge, false, false);
    s.pointer_down(edge, false);
    s.pointer_up(edge, false, false);
    assert_eq!(jobfx(&s).len(), 1, "a click drops one fixture");
    assert!(s.select_bar_state().radius.is_none());
    assert_eq!(
        s.selected_fixture().map(|id| row(&s, id)["name"].clone()),
        Some(json!("Clamp 1")),
        "the new fixture is the selection, not the circle"
    );
    assert!(s.armed_job_fixture().is_some(), "still armed");
}

// ---- criteria 14, 15: job fixtures are page-only fixtures ----------------------------------------

#[test]
fn ac14_a_set_off_does_not_touch_job_fixtures() {
    let mut s = one();
    let a = s.add_fixture_set().unwrap();
    let pin = place(&mut s, "pin");
    assert!(s.set_fixture_set_on_page(a, false));
    assert_eq!(row(&s, pin)["on_bed"], true);
    let st = stored(&s, pin);
    assert_eq!(st.set, None, "a job fixture is in no set");
}

#[test]
fn ac15_a_job_fixture_is_hit_by_its_outline_hidden_by_its_eye_and_in_layers() {
    let mut s = one();
    let h = place(&mut s, "holder");
    s.escape();
    assert_eq!(s.selected_fixture(), None);
    // The holder is 100 x 60 mm centred at (200, 150): left edge at x = 150.
    let p = at(&s, 150.0, 150.0);
    click(&mut s, p);
    assert_eq!(s.selected_fixture(), Some(h));
    assert!(s.set_fixture_visible(h, false));
    assert_eq!(row(&s, h)["visible"], false);
    let tree: Value = serde_json::from_str(&s.layers_view_json(0, 100)).unwrap();
    assert!(
        tree["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["fixture"]["id"] == h.full_id()),
        "listed in the Layers tree"
    );
    assert!(s.set_fixture_active(h, false));
    assert_eq!(row(&s, h)["on_bed"], false);
}

// ---- criterion 17: limits ------------------------------------------------------------------------

#[test]
fn ac17_at_16_sets_plus_set_is_hidden() {
    let mut s = one();
    for _ in 0..16 {
        s.add_fixture_set().expect("up to 16 sets");
    }
    assert_eq!(view(&s)["can_add_set"], false);
    assert!(s.add_fixture_set().is_none());
    assert_eq!(sets(&s).len(), 16);
}

#[test]
fn ac17_at_64_job_fixtures_the_add_bar_is_hidden() {
    let mut s = one();
    assert!(s.arm_job_fixture("stop"));
    for i in 0..64 {
        assert!(s.drop_fixture_placement().is_some(), "drop {i}");
    }
    assert!(s.drop_fixture_placement().is_none(), "no 65th");
    s.end_fixture_placement();
    assert_eq!(jobfx(&s).len(), 64);
    assert_eq!(view(&s)["can_add_job_fixture"], false);
    assert!(!s.arm_job_fixture("pin"));
}

// ---- criteria 18 to 22: pins and grids -----------------------------------------------------------

#[test]
fn ac18_a_pin_is_6_mm_and_its_diameter_is_clamped_to_half_to_1000_with_a_notice() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    assert_eq!(row(&s, pin)["round"]["diameter_mm"], 6.0);
    let _ = s.take_fixture_notice();
    assert!(s.set_fixture_grid_value(pin, "diameter_mm", 2000.0));
    assert_eq!(row(&s, pin)["round"]["diameter_mm"], 1000.0);
    let n = s.take_fixture_notice().expect("a notice");
    notice_ok(&n);
    assert!(s.set_fixture_grid_value(pin, "diameter_mm", 0.1));
    assert_eq!(row(&s, pin)["round"]["diameter_mm"], 0.5);
    notice_ok(&s.take_fixture_notice().expect("a notice"));
    assert!(s.set_fixture_grid_value(pin, "diameter_mm", 8.0));
    assert_eq!(undo_label(&mut s), "Resize fixture");
    assert!(
        s.take_fixture_notice().is_none(),
        "an in-range value needs no notice"
    );
}

#[test]
fn ac19_counts_are_1_to_50_spacing_1_to_1000_defaults_1_and_20() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    let r = row(&s, pin)["round"].clone();
    assert_eq!(r["columns"], 1);
    assert_eq!(r["rows"], 1);
    assert_eq!(r["column_spacing_mm"], 20.0);
    assert_eq!(r["row_spacing_mm"], 20.0);
    assert!(s.set_fixture_grid_value(pin, "columns", 70.0));
    assert_eq!(row(&s, pin)["round"]["columns"], 50);
    assert!(s.set_fixture_grid_value(pin, "columns", 0.0));
    assert_eq!(row(&s, pin)["round"]["columns"], 1);
    assert!(s.set_fixture_grid_value(pin, "column_spacing_mm", 0.2));
    assert_eq!(row(&s, pin)["round"]["column_spacing_mm"], 1.0);
    assert!(s.set_fixture_grid_value(pin, "row_spacing_mm", 5000.0));
    assert_eq!(row(&s, pin)["round"]["row_spacing_mm"], 1000.0);
}

#[test]
fn ac19_the_product_is_at_most_400_and_a_larger_count_is_cut_with_a_notice() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    assert!(s.set_fixture_grid_value(pin, "columns", 50.0));
    let _ = s.take_fixture_notice();
    assert!(s.set_fixture_grid_value(pin, "rows", 9.0));
    assert_eq!(row(&s, pin)["round"]["rows"], 8, "50 x 8 = 400");
    notice_ok(&s.take_fixture_notice().expect("a notice"));
    let screw = place(&mut s, "screw");
    assert!(s.set_fixture_grid_value(screw, "rows", 20.0));
    assert!(s.set_fixture_grid_value(screw, "columns", 21.0));
    assert_eq!(row(&s, screw)["round"]["columns"], 20, "20 x 20 = 400");
    assert_eq!(jobfx(&s).len(), 2, "a grid is one fixture with one row");
}

#[test]
fn ac20_a_screw_row_has_no_head_or_thread_field_and_counts_are_resize_steps() {
    let mut s = one();
    let screw = place(&mut s, "screw");
    let r = row(&s, screw)["round"].clone();
    assert_eq!(
        r["diameter_mm"],
        Value::Null,
        "no Diameter field on a screw"
    );
    assert!(
        !s.set_fixture_grid_value(screw, "diameter_mm", 5.0),
        "the key is refused"
    );
    let steps = s.undo_len();
    assert!(s.set_fixture_grid_value(screw, "columns", 4.0));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Resize fixture");
}

#[test]
fn ac21_a_pin_grid_does_not_turn() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    assert!(s.set_fixture_grid_value(pin, "columns", 3.0));
    assert!(s.select_fixture(pin));
    assert_ne!(key(&mut s, "r"), KeyOutcome::EntryOpened, "no R chip");
    let m = stored(&s, pin).matrix.unwrap();
    assert_eq!((m.a, m.b, m.c, m.d), (1.0, 0.0, 0.0, 1.0), "never turned");
}

#[test]
fn ac22_pin_and_screw_store_their_grid_parameters_and_mark_size() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    let screw = place(&mut s, "screw");
    let p = stored(&s, pin);
    assert_eq!(p.kind, "curvyo.laser/pin");
    for key in [
        "diameter_mm",
        "columns",
        "rows",
        "column_spacing_mm",
        "row_spacing_mm",
        "mark_mm",
    ] {
        assert!(
            p.params.contains_key(key),
            "pin stores {key}: {:?}",
            p.params
        );
    }
    let sc = stored(&s, screw);
    for key in [
        "thread_diameter_mm",
        "head_diameter_mm",
        "columns",
        "rows",
        "column_spacing_mm",
        "row_spacing_mm",
        "mark_mm",
    ] {
        let short = key.trim_end_matches("_mm");
        assert!(
            sc.params.contains_key(key) || sc.params.contains_key(short),
            "screw stores {key}: {:?}",
            sc.params
        );
    }
}

// ---- criteria 25 to 28: Centre mark and Diameters ------------------------------------------------

#[test]
fn ac27_a_pin_and_a_screw_get_two_score_role_jobs_output_off_with_fixed_ticks() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    let screw = place(&mut s, "screw");
    for (f, diam_parts) in [(pin, 1), (screw, 2)] {
        let jobs = jobs_of(&s, f);
        assert_eq!(jobs.len(), 2, "{jobs:?}");
        assert_eq!(jobs[0]["name"], "Centre mark");
        assert_eq!(jobs[1]["name"], "Diameters");
        for j in &jobs {
            assert_eq!(j["kind"]["text"], "score");
            assert_eq!(j["output"], false);
        }
        assert_eq!(jobs[0]["parts_ticked"], 1, ".marks");
        assert_eq!(
            jobs[1]["parts_ticked"], diam_parts,
            ".circle or .head and .thread"
        );
    }
    let st = stored(&s, screw);
    let diam = st.jobs.iter().find(|j| j.name == "Diameters").unwrap();
    let mut suffixes: Vec<String> = diam
        .targets
        .iter()
        .filter(|t| t.ticked)
        .filter_map(|t| t.key.suffix.clone())
        .collect();
    suffixes.sort();
    assert_eq!(suffixes, ["head", "thread"]);
}

#[test]
fn ac27_role_jobs_cannot_be_deleted_renamed_retickeded_and_no_further_job_is_offered() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    let (cm, _) = role_job(&s, pin, "centre_mark");
    let (dm, _) = role_job(&s, pin, "diameters");
    let steps = s.undo_len();
    assert!(!s.delete_fixture_job(cm), "no trash");
    assert!(!s.rename_fixture_job(dm, "Other"), "no rename");
    assert!(!s.set_fixture_job_part(cm, "circle", true), "no other tick");
    assert!(!s.set_fixture_job_part(dm, "circle", false), "no untick");
    assert!(s.select_fixture(pin));
    assert!(
        s.add_fixture_job("engrave").is_none(),
        "no further job on a pin"
    );
    assert_eq!(s.undo_len(), steps, "no refused change is a step");
    assert_eq!(jobs_of(&s, pin).len(), 2);
    // The jobs live and die with the fixture.
    assert!(s.delete_fixture(pin));
    assert_eq!(jobs_of(&s, pin).len(), 0);
}

#[test]
fn ac25_the_toggles_are_the_output_switch_one_state_in_two_places() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    let holder = place(&mut s, "holder");
    assert!(s.select_fixture(holder));
    let hj = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_switch(hj, FixtureJobSwitch::Output, true));
    let output_on_label = undo_label(&mut s);
    assert_eq!(
        row(&s, pin)["round"]["centre_mark"]["on"],
        false,
        "both start off"
    );
    assert_eq!(row(&s, pin)["round"]["diameters"]["on"], false);
    let (cm, _) = role_job(&s, pin, "centre_mark");
    let ink = draw(&s);
    assert!(s.set_fixture_job_switch(cm, FixtureJobSwitch::Output, true));
    assert_eq!(undo_label(&mut s), output_on_label, "the 0103 Output step");
    assert_eq!(row(&s, pin)["round"]["centre_mark"]["on"], true);
    assert_eq!(role_job(&s, pin, "centre_mark").1["output"], true);
    assert_ne!(draw(&s), ink, "Score look while on");
    s.undo();
    assert_eq!(row(&s, pin)["round"]["centre_mark"]["on"], false);
}

#[test]
fn ac25_26_mark_size_is_half_to_50_default_4_and_may_exceed_the_pin() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    assert_eq!(row(&s, pin)["round"]["mark_mm"], 4.0);
    let _ = s.take_fixture_notice();
    assert!(s.set_fixture_grid_value(pin, "mark_mm", 20.0));
    assert_eq!(row(&s, pin)["round"]["mark_mm"], 20.0);
    assert!(
        s.take_fixture_notice().is_none(),
        "larger than the 6 mm pin: no notice"
    );
    assert!(s.set_fixture_grid_value(pin, "mark_mm", 80.0));
    assert_eq!(row(&s, pin)["round"]["mark_mm"], 50.0);
    notice_ok(&s.take_fixture_notice().expect("a notice"));
    assert!(s.set_fixture_grid_value(pin, "mark_mm", 0.1));
    assert_eq!(row(&s, pin)["round"]["mark_mm"], 0.5);
}

#[test]
fn ac25_marks_are_drawn_at_every_centre_of_the_grid() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    let (cm, _) = role_job(&s, pin, "centre_mark");
    assert!(s.set_fixture_job_switch(cm, FixtureJobSwitch::Output, true));
    s.escape();
    let one_centre = tri(&s);
    assert!(s.set_fixture_grid_value(pin, "columns", 4.0));
    s.escape();
    let four = tri(&s);
    assert!(
        four > one_centre,
        "more centres draw more: {one_centre} -> {four}"
    );
}

#[test]
fn ac28_a_job_of_a_fixture_on_a_set_that_is_off_reads_not_on_this_page() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    let screw = s.add_set_fixture(set, "screw").unwrap();
    let (cm, _) = role_job(&s, screw, "centre_mark");
    assert!(s.set_fixture_job_switch(cm, FixtureJobSwitch::Output, true));
    assert_eq!(role_job(&s, screw, "centre_mark").1["runs"], true);
    assert!(s.set_fixture_set_on_page(set, false));
    let j = role_job(&s, screw, "centre_mark").1;
    assert_eq!(j["runs"], false);
    assert_eq!(j["on_this_page"], false);
    assert_eq!(jobs_view(&s)["running"], 0);
    // A job fixture has no set and no such condition.
    let pin = place(&mut s, "pin");
    assert_eq!(role_job(&s, pin, "centre_mark").1["on_this_page"], true);
}

// ---- criterion 30: centre snapping --------------------------------------------------------------

#[test]
fn ac30_a_placed_job_fixture_snaps_its_centre_to_a_circle_centre() {
    let (mut s, _) = pages(&[(400.0, 300.0)], &[(100.0, 80.0, 10.0)]);
    snapping_on(&mut s, true);
    assert!(s.arm_job_fixture("pin"));
    let near = at(&s, 100.6, 80.4);
    s.pointer_hover(near, false, false);
    s.pointer_down(near, false);
    s.pointer_up(near, false, false);
    s.end_fixture_placement();
    let pin = jobfx(&s)[0].clone();
    let (x, y) = centre(&s, node(&s, &pin));
    assert!(
        (x - 100.0).abs() < 1e-6 && (y - 80.0).abs() < 1e-6,
        "snapped: {x}, {y}"
    );
}

#[test]
fn ac30_no_snap_while_the_centres_target_is_off() {
    let (mut s, _) = pages(&[(400.0, 300.0)], &[(100.0, 80.0, 10.0)]);
    // Amended 2026-11-16 (customer): Centres is its own target; Objects no longer gates it.
    snapping_on(&mut s, true);
    assert!(s.set_snapping_centres(false));
    assert!(s.arm_job_fixture("pin"));
    let near = at(&s, 100.6, 80.4);
    s.pointer_hover(near, false, false);
    s.pointer_down(near, false);
    s.pointer_up(near, false, false);
    s.end_fixture_placement();
    let pin = jobfx(&s)[0].clone();
    let (x, _) = centre(&s, node(&s, &pin));
    assert!((x - 100.0).abs() > 0.1, "not snapped with Centres off: {x}");
}

#[test]
fn ac19_30_each_centre_of_a_grid_snaps_and_columns_run_right() {
    // A 2 x 1 grid at 30 mm: its second centre is 30 mm to the right of the first.
    let (mut s, _) = pages(&[(400.0, 300.0)], &[(250.0, 150.0, 5.0)]);
    snapping_on(&mut s, true);
    let pin = place_at(&mut s, "pin", 50.0, 50.0);
    assert!(s.set_fixture_grid_value(pin, "columns", 2.0));
    assert!(s.set_fixture_grid_value(pin, "column_spacing_mm", 30.0));
    assert!(s.select_fixture(pin));
    // Drag the grid so that its second centre lands near the circle at (250, 150).
    let from = at(&s, 50.0, 50.0);
    let to = at(&s, 220.5, 150.4);
    drag(&mut s, from, to);
    let (x, y) = centre(&s, pin);
    assert!(
        (x - 220.0).abs() < 1e-6 && (y - 150.0).abs() < 1e-6,
        "the second centre snapped, columns run right: first at {x}, {y}"
    );
}

#[test]
fn ac30_set_fixtures_do_not_centre_snap() {
    let (mut s, _) = pages(&[(400.0, 300.0)], &[(100.0, 80.0, 10.0)]);
    snapping_on(&mut s, true);
    let set = s.add_fixture_set().unwrap();
    let pin = s.add_set_fixture(set, "pin").unwrap();
    assert!(s.select_fixture(pin));
    let (cx, cy) = centre(&s, pin);
    let from = at(&s, cx, cy);
    let to = at(&s, 100.6, 80.4);
    drag(&mut s, from, to);
    let (x, y) = centre(&s, pin);
    assert!(
        (x - 100.0).abs() > 1e-3 || (y - 80.0).abs() > 1e-3,
        "a set fixture is not centre-snapped: {x}, {y}"
    );
}

// ---- criteria 31, 32: Lock ----------------------------------------------------------------------

#[test]
fn ac31_lock_is_one_step_shown_in_the_row_and_the_select_bar_and_not_on_set_fixtures() {
    let mut s = one();
    let pin = place(&mut s, "pin");
    assert!(s.select_fixture(pin));
    assert_eq!(s.selected_fixture_lock(), Some(false));
    let steps = s.undo_len();
    assert!(s.set_fixture_locked(pin, true));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Lock fixture");
    assert_eq!(row(&s, pin)["locked"], true);
    assert_eq!(s.selected_fixture_lock(), Some(true));
    assert!(s.set_fixture_locked(pin, false));
    assert_eq!(undo_label(&mut s), "Unlock fixture");
    // Superseded by rework (2026-11-16): a set fixture has a Lock too, and the Select bar shows
    // it (criterion 32); it used to have none.
    let set = s.add_fixture_set().unwrap();
    let h = s.add_set_fixture(set, "holder").unwrap();
    assert!(s.select_fixture(h));
    assert_eq!(s.selected_fixture_lock(), Some(false));
    assert_eq!(s.selected_fixture_lock_pressable(), Some(true));
    assert!(s.set_fixture_locked(h, true), "a set fixture has a Lock");
    assert_eq!(undo_label(&mut s), "Lock fixture");
    assert_eq!(row(&s, h)["locked"], true);
    assert_eq!(s.selected_fixture_lock(), Some(true), "the bar shows it");
}

#[test]
fn ac32_a_locked_job_fixture_does_not_move_resize_or_open_chips_but_stays_usable() {
    let mut s = one();
    let holder = place(&mut s, "holder");
    let pin = place_at(&mut s, "pin", 60.0, 60.0);
    assert!(s.set_fixture_locked(holder, true));
    assert!(s.set_fixture_locked(pin, true));
    assert!(s.select_fixture(holder));
    let before = stored(&s, holder);
    let steps = s.undo_len();
    // Drag from inside its box and from its outline.
    let (p1, p2, p3, p4) = (
        at(&s, 200.0, 150.0),
        at(&s, 260.0, 190.0),
        at(&s, 150.0, 150.0),
        at(&s, 120.0, 190.0),
    );
    drag(&mut s, p1, p2);
    drag(&mut s, p3, p4);
    assert_eq!(
        s.selected_fixture(),
        Some(holder),
        "stays selected, no marquee"
    );
    for k in ["ArrowRight", "ArrowDown"] {
        let _ = key(&mut s, k);
    }
    assert_ne!(key(&mut s, "m"), KeyOutcome::EntryOpened, "no Move chip");
    s.escape();
    assert!(s.select_fixture(holder));
    assert_ne!(key(&mut s, "s"), KeyOutcome::EntryOpened, "no S chip");
    s.escape();
    let after = stored(&s, holder);
    assert_eq!(after.matrix, before.matrix, "kept its place");
    assert_eq!(after.params, before.params, "kept its size");
    assert_eq!(s.undo_len(), steps, "nothing was a step");
    assert!(
        !s.set_fixture_grid_value(pin, "columns", 3.0),
        "no knobs or size change"
    );
    assert!(
        s.fixture_labels().iter().any(|l| l.kind == "lock"),
        "a lock badge"
    );
    assert!(row(&s, pin)["line2"].as_str().unwrap().contains("locked"));
    // Eye, Active and its jobs still work; the trash still works.
    let (cm, _) = role_job(&s, pin, "centre_mark");
    assert!(s.set_fixture_job_switch(cm, FixtureJobSwitch::Output, true));
    assert_eq!(
        role_job(&s, pin, "centre_mark").1["runs"],
        true,
        "still runs its jobs"
    );
    assert!(s.set_fixture_visible(pin, false));
    assert!(s.set_fixture_active(pin, false));
    assert!(s.delete_fixture(holder), "the trash still works");
    assert!(all_rows(&s).iter().all(|r| r["id"] != holder.full_id()));
}

// ---- criteria 33 to 37: Apply fixtures from page -------------------------------------------------

#[test]
fn ac33_candidates_are_the_other_laser_pages_with_job_fixtures_and_cancel_is_no_step() {
    let (mut s, ids) = pages(&[(400.0, 300.0), (400.0, 300.0), (400.0, 300.0)], &[]);
    assert!(
        s.apply_fixture_candidates().is_empty(),
        "hidden while no page qualifies"
    );
    place(&mut s, "pin");
    place(&mut s, "screw");
    assert!(
        s.apply_fixture_candidates().is_empty(),
        "the page itself is not listed"
    );
    assert!(s.activate_page(&ids[1]));
    let c = s.apply_fixture_candidates();
    assert_eq!(c.len(), 1, "page 3 has none and page 2 is this page");
    assert_eq!(c[0].count, 2);
    let front = d_name(&s, &ids[0]);
    assert_eq!(c[0].name, front);
    let src = s.apply_candidate_named(&c[0].id).unwrap();
    let steps = s.undo_len();
    assert!(s.set_apply_preview(Some(src)));
    assert_eq!(view(&s)["apply_picked"], c[0].id.as_str());
    let previewed = tri(&s);
    assert!(s.set_apply_preview(None));
    assert!(tri(&s) < previewed, "the preview is drawn while picked");
    assert_eq!(view(&s)["apply_picked"], Value::Null);
    assert_eq!(s.undo_len(), steps, "Cancel is no step");
}

#[test]
fn ac35_37_apply_replaces_the_job_fixtures_by_independent_copies_output_off_never_mirrored() {
    let (mut s, ids) = pages(&[(400.0, 300.0), (400.0, 300.0)], &[]);
    let pin = place_at(&mut s, "pin", 50.0, 40.0);
    assert!(s.set_fixture_grid_value(pin, "columns", 2.0));
    assert!(s.set_fixture_grid_value(pin, "rows", 2.0));
    assert!(s.set_fixture_grid_value(pin, "diameter_mm", 8.0));
    let (dm, _) = role_job(&s, pin, "diameters");
    assert!(s.set_fixture_job_switch(dm, FixtureJobSwitch::Output, true));
    assert!(s.set_fixture_locked(pin, true));
    let holder = place_at(&mut s, "holder", 300.0, 200.0);
    assert!(s.set_fixture_visible(holder, false));
    let src_rows: Vec<Value> = jobfx(&s);
    let src_centres = [centre(&s, pin), centre(&s, holder)];
    assert!(s.activate_page(&ids[1]));
    let own = place(&mut s, "stop");
    let src = s
        .apply_candidate_named(&s.apply_fixture_candidates()[0].id)
        .unwrap();
    assert!(s.set_apply_preview(Some(src)));
    let steps = s.undo_len();
    assert_eq!(s.apply_fixtures_from_page(src), Some(2));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Apply fixtures from page");
    let rows = jobfx(&s);
    assert_eq!(rows.len(), 2, "this page's own job fixture is replaced");
    assert!(
        rows.iter().all(|r| r["id"] != own.full_id()),
        "the stop is gone"
    );
    for (copy, orig) in rows.iter().zip(&src_rows) {
        assert_ne!(copy["id"], orig["id"], "fresh ids");
        for k in ["name", "kind", "visible", "locked", "active"] {
            assert_eq!(copy[k], orig[k], "{k}");
        }
        let mut cr = copy["round"].clone();
        let mut or = orig["round"].clone();
        for r in [&mut cr, &mut or] {
            if let Some(o) = r.as_object_mut() {
                for t in ["centre_mark", "diameters"] {
                    if let Some(x) = o.get_mut(t) {
                        x["job"] = Value::Null;
                    }
                }
            }
        }
        if copy["kind"] == "pin" {
            assert_eq!(cr["diameters"]["on"], false, "jobs copied with Output off");
            cr["diameters"]["on"] = or["diameters"]["on"].clone();
        }
        assert_eq!(cr, or, "same size and grid");
    }
    let copy_pin = node(&s, &rows[0]);
    let copy_holder = node(&s, &rows[1]);
    assert_eq!(
        [centre(&s, copy_pin), centre(&s, copy_holder)],
        src_centres,
        "same positions"
    );
    let m = stored(&s, copy_pin).matrix.unwrap();
    assert_eq!((m.a, m.d), (1.0, 1.0), "never mirrored");
    let diam = stored(&s, copy_pin)
        .jobs
        .into_iter()
        .find(|j| j.name == "Diameters")
        .unwrap();
    assert!(!diam.output);
    assert!(
        diam.targets
            .iter()
            .any(|t| t.ticked && t.key.node == copy_pin),
        "the ticks follow the copy"
    );
    // Independent copies.
    assert!(s.set_fixture_locked(copy_pin, false));
    assert!(s.set_fixture_grid_value(copy_pin, "columns", 5.0));
    assert_eq!(
        stored(&s, pin).params["columns"],
        2,
        "the source is unchanged"
    );
    // Applying again replaces the copies, no rename.
    assert_eq!(s.apply_fixtures_from_page(src), Some(2));
    let names: Vec<Value> = jobfx(&s).iter().map(|r| r["name"].clone()).collect();
    assert_eq!(
        names.iter().filter(|n| **n == json!("Pin 1")).count(),
        1,
        "{names:?}"
    );
    s.undo();
    s.undo();
    s.undo();
    s.undo();
    let back = jobfx(&s);
    assert_eq!(
        back.len(),
        1,
        "undo brings this page's own job fixture back"
    );
    assert_eq!(back[0]["id"], own.full_id());
}

#[test]
fn ac36_a_copy_outside_the_work_area_is_kept_with_a_counted_notice() {
    let (mut s, ids) = pages(&[(800.0, 600.0), (400.0, 300.0)], &[]);
    place_at(&mut s, "pin", 700.0, 500.0);
    place_at(&mut s, "pin", 100.0, 100.0);
    place_at(&mut s, "screw", 600.0, 100.0);
    assert!(s.activate_page(&ids[1]));
    let src = s
        .apply_candidate_named(&s.apply_fixture_candidates()[0].id)
        .unwrap();
    let _ = s.take_fixture_notice();
    assert_eq!(s.apply_fixtures_from_page(src), Some(3));
    assert_eq!(jobfx(&s).len(), 3, "kept at its position");
    let outside = node(&s, &jobfx(&s)[0]);
    assert_eq!(centre(&s, outside), (700.0, 500.0));
    let n = s.take_fixture_notice().expect("the outside notice");
    assert_eq!(n, "2 fixtures lie outside the work area.");
    notice_ok(&n);
}

// ---- criteria 38 to 41: files --------------------------------------------------------------------

#[test]
fn ac38_save_and_reopen_keep_sets_grids_size_lock_output_and_ticks() {
    let mut s = one();
    let a = s.add_fixture_set().unwrap();
    s.add_set_fixture(a, "grid").unwrap();
    let holder = s.add_set_fixture(a, "holder").unwrap();
    s.add_slot(holder).unwrap();
    s.add_slot(holder).unwrap();
    assert!(s.select_fixture(holder));
    let hj = s.add_fixture_job("engrave").unwrap();
    assert!(
        s.set_fixture_job_part(hj, "slot:1", true),
        "a hand-set tick"
    );
    let b = s.add_fixture_set().unwrap();
    assert!(s.rename_fixture_set(b, "Jig"));
    let screw = s.add_set_fixture(b, "screw").unwrap();
    assert!(s.set_fixture_grid_value(screw, "columns", 4.0));
    assert!(s.set_fixture_grid_value(screw, "mark_mm", 3.0));
    let (cm, _) = role_job(&s, screw, "centre_mark");
    assert!(s.set_fixture_job_switch(cm, FixtureJobSwitch::Output, true));
    let clamp = s.add_set_fixture(b, "clamp").unwrap();
    assert!(s.set_fixture_active(clamp, false));
    assert!(s.set_fixture_set_on_page(b, false));
    let pin = place(&mut s, "pin");
    assert!(s.set_fixture_grid_value(pin, "columns", 2.0));
    assert!(s.set_fixture_grid_value(pin, "rows", 2.0));
    let (dm, _) = role_job(&s, pin, "diameters");
    assert!(s.set_fixture_job_switch(dm, FixtureJobSwitch::Output, true));
    assert!(s.set_fixture_locked(pin, true));
    place(&mut s, "pin");
    s.escape();
    let clean = |mut v: Value| {
        v["selected"] = Value::Null;
        v["selection_announcement"] = Value::Null;
        for set in v["sets"].as_array_mut().unwrap() {
            set["unfolded"] = Value::Null;
        }
        v
    };
    let before = clean(view(&s));
    let before_jobs = jobs_view(&s)["groups"].clone();
    let bytes = s.pack("t").unwrap();
    let mut r = Session::open(9140, &bytes).unwrap();
    // Superseded by rework (2026-11-16): the sets are the profile's; the reopened project gets
    // the same library (without it the sets are read only, criterion 46).
    r.load_machine_profiles_json(&library(&mut s));
    let d = unpack(9140, &bytes).unwrap();
    let laser = d
        .pages()
        .into_iter()
        .find(|p| r.activate_page(&p.id) && view(&r) != Value::Null);
    assert!(laser.is_some());
    assert_eq!(
        clean(view(&r)),
        before,
        "every set, fixture, grid, Size and Lock"
    );
    assert_eq!(
        jobs_view(&r)["groups"],
        before_jobs,
        "every Output and tick"
    );
}

#[test]
fn ac38_the_golden_v47_opens_with_two_sets_one_off_and_no_damage() {
    let s = open_golden("fixture_sets_v47.curvyo");
    let v = view(&s);
    assert_eq!(v["sets"].as_array().unwrap().len(), 2);
    assert_eq!(v["sets_on_page"], 1);
    assert_eq!(v["damage_notices"], json!([]));
    let j = jobfx(&s);
    assert_eq!(j.len(), 2);
    assert!(
        j.iter()
            .any(|r| r["locked"] == true && r["round"]["columns"] == 2 && r["round"]["rows"] == 2)
    );
    assert!(
        all_rows(&s).iter().any(|r| r["kind"] == "screw"
            && r["round"]["columns"] == 4
            && r["round"]["mark_mm"] == 3.0),
        "the 4 x 1 screw grid with a 3 mm Size"
    );
    assert!(!s.is_dirty(), "opening writes nothing");
}

#[test]
fn ac39_a_format_46_file_opens_unchanged_with_job_fixtures_and_no_set() {
    let s = open_golden("fixtures_pre_sets_v46.curvyo");
    assert!(sets(&s).is_empty(), "no set is made");
    let j = jobfx(&s);
    assert!(!j.is_empty(), "flat fixtures read as job fixtures");
    assert!(j.iter().all(|r| r["locked"] == false), "Lock off");
    let screw = j.iter().find(|r| r["kind"] == "screw").expect("a screw");
    assert_eq!(
        screw["round"]["centre_mark"],
        Value::Null,
        "no role jobs: toggles hidden"
    );
    assert_eq!(screw["round"]["diameters"], Value::Null);
    let screw_id = node(&s, screw);
    assert!(
        jobs_of(&s, screw_id).iter().all(|j| j["role"].is_null()),
        "the 0103 jobs stay and get no role"
    );
    assert!(!s.is_dirty(), "nothing is written");
}

#[test]
fn ac39_an_older_profile_record_is_skipped_with_the_counted_notice_and_not_rewritten() {
    let mut s = one();
    // The writes of the tester page's own Save as profile (rework 2026-11-16) are not the load's.
    let _ = s.take_machine_writes_json();
    let name = curvyo_laser_core::record_file_name("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
    s.load_machine_profiles_json(
        &json!([{ "name": name, "text": profile_golden("format4_motorised_z.json") }]).to_string(),
    );
    assert!(
        record_texts(&s.take_machine_writes_json()).is_empty(),
        "never rewritten"
    );
    let m = machine(&s);
    assert_eq!(
        m["profiles"].as_array().unwrap().len(),
        1,
        "only Generic laser"
    );
    assert!(
        m.to_string()
            .contains("1 profile has an older format. It was left as it is."),
        "{m}"
    );
}

#[test]
fn ac40_the_damaged_golden_counts_per_set_and_job_fixtures_in_short_notices() {
    let s = open_golden("fixture_sets_damaged_v47.curvyo");
    let notices: Vec<String> = view(&s)["damage_notices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().map_or_else(|| n.to_string(), str::to_owned))
        .collect();
    assert!(!notices.is_empty(), "damage is reported");
    for n in &notices {
        notice_ok(n);
        assert!(n.contains("damaged"), "{n}");
    }
    assert!(!s.is_dirty(), "never rewritten on read");
}

#[test]
fn ac41_a_newer_profile_record_is_skipped_with_its_notice() {
    let mut s = one();
    // The writes of the tester page's own Save as profile (rework 2026-11-16) are not the load's.
    let _ = s.take_machine_writes_json();
    let name = curvyo_laser_core::record_file_name("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
    s.load_machine_profiles_json(
        &json!([{ "name": name, "text": profile_golden("newer_format.json") }]).to_string(),
    );
    let m = machine(&s);
    assert_eq!(m["profiles"].as_array().unwrap().len(), 1);
    assert_eq!(m["library_notices"].as_array().unwrap().len(), 1, "{m}");
}

#[test]
fn ac40_a_damaged_set_in_a_profile_record_skips_the_whole_record() {
    let mut s = one();
    // The writes of the tester page's own Save as profile (rework 2026-11-16) are not the load's.
    let _ = s.take_machine_writes_json();
    let name = curvyo_laser_core::record_file_name("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
    s.load_machine_profiles_json(
        &json!([{ "name": name, "text": profile_golden("format5_damaged_pin.json") }]).to_string(),
    );
    let m = machine(&s);
    assert_eq!(m["profiles"].as_array().unwrap().len(), 1, "skipped");
    assert_eq!(m["library_notices"].as_array().unwrap().len(), 1);
}

// ---- peer merge and format -----------------------------------------------------------------------

fn merged(a: &[u8], b: &[u8]) -> Document {
    let loro = LoroDoc::new();
    for bytes in [a, b] {
        let d = unpack(1, bytes).unwrap();
        loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    }
    loro.commit();
    let snap = loro.export(loro::ExportMode::Snapshot).unwrap();
    let manifest = json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": 1,
        "app_version": "tester-merge",
    });
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default();
    w.start_file("manifest.json", o).unwrap();
    w.write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    w.start_file("document.loro", o).unwrap();
    w.write_all(&snap).unwrap();
    w.start_file("document.json", o).unwrap();
    w.write_all(b"{}").unwrap();
    unpack(9, &w.finish().unwrap().into_inner()).expect("the merge opens")
}

#[test]
fn peers_adding_a_set_and_a_job_fixture_both_survive_the_merge() {
    // Superseded by rework (2026-11-16): both peers have the page's profile, so sets can be
    // added (criterion 3).
    let (base, lib) = {
        let mut s = one();
        (s.pack("base").unwrap(), library(&mut s))
    };
    let open = |peer: u64| {
        let d = unpack(peer, &base).unwrap();
        let mut s = Session::open(peer, &base).unwrap();
        s.load_machine_profiles_json(&lib);
        let p = d
            .pages()
            .into_iter()
            .find(|p| s.activate_page(&p.id) && view(&s) != Value::Null);
        assert!(p.is_some());
        s
    };
    let mut a = open(71);
    let set = a.add_fixture_set().unwrap();
    a.add_set_fixture(set, "holder").unwrap();
    assert!(a.set_fixture_set_on_page(set, false));
    let mut b = open(72);
    let pin = place(&mut b, "pin");
    assert!(b.set_fixture_locked(pin, true));
    let m = merged(&a.pack("a").unwrap(), &b.pack("b").unwrap());
    let mf = m
        .manufacturings()
        .into_iter()
        .find(|m| !m.fixtures.is_empty())
        .expect("the laser page");
    assert_eq!(mf.sets.len(), 1);
    assert!(!mf.sets[0].on_page);
    assert_eq!(mf.fixtures.len(), 2);
    assert!(mf.fixtures.iter().any(|f| f.set.is_none() && f.locked));
    assert!(mf.fixtures.iter().any(|f| f.set == Some(mf.sets[0].id)));
}

// ---- criterion 43: nothing leaves through export --------------------------------------------------

#[test]
fn ac43_sets_job_fixtures_marks_and_circles_are_not_in_the_svg_export() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    s.add_set_fixture(set, "holder").unwrap();
    let pin = place(&mut s, "pin");
    let (cm, _) = role_job(&s, pin, "centre_mark");
    let (dm, _) = role_job(&s, pin, "diameters");
    assert!(s.set_fixture_job_switch(cm, FixtureJobSwitch::Output, true));
    assert!(s.set_fixture_job_switch(dm, FixtureJobSwitch::Output, true));
    let o = s.active_page().origin;
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(o.x + 10.0, o.y + 10.0), false);
    s.pointer_up(Point::new(o.x + 50.0, o.y + 40.0), false, false);
    let SvgExportResult::Written { text, objects, .. } = s.export_svg(SvgExportOptions::default())
    else {
        panic!("written")
    };
    assert_eq!(objects, 1, "only the rectangle: {text}");
    for word in ["Pin 1", "Holder 1", "Set 1", "Centre mark", "Diameters"] {
        assert!(!text.contains(word), "{word} in the export");
    }
    assert!(text.matches("<circle").count() == 0 && text.matches("<ellipse").count() == 0);
}

// ---- rework 2026-11-16: sets in the profile (criteria 3, 17, 32, 40, 44 to 47) -----------------

const HINT: &str =
    "Fixture sets are kept in a machine profile. Save this machine as a profile to add sets.";

/// The name of the active page's machine as the Machine block shows it.
fn machine_name(s: &Session) -> String {
    machine(s)["copy"]["name"].as_str().unwrap().to_owned()
}

/// A second laser page on the active page's profile; it becomes active. Returns (first, second).
fn twin_page(s: &mut Session) -> (String, String) {
    let first = s.active_page().id;
    let id = user_profile_id(s);
    let second = s
        .add_output_page_with(LASER, Some(&id))
        .expect("a page on the same profile");
    assert!(s.activate_page(&second));
    (first, second)
}

fn set_id(s: &Session, name: &str) -> NodeId {
    s.fixture_set_named(set_row(s, name)["id"].as_str().unwrap())
        .unwrap()
}

#[test]
fn ac03_generic_laser_has_no_plus_set_and_shows_the_profile_hint_until_saved() {
    let mut s = generic();
    let v = view(&s);
    assert_eq!(v["can_add_set"], false, "no + Set on Generic laser");
    assert_eq!(v["profile_hint"], HINT);
    assert_eq!(v["read_only"], Value::Null);
    assert!(s.add_fixture_set().is_none(), "the command is refused too");
    assert_eq!(v["can_add_job_fixture"], true, "job fixtures stay");
    place(&mut s, "pin");
    let _ = s.save_machine_profile([31; 16], 10);
    let v = view(&s);
    assert_eq!(v["can_add_set"], true, "Save as profile makes + Set appear");
    assert_eq!(v["profile_hint"], Value::Null);
    assert!(s.add_fixture_set().is_some());
}

#[test]
fn ac03_a_set_added_on_one_page_is_on_there_and_off_on_the_other_pages_of_the_machine() {
    let mut s = one();
    let a = s.add_fixture_set().unwrap();
    s.add_set_fixture(a, "holder").unwrap();
    let (first, second) = twin_page(&mut s);
    assert_eq!(
        set_names(&s),
        ["Set 1"],
        "the profile's sets on the new page"
    );
    let steps = s.undo_len();
    s.add_fixture_set().expect("Set 2 on the second page");
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert_eq!(
        set_row(&s, "Set 2")["on_page"],
        true,
        "on where it was added"
    );
    assert!(s.activate_page(&first));
    assert_eq!(
        set_names(&s),
        ["Set 1", "Set 2"],
        "every page of the machine has it"
    );
    let r = set_row(&s, "Set 2");
    assert_eq!(r["on_page"], false, "Frage 218 A: off on the other pages");
    assert!(r["line2"].as_str().unwrap().contains("not on this page"));
    assert_eq!(set_row(&s, "Set 1")["on_page"], true);
    let _ = second;
}

#[test]
fn ac17_the_set_limits_count_the_profile_64_fixtures_in_all_sets() {
    let mut s = one();
    let kinds = ["holder", "clamp", "stop", "screw", "pin"];
    for n in [20, 20, 20, 4] {
        let set = s.add_fixture_set().unwrap();
        for k in 0..n {
            s.add_set_fixture(set, kinds[k % kinds.len()])
                .expect("up to 64 fixtures in the profile");
        }
    }
    assert!(
        sets(&s).iter().all(|r| r["can_add"] == false),
        "every set's add bar is hidden at 64 fixtures"
    );
    let first = set_id(&s, "Set 1");
    assert!(s.add_set_fixture(first, "clamp").is_none(), "no 65th");
    // The limit counts the profile, so another page of the machine is full too.
    twin_page(&mut s);
    assert!(sets(&s).iter().all(|r| r["can_add"] == false));
}

#[test]
fn ac32_a_locked_set_locks_its_fixtures_hides_its_add_bar_and_writes_the_profile() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    let h = s.add_set_fixture(set, "holder").unwrap();
    s.escape();
    let _ = s.take_machine_writes_json();
    let steps = s.undo_len();
    assert!(s.set_fixture_set_locked(set, true));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Lock fixture set");
    let r = set_row(&s, "Set 1");
    assert_eq!(r["locked"], true);
    assert_eq!(r["can_add"], false, "the add bar is hidden while locked");
    assert!(s.add_set_fixture(set, "clamp").is_none());
    assert_eq!(row(&s, h)["locked_by_set"], true);
    let line = row(&s, h)["line2"].as_str().unwrap().to_owned();
    assert!(line.contains("locked"), "{line}");
    let record = record_texts(&s.take_machine_writes_json())
        .last()
        .expect("the Lock writes the profile")
        .clone();
    let rec: Value = serde_json::from_str(&record).unwrap();
    assert_eq!(rec["fixture_sets"][0]["locked"], true);
    // The fixture keeps its place: no drag moves it.
    let before = centre(&s, h);
    let (from, to) = (at(&s, 150.0, 150.0), at(&s, 120.0, 100.0));
    drag(&mut s, from, to);
    assert_eq!(
        centre(&s, h),
        before,
        "a fixture of a locked set does not move"
    );
    assert!(s.select_fixture(h));
    assert_eq!(
        s.selected_fixture_lock(),
        Some(true),
        "the bar reads pressed"
    );
    assert_eq!(
        s.selected_fixture_lock_pressable(),
        Some(false),
        "and is not pressable"
    );
    assert!(
        !s.set_fixture_locked(h, false),
        "its own Lock is not pressable"
    );
    // A locked set keeps rename, On this page and its trash.
    assert!(s.rename_fixture_set(set, "Jig"));
    assert!(s.set_fixture_set_on_page(set, false));
    assert!(s.set_fixture_set_on_page(set, true));
    // Locked on every page of the machine.
    let (first, _) = twin_page(&mut s);
    assert_eq!(
        set_row(&s, "Jig")["locked"],
        true,
        "the Lock is the profile's"
    );
    assert!(s.activate_page(&first));
    assert!(s.set_fixture_set_locked(set, false));
    assert_eq!(undo_label(&mut s), "Unlock fixture set");
    assert_eq!(row(&s, h)["locked_by_set"], false);
    assert!(s.remove_fixture_set(set), "the trash works");
}

#[test]
fn ac32_a_locked_set_fixture_still_goes_by_its_trash_and_undo_brings_it_back() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    let h = s.add_set_fixture(set, "holder").unwrap();
    assert!(s.set_fixture_locked(h, true));
    assert_eq!(row(&s, h)["locked"], true);
    assert_eq!(row(&s, h)["locked_by_set"], false);
    assert!(
        s.delete_fixture(h),
        "the trash still removes a locked fixture"
    );
    assert_eq!(all_rows(&s).len(), 0, "the fixture is gone");
    s.undo();
    assert_eq!(row(&s, h)["locked"], true, "undo brings it back locked");
}

#[test]
fn ac44_a_set_change_on_one_page_shows_on_the_other_page_at_once_each_keeping_its_choices() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    let h = s.add_set_fixture(set, "holder").unwrap();
    let (first, second) = twin_page(&mut s);
    // Page 2 turns the holder off its bed and hides it; page 1 keeps its own choices.
    let h2 = node(&s, &row_named(&s, "Holder 1"));
    assert!(s.set_fixture_active(h2, false));
    assert!(s.set_fixture_visible(h2, false));
    let s2 = set_id(&s, "Set 1");
    let steps = s.undo_len();
    assert!(s.rename_fixture_set(s2, "Bench"));
    let added = s.add_set_fixture(s2, "clamp").unwrap();
    assert!(s.set_fixture_active(added, false));
    assert_eq!(
        s.undo_len(),
        steps + 3,
        "the change is this page's steps only"
    );
    assert!(s.activate_page(&first));
    assert_eq!(set_names(&s), ["Bench"], "the rename is there at once");
    assert_eq!(row(&s, h)["active"], true, "page 1 keeps its On the bed");
    assert_eq!(row(&s, h)["visible"], true, "and its eye");
    let clamp = row_named(&s, "Clamp 1");
    assert_eq!(
        clamp["active"], true,
        "a fixture added elsewhere starts On the bed here"
    );
    assert_eq!(s.undo_len(), steps + 3, "no step on page 1");
    // A field of a set fixture follows too.
    assert!(s.activate_page(&second));
    let pin = s.add_set_fixture(s2, "pin").unwrap();
    assert!(s.set_fixture_grid_value(pin, "columns", 3.0));
    assert!(s.activate_page(&first));
    assert_eq!(row_named(&s, "Pin 1")["round"]["columns"], 3);
}

#[test]
fn ac44_47_another_project_on_the_profile_follows_on_reload_without_an_undo_step() {
    let mut a = one();
    let set = a.add_fixture_set().unwrap();
    a.add_set_fixture(set, "holder").unwrap();
    let id = user_profile_id(&a);
    let lib = library(&mut a);
    // Project b: its own file, a laser page on the same profile.
    let mut b = generic();
    b.load_machine_profiles_json(&lib);
    b.add_output_page_with(LASER, Some(&id)).unwrap();
    assert_eq!(set_names(&b), ["Set 1"]);
    let bset = set_id(&b, "Set 1");
    assert!(b.set_fixture_set_on_page(bset, false), "b's own choice");
    b.wipe_history();
    let b_saved = b.pack("b").unwrap();
    // a renames the set and adds one; b reloads the library (window focus).
    assert!(a.rename_fixture_set(set, "Bench"));
    a.add_fixture_set().unwrap();
    let lib2 = library(&mut a);
    b.load_machine_profiles_json(&lib2);
    assert_eq!(
        set_names(&b),
        ["Bench", "Set 1"],
        "b shows the profile's sets"
    );
    assert_eq!(
        set_row(&b, "Bench")["on_page"],
        false,
        "b keeps its On this page"
    );
    assert_eq!(
        set_row(&b, "Set 1")["on_page"],
        false,
        "a new set is off here (Frage 218 A)"
    );
    assert_eq!(b.undo_len(), 0, "no undo step in b");
    // A project opened later shows the profile's current sets (criterion 47).
    let mut later = Session::open(9160, &b_saved).unwrap();
    later.load_machine_profiles_json(&lib2);
    let d = unpack(9160, &b_saved).unwrap();
    assert!(
        d.pages()
            .into_iter()
            .any(|p| later.activate_page(&p.id) && !sets(&later).is_empty())
    );
    assert_eq!(set_names(&later), ["Bench", "Set 1"]);
    assert_eq!(
        set_row(&later, "Bench")["on_page"],
        false,
        "its own choice is kept"
    );
    assert_eq!(set_row(&later, "Set 1")["on_page"], false);
    assert!(later.is_dirty(), "the project counts as changed");
    assert_eq!(later.undo_len(), 0, "without an undo step");
}

/// The newest record text of the profile `id` among the writes `json`.
fn record_of(json: &str, id: &str) -> Option<Value> {
    let writes: Vec<Value> = serde_json::from_str(json).unwrap();
    writes
        .iter()
        .rev()
        .find(|w| w["kind"] == "write" && w["id"] == id)
        .map(|w| serde_json::from_str(w["text"].as_str().unwrap()).unwrap())
}

#[test]
fn ac45_undo_takes_a_set_change_back_on_the_page_and_in_the_profile_and_redo_again() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    let id = user_profile_id(&s);
    let _ = s.take_machine_writes_json();
    assert!(s.rename_fixture_set(set, "Bench"));
    let after = record_of(&s.take_machine_writes_json(), &id).expect("the edit writes");
    assert_eq!(after["fixture_sets"][0]["name"], "Bench");
    s.undo();
    assert_eq!(set_names(&s), ["Set 1"], "undone on the page");
    let back = record_of(&s.take_machine_writes_json(), &id).expect("undo writes the profile");
    assert_eq!(
        back["fixture_sets"][0]["name"], "Set 1",
        "and in the profile"
    );
    s.redo();
    assert_eq!(set_names(&s), ["Bench"]);
    let again = record_of(&s.take_machine_writes_json(), &id).expect("redo writes it");
    assert_eq!(again["fixture_sets"][0]["name"], "Bench");
    // An undo on another page of the machine takes the change back on every page.
    let (first, _) = twin_page(&mut s);
    let set2 = set_id(&s, "Bench");
    assert!(s.rename_fixture_set(set2, "Other"));
    s.undo();
    assert_eq!(set_names(&s), ["Bench"]);
    assert!(s.activate_page(&first));
    assert_eq!(set_names(&s), ["Bench"]);
}

#[test]
fn ac45_a_profile_changed_elsewhere_is_left_and_the_page_step_is_undone_with_a_notice() {
    let mut a = one();
    let set = a.add_fixture_set().unwrap();
    let id = user_profile_id(&a);
    let lib = library(&mut a);
    // b, another project on the same profile, renames the set after a's edit.
    let mut b = generic();
    b.load_machine_profiles_json(&lib);
    b.add_output_page_with(LASER, Some(&id)).unwrap();
    assert!(a.rename_fixture_set(set, "Mine"));
    let lib_a = library(&mut a);
    b.load_machine_profiles_json(&lib_a);
    let bset = set_id(&b, "Mine");
    assert!(b.rename_fixture_set(bset, "Theirs"));
    let lib_b = library(&mut b);
    // a reloads (focus), then undoes its rename.
    a.load_machine_profiles_json(&lib_b);
    let _ = a.take_machine_writes_json();
    let n = a.undo().notice;
    let name = machine_name(&a);
    assert_eq!(
        n,
        format!("{name} changed since, its sets were not undone.")
    );
    notice_ok(&n);
    assert!(
        record_of(&a.take_machine_writes_json(), &id).is_none(),
        "the profile is left as it is"
    );
}

#[test]
fn ac45_an_edit_whose_profile_write_failed_is_undone_with_a_notice() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    let id = user_profile_id(&s);
    let steps = s.undo_len();
    assert!(s.rename_fixture_set(set, "Bench"));
    let name = machine_name(&s);
    let n = s
        .machine_record_write_failed(&id)
        .expect("the failed write undoes the edit");
    assert_eq!(n, format!("Could not save {name}. The change was undone."));
    notice_ok(&n);
    assert_eq!(set_names(&s), ["Set 1"], "the change was undone");
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac45_the_notice_cuts_a_long_profile_name_with_an_ellipsis() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    let id = user_profile_id(&s);
    let long = "A very long machine profile name for the bench";
    let _ = s.rename_machine_profile(&id, long, 2_000);
    assert!(s.rename_fixture_set(set, "Bench"));
    let n = s.machine_record_write_failed(&id).expect("the notice");
    notice_ok(&n);
    assert!(n.contains('…'), "the name is cut with an ellipsis: {n}");
}

/// The tester page with "Set 1" (a holder) and "Set 2" (a set pin) and a job pin, saved and
/// reopened on a computer whose library is `library` (the page's record as it is, missing,
/// damaged, older or newer).
fn reopened_with(library: impl FnOnce(&str, Value) -> String) -> Session {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    s.add_set_fixture(set, "holder").unwrap();
    let second = s.add_fixture_set().unwrap();
    let set_pin = s.add_set_fixture(second, "pin").unwrap();
    assert!(s.rename_fixture(set_pin, "Set pin"));
    place(&mut s, "pin");
    let id = user_profile_id(&s);
    let record = record_of(&s.take_machine_writes_json(), &id).unwrap();
    let bytes = s.pack("t").unwrap();
    let mut r = Session::open(9180, &bytes).unwrap();
    r.resize_viewport(1600.0, 1000.0);
    r.set_tool(Tool::Select);
    r.load_machine_profiles_json(&library(&id, record));
    let d = unpack(9180, &bytes).unwrap();
    assert!(
        d.pages()
            .into_iter()
            .any(|p| r.activate_page(&p.id) && !sets(&r).is_empty())
    );
    r
}

fn listing(id: &str, text: &str) -> String {
    json!([{ "name": curvyo_laser_core::record_file_name(id), "text": text }]).to_string()
}

#[test]
fn ac46_sets_of_a_missing_profile_are_read_only_but_the_page_choices_still_work() {
    let mut r = reopened_with(|_, _| "[]".to_owned());
    let name = machine_name(&r);
    let v = view(&r);
    assert_eq!(
        v["read_only"],
        format!("From the project file. {name} is not on this computer.")
    );
    assert_eq!(v["can_add_set"], false, "no + Set");
    let set = set_id(&r, "Set 1");
    assert_eq!(set_row(&r, "Set 1")["can_add"], false, "no add bar");
    let h = node(&r, &row_named(&r, "Holder 1"));
    let drawn = tri(&r);
    assert!(drawn > 0, "the stored sets are drawn");
    // Every control that changes a set's contents is refused.
    assert!(!r.rename_fixture_set(set, "X"));
    assert!(r.add_set_fixture(set, "clamp").is_none());
    let set_pin = node(&r, &row_named(&r, "Set pin"));
    assert!(!r.move_fixture_set(set, 1), "no order change");
    assert!(!r.set_fixture_set_locked(set, true));
    assert!(!r.set_fixture_locked(h, true));
    assert!(
        !r.set_fixture_grid_value(set_pin, "columns", 2.0),
        "no field commit"
    );
    assert!(!r.rename_fixture(h, "X"), "no fixture rename");
    assert!(!r.remove_fixture_set(set));
    assert!(!r.delete_fixture(h), "no trash on a read-only set fixture");
    assert!(r.add_slot(h).is_none(), "no slot");
    let before = centre(&r, h);
    let (from, to) = (at(&r, 150.0, 150.0), at(&r, 110.0, 120.0));
    drag(&mut r, from, to);
    assert_eq!(
        centre(&r, h),
        before,
        "a read-only set fixture keeps its place"
    );
    assert_eq!(set_names(&r), ["Set 1", "Set 2"]);
    assert_eq!(row(&r, set_pin)["round"]["columns"], 1);
    assert_eq!(all_rows(&r).len(), 3);
    // The page's choices still work.
    assert!(r.set_fixture_active(h, false), "On the bed");
    assert!(r.set_fixture_visible(h, false), "the eye");
    assert!(r.set_fixture_set_on_page(set, false), "On this page");
    assert!(r.set_fixture_set_on_page(set, true));
    // Job fixtures stay fully editable.
    let pin = node(&r, &row_named(&r, "Pin 1"));
    assert!(r.set_fixture_grid_value(pin, "columns", 2.0));
    assert!(r.set_fixture_locked(pin, true));
    place(&mut r, "screw");
    // Save as profile makes the sets editable here.
    let _ = r.save_machine_profile([41; 16], 3_000);
    assert_eq!(view(&r)["read_only"], Value::Null);
    assert!(r.rename_fixture_set(set, "Now mine"));
    assert_eq!(set_names(&r), ["Now mine", "Set 2"]);
}

#[test]
fn ac46_sets_of_a_damaged_older_or_newer_profile_are_read_only() {
    type Bend = fn(Value) -> String;
    let cases: [(&str, Bend); 3] = [
        ("damaged", |mut r| {
            r["fixture_sets"][0]["active"] = json!(true);
            r.to_string()
        }),
        ("older", |mut r| {
            r["format_version"] = json!(4);
            r.to_string()
        }),
        ("newer", |mut r| {
            r["format_version"] = json!(99);
            r.to_string()
        }),
    ];
    for (what, bend) in cases {
        let r = reopened_with(|id, record| listing(id, &bend(record)));
        let v = view(&r);
        assert!(
            v["read_only"]
                .as_str()
                .is_some_and(|t| t.starts_with("From the project file.")),
            "{what}: read only: {}",
            v["read_only"]
        );
        assert_eq!(v["can_add_set"], false, "{what}");
        assert_eq!(
            set_names(&r),
            ["Set 1", "Set 2"],
            "{what}: the stored sets are shown"
        );
    }
}

#[test]
fn ac47_a_profile_that_is_not_newer_shows_the_projects_sets_and_writes_nothing() {
    let r = reopened_with(|id, record| listing(id, &record.to_string()));
    assert_eq!(view(&r)["read_only"], Value::Null, "editable");
    assert_eq!(set_names(&r), ["Set 1", "Set 2"]);
    assert!(!r.is_dirty(), "nothing is written");
    assert_eq!(r.undo_len(), 0);
}

#[test]
fn ac47_a_newer_profile_replaces_the_stored_sets_keeping_the_page_choices() {
    let mut s = one();
    let set = s.add_fixture_set().unwrap();
    let h = s.add_set_fixture(set, "holder").unwrap();
    assert!(s.set_fixture_active(h, false), "the page's own On the bed");
    let bytes = s.pack("t").unwrap();
    s.add_set_fixture(set, "clamp").unwrap();
    assert!(s.rename_fixture_set(set, "Bench"));
    s.add_fixture_set().unwrap();
    let lib = library(&mut s);
    let mut r = Session::open(9190, &bytes).unwrap();
    r.load_machine_profiles_json(&lib);
    let d = unpack(9190, &bytes).unwrap();
    assert!(
        d.pages()
            .into_iter()
            .any(|p| r.activate_page(&p.id) && !sets(&r).is_empty())
    );
    assert_eq!(set_names(&r), ["Bench", "Set 1"], "the profile's sets");
    assert_eq!(set_row(&r, "Bench")["on_page"], true);
    assert_eq!(
        set_row(&r, "Set 1")["on_page"],
        false,
        "a new set is not on this page"
    );
    assert_eq!(row(&r, h)["active"], false, "the page keeps its own choice");
    assert_eq!(
        row_named(&r, "Clamp 1")["active"],
        true,
        "a new fixture is on"
    );
    assert!(r.is_dirty(), "the project counts as changed");
    assert_eq!(r.undo_len(), 0, "without an undo step");
}

#[test]
fn ac40_a_record_with_a_missing_or_repeated_id_or_an_active_key_is_skipped() {
    for golden in [
        "format5_missing_id.json",
        "format5_duplicate_id.json",
        "format5_active_key.json",
    ] {
        let mut s = generic();
        let name = curvyo_laser_core::record_file_name("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
        s.load_machine_profiles_json(&listing_named(&name, &profile_golden(golden)));
        let m = machine(&s);
        assert!(
            m["profiles"]
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p["builtin"] == true),
            "{golden}: skipped: {m}"
        );
        assert_eq!(
            m["library_notices"].as_array().map_or(0, Vec::len),
            1,
            "{golden}: its notice: {m}"
        );
        let n = m["library_notices"][0].as_str().unwrap_or_default();
        notice_ok(n);
    }
}

fn listing_named(name: &str, text: &str) -> String {
    json!([{ "name": name, "text": text }]).to_string()
}

#[test]
fn ac38_the_golden_v47_holds_a_locked_set_and_a_locked_set_fixture() {
    let s = open_golden("fixture_sets_v47.curvyo");
    assert!(
        sets(&s).iter().any(|r| r["locked"] == true),
        "a locked set: {}",
        view(&s)
    );
    assert!(
        sets(&s)
            .iter()
            .flat_map(|r| r["fixtures"].as_array().cloned().unwrap_or_default())
            .any(|f| f["locked"] == true && f["locked_by_set"] == false),
        "a set fixture locked by its own Lock"
    );
}
