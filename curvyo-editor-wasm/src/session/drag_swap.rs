//! What a Select-tool drag draws (customer 2026-10-27, `specs/0009-unified-object-editing` note
//! of that date): the objects the drag moves are drawn as themselves where a release would put
//! them, and the outlines of the committed ones stay behind as the ghost.

use std::collections::HashMap;

use curvyo_document_core::{NodeId, ObjectSnapshot, ViewTransform};
use curvyo_render_core::DrawList;
use curvyo_ui_core::LiveEdit;

use super::Session;

/// The objects one frame of a drag replaces in the artwork.
#[derive(Default)]
pub(super) struct DragSwap {
    /// The objects as a release would leave them, each with the id of the committed object it
    /// replaces (in a copy: of the original it is copied from).
    moved: Vec<ObjectSnapshot>,
    /// Whether the moved objects are copies: they then go right above their originals, which
    /// stay drawn as themselves and need no ghost.
    copy: bool,
}

impl DragSwap {
    /// Whether the frame has nothing in flight.
    pub(super) fn is_empty(&self) -> bool {
        self.moved.is_empty()
    }

    /// `objects` (the artwork, in tree order) with the moved objects in place of the committed
    /// ones, or in a copy right above them, as `duplicate_objects` places a copy.
    pub(super) fn apply_to(&self, objects: &mut Vec<ObjectSnapshot>) {
        let moved: HashMap<NodeId, &ObjectSnapshot> = self
            .moved
            .iter()
            .map(|object| (object.id(), object))
            .collect();
        if self.copy {
            let mut with_copies = Vec::with_capacity(objects.len() + moved.len());
            for object in objects.drain(..) {
                let copy = moved.get(&object.id()).copied().cloned();
                with_copies.push(object);
                with_copies.extend(copy);
            }
            *objects = with_copies;
        } else {
            for slot in objects.iter_mut() {
                if let Some(new) = moved.get(&slot.id()) {
                    *slot = (*new).clone();
                }
            }
        }
    }

    /// The ghost: the outlines of the committed objects the drag moves, from `committed` (the
    /// document read). A copy leaves no ghost.
    pub(super) fn ghost(&self, committed: &[ObjectSnapshot], view: ViewTransform) -> DrawList {
        if self.copy || self.moved.is_empty() {
            return DrawList::default();
        }
        let moved: std::collections::HashSet<NodeId> =
            self.moved.iter().map(ObjectSnapshot::id).collect();
        let old: Vec<ObjectSnapshot> = committed
            .iter()
            .filter(|object| moved.contains(&object.id()))
            .cloned()
            .collect();
        curvyo_render_core::build_drag_ghost(&old, view)
    }
}

impl Session {
    /// This frame's [`DragSwap`]: the Select tool's live edit (`live`) with the clones and the
    /// texts on path that follow it (`0049` criterion 11, `0080` criterion 24), or a text on path
    /// dragged along its carrier.
    pub(super) fn drag_swap_in(
        &self,
        objects: &[ObjectSnapshot],
        live: Option<&LiveEdit>,
    ) -> DragSwap {
        let mut swap = DragSwap::default();
        if let Some(live) = live {
            swap.copy = live.copy;
            swap.moved.extend(live.objects.iter().cloned());
            // A copy moves nothing that follows the originals.
            if !live.copy {
                swap.moved
                    .extend(Self::clone_previews_in(objects, &live.objects, true));
                swap.moved
                    .extend(self.text_path_followers(objects, &live.objects));
            }
        }
        if let Some(text) = self.text_path_live() {
            swap.moved.push(ObjectSnapshot::Text(Box::new(text)));
        }
        swap
    }
}
