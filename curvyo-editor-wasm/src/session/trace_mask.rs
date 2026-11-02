//! The Trace tool's Mask view on the session side (`specs/0089-trace-quality-and-colour`
//! criterion 21, `ux-notes.md` §4): the mask the worker made for the shown image, kept as an
//! opaque picture (black on white, or a multicolour trace's segmented palette on white with the
//! colours Isolate leaves out dimmed) and drawn in the image's place at the image's frame.

use std::sync::Arc;

use curvyo_document_core::{AssetHash, ImageSnapshot, NodeId, ObjectSnapshot, Opacity};
use curvyo_image_core::WorkingImage;

use curvyo_ui_core::TraceRgb;

use super::Session;
use super::trace_colours::DIMMED;

/// The kept mask of one image.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TraceMask {
    image: NodeId,
    /// The name its picture is drawn under; new for every mask, so a texture is never reused.
    hash: AssetHash,
    picture: Arc<WorkingImage>,
    /// A multicolour mask's segmented palette as the worker sent it (straight RGBA, the
    /// background transparent), kept so Isolate can dim it again.
    colours: Option<Vec<u8>>,
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
    /// Keeps the worker's mask of `image` for the Mask view: `width` x `height` bits (row-major,
    /// non-zero for content, `LEFT_OUT_LEVEL` for pixels left out for text), or for a multicolour trace four bytes per pixel, the segmented
    /// palette in straight RGBA with the background transparent; `false` when the bytes match
    /// neither size.
    pub fn set_trace_mask(&mut self, image: NodeId, width: u32, height: u32, bits: &[u8]) -> bool {
        let pixels = u64::from(width) * u64::from(height);
        let Ok(count) = usize::try_from(pixels) else {
            return false;
        };
        if count == 0 {
            return false;
        }
        let (rgba, colours) = if bits.len() == count {
            let rgba = bits
                .iter()
                .flat_map(|&bit| match bit {
                    0 => [255, 255, 255, 255],
                    // `specs/0091` UX notes §2: the pixels left out for text, flat mid grey.
                    curvyo_ui_core::LEFT_OUT_LEVEL => [128, 128, 128, 255],
                    _ => [0, 0, 0, 255],
                })
                .collect();
            (rgba, None)
        } else if Some(bits.len()) == count.checked_mul(4) {
            (self.palette_picture(bits), Some(bits.to_vec()))
        } else {
            return false;
        };
        let hash = self.next_mask_hash();
        self.trace.mask.shown = Some(TraceMask {
            image,
            hash,
            picture: Arc::new(WorkingImage {
                width,
                height,
                pixels: rgba,
                original_size: (width, height),
                truncated: false,
                animated: false,
            }),
            colours,
            stale: false,
        });
        true
    }

    /// A fresh name for the next mask picture, so a texture is never reused.
    fn next_mask_hash(&mut self) -> AssetHash {
        let state = &mut self.trace.mask;
        state.made += 1;
        let mut name = b"curvyo trace mask ".to_vec();
        name.extend_from_slice(&state.made.to_le_bytes());
        AssetHash::of(&name)
    }

    /// A segmented palette (straight RGBA) as an opaque picture: the background white, every
    /// colour its fill, the colours Isolate leaves out mixed with white to [`DIMMED`].
    fn palette_picture(&self, rgba: &[u8]) -> Vec<u8> {
        let view = &self.trace.palette_view;
        rgba.as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| {
                if p[3] == 0 {
                    return [255, 255, 255, 255];
                }
                let colour = TraceRgb::new(p[0], p[1], p[2]);
                let fill = view.fill(colour);
                let strength = if view.dimmed(colour) { DIMMED } else { 1.0 };
                // invariant: a mix of two bytes stays a byte.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let mix =
                    |c: u8| (f64::from(c) * strength + 255.0 * (1.0 - strength)).round() as u8;
                [mix(fill.r), mix(fill.g), mix(fill.b), 255]
            })
            .collect()
    }

    /// Draws a multicolour mask again after the recolours or Isolate changed.
    pub(super) fn refresh_trace_mask(&mut self) {
        let Some(colours) = self
            .trace
            .mask
            .shown
            .as_ref()
            .and_then(|mask| mask.colours.clone())
        else {
            return;
        };
        let pixels = self.palette_picture(&colours);
        let hash = self.next_mask_hash();
        if let Some(mask) = &mut self.trace.mask.shown {
            let (width, height) = (mask.picture.width, mask.picture.height);
            mask.hash = hash;
            mask.picture = Arc::new(WorkingImage {
                width,
                height,
                pixels,
                original_size: (width, height),
                truncated: false,
                animated: false,
            });
        }
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
