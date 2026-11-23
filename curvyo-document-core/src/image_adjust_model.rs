//! An image's adjustment registers as read (ADR 0072 §1, §5, amendment A1): Preset, Strength, the
//! stack and the Dither, known to this crate only by their stored shape; the laser extension gives
//! them meaning.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::assets::AssetHash;

/// The label of every edit of a picture's adjustments: a preset pick, Strength, a step added,
/// edited, switched, moved or removed, Make editable; its history name is "Edit adjustments"
/// (ADR 0072 §7).
pub const EDIT_ADJUSTMENTS_COMMIT_LABEL: &str = "edit_adjustments";
/// The label of Paste adjustments (`specs/0135` criterion 10); "Paste adjustments".
pub const PASTE_ADJUSTMENTS_COMMIT_LABEL: &str = "paste_adjustments";
/// The label of Restore original (`specs/0135` criterion 59); "Restore original".
pub const RESTORE_ORIGINAL_COMMIT_LABEL: &str = "restore_original";
/// The label of Original's on a clone's Adjustments override row (criterion 9); "Follow original".
pub const FOLLOW_ORIGINAL_COMMIT_LABEL: &str = "follow_original";
/// The label of a Run of Remove background (`specs/0135` criterion 24); "Run Remove background".
pub const RUN_REMOVE_BACKGROUND_COMMIT_LABEL: &str = "run_remove_background";
/// The label of a Run of Upscale; "Run Upscale".
pub const RUN_UPSCALE_COMMIT_LABEL: &str = "run_upscale";
/// The label of a Run of AI denoise; "Run AI denoise".
pub const RUN_AI_DENOISE_COMMIT_LABEL: &str = "run_ai_denoise";

/// The stored preset of a picture that has none: no preset, everything by hand.
pub const EXPERT_PRESET: &str = "expert";
/// The Strength of a picture that never set one.
pub const DEFAULT_IMAGE_STRENGTH: u8 = 50;
/// The largest Strength.
pub const MAX_IMAGE_STRENGTH: u8 = 100;

/// One step of the stack whose shape reads: its kind, whether it is on and its integer fields.
/// Whether this build knows the kind or the fields are in range is the laser extension's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdjustStep {
    /// The stored kind, e.g. `levels`.
    pub kind: String,
    /// Whether the step is switched on.
    pub enabled: bool,
    /// The fields by key, each an integer in the unit its key names (`_pct`, `_centi`, `_um`).
    pub fields: BTreeMap<String, i64>,
}

/// A picture's Dither whose shape reads (ADR 0072 amendment A1): a map with a text `mode` and
/// integer fields. Whether this build knows the mode, every field is there and in range is the
/// laser extension's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredDither {
    /// The stored mode, e.g. `floyd_steinberg`.
    pub mode: String,
    /// The other fields by key, each an integer in the unit its key names.
    #[serde(flatten)]
    pub fields: BTreeMap<String, i64>,
}

/// The stored result of an AI step (ADR 0077 §2): the derived asset holding the raw model output,
/// the model that made it, and what it was made from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct StepResult {
    /// The derived asset (`assets/derived/<hash>.png`).
    pub asset: AssetHash,
    /// The model's name, e.g. `u2netp`.
    pub model: String,
    /// The model's version.
    pub model_version: String,
    /// The SHA-256 of the step's input and model fields, in the asset hash's form.
    pub input_hash: AssetHash,
    /// The stored picture's width in pixels.
    pub width_px: u32,
    /// The stored picture's height in pixels.
    pub height_px: u32,
}

/// An AI step with its stored result (ADR 0077 §2), stored as the step's map with a `result`
/// key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RanStep {
    /// The step.
    #[serde(flatten)]
    pub step: AdjustStep,
    /// Its stored result.
    pub result: StepResult,
}

/// One entry of the stack as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum AdjustEntry {
    /// An entry of the step shape.
    Step(AdjustStep),
    /// An entry of the step shape with a stored AI result.
    Ran(RanStep),
    /// An entry that is not of the step shape (not a map, a kind that is no text, a flag that is
    /// no bool, a field that is no integer, a key this build does not know): kept as stored and
    /// written back unchanged, never run (ADR 0072 §5). A value that has no plain form is `null`.
    Unreadable(serde_json::Value),
}

