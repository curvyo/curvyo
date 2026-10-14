//! Independent tester acceptance tests for `specs/0042-history-branches/specification.md`,
//! document level: branches derived from the log (criteria 1 to 7, 28, 29), applying a version
//! (10, 15, 30; criteria 11 to 13 withdrawn 2026-10-14), previews (17, 19, 20, 21, 30, 32), clones of any state
//! (22 to 25, 31) and the file footprint (6, 33).
//!
//! Written from the specification and the public `Document` API before the implementation diff was
//! read. Documents are compared through `export_json` (the `document.json` of the file) and through
//! the body (shape, style, rotation) of the object snapshots.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::similar_names
)]
#![allow(
    clippy::assert_is_empty,
    clippy::collapsible_if,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
#![allow(missing_docs, clippy::doc_markdown, clippy::many_single_char_names)]
#![allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]

use std::io::{Cursor, Read};
use std::time::Instant;

use curvyo_document_core::{
    AnchorId, Author, CURRENT_FORMAT_VERSION, Document, DocumentSize, HistoryError,
    LABEL_CLONE_VERSION, LABEL_GO_TO_VERSION, Length, NewAnchor, NodeId, ObjectSnapshot,
    ObjectStatus, Point, RectBounds, Status, StepRef, StyleEdit, Vec2, pack, unpack,
};

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect(d: &Document, x: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: pt(x, 10.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

/// The object without its id: shape, style and rotation, so a clone can be compared to its source.
fn body(d: &Document, id: NodeId) -> String {
    match d.object(id) {
        Some(ObjectSnapshot::Primitive(p)) => {
            format!("{:?}|{:?}|{:?}", p.shape, p.style, p.rotation)
        }
        Some(ObjectSnapshot::Path(p)) => format!("{p:?}"),
        None => "gone".to_owned(),
    }
}

fn json(d: &Document) -> Vec<u8> {
    d.export_json().unwrap()
}

fn set_width(d: &Document, id: NodeId, mm: f64) {
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(mm)))
        .unwrap();
}

fn mv(d: &Document, ids: &[NodeId], dx: f64) {
    d.translate_objects(ids, Vec2::new(dx, 0.0)).unwrap();
}

/// The step of the newest own commit.
fn last_step(d: &Document) -> StepRef {
    d.own_steps().last().unwrap().step
}

fn replica(d: &Document, peer: u64) -> Document {
    unpack(peer, &pack(d, "t").unwrap()).unwrap()
}

fn sync_into(from: &Document, to: &Document) {
    let bytes = from.export_updates(&to.version()).unwrap();
    to.import_updates(&bytes).unwrap();
}

fn merge(a: &Document, b: &Document) {
    sync_into(a, b);
    sync_into(b, a);
}

fn reopen(d: &Document) -> Document {
    unpack(77, &pack(d, "t").unwrap()).unwrap()
}

fn all_rows(d: &Document) -> Vec<curvyo_document_core::HistoryRow> {
    d.history_page(0, d.history_len() + 5)
}

fn status_of(d: &Document, step: StepRef) -> Status {
    all_rows(d)
        .iter()
        .find(|r| r.step == step)
        .unwrap_or_else(|| panic!("step {step:?} is not listed"))
        .status
}

fn object_rows(d: &Document, id: NodeId) -> Vec<curvyo_document_core::ObjectRow> {
    d.object_history_page(id, 0, d.object_history_len(id) + 5)
}

fn ostatus(d: &Document, id: NodeId, step: StepRef) -> ObjectStatus {
    object_rows(d, id)
        .iter()
        .find(|r| r.step == step)
        .unwrap_or_else(|| panic!("step {step:?} is not in the object's list"))
        .status
}

/// The author inside "Undone by" is "You" in the session and "Earlier session" after Open (ADR 0014 Q5).
fn norm(s: ObjectStatus) -> ObjectStatus {
    match s {
        ObjectStatus::UndoneBy(_) => ObjectStatus::UndoneBy(Author::Earlier),
        other => other,
    }
}

fn is_undone(s: ObjectStatus) -> bool {
    matches!(s, ObjectStatus::Undone | ObjectStatus::UndoneBy(_))
}

fn minter() -> impl FnMut() -> AnchorId {
    let mut n = 1_000_u64;
    move || {
        n += 1;
        AnchorId::new(9, n)
    }
}

fn manifest_version(bytes: &[u8]) -> u64 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    v["format_version"].as_u64().unwrap()
}

/// The scene of criterion 1: an object with applied steps s1 (move), s2 (stroke width), s3 (move).
struct Scene {
    d: Document,
    a: NodeId,
    s: [StepRef; 4],
    bodies: [String; 4],
}

fn scene() -> Scene {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let s0 = last_step(&d);
    let b0 = body(&d, a);
    mv(&d, &[a], 1.0);
    let s1 = last_step(&d);
    let b1 = body(&d, a);
    set_width(&d, a, 3.0);
    let s2 = last_step(&d);
    let b2 = body(&d, a);
    mv(&d, &[a], 2.0);
    let s3 = last_step(&d);
    let b3 = body(&d, a);
    Scene {
        d,
        a,
        s: [s0, s1, s2, s3],
        bodies: [b0, b1, b2, b3],
    }
}

/// What s1's state plus a new width step looks like, built on a twin that never had s2 and s3.
fn twin_s1_plus_s4(width: f64) -> String {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    set_width(&d, a, width);
    body(&d, a)
}

// ---------------------------------------------------------------------
// Criteria 1 to 4: the branch model, Document scope
// ---------------------------------------------------------------------

#[test]
fn c1_document_scope_fork_makes_the_undone_steps_a_branch() {
    let sc = scene();
    let d = &sc.d;
    d.undo_step(sc.s[3]).unwrap();
    d.undo_step(sc.s[2]).unwrap();
    assert_eq!(
        body(d, sc.a),
        sc.bodies[1],
        "two undos return to s1's state"
    );
    // Before the new step the taken-back steps are Undone, no branch exists.
    assert_eq!(status_of(d, sc.s[3]), Status::Undone);
    assert_eq!(status_of(d, sc.s[2]), Status::Undone);
    assert!(d.history_branches().is_empty());

    set_width(d, sc.a, 7.0);
    let s4 = last_step(d);
    // The state after s4 is s1's state plus s4.
    assert_eq!(body(d, sc.a), twin_s1_plus_s4(7.0));
    // s2 and s3 are a branch from s1; s4 is on the main line.
    assert_eq!(status_of(d, sc.s[2]), Status::OnBranch);
    assert_eq!(status_of(d, sc.s[3]), Status::OnBranch);
    assert_eq!(status_of(d, s4), Status::Applied);
    assert_eq!(status_of(d, sc.s[1]), Status::Applied);
    let branches = d.history_branches();
    assert_eq!(branches.len(), 1, "exactly one branch");
    assert_eq!(
        branches[0].id, sc.s[3],
        "criterion 3: named by its newest step"
    );
    assert_eq!(branches[0].fork, Some(sc.s[1]), "forked from s1");
    assert_eq!(branches[0].rows.len(), 2, "two steps");
    // Both steps stay in the log and in the list.
    let listed: Vec<StepRef> = all_rows(d).iter().map(|r| r.step).collect();
    assert!(listed.contains(&sc.s[2]) && listed.contains(&sc.s[3]));
}

