//! The contract of the JSON views of `specs/0028-material-test-library` (`adrs.md` §6): each
//! named scenario is built through the session and its JSON value compared with the shared
//! fixture the TypeScript decoders read, in `tests/fixtures/materials_view/`, `test_grid_view/`,
//! `operations_view/settings_*.json` and `material_writes/`. A drift fails. The fixtures are
//! written only when `CURVYO_UPDATE_GOLDEN` is set, as the other goldens of this crate.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use curvyo_document_core::{
    Document, DocumentSize, NewOperation, NewOutputPage, NodeId, SettingsStep, SettingsWrite,
    Timestamp, pack,
};
use curvyo_editor_wasm::Session;
use curvyo_ui_core::FontBlock;

const LASER: &str = "curvyo.laser";
/// The id of every golden record of `curvyo-laser-core/tests/fixtures/materials/`.
const GOLDEN_ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
/// The machine those goldens name.
const GOLDEN_MACHINE: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";
const CARDBOARD: &str = "11111111-1111-4111-8111-111111111111";
const BIRCH: &str = "22222222-2222-4222-8222-222222222222";
const DAMAGED: &str = "33333333-3333-4333-8333-333333333333";
const NEWER: &str = "44444444-4444-4444-8444-444444444444";
const OTHER: &str = "55555555-5555-4555-8555-555555555555";
const GONE: &str = "66666666-6666-4666-8666-666666666666";

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn clock() -> Timestamp {
    Timestamp::from_secs(1_762_948_800)
}

/// A golden record of laser-core with its id and machine replaced.
fn record(golden: &str, id: &str, machine: &str) -> (String, String) {
    let path = crate_dir()
        .join("../curvyo-laser-core/tests/fixtures/materials")
        .join(golden);
    let text = std::fs::read_to_string(path)
        .unwrap()
        .replace(GOLDEN_ID, id)
        .replace(GOLDEN_MACHINE, machine)
        .replace("\"generic-laser\"", &format!("\"{machine}\""));
    (format!("{id}.json"), text)
}

/// The records of the scenarios: Cardboard (Cut set) and Birch (three sets) of Generic laser, a
/// damaged and a newer record of it, and a material of another machine.
fn library() -> Vec<(String, String)> {
    vec![
        record("generic_laser.json", CARDBOARD, "generic-laser"),
        birch_record(),
        record("unknown_key.json", DAMAGED, "generic-laser"),
        record("newer_format.json", NEWER, "generic-laser"),
        record("generic_laser.json", OTHER, GOLDEN_MACHINE),
    ]
}

/// Birch: the three-set golden, tied to Generic laser.
fn birch_record() -> (String, String) {
    let (name, text) = record("full.json", BIRCH, "generic-laser");
    (name, text.replace("K40 in the garage", "Generic laser"))
}

/// A document with one laser page of 400 x 300 mm and operations of `kinds`; with `damaged`
/// the last one holds a damaged `op_settings`.
fn document(kinds: &[&str], damaged: bool) -> (Document, String, Vec<NodeId>) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let m = laser.kind.manufacturing().unwrap();
    let ids: Vec<NodeId> = kinds
        .iter()
        .enumerate()
        .map(|(index, kind)| {
            let name = format!("{} {}", kind, index + 1);
            d.add_operation(m, &NewOperation { kind, name: &name }, index, &[])
                .unwrap()
        })
        .collect();
    if damaged {
        let mut map = serde_json::Map::new();
        map.insert("main".to_owned(), serde_json::json!(5));
        let write = SettingsWrite {
            operation: *ids.last().unwrap(),
            value: Some(map),
        };
        d.write_operation_settings(SettingsStep::EditSettings, &[write])
            .unwrap();
    }
    (d, laser.id, ids)
}

/// The session of `d` on its laser page with `files` loaded.
fn session_of(d: &Document, laser: &str, files: &[(String, String)]) -> Session {
    let bytes = pack(d, "material test library views").unwrap();
    let mut s = Session::open(3, &bytes).unwrap();
    s.set_clock(clock);
    s.load_materials(files, None);
    assert!(s.activate_page(laser));
    s
}

fn fonts_dir() -> PathBuf {
    crate_dir().join("../frontend/public/fonts")
}

