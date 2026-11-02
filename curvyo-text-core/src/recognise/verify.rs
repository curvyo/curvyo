//! Fits one face to a line's glyph candidates and judges its letters (`adrs.md` decision 3,
//! stages 5 and 6).
//!
//! Each shortlisted character (`shortlist.rs`) is placed at the line's median size and baseline, moved by a few
//! translation-only steps and measured. A dynamic programme picks the segmentation, the lowest mean
//! wins per candidate, and a letter is sure within the tolerance and a 15 % margin.

use kurbo::{Point, Vec2};

use super::candidates::{COARSE_PER_HEIGHT, Candidate, Prepared, stride, thin};
use super::distance::{Distance, SegmentGrid, Side, two_sided};
use super::reference::ReferenceGlyph;
use super::sample::{Ring, f};
use super::shape::{RESIDUAL_MARGIN, residual, residual_limit};
use super::shortlist::Shortlist;
use super::size::{refined, size_and_baseline};

/// Once a face has every letter sure, a face whose descriptor sum is this many times its sum is
/// not verified any more (Proposal: the shortlist already tells such faces apart).
pub(crate) const RANK_STOP: f64 = 1.3;
/// The share of the second best mean a letter must beat it by (Proposal, `adrs.md` decision 3).
pub(crate) const MARGIN: f64 = 0.15;
/// A sure letter's largest distance is at most this share of the line's median shape height,
/// whatever the tolerance (Proposal, tester finding 2026-10-31): a 2 mm tolerance on 3 mm text
/// lets any glyph of a neighbouring family through; a bolder cut of the right letter stays
/// within about 11 %.
const SURE_SHARE: f64 = 0.12;
/// A candidate whose best fit is further than this many tolerances is no letter: it stays a path
/// and the text goes on past it (Proposal).
const NO_LETTER: f64 = 4.0;
/// Distances are measured up to this many tolerances; farther is far enough.
pub(crate) const CAP_TOLERANCES: f64 = 8.0;
/// The longest a translation step may move a glyph from its first place, in em.
const SHIFT_LIMIT_EM: f64 = 0.15;
/// Translation steps per fit.
const STEPS: usize = 3;
/// The characters per candidate that get every translation step.
const REFINED: usize = 2;

/// A reference glyph at one size, ready to measure: points with the origin at (0, 0).
pub(crate) struct Scaled {
    /// Every point at the reference spacing.
    pub points: Vec<Point>,
    /// The points at the stride asked for.
    pub coarse: Vec<Point>,
    /// The grid of the segments between the strided points.
    pub grid: SegmentGrid,
    /// The strided rings, for telling inside from outside.
    pub coarse_rings: Vec<Ring>,
}

impl Scaled {
    /// The glyph at `scale`, its grid built from every `stride`-th point.
    pub(crate) fn new(glyph: &ReferenceGlyph, scale: f64, stride: usize, cap: f64) -> Self {
        let rings: Vec<Ring> = glyph
            .rings
            .iter()
            .map(|r| {
                r.iter()
                    .map(|p| Point::new(p.x * scale, p.y * scale))
                    .collect()
            })
            .collect();
        let coarse_rings = thin(&rings, stride);
        let refs: Vec<&[Point]> = coarse_rings.iter().map(Vec::as_slice).collect();
        Self {
            coarse: coarse_rings.iter().flatten().copied().collect(),
            grid: SegmentGrid::new(&refs, cap),
            coarse_rings: coarse_rings.clone(),
            points: rings.into_iter().flatten().collect(),
        }
    }
}

/// One character tried on one candidate.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Trial {
    /// The reference glyph's index in the face.
    pub glyph: usize,
    pub distance: Distance,
    /// Where the glyph's origin sits, line space.
    pub origin: Point,
}

/// Moves the glyph from `start` by translation steps and measures it.
fn fit(candidate: &Candidate, glyph: &Scaled, start: Point, limit: f64, steps: usize) -> Trial {
    let fixed = Side {
        points: &candidate.coarse,
        grid: &candidate.coarse_grid,
    };
    let moved = Side {
        points: &glyph.coarse,
        grid: &glyph.grid,
    };
    let mut offset = start.to_vec2();
    for _ in 0..steps {
        let (_, pull) = two_sided(&fixed, &moved, offset);
        let mut next = offset + pull - start.to_vec2();
        let length = next.hypot2().sqrt();
        if length > limit {
            next *= limit / length;
        }
        let next = start.to_vec2() + next;
        let moved_by = (next - offset).hypot2();
        offset = next;
        if moved_by < 1e-6 {
            break;
        }
    }
    let (distance, _) = two_sided(&fixed, &moved, offset);
    Trial {
        glyph: 0,
        distance,
        origin: offset.to_point(),
    }
}

