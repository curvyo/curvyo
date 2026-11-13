//! The look of the output page's items (`specs/0101-laser-operations` criteria 14 to 16, 18;
//! ux-notes sections 13.2 and 13.3). Under "View: Operations" each leaf of the active laser page is
//! drawn in the looks of the operations that hold it, in list order; an item with no operation, or
//! with an Output off one, is in the Not assigned look; an item held only by Show-off operations is
//! not drawn. The kind decides the look (ADR 0046 decision 8): there is no colour per operation.
//! The hatch of the Engrave kinds and the symbol chips are drawn over the artwork. With the page's
//! Operations view toggle unpressed (`specs/0111` criterion 15) every item keeps its own look and
//! no symbols are drawn.

use std::collections::{HashMap, HashSet};

use curvyo_document_core::{NodeId, ObjectSnapshot, OperationSnapshot, Point, ViewTransform};
use curvyo_laser_core::operations::{ChainScope, Hidden, KindRead, OperationKind, OperationList};
use curvyo_render_core::{
    CONTEXT_DIM, ChipItem, DrawList, Glyph, OperationLook, chips_list, dim_outside, hatch_list,
    object_polygons, restyled,
};

use super::super::Session;
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

/// What the chip overlay of a frame needs: the items, the looks, the chips in drawing order of the
/// items inside and the ones outside an entered group.
struct ChipRows<'a> {
    objects: &'a [ObjectSnapshot],
    grids: &'a GridLooks,
    table: &'a LookTable,
    chips: Vec<(bool, ChipItem)>,
    outside: Vec<(bool, ChipItem)>,
    operations_view: bool,
}

impl ChipRows<'_> {
    /// Adds the chip of item `id`: drawing order is selected items first, then the rest top first
    /// (ux-notes 13.4); an item outside an entered group goes to the dimmed ones.
    fn add(&mut self, id: NodeId, chip: ChipItem, selected: bool, dimming: &Dimming) {
        if dimming.outside(id) {
            self.outside.push((false, chip));
        } else {
            self.chips.push((selected, chip));
        }
    }
}

