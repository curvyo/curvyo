//! Tester acceptance cases for `specs/0028-material-test-library`, written from the
//! specification (32 criteria) before the implementation was read. Black box through the
//! session: the Grid block, the result table, Save as material, the Materials section, the
//! settings block with its one material pick, the undo of material edits and its session limit,
//! the stored numbers beside a link (refresh on material edit and on save) and the project file
//! round trip. The host side of the library is simulated by applying the session's write list
//! to an in-memory folder and loading it back, as the frontend does.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::needless_pass_by_value,
    missing_docs
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use curvyo_document_core::{
    Direction, Document, DocumentSize, NewOperation, NewOutputPage, NodeId, ObjectSnapshot,
    OperationSwitch, Point, RectBounds, Shape, Timestamp, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, SaveAsMaterial, Session, Tool};
use curvyo_ui_core::FontBlock;
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";

// ---- fixtures ---------------------------------------------------------------------------------

fn clock() -> Timestamp {
    Timestamp::from_secs(1_763_000_000)
}

/// The simulated per-user data directory: `materials/<uuid>.json` and the order list.
#[derive(Default, Clone)]
struct Folder {
    files: BTreeMap<String, String>,
    order: Option<String>,
}

impl Folder {
    fn record(&self, id: &str) -> Option<Value> {
        self.files
            .get(&format!("{id}.json"))
            .map(|t| serde_json::from_str(t).unwrap())
    }
    fn list(&self) -> Vec<(String, String)> {
        self.files
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

struct Fx {
    s: Session,
    folder: Folder,
    laser: String,
    seed: u8,
}

impl Fx {
    /// 32 fresh caller bytes for an id.
    fn random(&mut self) -> Vec<u8> {
        self.seed = self.seed.wrapping_add(1);
        (0..32u8)
            .map(|i| i.wrapping_mul(7).wrapping_add(self.seed))
            .collect()
    }
    /// Performs the session's host writes on the folder; returns them.
    fn sync(&mut self) -> Vec<Value> {
        let writes: Vec<Value> = serde_json::from_str(&self.s.take_material_writes_json()).unwrap();
        for w in &writes {
            match w["op"].as_str().unwrap() {
                "write" => {
                    let id = w["id"].as_str().unwrap();
                    self.folder
                        .files
                        .insert(format!("{id}.json"), w["text"].as_str().unwrap().to_owned());
                }
                "remove" => {
                    let id = w["id"].as_str().unwrap();
                    self.folder.files.remove(&format!("{id}.json"));
                }
                "order" => self.folder.order = Some(w["text"].as_str().unwrap().to_owned()),
                _ => {}
            }
        }
        writes
    }
    fn reload(&mut self) {
        let files = self.folder.list();
        let order = self.folder.order.clone();
        self.s.load_materials(&files, order.as_deref());
    }
    fn ops(&self) -> Value {
        serde_json::from_str(&self.s.operations_view_json()).unwrap()
    }
    fn grid(&self) -> Value {
        serde_json::from_str(&self.s.test_grid_view_json()).unwrap()
    }
    fn mats(&self) -> Value {
        serde_json::from_str(&self.s.materials_view_json()).unwrap()
    }
    fn row(&self, id: NodeId) -> Value {
        let want = serde_json::to_value(id).unwrap();
        self.ops()["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == want)
            .cloned()
            .unwrap_or(Value::Null)
    }
    fn grid_rows(&self) -> Vec<Value> {
        self.ops()["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| !r["grid"].is_null())
            .cloned()
            .collect()
    }
    fn undo_name(&mut self) -> String {
        self.s
            .history_target_name(Direction::Undo)
            .unwrap_or_default()
    }
    fn saved_doc(&mut self) -> Document {
        unpack(9, &self.s.pack_for_save().unwrap()).unwrap()
    }
    /// Marks cell (2, 1) of the grid in the block and saves it as a new material.
    fn save_new(&mut self, name: &str) -> String {
        assert!(self.s.press_test_cell(2, 1));
        let r = self.random();
        let id = self
            .s
            .save_as_material(
                &SaveAsMaterial {
                    existing: None,
                    name: name.to_owned(),
                    thickness_mm: 3.0,
                    passes: 1.0,
                },
                &r,
            )
            .unwrap();
        self.sync();
        id
    }
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn with_fonts(s: &mut Session) {
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
    let _ = s.press_add_test_grid();
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
}

/// A laser page (Generic laser) of `w` x `h` mm with operations of `kinds` and one rectangle.
fn document(w: f64, h: f64, kinds: &[&str]) -> (Document, String, Vec<NodeId>) {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(w, h),
        })
        .unwrap();
    let m = made.kind.manufacturing().unwrap();
    let ids = kinds
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            let name = format!("{kind} {}", i + 1);
            d.add_operation(m, &NewOperation { kind, name: &name }, i, &[])
                .unwrap()
        })
        .collect();
    (d, made.id, ids)
}

fn open(d: &Document, laser: &str, folder: Folder) -> Fx {
    let mut s = Session::open(3, &pack(d, "0028-tester").unwrap()).unwrap();
    s.set_clock(clock);
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    let files = folder.list();
    s.load_materials(&files, folder.order.as_deref());
    assert!(s.activate_page(laser));
    with_fonts(&mut s);
    // The font probe of `with_fonts` may have added nothing; start from a clean history.
    s.wipe_history();
    Fx {
        s,
        folder,
        laser: laser.to_owned(),
        seed: 0,
    }
}

fn fx(kinds: &[&str]) -> (Fx, Vec<NodeId>) {
    let (d, laser, ids) = document(400.0, 300.0, kinds);
    (open(&d, &laser, Folder::default()), ids)
}

/// A session with one default grid added.
fn grid_fx(kinds: &[&str]) -> (Fx, Vec<NodeId>, NodeId) {
    let (mut f, ids) = fx(kinds);
    let row = f.s.press_add_test_grid().unwrap();
    (f, ids, row)
}

fn group_of(f: &Fx) -> NodeId {
    serde_json::from_value(f.grid()["group"].clone()).unwrap()
}

/// A material record text tied to `machine` with the sets given.
fn record_text(id: &str, name: &str, machine: &str, sets: Value, modified: u64) -> String {
    let mut history = Vec::new();
    let mut with_sources = serde_json::Map::new();
    for (i, (kind, set)) in sets.as_object().unwrap().iter().enumerate() {
        let entry = format!("aaaaaaaa-0000-4000-8000-00000000000{i}");
        history.push(json!({
            "id": entry, "date": 1_762_948_800_000_u64, "event": "edited", "kind": kind,
            "set": set
        }));
        let mut s = set.as_object().unwrap().clone();
        s.insert("source".into(), json!("edited"));
        with_sources.insert(kind.clone(), Value::Object(s));
    }
    let v = json!({
        "format_version": 1, "id": id, "kind": "laser", "modified": modified, "name": name,
        "thickness_mm": 3.0, "machine": {"id": machine, "name": "Generic laser"},
        "sets": with_sources, "history": history
    });
    serde_json::to_string_pretty(&v).unwrap()
}

const BIRCH: &str = "22222222-2222-4222-8222-222222222222";
const CARD: &str = "11111111-1111-4111-8111-111111111111";
const ALIEN: &str = "55555555-5555-4555-8555-555555555555";

fn birch_folder(modified: u64) -> Folder {
    let mut f = Folder::default();
    f.files.insert(
        format!("{BIRCH}.json"),
        record_text(
            BIRCH,
            "Birch",
            "generic-laser",
            json!({
                "cut": {"power_pct": 60.0, "speed_mm_min": 300, "passes": 2},
                "score": {"power_pct": 12.5, "speed_mm_min": 3000, "passes": 1},
                "engrave": {"power_pct": 35.0, "speed_mm_min": 6000, "passes": 1}
            }),
            modified,
        ),
    );
    f.files.insert(
        format!("{CARD}.json"),
        record_text(
            CARD,
            "Card",
            "generic-laser",
            json!({"cut": {"power_pct": 40.0, "speed_mm_min": 900, "passes": 1}}),
            modified,
        ),
    );
    f.files.insert(
        format!("{ALIEN}.json"),
        record_text(
            ALIEN,
            "Alien",
            "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f",
            json!({"cut": {"power_pct": 10.0, "speed_mm_min": 100, "passes": 1}}),
            modified,
        ),
    );
    f
}

fn lib_fx(kinds: &[&str]) -> (Fx, Vec<NodeId>) {
    let (d, laser, ids) = document(400.0, 300.0, kinds);
    (open(&d, &laser, birch_folder(1_762_948_800_000)), ids)
}

fn op_settings(d: &Document, op: NodeId) -> Option<Value> {
    let m = d
        .pages()
        .into_iter()
        .find_map(|p| p.kind.manufacturing())
        .unwrap();
    d.operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.id == op)
        .unwrap()
        .settings
}

