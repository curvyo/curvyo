//! Wipe the history of one object (`specs/0041-object-history` criteria 29 to 34, 38): it hides
//! and cuts, the data stays in the file and in the document list.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_precision_loss
)]

use curvyo_document_core::{
    CopySource, Document, HistoryError, NodeId, ObjectRow, ObjectRowClass, Point, RectBounds, Vec2,
    pack, unpack,
};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn labels(document: &Document, id: NodeId) -> Vec<String> {
    rows(document, id).into_iter().map(|r| r.label).collect()
}

fn rows(document: &Document, id: NodeId) -> Vec<ObjectRow> {
    document.object_history_page(id, 0, 100)
}

fn moved(document: &Document, id: NodeId, times: usize) {
    for _ in 0..times {
        document
            .translate_objects(&[id], Vec2::new(1.0, 0.0))
            .unwrap();
    }
}

#[test]
fn a_wipe_hides_the_earlier_steps_and_the_timeline_starts_at_the_floor() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    moved(&document, a, 3);
    assert_eq!(document.object_wipe_count(a), 4);
    let hidden = document.wipe_object_history(a).unwrap();
    assert_eq!(hidden, 4);
    assert_eq!(labels(&document, a), ["wipe_object_history"]);
    assert_eq!(rows(&document, a)[0].class, ObjectRowClass::Floor);
    assert_eq!(document.object_wipe_count(a), 0, "nothing left to hide");
    moved(&document, a, 2);
    assert_eq!(
        labels(&document, a),
        [
            "translate_objects",
            "translate_objects",
            "wipe_object_history"
        ]
    );
    assert_eq!(document.object_wipe_count(a), 2);
}

#[test]
fn nothing_to_hide_is_refused_and_writes_nothing() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    document.wipe_object_history(a).unwrap();
    let version = document.version();
    assert_eq!(document.wipe_object_history(a), Err(HistoryError::NoSteps));
    assert_eq!(document.version(), version);
}

#[test]
fn the_wipe_is_one_step_on_no_stack_and_the_document_list_keeps_everything() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    moved(&document, a, 2);
    let own = document.own_step_count();
    let before = document.history_len();
    document.wipe_object_history(a).unwrap();
    assert_eq!(
        document.own_step_count(),
        own,
        "not a step of the maker's stack"
    );
    assert_eq!(document.history_len(), before + 1);
    let global = document.history_page(0, 20);
    assert_eq!(global[0].label, "wipe_object_history");
    assert_eq!(global[0].objects.len(), 1, "it names the object");
    assert_eq!(global.len(), 4, "every earlier step is still listed");
}

#[test]
fn an_object_undo_cannot_go_before_the_floor() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    moved(&document, a, 2);
    document.wipe_object_history(a).unwrap();
    assert!(document.object_history_targets(a).undo.is_none());
    assert!(document.undo_object_step(a).unwrap().is_none());
    moved(&document, a, 1);
    let done = document.undo_object_step(a).unwrap().unwrap();
    assert_eq!(done.target.label, "translate_objects");
    assert!(document.undo_object_step(a).unwrap().is_none());
}

#[test]
fn the_wipe_is_saved() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    moved(&document, a, 2);
    document.wipe_object_history(a).unwrap();
    moved(&document, a, 1);
    let reopened = unpack(9, &pack(&document, "test").unwrap()).unwrap();
    assert_eq!(
        labels(&reopened, a),
        ["translate_objects", "wipe_object_history"]
    );
}

#[test]
fn a_clone_made_earlier_keeps_its_rows_and_a_clone_made_later_inherits_from_the_floor() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    moved(&document, a, 2);
    let early = document
        .duplicate_objects(
            &[CopySource {
                id: a,
                anchor_ids: vec![],
            }],
            Vec2::new(0.0, 50.0),
        )
        .unwrap()[0];
    document.wipe_object_history(a).unwrap();
    moved(&document, a, 1);
    let late = document
        .duplicate_objects(
            &[CopySource {
                id: a,
                anchor_ids: vec![],
            }],
            Vec2::new(0.0, 80.0),
        )
        .unwrap()[0];
    assert_eq!(
        labels(&document, early),
        [
            "duplicate_objects",
            "translate_objects",
            "translate_objects",
            "create_rect"
        ],
        "the wipe of the original does not change a clone made earlier"
    );
    assert_eq!(
        labels(&document, late),
        [
            "duplicate_objects",
            "translate_objects",
            "wipe_object_history"
        ],
        "a clone made later starts at the original's floor"
    );
}

#[test]
fn wiping_a_clone_leaves_the_original_alone() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    moved(&document, a, 1);
    let clone = document
        .duplicate_objects(
            &[CopySource {
                id: a,
                anchor_ids: vec![],
            }],
            Vec2::new(0.0, 50.0),
        )
        .unwrap()[0];
    let original_before = rows(&document, a);
    assert_eq!(document.object_wipe_count(clone), 3);
    document.wipe_object_history(clone).unwrap();
    assert_eq!(labels(&document, clone), ["wipe_object_history"]);
    assert_eq!(rows(&document, a), original_before);
}

#[test]
fn a_step_by_a_peer_concurrent_with_the_wipe_stays_visible() {
    let a = Document::new(1);
    let id = rect(&a, 0.0);
    let b = unpack(2, &pack(&a, "test").unwrap()).unwrap();
    b.translate_objects(&[id], Vec2::new(0.0, 9.0)).unwrap();
    a.wipe_object_history(id).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    a.import_updates(&to_a).unwrap();
    let list = labels(&a, id);
    assert!(list.contains(&"wipe_object_history".to_owned()));
    assert!(
        list.contains(&"translate_objects".to_owned()),
        "the concurrent step was not in the causal past of the wipe: {list:?}"
    );
    assert!(!list.contains(&"create_rect".to_owned()));
}

#[test]
fn a_deleted_object_cannot_be_wiped() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    document.delete_objects(&[a]).unwrap();
    assert_eq!(
        document.wipe_object_history(a),
        Err(HistoryError::NoSuchObject)
    );
}
