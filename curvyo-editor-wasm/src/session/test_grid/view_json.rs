//! The Grid block's view as JSON (`specs/0028` criteria 1, 2, 7 to 14; UX notes §6, §7;
//! `adrs.md` §6): the one producer of `test_grid_view`, pinned by the shared fixtures in
//! `curvyo-editor-wasm/tests/fixtures/test_grid_view/`.

use curvyo_document_core::NodeId;
use curvyo_laser_core::settings::CellIndex;
use curvyo_laser_core::test_grids::{CellTag, GridRecipe, fit_bed};
use serde::Serialize;

use super::super::Session;
use super::super::materials::SetKindView;
use super::GridRead;
use super::build::is_hand_edited;

/// The Grid block of the grid the canvas selection or the pressed row names.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TestGridView {
    /// The grid's top group.
    pub group: NodeId,
    /// Its row, `None` when it has none (the row was deleted).
    pub row: Option<NodeId>,
    /// The top group's name ("Test grid 1").
    pub name: String,
    /// The page's machine ("for `<profile name>`").
    pub machine: String,
    /// The grid's size with labels, mm.
    pub width_mm: f64,
    /// The grid's size with labels, mm.
    pub height_mm: f64,
    /// The machine's maximum feed, the top of the speed fields.
    pub max_feed_mm_min: u64,
    /// The Kind field.
    pub kind: SetKindView,
    /// The cell fields.
    pub fields: FieldsView,
    /// The label fields.
    pub labels: LabelsView,
    /// The field value the grid did not take, kept with its chip.
    pub invalid: Option<InvalidView>,
    /// Why the fields' grid does not fit the bed ("Grid is 260 mm wide, the bed is 200 mm.").
    pub fit: Option<String>,
    /// Whether an item was changed by hand (criterion 10): "Edited by hand" with Rebuild.
    pub hand_edited: bool,
    /// The result table (criterion 12).
    pub result: ResultView,
    /// The Worked line, `None` without a mark ("No cell marked").
    pub worked: Option<WorkedView>,
    /// The table cell whose square is the canvas selection.
    pub selected_cell: Option<CellView>,
}

/// The cell fields of the Grid block.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FieldsView {
    /// Columns.
    pub columns: u8,
    /// Rows.
    pub rows: u8,
    /// Power from, percent.
    pub power_from_pct: f64,
    /// Power to, percent.
    pub power_to_pct: f64,
    /// Speed from, mm/min.
    pub speed_from_mm_min: u64,
    /// Speed to, mm/min.
    pub speed_to_mm_min: u64,
    /// Cell size, mm.
    pub cell_mm: f64,
    /// Gap, mm.
    pub gap_mm: f64,
}

/// The label fields of the Grid block.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct LabelsView {
    /// The Labels switch.
    pub on: bool,
    /// Engrave or Score.
    pub kind: SetKindView,
    /// Height, mm.
    pub height_mm: f64,
    /// Power, percent.
    pub power_pct: f64,
    /// Speed, mm/min.
    pub speed_mm_min: u64,
    /// Passes.
    pub passes: u8,
}

/// A typed value the grid did not take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InvalidView {
    /// The field's name (`columns`, `power_from`, ...).
    pub field: String,
    /// The text as typed.
    pub typed: String,
    /// The chip text.
    pub chip: String,
}

/// One cell's place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CellView {
    /// Its column, from 0 at the left.
    pub column: u8,
    /// Its row, from 0 at the top.
    pub row: u8,
}

/// The result table: power across, speed down.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResultView {
    /// The column heads, percent.
    pub powers: Vec<f64>,
    /// The row heads, mm/min.
    pub speeds: Vec<u64>,
}

/// The Worked line (criterion 14).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkedView {
    /// The marked cell.
    pub cell: CellView,
    /// Its power, percent.
    pub power_pct: f64,
    /// Its speed, mm/min.
    pub speed_mm_min: u64,
    /// "Worked: 60 % power, 3000 mm/min".
    pub text: String,
}

fn fields_view(recipe: &GridRecipe) -> FieldsView {
    let range = &recipe.range;
    FieldsView {
        columns: range.columns,
        rows: range.rows,
        power_from_pct: range.power_from.percent(),
        power_to_pct: range.power_to.percent(),
        speed_from_mm_min: u64::from(range.speed_from.get()),
        speed_to_mm_min: u64::from(range.speed_to.get()),
        cell_mm: range.cell.as_mm(),
        gap_mm: range.gap.as_mm(),
    }
}

