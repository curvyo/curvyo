//! The Tile mode of the Select bar (`specs/0069-tiling` criteria 1, 2, 26, 27, 32, 34 to 37):
//! entering with the defaults or a stored tiling, the recipe the bar edits, Apply, Cancel and
//! Release, and the locks while the bar is open. The preview is in `tiling_view.rs`, the circle
//! handles and their entry boxes in `tiling_handles.rs`.

use std::collections::HashSet;

use curvyo_document_core::{
    CircleOptions, NodeId, ObjectSnapshot, PathSnapshot, Point, SpacingMode, TILING_MAX_COUNT,
    TilingCell, TilingLayout, TilingRecipe,
};
use curvyo_ui_core::AnchorIdMinter;
use curvyo_ui_core::tiling::{
    CellBox, CircleHandle, CircleView, CountStatus, TileButton, cell_box, converted_spacing,
    count_status, default_recipe, images_become_copies, normalised_start, tile_button,
    tiling_layout, tiling_request,
};

use super::{Session, Tool};

/// The open entry box of a circle handle (UX notes §12.4): its handle and the circle values it
/// opened with, which Escape restores.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HandleEntry {
    pub(super) handle: CircleHandle,
    pub(super) opened_with: CircleOptions,
}

/// The open tiling bar: everything is session state until Apply (criterion 1).
#[derive(Debug, Clone)]
pub(super) struct TilingMode {
    /// The stored tiling being edited, `None` for a new one.
    pub(super) tiling: Option<NodeId>,
    /// The top-level sources.
    pub(super) sources: Vec<NodeId>,
    /// The recipe the bar shows.
    pub(super) recipe: TilingRecipe,
    /// The box of the sources.
    pub(super) cell_box: CellBox,
    /// The outline of every source leaf in document space with the leaf's opacity, for the
    /// preview.
    pub(super) outlines: Vec<(PathSnapshot, f64)>,
    /// The managed instances hidden while a stored tiling is reopened (criterion 34).
    pub(super) hidden: HashSet<NodeId>,
    /// The cells of the recipe, computed once per change.
    pub(super) cells: Vec<TilingCell>,
    /// A circle handle drag in flight.
    pub(super) drag: Option<CircleHandle>,
    /// The entry box of a circle handle, while it is open.
    pub(super) entry: Option<HandleEntry>,
    /// The core's refusal of the last Apply, until the next change.
    pub(super) refusal: Option<String>,
}

/// What the bar shows besides the recipe (UX notes §2.4 and §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TilingStatus {
    /// The count and whether it warns or refuses.
    pub count: CountStatus,
    /// Clones are chosen and the source holds an image.
    pub images_become_copies: bool,
    /// The core's refusal of the last Apply.
    pub refusal: Option<String>,
    /// A stored tiling is open ("Edit tiling").
    pub reopened: bool,
}

/// The outline of one source leaf for the preview: its drawn path, a primitive's outline, an
/// image's frame.
fn preview_outline(object: &ObjectSnapshot) -> Option<PathSnapshot> {
    match object {
        ObjectSnapshot::Primitive(primitive) => Some(curvyo_ui_core::primitive_as_path(
            primitive,
            &mut AnchorIdMinter::new(0),
        )),
        ObjectSnapshot::Image(image) => {
            let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
                .map(|(x, y)| image.placement.apply_point(Point::new(x, y)));
            Some(super::tiling_view::box_path(image.id, &corners))
        }
        ObjectSnapshot::Dimension(_) => None,
        other => other.drawn_path().cloned(),
    }
}

/// `recipe` with the count of the layout not in use brought under the limit (review item 8): a
/// grid of 1,000 x 1,000 left behind after switching to Circle would refuse the Apply and be
/// damage on open (ADR 0030 §10), so its columns and rows (or the circle count) are cut to the limit.
fn fit_inactive_layout(mut recipe: TilingRecipe) -> TilingRecipe {
    match recipe.layout {
        TilingLayout::Circle => {
            let columns = recipe.grid.columns.clamp(1, TILING_MAX_COUNT);
            recipe.grid.columns = columns;
            if u64::from(columns) * u64::from(recipe.grid.rows) > u64::from(TILING_MAX_COUNT) {
                recipe.grid.rows = (TILING_MAX_COUNT / columns).max(1);
            }
        }
        TilingLayout::Grid => recipe.circle.count = recipe.circle.count.min(TILING_MAX_COUNT),
    }
    recipe
}

impl Session {
    /// The state of the Tile slot of the Select bar (criteria 3 and 26).
    #[must_use]
    pub fn tile_button(&self) -> TileButton {
        if self.tool != Tool::Select {
            return TileButton::NothingSelected;
        }
        tile_button(&self.document, &self.units(), self.selection.ids())
    }

    /// The centre of the sources' box while the bar is open: the circle centre field shows
    /// document coordinates, the recipe keeps it from this point.
    #[must_use]
    pub fn tiling_box_centre(&self) -> Option<Point> {
        self.tiling.as_ref().map(|mode| mode.cell_box.centre())
    }

