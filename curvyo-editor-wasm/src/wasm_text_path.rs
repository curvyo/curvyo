//! The text on path `wasm-bindgen` surface (`specs/0080-text-on-path` slice 1): the On path
//! button and pick mode, the entry box of the position handle, Release and the announcement.
//! Pass-throughs to `Session`; strings and scalars only (ADR 0001 §5).

use curvyo_document_core::{LineOnPath, PathSide};
use wasm_bindgen::prelude::*;

use crate::session::{TextPathEntryView, TextPathPress, TextPathView};
use crate::wasm_api::WasmSession;

fn view_json(view: &TextPathView) -> String {
    serde_json::json!({
        "texts": view.texts,
        "picking": view.picking,
        "onPath": view.on_path,
        "vertical": view.vertical,
    })
    .to_string()
}

const fn side_code(side: PathSide) -> &'static str {
    match side {
        PathSide::Above => "above",
        PathSide::Below => "below",
    }
}

const fn line_code(line: LineOnPath) -> &'static str {
    match line {
        LineOnPath::Baseline => "baseline",
        LineOnPath::TopEdge => "top",
    }
}

fn entry_json(entry: &TextPathEntryView, (x, y): (f64, f64)) -> String {
    serde_json::json!({
        "handle": { "x": x, "y": y },
        "side": side_code(entry.side),
        "line": line_code(entry.line),
        "position": entry.position,
        "degrees": entry.degrees,
        "distance": entry.distance_mm,
    })
    .to_string()
}

#[wasm_bindgen]
impl WasmSession {
    /// The On path button: `"ignored"`, `"picking"`, `"ended"`, `"put"` or `"locked"`.
    pub fn text_path_press(&mut self) -> String {
        match self.session.text_path_press() {
            TextPathPress::Ignored => "ignored",
            TextPathPress::Picking => "picking",
            TextPathPress::Ended => "ended",
            TextPathPress::Put => "put",
            TextPathPress::Locked => "locked",
        }
        .to_owned()
    }

    /// Whether pick mode runs: the pick cursor and the pressed button.
    #[must_use]
    pub fn text_path_picking(&self) -> bool {
        self.session.text_path_picking()
    }

    /// A right click or Escape in pick mode: it ends without a change. Returns whether it ran.
    pub fn cancel_text_path_pick(&mut self) -> bool {
        self.session.cancel_text_path_pick()
    }

    /// What the On path button and the row-2 dot show, as JSON: `{"texts", "picking",
    /// "onPath", "vertical"}`.
    #[must_use]
    pub fn text_path_view_json(&self) -> String {
        view_json(&self.session.text_path_view())
    }

    /// The open entry box of the position handle as JSON, or an empty string: `{"handle": {x,
    /// y} (canvas px), "side": "above" | "below", "line": "baseline" | "top", "position",
    /// "degrees", "distance"}`.
    #[must_use]
    pub fn text_path_entry_json(&self) -> String {
        self.session
            .text_path_entry()
            .map_or_else(String::new, |entry| {
                let at = self.session.view().document_to_screen(entry.handle);
                entry_json(&entry, at)
            })
    }

    /// A Side button in the entry box: `"above"` or `"below"`; previewed.
    pub fn text_path_entry_side(&mut self, code: &str) {
        let side = match code {
            "above" => PathSide::Above,
            "below" => PathSide::Below,
            _ => return,
        };
        self.session.text_path_entry_side(side);
    }

    /// A Line on path button in the entry box: `"baseline"` or `"top"`; previewed.
    pub fn text_path_entry_line(&mut self, code: &str) {
        let line = match code {
            "baseline" => LineOnPath::Baseline,
            "top" => LineOnPath::TopEdge,
            _ => return,
        };
        self.session.text_path_entry_line(line);
    }

    /// Mirror in the entry box; previewed.
    pub fn text_path_entry_mirror(&mut self) {
        self.session.text_path_entry_mirror();
    }

    /// Enter in the entry box: `"committed"`, `"unchanged"` or `"invalid:<field>"`.
    pub fn commit_text_path_entry(&mut self, position: &str, distance: &str) -> String {
        self.session.commit_text_path_entry(position, distance)
    }

    /// Escape in the entry box: it closes without a change.
    pub fn cancel_text_path_entry(&mut self) {
        self.session.cancel_text_path_entry();
    }

    /// Release from path.
    pub fn release_text_path(&mut self) {
        self.session.release_text_path();
    }

    /// The polite announcement after a put ("Put on path"), or an empty string.
    pub fn take_text_path_announcement(&mut self) -> String {
        self.session
            .take_text_path_announcement()
            .unwrap_or_default()
            .to_owned()
    }
}
