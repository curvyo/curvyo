//! The Adjustments group of the selected picture (`specs/0135` criteria 1, 2, 9, 25 to 30, 59,
//! 60; `ux-notes.md` §3 to §5, §9): the picture, its Image operations and the edits of its
//! registers, each one undo step.

mod before_after;
#[cfg(test)]
mod budget_tests;
#[cfg(test)]
mod contract_tests;
mod levels_input;
mod look;
mod picture;
#[cfg(test)]
mod tests;
mod view;

use curvyo_document_core::{AdjustmentsWrite, NodeId};
use curvyo_laser_core::adjust::{AdjustRegisters, DitherSettings, Preset, StepKind};
use curvyo_laser_core::settings::{Dither, ImageEdit, ImageField};
use curvyo_ui_core::{
    PICK_INTERVAL_UM, add_step, edit_field, make_editable, make_editable_notice, move_step,
    move_step_to, pick_preset, remove_step, set_dither, set_enabled, set_strength,
};

/// The Adjustments group's session state, never saved.
#[derive(Debug, Default)]
pub(in crate::session) struct AdjustState {
    /// The Strength shown on a picture while its slider is held: the views and the job read it,
    /// the release writes it as one step (criterion 26).
    held_strength: Option<(NodeId, u8)>,
    /// The notice of the last edit ("Numbers fixed for Image 1, 0.10 mm."), once.
    notice: Option<String>,
    /// The Before and after pane.
    pane: before_after::PaneState,
    /// The canvas looks of the adjusted pictures (criterion 15).
    look: look::LookCache,
    /// The inputs of the Levels rows (criterion 16).
    levels: levels_input::LevelsInputs,
}

impl super::Session {
    /// The Strength held on a picture while its slider is dragged, for the job and the views.
    pub(in crate::session) fn held_adjust_strength(&self) -> Option<(NodeId, u8)> {
        self.image_adjust.held_strength
    }

    /// The notice of the last adjustment edit, once; `None` when it had none.
    pub fn take_adjust_notice(&mut self) -> Option<String> {
        self.image_adjust.notice.take()
    }

    /// Writes `write` on the selected picture as one step "Edit adjustments"; whether anything
    /// changed.
    fn write_adjustments(&mut self, write: &AdjustmentsWrite) -> bool {
        let Some(target) = self.adjust_target() else {
            return false;
        };
        self.document
            .edit_image_adjustments(target, write)
            .unwrap_or(false)
    }

    /// The stack shown on the selected picture.
    fn shown_steps(&self) -> Option<Vec<curvyo_document_core::AdjustEntry>> {
        let target = self.adjust_target()?;
        let shown = self.document.shown_adjustments(target).ok()?;
        Some(shown.adjustments.steps)
    }

    /// The dither the selected picture burns with (Floyd-Steinberg when it stores none).
    fn shown_dither(&self) -> Option<DitherSettings> {
        let target = self.adjust_target()?;
        let shown = self.document.shown_adjustments(target).ok()?;
        Some(AdjustRegisters::read(&shown.adjustments).dither)
    }

    /// A press of preset `key` in the Preset strip (criteria 25, 27; ADR 0072 amendment A3): the
    /// Preset and the picture's Dither in one step, Cell size and Line pitch sized by the
    /// Interval of the first Image operation the picture is in (0.10 mm with none). No operation
    /// is written and nothing is said.
    pub fn pick_adjust_preset(&mut self, key: &str) -> bool {
        let (Some(preset), Some(target), Some(current)) = (
            Preset::from_key(key),
            self.adjust_target(),
            self.shown_dither(),
        ) else {
            return false;
        };
        let interval_um = self
            .picture_operations(target)
            .first()
            .map_or(PICK_INTERVAL_UM, |op| op.interval_um);
        self.write_adjustments(&pick_preset(preset, current, interval_um))
    }

    /// A press of mode `mode` in the picture's Dither strip (`ux-notes.md` §3.10): the whole
    /// Dither register, Preset and Strength kept, one step "Edit adjustments"; `false` for an
    /// unknown mode or the mode it has.
    pub fn set_picture_dither(&mut self, mode: &str) -> bool {
        let (Some(mode), Some(current)) = (Dither::from_key(mode), self.shown_dither()) else {
            return false;
        };
        set_dither(current, ImageEdit::Dither(mode)).is_some_and(|w| self.write_adjustments(&w))
    }

    /// A field commit of the picture's dither (`threshold`, `screen_size`, `screen_angle`),
    /// `value` in the field's shown unit: one step.
    ///
    /// # Errors
    /// "Enter a number." for an unknown field or no number; the field's range text out of range.
    pub fn edit_picture_dither_field(&mut self, field: &str, value: f64) -> Result<bool, String> {
        let field = match field {
            "threshold" => ImageField::Threshold,
            "screen_size" => ImageField::ScreenSize,
            "screen_angle" => ImageField::ScreenAngle,
            _ => return Err(curvyo_laser_core::settings::ENTER_NUMBER.to_owned()),
        };
        let edit = field.typed(value).map_err(str::to_owned)?;
        let Some(current) = self.shown_dither() else {
            return Ok(false);
        };
        Ok(set_dither(current, edit).is_some_and(|w| self.write_adjustments(&w)))
    }

