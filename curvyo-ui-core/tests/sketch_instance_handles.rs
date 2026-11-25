//! Slice 2 of sketch instances in ui-core (`specs/0099-sketch-pages` criteria 24, 25, 32 to 34,
//! 36): the knobs of a sketch clone and their rates, a coupled edit, Shift = only this, Back, and
//! the Parameters row keeping its committed state while a gesture is in flight.
#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::{
    Affine, CloneSnapshot, Document, ExposedKey, NodeId, ObjectSnapshot, Point, Style,
    ViewTransform,
};
use curvyo_sketch_core as sk;
use curvyo_ui_core::instance_handles::{Knob, KnobMotion, dragged};
use curvyo_ui_core::instance_sketch::{change, plan, reset, view};
use curvyo_ui_core::instance_sketch_handles::knobs;

/// A line `la` of 60 mm along x with a driving length `len`, a line `lb` of 40 mm along y whose
/// length is the parameter `w`, a circle `c` whose diameter is `f * 2`, an axis angle `ang` on `la`, and
/// a parameter `f` that only a formula uses.
fn graph() -> sk::Sketch {
    let mut s = sk::Sketch::new("origin".into());
    for (id, x, y) in [("a", 60.0, 0.0), ("b", 0.0, 40.0), ("m", 100.0, 0.0)] {
        s.entities.insert(
            id.into(),
            sk::Entity::Point {
                position: sk::Position::mm(x, y),
                fixed: id == "m",
            },
        );
    }
    for (line, end) in [("la", "a"), ("lb", "b")] {
        s.entities.insert(
            line.into(),
            sk::Entity::Line {
                start: "origin".into(),
                end: end.into(),
            },
        );
    }
    s.entities.insert(
        "c".into(),
        sk::Entity::Circle {
            center: "m".into(),
            radius: sk::Length(10.0),
            fixed_radius: false,
        },
    );
    let length = |mm: f64| sk::Expression::Literal(sk::Quantity::length(mm).unwrap());
    for (id, value) in [("w", 40.0), ("f", 5.0)] {
        s.parameters.insert(
            id.into(),
            sk::Parameter::literal(id, sk::Quantity::length(value).unwrap()),
        );
    }
    let formula = sk::Expression::parse("f * 2", &s.parameters).unwrap();
    let constraints = [
        (
            "len",
            sk::Relation::Length {
                curve: "la".into(),
                value: length(60.0),
            },
        ),
        (
            "wlen",
            sk::Relation::Length {
                curve: "lb".into(),
                value: sk::Expression::Reference("w".into()),
            },
        ),
        (
            "dia",
            sk::Relation::Diameter {
                curve: "c".into(),
                value: formula,
            },
        ),
        (
            "ang",
            sk::Relation::AxisAngle {
                line: "la".into(),
                value: sk::Expression::Literal(sk::Quantity::angle(0.0).unwrap()),
            },
        ),
        (
            "vert",
            sk::Relation::AxisAngle {
                line: "lb".into(),
                value: sk::Expression::Literal(
                    sk::Quantity::angle(std::f64::consts::FRAC_PI_2).unwrap(),
                ),
            },
        ),
    ];
    for (id, relation) in constraints {
        s.constraints
            .insert(id.into(), sk::Constraint::driving(relation));
    }
    s
}

fn dim(id: &str) -> ExposedKey {
    ExposedKey::dimension(&id.into())
}

fn param(id: &str) -> ExposedKey {
    ExposedKey::parameter(&id.into())
}

/// The sketch exposing `len`, `w`, `dia`, `ang` and `f`, with one clone.
fn setup() -> (Document, NodeId, NodeId) {
    let d = Document::new(1);
    let sketch = d
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    d.expose_parameter(sketch, &dim("len"), Some("Leg"))
        .unwrap();
    d.expose_parameter(sketch, &param("w"), None).unwrap();
    d.expose_parameter(sketch, &dim("dia"), Some("Bore"))
        .unwrap();
    d.expose_parameter(sketch, &dim("ang"), Some("Tilt"))
        .unwrap();
    d.expose_parameter(sketch, &param("f"), None).unwrap();
    let clone = d.linked_clone(&[sketch], &[]).unwrap()[0];
    (d, sketch, clone)
}

