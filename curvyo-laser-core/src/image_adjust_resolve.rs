//! The effective pixel steps of one picture at one Interval (ADR 0073 §6; `specs/0135` criteria
//! 28 to 30, 38, 40, 60, 61).

use curvyo_document_core::Point;
use curvyo_image_core::{Adjusted, Analysis, PixelStep, WorkingImage, run_steps};

use crate::image_adjust_fields::ManualStep;
use crate::image_dither::DitherSettings;
use crate::image_preset::Preset;
use crate::job_raster_sample::pixel_pitch_mm;

/// The Strength of a picture that never set one (ADR 0072 §1).
pub const DEFAULT_STRENGTH: u8 = 50;

/// One entry of a picture's stack as read: switched on or off, and the step or why it does not
/// run (unknown kind, damaged field; kept, never run, ADR 0072 §5).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StoredStep {
    /// Whether the step is switched on.
    pub enabled: bool,
    /// The step, `None` when it cannot run.
    pub step: Option<ManualStep>,
}

/// The adjustment registers of a picture as read (ADR 0072 §1, amendment A1): Preset, Strength,
/// stack and the dither it burns with.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdjustRegisters {
    /// The preset; Expert when absent or damaged.
    pub preset: Preset,
    /// The Strength, 0 to 100.
    pub strength: u8,
    /// The stack in stored order.
    pub steps: Vec<StoredStep>,
    /// The dither the picture burns with; no derived step reads it (criterion 27).
    pub dither: DitherSettings,
    /// How many registers did not read (a damaged register, an unknown preset; ADR 0072 §5);
    /// each counts as one step that could not run.
    pub damaged_registers: usize,
}

impl Default for AdjustRegisters {
    fn default() -> Self {
        Self {
            preset: Preset::Expert,
            strength: DEFAULT_STRENGTH,
            steps: Vec::new(),
            dither: DitherSettings::DEFAULT,
            damaged_registers: 0,
        }
    }
}

impl AdjustRegisters {
    /// Whether anything would change the picture (Restore original is enabled, criterion 59).
    #[must_use]
    pub fn is_adjusted(&self) -> bool {
        self.preset != Preset::Expert || !self.steps.is_empty()
    }

    /// The steps that run at `interval_um`, in stored units: with Expert the enabled manual steps
    /// that read; with another preset its derived steps in place of the manual tone steps
    /// (criterion 30; every slice-1 kind is a tone step). `analysis` is asked only for a preset.
    #[must_use]
    pub fn effective_steps(
        &self,
        interval_um: u32,
        analysis: impl FnOnce() -> Analysis,
    ) -> Vec<ManualStep> {
        if self.preset == Preset::Expert {
            return self
                .steps
                .iter()
                .filter(|s| s.enabled)
                .filter_map(|s| s.step)
                .collect();
        }
        self.preset.derive(self.strength, &analysis(), interval_um)
    }

    /// The number of stored steps that cannot run, the damaged registers included (criterion
    /// 23).
    #[must_use]
    pub fn broken_steps(&self) -> usize {
        self.steps.iter().filter(|s| s.step.is_none()).count() + self.damaged_registers
    }
}

/// `steps` in pixel units for a picture of `width` x `height` pixels placed in `frame` (page mm,
/// top-left, top-right, bottom-right, bottom-left): mm become 1/16 px by the pixel pitch along
/// the picture's longer page axis (ADR 0073 §2).
#[must_use]
pub fn pixel_steps(
    steps: &[ManualStep],
    frame: &[Point; 4],
    width: u32,
    height: u32,
) -> Vec<PixelStep> {
    let pitch = pixel_pitch_mm(frame, width, height);
    steps.iter().map(|s| s.pixel_step(pitch)).collect()
}

/// The adjusted image of `image` placed in `frame` with `registers` at `interval_um`, and the
/// steps that ran in stored units (what Make editable writes, criterion 29). Nothing to run is
/// [`Adjusted::Unchanged`], so the empty stack is the `0123` input byte for byte.
/// A test hook for the goldens; the job reads the cache instead.
#[doc(hidden)]
#[must_use]
pub fn adjust_picture(
    image: &WorkingImage,
    frame: &[Point; 4],
    registers: &AdjustRegisters,
    interval_um: u32,
) -> (Vec<ManualStep>, Adjusted) {
    let steps = registers.effective_steps(interval_um, || Analysis::of(image));
    let pixel = pixel_steps(&steps, frame, image.width, image.height);
    let adjusted = run_steps(image, &pixel);
    (steps, adjusted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_adjust_fields::StepKind;

    fn image() -> WorkingImage {
        WorkingImage {
            width: 16,
            height: 16,
            pixels: (0..256_u32)
                .flat_map(|i| {
                    let g = u8::try_from(i % 16 * 16).unwrap();
                    [g, g, g, 255]
                })
                .collect(),
            original_size: (16, 16),
            truncated: false,
            animated: false,
        }
    }

    fn frame() -> [Point; 4] {
        [
            Point::new(0.0, 0.0),
            Point::new(1.6, 0.0),
            Point::new(1.6, 1.6),
            Point::new(0.0, 1.6),
        ]
    }

    #[test]
    fn an_empty_stack_is_unchanged() {
        let (steps, adjusted) =
            adjust_picture(&image(), &frame(), &AdjustRegisters::default(), 100);
        assert_eq!(steps, Vec::new());
        assert_eq!(adjusted, Adjusted::Unchanged);
        assert!(!AdjustRegisters::default().is_adjusted());
    }

    #[test]
    fn expert_runs_the_enabled_steps_that_read() {
        let levels = StepKind::Levels.default_step(100);
        let sharpen = StepKind::Sharpen.default_step(100);
        let registers = AdjustRegisters {
            steps: vec![
                StoredStep {
                    enabled: true,
                    step: Some(levels),
                },
                StoredStep {
                    enabled: false,
                    step: Some(sharpen),
                },
                StoredStep {
                    enabled: true,
                    step: None,
                },
            ],
            ..AdjustRegisters::default()
        };
        assert_eq!(
            registers.effective_steps(100, || unreachable!()),
            vec![levels]
        );
        assert_eq!(registers.broken_steps(), 1);
    }

    #[test]
    fn a_preset_replaces_the_manual_tone_steps() {
        let registers = AdjustRegisters {
            preset: Preset::Graphics,
            strength: 50,
            steps: vec![StoredStep {
                enabled: true,
                step: Some(StepKind::Levels.default_step(100)),
            }],
            ..AdjustRegisters::default()
        };
        let (steps, adjusted) = adjust_picture(&image(), &frame(), &registers, 100);
        let kinds: Vec<StepKind> = steps.iter().map(ManualStep::kind).collect();
        assert_eq!(
            kinds,
            [StepKind::Denoise, StepKind::Levels, StepKind::Sharpen]
        );
        assert!(matches!(adjusted, Adjusted::Grey(_)));
    }

    #[test]
    fn make_editable_runs_the_same_steps() {
        let preset = AdjustRegisters {
            preset: Preset::Photo,
            strength: 70,
            steps: Vec::new(),
            ..AdjustRegisters::default()
        };
        let (steps, before) = adjust_picture(&image(), &frame(), &preset, 100);
        let editable = AdjustRegisters {
            preset: Preset::Expert,
            strength: 70,
            steps: steps
                .iter()
                .map(|&s| StoredStep {
                    enabled: true,
                    step: Some(s),
                })
                .collect(),
            ..AdjustRegisters::default()
        };
        let (_, after) = adjust_picture(&image(), &frame(), &editable, 100);
        assert_eq!(before, after, "criterion 29: byte for byte");
    }
}
