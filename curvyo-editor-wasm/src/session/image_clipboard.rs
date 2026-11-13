//! The session side of Ctrl+C and Ctrl+V for images (`specs/0087-image-clipboard-and-drop`
//! criteria 3, 12, 13 and 16, `adrs.md` decisions 2 and 5): which image a copy takes, its stored
//! pixels with their fingerprint and the image's trace mask (`specs/0114` criterion 14, ADR 0061
//! §8); when the page may take the chords. The paste itself is an import with the paste origin.

use curvyo_document_core::ObjectSnapshot;

use super::Session;

/// What Ctrl+C puts on the clipboard: the stored pixels of one image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageCopy {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The working image: RGBA8, straight alpha, upright, without the object's transform.
    pub rgba: Vec<u8>,
    /// `picture_fingerprint` of the working image: a paste whose picture decodes to the same
    /// one gets [`ImageCopy::mask`].
    pub fingerprint: String,
    /// The image's trace mask as JSON ([`curvyo_document_core::TraceMask`]), empty when it has
    /// none.
    pub mask: String,
}

/// FNV-1a 64 over the width, the height (little endian) and the RGBA of a picture, as 16 hex
/// digits (ADR 0061 §8): equal for the copied working image and a paste that decodes to it.
#[must_use]
pub fn picture_fingerprint(width: u32, height: u32, rgba: &[u8]) -> String {
    let bytes = width
        .to_le_bytes()
        .into_iter()
        .chain(height.to_le_bytes())
        .chain(rgba.iter().copied());
    let hash = bytes.fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    format!("{hash:016x}")
}

impl Session {
    /// The topmost selected image's stored pixels (criteria 12 and 13), or `None` when the
    /// selection holds no image (a group is not an image) or its picture cannot be read.
    pub fn image_copy(&mut self) -> Option<ImageCopy> {
        let ids = self.selection.ids().to_vec();
        let objects = self.objects();
        // The leaves are in painting order: the last selected image is the topmost.
        let images: Vec<_> = objects
            .iter()
            .filter_map(|object| match object {
                ObjectSnapshot::Image(image) if ids.contains(&image.id) => Some(image),
                _ => None,
            })
            .collect();
        let top = images.last()?;
        let id = top.id;
        let working = self.working_image(&top.asset)?;
        let mask = self
            .document
            .trace_mask(id)
            .filter(|mask| !mask.is_empty())
            .and_then(|mask| serde_json::to_string(&mask).ok())
            .unwrap_or_default();
        Some(ImageCopy {
            width: working.width,
            height: working.height,
            fingerprint: picture_fingerprint(working.width, working.height, &working.pixels),
            rgba: working.pixels.clone(),
            mask,
        })
    }

    /// Whether Ctrl+C and Ctrl+V may act on images now (criteria 3 and 16): not while the page
    /// reports it `blocked` (an IME composition, a long operation), a pointer drag runs, a typed
    /// entry is open or a text is edited on the canvas, whose own clipboard keeps the keys.
    #[must_use]
    pub fn clipboard_allowed(&self, blocked: bool) -> bool {
        !(blocked
            || self.in_read_only_group()
            || self.button_down
            || self.operation_in_flight()
            || self.select.has_entry()
            || self.text_editing())
    }
}
