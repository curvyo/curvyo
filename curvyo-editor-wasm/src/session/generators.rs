//! The Generators tool and the Generator block of the Properties panel
//! (`specs/0141-parametric-generators` criteria 1 to 7, UX notes sections 1, 2, 3, 11 and 12): the
//! chosen kind, placing at the pointer, the block's rows as JSON, a value typed into a row, Convert
//! to paths and the notice of a file with a generator this build does not have. The rules live in
//! `curvyo_ui_core::instance`; this is the glue to the session's selection and history.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::{
    FontRef, GeneratedRecord, NodeId, ObjectSnapshot, Point, ViewTransform,
};
use curvyo_render_core::DrawList;
use curvyo_ui_core::instance;
use curvyo_ui_core::instance_source;
use serde_json::{Value, json};

use super::object_cache::DocumentRead;
use super::{Session, Tool};

/// The status when a resize of an object from a generator this version does not have is refused
/// (UX notes §12).
const UNKNOWN_RESIZE_NOTICE: &str =
    "This object comes from a generator this version does not have; it cannot be resized.";

/// The status when the generator exists but the maker switched its extension off.
const OFF_RESIZE_NOTICE: &str =
    "This object's generator is switched off under Extensions; it cannot be resized.";

/// The status while the face a code's text is set in is still loading (ADR 0092 §3.4).
const LOADING_FONT_NOTICE: &str = "Loading font\u{2026}";

/// A regenerated outline shown before it is written: a field being stepped or a handle being dragged
/// (`0141` UX notes §7). Nothing is in the document until the commit.
#[derive(Debug)]
struct Preview {
    id: NodeId,
    record: GeneratedRecord,
    frame: curvyo_document_core::Affine,
    /// Drawn as the blue outline over the committed object (a handle drag) instead of in place.
    overlay: bool,
    /// The generator answered no parts: the preview draws the last valid outline.
    refused: bool,
}

mod code_fonts;
mod handles;
mod panel;
mod parts;
mod refusal;
mod resize_rules;
mod source;

/// The Generators tool's state.
#[derive(Debug, Default)]
pub(super) struct GeneratorsState {
    /// The stored `<extension>/<kind>` the bar has pressed; empty until the first use.
    pub(in crate::session) kind: String,
    /// The last kind of a family the session placed or switched to (the Code segment places it);
    /// empty until the first use.
    family_kind: String,
    /// A face a pick is about to write to a code, so the funnel has its table before the write.
    extra_font: Option<FontRef>,
    /// The glyph tables of the faces codes are set in, built once per face (`code_fonts.rs`).
    glyph_tables: RefCell<HashMap<(String, u32), Arc<curvyo_extension_core::GlyphTable>>>,
    /// The pointer over the canvas with the tool active, for the ghost.
    hover: Option<Point>,
    /// Whether the notice of an open file with unknown generator objects was not shown yet.
    notice_pending: bool,
    /// The path box whose outline the next pick replaces (the Replace outline button).
    pub(super) replace: Option<NodeId>,
    /// The refusal of the last pick of an outline, for the canvas notice slot, taken once.
    refusal: String,
    /// The drag knobs of the selected object and the drag in flight.
    knobs: handles::KnobState,
    /// The live preview in flight, if any.
    preview: Option<Preview>,
    /// The defaults of the chosen kind, for the ghost: generated once per kind.
    ghost: RefCell<Option<(String, GeneratedRecord)>>,
    /// The generated objects whose stored parameters cannot be drawn (their stored outline is the
    /// last valid one), worked out once per read of the document (`0144` criterion 13).
    unsolved: RefCell<Option<(Arc<DocumentRead>, Vec<NodeId>)>>,
    /// Whether the generator refuses the parameters last seen of an object (a pure function of
    /// them): a commit of this session fills it, so the frame after it generates nothing again.
    refusals: RefCell<HashMap<NodeId, (curvyo_extension_core::StoredParams, bool)>>,
}

/// The render tone of an overlay tone.
fn tone(tone: curvyo_ui_core::path_box_marks::Tone) -> curvyo_render_core::MarkTone {
    use curvyo_render_core::MarkTone;
    use curvyo_ui_core::path_box_marks::Tone;
    match tone {
        Tone::Accent => MarkTone::Accent,
        Tone::Stripe => MarkTone::Stripe,
        Tone::Warning => MarkTone::Warning,
        Tone::OnDisc => MarkTone::OnDisc,
    }
}

