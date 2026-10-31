//! Independent page-sketch ownership cases derived from the revised acceptance matrix.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{AnchorId, Document, NodeId, SketchDelta, Style, Vec2, pack, unpack};
use curvyo_sketch_core::{Entity, Position, Sketch};

fn graph(x: f64) -> Sketch {
    let mut graph = Sketch::new("origin".into());
    graph.entities.insert(
        "end".into(),
        Entity::Point {
            position: Position::mm(x, 20.0),
            fixed: false,
        },
    );
    graph.entities.insert(
        "line".into(),
        Entity::Line {
            start: "origin".into(),
            end: "end".into(),
        },
    );
    graph
}
fn create(d: &Document, page: &str, x: f64) -> NodeId {
    d.create_page_sketch(page, &graph(x), &Style::default())
        .unwrap()
}
fn reopen(d: &Document, peer: u64) -> Document {
    unpack(peer, &pack(d, "revision-acceptance").unwrap()).unwrap()
}
fn exchange(a: &Document, b: &Document) {
    let aa = a.export_updates(&b.version()).unwrap();
    let bb = b.export_updates(&a.version()).unwrap();
    a.import_updates(&bb).unwrap();
    b.import_updates(&aa).unwrap();
}
#[test]
fn r01_r02_first_owner_is_atomic_centered_and_roundtrips_with_stable_ids() {
    let d = Document::new(71);
    let page = d.pages()[0].clone();
    d.move_page(&page.id, Vec2::new(300.0, -40.0), &[]).unwrap();
    let frame = d.pages()[0].clone();
    let before = d.version();
    let count = d.history_len();
    assert_eq!(d.page_sketch(&page.id), None);
    assert_eq!(d.version(), before);
    let id = create(&d, &page.id, 10.0);
    assert_eq!(d.history_len(), count + 1);
    assert_eq!(d.page_sketch(&page.id), Some(id));
    assert_eq!(d.sketch_page(id).as_deref(), Some(page.id.as_str()));
    let saved = d.sketch(id).unwrap();
    assert_eq!(saved.graph, graph(10.0));
    let cx = frame.origin.x + frame.size.width.as_mm() / 2.0;
    let cy = frame.origin.y + frame.size.height.as_mm() / 2.0;
    assert!(
        saved
            .resolved
            .all_anchors()
            .any(|a| (a.point.x - cx - 10.0).abs() < 1e-6 && (a.point.y - cy - 20.0).abs() < 1e-6)
    );
    assert!(
        saved
            .resolved
            .all_anchors()
            .any(|a| (a.point.x - cx).abs() < 1e-6 && (a.point.y - cy).abs() < 1e-6)
    );
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert_eq!(d.page_sketch(&page.id), None);
    assert!(d.sketch(id).is_none());
    d.redo_step(step).unwrap();
    assert_eq!(d.page_sketch(&page.id), Some(id));
    let restored = reopen(&d, 72);
    assert_eq!(restored.page_sketch(&page.id), Some(id));
    assert_eq!(restored.sketch(id).unwrap().graph, graph(10.0));
    assert_eq!(restored.sketch(id).unwrap().placement, saved.placement);
}
#[test]
fn r01_occupied_and_missing_page_refusal_is_inert() {
    let d = Document::new(73);
    let page = d.pages()[0].id.clone();
    let id = create(&d, &page, 10.0);
    let before = d.version();
    let steps = d.history_len();
    for target in [page.as_str(), "nonexistent-page"] {
        assert!(
            d.create_page_sketch(target, &graph(99.0), &Style::default())
                .is_err()
        );
        assert_eq!(d.version(), before);
        assert_eq!(d.history_len(), steps);
    }
    assert_eq!(d.page_sketch(&page), Some(id));
    assert_eq!(d.sketch(id).unwrap().graph, graph(10.0));
}
#[test]
fn r04_clones_are_nonowners_and_source_edits_keep_their_links() {
    let d = Document::new(74);
    let page = d.pages()[0].id.clone();
    let source = create(&d, &page, 10.0);
    let target = d.add_page();
    let clone = d.linked_clone(&[source], &[]).unwrap()[0];
    assert_eq!(d.sketch_page(clone), None);
    assert_eq!(d.page_sketch(&target.id), None);
    assert_eq!(d.page_sketch(&page), Some(source));
    let mut delta = SketchDelta::default();
    delta.entities.insert(
        "end".into(),
        Some(curvyo_document_core::SketchEntityRecord {
            entity: graph(45.0).entities[&"end".into()].clone(),
            construction: false,
        }),
    );
    d.apply_sketch_delta(source, &delta).unwrap();
    let obj = d.object(clone).unwrap();
    let curvyo_document_core::ObjectSnapshot::Clone(clone) = obj else {
        panic!("derived clone required")
    };
    assert_eq!(clone.chain.origin, source);
    assert_eq!(
        clone.resolved.anchors,
        d.sketch(source).unwrap().resolved.anchors
    );
    assert!(
        clone
            .resolved
            .all_anchors()
            .any(|a| (a.point.x - 150.0).abs() < 1e-6 && (a.point.y - 168.5).abs() < 1e-6)
    );
    let restored = reopen(&d, 75);
    assert_eq!(restored.page_sketch(&page), Some(source));
    assert_eq!(restored.page_sketch(&target.id), None);
}
#[test]
fn r04_concurrent_claims_converge_without_discarding_or_editing_loser() {
    let a = Document::new(76);
    let page = a.pages()[0].id.clone();
    let b = reopen(&a, 77);
    let left = create(&a, &page, 10.0);
    let right = create(&b, &page, 40.0);
    exchange(&a, &b);
    let winner = a.page_sketch(&page).unwrap();
    assert_eq!(b.page_sketch(&page), Some(winner));
    let loser = if winner == left { right } else { left };
    for d in [&a, &b] {
        let version = d.version();
        let count = d.history_len();
        assert_eq!(d.sketch(left).unwrap().graph, graph(10.0));
        assert_eq!(d.sketch(right).unwrap().graph, graph(40.0));
        assert!(d.sketch_ownership_conflict(loser));
        assert!(!d.sketch_ownership_conflict(winner));
        assert!(d.sketch(loser).unwrap().diagnosis.is_some());
        let mut delta = SketchDelta::default();
        delta.entities.insert(
            "extra".into(),
            Some(curvyo_document_core::SketchEntityRecord {
                entity: Entity::Point {
                    position: Position::mm(8.0, 9.0),
                    fixed: false,
                },
                construction: false,
            }),
        );
        assert!(d.apply_sketch_delta(loser, &delta).is_err());
        assert_eq!(d.version(), version);
        assert_eq!(d.history_len(), count);
    }
    a.delete_objects(&[loser]).unwrap();
    assert_eq!(a.page_sketch(&page), Some(winner));
    let step = a.own_steps().last().unwrap().step;
    a.undo_step(step).unwrap();
    assert_eq!(a.page_sketch(&page), Some(winner));
    assert!(a.sketch(loser).is_some());
}
#[test]
fn r03_delete_recreate_and_object_history_restore_claims() {
    let d = Document::new(78);
    let page = d.pages()[0].id.clone();
    let id = create(&d, &page, 10.0);
    d.delete_objects(&[id]).unwrap();
    assert_eq!(d.page_sketch(&page), None);
    let deletion = d.own_steps().last().unwrap().step;
    d.undo_step(deletion).unwrap();
    assert_eq!(d.page_sketch(&page), Some(id));
    d.redo_step(deletion).unwrap();
    let newer = create(&d, &page, 50.0);
    assert_ne!(id, newer);
    assert_eq!(d.page_sketch(&page), Some(newer));
    d.undo_object_step(newer).unwrap();
    assert_eq!(d.page_sketch(&page), None);
    d.redo_object_step(newer).unwrap();
    assert_eq!(d.page_sketch(&page), Some(newer));
}
#[test]
fn r03_conversion_releases_claim_and_object_undo_restores_same_owner() {
    let d = Document::new(79);
    let page = d.pages()[0].id.clone();
    let id = create(&d, &page, 10.0);
    let ids: Vec<_> = (0..d.sketch(id).unwrap().resolved.all_anchors().count())
        .map(|n| AnchorId::new(79, u64::try_from(n).unwrap()))
        .collect();
    let count = d.history_len();
    d.convert_sketch_to_path(id, &ids).unwrap();
    assert_eq!(d.history_len(), count + 1);
    assert_eq!(d.page_sketch(&page), None);
    assert!(d.sketch(id).is_none());
    d.undo_object_step(id).unwrap();
    assert_eq!(d.page_sketch(&page), Some(id));
    assert_eq!(d.sketch(id).unwrap().graph, graph(10.0));
    d.redo_object_step(id).unwrap();
    assert_eq!(d.page_sketch(&page), None);
}
#[test]
fn r03_page_delete_keeps_artwork_and_undo_restores_association() {
    let d = Document::new(80);
    let page = d.add_page();
    let id = create(&d, &page.id, 10.0);
    d.delete_page(&page.id).unwrap();
    let step = d.own_steps().last().unwrap().step;
    assert!(d.sketch(id).is_some());
    assert_eq!(d.page_sketch(&page.id), None);
    assert!(d.sketch_ownership_conflict(id));
    d.undo_step(step).unwrap();
    assert_eq!(d.page_sketch(&page.id), Some(id));
    assert!(!d.sketch_ownership_conflict(id));
}
#[test]
fn r10_current_format_owner_goldens_distinguish_structural_damage() {
    for (name, valid) in [
        ("sketch_page_owner_v28.curvyo", true),
        ("sketch_page_claim_invalid_v28.curvyo", false),
        ("sketch_page_assignment_invalid_v28.curvyo", false),
    ] {
        let bytes = std::fs::read(format!(
            "{}/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let opened = unpack(81, &bytes);
        if valid {
            let d = opened.unwrap();
            assert!(d.pages().iter().any(|p| d.page_sketch(&p.id).is_some()));
            let v = d.version();
            let again = reopen(&d, 82);
            assert_eq!(again.pages(), d.pages());
            assert_eq!(d.version(), v);
        } else {
            assert!(
                matches!(opened, Err(curvyo_document_core::OpenError::Damaged)),
                "{name}"
            );
        }
    }
}

#[test]
fn r03_resize_does_not_recenter_and_explicit_content_move_applies_once() {
    let d = Document::new(83);
    let page = d.pages()[0].id.clone();
    let id = create(&d, &page, 10.0);
    let old = d.sketch(id).unwrap();
    d.resize_page(
        &page,
        curvyo_document_core::DocumentSize::from_mm(500.0, 400.0),
        &[],
    )
    .unwrap();
    assert_eq!(d.sketch(id).unwrap().placement, old.placement);
    d.move_page(&page, Vec2::new(35.0, -12.0), &[id]).unwrap();
    let new = d.sketch(id).unwrap();
    assert_eq!(new.graph, old.graph);
    for (a, b) in old.resolved.all_anchors().zip(new.resolved.all_anchors()) {
        assert!((b.point.x - a.point.x - 35.0).abs() < 1e-6);
        assert!((b.point.y - a.point.y + 12.0).abs() < 1e-6);
    }
}

#[test]
fn r23_dimension_label_metadata_is_finite_atomic_and_independent_of_solver_cache() {
    use curvyo_sketch_core::{Constraint, Expression, Quantity, Relation};
    let d = Document::new(920);
    let page = d.pages()[0].id.clone();
    let mut shape = graph(10.0);
    shape.constraints.insert(
        "length".into(),
        Constraint::driving(Relation::Length {
            curve: "line".into(),
            value: Expression::Literal(Quantity::length(500.0_f64.sqrt()).unwrap()),
        }),
    );
    let id = d
        .create_page_sketch(&page, &shape, &Style::default())
        .unwrap();
    let before = d.sketch(id).unwrap();
    for (constraint, position) in [
        ("length", Position::mm(f64::NAN, 1.0)),
        ("length", Position::mm(1.0, f64::INFINITY)),
        ("missing", Position::mm(1.0, 2.0)),
    ] {
        let version = d.version();
        let count = d.history_len();
        assert!(
            d.set_sketch_label(id, &constraint.into(), position)
                .is_err()
        );
        assert_eq!(d.version(), version);
        assert_eq!(d.history_len(), count);
        assert!(d.sketch(id).unwrap().labels.is_empty());
    }
    let position = Position::mm(31.0, -7.0);
    let count = d.history_len();
    assert!(d.set_sketch_label(id, &"length".into(), position).unwrap());
    assert_eq!(d.history_len(), count + 1);
    let after = d.sketch(id).unwrap();
    assert_eq!(after.graph, before.graph);
    assert_eq!(after.solution, before.solution);
    assert_eq!(after.checkpoint, before.checkpoint);
    assert_eq!(after.labels[&"length".into()], position);
    let version = d.version();
    assert!(!d.set_sketch_label(id, &"length".into(), position).unwrap());
    assert_eq!(d.version(), version);
    let restored = reopen(&d, 921);
    assert_eq!(restored.sketch(id).unwrap().labels, after.labels);
    let target = d.add_page();
    let copied = d
        .duplicate_objects_to_page(
            &[curvyo_document_core::CopySource {
                id,
                anchor_ids: vec![],
            }],
            Vec2::new(0.0, 0.0),
            &target.id,
        )
        .unwrap()[0];
    let copy = d.sketch(copied).unwrap();
    assert_eq!(copy.labels.len(), 1);
    let (key, point) = copy.labels.iter().next().unwrap();
    assert_ne!(key, &"length".into());
    assert!(copy.graph.constraints.contains_key(key));
    assert_eq!(*point, position);
    let mut delta = SketchDelta::default();
    delta.constraints.insert("length".into(), None);
    d.apply_sketch_delta(id, &delta).unwrap();
    let step = d.own_steps().last().unwrap().step;
    assert!(d.sketch(id).unwrap().labels.is_empty());
    d.undo_step(step).unwrap();
    assert_eq!(d.sketch(id).unwrap().labels, after.labels);
}
#[test]
fn r23_label_goldens_open_or_refuse_structural_damage() {
    for (name, valid) in [
        ("sketch_dimension_label_v28.curvyo", true),
        ("sketch_label_invalid_v28.curvyo", false),
    ] {
        let bytes = std::fs::read(format!(
            "{}/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let opened = unpack(922, &bytes);
        if valid {
            let d = opened.unwrap();
            let id = d.pages().iter().find_map(|p| d.page_sketch(&p.id)).unwrap();
            assert!(!d.sketch(id).unwrap().labels.is_empty());
            assert_eq!(
                reopen(&d, 923).sketch(id).unwrap().labels,
                d.sketch(id).unwrap().labels
            );
        } else {
            assert!(matches!(
                opened,
                Err(curvyo_document_core::OpenError::Damaged)
            ));
        }
    }
}
