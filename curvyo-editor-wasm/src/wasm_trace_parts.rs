//! The parts of a composite trace across the wasm boundary (`specs/0094-masked-composite-trace`
//! slice 2): the chips, Add part, a chip pick, Remove part, a part's pixels and the join of the
//! parts' results.

use wasm_bindgen::prelude::*;

use crate::Session;
use crate::wasm_api::WasmSession;
use crate::wasm_trace::{TraceSourceJs, id_of};

/// The chips of the parts the session reads now, as JSON.
fn parts_json(session: &Session) -> String {
    let count = session.trace_part_count();
    let parts: Vec<serde_json::Value> = session
        .trace_parts()
        .iter()
        .enumerate()
        .map(|(index, part)| {
            serde_json::json!({
                "id": part.id,
                "regions": part.regions,
                "outside": part.outside,
                "whole": part.whole,
                "muted": part.muted,
                "active": part.active,
                "contributes": part.contributes,
                "mask": if count > 1 || part.regions > 0 {
                    session.trace_part_mask_json(index)
                } else {
                    String::new()
                },
            })
        })
        .collect();
    serde_json::Value::Array(parts).to_string()
}

#[wasm_bindgen]
impl WasmSession {
    /// The chips of the Parts row as JSON, in order: `[{id, regions, outside, whole, muted,
    /// active, contributes, mask}]`, `mask` the part's clip for the worker's settings (empty for
    /// the whole image, a part that adds nothing, and the one part of an image without regions).
    #[must_use]
    pub fn trace_parts(&self) -> String {
        parts_json(&self.session)
    }

    /// The parts of the selected picture `id` as [`Self::trace_parts`] gives them (several
    /// images selected: each traced with its own stored mask, Frage 11 A); `[]` for an unknown id.
    #[must_use]
    pub fn trace_image_parts(&self, id: &str) -> String {
        let Some(image) = id_of(id) else {
            return "[]".to_owned();
        };
        self.session.for_trace_image(image, parts_json)
    }

    /// Add part: the new part's id, or -1 at 8 parts or without one traced image.
    pub fn add_trace_part(&mut self) -> i32 {
        self.session
            .add_trace_part()
            .and_then(|id| i32::try_from(id).ok())
            .unwrap_or(-1)
    }

    /// A chip pick: part `index` becomes active.
    pub fn set_trace_active_part(&mut self, index: usize) -> bool {
        self.session.set_trace_active_part(index)
    }

    /// Remove part `index`; `false` with one part.
    pub fn remove_trace_part(&mut self, index: usize) -> bool {
        self.session.remove_trace_part(index)
    }

    /// The pixels of part `index` of image `id`, cut to its clip (criterion 31); `specks_mm` is
    /// the part's Ignore below and `upscale_auto` whether its Upscale is Auto.
    #[must_use]
    pub fn trace_part_source(
        &mut self,
        id: &str,
        index: usize,
        reduced: bool,
        specks_mm: f64,
        upscale_auto: bool,
    ) -> Option<TraceSourceJs> {
        let source =
            self.session
                .trace_part_source(id_of(id)?, index, reduced, specks_mm, upscale_auto)?;
        Some(TraceSourceJs {
            width: source.width,
            height: source.height,
            rgba: source.rgba,
            map: source.map.to_vec(),
        })
    }

    /// The parts' results joined as the preview of image `id` (and with `full` kept for Create
    /// paths): `wires` the parts' numbers one after the other, `lengths` how many each has (0 for
    /// a part that adds nothing). The joined result's header numbers, empty when a wire is no
    /// result.
    #[must_use]
    pub fn set_trace_joined(
        &mut self,
        id: &str,
        wires: &[f64],
        lengths: &[u32],
        full: bool,
    ) -> Vec<f64> {
        let Some(id) = id_of(id) else {
            return Vec::new();
        };
        let mut parts: Vec<Option<&[f64]>> = Vec::with_capacity(lengths.len());
        let mut at = 0_usize;
        for &length in lengths {
            let length = length as usize;
            let Some(wire) = wires.get(at..at + length) else {
                return Vec::new();
            };
            parts.push((length > 0).then_some(wire));
            at += length;
        }
        self.session
            .set_trace_joined(id, &parts, full)
            .unwrap_or_default()
    }
}
