//! The look of test grids on the output page (`specs/0028` criterion 12; UX notes §7.3, §11):
//! cells in their grid's kind look, labels in their label kind's, one chip row per grid instead
//! of one per cell, and the Worked chip on the marked cell in both views.

use std::collections::HashMap;

use curvyo_document_core::{NodeId, OperationSnapshot, Point, ViewTransform};
use curvyo_laser_core::settings::SetKind;
use curvyo_laser_core::test_grids::CellTag;
use curvyo_render_core::{ChipItem, DrawList, Glyph, OperationLook, worked_chip_list};

use super::super::Session;

/// One grid of the active laser page as drawn.
pub(in crate::session) struct GridChip {
    /// Its tagged items.
    leaves: Vec<NodeId>,
    /// The kinds of its chip row: the grid's kind, then the labels' when they are on.
    glyphs: Vec<Glyph>,
    /// The marked cell's item.
    worked: Option<NodeId>,
    /// Whether its row has Show on.
    shown: bool,
}

/// The grids of the active laser page: the look of each tagged item and the chips.
#[derive(Default)]
pub(in crate::session) struct GridLooks {
    /// The look of each tagged item of a grid with a readable recipe.
    pub(in crate::session) parts: HashMap<NodeId, OperationLook>,
    grids: Vec<GridChip>,
}

/// The look of a set kind (`0101` ux-notes section 3).
const fn look_of_set(kind: SetKind) -> OperationLook {
    match kind {
        SetKind::Cut => OperationLook::SolidLine,
        SetKind::Score => OperationLook::DashedLine,
        SetKind::Engrave => OperationLook::AreaFill,
    }
}

const fn glyph_of_set(kind: SetKind) -> Glyph {
    match kind {
        SetKind::Cut => Glyph::Cut,
        SetKind::Score => Glyph::Score,
        SetKind::Engrave => Glyph::Engrave,
    }
}

impl Session {
    /// The grids whose rows are among `operations`.
    pub(in crate::session) fn grid_looks(&self, operations: &[OperationSnapshot]) -> GridLooks {
        let mut looks = GridLooks::default();
        for row in operations.iter().filter(|op| op.grid) {
            let Some(group) = self.document.test_grid_of_row(row.id) else {
                continue;
            };
            let Some(grid) = self.read_test_grid(group) else {
                continue;
            };
            let Some(recipe) = grid.recipe else {
                continue;
            };
            let mut leaves = Vec::new();
            let mut worked = None;
            for item in &grid.snapshot.cells {
                if let Some(tag) = CellTag::from_value(&item.tag) {
                    looks.parts.insert(item.id, look_of_set(recipe.kind));
                    leaves.push(item.id);
                    if grid.mark == Some(tag.cell) {
                        worked = Some(item.id);
                    }
                }
            }
            for item in &grid.snapshot.labels {
                looks.parts.insert(item.id, look_of_set(recipe.labels.kind));
                leaves.push(item.id);
            }
            let mut glyphs = vec![glyph_of_set(recipe.kind)];
            if recipe.labels.on && !grid.snapshot.labels.is_empty() {
                glyphs.push(glyph_of_set(recipe.labels.kind));
            }
            glyphs.dedup();
            looks.grids.push(GridChip {
                leaves,
                glyphs,
                worked,
                shown: row.show,
            });
        }
        looks
    }
}

impl GridLooks {
    /// The chip row of each shown grid over the box of its items (`bounds` of a leaf, `None`
    /// when it is not drawn).
    pub(in crate::session) fn chips(
        &self,
        bounds: &dyn Fn(NodeId) -> Option<(Point, Point)>,
    ) -> Vec<ChipItem> {
        self.grids
            .iter()
            .filter(|grid| grid.shown)
            .filter_map(|grid| {
                let boxes: Vec<(Point, Point)> =
                    grid.leaves.iter().filter_map(|&id| bounds(id)).collect();
                let first = *boxes.first()?;
                let union = boxes.iter().fold(first, |(lo, hi), (min, max)| {
                    (
                        Point::new(lo.x.min(min.x), lo.y.min(min.y)),
                        Point::new(hi.x.max(max.x), hi.y.max(max.y)),
                    )
                });
                Some(ChipItem {
                    bounds: union,
                    glyphs: grid.glyphs.clone(),
                })
            })
            .collect()
    }

    /// The Worked chip of each shown grid with a mark, centred on the marked cell's square.
    pub(in crate::session) fn worked_chips(
        &self,
        bounds: &dyn Fn(NodeId) -> Option<(Point, Point)>,
        view: ViewTransform,
    ) -> DrawList {
        let mut list = DrawList::default();
        for grid in self.grids.iter().filter(|grid| grid.shown) {
            if let Some((min, max)) = grid.worked.and_then(bounds) {
                let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
                list.extend(worked_chip_list(centre, view));
            }
        }
        list
    }
}