    /// Whether the tiling bar is open.
    #[must_use]
    pub fn tiling_open(&self) -> bool {
        self.tiling.is_some()
    }

    /// The recipe the bar shows, while it is open.
    #[must_use]
    pub fn tiling_recipe(&self) -> Option<TilingRecipe> {
        self.tiling.as_ref().map(|mode| mode.recipe)
    }

    /// Opens the bar for the selection (criteria 1 and 26): the defaults with `seed` (the
    /// frontend's fresh random value, criterion 19) for a new tiling, the stored recipe for a
    /// selected tiling. Returns false when nothing can be tiled or a source of the stored tiling
    /// was deleted (only Release is offered then, criterion 30).
    pub fn tiling_enter(&mut self, seed: u32) -> bool {
        let (tiling, sources, stored) = match self.tile_button() {
            TileButton::Tile(sources) => (None, sources, None),
            TileButton::Edit(group) => match self.document.tiling(group) {
                Some(stored) if stored.sources_live => {
                    (Some(group), stored.sources, Some(stored.recipe))
                }
                _ => return false,
            },
            TileButton::NothingSelected | TileButton::TilingWithOthers => return false,
        };
        let read = self.document.read_objects();
        let Some(cell_box) = cell_box(&self.document, &read, &sources) else {
            return false;
        };
        let laid = self.objects();
        let outlines = self
            .document
            .tiling_leaves(&sources)
            .iter()
            .filter_map(|leaf| {
                let object = laid
                    .iter()
                    .find(|o| o.id() == *leaf)
                    .or_else(|| read.get(*leaf))?;
                let opacity = self
                    .document
                    .node_registers(*leaf)
                    .map_or(1.0, |r| r.opacity.get());
                Some((preview_outline(object)?, opacity))
            })
            .collect();
        let hidden = tiling.map_or_else(HashSet::new, |group| self.managed_instances(group));
        let recipe = stored.unwrap_or_else(|| default_recipe(cell_box, seed));
        self.end_snapping_drag();
        self.tiling = Some(TilingMode {
            tiling,
            sources,
            recipe,
            cell_box,
            outlines,
            hidden,
            cells: Vec::new(),
            drag: None,
            entry: None,
            refusal: None,
        });
        self.refresh_tiling_cells();
        true
    }

    /// Every leaf under the managed instances of `group` (its children with a cell key): what a
    /// reopen hides. Unkeyed children and the kept source stay drawn (criterion 34).
    fn managed_instances(&self, group: NodeId) -> HashSet<NodeId> {
        let instances = self.document.tiling_instances(group);
        self.document
            .tiling_leaves(&instances)
            .into_iter()
            .collect()
    }

    fn refresh_tiling_cells(&mut self) {
        if let Some(mode) = self.tiling.as_mut() {
            mode.cells = match count_status(&mode.recipe) {
                CountStatus::Refused(_) => Vec::new(),
                _ => tiling_layout(mode.cell_box, &mode.recipe),
            };
        }
    }

    /// Takes `recipe` from the bar when it is valid (criterion 34: an invalid entry keeps the
    /// last valid preview). Returns whether it was taken.
    pub fn tiling_set_recipe(&mut self, recipe: TilingRecipe) -> bool {
        let mut recipe = fit_inactive_layout(recipe);
        recipe.circle.start = normalised_start(recipe.circle.start);
        let valid = recipe.check().is_ok()
            || matches!(count_status(&recipe), CountStatus::Refused(_))
                && recipe.check_stored() == Err(curvyo_document_core::RecipeError::TooMany);
        let Some(mode) = self.tiling.as_mut() else {
            return false;
        };
        if !valid {
            return false;
        }
        mode.recipe = recipe;
        mode.refusal = None;
        self.refresh_tiling_cells();
        true
    }

    /// Switches Gap and Pitch and converts the numbers so the layout does not move (criterion 7).
    pub fn tiling_set_spacing_mode(&mut self, mode_to: SpacingMode) -> bool {
        let Some(mode) = self.tiling.as_ref() else {
            return false;
        };
        let mut recipe = mode.recipe;
        let (x, y) = converted_spacing(mode.cell_box, &recipe.grid, mode_to);
        recipe.grid.spacing_x = x;
        recipe.grid.spacing_y = y;
        recipe.grid.spacing_mode = mode_to;
        self.tiling_set_recipe(recipe)
    }

    /// What the bar shows besides the recipe.
    #[must_use]
    pub fn tiling_status(&self) -> Option<TilingStatus> {
        let mode = self.tiling.as_ref()?;
        let read = self.document.read_objects();
        Some(TilingStatus {
            count: count_status(&mode.recipe),
            images_become_copies: images_become_copies(
                &self.document,
                &read,
                &mode.sources,
                &mode.recipe,
            ),
            refusal: mode.refusal.clone(),
            reopened: mode.tiling.is_some(),
        })
    }

