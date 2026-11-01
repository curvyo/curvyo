//! Where SVG sets the characters of an exported line before its `dx` (`specs/0024-svg-import-export`
//! criteria 87, 133 (e) and 152), measured as the import measures them: a turned character is
//! shaped alone, so it keeps no kerning pair with its neighbours, while the canvas keeps it.

use curvyo_document_core::{TextAlign, TextChars, TextSnapshot};
use curvyo_text_core::{Face, LineLayout};

use crate::svg_text_layout::SPACES;
use crate::text_layout::text_layout;

/// The SVG pens of one line.
#[derive(Debug, Clone)]
pub(crate) struct SvgPens {
    /// Per character, its pen from the line's start, kerns excluded.
    pub(crate) pens: Vec<f64>,
    /// The line's whole advance with every letter and word spacing (the last one included).
    pub(crate) advance: f64,
}

/// The SVG pens of `line` of `text` when a character of it turns (by `text_turn` plus its own
/// turn); `None` when none turns, then SVG sets the characters as the canvas does.
pub(crate) fn svg_pens(
    text: &TextSnapshot,
    face: &Face,
    line: &LineLayout,
    text_turn: f64,
) -> Option<SvgPens> {
    let string = text.string.get(line.range.clone())?;
    let turned: Vec<bool> = string
        .char_indices()
        .map(|(offset, _)| {
            line.glyphs
                .iter()
                .find(|g| g.cluster == line.range.start + offset)
                .is_some_and(|g| text_turn + g.rotation_rad != 0.0)
        })
        .collect();
    if !turned.contains(&true) {
        return None;
    }
    let size = text.size.as_mm();
    let letter = text.layout.letter_spacing * size;
    let word = text.layout.word_spacing * size;
    let chars: Vec<(usize, char)> = string.char_indices().collect();
    let mut advances = vec![0.0; chars.len()];
    let mut start = 0;
    while start < chars.len() {
        let mut end = start + 1;
        while end < chars.len() && !turned[end] && !turned[end - 1] {
            end += 1;
        }
        let from = chars[start].0;
        let to = chars.get(end).map_or(string.len(), |c| c.0);
        run_advances(
            text,
            face,
            &string[from..to],
            letter,
            &mut advances[start..end],
        );
        start = end;
    }
    let mut pens = Vec::with_capacity(chars.len());
    let mut pen = 0.0;
    for ((_, c), advance) in chars.iter().zip(&advances) {
        pens.push(pen);
        pen += advance + letter + if SPACES.contains(c) { word } else { 0.0 };
    }
    Some(SvgPens { pens, advance: pen })
}

/// The advance of each character of `run` shaped alone with the letter spacing `letter`, the
/// spacing taken out (`svg_text_layout::run_advances`); a character inside a cluster gets 0.
fn run_advances(text: &TextSnapshot, face: &Face, run: &str, letter: f64, out: &mut [f64]) {
    let mut alone = text.clone();
    run.clone_into(&mut alone.string);
    alone.chars = TextChars::default();
    alone.layout.align = TextAlign::Left;
    alone.layout.word_spacing = 0.0;
    alone.layout.rotation = curvyo_document_core::Angle::from_radians(0.0);
    let Some(laid) = text_layout(&alone, face) else {
        return;
    };
    let Some(line) = laid.lines.first() else {
        return;
    };
    let mut pens: Vec<(usize, f64)> = Vec::new();
    for glyph in &line.glyphs {
        if pens.last().is_none_or(|(byte, _)| *byte != glyph.cluster) {
            pens.push((glyph.cluster, glyph.origin.x));
        }
    }
    pens.sort_by_key(|(byte, _)| *byte);
    let end = line.start.0 + line.width.0;
    for (k, (byte, pen)) in pens.iter().enumerate() {
        let next = pens.get(k + 1).map_or(end, |(_, next)| next - letter);
        if let Some(at) = run.char_indices().position(|(b, _)| b == *byte)
            && let Some(slot) = out.get_mut(at)
        {
            *slot = next - pen;
        }
    }
}
