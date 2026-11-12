//! The laser extension reads the format 43 goldens of `curvyo-document-core/tests/fixtures/`
//! (`specs/0028-material-test-library` criteria 26, 28; ADR 0054 §1, §2, §4, §6): every operation's
//! settings state, every damage, the grid's recipe, mark and tags, the run of a grid row, and the
//! orphan row's "no grid".

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{Document, OperationSnapshot, unpack};
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{
    Field, Role, SettingsDamage, SettingsRead, SettingsState, StoredReason, read_settings, resolve,
};
use curvyo_laser_core::test_grids::{
    GridLeafRun, GridRecipe, GridTag, LabelTag, grid_leaf_run, grid_run_order, read_mark,
};

fn golden(name: &str) -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    unpack(5, &std::fs::read(path).unwrap()).unwrap()
}

fn operations(d: &Document) -> Vec<OperationSnapshot> {
    d.operations(d.manufacturings()[0].id).unwrap()
}

fn read(op: &OperationSnapshot) -> (OperationKind, SettingsRead) {
    let kind = KindRead::of(op.kind.as_deref()).known().unwrap();
    (kind, read_settings(op.settings.as_ref(), kind))
}

#[test]
fn the_settings_golden_resolves_stored_own_and_no_settings_without_the_library() {
    let d = golden("laser_settings_v43.curvyo");
    let ops = operations(&d);
    let state = |at: usize| {
        let (kind, settings) = read(&ops[at]);
        resolve(&settings, kind, &[], "generic-laser")
    };
    let SettingsState::Stored {
        name,
        reason,
        material,
        run,
        overridden,
    } = state(0)
    else {
        panic!("the material is not on this computer");
    };
    assert_eq!(
        (name.as_str(), reason),
        ("Birch 3 mm", StoredReason::Missing)
    );
    assert_eq!(material.main.power.to_string(), "30");
    assert_eq!(run.main.power.to_string(), "35", "the override runs");
    assert_eq!(
        run.outline.unwrap().speed.get(),
        9000,
        "the stored outline set"
    );
    assert_eq!(overridden, [(Role::Main, Field::Power)]);
    assert!(state(0).differs_text().is_some());
    let cut = state(1);
    assert_eq!(cut.run().unwrap().main.passes.get(), 2);
    assert_eq!(cut.differs_text(), None);
    assert!(matches!(state(2), SettingsState::Own(_)));
    assert_eq!(state(3), SettingsState::NoSettings);
}

#[test]
fn every_damaged_register_reads_as_damaged_settings_with_its_reason() {
    use SettingsDamage::{BadLink, BadOverride, BadSet, NotAMap, OutlineOnKind};
    let d = golden("laser_settings_damaged_v43.curvyo");
    let damages: Vec<SettingsDamage> = operations(&d)
        .iter()
        .map(|op| match read(op).1 {
            SettingsRead::Damaged(damage) => damage,
            other => panic!("{}: {other:?}", op.name),
        })
        .collect();
    assert_eq!(
        damages,
        [
            NotAMap,
            BadSet,
            BadSet,
            BadLink,
            BadLink,
            OutlineOnKind,
            BadOverride,
            BadOverride,
            BadOverride
        ]
    );
}

#[test]
fn the_golden_grid_reads_its_recipe_mark_and_tags_and_runs_labels_first() {
    let d = golden("laser_settings_v43.curvyo");
    let row = operations(&d).pop().unwrap();
    assert!(row.grid);
    let grid = d.test_grid(d.test_grid_of_row(row.id).unwrap()).unwrap();
    let recipe = GridRecipe::from_value(&grid.recipe).unwrap();
    assert_eq!((recipe.range.columns, recipe.range.rows), (3, 2));
    assert_eq!(recipe.column_power(1).to_string(), "60");
    let mark = read_mark(grid.worked.as_ref().unwrap(), &recipe).unwrap();
    assert_eq!((mark.column, mark.row), (1, 0));
    let mut runs = Vec::new();
    for item in grid.cells.iter().chain(&grid.labels) {
        let (cell, label) = d.grid_item_tags(item.id);
        runs.push(grid_leaf_run(
            Some(&recipe),
            GridTag::of(cell.as_ref(), label.as_ref()),
        ));
    }
    let GridLeafRun::Cell { settings, .. } = runs[4] else {
        panic!("a cell");
    };
    assert_eq!(
        (settings.power.to_string(), settings.speed.get()),
        ("60".to_owned(), 6000)
    );
    let order = grid_run_order(&runs);
    assert_eq!(order.len(), 12);
    assert!(
        order[..6]
            .iter()
            .all(|&at| matches!(runs[at], GridLeafRun::Label { .. })),
        "labels before cells"
    );
    assert_eq!(
        GridTag::of(None, d.grid_item_tags(grid.labels[0].id).1.as_ref()),
        GridTag::Label(LabelTag::Unit)
    );
}

#[test]
fn the_hand_edited_grid_misses_a_label_and_the_orphan_row_has_no_grid() {
    let d = golden("test_grid_hand_edited_v43.curvyo");
    let row = operations(&d).remove(0);
    let grid = d.test_grid(d.test_grid_of_row(row.id).unwrap()).unwrap();
    let recipe = GridRecipe::from_value(&grid.recipe).unwrap();
    assert_eq!(
        grid.labels.len() + 1,
        1 + usize::from(recipe.range.columns) + usize::from(recipe.range.rows),
        "a deleted label"
    );
    let orphan = golden("test_grid_orphan_row_v43.curvyo");
    let row = operations(&orphan).remove(0);
    assert_eq!(orphan.test_grid_of_row(row.id), None);
    assert_eq!(
        grid_leaf_run(None, GridTag::None),
        GridLeafRun::Nothing,
        "no grid runs nothing"
    );
}
