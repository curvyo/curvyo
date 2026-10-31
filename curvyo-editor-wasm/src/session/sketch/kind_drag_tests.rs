//! Select drags every analytic entity kind by its edge or a defining point (AC29).
use super::*;
use curvyo_document_core::{Point, SketchDelta};
use curvyo_sketch_core::{
    Constraint, Construction, Entity, EntityId, Expression, Length, Position, Quantity, Relation,
    Sketch,
};

const SHIFT: (f64, f64) = (6.0, 4.0);

struct Case {
    name: &'static str,
    command: Construction,
    /// A point on the entity's edge, away from its defining points.
    edge: (f64, f64),
    /// The kind of the entity the edge press must grab.
    kind: fn(&Entity) -> bool,
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "point",
            command: Construction::Point(Position::mm(30.0, 30.0)),
            edge: (30.0, 30.0),
            kind: |e| matches!(e, Entity::Point { .. }),
        },
        Case {
            name: "line",
            command: Construction::Polyline(vec![
                Position::mm(20.0, 20.0),
                Position::mm(40.0, 20.0),
            ]),
            edge: (25.0, 20.0),
            kind: |e| matches!(e, Entity::Line { .. }),
        },
        Case {
            name: "circle",
            command: Construction::Circle {
                center: Position::mm(30.0, 30.0),
                radius: Length(10.0),
            },
            edge: (40.0, 30.0),
            kind: |e| matches!(e, Entity::Circle { .. }),
        },
        Case {
            name: "arc",
            command: Construction::ArcCenter {
                center: Position::mm(30.0, 30.0),
                start: Position::mm(40.0, 30.0),
                end: Position::mm(30.0, 40.0),
            },
            edge: (30.0 + 10.0 * 0.866_025_403_784_438_6, 35.0),
            kind: |e| matches!(e, Entity::Arc { .. }),
        },
        Case {
            name: "ellipse",
            command: Construction::Ellipse {
                center: Position::mm(30.0, 30.0),
                axis_end: Position::mm(42.0, 30.0),
                second_axis: Length(6.0),
            },
            edge: (30.0, 36.0),
            kind: |e| matches!(e, Entity::Ellipse { .. }),
        },
        Case {
            name: "rectangle member",
            command: Construction::Rectangle(Position::mm(20.0, 20.0), Position::mm(40.0, 32.0)),
            edge: (25.0, 20.0),
            kind: |e| matches!(e, Entity::Line { .. }),
        },
        Case {
            name: "slot member",
            command: Construction::Slot {
                start: Position::mm(20.0, 30.0),
                end: Position::mm(40.0, 30.0),
                width: Length(10.0),
            },
            edge: (25.0, 35.0),
            kind: |e| matches!(e, Entity::Line { .. }),
        },
    ]
}

fn placed(s: &Session, x: f64, y: f64) -> Point {
    s.sketch_placement().unwrap().apply_point(Point::new(x, y))
}

fn setup(case: &Case) -> (Session, curvyo_document_core::NodeId) {
    let mut s = Session::new(66_500);
    s.set_workbench(Workbench::Sketch);
    let id = s.new_sketch().unwrap();
    s.sketch_construct(&case.command).unwrap();
    s.set_sketch_tool(SketchTool::Select);
    (s, id)
}

fn graph(s: &Session, id: curvyo_document_core::NodeId) -> Sketch {
    s.document.sketch(id).unwrap().graph
}

fn solution(s: &Session, id: curvyo_document_core::NodeId) -> Sketch {
    s.document.sketch(id).unwrap().solution
}

/// The entity an edge press grabs, as hit-tested by the session.
fn grabbed(s: &mut Session, case: &Case, at: (f64, f64)) -> EntityId {
    s.pointer_down(placed(s, at.0, at.1), false);
    let entity = s.sketch.entities.iter().next().cloned();
    entity.unwrap_or_else(|| panic!("{}: nothing grabbed", case.name))
}

fn drag(s: &mut Session, from: (f64, f64)) {
    let to = (from.0 + SHIFT.0, from.1 + SHIFT.1);
    s.pointer_hover(placed(s, to.0, to.1), false, false);
    s.pointer_up(placed(s, to.0, to.1), false, false);
}

fn moved(before: &Sketch, after: &Sketch, id: &EntityId) -> (f64, f64) {
    let a = before.point(id).unwrap();
    let b = after.point(id).unwrap();
    (b.x.0 - a.x.0, b.y.0 - a.y.0)
}

fn near(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
}

