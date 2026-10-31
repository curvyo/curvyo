//! Page ownership survives history without creating artwork during navigation.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, Point, Style, pack, unpack};
use curvyo_sketch_core::{Entity, Position, Sketch};
fn graph() -> Sketch {
    let mut graph = Sketch::new("origin".into());
    graph.entities.insert(
        "point".into(),
        Entity::Point {
            position: Position::mm(10.0, 20.0),
            fixed: false,
        },
    );
    graph
}
#[test]
fn first_page_geometry_claim_is_centered_atomic_and_round_trips() {
    let doc = Document::new(9100);
    let page = doc.add_page();
    let before = doc.version();
    assert_eq!(doc.page_sketch(&page.id), None);
    assert_eq!(doc.version(), before);
    let count = doc.own_steps().len();
    let id = doc
        .create_page_sketch(&page.id, &graph(), &Style::default())
        .unwrap();
    assert_eq!(doc.own_steps().len(), count + 1);
    assert_eq!(doc.page_sketch(&page.id), Some(id));
    let snapshot = doc.sketch(id).unwrap();
    assert_eq!(
        snapshot.placement.apply_point(Point::new(0.0, 0.0)),
        Point::new(
            page.origin.x + page.size.width.as_mm() / 2.0,
            page.origin.y + page.size.height.as_mm() / 2.0
        )
    );
    let loaded = unpack(9101, &pack(&doc, "page sketch").unwrap()).unwrap();
    assert_eq!(loaded.page_sketch(&page.id), Some(id));
    let step = doc.own_steps().last().unwrap().step;
    doc.undo_step(step).unwrap();
    assert_eq!(doc.page_sketch(&page.id), None);
    assert!(doc.sketch(id).is_none());
    doc.redo_step(step).unwrap();
    assert_eq!(doc.page_sketch(&page.id), Some(id));
}
#[test]
fn occupied_page_refuses_creation_without_writes_and_deletion_releases_claim() {
    let doc = Document::new(9110);
    let page = doc.pages()[0].clone();
    let first = doc
        .create_page_sketch(&page.id, &graph(), &Style::default())
        .unwrap();
    let before = doc.version();
    assert!(
        doc.create_page_sketch(&page.id, &graph(), &Style::default())
            .is_err()
    );
    assert_eq!(doc.version(), before);
    doc.delete_objects(&[first]).unwrap();
    assert_eq!(doc.page_sketch(&page.id), None);
    let deletion = doc.own_steps().last().unwrap().step;
    doc.undo_step(deletion).unwrap();
    assert_eq!(doc.page_sketch(&page.id), Some(first));
}

#[test]
fn concurrent_first_claim_elects_one_owner_without_losing_the_other_graph() {
    let a = Document::new(9120);
    let b = unpack(9121, &pack(&a, "base").unwrap()).unwrap();
    let page = a.pages()[0].id.clone();
    let left = a
        .create_page_sketch(&page, &graph(), &Style::default())
        .unwrap();
    let right = b
        .create_page_sketch(&page, &graph(), &Style::default())
        .unwrap();
    let av = a.version();
    let bv = b.version();
    let au = a.export_updates(&bv).unwrap();
    let bu = b.export_updates(&av).unwrap();
    a.import_updates(&bu).unwrap();
    b.import_updates(&au).unwrap();
    let winner = a.page_sketch(&page).unwrap();
    assert_eq!(b.page_sketch(&page), Some(winner));
    let loser = if winner == left { right } else { left };
    assert_eq!(a.sketch(loser).unwrap().graph, graph());
    assert!(
        a.sketch(loser)
            .unwrap()
            .diagnosis
            .unwrap()
            .contains("owning sketch")
    );
    let before = a.version();
    let mut delta = curvyo_document_core::SketchDelta::default();
    delta.entities.insert("point".into(), None);
    assert!(a.apply_sketch_delta(loser, &delta).is_err());
    assert_eq!(a.version(), before);
    a.delete_objects(&[loser]).unwrap();
    assert_eq!(a.page_sketch(&page), Some(winner));
}

