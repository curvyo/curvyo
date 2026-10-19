//! The `wasm-bindgen` surface of the Select bar's ellipse group and "Curve" field
//! (`specs/0021-ellipse-arcs-and-shaping/` criteria 3, 26, 44 and 65): a further
//! `impl WasmSession` block. Strings and scalars only (ADR 0001 §5); every method passes
//! through to `Session`.

use curvyo_document_core::ArcKind;
use curvyo_ui_core::{ArcEnd, BarValue, CurveBarState, EllipseBarState, EntryOutcome, GreyReason};
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// The JS-facing state of the ellipse group and the "Curve" field. A `*_mixed` flag
/// means the objects differ (the field shows "Mixed"); the value beside it is then
/// meaningless. A `*_grey` string is empty when the control applies, else the reason:
/// `"curve"` (the curve is not 100 %), `"no-arc"` (no selected ellipse has an arc) or
/// `"arc"` (the ellipse has an arc).
#[wasm_bindgen]
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ArcBarView {
    /// The ellipse group is shown (the selection holds an ellipse).
    pub ellipse_shown: bool,
    /// The arc kind: `"pie"`, `"arc"`, `"chord"`, `"mixed"`, or `""` while no selected
    /// ellipse has an arc.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Why the kind choice and "Whole ellipse" are greyed.
    #[wasm_bindgen(getter_with_clone)]
    pub kind_grey: String,
    /// "Start", degrees.
    pub start: f64,
    /// The selected ellipses show different start angles.
    pub start_mixed: bool,
    /// "End", degrees.
    pub end: f64,
    /// The selected ellipses show different end angles.
    pub end_mixed: bool,
    /// Why "Start" and "End" are greyed.
    #[wasm_bindgen(getter_with_clone)]
    pub angles_grey: String,
    /// The "Curve" field is shown.
    pub curve_shown: bool,
    /// The effective curve, percent.
    pub curve: f64,
    /// The objects show different curves.
    pub curve_mixed: bool,
    /// Why the field is greyed.
    #[wasm_bindgen(getter_with_clone)]
    pub curve_grey: String,
    /// A polygon's or star's stored curve lies below its limit (the "limited" tag).
    pub curve_limited: bool,
    /// The stored curve, percent, for the tag's tooltip.
    pub curve_stored: f64,
}

fn grey(reason: Option<GreyReason>) -> String {
    match reason {
        None => "",
        Some(GreyReason::CurveNotFull) => "curve",
        Some(GreyReason::NoArc) => "no-arc",
        Some(GreyReason::HasArc) => "arc",
    }
    .to_string()
}

fn value(value: Option<BarValue<f64>>) -> (f64, bool) {
    match value {
        Some(BarValue::Uniform(v)) => (v, false),
        Some(BarValue::Mixed) => (0.0, true),
        None => (0.0, false),
    }
}

impl ArcBarView {
    fn new(ellipse: Option<&EllipseBarState>, curve: Option<&CurveBarState>) -> Self {
        let kind = match ellipse.and_then(|e| e.kind) {
            None => "",
            Some(BarValue::Mixed) => "mixed",
            Some(BarValue::Uniform(kind)) => kind.name(),
        };
        let (start, start_mixed) = value(ellipse.and_then(|e| e.start));
        let (end, end_mixed) = value(ellipse.and_then(|e| e.end));
        let (curve_value, curve_mixed) = value(curve.and_then(|c| c.value));
        let limited = curve.and_then(|c| c.limited);
        Self {
            ellipse_shown: ellipse.is_some(),
            kind: kind.to_string(),
            kind_grey: grey(ellipse.and_then(|e| e.kind_grey)),
            start,
            start_mixed,
            end,
            end_mixed,
            angles_grey: grey(ellipse.and_then(|e| e.angles_grey)),
            curve_shown: curve.is_some(),
            curve: curve_value,
            curve_mixed,
            curve_grey: grey(curve.and_then(|c| c.grey)),
            curve_limited: limited.is_some(),
            curve_stored: limited.map_or(0.0, |(stored, _)| stored),
        }
    }
}

/// The string an edit reports to the host: `"committed"`, `"unchanged"` or
/// `"invalid:number"`.
fn outcome_code(outcome: EntryOutcome) -> String {
    match outcome {
        EntryOutcome::Committed => "committed",
        EntryOutcome::Unchanged => "unchanged",
        EntryOutcome::Invalid { .. } => "invalid:number",
    }
    .to_string()
}

#[wasm_bindgen]
impl WasmSession {
    /// What the ellipse group and the "Curve" field show for the current selection.
    #[must_use]
    pub fn arc_bar_state(&self) -> ArcBarView {
        let state = self.session.select_bar_state();
        ArcBarView::new(state.ellipse.as_ref(), state.curve.as_ref())
    }

    /// "Pie", "Arc" or "Chord" (`kind` is `"pie"`, `"arc"` or `"chord"`).
    pub fn set_selected_arc_kind(&mut self, kind: &str) -> String {
        ArcKind::from_name(kind).map_or_else(
            || "unchanged".to_string(),
            |kind| outcome_code(self.session.set_selected_arc_kind(kind)),
        )
    }

    /// "Whole ellipse".
    pub fn make_selected_ellipses_whole(&mut self) -> String {
        outcome_code(self.session.make_selected_ellipses_whole())
    }

    /// Enter in "Start" (`end` false) or "End" (`end` true), degrees.
    pub fn set_selected_arc_angle(&mut self, end: bool, text: &str) -> String {
        let which = if end { ArcEnd::End } else { ArcEnd::Start };
        outcome_code(self.session.set_selected_arc_angle_text(which, text))
    }

    /// Enter in "Curve", percent.
    pub fn set_selected_curve(&mut self, text: &str) -> String {
        outcome_code(self.session.set_selected_curve_text(text))
    }

    /// A drag of the "Curve" field, on every tick: writes nothing.
    pub fn preview_selected_curve(&mut self, percent: f64) {
        self.session.preview_selected_curve(percent);
    }

    /// The release of a "Curve" field drag: one commit.
    pub fn commit_selected_curve(&mut self) {
        self.session.commit_selected_curve();
    }

    /// Escape during a "Curve" field drag: writes nothing.
    pub fn cancel_selected_curve(&mut self) {
        self.session.cancel_selected_ratio();
    }
}
