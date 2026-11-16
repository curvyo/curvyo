//! The `wasm-bindgen` surface of Find centre lines (`specs/0117-centre-lines`): the bar's open,
//! preview and sliced Apply on the session, the worker's class, and the outcome as codes and
//! counts (the sentences are the host's, ADR 0001 §5).

use curvyo_ui_core::CentreLinesRefusal;
use wasm_bindgen::prelude::*;

use crate::centre_lines_glue::{CentreLinesWorker, centre_options};
use crate::session::CentreLinesOutcome;
use crate::wasm_api::WasmSession;
use crate::wasm_trace::TraceProgressSink;

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What `apply_centre_lines` did. `kind` is `"ignored"`, `"applied"` (with the counts of the
/// notice), `"stale"` (the subjects changed: open again) or a refusal: `"text"`, `"image"`,
/// `"not_filled"`, `"too_large"` (`nodes`). Nothing was changed by anything but `"applied"`.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct CentreLinesView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Subjects converted.
    pub converted: u32,
    /// Regions kept filled (blobs, thick parts and failed skeletons), of kept and converted
    /// subjects.
    pub kept: u32,
    /// Selected objects that are not subjects.
    pub skipped: u32,
    /// Clones that became ordinary paths.
    pub unlinked: u32,
    /// The node count of a `"too_large"` refusal.
    pub nodes: u32,
}

fn refusal_view(refusal: &CentreLinesRefusal) -> CentreLinesView {
    let (kind, nodes) = match refusal {
        CentreLinesRefusal::Text => ("text", 0),
        CentreLinesRefusal::Image => ("image", 0),
        CentreLinesRefusal::NotFilled => ("not_filled", 0),
        CentreLinesRefusal::TooLarge { nodes } => ("too_large", *nodes),
    };
    CentreLinesView {
        kind: kind.to_owned(),
        nodes: count(nodes),
        ..CentreLinesView::default()
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Why a press of Find centre lines would be refused now, as a refusal view, or `"none"`.
    #[must_use]
    pub fn centre_lines_refusal(&self) -> CentreLinesView {
        self.session.centre_lines_refusal().map_or_else(
            || CentreLinesView {
                kind: "none".to_owned(),
                ..CentreLinesView::default()
            },
            |r| refusal_view(&r),
        )
    }

    /// The screen box around the selection as `[left, top, right, bottom]`, where Shift+F10 or
    /// the Menu key opens the context menu from (`ux-notes.md` §1); empty without a selection.
    #[must_use]
    pub fn centre_lines_anchor(&self) -> Vec<f64> {
        let Some((min, max)) = self.session.centre_lines_anchor() else {
            return Vec::new();
        };
        let view = self.session.view();
        let (x0, y0) = view.document_to_screen(min);
        let (x1, y1) = view.document_to_screen(max);
        vec![x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)]
    }

    /// Find centre lines on the selection, in one commit (criterion 2). `overlap` is a fraction
    /// (-0.5 to 0.5), `thick_parts_mm` from 0.1 mm to [`THICK_PARTS_MAX_MM`] (which means no
    /// limit), `optimise` 0 to 1, `inner_offset_mm` is used when `keep_inner`.
    pub fn apply_centre_lines(
        &mut self,
        overlap: f64,
        thick_parts_mm: f64,
        optimise: f64,
        keep_inner: bool,
        inner_offset_mm: f64,
    ) -> CentreLinesView {
        let options = centre_options(
            overlap,
            thick_parts_mm,
            optimise,
            keep_inner,
            inner_offset_mm,
        );
        outcome_view(self.session.apply_centre_lines(options))
    }
}

fn outcome_view(outcome: CentreLinesOutcome) -> CentreLinesView {
    match outcome {
        CentreLinesOutcome::Ignored => CentreLinesView {
            kind: "ignored".to_owned(),
            ..CentreLinesView::default()
        },
        CentreLinesOutcome::Stale => CentreLinesView {
            kind: "stale".to_owned(),
            ..CentreLinesView::default()
        },
        CentreLinesOutcome::Applied {
            converted,
            kept_regions,
            failed,
            skipped,
            unlinked,
            ..
        } => CentreLinesView {
            kind: "applied".to_owned(),
            converted: count(converted),
            kept: count(kept_regions + failed),
            skipped: count(skipped),
            unlinked: count(unlinked),
            nodes: 0,
        },
        CentreLinesOutcome::Refused(refusal) => refusal_view(&refusal),
    }
}

/// The open bar's answer: `kind` `"open"` with the subjects' `wire` for the worker, or a
/// refusal code as in [`CentreLinesView`].
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct CentreLinesOpenView {
    /// `"open"` or the refusal.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// The node count of a `"too_large"` refusal.
    pub nodes: u32,
    /// The subjects for the worker's `subjects`.
    #[wasm_bindgen(getter_with_clone)]
    pub wire: Vec<f64>,
}

/// A shown preview's counts (UX notes §3.3 and §4); `shown` is false when it did not fit.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, Default)]
pub struct CentreLinesPreviewView {
    /// The preview was taken.
    pub shown: bool,
    /// Centre paths.
    pub paths: u32,
    /// Their nodes.
    pub nodes: u32,
    /// Regions kept filled.
    pub kept: u32,
}

