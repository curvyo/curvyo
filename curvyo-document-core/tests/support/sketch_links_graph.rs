//! The sketch the linked-values tests share (`specs/0099-sketch-pages` slice 2).

use curvyo_sketch_core as sketch;

/// Two lines from the origin, each with a driving length and an axis angle, and the length
/// parameters `w` and `v` used by nothing.
pub fn graph() -> sketch::Sketch {
    let mut s = sketch::Sketch::new("origin".into());
    for (point, x, y) in [("a", 60.0, 0.0), ("b", 0.0, 40.0)] {
        s.entities.insert(
            point.into(),
            sketch::Entity::Point {
                position: sketch::Position::mm(x, y),
                fixed: false,
            },
        );
    }
    for (line, end) in [("la", "a"), ("lb", "b")] {
        s.entities.insert(
            line.into(),
            sketch::Entity::Line {
                start: "origin".into(),
                end: end.into(),
            },
        );
    }
    let length = |mm: f64| sketch::Expression::Literal(sketch::Quantity::length(mm).unwrap());
    let angle = |deg: f64| {
        sketch::Expression::Literal(sketch::Quantity::angle(f64::to_radians(deg)).unwrap())
    };
    for (id, line, mm, deg) in [("a", "la", 60.0, 0.0), ("b", "lb", 40.0, 90.0)] {
        s.constraints.insert(
            format!("len_{id}").as_str().into(),
            sketch::Constraint::driving(sketch::Relation::Length {
                curve: line.into(),
                value: length(mm),
            }),
        );
        s.constraints.insert(
            format!("ang_{id}").as_str().into(),
            sketch::Constraint::driving(sketch::Relation::AxisAngle {
                line: line.into(),
                value: angle(deg),
            }),
        );
    }
    for (id, mm) in [("w", 10.0), ("v", 20.0)] {
        s.parameters.insert(
            id.into(),
            sketch::Parameter::literal(id, sketch::Quantity::length(mm).unwrap()),
        );
    }
    s
}
