//! The `wasm-bindgen` surface of Split and Break apart (`specs/0048-split-compound-path`): the
//! two commands of the Path card. A further `impl WasmSession` block; codes and counts only (the
//! sentences are the host's, ADR 0001 §5), every method a pass-through to `Session`.

use curvyo_ui_core::{BreakApartRefusal, SplitRefusal};
use wasm_bindgen::prelude::*;

use crate::session::{BreakApartOutcome, SplitOutcome};
use crate::wasm_api::WasmSession;

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What `apply_split` did. `kind` is `"ignored"`, `"applied"` (`compounds` compound paths became
/// `pieces` objects; `with_holes` says a piece has a hole; `left_alone` compound paths were one
/// piece already) or a refusal: `"no_compound"`, `"groups"` or `"one_piece"` (`compounds` selected
/// compound paths are one piece each). Nothing was changed by anything but `"applied"`.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct SplitResultView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Compound paths split, or selected compound paths when refused as one piece.
    pub compounds: u32,
    /// Objects they became.
    pub pieces: u32,
    /// A piece has a hole.
    pub with_holes: bool,
    /// Compound paths left as they are.
    pub left_alone: u32,
}

impl From<SplitOutcome> for SplitResultView {
    fn from(outcome: SplitOutcome) -> Self {
        let view =
            |kind: &str, compounds: usize, pieces: usize, with_holes: bool, left: usize| Self {
                kind: kind.to_string(),
                compounds: count(compounds),
                pieces: count(pieces),
                with_holes,
                left_alone: count(left),
            };
        match outcome {
            SplitOutcome::Ignored => view("ignored", 0, 0, false, 0),
            SplitOutcome::Applied {
                compounds,
                pieces,
                with_holes,
                left_alone,
            } => view("applied", compounds, pieces, with_holes, left_alone),
            SplitOutcome::Refused(SplitRefusal::NoCompound) => view("no_compound", 0, 0, false, 0),
            SplitOutcome::Refused(SplitRefusal::OnePiece { compounds }) => {
                view("one_piece", compounds, 0, false, 0)
            }
            SplitOutcome::Refused(SplitRefusal::Groups { offenders }) => {
                view("groups", offenders.len(), 0, false, 0)
            }
        }
    }
}

/// What `apply_break_apart` did. `kind` is `"ignored"`, `"applied"` (`compounds` compound paths
/// became `pieces` objects; `with_holes` says a hole became a filled shape) or a refusal:
/// `"no_compound"` or `"groups"`. Nothing was changed by anything but `"applied"`.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct BreakApartResultView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Compound paths broken apart, or leaves of groups to blame.
    pub compounds: u32,
    /// Objects they became.
    pub pieces: u32,
    /// A hole became a filled shape.
    pub with_holes: bool,
}

impl From<BreakApartOutcome> for BreakApartResultView {
    fn from(outcome: BreakApartOutcome) -> Self {
        let view = |kind: &str, compounds: usize, pieces: usize, with_holes: bool| Self {
            kind: kind.to_string(),
            compounds: count(compounds),
            pieces: count(pieces),
            with_holes,
        };
        match outcome {
            BreakApartOutcome::Ignored => view("ignored", 0, 0, false),
            BreakApartOutcome::Applied {
                compounds,
                pieces,
                with_holes,
            } => view("applied", compounds, pieces, with_holes),
            BreakApartOutcome::Refused(BreakApartRefusal::NoCompound) => {
                view("no_compound", 0, 0, false)
            }
            BreakApartOutcome::Refused(BreakApartRefusal::Groups { offenders }) => {
                view("groups", offenders.len(), 0, false)
            }
        }
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Splits the selected compound paths into their unconnected parts, in one commit.
    pub fn apply_split(&mut self) -> SplitResultView {
        self.session.apply_split().into()
    }

    /// Breaks the selected compound paths apart into one closed path per outline, in one commit.
    pub fn apply_break_apart(&mut self) -> BreakApartResultView {
        self.session.apply_break_apart().into()
    }
}
