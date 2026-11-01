//! The Select bar's `wasm-bindgen` surface (`specs/0009-unified-object-editing`,
//! criteria 21 to 23): a second `impl WasmSession` block, so `wasm_api.rs`
//! does not grow. Strings and scalars only (ADR 0001 §5); every method is a
//! direct pass-through to `Session`, which holds the orchestration, over
//! `curvyo-ui-core`'s rules.

use curvyo_ui_core::{
    AlignOutcome, AlignReference, BarValue, EntryOutcome, FlipAxis, FlipOutcome, InvalidReason,
    SelectBarState, clamped_ratio,
};
use wasm_bindgen::prelude::*;

use crate::session::{GuideButtonState, align_command_from_code, distribute_from_code};
use crate::wasm_api::WasmSession;

/// `wasm-bindgen`'s JS-facing mirror of [`curvyo_ui_core::SelectBarState`]:
/// which kind controls the Select bar shows and their values. A `*_mixed`
/// flag means the selected objects differ (the field is empty with the
/// placeholder "Mixed"); the value beside it is then meaningless.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub struct SelectBarView {
    /// "Radius" is shown (the selection holds a rectangle).
    pub radius_shown: bool,
    /// The rectangles hold different effective radii.
    pub radius_mixed: bool,
    /// The effective radius, millimetres.
    pub radius: f64,
    /// The stored radius exceeds what the rectangle allows ("limited").
    pub radius_limited: bool,
    /// The stored radius, millimetres, for the tooltip of "limited".
    pub radius_stored: f64,
    /// One rectangle with unequal corners is selected: `radius_tl` to
    /// `radius_bl` are its effective radii for the "Mixed" tooltip.
    pub radius_corners_shown: bool,
    /// Top-left effective radius, millimetres.
    pub radius_tl: f64,
    /// Top-right effective radius, millimetres.
    pub radius_tr: f64,
    /// Bottom-right effective radius, millimetres.
    pub radius_br: f64,
    /// Bottom-left effective radius, millimetres.
    pub radius_bl: f64,
    /// "Remove rounding" is shown.
    pub remove_rounding_shown: bool,
    /// "Remove rounding" would change something.
    pub remove_rounding_enabled: bool,
    /// "Points" is shown (the selection holds a polygon or star).
    pub points_shown: bool,
    /// The polygons and stars hold different point counts.
    pub points_mixed: bool,
    /// The point count.
    pub points: u32,
    /// "Ratio" is shown (the selection holds a star).
    pub ratio_shown: bool,
    /// The stars hold different ratios.
    pub ratio_mixed: bool,
    /// The inner ratio.
    pub ratio: f64,
    /// "Object to path" is shown.
    pub object_to_path_shown: bool,
    /// "Scale stroke width" and "Scale corner radius" are shown (not with only linked clones).
    pub scale_switches_shown: bool,
    /// "Stroke to path" is enabled: some selected object draws a stroke
    /// (`specs/0085-stroke-to-path` criterion 1). Shown whenever anything is selected.
    pub stroke_to_path_enabled: bool,
    /// "Stroke to path" is shown: something is selected and not only dimensions
    /// (`specs/0070-measure-and-dimension` UI review item 3).
    pub stroke_to_path_shown: bool,
}