    /// A typed Strength or one key press on the slider (criterion 26): one step.
    ///
    /// # Errors
    /// "Strength is 0 to 100 %." for a value out of range.
    pub fn set_adjust_strength(&mut self, value: f64) -> Result<bool, &'static str> {
        let write = set_strength(value)?;
        self.image_adjust.held_strength = None;
        Ok(self.write_adjustments(&write))
    }

    /// A tick of a Strength drag (criterion 26): shown on the picture, its job and the views
    /// without writing; clamped to 0 to 100 and rounded to whole percent.
    pub fn hold_adjust_strength(&mut self, value: f64) {
        let Some(target) = self.adjust_target() else {
            return;
        };
        if !value.is_finite() {
            return;
        }
        // invariant: clamped to 0 to 100 first, exact in a u8.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let strength = value.clamp(0.0, 100.0).round() as u8;
        self.image_adjust.held_strength = Some((target, strength));
    }

    /// The release of a Strength drag: the held value as one step "Edit adjustments"; whether it
    /// changed the picture.
    pub fn release_adjust_strength(&mut self) -> bool {
        let Some((target, strength)) = self.image_adjust.held_strength.take() else {
            return false;
        };
        if self.adjust_target() != Some(target) {
            return false;
        }
        self.set_adjust_strength(f64::from(strength))
            .unwrap_or(false)
    }

    /// Escape during a Strength drag: the stored Strength shows again.
    pub fn cancel_adjust_strength(&mut self) {
        self.image_adjust.held_strength = None;
    }

    /// A press in the add toolbar (criterion 3): step `kind` at its pipeline place with its
    /// defaults at the shown operation's Interval; one step.
    pub fn add_adjust_step(&mut self, kind: &str) -> bool {
        let (Some(kind), Some(steps)) = (StepKind::from_key(kind), self.shown_steps()) else {
            return false;
        };
        let interval_um = self.shown_interval_um();
        let next = add_step(&steps, kind, interval_um);
        self.write_steps(next)
    }

    /// A field commit on row `index` (criterion 5), `typed` in the field's shown unit.
    ///
    /// # Errors
    /// The field's range text, or the Levels order text; nothing is written.
    pub fn edit_adjust_field(
        &mut self,
        index: usize,
        key: &str,
        typed: f64,
    ) -> Result<bool, &'static str> {
        let Some(steps) = self.shown_steps() else {
            return Ok(false);
        };
        Ok(match edit_field(&steps, index, key, typed)? {
            Some(next) => self.write_steps(next),
            None => false,
        })
    }

    /// The power toggle of row `index` (criterion 2).
    pub fn set_adjust_step_enabled(&mut self, index: usize, on: bool) -> bool {
        self.shown_steps()
            .and_then(|steps| set_enabled(&steps, index, on))
            .is_some_and(|next| self.write_steps(next))
    }

    /// The up or down arrow of row `index`, inside its band (criterion 3).
    pub fn move_adjust_step(&mut self, index: usize, up: bool) -> bool {
        self.shown_steps()
            .and_then(|steps| move_step(&steps, index, up))
            .is_some_and(|next| self.write_steps(next))
    }

    /// A drag of row `from` landing at index `to` inside its band: one step (criterion 2).
    pub fn move_adjust_step_to(&mut self, from: usize, to: usize) -> bool {
        self.shown_steps()
            .and_then(|steps| move_step_to(&steps, from, to))
            .is_some_and(|next| self.write_steps(next))
    }

    /// The second press of row `index`'s armed trash (criterion 2).
    pub fn remove_adjust_step(&mut self, index: usize) -> bool {
        self.shown_steps()
            .and_then(|steps| remove_step(&steps, index))
            .is_some_and(|next| self.write_steps(next))
    }

    fn write_steps(&mut self, steps: Vec<curvyo_document_core::AdjustEntry>) -> bool {
        self.write_adjustments(&AdjustmentsWrite {
            steps: Some(steps),
            ..AdjustmentsWrite::default()
        })
    }

    /// Make editable (criterion 29): the steps that run at the shown operation's Interval become
    /// the manual stack, Preset Expert, one step; with more than one operation the notice
    /// "Numbers fixed for `operation`, `interval` mm.".
    pub fn make_adjust_editable(&mut self) -> bool {
        let Some(target) = self.adjust_target() else {
            return false;
        };
        let Some(resolved) = self.resolved_picture(target) else {
            return false;
        };
        let Some(steps) = self.shown_steps() else {
            return false;
        };
        let write = make_editable(&steps, &resolved.steps);
        let done = self.write_adjustments(&write);
        if done {
            let operations = self.picture_operations(target);
            let shown = self.shown_operation(&operations);
            self.image_adjust.notice = make_editable_notice(
                operations.len(),
                shown.map_or("", |op| op.name.as_str()),
                resolved.interval_um,
            );
        }
        done
    }

    /// Restore original (criterion 59): one step "Restore original", the Dither back to
    /// Floyd-Steinberg; on a clone its own adjustments become empty.
    pub fn restore_original_adjustments(&mut self) -> bool {
        self.image_adjust.held_strength = None;
        self.adjust_target()
            .is_some_and(|target| self.document.restore_original(target).unwrap_or(false))
    }

    /// Own on the clone's Adjustments override row (criterion 9): a full copy of the original's
    /// Preset, Strength, stack and Dither, one step; `false` on an image or a clone with its own.
    pub fn own_adjustments(&mut self) -> bool {
        self.image_adjust.held_strength = None;
        self.adjust_target()
            .is_some_and(|target| self.document.own_adjustments(target).unwrap_or(false))
    }

    /// Original's on the clone's Adjustments override row (criterion 9): Follow original, one
    /// step; `false` on an image or a clone that follows already.
    pub fn follow_original_adjustments(&mut self) -> bool {
        self.image_adjust.held_strength = None;
        self.adjust_target().is_some_and(|target| {
            self.document
                .follow_original_adjustments(target)
                .unwrap_or(false)
        })
    }
}