fn labels_view(recipe: &GridRecipe) -> LabelsView {
    let labels = &recipe.labels;
    LabelsView {
        on: labels.on,
        kind: labels.kind.into(),
        height_mm: labels.height.as_mm(),
        power_pct: labels.settings.power.percent(),
        speed_mm_min: u64::from(labels.settings.speed.get()),
        passes: labels.settings.passes.get(),
    }
}

/// The worked cell with the numbers it was cut with.
fn worked_view(recipe: &GridRecipe, cell: CellIndex) -> WorkedView {
    let set = recipe.cell_settings(cell);
    WorkedView {
        cell: CellView {
            column: cell.column,
            row: cell.row,
        },
        power_pct: set.power.percent(),
        speed_mm_min: u64::from(set.speed.get()),
        text: format!("Worked: {} % power, {} mm/min", set.power, set.speed.get()),
    }
}

impl Session {
    /// The grid's row name, else its group name, else "Test grid".
    fn grid_name(&self, group: NodeId, row: Option<NodeId>) -> String {
        let row_name = self.active_operations().and_then(|active| {
            active
                .operations
                .into_iter()
                .find(|op| Some(op.id) == row)
                .map(|op| op.name)
        });
        row_name
            .or_else(|| self.document.node_registers(group).and_then(|r| r.name))
            .unwrap_or_else(|| "Test grid".to_owned())
    }

    /// The cell of `grid` that the one selected item is, if any.
    fn selected_grid_cell(&self, grid: &GridRead) -> Option<CellView> {
        match self.selection.ids() {
            [one] => grid
                .snapshot
                .cells
                .iter()
                .find(|item| item.id == *one)
                .and_then(|item| CellTag::from_value(&item.tag))
                .map(|tag| CellView {
                    column: tag.cell.column,
                    row: tag.cell.row,
                }),
            _ => None,
        }
    }

    /// The rejected field entry of the grid `group`, if one is held.
    fn invalid_view(&self, group: NodeId) -> Option<InvalidView> {
        self.test_grids
            .invalid
            .as_ref()
            .filter(|invalid| invalid.group == group)
            .map(|invalid| InvalidView {
                field: invalid.field.clone(),
                typed: invalid.typed.clone(),
                chip: invalid.chip.clone(),
            })
    }

    /// The Grid block, `None` when no grid is selected on the canvas or in the list, or its
    /// recipe does not read (its tags are inert then).
    #[must_use]
    pub fn test_grid_view(&self) -> Option<TestGridView> {
        let group = self.focused_test_grid()?;
        let grid = self.read_test_grid(group)?;
        let recipe = grid.recipe?;
        let laid = self.grid_layout(&recipe);
        let bed = self.active_manufacturing()?.work_area();
        let copy = self.active_copy()?;
        let row = grid.snapshot.rows.first().copied();
        let name = self.grid_name(group, row);
        let selected_cell = self.selected_grid_cell(&grid);
        let worked = grid.mark.map(|cell| worked_view(&recipe, cell));
        let invalid = self.invalid_view(group);
        Some(TestGridView {
            group,
            row,
            name,
            machine: copy.read.copy.values.name,
            width_mm: laid.width.as_mm(),
            height_mm: laid.height.as_mm(),
            max_feed_mm_min: u64::from(self.active_max_feed().get()),
            kind: recipe.kind.into(),
            fields: fields_view(&recipe),
            labels: labels_view(&recipe),
            invalid,
            fit: fit_bed(laid.width, laid.height, bed).err(),
            hand_edited: is_hand_edited(self, &grid, &recipe, &laid),
            result: ResultView {
                powers: (0..recipe.range.columns)
                    .map(|c| recipe.column_power(c).percent())
                    .collect(),
                speeds: (0..recipe.range.rows)
                    .map(|r| u64::from(recipe.row_speed(r).get()))
                    .collect(),
            },
            worked,
            selected_cell,
        })
    }

    /// The Grid block as JSON, `null` when none shows.
    #[must_use]
    pub fn test_grid_view_json(&self) -> String {
        serde_json::to_string(&self.test_grid_view()).unwrap_or_else(|_| "null".to_owned())
    }
}
