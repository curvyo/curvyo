//! Sketch instances (`specs/0099-sketch-pages`, criteria 23, 26 to 28, 30): own values per clone,
//! one solve per distinct set, the saved fallback and the damage rules, at document level.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{
    Affine, CloneSnapshot, Document, ExposedKey, InstanceError, InstanceValue, InstanceWrite,
    NodeId, ObjectSnapshot, Style, ValueSet, pack, unpack,
};
use curvyo_sketch_core as sketch;

fn graph() -> sketch::Sketch {
    let mut s = sketch::Sketch::new("origin".into());
    s.entities.insert(
        "end".into(),
        sketch::Entity::Point {
            position: sketch::Position::mm(60.0, 0.0),
            fixed: false,
        },
    );
    s.entities.insert(
        "line".into(),
        sketch::Entity::Line {
            start: "origin".into(),
            end: "end".into(),
        },
    );
    for (id, value) in [("width", 80.0), ("margin", 10.0)] {
        s.parameters.insert(
            id.into(),
            sketch::Parameter::literal(id, sketch::Quantity::length(value).unwrap()),
        );
    }
    let formula = sketch::Expression::parse("width - 2 * margin", &s.parameters).unwrap();
    s.parameters.insert(
        "spacing".into(),
        sketch::Parameter::expression("spacing", sketch::Dimension::Length, formula),
    );
    s.constraints.insert(
        "horizontal".into(),
        sketch::Constraint::driving(sketch::Relation::Horizontal("line".into())),
    );
    s.constraints.insert(
        "length".into(),
        sketch::Constraint::driving(sketch::Relation::Length {
            curve: "line".into(),
            value: sketch::Expression::Reference("spacing".into()),
        }),
    );
    s
}

/// The same line with a second, redundant dimension `reach` that conflicts once `width` grows.
fn graph_with_reach() -> sketch::Sketch {
    let mut s = graph();
    s.constraints.insert(
        "reach".into(),
        sketch::Constraint::driving(sketch::Relation::Distance {
            a: "origin".into(),
            b: "end".into(),
            value: sketch::Expression::Literal(sketch::Quantity::length(60.0).unwrap()),
        }),
    );
    s
}

fn width_key() -> ExposedKey {
    ExposedKey::parameter(&"width".into())
}

fn length_key() -> ExposedKey {
    ExposedKey::dimension(&"length".into())
}

fn setup(graph: &sketch::Sketch) -> (Document, NodeId, NodeId) {
    let d = Document::new(1);
    let sketch = d
        .create_sketch(graph, Affine::IDENTITY, &Style::default())
        .unwrap();
    let clone = d.linked_clone(&[sketch], &[]).unwrap()[0];
    (d, sketch, clone)
}

fn clone_of(d: &Document, id: NodeId) -> CloneSnapshot {
    match d.read_objects().get(id).unwrap() {
        ObjectSnapshot::Clone(clone) => clone.clone(),
        other => panic!("not a clone: {other:?}"),
    }
}

/// The width of the clone's outline in millimetres.
fn span(clone: &CloneSnapshot) -> f64 {
    let xs: Vec<f64> = clone.resolved.anchors.iter().map(|a| a.point.x).collect();
    xs.iter().copied().fold(f64::MIN, f64::max) - xs.iter().copied().fold(f64::MAX, f64::min)
}

fn own(clone: NodeId, pairs: &[(&ExposedKey, f64)]) -> InstanceWrite {
    let mut set = ValueSet::default();
    for (key, mm) in pairs {
        set.insert((*key).clone(), InstanceValue::from_mm(*mm));
    }
    InstanceWrite {
        clone,
        own: set,
        last_solvable: None,
    }
}

#[test]
fn a_clone_with_an_own_value_draws_the_sketch_solved_with_it() {
    let (d, sketch, clone) = setup(&graph());
    assert!((span(&clone_of(&d, clone)) - 60.0).abs() < 1e-6);
    d.expose_parameter(sketch, &width_key(), None).unwrap();
    d.write_instance_values(
        &[own(clone, &[(&width_key(), 100.0)])],
        "change_parameter",
        None,
    )
    .unwrap();
    let read = clone_of(&d, clone);
    assert!(
        (span(&read) - 80.0).abs() < 1e-6,
        "width 100 gives spacing 80"
    );
    assert!(read.chain.own_params && !read.chain.no_solution);
    let sketch_read = d.sketch(sketch).unwrap();
    assert!((sketch_read.solution.point(&"end".into()).unwrap().x.0 - 60.0).abs() < 1e-6);
}

