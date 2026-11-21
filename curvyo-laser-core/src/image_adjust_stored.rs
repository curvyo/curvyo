//! A picture's adjustment registers translated between the document's stored shape and the typed
//! steps (ADR 0072 §1, §5; `specs/0135` criteria 8, 23).

use curvyo_document_core::{AdjustEntry, AdjustStep, ImageAdjustments};

use crate::image_adjust_fields::ManualStep;
use crate::image_adjust_resolve::{AdjustRegisters, StoredStep};
use crate::image_dither::DitherSettings;
use crate::image_preset::Preset;

impl AdjustRegisters {
    /// The registers of a picture as stored: an absent preset is Expert, an unknown one reads as
    /// Expert and counts as one step that could not run, as each damaged register does; an entry
    /// whose kind this build does not know, whose field is missing or out of range, or that is
    /// not of the step shape is kept and never runs (ADR 0072 §5); a damaged Dither reads as
    /// Floyd-Steinberg and counts once (amendment A5).
    #[must_use]
    pub fn read(stored: &ImageAdjustments) -> Self {
        let mut damaged = stored.damage.count();
        let preset = match stored.preset.as_deref() {
            None => Preset::Expert,
            Some(key) => Preset::from_key(key).unwrap_or_else(|| {
                damaged += 1;
                Preset::Expert
            }),
        };
        let steps = stored
            .steps
            .iter()
            .map(|entry| match entry {
                AdjustEntry::Step(step) => StoredStep {
                    enabled: step.enabled,
                    step: ManualStep::read(&step.kind, |key| step.fields.get(key).copied()).ok(),
                },
                AdjustEntry::Unreadable(_) => StoredStep {
                    enabled: false,
                    step: None,
                },
            })
            .collect();
        let (dither, dither_damaged) = DitherSettings::read(stored.dither.as_ref());
        damaged += usize::from(dither_damaged);
        Self {
            preset,
            strength: stored.strength,
            steps,
            dither,
            damaged_registers: damaged,
        }
    }
}

impl ManualStep {
    /// The step in the document's stored shape, switched on or off.
    #[must_use]
    pub fn entry(&self, enabled: bool) -> AdjustEntry {
        AdjustEntry::Step(AdjustStep {
            kind: self.kind().key().to_owned(),
            enabled,
            fields: self
                .fields()
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::AdjustDamage;
    use serde_json::json;

    use super::*;
    use crate::image_adjust_fields::StepKind;

    fn entry(kind: &str, fields: &[(&str, i64)]) -> AdjustEntry {
        AdjustEntry::Step(AdjustStep {
            kind: kind.to_owned(),
            enabled: true,
            fields: fields.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
        })
    }

    #[test]
    fn absent_registers_read_as_the_defaults() {
        assert_eq!(
            AdjustRegisters::read(&ImageAdjustments::default()),
            AdjustRegisters::default()
        );
    }

    #[test]
    fn every_kind_round_trips_through_the_stored_shape() {
        for kind in StepKind::ALL {
            let step = kind.default_step(100);
            let stored = ImageAdjustments {
                preset: Some("photo".into()),
                strength: 20,
                steps: vec![step.entry(true), step.entry(false)],
                ..ImageAdjustments::default()
            };
            let read = AdjustRegisters::read(&stored);
            assert_eq!(read.preset, Preset::Photo);
            assert_eq!(read.strength, 20);
            assert_eq!(
                read.steps,
                vec![
                    StoredStep {
                        enabled: true,
                        step: Some(step)
                    },
                    StoredStep {
                        enabled: false,
                        step: Some(step)
                    }
                ]
            );
            assert_eq!(read.broken_steps(), 0);
        }
    }

    /// Criterion 23, ADR 0072 §5: one count per damage reason, nothing dropped.
    #[test]
    fn each_damage_reason_counts_one_step_that_could_not_run() {
        let stored = ImageAdjustments {
            preset: Some("sepia".into()),
            strength: 50,
            steps: vec![
                entry("vignette", &[("amount_pct", 5)]),
                entry("levels", &[("black_pct", 3), ("white_pct", 97)]),
                entry(
                    "sharpen",
                    &[
                        ("radius_um", 100),
                        ("amount_pct", 501),
                        ("threshold_pct", 0),
                    ],
                ),
                AdjustEntry::Unreadable(json!(12)),
                StepKind::Levels.default_step(100).entry(true),
            ],
            damage: AdjustDamage {
                preset: false,
                strength: true,
                stack: false,
                dither: false,
            },
            dither: Some(curvyo_document_core::StoredDither {
                mode: "halftone_x".into(),
                fields: std::collections::BTreeMap::new(),
            }),
        };
        let read = AdjustRegisters::read(&stored);
        assert_eq!(
            read.preset,
            Preset::Expert,
            "an unknown preset reads as Expert"
        );
        assert_eq!(read.steps.len(), 5, "every entry is kept");
        assert_eq!(
            read.broken_steps(),
            4 + 3,
            "four entries, the preset, the Strength and the Dither"
        );
        assert_eq!(read.dither, DitherSettings::DEFAULT);
        assert_eq!(
            read.effective_steps(100, |_| unreachable!()),
            vec![StepKind::Levels.default_step(100)]
        );
    }
}
