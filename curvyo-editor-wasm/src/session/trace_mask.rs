//! The Trace tool's Mask view on the session side (`specs/0089-trace-quality-and-colour`
//! criterion 21, `ux-notes.md` §4): the mask the worker made for the shown image, kept as an
//! opaque black-on-white picture and drawn in the image's place at the image's frame.

use std::sync::Arc;

use curvyo_document_core::{AssetHash, ImageSnapshot, NodeId, ObjectSnapshot, Opacity};
use curvyo_image_core::WorkingImage;

use super::Session;

/// The kept mask of one image.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TraceMask {
    image: NodeId,
    /// The name its picture is drawn under; new for every mask, so a texture is never reused.
    hash: AssetHash,
    picture: Arc<WorkingImage>,
    /// A stopped job left it out of date: drawn at half strength (UX notes §4).
    stale: bool,
}

/// The kept mask and the count that names the next one.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct TraceMaskState {
    shown: Option<TraceMask>,
    made: u64,
}

impl Session {
    /// Keeps the worker's mask of `image` (`width` x `height`, row-major, non-zero for content)
    /// for the Mask view; `false` when the bits do not match the size.
    pub fn set_trace_mask(&mut self, image: NodeId, width: u32, height: u32, bits: &[u8]) -> bool {
        let pixels = u64::from(width) * u64::from(height);
        if pixels == 0 || usize::try_from(pixels).ok() != Some(bits.len()) {
            return false;
        }
        let rgba = bits
            .iter()
            .flat_map(|&bit| {
                if bit == 0 {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                }
            })
            .collect();
        let state = &mut self.trace.mask;
        state.made += 1;
        let mut name = b"curvyo trace mask ".to_vec();
        name.extend_from_slice(&state.made.to_le_bytes());
        state.shown = Some(TraceMask {
            image,
            hash: AssetHash::of(&name),
            picture: Arc::new(WorkingImage {
                width,
                height,
                pixels: rgba,
                original_size: (width, height),
                truncated: false,
                animated: false,
            }),
            stale: false,
        });
        true
    }

    /// Marks the kept mask out of date (a stopped job) or current again.
    pub(super) fn set_trace_mask_stale(&mut self, stale: bool) {
        if let Some(mask) = &mut self.trace.mask.shown {
            mask.stale = stale;
        }
    }

    /// The mask's picture when `hash` names it: the image cache has no asset for it.
    pub(super) fn trace_mask_picture(&self, hash: &AssetHash) -> Option<Arc<WorkingImage>> {
        self.trace
            .mask
            .shown
            .as_ref()
            .filter(|mask| mask.hash == *hash)
            .map(|mask| Arc::clone(&mask.picture))
    }

    /// The Mask view's artwork: the shown image and its wash replaced by its mask at the same
    /// frame, at half strength when stale; the image stays washed while it has no mask yet.
    pub(super) fn trace_mask_in(&self, objects: &mut Vec<ObjectSnapshot>) {
        let Some(mask) = &self.trace.mask.shown else {
            return;
        };
        if self.trace_view().shown != Some(mask.image) {
            return;
        }
        let Some(at) = objects.iter().position(
            |object| matches!(object, ObjectSnapshot::Image(image) if image.id == mask.image),
        ) else {
            return;
        };
        let ObjectSnapshot::Image(image) = &objects[at] else {
            return;
        };
        let mut drawn = ImageSnapshot::new(image.id, mask.hash.clone(), image.placement);
        let strength = if mask.stale { 0.5 } else { 1.0 };
        drawn.shown_opacity =
            Opacity::new(image.shown_opacity.get() * strength).unwrap_or(Opacity::OPAQUE);
        objects[at] = ObjectSnapshot::Image(drawn);
        // The wash directly above carries the image's id; the mask is drawn unwashed.
        if objects
            .get(at + 1)
            .is_some_and(|object| object.id() == mask.image)
        {
            objects.remove(at + 1);
        }
    }
}
