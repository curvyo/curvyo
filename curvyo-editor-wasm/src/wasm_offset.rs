//! The Offset tool's `wasm-bindgen` surface (`specs/0038-path-offset`): what the Offset entry
//! chip, the readout chip and the notice show, the remembered values, the chip's Enter and the
//! drag's cancel. A further
//! `impl WasmSession` block; codes and numbers only (the sentences are the host's, ADR 0001 §5),
//! every method a pass-through to `Session`.

use curvyo_ui_core::{
    OFFSET_MAX_MM, OFFSET_MIN_MM, OffsetDirection, OffsetJoin, OffsetRefusal,
    offset_direction_from_code, offset_join_from_code, offset_result_from_code,
};
use wasm_bindgen::prelude::*;

use crate::session::{DragStop, OffsetOutcome};
use crate::wasm_api::WasmSession;

/// What the Offset entry chip, the readout and the handles show.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct OffsetViewJs {
    /// Operands selected (groups not counted).
    pub operands: u32,
    /// The selection holds a group.
    pub has_group: bool,
    /// The handles are drawn.
    pub handles: bool,
    /// A drag is in flight.
    pub dragging: bool,
    /// The signed distance shown: positive outward.
    pub distance_mm: f64,
    /// Both sides.
    pub both: bool,
    /// The release or Apply changes the original.
    pub change: bool,
    /// `""` or `"nothing_left"`.
    #[wasm_bindgen(getter_with_clone)]
    pub stop: String,
    /// Operands whose inset vanishes in the preview.
    pub vanished: u32,
    /// Operands the preview covers.
    pub of: u32,
    /// The preview is replaced by the grown box.
    pub large: bool,
    /// `""` or the refusal code of the preview (see [`OffsetOutcomeJs`]).
    #[wasm_bindgen(getter_with_clone)]
    pub refusal: String,
    /// The objects the preview's refusal blames, and how many are selected.
    pub refusal_count: u32,
    /// How many objects the refusal counts against.
    pub refusal_of: u32,
    /// The remembered Distance in millimetres.
    pub values_distance_mm: f64,
    /// `"out"`, `"in"` or `"both"`.
    #[wasm_bindgen(getter_with_clone)]
    pub direction: String,
    /// The remembered Corners: `"round"`, `"miter"` or `"bevel"`.
    #[wasm_bindgen(getter_with_clone)]
    pub join: String,
    /// The corner style the preview and the next drag use, for the readout's first line
    /// (criteria 15, 56).
    #[wasm_bindgen(getter_with_clone)]
    pub drag_join: String,
    /// `""` or the corner style a double-click on a corner handle set for the next drag
    /// (criterion 56).
    #[wasm_bindgen(getter_with_clone)]
    pub pending_join: String,
    /// `"new"` or `"change"`: the remembered Mode, as Ctrl inverts it during a drag (criterion 19).
    #[wasm_bindgen(getter_with_clone)]
    pub result: String,
    /// The Offset entry chip is open (criterion 53).
    pub chip_open: bool,
    /// The smallest Distance.
    pub min_mm: f64,
    /// The largest Distance.
    pub max_mm: f64,
}

/// What a release or the chip's Enter did. `kind` is `"ignored"`, `"applied"` (`added` new,
/// `changed` changed, at `distance_mm` in `direction`, `vanished` of `of` gone), `"groups"`
/// (`count` leaves outlined), or a refusal: `"out_of_range"`, `"no_area"` (`count` of `of` to
/// blame) or `"empty"` (at `distance_mm`).
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct OffsetOutcomeJs {
    /// The outcome code.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// New objects.
    pub added: u32,
    /// Changed originals.
    pub changed: u32,
    /// Objects to blame.
    pub count: u32,
    /// Operands or selected objects.
    pub of: u32,
    /// Operands whose inset vanished.
    pub vanished: u32,
    /// The unsigned distance in millimetres.
    pub distance_mm: f64,
    /// `"out"`, `"in"` or `"both"`.
    #[wasm_bindgen(getter_with_clone)]
    pub direction: String,
}

