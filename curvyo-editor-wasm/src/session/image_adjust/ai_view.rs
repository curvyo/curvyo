//! The body of an AI step row and the stored results' budget line as the host reads them
//! (`specs/0135` criteria 47, 48, 63 to 66; `ux-notes.md` §8): Run, the model's state, the
//! consent line, progress, the stored result and Out of date.

use curvyo_document_core::AdjustEntry;
use curvyo_imageai_core::{MODEL_HOST, model};
use curvyo_laser_core::adjust::{AdjustRegisters, ManualStep};
use curvyo_ui_core::{
    CHECKSUM_TEXT, HOST_UNREACHABLE_TEXT, OUT_OF_DATE_TEXT, WITHDRAWN_TEXT, budget_line,
    consent_line, downloading_line, not_downloaded_line, paused_line, ready_line, running_line,
    storage_line, stored_line, upscale_line,
};
use serde::Serialize;

use super::ai::{ModelState, Running, model_kind};

/// The progress bar of a download or a Run, with Cancel.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AiProgressView {
    /// "Downloading 21 of 64 MB" or "Running · 62 %".
    pub line: String,
    /// 0 to 1.
    pub fraction: f64,
}

/// The body of an AI step row (`ux-notes.md` §8).
#[allow(clippy::struct_excessive_bools)] // independent row states, read by name
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AiBodyView {
    /// The model kind the host downloads and runs (`background`, `upscale`, `denoise`).
    pub model: &'static str,
    /// "Run", "Resume" or "Try again".
    pub run: &'static str,
    /// The consent line while the host asks before the first download (criterion 63); the
    /// host shows Cancel and Download.
    pub consent: Option<String>,
    /// The download or the Run in progress.
    pub progress: Option<AiProgressView>,
    /// The model's state line ("Model not downloaded yet (38 MB).", …); `None` when ready.
    pub state: Option<String>,
    /// Whether the state line is an alert (the checksum failure).
    pub alert: bool,
    /// Whether the model is on this device: the Storage line offers Remove (criterion 64).
    pub model_ready: bool,
    /// "Stored result 2.1 MB" when the step has a result.
    pub stored: Option<String>,
    /// Whether the stored result is out of date (criterion 48).
    pub out_of_date: Option<&'static str>,
    /// Upscale: "1,200 × 900 px to 2,400 × 1,800 px" or the refusal before running.
    pub size_line: Option<String>,
    /// Whether the size line is a refusal.
    pub size_refused: bool,
    /// The refusal of the last Run (a limit), an alert line.
    pub refusal: Option<String>,
}

/// The stored results' budget line (criterion 66).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BudgetView {
    /// "Stored results 120 MB of 512 MB".
    pub line: String,
    /// Whether it warns (from 384 MiB).
    pub warns: bool,
}

/// The word of the row's button: "Resume" a paused download, "Try again" after a failure, else
/// "Run" (UX review).
const fn run_word(state: ModelState) -> &'static str {
    match state {
        ModelState::Paused(_) => "Resume",
        ModelState::Unreachable | ModelState::Checksum => "Try again",
        _ => "Run",
    }
}

/// The progress bar of a download of a model of `bytes`, else of the row's Run.
fn progress_view(
    running: Option<Running>,
    state: ModelState,
    bytes: u64,
) -> Option<AiProgressView> {
    match (running, state) {
        (_, ModelState::Downloading(done)) => Some(AiProgressView {
            line: downloading_line(done, bytes),
            #[allow(clippy::cast_precision_loss)]
            fraction: done as f64 / bytes.max(1) as f64,
        }),
        (Some(r), _) => Some(AiProgressView {
            line: running_line(r.done, r.total, r.one_core),
            fraction: f64::from(r.done) / f64::from(r.total.max(1)),
        }),
        _ => None,
    }
}

