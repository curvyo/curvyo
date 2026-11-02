//! The session side of text recognition in Trace (`specs/0091-trace-text-recognition` `adrs.md`
//! decisions 1, 4 and 5): the OCR lines kept per picture for the session, the request of the font
//! fit the trace worker runs (`trace_fit_glue.rs`), and its result kept for the preview and
//! Create paths.

use curvyo_document_core::{AssetHash, FontKind, FontRef, NodeId};
use curvyo_text_core::RecognisedRun;
use curvyo_ui_core::{OcrLine, TraceTextWire, TraceWire};
use kurbo::BezPath;

use super::Session;
use super::trace_text_write::TracedText;

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
}

/// What the fit made of one picture's lines.
#[derive(Debug, Clone)]
pub(crate) struct FittedImage {
    pub(crate) image: NodeId,
    pub(crate) texts: Vec<FittedText>,
    /// The lines' outlines no text replaces: they join the traced paths.
    pub(crate) leftovers: Vec<BezPath>,
}

/// The OCR lines of the pictures read in this session and the fit of the kept results.
#[derive(Debug, Default)]
pub(crate) struct TraceTextState {
    /// Per picture (its asset), the lines in the pixels of the picture read and that picture's
    /// size; never in the document.
    ocr: Vec<(AssetHash, Vec<OcrLine>, (u32, u32))>,
    /// The lines of the last fit request: their picture and line, in request order.
    requested: Vec<(NodeId, TraceTextWire)>,
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
    dropped_lines: Vec<(NodeId, TraceTextWire)>,
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
    pub(super) fn keep_trace_fit(&mut self, image: NodeId, read: &TraceWire) {
        let state = &self.trace_text;
        let kept: Vec<&TraceTextWire> = state
            .requested
            .iter()
            .filter(|(id, line)| {
                *id == image
                    && !state
                        .dropped_lines
                        .iter()
                        .any(|(d, l)| *d == image && l == line)
            })
            .map(|(_, line)| line)
            .collect();
        let matches = state.fitted.is_some()
            && kept.len() == read.texts.lines.len()
            && kept.iter().zip(&read.texts.lines).all(|(a, b)| *a == b);
        if !matches {
            self.drop_trace_fit();
            return;
        }
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
            fitted.leftovers.retain(|path| !gone.contains(path));
        }
        self.show_fitted_preview();
    }

    /// The font fit of the kept results' text lines for the trace worker (criterion 4, decision
    /// 4) with `tolerance` mm and the maker's `favourites`: the JSON request of
    /// `trace_fit_glue`, the faces of the tiers by their host file; `None` when no result has a
    /// line, so Create paths can run at once.
    pub fn trace_fit_request(&mut self, tolerance: f64, favourites: &[String]) -> Option<String> {
        let (_, images) = self.trace_images();
        let lines: Vec<(NodeId, TraceTextWire)> = images
            .iter()
            .filter_map(|image| self.trace.results.iter().find(|(id, _)| *id == image.id))
            .flat_map(|(id, wire): &(NodeId, TraceWire)| {
                wire.texts.lines.iter().map(|l| (*id, l.clone()))
            })
            .collect();
        self.fit_request_of(lines, tolerance, favourites)
    }

    /// As [`Self::trace_fit_request`] for the lines of `regions` (the text pass alone of selected
    /// image `image`, `trace_text_regions_with`), before its main trace (criterion 25): the fit
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
        lines: Vec<(NodeId, TraceTextWire)>,
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
        let faces: Vec<serde_json::Value> = used
            .iter()
            .map(|&i| serde_json::json!([listed[i].host_id, listed[i].index]))
            .collect();
        let request = serde_json::json!({
            "tolerance": tolerance,
            "faces": faces,
            "tiers": tiers_json,
            "strokes": strokes,
            "lines": lines.iter().map(|(_, l)| serde_json::json!({
                "text": l.text,
                "direction": [l.direction.x, l.direction.y],
                "plausible": l.plausible,
                "pixel": l.pixel,
                "outlines": l.outlines.iter().map(crate::trace_fit_glue::path_svg).collect::<Vec<_>>(),
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
        let mut dropped_lines: Vec<(NodeId, TraceTextWire)> = Vec::new();
        for ((texts, leftovers), (image, line)) in lines.into_iter().zip(&self.trace_text.requested)
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
                    })
                })
                .collect();
            let leftovers: Vec<BezPath> = leftovers
                .iter()
                .filter_map(|&i| line.outlines.get(i).cloned())
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
        paths.extend(fitted.leftovers.iter().cloned());
        for text in &fitted.texts {
            paths.extend(text.glyphs.iter().cloned());
        }
        let read = read.clone();
        let stale = self.trace.preview.as_ref().is_some_and(|p| p.stale);
        self.show_trace_preview(image, read, &paths, stale);
    }
}
