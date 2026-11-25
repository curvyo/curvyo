//! The Parameters override of a clone of a sketch and Expose on the sketch page
//! (`specs/0099-sketch-pages` criteria 23, 24, 26, 31, UX notes section 12): the rows as JSON, a
//! value stepped (previewed, nothing written) or committed, Original's, Own and Back, Expose with
//! its limits and name, and the dashed outline of a clone whose values have no solution. The rules
//! live in `curvyo_ui_core::instance_sketch` and document-core; this is the glue to the selection
//! and the history.

use std::collections::HashMap;

use curvyo_document_core::{
    CloneSnapshot, ExposedKey, NodeId, ObjectSnapshot, ValueSet, ViewTransform,
};
use curvyo_render_core::DrawList;
use curvyo_ui_core::instance::InstanceError;
use curvyo_ui_core::instance_sketch;
use serde_json::json;

use super::Session;

/// What a field being stepped has shown: the clones as they would read, and per clone the last
/// value set that solved (saved as the fallback when the commit has no solution).
#[derive(Debug, Default)]
pub(super) struct InstanceState {
    pub(super) previews: Vec<CloneSnapshot>,
    pub(super) last_solvable: HashMap<NodeId, ValueSet>,
    /// A knob drag: the preview is drawn blue over the committed clone instead of in its place
    /// (UX notes 12.3).
    pub(super) overlay: bool,
}

impl Session {
    /// The selected clones of one kind of selection: only clones, linked to a sketch.
    fn sketch_clones(&self) -> Vec<NodeId> {
        let read = self.object_read();
        self.block_clones()
            .into_iter()
            .filter(|id| {
                matches!(read.get(*id), Some(ObjectSnapshot::Clone(clone))
                    if self.document.sketch(clone.chain.root).is_some())
            })
            .collect()
    }

    /// The Parameters row of the clone block as JSON, or `null` when the selection holds no clone
    /// of a sketch that exposes a value (the row is not in the tree).
    #[must_use]
    pub fn sketch_instance_json(&self) -> String {
        let clones = self.sketch_clones();
        if clones.len() != self.selection.ids().len() {
            return "null".to_owned();
        }
        instance_sketch::view(&self.document, &clones, &self.instances.previews)
            .and_then(|view| serde_json::to_string(&view).ok())
            .unwrap_or_else(|| "null".to_owned())
    }

    /// A value stepped into the field `key` (millimetres or degrees), shown without writing: the
    /// clones are drawn as the sketch solved with it. Answers JSON `{"notices": [...],
    /// "no_solution": bool}`, or `{"error": "..."}`. A value without solution keeps the last
    /// solvable preview on the canvas.
    /// `only_this` (Shift at the gesture's start on a value in a link group) changes that value
    /// alone and unlinks it (criterion 34); without it the group's other members follow (33).
    pub fn preview_sketch_instance_value(
        &mut self,
        key: &str,
        number: f64,
        only_this: bool,
    ) -> String {
        let clones = self.sketch_clones();
        if clones.is_empty() {
            return json!({"error": "no sketch clone"}).to_string();
        }
        let mut notices = Vec::new();
        let mut no_solution = false;
        let mut previews = Vec::new();
        for id in clones {
            match instance_sketch::plan(&self.document, id, key, number, only_this) {
                Ok(plan) => {
                    if notices.is_empty() {
                        notices.extend(plan.notices);
                    }
                    if let Some(shown) = plan.preview {
                        self.instances.last_solvable.insert(id, plan.own);
                        previews.push(shown);
                    } else {
                        no_solution = true;
                        previews.extend(
                            self.instances
                                .previews
                                .iter()
                                .find(|shown| shown.id() == id)
                                .cloned(),
                        );
                    }
                }
                Err(_) => return json!({"error": "refused"}).to_string(),
            }
        }
        self.instances.previews = previews;
        json!({"notices": notices, "no_solution": no_solution}).to_string()
    }

    /// Drops the preview in flight: Escape, or a field left without a commit.
    pub fn cancel_sketch_instance_preview(&mut self) {
        self.instances = InstanceState::default();
    }