impl From<SelectBarState> for SelectBarView {
    fn from(state: SelectBarState) -> Self {
        let (radius_shown, radius_mixed, radius) = match state.radius {
            None => (false, false, 0.0),
            Some(BarValue::Mixed) => (true, true, 0.0),
            Some(BarValue::Uniform(length)) => (true, false, length.as_mm()),
        };
        let (points_shown, points_mixed, points) = match state.points {
            None => (false, false, 0),
            Some(BarValue::Mixed) => (true, true, 0),
            Some(BarValue::Uniform(count)) => (true, false, count),
        };
        let (ratio_shown, ratio_mixed, ratio) = match state.ratio {
            None => (false, false, 0.0),
            Some(BarValue::Mixed) => (true, true, 0.0),
            Some(BarValue::Uniform(value)) => (true, false, value),
        };
        Self {
            radius_shown,
            radius_mixed,
            radius,
            radius_limited: state.radius_limited.is_some(),
            radius_stored: state
                .radius_limited
                .map_or(0.0, curvyo_document_core::Length::as_mm),
            radius_corners_shown: state.radius_corners.is_some(),
            radius_tl: state.radius_corners.map_or(0.0, |c| c.tl.as_mm()),
            radius_tr: state.radius_corners.map_or(0.0, |c| c.tr.as_mm()),
            radius_br: state.radius_corners.map_or(0.0, |c| c.br.as_mm()),
            radius_bl: state.radius_corners.map_or(0.0, |c| c.bl.as_mm()),
            remove_rounding_shown: state.remove_rounding_shown,
            remove_rounding_enabled: state.remove_rounding_enabled,
            points_shown,
            points_mixed,
            points,
            ratio_shown,
            ratio_mixed,
            ratio,
            object_to_path_shown: state.object_to_path,
            scale_switches_shown: state.scale_switches_shown,
            stroke_to_path_enabled: false,
            stroke_to_path_shown: false,
        }
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// What the Select bar shows for the current selection. Call after every
    /// pointer release and tool or selection change, and after every bar edit.
    #[must_use]
    pub fn select_bar_state(&self) -> SelectBarView {
        let mut view: SelectBarView = self.session.select_bar_state().into();
        view.stroke_to_path_enabled = self.session.stroke_to_path_available();
        view.stroke_to_path_shown = self.session.stroke_to_path_shown();
        view
    }

    /// The Select tool's "Scale corner radius" switch (criterion 23). Off in
    /// every new session; not persisted.
    #[must_use]
    pub fn scale_corner_radius(&self) -> bool {
        self.session.scale_corner_radius()
    }

    /// Sets the "Scale corner radius" switch for the next resize. Clicking it
    /// closes an open numeric entry without writing.
    pub fn set_scale_corner_radius(&mut self, on: bool) {
        self.session.set_scale_corner_radius(on);
    }

    /// The Select tool's "Link corners" switch (`specs/0013-rectangle-corner-radii/`
    /// criterion 2). On in every new session; not persisted.
    #[must_use]
    pub fn link_corners(&self) -> bool {
        self.session.link_corners()
    }

    /// Sets the "Link corners" switch for the next corner radius drag or
    /// entry. Writes nothing; an open numeric entry closes without writing.
    pub fn set_link_corners(&mut self, on: bool) {
        self.session.set_link_corners(on);
    }

    /// Whether the two flip buttons act (`specs/0056-mirror-and-negative-scale/` criterion
    /// 1): something is selected. The buttons are shown in every selection state.
    #[must_use]
    pub fn flip_enabled(&self) -> bool {
        self.session.flip_enabled()
    }

    /// "Flip horizontally" (criterion 2): `"flipped"`, `"unchanged"` (the selection looks
    /// the same flipped, nothing was written, criterion 3) or `"nothing"`.
    pub fn flip_horizontally(&mut self) -> String {
        flip_code(self.session.flip_selection(FlipAxis::Horizontal))
    }

    /// "Flip vertically": as [`WasmSession::flip_horizontally`].
    pub fn flip_vertically(&mut self) -> String {
        flip_code(self.session.flip_selection(FlipAxis::Vertical))
    }

    /// Whether the Align and Distribute groups are in the Select bar
    /// (`specs/0061-align-and-distribute/` criteria 1, 15, 16): the Select
    /// tool, in every selection state.
    #[must_use]
    pub fn align_shown(&self) -> bool {
        self.session.align_group_shown()
    }

    /// What an align press acts on now: `"page-objects"` (nothing selected),
    /// `"page"` (one object) or `"selection"`.
    #[must_use]
    pub fn align_reference(&self) -> String {
        reference_code(self.session.align_reference())
    }

    /// What a distribute press acts on now: `"page-objects"` (nothing
    /// selected), `"page"` (one or two objects) or `"selection"`.
    #[must_use]
    pub fn distribute_reference(&self) -> String {
        reference_code(self.session.distribute_reference())
    }

    /// The number of selected objects (groups count once) the Arrange in a
    /// grid group works on; the group is shown from two (criterion 18).
    #[must_use]
    pub fn grid_count(&self) -> u32 {
        u32::try_from(self.session.grid_count()).unwrap_or(u32::MAX)
    }

    /// A key that changes with the selected objects: the grid fields keep
    /// their values until it does (criterion 21).
    #[must_use]
    pub fn grid_selection_key(&self) -> String {
        self.session.grid_selection_key().to_string()
    }

    /// An Arrange in a grid press (criteria 20, 21). Returns `"aligned"`,
    /// `"unchanged"`, `"too-few-cells"` (criterion 19) or `"nothing"`.
    pub fn arrange_grid(&mut self, rows: u32, columns: u32, gap_mm: f64) -> String {
        align_code(self.session.arrange_grid(rows, columns, gap_mm))
    }

    /// Shows or hides the page cue: the active page's frame marked while a
    /// button whose reference is the page is hovered or focused (UX notes,
    /// "Canvas cue").
    pub fn set_page_cue(&mut self, on: bool) {
        self.session.set_page_cue(on);
    }

    /// An align press: `command` is one of `"left"`, `"centre-h"`, `"right"`,
    /// `"top"`, `"centre-v"`, `"bottom"`. Returns `"aligned"`, `"unchanged"`
    /// (criterion 3) or `"nothing"`, also for an unknown code.
    pub fn align(&mut self, command: &str) -> String {
        match align_command_from_code(command) {
            Some(command) => align_code(self.session.align_selection(command)),
            None => align_code(AlignOutcome::NothingSelected),
        }
    }

    /// A distribute press: `code` is one of `"h-gaps"`, `"h-centres"`,
    /// `"v-gaps"`, `"v-centres"`; `gap_mm` is the typed gap of criterion 8,
    /// `None` keeps the ends. Returns `"aligned"`, `"unchanged"` (criterion 7)
    /// or `"nothing"`, also for an unknown code.
    pub fn distribute(&mut self, code: &str, gap_mm: Option<f64>) -> String {
        match distribute_from_code(code) {
            Some((axis, mode)) => align_code(self.session.distribute_selection(axis, mode, gap_mm)),
            None => align_code(AlignOutcome::NothingSelected),
        }
    }

    /// The Guide button (`specs/0046-document-shapes` criteria 9, 10 and 34): `"disabled"` when
    /// nothing selected can be a guide, `"pressed"` when everything that can is one,
    /// `"pressed-inert"` when only linked clones of guides are selected, else `"off"`.
    #[must_use]
    pub fn guide_state(&self) -> String {
        match self.session.guide_button_state() {
            GuideButtonState::Disabled => "disabled",
            GuideButtonState::Off => "off",
            GuideButtonState::Pressed => "pressed",
            GuideButtonState::PressedInert => "pressed-inert",
        }
        .to_string()
    }

    /// A press of the Guide button: marks the selection as guides or turns them back, one step
    /// "Guide". Whether anything was written.
    pub fn toggle_guide(&mut self) -> bool {
        self.session.toggle_guide()
    }

    /// Removes the "max" notice of a limited typed radius, 1.5 s after it
    /// appeared.
    pub fn clear_limit_notice(&mut self) {
        self.session.clear_limit_notice();
    }

    /// Enter in the bar's "Radius" field (criterion 21a): `"committed"`,
    /// `"unchanged"`, or `"invalid:number"` / `"invalid:negative"` (the field
    /// stays open, marked invalid, and nothing is written).
    pub fn set_selected_radius(&mut self, text: &str) -> String {
        match self.session.set_selected_radius_text(text) {
            EntryOutcome::Committed => "committed".to_string(),
            EntryOutcome::Unchanged => "unchanged".to_string(),
            EntryOutcome::Invalid { reason, .. } => format!(
                "invalid:{}",
                match reason {
                    InvalidReason::Negative => "negative",
                    _ => "number",
                }
            ),
        }
    }

    /// The "Points" field and stepper: one commit for every selected polygon
    /// and star.
    ///
    /// # Errors
    /// A `JsValue` if `count` is outside `3..=1024`.
    pub fn set_selected_point_count(&mut self, count: u32) -> Result<(), JsValue> {
        let count = curvyo_document_core::PointCount::new(count)
            .map_err(|err| JsValue::from_str(&format!("{err}")))?;
        self.session.set_selected_point_count(count);
        Ok(())
    }

    /// The "Ratio" field's discrete commit: one commit for every selected
    /// star. A value outside 0.01 to 0.99 is limited to it.
    pub fn set_selected_ratio(&mut self, ratio: f64) {
        self.session.set_selected_ratio(clamped_ratio(ratio));
    }

    /// The "Ratio" slider's live preview, on every tick: writes nothing, the
    /// stars show in blue over their unchanged old shape.
    pub fn preview_selected_ratio(&mut self, ratio: f64) {
        self.session.preview_selected_ratio(clamped_ratio(ratio));
    }

    /// Drops the slider edit pending without writing it: Escape in the middle of
    /// a drag.
    pub fn cancel_selected_ratio(&mut self) {
        self.session.cancel_selected_ratio();
    }

    /// Commits the slider edit pending, as one commit, on the slider's
    /// release, key-up or blur.
    pub fn commit_selected_ratio(&mut self) {
        self.session.commit_selected_ratio();
    }
}

/// The string a flip reports to the host.
fn flip_code(outcome: FlipOutcome) -> String {
    match outcome {
        FlipOutcome::Flipped => "flipped",
        FlipOutcome::LooksTheSame => "unchanged",
        FlipOutcome::NothingSelected => "nothing",
    }
    .to_string()
}

/// The wasm code of an align or distribute outcome.
fn align_code(outcome: AlignOutcome) -> String {
    match outcome {
        AlignOutcome::Aligned => "aligned",
        AlignOutcome::Unchanged => "unchanged",
        AlignOutcome::NothingSelected => "nothing",
        AlignOutcome::TooFewCells => "too-few-cells",
    }
    .to_string()
}

/// The wasm code of an align or distribute reference.
fn reference_code(reference: AlignReference) -> String {
    match reference {
        AlignReference::PageObjects => "page-objects",
        AlignReference::Page => "page",
        AlignReference::Selection => "selection",
    }
    .to_string()
}
