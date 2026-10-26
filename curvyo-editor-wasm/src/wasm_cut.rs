//! The `wasm-bindgen` surface of Cut (`specs/0036-cut-at-crossings`): the fourth command of the
//! Path card. A further `impl WasmSession` block; codes and counts only (the sentences are the
//! host's, ADR 0001 §5), a pass-through to `Session`.

use curvyo_ui_core::CutRefusal;
use wasm_bindgen::prelude::*;

use crate::session::CutOutcome;
use crate::wasm_api::WasmSession;

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What `apply_cut` did. `kind` is `"ignored"`, `"applied"` (`count` paths became `pieces`
/// pieces) or a refusal: `"none_selected"`, `"groups"` or `"compound"` (`count` objects are to
/// blame), `"out_of_range"` (`count` of the `of` selected objects), `"no_crossing"` (`of` objects
/// selected) or
/// `"too_many"` (the cut would make `pieces` pieces). Nothing was changed by anything but
/// `"applied"`.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct CutResultView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Paths divided, or objects to blame.
    pub count: u32,
    /// Objects selected, for the out-of-range sentence.
    pub of: u32,
    /// Pieces made, or that the refused cut would have made.
    pub pieces: u32,
}

impl From<CutOutcome> for CutResultView {
    fn from(outcome: CutOutcome) -> Self {
        let view = |kind: &str, n: usize, of: usize, pieces: usize| Self {
            kind: kind.to_string(),
            count: count(n),
            of: count(of),
            pieces: count(pieces),
        };
        match outcome {
            CutOutcome::Ignored => view("ignored", 0, 0, 0),
            CutOutcome::Applied { paths, pieces } => view("applied", paths, paths, pieces),
            CutOutcome::Refused(refusal) => match refusal {
                CutRefusal::NoneSelected => view("none_selected", 0, 0, 0),
                CutRefusal::Groups { offenders } => {
                    view("groups", offenders.len(), offenders.len(), 0)
                }
                CutRefusal::Compound { offenders } => {
                    view("compound", offenders.len(), offenders.len(), 0)
                }
                CutRefusal::TextObjects { offenders } => {
                    view("texts", offenders.len(), offenders.len(), 0)
                }
                CutRefusal::OutOfRange { offenders, of } => {
                    view("out_of_range", offenders.len(), of, 0)
                }
                CutRefusal::NoCrossing => view("no_crossing", 0, 0, 0),
                CutRefusal::TooMany { pieces } => view("too_many", 0, 0, pieces),
                CutRefusal::Texts(texts) => view(
                    crate::wasm_text::refusal_kind(&texts),
                    texts.offenders.len(),
                    0,
                    0,
                ),
            },
        }
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Cuts the selected paths open where they cross, touch or overlap, in one commit. For
    /// `"no_crossing"`, `of` is the number of selected objects (the sentence says "path" or
    /// "paths"); a refusal leaves the selection as it was.
    pub fn apply_cut(&mut self) -> CutResultView {
        if self.session.refuse_images() {
            return CutResultView {
                kind: "images".to_owned(),
                ..CutResultView::default()
            };
        }
        let mut view: CutResultView = self.session.apply_cut().into();
        if view.kind == "no_crossing" {
            view.of = count(self.session.selected_object_count());
        }
        view
    }
}
