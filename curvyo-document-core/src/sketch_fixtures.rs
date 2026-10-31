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
    let id = doc
        .create_sketch(&plate, Affine::IDENTITY, &Style::default())
        .unwrap();
    let clone = doc.linked_clone(&[id], &[]).unwrap()[0];
    doc.linked_clone(&[clone], &[]).unwrap();
    fixture("sketch_plate_v28.curvyo", &doc, true);
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
    fixture("sketch_dangling_v28.curvyo", &doc, true);
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
    fixture("sketch_checkpoint_invalid_v28.curvyo", &doc, false);
    meta.delete(crate::sketch_codec::KEYS[5]).unwrap();
    entities.insert("origin", "bad record").unwrap();
    doc.commit_with_label("damaged_record");
    fixture("sketch_record_invalid_v28.curvyo", &doc, false);
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
    fixture("sketch_slot_v28.curvyo", &doc, true);
    let ids: Vec<_> = (0..doc.sketch(id).unwrap().resolved.all_anchors().count())
        .map(|n| AnchorId::new(902, 1000 + n as u64))
        .collect();
    doc.convert_sketch_to_path(id, &ids).unwrap();
    fixture("sketch_converted_v28.curvyo", &doc, true);
}

#[test]
fn page_sketch_claim_goldens() {
    let doc = Document::new(903);
    let page = doc.pages()[0].id.clone();
    let id = doc
        .create_page_sketch(&page, &Sketch::new("origin".into()), &Style::default())
        .unwrap();
    fixture("sketch_page_owner_v28.curvyo", &doc, true);
    let root = doc.loro().get_map(crate::document::ROOT_MAP);
    let key = crate::pages::key(&page, "sketch_owner");
    root.insert(&key, "invalid identity").unwrap();
    doc.commit_with_label("damaged_claim");
    fixture("sketch_page_claim_invalid_v28.curvyo", &doc, false);
    root.delete(&key).unwrap();
    let meta = doc.sketch_meta(id).unwrap();
    meta.insert(crate::sketch_page::PAGE_KEY, 123).unwrap();
    doc.commit_with_label("damaged_assignment");
    fixture("sketch_page_assignment_invalid_v28.curvyo", &doc, false);
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
    fixture("sketch_dimension_label_v28.curvyo", &doc, true);
    let meta = doc.sketch_meta(id).unwrap();
    let labels = crate::sketch_codec::collection(&meta, crate::sketch_labels::KEY).unwrap();
    labels.insert("radius", "invalid position").unwrap();
    doc.commit_with_label("damaged_label");
    fixture("sketch_label_invalid_v28.curvyo", &doc, false);
}
