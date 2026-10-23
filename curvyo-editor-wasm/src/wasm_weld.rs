//! The `wasm-bindgen` surface of Weld (`specs/0068-laserable-lettering`), the eighth command of
//! the Boolean card: its own availability and the command. Codes and counts only (the sentences
//! are the host's, ADR 0001 §5); every method a pass-through to `Session`.

use curvyo_ui_core::{WeldAvailability, WeldRefusal};
use wasm_bindgen::prelude::*;

use crate::session::WeldOutcome;
use crate::wasm_api::WasmSession;

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What the Weld button shows: `needs_one` dimmed (nothing selected, or not the Select tool);
/// else enabled, with `open` open outlines a press closes with a straight line.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct WeldAvailabilityView {
    /// Nothing is selected, or the Select tool is not active.
    pub needs_one: bool,
    /// Open outlines a press closes with a straight line.
    pub open: u32,
}

impl From<WeldAvailability> for WeldAvailabilityView {
    fn from(availability: WeldAvailability) -> Self {
        let (needs_one, open) = match availability {
            WeldAvailability::NeedsOne => (true, 0),
            WeldAvailability::ClosesOpen { outlines } => (false, count(outlines)),
            WeldAvailability::Ready => (false, 0),
        };
        Self { needs_one, open }
    }
}

/// What `apply_weld` did. `kind` is `"ignored"`, `"applied"` (`of` selected objects became one
/// path, `compound` or not, `length_text` of line, and the notice's counts and flags) or a refusal:
/// `"no_area"`, `"out_of_range"` (`count` of the `of` selected objects are to blame),
/// `"missing_fonts"`, `"fonts_loading"` or `"single_line_texts"` (`count` texts). Nothing was changed by anything but
/// `"applied"`.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)] // one flag per sentence of the notice, read by name
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
    /// The result's total line length in the display unit with its symbol, as "140.0 mm".
    #[wasm_bindgen(getter_with_clone)]
    pub length_text: String,
    /// Shapes covered completely and removed.
    pub hidden: u32,
    /// Open outlines closed with a straight line.
    pub closed_open: u32,
    /// Open outlines that enclosed no area and were removed.
    pub removed_open: u32,
    /// The operands' styles differed.
    pub mixed_styles: bool,
    /// Fill was switched off.
    pub fill_switched_off: bool,
    /// Stroke was switched on.
    pub stroke_switched_on: bool,
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
                length_text,
                hidden,
                closed_open,
                removed_open,
                mixed_styles,
                fill_switched_off,
                stroke_switched_on,
                ..
            } => Self {
                texts_only,
                compound,
                length_text,
                hidden: count(hidden),
                closed_open: count(closed_open),
                removed_open: count(removed_open),
                mixed_styles,
                fill_switched_off,
                stroke_switched_on,
                ..view("applied", 0, objects)
            },
            WeldOutcome::Refused(WeldRefusal::NoArea { of, .. }) => view("no_area", of, of),
            WeldOutcome::Refused(WeldRefusal::OutOfRange { objects, of, .. }) => {
                view("out_of_range", objects, of)
            }
            WeldOutcome::Refused(WeldRefusal::Texts(texts)) => view(
                crate::wasm_text::refusal_kind(&texts),
                texts.offenders.len(),
                0,
            ),
        }
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// What the Weld button of the Boolean card shows. Call it with every other state read.
    #[must_use]
    pub fn weld_availability(&self) -> WeldAvailabilityView {
        self.session.weld_availability().into()
    }

    /// Weld on the selected objects, in one commit.
    pub fn apply_weld(&mut self) -> WeldResultView {
        self.session.apply_weld().into()
    }
}
