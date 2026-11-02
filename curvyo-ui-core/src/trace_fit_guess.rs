//! Sets a read line no face fits as text, every letter guessed (`specs/0091-trace-text-recognition`
//! Frage 114 B, criterion 29): in the plain included sans (the face that came nearest only when
//! that is not listed); its spaces where the ink has word gaps; cut into several texts
//! where the ink has much wider gaps.

use std::sync::Arc;

use curvyo_text_core::{Face, LayoutOptions, Mm, RecognisedRun, Recogniser, layout};
use kurbo::Vec2;

use crate::trace_fit::{FallbackFace, FitText, TraceFit, glyphs, unscaled};
use crate::trace_words::{ink_spans, plan_words};

/// A line leaning more than this many degrees is set in an italic cut (Proposal, tester round 3:
/// 3 degrees stays upright, 8 and 14 are italic).
pub const ITALIC_DEGREES: f64 = 6.0;

/// The stroke width of `text` as `face` draws it over the line's ink (the size `recogniser`'s
/// fit gives it), measured as the line's own: the same text, the same measure.
fn drawn_width(recogniser: &Recogniser, face: &Face, text: &str, direction: Vec2) -> Option<f64> {
    let run = recogniser.fit_text(face, text, direction)?;
    let laid = layout(face, text, run.size, &LayoutOptions::default()).ok()?;
    let drawn = Recogniser::new(&laid.glyph_paths(face), Mm(0.01));
    Some(drawn.stroke_width(Vec2::new(1.0, 0.0))?.get())
}

/// Where each character starts along the line: the first at the reader's position of it, each
/// other halfway between its position and the one before.
fn boundaries(positions: &[f64]) -> Vec<f64> {
    positions
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            if i == 0 {
                p
            } else {
                f64::midpoint(positions[i - 1], p)
            }
        })
        .collect()
}

impl TraceFit {
    /// The fallback face for line `source`: the included sans cut that, drawing the line's own
    /// text over its ink, has the stroke width nearest the line's (each weight measured as the
    /// line is); italic only when the line leans more than [`ITALIC_DEGREES`], whatever its
    /// weight; Regular when the strokes cannot be measured.
    fn fallback_face(
        &self,
        source: usize,
        face: &dyn Fn(usize) -> Option<Arc<Face>>,
    ) -> Option<usize> {
        let line = &self.lines[source];
        let recogniser = &self.job.recognisers()[source];
        let italic = recogniser.slant(line.direction) > ITALIC_DEGREES;
        let cuts: Vec<&FallbackFace> = {
            let wanted: Vec<_> = self
                .fallback
                .iter()
                .filter(|f| f.italic == italic)
                .collect();
            if wanted.is_empty() {
                self.fallback.iter().collect()
            } else {
                wanted
            }
        };
        let regular = cuts
            .iter()
            .find(|f| f.regular)
            .or_else(|| cuts.first())
            .map(|f| f.face)?;
        let Some(measured) = recogniser.stroke_width(line.direction) else {
            return Some(regular);
        };
        Some(
            cuts.iter()
                .filter_map(|f| {
                    let found = face(f.face)?;
                    let width = drawn_width(recogniser, &found, &line.text, line.direction)?;
                    Some((f.face, (width - measured.get()).abs()))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                .map_or(regular, |(index, _)| index),
        )
    }

    /// Line `source` set as text with every letter guessed: in the fallback face when no face
    /// gave it a sure letter (else in the face that came nearest on it, else `overall`), spaced
    /// at its word gaps and cut at its wide gaps.
    pub(crate) fn guessed_texts(
        &self,
        source: usize,
        face: &dyn Fn(usize) -> Option<Arc<Face>>,
        overall: Option<usize>,
    ) -> Vec<FitText> {
        let job = &self.job;
        let line = &self.lines[source];
        // Criterion 29 (tester round 3): a line that ends with no sure letter is set in the
        // fallback sans whatever single letters the search found; the nearest face only when
        // the sans is not listed.
        let index = if self.fallback.is_empty() {
            job.line_nearest()
                .iter()
                .filter(|(r, _, _)| *r == source)
                .filter_map(|(_, _, near)| *near)
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                .map(|(index, _)| index)
                .or(overall)
        } else {
            self.fallback_face(source, face)
        };
        let Some((index, found)) = index.and_then(|i| Some((i, face(i)?))) else {
            return Vec::new();
        };
        let recogniser = &job.recognisers()[source];
        let Some(whole) = recogniser.fit_text(&found, &line.text, line.direction) else {
            return Vec::new();
        };
        let (outlines, tolerance) = &self.scaled[source];
        let length = line.direction.hypot();
        let u = if length > 0.0 {
            line.direction / length
        } else {
            Vec2::new(1.0, 0.0)
        };
        let k = self.scales[source];
        let positions: Vec<f64> = line.positions.iter().map(|p| p * k).collect();
        let em = layout(&found, &line.text, whole.size, &LayoutOptions::default())
            .map_or(0.0, |laid| laid.mm_per_em);
        let parts = if positions.len() == line.text.chars().count() {
            plan_words(
                &line.text,
                &boundaries(&positions),
                &ink_spans(outlines, u),
                em,
            )
        } else {
            Vec::new()
        };
        let text_of = |run: RecognisedRun| {
            let run = unscaled(run, k);
            FitText {
                glyphs: glyphs(&found, &run),
                run,
                face: index,
                nearest: true,
                stroke: None,
            }
        };
        if parts.len() <= 1 {
            let mut run = whole;
            if let Some(part) = parts.first()
                && part.text != line.text
                && let Some(spaced) = recogniser.fit_text(&found, &part.text, line.direction)
            {
                run = spaced;
            }
            return vec![text_of(run)];
        }
        parts
            .iter()
            .filter_map(|part| {
                let mine: Vec<usize> = (0..outlines.len())
                    .filter(|&i| {
                        ink_spans(&outlines[i..=i], u).first().is_some_and(|s| {
                            f64::midpoint(s.0, s.1) >= part.span.0
                                && f64::midpoint(s.0, s.1) <= part.span.1
                        })
                    })
                    .collect();
                let paths: Vec<_> = mine.iter().map(|&i| outlines[i].clone()).collect();
                let mut run = Recogniser::with_hint(&paths, *tolerance, &part.text).fit_text(
                    &found,
                    &part.text,
                    line.direction,
                )?;
                run.outlines = run.outlines.iter().map(|&i| mine[i]).collect();
                Some(text_of(run))
            })
            .collect()
    }
}
