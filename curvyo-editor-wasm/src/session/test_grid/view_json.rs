//! The Test grid bar's view as JSON (`specs/0112` criteria 3 to 6, 11; UX notes 4; `adrs.md`
//! finding 3): the one producer of `test_grid_view`, pinned by the shared fixtures in
//! `curvyo-editor-wasm/tests/fixtures/test_grid_view/`.

use curvyo_document_core::NodeId;
use curvyo_laser_core::grids::{Param, Recipe, Side, fit_bed};
use curvyo_laser_core::library::MaterialRead;
use serde::Serialize;

use super::super::Session;
use super::super::materials::SetKindView;

/// The bar of the selected grid, or of the tool's draft.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TestGridView {
    /// The grid's group, `None` for the tool's draft (no grid selected).
    pub group: Option<NodeId>,
    /// The test id, `None` for the draft.
    pub id: Option<String>,
    /// Whether the grid is Lasered.
    pub lasered: bool,
    /// The Kind segments.
    pub kind: SetKindView,
    /// The picked material.
    pub material: Option<MaterialPickView>,
    /// The dropdown's materials in the tab's order (readable only).
    pub materials: Vec<MaterialPickView>,
    /// The shared values of row 1, in the order Power, Speed, Passes, Interval.
    pub shared: Vec<ValueView>,
    /// Crosshatch, Engrave only.
    pub crosshatch: Option<bool>,
    /// Cell size in mm.
    pub cell_mm: f64,
    /// The Columns unit.
    pub columns: AxisFieldsView,
    /// The Rows unit.
    pub rows: AxisFieldsView,
    /// The grid's size in mm.
    pub width_mm: f64,
    /// The grid's size in mm.
    pub height_mm: f64,
    /// The machine's maximum feed, the top of the speed fields.
    pub max_feed_mm_min: u32,
    /// The value the grid did not take, kept with its cause.
    pub invalid: Option<InvalidView>,
    /// Why the grid does not fit the bed ("Grid is 260 mm wide, the bed is 200 mm.").
    pub fit: Option<String>,
}

/// A material of the dropdown.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaterialPickView {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its thickness in mm, `None` for the picked material's stored name only.
    pub thickness_mm: Option<f64>,
}

/// One value of a parameter.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValueView {
    /// `power`, `speed`, `passes` or `interval`.
    pub param: &'static str,
    /// The number in the field.
    pub value: f64,
}

/// One axis unit of row 2.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AxisFieldsView {
    /// Its parameter.
    pub param: &'static str,
    /// The parameters its switch offers.
    pub offered: Vec<&'static str>,
    /// Min.
    pub min: f64,
    /// Max.
    pub max: f64,
    /// Count.
    pub count: u8,
}

/// A typed value the grid did not take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InvalidView {
    /// The field's name (`columns_count`, `cell`, `passes`, ...).
    pub field: String,
    /// The text as typed.
    pub typed: String,
    /// The cause.
    pub cause: String,
}

fn axis_view(recipe: &Recipe, side: Side) -> AxisFieldsView {
    let axis = recipe.test.axis(side);
    AxisFieldsView {
        param: axis.param().as_str(),
        offered: Param::of_kind(recipe.test.kind())
            .map(Param::as_str)
            .collect(),
        min: axis.min().number(),
        max: axis.max().number(),
        count: axis.count(),
    }
}

impl Session {
    /// The bar's recipe: the selected grid's, else the tool's draft with a placeholder id.
    fn bar_view_recipe(&self) -> Option<(Option<NodeId>, Recipe, bool)> {
        if let Some(group) = self.whole_selected_grid() {
            let grid = self.grid_object(group)?;
            return Some((Some(group), grid.recipe, grid.lasered.is_some()));
        }
        if self.tool != super::super::Tool::TestGrid {
            return None;
        }
        let recipe = Recipe {
            id: curvyo_laser_core::grids::TestId::parse("00000")?,
            test: self.draft_recipe(),
            material: self.test_grid.draft_material.clone(),
        };
        Some((None, recipe, false))
    }

    /// The Test grid bar: the selected grid's (also with the Select tool, G3), or the draft
    /// with the tool active; `None` otherwise or off a laser page.
    #[must_use]
    pub fn test_grid_view(&self) -> Option<TestGridView> {
        if !self.test_grid_tool_available() {
            return None;
        }
        let (group, recipe, lasered) = self.bar_view_recipe()?;
        let manufacturing = self.active_manufacturing()?;
        let font = self.grid_label_font();
        let laid = font.as_ref().map(|font| self.grid_layout(&recipe, font));
        let (width_mm, height_mm) = laid
            .as_ref()
            .map_or((0.0, 0.0), |l| (l.width.as_mm(), l.height.as_mm()));
        let fit = laid
            .as_ref()
            .and_then(|l| fit_bed(l.width, l.height, manufacturing.work_area()).err());
        let materials = self.material_picks();
        let material = self.picked_material(&recipe);
        let invalid = self.invalid_view(group);
        let kind = recipe.test.kind();
        Some(TestGridView {
            group,
            id: group.map(|_| recipe.id.to_string()),
            lasered,
            kind: kind.into(),
            material,
            materials,
            shared: recipe
                .test
                .fixed_values()
                .into_iter()
                .map(|v| ValueView {
                    param: v.param.as_str(),
                    value: v.number(),
                })
                .collect(),
            crosshatch: (kind == curvyo_laser_core::settings::SetKind::Engrave)
                .then(|| recipe.test.crosshatch()),
            cell_mm: recipe.test.cell().as_mm(),
            columns: axis_view(&recipe, Side::Columns),
            rows: axis_view(&recipe, Side::Rows),
            width_mm,
            height_mm,
            max_feed_mm_min: self.active_max_feed().get(),
            invalid,
            fit,
        })
    }

    /// Every readable material of the library, for the bar's picker.
    fn material_picks(&self) -> Vec<MaterialPickView> {
        self.materials
            .library()
            .listed()
            .into_iter()
            .filter_map(|read| match read {
                MaterialRead::Loaded(m) => Some(MaterialPickView {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    thickness_mm: Some(m.thickness.mm()),
                }),
                _ => None,
            })
            .collect()
    }

    /// The material `recipe` links to; the recipe's own name when the library no longer has it.
    fn picked_material(&self, recipe: &Recipe) -> Option<MaterialPickView> {
        recipe.material.as_ref().map(|m| {
            let found = self.materials.library().find(&m.id);
            MaterialPickView {
                id: m.id.clone(),
                name: found.map_or_else(|| m.name.clone(), |f| f.name.clone()),
                thickness_mm: found.map(|f| f.thickness.mm()),
            }
        })
    }

    /// The kept field of a refused commit, when it is on `group`.
    fn invalid_view(&self, group: Option<NodeId>) -> Option<InvalidView> {
        self.test_grid
            .invalid
            .as_ref()
            .filter(|invalid| invalid.group == group)
            .map(|invalid| InvalidView {
                field: invalid.field.clone(),
                typed: invalid.typed.clone(),
                cause: invalid.cause.clone(),
            })
    }

    /// The bar as JSON, `null` when none shows.
    #[must_use]
    pub fn test_grid_view_json(&self) -> String {
        serde_json::to_string(&self.test_grid_view()).unwrap_or_else(|_| "null".to_owned())
    }
}
