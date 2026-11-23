//! The writes and the words of the AI steps (`specs/0135` criteria 24, 47, 48, 63 to 66; ADR 0076
//! §7, ADR 0077 §6; `ux-notes.md` §8): a Run's result stored on its row in one step "Run ...",
//! Remove stored result, the limits' refusals, and the state lines of a step and its model.

use curvyo_document_core::{
    AdjustEntry, AssetError, MAX_DERIVED_ASSET_BYTES, MAX_DERIVED_PROJECT_BYTES,
    RUN_AI_DENOISE_COMMIT_LABEL, RUN_REMOVE_BACKGROUND_COMMIT_LABEL, RUN_UPSCALE_COMMIT_LABEL,
    RanStep, StepResult,
};
use curvyo_image_core::{MAX_WORKING_PIXELS, MAX_WORKING_SIDE};
use curvyo_laser_core::adjust::StepKind;

/// The stored results' share of the document limit from which the group warns (criterion 66):
/// 384 MiB, 75 % of 512 MiB.
pub const STORED_RESULTS_WARN_BYTES: u64 = MAX_DERIVED_PROJECT_BYTES / 4 * 3;

/// The text of Out of date in an unfolded AI row (`ux-notes.md` §8).
pub const OUT_OF_DATE_TEXT: &str = "Out of date. The stored result still burns.";
/// The model host cannot be reached (criterion 65).
pub const HOST_UNREACHABLE_TEXT: &str =
    "Model host not reachable. Everything else keeps working and stored results still burn.";
/// The downloaded file did not match its pinned hash (criterion 64).
pub const CHECKSUM_TEXT: &str = "The downloaded model did not match its checksum and was removed.";
/// The model was withdrawn from the host (ADR 0076 §7, §10).
pub const WITHDRAWN_TEXT: &str = "This model is no longer available. Stored results still burn.";

/// The commit label of a Run of `kind` (criterion 24), `None` for a step that runs no model.
#[must_use]
pub const fn run_label(kind: StepKind) -> Option<&'static str> {
    match kind {
        StepKind::RemoveBackground => Some(RUN_REMOVE_BACKGROUND_COMMIT_LABEL),
        StepKind::Upscale => Some(RUN_UPSCALE_COMMIT_LABEL),
        StepKind::AiDenoise => Some(RUN_AI_DENOISE_COMMIT_LABEL),
        _ => None,
    }
}

/// The stack with `result` stored on row `index` (a Run, criterion 48); `None` when the row is
/// not an AI step.
#[must_use]
pub fn store_result(
    entries: &[AdjustEntry],
    index: usize,
    result: StepResult,
) -> Option<Vec<AdjustEntry>> {
    let step = entries.get(index)?.step()?.clone();
    StepKind::from_key(&step.kind).filter(|k| k.is_ai())?;
    let mut next = entries.to_vec();
    next[index] = AdjustEntry::Ran(RanStep { step, result });
    Some(next)
}

/// The stack without the stored result of row `index` (Remove stored result, criterion 66);
/// `None` when the row has none.
#[must_use]
pub fn remove_result(entries: &[AdjustEntry], index: usize) -> Option<Vec<AdjustEntry>> {
    let AdjustEntry::Ran(ran) = entries.get(index)? else {
        return None;
    };
    let mut next = entries.to_vec();
    next[index] = AdjustEntry::Step(ran.step.clone());
    Some(next)
}

/// `bytes` in MB (2^20 bytes): one decimal below 10, whole above.
#[must_use]
pub fn megabytes(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss)] // invariant: at most a few GiB, exact enough here
    let mb = bytes as f64 / 1_048_576.0;
    if mb < 10.0 {
        format!("{mb:.1}")
    } else {
        format!("{mb:.0}")
    }
}