impl Session {
    /// The kinds the Generators bar offers, as JSON `[{id, label, icon}]` in registry order; a
    /// family is one entry that places the last kind of it the session used (ADR 0092 §2), and
    /// `loading` says its placed kind waits for the included sans.
    #[must_use]
    pub fn generator_kinds_json(&self) -> String {
        let mut seen: Vec<&'static str> = Vec::new();
        let kinds: Vec<Value> = instance::enabled_kinds(&self.manufacturing.extensions)
            .iter()
            .filter(|found| {
                found.kind.family.is_none_or(|family| {
                    let first = !seen.contains(&family.id);
                    seen.push(family.id);
                    first
                })
            })
            .map(|found| {
                let stored = format!("{}/{}", found.manifest.id, found.kind.id);
                // A family entry places the last kind of the family the session used.
                let placed = found
                    .kind
                    .family
                    .filter(|_| !self.generators.family_kind.is_empty())
                    .and_then(|_| {
                        instance::find_kind(
                            &self.manufacturing.extensions,
                            &self.generators.family_kind,
                        )
                    })
                    .unwrap_or_else(|| found.clone());
                let placed_id = format!("{}/{}", placed.manifest.id, placed.kind.id);
                json!({
                    "id": placed_id,
                    "label": placed.kind.label,
                    // What the segment shows: the command of a kind built from an outline, the
                    // family's segment ("Code").
                    "segment": found.kind.segment(),
                    "source": found.kind.source.is_some(),
                    "icon": placed.kind.icon,
                    "family": found.kind.family.map(|family| family.id),
                    "loading": self.included_sans_loading(&stored)
                        || self.included_sans_loading(&placed_id),
                })
            })
            .collect();
        Value::Array(kinds).to_string()
    }

    /// The kind the tool has armed (`<extension>/<kind>`), empty while none is: entering the tool
    /// arms nothing, a press on a segment arms it (`0151` slice 1). A kind switched off while armed
    /// reads as none.
    #[must_use]
    pub fn generator_kind(&self) -> String {
        if instance::find_kind(&self.manufacturing.extensions, &self.generators.kind).is_some() {
            self.generators.kind.clone()
        } else {
            String::new()
        }
    }

    /// The bar's choice: arms `kind`, moving the arming from another. An unknown kind is ignored.
    pub fn set_generator_kind(&mut self, kind: &str) {
        if instance::find_kind(&self.manufacturing.extensions, kind).is_some() {
            kind.clone_into(&mut self.generators.kind);
            self.remember_family_kind(kind);
        }
    }

    /// A press on a segment of the bar: arms `kind`, or releases it when it is the armed one (or,
    /// for the Code segment, of the armed family). `kind` is what the segment carries.
    pub fn toggle_generator_kind(&mut self, kind: &str) {
        let armed = self.generator_kind();
        let family = |id: &str| {
            instance::find_kind(&self.manufacturing.extensions, id)
                .and_then(|found| found.kind.family)
                .map(|family| family.id)
        };
        let same = !armed.is_empty()
            && (armed == kind || family(&armed).is_some_and(|f| family(kind) == Some(f)));
        if same {
            self.generators.kind.clear();
        } else {
            self.set_generator_kind(kind);
        }
    }

    /// Releases the armed generator, `true` when one was armed (the first step of Escape).
    pub(in crate::session) fn disarm_generator(&mut self) -> bool {
        !std::mem::take(&mut self.generators.kind).is_empty()
    }

    /// The pointer over the canvas with the tool active.
    pub(in crate::session) fn generators_hover(&mut self, point: Option<Point>) {
        self.generators.hover = point;
    }

    /// A release of the tool at `point`: places the chosen kind there.
    pub(in crate::session) fn generators_pointer_up(&mut self, point: Point) {
        let _ = self.place_generated_at(point);
    }

