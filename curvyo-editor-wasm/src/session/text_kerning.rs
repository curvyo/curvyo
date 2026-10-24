//! `Session`'s glue for the per-character fields (`specs/0079-text-layout-controls` slice 4,
//! criteria 26 to 29, 27b, 27c; ADR 0021): which characters the kerning and per-character rotation
//! fields act on, their live preview, the step a change writes, the reset, "All characters" and
//! the Alt+arrow keys. The rules live in `curvyo_ui_core`
//! (`text_layout_fields`, `TextEdit`).

use curvyo_document_core::{
    CharValues, NodeId, TextChars, TextCharsLabel, TextLayoutValues, TextSnapshot, TextWritingMode,
};
use curvyo_ui_core::{LayoutField, LayoutSubject, TextInput, field_view, set_field};

use super::Session;
use super::text::{NEW_TEXT_ID, edit_id};
use super::text_layout::LayoutOutcome;

/// Alt+arrow steps (criterion 29): 0.1 mm, Shift 1 mm.
const KEY_STEP_MM: f64 = 0.1;
const COARSE_KEY_STEP_MM: f64 = 1.0;

/// A text's new values: all of them, and the subject characters' that changed.
type KernWrite = (NodeId, TextChars, Vec<(usize, CharValues)>);

/// The characters of one subject text a kerning field acts on.
struct KernTarget {
    text: TextSnapshot,
    /// The characters and their values.
    chars: Vec<(usize, CharValues)>,
}

/// Every character of `text` but its line breaks, with its values.
fn every_char(text: &TextSnapshot) -> Vec<(usize, CharValues)> {
    text.string
        .chars()
        .enumerate()
        .filter(|(_, c)| *c != '\n')
        .map(|(index, _)| (index, text.chars.get(index)))
        .collect()
}

/// `chars` with `values` set on their characters.
fn with_values(chars: &TextChars, values: &[(usize, CharValues)]) -> TextChars {
    TextChars::new(
        chars
            .entries()
            .iter()
            .copied()
            .chain(values.iter().copied()),
    )
}

impl Session {
    /// The subjects of a per-character field (criteria 27, 28, 27c): in edit mode the character
    /// after the caret or the selection's, or with `all` every character of the edited text; else
    /// every character of the selected texts.
    fn kern_targets(&self, all: bool) -> Vec<KernTarget> {
        if let Some(edit) = self.text.edit.as_ref() {
            let text = edit.snapshot(edit_id(edit).unwrap_or(NEW_TEXT_ID));
            let chars = if all {
                every_char(&text)
            } else {
                edit.kern_subject()
            };
            return vec![KernTarget { text, chars }];
        }
        self.bar_texts()
            .into_iter()
            .map(|text| KernTarget {
                chars: every_char(&text),
                text,
            })
            .collect()
    }

    /// The live kerning value of a drag or an arrow on a kerning field, while its subject is the
    /// one it started on.
    fn live_kern_preview(&self) -> Option<(LayoutField, Option<f64>)> {
        self.live_layout_preview()
            .filter(|(field, _)| field.is_per_character())
    }

    /// The new values of `target`'s subject characters when `field` is set to `value` (mm), each
    /// held to its range.
    fn kerned_chars(
        field: LayoutField,
        target: &KernTarget,
        value: Option<f64>,
    ) -> Vec<(usize, CharValues)> {
        let subjects: Vec<LayoutSubject> = target
            .chars
            .iter()
            .map(|(_, kern)| kern_subject(&target.text, *kern))
            .collect();
        let change = set_field(field, &subjects, value);
        target
            .chars
            .iter()
            .zip(change.kerns)
            .map(|((index, _), kern)| (*index, kern))
            .collect()
    }

    /// The per-character values `text` is drawn with now: a kerning field's live value applied to
    /// its subject characters (`text`'s own values without one).
    pub(super) fn previewed_chars(&self, text: &TextSnapshot) -> TextChars {
        let Some((field, value)) = self.live_kern_preview() else {
            return text.chars.clone();
        };
        let chars = match self.text.edit.as_ref() {
            Some(edit) => edit.kern_subject(),
            None => every_char(text),
        };
        let target = KernTarget {
            text: text.clone(),
            chars,
        };
        with_values(&text.chars, &Self::kerned_chars(field, &target, value))
    }

