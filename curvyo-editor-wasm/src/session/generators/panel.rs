//! The Generator block of the Properties panel as the session serves it: the selected object's
//! rows as JSON, a value typed or stepped into a row, and the live preview of a stepped value
//! (`specs/0141-parametric-generators` UX notes §3 and §7).

use curvyo_document_core::{NodeId, ObjectSnapshot};
use curvyo_ui_core::instance::{self, InstanceError};
use serde_json::{Value, json};

use super::Preview;
use crate::session::Session;

impl Session {
    /// The generated objects of the selection.
    pub(in crate::session) fn selected_generated(&self) -> Vec<NodeId> {
        self.selection
            .ids()
            .iter()
            .copied()
            .filter(|id| self.document.is_generated(*id))
            .collect()
    }

    /// The Generator block of the Style tab: JSON of the one selected generated object (kind,
    /// whether its generator is available and its rows with their values), or `null` for no
    /// selection, several objects or another kind of object (UX notes §10 keeps one kind per
    /// block; mixed selections show none in this first slice).
    #[must_use]
    pub fn generated_panel_json(&self) -> String {
        let selected = self.selected_generated();
        let [id] = selected[..] else {
            return "null".to_owned();
        };
        // The face the text is set in, for the drag preview that regenerates outside a write.
        self.publish_code_glyphs(false);
        if self.selection.ids().len() != 1 {
            return "null".to_owned();
        }
        let Some(mut snapshot) = self.document.generated(id) else {
            return "null".to_owned();
        };
        // The fields follow a preview in flight (Pitch diameter while the Module is stepped).
        if let Some(preview) = self.generators.preview.as_ref().filter(|p| p.id == id) {
            snapshot.record = preview.record.clone();
        }
        let view = instance::view(&self.manufacturing.extensions, &snapshot);
        let rows = self.parts_rows(id, &view.parts);
        let Ok(mut value) = serde_json::to_value(&view) else {
            return "null".to_owned();
        };
        value["parts"] = Value::Array(rows);
        value["contrast_low"] = Value::Bool(self.code_contrast_is_low(id, &snapshot.record.kind));
        value["family"] = self.family_json(&snapshot.record.kind);
        // The face the text is set in is not loaded yet, or missing on this computer (ADR 0092 §3).
        value["loading"] = Value::Bool(self.code_loading(id));
        value["font_missing"] = Value::Bool(self.code_font_missing(id));
        value.to_string()
    }

    /// The Symbology group of the block: the family's heading, its kinds in manifest order and the
    /// current one; `null` for a kind without a family.
    fn family_json(&self, kind: &str) -> Value {
        let Some(family) = instance::find_kind(&self.manufacturing.extensions, kind)
            .and_then(|found| found.kind.family)
        else {
            return Value::Null;
        };
        let kinds: Vec<Value> = instance::enabled_kinds(&self.manufacturing.extensions)
            .iter()
            .filter(|found| found.kind.family == Some(family))
            .map(|found| {
                json!({
                    "id": format!("{}/{}", found.manifest.id, found.kind.id),
                    "label": found.kind.label,
                })
            })
            .collect();
        json!({"label": family.label, "current": kind, "kinds": kinds})
    }

    /// Whether the fill of the code `id` is below 3 : 1 against the document background, so the
    /// Colours group says "Low contrast, may not scan" (`0144` criterion 9). Never for another kind
    /// of object, on a page that is not a drawing page (colours there are operations, `0101`) or
    /// while the background is not painted.
    fn code_contrast_is_low(&self, id: NodeId, kind: &str) -> bool {
        if !instance::find_kind(&self.manufacturing.extensions, kind)
            .is_some_and(|found| found.kind.scanned)
            || !self.active_page().kind.is_drawing()
        {
            return false;
        }
        let background = self.shown_background();
        if background.paint == curvyo_document_core::BackgroundPaint::None {
            return false;
        }
        let Some(ObjectSnapshot::Path(path)) = self.document.object(id) else {
            return false;
        };
        curvyo_ui_core::code_contrast::is_low(&path.style.fill, background.color)
    }