// ---- criteria 1 to 3: making a grid -------------------------------------------------------------

#[test]
fn c1_add_test_grid_uses_the_defaults_and_names_the_machine_and_size() {
    let (f, _, _) = grid_fx(&[]);
    let g = f.grid();
    assert_eq!(g["kind"], "cut");
    let fl = &g["fields"];
    assert_eq!(fl["columns"], 5);
    assert_eq!(fl["rows"], 5);
    assert_eq!(fl["power_from_pct"].as_f64(), Some(20.0));
    assert_eq!(fl["power_to_pct"].as_f64(), Some(100.0));
    let max = g["max_feed_mm_min"].as_u64().unwrap();
    assert_eq!(fl["speed_from_mm_min"].as_u64(), Some(max / 10));
    assert_eq!(fl["speed_to_mm_min"].as_u64(), Some(max));
    assert_eq!(fl["cell_mm"].as_f64(), Some(10.0));
    assert_eq!(fl["gap_mm"].as_f64(), Some(3.0));
    assert_eq!(g["machine"], "Generic laser");
    assert!(g["width_mm"].as_f64().unwrap() > 50.0);
    assert!(g["height_mm"].as_f64().unwrap() > 50.0);
    // Labels defaults (criterion 7).
    let l = &g["labels"];
    assert_eq!(l["on"], true);
    assert_eq!(l["kind"], "engrave");
    assert_eq!(l["height_mm"].as_f64(), Some(3.0));
    assert_eq!(l["power_pct"].as_f64(), Some(20.0));
    assert_eq!(l["speed_mm_min"].as_u64(), Some(max / 2));
    assert_eq!(l["passes"], 1);
}

#[test]
fn c2_out_of_range_values_are_kept_marked_never_clamped_and_change_nothing() {
    let (mut f, _, _) = grid_fx(&[]);
    let before_undo = f.s.undo_len();
    let before = f.grid()["fields"].clone();
    let max = f.grid()["max_feed_mm_min"].as_u64().unwrap();
    for (field, typed) in [
        ("columns", "13".to_owned()),
        ("columns", "1".to_owned()),
        ("rows", "13".to_owned()),
        ("power_from", "0".to_owned()),
        ("power_to", "101".to_owned()),
        ("speed_from", "0".to_owned()),
        ("speed_to", (max + 1).to_string()),
        ("cell", "2.9".to_owned()),
        ("cell", "51".to_owned()),
        ("gap", "20.5".to_owned()),
        ("gap", "-1".to_owned()),
        ("label_passes", "0".to_owned()),
        ("label_power", "0".to_owned()),
        ("label_speed", (max + 1).to_string()),
    ] {
        let r = f.s.set_test_grid_field(field, &typed);
        assert!(r.is_err(), "{field} = {typed} must be refused");
        let g = f.grid();
        assert!(!g["invalid"].is_null(), "{field} = {typed} is marked");
        assert_eq!(
            g["fields"], before,
            "{field} = {typed}: the grid stays as it was"
        );
        assert_eq!(f.s.undo_len(), before_undo, "{field} = {typed}: no step");
    }
}

#[test]
fn c2_a_grid_wider_than_the_bed_names_the_cause() {
    let (mut f, _, _) = grid_fx(&[]);
    f.s.set_test_grid_field("cell", "50").unwrap();
    let err = f.s.set_test_grid_field("columns", "12").unwrap_err();
    assert!(err.starts_with("Grid is "), "{err}");
    assert!(err.contains("mm wide, the bed is 400 mm."), "{err}");
    assert_eq!(f.grid()["fields"]["columns"], 5);
}

#[test]
fn c2_add_on_a_full_bed_is_refused_under_the_button_and_adds_nothing() {
    let (d, laser, _) = document(80.0, 80.0, &[]);
    let mut f = open(&d, &laser, Folder::default());
    f.s.press_add_test_grid().unwrap();
    let undo = f.s.undo_len();
    let rows = f.ops()["rows"].as_array().unwrap().len();
    let err = f.s.press_add_test_grid().unwrap_err();
    assert_ne!(err, "");
    assert_eq!(f.s.test_grid_notice().as_deref(), Some(err.as_str()));
    assert_eq!(f.s.undo_len(), undo);
    assert_eq!(f.ops()["rows"].as_array().unwrap().len(), rows);
}

#[test]
fn c2_the_last_grid_values_are_remembered_for_the_next_grid() {
    let (mut f, _, _) = grid_fx(&[]);
    assert!(f.s.set_test_grid_field("columns", "6").unwrap());
    assert!(f.s.set_test_grid_field("cell", "8").unwrap());
    let writes = f.sync();
    assert!(
        writes.iter().any(|w| w["op"] == "grid_choice"),
        "the choice goes to the config directory: {writes:?}"
    );
    f.s.press_add_test_grid().unwrap();
    let g = f.grid();
    assert_eq!(g["name"], "Test grid 2");
    assert_eq!(g["fields"]["columns"], 6);
    assert_eq!(g["fields"]["cell_mm"].as_f64(), Some(8.0));
}

#[test]
fn c3_add_is_one_step_selected_at_a_free_spot_and_touches_nothing_else() {
    let (d, laser, _) = document(400.0, 300.0, &[]);
    let page = d.pages().into_iter().find(|p| p.id == laser).unwrap();
    let rect = d.create_rect(RectBounds::from_corners(
        Point::new(page.origin.x + 300.0, page.origin.y + 200.0),
        Point::new(page.origin.x + 320.0, page.origin.y + 220.0),
    ));
    let before = d.object(rect).unwrap();
    let mut f = open(&d, &laser, Folder::default());
    let undo = f.s.undo_len();
    f.s.press_add_test_grid().unwrap();
    assert_eq!(f.s.undo_len(), undo + 1);
    assert_eq!(f.undo_name(), "Add test grid");
    assert!(f.s.selected_object_count() > 0, "the grid is selected");
    assert_eq!(
        f.s.key_down(KeyInput {
            key: "s",
            ..KeyInput::default()
        }),
        KeyOutcome::Consumed,
        "the whole grid is selected: scale does nothing"
    );
    let g1 = group_of(&f);
    f.s.press_add_test_grid().unwrap();
    let g2 = group_of(&f);
    assert_ne!(g1, g2);
    let d2 = f.saved_doc();
    assert_eq!(
        d2.object(rect).unwrap(),
        before,
        "nothing else on the page changes"
    );
    // The two grids do not overlap: the second takes the first free spot.
    let b1 = bounds(&d2, g1);
    let b2 = bounds(&d2, g2);
    let overlap = b1.0 < b2.2 && b2.0 < b1.2 && b1.1 < b2.3 && b2.1 < b1.3;
    assert!(!overlap, "{b1:?} {b2:?}");
    // The first sits at the bed's top left, its cells right of and below the labels.
    assert!(
        b1.0 >= page.origin.x && b1.0 - page.origin.x < 30.0,
        "{b1:?}"
    );
    assert!(
        b1.1 >= page.origin.y && b1.1 - page.origin.y < 30.0,
        "{b1:?}"
    );
    // Undo takes the grid and its row away.
    f.s.undo();
    assert_eq!(f.grid_rows().len(), 1);
}

/// (min x, min y, max x, max y) of the cell squares of a grid.
fn bounds(d: &Document, group: NodeId) -> (f64, f64, f64, f64) {
    let g = d.test_grid(group).unwrap();
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for item in &g.cells {
        let r = cell_rect(d, item.id);
        b.0 = b.0.min(r.origin.x);
        b.1 = b.1.min(r.origin.y);
        b.2 = b.2.max(r.origin.x + r.width.as_mm());
        b.3 = b.3.max(r.origin.y + r.height.as_mm());
    }
    b
}

fn cell_prim(d: &Document, id: NodeId) -> curvyo_document_core::PrimitiveSnapshot {
    match d.object(id).unwrap() {
        ObjectSnapshot::Primitive(p) => p,
        other => panic!("a cell is a rectangle primitive: {other:?}"),
    }
}