    /// The subjects of a kerning field, one per character, with a live value applied; one at 0
    /// when there is no character (the end of a line, criterion 27).
    pub(super) fn kern_layout_subjects(&self) -> Vec<LayoutSubject> {
        let targets = self.kern_targets(false);
        let mut subjects: Vec<LayoutSubject> = targets
            .iter()
            .flat_map(|target| {
                let chars = self.previewed_chars(&target.text);
                target
                    .chars
                    .iter()
                    .map(move |(index, _)| kern_subject(&target.text, chars.get(*index)))
                    .collect::<Vec<_>>()
            })
            .collect();
        if subjects.is_empty()
            && let Some(target) = targets.first()
        {
            subjects.push(kern_subject(&target.text, CharValues::DEFAULT));
        }
        subjects
    }

    /// "All characters" (criterion 27c): the field's value as shown sets every character of the
    /// subject texts, also in edit mode, in one step; nothing while the field is Mixed.
    pub fn apply_text_layout_to_all(&mut self, field: LayoutField) -> LayoutOutcome {
        if !field.is_per_character() {
            return LayoutOutcome::Unchanged;
        }
        let view = field_view(field, &self.kern_layout_subjects());
        if view.mixed {
            return LayoutOutcome::Unchanged;
        }
        self.set_kerning_value(field, Some(view.value), true)
    }

    /// Sets a per-character field to `value` (mm or degrees; `None` is 0) on its subject
    /// characters (every character of the text with `all`): in edit mode to the session and the
    /// edited node, else to the selected texts, in one step "Change kerning" or "Change
    /// per-character rotation" (criteria 27 to 29, 27b).
    pub(super) fn set_kerning_value(
        &mut self,
        field: LayoutField,
        value: Option<f64>,
        all: bool,
    ) -> LayoutOutcome {
        self.text.layout_preview = None;
        let label = field.chars_label(false).unwrap_or(TextCharsLabel::Kerning);
        self.recording(|this| {
            let targets = this.kern_targets(all);
            // The clamps per text: a text's characters share its Size.
            let per_text: Vec<LayoutSubject> = targets
                .iter()
                .filter(|t| !t.chars.is_empty())
                .map(|t| kern_subject(&t.text, CharValues::DEFAULT))
                .collect();
            if per_text.is_empty() {
                return LayoutOutcome::Unchanged;
            }
            let held = set_field(field, &per_text, value);
            let writes: Vec<KernWrite> = targets
                .iter()
                .filter_map(|t| {
                    let kerned = Self::kerned_chars(field, t, value);
                    let changed = kerned.iter().zip(&t.chars).any(|(new, old)| new.1 != old.1);
                    changed.then(|| (t.text.id, with_values(&t.text.chars, &kerned), kerned))
                })
                .collect();
            if writes.is_empty() && held.limited == 0 {
                return LayoutOutcome::Unchanged;
            }
            this.write_kerns(&writes, label);
            match (held.limit, per_text.len()) {
                (Some((top, mm)), 1) => LayoutOutcome::Clamped(top, mm),
                (Some(_), of) => LayoutOutcome::Limited {
                    limited: held.limited,
                    of,
                },
                (None, _) => LayoutOutcome::Set,
            }
        })
    }

    /// Writes new character values: in edit mode to the session and, for an existing text, to its
    /// node as its own step (the session's string first, if it was typed in); else to the
    /// selected texts in one step.
    fn write_kerns(&mut self, writes: &[KernWrite], label: TextCharsLabel) {
        if let Some(edit) = self.text.edit.as_mut() {
            let Some((_, _, kerned)) = writes.first() else {
                return;
            };
            let before = edit.chars();
            edit.set_chars(kerned);
            if let Some(id) = edit_id(edit) {
                let typed = self
                    .document
                    .text(id)
                    .is_some_and(|stored| stored.string != edit.string());
                if typed {
                    let _ = self.document.edit_text(id, edit.string(), &before);
                }
                let written = self.document.set_text_chars(&[(id, edit.chars())], label);
                if written.is_ok() {
                    edit.mark_written();
                }
            }
            return;
        }
        let texts: Vec<(NodeId, TextChars)> = writes
            .iter()
            .map(|(id, chars, _)| (*id, chars.clone()))
            .collect();
        let _ = self.document.set_text_chars(&texts, label);
    }