/// The model state's line, and whether it is an alert.
fn state_line(state: ModelState, name: &str, bytes: u64) -> (Option<String>, bool) {
    match state {
        ModelState::Absent | ModelState::Asking => (Some(not_downloaded_line(bytes)), false),
        ModelState::Paused(done) => (Some(paused_line(done, bytes)), false),
        ModelState::Unreachable => (Some(HOST_UNREACHABLE_TEXT.to_owned()), false),
        ModelState::Withdrawn => (Some(WITHDRAWN_TEXT.to_owned()), false),
        ModelState::NoStorage => (Some(storage_line(name, bytes)), false),
        ModelState::Checksum => (Some(CHECKSUM_TEXT.to_owned()), true),
        ModelState::Ready => (Some(ready_line(bytes)), false),
        ModelState::Downloading(_) => (None, false),
    }
}

impl crate::session::Session {
    /// The AI body of row `row` (step `step` of `entries`), with the rows `stale` out of date;
    /// `None` for another step. Also the prefix line 2 shows folded.
    pub(super) fn ai_body(
        &self,
        entries: &[AdjustEntry],
        row: usize,
        step: ManualStep,
        stale: &[usize],
    ) -> Option<(AiBodyView, String)> {
        let kind = model_kind(step.kind())?;
        let file = model(kind);
        let model_state = self.ai_model_state(kind);
        let running = self.image_adjust.ai.running.filter(|r| r.row == row);
        let (state_text, alert) = state_line(model_state, step.kind().name(), file.bytes);
        let progress = progress_view(running, model_state, file.bytes);
        let stored = entries.get(row)?.result().map(|result| {
            let bytes = self.document.asset(&result.asset).map_or(0, |b| b.len());
            stored_line(bytes as u64)
        });
        let out_of_date = (stored.is_some() && stale.contains(&row)).then_some(OUT_OF_DATE_TEXT);
        let (size_line, size_refused) = self.upscale_size_line(step);
        let folded = match &progress {
            Some(p) => format!("{} \u{b7} ", p.line),
            None if out_of_date.is_some() => "Out of date \u{b7} ".to_owned(),
            None => String::new(),
        };
        let host = MODEL_HOST.trim_start_matches("https://");
        Some((
            AiBodyView {
                model: kind.key(),
                run: run_word(model_state),
                consent: (model_state == ModelState::Asking)
                    .then(|| consent_line(step.kind().name(), file.name, file.bytes, host)),
                progress,
                state: state_text,
                alert,
                model_ready: model_state == ModelState::Ready,
                stored,
                out_of_date,
                size_line,
                size_refused,
                refusal: self
                    .image_adjust
                    .ai
                    .refusal
                    .as_ref()
                    .filter(|(r, _)| *r == row)
                    .map(|(_, text)| text.clone()),
            },
            folded,
        ))
    }

    /// Upscale's size line for the selected picture, and whether it is a refusal.
    fn upscale_size_line(&self, step: ManualStep) -> (Option<String>, bool) {
        let ManualStep::Upscale { factor, .. } = step else {
            return (None, false);
        };
        self.adjust_target()
            .and_then(|t| self.picture_size(t))
            .map_or((None, false), |(w, h)| match upscale_line(w, h, factor) {
                Ok(line) => (Some(line), false),
                Err(refusal) => (Some(refusal), true),
            })
    }

    /// The out-of-date rows of the selected picture's stored results (criterion 48).
    pub(super) fn stale_rows(&self, registers: &AdjustRegisters) -> Vec<usize> {
        let Some(leaf) = self.adjust_target().and_then(|t| self.picture_leaf(t)) else {
            return Vec::new();
        };
        let Some(pixels) = self.job_pixels(&leaf.asset) else {
            return Vec::new();
        };
        let cache = self.adjust_pictures();
        self.hold_stored_results(&cache, registers);
        cache.stale_rows(&leaf.asset, &pixels, registers)
    }

    /// The budget line while the document holds stored results (criterion 66).
    pub(super) fn budget_view(&self) -> Option<BudgetView> {
        let bytes = self.document.derived_asset_bytes();
        (bytes > 0).then(|| {
            let (line, warns) = budget_line(bytes);
            BudgetView { line, warns }
        })
    }
}
