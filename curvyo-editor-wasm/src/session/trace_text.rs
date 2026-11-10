//! The session side of text recognition in Trace (`specs/0091-trace-text-recognition` `adrs.md`
//! decisions 1, 4 and 5): the OCR lines kept per picture for the session, the request of the font
//! fit the trace worker runs (`trace_fit_glue.rs`), and its result kept for the preview and
//! Create paths.

use curvyo_document_core::{AssetHash, FontKind, FontRef, NodeId};
use curvyo_text_core::RecognisedRun;
use kurbo::BezPath;
use vectrace::Output;
use vectrace::Rgb8 as TraceRgb;
use vectrace::ocr::OcrLine;
use vectrace::wire::Text;

use super::Session;
use super::trace_text_write::TracedText;

/// The included family a line without any sure letter is set in (criterion 29, customer
/// 2026-11-02: "a plain included sans").
const FALLBACK_FAMILY: &str = "Source Sans 3";

/// The most single-line faces a fit matches thin strokes against (criterion 27, Proposal).
const MAX_STROKE_FACES: usize = 16;

/// One text Create paths writes for a traced picture.
#[derive(Debug, Clone)]
pub(crate) struct FittedText {
    pub(crate) run: RecognisedRun,
    pub(crate) font: FontRef,
    /// Its glyphs, document mm, for the preview.
    pub(crate) glyphs: Vec<BezPath>,
    /// Set in the nearest face, every letter guessed (Frage 114 B).
    pub(crate) nearest: bool,
    /// Set in a single-line face: drawn with a stroke this wide, mm (criterion 27).
    pub(crate) stroke: Option<f64>,
    /// Its line in the fit request.
    pub(crate) line: usize,
    /// Its fill in a multicolour result (slice 2, decision 6); `None`: black.
    pub(crate) fill: Option<TraceRgb>,
    /// The maker excluded its fill: left out, as the shapes of that colour.
    pub(crate) excluded: bool,
}

/// An outline of a line that no text replaces: it joins the traced paths.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Leftover {
    pub(crate) path: BezPath,
    /// Its line in the fit request.
    pub(crate) line: usize,
    /// The line's fill in a multicolour result; `None`: black.
    pub(crate) fill: Option<TraceRgb>,
    /// The maker excluded its fill: left out, as the shapes of that colour.
    pub(crate) excluded: bool,
}

/// What the fit made of one picture's lines.
#[derive(Debug, Clone)]
pub(crate) struct FittedImage {
    pub(crate) image: NodeId,
    pub(crate) texts: Vec<FittedText>,
    /// The lines' outlines no text replaces: they join the traced paths.
    pub(crate) leftovers: Vec<Leftover>,
}

/// The outlines of `image`'s text lines that join its traced paths, each with its fill (black
/// without one): the fit's leftovers, or every line's outlines without a fit of `image`; none of
/// a line whose fill the maker excluded.
pub(super) fn leftovers_of(
    fitted: &[FittedImage],
    image: NodeId,
    wire: &Output,
) -> Vec<(BezPath, TraceRgb)> {
    let black = TraceRgb::new(0, 0, 0);
    match fitted.iter().find(|f| f.image == image) {
        Some(found) => found
            .leftovers
            .iter()
            .filter(|l| !l.excluded)
            .map(|l| (l.path.clone(), l.fill.unwrap_or(black)))
            .collect(),
        None => wire
            .texts
            .lines
            .iter()
            .filter(|l| !l.excluded)
            .flat_map(|l| {
                l.outlines
                    .iter()
                    .map(move |p| (p.clone(), l.fill.unwrap_or(black)))
            })
            .collect(),
    }
}

/// The OCR lines of the pictures read in this session and the fit of the kept results.
#[derive(Debug, Default)]
pub(crate) struct TraceTextState {
    /// Per picture (its asset), the lines in the pixels of the picture read and that picture's
    /// size; never in the document.
    ocr: Vec<(AssetHash, Vec<OcrLine>, (u32, u32))>,
    /// The lines of the last fit request: their picture and line, in request order.
    requested: Vec<(NodeId, Text)>,
    /// The finished fit, for Create paths.
    pub(crate) fitted: Option<Vec<FittedImage>>,
    /// The texts the last Create paths wrote.
    pub(super) created: Vec<TracedText>,
    /// Per picture (its asset), the strings of the accepted lines a fit set no text for (lead
    /// decision 1, 2026-11-01): they are traced as shapes, so the next trace no longer accepts
    /// them.
    dropped: Vec<(AssetHash, String)>,
    /// The lines the last fit set no text for.
    last_dropped: usize,
    /// Those lines with their picture: once a trace of that picture traced them as shapes, their
    /// outlines leave the fit's leftovers (criterion 25).
    dropped_lines: Vec<(NodeId, Text)>,
}

