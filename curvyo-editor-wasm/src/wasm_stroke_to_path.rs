//! The `wasm-bindgen` surface of Stroke to path (`specs/0085-stroke-to-path`): the command and
//! its outcome as codes and counts (the sentences are the host's, ADR 0001 §5); the button's
//! enabled state is in the Select bar view.

use curvyo_ui_core::StrokeToPathRefusal;
use wasm_bindgen::prelude::*;

use crate::session::StrokeToPathOutcome;
use crate::wasm_api::WasmSession;

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What `apply_stroke_to_path` did. `kind` is `"ignored"`, `"applied"` (with the counts of the
/// notice, criterion 22) or a refusal: `"out_of_range"` (`count` of the `of` selected objects are
/// to blame), `"missing_fonts"` or `"fonts_loading"` (`count` texts, `face` the one missing face).
/// Nothing was changed by anything but `"applied"`.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct StrokeToPathView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Objects or texts to blame for a refusal.
    pub count: u32,
    /// Objects selected (a group is one).
    pub of: u32,
    /// Strokes that became filled outlines.
    pub strokes: u32,
    /// Selected objects without a stroke, left as they are.
    pub unstroked: u32,
    /// Texts converted to paths.
    pub texts: u32,
    /// Clones unlinked.
    pub unlinked: u32,
    /// The one missing face of a refusal of texts, else empty.
    #[wasm_bindgen(getter_with_clone)]
    pub face: String,
}

#[wasm_bindgen]
impl WasmSession {
    /// Stroke to path on the selected objects, in one commit (criteria 2, 19 to 22).
    pub fn apply_stroke_to_path(&mut self) -> StrokeToPathView {
        let of = count(self.session.selected_object_count());
        let face = self.session.text_refusal_face();
        match self.session.apply_stroke_to_path() {
            StrokeToPathOutcome::Ignored => StrokeToPathView {
                kind: "ignored".to_owned(),
                ..StrokeToPathView::default()
            },
            StrokeToPathOutcome::Applied {
                strokes,
                unstroked,
                texts,
                unlinked,
            } => StrokeToPathView {
                kind: "applied".to_owned(),
                of,
                strokes: count(strokes),
                unstroked: count(unstroked),
                texts: count(texts),
                unlinked: count(unlinked),
                ..StrokeToPathView::default()
            },
            StrokeToPathOutcome::Refused(StrokeToPathRefusal::OutOfRange { offenders }) => {
                StrokeToPathView {
                    kind: "out_of_range".to_owned(),
                    count: count(offenders.len()),
                    of,
                    ..StrokeToPathView::default()
                }
            }
            StrokeToPathOutcome::Refused(StrokeToPathRefusal::Texts(texts)) => StrokeToPathView {
                kind: crate::wasm_text::refusal_kind(&texts).to_owned(),
                count: count(texts.offenders.len()),
                of,
                face,
                ..StrokeToPathView::default()
            },
        }
    }
}
