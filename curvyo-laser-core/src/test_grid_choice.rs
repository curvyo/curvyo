//! Reads and writes `test-grid-choice.json` of format 2: the recipe a maker last used per
//! machine (ADR 0058 §9, `specs/0112` criterion 2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::material_preferences::MAX_GRID_CHOICE_MACHINES;
use crate::record_envelope::is_machine_id;
use crate::record_envelope::pretty;
use crate::test_recipe::TestRecipe;
use crate::test_recipe_json::{RecipeJson, read_recipe, recipe_json};

/// The format this build reads and writes; a file of format 1 reads as empty (a preference,
/// nothing is lost).
pub const GRID_CHOICE_FORMAT_VERSION: u64 = 2;
/// The longest file that is read.
const MAX_CHOICE_BYTES: usize = 256 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChoiceJson {
    format_version: u64,
    by_machine: BTreeMap<String, RecipeJson>,
}

/// The recipes the file remembers per profile id; empty for a damaged, older or newer file or
/// one with more than 100 machines. A read never rewrites the file.
#[must_use]
pub fn read_test_grid_choice(text: &str) -> BTreeMap<String, TestRecipe> {
    if text.len() > MAX_CHOICE_BYTES {
        return BTreeMap::new();
    }
    let Some(choice) = serde_json::from_str::<ChoiceJson>(text)
        .ok()
        .filter(|c| c.format_version == GRID_CHOICE_FORMAT_VERSION)
        .filter(|c| c.by_machine.len() <= MAX_GRID_CHOICE_MACHINES)
    else {
        return BTreeMap::new();
    };
    let mut recipes = BTreeMap::new();
    for (id, json) in &choice.by_machine {
        let readable = json.id.is_none() && json.material.is_none() && is_machine_id(id);
        match read_recipe(json).filter(|_| readable) {
            Some(recipe) => recipes.insert(id.clone(), recipe),
            None => return BTreeMap::new(),
        };
    }
    recipes
}

/// The text of the file holding `recipes`.
#[must_use]
pub fn write_test_grid_choice(recipes: &BTreeMap<String, TestRecipe>) -> String {
    pretty(&ChoiceJson {
        format_version: GRID_CHOICE_FORMAT_VERSION,
        by_machine: recipes
            .iter()
            .map(|(id, recipe)| (id.clone(), recipe_json(recipe)))
            .collect(),
    })
}

/// Remembers `recipe` for the profile `id`; with 100 other machines remembered, the one with the
/// lowest id is forgotten first.
pub fn remember_test_grid(
    recipes: &mut BTreeMap<String, TestRecipe>,
    id: &str,
    recipe: TestRecipe,
) {
    if !recipes.contains_key(id) && recipes.len() >= MAX_GRID_CHOICE_MACHINES {
        let first = recipes.keys().next().cloned();
        if let Some(first) = first {
            recipes.remove(&first);
        }
    }
    recipes.insert(id.to_owned(), recipe);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_profile::FeedRate;
    use crate::material::SetKind;
    use crate::test_recipe::RecipeDefaults;

    const GOLDEN: &str = include_str!("../tests/fixtures/materials/test_grid_choice_v2.json");

    #[test]
    fn the_golden_reads_and_writes_back() {
        let recipes = read_test_grid_choice(GOLDEN);
        assert_eq!(recipes.len(), 2);
        assert_eq!(recipes["generic-laser"].kind(), SetKind::Engrave);
        assert_eq!(write_test_grid_choice(&recipes), GOLDEN);
    }

    #[test]
    fn format_1_damage_and_an_id_inside_read_as_empty() {
        let old = include_str!("../tests/fixtures/materials/test_grid_choice.json");
        assert!(read_test_grid_choice(old).is_empty());
        assert!(read_test_grid_choice("{").is_empty());
        let with_id = GOLDEN.replacen("\"kind\"", "\"id\": \"K7Q2M\",\n      \"kind\"", 1);
        assert!(read_test_grid_choice(&with_id).is_empty());
        let bad_machine = GOLDEN.replace("generic-laser", "not-a-machine");
        assert!(read_test_grid_choice(&bad_machine).is_empty());
    }

    #[test]
    fn the_hundred_and_first_machine_forgets_the_lowest_id() {
        let recipe = TestRecipe::new(&RecipeDefaults {
            max_feed: FeedRate::new(6000).unwrap(),
            set: None,
        });
        let mut recipes = BTreeMap::new();
        for n in 0..100_u8 {
            let id = crate::record_envelope::new_record_id([n; 16]);
            remember_test_grid(&mut recipes, &id, recipe.clone());
        }
        remember_test_grid(&mut recipes, "generic-laser", recipe);
        assert_eq!(recipes.len(), 100);
        assert!(recipes.contains_key("generic-laser"));
        assert!(!recipes.contains_key(&crate::record_envelope::new_record_id([0; 16])));
    }
}