    /// A value typed or stepped into the field `key`, one step "Change parameter" for every
    /// selected clone. Answers JSON `{"written": bool, "notices": [...]}` or `{"error": "..."}`.
    /// `only_this` as for [`Session::preview_sketch_instance_value`].
    pub fn set_sketch_instance_value(&mut self, key: &str, number: f64, only_this: bool) -> String {
        let clones = self.sketch_clones();
        let last: Vec<(NodeId, ValueSet)> = std::mem::take(&mut self.instances.last_solvable)
            .into_iter()
            .collect();
        self.instances.previews.clear();
        if clones.is_empty() {
            return json!({"error": "no sketch clone"}).to_string();
        }
        let outcome = self.recording(|this| {
            Some(instance_sketch::change(
                &this.document,
                &clones,
                key,
                number,
                only_this,
                &last,
            ))
        });
        match outcome {
            Some(Ok(changed)) => json!({"written": changed.written, "notices": changed.notices}),
            Some(Err(InstanceError::NoSuchObject)) => json!({"error": "no sketch clone"}),
            _ => json!({"error": "refused"}),
        }
        .to_string()
    }

    /// Back to the sketch's value of one field (`key`), or Original's (`key` empty): every own
    /// value of the selected clones dropped, one step. `true` when something was written.
    pub fn reset_sketch_instance(&mut self, key: &str) -> bool {
        let clones = self.sketch_clones();
        self.instances = InstanceState::default();
        self.recording(|this| {
            instance_sketch::reset(&this.document, &clones, (!key.is_empty()).then_some(key))
                .unwrap_or(false)
        })
    }

    /// Own: every value the selected clones show becomes their own, so nothing moves; one step.
    pub fn own_sketch_instance(&mut self) -> bool {
        let clones = self.sketch_clones();
        self.instances = InstanceState::default();
        self.recording(|this| instance_sketch::own_all(&this.document, &clones).unwrap_or(false))
    }

    /// Replaces the clones being previewed in the artwork of this frame by what they would read.
    pub(super) fn apply_instance_preview(&self, objects: &mut [ObjectSnapshot]) {
        if self.instances.previews.is_empty() || self.instances.overlay {
            return;
        }
        for object in objects.iter_mut() {
            if let ObjectSnapshot::Clone(clone) = object
                && let Some(shown) = self
                    .instances
                    .previews
                    .iter()
                    .find(|shown| shown.id() == clone.id())
            {
                *clone = shown.clone();
            }
        }
    }

    /// The blue outline of a knob drag's clone over the committed one (UX notes 12.3: blue new,
    /// black old).
    pub(super) fn instance_preview_overlay(&self, view: ViewTransform) -> DrawList {
        if !self.instances.overlay || self.instances.previews.is_empty() {
            return DrawList::default();
        }
        let paths: Vec<ObjectSnapshot> = self
            .instances
            .previews
            .iter()
            .map(|clone| ObjectSnapshot::Path((*clone.resolved).clone()))
            .collect();
        curvyo_render_core::build_live_edit_preview(&paths, view)
    }

    /// Links the exposed values `a` and `b` of the active sketch page's sketch (criterion 32);
    /// one step "Link parameters". Answers an empty string or the refusal sentence.
    pub fn sketch_link(&mut self, a: &str, b: &str) -> String {
        let (Some(sketch), Some(a), Some(b)) = (
            self.sketch.active,
            ExposedKey::parse(a),
            ExposedKey::parse(b),
        ) else {
            return "These values cannot be linked".to_owned();
        };
        let outcome = self.recording(|this| Some(this.document.link_exposed(sketch, &a, &b)));
        link_refusal(outcome)
    }

    /// Takes the exposed `key` out of its link group (criteria 32, 35); one step "Unlink
    /// parameters", a group left with one member dissolves.
    pub fn sketch_unlink(&mut self, key: &str) -> String {
        let (Some(sketch), Some(key)) = (self.sketch.active, ExposedKey::parse(key)) else {
            return "This value is not linked".to_owned();
        };
        let outcome = self.recording(|this| Some(this.document.unlink_exposed(sketch, &key)));
        link_refusal(outcome)
    }

    /// How many sketch solves the instance cache has run: a diagnostic for the budget test and
    /// the PR.
    #[doc(hidden)]
    #[must_use]
    pub fn instance_solve_count(&self) -> usize {
        self.document.instance_solve_count()
    }