#[test]
fn every_kind_translates_as_a_whole_by_its_edge_in_one_step() {
    for case in cases() {
        let (mut s, id) = setup(&case);
        let before = solution(&s, id);
        let history = s.undo_len();
        let entity = grabbed(&mut s, &case, case.edge);
        assert!(
            (case.kind)(&before.entities[&entity]),
            "{}: grabbed {entity:?}",
            case.name
        );
        drag(&mut s, case.edge);
        assert_eq!(s.undo_len(), history + 1, "{}: one history step", case.name);
        let after = solution(&s, id);
        let points: Vec<EntityId> = if matches!(before.entities[&entity], Entity::Point { .. }) {
            vec![entity.clone()]
        } else {
            before.entities[&entity]
                .point_ids()
                .into_iter()
                .cloned()
                .collect()
        };
        for point in points {
            assert!(
                near(moved(&before, &after, &point), SHIFT),
                "{}: {point:?} moved {:?}",
                case.name,
                moved(&before, &after, &point)
            );
        }
        s.undo();
        assert_eq!(graph(&s, id), {
            let mut s2 = setup(&case).0;
            s2.set_sketch_tool(SketchTool::Select);
            graph(&s2, s2.active_sketch().unwrap())
        });
    }
}

#[test]
fn every_kind_moves_a_defining_point_alone_and_keeps_hard_constraints() {
    for case in cases() {
        let (mut s, id) = setup(&case);
        let before = solution(&s, id);
        let intent = graph(&s, id);
        let created: Vec<EntityId> = intent
            .entities
            .iter()
            .filter(|(eid, e)| **eid != intent.origin && matches!(e, Entity::Point { .. }))
            .map(|(eid, _)| eid.clone())
            .collect();
        for point in created {
            let p = before.point(&point).unwrap();
            let history = s.undo_len();
            s.sketch.entities.clear();
            s.pointer_down(placed(&s, p.x.0, p.y.0), false);
            let held = s.sketch.entities.iter().next().cloned();
            if held.as_ref() != Some(&point) {
                // Only visible point handles are pickable; helper points such as foci are not.
                s.escape();
                s.escape();
                continue;
            }
            drag(&mut s, (p.x.0, p.y.0));
            assert_eq!(s.undo_len(), history + 1, "{}: {point:?}", case.name);
            let after = solution(&s, id);
            assert!(
                near(moved(&before, &after, &point), SHIFT),
                "{}: {point:?} reached {:?}",
                case.name,
                moved(&before, &after, &point)
            );
            curvyo_sketch_core::validate_candidate(&graph(&s, id), &after).unwrap();
            s.undo();
        }
    }
}

#[test]
fn escape_mid_drag_restores_every_kind() {
    for case in cases() {
        let (mut s, id) = setup(&case);
        let before = s.document.sketch(id).unwrap();
        let history = s.undo_len();
        grabbed(&mut s, &case, case.edge);
        let to = (case.edge.0 + SHIFT.0, case.edge.1 + SHIFT.1);
        s.pointer_hover(placed(&s, to.0, to.1), false, false);
        assert!(s.sketch_preview().is_some(), "{}: live preview", case.name);
        s.escape();
        assert!(s.sketch_preview().is_none(), "{}", case.name);
        s.pointer_up(placed(&s, to.0, to.1), false, false);
        assert_eq!(s.undo_len(), history, "{}", case.name);
        assert_eq!(s.document.sketch(id).unwrap().solution, before.solution);
    }
}

fn constrain(s: &mut Session, relation: Relation) {
    let id: curvyo_sketch_core::ConstraintId = s.minter.mint().to_hex().into();
    s.sketch_apply_delta(&SketchDelta {
        constraints: [(id, Some(Constraint::driving(relation)))].into(),
        ..Default::default()
    })
    .unwrap();
}