#[test]
fn original_copy_requires_empty_destination_and_clones_never_claim_it() {
    use curvyo_document_core::{CopySource, Vec2};
    let doc = Document::new(9130);
    let first = doc.pages()[0].id.clone();
    let original = doc
        .create_page_sketch(&first, &graph(), &Style::default())
        .unwrap();
    let source = CopySource {
        id: original,
        anchor_ids: vec![],
    };
    let before = doc.version();
    assert!(
        doc.duplicate_objects(std::slice::from_ref(&source), Vec2::new(5.0, 0.0))
            .is_err()
    );
    assert_eq!(doc.version(), before);
    let second = doc.add_page();
    let copied = doc
        .duplicate_objects_to_page(
            &[source],
            Vec2::new(second.origin.x, second.origin.y),
            &second.id,
        )
        .unwrap()[0];
    assert_eq!(doc.page_sketch(&second.id), Some(copied));
    assert!(
        doc.sketch(copied)
            .unwrap()
            .graph
            .entities
            .keys()
            .all(|id| !graph().entities.contains_key(id))
    );
    let clone = doc.linked_clone(&[original], &[]).unwrap()[0];
    assert_eq!(doc.sketch_page(clone), None);
    assert_eq!(doc.page_sketch(&first), Some(original));
}

#[test]
fn conversion_and_page_removal_preserve_claim_history_without_recentering() {
    use curvyo_document_core::AnchorId;
    let doc = Document::new(9140);
    let page = doc.add_page();
    let graph = curvyo_sketch_core::create_rectangle(
        &graph(),
        "plate",
        Position::mm(0.0, 0.0),
        Position::mm(20.0, 10.0),
    )
    .unwrap();
    let id = doc
        .create_page_sketch(&page.id, &graph, &Style::default())
        .unwrap();
    let placement = doc.sketch(id).unwrap().placement;
    doc.delete_page(&page.id).unwrap();
    assert_eq!(doc.page_sketch(&page.id), None);
    assert_eq!(doc.sketch(id).unwrap().placement, placement);
    assert!(doc.sketch_ownership_conflict(id));
    doc.undo_step(doc.own_steps().last().unwrap().step).unwrap();
    assert_eq!(doc.page_sketch(&page.id), Some(id));
    let anchors: Vec<_> = (0..doc.sketch(id).unwrap().resolved.all_anchors().count())
        .map(|n| AnchorId::new(9140, 100 + n as u64))
        .collect();
    doc.convert_sketch_to_path(id, &anchors).unwrap();
    assert_eq!(doc.page_sketch(&page.id), None);
    assert_eq!(doc.sketch_page(id), None);
    doc.undo_step(doc.own_steps().last().unwrap().step).unwrap();
    assert_eq!(doc.page_sketch(&page.id), Some(id));
    assert_eq!(doc.sketch(id).unwrap().placement, placement);
}

#[test]
fn object_undo_of_conversion_restores_its_page_claim() {
    use curvyo_document_core::AnchorId;
    let doc = Document::new(9150);
    let page = doc.pages()[0].id.clone();
    let graph = curvyo_sketch_core::create_rectangle(
        &graph(),
        "plate",
        Position::mm(0.0, 0.0),
        Position::mm(20.0, 10.0),
    )
    .unwrap();
    let id = doc
        .create_page_sketch(&page, &graph, &Style::default())
        .unwrap();
    let anchors: Vec<_> = (0..doc.sketch(id).unwrap().resolved.all_anchors().count())
        .map(|n| AnchorId::new(9150, 100 + n as u64))
        .collect();
    doc.convert_sketch_to_path(id, &anchors).unwrap();
    doc.undo_object_step(id).unwrap();
    assert!(doc.sketch(id).is_some());
    assert_eq!(doc.page_sketch(&page), Some(id));
}
