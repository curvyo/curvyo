//! The shape tools' `wasm-bindgen` surface (`specs/0003-primitive-shapes`):
//! the polygon/star tool-options bar, "Object to path" and the click-to-create placement
//! chip (`specs/0052-click-to-create-shapes`). A further
//! `impl WasmSession` block (`wasm_api.rs` holds the struct); strings and
//! scalars only (ADR 0001 §5), every method a pass-through to `Session`.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

fn poly_star_mode_from_str(name: &str) -> Result<curvyo_ui_core::PolyStarMode, JsValue> {
    match name {
        "polygon" => Ok(curvyo_ui_core::PolyStarMode::Polygon),
        "star" => Ok(curvyo_ui_core::PolyStarMode::Star),
        other => Err(JsValue::from_str(&format!(
            "unknown polygon/star mode: {other}"
        ))),
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// The polygon/star tool-options bar's current mode: `"polygon"` or
    /// `"star"`.
    #[must_use]
    pub fn poly_star_mode(&self) -> String {
        match self.session.poly_star_mode() {
            curvyo_ui_core::PolyStarMode::Polygon => "polygon".to_string(),
            curvyo_ui_core::PolyStarMode::Star => "star".to_string(),
        }
    }

    /// The mode toggle (acceptance criteria 11 vs. 12).
    ///
    /// # Errors
    /// A `JsValue` if `mode` is neither `"polygon"` nor `"star"`.
    pub fn set_poly_star_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        self.session
            .set_poly_star_mode(poly_star_mode_from_str(mode)?);
        Ok(())
    }

    /// The polygon/star bar's point count for the next shape (acceptance
    /// criterion 10).
    #[must_use]
    pub fn poly_star_point_count(&self) -> u32 {
        self.session.poly_star_point_count().get()
    }

    /// The point-count setting for the next shape (acceptance criterion 10);
    /// it never changes a selected shape (`unified-object-editing` criterion
    /// 29; the Select bar's `set_selected_point_count` does).
    ///
    /// # Errors
    /// A `JsValue` if `count` is outside `3..=1024`.
    pub fn set_poly_star_point_count(&mut self, count: u32) -> Result<(), JsValue> {
        let count = curvyo_document_core::PointCount::new(count)
            .map_err(|err| JsValue::from_str(&format!("{err}")))?;
        self.session.set_poly_star_point_count(count);
        Ok(())
    }

    /// The polygon/star tool-options bar's current ratio (acceptance
    /// criterion 12).
    #[must_use]
    pub fn poly_star_ratio(&self) -> f64 {
        self.session.poly_star_ratio().get()
    }

    /// The ratio setting for the next star (acceptance criteria 12); it never
    /// changes a selected star.
    ///
    /// # Errors
    /// A `JsValue` if `ratio` is outside the open interval `(0, 1)`.
    pub fn set_poly_star_ratio(&mut self, ratio: f64) -> Result<(), JsValue> {
        let ratio = curvyo_document_core::InnerRatio::new(ratio)
            .map_err(|err| JsValue::from_str(&format!("{err}")))?;
        self.session.set_poly_star_ratio(ratio);
        Ok(())
    }

    /// "Object to path" (acceptance criteria 17, 21, 22).
    pub fn convert_selected_to_paths(&mut self) {
        self.session.convert_selected_to_paths();
    }
}

/// `wasm-bindgen`'s mirror of [`crate::session::PlacementChipView`]
/// (`specs/0052-click-to-create-shapes/` criteria 11, 12, 17, 20, 23, 29). Positions are
/// canvas-relative CSS pixels, already converted.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct PlacementChip {
    /// Point A on screen.
    pub a_x: f64,
    /// Point A on screen.
    pub a_y: f64,
    /// The pointer the chip is placed from, on screen.
    pub pointer_x: f64,
    /// The pointer the chip is placed from, on screen.
    pub pointer_y: f64,
    /// W and H are linked (the chain glyph).
    pub linked: bool,
    /// A field was edited: the chip stands still.
    pub frozen: bool,
    view: crate::session::PlacementChipView,
}

#[wasm_bindgen]
impl PlacementChip {
    /// "New rectangle", "New ellipse", "New polygon" or "New star".
    #[must_use]
    pub fn group_name(&self) -> String {
        self.view.group_name.clone()
    }

    /// How many fields the chip has (3 for the rectangle, 2 otherwise).
    #[must_use]
    pub fn field_count(&self) -> u32 {
        u32::try_from(self.view.fields.len()).unwrap_or(0)
    }

    fn field(&self, index: u32) -> Option<&crate::session::PlacementFieldView> {
        self.view.fields.get(index as usize)
    }

    /// Field `index`'s visible label.
    #[must_use]
    pub fn label(&self, index: u32) -> String {
        self.field(index)
            .map(|f| f.label.to_owned())
            .unwrap_or_default()
    }

