//! What is drawn over the looked artwork of the output page (`specs/0101` ux-notes 13.3, 13.4):
//! the hatch of the Engrave kinds and the symbol chips, ink for a kind that runs an item, muted
//! for one that holds it without running it.

use std::collections::HashMap;

use curvyo_document_core::{NodeId, ObjectSnapshot, Point, ViewTransform};
use curvyo_render_core::{
    CONTEXT_DIM, ChipItem, DrawList, Glyph, chips_list, chips_list_clear_of, hatch_list,
    object_polygons,
};

use super::super::Session;
use super::grid_look::GridLooks;
use super::look::{Dimming, KindLook, LeafLook, LookTable, hatched};

/// What the chip overlay of a frame needs: the items, the looks, the chips in drawing order of the
/// items inside and the ones outside an entered group.
pub(super) struct ChipRows<'a> {
    objects: &'a [ObjectSnapshot],
    grids: &'a GridLooks,
    pub(super) table: &'a LookTable,
    chips: Vec<(bool, ChipItem)>,
    outside: Vec<(bool, ChipItem)>,
    pub(super) operations_view: bool,
}

impl<'a> ChipRows<'a> {
    /// No chips yet for the frame of `objects`.
    pub(super) const fn new(
        objects: &'a [ObjectSnapshot],
        grids: &'a GridLooks,
        table: &'a LookTable,
        operations_view: bool,
    ) -> Self {
        Self {
            objects,
            grids,
            table,
            chips: Vec::new(),
            outside: Vec::new(),
            operations_view,
        }
    }

    /// Adds the pill of item `id`: drawing order is selected items first, then the rest top first
    /// (ux-notes 13.4); an item outside an entered group goes to the dimmed ones.
    pub(super) fn add(&mut self, id: NodeId, chip: ChipItem, selected: bool, dimming: &Dimming) {
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

/// The hatch of `object` (of the item `id`) when one of its looks is hatched, dimmed when the
/// item is outside the entered group.
pub(super) fn hatch_overlay(
    object: &ObjectSnapshot,
    id: NodeId,
    looks: &[KindLook],
    view: ViewTransform,
    dimming: &Dimming,
) -> Option<DrawList> {
    let polygons = object_polygons(object).filter(|_| looks.iter().any(|l| hatched(l.look)))?;
    let hatch = hatch_list(&polygons, view);
    Some(if dimming.outside(id) {
        dimmed_list(hatch)
    } else {
        hatch
    })
}

/// The symbol chip of `object`, `None` when none of its looks has a symbol; a kind that holds it
/// without running it is muted unless another operation of that kind runs it.
pub(super) fn chip_of(object: &ObjectSnapshot, looks: &[KindLook]) -> Option<ChipItem> {
    let running: Vec<Glyph> = looks
        .iter()
        .filter_map(|l| l.symbol.filter(|(_, muted)| !muted).map(|(g, _)| g))
        .collect();
    let mut muted: Vec<Glyph> = looks
        .iter()
        .filter_map(|l| l.symbol.filter(|(_, muted)| *muted).map(|(g, _)| g))
        .filter(|g| !running.contains(g))
        .collect();
    let mut glyphs: Vec<Glyph> = running.iter().chain(&muted).copied().collect();
    glyphs.sort_by_key(|g| *g as u8);
    glyphs.dedup();
    muted.sort_by_key(|g| *g as u8);
    muted.dedup();
    (!glyphs.is_empty()).then(|| ChipItem {
        bounds: curvyo_ui_core::object_bounds(object),
        glyphs,
        muted,
        handles: false,
    })
}

/// The hatch of the page's own item `object` into `overlay` and its symbol chip into `rows`; a
/// grid's items share one chip row over the grid (UX notes §11).
pub(super) fn own_overlay(
    object: &ObjectSnapshot,
    looks: &[KindLook],
    rows: &mut ChipRows<'_>,
    overlay: &mut DrawList,
    view: ViewTransform,
    dimming: &Dimming,
    selected: &[NodeId],
) {
    let id = object.id();
    if let Some(hatch) = hatch_overlay(object, id, looks, view, dimming) {
        overlay.extend(hatch);
    }
    if !rows.grids.parts.contains_key(&id)
        && let Some(chip) = chip_of(object, looks)
    {
        let selected_here = selected.contains(&id);
        // The one selected object shows its own handles: its pill takes the corner anchor
        // (`specs/0164` Frage 459 A); the members of a multi-selection keep theirs centred.
        let chip = ChipItem {
            handles: selected_here && selected.len() == 1,
            ..chip
        };
        rows.add(id, chip, selected_here, dimming);
    }
}

impl Session {
    /// The symbol chips of the looked items, the dimmed ones of the items outside an entered
    /// group, and the grids' chips, drawn into `overlay`.
    pub(super) fn chips_overlay(
        &self,
        overlay: &mut DrawList,
        rows: ChipRows<'_>,
        view: ViewTransform,
    ) {
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
            // No pill lies over a transform handle of the selection or the group box
            // (`specs/0164` S3).
            let handles = if self.selection.ids().is_empty() {
                Vec::new()
            } else {
                self.select_transform_decoration_input_in(objects).handles
            };
            overlay.extend(chips_list_clear_of(
                &items,
                self.viewport.canvas_size(),
                view,
                &handles,
            ));
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
}
