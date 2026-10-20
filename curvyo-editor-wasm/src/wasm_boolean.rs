//! The boolean command's `wasm-bindgen` surface (`specs/0016-boolean-operations`): what the
//! rail section shows, the command, and the refusal outline. A further `impl WasmSession`
//! block; codes and counts only (the sentences are the host's, ADR 0001 §5), every method a
//! pass-through to `Session`.

use curvyo_ui_core::{BooleanAvailability, BooleanOp, BooleanRefusal};
use wasm_bindgen::prelude::*;

use crate::session::BooleanOutcome;
use crate::wasm_api::WasmSession;

/// `wasm-bindgen`'s JS-facing mirror of [`curvyo_ui_core::BooleanAvailability`]: `needs_two`
/// dimmed; else enabled, with `open` open outlines that an operation closes with a straight line
/// (0 when there are none, `specs/0054-combine-open-paths` criterion 23).
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct BooleanAvailabilityView {
    /// Fewer than two objects are selected, or the Select tool is not active.
    pub needs_two: bool,
    /// How many open outlines of two or more nodes the selected objects hold.
    pub open: u32,
}

impl From<BooleanAvailability> for BooleanAvailabilityView {
    fn from(availability: BooleanAvailability) -> Self {
        match availability {
            BooleanAvailability::NeedsTwo => Self {
                needs_two: true,
                open: 0,
            },
            BooleanAvailability::ClosesOpen { outlines } => Self {
                needs_two: false,
                open: count(outlines),
            },
            BooleanAvailability::Ready => Self {
                needs_two: false,
                open: 0,
            },
        }
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What `apply_boolean` did. `kind` is `"ignored"`, `"applied"` (`count` objects became one;
/// `compound` says whether it is a compound path; `closed_open` open outlines were closed with a
/// straight line, `removed_open` enclosed no area and were removed) or a refusal: `"needs_two"`,
/// `"no_area"`, `"groups"`, `"out_of_range"` (`count` of the `of` selected objects are to blame) or
/// `"empty"`. Nothing was changed by anything but `"applied"`.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct BooleanResultView {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// Objects replaced, or objects to blame.
    pub count: u32,
    /// Objects selected.
    pub of: u32,
    /// The result is a compound path.
    pub compound: bool,
    /// Open outlines closed with a straight line.
    pub closed_open: u32,
    /// Open outlines that enclosed no area and were removed.
    pub removed_open: u32,
}

impl From<BooleanOutcome> for BooleanResultView {
    fn from(outcome: BooleanOutcome) -> Self {
        let view = |kind: &str, count: usize, of: usize| Self {
            kind: kind.to_string(),
            count: self::count(count),
            of: self::count(of),
            ..Self::default()
        };
        match outcome {
            BooleanOutcome::Ignored => view("ignored", 0, 0),
            BooleanOutcome::Applied {
                operands,
                compound,
                closed_open,
                removed_open,
            } => Self {
                compound,
                closed_open: count(closed_open),
                removed_open: count(removed_open),
                ..view("applied", operands, operands)
            },
            BooleanOutcome::Refused(refusal) => match refusal {
                BooleanRefusal::NeedsTwo => view("needs_two", 0, 0),
                BooleanRefusal::NoArea { offenders, of } => view("no_area", offenders.len(), of),
                BooleanRefusal::OutOfRange { offenders, of } => {
                    view("out_of_range", offenders.len(), of)
                }
                BooleanRefusal::Groups { offenders } => {
                    view("groups", offenders.len(), offenders.len())
                }
                BooleanRefusal::Empty => view("empty", 0, 0),
                BooleanRefusal::Texts(texts) => view(
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
    /// What the Boolean toolbox of the tool rail shows. Call it with every other state read.
    #[must_use]
    pub fn boolean_availability(&self) -> BooleanAvailabilityView {
        self.session.boolean_availability().into()
    }

    /// Runs one operation (`"union"`, `"difference"`, `"intersection"`, `"exclusion"` or
    /// `"reverse_difference"`) on the selected objects, in one commit.
    ///
    /// # Errors
    /// A `JsValue` if `op` is none of those.
    pub fn apply_boolean(&mut self, op: &str) -> Result<BooleanResultView, JsValue> {
        let op = BooleanOp::from_code(op)
            .ok_or_else(|| JsValue::from_str(&format!("unknown boolean operation: {op}")))?;
        Ok(self.session.apply_boolean(op).into())
    }

    /// Removes the red outline of a refusal (its notice ran out, or the next action).
    pub fn clear_boolean_refusal(&mut self) {
        self.session.clear_boolean_refusal();
    }
}
