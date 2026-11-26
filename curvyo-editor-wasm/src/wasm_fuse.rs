//! The `wasm-bindgen` surface of Bridge (`specs/0156-fused-lettering`): its availability on the
//! active page, the field limits, and the command with the host's remembered values. Codes and
//! counts only (the sentences are the host's, ADR 0001 §5); every method a pass-through to
//! `Session`.

use curvyo_ui_core::{
    FUSE_BRIDGES_PER_ISLAND, FUSE_REACH_MM, FUSE_WIDTH_MM, FieldRange, FuseMode, FuseRefusal,
    FuseSettings,
};
use wasm_bindgen::prelude::*;

use crate::session::{FuseAvailability, FuseOutcome, FuseSessionRefusal};
use crate::wasm_api::WasmSession;

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What `apply_fuse` did. `kind` is `"ignored"`, `"applied"` (`count` bridges) or a refusal:
/// `"one_page"`, `"nothing_to_join"`, `"too_many_bridges"`, `"no_islands"`,
/// `"extended_path"`, `"no_area"`, `"out_of_range"` (`count` of the `of` selected objects are to
/// blame), `"missing_fonts"`, `"fonts_loading"` or `"single_line_texts"` (`count` texts).
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct FuseResultView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Bridges made, or objects or texts to blame for a refusal.
    pub count: u32,
    /// Objects selected.
    pub of: u32,
}

impl From<FuseOutcome> for FuseResultView {
    fn from(outcome: FuseOutcome) -> Self {
        let view = |kind: &str, count_of: usize, of: usize| Self {
            kind: kind.to_owned(),
            count: count(count_of),
            of: count(of),
        };
        match outcome {
            FuseOutcome::Ignored => view("ignored", 0, 0),
            FuseOutcome::Applied { bridges } => view("applied", bridges, 0),
            FuseOutcome::Refused(FuseSessionRefusal::OnePage) => view("one_page", 0, 0),
            FuseOutcome::Refused(FuseSessionRefusal::Plan(refusal)) => match refusal {
                FuseRefusal::NothingToJoin => view("nothing_to_join", 0, 0),
                FuseRefusal::TooManyBridges => view("too_many_bridges", 0, 0),
                FuseRefusal::NoIslands => view("no_islands", 0, 0),
                FuseRefusal::ExtendedPath { offenders, of } => {
                    view("extended_path", offenders.len(), of)
                }
                FuseRefusal::NoArea { of, .. } => view("no_area", of, of),
                FuseRefusal::OutOfRange { objects, of, .. } => view("out_of_range", objects, of),
                FuseRefusal::Texts(texts) => view(
                    crate::wasm_text::refusal_kind(&texts),
                    texts.offenders.len(),
                    0,
                ),
            },
        }
    }
}

/// The limits and default of a Bridge field.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct FuseFieldView {
    /// The smallest value.
    pub min: f64,
    /// The largest value.
    pub max: f64,
    /// The first-use value.
    pub default: f64,
}

impl From<FieldRange> for FuseFieldView {
    fn from(range: FieldRange) -> Self {
        Self {
            min: range.min,
            max: range.max,
            default: range.default,
        }
    }
}

/// The Bridge fields' limits: `width`, `reach` (mm) and `bridges` (per island).
#[wasm_bindgen]
#[must_use]
pub fn fuse_field(name: &str) -> Option<FuseFieldView> {
    match name {
        "width" => Some(FUSE_WIDTH_MM.into()),
        "reach" => Some(FUSE_REACH_MM.into()),
        "bridges" => Some(FUSE_BRIDGES_PER_ISLAND.into()),
        _ => None,
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// What the Bridge buttons show: `"absent"` (not a laser page: no button), `"needs_one"`
    /// (dimmed) or `"ready"`. Call it with every other state read.
    #[must_use]
    pub fn fuse_availability(&self) -> String {
        match self.session.fuse_availability() {
            FuseAvailability::Absent => "absent",
            FuseAvailability::NeedsOne => "needs_one",
            FuseAvailability::Ready => "ready",
        }
        .to_owned()
    }

    /// Bridge on the selected objects, in one commit, with the host's remembered values (clamped
    /// again here). `stencil` picks Stencil mode; `join_all` ignores Reach.
    pub fn apply_fuse(
        &mut self,
        stencil: bool,
        width_mm: f64,
        reach_mm: f64,
        join_all: bool,
        bridges_per_island: f64,
    ) -> FuseResultView {
        if self.session.refuse_images() {
            return FuseResultView {
                kind: "images".to_owned(),
                ..FuseResultView::default()
            };
        }
        let settings = FuseSettings {
            mode: if stencil {
                FuseMode::Stencil
            } else {
                FuseMode::Lettering
            },
            width_mm,
            reach_mm,
            join_all,
            bridges_per_island,
        };
        self.session.apply_fuse(settings).into()
    }
}
