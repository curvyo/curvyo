//! Constraint inspection links stable relationships to their current analytic geometry.
use super::Session;
use curvyo_document_core::SketchSnapshot;
use curvyo_sketch_core::ConstraintId;
use serde::Serialize;
/// An annotation's screen placement is computed by Rust, never by the host.
#[derive(Debug, Clone, Default, Serialize)]
#[allow(clippy::struct_excessive_bools)] // Independent serialized presentation flags, not a domain state machine.
pub struct SketchAnnotation {
    /// Stable relationship identity.
    pub id: String,
    /// CSS-pixel horizontal label centre.
    pub x: f64,
    /// CSS-pixel vertical label centre.
    pub y: f64,
    /// Measured value or relationship name.
    pub text: String,
    /// Whether the relationship opens a numeric entry.
    pub dimension: bool,
    /// Whether inspection currently selects this relationship.
    pub selected: bool,
    /// Transient placement feedback, not an inspectable stored relationship.
    pub preview: bool,
    /// Proven conflicting relationship, distinct from numerical failure.
    pub conflicting: bool,
    /// Redundant relationship without a proven conflict.
    pub redundant: bool,
    /// Screen-reader name; empty for transient feedback, which reads its text.
    pub name: String,
    /// The pointer is over the dimension.
    pub hovered: bool,
    /// A reference dimension (parentheses and dashed lines).
    pub reference: bool,
    /// The value comes from a formula or named parameter: an "fx" prefix.
    pub fx: bool,
    /// The on-canvas entry box edits this dimension; its text is hidden.
    pub editing: bool,
    /// A pending constraint drawn as its symbol badge in the preview state, named by `text`
    /// (UX notes revision 2, section 5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glyph: Option<&'static str>,
}

impl Session {
    /// Selects a relationship and optionally opens its numeric entry in Sketches.
    pub fn sketch_inspect_constraint(&mut self, id: &ConstraintId, edit: bool) -> bool {
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return false;
        };
        let Some(constraint) = snapshot.graph.constraints.get(id) else {
            return false;
        };
        self.sketch.constraints.clear();
        self.sketch.constraints.insert(id.clone());
        self.sketch.entities = constraint
            .relation
            .entity_ids()
            .into_iter()
            .filter(|id| snapshot.graph.entities.contains_key(*id))
            .cloned()
            .collect();
        self.sketch.edit_constraint =
            (edit && constraint.relation.expression().is_some()).then(|| id.clone());
        self.sketch_reveal_selection();
        self.press_panel_tab("sketches");
        true
    }

    /// Selects an existing analytic entity from the Properties inspection list.
    pub fn sketch_select_entity(
        &mut self,
        id: &curvyo_sketch_core::EntityId,
        extend: bool,
    ) -> bool {
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return false;
        };
        if !snapshot.graph.entities.contains_key(id)
            || super::view::internal_focus(&snapshot.graph, id)
        {
            return false;
        }
        self.sketch.constraints.clear();
        self.sketch.edit_constraint = None;
        if !extend {
            self.sketch.entities.clear();
        }
        if !self.sketch.entities.insert(id.clone()) {
            self.sketch.entities.remove(id);
        }
        true
    }

    #[cfg(test)]
    pub(super) fn sketch_annotations(&self) -> Vec<SketchAnnotation> {
        let Some(snapshot) = self.sketch.active.and_then(|id| self.document.sketch(id)) else {
            return Vec::new();
        };
        self.sketch_annotations_for(&snapshot)
    }

    pub(super) fn sketch_annotations_for(
        &self,
        snapshot: &SketchSnapshot,
    ) -> Vec<SketchAnnotation> {
        let mut annotations: Vec<SketchAnnotation> = self
            .sketch_dimension_records(snapshot)
            .into_iter()
            .map(|record| SketchAnnotation {
                editing: self.sketch.canvas_edit.as_ref() == Some(&record.id),
                id: record.id.0,
                x: record.screen.0,
                y: record.screen.1,
                text: record.text,
                name: record.name,
                dimension: true,
                selected: record.selected,
                hovered: record.hovered,
                reference: record.reference,
                fx: record.fx,
                conflicting: record.conflicting,
                redundant: record.redundant,
                preview: false,
                glyph: None,
            })
            .collect();
        if let Some(preview) = self.sketch_dimension_annotation(snapshot) {
            annotations.push(preview);
        }
        annotations
    }
}
