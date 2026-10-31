//! Dimension annotations: what each driving or reference dimension measures and where its marks go (AC31).
use super::Session;
use curvyo_document_core::{Affine, DisplayUnit, Point, SketchSnapshot, Vec2};
use curvyo_sketch_core::{
    ConstraintId, Dimension, Entity, EntityId, Expression, Position, Relation, Sketch,
};
use curvyo_ui_core::{DimensionGeometry, DimensionItem, DimensionMarks};

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
    pub conflicting: bool,
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

impl Session {
    /// Every visible dimension of `snapshot`, laid out on the current solution and view.
    pub(super) fn sketch_dimension_records(
        &self,
        snapshot: &SketchSnapshot,
    ) -> Vec<DimensionRecord> {
        let solution = self.sketch_preview().unwrap_or(&snapshot.solution);
        let view = self.view();
        let unit = self.display_unit();
        let mut pending = Vec::new();
        let mut items = Vec::new();
        for (id, constraint) in &snapshot.graph.constraints {
            let Some(expression) = constraint.relation.expression() else {
                continue;
            };
            let Some((geometry, span)) =
                dimension_geometry(solution, &constraint.relation, snapshot.placement)
            else {
                continue;
            };
            let selected = self.sketch.constraints.contains(id);
            let hovered = self.sketch.annotation_hover.as_ref() == Some(id);
            if span * view.scale() < HIDDEN_BELOW_PX && !selected && !hovered {
                continue;
            }
            let Ok(value) = curvyo_sketch_core::measure(solution, &constraint.relation) else {
                continue;
            };
            let conflicting = snapshot.conflicting.contains(id) || snapshot.redundant.contains(id);
            let value_text =
                value_text(&constraint.relation, value.value(), value.dimension(), unit);
            let text = format!(
                "{}{}",
                if conflicting { "! " } else { "" },
                if constraint.reference {
                    format!("({value_text})")
                } else {
                    value_text.clone()
                }
            );
            let fx = !matches!(expression, Expression::Literal(_));
            let formula = expression
                .display_text(&snapshot.graph.parameters)
                .ok()
                .filter(|_| fx);
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
                if conflicting { ", conflicting" } else { "" },
            );
            let label = self
                .sketch_label_for(snapshot, id)
                .map(|p| snapshot.placement.apply_point(Point::new(p.x.0, p.y.0)));
            items.push(DimensionItem {
                geometry,
                label,
                text_chars: text.chars().count(),
            });
            pending.push((
                id.clone(),
                text,
                name,
                fx,
                constraint.reference,
                conflicting,
                selected,
                hovered,
            ));
        }
        let mut records: Vec<DimensionRecord> = curvyo_ui_core::layout_dimensions(&items, view)
            .into_iter()
            .zip(pending)
            .map(
                |(marks, (id, text, name, fx, reference, conflicting, selected, hovered))| {
                    DimensionRecord {
                        screen: view.document_to_screen(marks.text),
                        id,
                        text,
                        name,
                        fx,
                        reference,
                        conflicting,
                        selected,
                        hovered,
                        marks,
                    }
                },
            )
            .collect();
        // Bottom to top: reference, driving, hovered, selected (UX notes, "Overlap").
        records.sort_by_key(|r| (r.selected, r.hovered, !r.reference));
        records
    }

    /// The stored text position of `id`, or the one a label drag in progress shows.
    fn sketch_label_for(&self, snapshot: &SketchSnapshot, id: &ConstraintId) -> Option<Position> {
        match &self.sketch.label_drag {
            Some(drag) if drag.id == *id => Some(drag.current),
            _ => snapshot.labels.get(id).copied(),
        }
    }
}

/// The value as the canvas shows it: the display unit, at most the panel's precision, trailing
/// zeros trimmed, with "R " for a radius and "⌀ " for a diameter.
fn value_text(relation: &Relation, value: f64, dimension: Dimension, unit: DisplayUnit) -> String {
    if dimension == Dimension::Angle {
        return format!("{}°", trimmed(value.to_degrees(), 2));
    }
    let decimals = if unit == DisplayUnit::Mm { 2 } else { 3 };
    let number = trimmed(value / unit.mm_per_unit(), decimals);
    let prefix = match relation {
        Relation::Radius { .. } => "R ",
        Relation::Diameter { .. } => "⌀ ",
        _ => "",
    };
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
        Relation::Length { .. } => "Length",
        Relation::Radius { .. } => "Radius",
        Relation::Diameter { .. } => "Diameter",
        Relation::Angle { .. } | Relation::Sweep { .. } => "Angle",
        _ => "Dimension",
    }
}

fn point(solution: &Sketch, id: &EntityId, placement: Affine) -> Option<Point> {
    let p = solution.point(id).ok()?;
    Some(placement.apply_point(Point::new(p.x.0, p.y.0)))
}

