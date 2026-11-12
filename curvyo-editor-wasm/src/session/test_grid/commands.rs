//! The commands of the Add group's sixth button and of the Grid block (`specs/0028` criteria 1
//! to 3, 7 to 10, 13): add, a field, the kind and the labels, Rebuild and the mark. Each is one
//! step with the label of the history table; a refused value writes nothing.

use curvyo_document_core::{NewTestGrid, NodeId, Point, RowMove, TestGridEdit};
use curvyo_laser_core::material_library::{RememberedGrid, remember_grid, write_grid_choice};
use curvyo_laser_core::operations::{KindRead, OperationKind, insert_index, next_test_grid_name};
use curvyo_laser_core::settings::{CellIndex, SetKind};
use curvyo_laser_core::test_grids::{
    GridField, GridRecipe, fit_bed, free_spot, kept_mark, mark_value, no_free_spot_text,
};

use super::super::Session;
use super::super::materials::MaterialWrite;
use super::build::{grid_items, grid_origin, is_hand_edited};
use super::{GridRead, Invalid};

/// The refusal while the labels' face is not loaded yet (Proposal).
const FONT_LOADING_NOTICE: &str = "The label font is still loading.";

/// The field of its name in the Grid block.
fn grid_field(name: &str) -> Option<GridField> {
    Some(match name {
        "columns" => GridField::Columns,
        "rows" => GridField::Rows,
        "power_from" => GridField::PowerFrom,
        "power_to" => GridField::PowerTo,
        "speed_from" => GridField::SpeedFrom,
        "speed_to" => GridField::SpeedTo,
        "cell" => GridField::Cell,
        "gap" => GridField::Gap,
        "label_height" => GridField::LabelHeight,
        "label_power" => GridField::LabelPower,
        "label_speed" => GridField::LabelSpeed,
        "label_passes" => GridField::LabelPasses,
        _ => return None,
    })
}

/// The plain map of a JSON value that is one.
fn plain(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    }
}

/// The operation kind of a grid's kind.
fn row_kind(kind: SetKind) -> OperationKind {
    match kind {
        SetKind::Cut => OperationKind::Cut,
        SetKind::Score => OperationKind::Score,
        SetKind::Engrave => OperationKind::Engrave,
    }
}

impl Session {
    /// Remembers `recipe` as this machine's last grid (criterion 2) and queues the file write.
    fn remember_test_grid(&mut self, recipe: &GridRecipe) {
        let Some(profile) = self.active_profile_id() else {
            return;
        };
        let grid = RememberedGrid {
            range: recipe.range,
            labels: recipe.labels,
        };
        if self.test_grids.choice.get(&profile) == Some(&grid) {
            return;
        }
        remember_grid(&mut self.test_grids.choice, &profile, grid);
        let text = write_grid_choice(&self.test_grids.choice);
        self.materials.queue(MaterialWrite::GridChoice { text });
    }

    /// The bed boxes the items of the active page take, in bed coordinates (top left 0, 0).
    fn occupied_on_bed(&self, origin: Point) -> Vec<(Point, Point)> {
        self.pickable_objects()
            .iter()
            .map(curvyo_ui_core::object_bounds)
            .map(|(min, max)| {
                (
                    Point::new(min.x - origin.x, min.y - origin.y),
                    Point::new(max.x - origin.x, max.y - origin.y),
                )
            })
            .collect()
    }

    /// Add test grid (criteria 1 to 3): a grid with this machine's remembered values or the
    /// defaults at the bed's top left or the first free spot, its row at the end of its section,
    /// in one step "Add test grid"; the grid is selected. The new row, or the refusal shown under
    /// the button (nothing is added).
    ///
    /// # Errors
    /// "Grid is 260 mm wide, the bed is 200 mm.", "No free spot on the bed for a 75 × 62 mm
    /// grid." or the font notice.
    pub fn press_add_test_grid(&mut self) -> Result<NodeId, String> {
        self.test_grids.notice = None;
        let result = self.add_test_grid();
        if let Err(refusal) = &result {
            self.test_grids.notice = Some(refusal.clone());
        }
        result
    }

