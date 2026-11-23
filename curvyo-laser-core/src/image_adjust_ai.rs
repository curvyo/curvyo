//! The AI steps' stored results as the pipeline reads them (ADR 0077 §2 to §4; `specs/0135`
//! criteria 47, 48, 66): the result reference a step carries, the input an AI step reads and its
//! hash, and the counts of stored results that are missing or out of date.

use curvyo_document_core::{AssetHash, StepResult};
use curvyo_image_core::{PixelStep, StoredPixels, WorkingImage, input_hash, step_input};

use crate::image_adjust_fields::{Band, ManualStep, StepKind};
use crate::image_adjust_resolve::AdjustRegisters;

/// The longest model name and version together, in bytes, that a result can name.
const MODEL_TAG_BYTES: usize = 47;

/// A model's name and version as a stored result names them, held inline so a step stays `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelTag {
    bytes: [u8; MODEL_TAG_BYTES],
    name_len: u8,
    len: u8,
}

impl ModelTag {
    /// The tag of `name` and `version`; `None` when they are longer than 47 bytes together.
    #[must_use]
    pub fn new(name: &str, version: &str) -> Option<Self> {
        let total = name.len() + version.len();
        if total > MODEL_TAG_BYTES {
            return None;
        }
        let mut bytes = [0; MODEL_TAG_BYTES];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        bytes[name.len()..total].copy_from_slice(version.as_bytes());
        Some(Self {
            bytes,
            name_len: u8::try_from(name.len()).ok()?,
            len: u8::try_from(total).ok()?,
        })
    }

    /// The model's name.
    #[must_use]
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.name_len)]).unwrap_or("")
    }

    /// The model's version.
    #[must_use]
    pub fn version(&self) -> &str {
        std::str::from_utf8(&self.bytes[usize::from(self.name_len)..usize::from(self.len)])
            .unwrap_or("")
    }
}

/// The stored result an AI step reads (ADR 0077 §2), as the step carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResultRef {
    /// The SHA-256 of the derived asset's PNG.
    pub asset: [u8; 32],
    /// The input hash it was made from.
    pub input_hash: [u8; 32],
    /// The model that made it.
    pub model: ModelTag,
    /// The stored picture's width in pixels.
    pub width_px: u32,
    /// The stored picture's height in pixels.
    pub height_px: u32,
}

impl ResultRef {
    /// The reference of a stored result record; `None` for a model name and version over 47
    /// bytes (a result this build cannot name, read as missing).
    #[must_use]
    pub fn of(result: &StepResult) -> Option<Self> {
        Some(Self {
            asset: result.asset.to_bytes(),
            input_hash: result.input_hash.to_bytes(),
            model: ModelTag::new(&result.model, &result.model_version)?,
            width_px: result.width_px,
            height_px: result.height_px,
        })
    }

    /// The stored record of this reference.
    #[must_use]
    pub fn record(&self) -> StepResult {
        StepResult {
            asset: AssetHash::from_bytes(&self.asset),
            model: self.model.name().to_owned(),
            model_version: self.model.version().to_owned(),
            input_hash: AssetHash::from_bytes(&self.input_hash),
            width_px: self.width_px,
            height_px: self.height_px,
        }
    }

    /// The derived asset's hash.
    #[must_use]
    pub fn asset_hash(&self) -> AssetHash {
        AssetHash::from_bytes(&self.asset)
    }
}

impl ManualStep {
    /// The stored result of an AI step, `None` for another step or one not run yet.
    #[must_use]
    pub const fn result(&self) -> Option<ResultRef> {
        match *self {
            Self::Upscale { result, .. }
            | Self::AiDenoise { result, .. }
            | Self::RemoveBackground { result, .. } => result,
            _ => None,
        }
    }

    /// This step with the stored result `result` (an AI step; another step is returned as it
    /// is).
    #[must_use]
    pub const fn with_result(self, result: Option<ResultRef>) -> Self {
        match self {
            Self::Upscale { factor, .. } => Self::Upscale { factor, result },
            Self::AiDenoise { strength_pct, .. } => Self::AiDenoise {
                strength_pct,
                result,
            },
            Self::RemoveBackground {
                softness_pct,
                threshold_pct,
                ..
            } => Self::RemoveBackground {
                softness_pct,
                threshold_pct,
                result,
            },
            other => other,
        }
    }

    /// The fields a model run reads (ADR 0077 §2: Upscale's factor; none for the other two).
    #[must_use]
    pub fn model_fields(&self) -> Vec<(&'static str, i64)> {
        match *self {
            Self::Upscale { factor, .. } => vec![("factor", i64::from(factor))],
            _ => Vec::new(),
        }
    }
}