    /// A value typed or stepped into the row `key` of the block, as JSON text (`12.5`, `true` or
    /// `"spokes"`): the generator clamps and derives, one step. Answers JSON
    /// `{"written": bool, "notices": [...]}`, or `{"error": "..."}`.
    pub fn set_generated_value(&mut self, key: &str, value: &str) -> String {
        self.generators.preview = None;
        let selected = self.selected_generated();
        let [id] = selected[..] else {
            return json!({"error": "no generated object"}).to_string();
        };
        // The face the text is set in is a precondition, never a refusal of the content.
        if self.code_loading(id) {
            return json!({"error": "loading"}).to_string();
        }
        let Ok(parsed) = serde_json::from_str::<Value>(value) else {
            return json!({"error": "not a value"}).to_string();
        };
        let Some(snapshot) = self.document.generated(id) else {
            return json!({"error": "no generated object"}).to_string();
        };
        let Some(found) =
            instance::find_kind(&self.manufacturing.extensions, &snapshot.record.kind)
        else {
            return json!({"error": "unknown generator"}).to_string();
        };
        let Some(typed) = found
            .kind
            .params()
            .find(|param| param.key == key)
            .and_then(|param| instance::value_from_json(param, &parsed))
        else {
            return json!({"error": "not a value"}).to_string();
        };
        let name = self.kind_label(&snapshot.record.kind).map(|label| {
            let field = found
                .kind
                .params()
                .find(|p| p.key == key)
                .map_or(key, |p| p.label);
            // Reset of a thickness that follows the wall (a stored 0) says so.
            let follows =
                key.ends_with("_thickness") && typed.length().is_some_and(|l| l.as_mm() == 0.0);
            if follows {
                format!("{label}: {field} follows wall thickness")
            } else {
                format!("{label}: {field}")
            }
        });
        let outcome = self.recording_named_guarded(name, |this| {
            Some(instance::change_value(
                &this.document,
                &this.manufacturing.extensions,
                id,
                key,
                &typed,
            ))
        });
        match outcome {
            Some(Ok(changed)) => {
                // The frame after this commit generates nothing again to know the answer.
                if let Some(found) = self.document.generated_params(id) {
                    let params = found.params;
                    self.generators
                        .refusals
                        .borrow_mut()
                        .insert(id, (params, changed.refused));
                }
                json!({"written": changed.written, "notices": changed.notices})
            }
            Some(Err(InstanceError::UnknownKind)) => json!({"error": "unknown generator"}),
            _ => json!({"error": "refused"}),
        }
        .to_string()
    }

    /// A value stepped into the row `key` of the block, shown without writing: the object is drawn
    /// as the generator would rebuild it (UX notes §7). The key-up commits through
    /// [`Session::set_generated_value`]; [`Session::cancel_generated_preview`] drops it. Answers
    /// JSON `{"notices": [...]}` or `{"error": "..."}`.
    pub fn preview_generated_value(&mut self, key: &str, value: &str) -> String {
        let selected = self.selected_generated();
        let [id] = selected[..] else {
            return json!({"error": "no generated object"}).to_string();
        };
        if self.code_loading(id) {
            return json!({"error": "loading"}).to_string();
        }
        self.publish_code_glyphs(false);
        let Ok(parsed) = serde_json::from_str::<Value>(value) else {
            return json!({"error": "not a value"}).to_string();
        };
        let typed = self
            .document
            .generated(id)
            .and_then(|snapshot| {
                instance::find_kind(&self.manufacturing.extensions, &snapshot.record.kind)
            })
            .and_then(|found| {
                found
                    .kind
                    .params()
                    .find(|param| param.key == key)
                    .and_then(|param| instance::value_from_json(param, &parsed))
            });
        let Some(typed) = typed else {
            return json!({"error": "not a value"}).to_string();
        };
        match instance::plan_value(
            &self.document,
            &self.manufacturing.extensions,
            id,
            key,
            &typed,
        ) {
            Ok(plan) => {
                self.generators.preview = Some(Preview {
                    id,
                    record: plan.record,
                    frame: plan.snapshot.frame,
                    overlay: false,
                    refused: plan.refused,
                });
                json!({"notices": plan.notices}).to_string()
            }
            Err(_) => json!({"error": "refused"}).to_string(),
        }
    }

    /// Drops the preview in flight: Escape, or a field left without a commit.
    pub fn cancel_generated_preview(&mut self) {
        self.generators.preview = None;
    }

    /// Whether a preview is in flight that draws in place of the committed object.
    pub(in crate::session) fn generated_preview_in_place(&self) -> bool {
        self.generators
            .preview
            .as_ref()
            .is_some_and(|p| !p.overlay && self.selection.contains(p.id))
    }

    /// Replaces the outline of the previewed object in `objects` (the artwork of this frame) by the
    /// rebuilt one, in its own style.
    pub(in crate::session) fn apply_generated_preview(&self, objects: &mut [ObjectSnapshot]) {
        let Some(preview) = self.generators.preview.as_ref().filter(|p| !p.overlay) else {
            return;
        };
        for object in objects.iter_mut() {
            let ObjectSnapshot::Path(path) = object else {
                continue;
            };
            if path.id != preview.id {
                continue;
            }
            let snapshot = curvyo_document_core::GeneratedSnapshot {
                id: preview.id,
                record: preview.record.clone(),
                frame: preview.frame,
            };
            if let Some(shown) =
                curvyo_ui_core::instance_preview::outline_path(&snapshot, path.style.clone())
            {
                *path = shown;
            }
        }
    }
}
