//! The numeric entry chip of a sketch-clone knob (`specs/0099-sketch-pages` criterion 25, UX notes
//! 12.3 and 12.9.4): a double-click on a knob opens one field with the value's name; Enter commits
//! one step "Change parameter" coupled through the sketch's links (an unlinked value alone),
//! Escape writes nothing.

use curvyo_document_core::{NodeId, Point};
use curvyo_ui_core::instance_handles::KnobMotion;
use curvyo_ui_core::instance_sketch;
use serde_json::json;

use super::Session;

/// An open knob entry chip.
#[derive(Debug, Clone)]
pub(super) struct KnobEntry {
    id: NodeId,
    key: String,
    label: String,
    angle: bool,
    value: f64,
    at: Point,
    /// `link` (coupled), `unlink` (held alone) or empty: the mark after the label.
    glyph: &'static str,
}

impl Session {
    /// A double-click at `point` on a knob of the selected sketch clone opens its entry chip;
    /// `true` when it did (the knob wins over Select original, criterion 13).
    pub(super) fn sketch_knob_open_entry(&mut self, point: Point) -> bool {
        let knobs = self.visible_sketch_knobs();
        let Some(index) = self.knob_at(&knobs, point) else {
            return false;
        };
        let Some(&id) = self.selection.ids().first() else {
            return false;
        };
        let _ = self.cancel_sketch_knob_drag();
        let knob = &knobs[index];
        let glyph = match self.knob_partners(id, &knob.key) {
            (true, partners) if partners.is_empty() => "unlink",
            (true, _) => "link",
            _ => "",
        };
        self.sketch_knobs.entry = Some(KnobEntry {
            id,
            key: knob.key.clone(),
            label: knob.label.clone(),
            angle: matches!(knob.motion, KnobMotion::Around { .. }),
            value: knob.value,
            at: knob.at,
            glyph,
        });
        true
    }

    /// The open knob entry chip as JSON: `key`, `label`, `unit` (`mm` or `deg`), `value`, `glyph`
    /// and the knob's `x`, `y` in canvas CSS pixels; empty while none is open.
    #[must_use]
    pub fn sketch_knob_entry_json(&self) -> String {
        let Some(entry) = self.sketch_knobs.entry.as_ref() else {
            return String::new();
        };
        let (x, y) = self.view().document_to_screen(entry.at);
        json!({
            "key": entry.key,
            "label": entry.label,
            "unit": if entry.angle { "deg" } else { "mm" },
            "value": entry.value,
            "glyph": entry.glyph,
            "x": x,
            "y": y,
        })
        .to_string()
    }

    /// Enter in the knob entry chip: the typed number becomes the clone's value, coupled through
    /// the links, one step "Change parameter", and the chip closes. Answers JSON `{"written": bool,
    /// "notices": [...]}`, or `{"error": "number"}` for text that is no number (the chip stays).
    pub fn commit_sketch_knob_entry(&mut self, text: &str) -> String {
        let Some(entry) = self.sketch_knobs.entry.clone() else {
            return json!({"error": "closed"}).to_string();
        };
        let Some(number) = text
            .trim()
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
        else {
            return json!({"error": "number"}).to_string();
        };
        self.sketch_knobs.entry = None;
        let outcome = self.recording(|this| {
            Some(instance_sketch::change(
                &this.document,
                &[entry.id],
                &entry.key,
                number,
                false,
                &[],
            ))
        });
        match outcome {
            Some(Ok(changed)) => json!({"written": changed.written, "notices": changed.notices}),
            _ => json!({"error": "refused"}),
        }
        .to_string()
    }

    /// Escape, a blur or a press elsewhere: the knob entry chip closes and nothing is written.
    pub fn cancel_sketch_knob_entry(&mut self) {
        self.sketch_knobs.entry = None;
    }
}
