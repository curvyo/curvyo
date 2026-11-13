//! Where grids come from and go (`specs/0112` criteria 2, 8, 11, 13, 28): the tool's press and
//! Enter, Place on page from a test, the copy rule and the removal of rows off the laser page
//! after another command's step, a new id for a hand edit of a Lasered grid's row, and the
//! Lasered test hook.

use curvyo_document_core::{
    ADD_TEST_GRID_COMMIT_LABEL, GridStep, MARK_TEST_GRID_LASERED_COMMIT_LABEL, NodeId,
    PLACE_TEST_GRID_COMMIT_LABEL, Point,
};
use curvyo_laser_core::grids::{
    GRID_TOLERANCE_MM, MaterialRef, Recipe, TestId, free_spot, no_free_spot_text,
};

use super::super::Session;
use super::super::materials::{LibraryChange, now_ms};
use super::prepared::{FONT_LOADING_NOTICE, Target};

impl Session {
    /// Whether the Test grid tool is offered: the active page is a laser page (criterion 1).
    #[must_use]
    pub fn test_grid_tool_available(&self) -> bool {
        self.active_operations().is_some() && self.active_copy().is_some()
    }

    /// The bed boxes the items of the active page take, in bed coordinates.
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

    /// The grid's top-left for a grid of `width` x `height` mm: `at` when the grid fits the bed
    /// there (G2), else the first free spot (`0028` criterion 3).
    pub(super) fn grid_spot(
        &self,
        width: f64,
        height: f64,
        at: Option<Point>,
    ) -> Result<Point, String> {
        let manufacturing = self.active_manufacturing().ok_or_else(String::new)?;
        let bed = manufacturing.work_area();
        let origin = self.active_page().origin;
        if let Some(at) = at {
            let (x, y) = (at.x - origin.x, at.y - origin.y);
            let fits = x >= -GRID_TOLERANCE_MM
                && y >= -GRID_TOLERANCE_MM
                && x + width <= bed.width.as_mm() + GRID_TOLERANCE_MM
                && y + height <= bed.height.as_mm() + GRID_TOLERANCE_MM;
            if fits {
                return Ok(at);
            }
        }
        let length = curvyo_document_core::Length::from_mm;
        let spot = free_spot(
            length(width),
            length(height),
            bed,
            &self.occupied_on_bed(origin),
        )
        .ok_or_else(|| no_free_spot_text(length(width), length(height)))?;
        Ok(Point::new(origin.x + spot.x, origin.y + spot.y))
    }

    /// Places a new grid of `recipe` (Lasered at `lasered`) at `at` or the first free spot, in
    /// one step labelled `label`, with its test record when a material is picked; selected.
    fn place_new_grid(
        &mut self,
        recipe: &Recipe,
        at: Option<Point>,
        lasered: Option<u64>,
        label: &str,
    ) -> Result<NodeId, String> {
        let font = self
            .grid_label_font()
            .ok_or_else(|| FONT_LOADING_NOTICE.to_owned())?;
        let laid = self.grid_layout(recipe, &font);
        let manufacturing = self.active_manufacturing().ok_or_else(String::new)?;
        curvyo_laser_core::grids::fit_bed(laid.width, laid.height, manufacturing.work_area())?;
        let spot = self.grid_spot(laid.width.as_mm(), laid.height.as_mm(), at)?;
        let step = if label == PLACE_TEST_GRID_COMMIT_LABEL {
            GridStep::Place
        } else {
            GridStep::Add
        };
        let prepared = self.prepare_grid(recipe, spot, Target::Place { lasered, step })?;
        let mut change = LibraryChange::default();
        if lasered.is_none() {
            self.follow_test_record(&mut change, recipe, None)?;
        }
        let group = self.commit_library_change(label, change, |d| prepared.apply(d))?;
        self.select_grid(group);
        self.show_operations_for_new_grid();
        Ok(group)
    }

