//! The Test grid bar's commits (`specs/0112` criteria 3 to 6, 11, 14, 22): one field, an axis
//! parameter, the kind, Crosshatch or the material per commit, each regenerating the grid and its
//! derived operations in one step with the test record written in the same step; a value that
//! does not take is kept with its cause and writes nothing.

use curvyo_document_core::{
    EDIT_TEST_GRID_COMMIT_LABEL, GridStep, NodeId, USE_MATERIAL_COMMIT_LABEL,
};
use curvyo_laser_core::grids::{
    MaterialRef, Param, Recipe, RecipeField, Side, TestRecipe, remember_test_grid,
    write_test_grid_choice,
};
use curvyo_laser_core::library::{RecordRefusal, TestRecord};
use curvyo_laser_core::settings::SetKind;

use super::super::Session;
use super::super::materials::MaterialWrite;
use super::super::materials::now_ms;
use super::prepared::{FONT_LOADING_NOTICE, PreparedGrid, Target};
use super::{GridObject, Invalid};

/// The bar's field of its name.
fn recipe_field(name: &str) -> Option<RecipeField> {
    Some(match name {
        "columns_count" => RecipeField::Count(Side::Columns),
        "rows_count" => RecipeField::Count(Side::Rows),
        "columns_min" => RecipeField::Min(Side::Columns),
        "columns_max" => RecipeField::Max(Side::Columns),
        "rows_min" => RecipeField::Min(Side::Rows),
        "rows_max" => RecipeField::Max(Side::Rows),
        "cell" => RecipeField::Cell,
        other => RecipeField::Fixed(Param::parse(other)?),
    })
}

fn side_of(name: &str) -> Option<Side> {
    match name {
        "columns" => Some(Side::Columns),
        "rows" => Some(Side::Rows),
        _ => None,
    }
}

impl Session {
    /// Writes the test record of `recipe` into its material as part of `change` (ADR 0058 §3):
    /// made at the pick, its recipe rewritten while not Lasered, moved from `old` (the material
    /// it was in) to a newly picked one without retiring its id.
    pub(in crate::session) fn follow_test_record(
        &mut self,
        change: &mut super::super::materials::LibraryChange,
        recipe: &Recipe,
        old: Option<&str>,
    ) -> Result<(), String> {
        let Some(picked) = recipe.material.as_ref() else {
            return Ok(());
        };
        let Some(machine) = self.active_laser() else {
            return Ok(());
        };
        let now = now_ms(self);
        let id = recipe.id;
        let mut record = TestRecord {
            id,
            created: now,
            machine,
            recipe: recipe.test.clone(),
            lasered: None,
            best: None,
            note: String::new(),
            photo: None,
        };
        if let Some(old) = old.filter(|old| *old != picked.id)
            && let Some(test) = self
                .materials
                .library()
                .find(old)
                .and_then(|m| m.test(id))
                .filter(|t| t.lasered.is_none())
                .cloned()
        {
            self.change_record(change, old, |m| m.take_test(id, now).map(|_| ()))?;
            record.created = test.created;
            record.best = test.best;
            record.note = test.note;
            record.photo = test.photo;
        }
        if self.materials.library().find(&picked.id).is_none() {
            return Ok(());
        }
        match self.change_record(change, &picked.id, |m| m.put_test(record, now)) {
            Err(text)
                if text == RecordRefusal::Unchanged.to_string()
                    || text == RecordRefusal::Lasered.to_string() =>
            {
                Ok(())
            }
            other => other,
        }
    }

    /// Remembers the recipe of `recipe` as this machine's last (criterion 2) and queues the file
    /// write; a preference, no step.
    pub(in crate::session) fn remember_test_grid(&mut self, recipe: &TestRecipe) {
        let Some(profile) = self.active_profile_id() else {
            return;
        };
        if self.test_grid.choice.get(&profile) == Some(recipe) {
            return;
        }
        remember_test_grid(&mut self.test_grid.choice, &profile, recipe.clone());
        let text = write_test_grid_choice(&self.test_grid.choice);
        self.materials.queue(MaterialWrite::GridChoice { text });
    }

    /// The recipe the tool places next: this machine's remembered one, else the defaults.
    pub(in crate::session) fn draft_recipe(&self) -> TestRecipe {
        let profile = self.active_profile_id().unwrap_or_default();
        self.test_grid
            .choice
            .get(&profile)
            .cloned()
            .unwrap_or_else(|| TestRecipe::new(&self.recipe_defaults(None, SetKind::Cut)))
    }

