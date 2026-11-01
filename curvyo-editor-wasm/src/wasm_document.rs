//! The Properties panel's Document section and the status bar texts, as
//! `wasm-bindgen` calls (`specs/0015-document-size-and-rulers/`): what the
//! panel shows, the size fields, the display unit, Fit to content, and the
//! unit-aware status texts. A further `impl WasmSession` block; strings,
//! booleans and numbers only (ADR 0001 §5), every method a pass-through to
//! `Session`.

use curvyo_document_core::{DisplayUnit, Orientation};
use wasm_bindgen::prelude::*;

use crate::session::{
    DocumentPresetsRecord, DocumentSide, FitOutcome, GridRecord, PanelTabsRecord, SizeOutcome,
    SnappingSettingsRecord, SnappingToolbarRecord,
};
use crate::wasm_api::WasmSession;

fn side_from_str(name: &str) -> Option<DocumentSide> {
    match name {
        "width" => Some(DocumentSide::Width),
        "height" => Some(DocumentSide::Height),
        _ => None,
    }
}

fn outcome_word(outcome: SizeOutcome) -> String {
    match outcome {
        SizeOutcome::Committed => "committed",
        SizeOutcome::Unchanged => "unchanged",
        SizeOutcome::Invalid => "invalid:number",
    }
    .to_string()
}

#[wasm_bindgen]
impl WasmSession {
    /// What the Properties panel shows: the body (`"document"`, `"style"` or
    /// `"empty"`), the active tab and the strip. Reading it lets the tab rule
    /// see a change of selection, so the tab changes in the same frame.
    pub fn panel_view(&self) -> PanelTabsRecord {
        PanelTabsRecord::new(&self.session.panel_view())
    }

    /// A press on the tab `name` (`"document"` or `"style"`): `false` when
    /// nothing changed (the dimmed Style tab, an unknown name).
    pub fn press_panel_tab(&mut self, name: &str) -> bool {
        self.session.press_panel_tab(name)
    }

    /// Shift+Ctrl+F: Style with a selection, Document without.
    pub fn panel_shortcut_style(&mut self) {
        self.session.panel_shortcut_style();
    }

    /// Shift+Ctrl+D: the Document tab.
    pub fn panel_shortcut_document(&mut self) {
        self.session.panel_shortcut_document();
    }

    /// The display unit's symbol: `"mm"`, `"cm"` or `"in"`.
    #[must_use]
    pub fn display_unit(&self) -> String {
        self.session.display_unit().symbol().to_string()
    }

    /// Sets the display unit (`"mm"`, `"cm"` or `"in"`): one commit that moves
    /// nothing. `false` for an unknown symbol or the unit already shown.
    pub fn set_display_unit(&mut self, symbol: &str) -> bool {
        DisplayUnit::from_symbol(symbol).is_some_and(|unit| self.session.set_display_unit(unit))
    }

    /// Whether the document has objects, so Fit to content is offered.
    #[must_use]
    pub fn document_has_objects(&self) -> bool {
        self.session.has_objects()
    }

    /// The text of the `"width"` or `"height"` field in the display unit.
    #[must_use]
    pub fn document_side_text(&self, side: &str) -> String {
        side_from_str(side).map_or_else(String::new, |side| self.session.side_text(side))
    }

    /// The message of a refused size, the limits in the display unit.
    #[must_use]
    pub fn document_side_message(&self) -> String {
        self.session.side_message()
    }

    /// Enter or Tab in the `"width"` or `"height"` field: `"committed"`,
    /// `"unchanged"` or `"invalid:number"`.
    pub fn set_document_side(&mut self, side: &str, text: &str) -> String {
        let Some(side) = side_from_str(side) else {
            return "unchanged".to_string();
        };
        outcome_word(self.session.set_document_side(side, text))
    }

    /// The presets part of the Document section: the subject line, the pressed
    /// orientation and every button in file order. Read in the same sync as the
    /// size fields.
    #[must_use]
    pub fn document_presets_view(&self) -> DocumentPresetsRecord {
        DocumentPresetsRecord::new(&self.session.document_presets_view())
    }

    /// A press on the preset button `id`: `"committed"`, `"unchanged"` or
    /// `"invalid:number"` (an unknown id).
    pub fn apply_document_preset(&mut self, id: &str) -> String {
        outcome_word(self.session.apply_document_preset(id))
    }

    /// A press on an orientation item, `"portrait"` or `"landscape"`:
    /// `"committed"` or `"unchanged"`.
    pub fn set_document_orientation(&mut self, orientation: &str) -> String {
        let wanted = match orientation {
            "portrait" => Orientation::Portrait,
            "landscape" => Orientation::Landscape,
            _ => return "unchanged".to_string(),
        };
        outcome_word(self.session.set_document_orientation(wanted))
    }

    /// The Fit to content button: `"fitted"`, `"already-fits"`, `"empty"`,
    /// `"blocked"` (the Pen has an unfinished path) or `"too-large:<message>"`.
    pub fn fit_document(&mut self) -> String {
        match self.session.fit_document() {
            FitOutcome::Fitted => "fitted".to_string(),
            FitOutcome::AlreadyFits => "already-fits".to_string(),
            FitOutcome::Empty => "empty".to_string(),
            FitOutcome::Blocked => "blocked".to_string(),
            FitOutcome::TooLarge(message) => format!("too-large:{message}"),
        }
    }

    /// The File tab's Template button (`specs/0046-document-shapes` criteria 19 and 22): adds the
    /// outlines of the Curvyo document `bytes` as selected guides in one step. `"loaded:<n>"`
    /// with how many came in, or `"error:<message>"` when nothing changed.
    pub fn load_template(&mut self, bytes: &[u8]) -> String {
        match self.session.load_template(bytes) {
            Ok(count) => format!("loaded:{count}"),
            Err(error) => format!("error:{}", error.message()),
        }
    }