/// The refusal of a result over a limit (criterion 66, `ux-notes.md` §8).
#[must_use]
pub fn result_refusal(error: AssetError, result_bytes: u64) -> String {
    let size = megabytes(result_bytes);
    match error {
        AssetError::ResultTooLarge { .. } => format!(
            "This result would be {size} MB; one stored result can be at most {} MB.",
            MAX_DERIVED_ASSET_BYTES >> 20
        ),
        AssetError::ResultsFull { .. } => format!(
            "This result would be {size} MB; the stored results of this document can be at most {} MB.",
            MAX_DERIVED_PROJECT_BYTES >> 20
        ),
        AssetError::ProjectFull { .. } => error.to_string(),
    }
}

/// The budget line "Stored results 120 MB of 512 MB" and whether it warns (from 384 MiB).
#[must_use]
pub fn budget_line(stored: u64) -> (String, bool) {
    (
        format!(
            "Stored results {} MB of {} MB",
            stored_megabytes(stored),
            MAX_DERIVED_PROJECT_BYTES >> 20
        ),
        stored >= STORED_RESULTS_WARN_BYTES,
    )
}

/// "Stored result 2.1 MB".
#[must_use]
pub fn stored_line(bytes: u64) -> String {
    format!("Stored result {} MB", stored_megabytes(bytes))
}

/// [`megabytes`] of a stored size, "under 0.1" for one that would read 0.0 (UX review).
fn stored_megabytes(bytes: u64) -> String {
    let mb = megabytes(bytes);
    if bytes > 0 && mb == "0.0" {
        "under 0.1".to_owned()
    } else {
        mb
    }
}

fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Upscale's state line "1,200 × 900 px to 2,400 × 1,800 px", or the refusal before running
/// "4× would be 9,600 × 7,200 px, over the working limit of 16 megapixels." (criterion 47).
///
/// # Errors
/// The refusal text when the result would pass the working limit.
pub fn upscale_line(width: u32, height: u32, factor: u8) -> Result<String, String> {
    let (w, h) = (
        u64::from(width) * u64::from(factor),
        u64::from(height) * u64::from(factor),
    );
    if w > u64::from(MAX_WORKING_SIDE)
        || h > u64::from(MAX_WORKING_SIDE)
        || w * h > MAX_WORKING_PIXELS
    {
        return Err(format!(
            "{factor}\u{d7} would be {} \u{d7} {} px, over the working limit of {} megapixels.",
            grouped(w),
            grouped(h),
            MAX_WORKING_PIXELS / 1_000_000
        ));
    }
    Ok(format!(
        "{} \u{d7} {} px to {} \u{d7} {} px",
        grouped(u64::from(width)),
        grouped(u64::from(height)),
        grouped(w),
        grouped(h)
    ))
}

/// The consent line before a model's first download (criterion 63): the step's name first, the
/// model's file name in brackets (UX review).
#[must_use]
pub fn consent_line(step: &str, model: &str, bytes: u64, host: &str) -> String {
    format!(
        "Downloads the {step} model ({model}, {} MB) once from {host}. Your picture does not leave this computer.",
        megabytes(bytes)
    )
}

/// "Model not downloaded yet (38 MB)." (criterion 65).
#[must_use]
pub fn not_downloaded_line(bytes: u64) -> String {
    format!("Model not downloaded yet ({} MB).", megabytes(bytes))
}

/// "Downloading 21 of 64 MB" (criterion 63).
#[must_use]
pub fn downloading_line(done: u64, total: u64) -> String {
    format!("Downloading {} of {} MB", megabytes(done), megabytes(total))
}

/// "Download paused at 21 of 64 MB." (ADR 0076 §7).
#[must_use]
pub fn paused_line(done: u64, total: u64) -> String {
    format!(
        "Download paused at {} of {} MB.",
        megabytes(done),
        megabytes(total)
    )
}

/// "Not enough browser storage for the Remove background model (4.5 MB)." (ADR 0076 §7).
#[must_use]
pub fn storage_line(step: &str, bytes: u64) -> String {
    format!(
        "Not enough browser storage for the {step} model ({} MB).",
        megabytes(bytes)
    )
}

