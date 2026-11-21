//! Copy and Paste adjustments of the selected picture (`specs/0135` criterion 10,
//! `ux-notes.md` §5, P16): the session holds the copied set, never the system clipboard.

use curvyo_ui_core::CopiedAdjustments;

impl crate::session::Session {
    /// Copy adjustments: the selected picture's Preset, Strength, stack and Dither as it shows
    /// them, held for the session; no step. `false` without a picture.
    pub fn copy_adjustments(&mut self) -> bool {
        let Some(shown) = self
            .adjust_target()
            .and_then(|target| self.document.shown_adjustments(target).ok())
        else {
            return false;
        };
        self.image_adjust.copied = Some(CopiedAdjustments::of(&shown.adjustments));
        true
    }

    /// Whether Paste adjustments has something to paste (greyed until a Copy).
    pub(super) fn can_paste_adjustments(&self) -> bool {
        self.image_adjust.copied.is_some()
    }

    /// Paste adjustments: the copied set replaces the selected picture's Preset, Strength, stack
    /// and Dither in one step "Paste adjustments" (a following clone gets its own); whether
    /// anything changed.
    pub fn paste_adjustments(&mut self) -> bool {
        self.image_adjust.held_strength = None;
        let (Some(target), Some(copied)) = (self.adjust_target(), &self.image_adjust.copied) else {
            return false;
        };
        let write = copied.write();
        self.document
            .paste_image_adjustments(target, &write)
            .unwrap_or(false)
    }
}
