//! Reads the text lines of a picture once for the trace worker (`specs/0091-trace-text-recognition`
//! `adrs.md` decision 1) and decides which of them Trace sets as text.

use std::ops::ControlFlow;

pub use curvyo_ocr_core::{OcrError, OcrModel};
use curvyo_ocr_core::{OcrProgress, read_lines};

/// The least mean character confidence of a line Trace sets as text (Proposal, `adrs.md`
/// decision 2).
pub const MIN_LINE_CONFIDENCE: f64 = 0.6;

/// The least mean character confidence of a line set as text without a sure letter (Frage 115 B
/// / 117 B garbage filter; Proposal, calibrated on the customer's screenshot and the posters:
/// UI chrome read as "88+" is 0.74, "ye" 0.34, real labels 0.89 and up).
pub const MIN_GUESSED_LINE_CONFIDENCE: f64 = 0.8;

/// A line whose letters repeat a unit of at most this many characters is a pattern, not text
/// (the `0025` dot grid read as "TOTOTO").
const MAX_PATTERN_UNIT: usize = 3;

/// The share of the OCR's percentage the detector takes; the lines take the rest.
const DETECT_PERCENT: usize = 20;

/// One line the OCR read, in the source pixels of the picture it read.
#[derive(Debug, Clone, PartialEq)]
pub struct OcrLine {
    /// Top left, top right, bottom right, bottom left along the line.
    pub quad: [[f64; 2]; 4],
    /// The string.
    pub text: String,
    /// The confidence of each character, 0 to 1.
    pub confidence: Vec<f64>,
}

impl OcrLine {
    /// Whether Trace sets the line as text: mean confidence at least [`MIN_LINE_CONFIDENCE`] and a
    /// letter or digit in it.
    #[must_use]
    pub fn accepted(&self) -> bool {
        if self.confidence.is_empty() || !self.text.chars().any(char::is_alphanumeric) {
            return false;
        }
        // invariant: a count of characters, far below 2^52.
        #[allow(clippy::cast_precision_loss)]
        let mean = self.confidence.iter().sum::<f64>() / self.confidence.len() as f64;
        mean >= MIN_LINE_CONFIDENCE
    }

    /// Whether the line becomes text even when no face gives it a sure letter (Frage 115 B /
    /// 117 B, the garbage filter): at least 2 characters besides spaces, more than half of them
    /// letters or digits, a mean confidence of at least [`MIN_GUESSED_LINE_CONFIDENCE`], and not
    /// one short unit repeated.
    #[must_use]
    pub fn plausible(&self) -> bool {
        let chars: Vec<char> = self.text.chars().filter(|c| !c.is_whitespace()).collect();
        let letters = chars.iter().filter(|c| c.is_alphanumeric()).count();
        if chars.len() < 2 || 2 * letters <= chars.len() || self.confidence.is_empty() {
            return false;
        }
        // invariant: a count of characters, far below 2^52.
        #[allow(clippy::cast_precision_loss)]
        let mean = self.confidence.iter().sum::<f64>() / self.confidence.len() as f64;
        let repeated = (1..=MAX_PATTERN_UNIT.min(chars.len() / 2))
            .any(|unit| (unit..chars.len()).all(|i| chars[i] == chars[i - unit]));
        mean >= MIN_GUESSED_LINE_CONFIDENCE && !repeated
    }

    /// The line with its quad scaled by `factor` (a reduced copy of the picture read).
    #[must_use]
    pub fn scaled(&self, factor: f64) -> Self {
        Self {
            quad: self.quad.map(|[x, y]| [x * factor, y * factor]),
            ..self.clone()
        }
    }
}

/// The lines of `rgba` (`width` x `height`, straight alpha) read with `model`, top to bottom;
/// `on_percent` hears each higher whole percent and may stop the read with `Break`.
///
/// # Errors
/// As [`curvyo_ocr_core::read_lines`]: a cancel, an unreadable raster or a failed inference.
pub fn trace_read_text(
    model: &OcrModel,
    rgba: &[u8],
    width: u32,
    height: u32,
    on_percent: &mut dyn FnMut(u8) -> ControlFlow<()>,
) -> Result<Vec<OcrLine>, OcrError> {
    let mut reported: Option<u8> = None;
    let mut poll = |progress: OcrProgress| {
        let percent = match progress {
            OcrProgress::Detecting => 0,
            OcrProgress::Reading { done, total } => {
                DETECT_PERCENT + (100 - DETECT_PERCENT) * done / total.max(1)
            }
        };
        let percent = u8::try_from(percent.min(100)).unwrap_or(100);
        if reported.is_some_and(|last| percent <= last) {
            return ControlFlow::Continue(());
        }
        reported = Some(percent);
        on_percent(percent)
    };
    let lines = read_lines(model, width, height, rgba, &mut poll)?;
    Ok(lines
        .into_iter()
        .map(|line| OcrLine {
            quad: line.quad.map(|[x, y]| [f64::from(x), f64::from(y)]),
            text: line.text,
            confidence: line.confidence.into_iter().map(f64::from).collect(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, confidence: &[f64]) -> OcrLine {
        OcrLine {
            quad: [[0.0, 0.0], [10.0, 0.0], [10.0, 4.0], [0.0, 4.0]],
            text: text.to_owned(),
            confidence: confidence.to_vec(),
        }
    }

    #[test]
    fn a_line_needs_a_letter_and_a_mean_confidence_of_0_6() {
        assert!(line("Ab", &[0.9, 0.4]).accepted());
        assert!(!line("Ab", &[0.5, 0.6]).accepted());
        assert!(!line("--", &[1.0, 1.0]).accepted());
        assert!(!line("", &[]).accepted());
        assert_eq!(line("A", &[1.0]).scaled(0.5).quad[2], [5.0, 2.0]);
    }

    /// Frage 115 B / 117 B: the garbage filter of a line without a sure letter.
    #[test]
    fn the_garbage_filter_keeps_words_and_drops_marks_and_patterns() {
        assert!(line("Sans", &[0.9; 4]).plausible());
        assert!(line("Script ×+", &[0.89; 9]).plausible());
        assert!(line("Aa", &[0.93, 0.93]).plausible());
        // One character: the 0025 ring read as "O".
        assert!(!line("O", &[0.8]).plausible());
        // Mostly marks.
        assert!(!line("*1", &[0.9, 0.9]).plausible());
        assert!(!line(". >", &[0.9; 3]).plausible());
        // Under the threshold.
        assert!(!line("ye", &[0.34, 0.34]).plausible());
        assert!(!line("88+", &[0.74; 3]).plausible());
        // A short unit repeated: the 0025 dot grid read as "TOTOTO".
        assert!(!line("TOTOTO", &[0.95; 6]).plausible());
        assert!(!line("O O O", &[0.95; 5]).plausible());
        assert!(!line("abcabc", &[0.95; 6]).plausible());
        assert!(line("abcdabce", &[0.95; 8]).plausible());
    }
}