impl Session {
    fn asset_of(&self, image: NodeId) -> Option<AssetHash> {
        let (_, images) = self.trace_images();
        images.into_iter().find(|i| i.id == image).map(|i| i.asset)
    }

    /// Keeps the OCR `lines` of selected image `image` for the session (decision 1), read from
    /// a picture of `size` pixels.
    pub fn set_trace_ocr_lines(
        &mut self,
        image: NodeId,
        lines: Vec<OcrLine>,
        size: (u32, u32),
    ) -> bool {
        let Some(asset) = self.asset_of(image) else {
            return false;
        };
        self.trace_text.ocr.retain(|(a, _, _)| *a != asset);
        self.trace_text.dropped.retain(|(a, _)| *a != asset);
        self.trace_text.ocr.push((asset, lines, size));
        true
    }

    /// The OCR lines a trace of selected image `image` takes and the size of the picture read:
    /// the kept lines, those a fit set no text for no longer accepted (lead decision 1), so the
    /// trace leaves them in the paths.
    #[must_use]
    pub fn trace_lines_to_trace(&self, image: NodeId) -> Option<(Vec<OcrLine>, (u32, u32))> {
        let asset = self.asset_of(image)?;
        let (lines, size) = self.trace_ocr_lines(image)?;
        let dropped = |text: &str| {
            self.trace_text
                .dropped
                .iter()
                .any(|(a, t)| *a == asset && t == text)
        };
        let lines = lines
            .iter()
            .map(|line| {
                if dropped(&line.text) {
                    OcrLine {
                        confidence: Vec::new(),
                        ..line.clone()
                    }
                } else {
                    line.clone()
                }
            })
            .collect();
        Some((lines, size))
    }

    /// How many lines the last fit set no text for: then the host traces again with
    /// [`Self::trace_lines_to_trace`] so they are traced as the rest of the picture.
    #[must_use]
    pub fn trace_fit_dropped(&self) -> usize {
        self.trace_text.last_dropped
    }

    /// The OCR lines kept for selected image `image` and the size of the picture read, if it was
    /// read in this session.
    #[must_use]
    pub fn trace_ocr_lines(&self, image: NodeId) -> Option<(&[OcrLine], (u32, u32))> {
        let asset = self.asset_of(image)?;
        self.trace_text
            .ocr
            .iter()
            .find(|(a, _, _)| *a == asset)
            .map(|(_, lines, size)| (lines.as_slice(), *size))
    }

    /// Drops a requested or finished fit (a result changed).
    pub(super) fn drop_trace_fit(&mut self) {
        self.trace_text.requested.clear();
        self.trace_text.fitted = None;
        self.trace_text.last_dropped = 0;
        self.trace_text.dropped_lines.clear();
    }

    /// Takes the new result `read` of `image` into the finished fit (criterion 25: the trace the
    /// fit decided runs once): when its text lines are the fit's lines of `image` it set text
    /// for, the fit stays, the lines it set none for (traced as shapes now) leave its leftovers,
    /// and the preview shows the texts; otherwise the fit is dropped.
    pub(super) fn keep_trace_fit(&mut self, image: NodeId, read: &Output) {
        let state = &self.trace_text;
        // The requested lines of `image` it set text for, by their place in the request.
        let kept: Vec<(usize, &Text)> = state
            .requested
            .iter()
            .enumerate()
            .filter(|(_, (id, line))| {
                *id == image
                    && !state
                        .dropped_lines
                        .iter()
                        .any(|(d, l)| *d == image && l == line)
            })
            .map(|(k, (_, line))| (k, line))
            .collect();
        // Slice 2: the text pass alone has no fill; the result's lines bring it.
        let matches = state.fitted.is_some()
            && kept.len() == read.texts.lines.len()
            && kept
                .iter()
                .zip(&read.texts.lines)
                .all(|((_, a), b)| a.same_line(b));
        if !matches {
            self.drop_trace_fit();
            return;
        }
        let fills: Vec<(usize, Option<TraceRgb>, bool)> = kept
            .iter()
            .zip(&read.texts.lines)
            .map(|((k, _), line)| (*k, line.fill, line.excluded))
            .collect();
        let gone: Vec<BezPath> = state
            .dropped_lines
            .iter()
            .filter(|(id, _)| *id == image)
            .flat_map(|(_, line)| line.outlines.iter().cloned())
            .collect();
        if let Some(fitted) = self
            .trace_text
            .fitted
            .as_mut()
            .and_then(|f| f.iter_mut().find(|f| f.image == image))
        {
            fitted.leftovers.retain(|l| !gone.contains(&l.path));
            let fill_of = |line: usize| {
                fills
                    .iter()
                    .find(|(k, _, _)| *k == line)
                    .map_or((None, false), |f| (f.1, f.2))
            };
            for text in &mut fitted.texts {
                (text.fill, text.excluded) = fill_of(text.line);
            }
            for leftover in &mut fitted.leftovers {
                (leftover.fill, leftover.excluded) = fill_of(leftover.line);
            }
        }
        self.show_fitted_preview();
    }

