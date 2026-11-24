//! Copy import of pages and elements from another document (`specs/0137-document-library`
//! criteria 4 to 9, ADR 0084 §6 and §7): plain copies, one step, the Came from record, linked
//! clones whose originals stay behind become paths, the sketch-page refusal.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use curvyo_document_core::{
    AnchorId, CameFrom, Document, FragmentTarget, IMPORT_ELEMENTS_COMMIT_LABEL,
    IMPORT_PAGE_COMMIT_LABEL, ImportError, NewAnchor, NodeId, ObjectSnapshot, PageKind, Point,
    UnixMillis, Vec2,
};

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn aid(peer: u64) -> AnchorId {
    AnchorId::new(peer, NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
}

fn square(d: &Document, x: f64, y: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(aid(1), Point::new(x, y)),
            NewAnchor::corner(aid(1), Point::new(x + 10.0, y)),
            NewAnchor::corner(aid(1), Point::new(x + 10.0, y + 10.0)),
            NewAnchor::corner(aid(1), Point::new(x, y + 10.0)),
        ],
        true,
    )
}

fn record() -> CameFrom {
    CameFrom {
        document: "Lid".into(),
        page: "Page 1".into(),
        page_id: "first".into(),
        item: None,
        at: UnixMillis::new(1_764_000_000_000),
    }
}

fn fresh(n: usize) -> Vec<AnchorId> {
    (0..n).map(|_| aid(2)).collect()
}

fn first_point(d: &Document, id: NodeId) -> Point {
    match d.object(id) {
        Some(ObjectSnapshot::Path(path)) => path.all_anchors().next().unwrap().point,
        other => panic!("not a path: {other:?}"),
    }
}

fn import_elements(
    target: &Document,
    source: &Document,
    roots: &[NodeId],
    offset: Vec2,
) -> Vec<NodeId> {
    let fragment = source.extract_fragment(roots, None).unwrap();
    target
        .insert_fragment(
            &fragment,
            FragmentTarget::Elements {
                page: "first",
                offset,
            },
            &record(),
            &fresh(fragment.anchor_count()),
        )
        .unwrap()
        .roots
}

/// Criteria 5 and 6: the picked objects land moved, as plain copies with Came from on each root
/// (its name as the item) and not on the children, in one undo step.
#[test]
fn elements_land_as_copies_with_came_from_in_one_step() {
    let source = Document::new(1);
    let a = square(&source, 0.0, 0.0);
    let b = square(&source, 20.0, 0.0);
    let c = square(&source, 40.0, 0.0);
    let group = source.group_objects(&[b, c]).unwrap();
    source.rename_node(group, "Handle").unwrap();
    let source_json = source.export_json().unwrap();

    let target = Document::new(2);
    let roots = import_elements(&target, &source, &[a, group], Vec2::new(5.0, 7.0));
    assert_eq!(roots.len(), 2);
    assert_eq!(first_point(&target, roots[0]), Point::new(5.0, 7.0));
    let leaves = target.leaves_of(roots[1]);
    assert_eq!(leaves.len(), 2);
    assert_eq!(first_point(&target, leaves[0]), Point::new(25.0, 7.0));
    assert_eq!(target.came_from(roots[0]), Some(record()));
    assert_eq!(
        target.came_from(roots[1]),
        Some(record().with_item(Some("Handle".into())))
    );
    assert_eq!(target.came_from(leaves[0]), None);
    assert_eq!(
        source.export_json().unwrap(),
        source_json,
        "the source is unchanged"
    );

    let step = target.own_steps().last().unwrap().clone();
    assert_eq!(step.label, IMPORT_ELEMENTS_COMMIT_LABEL);
    target.undo_step(step.step).unwrap();
    assert_eq!(target.object_ids(), Vec::<NodeId>::new());
}