impl AdjustEntry {
    /// The step, `None` for an unreadable entry.
    #[must_use]
    pub const fn step(&self) -> Option<&AdjustStep> {
        match self {
            Self::Step(step) | Self::Ran(RanStep { step, .. }) => Some(step),
            Self::Unreadable(_) => None,
        }
    }

    /// The stored AI result, `None` for a step without one.
    #[must_use]
    pub const fn result(&self) -> Option<&StepResult> {
        match self {
            Self::Ran(ran) => Some(&ran.result),
            _ => None,
        }
    }
}

/// Which of the four registers is stored but does not read (ADR 0072 §5, A5); each reads as its
/// default and counts as one step that could not run.
#[allow(clippy::struct_excessive_bools)] // one independent flag per register, read by name
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct AdjustDamage {
    /// `image_preset` is not text.
    pub preset: bool,
    /// `image_strength` is not an integer from 0 to 100.
    pub strength: bool,
    /// `image_adjustments` is not a list.
    pub stack: bool,
    /// `image_dither` is not a map with a text `mode` and integer fields.
    pub dither: bool,
}

impl AdjustDamage {
    /// Whether every register reads.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.count() == 0
    }

    /// How many registers are damaged.
    #[must_use]
    pub fn count(self) -> usize {
        usize::from(self.preset)
            + usize::from(self.strength)
            + usize::from(self.stack)
            + usize::from(self.dither)
    }
}

/// The adjustment registers of one picture as read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImageAdjustments {
    /// The stored preset text; `None` when absent or damaged (Expert).
    pub preset: Option<String>,
    /// The Strength, 0 to 100; 50 when absent or damaged.
    pub strength: u8,
    /// The stack in stored order; empty when absent or damaged.
    pub steps: Vec<AdjustEntry>,
    /// The Dither; `None` when absent or damaged (Floyd-Steinberg with the `0123` defaults).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dither: Option<StoredDither>,
    /// The registers that are stored but do not read.
    #[serde(skip_serializing_if = "AdjustDamage::is_none")]
    pub damage: AdjustDamage,
}

impl Default for ImageAdjustments {
    fn default() -> Self {
        Self {
            preset: None,
            strength: DEFAULT_IMAGE_STRENGTH,
            steps: Vec::new(),
            dither: None,
            damage: AdjustDamage::default(),
        }
    }
}

impl ImageAdjustments {
    /// Whether anything would change the picture: a preset other than Expert, or a step
    /// (Restore original is enabled, criterion 59).
    #[must_use]
    pub fn is_adjusted(&self) -> bool {
        self.preset.as_deref().is_some_and(|p| p != EXPERT_PRESET) || !self.steps.is_empty()
    }

    /// Whether Restore original has something to do: the picture is adjusted or a Dither is
    /// stored, damaged or not (ADR 0072 amendment A3, Proposal).
    #[must_use]
    pub fn is_restorable(&self) -> bool {
        self.is_adjusted() || self.dither.is_some() || self.damage.dither
    }
}

/// A write of some of the four registers in one step; `None` leaves that register as stored.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdjustmentsWrite {
    /// The new preset text.
    pub preset: Option<String>,
    /// The new Strength, 0 to 100.
    pub strength: Option<u8>,
    /// The new stack, whole.
    pub steps: Option<Vec<AdjustEntry>>,
    /// The new Dither, every field written (ADR 0072 amendment A1).
    pub dither: Option<StoredDither>,
}

/// Where the adjustments a selected picture shows come from (criterion 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AdjustOwner {
    /// An image: its own registers.
    Image,
    /// A linked clone of an image showing its original's (the nearest own on its chain, or the
    /// root's): the Adjustments override row reads Original's.
    CloneFollowing,
    /// A linked clone with its own adjustments: the row reads Own.
    CloneOwn,
}

/// The adjustments a picture shows, and whose they are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShownAdjustments {
    /// The effective registers.
    pub adjustments: ImageAdjustments,
    /// Whose they are.
    pub owner: AdjustOwner,
}

/// Why an adjustment command did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AdjustError {
    /// The id names neither a live image nor a resolvable clone of an image.
    #[error("not an image")]
    NotAnImage,
    /// Follow original on an image, which follows nothing.
    #[error("not a clone of an image")]
    NotAClone,
    /// A Strength above 100.
    #[error("Strength is 0 to 100 %.")]
    StrengthOutOfRange,
}
