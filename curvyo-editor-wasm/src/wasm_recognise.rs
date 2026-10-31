//! The `wasm-bindgen` surface of Recognise text (`specs/0088-trace-recognition`): availability,
//! the face sources readout, start, step, stop and the guessed letters, as codes, counts and JSON;
//! the sentences are the host's (ADR 0001 §5).

use curvyo_text_core::Mm;
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// The family names of a JSON array of strings; anything else is none.
fn favourites(json: &str) -> Vec<String> {
    serde_json::from_str(json).unwrap_or_default()
}

#[wasm_bindgen]
impl WasmSession {
    /// Whether the Select bar shows Recognise text (UX notes §1).
    #[must_use]
    pub fn recognise_available(&self) -> bool {
        self.session.recognise_available()
    }

    /// How many face sources have a face to search, for the readout "5 face sources"
    /// (criterion 10); `favourites` is the JSON array of starred family names.
    #[must_use]
    pub fn recognise_sources(&self, favourites_json: &str) -> u32 {
        let tiers = self.session.recognise_tiers(&favourites(favourites_json));
        u32::try_from(tiers.sources()).unwrap_or(u32::MAX)
    }

    /// Starts a run over the selection at `tolerance_mm` (criterion 6); false when nothing can
    /// be read.
    pub fn recognise_start(&mut self, tolerance_mm: f64, favourites_json: &str) -> bool {
        self.session
            .recognise_start(Mm(tolerance_mm), &favourites(favourites_json))
    }

    /// One step of the run, as the JSON of `RecogniseOutcome::to_json`.
    pub fn recognise_step(&mut self) -> String {
        self.session.recognise_step().to_json()
    }

    /// Stops the run in flight; nothing was changed.
    pub fn recognise_stop(&mut self) {
        self.session.recognise_stop();
    }

    /// The guessed characters of the Glyph table's subject while the record holds them
    /// (criterion 7), as one string.
    #[must_use]
    pub fn recognise_guessed_chars(&self) -> String {
        self.session.recognise_guessed_chars().into_iter().collect()
    }
}
