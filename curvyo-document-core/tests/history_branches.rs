//! Branches of the history lists (`specs/0042-history-branches` criteria 1 to 7, 28 and 29): a
//! new step after a take-back leaves the taken-back steps as a branch, in the Document scope and
//! in the Object scope, the same live, after Save and Open and on every replica.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{
    Document, HistoryBranch, Length, NodeId, ObjectStatus, Point, RectBounds, Status, StepRef,
    StyleEdit, pack, unpack,
};

fn rect(d: &Document) -> NodeId {
    d.create_rect(RectBounds {
        origin: Point::new(10.0, 10.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn set_width(d: &Document, id: NodeId, mm: f64) -> StepRef {
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(mm)))
        .unwrap();
    d.own_steps().last().unwrap().step
}

fn reopen(d: &Document, peer: u64) -> Document {
    unpack(peer, &pack(d, "t").unwrap()).unwrap()
}

fn merge(a: &Document, b: &Document) {
    b.import_updates(&a.export_updates(&b.version()).unwrap())
        .unwrap();
    a.import_updates(&b.export_updates(&a.version()).unwrap())
        .unwrap();
}

/// The steps of the Document scope's list, newest first, with their statuses.
fn document_rows(d: &Document) -> Vec<(StepRef, Status)> {
    d.history_page(0, d.history_len())
        .into_iter()
        .map(|row| (row.step, row.status))
        .collect()
}

fn object_rows(d: &Document, id: NodeId) -> Vec<(StepRef, ObjectStatus)> {
    d.object_history_page(id, 0, d.object_history_len(id))
        .into_iter()
        .map(|row| (row.step, row.status))
        .collect()
}

/// r, s1, s2, s3 on one rectangle, Ctrl+Z twice, s4: the fork of criterion 1.
struct Fork {
    d: Document,
    r: NodeId,
    create: StepRef,
    s: [StepRef; 4],
}

fn fork() -> Fork {
    let d = Document::new(1);
    let r = rect(&d);
    let create = d.own_steps().last().unwrap().step;
    let s1 = set_width(&d, r, 1.0);
    let s2 = set_width(&d, r, 2.0);
    let s3 = set_width(&d, r, 3.0);
    d.undo_step(s3).unwrap();
    d.undo_step(s2).unwrap();
    let s4 = set_width(&d, r, 4.0);
    Fork {
        d,
        r,
        create,
        s: [s1, s2, s3, s4],
    }
}

#[test]
fn c1_c28_a_new_step_after_two_undos_leaves_them_as_a_branch() {
    let Fork { d, r, create, s } = fork();
    let [s1, s2, s3, s4] = s;
    let reference = Document::new(1);
    let rr = rect(&reference);
    set_width(&reference, rr, 1.0);
    set_width(&reference, rr, 4.0);
    assert_eq!(
        d.object(r).unwrap(),
        reference.object(rr).unwrap(),
        "s1's state plus s4"
    );
    assert_eq!(d.stacks_in_log(1).top_redo(), None, "Nothing to redo");
    assert_eq!(
        document_rows(&d),
        [
            (s4, Status::Applied),
            (s3, Status::OnBranch),
            (s2, Status::OnBranch),
            (s1, Status::Applied),
            (create, Status::Applied),
        ]
    );
    let branch = HistoryBranch {
        id: s3,
        fork: Some(s1),
        rows: vec![1, 2],
        fork_row: Some(3),
    };
    assert_eq!(d.history_branches(), std::slice::from_ref(&branch));
    assert_eq!(
        object_rows(&d, r),
        [
            (s4, ObjectStatus::Applied),
            (s3, ObjectStatus::OnBranch),
            (s2, ObjectStatus::OnBranch),
            (s1, ObjectStatus::Applied),
            (create, ObjectStatus::Applied),
        ]
    );
    assert_eq!(d.object_history_branches(r), [branch]);
}

#[test]
fn c1_the_same_fork_with_object_undo() {
    let d = Document::new(1);
    let r = rect(&d);
    let s1 = set_width(&d, r, 1.0);
    let s2 = set_width(&d, r, 2.0);
    let s3 = set_width(&d, r, 3.0);
    d.undo_object_step(r).unwrap();
    d.undo_object_step(r).unwrap();
    let s4 = set_width(&d, r, 4.0);
    assert_eq!(d.object_history_targets(r).redo, None, "nothing to redo");
    let branches = d.object_history_branches(r);
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].id, s3);
    assert_eq!(branches[0].fork, Some(s1));
    let rows = object_rows(&d, r);
    let status = |step| rows.iter().find(|(s, _)| *s == step).unwrap().1;
    assert_eq!(status(s2), ObjectStatus::OnBranch);
    assert_eq!(status(s3), ObjectStatus::OnBranch);
    assert_eq!(status(s4), ObjectStatus::Applied);
}

