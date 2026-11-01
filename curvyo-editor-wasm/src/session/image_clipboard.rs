//! The session side of Ctrl+C and Ctrl+V for images (`specs/0087-image-clipboard-and-drop`
//! criteria 3, 12, 13 and 16, `adrs.md` decisions 2 and 5): which image a copy takes and its stored
//! pixels; when the page may take the chords. The paste itself is an import with the paste origin.

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
        let working = self.working_image(&top.asset)?;
        Some(ImageCopy {
            width: working.width,
            height: working.height,
            rgba: working.pixels.clone(),
        })
    }

    /// Whether Ctrl+C and Ctrl+V may act on images now (criteria 3 and 16): not while the page
    /// reports it `blocked` (an IME composition, a long operation), a pointer drag runs, a typed
    /// entry is open or a text is edited on the canvas, whose own clipboard keeps the keys.
    #[must_use]
    pub fn clipboard_allowed(&self, blocked: bool) -> bool {
        !(blocked
            || self.button_down
            || self.operation_in_flight()
            || self.select.has_entry()
            || self.text_editing())
    }
}
