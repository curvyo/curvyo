//! The `wasm-bindgen` surface of Fracture and Flatten (`specs/0037-fracture-and-flatten`), the
//! sixth and seventh commands of the Boolean card. A further `impl WasmSession` block; codes and
//! counts only (the sentences are the host's, ADR 0001 §5), every method a pass-through to
//! `Session`.

use curvyo_ui_core::{BooleanRefusal, StackRefusal};
use wasm_bindgen::prelude::*;

use crate::session::StackOutcome;
use crate::wasm_api::WasmSession;

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What `apply_fracture` or `apply_flatten` did. `kind` is `"ignored"`, `"applied"` (`removed` of
/// the `of` selected objects were replaced or removed, `pieces` objects were made, `hidden` were
/// covered completely, `strokes_follow` says a trimmed object has a stroke, and the open-outline
/// counts) or a refusal: `"needs_two"`, `"no_area"`, `"out_of_range"`, `"groups"` (`count` of the
/// `of` selected objects are to blame), `"no_overlap"` or `"too_many"`. Nothing was changed by
/// anything but `"applied"`.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct StackResultView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Objects to blame for a refusal.
    pub count: u32,
    /// Objects selected.
    pub of: u32,
    /// Objects replaced or removed.
    pub removed: u32,
    /// Objects made.
    pub pieces: u32,
    /// Objects covered completely and removed (Flatten).
    pub hidden: u32,
    /// A trimmed object has a stroke that now follows the trim (Flatten).
    pub strokes_follow: bool,
    /// Open outlines closed with a straight line.
    pub closed_open: u32,
    /// Open outlines that enclosed no area and were removed.
    pub removed_open: u32,
}

impl From<StackOutcome> for StackResultView {
    fn from(outcome: StackOutcome) -> Self {
        let view = |kind: &str, blamed: usize, of: usize| Self {
            kind: kind.to_string(),
            count: count(blamed),
            of: count(of),
            ..Self::default()
        };
        match outcome {
            StackOutcome::Ignored => view("ignored", 0, 0),
            StackOutcome::Applied {
                operands,
                removed,
                pieces,
                hidden,
                strokes_follow,
                closed_open,
                removed_open,
            } => Self {
                removed: count(removed),
                pieces: count(pieces),
                hidden: count(hidden),
                strokes_follow,
                closed_open: count(closed_open),
                removed_open: count(removed_open),
                ..view("applied", 0, operands)
            },
            StackOutcome::Refused(refusal) => match refusal {
                StackRefusal::Operands(BooleanRefusal::NeedsTwo) => view("needs_two", 0, 0),
                StackRefusal::Operands(BooleanRefusal::NoArea { offenders, of }) => {
                    view("no_area", offenders.len(), of)
                }
                StackRefusal::Operands(BooleanRefusal::OutOfRange { offenders, of }) => {
                    view("out_of_range", offenders.len(), of)
                }
                StackRefusal::Operands(BooleanRefusal::Groups { offenders }) => {
                    view("groups", offenders.len(), offenders.len())
                }
                // The kernel of Fracture and Flatten never reports an empty result.
                StackRefusal::Operands(BooleanRefusal::Empty) | StackRefusal::NoOverlap => {
                    view("no_overlap", 0, 0)
                }
                StackRefusal::TooManyPieces => view("too_many", 0, 0),
                StackRefusal::Operands(BooleanRefusal::Texts(texts)) => view(
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
    /// Fracture on the selected objects, in one commit.
    pub fn apply_fracture(&mut self) -> StackResultView {
        if self.session.refuse_images() {
            return StackResultView {
                kind: "images".to_owned(),
                ..StackResultView::default()
            };
        }
        self.session.apply_fracture().into()
    }

    /// Flatten on the selected objects, in one commit.
    pub fn apply_flatten(&mut self) -> StackResultView {
        if self.session.refuse_images() {
            return StackResultView {
                kind: "images".to_owned(),
                ..StackResultView::default()
            };
        }
        self.session.apply_flatten().into()
    }
}
