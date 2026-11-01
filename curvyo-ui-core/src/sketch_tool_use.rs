//! What pressing a Constraints rail button does with the current sketch selection (AC24).
use curvyo_sketch_core::{Entity, EntityId, Expression, Quantity, Sketch};

use crate::sketch_actions::{SKETCH_AXIS_X, SKETCH_AXIS_Y, sketch_relation, sketch_relations};

/// The rail's constraint tools in card order; `"dimension"` is the smart dimension tool.
pub const SKETCH_RAIL_TOOLS: &[&str] = &[
    "coincident",
    "horizontal",
    "vertical",
    "parallel",
    "perpendicular",
    "tangent",
    "equal",
    "fixed",
    "symmetric",
    "midpoint",
    "dimension",
    "horizontal-distance",
    "vertical-distance",
];

/// The dimension kinds the smart dimension tool chooses between.
const DIMENSION_KINDS: &[&str] = &["length", "distance", "radius", "diameter", "angle"];

/// The result of pressing a rail tool now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SketchToolUse {
    /// The selection is complete for this tool: pressing applies it at once.
    Apply,
    /// The selection is empty or a valid start: pressing arms the tool for further picks.
    Arm,
    /// The selection cannot become eligible; the reason names what to select.
    Unavailable(String),
}

/// Classifies `tool` for `selected` without solving: complete, a valid start, or not usable.
#[must_use]
pub fn sketch_tool_use(tool: &str, graph: &Sketch, selected: &[EntityId]) -> SketchToolUse {
    if tool == "dimension" {
        // A selection a dimension fits is dimensioned at once (0066 AC40); one point starts a
        // distance to a second point or to a segment.
        let point = matches!(selected, [one] if matches!(graph.entities.get(one), Some(Entity::Point { .. })));
        return if dimension_kind(graph, selected).is_some() {
            SketchToolUse::Apply
        } else if selected.is_empty() || point {
            SketchToolUse::Arm
        } else {
            SketchToolUse::Unavailable(
                "Select a segment, a circle or arc, two points, a point and a segment, or two segments"
                    .into(),
            )
        };
    }
    if selected.is_empty() {
        return SketchToolUse::Arm;
    }
    match sketch_relations(tool, graph, selected, probe().as_ref()) {
        Ok(_) => SketchToolUse::Apply,
        Err(_) if completes(tool, graph, selected) => SketchToolUse::Arm,
        Err(reason) => SketchToolUse::Unavailable(reason),
    }
}

/// A stand-in length for the dimension tools' eligibility; relations ignore it.
fn probe() -> Option<Expression> {
    Quantity::length(1.0).ok().map(Expression::Literal)
}

/// The first dimension kind the selection supports, in the smart tool's order.
#[must_use]
pub fn dimension_kind(graph: &Sketch, selected: &[EntityId]) -> Option<&'static str> {
    let value = Quantity::length(1.0).ok().map(Expression::Literal);
    DIMENSION_KINDS
        .iter()
        .copied()
        .find(|kind| sketch_relation(kind, graph, selected, value.as_ref()).is_ok())
}

/// Whether one or two more picks of existing entities can make `selected` eligible.
fn completes(tool: &str, graph: &Sketch, selected: &[EntityId]) -> bool {
    // Up to two unselected representatives per entity kind are enough to try every completion.
    // Counting per kind as entities come keeps working when `Entity` gains a variant.
    let mut taken = std::collections::BTreeMap::<usize, usize>::new();
    let candidates: Vec<EntityId> = graph
        .entities
        .iter()
        .filter(|(id, _)| !selected.contains(id))
        .filter(|(_, entity)| {
            let count = taken.entry(kind_index(entity)).or_default();
            *count += 1;
            *count <= 2
        })
        .map(|(id, _)| id.clone())
        .chain(
            [SKETCH_AXIS_X, SKETCH_AXIS_Y]
                .into_iter()
                .map(EntityId::from)
                .filter(|id| !selected.contains(id)),
        )
        .collect();
    let fits = |extra: &[&EntityId]| {
        let mut picks = selected.to_vec();
        picks.extend(extra.iter().map(|id| (*id).clone()));
        sketch_relations(tool, graph, &picks, probe().as_ref()).is_ok()
    };
    candidates
        .iter()
        .any(|one| fits(&[one]) || candidates.iter().any(|two| two != one && fits(&[one, two])))
}