/// `list` with every colour at [`CONTEXT_DIM`] of its alpha: what lies outside an entered group.
fn dimmed_list(mut list: DrawList) -> DrawList {
    for vertex in &mut list.triangles {
        // invariant: the product of a byte and a fraction in 0..=1 is a byte.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let alpha = (f64::from(vertex.color.a) * CONTEXT_DIM).round() as u8;
        vertex.color.a = alpha;
    }
    list
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

    /// Whether the active laser page's leaves are drawn by their looks: its Operations view
    /// toggle is pressed (`specs/0111` criterion 15).
    pub(in crate::session) fn operations_view_on(&self) -> bool {
        match self.workbench() {
            super::super::Workbench::Manufacturing(id) => self.operations_view_pressed(id),
            _ => true,
        }
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
        let dimming = self.entered_group_dimming();
        let mut rows = ChipRows {
            objects,
            grids: &grids,
            table: &table,
            chips: Vec::new(),
            outside: Vec::new(),
            operations_view,
        };
        let selected = self.selection.ids();
        for object in objects {
            if let ObjectSnapshot::Clone(clone) = object
                && clone.chain.picture.is_some()
            {
                out.extend(picture_clone_leaves(
                    clone,
                    &table,
                    view,
                    operations_view,
                    &dimming,
                ));
                continue;
            }
            let Some(look) = table.own.get(&object.id()) else {
                out.push(object.clone());
                continue;
            };
            if !operations_view {
                out.extend((!matches!(look, LeafLook::Hidden)).then(|| object.clone()));
                continue;
            }
            out.extend(dimming.dim(object.id(), drawn_as(object, look, view)));
            if let LeafLook::Shown(looks) = look {
                if let Some(hatch) = hatch_overlay(object, looks, view, &dimming) {
                    overlay.extend(hatch);
                }
                // A grid's items share one chip row over the grid (UX notes §11).
                if grids.parts.contains_key(&object.id()) {
                    continue;
                }
                if let Some(chip) = chip_of(object, looks) {
                    rows.add(object.id(), chip, selected.contains(&object.id()), &dimming);
                }
            }
        }
        self.chips_overlay(&mut overlay, rows, view);
        Some((out, overlay))
    }

    /// In a read-only entered group everything outside recedes: the looks restyle the already
    /// dimmed objects, so the dimming is applied again to the looked copies and the overlay.
    fn entered_group_dimming(&self) -> Dimming {
        Dimming(
            self.in_read_only_group()
                .then(|| self.objects().iter().map(ObjectSnapshot::id).collect()),
        )
    }

    /// The symbol chips of the looked items, the dimmed ones of the items outside an entered
    /// group, and the grids' chips, drawn into `overlay`.
    fn chips_overlay(&self, overlay: &mut DrawList, rows: ChipRows<'_>, view: ViewTransform) {
        let ChipRows {
            objects,
            grids,
            table,
            mut chips,
            outside: mut outside_chips,
            operations_view,
        } = rows;
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
            if !outside_chips.is_empty() {
                outside_chips.reverse();
                let items: Vec<ChipItem> =
                    outside_chips.into_iter().map(|(_, chip)| chip).collect();
                overlay.extend(dimmed_list(chips_list(
                    &items,
                    self.viewport.canvas_size(),
                    view,
                )));
            }
        }
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

/// The entered laser group's members, when one is entered: everything else recedes.
struct Dimming(Option<HashSet<NodeId>>);

impl Dimming {
    fn outside(&self, id: NodeId) -> bool {
        self.0.as_ref().is_some_and(|inside| !inside.contains(&id))
    }

    fn dim(&self, id: NodeId, looked: Vec<ObjectSnapshot>) -> Vec<ObjectSnapshot> {
        if self.outside(id) {
            dim_outside(&looked, &|_| false)
        } else {
            looked
        }
    }
}

/// The looked leaves of a clone with a picture; in the Drawn view only the Show-off leaves are
/// dropped, every other leaf keeps its own look.
fn picture_clone_leaves(
    clone: &curvyo_document_core::CloneSnapshot,
    table: &LookTable,
    view: ViewTransform,
    operations_view: bool,
    dimming: &Dimming,
) -> Vec<ObjectSnapshot> {
    let looked = dimming.dim(clone.resolved.id, picture_looked(clone, table, view));
    if operations_view {
        return looked;
    }
    looked
        .into_iter()
        .filter(|leaf| {
            table
                .clone_members
                .get(&(clone.resolved.id, leaf.id()))
                .or_else(|| table.own.get(&clone.resolved.id))
                .is_none_or(|look| !matches!(look, LeafLook::Hidden))
        })
        .collect()
}

/// The hatch of `object` when one of its looks is hatched, dimmed when it is outside the group.
fn hatch_overlay(
    object: &ObjectSnapshot,
    looks: &[OperationLook],
    view: ViewTransform,
    dimming: &Dimming,
) -> Option<DrawList> {
    let polygons = object_polygons(object).filter(|_| looks.iter().any(|&l| hatched(l)))?;
    let hatch = hatch_list(&polygons, view);
    Some(if dimming.outside(object.id()) {
        dimmed_list(hatch)
    } else {
        hatch
    })
}

/// The symbol chip of `object`, `None` when none of its looks has a glyph.
fn chip_of(object: &ObjectSnapshot, looks: &[OperationLook]) -> Option<ChipItem> {
    let mut glyphs: Vec<Glyph> = looks.iter().filter_map(|&l| glyph_of(l)).collect();
    glyphs.sort_by_key(|g| *g as u8);
    glyphs.dedup();
    (!glyphs.is_empty()).then(|| ChipItem {
        bounds: curvyo_ui_core::object_bounds(object),
        glyphs,
    })
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
