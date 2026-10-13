//! The Offset tool's `wasm-bindgen` surface (`specs/0038-path-offset`): what the Offset bar, the
//! readout chip and the notice show, the bar's values, Apply and the drag's cancel. A further
//! `impl WasmSession` block; codes and numbers only (the sentences are the host's, ADR 0001 §5),
//! every method a pass-through to `Session`.

use curvyo_ui_core::{
    KindNote, OFFSET_MAX_MM, OFFSET_MIN_MM, OffsetCap, OffsetDirection, OffsetJoin, OffsetRefusal,
    OffsetResultChoice, PrimitiveKind, offset_cap_from_code, offset_direction_from_code,
    offset_join_from_code, offset_result_from_code,
};
use wasm_bindgen::prelude::*;

use crate::session::{DragStop, OffsetOutcome};
use crate::wasm_api::WasmSession;

/// What the Offset bar and the readout show.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct OffsetViewJs {
    /// Operands selected (groups not counted).
    pub operands: u32,
    /// An open path is selected: Ends applies.
    pub has_open: bool,
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
    /// `""`, `"nothing_left"` or `"inset_open"`.
    #[wasm_bindgen(getter_with_clone)]
    pub stop: String,
    /// Operands whose inset vanishes in the preview.
    pub vanished: u32,
    /// Operands the preview covers.
    pub of: u32,
    /// The preview is replaced by the grown box.
    pub large: bool,
    /// `""`, `"becomes_path"` or `"stays_<kind>"` (`rectangle`, `circle`, `ellipse`, `polygon`,
    /// `star`).
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// `""` or the refusal code of the preview (see [`OffsetOutcomeJs`]).
    #[wasm_bindgen(getter_with_clone)]
    pub refusal: String,
    /// The bar's Distance in millimetres.
    pub bar_distance_mm: f64,
    /// `"out"`, `"in"` or `"both"`.
    #[wasm_bindgen(getter_with_clone)]
    pub direction: String,
    /// `"round"`, `"miter"` or `"bevel"`.
    #[wasm_bindgen(getter_with_clone)]
    pub join: String,
    /// `"round"`, `"flat"` or `"square"`.
    #[wasm_bindgen(getter_with_clone)]
    pub cap: String,
    /// `"new"` or `"change"`.
    #[wasm_bindgen(getter_with_clone)]
    pub result: String,
    /// The smallest Distance.
    pub min_mm: f64,
    /// The largest Distance.
    pub max_mm: f64,
}

/// What a release or Apply did. `kind` is `"ignored"`, `"applied"` (`added` new, `changed`
/// changed, at `distance_mm` in `direction`, `vanished` of `of` gone, `open_left_out` open paths
/// left out), `"groups"` (`count` leaves outlined), or a refusal: `"inset_open"`,
/// `"out_of_range"`, `"no_area"` (`count` of `of` to blame) or `"empty"` (at `distance_mm`).
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
    /// Open paths left out of an inset.
    pub open_left_out: u32,
    /// The unsigned distance in millimetres.
    pub distance_mm: f64,
    /// `"out"`, `"in"` or `"both"`.
    #[wasm_bindgen(getter_with_clone)]
    pub direction: String,
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The code, blamed count and selected count of a refusal.
fn refusal_code(refusal: &OffsetRefusal) -> (&'static str, usize, usize) {
    match refusal {
        OffsetRefusal::NoOperands => ("no_operands", 0, 0),
        OffsetRefusal::ZeroDistance => ("zero", 0, 0),
        OffsetRefusal::InsetOfOpenPaths { offenders, of } => ("inset_open", offenders.len(), *of),
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

const fn cap_code(cap: OffsetCap) -> &'static str {
    match cap {
        OffsetCap::Round => "round",
        OffsetCap::Flat => "flat",
        OffsetCap::Square => "square",
    }
}

fn kind_code(note: Option<KindNote>) -> &'static str {
    match note {
        None => "",
        Some(KindNote::BecomesPath) => "becomes_path",
        Some(KindNote::Stays(kind)) => match kind {
            PrimitiveKind::Rectangle => "stays_rectangle",
            PrimitiveKind::Circle => "stays_circle",
            PrimitiveKind::Ellipse => "stays_ellipse",
            PrimitiveKind::Polygon => "stays_polygon",
            PrimitiveKind::Star => "stays_star",
        },
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
            open_left_out: 0,
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
                open_left_out,
            } => {
                view.kind = "applied".to_string();
                view.added = count(added);
                view.changed = count(changed);
                view.distance_mm = distance_mm;
                view.direction = direction_code(direction).to_string();
                view.vanished = count(vanished);
                view.of = count(of);
                view.open_left_out = count(open_left_out);
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
    /// Everything the Offset bar and the readout show. Read it with every other state read.
    #[must_use]
    pub fn offset_view(&self) -> OffsetViewJs {
        let view = self.session.offset_view();
        let bar = self.session.offset_bar();
        OffsetViewJs {
            operands: count(view.operands),
            has_open: view.has_open,
            has_group: view.has_group,
            handles: view.handles,
            dragging: view.dragging,
            distance_mm: view.distance_mm,
            both: view.both,
            change: view.change,
            stop: match view.stop {
                None => "",
                Some(DragStop::NothingLeft) => "nothing_left",
                Some(DragStop::InsetNeedsClosed) => "inset_open",
            }
            .to_string(),
            vanished: count(view.vanished),
            of: count(view.of),
            large: view.large,
            kind: kind_code(view.kind_note).to_string(),
            refusal: view
                .refusal
                .as_ref()
                .map_or("", |refusal| refusal_code(refusal).0)
                .to_string(),
            bar_distance_mm: bar.distance_mm(),
            direction: direction_code(bar.direction()).to_string(),
            join: join_code(bar.join()).to_string(),
            cap: cap_code(bar.cap()).to_string(),
            result: match bar.result() {
                OffsetResultChoice::NewObject => "new",
                OffsetResultChoice::ChangeOriginal => "change",
            }
            .to_string(),
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

    /// Sets the Ends (`"round"`, `"flat"`, `"square"`).
    ///
    /// # Errors
    /// A `JsValue` if the code is none of those.
    pub fn set_offset_cap(&mut self, code: &str) -> Result<(), JsValue> {
        let cap = offset_cap_from_code(code)
            .ok_or_else(|| JsValue::from_str(&format!("unknown cap: {code}")))?;
        self.session.set_offset_cap(cap);
        Ok(())
    }

    /// Sets the Result (`"new"`, `"change"`).
    ///
    /// # Errors
    /// A `JsValue` if the code is none of those.
    pub fn set_offset_result(&mut self, code: &str) -> Result<(), JsValue> {
        let result = offset_result_from_code(code)
            .ok_or_else(|| JsValue::from_str(&format!("unknown result: {code}")))?;
        self.session.set_offset_result(result);
        Ok(())
    }

    /// Focus entered (`true`) or left the Offset bar.
    pub fn set_offset_bar_focus(&mut self, focused: bool) {
        self.session.set_offset_bar_focus(focused);
    }

    /// Applies the bar's values to the selection in one commit.
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

    /// The other mouse button during a drag: cancels it; returns whether one was in flight.
    pub fn cancel_offset_drag(&mut self) -> bool {
        self.session.cancel_offset_drag()
    }
}
