//! Fits Trace's read lines to the faces of the tiers (`specs/0091-trace-text-recognition`
//! `adrs.md` decisions 4 and 5): each line is matched tier by tier with its OCR string as a hint,
//! and a line no face fits is set in the nearest face (Frage 114 B); a text with no sure letter
//! in the face it is set in is dropped, so its line stays traced shapes (lead decision 1,
//! 2026-11-01).
//!
//! Pure and stepped: the trace worker runs it with the faces the page hands in (so the main thread
//! never matches outlines), one line and tier per step, asking for the faces a tier needs.

use std::sync::Arc;

use curvyo_text_core::{Face, LayoutOptions, Mm, RecognisedRun, layout};
use kurbo::{BezPath, Vec2};

use crate::text_recognise_job::{FaceState, JobStep, RecogniseJob, Tiers};

/// One read line to fit: its string, its direction and its own outlines (document mm).
#[derive(Debug, Clone, PartialEq)]
pub struct FitLine {
    /// The OCR's string, the hint.
    pub text: String,
    /// The line's direction.
    pub direction: Vec2,
    /// The line's own outlines, one closed subpath each.
    pub outlines: Vec<BezPath>,
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
}

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
    /// A fit of `lines` within `tolerance` over `tiers`.
    #[must_use]
    pub fn new(lines: Vec<FitLine>, tolerance: Mm, tiers: Tiers) -> Self {
        let hinted: Vec<(Vec<BezPath>, String)> = lines
            .iter()
            .map(|l| (l.outlines.clone(), l.text.clone()))
            .collect();
        Self {
            job: RecogniseJob::with_hints(&hinted, tolerance, tiers),
            lines,
        }
    }

    /// One step: asks for faces, or searches one line in one tier.
    pub fn step(&mut self, face: &dyn Fn(usize) -> FaceState) -> JobStep {
        self.job.step(face)
    }

    /// The texts and leftovers of every line once the job is finished: its runs, or its string
    /// in the nearest face (of the line, else of the whole fit); a text with no sure letter in its
    /// face is dropped, so the line stays traced shapes. `face` gives a listed face that is
    /// loaded.
    #[must_use]
    pub fn finish(&self, face: &dyn Fn(usize) -> Option<Arc<Face>>) -> Vec<FitResult> {
        let job = &self.job;
        let nearest_lines = job.line_nearest();
        let mut out = Vec::with_capacity(self.lines.len());
        for (source, line) in self.lines.iter().enumerate() {
            let mut texts: Vec<FitText> = job
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
                    })
                })
                .collect();
            if texts.is_empty() {
                let nearest = nearest_lines
                    .iter()
                    .filter(|(r, _, _)| *r == source)
                    .filter_map(|(_, _, near)| *near)
                    .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                    .or_else(|| job.nearest());
                let set = nearest.and_then(|(index, _)| {
                    let found = face(index)?;
                    let run =
                        job.recognisers()[source].fit_text(&found, &line.text, line.direction)?;
                    Some(FitText {
                        glyphs: glyphs(&found, &run),
                        run,
                        face: index,
                        nearest: true,
                    })
                });
                texts.extend(set);
            }
            // Lead decision 1: a text needs a sure letter in the face it is set in.
            texts.retain(|t| t.run.guessed.len() < t.run.letters);
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

    /// The fit of one line `text` with `outlines` against Source Sans 3 Regular.
    fn fit_one(text: &str, outlines: Vec<BezPath>) -> FitResult {
        let face = sans();
        let mut tiers = Tiers::default();
        tiers.faces[0] = vec![0];
        let line = FitLine {
            text: text.to_owned(),
            direction: Vec2::new(1.0, 0.0),
            outlines,
        };
        let mut fit = TraceFit::new(vec![line], Mm(0.3), tiers);
        let ready = |_: usize| FaceState::Ready(face.clone());
        for _ in 0..64 {
            if fit.step(&ready) == JobStep::Finished {
                break;
            }
        }
        fit.finish(&|_| Some(face.clone())).remove(0)
    }

    /// Lead decision 1: a lone sure "H" is no run (`0088` needs two sure letters), so its nearest
    /// face text has every letter guessed and the line stays traced shapes.
    #[test]
    fn a_lone_letter_marked_guessed_stays_shapes() {
        let face = sans();
        let laid = layout(&face, "H", Mm(10.0), &LayoutOptions::default()).unwrap();
        let outlines = subpaths(&laid.glyph_paths(&face));
        let count = outlines.len();
        let result = fit_one("H", outlines);
        assert!(result.texts.is_empty(), "{result:?}");
        assert_eq!(result.leftovers, (0..count).collect::<Vec<_>>());
    }

    /// Lead decision 1: a ring the OCR reads as "O" has no sure letter, so it stays shapes.
    #[test]
    fn a_ring_read_as_o_stays_shapes() {
        let ring = vec![
            Circle::new((10.0, 10.0), 6.0).to_path(0.01),
            Circle::new((10.0, 10.0), 2.0).to_path(0.01),
        ];
        let result = fit_one("O", ring);
        assert!(result.texts.is_empty(), "{result:?}");
        assert_eq!(result.leftovers, vec![0, 1]);
    }
}
