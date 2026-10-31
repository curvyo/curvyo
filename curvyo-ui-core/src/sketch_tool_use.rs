//! What pressing a Constraints rail button does with the current sketch selection (AC24).
use curvyo_sketch_core::{Entity, EntityId, Expression, Quantity, Sketch};

use crate::sketch_actions::sketch_relation;

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
        return if selected.is_empty() || dimension_kind(graph, selected).is_some() {
            SketchToolUse::Arm
        } else {
            SketchToolUse::Unavailable(
                "Select a segment, a circle or arc, two points or two segments".into(),
            )
        };
    }
    if selected.is_empty() {
        return SketchToolUse::Arm;
    }
    match sketch_relation(tool, graph, selected, None) {
        Ok(_) => SketchToolUse::Apply,
        Err(_) if completes(tool, graph, selected) => SketchToolUse::Arm,
        Err(reason) => SketchToolUse::Unavailable(reason),
    }
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
    let mut candidates: Vec<EntityId> = Vec::new();
    for kind in 0..5 {
        candidates.extend(
            graph
                .entities
                .iter()
                .filter(|(id, entity)| kind_index(entity) == kind && !selected.contains(id))
                .take(2)
                .map(|(id, _)| id.clone()),
        );
    }
    let fits = |extra: &[&EntityId]| {
        let mut picks = selected.to_vec();
        picks.extend(extra.iter().map(|id| (*id).clone()));
        sketch_relation(tool, graph, &picks, None).is_ok()
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
            SketchToolUse::Arm
        );
    }

    #[test]
    fn an_ineligible_selection_is_unavailable_with_its_reason() {
        let graph = graph();
        let point = ids(&["r:p0"]);
        assert_eq!(
            sketch_tool_use("horizontal", &graph, &point),
            SketchToolUse::Unavailable("Select one segment".into())
        );
        let circle = ids(&["c:curve"]);
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
