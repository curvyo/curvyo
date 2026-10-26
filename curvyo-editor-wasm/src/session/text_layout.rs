//! `Session`'s glue for the text layout controls (`specs/0079-text-layout-controls` slice 1,
//! `adrs.md` decision 3): the Alignment group, and the fields of the text layout block with
//! their live preview, commit, reset and typed values. The rules live in `curvyo_ui_core`
//! (`realigned_placement`, `text_layout_fields`).

use curvyo_document_core::{
    Affine, CharValues, Length, NodeId, TextAlign, TextLayoutLabel, TextLayoutValues,
    TextWritingMode,
};
use curvyo_ui_core::{
    Grid, LayoutChange, LayoutField, LayoutFieldView, LayoutSubject, field_view, parse_field_text,
    realigned_placement, set_field, stepped, value_at,
};

use super::Session;
use super::text::{NEW_TEXT_ID, edit_id};

/// What a typed value, a reset or a commit did (criterion 7; UX notes §0.4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutOutcome {
    /// Written as asked.
    Set,
    /// Nothing changed.
    Unchanged,
    /// Not a number.
    Invalid,
    /// The one text was held to its limit: at the top (`true`) or the bottom, in mm (degrees,
    /// percent for the taper, which is held alike on every text).
    Clamped(bool, f64),
    /// Of `of` texts, `limited` were held to their own limit.
    Limited {
        /// How many were held.
        limited: usize,
        /// How many texts changed.
        of: usize,
    },
}

impl Session {
    /// The layout values a text of `size` with `values` is drawn with now: a layout field's live
    /// value applied (the text's own values without one).
    pub(super) fn previewed_layout(
        &self,
        size: Length,
        values: &TextLayoutValues,
    ) -> TextLayoutValues {
        match self.live_layout_preview() {
            Some((field, value)) => {
                let subject = LayoutSubject {
                    size_mm: size.as_mm(),
                    values: *values,
                    natural_line_mm: 0.0,
                    kern: CharValues::DEFAULT,
                };
                set_field(field, &[subject], value)
                    .values
                    .first()
                    .copied()
                    .unwrap_or(*values)
            }
            None => *values,
        }
    }

    /// The layout field's live value while its subject is the one it was started on; a subject
    /// change drops it as Escape does (UX notes §5).
    pub(super) fn live_layout_preview(&self) -> Option<(LayoutField, Option<f64>)> {
        self.text
            .layout_preview
            .as_ref()
            .filter(|(_, _, ids)| *ids == self.layout_subject_ids())
            .map(|(field, value, _)| (*field, *value))
    }

    /// The ids of the texts the layout fields act on: the edited text, else the selection.
    fn layout_subject_ids(&self) -> Vec<NodeId> {
        match self.text.edit.as_ref() {
            Some(edit) => vec![edit_id(edit).unwrap_or(NEW_TEXT_ID)],
            None => self.selection.ids().to_vec(),
        }
    }

    /// The pressed alignment of the Text bar: the subject texts' common one (`None`: they
    /// differ), or the next text's (criteria 1, 33).
    #[must_use]
    pub fn text_alignment(&self) -> Option<TextAlign> {
        let texts = self.bar_texts();
        let Some(first) = texts.first() else {
            return Some(self.text.next_align);
        };
        texts
            .iter()
            .all(|t| t.layout.align == first.layout.align)
            .then_some(first.layout.align)
    }

    /// A press in the Alignment group (criteria 1, 4, 33): the subject texts take `align` in one
    /// step "Change text alignment" and stay where they are on the canvas; with none, the next
    /// text takes it, without a step.
    pub fn set_text_alignment(&mut self, align: TextAlign) {
        self.recording(|this| {
            let texts = this.bar_texts();
            if texts.is_empty() {
                this.text.next_align = align;
                return;
            }
            let writes: Vec<(NodeId, TextLayoutValues, Affine)> = texts
                .iter()
                .filter(|text| text.layout.align != align)
                .map(|text| {
                    let placement = this.face_of(&text.font).map_or(text.placement, |face| {
                        realigned_placement(text, face, align)
                    });
                    let mut values = text.layout;
                    values.align = align;
                    (text.id, values, placement)
                })
                .collect();
            this.write_layout(&writes, TextLayoutLabel::Alignment);
        });
    }

    /// The subject texts as the fields see them, with a live value applied.
    fn layout_subjects(&self) -> Vec<LayoutSubject> {
        self.bar_texts()
            .iter()
            .map(|text| LayoutSubject {
                size_mm: text.size.as_mm(),
                values: self.previewed_layout(text.size, &text.layout),
                natural_line_mm: self.face_of(&text.font).map_or(text.size.as_mm(), |face| {
                    let metrics = face.metrics();
                    metrics
                        .auto_line_height(text.layout.writing_mode != TextWritingMode::Horizontal)
                        * text.size.as_mm()
                        / metrics.cap_height
                }),
                kern: CharValues::DEFAULT,
            })
            .collect()
    }

    /// The subjects of `field`: the texts, or for a kerning field their characters.
    fn field_subjects(&self, field: LayoutField) -> Vec<LayoutSubject> {
        if field.is_per_character() {
            self.kern_layout_subjects()
        } else {
            self.layout_subjects()
        }
    }