/// One slice of Apply: `kind` `"pending"` with `progress` (0 to 1) while slices remain, or the
/// outcome's view.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct CentreLinesStepView {
    /// `"pending"` or the outcome's kind.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// The share done, while pending.
    pub progress: f64,
    /// The outcome once done.
    #[wasm_bindgen(getter_with_clone)]
    pub outcome: CentreLinesView,
}

#[wasm_bindgen]
impl WasmSession {
    /// Opens the Find centre lines bar: the subjects for the worker, or the refusal (nothing
    /// opens then, criterion 4).
    pub fn centre_lines_open(&mut self) -> CentreLinesOpenView {
        match self.session.centre_lines_open() {
            Ok(wire) => CentreLinesOpenView {
                kind: "open".to_owned(),
                nodes: 0,
                wire,
            },
            Err(refusal) => {
                let view = refusal_view(&refusal);
                CentreLinesOpenView {
                    kind: view.kind,
                    nodes: view.nodes,
                    wire: Vec::new(),
                }
            }
        }
    }

    /// Whether the bar is open (a press on empty canvas closes it).
    #[must_use]
    pub fn centre_lines_is_open(&self) -> bool {
        self.session.centre_lines_is_open()
    }

    /// Whether the subjects sent are still the document's and the selection's.
    #[must_use]
    pub fn centre_lines_current(&self) -> bool {
        self.session.centre_lines_current()
    }

    /// Shows the worker's preview.
    pub fn centre_lines_show(&mut self, wire: &[f64]) -> CentreLinesPreviewView {
        self.session
            .centre_lines_show(wire)
            .map_or_else(CentreLinesPreviewView::default, |c| {
                CentreLinesPreviewView {
                    shown: true,
                    paths: count(c.paths),
                    nodes: count(c.nodes),
                    kept: count(c.kept),
                }
            })
    }

    /// The shown preview is older than the values (drawn at half strength) or current again.
    pub fn centre_lines_set_stale(&mut self, stale: bool) {
        self.session.centre_lines_set_stale(stale);
    }

    /// Closes the bar without a write (Cancel, Escape).
    pub fn centre_lines_close(&mut self) {
        self.session.centre_lines_close();
    }

    /// Starts Apply with the worker's result: `"pending"` with progress 0, or the outcome when
    /// nothing will be written (`"stale"`, `"ignored"`).
    pub fn centre_lines_apply_begin(&mut self, wire: &[f64]) -> CentreLinesStepView {
        match self.session.centre_lines_apply_begin(wire) {
            Ok(_) => CentreLinesStepView {
                kind: "pending".to_owned(),
                progress: 0.0,
                outcome: CentreLinesView::default(),
            },
            Err(outcome) => {
                let view = outcome_view(outcome);
                CentreLinesStepView {
                    kind: view.kind.clone(),
                    progress: 1.0,
                    outcome: view,
                }
            }
        }
    }

    /// One slice of Apply; the last one writes the one undo step.
    pub fn centre_lines_apply_step(&mut self) -> CentreLinesStepView {
        match self.session.centre_lines_apply_step() {
            None => CentreLinesStepView {
                kind: "pending".to_owned(),
                progress: self.session.centre_lines_apply_progress(),
                outcome: CentreLinesView::default(),
            },
            Some(outcome) => {
                let view = outcome_view(outcome);
                CentreLinesStepView {
                    kind: view.kind.clone(),
                    progress: 1.0,
                    outcome: view,
                }
            }
        }
    }
}

/// The worker's Find centre lines state (ADR 0063 amendment 2026-11-18, W): it lives in the
/// second worker and keeps each subject's skeleton across option changes.
#[wasm_bindgen]
pub struct WasmCentreLinesWorker {
    inner: CentreLinesWorker,
}

impl Default for WasmCentreLinesWorker {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmCentreLinesWorker {
    /// An empty worker state.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: CentreLinesWorker::new(),
        }
    }

    /// Takes the subjects' wire, building the skeletons it lacks; `sink.report(percent)`
    /// hears the share built. False for a malformed wire (nothing changed).
    pub fn subjects(&mut self, wire: &[f64], sink: &TraceProgressSink) -> bool {
        let mut last = 0;
        let mut hear = |percent: u32| {
            if percent != last {
                last = percent;
                sink.report(u8::try_from(percent.min(100)).unwrap_or(100));
            }
        };
        self.inner.set_subjects(wire, &mut hear).is_ok()
    }

    /// The preview of the options (as `apply_centre_lines` reads them), with each path's
    /// painted outline; kept as the latest result.
    pub fn finish(
        &mut self,
        overlap: f64,
        thick_parts_mm: f64,
        optimise: f64,
        keep_inner: bool,
        inner_offset_mm: f64,
    ) -> Vec<f64> {
        let options = centre_options(
            overlap,
            thick_parts_mm,
            optimise,
            keep_inner,
            inner_offset_mm,
        );
        self.inner.finish(&options, true)
    }

    /// The latest result for Apply, without painted outlines.
    #[must_use]
    pub fn result(&self) -> Vec<f64> {
        self.inner.result()
    }
}
