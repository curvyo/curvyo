//! A Test grid write prepared outside the document, ready to apply in one step.

use curvyo_document_core::{
    DerivedPart, DerivedRow, Document, GridItems, GridStep, NewGridObject, NodeId, Point,
    TestGridWrite,
};
use curvyo_laser_core::grids::{
    DerivedOperation, GridPart, LabelSet, MaterialRef, Recipe, RecipeDefaults, derived_operations,
    fit_bed, fit_bed_at,
};
use curvyo_laser_core::operations::insert_index;
use curvyo_laser_core::settings::{MaterialLink, SetKind};

use super::super::Session;
use super::items::grid_items;

pub(super) type Plain = serde_json::Map<String, serde_json::Value>;

/// The refusal while the labels' face is not loaded yet (Proposal).
pub(in crate::session) const FONT_LOADING_NOTICE: &str = "The label font is still loading.";

pub(super) fn plain(value: serde_json::Value) -> Plain {
    match value {
        serde_json::Value::Object(map) => map,
        _ => Plain::new(),
    }
}

/// One derived operation, owned until the write borrows it.
#[derive(Debug, Clone, PartialEq)]
struct OwnedRow {
    part: DerivedPart,
    kind: &'static str,
    name: String,
    settings: Plain,
    options: Option<Plain>,
    at: usize,
}

/// What a prepared grid write does.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::session) enum Target {
    /// Regenerate the grid `group` (a bar commit).
    Regenerate(NodeId, GridStep),
    /// A new grid (the tool's press, Place on page).
    Place {
        lasered: Option<u64>,
        step: GridStep,
    },
    /// The copy `group` gets a new id and its own rows.
    Copy(NodeId),
}

/// A grid write prepared outside the document: the recipe, the items and the derived rows.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::session) struct PreparedGrid {
    manufacturing: NodeId,
    recipe: Plain,
    items: GridItems,
    rows: Vec<OwnedRow>,
    target: Target,
}

impl PreparedGrid {
    /// Writes it into `document` (inside the caller's batch); the grid's group.
    ///
    /// # Errors
    /// The document's refusal as text.
    pub(in crate::session) fn apply(&self, document: &Document) -> Result<NodeId, String> {
        let rows: Vec<DerivedRow<'_>> = self
            .rows
            .iter()
            .map(|row| DerivedRow {
                part: row.part,
                kind: row.kind,
                name: &row.name,
                settings: &row.settings,
                options: row.options.as_ref(),
                at: row.at,
            })
            .collect();
        let write = TestGridWrite {
            manufacturing: self.manufacturing,
            recipe: &self.recipe,
            items: &self.items,
            rows: &rows,
        };
        let refused = |e: curvyo_document_core::TestGridError| e.to_string();
        match &self.target {
            Target::Regenerate(group, step) => document
                .regenerate_test_grid(*group, &write, *step)
                .map(|()| *group)
                .map_err(refused),
            Target::Copy(group) => document
                .copy_test_grid(*group, &write)
                .map(|()| *group)
                .map_err(refused),
            Target::Place { lasered, step } => document
                .place_test_grid(&NewGridObject {
                    name: "Test grid",
                    write,
                    lasered: *lasered,
                    step: *step,
                })
                .map_err(refused),
        }
    }
}

const fn part_of(part: GridPart) -> DerivedPart {
    match part {
        GridPart::Labels => DerivedPart::Labels,
        GridPart::Cell { column, row } => DerivedPart::Cell { column, row },
    }
}

impl Session {
    /// The Engrave set of the picked material for the page's laser, which the labels link to
    /// (criterion 10).
    fn label_set(&self, recipe: &Recipe) -> Option<LabelSet> {
        let picked = recipe.material.as_ref()?;
        let material = self.materials.library().find(&picked.id)?;
        let laser = self.active_profile_id()?;
        let set = material.set_for(&laser, SetKind::Engrave)?;
        Some(LabelSet {
            link: MaterialLink {
                id: material.id.clone(),
                name: material.name.clone(),
                outline_from: None,
                modified: material.modified,
            },
            numbers: set.numbers,
        })
    }

    /// The defaults a recipe on the active page takes (criterion 6, G7): the machine's maximum
    /// feed and the picked material's set of `kind` for the page's laser.
    pub(in crate::session) fn recipe_defaults(
        &self,
        material: Option<&MaterialRef>,
        kind: SetKind,
    ) -> RecipeDefaults {
        let set = material.and_then(|m| {
            let laser = self.active_profile_id()?;
            self.materials
                .library()
                .find(&m.id)?
                .set_for(&laser, kind)
                .map(|set| set.numbers)
        });
        RecipeDefaults {
            max_feed: self.active_max_feed(),
            set,
        }
    }

    /// The derived rows of `recipe` with their place for a new grid: the end of each kind's
    /// section of the active list.
    fn owned_rows(&self, recipe: &Recipe) -> Result<Vec<OwnedRow>, String> {
        let active = self.active_operations().ok_or_else(String::new)?;
        let kinds = active.kinds();
        let derived: Vec<DerivedOperation> =
            derived_operations(recipe, self.label_set(recipe).as_ref());
        Ok(derived
            .into_iter()
            .map(|op| OwnedRow {
                part: part_of(op.part),
                kind: op.kind.as_str(),
                at: insert_index(&kinds, op.kind.section(), active.cuts_last),
                name: op.name,
                settings: plain(op.settings.to_value()),
                options: op.options.map(|options| plain(options.to_value(op.kind))),
            })
            .collect())
    }

    /// The write of `recipe` with its layout's top-left at `at` as `target`; refused when the
    /// face is loading or the grid does not fit the bed, a regenerated grid also when it would
    /// run past the bed's edge from where it lies (criterion 5).
    pub(in crate::session) fn prepare_grid(
        &self,
        recipe: &Recipe,
        at: Point,
        target: Target,
    ) -> Result<PreparedGrid, String> {
        let font = self
            .grid_label_font()
            .ok_or_else(|| FONT_LOADING_NOTICE.to_owned())?;
        let laid = self.grid_layout(recipe, &font);
        let manufacturing = self.active_manufacturing().ok_or_else(String::new)?;
        let bed = manufacturing.work_area();
        if matches!(target, Target::Regenerate(..)) {
            let origin = self.active_page().origin;
            let on_bed = Point::new(at.x - origin.x, at.y - origin.y);
            fit_bed_at(on_bed, laid.width, laid.height, bed)?;
        } else {
            fit_bed(laid.width, laid.height, bed)?;
        }
        Ok(PreparedGrid {
            manufacturing: manufacturing.id,
            recipe: plain(recipe.to_value()),
            items: grid_items(recipe, &laid, at, &font),
            rows: self.owned_rows(recipe)?,
            target,
        })
    }
}