#[test]
fn constrained_kinds_follow_their_constraints_and_fixed_ones_stay() {
    for case in cases() {
        let (mut s, id) = setup(&case);
        let entity = grabbed(&mut s, &case, case.edge);
        s.pointer_up(placed(&s, case.edge.0, case.edge.1), false, false);
        s.escape();
        let before = solution(&s, id);
        // A size constraint leaves translation free and must still hold after the drag.
        let size = match &before.entities[&entity] {
            Entity::Line { .. } => Some(Relation::Length {
                curve: entity.clone(),
                value: Expression::Literal(
                    Quantity::length(
                        curvyo_sketch_core::measure(
                            &before,
                            &Relation::Length {
                                curve: entity.clone(),
                                value: Expression::Literal(Quantity::length(1.0).unwrap()),
                            },
                        )
                        .unwrap()
                        .value(),
                    )
                    .unwrap(),
                ),
            }),
            Entity::Circle { radius, .. } | Entity::Arc { radius, .. } => Some(Relation::Radius {
                curve: entity.clone(),
                value: Expression::Literal(Quantity::length(radius.0).unwrap()),
            }),
            _ => None,
        };
        let mut at = case.edge;
        if let Some(size) = size {
            constrain(&mut s, size);
            let before = solution(&s, id);
            let history = s.undo_len();
            grabbed(&mut s, &case, case.edge);
            drag(&mut s, case.edge);
            assert_eq!(s.undo_len(), history + 1, "{}: sized drag", case.name);
            let after = solution(&s, id);
            let anchor = before.entities[&entity].point_ids()[0].clone();
            assert!(
                near(moved(&before, &after, &anchor), SHIFT),
                "{}: sized translation {:?}",
                case.name,
                moved(&before, &after, &anchor)
            );
            curvyo_sketch_core::validate_candidate(&graph(&s, id), &after).unwrap();
            at = (at.0 + SHIFT.0, at.1 + SHIFT.1);
        }
        constrain(&mut s, Relation::Fixed(entity.clone()));
        let fixed = solution(&s, id);
        let history = s.undo_len();
        grabbed(&mut s, &case, at);
        drag(&mut s, at);
        let after = solution(&s, id);
        let points: Vec<EntityId> = if matches!(fixed.entities[&entity], Entity::Point { .. }) {
            vec![entity.clone()]
        } else {
            fixed.entities[&entity]
                .point_ids()
                .into_iter()
                .cloned()
                .collect()
        };
        for point in points {
            assert!(
                near(moved(&fixed, &after, &point), (0.0, 0.0)),
                "{}: fixed {point:?} moved",
                case.name
            );
        }
        assert!(s.undo_len() <= history + 1, "{}", case.name);
    }
}

#[test]
fn arcs_inside_a_drawn_chain_drag_with_their_tangent_joins() {
    let mut s = Session::new(66_501);
    s.set_workbench(Workbench::Sketch);
    let id = s.new_sketch().unwrap();
    s.set_sketch_tool(SketchTool::Polyline);
    let click = |s: &mut Session, x: f64, y: f64| {
        let p = placed(s, x, y);
        s.pointer_hover(p, false, false);
        s.pointer_down(p, false);
        s.pointer_up(p, false, false);
    };
    click(&mut s, 5.0, 7.0);
    click(&mut s, 25.0, 7.0);
    assert!(s.set_sketch_mode("tangent-arc"));
    // Left turn to (35, 17), then keep turning left (inside tangency) to (28, 30).
    click(&mut s, 35.0, 17.0);
    click(&mut s, 28.0, 30.0);
    s.sketch_secondary_press();
    s.set_sketch_tool(SketchTool::Select);
    let arcs: Vec<EntityId> = solution(&s, id)
        .entities
        .iter()
        .filter(|(_, e)| matches!(e, Entity::Arc { .. }))
        .map(|(eid, _)| eid.clone())
        .collect();
    assert_eq!(arcs.len(), 2);
    for arc in arcs {
        let mid = arc_mid(&solution(&s, id), &arc);
        let before = solution(&s, id);
        let history = s.undo_len();
        s.sketch.entities.clear();
        s.pointer_down(placed(&s, mid.x.0, mid.y.0), false);
        assert!(s.sketch.entities.contains(&arc), "{arc:?} grabbed");
        let to = (mid.x.0 + 2.0, mid.y.0 + 1.0);
        s.pointer_hover(placed(&s, to.0, to.1), false, false);
        assert!(
            s.sketch_preview().is_some(),
            "{arc:?}: {:?}",
            s.sketch.error
        );
        s.pointer_up(placed(&s, to.0, to.1), false, false);
        assert_eq!(s.undo_len(), history + 1, "{arc:?}");
        assert_ne!(solution(&s, id), before);
        curvyo_sketch_core::validate_candidate(&graph(&s, id), &solution(&s, id)).unwrap();
    }
}

fn arc_mid(sketch: &Sketch, arc: &EntityId) -> Position {
    let Entity::Arc {
        center,
        radius,
        start_angle,
        end_angle,
        ..
    } = &sketch.entities[arc]
    else {
        panic!("arc")
    };
    let c = sketch.point(center).unwrap();
    let mid = f64::midpoint(start_angle.0, end_angle.0);
    Position::mm(c.x.0 + radius.0 * mid.cos(), c.y.0 + radius.0 * mid.sin())
}