/// Where the Offset entry chip is placed from, in canvas-relative CSS pixels (`ux-rework.md` 2.4):
/// the center handle and the box around the operand box and the standing preview.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct OffsetChipAnchorJs {
    /// The center handle.
    pub center_x: f64,
    /// The center handle.
    pub center_y: f64,
    /// The anchor box's left edge.
    pub left: f64,
    /// The anchor box's top edge.
    pub top: f64,
    /// The anchor box's right edge.
    pub right: f64,
    /// The anchor box's bottom edge.
    pub bottom: f64,
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The code, blamed count and selected count of a refusal.
fn refusal_code(refusal: &OffsetRefusal) -> (&'static str, usize, usize) {
    match refusal {
        OffsetRefusal::NoOperands => ("no_operands", 0, 0),
        OffsetRefusal::ZeroDistance => ("zero", 0, 0),
        OffsetRefusal::OutOfRange { offenders, of } => ("out_of_range", offenders.len(), *of),
        OffsetRefusal::NoArea { offenders, of } => ("no_area", offenders.len(), *of),
        OffsetRefusal::Empty => ("empty", 0, 0),
    }
}

const fn direction_code(direction: OffsetDirection) -> &'static str {
    match direction {
        OffsetDirection::Out => "out",
        OffsetDirection::In => "in",
        OffsetDirection::Both => "both",
    }
}

const fn join_code(join: OffsetJoin) -> &'static str {
    match join {
        OffsetJoin::Round => "round",
        OffsetJoin::Miter => "miter",
        OffsetJoin::Bevel => "bevel",
    }
}

