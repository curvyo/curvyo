//! Sets a read string in a given face over a line's outlines when no face matched it
//! (`specs/0091-trace-text-recognition` Frage 114 B, `adrs.md` decision 5).
//!
//! The string is laid out once at size 1 mm; its ink box is scaled so its height is the height
//! of the outlines' ink across the line direction, the letter spacing takes up the difference in
//! width, and the ink's left and top edges are put on the outlines'. Every letter is guessed.

use kurbo::{Affine, Shape, Vec2};

use super::{RecognisedRun, Recogniser};
use crate::face::Face;
use crate::layout::{LayoutOptions, layout};
use crate::units::Mm;

/// The letter spacing stays within these shares of the size (Proposal): a wrong count of letters
/// must not spread the text apart or stack it.
const SPACING_RANGE: (f64, f64) = (-0.3, 1.5);

/// The ink box of `text` in `face` at 1 mm with `spacing` mm after each cluster, in text space
/// (x along the baseline, y down): `(x0, y0, x1, y1)`.
fn ink_box(face: &Face, text: &str, spacing: f64) -> Option<(f64, f64, f64, f64)> {
    let options = LayoutOptions {
        letter_spacing: Mm(spacing),
        ..LayoutOptions::default()
    };
    let laid = layout(face, text, Mm(1.0), &options).ok()?;
    let line = laid.lines.first()?;
    let mut ink: Option<(f64, f64, f64, f64)> = None;
    for glyph in &line.glyphs {
        let outline = face.outline(glyph.glyph);
        if outline.path.elements().is_empty() {
            continue;
        }
        let b = outline.path.bounding_box();
        let k = laid.mm_per_em;
        let (x0, y0) = (glyph.origin.x + b.x0 * k, glyph.origin.y + b.y0 * k);
        let (x1, y1) = (glyph.origin.x + b.x1 * k, glyph.origin.y + b.y1 * k);
        ink = Some(ink.map_or((x0, y0, x1, y1), |(a, b, c, d)| {
            (a.min(x0), b.min(y0), c.max(x1), d.max(y1))
        }));
    }
    ink
}

/// `text` in `face` set over the box `(x0, y0, x1, y1)` of a line frame (x along the line, y
/// down): the size from the box's height, the letter spacing from its width (clamped), and
/// where text space's origin goes so the ink's left and top edges lie on the box's:
/// `(size, spacing, origin)`. `None` for a text or face that draws nothing.
pub(super) fn set_over(
    face: &Face,
    text: &str,
    (a0, b0, a1, b1): (f64, f64, f64, f64),
) -> Option<(f64, f64, Vec2)> {
    let (x0, y0, x1, y1) = ink_box(face, text, 0.0)?;
    if y1 - y0 <= 0.0 || x1 - x0 <= 0.0 {
        return None;
    }
    let size = (b1 - b0) / (y1 - y0);
    let clusters = text.chars().count().max(2) - 1;
    // invariant: a count of characters, far below 2^52.
    #[allow(clippy::cast_precision_loss)]
    let spacing = (((a1 - a0) - size * (x1 - x0)) / clusters as f64)
        .clamp(SPACING_RANGE.0 * size, SPACING_RANGE.1 * size);
    // The ink's left edge at that spacing (a leading glyph's side bearing does not change).
    let (x0, y0, _, _) = ink_box(face, text, spacing / size)?;
    Some((size, spacing, Vec2::new(a0 - size * x0, b0 - size * y0)))
}

impl Recogniser {
    /// `text` set in `face` over every read outline, along `direction` (outline space, pointing
    /// along the line; y down): the size from the ink height, the letter spacing from the ink
    /// width, every letter guessed. `None` for an empty text, no outline or a face that draws
    /// nothing of it.
    #[must_use]
    #[allow(clippy::many_single_char_names)] // the frame u, n and its coordinates a, b
    pub fn fit_text(&self, face: &Face, text: &str, direction: Vec2) -> Option<RecognisedRun> {
        let length = direction.hypot();
        if text.trim().is_empty() || self.loops.is_empty() || length.partial_cmp(&0.0)?.is_le() {
            return None;
        }
        let u = direction / length;
        let n = Vec2::new(-u.y, u.x);
        let (mut a0, mut a1, mut b0, mut b1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for p in self.loops.iter().flat_map(|l| l.points.iter()) {
            let v = p.to_vec2();
            let (a, b) = (v.dot(u), v.dot(n));
            a0 = a0.min(a);
            a1 = a1.max(a);
            b0 = b0.min(b);
            b1 = b1.max(b);
        }
        let (size, spacing, at) = set_over(face, text, (a0, b0, a1, b1))?;
        let origin = u * at.x + n * at.y;
        let mut outlines: Vec<usize> = self.loops.iter().map(|l| l.outline).collect();
        outlines.sort_unstable();
        outlines.dedup();
        let guessed: Vec<usize> = text
            .chars()
            .enumerate()
            .filter(|(_, c)| !c.is_whitespace())
            .map(|(i, _)| i)
            .collect();
        Some(RecognisedRun {
            face: 0,
            text: text.to_owned(),
            size: Mm(size),
            placement: Affine::new([u.x, u.y, n.x, n.y, origin.x, origin.y]),
            letter_spacing: Mm(spacing),
            word_spacing: Mm(0.0),
            max_deviation: Mm(0.0),
            letters: guessed.len(),
            guessed,
            outlines,
            sure: 0,
        })
    }
}
