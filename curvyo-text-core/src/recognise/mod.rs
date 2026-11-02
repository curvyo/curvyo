//! Runs the recognition stages in a fixed order and returns the result
//! (`specs/0088-trace-recognition` Part A, `adrs.md` decisions 2 to 7).
//!
//! Outlines (closed, in document millimetres, y down) are sampled, nested into shapes and grouped
//! into lines once ([`Recogniser::new`]); each line is then matched against one tier of faces
//! ([`Recogniser::match_line`]), so a caller can search tier by tier, stop early and report
//! progress between lines. Basic
//! operations and `sqrt` only, fixed orders, no hash maps: the same input gives the same result
//! natively and in wasm.

mod assemble;
mod blobs;
mod candidates;
mod confusable;
mod descriptor;
mod distance;
mod fit;
mod lines;
mod reference;
mod sample;
mod shape;
mod shortlist;
mod size;
mod stroke;
mod stroke_grid;
mod verify;

use std::sync::Arc;

use kurbo::{Affine, BezPath};

pub(crate) use reference::ReferenceGlyphs;
pub use stroke::StrokeFit;

use crate::face::Face;
use crate::units::Mm;
use blobs::{Blob, Loop, nest};
use candidates::prepare;
use lines::{Frame, Line, group_lines};
use shortlist::rank_faces;
use verify::{FaceFit, fit_face, origin_direction, resolve_coinciding};

/// The sample spacing of traced outlines, mm (`adrs.md` decision 3).
pub(crate) const SAMPLE_SPACING: Mm = Mm(0.05);
/// The most samples one outline may have at [`SAMPLE_SPACING`]; a longer outline is sampled
/// coarser, up to half the tolerance apart, and left as a path beyond that (Proposal).
pub(crate) const MAX_OUTLINE_POINTS: usize = 4_000;
/// How often a line's baseline direction is refined at most.
const REFITS: usize = 8;

/// One recognised line: a text in one face.
#[derive(Debug, Clone, PartialEq)]
pub struct RecognisedRun {
    /// The face's index in the tier.
    pub face: usize,
    /// The string, letters and spaces.
    pub text: String,
    /// The cap height.
    pub size: Mm,
    /// Text space (x along the baseline, y down, origin at the anchor on the baseline) to the
    /// outlines' space.
    pub placement: Affine,
    /// The extra width after each cluster.
    pub letter_spacing: Mm,
    /// The extra width of each space.
    pub word_spacing: Mm,
    /// The largest distance of a sure letter's laid-out glyph from its traced shape.
    pub max_deviation: Mm,
    /// The character positions in `text` of the guessed letters, ascending.
    pub guessed: Vec<usize>,
    /// The input outlines the text replaces, ascending.
    pub outlines: Vec<usize>,
    /// The sure letters.
    pub sure: usize,
    /// The glyphs the line was cut into, those too far from every character (left as paths)
    /// included: the 50 % and 80 % rules count sure letters against these, so a line whose
    /// shapes mostly match nothing (mirrored text, a drawing) stays paths.
    pub letters: usize,
}

/// The face that came nearest when nothing was within the tolerance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearestFace {
    /// The face's index in the tier.
    pub face: usize,
    /// The largest distance of any of its letters on the line.
    pub max_deviation: Mm,
}

/// One line matched against one tier.
#[derive(Debug, Clone, PartialEq)]
pub struct LineMatch {
    /// The text, when at least half of its letters are sure.
    pub run: Option<RecognisedRun>,
    /// The sure letters of the best face (after the final check when a run was assembled).
    pub sure: usize,
    /// The letters of the best face.
    pub letters: usize,
    /// The best face's summed mean distance, mm (ranks equal sure counts across tiers).
    pub mean: f64,
    /// The face that came nearest.
    pub nearest: Option<NearestFace>,
    /// Every input outline of the line, ascending.
    pub outlines: Vec<usize>,
}

impl LineMatch {
    /// Whether this match ranks above `other`: more sure letters, then the lower mean.
    #[must_use]
    pub fn beats(&self, other: &Self) -> bool {
        self.sure > other.sure || (self.sure == other.sure && self.mean < other.mean)
    }
}

/// Why a line gave no run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecognitionError {
    NoLetters,
    Layout,
}

/// The outlines of one selection, nested and grouped into lines, ready to match line by line.
#[derive(Debug, Clone)]
pub struct Recogniser {
    loops: Vec<Loop>,
    blobs: Vec<Blob>,
    lines: Vec<Line>,
    skipped: Vec<usize>,
    tolerance: f64,
    /// The characters a hinted line may hold (`specs/0091` `adrs.md` decision 4); `None`: all.
    hint: Option<Vec<char>>,
}