/// A chosen letter of a face fit.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Letter {
    /// The candidate's index.
    pub candidate: usize,
    /// The best trial (fine).
    pub best: Trial,
    /// The second best character, if one was tried.
    pub second: Option<Trial>,
    /// Within the tolerance with the margin.
    pub sure: bool,
    /// The best and second both fit within the tolerance and coincide: the neighbours decide.
    pub coincide: bool,
    /// Too far from every character: it stays a path.
    pub stays_path: bool,
}

/// One face fitted to one line.
#[derive(Debug, Clone)]
pub(crate) struct FaceFit {
    /// The face's index in the tier.
    pub face: usize,
    /// Millimetres per em.
    pub scale: f64,
    /// The letters in reading order, the segmentation's choice.
    pub letters: Vec<Letter>,
}

impl FaceFit {
    /// The sure letters.
    pub(crate) fn sure(&self) -> usize {
        self.letters.iter().filter(|l| l.sure).count()
    }

    /// The letters that are text (sure or guessed).
    pub(crate) fn counted(&self) -> usize {
        self.letters.iter().filter(|l| !l.stays_path).count()
    }

    /// The summed best mean of the letters that are text.
    pub(crate) fn mean_sum(&self) -> f64 {
        self.letters
            .iter()
            .filter(|l| !l.stays_path)
            .map(|l| l.best.distance.mean)
            .sum()
    }

    /// The largest distance of any letter: how near the face comes, judged as the tolerance
    /// judges (the notice's "nearest face", lead decision 2026-10-31).
    pub(crate) fn largest(&self) -> f64 {
        self.letters
            .iter()
            .map(|l| l.best.distance.max)
            .fold(None, |most: Option<f64>, d| {
                Some(most.map_or(d, |m| m.max(d)))
            })
            .unwrap_or(f64::INFINITY)
    }
}

/// The cost of a candidate's best trial in the segmentation.
fn cost(candidate: &Candidate, trial: &Trial, tolerance: f64) -> f64 {
    let q = trial.distance.mean / tolerance;
    let per_shape = if trial.distance.max <= tolerance {
        q
    } else {
        2.0 + q.min(8.0)
    };
    f(candidate.len) * per_shape + 0.1
}

/// The shortlisted characters of one face placed on every candidate at `scale` and `baseline`,
/// best mean first (stage 5): all coarse, the best [`REFINED`] with every translation step.
fn trials(
    prepared: &Prepared,
    glyphs: &[ReferenceGlyph],
    lists: &[Vec<(usize, f64)>],
    (scale, baseline): (f64, f64),
    tolerance: f64,
    spacing: f64,
) -> (Vec<Vec<Trial>>, Vec<Option<Scaled>>) {
    let coarse_stride = stride(
        0.004 * scale,
        (prepared.height / COARSE_PER_HEIGHT).max(spacing),
    );
    let cap = CAP_TOLERANCES * tolerance;
    let mut scaled: Vec<Option<Scaled>> = (0..glyphs.len()).map(|_| None).collect();
    let limit = SHIFT_LIMIT_EM * scale;
    let by_mean = |a: &Trial, b: &Trial| {
        a.distance
            .mean
            .total_cmp(&b.distance.mean)
            .then(a.glyph.cmp(&b.glyph))
    };
    let mut out: Vec<Vec<Trial>> = Vec::with_capacity(prepared.candidates.len());
    for (candidate, list) in prepared.candidates.iter().zip(lists) {
        let mut tried = Vec::with_capacity(list.len());
        for &(g, _) in list {
            let reference = &glyphs[g];
            let glyph =
                scaled[g].get_or_insert_with(|| Scaled::new(reference, scale, coarse_stride, cap));
            let start = Point::new(candidate.bbox.x0 - reference.bbox.x0 * scale, baseline);
            let mut trial = fit(candidate, glyph, start, limit, 1);
            trial.glyph = g;
            tried.push((trial, start));
        }
        tried.sort_by(|a, b| by_mean(&a.0, &b.0));
        let mut tried: Vec<Trial> = tried
            .into_iter()
            .enumerate()
            .map(|(rank, (trial, start))| {
                if rank >= REFINED {
                    return trial;
                }
                let Some(glyph) = scaled[trial.glyph].as_ref() else {
                    return trial;
                };
                let mut refined = fit(candidate, glyph, trial.origin, limit, STEPS);
                // The limit stays measured from the first place.
                if (refined.origin - start).hypot2().sqrt() > limit {
                    return trial;
                }
                refined.glyph = trial.glyph;
                refined
            })
            .collect();
        tried.sort_by(by_mean);
        out.push(tried);
    }
    (out, scaled)
}

