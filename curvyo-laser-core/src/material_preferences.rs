//! Reads and writes the material library's preference files (ADR 0055 §5, §6):
//! `material-order.json` and `test-grid-choice.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::material::SetKind;
use crate::material_file::{grid_json, is_machine_id};
use crate::record_envelope::pretty;
use crate::test_grid::{GridLabels, GridRange, GridRecipe};

/// The most machines `test-grid-choice.json` remembers a grid for (ADR 0055 §6).
pub const MAX_GRID_CHOICE_MACHINES: usize = 100;
/// The format of `material-order.json` and of `test-grid-choice.json`.
const PREFERENCE_FORMAT_VERSION: u64 = 1;
/// The longest preference file that is read.
const MAX_PREFERENCE_BYTES: usize = 256 * 1024;

/// The grid fields and label settings a maker last used for one machine (spec criterion 2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RememberedGrid {
    /// The cells.
    pub range: GridRange,
    /// The labels.
    pub labels: GridLabels,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderJson {
    format_version: u64,
    order: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChoiceJson {
    format_version: u64,
    by_machine: BTreeMap<String, RememberedJson>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RememberedJson {
    columns: u64,
    rows: u64,
    power_from_pct: f64,
    power_to_pct: f64,
    speed_from_mm_min: u64,
    speed_to_mm_min: u64,
    cell_mm: f64,
    gap_mm: f64,
    labels: LabelsJson,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LabelsJson {
    on: bool,
    kind: String,
    height_mm: f64,
    power_pct: f64,
    speed_mm_min: u64,
    passes: u64,
}

/// The ids `material-order.json` lists, in order; empty for a damaged or newer file (ADR 0055
/// §5). Ids no record has are kept, so they survive until the next order write.
#[must_use]
pub fn read_order(text: &str) -> Vec<String> {
    if text.len() > MAX_PREFERENCE_BYTES {
        return Vec::new();
    }
    serde_json::from_str::<OrderJson>(text)
        .ok()
        .filter(|order| order.format_version == PREFERENCE_FORMAT_VERSION)
        .map(|order| order.order)
        .unwrap_or_default()
}

/// The text of `material-order.json` listing `ids`.
#[must_use]
pub fn write_order(ids: &[String]) -> String {
    pretty(&OrderJson {
        format_version: PREFERENCE_FORMAT_VERSION,
        order: ids.to_vec(),
    })
}

/// The grids `test-grid-choice.json` remembers per profile id; empty for a damaged or newer file
/// or one with more than 100 machines (ADR 0055 §6). A read never rewrites the file.
#[must_use]
pub fn read_grid_choice(text: &str) -> BTreeMap<String, RememberedGrid> {
    if text.len() > MAX_PREFERENCE_BYTES {
        return BTreeMap::new();
    }
    let Some(choice) = serde_json::from_str::<ChoiceJson>(text)
        .ok()
        .filter(|choice| choice.format_version == PREFERENCE_FORMAT_VERSION)
        .filter(|choice| choice.by_machine.len() <= MAX_GRID_CHOICE_MACHINES)
    else {
        return BTreeMap::new();
    };
    let mut grids = BTreeMap::new();
    for (id, remembered) in choice.by_machine {
        match (is_machine_id(&id), read_remembered(&remembered)) {
            (true, Some(grid)) => grids.insert(id, grid),
            _ => return BTreeMap::new(),
        };
    }
    grids
}

/// The recipe rule of ADR 0054 §4 reads the remembered fields, with a kind it then drops.
fn read_remembered(remembered: &RememberedJson) -> Option<RememberedGrid> {
    let Ok(Value::Object(mut map)) = serde_json::to_value(remembered) else {
        return None;
    };
    map.insert("kind".to_owned(), json!(SetKind::Cut.as_str()));
    let recipe = GridRecipe::from_value(&Value::Object(map))?;
    Some(RememberedGrid {
        range: recipe.range,
        labels: recipe.labels,
    })
}

/// The text of `test-grid-choice.json` holding `grids`.
#[must_use]
pub fn write_grid_choice(grids: &BTreeMap<String, RememberedGrid>) -> String {
    let by_machine = grids
        .iter()
        .map(|(id, grid)| {
            let range = grid_json(&grid.range);
            let labels = &grid.labels;
            let remembered = RememberedJson {
                columns: range.columns,
                rows: range.rows,
                power_from_pct: range.power_from_pct,
                power_to_pct: range.power_to_pct,
                speed_from_mm_min: range.speed_from_mm_min,
                speed_to_mm_min: range.speed_to_mm_min,
                cell_mm: range.cell_mm,
                gap_mm: range.gap_mm,
                labels: LabelsJson {
                    on: labels.on,
                    kind: labels.kind.as_str().to_owned(),
                    height_mm: labels.height.as_mm(),
                    power_pct: labels.settings.power.percent(),
                    speed_mm_min: u64::from(labels.settings.speed),
                    passes: u64::from(labels.settings.passes.get()),
                },
            };
            (id.clone(), remembered)
        })
        .collect();
    pretty(&ChoiceJson {
        format_version: PREFERENCE_FORMAT_VERSION,
        by_machine,
    })
}

/// Remembers `grid` for the profile `id`; with 100 other machines remembered, the one with the
/// lowest id is forgotten first (Proposal: the file keeps no dates).
pub fn remember_grid(grids: &mut BTreeMap<String, RememberedGrid>, id: &str, grid: RememberedGrid) {
    if !grids.contains_key(id) && grids.len() >= MAX_GRID_CHOICE_MACHINES {
        let first = grids.keys().next().cloned();
        if let Some(first) = first {
            grids.remove(&first);
        }
    }
    grids.insert(id.to_owned(), grid);
}
#[cfg(test)]
#[path = "material_preferences_tests.rs"]
mod tests;
