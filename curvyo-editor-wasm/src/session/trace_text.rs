//! The session side of text recognition in Trace (`specs/0091-trace-text-recognition` `adrs.md`
//! decisions 1, 4 and 5): the OCR lines kept per picture for the session, the request of the font
//! fit the trace worker runs (`trace_fit_glue.rs`), and its result kept for the preview and
//! Create paths.

use curvyo_document_core::{AssetHash, FontKind, FontRef, NodeId};
use curvyo_text_core::RecognisedRun;
use curvyo_ui_core::{OcrLine, TraceTextWire, TraceWire};
use kurbo::BezPath;

use super::Session;

/// One text Create paths writes for a traced picture.
#[derive(Debug, Clone)]
pub(crate) struct FittedText {
    pub(crate) run: RecognisedRun,
    pub(crate) font: FontRef,
    /// Its glyphs, document mm, for the preview.
    pub(crate) glyphs: Vec<BezPath>,
    /// Set in the nearest face, every letter guessed (Frage 114 B).
    pub(crate) nearest: bool,
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
    created: Vec<TracedText>,
    /// Per picture (its asset), the strings of the accepted lines a fit set no text for (lead
    /// decision 1, 2026-11-01): they are traced as shapes, so the next trace no longer accepts
    /// them.
    dropped: Vec<(AssetHash, String)>,
    /// The lines the last fit set no text for.
    last_dropped: usize,
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
    }

    /// The font fit of the kept results' text lines for the trace worker (criterion 4, decision
    /// 4) with `tolerance` mm and the maker's `favourites`: the JSON request of
    /// `trace_fit_glue`, the faces of the tiers by their host file; `None` when no result has a
    /// line, so Create paths can run at once.
    pub fn trace_fit_request(&mut self, tolerance: f64, favourites: &[String]) -> Option<String> {
        self.drop_trace_fit();
        let (_, images) = self.trace_images();
        let lines: Vec<(NodeId, TraceTextWire)> = images
            .iter()
            .filter_map(|image| self.trace.results.iter().find(|(id, _)| *id == image.id))
            .flat_map(|(id, wire): &(NodeId, TraceWire)| {
                wire.texts.lines.iter().map(|l| (*id, l.clone()))
            })
            .collect();
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
        let faces: Vec<serde_json::Value> = used
            .iter()
            .map(|&i| serde_json::json!([listed[i].host_id, listed[i].index]))
            .collect();
        let request = serde_json::json!({
            "tolerance": tolerance,
            "faces": faces,
            "tiers": tiers_json,
            "lines": lines.iter().map(|(_, l)| serde_json::json!({
                "text": l.text,
                "direction": [l.direction.x, l.direction.y],
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
        for ((texts, leftovers), (image, line)) in lines.into_iter().zip(&self.trace_text.requested)
        {
            if texts.is_empty()
                && let Some(asset) = self.asset_of(*image)
            {
                dropped.push((asset, line.text.clone()));
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
                            kind: FontKind::Outline,
                        },
                        run: t.run,
                        glyphs: t.glyphs,
                        nearest: t.nearest,
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
        self.trace_text.fitted = Some(out);
        self.show_fitted_preview();
        true
    }

    /// Whether a finished fit is kept for the current results, so Create paths need not fit again.
    #[must_use]
    pub fn trace_text_fitted(&self) -> bool {
        self.trace_text.fitted.is_some()
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

/// One text Create paths wrote: its node, string, guessed positions, size bits (for the guessed
/// record) and whether it was set in the nearest face.
pub(crate) type WrittenText = (NodeId, String, Vec<usize>, u64, bool);

/// Writes the texts of `fitted` into `document`, each directly above its picture's entry of
/// `above` (the "Trace of" group, or the picture when it has no paths), black, no stroke
/// (decision 6, criterion 10). `None` when a write failed (the caller's step is then reverted).
pub(super) fn write_texts(
    document: &curvyo_document_core::Document,
    fitted: &[FittedImage],
    above: &[(NodeId, NodeId)],
) -> Option<Vec<WrittenText>> {
    use curvyo_document_core::{
        Affine, Length, MAX_TEXT_SIZE, MIN_TEXT_SIZE, MoveTarget, TextChars, TextLayoutValues,
    };
    let range = MIN_TEXT_SIZE.as_mm()..=MAX_TEXT_SIZE.as_mm();
    let style = curvyo_ui_core::trace_style();
    let mut out = Vec::new();
    for image in fitted {
        let Some(&(_, anchor)) = above.iter().find(|(id, _)| *id == image.image) else {
            continue;
        };
        for text in &image.texts {
            let run = &text.run;
            let size = run.size.get();
            if !range.contains(&size) {
                continue;
            }
            let p = run.placement.as_coeffs();
            let id = document
                .create_text_with_chars(
                    &run.text,
                    &text.font,
                    Length::from_mm(size),
                    Affine::new(p[0], p[1], p[2], p[3], p[4], p[5]),
                    &TextLayoutValues {
                        letter_spacing: run.letter_spacing.get() / size,
                        word_spacing: run.word_spacing.get() / size,
                        ..TextLayoutValues::DEFAULT
                    },
                    &TextChars::default(),
                )
                .ok()?;
            for edit in super::recognise::style_edits(&style) {
                document.edit_style(&[id], &edit).ok()?;
            }
            document
                .move_nodes(&curvyo_document_core::MovePlan {
                    nodes: vec![id],
                    target: MoveTarget::Above(anchor),
                })
                .ok()?;
            out.push((
                id,
                run.text.clone(),
                run.guessed.clone(),
                Length::from_mm(size).as_mm().to_bits(),
                text.nearest,
            ));
        }
    }
    Some(out)
}

/// One text of the last Create paths as the host words its warning (criterion 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracedText {
    /// The string.
    pub string: String,
    /// The guessed letters: character and its 1-based position.
    pub guessed: Vec<(char, usize)>,
    /// Set in the nearest face, every letter guessed (114 B).
    pub nearest: bool,
}

impl Session {
    /// Keeps the guessed letters of the texts Create paths just wrote in the session record the
    /// Glyph table and the Text tool highlight (criterion 4, `0088` decision 6), and the report of
    /// [`Session::trace_created_texts`].
    pub(super) fn record_trace_texts(&mut self, texts: &[WrittenText]) {
        let record: Vec<(NodeId, Vec<usize>, String, u64)> = texts
            .iter()
            .filter(|t| !t.2.is_empty())
            .map(|(id, string, guessed, size, _)| (*id, guessed.clone(), string.clone(), *size))
            .collect();
        if !record.is_empty() {
            self.recognise.guessed = super::recognise::GuessedRecord {
                texts: record,
                selection: self.selection.ids().to_vec(),
            };
        }
        self.trace_text.created = texts
            .iter()
            .map(|(_, string, guessed, _, nearest)| {
                let chars: Vec<char> = string.chars().collect();
                TracedText {
                    string: string.clone(),
                    guessed: guessed
                        .iter()
                        .filter_map(|&p| chars.get(p).map(|&c| (c, p + 1)))
                        .collect(),
                    nearest: *nearest,
                }
            })
            .collect();
        self.drop_trace_fit();
    }

    /// The warning about the guessed letters of the last Create paths (criterion 5, UX notes §5),
    /// `None` when every letter is sure.
    #[must_use]
    pub fn trace_created_warning(&self) -> Option<String> {
        let texts: Vec<curvyo_ui_core::WarnedText<'_>> = self
            .trace_text
            .created
            .iter()
            .map(|t| curvyo_ui_core::WarnedText {
                string: &t.string,
                guessed: &t.guessed,
                nearest: t.nearest,
            })
            .collect();
        curvyo_ui_core::traced_text_warning(&texts)
    }

    /// The texts the last Create paths wrote (criterion 5: the host warns about guessed letters).
    #[must_use]
    pub fn trace_created_texts(&self) -> &[TracedText] {
        &self.trace_text.created
    }
}