    /// The first view after a template load (`specs/0046-document-shapes` criterion 13), called
    /// once the bars are laid out for the new selection: zooms out to the page and the guides when
    /// part of them is outside the canvas area right of `free_left` and below `free_top` (CSS
    /// pixels from the canvas's top-left corner: the tool rail and the bar overlay).
    pub fn show_page_and_guides(&mut self, free_left: f64, free_top: f64) {
        self.session.show_page_and_guides(free_left, free_top);
    }

    /// The status bar's cursor readout for a document point in millimetres,
    /// for example `"x: 12.3  y: 45.6 mm"`.
    #[must_use]
    pub fn cursor_text(&self, x_mm: f64, y_mm: f64) -> String {
        self.session.cursor_text(x_mm, y_mm)
    }

    /// The status bar's size readout, for example `"210.0 × 297.0 mm"`.
    #[must_use]
    pub fn size_text(&self) -> String {
        self.session.size_text()
    }

    /// Whether snapping is enabled.
    #[must_use]
    pub fn snapping_enabled(&self) -> bool {
        self.session.snapping().enabled
    }

    /// Whether objects snap to other objects.
    #[must_use]
    pub fn snapping_objects(&self) -> bool {
        self.session.snapping().objects
    }

    /// Whether objects snap to guides.
    #[must_use]
    pub fn snapping_guides(&self) -> bool {
        self.session.snapping().guides
    }

    /// Whether objects snap to nodes (anchors).
    #[must_use]
    pub fn snapping_nodes(&self) -> bool {
        self.session.snapping().nodes
    }

    /// Whether objects snap to page fractions.
    #[must_use]
    pub fn snapping_page(&self) -> bool {
        self.session.snapping().page
    }

    /// The capture distance in screen pixels.
    #[must_use]
    pub fn snapping_capture_px(&self) -> f64 {
        self.session.snapping().capture_px
    }

    /// The release distance in screen pixels.
    #[must_use]
    pub fn snapping_release_px(&self) -> f64 {
        self.session.snapping().release_px
    }

    /// Sets snapping settings. Returns whether anything changed.
    // The WASM ABI exposes independent switches and numeric fields as primitives;
    // Session immediately groups them into the typed document settings record.
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub fn set_snapping(
        &mut self,
        enabled: bool,
        objects: bool,
        guides: bool,
        nodes: bool,
        page: bool,
        capture_px: f64,
        release_px: f64,
    ) -> bool {
        use curvyo_document_core::DocumentSnapping;
        let snapping = DocumentSnapping {
            enabled,
            objects,
            guides,
            nodes,
            page,
            capture_px,
            release_px,
            ..self.session.snapping()
        };
        self.session.set_snapping(snapping)
    }

    /// Sets the independently permitted snapping coordinates.
    pub fn set_snapping_axes(&mut self, x: bool, y: bool) -> bool {
        let settings = curvyo_document_core::DocumentSnapping {
            x,
            y,
            ..self.session.snapping()
        };
        self.session.set_snapping(settings)
    }

    /// The snapping settings as a record.
    #[must_use]
    pub fn snapping_settings(&self) -> SnappingSettingsRecord {
        self.session.snapping_settings()
    }

    /// The snapping toolbar state for the panel header.
    #[must_use]
    pub fn snapping_toolbar(&self) -> SnappingToolbarRecord {
        self.session.snapping_toolbar()
    }

    /// Toggles snapping enabled state. Returns the new state.
    pub fn toggle_snapping(&mut self) -> bool {
        self.session.toggle_snapping()
    }

    /// Sets the Grid snapping target (`specs/0090-grid` criterion 10).
    pub fn set_snapping_grid(&mut self, grid: bool) -> bool {
        self.session.set_snapping_grid(grid)
    }

    /// Sets the grid snap point: `"centre"`, `"box"` or `"nodes"` (criterion 11).
    pub fn set_snapping_grid_point(&mut self, name: &str) -> bool {
        self.session.set_snapping_grid_point(name)
    }

    /// The active page's grid for the Grid panel and rail entry (criteria 2 and 3).
    pub fn active_page_grid(&self) -> GridRecord {
        self.session.active_page_grid()
    }

    /// Shift+click on the Grid entry: shows or hides the active page's grid. Returns the new
    /// state.
    pub fn toggle_active_page_grid(&mut self) -> bool {
        self.session.toggle_active_page_grid()
    }

    /// The Grid panel's switch.
    pub fn set_active_page_grid_shown(&mut self, shown: bool) -> bool {
        self.session.set_active_page_grid_shown(shown)
    }

    /// The Mode radios: `"adaptive"` or `"fixed"`.
    pub fn set_active_page_grid_mode(&mut self, mode: &str) -> bool {
        self.session.set_active_page_grid_mode(mode)
    }

    /// The Scale radios: `"linear"` or `"log"`.
    pub fn set_active_page_grid_scale(&mut self, scale: &str) -> bool {
        self.session.set_active_page_grid_scale(scale)
    }

    /// The origin corner buttons (`specs/0090-grid` criterion 18): `"top_left"`, `"top_right"`,
    /// `"bottom_left"` or `"bottom_right"`.
    pub fn set_active_page_origin(&mut self, corner: &str) -> bool {
        self.session.set_active_page_origin(corner)
    }

    /// A typed Grid panel field: `"committed"`, `"unchanged"` or `"invalid:<key>"`.
    pub fn set_active_page_grid_field(&mut self, field: &str, text: &str) -> String {
        self.session.set_active_page_grid_field(field, text)
    }
}
