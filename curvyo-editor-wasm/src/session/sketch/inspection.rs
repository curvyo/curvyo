//! Constraint inspection links stable relationships to their current analytic geometry.
use super::Session;
use curvyo_document_core::{Point, SketchSnapshot};
use curvyo_sketch_core::{ConstraintId, Entity, Position};
use serde::Serialize;
/// An annotation's screen placement is computed by Rust, never by the host.
#[derive(Debug, Clone, Serialize)]
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
        let solution = self.sketch_preview().unwrap_or(&snapshot.solution);
        let mut annotations: Vec<SketchAnnotation> = snapshot
            .graph
            .constraints
            .iter()
            .filter_map(|(id, constraint)| {
                let mut positions = Vec::new();
                for entity in constraint.relation.entity_ids() {
                    match solution.entities.get(entity) {
                        Some(Entity::Point { position, .. }) => positions.push(*position),
                        Some(entity) => positions.extend(
                            entity
                                .point_ids()
                                .into_iter()
                                .filter_map(|id| solution.point(id).ok()),
                        ),
                        None => {}
                    }
                }
                if positions.is_empty() {
                    return None;
                }
                let sum = positions
                    .iter()
                    .fold((0.0, 0.0), |(x, y), p| (x + p.x.0, y + p.y.0));
                #[allow(clippy::cast_precision_loss)]
                // a collection's point count used only for a display centroid
                let count = positions.len() as f64;
                let center = snapshot
                    .labels
                    .get(id)
                    .copied()
                    .unwrap_or_else(|| Position::mm(sum.0 / count, sum.1 / count));
                let document = snapshot
                    .placement
                    .apply_point(Point::new(center.x.0, center.y.0));
                let screen = self.view().document_to_screen(document);
                let value = curvyo_sketch_core::measure(solution, &constraint.relation).ok();
                let text = value.map_or_else(
                    || super::view::relation_name(&constraint.relation).to_owned(),
                    |value| {
                        let (number, unit) =
                            if value.dimension() == curvyo_sketch_core::Dimension::Angle {
                                (value.value().to_degrees(), "°")
                            } else {
                                (value.value(), " mm")
                            };
                        format!(
                            "{number:.2}{unit}{}",
                            if constraint.reference { " (ref)" } else { "" }
                        )
                    },
                );
                Some(SketchAnnotation {
                    id: id.0.clone(),
                    x: screen.0,
                    y: screen.1
                        - if snapshot.labels.contains_key(id) {
                            0.0
                        } else {
                            18.0
                        },
                    text,
                    dimension: value.is_some(),
                    selected: self.sketch.constraints.contains(id),
                    conflicting: snapshot.conflicting.contains(id),
                    preview: false,
                })
            })
            .collect();
        // Stable constraint order separates coincident labels without changing their
        // geometry anchors. The matching hit-test uses these same screen positions.
        for index in 0..annotations.len() {
            if snapshot
                .labels
                .contains_key(&annotations[index].id.clone().into())
            {
                continue;
            }
            while annotations[..index].iter().any(|other| {
                (other.x - annotations[index].x).abs() < 100.0
                    && (other.y - annotations[index].y).abs() < 18.0
            }) {
                annotations[index].y -= 18.0;
            }
        }
        if let Some(preview) = self.sketch_dimension_annotation(snapshot) {
            annotations.push(preview);
        }
        annotations
    }
    pub(in crate::session) fn sketch_annotation_press(&mut self, point: Point, edit: bool) -> bool {
        let screen = self.view().document_to_screen(point);
        let nearest = self
            .sketch_annotations()
            .into_iter()
            .filter(|annotation| !annotation.preview)
            .map(|annotation| {
                let distance = (annotation.x - screen.0).hypot(annotation.y - screen.1);
                (annotation.id, distance)
            })
            .filter(|(_, distance)| *distance <= 14.0)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        nearest.is_some_and(|(id, _)| self.sketch_inspect_constraint(&id.into(), edit))
    }
}