/// The included fonts listed and the wanted ones handed in, as the host does.
fn with_fonts(s: &mut Session) {
    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|face| {
            (
                face["file"].as_str().unwrap().to_owned(),
                face["family"].as_str().unwrap().to_owned(),
                face["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect();
    s.set_font_list(FontBlock::Included, faces, Vec::new());
    // The first press asks for the included sans while it is not loaded yet.
    assert!(s.press_add_test_grid().is_err());
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
}

/// A grid session: a laser page with the default grid added.
fn grid_session() -> Session {
    let (d, laser, _) = document(&[], false);
    let mut s = session_of(&d, &laser, &library());
    with_fonts(&mut s);
    s.press_add_test_grid().unwrap();
    s
}

fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap()
}

fn materials_scenarios() -> Vec<(&'static str, serde_json::Value)> {
    let (d, laser, ids) = document(&["cut"], false);
    let empty = session_of(&d, &laser, &[]);
    let mut listed = session_of(&d, &laser, &library());
    assert!(listed.use_material_on(&ids, CARDBOARD));
    let mut selected = session_of(&d, &laser, &library());
    selected.select_material(Some(BIRCH));
    vec![
        ("empty", json(&empty.materials_view_json())),
        (
            "damaged_newer_and_used",
            json(&listed.materials_view_json()),
        ),
        ("selected_fields", json(&selected.materials_view_json())),
    ]
}

fn test_grid_scenarios() -> Vec<(&'static str, serde_json::Value)> {
    let unmarked = grid_session();
    let mut marked = grid_session();
    assert!(marked.press_test_cell(2, 1));
    let mut too_wide = grid_session();
    let _ = too_wide.set_test_grid_field("cell", "50");
    assert!(too_wide.set_test_grid_field("columns", "12").is_err());
    let mut invalid = grid_session();
    assert!(invalid.set_test_grid_field("columns", "13").is_err());
    vec![
        ("default_unmarked", json(&unmarked.test_grid_view_json())),
        ("marked", json(&marked.test_grid_view_json())),
        ("does_not_fit", json(&too_wide.test_grid_view_json())),
        ("invalid_value", json(&invalid.test_grid_view_json())),
    ]
}

/// The `settings` and `pick` parts of the Operations tab: the rest is pinned by
/// `operations_view_fixtures.rs`.
fn settings_part(s: &Session) -> serde_json::Value {
    let view = json(&s.operations_view_json());
    let rows: Vec<serde_json::Value> = view["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            serde_json::json!({
                "name": row["name"],
                "kind": row["kind"],
                "settings": row["settings"],
                "grid": row["grid"],
            })
        })
        .collect();
    serde_json::json!({ "pick": view["pick"], "rows": rows })
}

fn settings_scenarios() -> Vec<(&'static str, serde_json::Value)> {
    // Live: a Cut on Cardboard, an Engrave + Outline on Birch with a typed hatch power.
    let (d, laser, ids) = document(&["cut", "engrave_outline", "score"], false);
    let mut live = session_of(&d, &laser, &library());
    assert!(live.use_material_on(&ids[..1], CARDBOARD));
    assert!(live.use_material_on(&ids[1..2], BIRCH));
    assert_eq!(
        live.edit_operation_setting(ids[1], "main", "power", 37.5),
        Ok(true)
    );
    // Stored, per reason, one with a typed number: each linked while its record loaded, then
    // the library changes under it (gone, damaged, newer, another machine, no Score set).
    let kinds = ["cut", "cut", "cut", "cut", "score"];
    let (d2, laser2, ids2) = document(&kinds, false);
    let linked = [GONE, DAMAGED, NEWER, OTHER, BIRCH];
    let mut before: Vec<(String, String)> = linked
        .iter()
        .take(4)
        .map(|id| record("generic_laser.json", id, "generic-laser"))
        .collect();
    before.push(birch_record());
    let mut stored = session_of(&d2, &laser2, &before);
    for (id, material) in ids2.iter().zip(linked) {
        assert!(stored.use_material_on(&[*id], material), "{material}");
    }
    assert_eq!(
        stored.edit_operation_setting(ids2[0], "main", "speed", 400.0),
        Ok(true)
    );
    let birch = birch_record();
    let after = vec![
        record("unknown_key.json", DAMAGED, "generic-laser"),
        record("newer_format.json", NEWER, "generic-laser"),
        record("generic_laser.json", OTHER, GOLDEN_MACHINE),
        (birch.0, without_score(&birch.1)),
    ];
    stored.load_materials(&after, None);
    // Own, No settings, Damaged settings.
    let (d3, laser3, ids3) = document(&["cut", "engrave", "score"], true);
    let mut own = session_of(&d3, &laser3, &library());
    assert_eq!(
        own.edit_operation_setting(ids3[0], "main", "power", 40.0),
        Ok(false)
    );
    assert_eq!(
        own.edit_operation_setting(ids3[0], "main", "speed", 900.0),
        Ok(true)
    );
    vec![
        ("settings_live", settings_part(&live)),
        ("settings_stored", settings_part(&stored)),
        ("settings_own_none_damaged", settings_part(&own)),
    ]
}

