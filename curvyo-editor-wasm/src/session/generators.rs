//! The Generators tool and the Generator block of the Properties panel
//! (`specs/0141-parametric-generators` criteria 1 to 7, UX notes sections 1, 2, 3, 11 and 12): the
//! chosen kind, placing at the pointer, the block's rows as JSON, a value typed into a row, Convert
//! to paths and the notice of a file with a generator this build does not have. The rules live in
//! `curvyo_ui_core::instance`; this is the glue to the session's selection and history.

use std::cell::RefCell;

use curvyo_document_core::{GeneratedRecord, NodeId, ObjectSnapshot, Point, ViewTransform};
use curvyo_render_core::DrawList;
use curvyo_ui_core::instance::{self, InstanceError};
use serde_json::{Value, json};

use super::{Session, Tool};

/// The kind the tool places until the maker picks another: Box, the first of the registry.
const FIRST_KIND: &str = "curvyo.generators/box";

/// The status when a resize of an object from a generator this version does not have is refused
/// (UX notes §12).
const UNKNOWN_RESIZE_NOTICE: &str =
    "This object comes from a generator this version does not have; it cannot be resized.";

/// The status when the generator exists but the maker switched its extension off.
const OFF_RESIZE_NOTICE: &str =
    "This object's generator is switched off under Extensions; it cannot be resized.";

/// A regenerated outline shown before it is written: a field being stepped or a handle being dragged
/// (`0141` UX notes §7). Nothing is in the document until the commit.
#[derive(Debug)]
struct Preview {
    id: NodeId,
    record: GeneratedRecord,
    frame: curvyo_document_core::Affine,
    /// Drawn as the blue outline over the committed object (a handle drag) instead of in place.
    overlay: bool,
}

mod handles;

/// The Generators tool's state.
#[derive(Debug, Default)]
pub(super) struct GeneratorsState {
    /// The stored `<extension>/<kind>` the bar has pressed; empty until the first use.
    kind: String,
    /// The pointer over the canvas with the tool active, for the ghost.
    hover: Option<Point>,
    /// Whether the notice of an open file with unknown generator objects was not shown yet.
    notice_pending: bool,
    /// The drag knobs of the selected object and the drag in flight.
    knobs: handles::KnobState,
    /// The live preview in flight, if any.
    preview: Option<Preview>,
    /// The defaults of the chosen kind, for the ghost: generated once per kind.
    ghost: RefCell<Option<(String, GeneratedRecord)>>,
}

impl Session {
    /// The kinds the Generators bar offers, as JSON `[{id, label, icon}]` in registry order.
    #[must_use]
    pub fn generator_kinds_json(&self) -> String {
        let kinds: Vec<Value> = instance::enabled_kinds(&self.manufacturing.extensions)
            .iter()
            .map(|found| {
                json!({
                    "id": format!("{}/{}", found.manifest.id, found.kind.id),
                    "label": found.kind.label,
                    "icon": found.kind.icon,
                })
            })
            .collect();
        Value::Array(kinds).to_string()
    }

    /// The kind the tool places, the last choice of the session (Box at first).
    #[must_use]
    pub fn generator_kind(&self) -> String {
        if self.generators.kind.is_empty() {
            FIRST_KIND.to_owned()
        } else {
            self.generators.kind.clone()
        }
    }

    /// The bar's choice. An unknown kind is ignored.
    pub fn set_generator_kind(&mut self, kind: &str) {
        if instance::find_kind(&self.manufacturing.extensions, kind).is_some() {
            kind.clone_into(&mut self.generators.kind);
        }
    }

    /// The pointer over the canvas with the tool active.
    pub(in crate::session) fn generators_hover(&mut self, point: Option<Point>) {
        self.generators.hover = point;
    }

    /// A release of the tool at `point`: places the chosen kind there.
    pub(in crate::session) fn generators_pointer_up(&mut self, point: Point) {
        let _ = self.place_generated_at(point);
    }

    /// Places the chosen kind with its centre at `(x, y)` in document millimetres, selected, in
    /// one step "Add gear" / "Add box", and returns to the Select tool (criterion 1, UX notes §2).
    /// `false` when nothing was placed.
    pub fn place_generated(&mut self, x: f64, y: f64) -> bool {
        self.place_generated_at(Point::new(x, y))
    }

