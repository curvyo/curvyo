//! Pointer input on dimension texts and constraint symbols: hover, select, drag the text, edit on the canvas.
use super::{Session, SketchTool};
use curvyo_document_core::{Point, SketchSnapshot};
use curvyo_sketch_core::{ConstraintId, Position, Relation};
use serde_json::{Value, json};

/// A press moves this many screen pixels before a text press becomes a drag.
const DRAG_THRESHOLD_PX: f64 = 3.0;

/// A dimension text being dragged; the stored position changes only on release.
pub(in crate::session) struct LabelDrag {
    pub id: ConstraintId,
    pub press: Point,
    /// The text position the drag shows now, sketch-local.
    pub current: Position,
    /// From the press point to the text position, sketch-local millimetres.
    grip: (f64, f64),
    moved: bool,
}

/// What the pointer is over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AnnotationHit {
    pub id: ConstraintId,
    pub dimension: bool,
    /// Hit on a dimension's line rather than its text or a symbol.
    pub line: bool,
}

impl Session {
    /// The topmost dimension text, dimension line or symbol at the document point `point`.
    pub(super) fn sketch_annotation_at(&self, point: Point) -> Option<AnnotationHit> {
        let snapshot = self.sketch.active.and_then(|id| self.document.sketch(id))?;
        let view = self.view();
        let (x, y) = view.document_to_screen(point);
        let dimensions = self.sketch_dimension_records(&snapshot);
        if let Some(record) = dimensions.iter().rev().find(|r| r.hit_text(x, y)) {
            return Some(AnnotationHit {
                id: record.id.clone(),
                dimension: true,
                line: false,
            });
        }
        let (symbols, _) = self.sketch_symbol_records(&snapshot);
        if let Some(record) = symbols.iter().rev().find(|r| r.hit(x, y)) {
            return Some(AnnotationHit {
                id: record.id.clone(),
                dimension: false,
                line: false,
            });
        }
        dimensions
            .iter()
            .rev()
            .find(|r| r.hit_line(x, y, view))
            .map(|r| AnnotationHit {
                id: r.id.clone(),
                dimension: true,
                line: true,
            })
    }

    /// A press on an annotation selects its constraint and its participants (AC22); in Select a
    /// press on a dimension also starts dragging its text.
    pub(in crate::session) fn sketch_annotation_press(&mut self, point: Point) -> bool {
        let Some(hit) = self.sketch_annotation_at(point) else {
            return false;
        };
        if self.sketch_annotation_yields(point, &hit) {
            return false;
        }
        self.sketch.canvas_edit = None;
        if !self.sketch_select_constraint(&hit.id) {
            return false;
        }
        // The pointer is already at the annotation: show its row, never move the view.
        self.press_panel_tab("sketches");
        if hit.dimension && self.sketch.tool == SketchTool::Select {
            self.sketch_start_label_drag(hit.id, point);
        }
        true
    }

    /// Geometry under the pointer wins over an annotation: a text lying over a small sketch never
    /// blocks a drag; a local axis wins over a dimension line crossing it, so the axis stays a
    /// constraint target there (0066 AC52).
    fn sketch_annotation_yields(&self, point: Point, hit: &AnnotationHit) -> bool {
        self.sketch_geometry_at(point)
            || (hit.line
                && self
                    .sketch_local(point)
                    .is_some_and(|local| !self.sketch_axes_at(local).is_empty()))
    }

    /// Whether sketch geometry is within pick reach of `point`.
    fn sketch_geometry_at(&self, point: Point) -> bool {
        let (Ok(snapshot), Some(local)) =
            (self.sketch_snapshot_for_edit(), self.sketch_local(point))
        else {
            return false;
        };
        curvyo_ui_core::sketch_hit(
            &snapshot.graph,
            &snapshot.solution,
            local,
            curvyo_sketch_core::Length(1.0 / self.view().scale()),
        )
        .is_some()
    }

    fn sketch_start_label_drag(&mut self, id: ConstraintId, press: Point) {
        let Some(snapshot) = self.sketch.active.and_then(|id| self.document.sketch(id)) else {
            return;
        };
        let Some(text) = self
            .sketch_dimension_records(&snapshot)
            .into_iter()
            .find(|r| r.id == id)
            .map(|r| r.marks.text)
        else {
            return;
        };
        let (Some(local_text), Some(local_press)) =
            (self.sketch_local(text), self.sketch_local(press))
        else {
            return;
        };
        self.sketch.label_drag = Some(LabelDrag {
            id,
            press,
            current: local_text,
            grip: (
                local_text.x.0 - local_press.x.0,
                local_text.y.0 - local_press.y.0,
            ),
            moved: false,
        });
        self.button_down = true;
    }

    /// Follows the pointer with a dragged text; `false` when no text is dragged.
    pub(super) fn sketch_label_drag_move(&mut self, point: Point) -> bool {
        let scale = self.view().scale();
        let local = self.sketch_local(point);
        let Some(drag) = &mut self.sketch.label_drag else {
            return false;
        };
        if drag.press.vector_to(point).length() * scale > DRAG_THRESHOLD_PX {
            drag.moved = true;
        }
        if drag.moved
            && let Some(local) = local
        {
            drag.current = Position::mm(local.x.0 + drag.grip.0, local.y.0 + drag.grip.1);
        }
        true
    }

    /// Stores a dragged text's new place as one history step; `false` when no text is dragged.
    pub(super) fn sketch_label_drag_release(&mut self) -> bool {
        let Some(drag) = self.sketch.label_drag.take() else {
            return false;
        };
        self.button_down = false;
        if !drag.moved {
            return true;
        }
        let Some(id) = self.sketch.active else {
            return true;
        };
        let result =
            self.recording(|this| this.document.set_sketch_label(id, &drag.id, drag.current));
        self.sketch.error = result.err().map(|e| e.to_string());
        true
    }