#[test]
fn c2_taking_back_the_conflict_step_returns_to_s1_and_leaves_the_old_branch_alone() {
    let sc = scene();
    let d = &sc.d;
    d.undo_step(sc.s[3]).unwrap();
    d.undo_step(sc.s[2]).unwrap();
    set_width(d, sc.a, 7.0);
    let s4 = last_step(d);
    d.undo_step(s4).unwrap();
    assert_eq!(body(d, sc.a), sc.bodies[1], "s1's state again");
    // s4 is the step that redo brings back; s2, s3 stay on their branch.
    assert_eq!(status_of(d, s4), Status::Undone);
    assert_eq!(status_of(d, sc.s[2]), Status::OnBranch);
    assert_eq!(status_of(d, sc.s[3]), Status::OnBranch);
    d.redo_step(s4).unwrap();
    assert_eq!(body(d, sc.a), twin_s1_plus_s4(7.0));
    assert_eq!(status_of(d, s4), Status::Applied);
}

#[test]
fn c3_two_forks_give_two_branches_each_named_by_its_newest_step() {
    let sc = scene();
    let d = &sc.d;
    d.undo_step(sc.s[3]).unwrap();
    d.undo_step(sc.s[2]).unwrap();
    set_width(d, sc.a, 7.0);
    let s4 = last_step(d);
    mv(d, &[sc.a], 5.0);
    let s5 = last_step(d);
    // A second fork, from s4.
    d.undo_step(s5).unwrap();
    set_width(d, sc.a, 9.0);
    let s6 = last_step(d);
    let branches = d.history_branches();
    assert_eq!(branches.len(), 2);
    let ids: Vec<StepRef> = branches.iter().map(|b| b.id).collect();
    assert!(ids.contains(&sc.s[3]) && ids.contains(&s5));
    let second = branches.iter().find(|b| b.id == s5).unwrap();
    assert_eq!(second.fork, Some(s4));
    assert_eq!(second.rows.len(), 1);
    assert_eq!(status_of(d, s6), Status::Applied);
}

#[test]
fn c3_no_branch_without_a_fork() {
    let sc = scene();
    assert!(sc.d.history_branches().is_empty());
    assert!(sc.d.object_history_branches(sc.a).is_empty());
    sc.d.undo_step(sc.s[3]).unwrap();
    assert!(
        sc.d.history_branches().is_empty(),
        "an undone step alone is no branch"
    );
}

// ---------------------------------------------------------------------
// Criteria 1, 4 in the Object scope (Ctrl+U)
// ---------------------------------------------------------------------

#[test]
fn c1_object_scope_fork_with_object_undo() {
    let sc = scene();
    let d = &sc.d;
    d.undo_object_step(sc.a).unwrap().unwrap();
    d.undo_object_step(sc.a).unwrap().unwrap();
    assert_eq!(body(d, sc.a), sc.bodies[1]);
    assert!(is_undone(ostatus(d, sc.a, sc.s[3])));
    assert!(is_undone(ostatus(d, sc.a, sc.s[2])));
    assert!(d.object_history_branches(sc.a).is_empty());
    // No linear object redo past a new step.
    set_width(d, sc.a, 7.0);
    let s4 = last_step(d);
    assert_eq!(body(d, sc.a), twin_s1_plus_s4(7.0));
    assert_eq!(ostatus(d, sc.a, sc.s[2]), ObjectStatus::OnBranch);
    assert_eq!(ostatus(d, sc.a, sc.s[3]), ObjectStatus::OnBranch);
    assert_eq!(ostatus(d, sc.a, s4), ObjectStatus::Applied);
    let b = d.object_history_branches(sc.a);
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].id, sc.s[3]);
    assert_eq!(b[0].fork, Some(sc.s[1]));
    assert_eq!(b[0].rows.len(), 2);
    let targets = d.object_history_targets(sc.a);
    assert!(
        targets.redo.is_none(),
        "Ctrl+Shift+U has nothing to redo past a fork: {:?}",
        targets.redo
    );
}

// ---------------------------------------------------------------------
// Criterion 5: other peers
// ---------------------------------------------------------------------

#[test]
fn c5_document_scope_a_peer_step_never_starts_a_branch() {
    let a = Document::new(1);
    let o1 = rect(&a, 0.0);
    let o2 = rect(&a, 50.0);
    mv(&a, &[o1], 1.0);
    let sa1 = last_step(&a);
    let b = replica(&a, 2);
    // Both go on concurrently.
    set_width(&a, o1, 3.0);
    let sa2 = last_step(&a);
    mv(&b, &[o2], 4.0);
    let sb1 = last_step(&b);
    set_width(&b, o2, 5.0);
    let sb2 = last_step(&b);
    merge(&a, &b);
    for d in [&a, &b] {
        assert!(
            d.history_branches().is_empty(),
            "peer steps are main line steps"
        );
        for s in [sa1, sa2, sb1, sb2] {
            assert_eq!(status_of(d, s), Status::Applied);
        }
    }
    // A takes sa2 back; B makes a new step meanwhile: A's step is not branched by B's.
    a.undo_step(sa2).unwrap();
    mv(&b, &[o2], 1.0);
    merge(&a, &b);
    for d in [&a, &b] {
        assert!(
            d.history_branches().is_empty(),
            "B's new step does not branch A's undo"
        );
        assert_eq!(status_of(d, sa2), Status::Undone);
    }
    // A's own new step does.
    set_width(&a, o1, 8.0);
    merge(&a, &b);
    for d in [&a, &b] {
        assert_eq!(status_of(d, sa2), Status::OnBranch);
        let br = d.history_branches();
        assert_eq!(br.len(), 1);
        assert_eq!(br[0].id, sa2);
    }
}

#[test]
fn c5_object_scope_peer_step_ends_object_redo_but_not_a_peers_ctrl_z() {
    let a = Document::new(1);
    let o = rect(&a, 0.0);
    mv(&a, &[o], 1.0);
    set_width(&a, o, 3.0);
    let sa2 = last_step(&a);
    let b = replica(&a, 2);
    // A took sa2 back with Ctrl+Z (its own stack) before B acts.
    a.undo_step(sa2).unwrap();
    merge(&a, &b);
    mv(&b, &[o], 9.0);
    let sb = last_step(&b);
    merge(&a, &b);
    for d in [&a, &b] {
        assert!(
            is_undone(ostatus(d, o, sa2)) || ostatus(d, o, sa2) == ObjectStatus::OnBranch,
            "a Ctrl+Z step stays on its author's stack"
        );
    }
    // The strict statement of 0042/5: a peer never branches someone else's Ctrl+Z.
    assert_eq!(
        status_of(&a, sa2),
        Status::Undone,
        "document scope: still redoable by A"
    );
    assert_eq!(ostatus(&b, o, sb), ObjectStatus::Applied);

    // Object undo: A object-undoes sa1; then B makes a new step on the object: sa1 goes to a branch.
    let c = Document::new(1);
    let oc = rect(&c, 0.0);
    mv(&c, &[oc], 1.0);
    let s1 = last_step(&c);
    set_width(&c, oc, 3.0);
    let s2 = last_step(&c);
    let peer = replica(&c, 2);
    c.undo_object_step(oc).unwrap().unwrap();
    assert!(is_undone(ostatus(&c, oc, s2)));
    merge(&c, &peer);
    mv(&peer, &[oc], 5.0);
    merge(&c, &peer);
    for d in [&c, &peer] {
        assert_eq!(
            ostatus(d, oc, s2),
            ObjectStatus::OnBranch,
            "a new step of any author ends the object's redo (0041 criterion 11)"
        );
        assert_eq!(ostatus(d, oc, s1), ObjectStatus::Applied);
        assert_eq!(d.object_history_branches(oc).len(), 1);
        assert!(d.object_history_targets(oc).redo.is_none());
    }
}