fn cell_rect(d: &Document, id: NodeId) -> RectBounds {
    match cell_prim(d, id).shape {
        Shape::Rect { bounds, .. } => bounds,
        other => panic!("a cell is a square: {other:?}"),
    }
}

#[test]
fn c5_cut_and_score_cells_are_outlines_engrave_cells_are_filled_squares() {
    let (mut f, _, _) = grid_fx(&[]);
    let group = group_of(&f);
    let d = f.saved_doc();
    for c in &d.test_grid(group).unwrap().cells {
        let p = cell_prim(&d, c.id);
        assert!(!p.style.fill.enabled, "a Cut cell is not filled");
        let r = cell_rect(&d, c.id);
        assert!((r.width.as_mm() - 10.0).abs() < 1e-6 && (r.height.as_mm() - 10.0).abs() < 1e-6);
    }
    assert!(f.s.set_test_grid_kind("engrave").unwrap());
    let d = f.saved_doc();
    for c in &d.test_grid(group).unwrap().cells {
        assert!(
            cell_prim(&d, c.id).style.fill.enabled,
            "an Engrave cell is filled"
        );
    }
}

// ---- criteria 4 to 7: the grid's items and its row ----------------------------------------------

#[test]
fn c4_the_grid_is_ordinary_items_cells_and_labels_in_two_groups_under_one_top_group() {
    let (mut f, _, _) = grid_fx(&[]);
    let group = group_of(&f);
    let d = f.saved_doc();
    let g = d.test_grid(group).unwrap();
    assert_eq!(g.cells.len(), 25);
    let tags: Vec<&str> = g.labels.iter().filter_map(|l| l.tag.as_str()).collect();
    assert_eq!(tags.iter().filter(|t| **t == "power").count(), 5);
    assert_eq!(tags.iter().filter(|t| **t == "speed").count(), 5);
    assert_eq!(tags.iter().filter(|t| **t == "unit").count(), 1);
    let children = d.children(Some(group));
    assert_eq!(children.len(), 2, "squares group and labels group");
    assert_eq!(f.grid()["name"], "Test grid 1");
    // Every cell has its own numbers: 5 distinct powers x 5 distinct speeds.
    let mut pairs: Vec<(String, u64)> = g
        .cells
        .iter()
        .map(|c| {
            (
                c.tag["power_pct"].to_string(),
                c.tag["speed_mm_min"].as_u64().unwrap(),
            )
        })
        .collect();
    pairs.sort();
    pairs.dedup();
    assert_eq!(pairs.len(), 25);
}

#[test]
fn c6_one_grid_row_in_its_section_with_kind_and_cell_count() {
    let (mut f, _, row) = grid_fx(&["cut"]);
    let r = f.row(row);
    assert_eq!(r["name"], "Test grid 1");
    assert_eq!(r["section"], "cutting");
    assert_eq!(r["grid"]["columns"], 5);
    assert_eq!(r["grid"]["rows"], 5);
    assert!(r["settings"].is_null(), "a grid row has no settings line");
    assert!(f.s.set_test_grid_kind("engrave").unwrap());
    assert_eq!(f.row(row)["section"], "marking");
    assert!(f.s.set_test_grid_kind("score").unwrap());
    assert_eq!(f.row(row)["section"], "marking");
}

#[test]
fn c6_deleting_the_top_group_deletes_its_row_in_the_same_step() {
    let (mut f, _, row) = grid_fx(&[]);
    let undo = f.s.undo_len();
    f.s.delete_selected();
    assert_eq!(f.s.undo_len(), undo + 1);
    assert!(f.row(row).is_null(), "the row goes with the grid");
    f.s.undo();
    assert!(!f.row(row).is_null(), "one undo brings both back");
    assert_eq!(f.grid_rows().len(), 1);
}

#[test]
fn c6_deleting_the_row_keeps_the_items_and_they_run_nothing() {
    let (mut f, _, row) = grid_fx(&[]);
    let group = group_of(&f);
    assert!(f.s.delete_operation(row));
    let d = f.saved_doc();
    let g = d.test_grid(group).expect("the items stay");
    assert_eq!(g.cells.len(), 25);
    assert_eq!(g.rows, Vec::<NodeId>::new());
    let na = f.ops()["summary"]["not_assigned"].as_u64().unwrap();
    assert!(na >= 25, "the cells become Not assigned: {na}");
}

#[test]
fn c6_items_cannot_be_assigned_to_a_grid_row() {
    let (d, laser, _) = document(400.0, 300.0, &[]);
    let page = d.pages().into_iter().find(|p| p.id == laser).unwrap();
    let rect = d.create_rect(RectBounds::from_corners(
        Point::new(page.origin.x + 300.0, page.origin.y + 250.0),
        Point::new(page.origin.x + 320.0, page.origin.y + 270.0),
    ));
    let mut f = open(&d, &laser, Folder::default());
    let row = f.s.press_add_test_grid().unwrap();
    // Select the rectangle alone and try to put it into the grid row.
    let at = Point::new(page.origin.x + 300.0, page.origin.y + 260.0);
    f.s.pointer_hover(at, false, false);
    f.s.pointer_down(at, false);
    f.s.pointer_up(at, false, false);
    let undo = f.s.undo_len();
    assert!(!f.s.set_selection_in_operation(row, true));
    assert_eq!(f.s.undo_len(), undo);
    let d2 = f.saved_doc();
    let m = d2
        .pages()
        .into_iter()
        .find_map(|p| p.kind.manufacturing())
        .unwrap();
    let op = d2
        .operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.id == row)
        .unwrap();
    assert!(!op.members.iter().any(|p| p.contains(&rect)));
}

#[test]
fn c7_labels_off_makes_no_label_and_cut_is_never_a_label_kind() {
    let (mut f, _, _) = grid_fx(&[]);
    let group = group_of(&f);
    assert!(f.s.set_test_grid_label_kind("cut").is_err());
    assert_eq!(f.grid()["labels"]["kind"], "engrave");
    assert!(f.s.set_test_grid_label_kind("score").unwrap());
    assert_eq!(f.grid()["labels"]["kind"], "score");
    assert!(f.s.set_test_grid_labels(false).unwrap());
    let d = f.saved_doc();
    let g = d.test_grid(group).unwrap();
    assert!(g.labels.is_empty(), "no label text, no unit line");
    assert_eq!(g.cells.len(), 25);
}

// ---- criteria 8 to 11: editing like a rectangle -------------------------------------------------

#[test]
fn c8_kind_change_rebuilds_clears_the_mark_and_moves_the_row() {
    let (mut f, _, row) = grid_fx(&[]);
    assert!(f.s.press_test_cell(2, 1));
    assert!(!f.grid()["worked"].is_null());
    assert!(f.s.set_test_grid_kind("engrave").unwrap());
    assert!(
        f.grid()["worked"].is_null(),
        "a Kind change clears the mark"
    );
    assert_eq!(f.grid()["kind"], "engrave");
    assert_eq!(f.row(row)["section"], "marking");
}

#[test]
fn c8_a_whole_grid_moves_like_a_rectangle_and_is_not_resized() {
    let (mut f, _, _) = grid_fx(&[]);
    let group = group_of(&f);
    for k in ["s", "r", "k"] {
        assert_eq!(
            f.s.key_down(KeyInput {
                key: k,
                ..KeyInput::default()
            }),
            KeyOutcome::Consumed,
            "{k}: no resize, rotate or skew entry for a whole grid"
        );
    }
    let before = bounds(&f.saved_doc(), group);
    let _ = f.s.key_down(KeyInput {
        key: "ArrowRight",
        ..KeyInput::default()
    });
    let after = bounds(&f.saved_doc(), group);
    assert!(
        after.0 > before.0 + 1e-6,
        "the arrow key moves the grid: {before:?} {after:?}"
    );
    assert!(
        (after.2 - after.0 - (before.2 - before.0)).abs() < 1e-6,
        "same size"
    );
    assert_eq!(
        f.grid()["hand_edited"],
        false,
        "a moved grid is not hand edited"
    );
}

