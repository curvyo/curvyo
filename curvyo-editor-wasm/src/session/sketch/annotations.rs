//! Dimension annotations: what each driving or reference dimension measures and where its marks go (AC31).
use super::Session;
use curvyo_document_core::{DisplayUnit, Point, SketchSnapshot, ViewTransform};
use curvyo_sketch_core::{
    Constraint, ConstraintId, Dimension, Expression, Position, Relation, Sketch,
};
use curvyo_ui_core::{DimensionItem, DimensionMarks};

/// Under this many screen pixels of measured geometry a dimension draws only while hovered or
/// selected (UX notes revision 2, section 7, "Small spans").
const HIDDEN_BELOW_PX: f64 = 16.0;
/// Hit target of a dimension text: at least 24px high, the estimated text width wide.
const TEXT_HIT_HEIGHT_PX: f64 = 24.0;
const TEXT_CHAR_PX: f64 = 7.0;
const TEXT_PAD_PX: f64 = 8.0;
/// Height of the 12px text's box.
const TEXT_HEIGHT_PX: f64 = 16.0;
/// Lines are hit within 4px on either side (an 8px band).
const LINE_HIT_PX: f64 = 4.0;

/// One dimension annotation, laid out for the current view.
#[allow(clippy::struct_excessive_bools)] // Independent presentation flags, not a state machine.
pub(super) struct DimensionRecord {
    pub id: ConstraintId,
    /// Canvas text: "50 mm", "R 25 mm", "⌀ 50 mm", "45°", "(50 mm)" for a reference.
    pub text: String,
    /// Screen-reader name: kind, value and, for a formula, the expression.
    pub name: String,
    /// The value comes from a formula or named parameter.
    pub fx: bool,
    pub reference: bool,
    /// Proven conflicting, or the value just typed left the solver without a shape.
    pub conflicting: bool,
    /// Redundant without a proven conflict.
    pub redundant: bool,
    pub selected: bool,
    pub hovered: bool,
    /// The dimension's marks in document coordinates.
    pub marks: DimensionMarks,
    /// Screen position of the text centre, CSS pixels.
    pub screen: (f64, f64),
}

impl DimensionRecord {
    /// Half the estimated text width, screen pixels.
    fn half_width(&self) -> f64 {
        #[allow(clippy::cast_precision_loss)] // a short label's character count
        f64::midpoint(TEXT_CHAR_PX * self.text.chars().count() as f64, TEXT_PAD_PX)
    }

    /// The text's box, which constraint symbols keep clear of.
    pub fn text_box(&self) -> curvyo_ui_core::TextBox {
        curvyo_ui_core::TextBox {
            center: self.marks.text,
            half_width_px: self.half_width(),
            half_height_px: TEXT_HEIGHT_PX / 2.0,
        }
    }

    /// Whether the screen point `(x, y)` is on the text box.
    pub fn hit_text(&self, x: f64, y: f64) -> bool {
        let half_width = self.half_width();
        (x - self.screen.0).abs() <= half_width
            && (y - self.screen.1).abs() <= TEXT_HIT_HEIGHT_PX / 2.0
    }

    /// Whether the screen point `(x, y)` is within 4px of one of the dimension's lines.
    pub fn hit_line(&self, x: f64, y: f64, view: curvyo_document_core::ViewTransform) -> bool {
        self.marks.lines.iter().any(|&(a, b)| {
            let (a, b) = (view.document_to_screen(a), view.document_to_screen(b));
            segment_distance((x, y), a, b) <= LINE_HIT_PX
        })
    }
}

fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.mul_add(dx, dy * dy);
    let t = if length > 0.0 {
        ((p.0 - a.0).mul_add(dx, (p.1 - a.1) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.0 - t.mul_add(dx, a.0)).hypot(p.1 - t.mul_add(dy, a.1))
}

/// A laid-out dimension's record before its marks are known.
#[allow(clippy::struct_excessive_bools)] // The record's presentation flags, carried over.
struct PendingRecord {
    id: ConstraintId,
    text: String,
    name: String,
    fx: bool,
    reference: bool,
    conflicting: bool,
    redundant: bool,
    selected: bool,
    hovered: bool,
}

impl PendingRecord {
    fn placed(self, marks: DimensionMarks, view: ViewTransform) -> DimensionRecord {
        DimensionRecord {
            screen: view.document_to_screen(marks.text),
            id: self.id,
            text: self.text,
            name: self.name,
            fx: self.fx,
            reference: self.reference,
            conflicting: self.conflicting,
            redundant: self.redundant,
            selected: self.selected,
            hovered: self.hovered,
            marks,
        }
    }
}

impl Session {
    /// Every visible dimension of `snapshot`, laid out on the current solution and view.
    pub(super) fn sketch_dimension_records(
        &self,
        snapshot: &SketchSnapshot,
    ) -> Vec<DimensionRecord> {
        let solution = self.sketch_preview().unwrap_or(&snapshot.solution);
        let view = self.view();
        let (items, pending): (Vec<_>, Vec<_>) = snapshot
            .graph
            .constraints
            .iter()
            .filter_map(|(id, constraint)| {
                self.sketch_dimension_entry(snapshot, solution, id, constraint)
            })
            .unzip();
        let mut records: Vec<DimensionRecord> = curvyo_ui_core::layout_dimensions(&items, view)
            .into_iter()
            .zip(pending)
            .map(|(marks, pending)| pending.placed(marks, view))
            .collect();
        // Bottom to top: reference, driving, hovered, selected (UX notes, "Overlap").
        records.sort_by_key(|r| (r.selected, r.hovered, !r.reference));
        records
    }

    /// The layout input and record of one constraint, or `None` when it is no visible dimension.
    fn sketch_dimension_entry(
        &self,
        snapshot: &SketchSnapshot,
        solution: &Sketch,
        id: &ConstraintId,
        constraint: &Constraint,
    ) -> Option<(DimensionItem, PendingRecord)> {
        let expression = constraint.relation.expression()?;
        let (geometry, span) = super::dimension_geometry::dimension_geometry(
            solution,
            &constraint.relation,
            snapshot.placement,
        )?;
        let selected = self.sketch.constraints.contains(id);
        let hovered = self.sketch.annotation_hover.as_ref() == Some(id);
        if span * self.view().scale() < HIDDEN_BELOW_PX && !selected && !hovered {
            return None;
        }
        let measured = curvyo_sketch_core::measure(solution, &constraint.relation).ok()?;
        let proven = snapshot.conflicting.contains(id);
        // The value just typed left no shape: marked like a conflict, named as what it is.
        let failed = !proven
            && self.sketch.failed_dimension.as_ref() == Some(id)
            && snapshot.status == Some(curvyo_sketch_core::SolveStatus::SolverFailure);
        let conflicting = proven || failed;
        // The kept shape does not hold a dimension that cannot be met: its text shows the value
        // it asks for, as its entry box does.
        let value = conflicting
            .then(|| snapshot.graph.parameters.evaluate().ok())
            .flatten()
            .and_then(|values| expression.evaluate(&values).ok())
            .unwrap_or(measured);
        let redundant = !conflicting && snapshot.redundant.contains(id);
        let state = if failed {
            Some(", solver failure")
        } else if proven {
            Some(", conflicting")
        } else if redundant {
            Some(", redundant")
        } else {
            None
        };
        let value_text = value_text(
            &constraint.relation,
            value.value(),
            value.dimension(),
            self.display_unit(),
        );
        let fx = !matches!(expression, Expression::Literal(_));
        let formula = expression
            .display_text(&snapshot.graph.parameters)
            .ok()
            .filter(|_| fx);
        let (text, name) = dimension_texts(constraint, &value_text, formula, state);
        let label = self
            .sketch_label_for(snapshot, id)
            .map(|p| snapshot.placement.apply_point(Point::new(p.x.0, p.y.0)));
        let item = DimensionItem {
            geometry,
            label,
            text_chars: text.chars().count(),
        };
        let pending = PendingRecord {
            id: id.clone(),
            text,
            name,
            fx,
            reference: constraint.reference,
            conflicting,
            redundant,
            selected,
            hovered,
        };
        Some((item, pending))
    }

    /// The stored text position of `id`, or the one a label drag in progress shows.
    fn sketch_label_for(&self, snapshot: &SketchSnapshot, id: &ConstraintId) -> Option<Position> {
        match &self.sketch.label_drag {
            Some(drag) if drag.id == *id => Some(drag.current),
            _ => snapshot.labels.get(id).copied(),
        }
    }
}

/// The canvas text ("! " for a `state` such as ", conflicting", parentheses for a reference) and
/// the screen-reader name (kind, value, formula, reference, state) of one dimension.
fn dimension_texts(
    constraint: &Constraint,
    value_text: &str,
    formula: Option<String>,
    state: Option<&str>,
) -> (String, String) {
    let text = format!(
        "{}{}",
        if state.is_some() { "! " } else { "" },
        if constraint.reference {
            format!("({value_text})")
        } else {
            value_text.to_owned()
        }
    );
    let name = format!(
        "{} {value_text}{}{}{}",
        kind_name(&constraint.relation),
        formula
            .map(|f| format!(", formula {f}"))
            .unwrap_or_default(),
        if constraint.reference {
            ", reference"
        } else {
            ""
        },
        state.unwrap_or_default(),
    );
    (text, name)
}

/// The value as the canvas shows it: the display unit, at most the panel's precision, trailing
/// zeros trimmed, with "R " for a radius, "⌀ " for a diameter and "↔ " / "↕ " for a horizontal
/// or vertical distance, shown as its size whatever side the geometry is on.
fn value_text(relation: &Relation, value: f64, dimension: Dimension, unit: DisplayUnit) -> String {
    if dimension == Dimension::Angle {
        return format!("{}°", trimmed(value.to_degrees(), 2));
    }
    let decimals = if unit == DisplayUnit::Mm { 2 } else { 3 };
    let prefix = match relation {
        Relation::Radius { .. } | Relation::EllipseRadius { .. } => "R ",
        Relation::Diameter { .. } => "⌀ ",
        Relation::HorizontalDistance { .. } => "↔ ",
        Relation::VerticalDistance { .. } => "↕ ",
        _ => "",
    };
    let value = if prefix.starts_with(['↔', '↕']) {
        value.abs()
    } else {
        value
    };
    let number = trimmed(value / unit.mm_per_unit(), decimals);
    format!("{prefix}{number} {}", unit.symbol())
}

fn trimmed(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    if text == "-0" {
        "0".into()
    } else {
        text.into()
    }
}

fn kind_name(relation: &Relation) -> &'static str {
    match relation {
        Relation::Distance { .. } => "Distance",
        Relation::HorizontalDistance { .. } => "Horizontal distance",
        Relation::VerticalDistance { .. } => "Vertical distance",
        Relation::Length { .. } => "Length",
        Relation::Radius { .. } | Relation::EllipseRadius { .. } => "Radius",
        Relation::Diameter { .. } => "Diameter",
        Relation::Angle { .. } | Relation::Sweep { .. } | Relation::AxisAngle { .. } => "Angle",
        _ => "Dimension",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_use_the_display_unit_prefixes_and_trimmed_decimals() {
        let radius = Relation::Radius {
            curve: "c".into(),
            value: Expression::Literal(curvyo_sketch_core::Quantity::length(1.0).unwrap()),
        };
        let diameter = Relation::Diameter {
            curve: "c".into(),
            value: Expression::Literal(curvyo_sketch_core::Quantity::length(1.0).unwrap()),
        };
        let length = Relation::Length {
            curve: "l".into(),
            value: Expression::Literal(curvyo_sketch_core::Quantity::length(1.0).unwrap()),
        };
        assert_eq!(
            value_text(&length, 50.0, Dimension::Length, DisplayUnit::Mm),
            "50 mm"
        );
        assert_eq!(
            value_text(&radius, 25.0, Dimension::Length, DisplayUnit::Mm),
            "R 25 mm"
        );
        assert_eq!(
            value_text(&diameter, 50.0, Dimension::Length, DisplayUnit::Mm),
            "⌀ 50 mm"
        );
        assert_eq!(
            value_text(&length, 21.174, Dimension::Length, DisplayUnit::Mm),
            "21.17 mm"
        );
        assert_eq!(
            value_text(&length, 25.4, Dimension::Length, DisplayUnit::In),
            "1 in"
        );
        assert_eq!(
            value_text(
                &length,
                std::f64::consts::FRAC_PI_4,
                Dimension::Angle,
                DisplayUnit::Mm
            ),
            "45°"
        );
    }
}
