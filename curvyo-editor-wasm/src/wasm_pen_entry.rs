//! The Pen entry chip and the Pen's right press on the `wasm-bindgen` surface
//! (`specs/0072-pen-angle-snap-and-typed-values/`, `adrs.md` decision 7): one view per
//! interaction, strings and scalars only (ADR 0001 §5). A further `impl WasmSession` block, every
//! method a pass-through to `Session`.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// `wasm-bindgen`'s mirror of [`crate::session::PenEntryChipView`]. Positions are
/// canvas-relative CSS pixels, already converted.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct PenEntryChip {
    /// The last node L on screen: the chip keeps clear of it.
    pub last_x: f64,
    /// The last node L on screen.
    pub last_y: f64,
    /// The pointer the chip is placed from, on screen.
    pub pointer_x: f64,
    /// The pointer the chip is placed from, on screen.
    pub pointer_y: f64,
    /// The typed point on screen (the rubber band's end); `NaN` with a zero length.
    pub end_x: f64,
    /// The typed point on screen.
    pub end_y: f64,
    /// A field was edited: the chip stands still.
    pub frozen: bool,
    view: crate::session::PenEntryChipView,
}

#[wasm_bindgen]
impl PenEntryChip {
    /// "Next node".
    #[must_use]
    pub fn group_name(&self) -> String {
        self.view.group_name.to_owned()
    }

    fn field(&self, index: u32) -> Option<&crate::session::PenEntryFieldView> {
        self.view.fields.get(index as usize)
    }

    /// Field `index`'s visible label (0 Length, 1 Angle).
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
            .map(|f| f.name.clone())
            .unwrap_or_default()
    }

    /// Field `index`'s unit suffix.
    #[must_use]
    pub fn suffix(&self, index: u32) -> String {
        self.field(index)
            .map(|f| f.suffix.to_owned())
            .unwrap_or_default()
    }

    /// Field `index`'s text.
    #[must_use]
    pub fn text(&self, index: u32) -> String {
        self.field(index)
            .map(|f| f.text.clone())
            .unwrap_or_default()
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// The open Pen entry chip, or `undefined`.
    #[must_use]
    pub fn pen_entry_chip(&self) -> Option<PenEntryChip> {
        let view = self.session.pen_entry_chip()?;
        let transform = self.session.view();
        let (last_x, last_y) = transform.document_to_screen(view.last);
        let (pointer_x, pointer_y) = transform.document_to_screen(view.pointer);
        let (end_x, end_y) = view.end.map_or((f64::NAN, f64::NAN), |end| {
            transform.document_to_screen(end)
        });
        Some(PenEntryChip {
            last_x,
            last_y,
            pointer_x,
            pointer_y,
            end_x,
            end_y,
            frozen: view.frozen,
            view,
        })
    }

    /// The maker edited field `index` of the chip to `text`.
    pub fn pen_entry_edit(&mut self, index: u32, text: &str) {
        self.session.pen_entry_edit(index as usize, text);
    }

    /// Enter in the chip: adds the node, or keeps the chip open with the notice.
    pub fn pen_entry_place(&mut self) -> bool {
        self.session.pen_entry_place()
    }

    /// Escape in the chip, or a press outside the canvas and the chip.
    pub fn cancel_pen_entry(&mut self) {
        self.session.cancel_pen_entry();
    }

    /// What the chip's live region says next, or empty.
    pub fn take_pen_entry_announcement(&mut self) -> String {
        self.session
            .take_pen_entry_announcement()
            .unwrap_or_default()
    }

    /// What the canvas notice slot shows next, or empty.
    pub fn take_pen_entry_notice(&mut self) -> String {
        self.session.take_pen_entry_notice().unwrap_or_default()
    }

    /// A secondary press with the Pen: finishes the path in progress as Enter does and returns
    /// `true`, or returns `false` with no path so the press goes on as before.
    pub fn pen_secondary_press(&mut self) -> bool {
        self.session.pen_secondary_press()
    }
}