    /// A press of the Test grid tool at `point` (criterion 2, G2): on a grid it selects that
    /// grid; elsewhere it places a grid with the bar's values, its top-left at the press or at
    /// the first free spot, one step "Add test grid", selected. The grid's group, or the refusal
    /// shown as the notice.
    ///
    /// # Errors
    /// The bed cause, "No free spot ..." or the font notice.
    pub fn test_grid_press(&mut self, point: Option<Point>) -> Result<NodeId, String> {
        self.test_grid.notice = None;
        if let Some(point) = point
            && let Some(group) = self.grid_at(point)
        {
            self.select_grid(group);
            return Ok(group);
        }
        let recipe = Recipe {
            id: self.next_test_id(),
            test: self.draft_recipe(),
            material: self.test_grid.draft_material.clone(),
        };
        let placed = self.place_new_grid(&recipe, point, None, ADD_TEST_GRID_COMMIT_LABEL);
        match &placed {
            Ok(_) => self.remember_test_grid(&recipe.test),
            Err(refusal) => self.test_grid.notice = Some(refusal.clone()),
        }
        placed
    }

    /// The tool's pointer press: [`Session::test_grid_press`], its refusal kept as the notice.
    pub(in crate::session) fn test_grid_pointer_down(&mut self, point: Point) {
        let _ = self.test_grid_press(Some(point));
    }

    /// The pointer over the canvas with the tool active: the outline preview's place.
    pub fn test_grid_hover(&mut self, point: Option<Point>) {
        self.test_grid.hover = point;
    }

    /// "Place on `<page>`" or "Select on page" of test `test` in material `material` (criteria 23,
    /// 28): a grid with the test's id on a laser page is selected and its page made active;
    /// else the grid is recreated on the active laser page with the same id and fields at the
    /// first free spot, one step "Place test grid", selected.
    ///
    /// # Errors
    /// The bed cause, or an unknown test.
    pub fn place_material_test(&mut self, material: &str, test: &str) -> Result<NodeId, String> {
        let id = TestId::parse(test).ok_or_else(|| "The test was not found.".to_owned())?;
        if let Some(placed) = self.placed_test_grids().into_iter().find(|g| g.id == id) {
            self.activate_page(&placed.page_id);
            self.select_grid(placed.group);
            return Ok(placed.group);
        }
        let found = self
            .materials
            .library()
            .find(material)
            .ok_or_else(|| "The material was not found.".to_owned())?;
        let record = found
            .test(id)
            .ok_or_else(|| "The test was not found.".to_owned())?
            .clone();
        let recipe = Recipe {
            id,
            test: record.recipe,
            material: Some(MaterialRef {
                id: found.id.clone(),
                name: found.name.clone(),
            }),
        };
        self.place_new_grid(&recipe, None, record.lasered, PLACE_TEST_GRID_COMMIT_LABEL)
    }

    /// The box of the grid `group`'s items.
    pub(super) fn grid_box(&self, group: NodeId) -> Option<(Point, Point)> {
        let boxes: Vec<(Point, Point)> = self
            .document
            .leaves_of(group)
            .into_iter()
            .filter_map(|id| self.document.object(id))
            .map(|object| curvyo_ui_core::object_bounds(&object))
            .collect();
        let first = *boxes.first()?;
        Some(boxes.iter().fold(first, |(lo, hi), (min, max)| {
            (
                Point::new(lo.x.min(min.x), lo.y.min(min.y)),
                Point::new(hi.x.max(max.x), hi.y.max(max.y)),
            )
        }))
    }

    /// The grid of the active laser page whose box holds `point`.
    pub(super) fn grid_at(&self, point: Point) -> Option<NodeId> {
        let page = self.active_page().id;
        self.placed_test_grids()
            .into_iter()
            .filter(|grid| grid.page_id == page)
            .find(|grid| {
                self.grid_box(grid.group).is_some_and(|(lo, hi)| {
                    point.x >= lo.x && point.y >= lo.y && point.x <= hi.x && point.y <= hi.y
                })
            })
            .map(|grid| grid.group)
    }

    /// The page whose frame holds the centre of the grid `group`'s box.
    fn grid_page_id(&self, group: NodeId) -> Option<String> {
        let (lo, hi) = self.grid_box(group)?;
        let centre = Point::new(f64::midpoint(lo.x, hi.x), f64::midpoint(lo.y, hi.y));
        self.document
            .pages()
            .into_iter()
            .find(|page| {
                centre.x >= page.origin.x
                    && centre.y >= page.origin.y
                    && centre.x <= page.origin.x + page.size.width.as_mm()
                    && centre.y <= page.origin.y + page.size.height.as_mm()
            })
            .map(|page| page.id)
    }

