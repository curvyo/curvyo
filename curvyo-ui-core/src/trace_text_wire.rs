//! The trace job with OCR lines and the text section of its numbers
//! (`specs/0091-trace-text-recognition` `adrs.md` decisions 1 to 3): the text pass runs first, the
//! main trace walks the filled picture, and the accepted lines follow the outlines.

use curvyo_document_core::Affine;
use curvyo_vectorize_core::{Mask as TraceMask, TraceParams};
use kurbo::{BezPath, Vec2};

use crate::trace_ocr::OcrLine;
use crate::trace_text::TextPass;
use crate::trace_text::text_pass;
use crate::trace_transfer::{
    TraceStatus, count, read_outlines, status_only, trace_job_with, trace_mask_job, write_outline,
};

/// The number that opens the text section (no count is negative).
pub(crate) const TEXT_MARK: f64 = -1.0;

/// The level of a left-out pixel in the Mask view (UX notes §2: flat mid grey).
pub const LEFT_OUT_LEVEL: u8 = 128;

/// One accepted line of a result.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceTextWire {
    /// The OCR's string.
    pub text: String,
    /// The line's direction in document millimetres.
    pub direction: Vec2,
    /// Pixels left out of the main trace for it.
    pub masked: usize,
    /// The OCR line passes the garbage filter (Frage 115 B / 117 B): it becomes text even
    /// without a sure letter.
    pub plausible: bool,
    /// The size of one picture pixel along the line, document millimetres (the fit's tolerance,
    /// criterion 26).
    pub pixel: f64,
    /// The line's own outlines in document millimetres.
    pub outlines: Vec<BezPath>,
}

/// The text section of a result.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TraceTexts {
    /// Lines the OCR found, accepted or not.
    pub found: usize,
    /// The accepted lines with a trace.
    pub lines: Vec<TraceTextWire>,
}

/// One trace with the OCR `lines` (in this picture's pixels): the text pass, then
/// [`trace_job_with`] over the filled picture, then the text section. Without lines the numbers
/// are exactly those of [`trace_job_with`] (criteria 12 and 18).
#[must_use]
pub fn trace_text_job_with(
    rgba: &[u8],
    width: u32,
    height: u32,
    pixel_map: [f64; 6],
    params: &TraceParams,
    lines: &[OcrLine],
    on_percent: &mut dyn FnMut(u8),
) -> Vec<f64> {
    if lines.is_empty() {
        return trace_job_with(rgba, width, height, pixel_map, params, on_percent);
    }
    let pass = text_pass(
        rgba,
        width,
        height,
        Affine::from_array(pixel_map),
        params,
        lines,
    );
    let mut wire = trace_job_with(&pass.rgba, width, height, pixel_map, params, on_percent);
    if wire.first() != Some(&0.0) {
        return wire;
    }
    write_texts(&mut wire, &pass);
    wire
}

/// The text section of `pass` after a result's numbers.
fn write_texts(wire: &mut Vec<f64>, pass: &TextPass) {
    #[allow(clippy::cast_precision_loss)] // counts far below 2^52
    {
        wire.extend([TEXT_MARK, pass.found as f64, pass.regions.len() as f64]);
        for region in &pass.regions {
            let chars: Vec<char> = region.text.chars().collect();
            wire.push(chars.len() as f64);
            wire.extend(chars.iter().map(|&c| f64::from(u32::from(c))));
            wire.extend([
                region.direction.x,
                region.direction.y,
                region.masked as f64,
                f64::from(u8::from(region.plausible)),
                region.pixel,
                region.outlines.len() as f64,
            ]);
            for outline in &region.outlines {
                write_outline(wire, outline);
            }
        }
    }
}

/// The text pass alone (`specs/0091` criterion 25: the lines are decided before the main trace):
/// the numbers of a result without outlines, then the text section of the accepted lines, as
/// [`trace_text_job_with`] would write it.
#[must_use]
pub fn trace_text_regions_with(
    rgba: &[u8],
    width: u32,
    height: u32,
    pixel_map: [f64; 6],
    params: &TraceParams,
    lines: &[OcrLine],
) -> Vec<f64> {
    let pass = text_pass(
        rgba,
        width,
        height,
        Affine::from_array(pixel_map),
        params,
        lines,
    );
    let mut wire = status_only(TraceStatus::Traced);
    write_texts(&mut wire, &pass);
    wire
}

/// The mask the trace of the same arguments walks (`specs/0089` criterion 21), the pixels the
/// text pass leaves out at [`LEFT_OUT_LEVEL`] (UX notes §2); without lines exactly
/// [`trace_mask_job`].
#[must_use]
pub fn trace_text_mask_job(
    rgba: &[u8],
    width: u32,
    height: u32,
    pixel_map: [f64; 6],
    params: &TraceParams,
    lines: &[OcrLine],
) -> Option<TraceMask> {
    if lines.is_empty() {
        return trace_mask_job(rgba, width, height, pixel_map, params);
    }
    let pass = text_pass(
        rgba,
        width,
        height,
        Affine::from_array(pixel_map),
        params,
        lines,
    );
    let mut mask = trace_mask_job(&pass.rgba, width, height, pixel_map, params)?;
    let (w, mw) = (width as usize, mask.width as usize);
    let factor = (mw / w.max(1)).max(1);
    for (i, _) in pass.left_out.iter().enumerate().filter(|(_, out)| **out) {
        let (x, y) = (i % w, i / w);
        for dy in 0..factor {
            for dx in 0..factor {
                if let Some(bit) = mask.bits.get_mut((y * factor + dy) * mw + x * factor + dx) {
                    *bit = LEFT_OUT_LEVEL;
                }
            }
        }
    }
    Some(mask)
}

