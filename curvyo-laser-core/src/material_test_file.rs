//! Reads and writes the tests of a material record of format 2 or 3 (ADR 0058 §2, §6; ADR 0069
//! §8).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::material::{CellIndex, PickKind, SetKind};
use crate::material_record::{MAX_NOTE_CHARS, Marks, PhotoKind, TestRecord};
use crate::material_record_file::{
    MachineJson, RecordFileError, damaged, machine_json, read_machine,
};
use crate::test_id::TestId;
use crate::test_recipe::PictureSource;
use crate::test_recipe_json::{RecipeJson, read_recipe, recipe_json};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestJson {
    id: String,
    created: u64,
    machine: MachineJson,
    recipe: RecipeJson,
    lasered: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    best: Option<CellJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    second_best: Option<CellJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fastest: Option<CellJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deepest: Option<CellJson>,
    note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    photo: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct CellJson {
    column: u64,
    row: u64,
}

/// The tests of a record of format `version`; a format-2 record holds no Image test, a format-3
/// record no picture test (`specs/0135` criterion 51).
pub(crate) fn read_tests(
    tests: Vec<TestJson>,
    version: u64,
) -> Result<Vec<TestRecord>, RecordFileError> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::with_capacity(tests.len());
    for test in tests {
        let id = TestId::parse(&test.id).ok_or_else(|| {
            damaged(format!(
                "Test id {} is not 5 characters of 0-9A-Z.",
                test.id
            ))
        })?;
        if !seen.insert(id) {
            return Err(damaged(format!("Test {id} is listed twice.")));
        }
        if test.recipe.id.is_some() || test.recipe.material.is_some() {
            return Err(damaged(format!("The recipe of test {id} is not valid.")));
        }
        let recipe = read_recipe(&test.recipe)
            .filter(|recipe| version > 2 || recipe.kind() != SetKind::Image)
            .filter(|recipe| version > 3 || recipe.picture() == PictureSource::Ramp)
            .ok_or_else(|| damaged(format!("The recipe of test {id} is not valid.")))?;
        let marks = read_marks(&test, id, &recipe, version)?;
        if test.note.chars().count() > MAX_NOTE_CHARS {
            return Err(damaged(format!(
                "The note of test {id} has more than 500 characters."
            )));
        }
        let photo = match test.photo.as_deref() {
            None => None,
            Some(text) => Some(
                PhotoKind::parse(text)
                    .ok_or_else(|| damaged(format!("The photo of test {id} is not jpg or png.")))?,
            ),
        };
        out.push(TestRecord {
            id,
            created: test.created,
            machine: read_machine(test.machine)?,
            recipe,
            lasered: test.lasered,
            marks,
            note: test.note,
            photo,
        });
    }
    Ok(out)
}

/// The marks of a test: each inside the recipe's counts; the three picks after Best exist from
/// format 5 only (ADR 0090 decision 5).
fn read_marks(
    test: &TestJson,
    id: TestId,
    recipe: &crate::test_recipe::TestRecipe,
    version: u64,
) -> Result<Marks, RecordFileError> {
    let mut marks = Marks::default();
    for (pick, cell) in [
        (PickKind::Best, test.best),
        (PickKind::SecondBest, test.second_best),
        (PickKind::Fastest, test.fastest),
        (PickKind::Deepest, test.deepest),
    ] {
        let Some(cell) = cell else { continue };
        if pick != PickKind::Best && version < 5 {
            return Err(damaged(format!(
                "A format-{version} record marks a {} cell.",
                pick.label().to_lowercase()
            )));
        }
        let index = |n: u64| u8::try_from(n).ok();
        match (index(cell.column), index(cell.row)) {
            (Some(column), Some(row)) if recipe.has_cell(column, row) => {
                marks.put(pick, Some(CellIndex { column, row }));
            }
            _ => {
                return Err(damaged(format!(
                    "The {} cell of test {id} is outside its grid.",
                    pick.label().to_lowercase()
                )));
            }
        }
    }
    Ok(marks)
}

pub(crate) fn test_json(test: &TestRecord) -> TestJson {
    TestJson {
        id: test.id.to_string(),
        created: test.created,
        machine: machine_json(&test.machine),
        recipe: recipe_json(&test.recipe),
        lasered: test.lasered,
        best: cell_json(test.marks.best),
        second_best: cell_json(test.marks.second_best),
        fastest: cell_json(test.marks.fastest),
        deepest: cell_json(test.marks.deepest),
        note: test.note.clone(),
        photo: test.photo.map(|p| p.as_str().to_owned()),
    }
}

fn cell_json(cell: Option<CellIndex>) -> Option<CellJson> {
    cell.map(|cell| CellJson {
        column: u64::from(cell.column),
        row: u64::from(cell.row),
    })
}