#[test]
fn a_value_outside_the_limits_is_clamped_at_read_and_not_rewritten() {
    let (d, sketch, clone) = setup(&graph());
    d.expose_parameter(sketch, &width_key(), None).unwrap();
    d.write_instance_values(
        &[own(clone, &[(&width_key(), 100.0)])],
        "change_parameter",
        None,
    )
    .unwrap();
    d.set_exposed_limits(sketch, &width_key(), Some(50.0), Some(90.0), None)
        .unwrap();
    let read = clone_of(&d, clone);
    assert!(
        (span(&read) - 70.0).abs() < 1e-6,
        "clamped to 90, spacing 70"
    );
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(json.contains("100"), "the stored 100 is kept: {json}");
}

#[test]
fn clones_with_equal_sets_share_one_outline_and_one_solve() {
    let (d, sketch, first) = setup(&graph());
    let before = d.instance_solve_count();
    let second = d.linked_clone(&[sketch], &[]).unwrap()[0];
    let third = d.linked_clone(&[sketch], &[]).unwrap()[0];
    d.expose_parameter(sketch, &width_key(), None).unwrap();
    d.write_instance_values(
        &[
            own(first, &[(&width_key(), 100.0)]),
            own(second, &[(&width_key(), 100.0)]),
            own(third, &[(&width_key(), 90.0)]),
        ],
        "change_parameter",
        None,
    )
    .unwrap();
    let (a, b, c) = (
        clone_of(&d, first),
        clone_of(&d, second),
        clone_of(&d, third),
    );
    assert!(std::sync::Arc::ptr_eq(
        &a.chain.root_outline,
        &b.chain.root_outline
    ));
    assert!(!std::sync::Arc::ptr_eq(
        &a.chain.root_outline,
        &c.chain.root_outline
    ));
    assert_eq!(
        d.instance_solve_count() - before,
        2,
        "one solve per distinct set"
    );
    let again = d.instance_solve_count();
    let _ = d.read_objects();
    assert_eq!(
        d.instance_solve_count(),
        again,
        "the cache is keyed by content"
    );
}

#[test]
fn a_chain_takes_the_nearest_own_value_per_key() {
    let (d, sketch, a) = setup(&graph_with_reach_free());
    d.expose_parameter(sketch, &width_key(), None).unwrap();
    d.expose_parameter(sketch, &ExposedKey::parameter(&"margin".into()), None)
        .unwrap();
    let b = d.linked_clone(&[a], &[]).unwrap()[0];
    let margin = ExposedKey::parameter(&"margin".into());
    d.write_instance_values(
        &[own(a, &[(&width_key(), 100.0), (&margin, 10.0)])],
        "change_parameter",
        None,
    )
    .unwrap();
    d.write_instance_values(&[own(b, &[(&margin, 20.0)])], "change_parameter", None)
        .unwrap();
    let read_b = clone_of(&d, b);
    assert!(
        (span(&read_b) - 60.0).abs() < 1e-6,
        "width 100 from A, margin 20 from B"
    );
    assert!(read_b.chain.own_params);
    assert!(read_b.chain.inherited_values.get(&width_key()).is_some());
    assert!((span(&clone_of(&d, a)) - 80.0).abs() < 1e-6);
}

fn graph_with_reach_free() -> sketch::Sketch {
    graph()
}

#[test]
fn a_set_without_solution_draws_the_saved_fallback_and_survives_a_reopen() {
    let (d, sketch, clone) = setup(&graph_with_reach());
    let reach = ExposedKey::dimension(&"reach".into());
    d.expose_parameter(sketch, &width_key(), None).unwrap();
    d.expose_parameter(sketch, &reach, Some("Reach 1")).unwrap();
    let good = own(clone, &[(&width_key(), 100.0), (&reach, 80.0)]);
    d.write_instance_values(std::slice::from_ref(&good), "change_parameter", None)
        .unwrap();
    assert!((span(&clone_of(&d, clone)) - 80.0).abs() < 1e-6);
    let mut bad = own(clone, &[(&width_key(), 120.0), (&reach, 80.0)]);
    bad.last_solvable = Some(good.own.clone());
    d.write_instance_values(&[bad], "change_parameter", None)
        .unwrap();
    let read = clone_of(&d, clone);
    assert!(read.chain.no_solution && read.chain.own_params);
    assert!(
        (span(&read) - 80.0).abs() < 1e-6,
        "the last values that solved"
    );
    let follower = d.linked_clone(&[clone], &[]).unwrap()[0];
    let plain = clone_of(&d, follower);
    assert!(plain.chain.no_solution && !plain.chain.own_params);
    assert!(
        (span(&plain) - 80.0).abs() < 1e-6,
        "a plain clone matches its original"
    );
    let reopened = unpack(2, &pack(&d, "instances").unwrap()).unwrap();
    assert!((span(&clone_of(&reopened, clone)) - 80.0).abs() < 1e-6);
    assert!(clone_of(&reopened, clone).chain.no_solution);
    let json = String::from_utf8(reopened.export_json().unwrap()).unwrap();
    assert!(json.contains("instance_fallback"));
    d.write_instance_values(
        &[own(clone, &[(&width_key(), 100.0), (&reach, 80.0)])],
        "change_parameter",
        None,
    )
    .unwrap();
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(
        !json.contains("instance_fallback"),
        "a set that solves removes it"
    );
}