    /// The font fit of the kept results' text lines for the trace worker (criterion 4, decision
    /// 4) with `tolerance` mm and the maker's `favourites`: the JSON request of
    /// `trace_fit_glue`, the faces of the tiers by their host file; `None` when no result has a
    /// line, so Create paths can run at once.
    pub fn trace_fit_request(&mut self, tolerance: f64, favourites: &[String]) -> Option<String> {
        let (_, images) = self.trace_images();
        let lines: Vec<(NodeId, Text)> = images
            .iter()
            .filter_map(|image| self.trace.results.iter().find(|(id, _)| *id == image.id))
            .flat_map(|(id, wire): &(NodeId, Output)| {
                wire.texts.lines.iter().map(|l| (*id, l.clone()))
            })
            .collect();
        self.fit_request_of(lines, tolerance, favourites)
    }

    /// As [`Self::trace_fit_request`] for the lines of `regions` (the text pass alone of selected
    /// image `image`, `trace_text_regions_in`), before its main trace (criterion 25): the fit
    /// decides the lines, the trace then runs once with [`Self::trace_lines_to_trace`] and keeps
    /// the fit.
    pub fn trace_fit_request_for(
        &mut self,
        image: NodeId,
        regions: &[f64],
        tolerance: f64,
        favourites: &[String],
    ) -> Option<String> {
        let read = curvyo_ui_core::read_trace_wire(regions)?;
        let lines = read.texts.lines.into_iter().map(|l| (image, l)).collect();
        self.fit_request_of(lines, tolerance, favourites)
    }

