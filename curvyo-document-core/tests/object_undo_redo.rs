//! Object undo and redo (`specs/0041-object-history` criteria 6 to 15, 40, 41): the engine of
//! `0020` restricted to one object, each press a step of its own.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_precision_loss
)]

use curvyo_document_core::{
    AnchorId, Author, CopySource, Document, NewAnchor, NodeId, ObjectRowClass, ObjectStatus, Point,
    RectBounds, Status, StepKind, StepRef, Vec2, pack, unpack,
};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn square(first: u64, x: f64) -> (Vec<NewAnchor>, bool) {
    let corners = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    (
        corners
            .iter()
            .enumerate()
            .map(|(n, (dx, dy))| {
                NewAnchor::corner(AnchorId::new(1, first + n as u64), Point::new(x + dx, *dy))
            })
            .collect(),
        true,
    )
}

fn json(document: &Document) -> Vec<u8> {
    document.export_json().unwrap()
}

fn replica(document: &Document, peer: u64) -> Document {
    unpack(peer, &pack(document, "test").unwrap()).unwrap()
}

fn sync(a: &Document, b: &Document) {
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
}

/// Three rectangles A, B, C and the same three with A and C moved.
fn three_with_a_and_c_moved() -> (Document, [NodeId; 3], Vec<u8>) {
    let expected = Document::new(1);
    let ids: Vec<NodeId> = (0..3)
        .map(|n| rect(&expected, f64::from(n) * 20.0))
        .collect();
    expected
        .translate_objects(&[ids[0], ids[2]], Vec2::new(5.0, 7.0))
        .unwrap();
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..3)
        .map(|n| rect(&document, f64::from(n) * 20.0))
        .collect();
    document
        .translate_objects(&ids, Vec2::new(5.0, 7.0))
        .unwrap();
    (document, [ids[0], ids[1], ids[2]], json(&expected))
}

#[test]
fn a_change_step_is_taken_back_for_the_selected_object_only() {
    let (document, [a, b, c], expected) = three_with_a_and_c_moved();
    let done = document.undo_object_step(b).unwrap().unwrap();
    assert!(done.report.wrote);
    assert!(!done.whole_step);
    assert!(!done.object_gone);
    assert_eq!(done.target.label, "translate_objects");
    assert_eq!(done.target.author, Author::You);
    assert_eq!(done.report.skipped(), 0);
    assert_eq!(json(&document), expected, "A and C stay moved");
    assert!(document.object(a).is_some() && document.object(c).is_some());
}

#[test]
fn the_duplicate_of_several_objects_is_not_taken_back_by_an_object_undo() {
    let document = Document::new(1);
    let source = rect(&document, 0.0);
    let copies = document
        .duplicate_objects(
            &[
                CopySource {
                    id: source,
                    anchor_ids: vec![],
                },
                CopySource {
                    id: source,
                    anchor_ids: vec![],
                },
            ],
            Vec2::new(30.0, 0.0),
        )
        .unwrap();
    // The duplicate of one source named twice makes one copy; duplicate three different ones.
    assert_eq!(copies.len(), 1);
    let others = [rect(&document, 100.0), rect(&document, 120.0)];
    let copies = document
        .duplicate_objects(
            &[
                CopySource {
                    id: source,
                    anchor_ids: vec![],
                },
                CopySource {
                    id: others[0],
                    anchor_ids: vec![],
                },
                CopySource {
                    id: others[1],
                    anchor_ids: vec![],
                },
            ],
            Vec2::new(0.0, 50.0),
        )
        .unwrap();
    // Ctrl+U never takes back the Duplicate itself (only Ctrl+Z does): nothing to take back.
    assert!(document.undo_object_step(copies[1]).unwrap().is_none());
    assert!(copies.iter().all(|copy| document.object(*copy).is_some()));
}

