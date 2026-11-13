//! Thin host bridge of the Test grid tool (`specs/0112-test-grid-and-materials` criteria 1 to
//! 6, 11): the press, the bar's commits and its view; strings and scalars only (ADR 0001 §5). A
//! command's text result is its cause or refusal, empty when taken. The Lasered test hook is not
//! bound here (ADR 0057 §5).

use curvyo_document_core::Point;
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// The text of a refusal, empty when the command was taken.
fn text_of<T>(result: Result<T, String>) -> String {
    result.err().unwrap_or_default()
}

#[wasm_bindgen]
impl WasmSession {
    /// Whether the Test grid tool is offered (a laser page).
    #[must_use]
    pub fn test_grid_tool_available(&self) -> bool {
        self.session.test_grid_tool_available()
    }

    /// A press of the tool at `x`, `y` in document mm; Enter places at the first free spot with
    /// `at_free_spot`. The refusal, empty when placed or selected.
    pub fn test_grid_press(&mut self, x: f64, y: f64, at_free_spot: bool) -> String {
        let point = (!at_free_spot).then(|| Point::new(x, y));
        text_of(self.session.test_grid_press(point))
    }

    /// Whether the Select bar shows the Test grid bar for a whole test grid (UX G3).
    #[must_use]
    pub fn select_bar_hides_grid_commands(&self) -> bool {
        self.session.select_bar_hides_grid_commands()
    }

    /// Types `typed` into the bar's field `field`.
    pub fn set_test_grid_field(&mut self, field: &str, typed: &str) -> String {
        text_of(self.session.set_test_grid_field(field, typed))
    }

    /// The parameter switch of `side` (`columns`, `rows`).
    pub fn set_test_grid_axis(&mut self, side: &str, param: &str) -> String {
        text_of(self.session.set_test_grid_axis(side, param))
    }

    /// The Kind segments.
    pub fn set_test_grid_kind(&mut self, kind: &str) -> String {
        text_of(self.session.set_test_grid_kind(kind))
    }

    /// The Crosshatch toggle.
    pub fn set_test_grid_crosshatch(&mut self, on: bool) -> String {
        text_of(self.session.set_test_grid_crosshatch(on))
    }

    /// The material dropdown (`id` empty: no material).
    pub fn pick_test_grid_material(&mut self, id: &str) -> String {
        text_of(
            self.session
                .pick_test_grid_material((!id.is_empty()).then_some(id)),
        )
    }

    /// Random bytes for new test ids (`crypto.getRandomValues`).
    pub fn feed_test_grid_random(&mut self, bytes: &[u8]) {
        self.session.feed_test_grid_random(bytes);
    }

    /// The bar as JSON, `null` when none shows.
    pub fn test_grid_view_json(&self) -> String {
        self.session.test_grid_view_json()
    }

    /// The refusal of the last press, `None` when it placed.
    pub fn test_grid_notice(&self) -> Option<String> {
        self.session.test_grid_notice()
    }

    /// The sr-only status ("New test id 8PL4D."), once.
    pub fn take_test_grid_status(&mut self) -> Option<String> {
        self.session.take_test_grid_status()
    }
}