    /// The tool's draft as a recipe: the next grid's values and material, a placeholder id.
    pub(in crate::session) fn draft_grid(&self) -> Recipe {
        Recipe {
            // invariant: a valid id text.
            #[allow(clippy::expect_used)]
            id: curvyo_laser_core::grids::TestId::parse("00000").expect("valid id"),
            test: self.draft_recipe(),
            material: self.test_grid.draft_material.clone(),
        }
    }

    /// The grid the bar edits: the one selected as a whole.
    fn bar_grid(&self) -> Option<GridObject> {
        self.grid_object(self.whole_selected_grid()?)
    }

    /// One bar commit on the selected grid, or on the tool's draft without one: `next` over the
    /// current recipe, kept as `field` = `typed` with its cause when it does not take.
    fn bar_commit(
        &mut self,
        next: impl FnOnce(&Self, &Recipe) -> Result<(TestRecipe, Option<MaterialRef>), String>,
        field: &str,
        typed: &str,
        label: &str,
    ) -> Result<bool, String> {
        self.test_grid.notice = None;
        let grid = self.bar_grid();
        let current = match &grid {
            Some(grid) => grid.recipe.clone(),
            None => self.draft_grid(),
        };
        let result = next(self, &current).and_then(|(test, material)| {
            if let Some(grid) = &grid {
                return self.commit_grid_recipe(grid, test, material, label);
            }
            let changed = test != current.test || material != current.material;
            self.test_grid.draft_material = material;
            self.remember_test_grid(&test);
            Ok(changed)
        });
        match result {
            Ok(changed) => {
                self.test_grid.invalid = None;
                Ok(changed)
            }
            Err(cause) => {
                self.test_grid.invalid = Some(Invalid {
                    group: grid.map(|g| g.group),
                    field: field.to_owned(),
                    typed: typed.to_owned(),
                    cause: cause.clone(),
                });
                Err(cause)
            }
        }
    }

    /// Regenerates `grid` with `test` and `material` in one step `label` (criterion 14): a
    /// Lasered grid whose recipe changes gets a new id (criterion 11), the test record follows
    /// in the same step (criterion 22). `Ok(false)` when nothing changes.
    fn commit_grid_recipe(
        &mut self,
        grid: &GridObject,
        test: TestRecipe,
        material: Option<MaterialRef>,
        label: &str,
    ) -> Result<bool, String> {
        if test == grid.recipe.test && material == grid.recipe.material {
            return Ok(false);
        }
        let font = self
            .grid_label_font()
            .ok_or_else(|| FONT_LOADING_NOTICE.to_owned())?;
        let old_laid = self.grid_layout(&grid.recipe, &font);
        let at = self
            .grid_origin(grid, &old_laid)
            .unwrap_or_else(|| self.active_page().origin);
        let renews = grid.lasered.is_some();
        let id = if renews {
            self.next_test_id()
        } else {
            grid.recipe.id
        };
        let recipe = Recipe { id, test, material };
        let step = if label == USE_MATERIAL_COMMIT_LABEL {
            GridStep::UseMaterial
        } else {
            GridStep::Edit
        };
        let prepared = self.prepare_grid(&recipe, at, Target::Regenerate(grid.group, step))?;
        let mut change = super::super::materials::LibraryChange::default();
        let old_material = grid.recipe.material.as_ref().map(|m| m.id.clone());
        self.follow_test_record(&mut change, &recipe, old_material.as_deref())?;
        let group = self.commit_library_change(label, change, |d| prepared.apply(d))?;
        self.select_grid(group);
        self.remember_test_grid(&recipe.test);
        if renews {
            self.test_grid.status = Some(format!("New test id {id}."));
        }
        Ok(true)
    }

    /// Selects the grid `group` as a whole (its items are new after a regeneration).
    pub(in crate::session) fn select_grid(&mut self, group: NodeId) {
        let leaves = self.document.leaves_of(group);
        self.selection.set(&leaves);
    }

    /// Types `typed` into the bar's field `field` (`columns_count`, `rows_min`, `cell`, `passes`,
    /// ...; criteria 3, 5): a valid value that fits the bed regenerates the grid, one step "Edit
    /// test grid"; otherwise the value is kept with its cause and nothing changes. `Ok(false)`
    /// when the value is the stored one.
    ///
    /// # Errors
    /// The cause, at most 70 characters.
    pub fn set_test_grid_field(&mut self, field: &str, typed: &str) -> Result<bool, String> {
        let max_feed = self.active_max_feed();
        self.bar_commit(
            |_, current| {
                let f = recipe_field(field).ok_or_else(|| "Enter a number.".to_owned())?;
                Ok((
                    current.test.with_field(f, typed, max_feed)?,
                    current.material.clone(),
                ))
            },
            field,
            typed,
            EDIT_TEST_GRID_COMMIT_LABEL,
        )
    }