fn clone_of(d: &Document, id: NodeId) -> CloneSnapshot {
    match d.read_objects().get(id).unwrap() {
        ObjectSnapshot::Clone(clone) => clone.clone(),
        other => panic!("{other:?}"),
    }
}

fn knobs_of(d: &Document, sketch: NodeId, clone: &CloneSnapshot) -> Vec<Knob> {
    let solved = d.instance_solution(clone.id(), None).unwrap();
    knobs(
        &d.sketch(sketch).unwrap(),
        &solved,
        clone,
        ViewTransform::new(4.0, Point::new(0.0, 0.0)),
    )
}

fn knob<'a>(knobs: &'a [Knob], key: &str) -> &'a Knob {
    knobs.iter().find(|knob| knob.key == key).unwrap()
}

fn length_of(d: &Document, clone: NodeId, key: &ExposedKey) -> Option<f64> {
    clone_of(d, clone)
        .chain
        .own_values
        .get(key)
        .map(curvyo_document_core::InstanceValue::number)
}

/// Criterion 25: a length, a radius-like diameter and an angle get knobs; a parameter that is one
/// dimension's bare name gets that dimension's knob; one used by a formula gets none.
#[test]
fn which_values_get_a_knob() {
    let (d, sketch, clone) = setup();
    let all = knobs_of(&d, sketch, &clone_of(&d, clone));
    let keys: Vec<&str> = all.iter().map(|knob| knob.key.as_str()).collect();
    assert_eq!(
        keys,
        ["d:len", "p:w", "d:dia", "d:ang"],
        "f is in a formula: field only"
    );
    let bore = knob(&all, "d:dia");
    let KnobMotion::Along { rate, .. } = bore.motion else {
        panic!("a diameter moves along its leader")
    };
    assert!(
        (rate - 2.0).abs() < 1e-9,
        "a diameter changes by twice the move: {rate}"
    );
    let rim = Point::new(100.0, 0.0).vector_to(bore.at).length();
    assert!(
        (rim - 5.0).abs() < 1e-6,
        "on the circle of diameter f * 2: {rim}"
    );
    assert!(matches!(
        knob(&all, "d:ang").motion,
        KnobMotion::Around { .. }
    ));
}

/// Criterion 25: on a 50 % clone a 10 mm pointer move changes a length by 20 mm; the anchor and
/// direction follow a rotated clone's matrix.
#[test]
fn the_value_is_in_sketch_units_and_the_knob_follows_the_matrix() {
    let (d, sketch, clone) = setup();
    let half = clone_of(&d, clone).mapped_by(Affine::scale(0.5, 0.5));
    let leg = knob(&knobs_of(&d, sketch, &half), "d:len").clone();
    let KnobMotion::Along { direction, .. } = leg.motion else {
        panic!("a length")
    };
    let to = Point::new(leg.at.x + direction.x * 10.0, leg.at.y + direction.y * 10.0);
    assert!((dragged(&leg, leg.at, to, false) - 80.0).abs() < 1e-6);
    let turn = Affine::rotation_about(
        Point::new(0.0, 0.0),
        curvyo_document_core::Angle::from_radians(std::f64::consts::FRAC_PI_2),
    );
    let turned = clone_of(&d, clone).mapped_by(turn);
    let leg = knob(&knobs_of(&d, sketch, &turned), "d:len").clone();
    let KnobMotion::Along { direction, rate } = leg.motion else {
        panic!("a length")
    };
    assert!(
        direction.x.abs() < 1e-9 && (direction.y.abs() - 1.0).abs() < 1e-9,
        "{direction:?}"
    );
    assert!((rate - 1.0).abs() < 1e-9);
    let snapped = dragged(
        &leg,
        leg.at,
        Point::new(leg.at.x, leg.at.y + 2.4 * direction.y),
        true,
    );
    assert!(
        (snapped - 62.0).abs() < 1e-9,
        "Ctrl snaps whole mm: {snapped}"
    );
}

