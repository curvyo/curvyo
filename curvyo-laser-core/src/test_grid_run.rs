//! What each leaf of a test grid row runs with and in what order (ADR 0054 §4, `specs/0028`
//! criteria 5 to 7, 10): labels with the label settings, cells with their own numbers.

use serde_json::{Value, json};

use crate::machine_profile::FeedRate;
use crate::material::{CellIndex, Passes, PowerPercent, SetKind, Settings};
use crate::test_grid::GridRecipe;
use crate::test_grid_layout::LabelTag;

/// The `x.curvyo.laser.grid_cell` tag of a cell item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellTag {
    /// Its index.
    pub cell: CellIndex,
    /// The power it was made with.
    pub power: PowerPercent,
    /// The speed it was made with.
    pub speed: FeedRate,
}

impl CellTag {
    /// The stored plain values.
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "column": self.cell.column,
            "row": self.cell.row,
            "power_pct": self.power.percent(),
            "speed_mm_min": self.speed.get(),
        })
    }

    /// The tag of stored plain values, `None` when they do not read.
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        let map = value.as_object()?;
        if map.len() != 4 {
            return None;
        }
        Some(Self {
            cell: CellIndex {
                column: u8::try_from(map.get("column")?.as_u64()?).ok()?,
                row: u8::try_from(map.get("row")?.as_u64()?).ok()?,
            },
            power: PowerPercent::from_percent(map.get("power_pct")?.as_f64()?)?,
            speed: FeedRate::new(map.get("speed_mm_min")?.as_u64()?)?,
        })
    }
}

/// The grid tag of one leaf whose nearest key is the grid row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridTag {
    /// A cell.
    Cell(CellTag),
    /// A label.
    Label(LabelTag),
    /// No readable tag: the leaf runs nothing.
    None,
}

impl GridTag {
    /// The tag of a leaf from its `grid_cell` and `grid_label` values; a cell tag wins.
    #[must_use]
    pub fn of(cell: Option<&Value>, label: Option<&Value>) -> Self {
        if let Some(tag) = cell.and_then(CellTag::from_value) {
            return Self::Cell(tag);
        }
        label
            .and_then(Value::as_str)
            .and_then(LabelTag::parse)
            .map_or(Self::None, Self::Label)
    }
}

/// What one leaf of a grid row runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridLeafRun {
    /// A label: the label kind with the label settings.
    Label {
        /// Engrave or Score.
        kind: SetKind,
        /// The label settings.
        settings: Settings,
    },
    /// A cell: the grid's kind with its own numbers, one pass.
    Cell {
        /// Its index (the run order).
        cell: CellIndex,
        /// The grid's kind.
        kind: SetKind,
        /// Its numbers.
        settings: Settings,
    },
    /// Not assigned: an untagged leaf, or any leaf of a row with no readable grid.
    Nothing,
}

/// What a leaf tagged `tag` runs in the grid `recipe` (`None`: the row's group is gone or its
/// recipe does not read, "no grid").
#[must_use]
pub fn grid_leaf_run(recipe: Option<&GridRecipe>, tag: GridTag) -> GridLeafRun {
    let Some(recipe) = recipe else {
        return GridLeafRun::Nothing;
    };
    match tag {
        GridTag::Cell(cell) => GridLeafRun::Cell {
            cell: cell.cell,
            kind: recipe.kind,
            settings: Settings {
                power: cell.power,
                speed: cell.speed,
                passes: Passes::ONE,
            },
        },
        GridTag::Label(_) => GridLeafRun::Label {
            kind: recipe.labels.kind,
            settings: recipe.labels.settings,
        },
        GridTag::None => GridLeafRun::Nothing,
    }
}