/// Birch's record text without its Score set (a set removal from another computer).
fn without_score(text: &str) -> String {
    let mut value: serde_json::Value = serde_json::from_str(text).unwrap();
    value["sets"].as_object_mut().unwrap().remove("score");
    format!("{}\n", serde_json::to_string_pretty(&value).unwrap())
}

fn writes_scenarios() -> Vec<(&'static str, serde_json::Value)> {
    let mut s = grid_session();
    let _ = s.take_material_writes_json();
    assert!(s.press_test_cell(1, 1));
    let request = curvyo_editor_wasm::SaveAsMaterial {
        existing: None,
        name: "Birch 3 mm".to_owned(),
        thickness_mm: 3.0,
        passes: 1.0,
    };
    s.save_as_material(&request, &[9; 32]).unwrap();
    let saved = json(&s.take_material_writes_json());
    s.undo();
    let undone = json(&s.take_material_writes_json());
    vec![
        ("save_as_material", saved),
        ("undo_save_as_material", undone),
    ]
}

fn check(dir: &Path, name: &str, value: &serde_json::Value) {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!("{name}.json"));
    let text = format!("{}\n", serde_json::to_string_pretty(value).unwrap());
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let want: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{} exists (CURVYO_UPDATE_GOLDEN=1)", path.display())),
    )
    .unwrap();
    assert_eq!(value, &want, "{name} drifted from {}", path.display());
}

#[test]
fn materials_view_matches_its_fixtures() {
    let dir = crate_dir().join("tests/fixtures/materials_view");
    for (name, value) in materials_scenarios() {
        check(&dir, name, &value);
    }
}

#[test]
fn test_grid_view_matches_its_fixtures() {
    let dir = crate_dir().join("tests/fixtures/test_grid_view");
    for (name, value) in test_grid_scenarios() {
        check(&dir, name, &value);
    }
}

#[test]
fn operations_view_settings_match_their_fixtures() {
    let dir = crate_dir().join("tests/fixtures/operations_view");
    for (name, value) in settings_scenarios() {
        check(&dir, name, &value);
    }
}

#[test]
fn material_writes_match_their_fixtures() {
    let dir = crate_dir().join("tests/fixtures/material_writes");
    for (name, value) in writes_scenarios() {
        check(&dir, name, &value);
    }
}

#[test]
fn the_named_cases_of_adrs_6_are_all_present() {
    let materials = materials_scenarios();
    let rows = &materials[1].1["rows"];
    let states: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["state"].as_str().unwrap())
        .collect();
    assert!(
        states.contains(&"damaged") && states.contains(&"newer"),
        "{states:?}"
    );
    let stored = settings_scenarios();
    let reasons: Vec<String> = stored[1].1["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["settings"]["reason"].as_str().map(str::to_owned))
        .collect();
    assert!(
        reasons.iter().any(|r| r.starts_with("Material not found")),
        "{reasons:?}"
    );
    assert!(
        reasons.iter().any(|r| r.contains("no Score set")),
        "{reasons:?}"
    );
    let live = &stored[0].1["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == "engrave_outline")
        .unwrap()["settings"];
    assert_eq!(live["state"], "live");
    assert_eq!(live["reset"], true);
    let grids = test_grid_scenarios();
    assert!(grids[0].1["worked"].is_null() && !grids[1].1["worked"].is_null());
    assert!(
        grids[2].1["invalid"]["chip"]
            .as_str()
            .unwrap()
            .starts_with("Grid is ")
    );
}