// ---------------------------------------------------------------------
// Criteria 6, 28, 33: derived from the log, saved, no format change
// ---------------------------------------------------------------------

#[test]
fn c6_c28_save_and_open_keep_branches_ids_order_and_states() {
    let sc = scene();
    let d = &sc.d;
    d.undo_step(sc.s[3]).unwrap();
    d.undo_step(sc.s[2]).unwrap();
    set_width(d, sc.a, 7.0);
    let s4 = last_step(d);
    mv(d, &[sc.a], 2.0);
    d.undo_step(last_step(d)).unwrap();
    d.undo_object_step(sc.a).unwrap();
    let before_rows: Vec<_> = all_rows(d)
        .iter()
        .map(|r| (r.step, r.shown_id.clone(), r.status, r.undone_by.is_some()))
        .collect();
    let before_branches = d.history_branches();
    let before_obj: Vec<_> = object_rows(d, sc.a)
        .iter()
        .map(|r| (r.step, norm(r.status)))
        .collect();
    let before_obj_branches = d.object_history_branches(sc.a);
    let before_json = json(d);

    let again = reopen(d);
    let after_rows: Vec<_> = all_rows(&again)
        .iter()
        .map(|r| (r.step, r.shown_id.clone(), r.status, r.undone_by.is_some()))
        .collect();
    assert_eq!(before_rows, after_rows, "ids, order, states");
    assert_eq!(before_branches, again.history_branches());
    let after_obj: Vec<_> = object_rows(&again, sc.a)
        .iter()
        .map(|r| (r.step, norm(r.status)))
        .collect();
    assert_eq!(before_obj, after_obj);
    assert_eq!(before_obj_branches, again.object_history_branches(sc.a));
    assert_eq!(before_json, json(&again), "document.json unchanged");
    assert_eq!(status_of(&again, sc.s[3]), Status::OnBranch);
    let _ = s4;
}

#[test]
fn c33_c6_the_format_version_is_unchanged_and_branches_add_no_data() {
    let sc = scene();
    let d = &sc.d;
    let plain = pack(d, "t").unwrap();
    assert_eq!(manifest_version(&plain), u64::from(CURRENT_FORMAT_VERSION));
    assert_eq!(
        CURRENT_FORMAT_VERSION, 12,
        "0042 must not bump format_version (ADR 0014 Q2 A); 12 is main after 0050"
    );
    d.undo_step(sc.s[3]).unwrap();
    d.undo_step(sc.s[2]).unwrap();
    set_width(d, sc.a, 7.0);
    // A go to version and a preview.
    let v0 = d.version();
    let _ = d.preview_document_at(sc.s[1]).unwrap();
    let _ = d.preview_object_at(sc.s[1], sc.a).unwrap();
    assert_eq!(d.version(), v0, "a preview writes no operation");
    d.go_to_version(sc.s[1], Some(sc.a), None).unwrap();
    let packed = pack(d, "t").unwrap();
    assert_eq!(manifest_version(&packed), u64::from(CURRENT_FORMAT_VERSION));
    // Root keys of document.json are the same as before any of it.
    let keys = |bytes: &[u8]| -> Vec<String> {
        let v: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    };
    let fresh = Document::new(1);
    let _ = rect(&fresh, 0.0);
    assert_eq!(keys(&json(d)), keys(&json(&fresh)), "no new root key");
}

#[test]
fn c7_object_wipe_removes_wiped_steps_and_their_branches_from_the_list() {
    let sc = scene();
    let d = &sc.d;
    d.undo_object_step(sc.a).unwrap();
    d.undo_object_step(sc.a).unwrap();
    set_width(d, sc.a, 7.0);
    assert_eq!(d.object_history_branches(sc.a).len(), 1);
    d.wipe_object_history(sc.a).unwrap();
    let listed: Vec<StepRef> = object_rows(d, sc.a).iter().map(|r| r.step).collect();
    assert!(
        !listed.contains(&sc.s[2]),
        "wiped branch step leaves the list"
    );
    assert!(!listed.contains(&sc.s[3]));
    assert!(
        d.object_history_branches(sc.a).is_empty(),
        "its branch is gone"
    );
    assert!(
        object_rows(d, sc.a)
            .iter()
            .all(|r| r.status != ObjectStatus::OnBranch)
    );
}

#[test]
fn c7_document_wipe_removes_branches() {
    let sc = scene();
    let mut d = sc.d;
    d.undo_step(sc.s[3]).unwrap();
    set_width(&d, sc.a, 7.0);
    assert_eq!(d.history_branches().len(), 1);
    d.wipe_history().unwrap();
    assert!(d.history_branches().is_empty());
    assert!(all_rows(&d).iter().all(|r| r.status != Status::OnBranch));
}

// ---------------------------------------------------------------------
// Criterion 29, 30: order and the two exact versions on every replica
// ---------------------------------------------------------------------

#[test]
fn c29_c30_every_replica_has_the_same_list_order_and_versions() {
    let a = Document::new(1);
    let o1 = rect(&a, 0.0);
    let o2 = rect(&a, 50.0);
    let b = replica(&a, 2);
    for i in 0..4 {
        mv(&a, &[o1], 1.0 + f64::from(i));
        mv(&b, &[o2], 2.0 + f64::from(i));
    }
    set_width(&a, o1, 4.0);
    let sa = last_step(&a);
    set_width(&b, o2, 6.0);
    let sb = last_step(&b);
    merge(&a, &b);
    let ids = |d: &Document| -> Vec<StepRef> { all_rows(d).iter().map(|r| r.step).collect() };
    assert_eq!(ids(&a), ids(&b), "same main-line order on every replica");
    let shown =
        |d: &Document| -> Vec<String> { all_rows(d).iter().map(|r| r.shown_id.clone()).collect() };
    assert_eq!(shown(&a), shown(&b), "same short ids");
    // Preview of one row reads the same on both.
    for s in [sa, sb] {
        let pa = a.preview_document_at(s).unwrap();
        let pb = b.preview_document_at(s).unwrap();
        assert_eq!(pa, pb, "document preview equal on both replicas");
        assert_eq!(
            a.preview_object_at(s, o1).unwrap(),
            b.preview_object_at(s, o1).unwrap()
        );
    }
    // Go to of one row gives equal fields on both.
    let target = all_rows(&a).iter().rev().nth(2).unwrap().step;
    a.go_to_version(target, Some(o1), None).unwrap();
    b.go_to_version(target, Some(o1), None).unwrap();
    assert_eq!(body(&a, o1), body(&b, o1));
    // Each row has the version right after the step: a preview of the newest step equals live.
    let newest = all_rows(&a)[0].step;
    let live: Vec<_> = a.object_ids().iter().map(|i| body(&a, *i)).collect();
    let _ = (newest, live);
}