    /// The text layout block's eight fields, in order, or `None` when it is not shown: no text
    /// is selected and none is in edit mode (criteria 6, 26).
    #[must_use]
    pub fn text_layout_block(&self) -> Option<Vec<LayoutFieldView>> {
        let subjects = self.layout_subjects();
        if subjects.is_empty() {
            return None;
        }
        let kerns = self.kern_layout_subjects();
        Some(
            LayoutField::ALL
                .iter()
                .map(|field| {
                    field_view(
                        *field,
                        if field.is_per_character() {
                            &kerns
                        } else {
                            &subjects
                        },
                    )
                })
                .collect(),
        )
    }

    /// A tick of a drag or a click on `field` at position `p` (criterion 12): shown on the
    /// subject texts, not written.
    pub fn preview_text_layout(&mut self, field: LayoutField, p: f64, grid: Grid) {
        if !p.is_finite() {
            return;
        }
        self.text.layout_preview = None;
        let subjects = self.field_subjects(field);
        if subjects.is_empty() {
            return;
        }
        let value = value_at(field, &subjects, p, grid);
        self.text.layout_preview = Some((field, Some(value), self.layout_subject_ids()));
    }

    /// An arrow key on `field`: `steps` steps from the shown value, previewed; the key-up commits.
    pub fn step_text_layout(&mut self, field: LayoutField, steps: i32, grid: Grid) {
        let Some(view) = self
            .text_layout_block()
            .and_then(|views| views.into_iter().nth(field_index(field)))
        else {
            return;
        };
        let value = stepped(field, view.value, steps, grid);
        self.text.layout_preview = Some((field, Some(value), self.layout_subject_ids()));
    }

    /// The release or key-up: writes the previewed value as one step; nothing after Escape.
    pub fn commit_text_layout_preview(&mut self) -> LayoutOutcome {
        if let Some((field, value)) = self.live_layout_preview() {
            self.set_text_layout_value(field, value)
        } else {
            self.text.layout_preview = None;
            LayoutOutcome::Unchanged
        }
    }

    /// Escape during a drag: the stored values show again and the release writes nothing.
    pub fn cancel_text_layout_preview(&mut self) {
        self.text.layout_preview = None;
    }

    /// The reset slot or Ctrl+Backspace: 0, or Auto for Line spacing, in one step.
    pub fn reset_text_layout(&mut self, field: LayoutField) -> LayoutOutcome {
        if field.is_per_character() {
            return self.reset_kerning(field);
        }
        let value = match field {
            LayoutField::LineSpacing => None,
            _ => Some(0.0),
        };
        self.set_text_layout_value(field, value)
    }

    /// A typed value of `field` (criterion 7): mm or degrees, each text held to its own range.
    pub fn set_text_layout_text(&mut self, field: LayoutField, text: &str) -> LayoutOutcome {
        match parse_field_text(field, text) {
            Some(value) => self.set_text_layout_value(field, Some(value)),
            None => LayoutOutcome::Invalid,
        }
    }

    /// Sets `field` to `value` on every subject text in one step (criteria 12 to 14).
    fn set_text_layout_value(&mut self, field: LayoutField, value: Option<f64>) -> LayoutOutcome {
        let Some(label) = field.label() else {
            return self.set_kerning_value(field, value, false);
        };
        self.text.layout_preview = None;
        self.recording(|this| {
            let texts = this.bar_texts();
            if texts.is_empty() {
                return LayoutOutcome::Unchanged;
            }
            let subjects: Vec<LayoutSubject> = texts
                .iter()
                .map(|text| LayoutSubject {
                    size_mm: text.size.as_mm(),
                    values: text.layout,
                    natural_line_mm: 0.0,
                    kern: CharValues::DEFAULT,
                })
                .collect();
            let LayoutChange {
                values,
                limited,
                limit,
                ..
            } = set_field(field, &subjects, value);
            let writes: Vec<(NodeId, TextLayoutValues, Affine)> = texts
                .iter()
                .zip(&values)
                .filter(|(text, values)| text.layout != **values)
                .map(|(text, values)| (text.id, *values, text.placement))
                .collect();
            if writes.is_empty() && limited == 0 {
                return LayoutOutcome::Unchanged;
            }
            this.write_layout(&writes, label);
            match (limit, texts.len()) {
                // The taper's range is the same for every text: its limit is one value.
                (Some((top, mm)), 1) => LayoutOutcome::Clamped(top, mm),
                (Some((top, percent)), _) if field == LayoutField::Taper => {
                    LayoutOutcome::Clamped(top, percent)
                }
                (Some(_), of) => LayoutOutcome::Limited { limited, of },
                (None, _) => LayoutOutcome::Set,
            }
        })
    }

    /// Writes new layout values and placements of subject texts: in edit mode to the session
    /// (and to the edited text's node, if it exists, as its own step), else to the selected texts
    /// in one step.
    pub(super) fn write_layout(
        &mut self,
        writes: &[(NodeId, TextLayoutValues, Affine)],
        label: TextLayoutLabel,
    ) {
        let Some(&(_, values, placement)) = writes.first() else {
            return;
        };
        if let Some(edit) = self.text.edit.as_mut() {
            edit.layout = values;
            edit.placement = placement;
            if let Some(id) = edit_id(edit) {
                let _ = self
                    .document
                    .set_text_layout(&[(id, values, placement)], label);
            }
            return;
        }
        let _ = self.document.set_text_layout(writes, label);
    }
}

const fn field_index(field: LayoutField) -> usize {
    match field {
        LayoutField::LetterSpacing => 0,
        LayoutField::WordSpacing => 1,
        LayoutField::LineSpacing => 2,
        LayoutField::CharacterRotation => 3,
        LayoutField::Taper => 4,
        LayoutField::HorizontalKerning => 5,
        LayoutField::VerticalKerning => 6,
        LayoutField::CharRotation => 7,
    }
}