/// "Model on this computer (4.5 MB)." (the Storage line, criterion 64; Remove beside it).
#[must_use]
pub fn ready_line(bytes: u64) -> String {
    format!("Model on this computer ({} MB).", megabytes(bytes))
}

/// "Running · 62 %" (`ux-notes.md` §3.6), or "Running on one core · 62 %" when the Run has a
/// pool of one worker (`specs/0143` criterion 10).
#[must_use]
pub fn running_line(done: u32, total: u32, one_core: bool) -> String {
    let pct = u64::from(done) * 100 / u64::from(total.max(1));
    let words = if one_core {
        "Running on one core"
    } else {
        "Running"
    };
    format!("{words} \u{b7} {pct} %")
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{AdjustStep, AssetHash};

    fn step(kind: &str) -> AdjustEntry {
        AdjustEntry::Step(AdjustStep {
            kind: kind.into(),
            enabled: true,
            fields: [("factor".to_owned(), 2)].into_iter().collect(),
        })
    }

    fn result() -> StepResult {
        StepResult {
            asset: AssetHash::of(b"x"),
            model: "m".into(),
            model_version: "1".into(),
            input_hash: AssetHash::of(b"y"),
            width_px: 2,
            height_px: 2,
        }
    }

    #[test]
    fn a_result_is_stored_on_an_ai_row_and_removed_again() {
        let entries = vec![step("levels"), step("upscale")];
        assert_eq!(store_result(&entries, 0, result()), None, "not an AI step");
        let ran = store_result(&entries, 1, result()).unwrap();
        assert_eq!(ran[1].result(), Some(&result()));
        let removed = remove_result(&ran, 1).unwrap();
        assert_eq!(removed, entries);
        assert_eq!(remove_result(&removed, 1), None);
    }

    #[test]
    fn the_words_of_ux_notes_8() {
        assert_eq!(
            upscale_line(1_200, 900, 2).unwrap(),
            "1,200 \u{d7} 900 px to 2,400 \u{d7} 1,800 px"
        );
        assert_eq!(
            upscale_line(2_400, 1_800, 4).unwrap_err(),
            "4\u{d7} would be 9,600 \u{d7} 7,200 px, over the working limit of 16 megapixels."
        );
        assert_eq!(
            result_refusal(AssetError::ResultTooLarge { bytes: 0 }, 70 << 20),
            "This result would be 70 MB; one stored result can be at most 64 MB."
        );
        assert_eq!(
            budget_line(120 << 20),
            ("Stored results 120 MB of 512 MB".to_owned(), false)
        );
        assert!(budget_line(384 << 20).1, "warns from 75 %");
        assert_eq!(stored_line(2_202_010), "Stored result 2.1 MB");
        assert_eq!(stored_line(20_000), "Stored result under 0.1 MB");
        assert_eq!(
            budget_line(20_000).0,
            "Stored results under 0.1 MB of 512 MB"
        );
        assert_eq!(
            not_downloaded_line(38 << 20),
            "Model not downloaded yet (38 MB)."
        );
        assert_eq!(
            consent_line(
                "Upscale",
                "realesr-animevideov3",
                2_500_000,
                "models.curvyo.org"
            ),
            "Downloads the Upscale model (realesr-animevideov3, 2.4 MB) once from models.curvyo.org. Your picture does not leave this computer."
        );
        assert_eq!(
            storage_line("Remove background", 4_700_000),
            "Not enough browser storage for the Remove background model (4.5 MB)."
        );
        assert_eq!(
            downloading_line(21 << 20, 64 << 20),
            "Downloading 21 of 64 MB"
        );
        assert_eq!(running_line(62, 100, false), "Running \u{b7} 62 %");
        assert_eq!(running_line(1, 4, true), "Running on one core \u{b7} 25 %");
        assert_eq!(run_label(StepKind::Upscale), Some("run_upscale"));
        assert_eq!(run_label(StepKind::Levels), None);
    }
}