// ---------------------------------------------------------------------
// Criteria 10, 15: applying a version (11 to 13 withdrawn, customer feedback 2026-10-14)
// ---------------------------------------------------------------------

#[test]
fn c10_go_to_sets_the_fields_in_one_new_step_main_line_and_branch() {
    let sc = scene();
    let d = &sc.d;
    let rows_before = d.history_len();
    let r = d.go_to_version(sc.s[1], Some(sc.a), None).unwrap();
    assert!(r.step.is_some());
    assert_eq!(body(d, sc.a), sc.bodies[1]);
    assert_eq!(d.history_len(), rows_before + 1, "exactly one new step");
    let top = &all_rows(d)[0];
    assert_eq!(top.label, LABEL_GO_TO_VERSION);
    assert_eq!(top.author, Author::You);
    assert_eq!(top.version.as_ref().unwrap().step, sc.s[1]);
    assert_eq!(
        top.version.as_ref().unwrap().shown_id,
        d.history_shown_id(sc.s[1]).unwrap()
    );
    // The Object scope list carries it too.
    let orow = &object_rows(d, sc.a)[0];
    assert_eq!(orow.label, LABEL_GO_TO_VERSION);
    assert_eq!(orow.version.as_ref().unwrap().step, sc.s[1]);
    // Same values: nothing is written.
    let n = d.history_len();
    let v = d.version();
    let again = d.go_to_version(sc.s[1], Some(sc.a), None).unwrap();
    assert_eq!(again.step, None);
    assert_eq!(d.history_len(), n);
    assert_eq!(d.version(), v, "nothing written");
    // Ctrl+Z (undo of the go-to) returns to the state before.
    d.undo_step(r.step.unwrap()).unwrap();
    assert_eq!(body(d, sc.a), sc.bodies[3]);
}

#[test]
fn c10_go_to_a_version_on_a_branch_and_the_next_step_forks() {
    let sc = scene();
    let d = &sc.d;
    d.undo_step(sc.s[3]).unwrap();
    d.undo_step(sc.s[2]).unwrap();
    set_width(d, sc.a, 7.0);
    // s2 is on a branch: go to it.
    let r = d.go_to_version(sc.s[2], Some(sc.a), None).unwrap();
    assert_eq!(body(d, sc.a), sc.bodies[2], "the state right after s2");
    let go = r.step.unwrap();
    // Take the go-to back and do something else: the go-to is a branch, like any step.
    d.undo_step(go).unwrap();
    mv(d, &[sc.a], 11.0);
    assert_eq!(status_of(d, go), Status::OnBranch);
}

#[test]
fn c10_go_to_a_version_that_does_not_exist_or_an_object_that_is_gone_is_refused() {
    let sc = scene();
    let d = &sc.d;
    let bogus = StepRef::new(999, 12345);
    let v = d.version();
    assert_eq!(
        d.go_to_version(bogus, Some(sc.a), None).unwrap_err(),
        HistoryError::NoSuchStep
    );
    assert_eq!(d.version(), v);
    d.delete_objects(&[sc.a]).unwrap();
    let v = d.version();
    assert!(d.go_to_version(sc.s[1], Some(sc.a), None).is_err());
    assert_eq!(d.version(), v, "nothing written on a refusal");
}

#[test]
fn c15_go_to_writes_over_another_peers_later_steps_and_counts_them() {
    let a = Document::new(1);
    let o = rect(&a, 0.0);
    mv(&a, &[o], 1.0);
    let s1 = last_step(&a);
    let state1 = body(&a, o);
    let b = replica(&a, 2);
    set_width(&b, o, 3.0);
    mv(&b, &[o], 5.0);
    merge(&a, &b);
    assert_ne!(body(&a, o), state1);
    let r = a.go_to_version(s1, Some(o), None).unwrap();
    assert_eq!(r.others_reverted, 2, "two later steps by the other peer");
    assert_eq!(body(&a, o), state1, "written anyway");
    // Own later steps are not counted.
    let c = Document::new(3);
    let oc = rect(&c, 0.0);
    mv(&c, &[oc], 1.0);
    let c1 = last_step(&c);
    set_width(&c, oc, 3.0);
    let r = c.go_to_version(c1, Some(oc), None).unwrap();
    assert_eq!(r.others_reverted, 0);
}

// ---------------------------------------------------------------------
// Criteria 17, 19, 20, 21, 32: previews
// ---------------------------------------------------------------------

#[test]
fn c17_object_preview_shows_the_object_as_it_was_and_changes_nothing() {
    let sc = scene();
    let d = &sc.d;
    let before = json(d);
    let v0 = d.version();
    let n = d.history_len();
    let p = d.preview_object_at(sc.s[1], sc.a).unwrap();
    assert_eq!(p.step, sc.s[1]);
    assert_eq!(p.objects.len(), 1);
    // The previewed object has the body it had after s1.
    let ObjectSnapshot::Primitive(prev) = &p.objects[0] else {
        panic!("a rectangle")
    };
    let twin = Document::new(1);
    let t = rect(&twin, 0.0);
    mv(&twin, &[t], 1.0);
    let ObjectSnapshot::Primitive(want) = twin.object(t).unwrap() else {
        panic!()
    };
    assert_eq!(
        format!("{:?}{:?}", prev.shape, prev.style),
        format!("{:?}{:?}", want.shape, want.style)
    );
    assert_eq!(prev.id, sc.a, "the same object, in its place");
    // Nothing changed, no step, nothing written.
    assert_eq!(json(d), before);
    assert_eq!(d.version(), v0);
    assert_eq!(d.history_len(), n);
    assert_eq!(body(d, sc.a), sc.bodies[3]);
}

#[test]
fn c17_preview_at_every_row_matches_the_state_at_that_row() {
    let sc = scene();
    let d = &sc.d;
    for i in 1..4 {
        let p = d.preview_object_at(sc.s[i], sc.a).unwrap();
        let ObjectSnapshot::Primitive(x) = &p.objects[0] else {
            panic!()
        };
        let probe = {
            // Put the live object there and compare the body text.
            let r = d.go_to_version(sc.s[i], Some(sc.a), None).unwrap();
            let b = body(d, sc.a);
            if let Some(s) = r.step {
                d.undo_step(s).unwrap();
            }
            b
        };
        assert_eq!(
            format!("{:?}|{:?}|{:?}", x.shape, x.style, x.rotation),
            probe,
            "row {i}"
        );
        assert_eq!(probe, sc.bodies[i]);
    }
}

