//! Copy and Paste adjustments (`specs/0135` criterion 10, `ux-notes.md` §5, P16, P35): the four
//! registers of one picture held in the app and written onto another in one step "Paste
//! adjustments".

use curvyo_document_core::{AdjustEntry, AdjustmentsWrite, EXPERT_PRESET, ImageAdjustments};
use curvyo_laser_core::adjust::DitherSettings;

/// What Copy adjustments holds: Preset, Strength, the stack and the Dither as the source shows
/// them (an absent Dither held as the Floyd-Steinberg defaults, so a paste writes it).
#[derive(Debug, Clone, PartialEq)]
pub struct CopiedAdjustments {
    preset: String,
    strength: u8,
    steps: Vec<AdjustEntry>,
    dither: curvyo_document_core::StoredDither,
}

impl CopiedAdjustments {
    /// Copy adjustments of a picture showing `shown`.
    #[must_use]
    pub fn of(shown: &ImageAdjustments) -> Self {
        Self {
            preset: shown
                .preset
                .clone()
                .unwrap_or_else(|| EXPERT_PRESET.to_owned()),
            strength: shown.strength,
            steps: shown.steps.clone(),
            dither: shown
                .dither
                .clone()
                .unwrap_or_else(|| DitherSettings::DEFAULT.stored()),
        }
    }

    /// The write of Paste adjustments: it replaces the target's Preset, Strength, stack and
    /// Dither.
    #[must_use]
    pub fn write(&self) -> AdjustmentsWrite {
        AdjustmentsWrite {
            preset: Some(self.preset.clone()),
            strength: Some(self.strength),
            steps: Some(self.steps.clone()),
            dither: Some(self.dither.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paste_writes_all_four_registers_with_the_default_dither_when_none_was_stored() {
        let shown = ImageAdjustments {
            preset: Some("photo".to_owned()),
            strength: 70,
            ..ImageAdjustments::default()
        };
        let write = CopiedAdjustments::of(&shown).write();
        assert_eq!(write.preset.as_deref(), Some("photo"));
        assert_eq!(write.strength, Some(70));
        assert_eq!(write.steps, Some(Vec::new()));
        assert_eq!(write.dither, Some(DitherSettings::DEFAULT.stored()));
        let expert = CopiedAdjustments::of(&ImageAdjustments::default()).write();
        assert_eq!(expert.preset.as_deref(), Some(EXPERT_PRESET));
    }
}