/// Criterion 25: a mirrored clone turns its angle knob the other way.
#[test]
fn a_mirrored_clone_turns_its_angle_knob_the_other_way() {
    let (d, sketch, clone) = setup();
    let plain = knob(&knobs_of(&d, sketch, &clone_of(&d, clone)), "d:ang").clone();
    let mirrored = clone_of(&d, clone).mapped_by(Affine::scale(1.0, -1.0));
    let flipped = knob(&knobs_of(&d, sketch, &mirrored), "d:ang").clone();
    let (KnobMotion::Around { sign: a, .. }, KnobMotion::Around { sign: b, .. }) =
        (plain.motion, flipped.motion)
    else {
        panic!("angles turn")
    };
    assert!((a * b + 1.0).abs() < 1e-9, "{a} {b}");
}

/// Criteria 33 and 24: a coupled edit sets every member to the same value, clamped to the tightest
/// limit with a notice naming the member whose limit held; the row reads Own after it.
#[test]
fn a_coupled_edit_moves_the_partner_and_names_its_limit() {
    let (d, sketch, clone) = setup();
    d.link_exposed(sketch, &dim("len"), &param("w")).unwrap();
    d.set_exposed_limits(sketch, &param("w"), None, Some(70.0), None)
        .unwrap();
    let changed = change(&d, &[clone], "d:len", 90.0, false, &[]).unwrap();
    assert!(changed.written);
    assert_eq!(changed.notices, ["Leg set to 70 mm, the maximum of w."]);
    assert_eq!(length_of(&d, clone, &dim("len")), Some(70.0));
    assert_eq!(length_of(&d, clone, &param("w")), Some(70.0));
    let row = view(&d, &[clone], &[]).unwrap();
    assert_eq!(row.state, "own");
    assert_eq!(row.rows[0].group.as_deref(), Some("A"));
    assert_eq!(row.rows[0].partners, ["w"]);
    assert_eq!(row.rows[0].link, "linked");
}

/// Criteria 34 and 36: only this changes one value and unlinks it on this clone; Back relinks it
/// and leaves the partner alone.
#[test]
fn only_this_unlinks_and_back_relinks() {
    let (d, sketch, clone) = setup();
    d.link_exposed(sketch, &dim("len"), &param("w")).unwrap();
    change(&d, &[clone], "d:len", 50.0, true, &[]).unwrap();
    assert_eq!(length_of(&d, clone, &dim("len")), Some(50.0));
    assert_eq!(length_of(&d, clone, &param("w")), None, "the partner stays");
    assert_eq!(view(&d, &[clone], &[]).unwrap().rows[0].link, "unlinked");
    change(&d, &[clone], "p:w", 30.0, false, &[]).unwrap();
    assert_eq!(
        length_of(&d, clone, &dim("len")),
        Some(50.0),
        "no longer coupled"
    );
    reset(&d, &[clone], Some("d:len")).unwrap();
    let row = view(&d, &[clone], &[]).unwrap();
    assert_eq!(row.rows[0].link, "linked");
    assert_eq!(row.rows[0].origin, "sketch");
    assert_eq!(
        length_of(&d, clone, &param("w")),
        Some(30.0),
        "the partner keeps its value"
    );
}

/// Criterion 24 (`adrs.md` A15 (b)): while a gesture is in flight the pill, the origins and the
/// link marks keep the committed state; only the numbers follow the preview.
#[test]
fn the_row_keeps_its_committed_state_during_a_gesture() {
    let (d, sketch, clone) = setup();
    d.link_exposed(sketch, &dim("len"), &param("w")).unwrap();
    let step = plan(&d, clone, "d:len", 75.0, true).unwrap();
    assert!(step.unlinked.contains(&dim("len")));
    let stepped = [step.preview.unwrap()];
    let row = view(&d, &[clone], &stepped).unwrap();
    assert_eq!(row.state, "original");
    assert_eq!(row.rows[0].origin, "sketch");
    assert_eq!(row.rows[0].link, "linked");
    assert_eq!(row.rows[0].value, Some(75.0));
}