    /// The label of the generator `kind` (`<extension>/<kind>`), `None` for a kind that is not
    /// available.
    fn kind_label(&self, kind: &str) -> Option<&'static str> {
        instance::find_kind(&self.manufacturing.extensions, kind).map(|found| found.kind.label)
    }

    /// The Layers row label of `id`: its generator's label, "Generated object" when the kind is
    /// unavailable, `None` when `id` is no generated object.
    pub(super) fn generated_row_label(&self, id: NodeId) -> Option<String> {
        if !self.document.is_generated(id) {
            return None;
        }
        let kind = self.document.generated(id)?.record.kind;
        Some(
            self.kind_label(&kind)
                .map_or_else(|| "Generated object".to_owned(), str::to_owned),
        )
    }

    /// Enter in the Generators tool: places the chosen kind at the pointer, or at the middle of the
    /// visible canvas when the pointer has not been over it (UX notes §13).
    pub(super) fn place_generated_key(&mut self) -> super::KeyOutcome {
        if self.refuse_drawing_into_layer() {
            return super::KeyOutcome::Ignored;
        }
        let at = self.pointer_position.unwrap_or_else(|| {
            let (width, height) = self.viewport.canvas_size();
            self.viewport.screen_to_document(width / 2.0, height / 2.0)
        });
        if self.place_generated_at(at) {
            super::KeyOutcome::ToolChanged
        } else {
            super::KeyOutcome::Ignored
        }
    }

    fn place_generated_at(&mut self, at: Point) -> bool {
        let kind = self.generator_kind();
        let name = self
            .kind_label(&kind)
            .map(|label| format!("Add {}", label.to_lowercase()));
        let placed: Option<NodeId> = self.recording_named_guarded(name, |this| {
            instance::place(&this.document, &this.manufacturing.extensions, &kind, at).ok()
        });
        let Some(id) = placed else {
            return false;
        };
        self.selection.set(&[id]);
        self.set_tool(Tool::Select);
        true
    }

    /// Whether the selection holds a generated object whose generator is unknown or switched off:
    /// it is kept as stored and cannot be resized or changed by number (ADR 0079 §5).
    pub(in crate::session) fn selection_has_unavailable_generated(&self) -> bool {
        self.selected_generated().iter().any(|id| {
            self.document.generated(*id).is_some_and(|snapshot| {
                instance::find_kind(&self.manufacturing.extensions, &snapshot.record.kind).is_none()
            })
        })
    }

    /// Whether the selection is generated objects and nothing else.
    pub(in crate::session) fn selection_all_generated(&self) -> bool {
        !self.selection.ids().is_empty()
            && self.selected_generated().len() == self.selection.ids().len()
    }

    /// Tells the Select tool whether the next resize keeps the proportions (generated objects only).
    pub(in crate::session) fn sync_generated_resize_mode(&mut self) {
        let proportional = self.selection_all_generated();
        self.select.set_proportional_resize(proportional);
    }

    /// A press on a resize handle, or a size entry, of a selection with an unavailable generated
    /// object is refused with the status of UX notes §12.
    pub(in crate::session) fn refuse_unavailable_generated_resize(&mut self) -> bool {
        let resizing = matches!(
            self.select.dragging_handle(),
            Some(curvyo_ui_core::EditHandle::Resize(_))
        );
        let sizing = self.select.size_entry_open();
        if !(resizing || sizing) || !self.selection_has_unavailable_generated() {
            return false;
        }
        if resizing {
            self.select.escape();
        }
        if sizing {
            self.select.cancel_entry();
        }
        // A kind the registry has, with its extension off, is told apart from one it has not.
        let all_on = curvyo_ui_core::ExtensionSettings::default();
        let unknown = self.selected_generated().iter().any(|id| {
            self.document.generated(*id).is_some_and(|snapshot| {
                instance::find_kind(&self.manufacturing.extensions, &snapshot.record.kind).is_none()
                    && instance::find_kind(&all_on, &snapshot.record.kind).is_none()
            })
        });
        self.transform_notice = Some(if unknown {
            UNKNOWN_RESIZE_NOTICE
        } else {
            OFF_RESIZE_NOTICE
        });
        true
    }

    /// The generated objects of the selection.
    pub(super) fn selected_generated(&self) -> Vec<NodeId> {
        self.selection
            .ids()
            .iter()
            .copied()
            .filter(|id| self.document.generated(*id).is_some())
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
        serde_json::to_string(&instance::view(&self.manufacturing.extensions, &snapshot))
            .unwrap_or_else(|_| "null".to_owned())
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
            format!("{label}: {field}")
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
            Some(Ok(changed)) => json!({"written": changed.written, "notices": changed.notices}),
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

    /// The blue outline of a regenerated object over the committed one while a handle is dragged.
    pub(in crate::session) fn generators_preview_overlay(&self, view: ViewTransform) -> DrawList {
        let Some(preview) = self
            .generators
            .preview
            .as_ref()
            .filter(|p| p.overlay && self.selection.contains(p.id))
        else {
            return DrawList::default();
        };
        let snapshot = curvyo_document_core::GeneratedSnapshot {
            id: preview.id,
            record: preview.record.clone(),
            frame: preview.frame,
        };
        curvyo_ui_core::instance_preview::outline_path(
            &snapshot,
            curvyo_document_core::Style::default(),
        )
        .map_or_else(DrawList::default, |path| {
            curvyo_render_core::build_live_edit_preview(&[ObjectSnapshot::Path(path)], view)
        })
    }

    /// Convert to paths for every generated object of the selection, one step "Convert to paths";
    /// the result is selected: a gear's path, a box's plates. `false` when nothing converted.
    pub fn convert_selected_generated(&mut self) -> bool {
        let ids = self.selected_generated();
        if ids.is_empty() {
            return false;
        }
        // "Convert gear to paths" for one kind, "Convert to paths" for a mix or an unknown kind.
        let mut kinds: Vec<String> = ids
            .iter()
            .filter_map(|id| self.document.generated(*id))
            .map(|snapshot| snapshot.record.kind)
            .collect();
        kinds.dedup();
        let name = match &kinds[..] {
            [kind] => self
                .kind_label(kind)
                .map(|label| format!("Convert {} to paths", label.to_lowercase())),
            _ => None,
        }
        .or_else(|| Some("Convert to paths".to_owned()));
        let converted = self.recording_named_guarded(name, |this| {
            let mut all = true;
            this.document.batch(
                curvyo_document_core::CONVERT_GENERATED_COMMIT_LABEL,
                |document| {
                    for id in &ids {
                        all &= instance::convert(document, &mut this.minter, *id).is_ok();
                    }
                    all
                },
            )
        });
        let mut leaves = Vec::new();
        for id in &ids {
            leaves.extend(self.document.leaves_of(*id));
        }
        self.selection.set(&leaves);
        converted
    }

    /// How many generated objects of the document have a generator this build does not have or
    /// the maker switched off.
    #[must_use]
    pub fn unknown_generated_count(&self) -> usize {
        self.objects_with_generated_kind()
            .iter()
            .filter(|kind| instance::find_kind(&self.manufacturing.extensions, kind).is_none())
            .count()
    }

    fn objects_with_generated_kind(&self) -> Vec<String> {
        self.document
            .read_objects()
            .objects
            .iter()
            .filter_map(|object| self.document.generated(object.id()))
            .map(|snapshot| snapshot.record.kind)
            .collect()
    }

    /// Marks the notice of unknown generator objects as due: called when a document was opened.
    pub(in crate::session) fn arm_generated_notice(&mut self) {
        self.generators.notice_pending = self.unknown_generated_count() > 0;
    }

    /// The status of an opened file with generator objects this build cannot rebuild, taken once
    /// (UX notes §12); empty when there is none or it was shown.
    pub fn take_generated_notice(&mut self) -> String {
        if !std::mem::take(&mut self.generators.notice_pending) {
            return String::new();
        }
        match self.unknown_generated_count() {
            0 => String::new(),
            1 => "1 object comes from a generator this version does not have. It is shown as a plain outline.".to_owned(),
            n => format!("{n} objects come from a generator this version does not have. They are shown as plain outlines."),
        }
    }

    /// The outline of what a release would place, drawn under the pointer in the "Drag ghost"
    /// look: the real outline of the chosen kind at its defaults (UX notes §2).
    pub(in crate::session) fn generators_draw_list(&self, view: ViewTransform) -> DrawList {
        let Some(at) = self.generators.hover.filter(|_| {
            self.tool == Tool::Generators && self.pointer_position.is_some() && !self.button_down
        }) else {
            return DrawList::default();
        };
        let kind = self.generator_kind();
        let mut memo = self.generators.ghost.borrow_mut();
        // The defaults of a kind are generated once, not on every frame of the hover.
        if memo.as_ref().is_none_or(|(held, _)| *held != kind) {
            let Some(found) = instance::find_kind(&self.manufacturing.extensions, &kind) else {
                return DrawList::default();
            };
            let Some((record, _)) = instance::new_record(&found) else {
                return DrawList::default();
            };
            *memo = Some((kind, record));
        }
        let Some((_, record)) = memo.as_ref() else {
            return DrawList::default();
        };
        let Some(path) = curvyo_ui_core::instance_preview::ghost_path(record, at) else {
            return DrawList::default();
        };
        curvyo_render_core::build_drag_ghost(&[ObjectSnapshot::Path(path)], view)
    }
}
