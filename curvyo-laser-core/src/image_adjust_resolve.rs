//! The effective pixel steps of one picture at one Interval (ADR 0073 §6; `specs/0135` criteria
//! 28 to 30, 38, 40, 60, 61).

use curvyo_document_core::Point;
use curvyo_image_core::{
    Adjusted, Analysis, PixelStep, WorkingImage, crop_rect, resized_size, run_steps,
};

use crate::image_adjust_fields::{Band, ManualStep};
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

    /// The enabled manual steps that read, in zone order (ADR 0073 §3, stable): the geometry band
    /// first in its fixed order, then the tone steps in the stored order.
    fn enabled_in_zone_order(&self) -> Vec<ManualStep> {
        let mut steps: Vec<ManualStep> = self
            .steps
            .iter()
            .filter(|s| s.enabled)
            .filter_map(|s| s.step)
            .collect();
        steps.sort_by_key(|s| match s.kind().band() {
            Band::Geometry => s.kind() as usize,
            Band::Tone => usize::MAX,
        });
        steps
    }

    /// The enabled geometry steps (Crop, Resize) in their fixed order: they run with any preset
    /// (criterion 30), and the picture analysis reads the picture after them (criterion 39).
    #[must_use]
    pub fn geometry_steps(&self) -> Vec<ManualStep> {
        let mut steps = self.enabled_in_zone_order();
        steps.retain(|s| s.kind().band() == Band::Geometry);
        steps
    }

    /// The steps that run at `interval_um`, in stored units and zone order: with Expert the
    /// enabled manual steps that read; with another preset the geometry steps and then its
    /// derived steps in place of the manual tone steps (criterion 30). `analysis` is asked only
    /// for a preset, with the geometry steps the picture is analysed after.
    #[must_use]
    pub fn effective_steps(
        &self,
        interval_um: u32,
        analysis: impl FnOnce(&[ManualStep]) -> Analysis,
    ) -> Vec<ManualStep> {
        if self.preset == Preset::Expert {
            return self.enabled_in_zone_order();
        }
        let mut steps = self.geometry_steps();
        let derived = self
            .preset
            .derive(self.strength, &analysis(&steps), interval_um);
        steps.extend(derived);
        steps
    }

    /// The number of stored steps that cannot run, the damaged registers included (criterion
    /// 23).
    #[must_use]
    pub fn broken_steps(&self) -> usize {
        self.steps.iter().filter(|s| s.step.is_none()).count() + self.damaged_registers
    }
}

/// `steps` (in zone order) in pixel units for an input of `input` pixels, a level of a working
/// image of `full` pixels, whose visible part is drawn in `frame` (page mm, top-left, top-right,
/// bottom-right, bottom-left; the cropped part, ADR 0072 §3): Resize's width scales from the
/// working image to the input; the mm of the tone steps become 1/16 px by the pixel pitch of the
/// picture after the geometry steps along its longer page axis (ADR 0073 §2).
#[must_use]
pub fn pixel_steps(
    steps: &[ManualStep],
    frame: &[Point; 4],
    full: (u32, u32),
    input: (u32, u32),
) -> Vec<PixelStep> {
    let (mut w, mut h) = input;
    let mut out = Vec::with_capacity(steps.len());
    for step in steps {
        match *step {
            ManualStep::Crop { pct } => {
                let (_, _, cw, ch) = crop_rect(w, h, pct);
                (w, h) = (cw, ch);
                out.push(PixelStep::Crop { pct });
            }
            ManualStep::Resize { width_px } => {
                let scaled = (2 * u64::from(width_px) * u64::from(input.0)
                    + u64::from(full.0.max(1)))
                    / (2 * u64::from(full.0.max(1)));
                let width_px = u32::try_from(scaled.max(1)).unwrap_or(u32::MAX);
                (w, h) = resized_size(w, h, width_px);
                out.push(PixelStep::Resize { width_px });
            }
            _ => {}
        }
    }
    let pitch = pixel_pitch_mm(frame, w, h);
    out.extend(
        steps
            .iter()
            .filter(|s| s.kind().band() == Band::Tone)
            .map(|s| s.pixel_step(pitch)),
    );
    out
}

