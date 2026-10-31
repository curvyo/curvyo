//! Turns matched glyphs into runs with string, size, placement and spacing (`adrs.md` decision 6).
//!
//! Spaces come from gaps wider than half the face's space advance beyond the line's letter
//! spacing; the letter and word spacing are medians rounded to 0.01 mm. The string is laid out
//! again with the real [`layout`] and every laid-out glyph is measured against its traced shape
//! without any move, so a sure letter in the document is what the renderer draws, within the
//! tolerance; a sure letter over it after layout is demoted to guessed.

use kurbo::{Affine, Point};

use super::candidates::Prepared;
use super::distance::{Side, two_sided};
use super::lines::median;
use super::reference::ReferenceGlyph;
use super::verify::{FaceFit, Letter, Scaled};
use super::{RecognisedRun, RecognitionError};
use crate::face::Face;
use crate::layout::{LayoutOptions, layout};
use crate::units::Mm;

/// The share of the space advance a gap must pass (beyond the letter spacing) to be a space.
const SPACE_SHARE: f64 = 0.5;

fn round_hundredth(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The x of the origin of each character's first glyph in the first line of `text` laid out in
/// `face` at `size` with `options`; `None` for a character that starts no glyph (a ligature's
/// second character) or when the layout fails.
fn origins(
    face: &Face,
    text: &str,
    size: f64,
    options: &LayoutOptions,
) -> Option<Vec<Option<f64>>> {
    let laid = layout(face, text, Mm(size), options).ok()?;
    let line = laid.lines.first()?;
    let mut out = Vec::new();
    for (byte, _) in text.char_indices() {
        out.push(
            line.glyphs
                .iter()
                .find(|g| g.cluster == byte)
                .map(|g| g.origin.x),
        );
    }
    Some(out)
}

/// The spaces before each letter: gaps wider than [`SPACE_SHARE`] of the space advance beyond
/// the letter spacing.
fn spaces_before(residuals: &[f64], letter_spacing: f64, space_advance: f64) -> Vec<usize> {
    let mut spaces = vec![0_usize; residuals.len() + 1];
    for (k, &r) in residuals.iter().enumerate() {
        let extra = r - letter_spacing;
        if extra > SPACE_SHARE * space_advance {
            let step = space_advance + letter_spacing;
            let count = (extra / step).round().clamp(1.0, 8.0);
            // invariant: clamped to 1 to 8.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let n = count as usize;
            spaces[k + 1] = n;
        }
    }
    spaces
}

/// A line's string as laid out: the text with its spaces, each letter's character position,
/// the spacing and each letter's laid-out x.
struct Spaced {
    text: String,
    positions: Vec<usize>,
    letter_spacing: f64,
    word_spacing: f64,
    char_x: Vec<Option<f64>>,
}

/// The string of `chars` measured at `measured` (each letter's traced origin x): spaces, letter
/// and word spacing as medians rounded to 0.01 mm, and the laid-out x of each letter.
fn spaced(
    face: &Face,
    chars: &[char],
    measured: &[f64],
    size: f64,
    fallback_space: f64,
) -> Result<Spaced, RecognitionError> {
    let plain: String = chars.iter().collect();
    // Without ligatures (any letter spacing turns them off), every character has its glyph.
    let measuring = LayoutOptions {
        letter_spacing: Mm(1e-9),
        ..LayoutOptions::default()
    };
    let natural = origins(face, &plain, size, &measuring).ok_or(RecognitionError::Layout)?;
    let space_advance = layout(face, " ", Mm(size), &LayoutOptions::default())
        .ok()
        .and_then(|l| l.lines.first().map(|line| line.width.0))
        .filter(|w| *w > 0.0)
        .unwrap_or(fallback_space);
    let residuals: Vec<f64> = (1..chars.len())
        .map(|k| {
            let natural_gap = match (natural[k], natural[k - 1]) {
                (Some(b), Some(a)) => b - a,
                _ => 0.0,
            };
            (measured[k] - measured[k - 1]) - natural_gap
        })
        .collect();
    let letter_spacing = round_hundredth(median(&mut residuals.clone()));
    let spaces = spaces_before(&residuals, letter_spacing, space_advance);
    let mut text = String::new();
    let mut positions = Vec::with_capacity(chars.len());
    for (k, c) in chars.iter().enumerate() {
        for _ in 0..spaces[k] {
            text.push(' ');
        }
        positions.push(text.chars().count());
        text.push(*c);
    }
    // The word spacing from the string with its spaces (a space ends the kerning of its pair).
    let mut options = LayoutOptions {
        letter_spacing: Mm(letter_spacing),
        ..LayoutOptions::default()
    };
    let with_spaces = origins(face, &text, size, &options).ok_or(RecognitionError::Layout)?;
    let mut per_space: Vec<f64> = (1..chars.len())
        .filter(|&k| spaces[k] > 0)
        .filter_map(|k| {
            let (a, b) = (with_spaces[positions[k - 1]]?, with_spaces[positions[k]]?);
            let n = f64::from(u32::try_from(spaces[k]).unwrap_or(1));
            Some(((measured[k] - measured[k - 1]) - (b - a)) / n)
        })
        .collect();
    let word_spacing = round_hundredth(median(&mut per_space));
    options.word_spacing = Mm(word_spacing);
    let laid = origins(face, &text, size, &options).ok_or(RecognitionError::Layout)?;
    let char_x = positions.iter().map(|&p| laid[p]).collect();
    Ok(Spaced {
        text,
        positions,
        letter_spacing,
        word_spacing,
        char_x,
    })
}

/// The text's anchor in line space: the median offset of the sure letters (of all letters when
/// none is sure) from their laid-out places.
fn anchor(letters: &[&Letter], char_x: &[Option<f64>]) -> Point {
    let offsets = |only_sure: bool| -> (Vec<f64>, Vec<f64>) {
        let picked = letters
            .iter()
            .zip(char_x)
            .filter(|(l, _)| l.sure || !only_sure);
        let dx = picked
            .clone()
            .filter_map(|(l, x)| x.map(|x| l.best.origin.x - x))
            .collect();
        let dy = picked.map(|(l, _)| l.best.origin.y).collect();
        (dx, dy)
    };
    let (mut dx, mut dy) = offsets(true);
    if dx.is_empty() {
        (dx, dy) = offsets(false);
    }
    Point::new(median(&mut dx), median(&mut dy))
}

/// The final check's findings: the guessed positions, the largest deviation of a sure letter,
/// the sure count and the outlines the run replaces.
struct Checked {
    guessed: Vec<usize>,
    max_deviation: f64,
    sure: usize,
    outlines: Vec<usize>,
}

/// The final check: every glyph where the layout puts it, measured without moving; a sure letter
/// over the tolerance is demoted to guessed.
fn check(
    letters: &[&Letter],
    spaced: &Spaced,
    anchor: Point,
    (prepared, glyphs, scale): (&Prepared, &[ReferenceGlyph], f64),
    tolerance: f64,
) -> Checked {
    let mut checked = Checked {
        guessed: Vec::new(),
        max_deviation: 0.0,
        sure: 0,
        outlines: Vec::new(),
    };
    for ((letter, x), &position) in letters.iter().zip(&spaced.char_x).zip(&spaced.positions) {
        let candidate = &prepared.candidates[letter.candidate];
        checked.outlines.extend_from_slice(&candidate.outlines);
        let deviation = x.map(|x| {
            let scaled = Scaled::new(
                &glyphs[letter.best.glyph],
                scale,
                1,
                super::verify::CAP_TOLERANCES * tolerance,
            );
            let fixed = Side {
                points: &candidate.points,
                grid: &candidate.grid,
            };
            let moved = Side {
                points: &scaled.points,
                grid: &scaled.grid,
            };
            two_sided(
                &fixed,
                &moved,
                (anchor + kurbo::Vec2::new(x, 0.0)).to_vec2(),
            )
            .0
            .max
        });
        match deviation {
            Some(d) if letter.sure && d <= tolerance => {
                checked.sure += 1;
                checked.max_deviation = checked.max_deviation.max(d);
            }
            _ => checked.guessed.push(position),
        }
    }
    checked.outlines.sort_unstable();
    checked.outlines.dedup();
    checked
}

/// The run of one line in `face`, or why there is none.
pub(crate) fn assemble(
    face: &Face,
    fit: &FaceFit,
    prepared: &Prepared,
    glyphs: &[ReferenceGlyph],
    tolerance: f64,
) -> Result<RecognisedRun, RecognitionError> {
    let letters: Vec<&Letter> = fit.letters.iter().filter(|l| !l.stays_path).collect();
    if letters.is_empty() {
        return Err(RecognitionError::NoLetters);
    }
    let chars: Vec<char> = letters.iter().map(|l| glyphs[l.best.glyph].c).collect();
    let size = fit.scale * face.metrics().cap_height;
    let measured: Vec<f64> = letters.iter().map(|l| l.best.origin.x).collect();
    let spaced = spaced(face, &chars, &measured, size, 0.25 * fit.scale)?;
    let anchor = anchor(&letters, &spaced.char_x);
    let checked = check(
        &letters,
        &spaced,
        anchor,
        (prepared, glyphs, fit.scale),
        tolerance,
    );
    let frame = &prepared.frame;
    let origin = frame.to_document(anchor);
    let n = frame.n();
    Ok(RecognisedRun {
        face: fit.face,
        text: spaced.text,
        size: Mm(size),
        placement: Affine::new([frame.u.x, frame.u.y, n.x, n.y, origin.x, origin.y]),
        letter_spacing: Mm(spaced.letter_spacing),
        word_spacing: Mm(spaced.word_spacing),
        max_deviation: Mm(checked.max_deviation),
        guessed: checked.guessed,
        outlines: checked.outlines,
        sure: checked.sure,
        letters: fit.letters.len(),
    })
}