impl StepKind {
    /// Whether the kind runs a model (Upscale, AI denoise, Remove background).
    #[must_use]
    pub const fn is_ai(self) -> bool {
        matches!(
            self,
            Self::Upscale | Self::AiDenoise | Self::RemoveBackground
        )
    }
}

/// An AI step's input picture and its hash with the model it would run (ADR 0077 §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiInput {
    /// The picture the model reads: after the steps before it, opaque, at full resolution.
    pub image: WorkingImage,
    /// The input hash.
    pub hash: [u8; 32],
}

impl AdjustRegisters {
    /// The enabled AI steps that run without a stored result (a step not run yet or a removed
    /// result): the picture does not burn until Run (ADR 0077 §4).
    #[must_use]
    pub fn missing_results(&self) -> usize {
        self.running_ai()
            .filter(|(_, step)| step.result().is_none())
            .count()
    }

    /// The stored results the enabled AI steps read, in stack order.
    #[must_use]
    pub fn stored_results(&self) -> Vec<ResultRef> {
        self.running_ai()
            .filter_map(|(_, step)| step.result())
            .collect()
    }

    /// The enabled AI steps that read, with their stack index.
    fn running_ai(&self) -> impl Iterator<Item = (usize, ManualStep)> + '_ {
        self.steps
            .iter()
            .enumerate()
            .filter(|(_, s)| s.enabled)
            .filter_map(|(i, s)| s.step.map(|step| (i, step)))
            .filter(|(_, step)| step.kind().is_ai())
    }

    /// The input of the AI step at stack index `index` on the working image `image` and its
    /// hash with `model` (name, version): the picture after the enabled geometry steps before it
    /// in the pipeline (for Remove background, after every geometry step), with their stored
    /// results from `results`. `None` when `index` is no AI step.
    #[must_use]
    pub fn ai_input(
        &self,
        index: usize,
        image: &WorkingImage,
        results: &StoredPixels,
        model: (&str, &str),
    ) -> Option<AiInput> {
        let step = self.steps.get(index)?.step?;
        let kind = step.kind();
        if !kind.is_ai() {
            return None;
        }
        let upstream: Vec<PixelStep> = self
            .geometry_steps()
            .into_iter()
            .filter(|s| kind.band() == Band::Mask || s.kind() < kind)
            .filter_map(geometry_pixel_step)
            .collect();
        let input = step_input(image, &upstream, results);
        let hash = input_hash(kind.key(), model, &step.model_fields(), &input);
        Some(AiInput { image: input, hash })
    }

    /// The stored results whose input hash differs from the current input (Out of date,
    /// criterion 48): each enabled AI step with a result is checked with the model it names.
    #[must_use]
    pub fn count_stale(&self, image: &WorkingImage, results: &StoredPixels) -> usize {
        self.stale_rows(image, results).len()
    }

    /// The stack indices of the out-of-date stored results ([`Self::count_stale`]).
    #[must_use]
    pub fn stale_rows(&self, image: &WorkingImage, results: &StoredPixels) -> Vec<usize> {
        self.running_ai()
            .filter(|(index, step)| {
                step.result().is_some_and(|r| {
                    self.ai_input(*index, image, results, (r.model.name(), r.model.version()))
                        .is_some_and(|input| input.hash != r.input_hash)
                })
            })
            .map(|(index, _)| index)
            .collect()
    }
}

/// The pixel step of a geometry-band step for the full working image (the run of an AI step's
/// input); `None` for an AI step without a result.
fn geometry_pixel_step(step: ManualStep) -> Option<PixelStep> {
    if step.kind().is_ai() && step.result().is_none() {
        return None;
    }
    Some(step.pixel_step(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_model_tag_keeps_name_and_version() {
        let tag = ModelTag::new("realesr-general-x4v3", "1").unwrap();
        assert_eq!((tag.name(), tag.version()), ("realesr-general-x4v3", "1"));
        assert!(ModelTag::new(&"x".repeat(40), "12345678").is_none());
    }

    #[test]
    fn a_result_reference_round_trips_its_record() {
        let record = StepResult {
            asset: AssetHash::of(b"png"),
            model: "u2netp".into(),
            model_version: "1".into(),
            input_hash: AssetHash::of(b"input"),
            width_px: 320,
            height_px: 320,
        };
        assert_eq!(ResultRef::of(&record).unwrap().record(), record);
    }
}