/// The candidates of the segmentation with the lowest cost, in line order (a dynamic programme
/// over the shapes); `None` when no segmentation covers the line.
fn segmentation(prepared: &Prepared, trials: &[Vec<Trial>], tolerance: f64) -> Option<Vec<usize>> {
    let n = prepared.blobs.len();
    let mut best: Vec<(f64, Option<usize>)> = vec![(f64::INFINITY, None); n + 1];
    best[0].0 = 0.0;
    for k in 0..n {
        if !best[k].0.is_finite() {
            continue;
        }
        for (index, candidate) in prepared.candidates.iter().enumerate() {
            if candidate.first != k {
                continue;
            }
            let Some(trial) = trials[index].first() else {
                continue;
            };
            let total = best[k].0 + prepared.cuts[k] + cost(candidate, trial, tolerance);
            let end = k + candidate.len;
            if total < best[end].0 {
                best[end] = (total, Some(index));
            }
        }
    }
    let mut chosen = Vec::new();
    let mut at = n;
    while at > 0 {
        let index = best[at].1?;
        chosen.push(index);
        at = prepared.candidates[index].first;
    }
    chosen.reverse();
    Some(chosen)
}

/// The face fitted at `scale` and `baseline` (stages 5 and 6).
fn fit_at(
    prepared: &Prepared,
    face: usize,
    (glyphs, lists): (&[ReferenceGlyph], &[Shortlist]),
    (scale, baseline): (f64, f64),
    tolerance: f64,
    spacing: f64,
) -> Option<FaceFit> {
    let (trials, mut scaled) = trials(
        prepared,
        glyphs,
        lists,
        (scale, baseline),
        tolerance,
        spacing,
    );
    let slack = 0.5 * (prepared.height / COARSE_PER_HEIGHT).max(spacing);
    let coarse_stride = stride(
        0.004 * scale,
        (prepared.height / COARSE_PER_HEIGHT).max(spacing),
    );
    let cap = CAP_TOLERANCES * tolerance;
    let chosen = segmentation(prepared, &trials, tolerance)?;
    // Each glyph is scaled once per fit, coarse (shared with the trials) and fine.
    let mut fine: Vec<Option<Scaled>> = (0..glyphs.len()).map(|_| None).collect();
    let mut letters = Vec::with_capacity(chosen.len());
    for index in chosen {
        let candidate = &prepared.candidates[index];
        let tried = &trials[index];
        let mut residual_of = |trial: &Trial| {
            let glyph = scaled[trial.glyph].get_or_insert_with(|| {
                Scaled::new(&glyphs[trial.glyph], scale, coarse_stride, cap)
            });
            residual(candidate, glyph, trial.origin)
        };
        let limit = residual_limit(prepared.height);
        let first = residual_of(&tried[0]);
        let next = tried.get(1).and_then(&mut residual_of);
        let shapes = Shapes {
            best: first.is_some_and(|r| r <= limit),
            second: next.is_some_and(|r| r <= limit),
            apart: first.is_some_and(|r| next.is_none_or(|n| r <= RESIDUAL_MARGIN * n)),
        };
        // Within the tolerance on the fine outlines: coarse points cut a sharp tip (a 'W' at
        // 36 mm) by more than the tolerance.
        let fine = fine[tried[0].glyph]
            .get_or_insert_with(|| Scaled::new(&glyphs[tried[0].glyph], scale, 1, cap));
        let fixed = Side {
            points: &candidate.points,
            grid: &candidate.grid,
        };
        let moved = Side {
            points: &fine.points,
            grid: &fine.grid,
        };
        let fine_max = two_sided(&fixed, &moved, tried[0].origin.to_vec2()).0.max;
        // However wide the tolerance, a sure letter lies within a share of the line height.
        let shapes = Shapes {
            best: shapes.best && fine_max <= SURE_SHARE * prepared.height,
            ..shapes
        };
        letters.push(judge(index, tried, (tolerance, slack, fine_max), shapes));
    }
    Some(FaceFit {
        face,
        scale,
        letters,
    })
}

/// Fits one face to the line (stages 5 and 6): at the voted size, then once more at the size
/// its fitted letters give when that differs by more than a quarter of the tolerance over the
/// line's height; the fit with more sure letters (then the lower mean) is kept.
pub(crate) fn fit_face(
    prepared: &Prepared,
    face: usize,
    glyphs: &[ReferenceGlyph],
    lists: &[Vec<(usize, f64)>],
    tolerance: f64,
    spacing: f64,
) -> Option<FaceFit> {
    let start = size_and_baseline(prepared, glyphs, lists)?;
    let first = fit_at(prepared, face, (glyphs, lists), start, tolerance, spacing)?;
    let Some(again) = refined(&first, prepared, glyphs, tolerance) else {
        return Some(first);
    };
    let moved = (again.0 - start.0).abs() / start.0 * prepared.height + (again.1 - start.1).abs();
    if moved <= 0.25 * tolerance {
        return Some(first);
    }
    let Some(second) = fit_at(prepared, face, (glyphs, lists), again, tolerance, spacing) else {
        return Some(first);
    };
    let better = second.sure() > first.sure()
        || (second.sure() == first.sure() && second.mean_sum() < first.mean_sum());
    Some(if better { second } else { first })
}

