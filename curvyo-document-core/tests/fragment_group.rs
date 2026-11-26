//! The copy import wrapped into one group (`specs/0154-library-into-slots` criteria 2, 3 and 14,
//! ADR 0088 decision 3): the roots become children of one new group named after the source page,
//! the group carries Came from with the page as the item, each copy keeps its own, one step, and
//! a wrap past the group depth limit writes nothing.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    AnchorId, CameFrom, Document, FragmentTarget, ImportError, NewAnchor, NodeId, Point,
    UnixMillis, Vec2,
};

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(50_000);

fn aid(peer: u64) -> AnchorId {
    AnchorId::new(peer, NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
}

fn record() -> CameFrom {
    CameFrom {
        document: "Box job".into(),
        page: "Coaster".into(),
        page_id: "first".into(),
        item: None,
        at: UnixMillis::new(0),
        online: None,
    }
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

#[test]
fn the_roots_land_in_one_group_named_after_the_page_with_came_from_on_it() {
    let source = Document::new(1);
    let a = square(&source, 0.0, 0.0);
    let b = square(&source, 20.0, 0.0);
    let fragment = source.extract_fragment(&[a, b], None).unwrap();
    let target = Document::new(2);
    let before = target.object_ids().len();
    let anchors: Vec<_> = (0..fragment.anchor_count()).map(|_| aid(2)).collect();
    let report = target
        .insert_fragment(
            &fragment,
            FragmentTarget::Group {
                page: "first",
                offset: Vec2::new(5.0, 0.0),
                name: "Coaster",
            },
            &record(),
            &anchors,
        )
        .unwrap();
    assert_eq!(report.roots.len(), 1, "one group is the unit");
    let group = report.roots[0];
    assert!(target.is_group(group));
    assert_eq!(target.leaves_of(group).len(), 2);
    assert_eq!(target.object_ids().len(), before + 2);
    let on_group = target.came_from(group).unwrap();
    assert_eq!(on_group.item.as_deref(), Some("Coaster"));
    for leaf in target.leaves_of(group) {
        assert!(target.came_from(leaf).is_some(), "each copy keeps its own");
    }
    assert_eq!(target.own_steps().last().unwrap().label, "import_elements");
}

#[test]
fn a_wrap_past_the_depth_limit_writes_nothing() {
    let source = Document::new(1);
    let mut top = square(&source, 0.0, 0.0);
    for _ in 0..curvyo_document_core::MAX_GROUP_DEPTH {
        let other = square(&source, 0.0, 20.0);
        top = source.group_objects(&[top, other]).unwrap();
    }
    let fragment = source.extract_fragment(&[top], None).unwrap();
    let target = Document::new(2);
    let before = target.object_ids().len();
    let anchors: Vec<_> = (0..fragment.anchor_count()).map(|_| aid(2)).collect();
    let error = target
        .insert_fragment(
            &fragment,
            FragmentTarget::Group {
                page: "first",
                offset: Vec2::ZERO,
                name: "Deep",
            },
            &record(),
            &anchors,
        )
        .unwrap_err();
    assert_eq!(error, ImportError::TooDeep);
    assert_eq!(target.object_ids().len(), before);
}
