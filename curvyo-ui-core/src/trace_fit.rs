//! Fits Trace's read lines to the faces of the tiers (`specs/0091-trace-text-recognition`
//! `adrs.md` decisions 4 and 5): each line is matched tier by tier with its OCR string as a hint
//! and a tolerance of its own picture's pixels (criterion 26), and a line no face fits is set in
//! the nearest face (Frage 114 B); a text with no sure letter is kept only when its OCR line
//! passes the garbage filter, otherwise its line stays traced shapes (Frage 115 B / 117 B).
//!
//! Pure and stepped: the trace worker runs it with the faces the page hands in (so the main thread
//! never matches outlines), one line and tier per step, asking for the faces a tier needs.

use std::sync::Arc;

use curvyo_text_core::{Face, LayoutOptions, Mm, RecognisedRun, layout};
use kurbo::{BezPath, Vec2};

use crate::text_recognise_job::{FaceState, JobStep, NearestFound, RecogniseJob, Tiers};

/// One read line to fit: its string, its direction and its own outlines (document mm).
#[derive(Debug, Clone, PartialEq)]
pub struct FitLine {
    /// The OCR's string, the hint.
    pub text: String,
    /// The line's direction.
    pub direction: Vec2,
    /// The line's own outlines, one closed subpath each.
    pub outlines: Vec<BezPath>,
    /// The OCR line passes the garbage filter: it becomes text even without a sure letter.
    pub plausible: bool,
    /// One picture pixel along the line, mm; 0 when unknown (the fit's own tolerance is used).
    pub pixel: f64,
}

/// A sure letter may lie this many picture pixels from its glyph at least (criterion 26,
/// Proposal): a traced edge is off by up to half a pixel, a hinted glyph by about another.
pub const TOLERANCE_PX: f64 = 1.5;
/// ... and this share of the line's ink height across the line, for large text (Proposal: the
/// 0.3 mm of `0088` on 10 mm text).
pub const TOLERANCE_SHARE: f64 = 0.03;

/// The tolerance of `line`, mm: [`TOLERANCE_PX`] pixels or [`TOLERANCE_SHARE`] of its ink height,
/// the larger; `fallback` when its pixel size is unknown.
#[must_use]
pub fn line_tolerance(line: &FitLine, fallback: Mm) -> Mm {
    let length = line.direction.hypot();
    if line.pixel <= 0.0 || length <= 0.0 {
        return fallback;
    }
    let n = Vec2::new(-line.direction.y, line.direction.x) / length;
    let (mut low, mut high) = (f64::MAX, f64::MIN);
    for path in &line.outlines {
        for el in path.elements() {
            if let Some(p) = el.end_point() {
                let b = p.to_vec2().dot(n);
                low = low.min(b);
                high = high.max(b);
            }
        }
    }
    let ink = if high > low { high - low } else { 0.0 };
    Mm((TOLERANCE_PX * line.pixel).max(TOLERANCE_SHARE * ink))
}

/// One text the fit sets.
#[derive(Debug, Clone, PartialEq)]
pub struct FitText {
    /// The text's string, size, placement, spacing and guessed letters.
    pub run: RecognisedRun,
    /// Its face, an index into the listing the tiers index.
    pub face: usize,
    /// Set in the nearest face, every letter guessed.
    pub nearest: bool,
    /// Its glyphs as the document draws them, document mm (the preview, UX notes §2).
    pub glyphs: Vec<BezPath>,
    /// Set in a single-line face: drawn with a stroke this wide, not filled (criterion 27).
    pub stroke: Option<Mm>,
}

/// What the fit made of one line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FitResult {
    /// The texts (usually one).
    pub texts: Vec<FitText>,
    /// The indices of the line's outlines no text replaces.
    pub leftovers: Vec<usize>,
}

/// A fit of several lines over the tiers.
#[derive(Debug, Clone)]
pub struct TraceFit {
    job: RecogniseJob,
    lines: Vec<FitLine>,
    /// The single-line faces (listing indices) a thin-stroked line is matched against by its
    /// centre line (criterion 27).
    strokes: Vec<usize>,
    /// The single-line faces were asked for once.
    strokes_asked: bool,
}