const fn kind_index(entity: &Entity) -> usize {
    match entity {
        Entity::Point { .. } => 0,
        Entity::Line { .. } => 1,
        Entity::Circle { .. } => 2,
        Entity::Arc { .. } => 3,
        Entity::Ellipse { .. } => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_sketch_core::{Length, Position};

    fn graph() -> Sketch {
        let graph = curvyo_sketch_core::create_rectangle(
            &Sketch::new("origin".into()),
            "r",
            Position::mm(10.0, 10.0),
            Position::mm(30.0, 20.0),
        )
        .unwrap();
        curvyo_sketch_core::create_circle(&graph, "c", Position::mm(40.0, 40.0), Length(5.0))
            .unwrap()
    }
    fn ids(names: &[&str]) -> Vec<EntityId> {
        names.iter().map(|name| (*name).into()).collect()
    }

    #[test]
    fn an_empty_selection_arms_every_tool() {
        let graph = graph();
        for tool in SKETCH_RAIL_TOOLS {
            assert_eq!(
                sketch_tool_use(tool, &graph, &[]),
                SketchToolUse::Arm,
                "{tool}"
            );
        }
    }

    #[test]
    fn a_complete_selection_applies_and_a_valid_start_arms() {
        let graph = graph();
        let line = ids(&["r:line0"]);
        assert_eq!(
            sketch_tool_use("horizontal", &graph, &line),
            SketchToolUse::Apply
        );
        assert_eq!(
            sketch_tool_use("fixed", &graph, &line),
            SketchToolUse::Apply
        );
        assert_eq!(
            sketch_tool_use("parallel", &graph, &line),
            SketchToolUse::Arm
        );
        assert_eq!(
            sketch_tool_use("midpoint", &graph, &line),
            SketchToolUse::Arm
        );
        let point = ids(&["r:p0"]);
        assert_eq!(
            sketch_tool_use("coincident", &graph, &point),
            SketchToolUse::Arm
        );
        assert_eq!(
            sketch_tool_use("symmetric", &graph, &point),
            SketchToolUse::Arm
        );
        let two = ids(&["r:line0", "r:line1"]);
        assert_eq!(
            sketch_tool_use("perpendicular", &graph, &two),
            SketchToolUse::Apply
        );
        assert_eq!(
            sketch_tool_use("dimension", &graph, &two),
            SketchToolUse::Apply
        );
        // A point measures to a second point or to a segment (0066 review change 2).
        assert_eq!(
            sketch_tool_use("dimension", &graph, &point),
            SketchToolUse::Arm
        );
        assert_eq!(
            sketch_tool_use("dimension", &graph, &ids(&["r:p0", "r:line1"])),
            SketchToolUse::Apply
        );
    }

    #[test]
    fn an_ineligible_selection_is_unavailable_with_its_reason() {
        let graph = graph();
        let circle = ids(&["c:curve"]);
        assert_eq!(
            sketch_tool_use("horizontal", &graph, &circle),
            SketchToolUse::Unavailable("Select one segment or two points".into())
        );
        assert!(matches!(
            sketch_tool_use("parallel", &graph, &circle),
            SketchToolUse::Unavailable(_)
        ));
        let three = ids(&["r:line0", "r:line1", "r:line2"]);
        assert!(matches!(
            sketch_tool_use("dimension", &graph, &three),
            SketchToolUse::Unavailable(_)
        ));
    }

    /// Horizontal and Vertical distance apply to two points, a point and the origin or the
    /// matching axis, and one line; one point arms them (0066 AC44, AC48).
    #[test]
    fn axis_distances_apply_to_points_a_line_or_a_point_and_an_axis() {
        let graph = graph();
        for tool in ["horizontal-distance", "vertical-distance"] {
            for selection in [
                ids(&["r:p0", "r:p2"]),
                ids(&["r:line0"]),
                ids(&["r:p0", "origin"]),
            ] {
                assert_eq!(
                    sketch_tool_use(tool, &graph, &selection),
                    SketchToolUse::Apply,
                    "{tool} {selection:?}"
                );
            }
            assert_eq!(
                sketch_tool_use(tool, &graph, &ids(&["r:p0"])),
                SketchToolUse::Arm
            );
            assert!(matches!(
                sketch_tool_use(tool, &graph, &ids(&["r:p0", "r:p1", "r:p2"])),
                SketchToolUse::Unavailable(_)
            ));
        }
        let point_and = |axis: &str| ids(&["r:p0", axis]);
        assert_eq!(
            sketch_tool_use("horizontal-distance", &graph, &point_and(SKETCH_AXIS_Y)),
            SketchToolUse::Apply
        );
        assert_eq!(
            sketch_tool_use("vertical-distance", &graph, &point_and(SKETCH_AXIS_X)),
            SketchToolUse::Apply
        );
        assert!(matches!(
            sketch_tool_use("horizontal-distance", &graph, &point_and(SKETCH_AXIS_X)),
            SketchToolUse::Unavailable(_)
        ));
    }

    #[test]
    fn the_smart_dimension_names_the_kind_of_the_selection() {
        let graph = graph();
        assert_eq!(dimension_kind(&graph, &ids(&["r:line0"])), Some("length"));
        assert_eq!(dimension_kind(&graph, &ids(&["c:curve"])), Some("radius"));
        assert_eq!(
            dimension_kind(&graph, &ids(&["r:p0", "r:p1"])),
            Some("distance")
        );
        assert_eq!(
            dimension_kind(&graph, &ids(&["r:line0", "r:line1"])),
            Some("angle")
        );
        assert_eq!(dimension_kind(&graph, &[]), None);
    }
}
