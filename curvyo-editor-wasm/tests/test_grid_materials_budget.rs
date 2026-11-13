//! `specs/0112-test-grid-and-materials` criterion 32 in thread CPU time (`adrs.md` "Goldens and
//! budgets"): placing a 12 x 12 grid within 50 ms, a bar edit regenerating it within 30 ms, the
//! Materials tab of 200 materials within 50 ms, loading 200 format-2 records within 200 ms, the
//! Operations view of a page with a 12 x 12 grid within 16 ms and deleting that grid with its rows
//! within 30 ms. Budgets are asserted in release builds (the `performance-budgets` job); a debug
//! build prints the times.

#![allow(clippy::unwrap_used, clippy::cast_possible_truncation)]

mod cpu_time;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use curvyo_document_core::{Document, DocumentSize, NewOutputPage, pack};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_laser_core::FeedRate;
use curvyo_laser_core::grids::{
    RecipeDefaults, RecipeField, Side, TestRecipe, write_test_grid_choice,
};
use curvyo_laser_core::library::{
    HistoryEntry, HistoryEvent, LaserSets, Material, Set, SetSource, write_record,
};
use curvyo_laser_core::settings::{
    Interval, MachineTie, Passes, PowerPercent, SetKind, SetNumbers, Settings, Thickness,
};
use curvyo_ui_core::FontBlock;

const PLACE_BUDGET: Duration = Duration::from_millis(50);
const EDIT_BUDGET: Duration = Duration::from_millis(30);
const VIEW_BUDGET: Duration = Duration::from_millis(50);
const LOAD_BUDGET: Duration = Duration::from_millis(200);
const OPERATIONS_BUDGET: Duration = Duration::from_millis(16);
const DELETE_BUDGET: Duration = Duration::from_millis(30);

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

/// The 12 x 12 Cut recipe the tool places.
fn recipe_12() -> TestRecipe {
    let max_feed = FeedRate::DEFAULT;
    TestRecipe::new(&RecipeDefaults {
        max_feed,
        set: None,
    })
    .with_field(RecipeField::Count(Side::Columns), "12", max_feed)
    .unwrap()
    .with_field(RecipeField::Count(Side::Rows), "12", max_feed)
    .unwrap()
    .with_field(RecipeField::Cell, "5", max_feed)
    .unwrap()
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
    // A press asks for the label face; the host hands it in.
    s.set_tool(Tool::TestGrid);
    let _ = s.test_grid_press(None);
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
    let mut choice = BTreeMap::new();
    choice.insert("generic-laser".to_owned(), recipe_12());
    s.load_test_grid_choice(&write_test_grid_choice(&choice));
    s.load_materials(files, None);
    s
}

/// A session with a placed 12 x 12 grid selected, the Select tool active.
fn with_grid() -> Session {
    let mut s = session(&[]);
    s.set_tool(Tool::TestGrid);
    s.test_grid_press(None).unwrap();
    s.set_tool(Tool::Select);
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

/// 200 format-2 records, each with two lasers of three sets and `history` entries.
fn records(history: usize) -> Vec<(String, String)> {
    let numbers = |kind| SetNumbers {
        settings: Settings {
            power: PowerPercent::from_tenths(600).unwrap(),
            speed: FeedRate::new(3000).unwrap(),
            passes: Passes::ONE,
        },
        interval: (kind == SetKind::Engrave).then_some(Interval::DEFAULT),
    };
    (0..200u32)
        .map(|n| {
            let id = format!("{n:08x}-0000-4000-8000-000000000000");
            let laser = |machine: &str, name: &str| LaserSets {
                machine: MachineTie {
                    id: machine.to_owned(),
                    name: name.to_owned(),
                },
                sets: [SetKind::Cut, SetKind::Score, SetKind::Engrave]
                    .into_iter()
                    .map(|kind| {
                        (
                            kind,
                            Set {
                                numbers: numbers(kind),
                                source: SetSource::Typed,
                            },
                        )
                    })
                    .collect(),
            };
            let material = Material {
                id: id.clone(),
                modified: 1_762_948_800_000,
                name: format!("Material {n}"),
                thickness: Thickness::from_mm(3.0).unwrap(),
                lasers: vec![
                    laser("generic-laser", "Generic laser"),
                    laser("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f", "Diode 20 W"),
                ],
                tests: Vec::new(),
                history: (0..history)
                    .map(|h| HistoryEntry {
                        id: format!("{n:08x}-{h:04x}-4000-8000-000000000000"),
                        date: 1_762_948_800_000,
                        machine: "generic-laser".to_owned(),
                        kind: SetKind::Cut,
                        event: HistoryEvent::Set {
                            numbers: numbers(SetKind::Cut),
                            source: SetSource::Typed,
                        },
                    })
                    .collect(),
            };
            (format!("{id}.json"), write_record(&material))
        })
        .collect()
}

#[test]
fn criterion_32_placing_a_12_by_12_grid() {
    let cost = cheapest(
        || {
            let mut s = session(&[]);
            s.set_tool(Tool::TestGrid);
            s
        },
        |s| {
            s.test_grid_press(None).unwrap();
            assert!(s.test_grid_view_json().contains("\"count\":12"));
        },
    );
    report("place 12 x 12", cost, PLACE_BUDGET);
}

#[test]
fn criterion_32_a_bar_edit_regenerates_a_12_by_12_grid() {
    let cost = cheapest(with_grid, |s| {
        assert_eq!(s.set_test_grid_field("columns_max", "90"), Ok(true));
    });
    report("bar edit 12 x 12", cost, EDIT_BUDGET);
}

#[test]
fn criterion_32_the_materials_tab_of_200_materials() {
    let files = records(10);
    let cost = cheapest(
        || session(&files),
        |s| {
            let json = s.materials_view_json();
            assert!(json.contains("Material 199"));
        },
    );
    report("materials tab of 200", cost, VIEW_BUDGET);
}

#[test]
fn adrs_loading_200_format_2_records() {
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

#[test]
fn adrs_the_operations_view_with_a_12_by_12_grid() {
    let cost = cheapest(with_grid, |s| {
        assert_eq!(s.operations_view().unwrap().rows.len(), 145);
    });
    report("operations view with 145 rows", cost, OPERATIONS_BUDGET);
}

#[test]
fn adrs_deleting_a_12_by_12_grid_with_its_rows() {
    let cost = cheapest(with_grid, |s| {
        s.delete_selected();
        assert_eq!(s.operations_view().unwrap().rows.len(), 0);
    });
    report("delete 12 x 12 with its rows", cost, DELETE_BUDGET);
}