/// The text section at the start of `tail` (it starts with [`TEXT_MARK`]) and the numbers it
/// took; `None` unless it is well formed.
pub(crate) fn read_texts(tail: &[f64]) -> Option<(TraceTexts, usize)> {
    let mut at = 1;
    let next = |at: &mut usize| -> Option<f64> {
        let value = *tail.get(*at)?;
        *at += 1;
        value.is_finite().then_some(value)
    };
    let found = count(next(&mut at)?)?;
    let n = count(next(&mut at)?)?;
    let mut lines = Vec::with_capacity(n.min(tail.len()));
    for _ in 0..n {
        let chars = count(next(&mut at)?)?;
        let mut text = String::new();
        for _ in 0..chars {
            let code = u32::try_from(count(next(&mut at)?)?).ok()?;
            text.push(char::from_u32(code)?);
        }
        let direction = Vec2::new(next(&mut at)?, next(&mut at)?);
        let masked = count(next(&mut at)?)?;
        let plausible = next(&mut at)? != 0.0;
        let pixel = next(&mut at)?;
        let outlines = count(next(&mut at)?)?;
        let outlines = read_outlines(tail, &mut at, outlines)?;
        lines.push(TraceTextWire {
            text,
            direction,
            masked,
            plausible,
            pixel,
            outlines,
        });
    }
    Some((TraceTexts { found, lines }, at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace_transfer::read_trace_wire;

    /// A 40 x 20 picture with a black bar (the "text") on white.
    fn picture() -> Vec<u8> {
        let mut rgba = vec![255_u8; 40 * 20 * 4];
        for y in 8..12 {
            for x in 8..32 {
                let i = (y * 40 + x) * 4;
                rgba[i..i + 3].fill(0);
            }
        }
        rgba
    }

    fn line(text: &str) -> OcrLine {
        OcrLine {
            quad: [[6.0, 6.0], [34.0, 6.0], [34.0, 14.0], [6.0, 14.0]],
            text: text.to_owned(),
            confidence: vec![0.9; text.chars().count()],
        }
    }

    const MAP: [f64; 6] = [0.1, 0.0, 0.0, 0.1, 0.0, 0.0];

    #[test]
    fn the_text_section_round_trips_and_the_bar_leaves_the_main_trace() {
        let params = TraceParams::with_threshold(128);
        let wire = trace_text_job_with(
            &picture(),
            40,
            20,
            MAP,
            &params,
            &[line("Grüße")],
            &mut |_| {},
        );
        let read = read_trace_wire(&wire).unwrap();
        assert_eq!(read.outlines.len(), 0);
        assert_eq!(read.texts.found, 1);
        assert_eq!(read.texts.lines[0].text, "Grüße");
        assert_ne!(read.texts.lines[0].outlines.len(), 0);
        // Frage 115 B and criterion 26: the garbage filter and the pixel size travel along.
        assert!(read.texts.lines[0].plausible);
        assert!((read.texts.lines[0].pixel - 0.1).abs() < 1e-12);
    }

    /// Criterion 25: the text pass alone gives the text section the full job gives, no outline.
    #[test]
    fn the_regions_are_the_text_section_of_the_job() {
        let params = TraceParams::with_threshold(128);
        let lines = [line("Grüße")];
        let full = read_trace_wire(&trace_text_job_with(
            &picture(),
            40,
            20,
            MAP,
            &params,
            &lines,
            &mut |_| {},
        ))
        .unwrap();
        let regions = read_trace_wire(&trace_text_regions_with(
            &picture(),
            40,
            20,
            MAP,
            &params,
            &lines,
        ))
        .unwrap();
        assert_eq!(regions.texts, full.texts);
        assert_eq!(regions.outlines.len(), 0);
    }

    /// Criteria 12 and 18: no lines, the same numbers as before.
    #[test]
    fn without_lines_the_job_is_the_plain_trace() {
        let params = TraceParams::with_threshold(128);
        let plain = trace_job_with(&picture(), 40, 20, MAP, &params, &mut |_| {});
        assert_eq!(
            trace_text_job_with(&picture(), 40, 20, MAP, &params, &[], &mut |_| {}),
            plain
        );
        let read = read_trace_wire(&plain).unwrap();
        assert_eq!(read.texts, TraceTexts::default());
    }

    #[test]
    fn the_mask_shows_left_out_pixels_grey() {
        let params = TraceParams::with_threshold(128);
        let mask = trace_text_mask_job(&picture(), 40, 20, MAP, &params, &[line("I")]).unwrap();
        let factor = (mask.width / 40) as usize;
        let at = |x: usize, y: usize| mask.bits[(y * factor) * mask.width as usize + x * factor];
        assert_eq!(at(20, 10), LEFT_OUT_LEVEL);
        assert_eq!(at(2, 2), 0);
    }
}