    /// Records which annotation the pointer is over, for highlight and cursor.
    pub(super) fn sketch_annotation_hover(&mut self, point: Point) {
        self.sketch.annotation_hover = self
            .sketch_annotation_at(point)
            .filter(|hit| !self.sketch_annotation_yields(point, hit))
            .map(|hit| hit.id);
    }

    /// Whether the hovered annotation is a dimension, whose text can be dragged.
    pub(super) fn sketch_hovering_dimension(&self) -> bool {
        let Some(id) = &self.sketch.annotation_hover else {
            return false;
        };
        self.sketch
            .active
            .and_then(|node| self.document.sketch(node))
            .and_then(|s| {
                s.graph
                    .constraints
                    .get(id)
                    .map(|c| c.relation.expression().is_some())
            })
            .unwrap_or(false)
    }

    /// Selects the constraint `id` and its participants, without moving the view.
    fn sketch_select_constraint(&mut self, id: &ConstraintId) -> bool {
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
            .filter(|e| snapshot.graph.entities.contains_key(*e))
            .cloned()
            .collect();
        self.sketch.edit_constraint = None;
        true
    }

    /// Opens the on-canvas entry box on the dimension `id`, selecting it (AC31, AC33).
    pub fn sketch_edit_dimension(&mut self, id: &ConstraintId) -> bool {
        let is_dimension = self.sketch_snapshot_for_edit().is_ok_and(|s| {
            s.graph
                .constraints
                .get(id)
                .is_some_and(|c| c.relation.expression().is_some())
        });
        if !is_dimension || !self.sketch_select_constraint(id) {
            return false;
        }
        self.sketch.label_drag = None;
        self.sketch.canvas_edit = Some(id.clone());
        true
    }

    /// Opens the entry box on the dimension whose text or line is at `point` (double-click).
    pub fn sketch_edit_dimension_at(&mut self, point: Point) -> bool {
        self.sync_sketch();
        match self.sketch_annotation_at(point) {
            Some(hit) if hit.dimension => self.sketch_edit_dimension(&hit.id),
            _ => false,
        }
    }

    /// Closes the on-canvas dimension edit without a change: a refused value the box kept is
    /// taken back.
    pub fn sketch_close_dimension_edit(&mut self) {
        self.sketch_revert_refused();
        self.sketch.canvas_edit = None;
        self.sketch.entry_error = None;
    }

    /// Escape in a box kept open on a refused value cancels that value: its step is taken back
    /// while it is still the latest (UX notes revision 2 review, change 6; AC20).
    pub(in crate::session) fn sketch_revert_refused(&mut self) {
        let refused = self.sketch.refused_step.take();
        if self.sketch.canvas_edit.is_some() && refused == Some(self.undo_len()) {
            self.undo();
        }
    }

    /// The entry box of the dimension edited on the canvas, anchored at its text.
    pub(super) fn sketch_edit_entry(&self) -> Option<Value> {
        let id = self.sketch.canvas_edit.as_ref()?;
        let snapshot = self.sketch.active.and_then(|n| self.document.sketch(n))?;
        let constraint = snapshot.graph.constraints.get(id)?;
        let expression = constraint.relation.expression()?;
        let record = self
            .sketch_dimension_records(&snapshot)
            .into_iter()
            .find(|r| r.id == *id)?;
        let label = field_label(&constraint.relation);
        let value = match expression {
            // A plain number is millimetres or degrees in the field, as when the dimension was made.
            curvyo_sketch_core::Expression::Literal(q) => {
                let number = if q.dimension() == curvyo_sketch_core::Dimension::Angle {
                    q.value().to_degrees()
                } else {
                    q.value()
                };
                let text = format!("{number:.6}");
                text.trim_end_matches('0').trim_end_matches('.').to_owned()
            }
            other => other
                .display_text(&snapshot.graph.parameters)
                .unwrap_or_default(),
        };
        Some(json!({
            "kind": "edit",
            "action": super::view::relation_name(&constraint.relation),
            "constraint": id.0,
            "group": "Dimension",
            "fields": [{
                "label": label,
                "name": format!("Sketch {label}"),
                "suffix": if label == "Angle" { "°" } else { "mm" },
                "value": value,
            }],
            "x": record.screen.0,
            "y": record.screen.1,
            "away_x": 1.0,
            "away_y": 1.0,
            "pointer_x": record.screen.0,
            "pointer_y": record.screen.1,
            "reference": true,
            "reference_on": constraint.reference,
            "error": self.sketch_entry_error(),
        }))
    }

    /// The constraint ids whose participants the hovered annotation highlights.
    pub(super) fn sketch_hover_participants(
        &self,
        snapshot: &SketchSnapshot,
    ) -> Vec<curvyo_sketch_core::EntityId> {
        self.sketch
            .annotation_hover
            .as_ref()
            .and_then(|id| snapshot.graph.constraints.get(id))
            .map(|c| c.relation.entity_ids().into_iter().cloned().collect())
            .unwrap_or_default()
    }
}

fn field_label(relation: &Relation) -> &'static str {
    match relation {
        Relation::Radius { .. } | Relation::EllipseRadius { .. } => "Radius",
        Relation::Diameter { .. } => "Diameter",
        Relation::Distance { .. }
        | Relation::HorizontalDistance { .. }
        | Relation::VerticalDistance { .. } => "Distance",
        Relation::Angle { .. } | Relation::Sweep { .. } | Relation::AxisAngle { .. } => "Angle",
        _ => "Length",
    }
}