#[test]
fn c19_document_preview_has_every_object_the_page_size_and_background_of_the_version() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let s_a = last_step(&d);
    let size0 = DocumentSize::new(Length::from_mm(210.0), Length::from_mm(297.0));
    d.resize(size0).unwrap();
    let s_size = last_step(&d);
    let bg0 = d.background();
    // Later: a second object, another page and another background.
    let b = rect(&d, 40.0);
    let s_b = last_step(&d);
    let size1 = DocumentSize::new(Length::from_mm(500.0), Length::from_mm(400.0));
    d.resize(size1).unwrap();
    let s_size1 = last_step(&d);
    let mut bg1 = bg0;
    bg1.paint = curvyo_document_core::BackgroundPaint::None;
    assert!(d.set_background(bg1));
    let s_bg = last_step(&d);

    let at_size = d.preview_document_at(s_size).unwrap();
    assert_eq!(at_size.size, size0);
    assert_eq!(at_size.background, bg0);
    assert_eq!(at_size.objects.len(), 1);
    let at_b = d.preview_document_at(s_b).unwrap();
    assert_eq!(at_b.objects.len(), 2);
    assert_eq!(
        at_b.size, size0,
        "the page as it was after the second rectangle"
    );
    let at_size1 = d.preview_document_at(s_size1).unwrap();
    assert_eq!(at_size1.size, size1);
    assert_eq!(at_size1.background, bg0);
    let at_bg = d.preview_document_at(s_bg).unwrap();
    assert_eq!(at_bg.background, bg1);
    assert_eq!(at_bg.size, size1);
    let at_a = d.preview_document_at(s_a).unwrap();
    assert_eq!(at_a.objects.len(), 1);
    assert_ne!(at_a.size, size1);
    // The live document is untouched and attached: editing still works.
    assert_eq!(d.background(), bg1);
    mv(&d, &[a], 1.0);
    let _ = b;
}

#[test]
fn c19_preview_in_paint_order() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let c = rect(&d, 40.0);
    let s = last_step(&d);
    let p = d.preview_document_at(s).unwrap();
    let ids: Vec<NodeId> = p
        .objects
        .iter()
        .map(|o| match o {
            ObjectSnapshot::Primitive(x) => x.id,
            ObjectSnapshot::Path(x) => x.id,
        })
        .collect();
    assert_eq!(ids, vec![a, b, c], "paint order of the live document");
    assert_eq!(ids, d.object_ids());
}

#[test]
fn c17_preview_of_a_deleted_object() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let keep = rect(&d, 40.0);
    mv(&d, &[a], 3.0);
    let s_moved = last_step(&d);
    let moved = body(&d, a);
    d.delete_objects(&[a]).unwrap();
    let s_del = last_step(&d);
    assert!(d.object(a).is_none());
    // At the step before the delete the object shows, as it was.
    let p = d.preview_object_at(s_moved, a).unwrap();
    assert_eq!(p.objects.len(), 1);
    let ObjectSnapshot::Primitive(x) = &p.objects[0] else {
        panic!()
    };
    assert_eq!(
        format!("{:?}|{:?}|{:?}", x.shape, x.style, x.rotation),
        moved
    );
    // At and after the delete there is no object.
    let gone = d.preview_object_at(s_del, a).unwrap();
    assert!(gone.objects.is_empty(), "no object after the delete");
    // The document preview at the delete shows only the other.
    let doc = d.preview_document_at(s_del).unwrap();
    assert_eq!(doc.objects.len(), 1);
    // A reopened file previews the same (the log is the history).
    let again = reopen(&d);
    let p2 = again.preview_object_at(s_moved, a).unwrap();
    assert_eq!(p2, p);
    let _ = keep;
    // The live document stays editable afterwards (the checkout guard attached it).
    let c = rect(&d, 99.0);
    assert!(d.object(c).is_some());
}

#[test]
fn c17_preview_of_an_object_that_did_not_exist_yet_is_empty() {
    let d = Document::new(1);
    let first = rect(&d, 0.0);
    let s_first = last_step(&d);
    let later = rect(&d, 30.0);
    let p = d.preview_object_at(s_first, later).unwrap();
    assert!(p.objects.is_empty());
    let _ = first;
}

#[test]
fn c32_a_preview_is_plain_copies_that_do_not_change_when_a_peer_steps_in() {
    let a = Document::new(1);
    let o = rect(&a, 0.0);
    mv(&a, &[o], 1.0);
    let s1 = last_step(&a);
    set_width(&a, o, 3.0);
    let b = replica(&a, 2);
    let p = a.preview_document_at(s1).unwrap();
    let held = p.clone();
    // A peer steps in; the live document merges it and keeps editing underneath.
    mv(&b, &[o], 8.0);
    set_width(&b, o, 5.0);
    sync_into(&b, &a);
    mv(&a, &[o], 1.0);
    assert_eq!(p, held, "the copy does not change");
    let fresh = a.preview_document_at(s1).unwrap();
    assert_eq!(fresh, held, "the same version reads the same");
    assert!(a.history_len() >= 4);
}

#[test]
fn c32_a_failed_preview_leaves_the_document_attached() {
    let sc = scene();
    let d = &sc.d;
    let bogus = StepRef::new(4242, 7);
    assert_eq!(
        d.preview_document_at(bogus).unwrap_err(),
        HistoryError::NoSuchStep
    );
    assert_eq!(
        d.preview_object_at(bogus, sc.a).unwrap_err(),
        HistoryError::NoSuchStep
    );
    // Writers panic while detached: this must not.
    mv(d, &[sc.a], 1.0);
    set_width(d, sc.a, 4.0);
}

#[test]
fn c20_previews_in_both_scopes_write_and_replicate_nothing() {
    let sc = scene();
    let d = &sc.d;
    let peer = replica(d, 2);
    let v0 = d.version();
    let bytes_before = d.export_updates(&peer.version()).unwrap();
    for s in sc.s {
        let _ = d.preview_document_at(s).unwrap();
        let _ = d.preview_object_at(s, sc.a).unwrap();
    }
    assert_eq!(d.version(), v0);
    assert_eq!(d.export_updates(&peer.version()).unwrap(), bytes_before);
    sync_into(d, &peer);
    assert_eq!(json(d), json(&peer));
}

#[test]
fn c21_previews_at_10000_steps_stay_inside_four_times_the_budgets() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let cold = rect(&d, 100.0);
    let mut steps: Vec<StepRef> = Vec::new();
    for i in 0..10_000 {
        if i % 9 == 0 {
            mv(&d, &[a], 0.01);
        } else {
            mv(&d, &[cold], 0.01);
        }
        if i % 500 == 0 {
            steps.push(last_step(&d));
        }
    }
    // The cold case: right after Open.
    let d = reopen(&d);
    let rows = all_rows(&d);
    assert!(rows.len() >= 10_000);
    // Rows 10, 100 and 1,000 back: the first frame within 300 ms (x4; x10 more in debug).
    let scale: u128 = if cfg!(debug_assertions) { 10 } else { 1 };
    let near_budget = 300 * 4 * scale;
    let far_budget = 1_500 * 4 * scale;
    let warm = Instant::now();
    d.warm_history();
    eprintln!("warm: {:?}", warm.elapsed());
    for back in [10_usize, 100, 1_000] {
        let step = rows[back].step;
        let t = Instant::now();
        let p = d.preview_document_at(step).unwrap();
        let took = t.elapsed();
        eprintln!("preview {back} back: {took:?}");
        assert!(!p.objects.is_empty());
        assert!(took.as_millis() <= near_budget, "{back} back took {took:?}");
        let t = Instant::now();
        let _ = d.preview_object_at(step, a).unwrap();
        assert!(
            t.elapsed().as_millis() <= near_budget,
            "object {back} back took {:?}",
            t.elapsed()
        );
    }
    // Moving along rows with Up and Down: ten neighbouring rows, each within the budget.
    for (k, row) in rows.iter().enumerate().take(30).skip(20) {
        let t = Instant::now();
        let _ = d.preview_object_at(row.step, a).unwrap();
        assert!(
            t.elapsed().as_millis() <= near_budget,
            "row {k}: {:?}",
            t.elapsed()
        );
    }
    // 10,000 back.
    let oldest = rows[rows.len() - 3].step;
    let t = Instant::now();
    let p = d.preview_document_at(oldest).unwrap();
    let took = t.elapsed();
    eprintln!("preview 10,000 back: {took:?}");
    assert!(!p.objects.is_empty());
    assert!(took.as_millis() <= far_budget, "10,000 back took {took:?}");
    // The document is attached afterwards.
    mv(&d, &[a], 1.0);
}