/// A single-line face is taken for a line when the mean distance between the line's centre line
/// and the face's strokes is at most this share of the ink height (criterion 27, Proposal,
/// calibrated on the customer's screenshot: its own faces 0.05 to 0.10, others 0.10 and up) ...
pub const STROKE_SCORE: f64 = 0.12;
/// ... and at least one in this many of its letters is sure (Proposal: own faces 48 % to 100 %,
/// others 10 % at most).
pub const STROKE_SURE_EVERY: usize = 3;
/// ... and at least this many (as `0088` asks of a run): one stroke matches many digits.
const MIN_STROKE_SURE: usize = 2;

/// The glyphs of `run` in `face`, document mm.
fn glyphs(face: &Face, run: &RecognisedRun) -> Vec<BezPath> {
    let options = LayoutOptions {
        letter_spacing: run.letter_spacing,
        word_spacing: run.word_spacing,
        ..LayoutOptions::default()
    };
    layout(face, &run.text, run.size, &options)
        .map(|laid| {
            laid.glyph_paths(face)
                .into_iter()
                .map(|path| run.placement * path)
                .collect()
        })
        .unwrap_or_default()
}

impl TraceFit {
    /// A fit of `lines` over `tiers`, each within its own tolerance ([`line_tolerance`]);
    /// `tolerance` stands for a line of unknown pixel size.
    #[must_use]
    pub fn new(lines: Vec<FitLine>, tolerance: Mm, tiers: Tiers, strokes: Vec<usize>) -> Self {
        let hinted: Vec<(Vec<BezPath>, String, Mm)> = lines
            .iter()
            .map(|l| {
                (
                    l.outlines.clone(),
                    l.text.clone(),
                    line_tolerance(l, tolerance),
                )
            })
            .collect();
        Self {
            job: RecogniseJob::with_hints(&hinted, tolerance, tiers),
            lines,
            strokes,
            strokes_asked: false,
        }
    }

    /// One step: asks for faces (the single-line faces first), or searches one line in one
    /// tier.
    pub fn step(&mut self, face: &dyn Fn(usize) -> FaceState) -> JobStep {
        if !self.strokes_asked && !self.lines.is_empty() {
            self.strokes_asked = true;
            let missing: Vec<usize> = self
                .strokes
                .iter()
                .copied()
                .filter(|&i| matches!(face(i), FaceState::Loading))
                .collect();
            if !missing.is_empty() {
                return JobStep::NeedFaces(missing);
            }
        }
        self.job.step(face)
    }

    /// Line `source` set in the single-line face whose strokes come nearest its centre line,
    /// when that face comes within [`STROKE_SCORE`] and has a third of its letters, and two at
    /// least, sure.
    fn stroke_text(
        &self,
        source: usize,
        face: &dyn Fn(usize) -> Option<Arc<Face>>,
    ) -> Option<FitText> {
        let line = &self.lines[source];
        let recogniser = &self.job.recognisers()[source];
        let (index, found, fit) = self
            .strokes
            .iter()
            .filter_map(|&index| {
                let found = face(index)?;
                let fit = recogniser.fit_strokes(&found, &line.text, line.direction)?;
                Some((index, found, fit))
            })
            .min_by(|a, b| a.2.score.total_cmp(&b.2.score).then(a.0.cmp(&b.0)))?;
        let taken = fit.score <= STROKE_SCORE
            && fit.run.sure >= MIN_STROKE_SURE
            && STROKE_SURE_EVERY * fit.run.sure >= fit.run.letters;
        taken.then(|| FitText {
            glyphs: glyphs(&found, &fit.run),
            nearest: fit.run.sure == 0,
            run: fit.run,
            face: index,
            stroke: Some(fit.width),
        })
    }

