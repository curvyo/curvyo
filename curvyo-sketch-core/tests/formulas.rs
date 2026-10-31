//! Independent behavior checks for typed sketch inputs.
#![allow(clippy::unwrap_used, clippy::float_cmp)]
use curvyo_sketch_core::{Dimension, Expression, Parameter, ParameterId, Parameters, Quantity};

#[test]
fn dimensions_follow_stable_parameter_references() {
    let mut parameters = Parameters::default();
    let width = ParameterId::from("width-id");
    parameters.insert(
        width.clone(),
        Parameter::literal("width", Quantity::length(80.0).unwrap()),
    );
    parameters.insert(
        "margin-id".into(),
        Parameter::literal("edge_margin", Quantity::length(10.0).unwrap()),
    );
    let expr = Expression::parse("width - 2 * edge_margin", &parameters).unwrap();
    parameters.insert(
        "spacing-id".into(),
        Parameter::expression("spacing", Dimension::Length, expr),
    );
    assert_eq!(
        parameters.evaluate().unwrap()[&ParameterId::from("spacing-id")].value(),
        60.0
    );
    parameters.get_mut(&width).unwrap().name = "plate_width".into();
    parameters.get_mut(&width).unwrap().expression =
        Expression::Literal(Quantity::length(100.0).unwrap());
    assert_eq!(
        parameters.evaluate().unwrap()[&ParameterId::from("spacing-id")].value(),
        80.0
    );
}

#[test]
fn invalid_units_unknown_names_and_cycles_are_rejected() {
    let mut parameters = Parameters::default();
    assert!(Expression::parse("missing+1 mm", &parameters).is_err());
    parameters.insert(
        "a".into(),
        Parameter::literal("a", Quantity::length(1.0).unwrap()),
    );
    let expr = Expression::parse("a+30 deg", &parameters).unwrap();
    parameters.insert(
        "b".into(),
        Parameter::expression("b", Dimension::Length, expr),
    );
    assert!(parameters.evaluate().is_err());
    parameters
        .get_mut(&ParameterId::from("a"))
        .unwrap()
        .expression = Expression::Reference("b".into());
    assert!(parameters.evaluate().is_err());
}

#[test]
fn unit_conversion_and_precedence_preserve_physical_values() {
    let parameters = Parameters::default();
    let expression = Expression::parse("(2 cm + 1 in) / 2", &parameters).unwrap();
    let value = expression
        .evaluate(&parameters.evaluate().unwrap())
        .unwrap();
    assert_eq!(value.dimension(), Dimension::Length);
    assert!((value.value() - 22.7).abs() < 1e-12);
    assert!(
        Expression::parse("1 mm / 0", &parameters)
            .unwrap()
            .evaluate(&std::collections::BTreeMap::default())
            .is_err()
    );
}

#[test]
fn names_are_referenceable_and_long_dependency_chains_do_not_recurse() {
    for name in ["plate width", "2width", " ", "a-b"] {
        let mut p = Parameters::default();
        p.insert(
            "bad".into(),
            Parameter::literal(name, Quantity::length(1.0).unwrap()),
        );
        assert!(p.evaluate().is_err(), "accepted {name}");
    }
    let mut p = Parameters::default();
    p.insert(
        "p0".into(),
        Parameter::literal("p0", Quantity::length(7.0).unwrap()),
    );
    for i in 1..10_000 {
        p.insert(
            format!("p{i}").into(),
            Parameter::expression(
                &format!("p{i}"),
                Dimension::Length,
                Expression::Reference(format!("p{}", i - 1).into()),
            ),
        );
    }
    assert_eq!(
        p.evaluate().unwrap()[&ParameterId::from("p9999")].value(),
        7.0
    );
}
