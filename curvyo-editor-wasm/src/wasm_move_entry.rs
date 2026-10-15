//! The typed move's `wasm-bindgen` surface
//! (`specs/0010-edit-interaction-polish/`, criteria 15 to 25): a second
//! `impl WasmSession` block, so `wasm_api.rs` does not grow. Strings and
//! scalars only (ADR 0001 §5); every method passes straight through to
//! `Session`, which holds the orchestration, over `curvyo-ui-core`'s rules.

use curvyo_ui_core::{EntryOutcome, InvalidReason, MoveEntryMode, ReferencePoint};
use wasm_bindgen::prelude::*;

use crate::session::MoveEntryView;
use crate::wasm_api::WasmSession;

/// The string an entry commit reports: `"committed"`, `"unchanged"` or
/// `"invalid:<field>:<reason>"` (the entry stays open). Shared by the size,
/// angle, skew and move chips.
pub(crate) fn outcome_code(outcome: EntryOutcome) -> String {
    match outcome {
        EntryOutcome::Committed => "committed".to_string(),
        EntryOutcome::Unchanged => "unchanged".to_string(),
        EntryOutcome::Invalid { field, reason } => format!(
            "invalid:{field}:{}",
            match reason {
                InvalidReason::NotANumber => "number",
                InvalidReason::Negative => "negative",
                InvalidReason::RatioRange => "ratio-range",
                InvalidReason::SkewRange => "skew-range",
                InvalidReason::TooLarge => "too-large",
            }
        ),
    }
}

/// `wasm-bindgen`'s JS-facing mirror of [`MoveEntryView`]: the typed move
/// chip's anchor and prefill. `center_*` are canvas-relative CSS pixels,
/// already converted; the chip opens 16 px right of and below it.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct MoveEntryChip {
    pub center_x: f64,
    pub center_y: f64,
    /// The y of the lower edge of the true bounding box, canvas-relative CSS pixels.
    pub box_bottom_y: f64,
    relative: [String; 2],
    absolute: [[String; 2]; 9],
    reference: usize,
    copy_preset: bool,
}

#[wasm_bindgen]
impl MoveEntryChip {
    /// The text X (`0`) or Y (`1`) opens with in Relative mode.
    #[must_use]
    pub fn relative_prefill(&self, axis: u32) -> String {
        self.relative
            .get(axis as usize)
            .cloned()
            .unwrap_or_default()
    }

    /// Whether the Copy check opens on: Ctrl was held at the second press of
    /// the double-click (criterion 23).
    #[must_use]
    pub fn copy_preset(&self) -> bool {
        self.copy_preset
    }

    /// The chosen reference point, 0 to 8 in the order of the radios (top left, top centre,
    /// top right, left middle, centre, right middle, bottom left, bottom centre, bottom right):
    /// the centre (4) on every open.
    #[must_use]
    pub fn reference(&self) -> u32 {
        u32::try_from(self.reference).unwrap_or(4)
    }

    /// The text X (`0`) or Y (`1`) shows in Absolute mode while untouched for the reference
    /// point `reference` (0 to 8): that point's current position, one decimal.
    #[must_use]
    pub fn absolute_prefill(&self, reference: u32, axis: u32) -> String {
        self.absolute
            .get(reference as usize)
            .and_then(|pair| pair.get(axis as usize))
            .cloned()
            .unwrap_or_default()
    }

