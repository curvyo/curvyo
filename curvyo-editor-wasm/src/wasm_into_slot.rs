//! Thin host bridge of a library item put into a slot (`specs/0154-library-into-slots`), with
//! ids as the full id texts the views carry.

use curvyo_document_core::{Length, Point};
use curvyo_laser_core::FitMode;
use wasm_bindgen::prelude::*;

use crate::session::{SlotSource, UnitBox};
use crate::wasm_api::WasmSession;
use crate::wasm_library::{CameFromWire, WasmLibrarySource, ids_of};

const fn mode(real_size: bool) -> FitMode {
    if real_size {
        FitMode::RealSize
    } else {
        FitMode::Fit
    }
}

fn unit(width: f64, height: f64, known: bool) -> Option<UnitBox> {
    known.then(|| UnitBox {
        size: (Length::from_mm(width), Length::from_mm(height)),
        centre: Point::new(0.0, 0.0),
    })
}

/// A record for the reads that write nothing.
fn empty_record() -> curvyo_document_core::CameFrom {
    curvyo_document_core::CameFrom {
        document: String::new(),
        page: String::new(),
        page_id: String::new(),
        item: None,
        at: curvyo_document_core::UnixMillis::new(0),
        online: None,
    }
}

fn error(text: &str) -> String {
    serde_json::json!({ "error": text }).to_string()
}

#[wasm_bindgen]
impl WasmLibrarySource {
    /// How many pages the source has (a source of several pages is refused over a slot).
    pub fn page_count(&self) -> usize {
        self.session.source_document().pages().len()
    }

    /// The id of the source's first page, or the empty string.
    pub fn first_page_id(&self) -> String {
        self.session
            .source_document()
            .pages()
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_default()
    }

    /// The size in mm of the unit the page `page` (or the picks `roots`, JSON ids) makes, as JSON
    /// `[width, height]`, or `null`.
    #[allow(clippy::needless_pass_by_value)]
    pub fn unit_size_json(&self, roots: &str, page: Option<String>) -> String {
        let roots = ids_of(roots);
        let came_from = empty_record();
        let from = SlotSource {
            source: &self.session,
            roots: &roots,
            page: page.as_deref(),
            came_from: &came_from,
        };
        from.unit_box().map_or_else(
            || "null".to_owned(),
            |b| serde_json::json!([b.size.0.as_mm(), b.size.1.as_mm()]).to_string(),
        )
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// The Into slot list of the active laser page as JSON (`{holders, selected}`), with the
    /// outcome per slot for a unit of `width` x `height` mm when `known`.
    pub fn slot_targets_json(
        &self,
        width: f64,
        height: f64,
        known: bool,
        real_size: bool,
    ) -> String {
        serde_json::to_string(
            &self
                .session
                .slot_targets_view(unit(width, height, known), real_size),
        )
        .unwrap_or_default()
    }

    /// The full ids of the filled target slots, as a JSON array (the Fixtures tab's "filled").
    pub fn filled_slots_json(&self) -> String {
        let view = self.session.slot_targets_view(None, false);
        let ids: Vec<&str> = view
            .holders
            .iter()
            .flat_map(|h| h.slots.iter())
            .filter(|r| r.filled)
            .map(|r| r.id.as_str())
            .collect();
        serde_json::to_string(&ids).unwrap_or_default()
    }

    /// The slot each top-level member of the active laser page lies in, as JSON
    /// `[[id, slot name], ...]` with the member's id as the Layers rows carry it.
    pub fn member_slots_json(&self) -> String {
        let pairs: Vec<(curvyo_document_core::NodeId, String)> = self.session.member_slots();
        serde_json::to_string(&pairs).unwrap_or_default()
    }

    /// Starts the slot cue of a library drag for a unit of `width` x `height` mm when `known`.
    pub fn begin_slot_drop(&mut self, width: f64, height: f64, known: bool) {
        self.session.begin_slot_drop(unit(width, height, known));
    }

    /// The cue at the canvas point `x`, `y` (CSS px) as JSON (`{kind, slot, holder, badge}`).
    pub fn slot_drop_cue_json(&mut self, x: f64, y: f64, alt: bool, shift: bool) -> String {
        let at = self.session.view().screen_to_document(x, y);
        serde_json::to_string(&self.session.slot_drop_cue(at, alt, shift)).unwrap_or_default()
    }

    /// Gives the drag in flight its source once it is read: the unit's size and its content
    /// outlines (the fitted ghost).
    #[allow(clippy::needless_pass_by_value)]
    pub fn set_slot_drop_source(
        &mut self,
        source: &WasmLibrarySource,
        roots: &str,
        page: Option<String>,
    ) {
        let roots = ids_of(roots);
        let came_from = empty_record();
        let from = SlotSource {
            source: &source.session,
            roots: &roots,
            page: page.as_deref(),
            came_from: &came_from,
        };
        self.session.set_slot_drop_source(&from);
    }

    /// Ends the slot cue of a library drag.
    pub fn end_slot_drop(&mut self) {
        self.session.end_slot_drop();
    }

    /// Puts the page `page` (or the picks `roots`) of `source` into the slot `slot`, as one
    /// step; with `replace` the slot's content goes in the same step. Returns `{"refused",
    /// "notice"}` or `{"error"}` as JSON.
    #[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
    pub fn put_into_slot(
        &mut self,
        source: &WasmLibrarySource,
        roots: &str,
        page: Option<String>,
        came_from: &str,
        slot: &str,
        real_size: bool,
        replace: bool,
    ) -> String {
        let Ok(record) = serde_json::from_str::<CameFromWire>(came_from) else {
            return error("the record is not readable");
        };
        let Some(slot) = self.session.fixture_part_named(slot) else {
            return error("the slot is gone");
        };
        let record = record.into_record();
        let roots = ids_of(roots);
        let from = SlotSource {
            source: &source.session,
            roots: &roots,
            page: page.as_deref(),
            came_from: &record,
        };
        match self
            .session
            .put_into_slot(&from, slot, mode(real_size), replace)
        {
            Ok(done) => serde_json::to_string(&done).unwrap_or_default(),
            Err(text) => error(&text),
        }
    }

    /// Puts one copy into every empty slot of the holder `holder`, as one step "Fill slots".
    /// Returns `{"refused", "notice"}` or `{"error"}` as JSON.
    #[allow(clippy::needless_pass_by_value)]
    pub fn fill_empty_slots(
        &mut self,
        source: &WasmLibrarySource,
        roots: &str,
        page: Option<String>,
        came_from: &str,
        holder: &str,
        real_size: bool,
    ) -> String {
        let Ok(record) = serde_json::from_str::<CameFromWire>(came_from) else {
            return error("the record is not readable");
        };
        let Some(holder) = self.session.fixture_part_named(holder) else {
            return error("the holder is gone");
        };
        let record = record.into_record();
        let roots = ids_of(roots);
        let from = SlotSource {
            source: &source.session,
            roots: &roots,
            page: page.as_deref(),
            came_from: &record,
        };
        match self
            .session
            .fill_empty_slots(&from, holder, mode(real_size))
        {
            Ok(done) => serde_json::to_string(&done).unwrap_or_default(),
            Err(text) => error(&text),
        }
    }
}