/// The run order of a grid row's leaves as positions in `runs`: every label first in the given
/// order, then the cells row by row from the top left; leaves that run nothing are left out.
#[must_use]
pub fn grid_run_order(runs: &[GridLeafRun]) -> Vec<usize> {
    let labels = (0..runs.len()).filter(|&at| matches!(runs[at], GridLeafRun::Label { .. }));
    let mut cells: Vec<(CellIndex, usize)> = runs
        .iter()
        .enumerate()
        .filter_map(|(at, run)| match run {
            GridLeafRun::Cell { cell, .. } => Some((*cell, at)),
            _ => None,
        })
        .collect();
    cells.sort_by_key(|&(cell, at)| (cell.row, cell.column, at));
    labels.chain(cells.into_iter().map(|(_, at)| at)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe() -> GridRecipe {
        GridRecipe::defaults(FeedRate::new(6000).unwrap())
    }

    fn cell(column: u8, row: u8) -> GridTag {
        let recipe = recipe();
        let index = CellIndex { column, row };
        GridTag::Cell(CellTag {
            cell: index,
            power: recipe.column_power(column),
            speed: recipe.row_speed(row),
        })
    }

    #[test]
    fn a_cell_runs_its_own_numbers_and_a_label_the_label_settings() {
        let recipe = recipe();
        let run = grid_leaf_run(Some(&recipe), cell(2, 1));
        let GridLeafRun::Cell { kind, settings, .. } = run else {
            panic!("a cell");
        };
        assert_eq!(kind, SetKind::Cut);
        assert_eq!(settings.power.to_string(), "60");
        assert_eq!((settings.speed.get(), settings.passes.get()), (1950, 1));
        assert_eq!(
            grid_leaf_run(Some(&recipe), GridTag::Label(LabelTag::Speed)),
            GridLeafRun::Label {
                kind: SetKind::Engrave,
                settings: recipe.labels.settings
            },
            "a Cut grid's labels are engraved"
        );
        assert_eq!(
            grid_leaf_run(Some(&recipe), GridTag::None),
            GridLeafRun::Nothing
        );
        assert_eq!(
            grid_leaf_run(None, cell(0, 0)),
            GridLeafRun::Nothing,
            "no grid"
        );
    }

    #[test]
    fn a_moved_cell_keeps_the_numbers_it_was_made_with() {
        let mut changed = recipe();
        changed.range.power_to = PowerPercent::from_tenths(500).unwrap();
        let run = grid_leaf_run(Some(&changed), cell(4, 0));
        let GridLeafRun::Cell { settings, .. } = run else {
            panic!("a cell");
        };
        assert_eq!(
            settings.power.to_string(),
            "100",
            "the tag's numbers, not the recipe's"
        );
    }

    #[test]
    fn labels_run_first_then_cells_row_by_row_from_the_top_left() {
        let recipe = recipe();
        let tags = [
            cell(1, 1),
            GridTag::Label(LabelTag::Power),
            cell(0, 1),
            GridTag::None,
            cell(1, 0),
            GridTag::Label(LabelTag::Unit),
            cell(0, 0),
        ];
        let runs: Vec<GridLeafRun> = tags
            .iter()
            .map(|&t| grid_leaf_run(Some(&recipe), t))
            .collect();
        assert_eq!(grid_run_order(&runs), [1, 5, 6, 4, 2, 0]);
    }

    #[test]
    fn tags_read_from_plain_values() {
        let GridTag::Cell(tag) = cell(3, 2) else {
            panic!("a cell");
        };
        assert_eq!(CellTag::from_value(&tag.to_value()), Some(tag));
        assert_eq!(GridTag::of(Some(&tag.to_value()), None), GridTag::Cell(tag));
        assert_eq!(
            GridTag::of(None, Some(&json!("unit"))),
            GridTag::Label(LabelTag::Unit)
        );
        assert_eq!(
            GridTag::of(Some(&json!({"column": 1})), Some(&json!("hm"))),
            GridTag::None
        );
        assert_eq!(
            CellTag::from_value(
                &json!({"column": 1, "row": 0, "power_pct": 0.0, "speed_mm_min": 3})
            ),
            None
        );
    }
}
