//! `Session`'s glue for the Writing mode and Glyph orientation groups of the Text bar
//! (`specs/0079-text-layout-controls` slice 3, criteria 20, 21, 33): the pressed state, a press
//! on the subject texts in one step that keeps them in place, and the next text's values. The
//! rule for the placement lives in `curvyo_ui_core::kept_placement`.

use curvyo_document_core::{
    Affine, NodeId, TextGlyphOrientation, TextLayoutLabel, TextLayoutValues, TextWritingMode,
};
use curvyo_ui_core::kept_placement;

use super::Session;

impl Session {
    /// The pressed writing mode: the subject texts' common one (`None`: they differ), or the
    /// next text's (criteria 20, 33).
    #[must_use]
    pub fn text_writing_mode(&self) -> Option<TextWritingMode> {
        self.common_layout(|values| values.writing_mode, self.text.next_writing_mode)
    }

    /// The pressed glyph orientation, as [`Self::text_writing_mode`].
    #[must_use]
    pub fn text_glyph_orientation(&self) -> Option<TextGlyphOrientation> {
        self.common_layout(|values| values.orientation, self.text.next_orientation)
    }

    fn common_layout<T: PartialEq + Copy>(
        &self,
        of: impl Fn(&TextLayoutValues) -> T,
        next: T,
    ) -> Option<T> {
        let texts = self.bar_texts();
        let Some(first) = texts.first() else {
            return Some(next);
        };
        let value = of(&first.layout);
        texts
            .iter()
            .all(|text| of(&text.layout) == value)
            .then_some(value)
    }

    /// A press in the Writing mode group: the subject texts take `mode` in one step "Change
    /// writing mode", the first column's start where the first line's start was (criterion 20);
    /// with none, the next text takes it without a step (criterion 33).
    pub fn set_text_writing_mode(&mut self, mode: TextWritingMode) {
        if self.bar_texts().is_empty() {
            self.text.next_writing_mode = mode;
            return;
        }
        self.write_layout_choice(TextLayoutLabel::WritingMode, |values| {
            values.writing_mode = mode;
        });
    }

    /// A press in the Glyph orientation group, as [`Self::set_text_writing_mode`]. A horizontal
    /// text keeps the value for when it is set vertical (criterion 21).
    pub fn set_text_glyph_orientation(&mut self, orientation: TextGlyphOrientation) {
        if self.bar_texts().is_empty() {
            self.text.next_orientation = orientation;
            return;
        }
        self.write_layout_choice(TextLayoutLabel::GlyphOrientation, |values| {
            values.orientation = orientation;
        });
    }

    fn write_layout_choice(
        &mut self,
        label: TextLayoutLabel,
        change: impl Fn(&mut TextLayoutValues),
    ) {
        self.recording(|this| {
            let writes: Vec<(NodeId, TextLayoutValues, Affine)> = this
                .bar_texts()
                .iter()
                .filter_map(|text| {
                    let mut values = text.layout;
                    change(&mut values);
                    (values != text.layout).then(|| {
                        let placement = this
                            .face_of(&text.font)
                            .map_or(text.placement, |face| kept_placement(text, face, &values));
                        (text.id, values, placement)
                    })
                })
                .collect();
            this.write_layout(&writes, label);
        });
    }
}