#[test]
fn c2_undoing_the_new_step_makes_it_the_one_redo_brings_back() {
    let Fork { d, s, .. } = fork();
    let [s1, s2, s3, s4] = s;
    d.undo_step(s4).unwrap();
    let machine = d.stacks_in_log(1);
    assert_eq!(machine.top_redo(), Some(s4));
    let rows = document_rows(&d);
    assert_eq!(rows[0], (s4, Status::Undone));
    assert_eq!(rows[1], (s3, Status::OnBranch));
    assert_eq!(rows[2], (s2, Status::OnBranch));
    assert_eq!(rows[3], (s1, Status::Applied));
    d.redo_step(s4).unwrap();
    assert_eq!(document_rows(&d)[0], (s4, Status::Applied));
}

#[test]
fn c2_c3_leaving_the_new_step_too_gives_two_branches_from_one_fork() {
    let Fork { d, r, s, .. } = fork();
    let [s1, _, s3, s4] = s;
    d.undo_step(s4).unwrap();
    let s5 = set_width(&d, r, 5.0);
    let branches = d.history_branches();
    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0].id, s4, "the newest branch first");
    assert_eq!(branches[0].fork, Some(s1));
    assert_eq!(branches[1].id, s3);
    assert_eq!(branches[1].fork, Some(s1));
    assert_eq!(document_rows(&d)[0], (s5, Status::Applied));
}

#[test]
fn c3_a_fork_inside_a_branch_points_into_that_branch() {
    let d = Document::new(1);
    let r = rect(&d);
    let s1 = set_width(&d, r, 1.0);
    let s2 = set_width(&d, r, 2.0);
    let s3 = set_width(&d, r, 3.0);
    d.undo_step(s3).unwrap();
    let s4 = set_width(&d, r, 4.0);
    d.undo_step(s4).unwrap();
    d.undo_step(s2).unwrap();
    set_width(&d, r, 5.0);
    let branches = d.history_branches();
    assert_eq!(branches.len(), 2);
    assert_eq!((branches[0].id, branches[0].fork), (s4, Some(s1)));
    assert_eq!((branches[1].id, branches[1].fork), (s3, Some(s2)));
}

#[test]
fn c5_a_peers_step_never_starts_a_branch() {
    let a = Document::new(1);
    let r = rect(&a);
    let s1 = set_width(&a, r, 1.0);
    let b = reopen(&a, 2);
    a.undo_step(s1).unwrap();
    // Anna's step arrives while mine waits to be redone.
    b.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(7.0)))
        .unwrap();
    merge(&a, &b);
    assert_eq!(a.stacks_in_log(1).top_redo(), Some(s1));
    assert_eq!(a.history_branches(), []);
    assert_eq!(b.history_branches(), []);
    let undone = |d: &Document| {
        document_rows(d)
            .into_iter()
            .find(|(step, _)| *step == s1)
            .unwrap()
            .1
    };
    assert_eq!(undone(&a), Status::Undone);
    assert_eq!(undone(&b), Status::Undone);
    // My own next step makes my branch, on both replicas.
    set_width(&a, r, 8.0);
    merge(&a, &b);
    assert_eq!(a.history_branches(), b.history_branches());
    assert_eq!(a.history_branches()[0].id, s1);
}

#[test]
fn c6_c28_branches_are_the_same_after_save_and_open() {
    let Fork { d, r, .. } = fork();
    let reopened = reopen(&d, 9);
    assert_eq!(reopened.history_branches(), d.history_branches());
    assert_eq!(
        reopened.object_history_branches(r),
        d.object_history_branches(r)
    );
    assert_eq!(document_rows(&reopened), document_rows(&d));
}

#[test]
fn c29_concurrent_steps_have_one_order_on_every_replica() {
    let a = Document::new(1);
    let r = rect(&a);
    let b = reopen(&a, 2);
    set_width(&a, r, 1.0);
    b.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(2.0)))
        .unwrap();
    merge(&a, &b);
    let ids = |d: &Document| -> Vec<StepRef> {
        d.history_page(0, d.history_len())
            .into_iter()
            .map(|row| row.step)
            .collect()
    };
    assert_eq!(ids(&a), ids(&b));
}

#[test]
fn c7_a_wipe_takes_the_branches_out_of_the_list() {
    let Fork { mut d, r, .. } = fork();
    d.wipe_history().unwrap();
    assert_eq!(d.history_branches(), []);
    assert_eq!(d.object_history_branches(r), []);
}