    /// Line `source` in the face that came nearest on it (else `overall`), every letter guessed
    /// (Frage 114 B).
    fn nearest_text(
        &self,
        source: usize,
        face: &dyn Fn(usize) -> Option<Arc<Face>>,
        overall: Option<usize>,
    ) -> Option<FitText> {
        let job = &self.job;
        let line = &self.lines[source];
        let index = job
            .line_nearest()
            .iter()
            .filter(|(r, _, _)| *r == source)
            .filter_map(|(_, _, near)| *near)
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            .map(|(index, _)| index)
            .or(overall)?;
        let found = face(index)?;
        let run = job.recognisers()[source].fit_text(&found, &line.text, line.direction)?;
        Some(FitText {
            glyphs: glyphs(&found, &run),
            run,
            face: index,
            nearest: true,
            stroke: None,
        })
    }

    /// The texts and leftovers of every line once the job is finished: its string in the
    /// single-line face its centre line matches, else its runs, else its string in the nearest
    /// face (of the line, else of the whole fit); a text with no sure letter is
    /// dropped unless its line passes the garbage filter, so such a line stays traced shapes.
    /// `face` gives a listed face that is loaded.
    #[must_use]
    pub fn finish(&self, face: &dyn Fn(usize) -> Option<Arc<Face>>) -> Vec<FitResult> {
        self.finish_with(face, self.job.nearest().map(|(index, _)| index))
    }

    /// The face that came nearest over this fit, to weigh against the fits of other lines
    /// (`specs/0091` round 2: a picture's lines fitted in several workers); `lines` gives each
    /// of this fit's lines its index among all of them (ascending).
    #[must_use]
    pub fn nearest_found(&self, lines: &[usize]) -> Option<NearestFound> {
        self.job.nearest_found(lines)
    }