    fn fit_request_of(
        &mut self,
        lines: Vec<(NodeId, Text)>,
        tolerance: f64,
        favourites: &[String],
    ) -> Option<String> {
        self.drop_trace_fit();
        if lines.is_empty() {
            return None;
        }
        let tiers = self.recognise_tiers(favourites);
        // Only the faces the tiers search cross, renumbered in tier order.
        let listed = self.fonts.faces();
        let mut used: Vec<usize> = Vec::new();
        let tiers_json: Vec<Vec<usize>> = tiers
            .faces
            .iter()
            .map(|tier| {
                tier.iter()
                    .map(|&index| {
                        if let Some(at) = used.iter().position(|&u| u == index) {
                            at
                        } else {
                            used.push(index);
                            used.len() - 1
                        }
                    })
                    .collect()
            })
            .collect();
        // Criterion 27: the single-line faces a thin-stroked line is matched against.
        let strokes: Vec<usize> = listed
            .iter()
            .enumerate()
            .filter(|(_, face)| face.kind == FontKind::SingleLine)
            .take(MAX_STROKE_FACES)
            .map(|(index, _)| {
                used.iter().position(|&u| u == index).unwrap_or_else(|| {
                    used.push(index);
                    used.len() - 1
                })
            })
            .collect();
        // Round 3 (criterion 29): the included sans a line without a sure letter is set in.
        let fallback: Vec<serde_json::Value> = listed
            .iter()
            .enumerate()
            .filter(|(_, face)| {
                face.block == curvyo_ui_core::FontBlock::Included
                    && face.family == FALLBACK_FAMILY
                    && face.kind == FontKind::Outline
            })
            .map(|(index, face)| {
                let at = used.iter().position(|&u| u == index).unwrap_or_else(|| {
                    used.push(index);
                    used.len() - 1
                });
                let italic = face.style.contains("Italic");
                let regular = face.style == "Regular" || face.style == "Italic";
                serde_json::json!([at, italic, regular])
            })
            .collect();
        let faces: Vec<serde_json::Value> = used
            .iter()
            .map(|&i| serde_json::json!([listed[i].host_id, listed[i].index]))
            .collect();
        let request = serde_json::json!({
            "tolerance": tolerance,
            "faces": faces,
            "tiers": tiers_json,
            "strokes": strokes,
            "fallback": fallback,
            "lines": lines.iter().map(|(_, l)| serde_json::json!({
                "text": l.text,
                "direction": [l.direction.x, l.direction.y],
                "plausible": l.plausible,
                "pixel": l.pixel,
                "positions": l.positions,
                "outlines": l.outlines.iter().map(kurbo::BezPath::to_svg).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "listed": used,
        });
        self.trace_text.requested = lines;
        Some(request.to_string())
    }

    /// Takes the worker's finished fit `result` of the last request: the texts with their fonts
    /// and glyphs, the leftovers; the preview then shows the texts (UX notes §2). `false` when it
    /// is no result for that request.
    pub fn set_trace_fit(&mut self, request: &str, result: &str) -> bool {
        let Some(lines) = crate::trace_fit_glue::read_fit_result(result) else {
            return false;
        };
        let listed: Vec<usize> = serde_json::from_str::<serde_json::Value>(request)
            .ok()
            .and_then(|v| {
                v.get("listed")?
                    .as_array()?
                    .iter()
                    .map(|n| n.as_u64().and_then(|n| usize::try_from(n).ok()))
                    .collect()
            })
            .unwrap_or_default();
        if lines.len() != self.trace_text.requested.len() {
            return false;
        }
        let faces = self.fonts.faces();
        let mut out: Vec<FittedImage> = Vec::new();
        let mut dropped: Vec<(AssetHash, String)> = Vec::new();
        let mut dropped_lines: Vec<(NodeId, Text)> = Vec::new();
        for (k, ((texts, leftovers), (image, line))) in lines
            .into_iter()
            .zip(&self.trace_text.requested)
            .enumerate()
        {
            if texts.is_empty() {
                dropped_lines.push((*image, line.clone()));
                if let Some(asset) = self.asset_of(*image) {
                    dropped.push((asset, line.text.clone()));
                }
            }
            let fitted: Vec<FittedText> = texts
                .into_iter()
                .filter_map(|t| {
                    let face = faces.get(*listed.get(t.face)?)?;
                    Some(FittedText {
                        font: FontRef {
                            family: face.family.clone(),
                            style: face.style.clone(),
                            face_hash: t.hash,
                            face_index: face.index,
                            kind: face.kind,
                        },
                        run: t.run,
                        glyphs: t.glyphs,
                        nearest: t.nearest,
                        stroke: t.stroke.map(curvyo_text_core::Mm::get),
                        line: k,
                        fill: line.fill,
                        excluded: line.excluded,
                    })
                })
                .collect();
            let leftovers: Vec<Leftover> = leftovers
                .iter()
                .filter_map(|&i| {
                    Some(Leftover {
                        path: line.outlines.get(i)?.clone(),
                        line: k,
                        fill: line.fill,
                        excluded: line.excluded,
                    })
                })
                .collect();
            match out.iter_mut().find(|f| f.image == *image) {
                Some(found) => {
                    found.texts.extend(fitted);
                    found.leftovers.extend(leftovers);
                }
                None => out.push(FittedImage {
                    image: *image,
                    texts: fitted,
                    leftovers,
                }),
            }
        }
        self.trace_text.last_dropped = dropped.len();
        self.trace_text.dropped.extend(dropped);
        self.trace_text.dropped_lines = dropped_lines;
        self.trace_text.fitted = Some(out);
        self.show_fitted_preview();
        true
    }

    /// Whether a finished fit is kept for the current results, so Create paths need not fit again.
    #[must_use]
    pub fn trace_text_fitted(&self) -> bool {
        let Some(fitted) = &self.trace_text.fitted else {
            return false;
        };
        // Every kept result with a text line is in the fit (a fit decided per picture covers
        // only its own).
        self.trace
            .results
            .iter()
            .filter(|(_, read)| !read.texts.lines.is_empty())
            .all(|(id, _)| fitted.iter().any(|f| f.image == *id))
    }

    /// The preview of the fitted picture with its texts in their faces (UX notes §2): the traced
    /// paths, the lines' leftovers and each text's glyphs where Create paths writes them.
    fn show_fitted_preview(&mut self) {
        let Some(image) = self.trace.preview.as_ref().map(|p| p.image) else {
            return;
        };
        let Some(fitted) = self
            .trace_text
            .fitted
            .as_ref()
            .and_then(|f| f.iter().find(|f| f.image == image))
        else {
            return;
        };
        let Some((_, read)) = self.trace.results.iter().find(|(id, _)| *id == image) else {
            return;
        };
        let mut paths: Vec<BezPath> = read.outlines.clone();
        let mut extra = Vec::new();
        for leftover in fitted.leftovers.iter().filter(|l| !l.excluded) {
            paths.push(leftover.path.clone());
            extra.push(super::trace_colours::text_layer(leftover.fill, 1));
        }
        for text in fitted.texts.iter().filter(|t| !t.excluded) {
            paths.extend(text.glyphs.iter().cloned());
            extra.push(super::trace_colours::text_layer(
                text.fill,
                text.glyphs.len(),
            ));
        }
        let read = read.clone();
        let stale = self.trace.preview.as_ref().is_some_and(|p| p.stale);
        self.show_trace_preview(image, read, &paths, &extra, stale);
    }
}
