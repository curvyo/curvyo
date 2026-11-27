//! The look of the output page's items (`specs/0101-laser-operations` criteria 14 to 16, 18;
//! ux-notes sections 13.2 and 13.3). Under "View: Operations" each leaf of the active laser page is
//! drawn in the looks of the operations that hold it, in list order; an item with no operation, or
//! with an Output off one, is in the Not assigned look; an item held only by Show-off operations is
//! not drawn. The kind decides the look (ADR 0046 decision 8): there is no colour per operation.
//! The hatch of the Engrave kinds and the symbol chips are drawn over the artwork. With the page's
//! Operations view toggle unpressed (`specs/0111` criterion 15) every item keeps its own look and
//! no symbols are drawn.

use std::collections::{HashMap, HashSet};

use curvyo_document_core::{NodeId, ObjectSnapshot, Opacity, OperationSnapshot, ViewTransform};
use curvyo_laser_core::operations::{
    ChainScope, Hidden, KindRead, LeafShape, OperationKind, OperationList, kind_run,
};
use curvyo_render_core::{CONTEXT_DIM, DrawList, Glyph, OperationLook, dim_outside, restyled};

use super::super::Session;
use super::grid_look::GridLooks;
use super::index::IndexedLeaf;
use super::look_overlay::{ChipRows, own_overlay};
use super::look_picture::{picture_clone_leaves, picture_overlay};

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
    Shown(Vec<KindLook>),
}

/// One operation's look of a leaf and the symbol its chip shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::session) struct KindLook {
    /// How the leaf is drawn for the operation.
    pub(in crate::session) look: OperationLook,
    /// The chip's symbol and whether it is muted: the kind holds the leaf but does not run it
    /// (Output off, `0101` UX notes 13.2; a vector shape in Image, `specs/0123` criterion 48).
    pub(in crate::session) symbol: Option<(Glyph, bool)>,
}

impl KindLook {
    /// The look `look` with its own symbol, in ink.
    fn running(look: OperationLook) -> Self {
        Self {
            look,
            symbol: glyph_of(look).map(|glyph| (glyph, false)),
        }
    }

    /// The Not assigned look with the symbol of `kind_look` muted.
    fn muted(kind_look: OperationLook) -> Self {
        Self {
            look: OperationLook::MutedDashed,
            symbol: glyph_of(kind_look).map(|glyph| (glyph, true)),
        }
    }
}

/// The looks of the active laser page's leaves: its own items by their node, placed-page members
/// by the pair of member and placement (criterion 11), clone members by their clone and member.
#[derive(Debug)]
pub(in crate::session) struct LookTable {
    /// The laser page the looks belong to.
    pub(in crate::session) manufacturing: NodeId,
    pub(super) own: HashMap<NodeId, LeafLook>,
    inner: HashMap<(NodeId, NodeId), LeafLook>,
    pub(super) clone_members: HashMap<(NodeId, NodeId), LeafLook>,
}

/// The look of an operation's kind (ux-notes section 3); Unknown is the Not assigned look.
fn look_of(kind: KindRead) -> OperationLook {
    match kind.known() {
        Some(OperationKind::Cut) => OperationLook::SolidLine,
        Some(OperationKind::Score) => OperationLook::DashedLine,
        Some(OperationKind::Engrave) => OperationLook::AreaFill,
        Some(OperationKind::Image) => OperationLook::Dots,
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
        OperationLook::Dots => Some(Glyph::Image),
        OperationLook::MutedDashed => None,
    }
}

/// Whether a look hatches the closed areas of its object.
pub(super) fn hatched(look: OperationLook) -> bool {
    matches!(look, OperationLook::AreaFill | OperationLook::AreaWithLine)
}

