//! Typed line/slot angles and ellipse radii as driving dimensions (0066 AC39): solved value
//! within tolerance, one degree of freedom each, and the measured value as an oracle.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::*;

fn point(s: &mut Sketch, id: &str, x: f64, y: f64, fixed: bool) {
    s.entities.insert(
        id.into(),
        Entity::Point {
            position: Position::mm(x, y),
            fixed,
        },
    );
}

fn dof(s: &Sketch) -> (usize, Sketch) {
    let result = solve(s, &SolveMode::Edit).unwrap();
    let dof = match result.status {
        SolveStatus::UnderConstrained(n) => n,
        SolveStatus::FullyConstrained => 0,
        other => panic!("{other:?}: {:?}", result.message),
    };
    (dof, result.candidate.unwrap())
}

fn with(s: &Sketch, relations: Vec<Relation>) -> Sketch {
    let mut s = s.clone();
    for (i, relation) in relations.into_iter().enumerate() {
        s.constraints
            .insert(format!("new{i}").into(), Constraint::driving(relation));
    }
    s
}

fn mm(value: f64) -> Expression {
    Expression::Literal(Quantity::length(value).unwrap())
}

fn line() -> Sketch {
    let mut s = Sketch::new("origin".into());
    point(&mut s, "a", 10.0, 10.0, true);
    point(&mut s, "b", 30.0, 12.0, false);
    s.entities.insert(
        "l".into(),
        Entity::Line {
            start: "a".into(),
            end: "b".into(),
        },
    );
    s
}

#[test]
fn an_axis_angle_turns_the_line_to_its_directed_value() {
    let s = line();
    let (before, _) = dof(&s);
    for degrees in [20.0_f64, 135.0, -60.0] {
        let angle = Relation::AxisAngle {
            line: "l".into(),
            value: Expression::Literal(Quantity::angle(degrees.to_radians()).unwrap()),
        };
        let (after, solved) = dof(&with(&s, vec![angle.clone()]));
        assert_eq!(before - after, 1, "{degrees}");
        let (a, b) = solved.line(&"l".into()).unwrap();
        let actual = (b.y.0 - a.y.0).atan2(b.x.0 - a.x.0).to_degrees();
        assert!((actual - degrees).abs() < 1e-6, "{degrees}: {actual}");
        let measured = measure(&solved, &angle).unwrap();
        assert!((measured.value().to_degrees() - degrees).abs() < 1e-6);
    }
}

fn ellipse(minor: f64) -> Sketch {
    let mut s = Sketch::new("origin".into());
    point(&mut s, "c", 0.0, 0.0, true);
    point(&mut s, "f", 8.0, 0.0, false);
    s.entities.insert(
        "e".into(),
        Entity::Ellipse {
            center: "c".into(),
            focus: "f".into(),
            minor_radius: Length(minor),
            fixed_radius: false,
        },
    );
    s
}

fn radius(axis: EllipseAxis, value: f64) -> Relation {
    Relation::EllipseRadius {
        curve: "e".into(),
        axis,
        value: mm(value),
    }
}

#[test]
fn ellipse_radii_drive_the_major_and_minor_semi_axes() {
    let s = ellipse(6.0);
    let (before, _) = dof(&s);
    for relations in [
        vec![radius(EllipseAxis::Major, 20.0)],
        vec![radius(EllipseAxis::Minor, 4.0)],
        vec![
            radius(EllipseAxis::Major, 15.0),
            radius(EllipseAxis::Minor, 5.0),
        ],
    ] {
        let graph = with(&s, relations.clone());
        let (after, solved) = dof(&graph);
        assert_eq!(before - after, relations.len(), "{relations:?}");
        for relation in &relations {
            let expected = relation
                .expression()
                .unwrap()
                .evaluate(&graph.parameters.evaluate().unwrap())
                .unwrap();
            let actual = measure(&solved, relation).unwrap();
            assert!(
                (actual.value() - expected.value()).abs() < 1e-6,
                "{relation:?}: {actual:?}"
            );
        }
    }
}