/// A round curve's centre, document radius and default leader direction.
fn round(solution: &Sketch, id: &EntityId, placement: Affine) -> Option<(Point, f64, Vec2)> {
    let (center, radius, toward) = match solution.entities.get(id)? {
        Entity::Circle { center, radius, .. } => (center, radius.0, Vec2::new(1.0, -1.0)),
        Entity::Arc {
            center,
            radius,
            start_angle,
            end_angle,
            ..
        } => {
            let mut sweep = end_angle.0 - start_angle.0;
            if sweep < 0.0 {
                sweep += std::f64::consts::TAU;
            }
            // Three quarters along the arc: its middle carries the arc's symbols.
            let leader = start_angle.0 + sweep * 0.75;
            (center, radius.0, Vec2::new(leader.cos(), leader.sin()))
        }
        _ => return None,
    };
    let c = solution.point(center).ok()?;
    let local = Point::new(c.x.0, c.y.0);
    let doc_center = placement.apply_point(local);
    let rim = placement.apply_point(Point::new(local.x + radius, local.y));
    Some((
        doc_center,
        doc_center.vector_to(rim).length(),
        placement.apply_vector(toward),
    ))
}

fn line(solution: &Sketch, id: &EntityId, placement: Affine) -> Option<(Point, Point)> {
    match solution.entities.get(id)? {
        Entity::Line { start, end } => Some((
            point(solution, start, placement)?,
            point(solution, end, placement)?,
        )),
        _ => None,
    }
}

/// The geometry a dimension measures, in document coordinates, and its size for the
/// small-span rule; `None` for relations that are not dimensions.
fn dimension_geometry(
    solution: &Sketch,
    relation: &Relation,
    placement: Affine,
) -> Option<(DimensionGeometry, f64)> {
    let linear = |from: Point, to: Point| {
        Some((
            DimensionGeometry::Linear { from, to },
            from.vector_to(to).length(),
        ))
    };
    match relation {
        Relation::Length { curve, .. } => {
            if let Some((from, to)) = line(solution, curve, placement) {
                return linear(from, to);
            }
            let (center, radius, toward) = round(solution, curve, placement)?;
            Some((radius_geometry(center, radius, toward), radius * 2.0))
        }
        Relation::Distance { a, b, .. } => {
            let from = point(solution, a, placement)?;
            if let Some((s, e)) = line(solution, b, placement) {
                let along = s.vector_to(e);
                let length = along.length();
                let t = if length > 0.0 {
                    (s.vector_to(from).x * along.x + s.vector_to(from).y * along.y)
                        / (length * length)
                } else {
                    0.0
                };
                return linear(from, s.translated(along.scaled(t)));
            }
            linear(from, point(solution, b, placement)?)
        }
        Relation::Radius { curve, .. } => {
            let (center, radius, toward) = round(solution, curve, placement)?;
            Some((radius_geometry(center, radius, toward), radius * 2.0))
        }
        Relation::Diameter { curve, .. } => {
            let (center, radius, toward) = round(solution, curve, placement)?;
            Some((
                DimensionGeometry::Diameter {
                    center,
                    radius: curvyo_document_core::Length::from_mm(radius),
                    toward,
                },
                radius * 2.0,
            ))
        }
        Relation::Angle { a, b, .. } => {
            let (a0, a1) = line(solution, a, placement)?;
            let (b0, b1) = line(solution, b, placement)?;
            angle_geometry(a0, a1, b0, b1)
        }
        // A sweep is the structural half turn of a slot cap or arc, not a maker's dimension.
        _ => None,
    }
}

fn radius_geometry(center: Point, radius: f64, toward: Vec2) -> DimensionGeometry {
    DimensionGeometry::Radius {
        center,
        radius: curvyo_document_core::Length::from_mm(radius),
        toward,
    }
}

/// The angle between two lines: at their intersection, each ray toward the line's far part.
fn angle_geometry(a0: Point, a1: Point, b0: Point, b1: Point) -> Option<(DimensionGeometry, f64)> {
    let (da, db) = (a0.vector_to(a1), b0.vector_to(b1));
    let cross = da.x * db.y - da.y * db.x;
    let size = da.length().min(db.length());
    if cross.abs() <= f64::EPSILON * da.length() * db.length() {
        return None;
    }
    let t = (a0.vector_to(b0).x * db.y - a0.vector_to(b0).y * db.x) / cross;
    let vertex = a0.translated(da.scaled(t));
    let ray = |s: Point, e: Point, d: Vec2| {
        let mid = Point::new(f64::midpoint(s.x, e.x), f64::midpoint(s.y, e.y));
        let toward = vertex.vector_to(mid);
        if toward.length() > size * 1e-6 {
            toward
        } else {
            d
        }
    };
    Some((
        DimensionGeometry::Angle {
            vertex,
            first: ray(a0, a1, da),
            second: ray(b0, b1, db),
        },
        size,
    ))
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