impl From<OffsetOutcome> for OffsetOutcomeJs {
    fn from(outcome: OffsetOutcome) -> Self {
        let mut view = Self {
            kind: "ignored".to_string(),
            added: 0,
            changed: 0,
            count: 0,
            of: 0,
            vanished: 0,
            distance_mm: 0.0,
            direction: String::new(),
        };
        match outcome {
            OffsetOutcome::Ignored => {}
            OffsetOutcome::Applied {
                added,
                changed,
                distance_mm,
                direction,
                vanished,
                of,
            } => {
                view.kind = "applied".to_string();
                view.added = count(added);
                view.changed = count(changed);
                view.distance_mm = distance_mm;
                view.direction = direction_code(direction).to_string();
                view.vanished = count(vanished);
                view.of = count(of);
            }
            OffsetOutcome::RefusedGroups { leaves } => {
                view.kind = "groups".to_string();
                view.count = count(leaves);
            }
            OffsetOutcome::Refused {
                refusal,
                distance_mm,
            } => {
                let (code, n, of) = refusal_code(&refusal);
                view.kind = code.to_string();
                view.count = count(n);
                view.of = count(of);
                view.distance_mm = distance_mm;
            }
        }
        view
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Everything the Offset entry chip, the readout and the handles show. Read it with every
    /// other state read.
    #[must_use]
    pub fn offset_view(&self) -> OffsetViewJs {
        let view = self.session.offset_view();
        let values = self.session.offset_values();
        let refusal = view.refusal.as_ref().map_or(("", 0, 0), refusal_code);
        OffsetViewJs {
            operands: count(view.operands),
            has_group: view.has_group,
            handles: view.handles,
            dragging: view.dragging,
            distance_mm: view.distance_mm,
            both: view.both,
            change: view.change,
            stop: match view.stop {
                None => "",
                Some(DragStop::NothingLeft) => "nothing_left",
            }
            .to_string(),
            vanished: count(view.vanished),
            of: count(view.of),
            large: view.large,
            refusal_count: count(refusal.1),
            refusal_of: count(refusal.2),
            refusal: view
                .refusal
                .as_ref()
                .map_or("", |refusal| refusal_code(refusal).0)
                .to_string(),
            values_distance_mm: values.distance_mm(),
            direction: direction_code(values.direction()).to_string(),
            join: join_code(values.join()).to_string(),
            drag_join: join_code(view.join).to_string(),
            pending_join: view.pending_join.map_or("", join_code).to_string(),
            result: if view.change { "change" } else { "new" }.to_string(),
            chip_open: self.session.offset_chip_open(),
            min_mm: OFFSET_MIN_MM,
            max_mm: OFFSET_MAX_MM,
        }
    }

    /// Sets the Distance in millimetres; returns whether it was within the range (it is clamped
    /// otherwise).
    pub fn set_offset_distance(&mut self, distance_mm: f64) -> bool {
        self.session.set_offset_distance(distance_mm)
    }

    /// Sets the Direction (`"out"`, `"in"`, `"both"`).
    ///
    /// # Errors
    /// A `JsValue` if the code is none of those.
    pub fn set_offset_direction(&mut self, code: &str) -> Result<(), JsValue> {
        let direction = offset_direction_from_code(code)
            .ok_or_else(|| JsValue::from_str(&format!("unknown direction: {code}")))?;
        self.session.set_offset_direction(direction);
        Ok(())
    }

    /// Sets the Corners (`"round"`, `"miter"`, `"bevel"`).
    ///
    /// # Errors
    /// A `JsValue` if the code is none of those.
    pub fn set_offset_join(&mut self, code: &str) -> Result<(), JsValue> {
        let join = offset_join_from_code(code)
            .ok_or_else(|| JsValue::from_str(&format!("unknown join: {code}")))?;
        self.session.set_offset_join(join);
        Ok(())
    }

    /// Sets the Mode (`"new"`, `"change"`).
    ///
    /// # Errors
    /// A `JsValue` if the code is none of those.
    pub fn set_offset_result(&mut self, code: &str) -> Result<(), JsValue> {
        let result = offset_result_from_code(code)
            .ok_or_else(|| JsValue::from_str(&format!("unknown result: {code}")))?;
        self.session.set_offset_result(result);
        Ok(())
    }

    /// Enter in the Offset entry chip: writes the remembered values for the selection in one
    /// commit.
    pub fn apply_offset(&mut self) -> OffsetOutcomeJs {
        self.session.apply_offset().into()
    }

    /// What the last release did, once; `"ignored"` when nothing happened since.
    pub fn take_offset_outcome(&mut self) -> OffsetOutcomeJs {
        self.session
            .take_offset_outcome()
            .unwrap_or(OffsetOutcome::Ignored)
            .into()
    }

    /// Opens the Offset entry chip (the host's route for a key it handles itself); returns whether
    /// it is open.
    pub fn open_offset_chip(&mut self) -> bool {
        self.session.open_offset_chip()
    }

    /// Closes the Offset entry chip without writing (Escape, a blur, a press outside).
    pub fn close_offset_chip(&mut self) {
        self.session.close_offset_chip();
    }

    /// Where the open chip is placed from; `None` without operands. Read it again on pan, zoom and
    /// resize.
    #[must_use]
    pub fn offset_chip_anchor(&self) -> Option<OffsetChipAnchorJs> {
        let (center, min, max) = self.session.offset_chip_anchor()?;
        let view = self.session.view();
        let (center_x, center_y) = view.document_to_screen(center);
        let (x0, y0) = view.document_to_screen(min);
        let (x1, y1) = view.document_to_screen(max);
        Some(OffsetChipAnchorJs {
            center_x,
            center_y,
            left: x0.min(x1),
            top: y0.min(y1),
            right: x0.max(x1),
            bottom: y0.max(y1),
        })
    }

    /// The other mouse button during a drag: cancels it; returns whether one was in flight.
    pub fn cancel_offset_drag(&mut self) -> bool {
        self.session.cancel_offset_drag()
    }
}