#[test]
fn a_replace_step_is_taken_back_whole_and_names_the_others() {
    let document = Document::new(1);
    let a = document.create_path(&square(1, 0.0).0, true);
    let b = document.create_path(&square(11, 5.0).0, true);
    let before = json(&document);
    let result = document
        .replace_with_path(&[a, b], a, &[square(21, 0.0)], "boolean_union")
        .unwrap();
    let after = json(&document);
    let done = document.undo_object_step(result).unwrap().unwrap();
    assert!(done.whole_step);
    assert_eq!(done.report.objects.len(), 3, "the result and both operands");
    assert_eq!(json(&document), before);
    assert!(document.object(a).is_some() && document.object(b).is_some());
    // The result's timeline now shows the step as taken back.
    let redo = document.redo_object_step(result).unwrap().unwrap();
    assert!(redo.whole_step);
    assert_eq!(json(&document), after, "redo brings the union back");
}

#[test]
fn an_object_undo_is_a_step_that_names_the_step_it_reverted() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    document
        .translate_objects(&[a], Vec2::new(1.0, 0.0))
        .unwrap();
    let moved = document.own_steps().last().unwrap().step;
    let done = document.undo_object_step(a).unwrap().unwrap();
    let step = done.report.step.expect("a step was written");
    let own = document.own_steps();
    assert_eq!(own.last().unwrap().step, step, "it is on the maker's stack");
    assert_eq!(own.last().unwrap().label, "object_undo");

    let global = document.history_page(0, 10);
    assert_eq!(global[0].step, step);
    assert_eq!(global[0].label, "object_undo");
    let reverts = global[0].reverts.as_ref().expect("it names the step");
    assert_eq!(reverts.step, moved);
    assert_eq!(reverts.label, "translate_objects");
    assert!(!reverts.redo);
    assert_eq!(global[1].step, moved);
    assert_eq!(global[1].status, Status::Applied);
    assert_eq!(global[1].undone_by, Some(Author::You), "Undone by You");

    let rows = document.object_history_page(a, 0, 10);
    assert_eq!(rows[0].step, step);
    assert!(matches!(
        rows[0].class,
        ObjectRowClass::Reverts { redo: false, .. }
    ));
    assert_eq!(rows[1].status, ObjectStatus::UndoneBy(Author::You));
}

#[test]
fn a_user_undo_of_an_object_undo_is_the_object_redo() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    let b = rect(&document, 20.0);
    let start = json(&document);
    document
        .translate_objects(&[a], Vec2::new(5.0, 0.0))
        .unwrap();
    let after_1 = json(&document);
    document
        .translate_objects(&[b], Vec2::new(0.0, 5.0))
        .unwrap();
    let after_2 = json(&document);
    let object_undo = document
        .undo_object_step(a)
        .unwrap()
        .unwrap()
        .report
        .step
        .unwrap();
    assert_ne!(json(&document), after_2);

    document.undo_step(object_undo).unwrap();
    assert_eq!(json(&document), after_2, "A is moved again");
    let step_2 = document.own_steps()[3].step;
    document.undo_step(step_2).unwrap();
    assert_eq!(json(&document), after_1, "then B");
    let step_1 = document.own_steps()[2].step;
    document.undo_step(step_1).unwrap();
    let _ = start;
    // The rectangles' creations remain.
    assert_eq!(document.object_ids().len(), 2);
}

#[test]
fn an_object_redo_is_a_step_and_ctrl_z_takes_it_back() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    document
        .translate_objects(&[a], Vec2::new(5.0, 0.0))
        .unwrap();
    let moved = json(&document);
    document.undo_object_step(a).unwrap().unwrap();
    let undone = json(&document);
    assert_ne!(moved, undone);
    let redo = document.redo_object_step(a).unwrap().unwrap();
    assert_eq!(json(&document), moved);
    let step = redo.report.step.unwrap();
    assert_eq!(document.own_steps().last().unwrap().label, "object_redo");
    document.undo_step(step).unwrap();
    assert_eq!(json(&document), undone, "the state after Ctrl+U");
    // The status words: the step is taken back by the object undo again.
    let rows = document.object_history_page(a, 0, 10);
    assert_eq!(
        rows[0].status,
        ObjectStatus::Undone,
        "the object redo is taken back"
    );
    assert_eq!(
        rows[2].status,
        ObjectStatus::UndoneBy(Author::You),
        "so the move stays undone by the object undo"
    );
    let targets = document.object_history_targets(a);
    assert_eq!(
        targets.undo.map(|t| t.label),
        Some("create_rect".to_owned())
    );
    assert_eq!(
        targets.redo.map(|t| t.label),
        Some("translate_objects".to_owned())
    );
}