/// Criterion 6: a second import makes a second copy.
#[test]
fn a_second_import_makes_a_second_copy() {
    let source = Document::new(1);
    let a = square(&source, 0.0, 0.0);
    let target = Document::new(2);
    let first = import_elements(&target, &source, &[a], Vec2::ZERO);
    let second = import_elements(&target, &source, &[a], Vec2::ZERO);
    assert_ne!(first, second);
    assert_eq!(target.object_ids().len(), 2);
}

/// Criterion 5: a linked clone whose original was not picked comes in as an ordinary path; with
/// its original it stays a clone of the original's copy.
#[test]
fn a_clone_without_its_original_becomes_a_path() {
    let source = Document::new(1);
    let original = square(&source, 0.0, 0.0);
    let clone = source.linked_clone(&[original], &[]).unwrap()[0];

    let alone = source.extract_fragment(&[clone], None).unwrap();
    assert_eq!(alone.clones_became_paths(), 1);
    let target = Document::new(2);
    let report = target
        .insert_fragment(
            &alone,
            FragmentTarget::Elements {
                page: "first",
                offset: Vec2::ZERO,
            },
            &record(),
            &fresh(alone.anchor_count()),
        )
        .unwrap();
    assert_eq!(report.clones_became_paths, 1);
    assert!(matches!(
        target.object(report.roots[0]),
        Some(ObjectSnapshot::Path(_))
    ));
    assert!(target.came_from(report.roots[0]).is_some());

    let both = Document::new(3);
    let roots = import_elements(&both, &source, &[original, clone], Vec2::ZERO);
    match both.object(roots[1]) {
        Some(ObjectSnapshot::Clone(copy)) => assert_eq!(copy.chain.origin, roots[0]),
        other => panic!("not a clone: {other:?}"),
    }
}

/// Criterion 5 (E4): elements cannot be dropped onto a sketch page; nothing is written.
#[test]
fn elements_are_refused_on_a_sketch_page() {
    let source = Document::new(1);
    let a = square(&source, 0.0, 0.0);
    let target = Document::new(2);
    let sketch = target.add_page_of(&PageKind::Sketch).unwrap();
    let steps = target.own_steps().len();
    let fragment = source.extract_fragment(&[a], None).unwrap();
    let refused = target.insert_fragment(
        &fragment,
        FragmentTarget::Elements {
            page: &sketch.id,
            offset: Vec2::ZERO,
        },
        &record(),
        &fresh(fragment.anchor_count()),
    );
    assert_eq!(refused.err(), Some(ImportError::SketchPage));
    assert_eq!(target.own_steps().len(), steps);
    let wrong = target.insert_fragment(
        &fragment,
        FragmentTarget::Elements {
            page: "first",
            offset: Vec2::ZERO,
        },
        &record(),
        &fresh(fragment.anchor_count() + 1),
    );
    assert_eq!(wrong.err(), Some(ImportError::AnchorIds));
}

/// Criterion 4: a page comes in as a new page after the last page, with its name, size, objects
/// moved by the change of the frame origin and its layer by name, in one step; it carries Came
/// from.
#[test]
fn a_page_comes_in_after_the_last_page_in_one_step() {
    let source = Document::new(1);
    source.set_draw_into_initial_layer(true);
    let a = square(&source, 10.0, 10.0);
    source.rename_page("first", "Front").unwrap();

    let target = Document::new(2);
    target.add_page();
    let fragment = source.extract_fragment(&[a], Some("first")).unwrap();
    let report = target
        .insert_fragment(
            &fragment,
            FragmentTarget::Page,
            &record(),
            &fresh(fragment.anchor_count()),
        )
        .unwrap();
    let page = report.page.unwrap();
    let pages = target.pages();
    assert_eq!(pages.len(), 3);
    assert_eq!(pages[2], page);
    assert_eq!(page.name, "Front");
    assert_eq!(page.size, source.pages()[0].size);
    assert_eq!(page.kind, PageKind::Drawing);
    assert_eq!(target.page_came_from(&page.id), Some(record()));
    let copy = report.roots[0];
    assert_eq!(
        first_point(&target, copy),
        Point::new(page.origin.x + 10.0, page.origin.y + 10.0)
    );
    let layer = target.parent_node(copy).unwrap();
    assert!(target.is_layer(layer));
    assert_eq!(
        target.node_registers(layer).unwrap().name.as_deref(),
        Some("Initial Layer")
    );
    assert_eq!(target.came_from(copy), Some(record()));

    let step = target.own_steps().last().unwrap().clone();
    assert_eq!(step.label, IMPORT_PAGE_COMMIT_LABEL);
    target.undo_step(step.step).unwrap();
    assert_eq!(target.pages().len(), 2);
    assert_eq!(target.object_ids(), Vec::<NodeId>::new());
}

