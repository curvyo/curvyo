//! The Offset entry's `wasm-bindgen` surface (`specs/0038-path-offset`): what the Select bar
//! button and the Properties panel section show, their values, and Apply. A further
//! `impl WasmSession` block; codes and counts only (the sentences are the host's, ADR 0001 §5),
//! every method a pass-through to `Session`.

use curvyo_ui_core::{
    OFFSET_DRAG_LIMIT_MM, OFFSET_TYPED_LIMIT_MM, OffsetCap, OffsetJoin, OffsetRefusal,
    offset_cap_from_code, offset_join_from_code,
};
use wasm_bindgen::prelude::*;

use crate::session::{OffsetOutcome, OffsetPreviewState};
use crate::wasm_api::WasmSession;

/// What the Offset entry shows. `kind` is `"closed"`, `"result"` (`vanished` of `of` operands
/// are gone), `"zero"`, `"no_operands"`, or the refusal Apply would give: `"inset_open"`,
/// `"out_of_range"`, `"no_area"` (`count` of `of` selected objects to blame) or `"empty"`.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct OffsetStateView {
    /// The state code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Objects to blame.
    pub count: u32,
    /// Operands or selected objects.
    pub of: u32,
    /// Operands that vanish.
    pub vanished: u32,
    /// The Offset button is shown.
    pub offered: bool,
    /// An open path is selected: the Cap choice is shown.
    pub has_open: bool,
    /// The distance in millimetres.
    pub distance_mm: f64,
    /// `"round"`, `"miter"` or `"bevel"`.
    #[wasm_bindgen(getter_with_clone)]
    pub join: String,
    /// `"round"`, `"flat"` or `"square"`.
    #[wasm_bindgen(getter_with_clone)]
    pub cap: String,
    /// The largest typed distance either way.
    pub typed_limit_mm: f64,
    /// The largest dragged distance either way.
    pub drag_limit_mm: f64,
}

/// What `apply_offset` did. `kind` is `"ignored"`, `"closed_zero"`, `"applied"` (`count` shapes
/// added, `vanished` of `of` operands gone, at `distance_mm`), `"groups"` (`count` leaves
/// outlined) or a refusal code of [`OffsetStateView`].
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct OffsetResultView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Shapes added, or objects to blame.
    pub count: u32,
    /// Operands or selected objects.
    pub of: u32,
    /// Operands that vanished.
    pub vanished: u32,
    /// The signed distance in millimetres.
    pub distance_mm: f64,
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The code, blamed count and selected count of a refusal.
fn refusal_code(refusal: &OffsetRefusal) -> (&'static str, usize, usize) {
    match refusal {
        OffsetRefusal::NoOperands => ("no_operands", 0, 0),
        OffsetRefusal::ZeroDistance => ("zero", 0, 0),
        OffsetRefusal::InsetOfOpenPaths { offenders, of } => ("inset_open", offenders.len(), *of),
        OffsetRefusal::OutOfRange { offenders, of } => ("out_of_range", offenders.len(), *of),
        OffsetRefusal::NoArea { offenders, of } => ("no_area", offenders.len(), *of),
        OffsetRefusal::Empty => ("empty", 0, 0),
    }
}

const fn join_code(join: OffsetJoin) -> &'static str {
    match join {
        OffsetJoin::Round => "round",
        OffsetJoin::Miter => "miter",
        OffsetJoin::Bevel => "bevel",
    }
}

const fn cap_code(cap: OffsetCap) -> &'static str {
    match cap {
        OffsetCap::Round => "round",
        OffsetCap::Flat => "flat",
        OffsetCap::Square => "square",
    }
}

impl From<OffsetOutcome> for OffsetResultView {
    fn from(outcome: OffsetOutcome) -> Self {
        let view = |kind: &str, n: usize, of: usize, vanished: usize, distance_mm: f64| Self {
            kind: kind.to_string(),
            count: count(n),
            of: count(of),
            vanished: count(vanished),
            distance_mm,
        };
        match outcome {
            OffsetOutcome::Ignored => view("ignored", 0, 0, 0, 0.0),
            OffsetOutcome::ClosedAtZero => view("closed_zero", 0, 0, 0, 0.0),
            OffsetOutcome::Applied {
                added,
                vanished,
                of,
                distance_mm,
            } => view("applied", added, of, vanished, distance_mm),
            OffsetOutcome::RefusedGroups { leaves } => view("groups", leaves, leaves, 0, 0.0),
            OffsetOutcome::Refused(refusal) => {
                let (code, n, of) = refusal_code(&refusal);
                view(code, n, of, 0, 0.0)
            }
        }
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Everything the Offset button and section show. Call it with every other state read.
    #[must_use]
    pub fn offset_state(&self) -> OffsetStateView {
        let availability = self.session.offset_availability();
        let settings = self.session.offset_settings();
        let (kind, n, of, vanished) = match self.session.offset_preview_state() {
            OffsetPreviewState::Closed => ("closed", 0, 0, 0),
            OffsetPreviewState::Result { vanished, of } => ("result", 0, of, vanished),
            OffsetPreviewState::Refused(refusal) => {
                let (code, n, of) = refusal_code(&refusal);
                (code, n, of, 0)
            }
        };
        OffsetStateView {
            kind: kind.to_string(),
            count: count(n),
            of: count(of),
            vanished: count(vanished),
            offered: availability.offered,
            has_open: availability.has_open,
            distance_mm: settings.distance_mm(),
            join: join_code(settings.join()).to_string(),
            cap: cap_code(settings.cap()).to_string(),
            typed_limit_mm: OFFSET_TYPED_LIMIT_MM,
            drag_limit_mm: OFFSET_DRAG_LIMIT_MM,
        }
    }

    /// Opens the Offset entry for the selection; returns whether it opened.
    pub fn open_offset(&mut self) -> bool {
        self.session.open_offset()
    }

    /// Closes the Offset entry without writing.
    pub fn close_offset(&mut self) {
        self.session.close_offset();
    }

    /// Sets the distance in millimetres; returns whether it was within the typed range.
    pub fn set_offset_distance(&mut self, distance_mm: f64) -> bool {
        self.session.set_offset_distance(distance_mm)
    }

    /// Sets the join (`"round"`, `"miter"`, `"bevel"`).
    ///
    /// # Errors
    /// A `JsValue` if the code is none of those.
    pub fn set_offset_join(&mut self, code: &str) -> Result<(), JsValue> {
        let join = offset_join_from_code(code)
            .ok_or_else(|| JsValue::from_str(&format!("unknown join: {code}")))?;
        self.session.set_offset_join(join);
        Ok(())
    }

    /// Sets the cap (`"round"`, `"flat"`, `"square"`).
    ///
    /// # Errors
    /// A `JsValue` if the code is none of those.
    pub fn set_offset_cap(&mut self, code: &str) -> Result<(), JsValue> {
        let cap = offset_cap_from_code(code)
            .ok_or_else(|| JsValue::from_str(&format!("unknown cap: {code}")))?;
        self.session.set_offset_cap(cap);
        Ok(())
    }

    /// Applies the previewed offset in one commit.
    pub fn apply_offset(&mut self) -> OffsetResultView {
        self.session.apply_offset().into()
    }
}