    /// As [`Self::finish`], a line with no nearest face of its own set in `overall` (the face
    /// that came nearest over every worker's lines).
    #[must_use]
    pub fn finish_with(
        &self,
        face: &dyn Fn(usize) -> Option<Arc<Face>>,
        overall: Option<usize>,
    ) -> Vec<FitResult> {
        let mut out = Vec::with_capacity(self.lines.len());
        for (source, line) in self.lines.iter().enumerate() {
            // Criterion 27: thin strokes near a single-line face are set in it.
            let mut texts: Vec<FitText> = self.stroke_text(source, face).into_iter().collect();
            if texts.is_empty() {
                texts = self
                    .job
                    .accepted()
                    .iter()
                    .filter(|a| a.source == source)
                    .filter_map(|a| {
                        let found = face(a.face)?;
                        Some(FitText {
                            glyphs: glyphs(&found, &a.run),
                            run: a.run.clone(),
                            face: a.face,
                            nearest: false,
                            stroke: None,
                        })
                    })
                    .collect();
            }
            if texts.is_empty() {
                texts.extend(self.nearest_text(source, face, overall));
            }
            // Frage 115 B / 117 B: a wrong font is better than paths, unless the line is garbage.
            texts.retain(|t| line.plausible || t.run.guessed.len() < t.run.letters);
            let used: Vec<usize> = texts
                .iter()
                .flat_map(|t| t.run.outlines.iter().copied())
                .collect();
            let leftovers = (0..line.outlines.len())
                .filter(|i| !used.contains(i))
                .collect();
            out.push(FitResult { texts, leftovers });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use curvyo_text_core::FaceStore;
    use kurbo::{Circle, PathEl, Shape};

    use super::*;

    fn sans() -> Arc<Face> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf");
        let mut store = FaceStore::new();
        let key = store.add(std::fs::read(path).unwrap(), 0).unwrap();
        store.get(&key).unwrap().clone()
    }

    /// Every closed subpath of `paths` on its own.
    fn subpaths(paths: &[BezPath]) -> Vec<BezPath> {
        let mut out = Vec::new();
        for path in paths {
            let mut current = BezPath::new();
            for el in path.elements() {
                if matches!(el, PathEl::MoveTo(_)) && !current.elements().is_empty() {
                    out.push(std::mem::take(&mut current));
                }
                current.push(*el);
            }
            if !current.elements().is_empty() {
                out.push(current);
            }
        }
        out
    }

    /// The fit of one line `text` with `outlines` against Source Sans 3 Regular; `plausible` as
    /// the garbage filter judged the OCR line.
    fn fit_one(text: &str, outlines: Vec<BezPath>, plausible: bool) -> FitResult {
        let face = sans();
        let mut tiers = Tiers::default();
        tiers.faces[0] = vec![0];
        let line = FitLine {
            text: text.to_owned(),
            direction: Vec2::new(1.0, 0.0),
            outlines,
            plausible,
            pixel: 0.0,
        };
        let mut fit = TraceFit::new(vec![line], Mm(0.3), tiers, Vec::new());
        let ready = |_: usize| FaceState::Ready(face.clone());
        for _ in 0..64 {
            if fit.step(&ready) == JobStep::Finished {
                break;
            }
        }
        fit.finish(&|_| Some(face.clone())).remove(0)
    }

    /// Frage 115 B / 117 B: a lone "H" fails the garbage filter (one character) and is no run
    /// (`0088` needs two sure letters), so the line stays traced shapes.
    #[test]
    fn a_lone_letter_marked_guessed_stays_shapes() {
        let face = sans();
        let laid = layout(&face, "H", Mm(10.0), &LayoutOptions::default()).unwrap();
        let outlines = subpaths(&laid.glyph_paths(&face));
        let count = outlines.len();
        let result = fit_one("H", outlines, false);
        assert!(result.texts.is_empty(), "{result:?}");
        assert_eq!(result.leftovers, (0..count).collect::<Vec<_>>());
    }

    /// Frage 115 B / 117 B: a ring the OCR reads as "O" has no sure letter and fails the
    /// garbage filter, so it stays shapes.
    #[test]
    fn a_ring_read_as_o_stays_shapes() {
        let ring = vec![
            Circle::new((10.0, 10.0), 6.0).to_path(0.01),
            Circle::new((10.0, 10.0), 2.0).to_path(0.01),
        ];
        let result = fit_one("O", ring, false);
        assert!(result.texts.is_empty(), "{result:?}");
        assert_eq!(result.leftovers, vec![0, 1]);
    }

    /// Frage 115 B / 117 B: a line that passes the garbage filter becomes text in the nearest
    /// face even with no sure letter, every letter guessed, its outlines replaced.
    #[test]
    fn a_plausible_line_without_a_sure_letter_is_set_all_guessed() {
        let blobs = vec![
            Circle::new((10.0, 10.0), 4.0).to_path(0.01),
            Circle::new((22.0, 10.0), 4.0).to_path(0.01),
        ];
        let result = fit_one("Ab", blobs, true);
        assert_eq!(result.texts.len(), 1, "{result:?}");
        let text = &result.texts[0];
        assert!(text.nearest);
        assert_eq!(
            (text.run.text.as_str(), text.run.guessed.clone()),
            ("Ab", vec![0, 1])
        );
        assert_eq!(result.leftovers, Vec::<usize>::new());
    }

    /// Criterion 26: the tolerance is 1.5 picture pixels, or 3 % of the ink height on large text.
    #[test]
    fn the_tolerance_follows_the_picture() {
        let line = |height: f64, pixel: f64| FitLine {
            text: "x".to_owned(),
            direction: Vec2::new(2.0, 0.0),
            outlines: vec![kurbo::Rect::new(0.0, 0.0, 5.0, height).to_path(0.01)],
            plausible: true,
            pixel,
        };
        // 13 px UI text at 0.26 mm per pixel: 1.5 px.
        assert!((line_tolerance(&line(2.4, 0.26), Mm(0.3)).get() - 0.39).abs() < 1e-9);
        // 40 mm letters: 3 % of their height.
        assert!((line_tolerance(&line(40.0, 0.1), Mm(0.3)).get() - 1.2).abs() < 1e-9);
        // Unknown pixel size: the fit's own tolerance.
        assert_eq!(line_tolerance(&line(40.0, 0.0), Mm(0.3)), Mm(0.3));
    }
}