/// The picture analysis of `image` after the geometry steps `geometry` (criterion 39).
#[must_use]
pub fn analysis_after(image: &WorkingImage, geometry: &[ManualStep]) -> Analysis {
    let pixel: Vec<PixelStep> = geometry
        .iter()
        .filter(|s| s.kind().band() == Band::Geometry)
        .map(|s| s.pixel_step(1.0))
        .collect();
    match run_steps(image, &pixel) {
        Adjusted::Rgba(cut) => Analysis::of(&cut),
        Adjusted::Grey(plane) => Analysis::of_plane(&plane),
        Adjusted::Unchanged => Analysis::of(image),
    }
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
    let steps = registers.effective_steps(interval_um, |geometry| analysis_after(image, geometry));
    let size = (image.width, image.height);
    let pixel = pixel_steps(&steps, frame, size, size);
    let adjusted = run_steps(image, &pixel);
    (steps, adjusted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_adjust_fields::StepKind;
    use curvyo_image_core::PixelStep;

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
            registers.effective_steps(100, |_| unreachable!()),
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

    fn on(step: ManualStep) -> StoredStep {
        StoredStep {
            enabled: true,
            step: Some(step),
        }
    }

    /// Criteria 3, 30: the stack runs in zone order (Crop before Resize before the tone steps,
    /// the tone steps in stored order); a preset keeps the geometry steps.
    #[test]
    fn the_geometry_band_runs_first_and_with_a_preset() {
        let crop = ManualStep::Crop {
            pct: [25, 0, 50, 100],
        };
        let resize = ManualStep::Resize { width_px: 4 };
        let levels = StepKind::Levels.default_step(100);
        let curves = StepKind::Curves.default_step(100);
        let registers = AdjustRegisters {
            steps: vec![on(curves), on(resize), on(levels), on(crop)],
            ..AdjustRegisters::default()
        };
        assert_eq!(
            registers.effective_steps(100, |_| unreachable!()),
            vec![crop, resize, curves, levels]
        );
        let preset = AdjustRegisters {
            preset: Preset::Graphics,
            ..registers
        };
        let steps = preset.effective_steps(100, |geometry| {
            assert_eq!(geometry, [crop, resize]);
            Analysis::of(&image())
        });
        assert_eq!(&steps[..2], &[crop, resize]);
        assert!(steps[2..].iter().all(|s| s.kind().band() == Band::Tone));
    }

    /// Criterion 39: the analysis reads the picture after Crop (a crop of the dark half has a
    /// dark median).
    #[test]
    fn the_analysis_reads_the_cropped_picture() {
        let whole = analysis_after(&image(), &[]);
        let dark = analysis_after(
            &image(),
            &[ManualStep::Crop {
                pct: [0, 0, 25, 100],
            }],
        );
        assert!(dark.histogram.median() < whole.histogram.median());
    }

    /// Criterion 5 and ADR 0073 §2: the tone steps' pixel pitch is the one after Resize, and a
    /// reduced level scales Resize's width.
    #[test]
    fn the_pitch_follows_resize_and_a_level_scales_it() {
        let sharpen = ManualStep::Sharpen {
            radius_um: 100,
            amount_pct: 100,
            threshold_pct: 0,
        };
        let resize = ManualStep::Resize { width_px: 8 };
        // 16 px over 1.6 mm is 0.1 mm a pixel; after Resize to 8 px 0.2 mm.
        let full = pixel_steps(&[sharpen], &frame(), (16, 16), (16, 16));
        assert_eq!(full, vec![sharpen.pixel_step(0.1)]);
        let half = pixel_steps(&[resize, sharpen], &frame(), (16, 16), (16, 16));
        assert_eq!(
            half,
            vec![PixelStep::Resize { width_px: 8 }, sharpen.pixel_step(0.2)]
        );
        let reduced = pixel_steps(&[resize, sharpen], &frame(), (16, 16), (8, 8));
        assert_eq!(
            reduced,
            vec![PixelStep::Resize { width_px: 4 }, sharpen.pixel_step(0.4)]
        );
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
