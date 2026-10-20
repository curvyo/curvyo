//! The `wasm-bindgen` surface of Weld (`specs/0068-laserable-lettering`): the command of the
//! Select bar's Weld group and the range of its Gap field. Codes and counts only (the sentences
//! are the host's, ADR 0001 §5); every method a pass-through to `Session` or `curvyo-ui-core`.

use curvyo_ui_core::{GapClamp, WeldRefusal, clamp_weld_gap};
use wasm_bindgen::prelude::*;

use crate::session::WeldOutcome;
use crate::wasm_api::WasmSession;

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What `apply_weld` did. `kind` is `"ignored"`, `"applied"` (`of` selected objects became one
/// path, `compound` or not, with `pieces` pieces and the notice's flags) or a refusal:
/// `"no_area"`, `"out_of_range"` (`count` of the `of` selected objects are to blame),
/// `"missing_fonts"` or `"fonts_loading"` (`count` texts, `face` when one face is missing).
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct WeldResultView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Objects or texts to blame for a refusal.
    pub count: u32,
    /// Objects selected.
    pub of: u32,
    /// Every selected object was a text.
    pub texts_only: bool,
    /// The result is a compound path.
    pub compound: bool,
    /// Pieces of the result.
    pub pieces: u32,
    /// A dashed open outline was welded as solid.
    pub dashed: bool,
    /// Open outlines closed with a straight line.
    pub closed_open: u32,
    /// Open outlines that enclosed no area and were removed.
    pub removed_open: u32,
    /// The operands' styles differed.
    pub mixed_styles: bool,
    /// The one missing face of a refusal of texts, else empty.
    #[wasm_bindgen(getter_with_clone)]
    pub face: String,
}

impl From<WeldOutcome> for WeldResultView {
    fn from(outcome: WeldOutcome) -> Self {
        let view = |kind: &str, blamed: usize, of: usize| Self {
            kind: kind.to_owned(),
            count: count(blamed),
            of: count(of),
            ..Self::default()
        };
        match outcome {
            WeldOutcome::Ignored => view("ignored", 0, 0),
            WeldOutcome::Applied {
                objects,
                texts_only,
                compound,
                pieces,
                dashed,
                closed_open,
                removed_open,
                mixed_styles,
            } => Self {
                texts_only,
                compound,
                pieces: count(pieces),
                dashed,
                closed_open: count(closed_open),
                removed_open: count(removed_open),
                mixed_styles,
                ..view("applied", 0, objects)
            },
            WeldOutcome::Refused(WeldRefusal::NoArea { of, .. }) => view("no_area", of, of),
            WeldOutcome::Refused(WeldRefusal::OutOfRange { objects, of, .. }) => {
                view("out_of_range", objects, of)
            }
            WeldOutcome::Refused(WeldRefusal::Texts(texts)) => Self {
                face: match texts.missing_faces.as_slice() {
                    [one] => one.clone(),
                    _ => String::new(),
                },
                ..view(
                    crate::wasm_text::refusal_kind(&texts),
                    texts.offenders.len(),
                    0,
                )
            },
        }
    }
}

/// A typed Gap in range: `mm` and `clamp`, `""`, `"largest"` or `"smallest"`.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WeldGapView {
    /// The Gap in mm, within 0 to 1.
    pub mm: f64,
    /// What the clamp did.
    #[wasm_bindgen(getter_with_clone)]
    pub clamp: String,
}

/// `mm` within the Gap's range, and whether it was moved there (criterion 2).
#[wasm_bindgen]
#[must_use]
pub fn weld_gap_in_range(mm: f64) -> WeldGapView {
    let (gap, clamp) = clamp_weld_gap(mm);
    WeldGapView {
        mm: gap.as_mm(),
        clamp: match clamp {
            GapClamp::InRange => "",
            GapClamp::Largest => "largest",
            GapClamp::Smallest => "smallest",
        }
        .to_owned(),
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Weld on the selected objects at the gap `gap_mm`, in one commit.
    pub fn apply_weld(&mut self, gap_mm: f64) -> WeldResultView {
        self.session.apply_weld(gap_mm).into()
    }
}