#[test]
fn c9_field_edit_is_one_step_keeps_the_mark_when_its_cell_keeps_power_and_speed() {
    let (mut f, _, row) = grid_fx(&[]);
    assert!(f.s.press_test_cell(2, 1)); // 60 %, row 1 speed
    let speed = f.grid()["worked"]["speed_mm_min"].clone();
    assert!(f.s.set_operation_switch(row, OperationSwitch::Output, false));
    assert!(f.s.set_operation_switch(row, OperationSwitch::Show, false));
    let undo = f.s.undo_len();
    assert!(f.s.set_test_grid_field("gap", "4").unwrap());
    assert_eq!(f.s.undo_len(), undo + 1);
    assert_eq!(f.undo_name(), "Edit test grid");
    assert_eq!(f.grid()["worked"]["power_pct"].as_f64(), Some(60.0));
    // 9 columns from 20 to 100: 60 % moves to column 4, speed unchanged: the mark follows.
    assert!(f.s.set_test_grid_field("columns", "9").unwrap());
    let w = f.grid()["worked"].clone();
    assert_eq!(w["power_pct"].as_f64(), Some(60.0));
    assert_eq!(w["speed_mm_min"], speed);
    assert_eq!(w["cell"]["column"], 4);
    // Show and Output of the row are kept.
    let r = f.row(row);
    assert_eq!(r["output"], false);
    assert_eq!(r["show"], false);
    // Power from 30: 30, 38.75, ... no 60 any more: the mark goes.
    assert!(f.s.set_test_grid_field("power_from", "30").unwrap());
    assert!(f.grid()["worked"].is_null());
    // Undo brings the mark back.
    f.s.undo();
    assert_eq!(f.grid()["worked"]["power_pct"].as_f64(), Some(60.0));
}

#[test]
fn c9_retyping_the_stored_value_is_no_step() {
    let (mut f, _, _) = grid_fx(&[]);
    let undo = f.s.undo_len();
    assert!(!f.s.set_test_grid_field("columns", "5").unwrap());
    assert_eq!(f.s.undo_len(), undo);
}

#[test]
fn c10_a_hand_edited_grid_is_not_rebuilt_until_rebuild_which_is_one_step() {
    let (mut f, _, _) = grid_fx(&[]);
    let group = group_of(&f);
    let d = f.saved_doc();
    let g = d.test_grid(group).unwrap();
    let cell = g.cells[3].id;
    let tag_before = g.cells[3].tag.clone();
    d.translate_objects(&[cell], Vec2::new(1.5, 0.0)).unwrap();
    let laser = f.laser.clone();
    let mut h = open(&d, &laser, Folder::default());
    h.s.select_test_grid_row(Some(d.test_grid(group).unwrap().rows[0]));
    assert_eq!(
        h.grid()["hand_edited"],
        true,
        "the fields show Edited by hand"
    );
    // A field edit on a hand-edited grid does not rebuild the items.
    let _ = h.s.set_test_grid_field("gap", "4");
    let d2 = h.saved_doc();
    let moved = d2.test_grid(group).unwrap();
    assert!(
        moved.cells.iter().any(|c| c.id == cell),
        "the moved cell is not replaced"
    );
    assert_eq!(
        moved.cells.iter().find(|c| c.id == cell).unwrap().tag,
        tag_before,
        "the moved cell keeps its own numbers"
    );
    let undo = h.s.undo_len();
    assert!(h.s.rebuild_test_grid());
    assert_eq!(h.s.undo_len(), undo + 1);
    assert_eq!(h.grid()["hand_edited"], false);
}

#[test]
fn c11_a_duplicate_is_a_grid_of_its_own_with_no_mark_and_its_own_row() {
    let (mut f, _, row) = grid_fx(&[]);
    assert!(f.s.press_test_cell(1, 1));
    let fields = f.grid()["fields"].clone();
    let group = group_of(&f);
    f.s.select_focused_test_grid();
    let _ = f.s.duplicate_layer_target();
    let rows = f.grid_rows();
    assert_eq!(rows.len(), 2, "the copy has its own row");
    let d = f.saved_doc();
    let other = rows
        .iter()
        .map(|r| serde_json::from_value::<NodeId>(r["grid"]["group"].clone()).unwrap())
        .find(|g| *g != group)
        .unwrap();
    let copy = d.test_grid(other).unwrap();
    assert!(copy.worked.is_none(), "no Worked mark on the copy");
    assert_eq!(copy.rows.len(), 1);
    assert_ne!(copy.rows[0], row);
    let original = d.test_grid(group).unwrap();
    assert!(original.worked.is_some(), "the original keeps its mark");
    assert_eq!(original.rows, vec![row]);
    assert_eq!(copy.recipe["columns"], fields["columns"]);
    assert_eq!(copy.recipe, original.recipe);
}

// ---- criteria 12 to 14: the mark ----------------------------------------------------------------

#[test]
fn c13_marking_is_exclusive_toggles_and_each_press_is_one_step() {
    let (mut f, _, _) = grid_fx(&[]);
    let undo = f.s.undo_len();
    assert!(f.s.press_test_cell(0, 0));
    assert_eq!(f.undo_name(), "Mark cell");
    assert!(f.s.press_test_cell(3, 4));
    let w = f.grid()["worked"].clone();
    assert_eq!(w["cell"], json!({"column": 3, "row": 4}));
    assert!(f.s.press_test_cell(3, 4));
    assert_eq!(f.undo_name(), "Unmark cell");
    assert!(f.grid()["worked"].is_null());
    assert_eq!(f.s.undo_len(), undo + 3);
    assert!(!f.s.press_test_cell(12, 0), "outside the grid");
}

#[test]
fn c13_the_mark_is_kept_in_the_project_file() {
    let (mut f, _, row) = grid_fx(&[]);
    assert!(f.s.press_test_cell(2, 3));
    let d = f.saved_doc();
    let laser = f.laser.clone();
    let mut h = open(&d, &laser, Folder::default());
    h.s.select_test_grid_row(Some(row));
    assert_eq!(h.grid()["worked"]["cell"], json!({"column": 2, "row": 3}));
}

#[test]
fn c14_the_worked_text_names_power_and_speed() {
    let (mut f, _, _) = grid_fx(&[]);
    assert!(f.grid()["worked"].is_null());
    assert!(f.s.press_test_cell(2, 1));
    let w = f.grid()["worked"].clone();
    let speed = w["speed_mm_min"].as_u64().unwrap();
    assert_eq!(w["text"], format!("Worked: 60 % power, {speed} mm/min"));
}

// ---- criteria 15 to 18: Save as material and the record -----------------------------------------

#[test]
fn c15_c16_save_as_material_writes_one_record_with_its_test_as_source() {
    let (mut f, _, _) = grid_fx(&[]);
    assert_eq!(f.mats()["next_name"], "Material 1");
    let undo = f.s.undo_len();
    let id = f.save_new("Birch 3 mm");
    assert_eq!(f.s.undo_len(), undo + 2, "mark + Save as material");
    assert_eq!(f.undo_name(), "Save as material");
    let rec = f
        .folder
        .record(&id)
        .expect("one file materials/<uuid>.json");
    assert_eq!(f.folder.files.len(), 1);
    assert_eq!(rec["format_version"], 1);
    assert!(rec["modified"].as_u64().is_some());
    assert_eq!(rec["name"], "Birch 3 mm");
    assert_eq!(rec["thickness_mm"].as_f64(), Some(3.0));
    assert_eq!(rec["machine"]["id"], "generic-laser");
    assert_eq!(rec["machine"]["name"], "Generic laser");
    let set = &rec["sets"]["cut"];
    assert_eq!(set["power_pct"].as_f64(), Some(60.0));
    assert_eq!(set["passes"], 1);
    let hist = rec["history"].as_array().unwrap();
    assert_eq!(hist.len(), 1);
    assert_eq!(set["source"]["test"], hist[0]["id"]);
    assert_eq!(hist[0]["kind"], "cut");
    assert_eq!(hist[0]["cell"], json!({"column": 2, "row": 1}));
    assert_eq!(hist[0]["grid"]["columns"], 5);
    assert!(rec["sets"].get("score").is_none());
    // The row lists it with its kinds.
    let rows = f.mats()["rows"].clone();
    assert_eq!(rows[0]["name"], "Birch 3 mm");
    assert_eq!(rows[0]["kinds"], json!(["cut"]));
}