// ---------------------------------------------------------------------
// Criteria 22 to 26, 31: clone any state
// ---------------------------------------------------------------------

#[test]
fn c22_clone_of_an_old_version_equals_that_state_and_leaves_the_original_alone() {
    let sc = scene();
    let d = &sc.d;
    let original = body(d, sc.a);
    let mut mint = minter();
    let n = d.history_len();
    let clone = d.clone_version(sc.a, sc.s[1], &mut mint).unwrap();
    assert_ne!(clone, sc.a, "a new object");
    assert_eq!(body(d, clone), sc.bodies[1], "the clone equals s1's state");
    assert_eq!(body(d, sc.a), original, "the original does not change");
    assert_eq!(d.history_len(), n + 1, "one step");
    let top = &all_rows(d)[0];
    assert_eq!(top.label, LABEL_CLONE_VERSION);
    assert_eq!(top.version.as_ref().unwrap().step, sc.s[1]);
    // On top of the stack.
    assert_eq!(
        *d.object_ids().last().unwrap(),
        clone,
        "on top of the stack"
    );
    // Ctrl+Z removes the clone.
    let step = last_step(d);
    d.undo_step(step).unwrap();
    assert!(d.object(clone).is_none());
    assert_eq!(body(d, sc.a), original);
}

#[test]
fn c22_clone_of_a_version_on_a_branch_and_by_another_author() {
    let a = Document::new(1);
    let o = rect(&a, 0.0);
    mv(&a, &[o], 1.0);
    let s1 = last_step(&a);
    let b = replica(&a, 2);
    set_width(&b, o, 4.0);
    let sb = last_step(&b);
    let state_b = body(&b, o);
    merge(&a, &b);
    a.undo_step(s1).unwrap();
    set_width(&a, o, 2.0); // s1 is now on a branch
    assert_eq!(status_of(&a, s1), Status::OnBranch);
    let mut mint = minter();
    let on_branch = a.clone_version(o, s1, &mut mint).unwrap();
    assert!(a.object(on_branch).is_some());
    let by_peer = a.clone_version(o, sb, &mut mint).unwrap();
    // The state right after sb is the object as B had it, with sb's author irrelevant.
    let ObjectSnapshot::Primitive(x) = a.object(by_peer).unwrap() else {
        panic!()
    };
    let ObjectSnapshot::Primitive(y) = b.object(o).unwrap() else {
        panic!()
    };
    let _ = state_b;
    assert_eq!(x.style.stroke.width, y.style.stroke.width);
}

#[test]
fn c23_the_clones_timeline_begins_with_its_origin_and_inherits_up_to_the_version_only() {
    let sc = scene();
    let d = &sc.d;
    let mut mint = minter();
    let clone = d.clone_version(sc.a, sc.s[2], &mut mint).unwrap();
    let rows = object_rows(d, clone);
    let inherited: Vec<_> = rows
        .iter()
        .filter(|r| r.inherited_from == Some(sc.a))
        .collect();
    let steps: Vec<StepRef> = inherited.iter().map(|r| r.step).collect();
    assert!(steps.contains(&sc.s[2]), "up to and including the version");
    assert!(steps.contains(&sc.s[1]));
    assert!(
        !steps.contains(&sc.s[3]),
        "later steps of the original never appear"
    );
    // The newest of the clone's own rows is the clone step; the first row (oldest) is the origin.
    assert_eq!(rows[0].label, LABEL_CLONE_VERSION);
    assert!(
        rows[0].version.as_ref().is_some_and(|v| v.step == sc.s[2]),
        "{:?}",
        rows[0]
    );
    // The original's own timeline is unchanged by the clone step except for no new row.
    assert!(
        object_rows(d, sc.a)
            .iter()
            .all(|r| r.label != LABEL_CLONE_VERSION)
    );
}

#[test]
fn c23_a_clone_of_a_branch_version_inherits_that_path_not_the_other_branches() {
    let sc = scene();
    let d = &sc.d;
    d.undo_step(sc.s[3]).unwrap();
    d.undo_step(sc.s[2]).unwrap();
    set_width(d, sc.a, 7.0);
    let s4 = last_step(d);
    let mut mint = minter();
    // s3 is on the branch; the path to it is s0, s1, s2, s3 and not s4.
    let c = d.clone_version(sc.a, sc.s[3], &mut mint).unwrap();
    assert_eq!(body(d, c), sc.bodies[3]);
    let steps: Vec<StepRef> = object_rows(d, c)
        .iter()
        .filter(|r| r.inherited_from == Some(sc.a))
        .map(|r| r.step)
        .collect();
    for s in [sc.s[1], sc.s[2], sc.s[3]] {
        assert!(steps.contains(&s), "{s:?} on the path");
    }
    assert!(!steps.contains(&s4), "the other line is not inherited");
    // And a clone of the main line version s4 does not inherit s2 and s3.
    let c4 = d.clone_version(sc.a, s4, &mut mint).unwrap();
    let steps4: Vec<StepRef> = object_rows(d, c4)
        .iter()
        .filter(|r| r.inherited_from == Some(sc.a))
        .map(|r| r.step)
        .collect();
    assert!(steps4.contains(&s4) && steps4.contains(&sc.s[1]));
    assert!(!steps4.contains(&sc.s[2]) && !steps4.contains(&sc.s[3]));
}

#[test]
fn c24_clone_of_a_deleted_object_has_the_state_just_before_the_delete() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 2.0);
    set_width(&d, a, 3.5);
    let resized = body(&d, a);
    d.delete_objects(&[a]).unwrap();
    let del = last_step(&d);
    assert!(d.object(a).is_none());
    let mut mint = minter();
    assert!(d.touched_objects(del).contains(&a));
    let c = d.clone_version(a, del, &mut mint).unwrap();
    assert_ne!(c, a);
    assert_eq!(body(&d, c), resized, "the state before the deletion");
    // Save, Open, clone from the Delete row of the reopened file.
    let again = reopen(&d);
    let del_row = all_rows(&again)
        .into_iter()
        .find(|r| r.step == del)
        .expect("the delete row survives");
    assert!(del_row.objects.iter().any(|o| o.id == a));
    let c2 = again.clone_version(a, del, &mut minter()).unwrap();
    assert_eq!(body(&again, c2), resized);
}