/// Criterion 4: a sketch page comes in as a sketch page that owns the copy of its sketch.
#[test]
fn a_sketch_page_comes_in_owning_its_sketch_copy() {
    let source = Document::new(1);
    let page = source.add_page_of(&PageKind::Sketch).unwrap();
    let sketch = source
        .create_page_sketch(
            &page.id,
            &curvyo_sketch_core::create_polyline(
                &curvyo_sketch_core::Sketch::new("origin".into()),
                "edge",
                &[
                    curvyo_sketch_core::Position::mm(-20.0, -15.0),
                    curvyo_sketch_core::Position::mm(20.0, -15.0),
                    curvyo_sketch_core::Position::mm(20.0, 15.0),
                ],
                true,
            )
            .unwrap(),
            &curvyo_document_core::Style::default(),
        )
        .unwrap();
    let target = Document::new(2);
    let fragment = source.extract_fragment(&[sketch], Some(&page.id)).unwrap();
    let report = target
        .insert_fragment(
            &fragment,
            FragmentTarget::Page,
            &record(),
            &fresh(fragment.anchor_count()),
        )
        .unwrap();
    let imported = report.page.unwrap();
    assert_eq!(imported.kind, PageKind::Sketch);
    assert_eq!(target.page_sketch(&imported.id), Some(report.roots[0]));
}

/// ADR 0084 §7: a Boolean result keeps Came from only when every operand carries an equal one;
/// Object to path keeps it.
#[test]
fn a_result_of_several_sources_keeps_an_equal_record_only() {
    let source = Document::new(1);
    let a = square(&source, 0.0, 0.0);
    let b = square(&source, 5.0, 5.0);
    let target = Document::new(2);
    let roots = import_elements(&target, &source, &[a, b], Vec2::ZERO);
    let outline = vec![(
        vec![
            NewAnchor::corner(aid(3), Point::new(0.0, 0.0)),
            NewAnchor::corner(aid(3), Point::new(15.0, 0.0)),
            NewAnchor::corner(aid(3), Point::new(15.0, 15.0)),
        ],
        true,
    )];
    let joined = target
        .replace_with_path(&roots, roots[0], &outline, "boolean_union")
        .unwrap();
    assert_eq!(target.came_from(joined), Some(record()));

    let other = square(&target, 50.0, 50.0);
    let outline = vec![(
        vec![
            NewAnchor::corner(aid(3), Point::new(0.0, 0.0)),
            NewAnchor::corner(aid(3), Point::new(60.0, 0.0)),
            NewAnchor::corner(aid(3), Point::new(60.0, 60.0)),
        ],
        true,
    )];
    let mixed = target
        .replace_with_path(&[joined, other], joined, &outline, "boolean_union")
        .unwrap();
    assert_eq!(target.came_from(mixed), None);
}

/// ADR 0084 §7: Duplicate carries the record of the copied root.
#[test]
fn duplicate_carries_came_from() {
    let source = Document::new(1);
    let a = square(&source, 0.0, 0.0);
    let target = Document::new(2);
    let roots = import_elements(&target, &source, &[a], Vec2::ZERO);
    let copies = target
        .duplicate_objects(
            &[curvyo_document_core::CopySource {
                id: roots[0],
                anchor_ids: fresh(4),
            }],
            Vec2::new(1.0, 1.0),
        )
        .unwrap();
    assert_eq!(target.came_from(copies[0]), Some(record()));
}