/// What the shape check says of a letter's best and second character.
#[derive(Debug, Clone, Copy)]
struct Shapes {
    /// The best has the traced shape.
    best: bool,
    /// The second has it too.
    second: bool,
    /// The best's residual stands out from the second's.
    apart: bool,
}

/// A letter from its trials, best first (stage 6). The best trial is within the tolerance by its
/// fine outlines (`fine_max`); `slack` allows for the chords of the coarse points of the second.
/// A sure letter is within the tolerance, has the traced shape, and stands out from the next
/// character by its mean distance (the 15 % margin) and by its shape: a wide tolerance lets a
/// neighbouring family's glyphs through, but only the right letter (in any weight) stands out.
fn judge(
    candidate: usize,
    fine: &[Trial],
    (tolerance, slack, fine_max): (f64, f64, f64),
    shapes: Shapes,
) -> Letter {
    let best = fine[0];
    let second = fine.get(1).copied();
    let within = fine_max <= tolerance;
    let margin = shapes.apart
        && second.is_none_or(|s| s.distance.mean - best.distance.mean >= MARGIN * s.distance.mean);
    // Coinciding letters (l and I) are decided by the neighbours, so both must have the shape.
    let coincide = within
        && shapes.best
        && shapes.second
        && !margin
        && second.is_some_and(|s| s.distance.max <= tolerance + slack);
    Letter {
        candidate,
        best,
        second,
        sure: within && shapes.best && margin,
        coincide,
        stays_path: fine_max > NO_LETTER * tolerance,
    }
}

/// The character class the neighbours decide coinciding letters by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Lower,
    Upper,
    Digit,
    Other,
}

fn class(c: char) -> Class {
    if c.is_ascii_digit() {
        Class::Digit
    } else if c.is_lowercase() {
        Class::Lower
    } else if c.is_uppercase() {
        Class::Upper
    } else {
        Class::Other
    }
}

/// Resolves coinciding letters (l/I/|, O/0) by the class most sure letters of the line have:
/// the one of the two in that class is taken and sure; otherwise it stays guessed.
pub(crate) fn resolve_coinciding(fit: &mut FaceFit, glyphs: &[ReferenceGlyph]) {
    let mut counts = [0_usize; 3];
    for letter in fit.letters.iter().filter(|l| l.sure) {
        match class(glyphs[letter.best.glyph].c) {
            Class::Lower => counts[0] += 1,
            Class::Upper => counts[1] += 1,
            Class::Digit => counts[2] += 1,
            Class::Other => {}
        }
    }
    let most = counts.iter().copied().max().unwrap_or(0);
    if most == 0 || counts.iter().filter(|&&c| c == most).count() > 1 {
        return;
    }
    let wanted = [Class::Lower, Class::Upper, Class::Digit]
        [counts.iter().position(|&c| c == most).unwrap_or(0)];
    for letter in fit.letters.iter_mut().filter(|l| l.coincide && !l.sure) {
        let Some(second) = letter.second else {
            continue;
        };
        let first_fits = class(glyphs[letter.best.glyph].c) == wanted;
        let second_fits = class(glyphs[second.glyph].c) == wanted;
        if first_fits && !second_fits {
            letter.sure = true;
        } else if second_fits && !first_fits {
            letter.second = Some(letter.best);
            letter.best = second;
            letter.sure = true;
        }
    }
}

/// The unit direction of the line through the letters' origins (the sure ones, or every letter
/// when fewer than three are sure), in line space, when at least three of them span two line
/// heights; `None` otherwise.
pub(crate) fn origin_direction(fit: &FaceFit, height: f64) -> Option<Vec2> {
    let sure: Vec<Point> = fit
        .letters
        .iter()
        .filter(|l| l.sure)
        .map(|l| l.best.origin)
        .collect();
    let origins = if sure.len() >= 3 {
        sure
    } else {
        fit.letters
            .iter()
            .filter(|l| !l.stays_path)
            .map(|l| l.best.origin)
            .collect()
    };
    if origins.len() < 3 {
        return None;
    }
    let (lo, hi) = origins
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.x), hi.max(p.x))
        });
    if hi - lo < 2.0 * height {
        return None;
    }
    // A letter moved off the baseline does not steer the direction.
    super::lines::median_slope(&origins, 0.5 * height)
}
