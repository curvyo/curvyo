//! Temporary analytic handle goals are eliminated from the returned sketch graph.
use crate::solver_input::Input;
use crate::{Entity, EntityId, Position, Sketch, SketchError, SketchHandle, handle_positions};
use serde_json::{Value, json};

pub(crate) fn append(
    input: &Input,
    graph: &Sketch,
    primitives: &mut Vec<Value>,
    id: &EntityId,
    handle: SketchHandle,
    target: Position,
) -> Result<(), SketchError> {
    if !target.x.is_finite() || !target.y.is_finite() {
        return Err(SketchError::Invalid("non-finite handle goal".into()));
    }
    let initial = handle_positions(graph, id)?
        .get(&handle)
        .copied()
        .ok_or_else(|| SketchError::Invalid("unsupported curve handle".into()))?;
    let Some(Entity::Circle { center, .. } | Entity::Ellipse { center, .. }) =
        graph.entities.get(id)
    else {
        return Err(SketchError::Invalid("unsupported curve handle".into()));
    };
    let center_position = graph.point(center)?;
    input.append_goal(graph, primitives, center, center_position, 0)?;
    match handle {
        SketchHandle::Radius => {
            let radius = center_position.distance(target).0;
            if radius <= 1e-9 {
                return Err(SketchError::Invalid("radius must be positive".into()));
            }
            primitives.push(json!({"id":"handle_radius","type":"radius","curve":input.id(id),"value":radius,"temporary":true}));
        }
        SketchHandle::MajorAxis | SketchHandle::MinorAxis => {
            primitives.push(json!({"id":"handle_point","type":"point","x":initial.x.0,"y":initial.y.0,"fixed":false}));
            primitives.push(json!({"id":"handle_axis","type":"ellipse_axis","ellipse":input.id(id),"point":"handle_point","which":if handle==SketchHandle::MajorAxis {"major"} else {"minor"}}));
            for (axis, value) in [("x", target.x.0), ("y", target.y.0)] {
                primitives.push(json!({"id":format!("handle_goal_{axis}"),"type":axis,"point":"handle_point","value":value,"temporary":true}));
            }
        }
    }
    Ok(())
}