    fn add_test_grid(&mut self) -> Result<NodeId, String> {
        let active = self.active_operations().ok_or_else(String::new)?;
        let manufacturing = self.active_manufacturing().ok_or_else(String::new)?;
        let max_feed = self.active_max_feed();
        let profile = self.active_profile_id().unwrap_or_default();
        let recipe = self.test_grids.choice.get(&profile).map_or_else(
            || GridRecipe::defaults(max_feed),
            |remembered| GridRecipe {
                kind: SetKind::Cut,
                range: remembered.range,
                labels: remembered.labels,
            },
        );
        let font = self.grid_label_font();
        if recipe.labels.on && font.is_none() {
            return Err(FONT_LOADING_NOTICE.to_owned());
        }
        let laid = self.grid_layout(&recipe);
        let bed = manufacturing.work_area();
        fit_bed(laid.width, laid.height, bed)?;
        let origin = self.active_page().origin;
        let occupied = self.occupied_on_bed(origin);
        let spot = free_spot(laid.width, laid.height, bed, &occupied)
            .ok_or_else(|| no_free_spot_text(laid.width, laid.height))?;
        let at = Point::new(origin.x + spot.x, origin.y + spot.y);
        let names = active.operations.iter().map(|op| op.name.as_str());
        let name = next_test_grid_name(names);
        let kind = row_kind(recipe.kind);
        let row_index = insert_index(&active.kinds(), kind.section(), active.cuts_last);
        let stored = plain(recipe.to_value());
        let new = NewTestGrid {
            manufacturing: active.manufacturing,
            name: &name,
            kind: kind.as_str(),
            recipe: &stored,
            items: grid_items(&recipe, &laid, at, font.as_ref()),
            row_index,
        };
        let made = self.recording_unguarded(|this| {
            let made = this.document.add_test_grid(&new).ok()?;
            // The selection holds the leaves, as every group selection does.
            let leaves = this.document.leaves_of(made.group);
            this.selection.set(&leaves);
            Some(made)
        });
        let made = made.ok_or_else(String::new)?;
        self.test_grids.row = Some(made.row);
        self.test_grids.invalid = None;
        self.remember_test_grid(&recipe);
        Ok(made.row)
    }

    /// The focused grid with a readable recipe.
    fn focused_recipe(&self) -> Option<(NodeId, GridRecipe)> {
        let group = self.focused_test_grid()?;
        let recipe = self.read_test_grid(group)?.recipe?;
        Some((group, recipe))
    }

    /// Types `typed` into the Grid block's field `field` (criteria 2, 9): a valid value that fits
    /// the bed rebuilds the grid in place, one step "Edit test grid"; otherwise the value is kept
    /// with its chip and nothing changes. `Ok(false)` when the value is the stored one.
    ///
    /// # Errors
    /// The chip text.
    pub fn set_test_grid_field(&mut self, field: &str, typed: &str) -> Result<bool, String> {
        let (group, recipe) = self.focused_recipe().ok_or_else(String::new)?;
        let checked = grid_field(field)
            .ok_or_else(|| "Enter a number.".to_owned())
            .and_then(|f| recipe.with_field(f, typed, self.active_max_feed()));
        self.edit_or_keep(group, &recipe, checked, field, typed)
    }

    /// The Kind radiogroup (criterion 8): rebuilds, clears the mark and moves the row to the end
    /// of its section. `Ok(false)` when unchanged.
    ///
    /// # Errors
    /// The bed cause.
    pub fn set_test_grid_kind(&mut self, kind: &str) -> Result<bool, String> {
        let (group, recipe) = self.focused_recipe().ok_or_else(String::new)?;
        let kind = SetKind::parse(kind).ok_or_else(String::new)?;
        self.edit_or_keep(
            group,
            &recipe,
            Ok(GridRecipe { kind, ..recipe }),
            "kind",
            kind.as_str(),
        )
    }

    /// The Labels switch (criterion 7). `Ok(false)` when unchanged.
    ///
    /// # Errors
    /// The bed cause, or the font notice.
    pub fn set_test_grid_labels(&mut self, on: bool) -> Result<bool, String> {
        let (group, mut next) = self.focused_recipe().ok_or_else(String::new)?;
        let recipe = next;
        next.labels.on = on;
        self.edit_or_keep(
            group,
            &recipe,
            Ok(next),
            "labels",
            if on { "on" } else { "off" },
        )
    }

    /// The label kind, Engrave or Score, never Cut (criterion 7). `Ok(false)` when unchanged.
    ///
    /// # Errors
    /// A Cut kind is refused.
    pub fn set_test_grid_label_kind(&mut self, kind: &str) -> Result<bool, String> {
        let (group, mut next) = self.focused_recipe().ok_or_else(String::new)?;
        let recipe = next;
        next.labels.kind = match SetKind::parse(kind) {
            Some(kind @ (SetKind::Engrave | SetKind::Score)) => kind,
            _ => return Err("Labels are engraved or scored.".to_owned()),
        };
        self.edit_or_keep(group, &recipe, Ok(next), "label_kind", kind)
    }

    /// Commits `checked` over `recipe` of grid `group`, or keeps the typed value with its chip.
    fn edit_or_keep(
        &mut self,
        group: NodeId,
        recipe: &GridRecipe,
        checked: Result<GridRecipe, String>,
        field: &str,
        typed: &str,
    ) -> Result<bool, String> {
        let committed = checked.and_then(|next| self.commit_grid_edit(group, recipe, &next));
        match committed {
            Ok(changed) => {
                self.test_grids.invalid = None;
                Ok(changed)
            }
            Err(chip) => {
                self.test_grids.invalid = Some(Invalid {
                    group,
                    field: field.to_owned(),
                    typed: typed.to_owned(),
                    chip: chip.clone(),
                });
                Err(chip)
            }
        }
    }