    /// Apply or Enter (criteria 2 and 27): one commit "Tile" or "Edit tiling", the tiling
    /// selected and the bar closed; an unchanged reopened tiling writes nothing. A refusal keeps
    /// the bar open with the core's sentence. Returns whether the bar closed.
    pub fn tiling_apply(&mut self) -> bool {
        self.recording(|this| {
            let Some(mode) = this.tiling.clone() else {
                return false;
            };
            if matches!(count_status(&mode.recipe), CountStatus::Refused(_)) {
                return false;
            }
            // The session's cached read of this version: an Apply reads the document no
            // second time (`specs/0069-tiling` criterion 40).
            let read = this.object_read();
            let request = tiling_request(
                &this.document,
                &read,
                &mut this.minter,
                (mode.tiling, &mode.sources),
                &mode.recipe,
                mode.cell_box,
            );
            let minter = &mut this.minter;
            let mut mint = || minter.mint();
            match this.document.apply_tiling_with_read(
                &request,
                std::sync::Arc::clone(&read),
                &mut mint,
            ) {
                Ok(outcome) => {
                    this.tiling = None;
                    this.node.clear_selection();
                    let leaves = this.document.tiling_leaves(&[outcome.tiling]);
                    this.selection.set(&leaves);
                    this.hovered_object = None;
                    true
                }
                Err(error) => {
                    if let Some(open) = this.tiling.as_mut() {
                        open.refusal = Some(error.to_string());
                    }
                    false
                }
            }
        })
    }

    /// Cancel or Escape: drops the preview, writes nothing (criterion 2).
    pub fn tiling_cancel(&mut self) {
        self.tiling = None;
    }

    /// Release tiling (criterion 32): one step, the selection stays on the group.
    pub fn tiling_release(&mut self) -> bool {
        self.recording(|this| {
            let TileButton::Edit(group) = this.tile_button() else {
                return false;
            };
            this.tiling = None;
            this.document.release_tiling(group).is_ok()
        })
    }

    /// Whether `id` is drawn now: a managed instance of a reopened tiling is hidden under the
    /// preview (criterion 34).
    pub(super) fn tiling_hides(&self, id: NodeId) -> bool {
        self.tiling
            .as_ref()
            .is_some_and(|mode| mode.hidden.contains(&id))
    }

    /// The circle as the canvas draws it, while a circle tiling is open.
    pub(super) fn tiling_circle(&self) -> Option<CircleView> {
        let mode = self.tiling.as_ref()?;
        if mode.recipe.layout != TilingLayout::Circle {
            return None;
        }
        let circle = &mode.recipe.circle;
        Some(CircleView::of(
            circle,
            mode.cell_box.centre().translated(circle.centre),
            1.0 / self.view().scale(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, MovePlan, MoveTarget, NewAnchor};

    use super::*;

    fn path(session: &Session, x: f64, n: u64) -> NodeId {
        session.document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, n), Point::new(x, 0.0)),
                NewAnchor::corner(AnchorId::new(1, n + 1), Point::new(x + 10.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, n + 2), Point::new(x + 10.0, 5.0)),
            ],
            true,
        )
    }

    #[test]
    fn a_reopen_hides_only_the_managed_instances() {
        let mut session = Session::new(1);
        let source = path(&session, 0.0, 1);
        session.selection.set(&[source]);
        assert!(session.tiling_enter(3));
        let mut recipe = session.tiling_recipe().unwrap();
        recipe.grid.columns = 2;
        recipe.grid.rows = 1;
        assert!(session.tiling_set_recipe(recipe));
        assert!(session.tiling_apply());
        let TileButton::Edit(group) = session.tile_button() else {
            panic!("the tiling is selected")
        };
        let extra = path(&session, 100.0, 10);
        session
            .document
            .move_nodes(&MovePlan {
                nodes: vec![extra],
                target: MoveTarget::FrontOf(Some(group)),
            })
            .unwrap();
        let instances = session.document.tiling_instances(group);
        assert_eq!(instances.len(), 1);
        session
            .selection
            .set(&session.document.tiling_leaves(&[group]));
        assert!(session.tiling_enter(1));
        assert!(session.tiling_hides(instances[0]));
        assert!(!session.tiling_hides(source), "the kept source stays drawn");
        assert!(
            !session.tiling_hides(extra),
            "a hand-added child stays drawn"
        );
    }

    #[test]
    fn the_readout_of_a_radius_drag_uses_the_real_minus_only_for_negatives() {
        let mut session = Session::new(1);
        let source = path(&session, 0.0, 1);
        session.selection.set(&[source]);
        assert!(session.tiling_enter(3));
        let mut recipe = session.tiling_recipe().unwrap();
        recipe.layout = TilingLayout::Circle;
        assert!(session.tiling_set_recipe(recipe));
        assert!(session.tiling_readout().is_none());
        if let Some(mode) = session.tiling.as_mut() {
            mode.drag = Some(CircleHandle::Radius);
        }
        let readout = session.tiling_readout().unwrap();
        assert_eq!(readout.text, "r 15.0 mm");
    }
}