#[test]
fn without_a_saved_fallback_a_clone_draws_its_original_then_the_sketch() {
    let (d, sketch, clone) = setup(&graph_with_reach());
    d.expose_parameter(sketch, &width_key(), None).unwrap();
    d.write_instance_values(
        &[own(clone, &[(&width_key(), 120.0)])],
        "change_parameter",
        None,
    )
    .unwrap();
    let read = clone_of(&d, clone);
    assert!(read.chain.no_solution);
    assert!(
        (span(&read) - 60.0).abs() < 1e-6,
        "the sketch's own geometry"
    );
}

#[test]
fn stop_exposing_drops_every_clone_value_and_undo_restores_it() {
    let (d, sketch, clone) = setup(&graph());
    d.expose_parameter(sketch, &width_key(), None).unwrap();
    d.write_instance_values(
        &[own(clone, &[(&width_key(), 100.0)])],
        "change_parameter",
        None,
    )
    .unwrap();
    d.unexpose_parameter(sketch, &width_key()).unwrap();
    let read = clone_of(&d, clone);
    assert!(!read.chain.own_params && (span(&read) - 60.0).abs() < 1e-6);
    let step = d
        .own_steps()
        .into_iter()
        .rfind(|s| s.reverts.is_none())
        .unwrap()
        .step;
    d.undo_step(step).unwrap();
    assert!((span(&clone_of(&d, clone)) - 80.0).abs() < 1e-6);
}

#[test]
fn expose_refuses_an_unknown_key_a_reference_dimension_and_a_name_in_use() {
    let (d, sketch, _) = setup(&graph());
    assert_eq!(
        d.expose_parameter(sketch, &ExposedKey::parameter(&"nope".into()), None),
        Err(InstanceError::NotExposable)
    );
    assert_eq!(
        d.expose_parameter(
            sketch,
            &ExposedKey::dimension(&"horizontal".into()),
            Some("H")
        ),
        Err(InstanceError::NotExposable)
    );
    d.expose_parameter(sketch, &length_key(), Some("Distance 1"))
        .unwrap();
    assert_eq!(d.expose_parameter(sketch, &width_key(), None), Ok(()));
    assert_eq!(
        d.set_exposed_limits(sketch, &length_key(), None, None, Some("width")),
        Err(InstanceError::BadName),
        "a parameter's name is an exposed name too"
    );
}

#[test]
fn the_values_are_saved_with_the_file_and_a_file_without_them_opens_unchanged() {
    let (d, sketch, clone) = setup(&graph());
    d.expose_parameter(sketch, &width_key(), None).unwrap();
    d.write_instance_values(
        &[own(clone, &[(&width_key(), 100.0)])],
        "change_parameter",
        None,
    )
    .unwrap();
    let bytes = pack(&d, "instances").unwrap();
    let reopened = unpack(2, &bytes).unwrap();
    assert!((span(&clone_of(&reopened, clone)) - 80.0).abs() < 1e-6);
    assert_eq!(reopened.export_json().unwrap(), d.export_json().unwrap());
}

fn anchor_ids(count: usize, start: u64) -> Vec<curvyo_document_core::AnchorId> {
    (0..count as u64)
        .map(|i| curvyo_document_core::AnchorId::new(77, start + i))
        .collect()
}

fn is_path(d: &Document, id: NodeId) -> bool {
    matches!(d.read_objects().get(id), Some(ObjectSnapshot::Path(_)))
}

fn outline_span(d: &Document, id: NodeId) -> f64 {
    let read = d.read_objects();
    let anchors: Vec<f64> = match read.get(id).unwrap() {
        ObjectSnapshot::Path(path) => path.anchors.iter().map(|a| a.point.x).collect(),
        ObjectSnapshot::Clone(clone) => clone.resolved.anchors.iter().map(|a| a.point.x).collect(),
        other => panic!("{other:?}"),
    };
    anchors.iter().copied().fold(f64::MIN, f64::max)
        - anchors.iter().copied().fold(f64::MAX, f64::min)
}

