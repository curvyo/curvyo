//! Reads and writes the tests of a format-2 material record (ADR 0058 §2, §6).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::material::CellIndex;
use crate::material_record::{MAX_NOTE_CHARS, PhotoKind, TestRecord};
use crate::material_record_file::{
    MachineJson, RecordFileError, damaged, machine_json, read_machine,
};
use crate::test_id::TestId;
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
    note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    photo: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CellJson {
    column: u64,
    row: u64,
}

pub(crate) fn read_tests(tests: Vec<TestJson>) -> Result<Vec<TestRecord>, RecordFileError> {
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
            .ok_or_else(|| damaged(format!("The recipe of test {id} is not valid.")))?;
        let best = match test.best {
            None => None,
            Some(cell) => {
                let index = |n: u64| u8::try_from(n).ok();
                match (index(cell.column), index(cell.row)) {
                    (Some(column), Some(row)) if recipe.has_cell(column, row) => {
                        Some(CellIndex { column, row })
                    }
                    _ => {
                        return Err(damaged(format!(
                            "The best cell of test {id} is outside its grid."
                        )));
                    }
                }
            }
        };
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
            best,
            note: test.note,
            photo,
        });
    }
    Ok(out)
}

pub(crate) fn test_json(test: &TestRecord) -> TestJson {
    TestJson {
        id: test.id.to_string(),
        created: test.created,
        machine: machine_json(&test.machine),
        recipe: recipe_json(&test.recipe),
        lasered: test.lasered,
        best: test.best.map(|cell| CellJson {
            column: u64::from(cell.column),
            row: u64::from(cell.row),
        }),
        note: test.note.clone(),
        photo: test.photo.map(|p| p.as_str().to_owned()),
    }
}