    /// Field `index`'s accessible name.
    #[must_use]
    pub fn name(&self, index: u32) -> String {
        self.field(index)
            .map(|f| f.name.to_owned())
            .unwrap_or_default()
    }

    /// Field `index`'s text.
    #[must_use]
    pub fn text(&self, index: u32) -> String {
        self.field(index)
            .map(|f| f.text.clone())
            .unwrap_or_default()
    }

    /// Field `index`'s placeholder, or empty.
    #[must_use]
    pub fn placeholder(&self, index: u32) -> String {
        self.field(index)
            .map(|f| f.placeholder.clone())
            .unwrap_or_default()
    }

    /// Field `index`'s suffix.
    #[must_use]
    pub fn suffix(&self, index: u32) -> String {
        self.field(index)
            .map(|f| f.suffix.to_owned())
            .unwrap_or_default()
    }

    /// Whether field `index`'s text is refused right now.
    #[must_use]
    pub fn invalid(&self, index: u32) -> bool {
        self.field(index).is_some_and(|f| f.invalid)
    }

    /// Whether field `index` is the size field, whose label is the r/⌀ button.
    #[must_use]
    pub fn is_size(&self, index: u32) -> bool {
        self.field(index).is_some_and(|f| f.size)
    }

    /// How many modifier checks the chip shows (0 once frozen).
    #[must_use]
    pub fn check_count(&self) -> u32 {
        u32::try_from(self.view.checks.len()).unwrap_or(0)
    }

    /// Check `index`'s label with its key.
    #[must_use]
    pub fn check_label(&self, index: u32) -> String {
        self.view
            .checks
            .get(index as usize)
            .map(|c| c.label.to_owned())
            .unwrap_or_default()
    }

    /// Whether check `index` is in effect.
    #[must_use]
    pub fn check_on(&self, index: u32) -> bool {
        self.view.checks.get(index as usize).is_some_and(|c| c.on)
    }

    /// Whether check `index` is the Shift check, not the Ctrl one.
    #[must_use]
    pub fn check_is_shift(&self, index: u32) -> bool {
        self.view
            .checks
            .get(index as usize)
            .is_some_and(|c| c.shift)
    }

    /// The refused field's index after an Enter or a click, or -1.
    #[must_use]
    pub fn error_field(&self) -> i32 {
        self.view
            .error
            .and_then(|(index, _)| i32::try_from(index).ok())
            .unwrap_or(-1)
    }

    /// The refusal's message, or empty.
    #[must_use]
    pub fn error_message(&self) -> String {
        self.view
            .error
            .map(|(_, message)| message.to_owned())
            .unwrap_or_default()
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// The running placement's chip, or `undefined` when none runs.
    #[must_use]
    pub fn placement_chip(&self) -> Option<PlacementChip> {
        let view = self.session.placement_chip()?;
        let transform = self.session.view();
        let (a_x, a_y) = transform.document_to_screen(view.a);
        let (pointer_x, pointer_y) = transform.document_to_screen(view.pointer);
        Some(PlacementChip {
            a_x,
            a_y,
            pointer_x,
            pointer_y,
            linked: view.linked,
            frozen: view.frozen,
            view,
        })
    }

    /// The maker edited field `index` of the chip to `text`.
    pub fn placement_edit(&mut self, index: u32, text: &str) {
        self.session.placement_edit(index as usize, text);
    }

    /// A key-down during a placement: Shift and Ctrl taps toggle. Returns whether the placement
    /// changed.
    pub fn placement_key_down(&mut self, key: &str, shift: bool, ctrl: bool, repeat: bool) -> bool {
        self.session.placement_key_down(key, shift, ctrl, repeat)
    }

    /// Whether a placement is running.
    #[must_use]
    pub fn placement_running(&self) -> bool {
        self.session.placement().is_some()
    }

    /// A click on a modifier check of the chip. Returns whether it flipped.
    pub fn placement_toggle_modifier(&mut self, shift: bool) -> bool {
        self.session.placement_toggle_modifier(shift)
    }

    /// Space or the r/⌀ button.
    pub fn placement_toggle_diameter(&mut self) {
        self.session.placement_toggle_diameter();
    }

    /// Enter in the chip: places the shape, or refuses with the first invalid field. Returns
    /// whether it placed.
    pub fn placement_place(&mut self) -> bool {
        self.session.place_placement()
    }

    /// Escape in the chip, a secondary press or a press outside: ends the placement.
    pub fn cancel_placement(&mut self) {
        self.session.cancel_placement();
    }

    /// The frontend says whether the next press is a touch.
    pub fn set_touch_input(&mut self, touch: bool) {
        self.session.set_touch_input(touch);
    }

    /// What the placement's live region says next, or empty.
    pub fn take_placement_announcement(&mut self) -> String {
        self.session
            .take_placement_announcement()
            .unwrap_or_default()
    }
}
