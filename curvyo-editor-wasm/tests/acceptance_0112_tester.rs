//! Tester acceptance cases for `specs/0112-test-grid-and-materials`, written from the
//! specification before the implementation was read: the Test grid tool and its bar (criteria 1
//! to 7), copies and deletes (8, 13), the test id (9, 11), the derived operations and their
//! regeneration over hand edits (10, 12, 14), the Materials tab with sets per laser (16 to 21),
//! tests, judging, note and photo (22 to 28), save and reopen, older files and peers (29, 30).
//! Black box through the public session; the Lasered hook stands in for `0029`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    clippy::cloned_ref_to_slice_refs,
    clippy::assert_is_empty,
    missing_docs
)]

use std::collections::HashSet;
use std::io::{Cursor, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Direction, Document, DocumentSize,
    NewOperation, NewOutputPage, NodeId, OperationSnapshot, OperationSwitch, Page, Point,
    RectBounds, Timestamp, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_laser_core::{MachineProfile, new_profile_id, record_file_name, write_record};
use curvyo_ui_core::FontBlock;
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";

// ---- fixture ----------------------------------------------------------------------------------

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fonts_dir() -> PathBuf {
    manifest().join("../frontend/public/fonts")
}

fn material_golden(name: &str) -> String {
    std::fs::read_to_string(
        manifest()
            .join("../curvyo-laser-core/tests/fixtures/materials")
            .join(name),
    )
    .unwrap()
}

/// A drawing page and a 400 x 300 mm laser page on Generic laser; the laser page's id.
fn base_doc() -> (Document, String) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    (d, laser.id)
}

fn load_fonts(s: &mut Session) {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect();
    s.set_font_list(FontBlock::Included, faces, Vec::new());
    s.set_tool(Tool::TestGrid);
    let _ = s.test_grid_press(None);
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
    s.set_tool(Tool::Select);
}

fn open_bytes(peer: u64, bytes: &[u8], laser: &str) -> Session {
    let mut s = Session::open(peer, bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    let mut snap = s.snapping();
    snap.enabled = false;
    s.set_snapping(snap);
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(laser));
    load_fonts(&mut s);
    s.load_materials(&[], None);
    s.wipe_history();
    s
}

fn session() -> Session {
    let (d, laser) = base_doc();
    open_bytes(5, &pack(&d, "0112-tester").unwrap(), &laser)
}

fn page(s: &Session) -> Page {
    s.active_page()
}

/// Places a grid with the bar's values with its top left at `(x, y)` mm from the page origin.
fn place_at(s: &mut Session, x: f64, y: f64) -> NodeId {
    let o = page(s).origin;
    s.set_tool(Tool::TestGrid);
    let g = s
        .test_grid_press(Some(Point::new(o.x + x, o.y + y)))
        .unwrap();
    s.set_tool(Tool::Select);
    g
}

fn place(s: &mut Session) -> NodeId {
    place_at(s, 10.0, 10.0)
}

fn grid_view(s: &Session) -> Value {
    serde_json::from_str(&s.test_grid_view_json()).unwrap()
}

fn grid_id(s: &Session) -> String {
    grid_view(s)["id"].as_str().unwrap().to_owned()
}

fn materials_view(s: &Session) -> Value {
    serde_json::from_str(&s.materials_view_json()).unwrap()
}

