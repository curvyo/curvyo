//! `specs/0028-material-test-library` criterion 30 in thread CPU time (`adrs.md` §8): making a
//! 12 x 12 grid with its 25 label texts within 50 ms, one field edit (a rebuild) within 30 ms,
//! the Materials view of 200 records within 50 ms, and loading 200 records with history within
//! 200 ms. Budgets are asserted in release builds (the `performance-budgets` job); a debug build
//! prints the times.

#![allow(clippy::unwrap_used, clippy::cast_possible_truncation)]

mod cpu_time;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use curvyo_document_core::{Document, DocumentSize, NewOutputPage, pack};
use curvyo_editor_wasm::Session;
use curvyo_laser_core::FeedRate;
use curvyo_laser_core::material_library::{RememberedGrid, write_grid_choice, write_material};
use curvyo_laser_core::settings::{
    HistoryEntry, HistoryEvent, MachineTie, Material, MaterialSet, SetKind, SetSource, Settings,
    Thickness,
};
use curvyo_laser_core::test_grids::GridRecipe;
use curvyo_ui_core::FontBlock;

const MAKE_BUDGET: Duration = Duration::from_millis(50);
const EDIT_BUDGET: Duration = Duration::from_millis(30);
const VIEW_BUDGET: Duration = Duration::from_millis(50);
const LOAD_BUDGET: Duration = Duration::from_millis(200);

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

/// A laser page of 400 x 300 mm on Generic laser, opened with the fonts loaded, the 12 x 12 grid
/// remembered and `files` in the library.
fn session(files: &[(String, String)]) -> Session {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mut s = Session::open(5, &pack(&d, "budget").unwrap()).unwrap();
    assert!(s.activate_page(&laser.id));
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
    let _ = s.press_add_test_grid();
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
    let mut recipe = GridRecipe::defaults(FeedRate::DEFAULT);
    recipe.range.columns = 12;
    recipe.range.rows = 12;
    let mut choice = BTreeMap::new();
    choice.insert(
        "generic-laser".to_owned(),
        RememberedGrid {
            range: recipe.range,
            labels: recipe.labels,
        },
    );
    s.load_test_grid_choice(&write_grid_choice(&choice));
    s.load_materials(files, None);
    s
}

/// The cheapest of five runs of `work` on fresh states made by `make`.
fn cheapest<S>(mut make: impl FnMut() -> S, mut work: impl FnMut(&mut S)) -> Duration {
    (0..5)
        .map(|_| {
            let mut s = make();
            cpu_time::cpu_time_of(|| work(&mut s)).1
        })
        .min()
        .unwrap()
}

fn report(what: &str, cost: Duration, budget: Duration) {
    println!("{what}: {cost:?} (budget {budget:?})");
    if TIMED {
        assert!(cost <= budget, "{what}: {cost:?} over {budget:?}");
    }
}

/// 200 records of Generic laser, each with three sets and `history` entries.
fn records(history: usize) -> Vec<(String, String)> {
    (0..200u32)
        .map(|n| {
            let id = format!("{n:08x}-0000-4000-8000-000000000000");
            let set = Settings {
                power: curvyo_laser_core::settings::PowerPercent::from_tenths(600).unwrap(),
                speed: FeedRate::new(3000).unwrap(),
                passes: curvyo_laser_core::settings::Passes::ONE,
            };
            let mut sets = BTreeMap::new();
            for kind in [SetKind::Cut, SetKind::Score, SetKind::Engrave] {
                sets.insert(
                    kind,
                    MaterialSet {
                        settings: set,
                        source: SetSource::Edited,
                    },
                );
            }
            let material = Material {
                id: id.clone(),
                modified: 1_762_948_800_000,
                name: format!("Material {n}"),
                thickness: Thickness::from_mm(3.0).unwrap(),
                machine: MachineTie {
                    id: "generic-laser".to_owned(),
                    name: "Generic laser".to_owned(),
                },
                sets,
                history: (0..history)
                    .map(|h| HistoryEntry {
                        id: format!("{n:08x}-{h:04x}-4000-8000-000000000000"),
                        date: 1_762_948_800_000,
                        kind: SetKind::Cut,
                        event: HistoryEvent::Edited { set },
                    })
                    .collect(),
            };
            (format!("{id}.json"), write_material(&material))
        })
        .collect()
}

#[test]
fn criterion_30_making_a_12_by_12_grid_with_its_25_label_texts() {
    let cost = cheapest(
        || session(&[]),
        |s| {
            s.press_add_test_grid().unwrap();
            assert!(s.test_grid_view_json().contains("\"columns\":12"));
        },
    );
    report("make 12 x 12", cost, MAKE_BUDGET);
}

#[test]
fn criterion_30_one_field_edit_rebuilds_a_12_by_12_grid() {
    let cost = cheapest(
        || {
            let mut s = session(&[]);
            s.press_add_test_grid().unwrap();
            s
        },
        |s| {
            assert_eq!(s.set_test_grid_field("power_to", "90"), Ok(true));
            assert!(s.test_grid_view_json().contains("\"power_to_pct\":90.0"));
        },
    );
    report("field edit 12 x 12", cost, EDIT_BUDGET);
}

#[test]
fn criterion_30_the_materials_view_of_200_records() {
    let files = records(10);
    let cost = cheapest(
        || session(&files),
        |s| {
            let json = s.materials_view_json();
            assert!(json.contains("Material 199"));
        },
    );
    report("materials view of 200", cost, VIEW_BUDGET);
}

#[test]
fn adrs_8_loading_200_records_with_history() {
    let files = records(100);
    let cost = cheapest(
        || session(&[]),
        |s| {
            s.load_materials(&files, None);
            assert_eq!(s.materials().reads().len(), 200);
        },
    );
    report("load of 200 records", cost, LOAD_BUDGET);
}
