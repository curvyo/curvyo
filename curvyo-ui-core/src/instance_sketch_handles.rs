//! Where each exposed value of a sketch clone puts its knob (`specs/0099-sketch-pages`
//! criterion 25, `adrs.md` A12): the sketch page's own dimension marks under the clone's matrix.

use curvyo_document_core::{
    Affine, CloneSnapshot, ExposedKey, ExposedParam, ExposedTarget, InstanceValue, Point,
    SketchSnapshot, Vec2, ViewTransform,
};
use curvyo_sketch_core::{Constraint, ConstraintId, Expression, Sketch};

use crate::instance_handles::{Knob, KnobMotion};
use crate::sketch_dimension_geometry::dimension_geometry;
use crate::{DimensionGeometry, DimensionItem, DimensionMarks, layout_dimensions};

/// The characters the layout reserves for a knob's dimension text (it draws none).
const TEXT_CHARS: usize = 6;

/// The driving dimension whose knob changes `key`: the dimension itself, or for a parameter the
/// one driving dimension whose expression is exactly that bare name (none for zero or several).
fn dimension_of<'a>(
    graph: &'a Sketch,
    key: &ExposedKey,
) -> Option<(&'a ConstraintId, &'a Constraint)> {
    match key.target() {
        ExposedTarget::Dimension(id) => graph
            .constraints
            .get_key_value(&id)
            .filter(|(_, constraint)| !constraint.reference),
        ExposedTarget::Parameter(id) => {
            let mut uses = graph.constraints.iter().filter(|(_, constraint)| {
                !constraint.reference
                    && matches!(constraint.relation.expression(), Some(Expression::Reference(name)) if *name == id)
            });
            let found = uses.next()?;
            uses.next().is_none().then_some(found)
        }
    }
}

/// How far the value moves per document millimetre along the document direction `toward`, in
/// sketch units: `base` divided by how much `placement` stretches that direction.
fn rate(placement: Affine, toward: Vec2, base: f64) -> Option<f64> {
    let unit = toward.normalized_to(1.0);
    let local = placement.inverse()?.apply_vector(unit);
    let rate = base * local.length();
    rate.is_finite().then_some(rate)
}

fn along(at: Point, toward: Vec2, placement: Affine, base: f64) -> Option<(Point, KnobMotion)> {
    if toward.length() <= 1e-9 {
        return None;
    }
    let direction = toward.normalized_to(1.0);
    Some((
        at,
        KnobMotion::Along {
            direction,
            rate: rate(placement, toward, base)?,
        },
    ))
}

/// The value the clone shows for `param`: its own, else the one up its chain, else the sketch's.
fn shown_value(clone: &CloneSnapshot, param: &ExposedParam) -> f64 {
    clone
        .chain
        .own_values
        .get(&param.key)
        .or_else(|| clone.chain.inherited_values.get(&param.key))
        .map_or(param.default, InstanceValue::number)
}

/// The knobs of `clone`, a clone of `sketch` whose solved graph is `solved`, in exposed order: one
/// per exposed length, radius, diameter or angle that has a dimension to sit on (a sweep and a
/// parameter used by a formula or by several dimensions have none).
#[must_use]
pub fn knobs(
    sketch: &SketchSnapshot,
    solved: &Sketch,
    clone: &CloneSnapshot,
    view: ViewTransform,
) -> Vec<Knob> {
    knobs_with_guides(sketch, solved, clone, view)
        .into_iter()
        .map(|(knob, _)| knob)
        .collect()
}

/// [`knobs`], each with its dimension's laid-out extension lines and arrowheads on the clone: the
/// guides drawn on hover and during a drag (UX notes 12.3).
#[must_use]
pub fn knobs_with_guides(
    sketch: &SketchSnapshot,
    solved: &Sketch,
    clone: &CloneSnapshot,
    view: ViewTransform,
) -> Vec<(Knob, DimensionMarks)> {
    let placement = clone.chain.composed.compose(sketch.placement);
    sketch
        .exposed
        .iter()
        .filter_map(|param| {
            let (id, constraint) = dimension_of(&sketch.graph, &param.key)?;
            let (geometry, _) = dimension_geometry(solved, &constraint.relation, placement)?;
            let label = sketch
                .labels
                .get(id)
                .map(|p| placement.apply_point(Point::new(p.x.0, p.y.0)));
            let marks = layout_dimensions(
                &[DimensionItem {
                    geometry,
                    label,
                    text_chars: TEXT_CHARS,
                }],
                view,
            )
            .pop()?;
            let second_tip =
                |fallback: Point| marks.arrows.get(1).map_or(fallback, |arrow| arrow.tip);
            let (at, motion) = match geometry {
                DimensionGeometry::Linear { from, to } => {
                    along(second_tip(to), from.vector_to(to), placement, 1.0)?
                }
                DimensionGeometry::Projected {
                    from,
                    to,
                    along: axis,
                } => {
                    let sign = if {
                        let d = from.vector_to(to);
                        d.x * axis.x + d.y * axis.y
                    } < 0.0
                    {
                        -1.0
                    } else {
                        1.0
                    };
                    along(second_tip(to), axis.scaled(sign), placement, 1.0)?
                }
                DimensionGeometry::Radius {
                    center,
                    radius,
                    toward,
                } => {
                    let rim = center.translated(toward.normalized_to(radius.as_mm()));
                    along(rim, toward, placement, 1.0)?
                }
                DimensionGeometry::Diameter {
                    center,
                    radius,
                    toward,
                } => {
                    let rim = center.translated(toward.normalized_to(radius.as_mm()));
                    along(rim, toward, placement, 2.0)?
                }
                DimensionGeometry::Angle { vertex, second, .. } => {
                    let tip = second_tip(vertex.translated(second.normalized_to(1.0)));
                    let sign = if placement.determinant() < 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                    (
                        tip,
                        KnobMotion::Around {
                            centre: vertex,
                            sign,
                        },
                    )
                }
            };
            Some((
                Knob {
                    key: param.key.as_str().to_owned(),
                    label: param.label.clone(),
                    at,
                    motion,
                    value: shown_value(clone, param),
                },
                marks,
            ))
        })
        .collect()
}
