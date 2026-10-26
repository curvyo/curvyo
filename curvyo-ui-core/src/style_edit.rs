//! A panel drag in flight and its single commit
//! (`specs/0007-stroke-and-fill-styling` criterion 36): while a slider or
//! colour area is dragged the selected objects are drawn in the new style
//! without touching the document, and the release writes exactly one commit,
//! to the objects the drag started on.

use std::sync::Arc;

use curvyo_document_core::{Document, NodeId, ObjectSnapshot, StyleEdit, StyleEditError};

/// The preview of the edit being dragged: one style property of whole objects.
#[derive(Debug, Clone, PartialEq)]
struct Pending {
    ids: Vec<NodeId>,
    edit: StyleEdit,
}

/// The ephemeral style override of a panel drag. Not part of the document and
/// never saved.
#[derive(Debug, Clone, Default)]
pub struct StyleEditor {
    pending: Option<Pending>,
    /// Escape dropped the preview mid-drag: further previews are ignored until
    /// the release ends the gesture.
    cancelled: bool,
}

impl StyleEditor {
    /// Shows `edit` on `ids` instead of their stored style. The objects are
    /// fixed by the first call of a drag; later calls change only the edit.
    /// Ignored after [`StyleEditor::cancel`] until the release.
    pub fn preview(&mut self, ids: &[NodeId], edit: StyleEdit) {
        if self.cancelled {
            return;
        }
        match &mut self.pending {
            Some(pending) => pending.edit = edit,
            None => {
                self.pending = Some(Pending {
                    ids: ids.to_vec(),
                    edit,
                });
            }
        }
    }

    /// The edit being previewed, if any.
    #[must_use]
    pub fn pending_edit(&self) -> Option<&StyleEdit> {
        self.pending.as_ref().map(|pending| &pending.edit)
    }

    /// Whether a preview is showing.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.pending.is_some()
    }

    /// Escape during a drag: drops the preview so the objects return to their
    /// committed style, and the release then writes nothing.
    pub fn cancel(&mut self) {
        self.cancelled = self.pending.take().is_some();
    }

    /// The release: writes the previewed edit as one commit to the objects it
    /// started on, and ends the gesture. Writes nothing if there is no
    /// preview (none was made, or Escape dropped it).
    ///
    /// # Errors
    /// The refusal of [`Document::edit_style`]; the preview is dropped either
    /// way.
    pub fn commit(&mut self, document: &Document) -> Result<(), StyleEditError> {
        self.cancelled = false;
        match self.pending.take() {
            Some(Pending { ids, edit }) => document.edit_style(&ids, &edit),
            None => Ok(()),
        }
    }

    /// Replaces the style of the previewed objects in `objects` with the
    /// previewed one, for drawing.
    pub fn apply_to(&self, objects: &mut [ObjectSnapshot]) {
        if let Some(Pending { ids, edit }) = &self.pending {
            for object in objects.iter_mut().filter(|o| ids.contains(&o.id())) {
                // A refused edit (a negative width) previews as nothing.
                let _ = edit.apply_to(object.style_mut());
                // A text is drawn through its laid-out glyphs, which carry their own copy of
                // the style: they take the previewed one too (customer 2026-10-26).
                if let ObjectSnapshot::Text(text) = object {
                    let style = crate::text_layout::glyph_style(text);
                    if let Some(outline) = text.outline.as_mut() {
                        Arc::make_mut(outline).glyphs.style = style;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use curvyo_document_core::{
        Affine, FontKind, FontRef, Length, Point, Style, StyleEdit, TextChars, TextLayoutValues,
        TextOutline, TextSnapshot,
    };

    use super::*;

    fn text(kind: FontKind) -> ObjectSnapshot {
        let mut text = TextSnapshot {
            id: curvyo_document_core::UNCREATED_TEXT_ID,
            string: "A".to_owned(),
            font: FontRef {
                family: "Source Sans 3".to_owned(),
                style: "Regular".to_owned(),
                face_hash: "0".repeat(64),
                face_index: 0,
                kind,
            },
            size: Length::from_mm(10.0),
            layout: TextLayoutValues::DEFAULT,
            chars: TextChars::default(),
            placement: Affine::IDENTITY,
            style: Style::default(),
            outline: None,
            on_path: None,
            source: None,
        };
        let glyphs = crate::text_layout::glyph_path(&text, Vec::new(), text.rotation());
        text.outline = Some(Arc::new(TextOutline {
            glyphs,
            line_box: [Point::new(0.0, 0.0); 4],
            glyph_outlines: Vec::new(),
            letter_boxes: Vec::new(),
            substitute: false,
        }));
        ObjectSnapshot::Text(Box::new(text))
    }

    fn glyph_style(object: &ObjectSnapshot) -> Style {
        match object {
            ObjectSnapshot::Text(text) => text.outline.as_ref().unwrap().glyphs.style.clone(),
            _ => unreachable!(),
        }
    }

    /// Customer 2026-10-26 (`specs/0007-stroke-and-fill-styling` criterion 36 for texts): a panel
    /// drag on a text draws its glyphs in the previewed style, not only its style register.
    #[test]
    fn a_previewed_text_draws_its_glyphs_in_the_new_style() {
        let mut objects = [text(FontKind::Outline)];
        let mut editor = StyleEditor::default();
        editor.preview(&[objects[0].id()], StyleEdit::FillEnabled(true));
        editor.apply_to(&mut objects);
        assert!(glyph_style(&objects[0]).fill.enabled);
        assert_eq!(&glyph_style(&objects[0]), objects[0].style());
    }

    /// A single-line text keeps its fill off in the glyphs (`specs/0076-single-line-fonts` 8).
    #[test]
    fn a_previewed_single_line_text_keeps_its_glyph_fill_off() {
        let mut objects = [text(FontKind::SingleLine)];
        let mut editor = StyleEditor::default();
        editor.preview(&[objects[0].id()], StyleEdit::FillEnabled(true));
        editor.apply_to(&mut objects);
        assert!(!glyph_style(&objects[0]).fill.enabled);
    }
}
