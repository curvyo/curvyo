//! The chip row of test grids on the output page (`specs/0112` UX 3.3, the `0028` 11 rule kept):
//! one chip row on each grid's box with its kinds instead of one per cell or label.

use std::collections::HashMap;

use curvyo_document_core::{NodeId, OperationSnapshot, Point};
use curvyo_laser_core::settings::SetKind;
use curvyo_render_core::{ChipItem, Glyph, OperationLook};

use super::super::Session;

/// One grid of the active laser page as drawn.
pub(in crate::session) struct GridChip {
    /// Its tagged items.
    leaves: Vec<NodeId>,
    /// The kinds of its chip row: the cells' kind, then Engrave for the labels.
    glyphs: Vec<Glyph>,
    /// Whether one of its rows has Show on.
    shown: bool,
}

/// The grids of the active laser page: the tagged items (no chip of their own) and the chips.
#[derive(Default)]
pub(in crate::session) struct GridLooks {
    /// The tagged items of a grid with a readable recipe, which take no chip of their own; an
    /// old folded grid row (format 43/44, inert) runs nothing here.
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
    /// The grids whose derived rows are among `operations`.
    pub(in crate::session) fn grid_looks(&self, operations: &[OperationSnapshot]) -> GridLooks {
        let mut looks = GridLooks::default();
        let mut seen: Vec<NodeId> = Vec::new();
        for row in operations.iter().filter(|op| op.grid_part.is_some()) {
            let Some(group) = self.document.test_grid_of_derived_row(row.id) else {
                continue;
            };
            let shown = operations.iter().any(|op| {
                op.show
                    && op.grid_part.is_some()
                    && self.document.test_grid_of_derived_row(op.id) == Some(group)
            });
            if seen.contains(&group) {
                continue;
            }
            seen.push(group);
            let Some(grid) = self.grid_object(group) else {
                continue;
            };
            let kind = grid.recipe.test.kind();
            let mut leaves = Vec::new();
            for item in &grid.cells {
                looks.parts.insert(item.id, look_of_set(kind));
                leaves.push(item.id);
            }
            for item in &grid.labels {
                looks.parts.insert(item.id, look_of_set(SetKind::Engrave));
                leaves.push(item.id);
            }
            let mut glyphs = vec![glyph_of_set(kind)];
            if !grid.labels.is_empty() {
                glyphs.push(glyph_of_set(SetKind::Engrave));
            }
            glyphs.dedup();
            looks.grids.push(GridChip {
                leaves,
                glyphs,
                shown,
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
}