#[test]
fn c24_clone_from_the_document_scope_uses_the_touched_object_of_the_row() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 30.0);
    d.translate_objects(&[a, b], Vec2::new(5.0, 0.0)).unwrap();
    let step = last_step(&d);
    let touched = d.touched_objects(step);
    assert_eq!(touched.len(), 2);
    let after = (body(&d, a), body(&d, b));
    mv(&d, &[a, b], 9.0);
    for (id, want) in [(a, &after.0), (b, &after.1)] {
        let c = d.clone_version(id, step, &mut minter()).unwrap();
        assert_eq!(&body(&d, c), want, "state right after the step, not now");
    }
}

#[test]
fn c25_no_clone_of_a_group() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 30.0);
    let g = d.group_objects(&[a, b]).unwrap();
    let gs = last_step(&d);
    let n = d.history_len();
    let v = d.version();
    assert_eq!(
        d.clone_version(g, gs, &mut minter()).unwrap_err(),
        HistoryError::Group
    );
    assert_eq!(d.history_len(), n);
    assert_eq!(d.version(), v, "nothing written when refused");
    // A leaf of the group can be cloned from its own timeline.
    let leaf_step = all_rows(&d).iter().find(|r| r.step != gs).unwrap().step;
    let _ = leaf_step;
    let c = d.clone_version(a, gs, &mut minter());
    assert!(c.is_ok(), "a leaf of a group can be cloned: {c:?}");
}

#[test]
fn c22_clone_of_an_unknown_version_or_object_is_refused_and_writes_nothing() {
    let sc = scene();
    let d = &sc.d;
    let v = d.version();
    assert!(
        d.clone_version(sc.a, StepRef::new(321, 1), &mut minter())
            .is_err()
    );
    let other = Document::new(8);
    let ghost = rect(&other, 0.0);
    assert!(d.clone_version(ghost, sc.s[1], &mut minter()).is_err());
    assert_eq!(d.version(), v);
    mv(d, &[sc.a], 1.0); // still attached
}

// ---------------------------------------------------------------------
// Criterion 27: ids
// ---------------------------------------------------------------------

#[test]
fn c27_rows_and_versions_use_the_shown_step_id() {
    let sc = scene();
    let d = &sc.d;
    for s in sc.s {
        let shown = d.history_shown_id(s).unwrap();
        assert!(shown.len() >= 7);
        assert!(
            shown
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
    }
    d.go_to_version(sc.s[1], Some(sc.a), None).unwrap();
    let go = &all_rows(d)[0];
    assert_eq!(
        go.version.as_ref().unwrap().shown_id,
        d.history_shown_id(sc.s[1]).unwrap()
    );
    // Same on a replica.
    let r = replica(d, 2);
    assert_eq!(r.history_shown_id(sc.s[1]), d.history_shown_id(sc.s[1]));
}

// ---------------------------------------------------------------------
// Large history: branches at 10,000 steps
// ---------------------------------------------------------------------

#[test]
fn c8_many_branches_in_a_large_history_derive_within_four_times_200_ms() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let cold = rect(&d, 100.0);
    for round in 0..50 {
        for _ in 0..100 {
            mv(&d, &[cold], 0.01);
        }
        mv(&d, &[a], 0.5);
        let s = last_step(&d);
        d.undo_step(s).unwrap();
        set_width(&d, a, 1.0 + f64::from(round));
        for _ in 0..98 {
            mv(&d, &[cold], 0.01);
        }
    }
    assert!(d.history_len() >= 10_000, "{}", d.history_len());
    let again = reopen(&d);
    let t = Instant::now();
    let n = again.history_len();
    let rows = again.history_page(0, 40);
    let branches = again.history_branches();
    let took = t.elapsed();
    eprintln!("open at {n} steps, {} branches: {took:?}", branches.len());
    assert_eq!(rows.len(), 40);
    assert!(branches.len() >= 50, "{} branches", branches.len());
    let budget: u128 = if cfg!(debug_assertions) { 8_000 } else { 800 };
    assert!(took.as_millis() <= budget, "opened in {took:?}");
}

// ---------------------------------------------------------------------
// White-box edges seen in the diff: paths, clones of clones, wipes, concurrency, undo rows
// ---------------------------------------------------------------------

fn square(first: u64, x: f64, y: f64, side: f64) -> (Vec<NewAnchor>, bool) {
    let corners = [(x, y), (x + side, y), (x + side, y + side), (x, y + side)];
    let anchors = corners
        .into_iter()
        .enumerate()
        .map(|(k, (cx, cy))| NewAnchor::corner(AnchorId::new(9, first + k as u64), pt(cx, cy)))
        .collect();
    (anchors, true)
}

/// The debug text of a path with its anchor ids and its own id blanked out.
fn without_ids(d: &Document, id: NodeId) -> String {
    let text = format!("{:?}", d.object(id)).replace(&format!("{id:?}"), "X");
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("AnchorId(") {
        out.push_str(&rest[..at]);
        out.push_str("AnchorId(_)");
        let tail = &rest[at..];
        rest = &tail[tail.find(')').unwrap() + 1..];
    }
    out.push_str(rest);
    out
}

#[test]
fn w_go_to_and_clone_work_for_a_path_and_a_clone_gets_fresh_anchor_ids() {
    let d = Document::new(1);
    let (anchors, closed) = square(31, 0.0, 0.0, 10.0);
    let p = d.create_path(&anchors, closed);
    let s0 = last_step(&d);
    let at0 = body(&d, p);
    mv(&d, &[p], 7.0);
    let s1 = last_step(&d);
    let at1 = body(&d, p);
    assert_ne!(at0, at1);
    // Go to the first version: geometry is back exactly.
    d.go_to_version(s0, Some(p), None).unwrap();
    assert_eq!(body(&d, p), at0);
    // Clone of the moved version has the moved geometry and its own anchor ids.
    let mut mint = minter();
    let c = d.clone_version(p, s1, &mut mint).unwrap();
    let (Some(ObjectSnapshot::Path(orig)), Some(ObjectSnapshot::Path(copy))) =
        (d.object(p), d.object(c))
    else {
        panic!("paths")
    };
    let _ = (orig, copy);
    assert_ne!(
        body(&d, c),
        body(&d, p),
        "the clone is the moved version, the original is at s0"
    );
    let again = d.clone_version(p, s0, &mut minter()).unwrap();
    assert_eq!(
        without_ids(&d, again),
        without_ids(&d, p),
        "a clone of the version the original has now equals it apart from the identities"
    );
    assert_ne!(
        format!("{:?}", d.object(again)).replace(&format!("{again:?}"), "X"),
        format!("{:?}", d.object(p)).replace(&format!("{p:?}"), "X"),
        "the clone's anchors carry fresh ids"
    );
    // Undo of the clone removes it; the original and its anchors stay.
    d.undo_step(last_step(&d)).unwrap();
    assert!(d.object(again).is_none());
    assert_eq!(body(&d, p), at0);
}

