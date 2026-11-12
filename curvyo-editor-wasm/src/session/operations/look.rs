//! The look of the output page's items (`specs/0101-laser-operations` criteria 14 to 16, 18;
//! ux-notes sections 13.2 and 13.3). Under "View: Operations" each leaf of the active laser page is
//! drawn in the looks of the operations that hold it, in list order; an item with no operation, or
//! with an Output off one, is in the Not assigned look; an item held only by Show-off operations is
//! not drawn. The kind decides the look (ADR 0046 decision 8): there is no colour per operation.
//! The hatch of the Engrave kinds and the symbol chips are drawn over the artwork. "View: Drawn"
//! keeps every item's own look and draws no symbols.

use std::collections::HashMap;

use curvyo_document_core::{NodeId, ObjectSnapshot, OperationSnapshot, Point, ViewTransform};
use curvyo_laser_core::operations::{ChainScope, Hidden, KindRead, OperationKind, OperationList};
use curvyo_render_core::{
    ChipItem, DrawList, Glyph, OperationLook, chips_list, hatch_list, object_polygons, restyled,
};

use super::super::Session;
use super::OperationsViewMode;
use super::grid_look::GridLooks;
use super::index::IndexedLeaf;

/// What a leaf is drawn as on the output page.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::session) enum LeafLook {
    /// No operation holds it: the Not assigned look under "View: Operations", its own look under
    /// "View: Drawn".
    Own,
    /// Hidden on the output page, or every operation holding it has Show off: not drawn.
    Hidden,
    /// Held by operations with Show on: one look per operation, in list order (Output off is the
    /// Not assigned look).
    Shown(Vec<OperationLook>),
}

/// The looks of the active laser page's leaves: its own items by their node, placed-page members
/// by the pair of member and placement (criterion 11), clone members by their clone and member.
#[derive(Debug)]
pub(in crate::session) struct LookTable {
    /// The laser page the looks belong to.
    pub(in crate::session) manufacturing: NodeId,
    own: HashMap<NodeId, LeafLook>,
    inner: HashMap<(NodeId, NodeId), LeafLook>,
    clone_members: HashMap<(NodeId, NodeId), LeafLook>,
}

/// The look of an operation's kind (ux-notes section 3); Unknown is the Not assigned look.
fn look_of(kind: KindRead) -> OperationLook {
    match kind.known() {
        Some(OperationKind::Cut) => OperationLook::SolidLine,
        Some(OperationKind::Score) => OperationLook::DashedLine,
        Some(OperationKind::Engrave) => OperationLook::AreaFill,
        Some(OperationKind::EngraveOutline) => OperationLook::AreaWithLine,
        Some(OperationKind::CutOut) => OperationLook::OuterLine,
        None => OperationLook::MutedDashed,
    }
}

/// The symbol of a look (ux-notes section 3); the Not assigned look has none.
fn glyph_of(look: OperationLook) -> Option<Glyph> {
    match look {
        OperationLook::SolidLine => Some(Glyph::Cut),
        OperationLook::OuterLine => Some(Glyph::CutOut),
        OperationLook::DashedLine => Some(Glyph::Score),
        OperationLook::AreaFill => Some(Glyph::Engrave),
        OperationLook::AreaWithLine => Some(Glyph::EngraveOutline),
        OperationLook::MutedDashed => None,
    }
}

/// Whether a look hatches the closed areas of its object.
fn hatched(look: OperationLook) -> bool {
    matches!(look, OperationLook::AreaFill | OperationLook::AreaWithLine)
}

/// The look of one leaf: the operations holding it, in stored order, with Show on. A grid row
/// gives its tagged items their part's look (`grid_parts`, `specs/0028` UX notes §11); any other
/// item under it runs nothing and is Not assigned.
fn leaf_look(
    list: &OperationList,
    operations: &[OperationSnapshot],
    chain: &[Vec<NodeId>],
    hidden: Hidden,
    grid_parts: &HashMap<NodeId, OperationLook>,
) -> LeafLook {
    if hidden != Hidden::No {
        return LeafLook::Hidden;
    }
    let Some(assignment) = list.nearest(chain, ChainScope::Whole) else {
        return LeafLook::Own;
    };
    let leaf = chain.first().and_then(|first| first.first()).copied();
    let mut positions = assignment.operations;
    positions.sort_unstable();
    let runs = |at: &usize| {
        let op = &operations[*at];
        !op.grid || leaf.is_some_and(|leaf| grid_parts.contains_key(&leaf))
    };
    if !positions.iter().any(runs) {
        return LeafLook::Own;
    }
    let looks: Vec<OperationLook> = positions
        .into_iter()
        .filter(|at| runs(at) && operations[*at].show)
        .map(|at| {
            let op = &operations[at];
            if !op.output {
                OperationLook::MutedDashed
            } else if op.grid {
                leaf.and_then(|leaf| grid_parts.get(&leaf).copied())
                    .unwrap_or(OperationLook::MutedDashed)
            } else {
                look_of(KindRead::of(op.kind.as_deref()))
            }
        })
        .collect();
    if looks.is_empty() {
        LeafLook::Hidden
    } else {
        LeafLook::Shown(looks)
    }
}

