//! The text layout controls' `wasm-bindgen` surface (`specs/0079-text-layout-controls` slice 1):
//! the Alignment group and the four fields of the text layout block. Pass-throughs to `Session`;
//! strings and scalars only (ADR 0001 §5).

use curvyo_document_core::TextAlign;
use curvyo_ui_core::{Grid, LayoutField, LayoutFieldView};
use wasm_bindgen::prelude::*;

use crate::session::LayoutOutcome;
use crate::wasm_api::WasmSession;

/// The host's code of an alignment; empty for `None` (the subject texts differ).
pub(crate) const fn align_code(align: Option<TextAlign>) -> &'static str {
    match align {
        Some(TextAlign::Left) => "left",
        Some(TextAlign::Centre) => "centre",
        Some(TextAlign::Right) => "right",
        Some(TextAlign::Justify) => "justify",
        None => "",
    }
}

fn align_of(code: &str) -> Option<TextAlign> {
    match code {
        "left" => Some(TextAlign::Left),
        "centre" => Some(TextAlign::Centre),
        "right" => Some(TextAlign::Right),
        "justify" => Some(TextAlign::Justify),
        _ => None,
    }
}

fn field_json(view: &LayoutFieldView) -> serde_json::Value {
    serde_json::json!({
        "text": view.text,
        "value": view.value,
        "mixed": view.mixed,
        "auto": view.auto,
        "barFrom": view.bar_from,
        "barTo": view.bar_to,
        "zero": view.zero,
        "dragBase": view.drag_base,
        "dragMin": view.drag_min,
        "dragLimit": view.drag_limit,
        "trackMin": view.track_min,
        "trackMax": view.track_max,
        "resettable": view.resettable,
    })
}

fn outcome_json(outcome: LayoutOutcome) -> String {
    match outcome {
        LayoutOutcome::Set => serde_json::json!({ "kind": "set" }),
        LayoutOutcome::Unchanged => serde_json::json!({ "kind": "unchanged" }),
        LayoutOutcome::Invalid => serde_json::json!({ "kind": "invalid" }),
        LayoutOutcome::Clamped(top, mm) => {
            serde_json::json!({ "kind": if top { "max" } else { "min" }, "mm": mm })
        }
        LayoutOutcome::Limited { limited, of } => {
            serde_json::json!({ "kind": "limited", "limited": limited, "of": of })
        }
    }
    .to_string()
}

#[wasm_bindgen]
impl WasmSession {
    /// A press in the Alignment group: `"left"`, `"centre"`, `"right"` or `"justify"`.
    pub fn set_text_alignment(&mut self, code: &str) {
        if let Some(align) = align_of(code) {
            self.session.set_text_alignment(align);
        }
    }

    /// The text layout block, as JSON: `null` when it is not shown, else the four fields in
    /// order (`{"text", "value", "mixed", "auto", "barFrom", "barTo", "zero", "dragBase",
    /// "dragMin", "dragLimit", "trackMin", "trackMax", "resettable"}`).
    #[must_use]
    pub fn text_layout_block_json(&self) -> String {
        self.session.text_layout_block().map_or_else(
            || "null".to_owned(),
            |views| {
                serde_json::Value::from(views.iter().map(field_json).collect::<Vec<_>>())
                    .to_string()
            },
        )
    }

    /// A tick of a drag or a click on the field `field` (`"letter-spacing"`, `"word-spacing"`,
    /// `"line-spacing"`, `"character-rotation"`) at position `p`, grid `"normal"`, `"coarse"` or
    /// `"fine"`.
    pub fn preview_text_layout(&mut self, field: &str, p: f64, grid: &str) {
        if let (Some(field), Some(grid)) = (LayoutField::from_name(field), Grid::from_name(grid)) {
            self.session.preview_text_layout(field, p, grid);
        }
    }

    /// An arrow key on a field: `steps` steps, previewed until the commit.
    pub fn step_text_layout(&mut self, field: &str, steps: i32, grid: &str) {
        if let (Some(field), Some(grid)) = (LayoutField::from_name(field), Grid::from_name(grid)) {
            self.session.step_text_layout(field, steps, grid);
        }
    }

    /// The release or key-up: one step; the outcome JSON (`{"kind": "set" | "unchanged" |
    /// "invalid" | "max" | "min" | "limited", "mm"?, "limited"?, "of"?}`).
    pub fn commit_text_layout_preview(&mut self) -> String {
        outcome_json(self.session.commit_text_layout_preview())
    }

    /// Escape during a drag: nothing is written.
    pub fn cancel_text_layout_preview(&mut self) {
        self.session.cancel_text_layout_preview();
    }

    /// The reset slot or Ctrl+Backspace of a field; the outcome JSON.
    pub fn reset_text_layout(&mut self, field: &str) -> String {
        LayoutField::from_name(field).map_or_else(
            || outcome_json(LayoutOutcome::Unchanged),
            |field| outcome_json(self.session.reset_text_layout(field)),
        )
    }

    /// A typed value of a field (mm, or degrees for the rotation); the outcome JSON.
    pub fn set_text_layout_text(&mut self, field: &str, text: &str) -> String {
        LayoutField::from_name(field).map_or_else(
            || outcome_json(LayoutOutcome::Invalid),
            |field| outcome_json(self.session.set_text_layout_text(field, text)),
        )
    }
}