fn writes(s: &mut Session) -> Vec<Value> {
    let v: Value = serde_json::from_str(&s.take_material_writes_json()).unwrap();
    v.as_array().unwrap().clone()
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn all_ops(s: &Session) -> Vec<OperationSnapshot> {
    let d = doc(s);
    let p = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    d.operations(p.kind.manufacturing().unwrap()).unwrap()
}

fn derived(s: &Session) -> Vec<OperationSnapshot> {
    all_ops(s)
        .into_iter()
        .filter(|o| o.grid_part.is_some())
        .collect()
}

fn cells(s: &Session) -> Vec<OperationSnapshot> {
    derived(s)
        .into_iter()
        .filter(|o| o.grid_part.as_ref().unwrap()["part"] == "cell")
        .collect()
}

fn labels(s: &Session) -> Vec<OperationSnapshot> {
    derived(s)
        .into_iter()
        .filter(|o| o.grid_part.as_ref().unwrap()["part"] == "labels")
        .collect()
}

fn main_num(op: &OperationSnapshot, key: &str) -> f64 {
    op.settings.as_ref().unwrap()["main"][key].as_f64().unwrap()
}

fn undo_label(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

/// The derived rows as comparable text (name, kind, settings, options, part).
fn fingerprint(s: &Session) -> Vec<String> {
    derived(s)
        .iter()
        .map(|o| {
            format!(
                "{}|{:?}|{:?}|{:?}|{:?}|{}|{}",
                o.name, o.kind, o.settings, o.options, o.grid_part, o.show, o.output
            )
        })
        .collect()
}

fn tests_of(s: &mut Session, material: &str) -> Vec<Value> {
    s.unfold_material(Some(material));
    materials_view(s)["unfolded"]["tests"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn test_ids_of(s: &mut Session, material: &str) -> Vec<String> {
    tests_of(s, material)
        .iter()
        .map(|t| t["id"].as_str().unwrap().to_owned())
        .collect()
}

/// A PNG header of `width` x `height`.
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

fn photo_golden() -> Vec<u8> {
    std::fs::read(
        manifest().join("../curvyo-laser-core/tests/fixtures/materials/photos/7k2qx_600.jpg"),
    )
    .unwrap()
}

/// A session with a grid picked to "Birch 3 mm"; (session, grid, material id, test id).
fn birch() -> (Session, NodeId, String, String) {
    let mut s = session();
    let b = s.add_material("Birch 3 mm", 3.0, &[3; 16]).unwrap();
    let g = place(&mut s);
    assert_eq!(s.pick_test_grid_material(Some(&b)), Ok(true));
    let id = grid_id(&s);
    let _ = writes(&mut s);
    (s, g, b, id)
}

fn second_laser() -> MachineProfile {
    let mut p = MachineProfile::generic_laser();
    p.id = new_profile_id([7; 16]);
    "Diode 20 W".clone_into(&mut p.values.name);
    p
}

fn load_second_laser(s: &mut Session) -> String {
    let p = second_laser();
    let lib = json!([{ "name": record_file_name(&p.id), "text": write_record(&p) }]);
    s.load_machine_profiles_json(&lib.to_string());
    p.id
}

/// The box of grid `g`: the union of its leaves' boxes.
fn grid_bounds(s: &Session, g: NodeId) -> (Point, Point) {
    let d = doc(s);
    let mut min = Point::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for leaf in d.leaves_of(g) {
        if let Some(o) = d.object(leaf) {
            let (a, b) = curvyo_ui_core::object_bounds(&o);
            min = Point::new(min.x.min(a.x), min.y.min(a.y));
            max = Point::new(max.x.max(b.x), max.y.max(b.y));
        }
    }
    assert!(min.x.is_finite(), "grid {g:?} has leaves");
    (min, max)
}

/// The test id of grid `g`, read by selecting it with a press of the tool on its box.
fn id_of(s: &mut Session, g: NodeId) -> String {
    let (min, _) = grid_bounds(s, g);
    s.set_tool(Tool::TestGrid);
    let got = s.test_grid_press(Some(Point::new(min.x + 15.0, min.y + 15.0)));
    s.set_tool(Tool::Select);
    assert_eq!(got, Ok(g), "a press on the grid selects it");
    grid_id(s)
}

/// Drags the selection from `from` by `(dx, dy)` with the Select tool.
fn drag(s: &mut Session, from: Point, dx: f64, dy: f64) {
    let to = Point::new(from.x + dx, from.y + dy);
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    for i in 1..=8 {
        let t = f64::from(i) / 8.0;
        s.pointer_hover(Point::new(from.x + dx * t, from.y + dy * t), false, false);
    }
    s.pointer_up(to, false, false);
}

// ---- criterion 1: the tool -------------------------------------------------------------------

#[test]
fn c1_test_grid_tool_only_on_laser_pages() {
    let mut s = session();
    assert!(s.test_grid_tool_available());
    assert!(s.activate_page("first"));
    assert!(!s.test_grid_tool_available(), "dimmed on a drawing page");
}

// ---- criterion 2: placing --------------------------------------------------------------------

#[test]
fn c2_a_press_places_one_grid_in_one_step_selected() {
    let mut s = session();
    let before = s.undo_len();
    let g = place(&mut s);
    assert_eq!(s.undo_len(), before + 1);
    assert_eq!(undo_label(&mut s), "Add test grid");
    assert!(s.select_bar_hides_grid_commands(), "the grid is selected");
    let v = grid_view(&s);
    assert_eq!(v["kind"], "cut");
    assert_eq!(v["cell_mm"], 10.0);
    assert_eq!(v["columns"]["count"], 5);
    assert_eq!(v["rows"]["count"], 5);
    assert_eq!(derived(&s).len(), 26);
    // Undo removes grid and rows together.
    let _ = s.undo();
    assert_eq!(derived(&s).len(), 0);
    let _ = s.redo();
    assert_eq!(derived(&s).len(), 26);
    let _ = g;
}

#[test]
fn c2_a_press_that_does_not_fit_falls_back_to_a_free_spot() {
    let mut s = session();
    // Bottom right corner: a 76 x 66 mm grid does not fit there on a 400 x 300 bed.
    let g = place_at(&mut s, 395.0, 295.0);
    assert_eq!(derived(&s).len(), 26);
    let (min, max) = grid_bounds(&s, g);
    let o = page(&s).origin;
    let tol = 0.01;
    assert!(min.x >= o.x - tol && min.y >= o.y - tol, "{min:?}");
    assert!(
        max.x <= o.x + 400.0 + tol && max.y <= o.y + 300.0 + tol,
        "{max:?}"
    );
}

#[test]
fn c2_bar_edits_live_one_step_per_committed_value() {
    let mut s = session();
    place(&mut s);
    let n = s.undo_len();
    assert_eq!(s.set_test_grid_field("columns_count", "6"), Ok(true));
    assert_eq!(s.undo_len(), n + 1);
    assert_eq!(undo_label(&mut s), "Edit test grid");
    assert_eq!(cells(&s).len(), 30);
    assert_eq!(s.set_test_grid_field("rows_count", "3"), Ok(true));
    assert_eq!(s.undo_len(), n + 2);
    assert_eq!(cells(&s).len(), 18);
    // The stored value again: no step.
    assert_eq!(s.set_test_grid_field("rows_count", "3"), Ok(false));
    assert_eq!(s.undo_len(), n + 2);
    let _ = s.undo();
    assert_eq!(cells(&s).len(), 30);
}

// ---- criteria 3 to 6: the bar ---------------------------------------------------------------

#[test]
fn c3_ranges_of_count_and_cell() {
    let mut s = session();
    place(&mut s);
    for bad in ["1", "13", "0", "abc", "5.5"] {
        assert!(
            s.set_test_grid_field("columns_count", bad).is_err(),
            "{bad}"
        );
    }
    for bad in ["2.9", "50.1", "0", "-3"] {
        assert!(s.set_test_grid_field("cell", bad).is_err(), "{bad}");
    }
    assert_eq!(s.set_test_grid_field("columns_count", "2"), Ok(true));
    assert_eq!(s.set_test_grid_field("columns_count", "12"), Ok(true));
    assert_eq!(s.set_test_grid_field("cell", "3"), Ok(true));
    assert_eq!(s.set_test_grid_field("rows_count", "12"), Ok(true));
    assert_eq!(cells(&s).len(), 144);
    assert_eq!(derived(&s).len(), 145);
}

#[test]
fn c3_gap_is_constant_2_mm() {
    let mut s = session();
    place(&mut s);
    let w10 = grid_view(&s)["width_mm"].as_f64().unwrap();
    s.set_test_grid_field("cell", "20").unwrap();
    let w20 = grid_view(&s)["width_mm"].as_f64().unwrap();
    // Five cells grow by 10 mm each; the gaps stay.
    assert!((w20 - w10 - 50.0).abs() < 1e-6, "{w10} {w20}");
    s.set_test_grid_field("columns_count", "6").unwrap();
    let w6 = grid_view(&s)["width_mm"].as_f64().unwrap();
    assert!(
        (w6 - w20 - 22.0).abs() < 1e-6,
        "one cell of 20 plus a 2 mm gap: {w20} {w6}"
    );
}

#[test]
fn c3_engrave_has_crosshatch_cut_has_none() {
    let mut s = session();
    place(&mut s);
    assert!(grid_view(&s)["crosshatch"].is_null());
    assert_eq!(s.set_test_grid_kind("engrave"), Ok(true));
    assert_eq!(grid_view(&s)["crosshatch"], false);
    assert_eq!(s.set_test_grid_crosshatch(true), Ok(true));
    assert_eq!(grid_view(&s)["crosshatch"], true);
    let c = &cells(&s)[0];
    assert_eq!(c.kind.as_deref(), Some("engrave"));
    let opts = format!("{:?}", c.options);
    assert!(opts.contains("crosshatch"), "{opts}");
}

#[test]
fn c4_offered_parameters_and_swap() {
    let mut s = session();
    place(&mut s);
    let v = grid_view(&s);
    for side in ["columns", "rows"] {
        let offered: Vec<&str> = v[side]["offered"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect();
        assert_eq!(offered, ["power", "speed", "passes"], "{side}");
    }
    // Choosing the other axis's parameter swaps.
    assert_eq!(s.set_test_grid_axis("columns", "speed"), Ok(true));
    let v = grid_view(&s);
    assert_eq!(v["columns"]["param"], "speed");
    assert_eq!(v["rows"]["param"], "power");
    // The freed parameter moves to row 1 (shared).
    assert_eq!(s.set_test_grid_axis("columns", "passes"), Ok(true));
    let v = grid_view(&s);
    assert_eq!(v["columns"]["param"], "passes");
    assert_eq!(v["rows"]["param"], "power");
    let shared: Vec<&str> = v["shared"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["param"].as_str().unwrap())
        .collect();
    assert_eq!(shared, ["speed"]);
    // Engrave offers Interval on both axes.
    s.set_test_grid_kind("engrave").unwrap();
    let v = grid_view(&s);
    for side in ["columns", "rows"] {
        let offered = v[side]["offered"].to_string();
        assert!(offered.contains("interval"), "{side} {offered}");
    }
    // Interval is not offered on a Cut grid: a switch to it is refused or changes nothing.
    s.set_test_grid_kind("cut").unwrap();
    let r = s.set_test_grid_axis("rows", "interval");
    let v = grid_view(&s);
    assert!(
        v["rows"]["param"] != "interval" && v["columns"]["param"] != "interval",
        "{r:?} {v}"
    );
    assert!(r.is_err() || r == Ok(false), "{r:?}");
}

#[test]
fn c5_out_of_range_kept_marked_grid_unchanged() {
    let mut s = session();
    place(&mut s);
    let before = fingerprint(&s);
    let n = s.undo_len();
    let err = s.set_test_grid_field("columns_max", "150").unwrap_err();
    assert!(!err.is_empty() && err.chars().count() <= 70, "{err}");
    let v = grid_view(&s);
    assert!(!v["invalid"].is_null(), "the value is marked");
    assert_eq!(fingerprint(&s), before);
    assert_eq!(s.undo_len(), n);
}

#[test]
fn c5_a_grid_wider_than_the_bed_names_the_cause() {
    let mut s = session();
    place(&mut s);
    s.set_test_grid_field("cell", "40").unwrap();
    let before = fingerprint(&s);
    let n = s.undo_len();
    let err = s.set_test_grid_field("columns_count", "12").unwrap_err();
    assert!(
        err.starts_with("Grid is ") && err.contains("mm wide, the bed is 400 mm."),
        "{err}"
    );
    assert_eq!(fingerprint(&s), before, "not clamped, unchanged");
    assert_eq!(s.undo_len(), n);
    assert!(!grid_view(&s)["fit"].is_null() || !grid_view(&s)["invalid"].is_null());
}

#[test]
fn c6_defaults_of_min_and_max() {
    let mut s = session();
    place(&mut s);
    let v = grid_view(&s);
    assert_eq!(v["columns"]["param"], "power");
    assert_eq!(v["columns"]["min"], 20.0);
    assert_eq!(v["columns"]["max"], 100.0);
    assert_eq!(v["rows"]["param"], "speed");
    let feed = v["max_feed_mm_min"].as_f64().unwrap();
    assert_eq!(v["rows"]["min"].as_f64().unwrap(), (feed * 0.1).round());
    assert_eq!(v["rows"]["max"].as_f64().unwrap(), feed);
    s.set_test_grid_kind("engrave").unwrap();
    s.set_test_grid_axis("rows", "interval").unwrap();
    let v = grid_view(&s);
    assert_eq!(v["rows"]["param"], "interval");
    assert!((v["rows"]["min"].as_f64().unwrap() - 0.05).abs() < 1e-9);
    assert!((v["rows"]["max"].as_f64().unwrap() - 0.25).abs() < 1e-9);
}

#[test]
fn c6_material_list_in_the_pick() {
    let mut s = session();
    let a = s.add_material("Acrylic", 3.0, &[1; 16]).unwrap();
    let b = s.add_material("Birch", 4.0, &[2; 16]).unwrap();
    place(&mut s);
    let ids: HashSet<String> = grid_view(&s)["materials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap().to_owned())
        .collect();
    assert!(ids.contains(&a) && ids.contains(&b));
    assert!(s.pick_test_grid_material(Some("no-such-material")).is_err());
}

// ---- criterion 7: moves, no resize -----------------------------------------------------------

#[test]
fn c7_the_grid_moves_only_and_its_size_follows_its_fields() {
    let mut s = session();
    let g = place(&mut s);
    // No resize handle: a drag on the box's corner never changes the grid's size.
    let (a, b) = grid_bounds(&s, g);
    drag(&mut s, b, 40.0, 40.0);
    let (a2, b2) = grid_bounds(&s, g);
    assert!(
        ((b2.x - a2.x) - (b.x - a.x)).abs() < 0.01 && ((b2.y - a2.y) - (b.y - a.y)).abs() < 0.01,
        "size kept: {a:?} {b:?} -> {a2:?} {b2:?}"
    );
    if s.undo_len() > 0 && (a2.x - a.x).abs() > 0.01 {
        let _ = s.undo();
    }
    let (min0, _) = grid_bounds(&s, g);
    let from = Point::new(min0.x + 1.0, min0.y + 30.0);
    drag(&mut s, from, 20.0, 10.0);
    let (min1, _) = grid_bounds(&s, g);
    assert!((min1.x - min0.x - 20.0).abs() < 0.01, "{min0:?} {min1:?}");
    assert!((min1.y - min0.y - 10.0).abs() < 0.01);
    assert_eq!(derived(&s).len(), 26, "moving on the page keeps the rows");
}

// ---- criteria 8 and 13: copies, deletes, ungroup, off the page ------------------------------

#[test]
fn c8_a_copy_is_a_new_grid_with_a_new_id_and_its_own_rows() {
    let mut s = session();
    let g = place(&mut s);
    let id = grid_id(&s);
    let _ = s.duplicate_layer_target();
    let copy_id = grid_id(&s);
    assert_ne!(copy_id, id);
    assert!(curvyo_laser_core::grids::TestId::parse(&copy_id).is_some());
    let d = derived(&s);
    assert_eq!(d.len(), 52);
    let names: HashSet<&str> = d.iter().map(|o| o.name.as_str()).collect();
    assert!(names.contains(format!("{copy_id} labels").as_str()));
    assert!(names.contains(format!("{id} labels").as_str()));
    let grids: HashSet<String> = d
        .iter()
        .map(|o| o.grid_part.as_ref().unwrap()["grid"].to_string())
        .collect();
    assert_eq!(grids.len(), 2);
    let _ = g;
}

#[test]
fn c13_delete_removes_the_rows_in_the_same_step_undo_returns_them() {
    let (mut s, _g, b, id) = birch();
    let n = s.undo_len();
    s.delete_selected();
    assert_eq!(s.undo_len(), n + 1);
    assert_eq!(derived(&s).len(), 0);
    assert_eq!(
        test_ids_of(&mut s, &b),
        [id.clone()],
        "the test stays in the material"
    );
    let _ = s.undo();
    assert_eq!(derived(&s).len(), 26);
}

#[test]
fn c13_ungroup_removes_the_rows() {
    let mut s = session();
    place(&mut s);
    let n = s.undo_len();
    let _ = s.ungroup_selection();
    assert_eq!(s.undo_len(), n + 1);
    assert_eq!(derived(&s).len(), 0);
    let _ = s.undo();
    assert_eq!(derived(&s).len(), 26);
}

#[test]
fn c13_a_grid_dragged_off_the_laser_page_loses_its_rows() {
    let mut s = session();
    let g = place(&mut s);
    let (min0, _) = grid_bounds(&s, g);
    let n = s.undo_len();
    drag(
        &mut s,
        Point::new(min0.x + 1.0, min0.y + 30.0),
        0.0,
        -2000.0,
    );
    let (min1, _) = grid_bounds(&s, g);
    assert!(min1.y < min0.y - 1000.0, "dragged: {min0:?} {min1:?}");
    assert_eq!(s.undo_len(), n + 1, "one step");
    assert_eq!(derived(&s).len(), 0);
    let _ = s.undo();
    assert_eq!(derived(&s).len(), 26);
}

// ---- criterion 9: ids -----------------------------------------------------------------------

#[test]
fn c9_every_new_grid_gets_a_unique_valid_id() {
    let mut s = session();
    let mut ids = HashSet::new();
    for (x, y) in [
        (5.0, 5.0),
        (100.0, 5.0),
        (200.0, 5.0),
        (5.0, 100.0),
        (100.0, 100.0),
    ] {
        place_at(&mut s, x, y);
        let id = grid_id(&s);
        assert!(
            curvyo_laser_core::grids::TestId::parse(&id).is_some(),
            "{id}"
        );
        assert!(ids.insert(id.clone()), "duplicate id {id}");
    }
}

#[test]
fn c9_a_retired_id_is_never_assigned() {
    // Learn the id this session would draw first, then retire it in a fresh one.
    let first = {
        let mut s = session();
        place(&mut s);
        grid_id(&s)
    };
    let mut s = session();
    s.load_retired_test_ids(Some(&format!(
        "{{\n  \"format_version\": 1,\n  \"retired\": [\n    \"{first}\"\n  ]\n}}\n"
    )));
    place(&mut s);
    assert_ne!(grid_id(&s), first);
}

#[test]
fn c9_a_live_id_in_the_store_is_never_assigned() {
    let first = {
        let mut s = session();
        place(&mut s);
        grid_id(&s)
    };
    // A record that already holds a test with that id.
    let mut v: Value = serde_json::from_str(&material_golden("format2_full.json")).unwrap();
    v["tests"][0]["id"] = json!(first);
    for laser in v["lasers"].as_array_mut().unwrap() {
        for (_, set) in laser["sets"].as_object_mut().unwrap() {
            if set["source"].get("test").is_some() {
                set["source"] = json!({ "test": first });
            }
        }
    }
    let id = v["id"].as_str().unwrap().to_owned();
    let text = serde_json::to_string_pretty(&v).unwrap();
    let mut s = session();
    s.load_materials(&[(format!("{id}.json"), text)], None);
    assert_eq!(materials_view(&s)["rows"][0]["state"], "loaded");
    place(&mut s);
    assert_ne!(grid_id(&s), first);
}

#[test]
fn c9_peers_with_the_same_id_both_keep_their_grids() {
    let (d, laser) = base_doc();
    let bytes = pack(&d, "base").unwrap();
    let mut a = open_bytes(11, &bytes, &laser);
    let mut b = open_bytes(12, &bytes, &laser);
    let random = [42u8; 64];
    a.feed_test_grid_random(&random);
    b.feed_test_grid_random(&random);
    place_at(&mut a, 5.0, 5.0);
    place_at(&mut b, 200.0, 150.0);
    let (ia, ib) = (grid_id(&a), grid_id(&b));
    assert_eq!(ia, ib, "same random bytes, same id before they sync");
    let merged = merged(&doc(&a), &doc(&b));
    let p = merged
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    let ops = merged.operations(p.kind.manufacturing().unwrap()).unwrap();
    let derived: Vec<_> = ops.iter().filter(|o| o.grid_part.is_some()).collect();
    assert_eq!(derived.len(), 52, "both grids keep their rows");
    let grids: HashSet<String> = derived
        .iter()
        .map(|o| o.grid_part.as_ref().unwrap()["grid"].to_string())
        .collect();
    assert_eq!(grids.len(), 2);
    // The merged file opens in a session and saves.
    let m = Session::open(13, &pack(&merged, "m").unwrap()).unwrap();
    assert!(m.pack("m").is_ok());
}

fn merged(a: &Document, b: &Document) -> Document {
    let loro = loro::LoroDoc::new();
    loro.import(&a.export_loro_snapshot().unwrap()).unwrap();
    loro.import(&b.export_loro_snapshot().unwrap()).unwrap();
    loro.commit();
    let loro_bytes = loro.export(loro::ExportMode::Snapshot).unwrap();
    let manifest = json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester-merge",
    });
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    w.start_file("manifest.json", o).unwrap();
    w.write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    w.start_file("document.loro", o).unwrap();
    w.write_all(&loro_bytes).unwrap();
    w.start_file("document.json", o).unwrap();
    w.write_all(b"{}").unwrap();
    unpack(9, &w.finish().unwrap().into_inner()).expect("merged opens")
}

// ---- criterion 10: the burned labels ----------------------------------------------------------

#[test]
fn c10_labels_are_one_engrave_operation_with_default_numbers() {
    let mut s = session();
    place(&mut s);
    let id = grid_id(&s);
    let l = labels(&s);
    assert_eq!(l.len(), 1);
    let l = &l[0];
    assert_eq!(l.kind.as_deref(), Some("engrave"));
    assert_eq!(l.name, format!("{id} labels"));
    assert_eq!(main_num(l, "power_pct"), 20.0);
    assert_eq!(main_num(l, "speed_mm_min"), 3000.0);
    // id + 2 titles + 5 + 5 values at least.
    assert!(l.members.len() >= 13, "{}", l.members.len());
}

#[test]
fn c10_labels_take_the_engrave_set_of_the_picked_material() {
    let mut s = session();
    let b = s.add_material("Birch", 3.0, &[3; 16]).unwrap();
    s.add_material_set(&b, "generic-laser", "engrave", &[4; 16])
        .unwrap();
    s.edit_material_set(&b, "generic-laser", "engrave", "power", 33.0, &[5; 16])
        .unwrap();
    s.edit_material_set(&b, "generic-laser", "engrave", "speed", 4100.0, &[6; 16])
        .unwrap();
    place(&mut s);
    s.pick_test_grid_material(Some(&b)).unwrap();
    let l = &labels(&s)[0];
    assert_eq!(main_num(l, "power_pct"), 33.0);
    assert_eq!(main_num(l, "speed_mm_min"), 4100.0);
}

// ---- criterion 11: Lasered gives a new id ------------------------------------------------------

#[test]
fn c11_not_lasered_edits_keep_the_id() {
    let mut s = session();
    place(&mut s);
    let id = grid_id(&s);
    s.set_test_grid_field("cell", "12").unwrap();
    assert_eq!(grid_id(&s), id);
    let cell = cells(&s)[0].id;
    s.edit_operation_setting(cell, "main", "power", 55.0)
        .unwrap();
    assert_eq!(grid_id(&s), id);
}

#[test]
fn c11_lasered_bar_change_gives_a_new_id_old_test_stays() {
    let (mut s, g, b, id) = birch();
    s.mark_test_grid_lasered_for_tests(g, 1_762_948_800_000)
        .unwrap();
    assert_eq!(grid_view(&s)["lasered"], true);
    s.set_test_grid_field("cell", "12").unwrap();
    let new = grid_id(&s);
    assert_ne!(new, id, "new id at once");
    assert_eq!(grid_view(&s)["lasered"], false);
    // The label and the row names carry the new id.
    assert_eq!(labels(&s)[0].name, format!("{new} labels"));
    assert!(cells(&s).iter().all(|c| c.name.ends_with(&new)));
    // The old test keeps its record, Lasered, with its old recipe (cell 10).
    let tests = tests_of(&mut s, &b);
    let old = tests
        .iter()
        .find(|t| t["id"] == id.as_str())
        .expect("old test kept");
    assert_eq!(old["lasered"], true);
    s.open_material_test(Some(&id));
    let detail = materials_view(&s)["unfolded"]["test"].clone();
    assert!(
        detail["axes_line"].as_str().unwrap().contains("cell 10 mm"),
        "{detail}"
    );
}

#[test]
fn c11_lasered_hand_edits_that_count_and_those_that_do_not() {
    // Rename, Show and Output keep the id.
    {
        let mut s = session();
        let g = place(&mut s);
        s.mark_test_grid_lasered_for_tests(g, 1).unwrap();
        let id = grid_id(&s);
        let cell = cells(&s)[0].id;
        assert!(s.rename_operation(cell, "My cell"));
        assert!(s.set_operation_switch(cell, OperationSwitch::Show, false));
        assert!(s.set_operation_switch(cell, OperationSwitch::Output, false));
        assert_eq!(grid_id(&s), id, "rename, Show, Output keep the id");
    }
    // Settings.
    {
        let mut s = session();
        let g = place(&mut s);
        s.mark_test_grid_lasered_for_tests(g, 1).unwrap();
        let id = grid_id(&s);
        let cell = cells(&s)[0].id;
        assert_eq!(
            s.edit_operation_setting(cell, "main", "power", 55.0),
            Ok(true)
        );
        assert_ne!(grid_id(&s), id, "a settings edit gives a new id");
    }
    // Options (an Engrave grid's Crosshatch on a cell row).
    {
        let mut s = session();
        let g = place(&mut s);
        s.set_test_grid_kind("engrave").unwrap();
        s.mark_test_grid_lasered_for_tests(g, 1).unwrap();
        let id = grid_id(&s);
        let cell = cells(&s)[0].id;
        assert_eq!(s.edit_operation_option(cell, "scan_angle", 45.0), Ok(true));
        assert_ne!(grid_id(&s), id, "an options edit gives a new id");
    }
    // Material.
    {
        let mut s = session();
        let b = s.add_material("Birch", 3.0, &[3; 16]).unwrap();
        s.add_material_set(&b, "generic-laser", "cut", &[4; 16])
            .unwrap();
        let g = place(&mut s);
        s.mark_test_grid_lasered_for_tests(g, 1).unwrap();
        let id = grid_id(&s);
        let cell = cells(&s)[0].id;
        assert!(s.use_material_on(&[cell], &b));
        assert_ne!(grid_id(&s), id, "a material edit gives a new id");
    }
    // Membership: a rectangle added to a cell row.
    {
        let (d, laser) = base_doc();
        let lp = d.pages().into_iter().find(|p| p.id == laser).unwrap();
        let r = d.create_rect(RectBounds::from_corners(
            Point::new(lp.origin.x + 300.0, lp.origin.y + 200.0),
            Point::new(lp.origin.x + 320.0, lp.origin.y + 210.0),
        ));
        let mut s = open_bytes(5, &pack(&d, "m").unwrap(), &laser);
        let g = place(&mut s);
        s.mark_test_grid_lasered_for_tests(g, 1).unwrap();
        let id = grid_id(&s);
        let cell = cells(&s)[0].id;
        // Select the rectangle by a click on its edge.
        let at = Point::new(lp.origin.x + 305.0, lp.origin.y + 200.0);
        s.pointer_hover(at, false, false);
        s.pointer_down(at, false);
        s.pointer_up(at, false, false);
        assert_eq!(s.selected_object_count(), 1);
        assert!(s.set_selection_in_operation(cell, true));
        let _ = r;
        // The grid is not the selection now; read its id from the document.
        let d = doc(&s);
        let text = format!("{:?}", d.object(g));
        assert!(
            !text.contains(&id),
            "membership edit gives a new id: {text}"
        );
    }
}

// ---- criterion 12: derived operations -------------------------------------------------------

#[test]
fn c12_a_5_by_5_grid_is_26_ordinary_operations() {
    let mut s = session();
    place(&mut s);
    let id = grid_id(&s);
    let d = derived(&s);
    assert_eq!(d.len(), 26);
    assert_eq!(cells(&s).len(), 25);
    assert_eq!(labels(&s).len(), 1);
    let names: HashSet<String> = d.iter().map(|o| o.name.clone()).collect();
    assert_eq!(names.len(), 26, "distinct names");
    // Column power 20..100 in 5, row speed 600..6000 in 5.
    for p in ["20 %", "40 %", "60 %", "80 %", "100 %"] {
        for v in ["600", "1950", "3300", "4650", "6000"] {
            let name = format!("{p} · {v} · {id}");
            assert!(names.contains(&name), "missing {name}");
        }
    }
    let c = cells(&s)
        .into_iter()
        .find(|c| c.name == format!("60 % · 1950 · {id}"))
        .unwrap();
    assert_eq!(main_num(&c, "power_pct"), 60.0);
    assert_eq!(main_num(&c, "speed_mm_min"), 1950.0);
    assert_eq!(main_num(&c, "passes"), 1.0);
    assert!(cells(&s).iter().all(|c| c.members.len() == 1));
}

#[test]
fn c12_an_unfolded_row_shows_its_source_line() {
    let (mut s, _, _, id) = birch();
    let cell = cells(&s)[0].id;
    s.unfold_operation_row(cell);
    let v: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    let row = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["unfolded"] == true)
        .unwrap()
        .clone();
    let text = row["source"]["text"].as_str().unwrap();
    assert!(text.contains("Birch 3 mm") && text.contains(&id), "{text}");
}

#[test]
fn c12_derived_rows_are_ordinary_rows() {
    let mut s = session();
    place(&mut s);
    let cell = cells(&s)[0].id;
    assert!(s.rename_operation(cell, "Renamed"));
    assert!(s.set_operation_switch(cell, OperationSwitch::Show, false));
    assert!(s.set_operation_switch(cell, OperationSwitch::Output, false));
    let c = derived(&s).into_iter().find(|o| o.id == cell).unwrap();
    assert_eq!(c.name, "Renamed");
    assert!(!c.show && !c.output);
}

// ---- criterion 14: regeneration overwrites hand edits ---------------------------------------

#[test]
fn c14_a_bar_change_overwrites_hand_edits_and_one_undo_restores_them() {
    let mut s = session();
    place(&mut s);
    let id = grid_id(&s);
    let target = format!("60 % · 1950 · {id}");
    let cell = cells(&s).into_iter().find(|c| c.name == target).unwrap().id;
    s.edit_operation_setting(cell, "main", "power", 55.0)
        .unwrap();
    assert!(s.rename_operation(cell, "Hand"));
    let edited = fingerprint(&s);
    s.set_test_grid_field("cell", "12").unwrap();
    let c = derived(&s).into_iter().find(|o| o.id == cell);
    let c = c.unwrap_or_else(|| {
        cells(&s)
            .into_iter()
            .find(|c| c.name == target)
            .expect("the cell row comes back")
    });
    assert_eq!(c.name, target, "name overwritten");
    assert_eq!(main_num(&c, "power_pct"), 60.0, "settings overwritten");
    let _ = s.undo();
    assert_eq!(
        fingerprint(&s),
        edited,
        "one undo step restores the hand edits"
    );
}

// ---- criterion 15 and 29: save and reopen ---------------------------------------------------

#[test]
fn c29_save_and_reopen_keeps_grids_and_rows() {
    let (mut s, g, _b, id) = birch();
    s.set_test_grid_kind("engrave").unwrap();
    s.set_test_grid_crosshatch(true).unwrap();
    let cell = cells(&s)[3].id;
    s.edit_operation_setting(cell, "main", "power", 44.0)
        .unwrap();
    place_at(&mut s, 200.0, 150.0);
    let before = fingerprint(&s);
    let bytes = s.pack("0.1.0").unwrap();
    let laser = page(&s).id;
    let mut r = open_bytes(5, &bytes, &laser);
    assert_eq!(fingerprint(&r), before);
    // The grid still reads as a grid with its id and recipe, and its bar still works.
    assert_eq!(id_of(&mut r, g), id);
    assert_eq!(grid_view(&r)["kind"], "engrave");
    assert_eq!(grid_view(&r)["crosshatch"], true);
    assert_eq!(r.set_test_grid_field("columns_count", "4"), Ok(true));
    assert_eq!(derived(&r).len(), 26 + 26 - 5);
}

#[test]
fn c29_golden_v46_round_trips() {
    let dir = manifest().join("../curvyo-document-core/tests/fixtures");
    for name in [
        "test_grid_v46.curvyo",
        "test_grid_damaged_v46.curvyo",
        "test_grid_orphan_rows_v46.curvyo",
    ] {
        let bytes = std::fs::read(dir.join(name)).unwrap();
        let d = unpack(1, &bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let s = Session::open(3, &bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let again = unpack(2, &s.pack("0.1.0").unwrap()).unwrap();
        let ops = |d: &Document| -> Vec<String> {
            d.pages()
                .into_iter()
                .filter_map(|p| p.kind.manufacturing())
                .flat_map(|m| d.operations(m).unwrap())
                .map(|o| {
                    format!(
                        "{} {:?} {:?} {:?}",
                        o.name, o.settings, o.options, o.grid_part
                    )
                })
                .collect()
        };
        if name == "test_grid_v46.curvyo" {
            // A 5 x 5 Cut grid (26 rows) and a smaller Engrave grid.
            assert!(ops(&d).iter().filter(|o| o.contains("\"part\"")).count() > 26);
        }
        assert_eq!(ops(&d), ops(&again), "{name}");
    }
}

#[test]
fn c29_format_44_files_open() {
    let dir = manifest().join("../curvyo-document-core/tests/fixtures");
    for name in [
        "operation_tools_v44.curvyo",
        "operation_options_damaged_v44.curvyo",
    ] {
        let bytes = std::fs::read(dir.join(name)).unwrap();
        let s = Session::open(3, &bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(s.pack("0.1.0").is_ok(), "{name}");
    }
}

// ---- criteria 16 to 21: the Materials tab ---------------------------------------------------

#[test]
fn c16_materials_tab_after_operations_on_a_laser_page() {
    let mut s = session();
    let names: Vec<&str> = s.panel_view().tabs.iter().map(|t| t.name).collect();
    let ops = names
        .iter()
        .position(|n| *n == "operations")
        .expect("Operations");
    let mats = names
        .iter()
        .position(|n| *n == "materials")
        .expect("Materials");
    assert_eq!(mats, ops + 1, "{names:?}");
    assert!(s.activate_page("first"));
    let names: Vec<&str> = s.panel_view().tabs.iter().map(|t| t.name).collect();
    assert!(!names.contains(&"materials"), "{names:?}");
}

#[test]
fn c17_list_rows_add_rename_thickness_order_delete() {
    let mut s = session();
    assert!(s.add_material("", 3.0, &[1; 16]).is_err(), "Enter a name.");
    assert!(s.add_material("X", 0.05, &[1; 16]).is_err());
    assert!(s.add_material("X", 100.1, &[1; 16]).is_err());
    let a = s.add_material("Acrylic", 0.1, &[1; 16]).unwrap();
    assert_eq!(undo_label(&mut s), "Add material");
    let b = s.add_material("Birch", 100.0, &[2; 16]).unwrap();
    let rows = materials_view(&s)["rows"].as_array().unwrap().clone();
    assert_eq!(rows.len(), 2);
    assert!(rows[0]["line"].is_string());
    assert_eq!(s.rename_material(&a, "Acryl"), Ok(true));
    assert_eq!(s.rename_material(&a, "Acryl"), Ok(false));
    assert_eq!(s.set_material_thickness(&a, 5.0), Ok(true));
    assert!(s.move_material(&b, 0));
    let rows = materials_view(&s)["rows"].as_array().unwrap().clone();
    assert_eq!(rows[0]["id"], b.as_str());
    assert_eq!(rows[1]["name"], "Acryl");
    s.delete_material(&a).unwrap();
    assert_eq!(materials_view(&s)["rows"].as_array().unwrap().len(), 1);
    let _ = s.undo();
    assert_eq!(materials_view(&s)["rows"].as_array().unwrap().len(), 2);
}

#[test]
fn c17_row_counts_lasers_sets_tests() {
    let (mut s, _, b, _) = birch();
    s.add_material_set(&b, "generic-laser", "cut", &[4; 16])
        .unwrap();
    s.add_material_set(&b, "generic-laser", "score", &[5; 16])
        .unwrap();
    let k = load_second_laser(&mut s);
    s.add_material_set(&b, &k, "engrave", &[6; 16]).unwrap();
    let row = materials_view(&s)["rows"][0].clone();
    assert_eq!(row["line"], "2 lasers · 3 sets · 1 test", "{row}");
}

#[test]
fn c17_deleting_a_material_retires_its_ids_and_removes_its_photos() {
    let (mut s, _, b, id) = birch();
    s.set_test_photo(&b, &id, &png(100, 100), false).unwrap();
    let _ = writes(&mut s);
    s.delete_material(&b).unwrap();
    let w = writes(&mut s);
    let text = Value::from(w.clone()).to_string();
    assert!(w.iter().any(|x| x["op"] == "remove_photo"), "{text}");
    let retired = w.iter().find(|x| x["op"] == "retired").expect(&text);
    assert!(retired["text"].as_str().unwrap().contains(&id), "{text}");
    // A later grid never gets that id: delete the grid on the page too and place anew.
    s.delete_selected();
    place_at(&mut s, 200.0, 150.0);
    assert_ne!(grid_id(&s), id);
}

#[test]
fn c18_one_material_with_sets_per_laser_page_laser_first() {
    let mut s = session();
    let b = s.add_material("Birch", 3.0, &[3; 16]).unwrap();
    let k = load_second_laser(&mut s);
    s.unfold_material(Some(&b));
    let v = materials_view(&s);
    let others = v["unfolded"]["other_lasers"].to_string();
    assert!(
        others.contains(&k),
        "the library's other profile is offered: {others}"
    );
    s.open_material_laser_pick(true);
    assert!(s.pick_material_laser(&b, &k));
    s.add_material_set(&b, &k, "cut", &[4; 16]).unwrap();
    s.add_material_set(&b, "generic-laser", "score", &[5; 16])
        .unwrap();
    let v = materials_view(&s);
    let lasers = v["unfolded"]["lasers"].as_array().unwrap();
    assert_eq!(lasers[0]["laser"]["id"], "generic-laser");
    assert_eq!(lasers[0]["this_page"], true);
    assert_eq!(lasers[0]["open"], true);
    assert_eq!(lasers[1]["laser"]["id"], k.as_str());
    assert_eq!(lasers[1]["this_page"], false);
    // The page's profile did not change.
    assert_eq!(v["page_laser"]["id"], "generic-laser");
    assert_eq!(
        materials_view(&s)["rows"].as_array().unwrap().len(),
        1,
        "one record"
    );
}

#[test]
fn c19_set_ranges_and_only_four_numbers() {
    let mut s = session();
    let b = s.add_material("Birch", 3.0, &[3; 16]).unwrap();
    s.add_material_set(&b, "generic-laser", "cut", &[4; 16])
        .unwrap();
    s.add_material_set(&b, "generic-laser", "engrave", &[5; 16])
        .unwrap();
    let e = |s: &mut Session, k: &str, f: &str, v: f64| {
        s.edit_material_set(&b, "generic-laser", k, f, v, &[9; 16])
    };
    for (f, v) in [
        ("power", 0.0),
        ("power", 100.5),
        ("power", 0.9),
        ("speed", 0.0),
        ("passes", 0.0),
        ("passes", 21.0),
    ] {
        assert!(e(&mut s, "cut", f, v).is_err(), "{f} {v}");
    }
    assert!(
        e(&mut s, "cut", "interval", 0.1).is_err(),
        "no interval on Cut"
    );
    for (f, v) in [
        ("power", 1.0),
        ("power", 100.0),
        ("power", 55.5),
        ("speed", 1.0),
        ("passes", 20.0),
    ] {
        assert!(e(&mut s, "cut", f, v).is_ok(), "{f} {v}");
    }
    assert_eq!(e(&mut s, "engrave", "interval", 0.08), Ok(true));
    s.unfold_material(Some(&b));
    let v = materials_view(&s);
    let sets = v["unfolded"]["lasers"][0]["sets"].as_array().unwrap();
    let cut = sets.iter().find(|x| x["kind"] == "cut").unwrap();
    assert_eq!(cut["numbers"]["power_pct"], 55.5);
    assert_eq!(cut["numbers"]["passes"], 20);
    assert_eq!(cut["source"], "Typed");
    let eng = sets.iter().find(|x| x["kind"] == "engrave").unwrap();
    assert_eq!(eng["interval_mm"], 0.08);
    // The record holds the four numbers and the source, nothing else.
    let w = writes(&mut s);
    let last = w.iter().rev().find(|x| x["op"] == "write").unwrap();
    let rec: Value = serde_json::from_str(last["text"].as_str().unwrap()).unwrap();
    for (_, set) in rec["lasers"][0]["sets"].as_object().unwrap() {
        for key in set.as_object().unwrap().keys() {
            assert!(
                [
                    "power_pct",
                    "speed_mm_min",
                    "passes",
                    "interval_mm",
                    "source"
                ]
                .contains(&key.as_str()),
                "{key}"
            );
        }
    }
}

#[test]
fn c20_missing_kinds_offer_a_set_button_typed_one_step() {
    let mut s = session();
    let b = s.add_material("Birch", 3.0, &[3; 16]).unwrap();
    s.unfold_material(Some(&b));
    let v = materials_view(&s);
    let missing = v["unfolded"]["lasers"][0]["missing"].to_string();
    for k in ["cut", "score", "engrave"] {
        assert!(missing.contains(k), "{missing}");
    }
    let n = s.undo_len();
    s.add_material_set(&b, "generic-laser", "score", &[4; 16])
        .unwrap();
    assert_eq!(s.undo_len(), n + 1);
    assert_eq!(undo_label(&mut s), "Add set");
    let v = materials_view(&s);
    let l = &v["unfolded"]["lasers"][0];
    assert!(!l["missing"].to_string().contains("score"));
    assert_eq!(l["sets"][0]["source"], "Typed");
    let n = s.undo_len();
    s.edit_material_set(&b, "generic-laser", "score", "power", 42.0, &[5; 16])
        .unwrap();
    assert_eq!(s.undo_len(), n + 1, "a set edit is one step");
    let _ = s.undo();
    let v = materials_view(&s);
    assert_ne!(
        v["unfolded"]["lasers"][0]["sets"][0]["numbers"]["power_pct"],
        42.0
    );
}

#[test]
fn c21_adding_a_set_for_a_new_laser_adds_the_laser_and_ops_use_the_page_laser() {
    let (d, laser) = base_doc();
    let lp = d.pages().into_iter().find(|p| p.id == laser).unwrap();
    let r = d.create_rect(RectBounds::from_corners(
        Point::new(lp.origin.x + 300.0, lp.origin.y + 200.0),
        Point::new(lp.origin.x + 320.0, lp.origin.y + 210.0),
    ));
    let m = lp.kind.manufacturing().unwrap();
    let op = d
        .add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
            },
            0,
            &[vec![r]],
        )
        .unwrap();
    let mut s = open_bytes(5, &pack(&d, "c21").unwrap(), &laser);
    let b = s.add_material("Birch", 3.0, &[3; 16]).unwrap();
    let k = load_second_laser(&mut s);
    // A set for the other laser only: the laser enters the material.
    s.add_material_set(&b, &k, "cut", &[4; 16]).unwrap();
    s.unfold_material(Some(&b));
    let lasers = materials_view(&s)["unfolded"]["lasers"].to_string();
    assert!(lasers.contains(&k), "{lasers}");
    // Both lasers have a Cut set: the operation uses the page laser's.
    s.add_material_set(&b, "generic-laser", "cut", &[5; 16])
        .unwrap();
    s.edit_material_set(&b, "generic-laser", "cut", "power", 61.0, &[6; 16])
        .unwrap();
    s.edit_material_set(&b, &k, "cut", "power", 17.0, &[7; 16])
        .unwrap();
    assert!(s.use_material_on(&[op], &b));
    s.unfold_operation_row(op);
    let v: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    let row = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["unfolded"] == true)
        .unwrap()
        .clone();
    assert_eq!(
        row["settings"]["sets"][0]["run"]["power_pct"], 61.0,
        "{row}"
    );
    // Without a set for the page's laser the operation keeps its stored numbers and says so.
    s.delete_material_set(&b, "generic-laser", "cut", &[8; 16])
        .unwrap();
    let v = s.operations_view_json();
    let wanted = "Material has no Cut set for this laser, using stored numbers";
    assert!(v.contains(wanted), "{v}");
    let v: Value = serde_json::from_str(&v).unwrap();
    let row = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["unfolded"] == true)
        .unwrap()
        .clone();
    assert_eq!(
        row["settings"]["sets"][0]["run"]["power_pct"], 61.0,
        "stored: {row}"
    );
}

// ---- criteria 22 to 28: tests ---------------------------------------------------------------

#[test]
fn c22_no_record_without_material_record_at_pick() {
    let mut s = session();
    let b = s.add_material("Birch", 3.0, &[3; 16]).unwrap();
    place(&mut s);
    let id = grid_id(&s);
    assert!(test_ids_of(&mut s, &b).is_empty(), "no test before a pick");
    s.pick_test_grid_material(Some(&b)).unwrap();
    assert_eq!(test_ids_of(&mut s, &b), [id.clone()]);
    let t = &tests_of(&mut s, &b)[0];
    assert_eq!(t["lasered"], false);
    assert!(
        t["line"].as_str().unwrap().starts_with("Not lasered · "),
        "{t}"
    );
}

#[test]
fn c22_record_follows_the_grid_and_moves_with_a_new_pick() {
    let (mut s, _g, b, id) = birch();
    let a = s.add_material("Acrylic", 3.0, &[1; 16]).unwrap();
    s.set_test_grid_field("columns_count", "7").unwrap();
    s.unfold_material(Some(&b));
    s.open_material_test(Some(&id));
    let t = materials_view(&s)["unfolded"]["test"].clone();
    assert_eq!(t["columns"]["values"].as_array().unwrap().len(), 7, "{t}");
    let n = s.undo_len();
    s.pick_test_grid_material(Some(&a)).unwrap();
    assert_eq!(s.undo_len(), n + 1);
    assert_eq!(grid_id(&s), id, "a move keeps the id");
    assert!(test_ids_of(&mut s, &b).is_empty());
    assert_eq!(test_ids_of(&mut s, &a), [id.clone()]);
    // Undo of the move restores both records.
    let _ = writes(&mut s);
    let _ = s.undo();
    let w = writes(&mut s);
    let ids: HashSet<String> = w
        .iter()
        .filter(|x| x["op"] == "write")
        .map(|x| x["id"].as_str().unwrap().to_owned())
        .collect();
    assert!(ids.contains(&a) && ids.contains(&b), "{w:?}");
    assert_eq!(test_ids_of(&mut s, &b), [id.clone()]);
    assert!(test_ids_of(&mut s, &a).is_empty());
}

#[test]
fn c22_tests_newest_first_with_state_and_photo_glyph() {
    let (mut s, g, b, first) = birch();
    s.mark_test_grid_lasered_for_tests(g, 1_762_948_800_000)
        .unwrap();
    s.set_test_photo(&b, &first, &png(50, 50), false).unwrap();
    place_at(&mut s, 200.0, 150.0);
    s.pick_test_grid_material(Some(&b)).unwrap();
    let second = grid_id(&s);
    let t = tests_of(&mut s, &b);
    assert_eq!(t.len(), 2);
    assert_eq!(t[0]["id"], second.as_str(), "newest first");
    assert_eq!(t[1]["id"], first.as_str());
    assert_eq!(t[1]["photo"], true);
    assert_eq!(t[0]["photo"], false);
    let l = t[1]["line"].as_str().unwrap();
    assert!(
        l.starts_with("Lasered ") && l.contains("Generic laser"),
        "{l}"
    );
    s.delete_selected();
    let t = tests_of(&mut s, &b);
    assert!(
        t[0]["line"].as_str().unwrap().contains("not on a page"),
        "{}",
        t[0]
    );
}

#[test]
fn c23_test_detail() {
    let (mut s, _, b, id) = birch();
    s.unfold_material(Some(&b));
    s.open_material_test(Some(&id));
    let t = materials_view(&s)["unfolded"]["test"].clone();
    assert_eq!(t["id"], id.as_str());
    assert_eq!(t["kind"], "cut");
    assert!(t["head"].as_str().unwrap().contains("Generic laser"));
    assert!(t["axes_line"].as_str().unwrap().contains("Power"));
    assert_eq!(t["columns"]["values"].as_array().unwrap().len(), 5);
    assert_eq!(t["rows"]["values"].as_array().unwrap().len(), 5);
    assert_eq!(t["place"]["label"], "Select on page");
    assert_eq!(t["place"]["on_page"], true);
    s.delete_selected();
    s.unfold_material(Some(&b));
    s.open_material_test(Some(&id));
    let t = materials_view(&s)["unfolded"]["test"].clone();
    assert!(
        t["place"]["label"]
            .as_str()
            .unwrap()
            .starts_with("Place on "),
        "{t}"
    );
    s.open_material_test(None);
    assert!(materials_view(&s)["unfolded"]["test"].is_null());
}

#[test]
fn c24_judging_one_cell_becomes_the_laser_set() {
    let (mut s, _, b, id) = birch();
    let n = s.undo_len();
    s.judge_test(&b, &id, 2, 1, &[8; 16]).unwrap();
    assert_eq!(s.undo_len(), n + 1);
    s.unfold_material(Some(&b));
    s.open_material_test(Some(&id));
    let v = materials_view(&s);
    let t = &v["unfolded"]["test"];
    assert_eq!(t["best"], json!({"column": 2, "row": 1}));
    assert_eq!(
        t["set_line"],
        "Cut set of Generic laser: 60 %, 1950 mm/min, 1 ×"
    );
    let set = &v["unfolded"]["lasers"][0]["sets"][0];
    assert_eq!(set["kind"], "cut");
    assert_eq!(set["source"], format!("Test {id}"));
    assert_eq!(set["numbers"]["power_pct"], 60.0);
    assert_eq!(set["numbers"]["speed_mm_min"], 1950);
    // Another cell moves the mark and the set.
    s.judge_test(&b, &id, 4, 4, &[9; 16]).unwrap();
    let v = materials_view(&s);
    assert_eq!(
        v["unfolded"]["test"]["best"],
        json!({"column": 4, "row": 4})
    );
    assert_eq!(
        v["unfolded"]["lasers"][0]["sets"][0]["numbers"]["power_pct"],
        100.0
    );
    assert_eq!(
        v["unfolded"]["lasers"][0]["sets"][0]["numbers"]["speed_mm_min"],
        6000
    );
    // The same cell again changes nothing.
    let n = s.undo_len();
    assert!(s.judge_test(&b, &id, 4, 4, &[10; 16]).is_err());
    assert_eq!(s.undo_len(), n);
    // Outside the table: refused.
    assert!(s.judge_test(&b, &id, 5, 0, &[11; 16]).is_err());
    let _ = s.undo();
    let v = materials_view(&s);
    assert_eq!(
        v["unfolded"]["test"]["best"],
        json!({"column": 2, "row": 1})
    );
}

#[test]
fn c24_engrave_shared_values_come_from_row_1() {
    let mut s = session();
    let b = s.add_material("Birch", 3.0, &[3; 16]).unwrap();
    place(&mut s);
    s.set_test_grid_kind("engrave").unwrap();
    s.pick_test_grid_material(Some(&b)).unwrap();
    let id = grid_id(&s);
    let shared = grid_view(&s)["shared"].clone();
    let interval = shared
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["param"] == "interval")
        .map(|x| x["value"].as_f64().unwrap())
        .expect("interval shared on a power x speed Engrave grid");
    s.judge_test(&b, &id, 0, 0, &[8; 16]).unwrap();
    s.unfold_material(Some(&b));
    let v = materials_view(&s);
    let set = v["unfolded"]["lasers"][0]["sets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["kind"] == "engrave")
        .unwrap()
        .clone();
    assert!(
        (set["interval_mm"].as_f64().unwrap() - interval).abs() < 1e-9,
        "{set}"
    );
}

#[test]
fn c24_judging_works_lasered_or_not() {
    let (mut s, g, b, id) = birch();
    s.mark_test_grid_lasered_for_tests(g, 5).unwrap();
    assert!(s.judge_test(&b, &id, 1, 1, &[8; 16]).is_ok());
}

#[test]
fn c25_delete_test_retires_its_id_and_removes_its_photo() {
    let (mut s, _, b, id) = birch();
    s.set_test_photo(&b, &id, &png(80, 60), false).unwrap();
    let _ = writes(&mut s);
    let n = s.undo_len();
    s.delete_test(&b, &id).unwrap();
    assert_eq!(s.undo_len(), n + 1);
    assert!(test_ids_of(&mut s, &b).is_empty());
    let w = writes(&mut s);
    let text = Value::from(w.clone()).to_string();
    assert!(w.iter().any(|x| x["op"] == "remove_photo"), "{text}");
    assert!(
        w.iter()
            .any(|x| x["op"] == "retired" && x["text"].as_str().unwrap().contains(&id)),
        "{text}"
    );
}

#[test]
fn c26_note_500_plain_one_step() {
    let (mut s, _, b, id) = birch();
    let n = s.undo_len();
    let note: String = "é".repeat(500);
    assert_eq!(s.set_test_note(&b, &id, &note), Ok(true));
    assert_eq!(s.undo_len(), n + 1);
    assert_eq!(s.set_test_note(&b, &id, &note), Ok(false));
    assert_eq!(s.undo_len(), n + 1);
    assert!(s.set_test_note(&b, &id, &"a".repeat(501)).is_err());
    assert_eq!(s.undo_len(), n + 1);
    s.unfold_material(Some(&b));
    s.open_material_test(Some(&id));
    assert_eq!(
        materials_view(&s)["unfolded"]["test"]["note"],
        note.as_str()
    );
    let _ = s.undo();
    assert_eq!(materials_view(&s)["unfolded"]["test"]["note"], "");
}

#[test]
fn c27_photo_checks_and_file_next_to_the_record() {
    let (mut s, _, b, id) = birch();
    assert!(s.set_test_photo(&b, &id, &png(601, 10), false).is_err());
    assert!(s.set_test_photo(&b, &id, b"GIF89a....", false).is_err());
    let mut big = png(600, 600);
    big.resize(1024 * 1024 + 1, 0);
    assert!(s.set_test_photo(&b, &id, &big, false).is_err());
    assert!(writes(&mut s).iter().all(|w| w["op"] != "write_photo"));
    let jpg = photo_golden();
    let n = s.undo_len();
    s.set_test_photo(&b, &id, &jpg, false).unwrap();
    assert_eq!(s.undo_len(), n + 1);
    let w = writes(&mut s);
    let text = Value::from(w.clone()).to_string();
    let pw = w.iter().find(|x| x["op"] == "write_photo").expect(&text);
    assert_eq!(pw["name"], format!("{b}.{id}.jpg"), "{text}");
    let handle = pw["handle"].as_u64().expect(&text) as u32;
    assert_eq!(s.take_material_photo_bytes(handle), Some(jpg.clone()));
    // A PNG replaces the JPEG in the same step.
    s.set_test_photo(&b, &id, &png(300, 200), true).unwrap();
    let w = writes(&mut s);
    let text = Value::from(w.clone()).to_string();
    assert!(
        w.iter()
            .any(|x| x["op"] == "remove_photo" && x["name"] == format!("{b}.{id}.jpg")),
        "{text}"
    );
    assert!(
        w.iter()
            .any(|x| x["op"] == "write_photo" && x["name"] == format!("{b}.{id}.png")),
        "{text}"
    );
    // Its own trash, one step; undo writes the bytes back.
    let n = s.undo_len();
    s.remove_test_photo(&b, &id).unwrap();
    assert_eq!(s.undo_len(), n + 1);
    let w = writes(&mut s);
    assert!(w.iter().any(|x| x["op"] == "remove_photo"));
    let _ = s.undo();
    let w = writes(&mut s);
    let text = Value::from(w.clone()).to_string();
    let back = w.iter().find(|x| x["op"] == "write_photo").expect(&text);
    let bytes = s
        .take_material_photo_bytes(back["handle"].as_u64().unwrap() as u32)
        .unwrap();
    assert_eq!(bytes, png(300, 200), "undo restores the photo bytes");
}

#[test]
fn c28_place_on_page_recreates_with_the_same_id_then_lasered_change_renews() {
    let (mut s, _g, b, id) = birch();
    s.set_test_grid_field("cell", "8").unwrap();
    let fields = grid_view(&s);
    s.delete_selected();
    assert_eq!(derived(&s).len(), 0);
    let n = s.undo_len();
    let g2 = s.place_material_test(&b, &id).unwrap();
    assert_eq!(s.undo_len(), n + 1, "one step");
    assert_eq!(undo_label(&mut s), "Place test grid");
    assert_eq!(grid_id(&s), id, "same id");
    let v = grid_view(&s);
    for k in ["kind", "cell_mm", "columns", "rows", "shared", "material"] {
        assert_eq!(v[k], fields[k], "{k}");
    }
    assert_eq!(derived(&s).len(), 26);
    // "Select on page" now selects it.
    assert_eq!(s.place_material_test(&b, &id), Ok(g2));
    assert_eq!(derived(&s).len(), 26, "no second copy");
    // Lasered, then a change: new id, the test keeps its record.
    s.mark_test_grid_lasered_for_tests(g2, 9).unwrap();
    s.set_test_grid_field("cell", "9").unwrap();
    assert_ne!(grid_id(&s), id);
    assert!(test_ids_of(&mut s, &b).contains(&id));
}

// ---- material undo journal: several records ---------------------------------------------------

#[test]
fn material_undo_of_a_deleted_material_restores_tests_and_photo() {
    let (mut s, _, b, id) = birch();
    s.set_test_photo(&b, &id, &png(64, 64), false).unwrap();
    s.set_test_note(&b, &id, "Clean edge").unwrap();
    s.delete_material(&b).unwrap();
    let _ = writes(&mut s);
    let _ = s.undo();
    let w = writes(&mut s);
    let text = Value::from(w.clone()).to_string();
    assert!(
        w.iter()
            .any(|x| x["op"] == "write" && x["id"] == b.as_str()),
        "{text}"
    );
    let p = w.iter().find(|x| x["op"] == "write_photo").expect(&text);
    let bytes = s
        .take_material_photo_bytes(p["handle"].as_u64().unwrap() as u32)
        .unwrap();
    assert_eq!(bytes, png(64, 64));
    s.unfold_material(Some(&b));
    s.open_material_test(Some(&id));
    assert_eq!(materials_view(&s)["unfolded"]["test"]["note"], "Clean edge");
}

// ---- criterion 30: record format 2 in the store -------------------------------------------

#[test]
fn c30_store_refuses_a_write_over_512_kib() {
    let text = material_golden("format2_max.json");
    let v: Value = serde_json::from_str(&text).unwrap();
    let id = v["id"].as_str().unwrap().to_owned();
    let tests: Vec<String> = v["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap().to_owned())
        .collect();
    let mut s = session();
    s.load_materials(&[(format!("{id}.json"), text)], None);
    assert_eq!(materials_view(&s)["rows"][0]["state"], "loaded");
    // 500 four-byte characters per note push the record past 512 KiB.
    let wide: String = "𝄞".repeat(500);
    let mut refused = None;
    for t in &tests {
        match s.set_test_note(&id, t, &wide) {
            Ok(_) => {}
            Err(e) => {
                refused = Some(e);
                break;
            }
        }
    }
    let e = refused.expect("a record over 512 KiB is refused");
    assert!(e.contains("512 KiB"), "{e}");
    let w = writes(&mut s);
    for x in w.iter().filter(|x| x["op"] == "write") {
        assert!(x["text"].as_str().unwrap().len() <= 512 * 1024);
    }
}

#[test]
fn c30_format2_record_loads_and_writes_back_format_2() {
    let text = material_golden("format2_full.json");
    let v: Value = serde_json::from_str(&text).unwrap();
    let id = v["id"].as_str().unwrap().to_owned();
    let mut s = session();
    s.load_materials(&[(format!("{id}.json"), text.clone())], None);
    assert!(
        writes(&mut s).iter().all(|w| w["op"] != "write"),
        "reads never write"
    );
    assert_eq!(s.rename_material(&id, "Birch renamed"), Ok(true));
    let w = writes(&mut s);
    let rec = w.iter().find(|x| x["op"] == "write").unwrap();
    let back: Value = serde_json::from_str(rec["text"].as_str().unwrap()).unwrap();
    assert_eq!(back["format_version"], 2);
    assert_eq!(back["name"], "Birch renamed");
    assert_eq!(back["tests"], v["tests"]);
    assert_eq!(back["lasers"], v["lasers"]);
}

#[test]
fn c30_newer_record_is_left_untouched() {
    let text = material_golden("newer_format.json");
    let v: Value = serde_json::from_str(&text).unwrap();
    let id = v["id"].as_str().unwrap().to_owned();
    let mut s = session();
    s.load_materials(&[(format!("{id}.json"), text)], None);
    assert!(s.rename_material(&id, "X").is_err() || writes(&mut s).is_empty());
    assert!(s.delete_material(&id).is_err());
    assert!(writes(&mut s).iter().all(|w| w["op"] != "write"));
}

// ---- white box: other copy and move paths ----------------------------------------------------

fn key(s: &mut Session, k: &str) {
    s.key_down(curvyo_editor_wasm::KeyInput {
        key: k,
        ..curvyo_editor_wasm::KeyInput::default()
    });
}

#[test]
fn c8_a_move_chip_copy_gets_a_new_id_and_its_own_rows() {
    let mut s = session();
    place(&mut s);
    let id = grid_id(&s);
    key(&mut s, "m");
    let out = s.commit_move_entry(
        "150",
        "0",
        curvyo_ui_core::MoveEntryMode {
            absolute: false,
            copy: true,
        },
    );
    assert_eq!(out, curvyo_ui_core::EntryOutcome::Committed);
    assert_eq!(derived(&s).len(), 52, "the copy has its own rows");
    let names: HashSet<String> = labels(&s).iter().map(|o| o.name.clone()).collect();
    assert_eq!(names.len(), 2, "two label rows with two ids: {names:?}");
    assert!(names.contains(&format!("{id} labels")));
    let _ = s.undo();
    assert_eq!(derived(&s).len(), 26);
}

#[test]
fn c8_a_copy_of_a_lasered_grid_is_not_lasered() {
    let mut s = session();
    let g = place(&mut s);
    s.mark_test_grid_lasered_for_tests(g, 3).unwrap();
    let _ = s.duplicate_layer_target();
    assert_eq!(grid_view(&s)["lasered"], false);
    assert_eq!(derived(&s).len(), 52);
}

#[test]
fn c7_a_move_chip_move_keeps_the_rows_and_the_id() {
    let mut s = session();
    let g = place(&mut s);
    let id = grid_id(&s);
    let (a, _) = grid_bounds(&s, g);
    key(&mut s, "m");
    let out = s.commit_move_entry(
        "30",
        "20",
        curvyo_ui_core::MoveEntryMode {
            absolute: false,
            copy: false,
        },
    );
    assert_eq!(out, curvyo_ui_core::EntryOutcome::Committed);
    let (b, _) = grid_bounds(&s, g);
    assert!((b.x - a.x - 30.0).abs() < 0.01 && (b.y - a.y - 20.0).abs() < 0.01);
    assert_eq!(derived(&s).len(), 26);
    assert_eq!(grid_id(&s), id);
}

#[test]
fn c13_a_move_chip_move_off_the_page_removes_the_rows() {
    let mut s = session();
    place(&mut s);
    key(&mut s, "m");
    let n = s.undo_len();
    let out = s.commit_move_entry(
        "0",
        "-3000",
        curvyo_ui_core::MoveEntryMode {
            absolute: false,
            copy: false,
        },
    );
    assert_eq!(out, curvyo_ui_core::EntryOutcome::Committed);
    assert_eq!(s.undo_len(), n + 1);
    assert_eq!(derived(&s).len(), 0);
    let _ = s.undo();
    assert_eq!(derived(&s).len(), 26);
}
