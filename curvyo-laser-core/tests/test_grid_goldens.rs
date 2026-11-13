//! The laser extension reads the format 46 goldens of `curvyo-document-core/tests/fixtures/`
//! (`specs/0112-test-grid-and-materials` criteria 12, 15; ADR 0057 §1, §2, §6): every good recipe
//! reads, every damaged one makes its group plain, every derived row's part reads (one that is
//! text leaves the row ordinary), and the kind buttons never reuse a derived row.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{Document, OperationSnapshot, unpack};
use curvyo_laser_core::grids::{GridPart, Recipe};
use curvyo_laser_core::operations::{OperationKind, last_of_kind};
use curvyo_laser_core::settings::{SettingsRead, read_options, read_settings};

fn golden(name: &str) -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    unpack(5, &std::fs::read(path).unwrap()).unwrap()
}

fn operations(d: &Document) -> Vec<OperationSnapshot> {
    d.operations(d.manufacturings()[0].id).unwrap()
}

/// The recipe of each grid group of the derived rows, in list order.
fn recipes(d: &Document) -> Vec<Option<Recipe>> {
    let mut groups = Vec::new();
    for op in operations(d) {
        if let Some(group) = d.test_grid_of_derived_row(op.id)
            && !groups.contains(&group)
        {
            groups.push(group);
        }
    }
    groups
        .into_iter()
        .map(|group| Recipe::from_value(&d.test_grid(group).unwrap().recipe))
        .collect()
}

#[test]
fn the_golden_grids_read_with_their_parts_settings_and_options() {
    let d = golden("test_grid_v46.curvyo");
    let recipes = recipes(&d);
    assert_eq!(recipes.len(), 2);
    let cut = recipes[0].as_ref().expect("the Cut grid reads");
    assert_eq!(cut.id.to_string(), "K7Q2M");
    assert_eq!(cut.material.as_ref().unwrap().name, "Birch 3 mm");
    let engrave = recipes[1].as_ref().expect("the Engrave grid reads");
    assert!(engrave.test.crosshatch());
    let ops = operations(&d);
    for op in &ops {
        let (grid, part) = GridPart::read(op.grid_part.as_ref().unwrap()).unwrap();
        assert_ne!(grid, "");
        let kind = if part == GridPart::Labels || op.name.ends_with("8PL4D") {
            OperationKind::Engrave
        } else {
            OperationKind::Cut
        };
        assert!(
            matches!(
                read_settings(op.settings.as_ref(), kind),
                SettingsRead::Stored(_)
            ),
            "{}",
            op.name
        );
        assert!(
            read_options(op.options.as_ref(), kind).is_ok(),
            "{}",
            op.name
        );
    }
    assert_eq!(
        GridPart::read(ops[7].grid_part.as_ref().unwrap())
            .unwrap()
            .1,
        GridPart::Cell { column: 1, row: 1 }
    );
    assert_eq!(
        last_of_kind(&ops, OperationKind::Cut),
        None,
        "derived rows are never reused"
    );
    assert_eq!(last_of_kind(&ops, OperationKind::Engrave), None);
}

#[test]
fn every_damaged_recipe_makes_its_group_plain_and_a_text_part_leaves_the_row_ordinary() {
    let d = golden("test_grid_damaged_v46.curvyo");
    let recipes = recipes(&d);
    assert_eq!(recipes.len(), 5);
    assert!(recipes[0].is_none(), "a bad id");
    assert!(recipes[1].is_none(), "both axes on power");
    assert!(recipes[2].is_none(), "a count of 13");
    assert!(
        recipes[3].is_some(),
        "a non-integer Lasered leaves the recipe readable"
    );
    assert!(recipes[4].is_some());
    let ops = operations(&d);
    let text = ops.iter().position(|op| op.name == "TXT01 labels").unwrap();
    assert_eq!(GridPart::read(ops[text].grid_part.as_ref().unwrap()), None);
    assert_eq!(
        last_of_kind(&ops, OperationKind::Engrave),
        Some(text),
        "the ordinary row is the only one a kind press may fill"
    );
}
