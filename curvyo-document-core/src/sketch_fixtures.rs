//! Current-format golden files preserve editable sketch records and damaged-input refusals.
use crate::*;
use curvyo_sketch_core::{Constraint, Expression, Parameter, Position, Quantity, Relation, Sketch};

fn fixture(name: &str, doc: &Document, valid: bool) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(&path, pack(doc, "sketch-format-test").unwrap()).unwrap();
    }
    let bytes = std::fs::read(path).unwrap();
    let loaded = unpack(919, &bytes);
    assert_eq!(loaded.is_ok(), valid, "{name}");
    if let Ok(loaded) = loaded {
        assert_eq!(
            loaded.export_json().unwrap(),
            doc.export_json().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn sketch_current_format_goldens() {
    let doc = Document::new(901);
    let mut plate = curvyo_sketch_core::create_rectangle(
        &Sketch::new("origin".into()),
        "plate",
        Position::mm(0.0, 0.0),
        Position::mm(80.0, 40.0),
    )
    .unwrap();
    plate.parameters.insert(
        "radius".into(),
        Parameter::literal("hole_radius", Quantity::length(4.0).unwrap()),
    );
    plate = curvyo_sketch_core::create_circle(
        &plate,
        "hole1",
        Position::mm(20.0, 20.0),
        curvyo_sketch_core::Length(4.0),
    )
    .unwrap();
    plate = curvyo_sketch_core::create_circle(
        &plate,
        "hole2",
        Position::mm(60.0, 20.0),
        curvyo_sketch_core::Length(4.0),
    )
    .unwrap();
    let circles: Vec<_> = plate
        .entities
        .iter()
        .filter_map(|(id, e)| {
            matches!(e, curvyo_sketch_core::Entity::Circle { .. }).then_some(id.clone())
        })
        .collect();
    plate.constraints.insert(
        "radius".into(),
        Constraint::driving(Relation::Radius {
            curve: circles[0].clone(),
            value: Expression::Reference("radius".into()),
        }),
    );
    plate.constraints.insert(
        "equal_holes".into(),
        Constraint::driving(Relation::Equal(circles[0].clone(), circles[1].clone())),
    );
    revision_3_relations(&mut plate);
    revision_4_relations(&mut plate);
    let id = doc
        .create_sketch(&plate, Affine::IDENTITY, &Style::default())
        .unwrap();
    let solved = doc.sketch(id).unwrap();
    assert!(
        matches!(
            solved.status,
            Some(curvyo_sketch_core::SolveStatus::UnderConstrained(_))
        ) && solved.conflicting.is_empty()
            && solved.redundant.is_empty(),
        "{:?}",
        solved.status
    );
    let clone = doc.linked_clone(&[id], &[]).unwrap()[0];
    doc.linked_clone(&[clone], &[]).unwrap();
    fixture("sketch_plate_v29.curvyo", &doc, true);
    let meta = doc.sketch_meta(id).unwrap();
    let entities = crate::sketch_codec::collection(&meta, crate::sketch_codec::KEYS[1]).unwrap();
    crate::sketch_codec::write(
        &entities,
        "dangling",
        &SketchEntityRecord {
            entity: curvyo_sketch_core::Entity::Line {
                start: "missing".into(),
                end: "origin".into(),
            },
            construction: true,
        },
    )
    .unwrap();
    doc.commit_with_label("merged_dangling");
    fixture("sketch_dangling_v29.curvyo", &doc, true);
    entities.delete("dangling").unwrap();
    let mut saved: SketchCheckpoint =
        crate::sketch_codec::atomic(&meta, crate::sketch_codec::KEYS[5]).unwrap();
    saved.candidate.entities.insert(
        "origin".into(),
        curvyo_sketch_core::Entity::Point {
            position: Position::mm(9.0, 0.0),
            fixed: true,
        },
    );
    crate::sketch_codec::write(&meta, crate::sketch_codec::KEYS[5], &saved).unwrap();
    doc.commit_with_label("damaged_checkpoint");
    fixture("sketch_checkpoint_invalid_v29.curvyo", &doc, false);
    meta.delete(crate::sketch_codec::KEYS[5]).unwrap();
    entities.insert("origin", "bad record").unwrap();
    doc.commit_with_label("damaged_record");
    fixture("sketch_record_invalid_v29.curvyo", &doc, false);
}

/// Customer revision 4 relation records (0066 AC44, format 29): a horizontal distance over a
/// line's endpoints and a vertical distance between two points.
fn revision_4_relations(plate: &mut Sketch) {
    let Some(curvyo_sketch_core::Entity::Line { start, end }) =
        plate.entities.get(&"guide:line0".into()).cloned()
    else {
        panic!("the guide line exists");
    };
    let length = |mm| Expression::Literal(Quantity::length(mm).unwrap());
    plate.constraints.insert(
        "guide_run".into(),
        Constraint::driving(Relation::HorizontalDistance {
            a: start,
            b: end,
            value: length(20.0),
        }),
    );
    plate.constraints.insert(
        "marks_apart".into(),
        Constraint::driving(Relation::VerticalDistance {
            a: "bottom_mark".into(),
            b: "top_mark".into(),
            value: length(20.0),
        }),
    );
}

/// Customer revision 3 relation records (0066 AC37, AC39, AC40): point on line, symmetric about
/// a point and about an axis, horizontal and vertical point pairs, a line's angle against the
/// X axis and both ellipse radii.
fn revision_3_relations(plate: &mut Sketch) {
    *plate = curvyo_sketch_core::create_polyline(
        plate,
        "guide",
        &[Position::mm(10.0, 50.0), Position::mm(30.0, 60.0)],
        false,
    )
    .unwrap();
    *plate = curvyo_sketch_core::create_ellipse(
        plate,
        "oval",
        Position::mm(60.0, 60.0),
        Position::mm(75.0, 60.0),
        curvyo_sketch_core::Length(6.0),
    )
    .unwrap();
    let length = |mm| Expression::Literal(Quantity::length(mm).unwrap());
    for (id, x, y) in [
        ("centre", 40.0, 20.0),
        ("mark", 40.0, 0.0),
        ("top_mark", 70.0, 10.0),
        ("bottom_mark", 70.0, -10.0),
    ] {
        plate.entities.insert(
            id.into(),
            curvyo_sketch_core::Entity::Point {
                position: Position::mm(x, y),
                fixed: false,
            },
        );
    }
    for (id, relation) in [
        (
            "holes_symmetric",
            Relation::SymmetricPoint {
                source: "hole1:center".into(),
                image: "hole2:center".into(),
                center: "centre".into(),
            },
        ),
        (
            "guide_angle",
            Relation::AxisAngle {
                line: "guide:line0".into(),
                value: Expression::Literal(Quantity::angle(0.5_f64.atan()).unwrap()),
            },
        ),
        (
            "oval_major",
            Relation::EllipseRadius {
                curve: "oval:curve".into(),
                axis: curvyo_sketch_core::EllipseAxis::Major,
                value: length(15.0),
            },
        ),
        (
            "oval_minor",
            Relation::EllipseRadius {
                curve: "oval:curve".into(),
                axis: curvyo_sketch_core::EllipseAxis::Minor,
                value: length(6.0),
            },
        ),
        (
            "holes_level",
            Relation::HorizontalPoints("hole1:center".into(), "hole2:center".into()),
        ),
        (
            "mark_on_edge",
            Relation::PointOn {
                point: "mark".into(),
                curve: "plate:line0".into(),
            },
        ),
        (
            "mark_below_centre",
            Relation::VerticalPoints("centre".into(), "mark".into()),
        ),
        (
            "marks_mirrored",
            Relation::SymmetricAxis {
                source: "top_mark".into(),
                image: "bottom_mark".into(),
                axis: curvyo_sketch_core::Axis::X,
            },
        ),
    ] {
        plate
            .constraints
            .insert(id.into(), Constraint::driving(relation));
    }
}

#[test]
fn slot_and_converted_goldens() {
    let doc = Document::new(902);
    let graph = curvyo_sketch_core::create_slot(
        &Sketch::new("origin".into()),
        "slot",
        Position::mm(0.0, 0.0),
        Position::mm(30.0, 0.0),
        curvyo_sketch_core::Length(10.0),
    )
    .unwrap();
    let id = doc
        .create_sketch(
            &graph,
            Affine::translation(Vec2::new(20.0, 30.0)),
            &Style::default(),
        )
        .unwrap();
    fixture("sketch_slot_v29.curvyo", &doc, true);
    let ids: Vec<_> = (0..doc.sketch(id).unwrap().resolved.all_anchors().count())
        .map(|n| AnchorId::new(902, 1000 + n as u64))
        .collect();
    doc.convert_sketch_to_path(id, &ids).unwrap();
    fixture("sketch_converted_v29.curvyo", &doc, true);
}

#[test]
fn page_sketch_claim_goldens() {
    let doc = Document::new(903);
    let page = doc.pages()[0].id.clone();
    let id = doc
        .create_page_sketch(&page, &Sketch::new("origin".into()), &Style::default())
        .unwrap();
    fixture("sketch_page_owner_v29.curvyo", &doc, true);
    let root = doc.loro().get_map(crate::document::ROOT_MAP);
    let key = crate::pages::key(&page, "sketch_owner");
    root.insert(&key, "invalid identity").unwrap();
    doc.commit_with_label("damaged_claim");
    fixture("sketch_page_claim_invalid_v29.curvyo", &doc, false);
    root.delete(&key).unwrap();
    let meta = doc.sketch_meta(id).unwrap();
    meta.insert(crate::sketch_page::PAGE_KEY, 123).unwrap();
    doc.commit_with_label("damaged_assignment");
    fixture("sketch_page_assignment_invalid_v29.curvyo", &doc, false);
}

#[test]
fn dimension_label_goldens() {
    let doc = Document::new(904);
    let mut graph = Sketch::new("origin".into());
    graph = curvyo_sketch_core::create_circle(
        &graph,
        "circle",
        Position::mm(10.0, 10.0),
        curvyo_sketch_core::Length(5.0),
    )
    .unwrap();
    let circle = graph
        .entities
        .iter()
        .find_map(|(id, e)| {
            matches!(e, curvyo_sketch_core::Entity::Circle { .. }).then(|| id.clone())
        })
        .unwrap();
    graph.constraints.insert(
        "radius".into(),
        Constraint::driving(Relation::Radius {
            curve: circle,
            value: Expression::Literal(Quantity::length(5.0).unwrap()),
        }),
    );
    let id = doc
        .create_page_sketch(&doc.pages()[0].id, &graph, &Style::default())
        .unwrap();
    doc.set_sketch_label(id, &"radius".into(), Position::mm(17.0, -3.0))
        .unwrap();
    fixture("sketch_dimension_label_v29.curvyo", &doc, true);
    let meta = doc.sketch_meta(id).unwrap();
    let labels = crate::sketch_codec::collection(&meta, crate::sketch_labels::KEY).unwrap();
    labels.insert("radius", "invalid position").unwrap();
    doc.commit_with_label("damaged_label");
    fixture("sketch_label_invalid_v29.curvyo", &doc, false);
}

/// A damaged v29 golden: a horizontal distance record with a negative value (0066 AC46). The
/// file opens, the record is diagnosed and the saved solution is not trusted.
#[test]
fn a_negative_axis_distance_golden_opens_stale() {
    let doc = Document::new(905);
    let graph = curvyo_sketch_core::create_polyline(
        &Sketch::new("origin".into()),
        "guide",
        &[Position::mm(10.0, 5.0), Position::mm(30.0, 15.0)],
        false,
    )
    .unwrap();
    let id = doc
        .create_sketch(&graph, Affine::IDENTITY, &Style::default())
        .unwrap();
    let meta = doc.sketch_meta(id).unwrap();
    let constraints = crate::sketch_codec::collection(&meta, crate::sketch_codec::KEYS[2]).unwrap();
    let record = Constraint::driving(Relation::HorizontalDistance {
        a: "guide:p0".into(),
        b: "guide:p1".into(),
        value: Expression::Literal(Quantity::length(-5.0).unwrap()),
    });
    crate::sketch_codec::write(&constraints, "guide_run", &record).unwrap();
    doc.commit_with_label("negative_axis_distance");
    fixture("sketch_axis_distance_negative_v29.curvyo", &doc, true);
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sketch_axis_distance_negative_v29.curvyo"),
    )
    .unwrap();
    let opened = unpack(919, &bytes).unwrap();
    assert!(opened.sketch(id).unwrap().stale);
}

/// A slot saved before its caps dropped the two Sweep relations (2026-10-31) still loads with
/// every record and solves to the same outline; its extra relations are only diagnosed redundant.
#[test]
fn a_slot_saved_with_cap_sweeps_still_loads_and_solves() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sketch_slot_sweeps_v28.curvyo");
    let loaded = unpack(919, &std::fs::read(path).unwrap()).unwrap();
    let id = Document::new(902)
        .create_sketch(
            &Sketch::new("origin".into()),
            Affine::translation(Vec2::new(20.0, 30.0)),
            &Style::default(),
        )
        .unwrap();
    let snapshot = loaded.sketch(id).unwrap();
    let sweeps = snapshot
        .graph
        .constraints
        .values()
        .filter(|c| matches!(c.relation, Relation::Sweep { .. }))
        .count();
    assert_eq!(sweeps, 2);
    let current = curvyo_sketch_core::create_slot(
        &Sketch::new("origin".into()),
        "slot",
        Position::mm(0.0, 0.0),
        Position::mm(30.0, 0.0),
        curvyo_sketch_core::Length(10.0),
    )
    .unwrap();
    let solved = curvyo_sketch_core::solve(&snapshot.graph, &curvyo_sketch_core::SolveMode::Edit)
        .unwrap()
        .candidate
        .unwrap();
    for (entity, expected) in &current.entities {
        if let curvyo_sketch_core::Entity::Point { position, .. } = expected {
            let curvyo_sketch_core::Entity::Point { position: got, .. } = &solved.entities[entity]
            else {
                panic!("{entity:?}");
            };
            assert!(got.distance(*position).0 < 1e-6, "{entity:?}");
        }
    }
}

/// The version-28 goldens stay as older-version fixtures (format 29, 0066 revision 4): the valid
/// ones still open, the damaged ones are still refused.
#[test]
fn format_28_sketch_goldens_still_open() {
    for (name, valid) in [
        ("plate", true),
        ("dangling", true),
        ("slot", true),
        ("converted", true),
        ("page_owner", true),
        ("dimension_label", true),
        ("slot_sweeps", true),
        ("checkpoint_invalid", false),
        ("record_invalid", false),
        ("page_claim_invalid", false),
        ("page_assignment_invalid", false),
        ("label_invalid", false),
    ] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/sketch_{name}_v28.curvyo"));
        let opened = unpack(919, &std::fs::read(path).unwrap());
        assert_eq!(opened.is_ok(), valid, "{name}");
    }
}