fn sample(path: &BezPath, tolerance: f64) -> Option<Vec<sample::Ring>> {
    let spacing = SAMPLE_SPACING.get();
    if let Some(rings) = sample::sample_closed(path, spacing, MAX_OUTLINE_POINTS) {
        return Some(rings);
    }
    let coarse = (tolerance / 2.0).max(spacing);
    sample::sample_closed(path, coarse, MAX_OUTLINE_POINTS)
}

impl Recogniser {
    /// Samples, nests and groups `outlines` (stages 1 and 2). Each outline should be one closed
    /// subpath; an open one, or one too long to sample, is skipped and stays a path.
    #[must_use]
    pub fn new(outlines: &[BezPath], tolerance: Mm) -> Self {
        let tolerance = tolerance.get().max(1e-3);
        let mut loops = Vec::new();
        let mut skipped = Vec::new();
        for (index, path) in outlines.iter().enumerate() {
            match sample(path, tolerance) {
                Some(rings) => loops.extend(rings.into_iter().map(|r| Loop::new(r, index))),
                None => skipped.push(index),
            }
        }
        let blobs = nest(&loops);
        let lines = group_lines(&blobs);
        Self {
            loops,
            blobs,
            lines,
            skipped,
            tolerance,
            hint: None,
        }
    }

    /// As [`Recogniser::new`], with the string an OCR read there as a hint: every line is matched
    /// against the characters of `text` and their look-alikes only (`specs/0091` `adrs.md`
    /// decision 4). Size, baseline, spacing and the tolerance check are unchanged; the matcher
    /// decides, the hint only narrows.
    #[must_use]
    pub fn with_hint(outlines: &[BezPath], tolerance: Mm, text: &str) -> Self {
        Self {
            hint: Some(confusable::hint_chars(text)),
            ..Self::new(outlines, tolerance)
        }
    }

    /// The number of lines found.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// The outlines that were not read (open or too long); they stay paths.
    #[must_use]
    pub fn skipped(&self) -> &[usize] {
        &self.skipped
    }

    /// Every input outline of line `line`, ascending.
    #[must_use]
    pub fn line_outlines(&self, line: usize) -> Vec<usize> {
        let mut outlines: Vec<usize> = self.lines.get(line).map_or_else(Vec::new, |l| {
            l.blobs
                .iter()
                .flat_map(|&b| self.blobs[b].loops.iter().map(|&i| self.loops[i].outline))
                .collect()
        });
        outlines.sort_unstable();
        outlines.dedup();
        outlines
    }

    /// Matches line `line` against the faces of one tier (stages 3 to 7). Single-line faces
    /// never match (a traced stroke is an outline, not a centre line).
    #[must_use]
    pub fn match_line(&self, line: usize, tier: &[&Face]) -> LineMatch {
        let outlines = self.line_outlines(line);
        let empty = LineMatch {
            run: None,
            sure: 0,
            letters: 0,
            mean: f64::INFINITY,
            nearest: None,
            outlines: outlines.clone(),
        };
        let Some(found) = self.lines.get(line) else {
            return empty;
        };
        let spacing = SAMPLE_SPACING.get();
        let tolerance = self.tolerance;
        let cap = verify::CAP_TOLERANCES * tolerance;
        let references: Vec<Arc<ReferenceGlyphs>> =
            tier.iter().map(|face| face.reference_glyphs()).collect();
        let frame = candidates::levelled(found.frame, &found.blobs, &self.blobs, &self.loops);
        let prepared = prepare(frame, &found.blobs, &self.blobs, &self.loops, spacing, cap);
        let (fits, nearest) = fits_of(
            &prepared,
            &references,
            (tolerance, spacing),
            self.hint.as_deref(),
        );
        let Some(best) = fits.into_iter().next() else {
            return LineMatch { nearest, ..empty };
        };
        let face = best.face;
        let (best, prepared) = self.refine_direction(line, best, prepared, &references[face]);
        let glyphs = &references[best.face].glyphs;
        let mean = best.mean_sum();
        match assemble::assemble(tier[best.face], &best, &prepared, glyphs, tolerance) {
            Ok(run) => LineMatch {
                sure: run.sure,
                letters: run.letters,
                mean,
                nearest,
                outlines,
                run: (run.letters > 0
                    && 2 * run.sure >= run.letters
                    && sure_letters_or_digits(&run) >= MIN_SURE_LETTERS)
                    .then_some(run),
            },
            Err(_) => LineMatch {
                sure: best.sure(),
                letters: best.letters.len(),
                mean,
                nearest,
                ..empty
            },
        }
    }
    /// Refits `best` in a frame turned through its letters' origins until the direction settles
    /// (at most [`REFITS`] times); a refit that loses a sure letter is not taken.
    fn refine_direction(
        &self,
        line: usize,
        mut best: FaceFit,
        mut prepared: candidates::Prepared,
        references: &Arc<ReferenceGlyphs>,
    ) -> (FaceFit, candidates::Prepared) {
        let spacing = SAMPLE_SPACING.get();
        let tolerance = self.tolerance;
        let cap = verify::CAP_TOLERANCES * tolerance;
        let Some(found) = self.lines.get(line) else {
            return (best, prepared);
        };
        // A hinted line with fewer than half its letters sure becomes no run whatever its
        // direction; its text is set in the nearest face (`specs/0091` decision 5).
        if self.hint.is_some() && 2 * best.sure() < best.counted() {
            return (best, prepared);
        }
        let glyphs = &references.glyphs;
        for _ in 0..REFITS {
            let Some(direction) = origin_direction(&best, prepared.height) else {
                break;
            };
            if direction.y.abs() <= 5e-5 {
                break;
            }
            let frame = prepared.frame;
            let u = frame.u * direction.x + frame.n() * direction.y;
            let length = u.hypot2().sqrt();
            if length.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
                break;
            }
            let turned = Frame {
                origin: frame.origin,
                u: u / length,
            };
            let again = prepare(turned, &found.blobs, &self.blobs, &self.loops, spacing, cap);
            let refit = rank_faces(
                &again,
                std::slice::from_ref(references),
                self.hint.as_deref(),
            )
            .into_iter()
            .next()
            .and_then(|(_, _, lists)| {
                let mut fit = fit_face(&again, best.face, glyphs, &lists, tolerance, spacing)?;
                resolve_coinciding(&mut fit, glyphs);
                Some(fit)
            });
            match refit {
                Some(refit) if refit.sure() >= best.sure() && refit.counted() >= best.counted() => {
                    best = refit;
                    prepared = again;
                }
                _ => break,
            }
        }
        (best, prepared)
    }
}