    /// After a command's step: a grid moved off the laser page of its rows loses them in the same
    /// step (criterion 13); a copy Duplicate made gets a new id and its own rows (criterion 8).
    pub(in crate::session) fn reconcile_test_grids(&mut self, duplicated: bool) {
        for placed in self.placed_test_grids() {
            if self.grid_page_id(placed.group).as_deref() != Some(placed.page_id.as_str()) {
                self.document.continue_step();
                let _ = self.document.remove_test_grid_rows(placed.group);
            }
        }
        if !duplicated || !self.test_grid_tool_available() {
            return;
        }
        let mut copies: Vec<NodeId> = Vec::new();
        for &id in self.selection.ids() {
            if let Some(group) = self.test_grid_above(id)
                && !copies.contains(&group)
            {
                copies.push(group);
            }
        }
        let mut renewed: Vec<NodeId> = Vec::new();
        for group in copies {
            let Some(grid) = self.grid_object(group) else {
                continue;
            };
            if !grid.derived.is_empty() {
                continue;
            }
            let Some(font) = self.grid_label_font() else {
                continue;
            };
            let laid = self.grid_layout(&grid.recipe, &font);
            let Some(at) = self.grid_origin(&grid, &laid) else {
                continue;
            };
            let recipe = Recipe {
                id: self.next_test_id(),
                ..grid.recipe.clone()
            };
            if let Ok(prepared) = self.prepare_grid(&recipe, at, Target::Copy(group)) {
                self.document.continue_step();
                let _ = prepared.apply(&self.document);
                renewed.push(group);
            }
        }
        // The copies' items are new: the selection holds them again.
        if !renewed.is_empty() {
            let mut leaves: Vec<NodeId> = self
                .selection
                .ids()
                .iter()
                .copied()
                .filter(|&id| self.document.object(id).is_some())
                .collect();
            for group in renewed {
                leaves.extend(self.document.leaves_of(group));
            }
            self.selection.set(&leaves);
        }
    }

    /// After a hand edit of derived rows `rows` (settings, options or members; criterion 11):
    /// each Lasered grid among their grids gets a new id in the same step.
    pub(in crate::session) fn renew_lasered_grids_of(&mut self, rows: &[NodeId]) {
        let mut groups: Vec<NodeId> = Vec::new();
        for &row in rows {
            if let Some(group) = self.document.test_grid_of_derived_row(row)
                && !groups.contains(&group)
            {
                groups.push(group);
            }
        }
        for group in groups {
            if self.grid_object(group).is_none_or(|g| g.lasered.is_none()) {
                continue;
            }
            let id = self.next_test_id();
            self.document.continue_step();
            if self.document.renew_test_id(group, id.as_str()) == Ok(true) {
                self.test_grid.status = Some(format!("New test id {id}."));
            }
        }
    }

    /// The Lasered test hook (ADR 0057 §5): marks the grid `group` Lasered at `at_ms` and its
    /// test record with it, in one step. Only `0029`'s output and the acceptance tests call it;
    /// it is not bound to wasm.
    ///
    /// # Errors
    /// No such grid.
    #[doc(hidden)]
    pub fn mark_test_grid_lasered_for_tests(
        &mut self,
        group: NodeId,
        at_ms: u64,
    ) -> Result<(), String> {
        let grid = self
            .grid_object(group)
            .ok_or_else(|| "No such grid.".to_owned())?;
        let mut change = LibraryChange::default();
        if let Some(material) = &grid.recipe.material
            && self
                .materials
                .library()
                .find(&material.id)
                .is_some_and(|m| m.test(grid.recipe.id).is_some())
        {
            let id = grid.recipe.id;
            let now = now_ms(self);
            self.change_record(&mut change, &material.id, |m| {
                if let Some(test) = m.tests.iter_mut().find(|t| t.id == id) {
                    test.lasered = Some(at_ms);
                }
                m.modified = curvyo_laser_core::settings::next_modified(m.modified, now);
                Ok(())
            })?;
        }
        self.commit_library_change(MARK_TEST_GRID_LASERED_COMMIT_LABEL, change, |d| {
            d.mark_test_grid_lasered_for_tests(group, at_ms)
                .map_err(|e| e.to_string())
        })
    }
}