    /// The address of the shared outline of the clone `id` (0 when it is no clone): a diagnostic
    /// for the budget test, which asserts a field drag leaves the other clones' outlines alone.
    #[doc(hidden)]
    #[must_use]
    pub fn object_outline_ptr(&self, id: NodeId) -> Option<usize> {
        match self.object_read().get(id)? {
            ObjectSnapshot::Clone(clone) => {
                Some(std::sync::Arc::as_ptr(&clone.chain.root_outline) as usize)
            }
            _ => None,
        }
    }

    /// Whether a field is being stepped.
    pub(super) fn instance_preview_active(&self) -> bool {
        !self.instances.previews.is_empty()
    }

    /// The dashed outline of every clone that follows a value set without solution, over the
    /// artwork, selected or not (UX notes 12.4); screen only, never exported or cut.
    pub(super) fn no_solution_overlay(objects: &[ObjectSnapshot], view: ViewTransform) -> DrawList {
        let marked: Vec<NodeId> = objects
            .iter()
            .filter_map(|object| match object {
                ObjectSnapshot::Clone(clone) if clone.chain.no_solution => Some(clone.id()),
                _ => None,
            })
            .collect();
        if marked.is_empty() {
            return DrawList::default();
        }
        curvyo_render_core::build_no_solution_outlines(objects, &marked, view)
    }

    /// Exposes the value `key` of the active sketch page's sketch (a dimension takes `name`); one
    /// step "Expose parameter". Answers an empty string or the refusal sentence.
    pub fn sketch_expose(&mut self, key: &str, name: &str) -> String {
        let (Some(sketch), Some(key)) = (self.sketch.active, ExposedKey::parse(key)) else {
            return "Nothing to expose".to_owned();
        };
        let outcome = self.recording(|this| {
            Some(
                this.document
                    .expose_parameter(sketch, &key, (!name.is_empty()).then_some(name)),
            )
        });
        refusal(outcome)
    }

    /// Stops exposing `key`; one step "Stop exposing parameter", every clone's own value for it
    /// goes with it (Undo brings them back).
    pub fn sketch_unexpose(&mut self, key: &str) -> String {
        let (Some(sketch), Some(key)) = (self.sketch.active, ExposedKey::parse(key)) else {
            return "Nothing to stop".to_owned();
        };
        let outcome = self.recording(|this| Some(this.document.unexpose_parameter(sketch, &key)));
        refusal(outcome)
    }

    /// Sets the limits (empty: none) and the name of the exposed `key`; one step "Change exposed
    /// parameter". Answers an empty string or the refusal sentence ("Name in use").
    pub fn sketch_exposed_limits(&mut self, key: &str, min: &str, max: &str, name: &str) -> String {
        let (Some(sketch), Some(key)) = (self.sketch.active, ExposedKey::parse(key)) else {
            return "Nothing to change".to_owned();
        };
        let number = |text: &str| text.trim().parse::<f64>().ok();
        let outcome = self.recording(|this| {
            Some(this.document.set_exposed_limits(
                sketch,
                &key,
                number(min),
                number(max),
                (!name.is_empty()).then_some(name),
            ))
        });
        refusal(outcome)
    }

    /// How many clones of `sketch` hold an own value for `key`, for the Stop exposing tooltip.
    pub(super) fn clones_holding(&self, sketch: NodeId, key: &str) -> usize {
        let Some(key) = ExposedKey::parse(key) else {
            return 0;
        };
        self.object_read()
            .objects
            .iter()
            .filter(|object| {
                matches!(object, ObjectSnapshot::Clone(clone)
                    if clone.chain.root == sketch && clone.chain.own_values.get(&key).is_some())
            })
            .count()
    }
}

fn link_refusal(outcome: Option<Result<(), curvyo_document_core::InstanceError>>) -> String {
    match outcome {
        Some(Ok(())) => String::new(),
        Some(Err(_)) | None => "These values cannot be linked".to_owned(),
    }
}

fn refusal(outcome: Option<Result<(), curvyo_document_core::InstanceError>>) -> String {
    match outcome {
        Some(Ok(())) => String::new(),
        Some(Err(curvyo_document_core::InstanceError::BadName)) => "Name in use".to_owned(),
        Some(Err(_)) | None => "This value cannot be exposed".to_owned(),
    }
}