    /// The accessible name of the radio of reference point `index` (0 to 8): "Top left", "Top
    /// centre", "Top right", "Left middle", "Centre", "Right middle", "Bottom left", "Bottom
    /// centre", "Bottom right"; empty beyond 8.
    #[must_use]
    #[allow(
        clippy::unused_self,
        reason = "an instance method of the chip for the JS side"
    )]
    pub fn reference_name(&self, index: u32) -> String {
        ReferencePoint::from_index(index as usize)
            .map(|point| point.name().to_string())
            .unwrap_or_default()
    }

    /// The first line of the hint chip of the mark of reference point `index` (0 to 8): "Top left
    /// corner", "Top centre", ..., "Centre"; empty beyond 8.
    #[must_use]
    #[allow(
        clippy::unused_self,
        reason = "an instance method of the chip for the JS side"
    )]
    pub fn reference_hint_name(&self, index: u32) -> String {
        ReferencePoint::from_index(index as usize)
            .map(|point| point.hint_name().to_string())
            .unwrap_or_default()
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// The typed move chip to show, or `undefined` (criteria 15, 18, 56).
    /// Call after every pointer release, key press and tool or selection
    /// change.
    #[must_use]
    pub fn move_entry(&self) -> Option<MoveEntryChip> {
        let MoveEntryView {
            center,
            relative_prefill,
            absolute_prefills,
            reference,
            box_bottom,
            copy_preset,
        } = self.session.move_entry()?;
        let view = self.session.view();
        let (center_x, center_y) = view.document_to_screen(center);
        Some(MoveEntryChip {
            center_x,
            center_y,
            box_bottom_y: view.document_to_screen(box_bottom).1,
            relative: relative_prefill,
            absolute: absolute_prefills,
            reference: reference.index(),
            copy_preset,
        })
    }

    /// The chip reports its mode: `true` in Absolute. The marks of the nine reference points
    /// exist in Absolute only (criterion 39).
    pub fn set_move_chip_absolute(&mut self, absolute: bool) {
        self.session.set_move_chip_absolute(absolute);
    }

    /// The chip reports the reference point its Reference control chose (0 to 8; others are
    /// ignored). Nothing is applied (criterion 41).
    pub fn set_move_reference(&mut self, index: u32) {
        if let Some(reference) = ReferencePoint::from_index(index as usize) {
            self.session.set_move_reference(reference);
        }
    }

    /// The chip reports the radio with the pointer or the focus (0 to 8), or `-1` for none: its
    /// mark is drawn in its hover state (criterion 41).
    pub fn set_move_radio_hover(&mut self, index: i32) {
        let hover = usize::try_from(index)
            .ok()
            .and_then(ReferencePoint::from_index);
        self.session.set_move_radio_hover(hover);
    }

    /// The mark (0 to 8) a press at the canvas-relative CSS pixel `(x, y)` is on, or `-1`: only
    /// while the move chip is open in Absolute mode (criterion 40). `touch` for a touch or a pen.
    #[must_use]
    pub fn move_mark_at(&self, x: f64, y: f64, touch: bool) -> i32 {
        let at = self.session.screen_to_document(x, y);
        self.session
            .move_mark_at(at, touch)
            .map_or(-1, |mark| i32::try_from(mark.index()).unwrap_or(-1))
    }

    /// The mark (0 to 8) under the pointer, or `-1` (criteria 43 and 44).
    #[must_use]
    pub fn move_mark_hover(&self) -> i32 {
        self.session
            .move_mark_hover()
            .map_or(-1, |mark| i32::try_from(mark.index()).unwrap_or(-1))
    }

    /// A click on a mark: the move executes with the mark `reference` (0 to 8) as the reference,
    /// in Absolute mode, with the texts of the fields and the state of the Copy check; the same
    /// result codes as [`WasmSession::commit_move_entry`] (criterion 40).
    pub fn commit_move_entry_at(
        &mut self,
        first: &str,
        second: &str,
        copy: bool,
        reference: u32,
    ) -> String {
        let Some(reference) = ReferencePoint::from_index(reference as usize) else {
            return outcome_code(EntryOutcome::Unchanged);
        };
        outcome_code(
            self.session
                .commit_move_entry_at(first, second, copy, reference),
        )
    }

    /// Enter in the move chip: `first` and `second` are the X and Y texts,
    /// `absolute` the chip's mode and `copy` its Copy check (criteria 19 to 23,
    /// 25). Returns `"committed"`, `"unchanged"` (both close the chip) or
    /// `"invalid:<field>:number"` (the chip stays open).
    pub fn commit_move_entry(
        &mut self,
        first: &str,
        second: &str,
        absolute: bool,
        copy: bool,
    ) -> String {
        outcome_code(self.session.commit_move_entry(
            first,
            second,
            MoveEntryMode { absolute, copy },
        ))
    }
}