#[test]
fn a_new_step_on_the_object_leaves_nothing_to_redo() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    document
        .translate_objects(&[a], Vec2::new(5.0, 0.0))
        .unwrap();
    document.undo_object_step(a).unwrap().unwrap();
    assert!(document.object_history_targets(a).redo.is_some());
    document
        .translate_objects(&[a], Vec2::new(0.0, 5.0))
        .unwrap();
    assert!(document.object_history_targets(a).redo.is_none());
    assert!(document.redo_object_step(a).unwrap().is_none());
}

#[test]
fn nothing_to_undo_writes_nothing() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    document.undo_object_step(a).unwrap().unwrap();
    assert!(document.object(a).is_none(), "the creation was taken back");
    let version = document.version();
    assert!(document.undo_object_step(a).unwrap().is_none());
    assert_eq!(document.version(), version);
}

#[test]
fn taking_back_the_creation_removes_the_object_and_ctrl_z_brings_it_back_with_its_id() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    let b = rect(&document, 20.0);
    document
        .translate_objects(&[a], Vec2::new(5.0, 0.0))
        .unwrap();
    let with_a = json(&document);
    document.undo_object_step(a).unwrap().unwrap();
    let done = document.undo_object_step(a).unwrap().unwrap();
    assert!(done.object_gone);
    assert_eq!(done.target.label, "create_rect");
    assert!(document.object(a).is_none());
    assert!(document.object(b).is_some());
    // Ctrl+Z: the removal is the newest step.
    let removal = document.own_steps().last().unwrap().step;
    document.undo_step(removal).unwrap();
    assert!(document.object(a).is_some(), "the same id");
    // Back to the state before the object undo of the move, with the move taken back.
    let move_undo = document.own_steps()[3].step;
    document.undo_step(move_undo).unwrap();
    assert_eq!(json(&document), with_a);
    // And it still has its history.
    assert_eq!(document.object_history_len(a), 4);
}

#[test]
fn a_delete_of_three_is_taken_back_for_one_object() {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..3)
        .map(|n| rect(&document, f64::from(n) * 20.0))
        .collect();
    document.delete_objects(&ids).unwrap();
    let done = document.undo_object_step(ids[1]).unwrap().unwrap();
    assert!(!done.whole_step);
    assert!(document.object(ids[1]).is_some(), "with its id");
    assert!(document.object(ids[0]).is_none() && document.object(ids[2]).is_none());
    assert_eq!(document.object_ids(), vec![ids[1]], "z-order intact");
}

#[test]
fn another_authors_latest_step_is_taken_back_and_named() {
    let a = Document::new(1);
    let id = rect(&a, 0.0);
    let b = replica(&a, 2);
    b.translate_objects(&[id], Vec2::new(0.0, 9.0)).unwrap();
    let before_b = {
        let c = replica(&a, 3);
        json(&c)
    };
    sync(&a, &b);
    let done = a.undo_object_step(id).unwrap().unwrap();
    assert_eq!(done.target.author, Author::Earlier);
    assert_eq!(done.target.label, "translate_objects");
    assert_eq!(json(&a), before_b);
    // The other peer sees the object undo as a step by someone else and Ctrl+Z of its own step
    // still works.
    sync(&a, &b);
    assert_eq!(json(&a), json(&b));
}

#[test]
fn the_step_ids_of_the_object_list_are_the_ones_of_the_global_list() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    document
        .translate_objects(&[a], Vec2::new(1.0, 0.0))
        .unwrap();
    document.undo_object_step(a).unwrap().unwrap();
    let global: Vec<(StepRef, String)> = document
        .history_page(0, 10)
        .into_iter()
        .map(|row| (row.step, row.shown_id))
        .collect();
    let object: Vec<(StepRef, String)> = document
        .object_history_page(a, 0, 10)
        .into_iter()
        .map(|row| (row.step, row.shown_id))
        .collect();
    assert_eq!(global, object);
    let _ = StepKind::Create;
}