/// The look of one leaf: the operations holding it, in stored order, with Show on. A grid row
/// gives its tagged items their part's look (`grid_parts`, `specs/0028` UX notes §11); any other
/// item under it runs nothing and is Not assigned. A leaf an Image operation does not burn (no
/// picture, `specs/0123` criterion 48) takes the muted look there.
fn leaf_look(
    list: &OperationList,
    operations: &[OperationSnapshot],
    (chain, hidden, shape): (&[Vec<NodeId>], Hidden, LeafShape),
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
    let looks: Vec<KindLook> = positions
        .into_iter()
        .filter(|at| runs(at) && operations[*at].show)
        .map(|at| {
            let op = &operations[at];
            let kind = KindRead::of(op.kind.as_deref());
            if !op.output {
                KindLook::muted(look_of(kind))
            } else if op.grid {
                KindLook::running(
                    leaf.and_then(|leaf| grid_parts.get(&leaf).copied())
                        .unwrap_or(OperationLook::MutedDashed),
                )
            } else if kind.known() == Some(OperationKind::Image)
                && kind_run(OperationKind::Image, shape).is_empty()
            {
                KindLook::muted(OperationLook::Dots)
            } else {
                KindLook::running(look_of(kind))
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
pub(super) fn drawn_as(
    object: &ObjectSnapshot,
    look: &LeafLook,
    view: ViewTransform,
) -> Vec<ObjectSnapshot> {
    let muted =
        || restyled(object, OperationLook::MutedDashed, view).unwrap_or_else(|| object.clone());
    match look {
        LeafLook::Hidden => Vec::new(),
        // A Not assigned image keeps its picture, dimmed, under the dashed frame (`specs/0123`
        // UX notes 3.1).
        LeafLook::Own => match object {
            ObjectSnapshot::Image(image) => {
                let mut dimmed = image.clone();
                dimmed.shown_opacity = Opacity::new(CONTEXT_DIM).unwrap_or(Opacity::OPAQUE);
                vec![ObjectSnapshot::Image(dimmed), muted()]
            }
            _ => vec![muted()],
        },
        LeafLook::Shown(looks) => looks
            .iter()
            .map(|look| restyled(object, look.look, view).unwrap_or_else(|| object.clone()))
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
            let look = leaf_look(
                &list,
                operations,
                (&leaf.chain, leaf.hidden, leaf.shape),
                grid_parts,
            );
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
            // The Toolpath view releases it while pressed (`specs/0029` UX P2).
            super::super::Workbench::Manufacturing(id) => {
                self.operations_view_pressed(id) && !self.toolpath_view_pressed(id)
            }
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
        if let Some(mut toolpath) = self.toolpath_overlay() {
            toolpath.extend(self.frame_head_overlay(view).unwrap_or_default());
            return Some((self.toolpath_artwork(objects, &table), toolpath));
        }
        let operations_view = self.operations_view_on();
        let dimming = self.entered_group_dimming();
        let mut rows = ChipRows::new(objects, &grids, &table, operations_view);
        let mut out = Vec::with_capacity(objects.len());
        let mut overlay = DrawList::default();
        let selected = self.selection.ids();
        // Raster items of Image operations show their dots (`specs/0123` criterion 20); the Dots
        // look draws no hatch, an Engrave of the same item still hatches it.
        let dots = if operations_view {
            self.preview_raster_objects(view)
        } else {
            Vec::new()
        };
        for object in objects {
            if let ObjectSnapshot::Clone(clone) = object
                && clone.is_picture()
            {
                out.extend(picture_clone_leaves(clone, &rows, view, &dimming));
                if operations_view {
                    picture_overlay(clone, &mut rows, &mut overlay, view, &dimming, selected);
                }
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
                own_overlay(
                    object,
                    looks,
                    &mut rows,
                    &mut overlay,
                    view,
                    &dimming,
                    selected,
                );
            }
        }
        out.extend(dots);
        if operations_view {
            overlay.extend(self.tab_marks_overlay(view));
        }
        self.chips_overlay(&mut overlay, rows, view);
        overlay.extend(self.frame_head_overlay(view).unwrap_or_default());
        Some((out, overlay))
    }

    /// What the Toolpath view keeps of the artwork (`without_looked_items`) and its dot pictures;
    /// while the job burns elsewhere (User origin), the items stay at their own place at 35 %
    /// (`specs/0118` W48).
    fn toolpath_artwork(
        &self,
        objects: &[ObjectSnapshot],
        table: &LookTable,
    ) -> Vec<ObjectSnapshot> {
        let job = self.laser_job();
        let mut kept = if job
            .as_ref()
            .is_some_and(|job| crate::session::output::shifted(&job.built.job))
        {
            let looked = looked_ids(table);
            dim_outside(objects, &|id| !looked.contains(&id))
        } else {
            without_looked_items(objects, table)
        };
        if let Some(job) = job {
            kept.extend(self.toolpath_raster_objects(&job));
        }
        kept
    }

    /// In a read-only entered group everything outside recedes: the looks restyle the already
    /// dimmed objects, so the dimming is applied again to the looked copies and the overlay.
    fn entered_group_dimming(&self) -> Dimming {
        Dimming(
            self.in_read_only_group()
                .then(|| self.objects().iter().map(ObjectSnapshot::id).collect()),
        )
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
        if self.toolpath_view_on() {
            // The Toolpath view draws the job instead (`specs/0029` criterion 8); while it burns
            // elsewhere the members stay at their place at 35 % (`specs/0118` W48).
            let looked =
                |object: &ObjectSnapshot| table.inner.contains_key(&(object.id(), placement));
            if self
                .laser_job()
                .is_some_and(|job| crate::session::output::shifted(&job.built.job))
            {
                let ids: HashSet<NodeId> = members
                    .iter()
                    .filter(|o| looked(o))
                    .map(ObjectSnapshot::id)
                    .collect();
                return dim_outside(members, &|id| !ids.contains(&id));
            }
            return members.iter().filter(|o| !looked(o)).cloned().collect();
        }
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
pub(super) struct Dimming(Option<HashSet<NodeId>>);

impl Dimming {
    pub(super) fn outside(&self, id: NodeId) -> bool {
        self.0.as_ref().is_some_and(|inside| !inside.contains(&id))
    }

    pub(super) fn dim(&self, id: NodeId, looked: Vec<ObjectSnapshot>) -> Vec<ObjectSnapshot> {
        if self.outside(id) {
            dim_outside(&looked, &|_| false)
        } else {
            looked
        }
    }
}

/// The objects the Toolpath view keeps: it draws the job instead of the page's items
/// (`specs/0029` criterion 8), a clone of a group by its members' keys; hit tests still use the
/// items.
fn without_looked_items(objects: &[ObjectSnapshot], table: &LookTable) -> Vec<ObjectSnapshot> {
    let looked = looked_ids(table);
    objects
        .iter()
        .filter(|object| !looked.contains(&object.id()))
        .cloned()
        .collect()
}

/// The objects the table looks by: its own leaves and the clones of groups with looked members.
fn looked_ids(table: &LookTable) -> HashSet<NodeId> {
    table
        .own
        .keys()
        .copied()
        .chain(table.clone_members.keys().map(|(c, _)| *c))
        .collect()
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
