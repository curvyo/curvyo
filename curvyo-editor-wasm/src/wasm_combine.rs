//! The Path card's `wasm-bindgen` surface (`specs/0035-combine-and-break-apart`): what its
//! buttons show and Combine. A further `impl WasmSession` block; codes and counts only (the
//! sentences are the host's, ADR 0001 §5), every method a pass-through to `Session`. Split and
//! Break apart are in `wasm_split.rs`, Cut in `wasm_cut.rs`. The refusal outline is cleared by
//! `clear_boolean_refusal`, which serves every rail command.

use curvyo_ui_core::{BooleanAvailability, CombineRefusal, PathAvailability};
use wasm_bindgen::prelude::*;

use crate::session::CombineOutcome;
use crate::wasm_api::WasmSession;

/// What the Path card shows: whether Combine is dimmed (`needs_two`; else enabled, open paths
/// included), whether Split and Break apart are enabled (one flag: the selection holds a compound
/// path) and whether Cut is.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct PathAvailabilityView {
    /// Fewer than two objects are selected, or the Select tool is not active.
    pub needs_two: bool,
    /// The selection holds a compound path: Split and Break apart are enabled.
    pub has_compound: bool,
    /// Something is selected: Cut is enabled.
    pub can_cut: bool,
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl From<PathAvailability> for PathAvailabilityView {
    fn from(availability: PathAvailability) -> Self {
        Self {
            needs_two: availability.combine == BooleanAvailability::NeedsTwo,
            has_compound: availability.compound,
            can_cut: availability.cut,
        }
    }
}

/// What `apply_combine` did. `kind` is `"ignored"`, `"applied"` (`count` objects became one
/// compound path with `holes` holes and `open` open outlines; `touching` closed outlines touch or
/// cross, `touching_holes` holes touch or cross each other, `single_nodes_removed` single-node
/// outlines were left out, `markers` says an operand had markers, `styles_differ` says the lowest
/// object's style was used) or a refusal: `"needs_two"`, `"out_of_range"` or `"groups"` (`count`
/// of the `of` selected objects are to blame), or `"single_nodes_only"`. Nothing was changed by
/// anything but `"applied"`.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct CombineResultView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Objects combined, or objects to blame.
    pub count: u32,
    /// Objects selected.
    pub of: u32,
    /// Holes in the result.
    pub holes: u32,
    /// Open outlines in the result.
    pub open: u32,
    /// Closed outlines that touch or cross another closed outline or themselves.
    pub touching: u32,
    /// Holes that touch or cross another hole.
    pub touching_holes: u32,
    /// Outlines with a single node that were left out.
    pub single_nodes_removed: u32,
    /// An operand had a marker on; the result draws none.
    pub markers: bool,
    /// The operands' styles were not all equal.
    pub styles_differ: bool,
}

impl From<CombineOutcome> for CombineResultView {
    fn from(outcome: CombineOutcome) -> Self {
        let refused = |kind: &str, n: usize, of: usize| Self {
            kind: kind.to_string(),
            count: count(n),
            of: count(of),
            ..Self::default()
        };
        match outcome {
            CombineOutcome::Ignored => refused("ignored", 0, 0),
            CombineOutcome::Applied {
                operands,
                holes,
                open,
                touching,
                touching_holes,
                single_nodes_removed,
                markers,
                styles_differ,
            } => Self {
                kind: "applied".to_string(),
                count: count(operands),
                of: count(operands),
                holes: count(holes),
                open: count(open),
                touching: count(touching),
                touching_holes: count(touching_holes),
                single_nodes_removed: count(single_nodes_removed),
                markers,
                styles_differ,
            },
            CombineOutcome::Refused(refusal) => match refusal {
                CombineRefusal::NeedsTwo => refused("needs_two", 0, 0),
                CombineRefusal::OutOfRange { offenders, of } => {
                    refused("out_of_range", offenders.len(), of)
                }
                CombineRefusal::Groups { offenders } => {
                    refused("groups", offenders.len(), offenders.len())
                }
                CombineRefusal::SingleNodesOnly => refused("single_nodes_only", 0, 0),
                CombineRefusal::Texts(texts) => refused(
                    crate::wasm_text::refusal_kind(&texts),
                    texts.offenders.len(),
                    0,
                ),
            },
        }
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// What the Path card of the tool rail shows. Call it with every other state read.
    #[must_use]
    pub fn path_availability(&self) -> PathAvailabilityView {
        self.session.path_availability().into()
    }

    /// Combines the selected objects into one compound path, in one commit.
    pub fn apply_combine(&mut self) -> CombineResultView {
        self.session.apply_combine().into()
    }
}