    /// A parameter switch of an axis (criterion 4, G6): `side` is `columns` or `rows`; the other
    /// axis's parameter swaps the axes. One step "Edit test grid".
    ///
    /// # Errors
    /// The bed cause.
    pub fn set_test_grid_axis(&mut self, side: &str, param: &str) -> Result<bool, String> {
        let (Some(side), Some(param)) = (side_of(side), Param::parse(param)) else {
            return Ok(false);
        };
        self.bar_commit(
            |this, current| {
                let defaults = this.recipe_defaults(current.material.as_ref(), current.test.kind());
                Ok((
                    current.test.with_axis_param(side, param, &defaults),
                    current.material.clone(),
                ))
            },
            "axis",
            param.as_str(),
            EDIT_TEST_GRID_COMMIT_LABEL,
        )
    }

    /// The Kind segments (criterion 3): one step "Edit test grid".
    ///
    /// # Errors
    /// The bed cause.
    pub fn set_test_grid_kind(&mut self, kind: &str) -> Result<bool, String> {
        let Some(kind) = SetKind::parse(kind) else {
            return Ok(false);
        };
        self.bar_commit(
            |this, current| {
                let defaults = this.recipe_defaults(current.material.as_ref(), kind);
                Ok((
                    current.test.with_kind(kind, &defaults),
                    current.material.clone(),
                ))
            },
            "kind",
            kind.as_str(),
            EDIT_TEST_GRID_COMMIT_LABEL,
        )
    }

    /// The Crosshatch toggle of an Engrave grid (criterion 3): one step "Edit test grid".
    ///
    /// # Errors
    /// The bed cause.
    pub fn set_test_grid_crosshatch(&mut self, on: bool) -> Result<bool, String> {
        self.bar_commit(
            |_, current| Ok((current.test.with_crosshatch(on), current.material.clone())),
            "crosshatch",
            if on { "on" } else { "off" },
            EDIT_TEST_GRID_COMMIT_LABEL,
        )
    }

    /// The material dropdown (criteria 6, 22, G8): the grid uses material `id` (`None`: no
    /// material), its shared values take the material's set, the labels link to its Engrave
    /// set and its test record is made or moved, one step "Use material".
    ///
    /// # Errors
    /// The bed cause, or an unknown material.
    pub fn pick_test_grid_material(&mut self, id: Option<&str>) -> Result<bool, String> {
        let material = match id {
            Some(id) => {
                let found = self
                    .materials
                    .library()
                    .find(id)
                    .ok_or_else(|| "The material was not found.".to_owned())?;
                Some(MaterialRef {
                    id: found.id.clone(),
                    name: found.name.clone(),
                })
            }
            None => None,
        };
        self.bar_commit(
            |this, current| {
                let defaults = this.recipe_defaults(material.as_ref(), current.test.kind());
                Ok((
                    current.test.with_shared_defaults(&defaults),
                    material.clone(),
                ))
            },
            "material",
            id.unwrap_or_default(),
            USE_MATERIAL_COMMIT_LABEL,
        )
    }

    /// The grid write that picks material `id` for the grid `group` as part of `change` (UX G9,
    /// the New material form opened from the bar); the caller commits it.
    ///
    /// # Errors
    /// The bed cause, or no such grid.
    pub(in crate::session) fn prepare_grid_material(
        &mut self,
        change: &mut super::super::materials::LibraryChange,
        group: NodeId,
        id: &str,
    ) -> Result<PreparedGrid, String> {
        let grid = self.grid_object(group).ok_or_else(String::new)?;
        let found = self.materials.library().find(id).ok_or_else(String::new)?;
        let material = Some(MaterialRef {
            id: found.id.clone(),
            name: found.name.clone(),
        });
        let defaults = self.recipe_defaults(material.as_ref(), grid.recipe.test.kind());
        let font = self
            .grid_label_font()
            .ok_or_else(|| FONT_LOADING_NOTICE.to_owned())?;
        let at = self
            .grid_origin(&grid, &self.grid_layout(&grid.recipe, &font))
            .unwrap_or_else(|| self.active_page().origin);
        let id = if grid.lasered.is_some() {
            self.next_test_id()
        } else {
            grid.recipe.id
        };
        let recipe = Recipe {
            id,
            test: grid.recipe.test.with_shared_defaults(&defaults),
            material,
        };
        let prepared = self.prepare_grid(
            &recipe,
            at,
            Target::Regenerate(group, GridStep::UseMaterial),
        )?;
        let old = grid.recipe.material.as_ref().map(|m| m.id.clone());
        self.follow_test_record(change, &recipe, old.as_deref())?;
        Ok(prepared)
    }
}
