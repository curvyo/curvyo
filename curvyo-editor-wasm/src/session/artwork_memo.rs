//! The artwork of the session's own read kept between frames (`specs/0126-performance-pass`
//! phase 4): a frame that tessellates the same objects of the same read at the same scale (a
//! pan that brings nothing into or out of view) draws the kept list again.

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;

use curvyo_document_core::{NodeId, ObjectSnapshot, ViewTransform};
use curvyo_render_core::DrawList;

use super::view_cull::{CullBoxes, VisibleRect};
use super::{Hit, Session, Tool};

/// What a frame's artwork is built from besides the objects the cull keeps. The tessellation
/// reads the view's scale, never its origin, so two frames with the same key and the same kept
/// objects build the same list. The tool, the selection, the hover, the canvas size and the
/// pixel ratio are not read by the artwork: they are in the key so that anything but a pan
/// builds the list afresh.
#[derive(Debug, Clone)]
struct ArtworkKey {
    /// The read the objects are; held, so its slice cannot be another's.
    read: Arc<[ObjectSnapshot]>,
    guides: Arc<HashSet<NodeId>>,
    laser: Arc<HashSet<NodeId>>,
    scale: u64,
    tool: Tool,
    selection: Vec<NodeId>,
    hover: (Option<Hit>, Option<NodeId>),
    canvas: [u64; 3],
}

impl ArtworkKey {
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.read, &other.read)
            && Arc::ptr_eq(&self.guides, &other.guides)
            && self.laser == other.laser
            && self.scale == other.scale
            && self.tool == other.tool
            && self.selection == other.selection
            && self.hover == other.hover
            && self.canvas == other.canvas
    }
}

/// The cull boxes of the last read and the artwork of the last frames.
#[derive(Debug, Default)]
pub(super) struct ArtworkMemo {
    /// The read the boxes were measured over, and the boxes.
    boxes: Option<(Arc<[ObjectSnapshot]>, Arc<CullBoxes>)>,
    /// The key and the kept objects of the frame before: a list is kept only from the second
    /// frame with both on, so a zoom (a scale of its own in every frame) or a pan that brings
    /// objects in or out in every frame never copies its list.
    last: Option<(ArtworkKey, Vec<usize>)>,
    /// The kept list, its key and the indices of the objects it draws.
    held: Option<(ArtworkKey, Vec<usize>, DrawList)>,
    /// How many frames drew the kept list, for the tests.
    pub(super) hits: u64,
}

impl Session {
    /// The artwork of `read`, the session's own read of the objects as it is (no live edit, no
    /// preview, no look), without the objects `visible` cannot show: the list
    /// [`Session::artwork_list`] builds, kept in [`ArtworkMemo`] while only the view's origin
    /// moves.
    pub(super) fn read_artwork_list(
        &self,
        read: &Arc<[ObjectSnapshot]>,
        guides: &Arc<HashSet<NodeId>>,
        view: ViewTransform,
        visible: Option<VisibleRect>,
    ) -> DrawList {
        let boxes = self.cull_boxes(read);
        // A pattern's pieces depend on the style preview and the pattern cache too.
        if boxes.patterned() {
            return self.artwork_list(read, guides, view, visible);
        }
        // Entries of shapes gone from the read are dropped, as `artwork_list` does.
        self.patterns
            .borrow_mut()
            .retain_only(&read.iter().map(ObjectSnapshot::id).collect());
        let kept = boxes.kept(read, guides, visible);
        let key = self.artwork_key(read, guides, view);
        {
            let mut memo = self.artwork_memo.borrow_mut();
            if let Some((held, held_kept, list)) = memo.held.as_ref()
                && held.same(&key)
                && *held_kept == kept
            {
                let list = list.clone();
                memo.hits += 1;
                return list;
            }
        }
        let objects: Cow<'_, [ObjectSnapshot]> = if kept.len() == read.len() {
            Cow::Borrowed(read)
        } else {
            Cow::Owned(kept.iter().map(|&index| read[index].clone()).collect())
        };
        let list = self.artwork_of(&objects, guides, view);
        let mut memo = self.artwork_memo.borrow_mut();
        let repeated = memo
            .last
            .as_ref()
            .is_some_and(|(last, last_kept)| last.same(&key) && *last_kept == kept);
        memo.held = repeated.then(|| (key.clone(), kept.clone(), list.clone()));
        memo.last = Some((key, kept));
        list
    }

    /// The cull boxes of `read`, measured once per read.
    fn cull_boxes(&self, read: &Arc<[ObjectSnapshot]>) -> Arc<CullBoxes> {
        let mut memo = self.artwork_memo.borrow_mut();
        if let Some((held, boxes)) = memo.boxes.as_ref()
            && Arc::ptr_eq(held, read)
        {
            return Arc::clone(boxes);
        }
        let boxes = Arc::new(CullBoxes::of(read));
        memo.boxes = Some((Arc::clone(read), Arc::clone(&boxes)));
        boxes
    }

    /// The key of this frame's artwork of `read`.
    fn artwork_key(
        &self,
        read: &Arc<[ObjectSnapshot]>,
        guides: &Arc<HashSet<NodeId>>,
        view: ViewTransform,
    ) -> ArtworkKey {
        let (_, height) = self.viewport.canvas_size();
        ArtworkKey {
            read: Arc::clone(read),
            guides: Arc::clone(guides),
            // A painting leaf is never left out by the cull, so the read's set is the kept
            // objects' set.
            laser: self.laser_gradient_ids(read),
            scale: view.scale().to_bits(),
            tool: self.tool,
            selection: self.selection.ids().to_vec(),
            hover: (self.hovered, self.hovered_object),
            canvas: [
                self.canvas_css_width.to_bits(),
                height.to_bits(),
                self.device_pixel_ratio.to_bits(),
            ],
        }
    }
}

#[cfg(test)]
#[path = "artwork_memo_tests.rs"]
mod tests;