    /// Places the armed kind with its centre at `(x, y)` in document millimetres, selected, in
    /// one step "Add gear" / "Add box". The kind stays armed, so several can be placed (customer
    /// decision 2026-11-27 **(Proposal)**, over `0151` criterion 5). `false` when nothing was
    /// placed, which includes no kind armed.
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
        if let Some(kind) = self.source_kind() {
            // A refusal also answers with `ToolChanged`, so the host redraws and shows it.
            return if self.pick_source(&kind, None) || !self.generators.refusal.is_empty() {
                super::KeyOutcome::ToolChanged
            } else {
                super::KeyOutcome::Ignored
            };
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
        if let Some(kind) = self.source_kind() {
            return self.pick_source(&kind, Some(at));
        }
        let kind = self.generator_kind();
        if kind.is_empty() {
            return false;
        }
        // A kind with text waits for the included sans (ADR 0092 §3.4): nothing is written yet.
        if self.included_sans_loading(&kind) {
            return false;
        }
        self.publish_code_glyphs(true);
        let name = self
            .kind_label(&kind)
            .map(|label| format!("Add {}", instance::noun(label)));
        let placed: Option<NodeId> = self.recording_named_guarded(name, |this| {
            instance::place(&this.document, &this.manufacturing.extensions, &kind, at).ok()
        });
        let Some(id) = placed else {
            return false;
        };
        self.selection.set(&[id]);
        true
    }

    /// The corner crosses and the bent stretches of a path box: on the selected box's inner base,
    /// or on the outline under the pointer in the pick mode (`0149` UX notes §3), in the ghost
    /// look; the hovered outline itself is drawn blue.
    pub(in crate::session) fn path_box_marks_list(&self, view: ViewTransform) -> DrawList {
        let pixel = 1.0 / view.scale();
        let mut overlay = curvyo_ui_core::path_box_marks::Overlay::default();
        let mut hovered: Option<ObjectSnapshot> = None;
        let selected = match self.selection.ids() {
            &[id] => self.document.generated(id),
            _ => None,
        };
        if let Some(snapshot) = selected
            && instance::draws_marks(&self.manufacturing.extensions, &snapshot)
            && let Some(marks) = instance::marks_of(&self.manufacturing.extensions, &snapshot)
        {
            let (frame, shift) = (snapshot.frame, marks.footprint);
            let base = move |p: Point| frame.apply_point(Point::new(p.x + shift.x, p.y + shift.y));
            let part = move |p: Point| frame.apply_point(p);
            overlay = curvyo_ui_core::path_box_marks::overlay(&marks, &base, &part, pixel, true);
        } else if self.tool == Tool::Generators
            && let (Some(kind), Some(at)) = (self.source_kind(), self.generators.hover)
        {
            let objects = self.objects();
            if let Some(id) = self.outline_under(Some(at))
                && let Ok((marks, centre)) = instance_source::hover_marks(
                    &self.document,
                    &self.manufacturing.extensions,
                    &kind,
                    id,
                )
            {
                let map = move |p: Point| Point::new(p.x + centre.x, p.y + centre.y);
                overlay = curvyo_ui_core::path_box_marks::overlay(&marks, &map, &map, pixel, false);
                hovered = objects.iter().find(|o| o.id() == id).cloned();
            }
        }
        let mut list = DrawList::default();
        if let Some(object) = hovered {
            list.extend(curvyo_render_core::build_live_edit_preview(&[object], view));
        }
        list.extend(curvyo_render_core::build_marks_overlay(
            &overlay
                .lines
                .iter()
                .map(|&(a, b, t)| (a, b, tone(t)))
                .collect::<Vec<_>>(),
            &overlay
                .discs
                .iter()
                .map(|&(p, d, t)| (p, d, tone(t)))
                .collect::<Vec<_>>(),
            &overlay
                .rings
                .iter()
                .map(|&(p, d, t)| (p, d, tone(t)))
                .collect::<Vec<_>>(),
            pixel,
        ));
        list
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
        let mut grounds = Vec::new();
        let converted = self.recording_named_guarded(name, |this| {
            let mut all = true;
            this.document.batch(
                curvyo_document_core::CONVERT_GENERATED_COMMIT_LABEL,
                |document| {
                    for id in &ids {
                        match instance::convert(document, &mut this.minter, *id) {
                            Ok(ground) => grounds.extend(ground),
                            Err(_) => all = false,
                        }
                    }
                    all
                },
            )
        });
        let mut leaves = Vec::new();
        for id in &ids {
            leaves.extend(self.document.leaves_of(*id));
        }
        leaves.extend(grounds);
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
        let refusal = std::mem::take(&mut self.generators.refusal);
        if !refusal.is_empty() {
            return refusal;
        }
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
        // No ghost for a kind built from an outline: the box lands beside the outline, not under
        // the pointer (`0149` UX notes §2).
        if self.source_kind().is_some() {
            return DrawList::default();
        }
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
            let Some(record) = instance::new_ghost_record(&found) else {
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