    /// With a Kind change: the rows' new kind and their index among the other operations.
    fn kind_change_move(
        &self,
        grid: &GridRead,
        recipe: &GridRecipe,
        next: &GridRecipe,
    ) -> Result<Option<(OperationKind, usize)>, String> {
        if next.kind == recipe.kind {
            return Ok(None);
        }
        let active = self.active_operations().ok_or_else(String::new)?;
        let kind = row_kind(next.kind);
        let others: Vec<KindRead> = active
            .operations
            .iter()
            .filter(|op| !grid.snapshot.rows.contains(&op.id))
            .map(|op| KindRead::of(op.kind.as_deref()))
            .collect();
        Ok(Some((
            kind,
            insert_index(&others, kind.section(), active.cuts_last),
        )))
    }

    /// One step "Edit test grid" from `recipe` to `next`: rebuilt in place unless hand edited,
    /// the mark kept when its cell keeps power and speed (cleared by a Kind change), the row
    /// moved with a Kind change.
    fn commit_grid_edit(
        &mut self,
        group: NodeId,
        recipe: &GridRecipe,
        next: &GridRecipe,
    ) -> Result<bool, String> {
        if next == recipe {
            return Ok(false);
        }
        let font = self.grid_label_font();
        if next.labels.on && font.is_none() {
            return Err(FONT_LOADING_NOTICE.to_owned());
        }
        let grid = self.read_test_grid(group).ok_or_else(String::new)?;
        let old_laid = self.grid_layout(recipe);
        let laid = self.grid_layout(next);
        let bed = self
            .active_manufacturing()
            .ok_or_else(String::new)?
            .work_area();
        fit_bed(laid.width, laid.height, bed)?;
        let edited = is_hand_edited(self, &grid, recipe, &old_laid);
        let at = grid_origin(self, &grid, &old_laid).unwrap_or(self.active_page().origin);
        let worked = if next.kind == recipe.kind {
            grid.mark.and_then(|mark| kept_mark(recipe, mark, next))
        } else {
            None
        };
        let row = self.kind_change_move(&grid, recipe, next)?;
        let stored = plain(next.to_value());
        let mark = worked.map(|cell| plain(mark_value(cell)));
        let items = (!edited).then(|| grid_items(next, &laid, at, font.as_ref()));
        let edit = TestGridEdit {
            group,
            recipe: &stored,
            worked: mark.as_ref(),
            items: items.as_ref(),
            row: row.map(|(kind, index)| RowMove {
                kind: kind.as_str(),
                index,
            }),
        };
        let reselect = self.selected_test_grid() == Some(group);
        let done = self.recording_unguarded(|this| {
            let done = this.document.edit_test_grid(&edit).is_ok();
            if done && reselect {
                this.select_rebuilt_test_grid(group);
            }
            done
        });
        if done {
            self.remember_test_grid(next);
        }
        Ok(done)
    }

    /// Rebuild of a hand-edited grid (criterion 10): the items replaced by the fields' grid at
    /// its place, the mark kept or cleared, one step "Rebuild test grid".
    pub fn rebuild_test_grid(&mut self) -> bool {
        let Some((group, recipe)) = self.focused_recipe() else {
            return false;
        };
        let Some(grid) = self.read_test_grid(group) else {
            return false;
        };
        let font = self.grid_label_font();
        if recipe.labels.on && font.is_none() {
            return false;
        }
        let laid = self.grid_layout(&recipe);
        let at = grid_origin(self, &grid, &laid).unwrap_or(self.active_page().origin);
        let items = grid_items(&recipe, &laid, at, font.as_ref());
        let mark = grid.mark.map(|cell| plain(mark_value(cell)));
        let reselect = self.selected_test_grid() == Some(group);
        self.recording_unguarded(|this| {
            let done = this
                .document
                .rebuild_test_grid(group, mark.as_ref(), &items)
                .is_ok();
            if done && reselect {
                this.select_rebuilt_test_grid(group);
            }
            done
        })
    }

    /// After a rebuild the old items are gone: a selection in grid `group` becomes its new leaves,
    /// so the next command (Add operation, a move) reads the grid and not dead ids.
    fn select_rebuilt_test_grid(&mut self, group: NodeId) {
        let leaves = self.document.leaves_of(group);
        self.selection.set(&leaves);
    }

    /// A press on the result table's cell (criterion 13): marks it and removes any other mark,
    /// or removes the mark of the marked cell; one step "Mark cell" or "Unmark cell".
    pub fn press_test_cell(&mut self, column: u8, row: u8) -> bool {
        let Some((group, recipe)) = self.focused_recipe() else {
            return false;
        };
        if column >= recipe.range.columns || row >= recipe.range.rows {
            return false;
        }
        let Some(grid) = self.read_test_grid(group) else {
            return false;
        };
        let cell = CellIndex { column, row };
        let mark = (grid.mark != Some(cell)).then(|| plain(mark_value(cell)));
        self.recording_unguarded(|this| {
            this.document
                .set_test_grid_mark(group, mark.as_ref())
                .unwrap_or(false)
        })
    }

    /// The Grid block's "Select": the grid becomes the canvas selection (UX notes §6.2).
    pub fn select_focused_test_grid(&mut self) -> bool {
        let Some(group) = self.focused_test_grid() else {
            return false;
        };
        let leaves = self.document.leaves_of(group);
        self.selection.set(&leaves);
        true
    }
}