#[test]
fn c15_invalid_form_values_are_refused_and_write_nothing() {
    let (mut f, _, _) = grid_fx(&[]);
    assert!(f.s.press_test_cell(1, 1));
    let undo = f.s.undo_len();
    for (name, t, p) in [
        ("", 3.0, 1.0),
        ("x", 0.0, 1.0),
        ("x", 0.05, 1.0),
        ("x", 100.1, 1.0),
        ("x", 3.05, 1.0),
        ("x", 3.0, 0.0),
        ("x", 3.0, 21.0),
        ("x", 3.0, 1.5),
    ] {
        let r = f.random();
        let res = f.s.save_as_material(
            &SaveAsMaterial {
                existing: None,
                name: name.to_owned(),
                thickness_mm: t,
                passes: p,
            },
            &r,
        );
        assert!(res.is_err(), "{name:?} {t} {p}");
    }
    let r = f.random();
    let long = "n".repeat(61);
    assert!(
        f.s.save_as_material(
            &SaveAsMaterial {
                existing: None,
                name: long,
                thickness_mm: 3.0,
                passes: 1.0
            },
            &r
        )
        .is_err(),
        "61 characters"
    );
    assert_eq!(f.s.undo_len(), undo);
    assert!(f.sync().iter().all(|w| w["op"] != "write"));
    // Without a mark the save is refused.
    assert!(f.s.press_test_cell(1, 1));
    let r = f.random();
    assert!(
        f.s.save_as_material(
            &SaveAsMaterial {
                existing: None,
                name: "x".into(),
                thickness_mm: 3.0,
                passes: 1.0
            },
            &r
        )
        .is_err()
    );
}

#[test]
fn c15_c18_c32_replace_appends_history_and_undo_removes_exactly_that_entry() {
    let (mut f, _, _) = grid_fx(&[]);
    let id = f.save_new("Birch");
    let first = f.folder.record(&id).unwrap();
    // Mark another cell, save into the existing material: the Cut set is replaced.
    assert!(f.s.press_test_cell(4, 4));
    let r = f.random();
    let got =
        f.s.save_as_material(
            &SaveAsMaterial {
                existing: Some(id.clone()),
                name: String::new(),
                thickness_mm: 0.0,
                passes: 2.0,
            },
            &r,
        )
        .unwrap();
    assert_eq!(got, id);
    f.sync();
    let second = f.folder.record(&id).unwrap();
    let h = second["history"].as_array().unwrap();
    assert_eq!(h.len(), 2, "the old set stays in the history");
    assert_eq!(h[0], first["history"][0]);
    assert_eq!(second["sets"]["cut"]["power_pct"].as_f64(), Some(100.0));
    assert_eq!(second["sets"]["cut"]["passes"], 2);
    assert_eq!(second["sets"]["cut"]["source"]["test"], h[1]["id"]);
    // Undo: the record is as before the step, entry removed.
    f.s.undo();
    f.sync();
    let back = f.folder.record(&id).unwrap();
    assert_eq!(back["history"], first["history"]);
    assert_eq!(back["sets"], first["sets"]);
    // Redo adds it again.
    f.s.redo();
    f.sync();
    let again = f.folder.record(&id).unwrap();
    assert_eq!(again["history"].as_array().unwrap().len(), 2);
    assert_eq!(again["sets"], second["sets"]);
}

#[test]
fn c15_c32_undo_of_save_as_new_material_removes_the_file_and_redo_writes_it() {
    let (mut f, _, _) = grid_fx(&[]);
    let id = f.save_new("Oak");
    let text = f.folder.files[&format!("{id}.json")].clone();
    let r = f.s.undo();
    assert!(r.changed);
    let w = f.sync();
    assert!(
        w.iter()
            .any(|w| w["op"] == "remove" && w["id"] == id.as_str()),
        "{w:?}"
    );
    assert!(f.folder.files.is_empty());
    f.s.redo();
    f.sync();
    let again: Value = serde_json::from_str(&f.folder.files[&format!("{id}.json")]).unwrap();
    let was: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(again["history"], was["history"]);
    assert_eq!(again["sets"], was["sets"]);
    // The mark is still in the project after the undo of the save.
    f.s.undo();
    assert!(!f.grid()["worked"].is_null());
}

// ---- criteria 19 to 23: the Materials section ---------------------------------------------------

#[test]
fn c19_c23_the_list_shows_only_this_machines_materials_by_name_with_their_kinds() {
    let (f, _) = lib_fx(&[]);
    let rows = f.mats()["rows"].as_array().unwrap().clone();
    let names: Vec<&str> = rows.iter().filter_map(|r| r["name"].as_str()).collect();
    assert_eq!(
        names,
        vec!["Birch", "Card"],
        "another machine's material is not shown"
    );
    assert_eq!(rows[0]["kinds"], json!(["cut", "score", "engrave"]));
    assert_eq!(rows[0]["thickness_mm"].as_f64(), Some(3.0));
    let text = f.s.materials_view_json();
    for colour in ["colour", "color", "#"] {
        assert!(!text.contains(colour), "nothing is coloured: {colour}");
    }
}

#[test]
fn c20_rename_in_place_rules() {
    let (mut f, _) = lib_fx(&[]);
    let undo = f.s.undo_len();
    assert!(!f.s.rename_material(BIRCH, "").unwrap(), "empty reverts");
    assert!(!f.s.rename_material(BIRCH, "Birch").unwrap(), "unchanged");
    assert!(f.s.rename_material(BIRCH, &"n".repeat(61)).is_err());
    assert_eq!(f.s.undo_len(), undo);
    assert!(
        f.s.rename_material(BIRCH, "Card").unwrap(),
        "names need not be unique"
    );
    assert_eq!(f.s.undo_len(), undo + 1);
    assert_eq!(f.undo_name(), "Rename material");
    f.sync();
    assert_eq!(f.folder.record(BIRCH).unwrap()["name"], "Card");
    f.s.undo();
    f.sync();
    assert_eq!(f.folder.record(BIRCH).unwrap()["name"], "Birch");
}

#[test]
fn c21_delete_removes_the_record_pointing_operations_use_stored_numbers_undo_restores() {
    let (mut f, ids) = lib_fx(&["cut"]);
    let text = f.folder.files[&format!("{BIRCH}.json")].clone();
    assert!(f.s.use_material_on(&ids, BIRCH));
    assert_eq!(f.row(ids[0])["settings"]["state"], "live");
    let uses = f.mats()["rows"][0]["uses"].as_u64();
    assert_eq!(
        uses,
        Some(1),
        "the arming text can name the operations pointing to it"
    );
    let undo = f.s.undo_len();
    f.s.delete_material(BIRCH).unwrap();
    assert_eq!(f.s.undo_len(), undo + 1);
    assert_eq!(f.undo_name(), "Delete material");
    let w = f.sync();
    assert!(w.iter().any(|w| w["op"] == "remove" && w["id"] == BIRCH));
    let st = f.row(ids[0])["settings"].clone();
    assert_eq!(st["state"], "stored");
    assert_eq!(st["reason"], "Material not found, using stored numbers");
    assert_eq!(st["sets"][0]["run"]["power_pct"].as_f64(), Some(60.0));
    f.s.undo();
    f.sync();
    let back: Value = serde_json::from_str(&f.folder.files[&format!("{BIRCH}.json")]).unwrap();
    let was: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(back["history"], was["history"], "the history comes back");
    assert_eq!(back["sets"], was["sets"]);
    assert_eq!(f.row(ids[0])["settings"]["state"], "live");
}