/// The copies of `object` to draw for its look under "View: Operations" (criterion 18).
fn drawn_as(object: &ObjectSnapshot, look: &LeafLook, view: ViewTransform) -> Vec<ObjectSnapshot> {
    let muted =
        || restyled(object, OperationLook::MutedDashed, view).unwrap_or_else(|| object.clone());
    match look {
        LeafLook::Hidden => Vec::new(),
        LeafLook::Own => vec![muted()],
        LeafLook::Shown(looks) => looks
            .iter()
            .map(|&look| restyled(object, look, view).unwrap_or_else(|| object.clone()))
            .collect(),
    }
}

impl LookTable {
    /// The table for the leaves of `manufacturing`'s output page.
    fn build(
        leaves: &[IndexedLeaf],
        operations: &[OperationSnapshot],
        cuts_last: bool,
        placements: &[NodeId],
        manufacturing: NodeId,
        grid_parts: &HashMap<NodeId, OperationLook>,
    ) -> Self {
        let list = OperationList::new(operations, cuts_last);
        let mut table = Self {
            manufacturing,
            own: HashMap::new(),
            inner: HashMap::new(),
            clone_members: HashMap::new(),
        };
        for leaf in leaves {
            let look = leaf_look(&list, operations, &leaf.chain, leaf.hidden, grid_parts);
            let (Some(first), Some(tail)) = (leaf.chain.first(), leaf.chain.last()) else {
                continue;
            };
            // A clone member's key is its path, outermost clone first (criterion 11).
            if first.len() >= 2 {
                table
                    .clone_members
                    .insert((first[0], first[first.len() - 1]), look);
                continue;
            }
            let member = first[0];
            match tail.as_slice() {
                [placement] if leaf.chain.len() >= 2 && placements.contains(placement) => {
                    table.inner.insert((member, *placement), look);
                }
                _ => {
                    table.own.insert(member, look);
                }
            }
        }
        table
    }
}

/// The leaves a clone of a group, text or image draws (its picture), each in its look: a member of
/// the clone by its path, or the clone's own look for a clone of a text or an image.
fn picture_looked(
    clone: &curvyo_document_core::CloneSnapshot,
    table: &LookTable,
    view: ViewTransform,
) -> Vec<ObjectSnapshot> {
    let Some(picture) = clone.chain.picture.as_ref() else {
        return Vec::new();
    };
    let own = clone.resolved.id;
    picture
        .iter()
        .flat_map(|leaf| {
            let look = table
                .clone_members
                .get(&(own, leaf.id()))
                .or_else(|| table.own.get(&own));
            match look {
                Some(look) => drawn_as(leaf, look, view),
                None => vec![leaf.clone()],
            }
        })
        .collect()
}

impl Session {
    /// The looks of the active laser page's leaves, `None` on another page or while its extension
    /// is switched off (criterion 28).
    pub(in crate::session) fn operation_look_table(&self) -> Option<LookTable> {
        let active = self.active_operations()?;
        let grids = self.grid_looks(&active.operations);
        self.look_table_with(&active, &grids)
    }

    fn look_table_with(
        &self,
        active: &super::ActiveOperations,
        grids: &GridLooks,
    ) -> Option<LookTable> {
        let manufacturing = self.active_manufacturing()?;
        let leaves = self.output_leaves(&manufacturing);
        let placements: Vec<NodeId> = manufacturing.placements.iter().map(|p| p.id).collect();
        Some(LookTable::build(
            &leaves,
            &active.operations,
            active.cuts_last,
            &placements,
            manufacturing.id,
            &grids.parts,
        ))
    }

    /// Whether the output page's leaves are drawn by their looks (criterion 18).
    pub(in crate::session) fn operations_view_on(&self) -> bool {
        self.operations.view_mode == OperationsViewMode::Operations
    }