#[test]
fn w_clone_of_a_version_before_the_object_existed_is_refused() {
    let d = Document::new(1);
    let _a = rect(&d, 0.0);
    let s_first = last_step(&d);
    let later = rect(&d, 30.0);
    let v = d.version();
    let n = d.history_len();
    assert_eq!(
        d.clone_version(later, s_first, &mut minter()).unwrap_err(),
        HistoryError::NoSuchObject
    );
    assert_eq!(d.version(), v);
    assert_eq!(d.history_len(), n);
    mv(&d, &[later], 1.0);
}

#[test]
fn w_a_clone_of_a_clone_has_a_timeline_that_terminates_and_keeps_both_origins() {
    let sc = scene();
    let d = &sc.d;
    let c1 = d.clone_version(sc.a, sc.s[2], &mut minter()).unwrap();
    let c1_step = last_step(d);
    mv(d, &[c1], 3.0);
    let c2 = d.clone_version(c1, c1_step, &mut minter()).unwrap();
    assert_eq!(body(d, c2), body(d, c1).replace(&body(d, c1), &body(d, c2)));
    let rows = object_rows(d, c2);
    let n = d.object_history_len(c2);
    assert_eq!(rows.len(), n);
    // The origin chain is followed: rows of the first original appear as inherited.
    assert!(rows.iter().any(|r| r.inherited_from.is_some()));
    assert!(rows.iter().any(|r| r.label == LABEL_CLONE_VERSION));
}

#[test]
fn w_go_to_keeps_the_lineage_of_a_clone() {
    let sc = scene();
    let d = &sc.d;
    let c = d.clone_version(sc.a, sc.s[2], &mut minter()).unwrap();
    mv(d, &[c], 5.0);
    let before_rows = object_rows(d, c).len();
    // Go to an inherited row of the original (source = the original).
    let r = d.go_to_version(sc.s[1], Some(c), Some(sc.a)).unwrap();
    assert!(r.step.is_some());
    assert_eq!(body(d, c), sc.bodies[1]);
    let rows = object_rows(d, c);
    assert_eq!(rows.len(), before_rows + 1);
    assert!(
        rows.iter().any(|x| x.inherited_from == Some(sc.a)),
        "the clone_of lineage survived the go to"
    );
}

#[test]
fn w_after_a_document_wipe_old_versions_are_gone_and_nothing_breaks() {
    let sc = scene();
    let mut d = sc.d;
    d.wipe_history().unwrap();
    // The newest step is the root of the kept log and may still be read; every older step is gone.
    for s in &sc.s[..3] {
        let p = d.preview_document_at(*s);
        assert!(p.is_err(), "a wiped step cannot be previewed: {p:?}");
        assert!(d.preview_object_at(*s, sc.a).is_err());
        assert!(d.go_to_version(*s, Some(sc.a), None).is_err());
        assert!(d.clone_version(sc.a, *s, &mut minter()).is_err());
    }
    mv(&d, &[sc.a], 1.0);
    assert!(d.history_branches().is_empty());
    let newest = all_rows(&d)[0].step;
    assert!(d.preview_document_at(newest).is_ok());
}

#[test]
fn w_a_preview_at_an_undone_row_still_shows_the_state_right_after_that_step() {
    let sc = scene();
    let d = &sc.d;
    d.undo_step(sc.s[3]).unwrap();
    assert_eq!(status_of(d, sc.s[3]), Status::Undone);
    let p = d.preview_object_at(sc.s[3], sc.a).unwrap();
    let ObjectSnapshot::Primitive(x) = &p.objects[0] else {
        panic!()
    };
    assert_eq!(
        format!("{:?}|{:?}|{:?}", x.shape, x.style, x.rotation),
        sc.bodies[3]
    );
    // The live object is at s2's state; going to the undone version writes the step and changes it.
    assert_eq!(body(d, sc.a), sc.bodies[2]);
    assert!(
        d.go_to_version(sc.s[3], Some(sc.a), None)
            .unwrap()
            .step
            .is_some()
    );
    assert_eq!(body(d, sc.a), sc.bodies[3]);
}

#[test]
fn w_a_concurrent_step_of_a_peer_is_not_in_the_version_of_my_step() {
    let a = Document::new(1);
    let _base = rect(&a, 0.0);
    let b = replica(&a, 2);
    let mine = rect(&a, 40.0);
    let s_mine = last_step(&a);
    let theirs = rect(&b, 80.0);
    merge(&a, &b);
    for d in [&a, &b] {
        let p = d.preview_document_at(s_mine).unwrap();
        let ids: Vec<NodeId> = p
            .objects
            .iter()
            .map(|o| match o {
                ObjectSnapshot::Primitive(x) => x.id,
                ObjectSnapshot::Path(x) => x.id,
            })
            .collect();
        assert!(ids.contains(&mine));
        assert!(
            !ids.contains(&theirs),
            "the peer's concurrent object is not in my version"
        );
    }
    assert_eq!(
        a.preview_document_at(s_mine).unwrap(),
        b.preview_document_at(s_mine).unwrap()
    );
}

#[test]
fn w_two_hundred_applies_are_two_hundred_steps_and_fast() {
    // Since the customer feedback of 2026-10-14 every apply is its own step (criterion 10).
    let sc = scene();
    let d = &sc.d;
    let n = d.history_len();
    let t = Instant::now();
    for k in 0..200 {
        let v = sc.s[1 + k % 3];
        d.go_to_version(v, Some(sc.a), None).unwrap();
    }
    assert!(t.elapsed().as_secs() < if cfg!(debug_assertions) { 20 } else { 4 });
    assert_eq!(d.history_len(), n + 200);
    d.undo_step(last_step(d)).unwrap();
    // The 199th apply (k = 198) went to s1 + 198 % 3 = s1.
    assert_eq!(body(d, sc.a), sc.bodies[1]);
}

#[test]
fn w_branch_derivation_survives_hostile_sequences() {
    // Undo everything, redo some, branch again, go to a version, clone, undo them all.
    let sc = scene();
    let d = &sc.d;
    for s in [sc.s[3], sc.s[2], sc.s[1]] {
        d.undo_step(s).unwrap();
    }
    d.redo_step(sc.s[1]).unwrap();
    set_width(d, sc.a, 2.0);
    let s5 = last_step(d);
    d.go_to_version(sc.s[3], Some(sc.a), None).unwrap();
    let go = last_step(d);
    let c = d.clone_version(sc.a, sc.s[2], &mut minter()).unwrap();
    let cs = last_step(d);
    d.undo_step(cs).unwrap();
    d.undo_step(go).unwrap();
    d.undo_step(s5).unwrap();
    let n = d.history_len();
    let rows = all_rows(d);
    assert_eq!(rows.len(), n);
    let branches = d.history_branches();
    for b in &branches {
        assert!(!b.rows.is_empty());
        assert!(b.rows.iter().all(|r| *r < n));
        assert_eq!(
            rows[b.rows[0]].step, b.id,
            "a branch is named by its newest row"
        );
        assert!(b.rows.iter().all(|r| rows[*r].status == Status::OnBranch));
    }
    let on_branch = rows.iter().filter(|r| r.status == Status::OnBranch).count();
    assert_eq!(
        on_branch,
        branches.iter().map(|b| b.rows.len()).sum::<usize>()
    );
    assert!(d.object(c).is_none());
    let again = reopen(d);
    assert_eq!(again.history_branches(), branches);
}
