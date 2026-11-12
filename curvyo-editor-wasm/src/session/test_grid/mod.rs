//! The test grids of the active laser page in a session (`specs/0028-material-test-library`
//! criteria 1 to 14; ADR 0054 §4, §5): which grid the Grid block shows, its reading and the
//! remembered values per machine. The rules are `curvyo-laser-core`'s; each write is one step.

mod build;
mod commands;
#[cfg(test)]
pub(in crate::session) mod tests;
mod view_json;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};

use curvyo_document_core::{NodeId, TestGridSnapshot};
use curvyo_laser_core::material_library::{RememberedGrid, read_grid_choice};
use curvyo_laser_core::settings::CellIndex;
use curvyo_laser_core::test_grids::{GridRecipe, read_mark};

use super::Session;

pub use view_json::TestGridView;

/// A typed value the grid did not take: kept in its field with the chip (criterion 2).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Invalid {
    group: NodeId,
    field: String,
    typed: String,
    chip: String,
}

/// What the session keeps for test grids.
#[derive(Debug, Default)]
pub(in crate::session) struct TestGridState {
    /// The remembered grid values per profile id (`test-grid-choice.json`, this computer).
    choice: BTreeMap<String, RememberedGrid>,
    /// The grid row pressed in the list.
    row: Option<NodeId>,
    /// The field value the grid did not take, with its chip.
    invalid: Option<Invalid>,
    /// The refusal of the last Add test grid, shown under the button.
    notice: Option<String>,
    /// Label widths in mm by face hash, height and text: each distinct text is shaped once.
    widths: RefCell<HashMap<(String, u64, String), f64>>,
    /// The grid the canvas selection held at the last panel read (UX notes §6.1, M3).
    seen: Cell<Option<NodeId>>,
}

/// A grid as read: its stored data, its recipe and mark when they read.
pub(in crate::session) struct GridRead {
    pub(in crate::session) snapshot: TestGridSnapshot,
    pub(in crate::session) recipe: Option<GridRecipe>,
    pub(in crate::session) mark: Option<CellIndex>,
}

impl Session {
    /// Reads the kept `test-grid-choice.json` (criterion 2); a damaged text reads as empty.
    pub fn load_test_grid_choice(&mut self, text: &str) {
        self.test_grids.choice = read_grid_choice(text);
    }

    /// The test grid whose top group is `group`.
    pub(in crate::session) fn read_test_grid(&self, group: NodeId) -> Option<GridRead> {
        let snapshot = self.document.test_grid(group)?;
        let recipe = GridRecipe::from_value(&snapshot.recipe);
        let mark = recipe.as_ref().and_then(|recipe| {
            snapshot
                .worked
                .as_ref()
                .and_then(|value| read_mark(value, recipe))
        });
        Some(GridRead {
            snapshot,
            recipe,
            mark,
        })
    }

    /// The grid the canvas selection is or lies in: a selected top group, or the nearest grid
    /// group above a selected item.
    pub(in crate::session) fn selected_test_grid(&self) -> Option<NodeId> {
        self.selection
            .ids()
            .iter()
            .find_map(|&id| self.test_grid_above(id))
    }

    /// The grid whose top group is `id` or lies above it.
    fn test_grid_above(&self, id: NodeId) -> Option<NodeId> {
        let mut at = Some(id);
        while let Some(node) = at {
            if self.document.test_grid(node).is_some() {
                return Some(node);
            }
            at = self.document.parent_group(node);
        }
        None
    }

    /// Whether `id` is a test grid's top group or an item in one: it runs with its grid row only
    /// and is never assigned to another operation (criterion 6).
    pub(in crate::session) fn in_test_grid(&self, id: NodeId) -> bool {
        self.test_grid_above(id).is_some()
    }

    /// Whether the selection is one test grid as a whole (all its cells and labels): such a selection is moved, never resized, rotated or skewed (UX notes §6.5). One
    /// item or a part of the grid, entered by a double-click, is edited like any item.
    pub(in crate::session) fn whole_test_grid_selected(&self) -> bool {
        let ids = self.selection.ids();
        if ids.is_empty() || !self.operations_available() {
            return false;
        }
        let Some(group) = self.selected_test_grid() else {
            return false;
        };
        let Some(grid) = self.document.test_grid(group) else {
            return false;
        };
        let items = || grid.cells.iter().chain(&grid.labels);
        ids.len() == items().count() && ids.iter().all(|id| items().any(|item| item.id == *id))
    }

    /// Whether the Select bar hides Flip, Guide, Object to path and Linked clone: the selection is
    /// one whole test grid, whose shape comes from its fields (UX notes §6.5).
    #[must_use]
    pub fn select_bar_hides_grid_commands(&self) -> bool {
        self.tool == super::Tool::Select && self.whole_test_grid_selected()
    }

    /// The grid the Grid block shows: the canvas selection's, else the pressed row's.
    pub(in crate::session) fn focused_test_grid(&self) -> Option<NodeId> {
        if !self.operations_available() {
            return None;
        }
        self.selected_test_grid().or_else(|| {
            let row = self.test_grids.row?;
            self.document.test_grid_of_row(row)
        })
    }

    /// The grid row pressed in the list (UX notes §5): the Grid block shows its grid. `None`, or
    /// a row that is no grid row, closes it.
    pub fn select_test_grid_row(&mut self, row: Option<NodeId>) {
        self.test_grids.row = row;
        self.test_grids.invalid = None;
    }

    /// The marked cell of the grid in the Grid block, with its recipe (criterion 14).
    pub(in crate::session) fn worked_test_cell(&self) -> Option<(GridRecipe, CellIndex)> {
        let grid = self.read_test_grid(self.focused_test_grid()?)?;
        Some((grid.recipe?, grid.mark?))
    }

    /// Whether the canvas selection has just come to hold another grid: the panel then shows the
    /// Operations tab instead of Style or Document (UX notes §6.1, M3).
    pub(in crate::session) fn grid_selection_edge(&self) -> bool {
        let now = if self.operations_available() {
            self.selected_test_grid()
        } else {
            None
        };
        let before = self.test_grids.seen.replace(now);
        now.is_some() && now != before
    }

    /// The refusal of the last Add test grid, `None` when it added one.
    #[must_use]
    pub fn test_grid_notice(&self) -> Option<String> {
        self.test_grids.notice.clone()
    }
}
