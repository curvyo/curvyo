//! `Session`'s glue for the Style group (`specs/0078-font-style-buttons`, `adrs.md` decisions 2
//! and 3): a weight or Italic press, a family pick that keeps the style, and the face a preview
//! tile shows; the rules live in `curvyo_ui_core::face_style_pick`.

use curvyo_document_core::{FontRef, NodeId};
use curvyo_text_core::WeightBand;
use curvyo_ui_core::{
    Pressed, StyleButtons, StylePress, family_target, family_tile, press_italic, press_weight,
    style_buttons,
};

use super::Session;
use super::text::edit_id;

/// A press or pick waiting for the bytes of its faces (criterion 17): applied, re-evaluated
/// against the then-current subject texts, once the last one has arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PendingStyle {
    /// A weight button.
    Weight(WeightBand),
    /// The Italic toggle.
    Italic,
}

/// A pick waiting for bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PendingPick {
    /// A face row: exactly that face (`0074` criterion 18).
    Face(String, u32),
    /// A family row or preview tile, by its picked face (criterion 22).
    Family(String, u32),
    /// A Style group press.
    Style(PendingStyle),
}

/// What a press did, for the notice of criteria 11 and 12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleOutcome {
    /// The texts in the pressed style afterwards.
    pub set: usize,
    /// The texts left as they were for want of a face.
    pub left: usize,
    /// Italic made the texts upright.
    pub upright: bool,
}

impl Session {
    /// The fonts the Style group acts on: the edited or selected texts', else the next text's.
    pub(super) fn style_subject_fonts(&self) -> Vec<FontRef> {
        let texts = self.bar_texts();
        if texts.is_empty() {
            self.next_text_font().into_iter().collect()
        } else {
            texts.into_iter().map(|text| text.font).collect()
        }
    }

    /// The six buttons' pressed and greyed states (criteria 3 to 9).
    #[must_use]
    pub fn style_buttons(&self) -> StyleButtons {
        style_buttons(self.fonts.faces(), &self.style_subject_fonts())
    }

    /// A press on the weight button of `band` (criteria 10 to 15): `None` when it is greyed or
    /// already pressed (nothing changes, no notice).
    pub fn press_weight(&mut self, band: WeightBand) -> Option<StyleOutcome> {
        let at = curvyo_ui_core::WEIGHT_STEPS
            .iter()
            .position(|step| *step == band)?;
        let button = &self.style_buttons().weights[at];
        if button.greyed.is_some() || button.pressed == Pressed::On {
            return None;
        }
        Some(self.press_style(PendingStyle::Weight(band)))
    }

    /// A press on Italic (criterion 12): `None` when it is greyed.
    pub fn press_italic(&mut self) -> Option<StyleOutcome> {
        if self.style_buttons().italic.greyed.is_some() {
            return None;
        }
        Some(self.press_style(PendingStyle::Italic))
    }

    fn style_press(&self, style: PendingStyle) -> StylePress {
        let fonts = self.style_subject_fonts();
        match style {
            PendingStyle::Weight(band) => press_weight(self.fonts.faces(), &fonts, band),
            PendingStyle::Italic => press_italic(self.fonts.faces(), &fonts),
        }
    }

    fn press_style(&mut self, style: PendingStyle) -> StyleOutcome {
        let press = self.style_press(style);
        let outcome = StyleOutcome {
            set: press.set,
            left: press.left,
            upright: press.upright,
        };
        self.apply_faces(&press.targets, PendingPick::Style(style));
        outcome
    }

    /// A press on a family row or a preview tile whose face is the listed face `face_index` of
    /// `host_id` (criteria 22 to 24): each subject text gets the family's face in its style
    /// where the family has it, else that face; one step "Change font".
    pub fn pick_font_family(&mut self, host_id: &str, face_index: u32) {
        let Some(picked) = self.fonts.position(host_id, face_index) else {
            return;
        };
        let targets: Vec<Option<usize>> = self
            .style_subject_fonts()
            .iter()
            .map(|font| Some(family_target(self.fonts.faces(), picked, font)))
            .collect();
        self.apply_faces(
            &targets,
            PendingPick::Family(host_id.to_owned(), face_index),
        );
    }

    /// The face a preview tile of the family of `host_id`'s face `face_index` shows and applies
    /// (criterion 24): its host id and index in the file.
    #[must_use]
    pub fn family_tile_face(&self, host_id: &str, face_index: u32) -> Option<(String, u32)> {
        let picked = self.fonts.position(host_id, face_index)?;
        let shown = family_tile(self.fonts.faces(), picked, &self.style_subject_fonts());
        let face = self.fonts.faces().get(shown)?;
        Some((face.host_id.clone(), face.index))
    }

    /// Gives each subject text (or the next text) the listed face of `targets` (`None` keeps it)
    /// in one step "Change font"; while a face is not loaded, asks for it and keeps `pending`.
    fn apply_faces(&mut self, targets: &[Option<usize>], pending: PendingPick) {
        self.recording(|this| {
            let mut fonts: Vec<Option<FontRef>> = Vec::with_capacity(targets.len());
            let mut loading = false;
            for target in targets {
                match target {
                    Some(index) => {
                        let font = this.fonts.font_of(*index);
                        if font.is_none() {
                            this.fonts.request(*index);
                            loading = true;
                        }
                        fonts.push(font);
                    }
                    None => fonts.push(None),
                }
            }
            if loading {
                this.text.pending_pick = Some(pending);
                return;
            }
            this.text.pending_pick = None;
            if let Some(first) = fonts.iter().flatten().next() {
                this.text.next_font = Some(first.clone());
            }
            if let Some(edit) = this.text.edit.as_mut()
                && let Some(Some(font)) = fonts.first()
            {
                edit.font = font.clone();
            }
            let ids: Vec<NodeId> = match this.text.edit.as_ref() {
                Some(edit) => edit_id(edit).into_iter().collect(),
                None => this.bar_text_ids(),
            };
            let pairs: Vec<(NodeId, FontRef)> = ids
                .into_iter()
                .zip(fonts)
                .filter_map(|(id, font)| Some((id, font?)))
                .filter(|(id, font)| this.document.text(*id).is_some_and(|t| t.font != *font))
                .collect();
            if !pairs.is_empty() {
                let _ = this.document.set_text_fonts(&pairs);
            }
        });
    }

    /// Applies a waiting pick or press after a file arrived.
    pub(super) fn retry_pending_pick(&mut self, arrived: &str) {
        match self.text.pending_pick.clone() {
            Some(PendingPick::Face(host_id, face_index)) if host_id == arrived => {
                self.pick_font_face(&host_id, face_index);
            }
            Some(PendingPick::Family(host_id, face_index)) => {
                self.pick_font_family(&host_id, face_index);
            }
            Some(PendingPick::Style(style)) => {
                let press = self.style_press(style);
                self.apply_faces(&press.targets, PendingPick::Style(style));
            }
            _ => {}
        }
    }
}