#[test]
fn c22_order_is_a_preference_not_a_step() {
    let (mut f, _) = lib_fx(&[]);
    let undo = f.s.undo_len();
    assert!(f.s.move_material(CARD, 0));
    assert_eq!(f.s.undo_len(), undo, "the order is not a step");
    let w = f.sync();
    assert!(w.iter().any(|w| w["op"] == "order"));
    let names: Vec<String> = f.mats()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["name"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(names, vec!["Card", "Birch"]);
    // A new material comes after the ordered ones, by name.
    let (d, laser, _) = document(400.0, 300.0, &[]);
    let mut folder = f.folder.clone();
    folder.files.insert(
        "77777777-7777-4777-8777-777777777777.json".into(),
        record_text(
            "77777777-7777-4777-8777-777777777777",
            "Acrylic",
            "generic-laser",
            json!({"cut": {"power_pct": 50.0, "speed_mm_min": 500, "passes": 1}}),
            1,
        ),
    );
    let h = open(&d, &laser, folder);
    let names: Vec<String> = h.mats()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["name"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(names.first().map(String::as_str), Some("Card"));
    let pos_acr = names.iter().position(|n| n == "Acrylic").unwrap();
    let pos_birch = names.iter().position(|n| n == "Birch").unwrap();
    let ordered_birch = f.folder.order.as_deref().unwrap_or("").contains(BIRCH);
    if ordered_birch {
        assert!(pos_birch < pos_acr, "ordered before unordered: {names:?}");
    } else {
        assert!(pos_acr < pos_birch, "unordered by name: {names:?}");
    }
}

#[test]
fn c22_c26_a_set_edit_is_one_step_that_refreshes_every_pointing_operation_except_overrides() {
    let (mut f, ids) = lib_fx(&["cut", "cut"]);
    assert!(f.s.use_material_on(&ids, BIRCH));
    assert!(
        f.s.edit_operation_setting(ids[1], "main", "speed", 450.0)
            .unwrap()
    );
    let undo = f.s.undo_len();
    let r = f.random();
    assert!(
        f.s.edit_material_set(BIRCH, "cut", "speed", 333.0, &r)
            .unwrap()
    );
    assert_eq!(f.s.undo_len(), undo + 1);
    assert_eq!(f.undo_name(), "Edit material");
    f.sync();
    let rec = f.folder.record(BIRCH).unwrap();
    assert_eq!(rec["sets"]["cut"]["speed_mm_min"], 333);
    assert_eq!(rec["sets"]["cut"]["source"], "edited");
    assert!(
        rec["history"].as_array().unwrap().len() >= 4,
        "the history keeps the old numbers"
    );
    // Live: the plain operation changes in the same frame; the overridden speed does not.
    assert_eq!(
        f.row(ids[0])["settings"]["sets"][0]["run"]["speed_mm_min"],
        333
    );
    assert_eq!(
        f.row(ids[1])["settings"]["sets"][0]["run"]["speed_mm_min"],
        450
    );
    assert_eq!(
        f.row(ids[1])["settings"]["sets"][0]["material"]["speed_mm_min"],
        333
    );
    // Stored numbers refreshed in the same step (the project file without a save refresh).
    let bytes = pack(&unpack(9, &f.s.pack_for_save().unwrap()).unwrap(), "x").unwrap();
    let d = unpack(10, &bytes).unwrap();
    let s0 = op_settings(&d, ids[0]).unwrap();
    assert_eq!(s0["main"]["speed_mm_min"], 333);
    let s1 = op_settings(&d, ids[1]).unwrap();
    assert_eq!(s1["main"]["speed_mm_min"], 333);
    assert_eq!(
        s1["overrides"]["main"]["speed_mm_min"], 450,
        "overrides never refreshed"
    );
    // Undo: record and stored numbers back.
    f.s.undo();
    f.sync();
    assert_eq!(
        f.folder.record(BIRCH).unwrap()["sets"]["cut"]["speed_mm_min"],
        300
    );
    assert_eq!(
        f.row(ids[0])["settings"]["sets"][0]["run"]["speed_mm_min"],
        300
    );
}

#[test]
fn c22_thickness_and_set_removal_are_steps_and_the_removal_is_in_the_history() {
    let (mut f, ids) = lib_fx(&["score"]);
    assert!(f.s.use_material_on(&ids, BIRCH));
    assert!(f.s.set_material_thickness(BIRCH, 4.5).unwrap());
    assert!(!f.s.set_material_thickness(BIRCH, 4.5).unwrap());
    assert!(f.s.set_material_thickness(BIRCH, 0.0).is_err());
    let before = {
        f.sync();
        f.folder.record(BIRCH).unwrap()["history"]
            .as_array()
            .unwrap()
            .len()
    };
    let r = f.random();
    f.s.remove_material_set(BIRCH, "score", &r).unwrap();
    f.sync();
    let rec = f.folder.record(BIRCH).unwrap();
    assert!(rec["sets"].get("score").is_none());
    assert_eq!(
        rec["history"].as_array().unwrap().len(),
        before + 1,
        "removal appends"
    );
    let st = f.row(ids[0])["settings"].clone();
    assert_eq!(st["state"], "stored");
    assert_eq!(
        st["reason"],
        "Material has no Score set, using stored numbers"
    );
    assert_eq!(st["sets"][0]["run"]["power_pct"].as_f64(), Some(12.5));
}

// ---- criteria 24 to 27: settings on an operation ------------------------------------------------

#[test]
fn c24_no_settings_and_own_numbers_on_a_new_operation() {
    let (mut f, ids) = fx(&["cut"]);
    assert_eq!(f.row(ids[0])["settings"]["state"], "no_settings");
    let _ = f.s.edit_operation_setting(ids[0], "main", "power", 55.5);
    assert!(
        f.s.edit_operation_setting(ids[0], "main", "speed", 1200.0)
            .unwrap()
    );
    let st = f.row(ids[0])["settings"].clone();
    assert_eq!(st["state"], "own");
    assert_eq!(st["sets"][0]["run"]["power_pct"].as_f64(), Some(55.5));
    assert_eq!(st["sets"][0]["run"]["passes"], 1);
    assert_eq!(f.undo_name(), "Edit settings");
    // Ranges: refused, nothing written.
    let undo = f.s.undo_len();
    for (field, v) in [
        ("power", 0.0),
        ("power", 100.1),
        ("speed", 0.0),
        ("speed", 6001.0),
        ("speed", 10.5),
        ("passes", 0.0),
        ("passes", 21.0),
    ] {
        assert!(
            f.s.edit_operation_setting(ids[0], "main", field, v)
                .is_err(),
            "{field} {v}"
        );
    }
    assert_eq!(f.s.undo_len(), undo);
}

#[test]
fn c24_a_typed_number_on_a_linked_operation_keeps_the_link_as_an_override() {
    let (mut f, ids) = lib_fx(&["cut"]);
    assert!(f.s.use_material_on(&ids, BIRCH));
    assert_eq!(f.undo_name(), "Use material");
    assert!(
        f.s.edit_operation_setting(ids[0], "main", "power", 65.0)
            .unwrap()
    );
    let st = f.row(ids[0])["settings"].clone();
    assert_eq!(st["state"], "live");
    assert_eq!(st["material"]["id"], BIRCH);
    assert_eq!(st["differs"], "Differs from Birch");
    assert_eq!(st["reset"], true);
    assert_eq!(st["sets"][0]["typed"], json!(["power"]));
    assert_eq!(st["sets"][0]["run"]["power_pct"].as_f64(), Some(65.0));
    assert_eq!(st["sets"][0]["material"]["power_pct"].as_f64(), Some(60.0));
    // Typing the material's number removes that override.
    assert!(
        f.s.edit_operation_setting(ids[0], "main", "power", 60.0)
            .unwrap()
    );
    let st = f.row(ids[0])["settings"].clone();
    assert!(st["differs"].is_null());
    assert_eq!(st["sets"][0]["typed"], json!([]));
    // Reset to material.
    assert!(
        f.s.edit_operation_setting(ids[0], "main", "passes", 3.0)
            .unwrap()
    );
    assert!(f.s.reset_operation_to_material(ids[0]));
    assert_eq!(f.undo_name(), "Reset to material");
    assert!(f.row(ids[0])["settings"]["differs"].is_null());
    assert!(!f.s.reset_operation_to_material(ids[0]), "nothing to reset");
}

#[test]
fn c24_c25_own_numbers_end_the_link_and_keep_what_ran() {
    let (mut f, ids) = lib_fx(&["cut"]);
    assert!(f.s.use_material_on(&ids, BIRCH));
    assert!(
        f.s.edit_operation_setting(ids[0], "main", "power", 70.0)
            .unwrap()
    );
    assert!(f.s.use_own_numbers_on(&ids));
    let st = f.row(ids[0])["settings"].clone();
    assert_eq!(st["state"], "own");
    assert!(st["material"].is_null());
    let run = &st["sets"][0]["run"];
    assert_eq!(run["power_pct"].as_f64(), Some(70.0));
    assert_eq!(run["speed_mm_min"], 300);
    assert_eq!(run["passes"], 2);
}

#[test]
fn c25_a_pick_clears_overrides_also_of_the_same_material() {
    let (mut f, ids) = lib_fx(&["cut"]);
    assert!(f.s.use_material_on(&ids, BIRCH));
    assert!(
        f.s.edit_operation_setting(ids[0], "main", "power", 65.0)
            .unwrap()
    );
    let undo = f.s.undo_len();
    assert!(f.s.use_material_on(&ids, BIRCH));
    assert_eq!(f.s.undo_len(), undo + 1);
    assert!(f.row(ids[0])["settings"]["differs"].is_null());
    assert!(f.s.use_material_on(&ids, CARD));
    let st = f.row(ids[0])["settings"].clone();
    assert_eq!(st["material"]["name"], "Card");
    assert_eq!(st["sets"][0]["run"]["power_pct"].as_f64(), Some(40.0));
}

#[test]
fn c25_engrave_outline_takes_engrave_for_the_hatch_and_score_for_the_outline() {
    let (mut f, ids) = lib_fx(&["engrave_outline", "cut_out"]);
    assert!(f.s.use_material_on(&ids[..1], BIRCH));
    let sets = f.row(ids[0])["settings"]["sets"].clone();
    assert_eq!(sets[0]["role"], "main");
    assert_eq!(sets[0]["run"]["power_pct"].as_f64(), Some(35.0));
    assert_eq!(sets[1]["role"], "outline");
    assert_eq!(sets[1]["run"]["power_pct"].as_f64(), Some(12.5));
    // Cut-out takes the Cut set.
    assert!(f.s.use_material_on(&ids[1..], BIRCH));
    assert_eq!(
        f.row(ids[1])["settings"]["sets"][0]["run"]["power_pct"].as_f64(),
        Some(60.0)
    );
    // Without a Score set the outline takes the Engrave set (live, refreshed by the edit).
    let r = f.random();
    f.s.remove_material_set(BIRCH, "score", &r).unwrap();
    let _ = f.s.use_material_on(&ids[..1], BIRCH);
    let sets = f.row(ids[0])["settings"]["sets"].clone();
    assert_eq!(sets[1]["run"]["power_pct"].as_f64(), Some(35.0));
    assert_eq!(f.row(ids[0])["settings"]["state"], "live");
}

#[test]
fn c25_a_material_without_the_needed_set_is_disabled_in_the_pick() {
    let (mut f, ids) = lib_fx(&["engrave"]);
    let st = f.row(ids[0])["settings"].clone();
    let disabled = st["disabled"].as_array().unwrap();
    assert!(
        disabled
            .iter()
            .any(|d| d["id"] == CARD && d["reason"] == "No Engrave set"),
        "{disabled:?}"
    );
    let undo = f.s.undo_len();
    assert!(!f.s.use_material_on(&ids, CARD));
    assert_eq!(f.s.undo_len(), undo);
    // The other machine's material is not in the pick.
    let pick = f.ops()["pick"].to_string();
    assert!(!pick.contains(ALIEN));
}

#[test]
fn c26_stored_state_for_missing_other_machine_damaged_and_newer_never_changes_numbers() {
    let (d, laser, ids) = document(400.0, 300.0, &["cut", "cut", "cut", "cut"]);
    let mut f = open(&d, &laser, birch_folder(1_762_948_800_000));
    assert!(f.s.use_material_on(&ids, BIRCH));
    assert!(
        f.s.edit_operation_setting(ids[3], "main", "passes", 4.0)
            .unwrap()
    );
    let saved = f.saved_doc();
    let before: Vec<Value> = ids
        .iter()
        .map(|i| op_settings(&saved, *i).unwrap())
        .collect();
    // Elsewhere: Birch missing, then damaged, then newer, then tied to another machine.
    let variants: Vec<(&str, Folder)> = vec![
        ("missing", Folder::default()),
        ("damaged", {
            let mut fo = Folder::default();
            fo.files
                .insert(format!("{BIRCH}.json"), "{ not json".into());
            fo
        }),
        ("newer", {
            let mut fo = birch_folder(1);
            let t = fo.files[&format!("{BIRCH}.json")]
                .replace("\"format_version\": 1", "\"format_version\": 2");
            fo.files.insert(format!("{BIRCH}.json"), t);
            fo
        }),
        ("other machine", {
            let mut fo = birch_folder(1);
            let t = fo.files[&format!("{BIRCH}.json")].replace(
                "\"generic-laser\"",
                "\"6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f\"",
            );
            fo.files.insert(format!("{BIRCH}.json"), t);
            fo
        }),
    ];
    for (what, folder) in variants {
        let mut h = open(&saved, &laser, folder);
        for (n, id) in ids.iter().enumerate() {
            let st = h.row(*id)["settings"].clone();
            assert_eq!(st["state"], "stored", "{what}");
            assert_eq!(
                st["reason"], "Material not found, using stored numbers",
                "{what}"
            );
            assert_eq!(st["material"]["name"], "Birch", "{what}");
            let run = &st["sets"][0]["run"];
            assert_eq!(run["power_pct"].as_f64(), Some(60.0), "{what}");
            assert_eq!(run["speed_mm_min"], 300, "{what}");
            let passes = if n == 3 { 4 } else { 2 };
            assert_eq!(
                run["passes"], passes,
                "{what}: overrides over stored numbers"
            );
        }
        // Saves back unchanged: link and stored numbers kept.
        let again = h.saved_doc();
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(op_settings(&again, *id).unwrap(), before[i], "{what}");
        }
        assert!(
            h.sync()
                .iter()
                .all(|w| w["op"] != "write" && w["op"] != "remove"),
            "{what}: reads never write"
        );
    }
}

#[test]
fn c26_opening_where_the_material_changed_is_live_and_a_save_refreshes_stored_numbers() {
    let (d, laser, ids) = document(400.0, 300.0, &["cut"]);
    let mut f = open(&d, &laser, birch_folder(1_762_948_800_000));
    assert!(f.s.use_material_on(&ids, BIRCH));
    assert!(
        f.s.edit_operation_setting(ids[0], "main", "passes", 5.0)
            .unwrap()
    );
    let saved = f.saved_doc();
    // Another computer's library has a newer Birch with other Cut numbers.
    let mut other = Folder::default();
    other.files.insert(
        format!("{BIRCH}.json"),
        record_text(
            BIRCH,
            "Birch",
            "generic-laser",
            json!({"cut": {"power_pct": 77.0, "speed_mm_min": 222, "passes": 1}}),
            1_762_999_999_000,
        ),
    );
    let mut h = open(&saved, &laser, other);
    let st = h.row(ids[0])["settings"].clone();
    assert_eq!(st["state"], "live");
    assert_eq!(st["sets"][0]["run"]["power_pct"].as_f64(), Some(77.0));
    assert_eq!(st["sets"][0]["run"]["passes"], 5, "the override stays");
    let mut base = open(&saved, &laser, birch_folder(1_762_948_800_000));
    assert_eq!(
        h.s.is_dirty(),
        base.s.is_dirty(),
        "nothing is written on open"
    );
    assert_eq!(
        h.s.history_len(),
        base.s.history_len(),
        "nothing is written on open"
    );
    // A save refreshes the stored numbers, never the overrides.
    let d2 = h.saved_doc();
    let s = op_settings(&d2, ids[0]).unwrap();
    assert_eq!(s["main"]["power_pct"].as_f64(), Some(77.0));
    assert_eq!(s["main"]["speed_mm_min"], 222);
    assert_eq!(s["overrides"]["main"]["passes"], 5);
    assert_eq!(s["material"]["modified"], 1_762_999_999_000_u64);
}

#[test]
fn c26_a_save_does_not_refresh_from_an_older_record() {
    // The guard: a record whose `modified` is not above the link's does not rewrite.
    let (d, laser, ids) = document(400.0, 300.0, &["cut"]);
    let mut f = open(&d, &laser, birch_folder(1_762_948_800_000));
    assert!(f.s.use_material_on(&ids, BIRCH));
    let saved = f.saved_doc();
    let before = op_settings(&saved, ids[0]).unwrap();
    let mut older = Folder::default();
    older.files.insert(
        format!("{BIRCH}.json"),
        record_text(
            BIRCH,
            "Birch",
            "generic-laser",
            json!({"cut": {"power_pct": 11.0, "speed_mm_min": 111, "passes": 1}}),
            1_000,
        ),
    );
    let mut h = open(&saved, &laser, older);
    assert_eq!(h.row(ids[0])["settings"]["state"], "live");
    let d2 = h.saved_doc();
    assert_eq!(op_settings(&d2, ids[0]).unwrap(), before);
}

#[test]
fn c27_a_pick_on_several_rows_is_one_step_and_names_the_skipped_ones() {
    let (mut f, ids) = lib_fx(&["cut", "engrave", "score", "cut"]);
    for id in &ids {
        let _ = f.s.edit_operation_setting(*id, "main", "power", 50.0);
        let _ = f.s.edit_operation_setting(*id, "main", "speed", 500.0);
    }
    let undo = f.s.undo_len();
    assert!(f.s.use_material_on(&ids, CARD));
    assert_eq!(f.s.undo_len(), undo + 1);
    let notice = f.s.operations_notice().unwrap_or_default();
    assert!(
        !notice.is_empty() && notice.chars().count() <= 70,
        "{notice:?}"
    );
    assert_eq!(f.row(ids[0])["settings"]["material"]["name"], "Card");
    assert_eq!(f.row(ids[3])["settings"]["material"]["name"], "Card");
    assert_eq!(
        f.row(ids[1])["settings"]["state"],
        "own",
        "skipped, untouched"
    );
    assert_eq!(
        f.row(ids[2])["settings"]["state"],
        "own",
        "skipped, untouched"
    );
}

// ---- criteria 28 and 32: files, undo limits -----------------------------------------------------

#[test]
fn c28_save_and_reopen_keeps_settings_links_grids_and_marks() {
    let (mut f, ids, row) = {
        let (d, laser, ids) = document(400.0, 300.0, &["cut", "engrave_outline", "score"]);
        let mut f = open(&d, &laser, birch_folder(1_762_948_800_000));
        let row = f.s.press_add_test_grid().unwrap();
        (f, ids, row)
    };
    assert!(f.s.press_test_cell(1, 2));
    assert!(f.s.use_material_on(&ids[..2], BIRCH));
    assert!(
        f.s.edit_operation_setting(ids[1], "outline", "power", 30.0)
            .unwrap()
    );
    let _ = f.s.edit_operation_setting(ids[2], "main", "power", 22.0);
    assert!(
        f.s.edit_operation_setting(ids[2], "main", "speed", 2200.0)
            .unwrap()
    );
    let view = f.ops();
    let grid = f.grid();
    let bytes = f.s.pack_for_save().unwrap();
    // Reopen with the library: identical view.
    let laser = f.laser.clone();
    let mut s = Session::open(4, &bytes).unwrap();
    s.set_clock(clock);
    s.load_materials(&f.folder.list(), None);
    assert!(s.activate_page(&laser));
    let v2: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    for id in &ids {
        let want = serde_json::to_value(id).unwrap();
        let a = view["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == want)
            .unwrap();
        let b = v2["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == want)
            .unwrap();
        assert_eq!(a["settings"], b["settings"]);
    }
    s.select_test_grid_row(Some(row));
    let g2: Value = serde_json::from_str(&s.test_grid_view_json()).unwrap();
    assert_eq!(g2["worked"], grid["worked"]);
    assert_eq!(g2["fields"], grid["fields"]);
    // Reopen without the library: the links read stored, numbers kept.
    let mut s = Session::open(5, &bytes).unwrap();
    s.load_materials(&[], None);
    assert!(s.activate_page(&laser));
    let v3: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    let want = serde_json::to_value(ids[1]).unwrap();
    let r = v3["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == want)
        .unwrap();
    assert_eq!(r["settings"]["state"], "stored");
    assert_eq!(
        r["settings"]["sets"][1]["run"]["power_pct"].as_f64(),
        Some(30.0)
    );
    assert_eq!(
        r["settings"]["sets"][0]["run"]["power_pct"].as_f64(),
        Some(35.0)
    );
}

#[test]
fn c32_a_later_session_undoes_only_the_project_never_a_record() {
    let (mut f, _, _) = grid_fx(&[]);
    let id = f.save_new("Pine");
    let bytes = f.s.pack_for_save().unwrap();
    let laser = f.laser.clone();
    let mut s = Session::open(6, &bytes).unwrap();
    s.set_clock(clock);
    s.load_materials(&f.folder.list(), None);
    assert!(s.activate_page(&laser));
    let mut undone = 0;
    for _ in 0..4 {
        if s.undo().changed {
            undone += 1;
        }
    }
    eprintln!(
        "later session undid {undone} steps (undo stack {})",
        s.undo_len()
    );
    let w: Vec<Value> = serde_json::from_str(&s.take_material_writes_json()).unwrap();
    assert!(
        w.iter().all(|w| w["op"] != "remove" && w["op"] != "write"),
        "no record is touched from another session: {w:?}"
    );
    assert!(f.folder.record(&id).is_some());
}

#[test]
fn c32_undo_after_the_record_changed_elsewhere_leaves_it_and_names_the_material() {
    let (mut f, _) = lib_fx(&[]);
    assert!(f.s.rename_material(BIRCH, "Birch new").unwrap());
    f.sync();
    // Another computer's sync changes the record after the step.
    let mut rec = f.folder.record(BIRCH).unwrap();
    rec["thickness_mm"] = json!(9.0);
    rec["modified"] = json!(rec["modified"].as_u64().unwrap() + 60_000);
    let text = serde_json::to_string_pretty(&rec).unwrap();
    f.folder.files.insert(format!("{BIRCH}.json"), text.clone());
    f.reload();
    let r = f.s.undo();
    let w = f.sync();
    assert!(
        w.iter().all(|w| w["id"] != BIRCH),
        "the record is left as it is: {w:?}"
    );
    assert_eq!(f.folder.files[&format!("{BIRCH}.json")], text);
    let notice = format!(
        "{} {} {}",
        r.notice,
        f.s.take_history_notice().unwrap_or_default(),
        f.s.take_materials_notice().unwrap_or_default()
    );
    assert!(
        notice.contains("Birch"),
        "the notice names the material: {notice:?}"
    );
}

#[test]
fn c31_the_slice_writes_only_the_host_list_and_never_an_output() {
    // Every write the library asks for is one of the four host operations of the record folder
    // and the two preference files; nothing else leaves the session.
    let (mut f, ids, _) = {
        let (d, laser, ids) = document(400.0, 300.0, &["cut"]);
        let mut f = open(&d, &laser, birch_folder(1));
        let row = f.s.press_add_test_grid().unwrap();
        (f, ids, row)
    };
    let id = f.save_new("Felt");
    assert!(f.s.use_material_on(&ids, &id));
    let r = f.random();
    let _ = f.s.edit_material_set(&id, "cut", "power", 61.0, &r);
    let _ = f.s.move_material(&id, 0);
    f.s.delete_material(&id).unwrap();
    let all: Vec<Value> = f.sync();
    for w in all {
        let op = w["op"].as_str().unwrap();
        assert!(
            matches!(op, "write" | "remove" | "order" | "grid_choice"),
            "{op}"
        );
    }
}

#[test]
fn c32_history_list_actions_restore_only_the_project_never_a_record() {
    let (mut f, _, _) = grid_fx(&[]);
    let id = f.save_new("Pine");
    let text = f.folder.files[&format!("{id}.json")].clone();
    // Go to the version before "Save as material" (the list is newest first: 0 is the save).
    let items: Value = serde_json::from_str(&f.s.history_items_json(false, 0, 10)).unwrap();
    assert_eq!(items[0]["name"], "Save as material");
    assert_eq!(items[1]["name"], "Mark cell");
    let _ = f.s.apply_history_item(false, 1);
    let w = f.sync();
    assert!(
        w.iter().all(|w| w["op"] != "remove" && w["op"] != "write"),
        "{w:?}"
    );
    assert_eq!(f.folder.files[&format!("{id}.json")], text);
    // Also in a later session.
    let bytes = f.s.pack_for_save().unwrap();
    let laser = f.laser.clone();
    let mut s = Session::open(6, &bytes).unwrap();
    s.load_materials(&f.folder.list(), None);
    assert!(s.activate_page(&laser));
    let items: Value = serde_json::from_str(&s.history_items_json(false, 0, 10)).unwrap();
    let at = items
        .as_array()
        .unwrap()
        .iter()
        .position(|i| i["name"] == "Add test grid")
        .unwrap();
    let _ = s.apply_history_item(false, at);
    let w: Vec<Value> = serde_json::from_str(&s.take_material_writes_json()).unwrap();
    assert!(
        w.iter().all(|w| w["op"] != "remove" && w["op"] != "write"),
        "{w:?}"
    );
}