/// A chain S, A (no values), B (own width 100), C (plain clone of B).
fn chain() -> (Document, NodeId, NodeId, NodeId, NodeId) {
    let (d, sketch, a) = setup(&graph());
    d.expose_parameter(sketch, &width_key(), None).unwrap();
    let b = d.linked_clone(&[a], &[]).unwrap()[0];
    let c = d.linked_clone(&[b], &[]).unwrap()[0];
    d.write_instance_values(
        &[own(b, &[(&width_key(), 100.0)])],
        "change_parameter",
        None,
    )
    .unwrap();
    (d, sketch, a, b, c)
}

#[test]
fn unlinking_a_clone_turns_the_clones_below_it_that_hold_values_into_paths() {
    let (d, _sketch, a, b, c) = chain();
    let count = clone_of(&d, a).resolved.all_anchors().count();
    d.unlink_clones(&[curvyo_document_core::CopySource {
        id: a,
        anchor_ids: anchor_ids(count, 100),
    }])
    .unwrap();
    assert!(is_path(&d, a));
    assert!(
        is_path(&d, b),
        "B holds an own value: it would jump on a path root"
    );
    assert!(
        (outline_span(&d, b) - 80.0).abs() < 1e-6,
        "from what it drew"
    );
    assert!(matches!(
        d.read_objects().get(c),
        Some(ObjectSnapshot::Clone(_))
    ));
    assert!(
        (outline_span(&d, c) - 80.0).abs() < 1e-6,
        "C keeps following B and draws the same"
    );
}

#[test]
fn deleting_the_sketch_turns_every_clone_with_values_into_a_path_that_draws_the_same() {
    let (d, sketch, a, b, c) = chain();
    d.delete_objects(&[sketch]).unwrap();
    assert!(is_path(&d, a));
    assert!(is_path(&d, b), "own values would be residue on a path root");
    assert!((outline_span(&d, b) - 80.0).abs() < 1e-6);
    assert!((outline_span(&d, c) - 80.0).abs() < 1e-6);
    assert!(
        matches!(d.read_objects().get(c), Some(ObjectSnapshot::Clone(_))),
        "C follows the path B"
    );
}

#[test]
fn deleting_a_middle_clone_that_gave_values_unlinks_the_clone_that_would_lose_them() {
    let (d, _sketch, _a, b, c) = chain();
    d.delete_objects(&[b]).unwrap();
    assert!(
        is_path(&d, c),
        "C drew B's width and must not jump back to the sketch's"
    );
    assert!((outline_span(&d, c) - 80.0).abs() < 1e-6);
}

#[test]
fn converting_the_sketch_to_paths_turns_the_clones_with_values_into_paths_too() {
    let (d, sketch, a, b, _c) = chain();
    let count = d.sketch(sketch).unwrap().resolved.all_anchors().count();
    d.convert_sketch_to_path(sketch, &anchor_ids(count, 500))
        .unwrap();
    assert!(is_path(&d, b));
    assert!((outline_span(&d, b) - 80.0).abs() < 1e-6);
    assert!(matches!(
        d.read_objects().get(a),
        Some(ObjectSnapshot::Clone(_))
    ));
    assert!(
        (outline_span(&d, a) - 60.0).abs() < 1e-6,
        "A had no values: it follows the paths"
    );
}

#[test]
fn a_field_drag_keeps_only_the_sets_a_read_used_and_the_one_preview_cached() {
    let (d, sketch, a) = setup(&graph());
    d.expose_parameter(sketch, &width_key(), None).unwrap();
    let b = d.linked_clone(&[sketch], &[]).unwrap()[0];
    let c = d.linked_clone(&[sketch], &[]).unwrap()[0];
    d.write_instance_values(
        &[
            own(a, &[(&width_key(), 100.0)]),
            own(b, &[(&width_key(), 90.0)]),
            own(c, &[(&width_key(), 80.0)]),
        ],
        "change_parameter",
        None,
    )
    .unwrap();
    let _ = d.read_objects();
    assert_eq!(
        d.instance_cached_sets(),
        3,
        "one set per distinct value set of the read"
    );
    for step in 0..40 {
        let mut set = ValueSet::default();
        set.insert(
            width_key(),
            InstanceValue::from_mm(70.0 + f64::from(step) * 0.37),
        );
        assert!(d.instance_preview(a, &set).is_some());
    }
    assert!(
        d.instance_cached_sets() <= 4,
        "a drag step replaces the previous preview: {}",
        d.instance_cached_sets()
    );
    d.write_instance_values(&[own(a, &[(&width_key(), 75.0)])], "change_parameter", None)
        .unwrap();
    let _ = d.read_objects();
    assert!(
        d.instance_cached_sets() <= 4,
        "{}",
        d.instance_cached_sets()
    );
}