/// The faces of a tier fitted to a prepared line, best first (most sure letters, then the lower
/// summed mean, then face order), and the face that came nearest (stages 4 to 6).
fn fits_of(
    prepared: &candidates::Prepared,
    references: &[Arc<ReferenceGlyphs>],
    (tolerance, spacing): (f64, f64),
    allowed: Option<&[char]>,
) -> (Vec<FaceFit>, Option<NearestFace>) {
    let ranked = rank_faces(prepared, references, allowed);
    let mut fits: Vec<FaceFit> = Vec::new();
    // The descriptor sum of the first face that had every letter sure.
    let mut all_sure: Option<f64> = None;
    for (face, sum, lists) in ranked {
        if all_sure.is_some_and(|first| sum > verify::RANK_STOP * first) {
            break;
        }
        let glyphs = &references[face].glyphs;
        let Some(mut fit) = fit_face(prepared, face, glyphs, &lists, tolerance, spacing) else {
            continue;
        };
        resolve_coinciding(&mut fit, glyphs);
        if all_sure.is_none() && fit.counted() > 0 && fit.sure() == fit.counted() {
            all_sure = Some(sum);
        }
        fits.push(fit);
    }
    // A lone dot or comma fits any face: the nearest face is named from fits of several
    // glyphs when there are any.
    let several = fits.iter().any(|fit| fit.counted() >= MIN_SURE_LETTERS);
    let nearest = fits
        .iter()
        .filter(|fit| !several || fit.counted() >= MIN_SURE_LETTERS)
        .min_by(|a, b| {
            a.largest()
                .total_cmp(&b.largest())
                .then(a.face.cmp(&b.face))
        })
        .map(|fit| NearestFace {
            face: fit.face,
            max_deviation: Mm(shape::largest_uncapped(
                fit,
                prepared,
                &references[fit.face].glyphs,
            )),
        });
    fits.sort_by(|a, b| {
        b.sure()
            .cmp(&a.sure())
            .then(a.mean_sum().total_cmp(&b.mean_sum()))
            .then(a.face.cmp(&b.face))
    });
    (fits, nearest)
}

/// The fewest sure letters or digits a line needs to become text (Proposal): a line of
/// punctuation or a lone dot has no baseline to check, and decoration dots match a full stop.
const MIN_SURE_LETTERS: usize = 2;

/// The sure letters and digits of `run`.
fn sure_letters_or_digits(run: &RecognisedRun) -> usize {
    run.text
        .chars()
        .enumerate()
        .filter(|(i, c)| c.is_alphanumeric() && !run.guessed.contains(i))
        .count()
}
