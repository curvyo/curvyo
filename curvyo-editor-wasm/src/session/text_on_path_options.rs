//! Value changes of selected texts on path (`specs/0080-text-on-path` slice 1): what the On path
//! button shows, Flip, the arrow nudge, Release.

use curvyo_document_core::{NodeId, OnPathLabel, OnPathValues, TextSnapshot};

use super::Session;

/// What the On path button and the second row's dot show.
#[derive(Debug, Clone, PartialEq)]
pub struct TextPathView {
    /// The number of subject texts (enabled with exactly one, UX notes §0.6).
    pub texts: u32,
    /// Pick mode runs (the button is pressed).
    pub picking: bool,
    /// How many subject texts are on a path: Release from path shows when it is not 0.
    pub on_path: u32,
    /// A subject text on path is vertical: Writing mode and Glyph orientation are greyed.
    pub vertical: bool,
}

impl Session {
    /// The subject texts on path of the bar, as read.
    fn subject_texts_on_path(&self) -> Vec<TextSnapshot> {
        self.bar_texts()
            .iter()
            .filter_map(|t| self.read_text(t.id))
            .filter(|t| t.on_path.is_some())
            .collect()
    }

    /// What the On path button and the row-2 dot show (criteria 1, 48; UX notes §5).
    #[must_use]
    pub fn text_path_view(&self) -> TextPathView {
        let texts = self.bar_texts();
        let on = self.subject_texts_on_path();
        TextPathView {
            texts: u32::try_from(texts.len()).unwrap_or(u32::MAX),
            picking: self.text_path.pick.is_some(),
            on_path: u32::try_from(on.len()).unwrap_or(u32::MAX),
            vertical: on.iter().any(|t| !t.layout.writing_mode.is_horizontal()),
        }
    }

    /// Writes `change` of the values of every subject text on path as one step labelled `label`.
    fn change_text_paths(
        &mut self,
        label: OnPathLabel,
        change: impl Fn(&Self, &TextSnapshot, OnPathValues) -> OnPathValues,
    ) {
        self.recording(|this| {
            let texts = this.subject_texts_on_path();
            let writes: Vec<(NodeId, OnPathValues)> = texts
                .iter()
                .filter_map(|t| {
                    t.on_path
                        .as_ref()
                        .map(|p| (t.id, change(this, t, p.values)))
                })
                .collect();
            if !writes.is_empty() {
                let _ = this.document.set_text_path(&writes, label);
            }
        });
    }

    /// Whether every selected object is a text on path (then Flip toggles Mirror letters).
    pub(super) fn selection_is_texts_on_path(&self) -> bool {
        let ids = self.selection.ids();
        !ids.is_empty()
            && ids.iter().all(|id| {
                self.read_text(*id)
                    .is_some_and(|t| t.on_path.as_ref().is_some_and(|p| p.curve.is_some()))
            })
    }

    /// Flip on a text on path (criterion 19): Mirror letters toggles. Returns whether a text on
    /// path was among the selection (the Flip then wrote this instead).
    pub(super) fn flip_text_paths(&mut self) -> bool {
        if self.subject_texts_on_path().is_empty() {
            return false;
        }
        self.change_text_paths(OnPathLabel::Flip, |_, _, v| OnPathValues {
            mirror_letters: !v.mirror_letters,
            ..v
        });
        true
    }

    /// An arrow nudge of the one selected text on path (criterion 21, UX notes §0.7): Left and
    /// Right along the reading direction, Up and Down the Distance. Returns whether it acted.
    pub(super) fn nudge_text_path(&mut self, event: curvyo_ui_core::NudgeEvent) -> bool {
        let Some(text) = self.sole_text_on_path() else {
            return false;
        };
        if self.text.edit.is_some() {
            return false;
        }
        let Some(values) = text.on_path.as_ref().map(|p| p.values) else {
            return false;
        };
        let step = event.offset();
        let nudged = curvyo_ui_core::nudged_values(values, step.x, -step.y);
        let label = if step.x == 0.0 {
            OnPathLabel::Distance
        } else {
            OnPathLabel::Move
        };
        self.recording(|this| {
            let _ = this.document.set_text_path(&[(text.id, nudged)], label);
        });
        true
    }

    /// Release from path (criterion 23): each subject text becomes straight, its first letter
    /// where it was; the carrier stays a guide. One step.
    pub fn release_text_path(&mut self) {
        self.recording(|this| {
            let texts = this.subject_texts_on_path();
            let mut writes = Vec::new();
            for text in &texts {
                let placement = match (this.on_carrier(text), this.face_of(&text.font)) {
                    (Some(on), Some(face)) => {
                        let vertical = curvyo_ui_core::stored_vertical_layout(text, face);
                        curvyo_ui_core::release_placement(
                            text,
                            &on.laid,
                            &on.placement,
                            vertical.as_ref(),
                        )
                    }
                    _ => text.placement,
                };
                writes.push((text.id, placement));
            }
            if !writes.is_empty() && this.document.release_from_path(&writes).is_ok() {
                this.cancel_text_path_drag();
            }
        });
    }
}