    /// The artwork objects with the looks of the active laser page applied, and the overlay of the
    /// hatch and the symbol chips (View: Operations only). `None` when the page has no operations to
    /// look by, so the drawing is left as it is. Show-off items are dropped in both views (criterion
    /// 15).
    pub(in crate::session) fn operation_looked_artwork(
        &self,
        objects: &[ObjectSnapshot],
        view: ViewTransform,
    ) -> Option<(Vec<ObjectSnapshot>, DrawList)> {
        let active = self.active_operations()?;
        let grids = self.grid_looks(&active.operations);
        let table = self.look_table_with(&active, &grids)?;
        let operations_view = self.operations_view_on();
        let mut out = Vec::with_capacity(objects.len());
        let mut overlay = DrawList::default();
        // Chips in drawing order: selected items first, then the rest top first (ux-notes 13.4).
        let mut chips: Vec<(bool, ChipItem)> = Vec::new();
        let selected = self.selection.ids();
        for object in objects {
            if let ObjectSnapshot::Clone(clone) = object
                && clone.chain.picture.is_some()
            {
                let looked = picture_looked(clone, &table, view);
                if operations_view {
                    out.extend(looked);
                } else {
                    // Drawn: only the Show-off leaves are dropped, every other leaf keeps its own look.
                    out.extend(looked.into_iter().filter(|leaf| {
                        table
                            .clone_members
                            .get(&(clone.resolved.id, leaf.id()))
                            .or_else(|| table.own.get(&clone.resolved.id))
                            .is_none_or(|look| !matches!(look, LeafLook::Hidden))
                    }));
                }
                continue;
            }
            let look = table.own.get(&object.id());
            let Some(look) = look else {
                out.push(object.clone());
                continue;
            };
            if !operations_view {
                if !matches!(look, LeafLook::Hidden) {
                    out.push(object.clone());
                }
                continue;
            }
            out.extend(drawn_as(object, look, view));
            if let LeafLook::Shown(looks) = look {
                if let Some(polygons) =
                    object_polygons(object).filter(|_| looks.iter().any(|&l| hatched(l)))
                {
                    overlay.extend(hatch_list(&polygons, view));
                }
                // A grid's items share one chip row over the grid (UX notes §11).
                if grids.parts.contains_key(&object.id()) {
                    continue;
                }
                let mut glyphs: Vec<Glyph> = looks.iter().filter_map(|&l| glyph_of(l)).collect();
                glyphs.sort_by_key(|g| *g as u8);
                glyphs.dedup();
                if !glyphs.is_empty() {
                    let (min, max) = curvyo_ui_core::object_bounds(object);
                    let chip = ChipItem {
                        bounds: (min, max),
                        glyphs,
                    };
                    chips.push((selected.contains(&object.id()), chip));
                }
            }
        }
        let drawn: HashMap<NodeId, (Point, Point)> = objects
            .iter()
            .filter(|object| grids.parts.contains_key(&object.id()))
            .filter(|object| {
                table
                    .own
                    .get(&object.id())
                    .is_none_or(|look| !matches!(look, LeafLook::Hidden))
            })
            .map(|object| (object.id(), curvyo_ui_core::object_bounds(object)))
            .collect();
        let bounds = |id: NodeId| drawn.get(&id).copied();
        if operations_view {
            chips.reverse();
            chips.sort_by_key(|(is_selected, _)| !is_selected);
            let mut items: Vec<ChipItem> = grids.chips(&bounds);
            items.extend(chips.into_iter().map(|(_, chip)| chip));
            overlay.extend(chips_list(&items, self.viewport.canvas_size(), view));
        }
        // The Worked chip is a record of the test: drawn in both views (UX notes §7.3).
        overlay.extend(grids.worked_chips(&bounds, view));
        Some((out, overlay))
    }

    /// The members of one placed page as drawn on the output page: each member is in its look for
    /// this placement (criterion 11, 14, 15). Objects not in the table keep their look.
    pub(in crate::session) fn looked_placed_members(
        &self,
        table: &LookTable,
        placement: NodeId,
        members: &[ObjectSnapshot],
        view: ViewTransform,
    ) -> Vec<ObjectSnapshot> {
        let operations_view = self.operations_view_on();
        let mut out = Vec::with_capacity(members.len());
        for object in members {
            match table.inner.get(&(object.id(), placement)) {
                Some(look) if operations_view => out.extend(drawn_as(object, look, view)),
                Some(look) if !matches!(look, LeafLook::Hidden) => out.push(object.clone()),
                Some(_) => {}
                None => out.push(object.clone()),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_look_has_its_symbol_except_the_not_assigned_one() {
        assert_eq!(glyph_of(OperationLook::SolidLine), Some(Glyph::Cut));
        assert_eq!(glyph_of(OperationLook::OuterLine), Some(Glyph::CutOut));
        assert_eq!(glyph_of(OperationLook::MutedDashed), None);
    }

    #[test]
    fn only_the_engrave_kinds_are_hatched() {
        assert!(hatched(OperationLook::AreaFill));
        assert!(hatched(OperationLook::AreaWithLine));
        assert!(!hatched(OperationLook::SolidLine));
    }
}
