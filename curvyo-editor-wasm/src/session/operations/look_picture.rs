//! The looks of a picture clone's leaves on the output page (`specs/0123` criteria 1, 46 b): each
//! leaf of a clone of a group, text or image in the look of the member it comes from, with its
//! hatch and its symbol chip.

use curvyo_document_core::{CloneSnapshot, NodeId, ObjectSnapshot, ViewTransform};
use curvyo_render_core::DrawList;

use super::look::{Dimming, LeafLook, LookTable, drawn_as};
use super::look_overlay::{ChipRows, chip_of, hatch_overlay};

/// Each leaf of `clone`'s picture with its look: the member's of a cloned group by its path, or
/// the clone's own for a clone of a text or an image; `None` when the page holds no look for it.
fn leaf_looks<'a>(
    clone: &'a CloneSnapshot,
    table: &'a LookTable,
) -> impl Iterator<Item = (&'a ObjectSnapshot, Option<&'a LeafLook>)> + 'a {
    let own = clone.id();
    let picture = clone.chain.picture.as_deref().unwrap_or_default();
    let members = clone.chain.picture_members.as_deref().unwrap_or_default();
    picture.iter().zip(members).map(move |(leaf, member)| {
        let look = table
            .clone_members
            .get(&(own, *member))
            .or_else(|| table.own.get(&own));
        (leaf, look)
    })
}

/// The leaves of a clone with a picture as drawn (`rows` holds the looks and the view): in the
/// Operations view each in its look; in the Drawn view each in its own look, the Show-off leaves
/// dropped.
pub(super) fn picture_clone_leaves(
    clone: &CloneSnapshot,
    rows: &ChipRows<'_>,
    view: ViewTransform,
    dimming: &Dimming,
) -> Vec<ObjectSnapshot> {
    let operations_view = rows.operations_view;
    let looked = leaf_looks(clone, rows.table)
        .flat_map(|(leaf, look)| match look {
            Some(look) if operations_view => drawn_as(leaf, look, view),
            Some(LeafLook::Hidden) => Vec::new(),
            _ => vec![leaf.clone()],
        })
        .collect();
    dimming.dim(clone.id(), looked)
}

/// The hatch and the symbol chip of each looked leaf of `clone` (View: Operations): the hatch
/// into `overlay`, the chips into `rows`, all by the clone's id for the dimming and the
/// selection.
pub(super) fn picture_overlay(
    clone: &CloneSnapshot,
    rows: &mut ChipRows<'_>,
    overlay: &mut DrawList,
    view: ViewTransform,
    dimming: &Dimming,
    selected: &[NodeId],
) {
    let id = clone.id();
    for (leaf, look) in leaf_looks(clone, rows.table) {
        let Some(LeafLook::Shown(looks)) = look else {
            continue;
        };
        if let Some(hatch) = hatch_overlay(leaf, id, looks, view, dimming) {
            overlay.extend(hatch);
        }
        if let Some(chip) = chip_of(leaf, looks) {
            rows.add(id, chip, selected.contains(&id), dimming);
        }
    }
}