    /// The reset slot of a per-character field (criterion 28): outside edit mode every manual
    /// kern of the selected texts goes, in one step "Clear kerning" (or every per-character
    /// rotation, "Clear per-character rotation"; each leaves the other's values); in edit mode the
    /// field's value of its characters goes back to 0.
    pub(super) fn reset_kerning(&mut self, field: LayoutField) -> LayoutOutcome {
        if self.text.edit.is_some() {
            return self.set_kerning_value(field, None, false);
        }
        let label = field
            .chars_label(true)
            .unwrap_or(TextCharsLabel::ClearKerning);
        let cleared = |values: CharValues| match field {
            LayoutField::CharRotation => CharValues {
                rotation: 0.0,
                ..values
            },
            _ => CharValues {
                kern_x: 0.0,
                kern_y: 0.0,
                ..values
            },
        };
        self.text.layout_preview = None;
        self.recording(|this| {
            let writes: Vec<(NodeId, TextChars)> = this
                .bar_texts()
                .into_iter()
                .filter_map(|text| {
                    let entries = text.chars.entries().iter();
                    let kept = TextChars::new(entries.map(|(at, values)| (*at, cleared(*values))));
                    (kept != text.chars).then_some((text.id, kept))
                })
                .collect();
            if writes.is_empty() {
                return LayoutOutcome::Unchanged;
            }
            let _ = this.document.set_text_chars(&writes, label);
            LayoutOutcome::Set
        })
    }

    /// Alt+Left, Alt+Right (`right` −1, 1) and Alt+Down, Alt+Up (`up` −1, 1) in edit mode
    /// (criterion 29): 0.1 mm per press, 1 mm with Shift, part of the edit session. In horizontal
    /// text Left/Right change the kern along the line and Up/Down across it; in vertical text
    /// Up/Down change it along the column (Down is +) and Left/Right across it (as the caret keys
    /// of criterion 24). Returns the readout of the character after the caret ("Kern +0.3,
    /// 0.0 mm", UX notes §10.3), empty when nothing changed.
    pub fn text_kern(&mut self, right: i32, up: i32, coarse: bool) -> String {
        let step = if coarse {
            COARSE_KEY_STEP_MM
        } else {
            KEY_STEP_MM
        };
        let vertical = self
            .text
            .edit
            .as_ref()
            .is_some_and(|edit| edit.layout.writing_mode != TextWritingMode::Horizontal);
        let (along, across) = if vertical { (-up, right) } else { (right, up) };
        let changed = self
            .text_input(TextInput::Kern {
                along_mm: f64::from(along) * step,
                across_mm: f64::from(across) * step,
            })
            .changed;
        let Some(edit) = self.text.edit.as_ref().filter(|_| changed) else {
            return String::new();
        };
        edit.kern_subject()
            .first()
            .map(|(_, kern)| kern_readout(*kern, edit.size.as_mm()))
            .unwrap_or_default()
    }
}

/// A kerning field's view of one character of `text`.
fn kern_subject(text: &TextSnapshot, kern: CharValues) -> LayoutSubject {
    LayoutSubject {
        size_mm: text.size.as_mm(),
        values: TextLayoutValues::DEFAULT,
        natural_line_mm: 0.0,
        kern,
    }
}

/// "Kern +0.3, 0.0 mm": the horizontal then the vertical kern, mm with one decimal.
fn kern_readout(kern: CharValues, size_mm: f64) -> String {
    let mm = |factor: f64| {
        let value = (factor * size_mm * 10.0).round() / 10.0 + 0.0;
        if value > 0.0 {
            format!("+{value:.1}")
        } else {
            format!("{value:.1}").replace('-', "\u{2212}")
        }
    };
    format!("Kern {}, {} mm", mm(kern.kern_x), mm(kern.kern_y))
}
