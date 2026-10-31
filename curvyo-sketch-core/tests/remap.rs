//! Independent graph copies preserve formulas while replacing stable references.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::*;
#[test]
fn remapping_preserves_solved_geometry_and_parameter_dependencies() {
    let mut graph = create_rectangle(
        &Sketch::new("o".into()),
        "r",
        Position::mm(0.0, 0.0),
        Position::mm(40.0, 20.0),
    )
    .unwrap();
    graph.parameters.insert(
        "width".into(),
        Parameter::literal("width", Quantity::length(40.0).unwrap()),
    );
    graph.parameters.insert(
        "twice".into(),
        Parameter::expression(
            "twice",
            Dimension::Length,
            Expression::Reference("width".into()),
        ),
    );
    let copied = graph.with_fresh_namespace("copy");
    copied.validate().unwrap();
    assert!(
        copied
            .entities
            .keys()
            .all(|id| !graph.entities.contains_key(id))
    );
    assert!(
        copied
            .constraints
            .keys()
            .all(|id| !graph.constraints.contains_key(id))
    );
    assert_eq!(
        copied.parameters.evaluate().unwrap()[&"copy:twice".into()],
        Quantity::length(40.0).unwrap()
    );
    assert_eq!(
        outline(&copied, Length(0.01)).unwrap(),
        outline(&graph, Length(0.01)).unwrap()
    );
}
