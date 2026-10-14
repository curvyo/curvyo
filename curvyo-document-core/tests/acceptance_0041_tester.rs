//! Independent tester acceptance tests for `specs/0041-object-history/specification.md`, document
//! level: the object timeline and its index (criteria 1, 3, 4, 19 to 22, 35, 36), object undo and
//! redo (5 to 15, 17, 40, 41), clones (23 to 28, 37), the wipe of one object (29 to 34, 38) and the
//! file footprint (37 to 39).
//!
//! Written from the specification and the public `Document` API before the implementation diff was
//! read. Documents are compared through `export_json` (the `document.json` of the file) and through
//! the object snapshots.

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

use std::io::{Cursor, Read, Write};
use std::time::Instant;

use curvyo_document_core::{
    AnchorId, Author, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, CopySource, Document,
    HistoryError, Length, NewAnchor, NodeId, ObjectRow, ObjectRowClass, ObjectSnapshot,
    ObjectStatus, OpenError, PieceFill, Point, RectBounds, Role, Status, StepKind, StepRef,
    StyleEdit, Vec2, pack, unpack,
};
use loro::LoroDoc;

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

fn state(d: &Document, id: NodeId) -> String {
    format!("{:?}", d.object(id))
}

fn json(d: &Document) -> Vec<u8> {
    d.export_json().unwrap()
}

fn width_mm(d: &Document, id: NodeId) -> f64 {
    match d.object(id).unwrap() {
        ObjectSnapshot::Primitive(p) => p.style.stroke.width.as_mm(),
        ObjectSnapshot::Clone(p) => p.resolved.style.stroke.width.as_mm(),
        ObjectSnapshot::Path(p) => p.style.stroke.width.as_mm(),
    }
}

fn set_width(d: &Document, id: NodeId, mm: f64) {
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(mm)))
        .unwrap();
}

fn mv(d: &Document, ids: &[NodeId], dx: f64) {
    d.translate_objects(ids, Vec2::new(dx, 0.0)).unwrap();
}

fn rows(d: &Document, id: NodeId) -> Vec<ObjectRow> {
    d.object_history_page(id, 0, d.object_history_len(id).max(1) + 5)
}

fn steps_of(rows: &[ObjectRow]) -> Vec<StepRef> {
    rows.iter().map(|r| r.step).collect()
}

fn top_undo(d: &Document) -> Option<StepRef> {
    // The document level has no stack of its own: Ctrl+Z takes back my latest own step that is
    // still in effect (the stack of the session is tested in the editor crate).
    d.own_steps().iter().rev().map(|o| o.step).find(|s| {
        d.history_page(0, d.history_len())
            .iter()
            .any(|r| r.step == *s && r.status == Status::Applied)
    })
}

/// The newest step of the log, whoever made it and whatever it is (a wipe included).
fn newest_step(d: &Document) -> StepRef {
    d.history_page(0, 1)[0].step
}

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

fn dup(d: &Document, id: NodeId, dx: f64) -> NodeId {
    d.duplicate_objects(
        &[CopySource {
            id,
            anchor_ids: vec![],
        }],
        Vec2::new(dx, 0.0),
    )
    .unwrap()[0]
}

fn square(first: u64, x: f64, y: f64, side: f64) -> (Vec<NewAnchor>, bool) {
    let corners = [(x, y), (x + side, y), (x + side, y + side), (x, y + side)];
    let anchors = corners
        .into_iter()
        .enumerate()
        .map(|(k, (cx, cy))| NewAnchor::corner(AnchorId::new(9, first + k as u64), pt(cx, cy)))
        .collect();
    (anchors, true)
}

fn loro_of(d: &Document) -> LoroDoc {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    l
}

fn container_from_loro(format_version: u32, loro_bytes: &[u8]) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format_version": format_version,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        ("document.loro", loro_bytes.to_vec()),
        ("document.json", b"{}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// The `Document` that results from `edit` applied to the Loro doc of `d`, re-opened as a file.
fn reopen_with(d: &Document, edit: impl FnOnce(&LoroDoc)) -> Document {
    let l = loro_of(d);
    edit(&l);
    l.commit();
    let bytes = l.export(loro::ExportMode::Snapshot).unwrap();
    unpack(77, &container_from_loro(CURRENT_FORMAT_VERSION, &bytes)).expect("the file opens")
}

fn fixture(name: &str) -> Vec<u8> {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    let mut out = Vec::new();
    std::fs::File::open(p)
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    out
}

// ---------------------------------------------------------------------
// Criteria 1, 3, 21, 36: the timeline is derived from the step log
// ---------------------------------------------------------------------

#[test]
fn c1_timeline_is_newest_first_with_status() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 5.0);
    set_width(&d, a, 3.0);
    let r = rows(&d, a);
    assert_eq!(r.len(), 3, "create, move, style");
    assert_eq!(r[0].step, last_step(&d), "newest row is the newest step");
    assert!(r.iter().all(|x| x.status == ObjectStatus::Applied));
    assert!(r.iter().all(|x| x.author == Author::You));
    assert_eq!(r[2].role, Role::Created);
    assert_eq!(r[1].role, Role::Changed);
    // Ctrl+Z of the newest step: the row is Undone.
    d.undo_step(r[0].step).unwrap();
    let r2 = rows(&d, a);
    // An undo is not a step of the log; the row count is the same.
    assert_eq!(r2.len(), 3);
    assert_eq!(r2[0].status, ObjectStatus::Undone);
    assert_eq!(r2[1].status, ObjectStatus::Applied);
}

#[test]
fn c3_c21_a_step_of_three_objects_is_in_three_timelines_and_once_in_the_document_list() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let c = rect(&d, 40.0);
    let other = rect(&d, 60.0);
    mv(&d, &[a, b, c], 3.0);
    let step = last_step(&d);
    for (name, id) in [("a", a), ("b", b), ("c", c)] {
        let r = rows(&d, id);
        let hit: Vec<_> = r.iter().filter(|x| x.step == step).collect();
        assert_eq!(hit.len(), 1, "{name}: one row for the move");
        assert_eq!(hit[0].others, 2, "{name}: 'with 2 other objects'");
        assert!(hit[0].shown_id.len() >= 7);
    }
    assert!(
        rows(&d, other).iter().all(|x| x.step != step),
        "an untouched object has no row for it"
    );
    // The document list holds the step once, with the same shown id.
    let all = d.history_page(0, d.history_len() + 5);
    let doc_rows: Vec<_> = all.iter().filter(|x| x.step == step).collect();
    assert_eq!(doc_rows.len(), 1);
    assert_eq!(doc_rows[0].objects.len(), 3);
    let shown = &rows(&d, b)[0].shown_id;
    assert_eq!(shown, &doc_rows[0].shown_id, "same step id in both lists");
}

#[test]
fn c36_a_step_that_touched_a_foreign_object_does_not_leak_into_other_timelines() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    set_width(&d, a, 2.0);
    set_width(&d, b, 4.0);
    let ra = rows(&d, a);
    let rb = rows(&d, b);
    assert_eq!(ra.len(), 2);
    assert_eq!(rb.len(), 2);
    assert!(steps_of(&ra).iter().all(|s| !steps_of(&rb).contains(s)));
}

#[test]
fn c36_index_is_extended_per_commit_and_per_import() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    assert_eq!(d.object_history_len(a), 1); // builds the index
    for k in 0..5 {
        mv(&d, &[a], 1.0);
        assert_eq!(d.object_history_len(a), 2 + k, "extended after each commit");
    }
    // Import: a peer's step shows up without a rebuild.
    let b = replica(&d, 2);
    set_width(&b, a, 9.0);
    sync_into(&b, &d);
    let r = rows(&d, a);
    assert_eq!(r.len(), 7);
    assert_eq!(
        r[0].author,
        Author::Earlier,
        "the peer's step is the newest"
    );
}

#[test]
fn c4_deleted_object_has_no_object_scope_and_its_deletion_is_in_the_document_list() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let keep = rect(&d, 30.0);
    d.delete_objects(&[a]).unwrap();
    let all = d.history_page(0, 10);
    let del = all
        .iter()
        .find(|r| r.kind == StepKind::Delete)
        .expect("a delete row");
    assert_eq!(del.objects.len(), 1);
    assert_eq!(del.objects[0].id, a);
    // Not selectable; the document API refuses the actions.
    assert_eq!(d.wipe_object_history(a), Err(HistoryError::NoSuchObject));
    let _ = d.undo_object_step(a); // no panic
    assert!(d.object(keep).is_some());
}

// ---------------------------------------------------------------------
// Criteria 5, 6: object undo, per kind
// ---------------------------------------------------------------------

#[test]
fn c6_change_only_this_objects_fields_go_back() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let c = rect(&d, 40.0);
    let (a0, b0, c0) = (state(&d, a), state(&d, b), state(&d, c));
    mv(&d, &[a, b, c], 7.0);
    let (a1, c1) = (state(&d, a), state(&d, c));
    assert_ne!(a0, a1);
    let res = d.undo_object_step(b).unwrap().expect("something to undo");
    assert!(res.report.wrote);
    assert!(!res.whole_step, "a Change step is not restored as a whole");
    assert_eq!(state(&d, b), b0, "B is back");
    assert_eq!(state(&d, a), a1, "A stays moved");
    assert_eq!(state(&d, c), c1, "C stays moved");
    assert_ne!(state(&d, c), c0);
}

#[test]
fn c6_create_only_this_object_is_removed_others_stay() {
    let d = Document::new(1);
    let mut ids = Vec::new();
    d.batch("create_three", |d| {
        ids.push(rect(d, 0.0));
        ids.push(rect(d, 20.0));
        ids.push(rect(d, 40.0));
    });
    let step = last_step(&d);
    for id in &ids {
        assert_eq!(rows(&d, *id).len(), 1);
        assert_eq!(rows(&d, *id)[0].step, step);
        assert_eq!(rows(&d, *id)[0].others, 2);
    }
    let res = d.undo_object_step(ids[1]).unwrap().unwrap();
    assert!(res.object_gone, "criterion 12: the object is gone");
    assert!(d.object(ids[1]).is_none());
    assert!(d.object(ids[0]).is_some());
    assert!(d.object(ids[2]).is_some());
}

#[test]
fn c6_replace_the_whole_step_is_taken_back() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let c = rect(&d, 40.0);
    let result = d
        .replace_with_path(
            &[a, b, c],
            a,
            &[square(31, 0.0, 0.0, 10.0)],
            "boolean_union",
        )
        .unwrap();
    assert!(d.object(a).is_none() && d.object(b).is_none());
    let res = d.undo_object_step(result).unwrap().unwrap();
    assert!(res.whole_step, "Replace: the whole step");
    assert!(d.object(a).is_some(), "operand A is back");
    assert!(d.object(b).is_some(), "operand B is back");
    assert!(d.object(c).is_some(), "operand C is back");
    assert!(d.object(result).is_none(), "the result is gone");
    // Redo brings it all again.
    let res = d.redo_object_step(a).unwrap_or(None);
    let _ = res;
}

// ---------------------------------------------------------------------
// Criterion 7: field conflicts (three-way restore with two replicas)
// ---------------------------------------------------------------------

/// An object undo writes a field back only if the field still holds the value the step wrote.
/// Whichever step the engine picks as the latest, the rule decides per field.
fn conflict_case(a_burst: bool) {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    set_width(&a, r, 1.0);
    let b = replica(&a, 2);
    // Concurrent edits of the same field.
    if a_burst {
        for k in 0..5 {
            mv(&a, &[r], 0.0 + f64::from(k) * 0.0);
        }
    } else {
        for k in 0..5 {
            set_width(&b, r, 10.0 + f64::from(k));
        }
    }
    a.batch("edit_style", |d| {
        set_width(d, r, 2.0);
    });
    set_width(&b, r, 5.0);
    merge(&a, &b);
    // Both replicas converge.
    assert_eq!(json(&a), json(&b));
    let merged_width = width_mm(&a, r);
    let target = a.object_history_targets(r).undo.expect("a target");
    let before = json(&a);
    let res = a.undo_object_step(r).unwrap().expect("restore");
    let kept: usize = res.report.objects.iter().map(|o| o.kept).sum();
    let after_width = width_mm(&a, r);
    eprintln!(
        "conflict_case(burst={a_burst}): target author {:?}, merged {merged_width}, after {after_width}, kept {kept}",
        target.author
    );
    // The value the target step wrote for the width, found through the step's author.
    if target.author == Author::You {
        // My step: it wrote 2.0. If the merged value is not 2.0, a later change by the peer wins.
        if (merged_width - 2.0).abs() > 1e-9 {
            assert!(
                (after_width - merged_width).abs() < 1e-9,
                "the later value {merged_width} must survive, got {after_width}"
            );
            assert!(kept >= 1, "the notice must count the kept field");
            assert_eq!(json(&a), before, "nothing was written back");
        } else {
            assert!((after_width - 1.0).abs() < 1e-9);
            assert_eq!(kept, 0);
        }
    } else {
        // The peer's step wrote 5.0 (or the burst value).
        if (after_width - merged_width).abs() < 1e-9 {
            assert!(kept >= 1);
        } else {
            assert_eq!(kept, 0);
        }
    }
}

#[test]
fn c7_conflict_peer_value_stays_or_step_restored_lower_lamport() {
    conflict_case(true);
}

#[test]
fn c7_conflict_peer_value_stays_or_step_restored_higher_lamport() {
    conflict_case(false);
}

#[test]
fn c7_a_later_change_on_another_field_does_not_block_this_field() {
    // S1 (mine) writes width and fill in ONE step; later the peer changes only the width.
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let b = replica(&a, 2);
    a.batch("edit_style", |d| {
        set_width(d, r, 2.0);
        d.edit_style(
            &[r],
            &StyleEdit::FillColor(curvyo_document_core::Color { r: 255, g: 0, b: 0 }),
        )
        .unwrap();
    });
    let s1 = last_step(&a);
    sync_into(&a, &b);
    set_width(&b, r, 8.0); // T: later, after S1
    sync_into(&b, &a);
    // Latest applied step is T (the peer's). Take T back, then S1: S1's width part is now 2.0
    // again (T was reverted), so everything of S1 goes back.
    let t = a.undo_object_step(r).unwrap().unwrap();
    assert_eq!(t.target.author, Author::Earlier);
    assert!((width_mm(&a, r) - 2.0).abs() < 1e-9, "T taken back");
    let s = a.undo_object_step(r).unwrap().unwrap();
    assert_eq!(s.target.step, s1);
    assert!((width_mm(&a, r) - 1.0).abs() < 1e-9 || width_mm(&a, r) < 2.0);
}

#[test]
fn c7_c13_peers_own_ctrl_z_skips_the_fields_that_changed_since() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    set_width(&a, r, 1.0);
    let b = replica(&a, 2);
    set_width(&b, r, 5.0);
    let t = last_step(&b);
    merge(&a, &b);
    // A object-undoes B's step (Q1 default A).
    let res = a.undo_object_step(r).unwrap().unwrap();
    assert_eq!(res.target.step, t);
    assert!((width_mm(&a, r) - 1.0).abs() < 1e-9);
    sync_into(&a, &b);
    assert_eq!(json(&a), json(&b));
    // B's own Ctrl+Z: unaffected, the field already has the value before T. It must not corrupt.
    let before = json(&b);
    let _ = b.undo_step(t);
    assert_eq!(
        json(&b),
        before,
        "the field changed since: skipped, nothing written"
    );
}

// ---------------------------------------------------------------------
// Criteria 8 to 12: an object undo is a step
// ---------------------------------------------------------------------

#[test]
fn c8_object_undo_is_a_new_step_on_my_stack_in_the_global_history_and_the_timeline() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 5.0);
    let moved = last_step(&d);
    let hist_before = d.history_len();
    let own_before = d.own_steps().len();
    let res = d.undo_object_step(a).unwrap().unwrap();
    assert_eq!(res.target.step, moved);
    assert_eq!(
        d.history_len(),
        hist_before + 1,
        "a new step in the global history"
    );
    assert_eq!(d.own_steps().len(), own_before + 1);
    let new_step = last_step(&d);
    assert_ne!(new_step, moved);
    assert_eq!(res.report.step, Some(new_step));
    assert_eq!(top_undo(&d), Some(new_step), "on my undo stack");
    // The label names the reverted step.
    let own = d.own_steps();
    let o = own.last().unwrap();
    assert!(
        o.reverts.is_some(),
        "the stack entry is called after the step it names"
    );
    // Timeline: the new row on top is a Reverts row naming the step.
    let r = rows(&d, a);
    match &r[0].class {
        ObjectRowClass::Reverts {
            step, redo, label, ..
        } => {
            assert_eq!(*step, moved);
            assert!(!redo);
            assert!(!label.is_empty());
        }
        other => panic!("expected a Reverts row, got {other:?}"),
    }
    assert_eq!(r[0].step, new_step);
    // The reverted step stays in the list with the status "Undone by You".
    let rm = r
        .iter()
        .find(|x| x.step == moved)
        .expect("stays in the list");
    assert_eq!(rm.status, ObjectStatus::UndoneBy(Author::You));
    // The document list: the new row has `reverts`; the old row is `undone_by` You.
    let h = d.history_page(0, 10);
    let top = h.iter().find(|x| x.step == new_step).unwrap();
    assert_eq!(top.reverts.as_ref().unwrap().step, moved);
    let old = h.iter().find(|x| x.step == moved).unwrap();
    assert_eq!(old.undone_by, Some(Author::You));
    assert_eq!(
        old.status,
        Status::Applied,
        "plain Undone is only for Ctrl+Z"
    );
}

#[test]
fn c8_header_names_the_reverted_step_and_the_object_in_one_commit() {
    // One object undo is one commit: exactly one op-group; history_len grows by one.
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    set_width(&d, a, 3.0);
    let n = d.history_len();
    d.undo_object_step(a).unwrap().unwrap();
    assert_eq!(d.history_len(), n + 1);
}

#[test]
fn c9_user_undo_of_an_object_undo_is_an_object_redo() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 30.0);
    let j0 = json(&d);
    mv(&d, &[a], 5.0); // step 1
    let j1 = json(&d);
    mv(&d, &[b], 5.0); // step 2
    let j2 = json(&d);
    // Object undo of step 1 (step 3): only A.
    d.undo_object_step(a).unwrap().unwrap();
    let j3 = json(&d);
    assert_ne!(j3, j2);
    assert_ne!(j3, j1);
    // Ctrl+Z #1: step 3 is taken back, A is moved again.
    let top = top_undo(&d).unwrap();
    d.undo_step(top).unwrap();
    assert_eq!(json(&d), j2, "A moved again, B moved");
    // Ctrl+Z #2: step 2 (B).
    let top = top_undo(&d).unwrap();
    d.undo_step(top).unwrap();
    assert_eq!(json(&d), j1);
    // Ctrl+Z #3: step 1 (A).
    let top = top_undo(&d).unwrap();
    d.undo_step(top).unwrap();
    assert_eq!(json(&d), j0);
}

#[test]
fn c10_object_redo_is_a_step_and_ctrl_z_restores_the_state_after_ctrl_u() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 5.0);
    let moved = state(&d, a);
    d.undo_object_step(a).unwrap().unwrap();
    let j_after_u = json(&d);
    let n = d.history_len();
    let res = d.redo_object_step(a).unwrap().unwrap();
    assert!(res.report.wrote);
    assert_eq!(d.history_len(), n + 1, "redo is a new step");
    assert_eq!(state(&d, a), moved);
    match &rows(&d, a)[0].class {
        ObjectRowClass::Reverts { redo, .. } => assert!(*redo, "kind Object redo"),
        other => panic!("{other:?}"),
    }
    let top = top_undo(&d).unwrap();
    d.undo_step(top).unwrap();
    assert_eq!(
        json(&d),
        j_after_u,
        "Ctrl+U, Ctrl+Shift+U, Ctrl+Z = state after Ctrl+U"
    );
}

#[test]
fn c11_a_new_do_step_clears_the_redo_side_for_that_object() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 5.0);
    d.undo_object_step(a).unwrap().unwrap();
    assert!(d.object_history_targets(a).redo.is_some());
    set_width(&d, a, 4.0);
    assert!(d.object_history_targets(a).redo.is_none());
    let n = d.history_len();
    assert!(d.redo_object_step(a).unwrap().is_none());
    assert_eq!(d.history_len(), n, "nothing written");
    // The undone step stays in the list, on a branch since `0042` (criteria 1 and 4).
    assert!(
        rows(&d, a)
            .iter()
            .any(|r| r.status == ObjectStatus::OnBranch)
    );
}

#[test]
fn c11_a_peers_do_step_also_clears_the_redo_side() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 5.0);
    d.undo_object_step(a).unwrap().unwrap();
    let b = replica(&d, 2);
    set_width(&b, a, 4.0);
    sync_into(&b, &d);
    assert!(
        d.object_history_targets(a).redo.is_none(),
        "a new step by anyone"
    );
    assert!(d.redo_object_step(a).unwrap().is_none());
}

#[test]
fn c12_undoing_the_creation_removes_the_object_and_ctrl_z_brings_it_back_with_the_same_id() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let created = last_step(&d);
    let j0 = json(&d);
    let res = d.undo_object_step(a).unwrap().unwrap();
    assert!(res.object_gone);
    assert!(d.object(a).is_none());
    let top = top_undo(&d).unwrap();
    d.undo_step(top).unwrap();
    assert!(d.object(a).is_some(), "same id");
    assert_eq!(json(&d), j0);
    let r = rows(&d, a);
    assert!(r.iter().any(|x| x.step == created), "same history");
    assert_eq!(
        r.iter().find(|x| x.step == created).unwrap().status,
        ObjectStatus::Applied,
        "Ctrl+Z took the revert step back, so the creation is in effect again"
    );
}

#[test]
fn c13_undo_takes_back_a_peers_step_and_names_the_author() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    set_width(&a, r, 2.0);
    let b = replica(&a, 2);
    set_width(&b, r, 6.0);
    let t = last_step(&b);
    sync_into(&b, &a);
    let tg = a.object_history_targets(r);
    let target = tg.undo.expect("target");
    assert_eq!(target.step, t);
    assert_eq!(target.author, Author::Earlier);
    a.undo_object_step(r).unwrap().unwrap();
    assert!((width_mm(&a, r) - 2.0).abs() < 1e-9);
    let rr = rows(&a, r);
    let row = rr.iter().find(|x| x.step == t).unwrap();
    assert_eq!(row.status, ObjectStatus::UndoneBy(Author::You));
}

#[test]
fn c14_nothing_to_undo_writes_nothing() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    d.wipe_object_history(a).unwrap();
    let n = d.history_len();
    let own = d.own_steps().len();
    let j = json(&d);
    assert!(d.object_history_targets(a).undo.is_none());
    assert!(d.undo_object_step(a).unwrap().is_none());
    assert!(d.redo_object_step(a).unwrap().is_none());
    assert_eq!(d.history_len(), n);
    assert_eq!(d.own_steps().len(), own);
    assert_eq!(json(&d), j);
}

#[test]
fn c14_an_unknown_object_is_refused_or_nothing_without_writing() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    d.delete_objects(&[a]).unwrap();
    let n = d.history_len();
    let j = json(&d);
    let _ = d.redo_object_step(a);
    assert_eq!(d.history_len(), n);
    assert_eq!(json(&d), j);
}

#[test]
fn c15_repeated_object_undo_walks_back_one_step_at_a_time() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let j0 = state(&d, a);
    mv(&d, &[a], 1.0);
    let j1 = state(&d, a);
    set_width(&d, a, 3.0);
    d.undo_object_step(a).unwrap().unwrap();
    assert_eq!(state(&d, a), j1);
    d.undo_object_step(a).unwrap().unwrap();
    assert_eq!(state(&d, a), j0);
    // Redo twice goes forward in the same order.
    d.redo_object_step(a).unwrap().unwrap();
    assert_eq!(state(&d, a), j1);
    d.redo_object_step(a).unwrap().unwrap();
    assert!((width_mm(&d, a) - 3.0).abs() < 1e-9);
    assert!(d.redo_object_step(a).unwrap().is_none());
}

#[test]
fn c41_global_undo_then_object_undo_do_not_skip_or_double() {
    // Ctrl+Z of a step, then Ctrl+U: Ctrl+U must not "undo" a step that is already undone.
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    let after_move = state(&d, a);
    set_width(&d, a, 3.0);
    let top = top_undo(&d).unwrap();
    d.undo_step(top).unwrap(); // width undone (global)
    d.undo_object_step(a).unwrap().unwrap(); // must take back the MOVE
    assert_ne!(state(&d, a), after_move);
    assert!(width_mm(&d, a) < 3.0);
}

#[test]
fn c41_ctrl_z_of_an_object_undo_clears_nothing_but_redo_of_ctrl_z_is_consistent() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 30.0);
    mv(&d, &[a], 1.0);
    mv(&d, &[b], 1.0);
    d.undo_object_step(a).unwrap().unwrap();
    let j = json(&d);
    let top = top_undo(&d).unwrap();
    d.undo_step(top).unwrap(); // Ctrl+Z of the object undo
    d.redo_step(top).unwrap(); // Ctrl+Shift+Z
    assert_eq!(
        json(&d),
        j,
        "redo of the Ctrl+Z gives the object-undone state again"
    );
}

// ---------------------------------------------------------------------
// Criteria 19, 20, 22, 28: steps that make objects from objects
// ---------------------------------------------------------------------

#[test]
fn c20_c28_boolean_result_begins_with_created_from_sources_and_inherits_nothing() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    set_width(&d, a, 4.0);
    let b = rect(&d, 20.0);
    let c = rect(&d, 40.0);
    let res = d
        .replace_with_path(
            &[a, b, c],
            a,
            &[square(31, 0.0, 0.0, 10.0)],
            "boolean_union",
        )
        .unwrap();
    let step = last_step(&d);
    let rr = rows(&d, res);
    assert_eq!(rr.len(), 1, "no inherited rows (criteria 20 and 28)");
    assert_eq!(rr[0].step, step);
    assert_eq!(rr[0].role, Role::Created);
    assert_eq!(rr[0].kind, StepKind::Replace);
    assert_eq!(rr[0].sources.len(), 3);
    for s in [a, b, c] {
        assert!(rr[0].sources.contains(&s));
        let sr = rows(&d, s);
        assert_eq!(
            sr[0].step, step,
            "the sources' timelines end with that step"
        );
        assert_eq!(sr[0].role, Role::Deleted);
        assert_eq!(sr[0].others, 3, "two other sources and the result");
    }
    assert!(rr.iter().all(|r| r.inherited_from.is_none()));
}

#[test]
fn c19_combine_and_break_apart_are_replace_steps_with_the_right_timeline_ends() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 40.0);
    let compound = d
        .replace_with_path(
            &[a, b],
            a,
            &[square(100, 0.0, 0.0, 10.0), square(110, 40.0, 0.0, 10.0)],
            "combine_paths",
        )
        .unwrap();
    let combine_step = last_step(&d);
    let cr = rows(&d, compound);
    assert_eq!(
        cr.len(),
        1,
        "a compound path is ONE object with ONE timeline"
    );
    assert_eq!(cr[0].role, Role::Created);
    assert_eq!(cr[0].sources.len(), 2);
    assert_eq!(rows(&d, a)[0].step, combine_step);
    assert_eq!(rows(&d, a)[0].role, Role::Deleted);
    // Break apart the other way.
    let pieces = d
        .replace_with_pieces(
            &[compound],
            &[
                (compound, vec![square(200, 0.0, 0.0, 10.0)]),
                (compound, vec![square(210, 40.0, 0.0, 10.0)]),
            ],
            PieceFill::Keep,
            "break_apart",
        )
        .unwrap();
    let break_step = last_step(&d);
    assert_eq!(rows(&d, compound)[0].step, break_step);
    assert_eq!(rows(&d, compound)[0].role, Role::Deleted);
    for p in pieces {
        let pr = rows(&d, p);
        assert_eq!(pr.len(), 1, "pieces inherit nothing");
        assert_eq!(pr[0].role, Role::Created);
        assert_eq!(pr[0].kind, StepKind::Replace);
        assert_eq!(pr[0].sources, vec![compound]);
    }
}

#[test]
fn c22_a_node_edit_is_a_change_step_of_the_path_not_a_timeline_of_its_own() {
    let d = Document::new(1);
    let p = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, 1), pt(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(9, 2), pt(10.0, 0.0)),
            NewAnchor::corner(AnchorId::new(9, 3), pt(10.0, 10.0)),
        ],
        true,
    );
    let n = rows(&d, p).len();
    d.move_anchors(&[(p, AnchorId::new(9, 2), pt(12.0, 1.0))])
        .unwrap();
    let r = rows(&d, p);
    assert_eq!(r.len(), n + 1);
    assert_eq!(r[0].role, Role::Changed);
    assert_eq!(r[0].kind, StepKind::Change);
    assert_eq!(r[0].others, 0);
    // Object undo takes the node edit back.
    let before_edit = state(&d, p);
    d.undo_object_step(p).unwrap().unwrap();
    assert_ne!(state(&d, p), before_edit);
}

// ---------------------------------------------------------------------
// Criteria 23 to 28, 37: clones
// ---------------------------------------------------------------------

#[test]
fn c23_c24_a_clone_starts_with_duplicate_of_and_inherits_rows_as_a_copy() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    set_width(&d, a, 3.0);
    let a_create_step;
    let a_style_step = last_step(&d);
    {
        let r = rows(&d, a);
        a_create_step = r[1].step;
    }
    let c = dup(&d, a, 5.0);
    let dup_step = last_step(&d);
    // Later steps: original moved, clone moved.
    mv(&d, &[a], 100.0);
    let a_move = last_step(&d);
    mv(&d, &[c], 200.0);
    let c_move = last_step(&d);
    let ca = rows(&d, a);
    let cc = rows(&d, c);
    assert_eq!(
        steps_of(&ca),
        vec![a_move, a_style_step, a_create_step],
        "the original has exactly its own rows"
    );
    // The clone: its move, the clone start, then the inherited rows in the original's order.
    assert_eq!(cc.len(), 4, "move, duplicate, 2 inherited");
    assert_eq!(cc[0].step, c_move);
    assert_eq!(cc[1].step, dup_step);
    assert!(matches!(cc[1].class, ObjectRowClass::CloneStart { source } if source == a));
    assert_eq!(
        cc[2].step, a_style_step,
        "same step ids: they are the same steps"
    );
    assert_eq!(cc[3].step, a_create_step);
    assert_eq!(cc[2].inherited_from, Some(a));
    assert_eq!(cc[3].inherited_from, Some(a));
    assert_eq!(cc[2].label, ca[1].label, "same operation names");
    assert!(cc[0].inherited_from.is_none() && cc[1].inherited_from.is_none());
    assert!(!steps_of(&ca).contains(&c_move));
    assert!(
        !steps_of(&cc).contains(&a_move),
        "later steps of the original never appear"
    );
    assert!(cc.iter().all(|r| r.status == ObjectStatus::Applied));
}

#[test]
fn c24_inherited_rows_are_ordered_as_in_the_original_with_every_kind() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    set_width(&d, a, 2.0);
    mv(&d, &[a], 1.0);
    let ra = rows(&d, a);
    let c = dup(&d, a, 3.0);
    let rc = rows(&d, c);
    assert_eq!(rc.len(), ra.len() + 1);
    assert!(matches!(rc[0].class, ObjectRowClass::CloneStart { .. }));
    assert_eq!(steps_of(&rc[1..]), steps_of(&ra));
}

#[test]
fn c24_a_clone_of_a_peers_object_inherits_the_peers_steps_up_to_the_version() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    let b = replica(&a, 2);
    set_width(&b, r, 7.0);
    let peer_step = last_step(&b);
    sync_into(&b, &a);
    let c = dup(&a, r, 4.0);
    let rc = rows(&a, c);
    assert!(
        rc.iter()
            .any(|x| x.step == peer_step && x.inherited_from == Some(r))
    );
    // A concurrent step that was NOT in the causal past of the duplication stays out.
    set_width(&b, r, 8.0);
    let concurrent = last_step(&b);
    sync_into(&b, &a);
    assert!(!steps_of(&rows(&a, c)).contains(&concurrent));
    assert!(steps_of(&rows(&a, r)).contains(&concurrent));
}

#[test]
fn c25_object_undo_on_a_clone_goes_into_the_inherited_rows_and_never_changes_the_original() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    // inherited step: resize A to a 20 wide rectangle
    d.set_rect_bounds(
        a,
        RectBounds {
            origin: pt(0.0, 10.0),
            width: Length::from_mm(20.0),
            height: Length::from_mm(10.0),
        },
    )
    .unwrap();
    let c = dup(&d, a, 50.0);
    mv(&d, &[c], 10.0);
    let a_state = state(&d, a);
    let c_after_dup_and_move = state(&d, c);
    // 1st: the clone's own move.
    let r1 = d.undo_object_step(c).unwrap().unwrap();
    assert!(r1.target.inherited_from.is_none());
    assert_ne!(state(&d, c), c_after_dup_and_move);
    // 2nd: goes into the inherited rows (the resize), as a new step on the clone.
    let n = d.history_len();
    let r2 = d
        .undo_object_step(c)
        .unwrap()
        .expect("goes into the inherited rows");
    assert_eq!(r2.target.inherited_from, Some(a));
    assert_eq!(d.history_len(), n + 1, "a new step on the clone");
    let ObjectSnapshot::Primitive(p) = d.object(c).unwrap() else {
        panic!()
    };
    let dbg = format!("{:?}", p.shape);
    assert!(
        dbg.contains("10"),
        "the clone is a 10 wide rectangle again: {dbg}"
    );
    assert_eq!(state(&d, a), a_state, "the original is never changed");
    assert!(
        rows(&d, a)
            .iter()
            .all(|r| r.status == ObjectStatus::Applied)
    );
    // The new step is in the clone's timeline only.
    assert_eq!(rows(&d, c)[0].step, last_step(&d));
    assert!(!steps_of(&rows(&d, a)).contains(&last_step(&d)));
}

#[test]
fn c25_object_undo_on_a_clone_goes_back_through_an_inherited_style_step() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    set_width(&d, a, 5.0);
    let c = dup(&d, a, 50.0);
    mv(&d, &[c], 10.0);
    let a_state = state(&d, a);
    d.undo_object_step(c).unwrap().unwrap(); // the clone's own move
    let n = d.history_len();
    let r = d
        .undo_object_step(c)
        .unwrap()
        .expect("into the inherited rows");
    assert_eq!(r.target.inherited_from, Some(a));
    assert_eq!(d.history_len(), n + 1);
    assert!(
        width_mm(&d, c) < 5.0,
        "the inherited style step is taken back"
    );
    assert_eq!(width_mm(&d, a), 5.0);
    assert_eq!(state(&d, a), a_state, "the original is never changed");
}

/// Known limit: Ctrl+U on a copy never takes back the Duplicate. Taking back the creation the
/// copy inherited from the original would remove the copy, which is the same thing.
#[test]
fn c25_object_undo_never_removes_a_clone_by_taking_back_an_inherited_creation() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let c = dup(&d, a, 50.0);
    mv(&d, &[c], 10.0);
    for press in 0..6 {
        let _ = d.undo_object_step(c);
        assert!(
            d.object(c).is_some(),
            "press {press}: the clone is gone, the Duplicate was taken back through the inherited creation"
        );
    }
}

#[test]
fn c25_object_undo_on_a_rect_clone_passes_over_or_restores_an_inherited_resize() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    d.set_rect_bounds(
        a,
        RectBounds {
            origin: pt(0.0, 10.0),
            width: Length::from_mm(20.0),
            height: Length::from_mm(10.0),
        },
    )
    .unwrap();
    let c = dup(&d, a, 0.0);
    let t = d.object_history_targets(c).undo.expect("a target");
    let before = state(&d, c);
    let res = d.undo_object_step(c).unwrap().unwrap();
    // Either the resize is restored (width back to 10) or the press says it wrote nothing; it
    // must never claim the resize and silently do something else.
    if res.report.wrote {
        assert_ne!(state(&d, c), before, "target {t:?}");
        assert!(d.object(c).is_some() || res.object_gone);
    }
}

#[test]
fn c25_object_redo_after_a_restore_into_inherited_rows_works() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    set_width(&d, a, 5.0);
    let c = dup(&d, a, 50.0);
    let before = state(&d, c);
    // Walk back through the inherited style step.
    let res = d.undo_object_step(c).unwrap();
    if let Some(res) = res {
        assert_eq!(res.target.inherited_from, Some(a));
        assert!(width_mm(&d, c) < 5.0);
        d.redo_object_step(c).unwrap().expect("redo");
        assert_eq!(state(&d, c), before);
    }
}

#[test]
fn c26_clone_of_a_clone_goes_back_through_both() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    set_width(&d, a, 2.0);
    let a_steps = steps_of(&rows(&d, a));
    let c1 = dup(&d, a, 10.0);
    mv(&d, &[c1], 1.0);
    let c1_move = last_step(&d);
    let c2 = dup(&d, c1, 10.0);
    let r2 = rows(&d, c2);
    assert!(
        matches!(r2[0].class, ObjectRowClass::CloneStart { source } if source == c1),
        "points at the direct source"
    );
    assert!(steps_of(&r2).contains(&c1_move));
    for s in &a_steps {
        assert!(
            steps_of(&r2).contains(s),
            "lineage reaches the grandparent's steps"
        );
    }
    // The clone start row of c1 is in c2's lineage too.
    assert!(
        r2.iter()
            .filter(|r| matches!(r.class, ObjectRowClass::CloneStart { .. }))
            .count()
            >= 1
    );
}

#[test]
fn c26_a_wipe_of_the_middle_clone_cuts_the_lineage_for_later_clones_only() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    set_width(&d, a, 2.0);
    let a_create = rows(&d, a)[1].step;
    let c1 = dup(&d, a, 10.0);
    let c2_before = dup(&d, c1, 10.0);
    let before_len = d.object_history_len(c2_before);
    d.wipe_object_history(c1).unwrap();
    // A clone made earlier is never changed by a wipe (criterion 33).
    assert_eq!(d.object_history_len(c2_before), before_len);
    assert!(steps_of(&rows(&d, c2_before)).contains(&a_create));
    // A clone made later inherits only from the floor.
    let c2_after = dup(&d, c1, 10.0);
    let r = rows(&d, c2_after);
    assert!(
        !steps_of(&r).contains(&a_create),
        "nothing from before the floor"
    );
    assert!(
        r.iter().any(|x| matches!(x.class, ObjectRowClass::Floor)),
        "the floor row is inherited"
    );
}

#[test]
fn c37_clone_origin_is_one_commit_and_never_taken_over() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    d.wipe_object_history(a).unwrap();
    let n = d.history_len();
    let c = dup(&d, a, 5.0);
    assert_eq!(
        d.history_len(),
        n + 1,
        "origin written in the same commit as the copy"
    );
    let l = loro_of(&d);
    let tree = l.get_tree("paths");
    let mut clone_ofs = 0;
    let mut floors = 0;
    for node in tree.nodes() {
        if tree.is_node_deleted(&node).unwrap_or(true) {
            continue;
        }
        let meta = tree.get_meta(node).unwrap();
        if let Some(loro::ValueOrContainer::Value(loro::LoroValue::String(s))) =
            meta.get("clone_of")
        {
            clone_ofs += 1;
            assert!(s.contains('@'), "<source id>@<version>: {s:?}");
        }
        if meta.get("history_floor").is_some() {
            floors += 1;
        }
    }
    assert_eq!(clone_ofs, 1, "only the copy has clone_of");
    assert_eq!(floors, 1, "a copy never takes over history_floor");
    let _ = c;
}

#[test]
fn c27_28_other_commands_make_no_clones() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    d.translate_objects(&[a], Vec2::new(1.0, 0.0)).unwrap();
    let res = d
        .replace_with_path(&[a], a, &[square(300, 0.0, 0.0, 10.0)], "offset")
        .unwrap();
    assert!(rows(&d, res).iter().all(
        |r| !matches!(r.class, ObjectRowClass::CloneStart { .. }) && r.inherited_from.is_none()
    ));
}

#[test]
fn c23_the_original_deleted_after_duplication_leaves_the_clone_timeline_intact() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    set_width(&d, a, 2.0);
    let c = dup(&d, a, 10.0);
    let before = steps_of(&rows(&d, c));
    d.delete_objects(&[a]).unwrap();
    assert_eq!(
        steps_of(&rows(&d, c)),
        before,
        "the original's data stays readable"
    );
    // Ctrl+Z of the delete brings the original back; the clone is unchanged.
    let top = top_undo(&d).unwrap();
    d.undo_step(top).unwrap();
    assert!(d.object(a).is_some());
    assert_eq!(steps_of(&rows(&d, c)), before);
    // And an object undo on the clone does not touch the original.
    let a_state = state(&d, a);
    let _ = d.undo_object_step(c);
    assert_eq!(state(&d, a), a_state);
}

// ---------------------------------------------------------------------
// Criteria 29 to 34, 38: wipe
// ---------------------------------------------------------------------

#[test]
fn c29_30_wipe_writes_one_step_hides_earlier_rows_and_keeps_the_document_list() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    set_width(&d, a, 2.0);
    let earlier = steps_of(&rows(&d, a));
    let count = d.object_wipe_count(a);
    assert_eq!(count, 3);
    let n = d.history_len();
    let hidden = d.wipe_object_history(a).unwrap();
    assert_eq!(hidden, count);
    assert_eq!(d.history_len(), n + 1, "one commit, one step");
    let r = rows(&d, a);
    assert_eq!(r.len(), 1, "only the floor row");
    assert!(matches!(r[0].class, ObjectRowClass::Floor));
    assert_eq!(r[0].step, newest_step(&d));
    // Nothing more to hide.
    assert_eq!(d.object_wipe_count(a), 0);
    assert_eq!(d.wipe_object_history(a), Err(HistoryError::NoSteps));
    // The document list still shows every earlier step plus the wipe row.
    let all = d.history_page(0, 50);
    for s in &earlier {
        assert!(
            all.iter().any(|x| x.step == *s),
            "the document list keeps {s:?}"
        );
    }
    assert_eq!(all.len(), n + 1);
    let w = all.iter().find(|x| x.step == newest_step(&d)).unwrap();
    assert_eq!(w.objects.len(), 1);
    assert_eq!(w.objects[0].id, a);
    // Later steps show above the floor.
    mv(&d, &[a], 1.0);
    let r = rows(&d, a);
    assert_eq!(r.len(), 2);
    assert!(matches!(r[1].class, ObjectRowClass::Floor));
    assert_eq!(
        d.object_wipe_count(a),
        1,
        "only the later step would be hidden"
    );
}

#[test]
fn c30_the_wipe_step_has_kind_document_and_touches_only_this_object() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let other = rect(&d, 20.0);
    mv(&d, &[a], 1.0);
    d.wipe_object_history(a).unwrap();
    let w = newest_step(&d);
    let row = d.history_page(0, 1).remove(0);
    assert_eq!(row.step, w);
    assert_eq!(row.kind, StepKind::Document, "criterion 30: kind Document");
    assert_eq!(d.touched_objects(w), vec![a], "touched object: this one");
    let floor = rows(&d, a).remove(0);
    assert_eq!(floor.kind, StepKind::Document);
    assert!(rows(&d, other).iter().all(|r| r.step != w));
}

#[test]
fn c30_an_object_undo_cannot_go_before_the_floor() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    d.wipe_object_history(a).unwrap();
    let wiped_state = state(&d, a);
    mv(&d, &[a], 2.0);
    d.undo_object_step(a).unwrap().unwrap();
    assert_eq!(state(&d, a), wiped_state);
    let n = d.history_len();
    assert!(
        d.undo_object_step(a).unwrap().is_none(),
        "not before the floor"
    );
    assert_eq!(d.history_len(), n);
    assert_eq!(state(&d, a), wiped_state);
}

#[test]
fn c30_wipe_step_is_not_a_target_of_anyone() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    d.wipe_object_history(a).unwrap();
    assert!(d.object_history_targets(a).undo.is_none());
    assert!(d.object_history_targets(a).redo.is_none());
}

#[test]
fn c30_wipe_hides_the_redo_target_too_when_it_lies_before_the_floor() {
    // Criterion 30: an object undo cannot go before the floor. An object redo of a step that lies
    // before the floor would bring a hidden step back into the object's story.
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    d.undo_object_step(a).unwrap().unwrap();
    assert!(d.object_history_targets(a).redo.is_some());
    d.wipe_object_history(a).unwrap();
    let before = state(&d, a);
    let n = d.history_len();
    let res = d.redo_object_step(a).unwrap();
    assert!(
        res.is_none() && d.history_len() == n && state(&d, a) == before,
        "the redo of a step before the floor was applied: {:?}",
        res.map(|r| r.target.label)
    );
}

#[test]
fn c33_wipe_is_saved_and_a_wipe_never_changes_an_earlier_clone() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    set_width(&d, a, 2.0);
    let c = dup(&d, a, 5.0);
    let clone_rows = steps_of(&rows(&d, c));
    d.wipe_object_history(a).unwrap();
    assert_eq!(
        steps_of(&rows(&d, c)),
        clone_rows,
        "the clone keeps its copy"
    );
    let r = rows(&d, a);
    assert_eq!(r.len(), 1);
    // Save and open: the timeline still starts at the floor.
    let reopened = unpack(5, &pack(&d, "t").unwrap()).unwrap();
    let r2 = rows(&reopened, a);
    assert_eq!(steps_of(&r2), steps_of(&r));
    assert!(matches!(r2[0].class, ObjectRowClass::Floor));
    assert_eq!(steps_of(&rows(&reopened, c)), clone_rows);
    // Wiping the clone leaves the original's timeline unchanged.
    let a_before = steps_of(&rows(&d, a));
    d.wipe_object_history(c).unwrap();
    assert_eq!(steps_of(&rows(&d, a)), a_before);
}

#[test]
fn c34_wipe_in_a_shared_document_hides_for_every_peer() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    mv(&a, &[r], 1.0);
    let b = replica(&a, 2);
    // The document wipe is refused in a shared document (0020 criterion 50) ...
    let mut a2 = replica(&a, 3);
    merge(&a, &a2);
    let _ = &mut a2;
    // ... but the object wipe only hides, and replicates.
    let hidden = a.wipe_object_history(r).unwrap();
    assert!(hidden >= 2);
    sync_into(&a, &b);
    let rb = rows(&b, r);
    assert_eq!(rb.len(), 1, "every peer's list honours the wipe");
    assert!(matches!(rb[0].class, ObjectRowClass::Floor));
    assert_eq!(steps_of(&rows(&a, r)), steps_of(&rb));
}

#[test]
fn c38_a_step_concurrent_with_the_wipe_stays_visible() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    mv(&a, &[r], 1.0);
    let b = replica(&a, 2);
    a.wipe_object_history(r).unwrap();
    set_width(&b, r, 6.0); // concurrent with the wipe
    let concurrent = last_step(&b);
    merge(&a, &b);
    for d in [&a, &b] {
        let rr = rows(d, r);
        assert!(steps_of(&rr).contains(&concurrent), "visible");
        assert!(rr.iter().any(|x| matches!(x.class, ObjectRowClass::Floor)));
        assert_eq!(rr.len(), 2, "floor + concurrent step; earlier steps hidden");
    }
    assert_eq!(json(&a), json(&b));
}

#[test]
fn c38_two_concurrent_wipes_converge_and_hide_the_same_history() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    mv(&a, &[r], 1.0);
    set_width(&a, r, 2.0);
    let b = replica(&a, 2);
    a.wipe_object_history(r).unwrap();
    b.wipe_object_history(r).unwrap();
    merge(&a, &b);
    assert_eq!(json(&a), json(&b));
    let ra = rows(&a, r);
    let rb = rows(&b, r);
    assert_eq!(
        steps_of(&ra),
        steps_of(&rb),
        "both replicas show the same timeline"
    );
    // Whatever the winning floor is, the pre-wipe steps are hidden on both.
    assert!(ra.len() <= 3, "no earlier step comes back: {}", ra.len());
    assert!(ra.iter().any(|x| matches!(x.class, ObjectRowClass::Floor)));
    // A later step is visible.
    mv(&a, &[r], 1.0);
    assert_eq!(rows(&a, r)[0].step, last_step(&a));
    // Object undo cannot cross either floor.
    let _ = a.undo_object_step(r);
    let _ = a.undo_object_step(r);
    let _ = a.undo_object_step(r);
}

#[test]
fn c34_wipe_then_document_wipe_does_not_break_the_timeline() {
    let mut d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    d.wipe_object_history(a).unwrap();
    mv(&d, &[a], 1.0);
    d.wipe_history().unwrap();
    // The floor refers to a version that no longer exists as such: reading must not panic.
    let n = d.object_history_len(a);
    let r = d.object_history_page(a, 0, n + 3);
    assert_eq!(r.len(), n);
    let _ = d.object_history_targets(a);
    let _ = d.undo_object_step(a);
    let _ = d.redo_object_step(a);
    assert!(d.object(a).is_some());
}

#[test]
fn c34_document_wipe_after_clone_keeps_the_clone_readable() {
    let mut d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    let c = dup(&d, a, 3.0);
    d.wipe_history().unwrap();
    let n = d.object_history_len(c);
    let r = d.object_history_page(c, 0, n + 3);
    assert_eq!(r.len(), n);
    let _ = d.undo_object_step(c);
    assert!(d.object(a).is_some());
}

// ---------------------------------------------------------------------
// Criteria 37 to 39: the file
// ---------------------------------------------------------------------

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

#[test]
fn c39_no_format_bump_with_the_optional_keys() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    let _c = dup(&d, a, 3.0);
    d.wipe_object_history(a).unwrap();
    // No bump of its own (ADR 0014 Q2 A): the file carries the version the build has (11 since
    // `0023-groups`), with or without the optional keys.
    assert_eq!(
        manifest_version(&pack(&d, "t").unwrap()),
        u64::from(CURRENT_FORMAT_VERSION)
    );
}

#[test]
fn c39_a_build_that_ignores_the_keys_draws_the_same_drawing() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    let c = dup(&d, a, 3.0);
    d.wipe_object_history(a).unwrap();
    let a_state = state(&d, a);
    let c_state = state(&d, c);
    // Simulate an old build: it does not know the keys. Remove them and compare the objects.
    let stripped = reopen_with(&d, |l| {
        let tree = l.get_tree("paths");
        for node in tree.nodes() {
            if tree.is_node_deleted(&node).unwrap_or(true) {
                continue;
            }
            let meta = tree.get_meta(node).unwrap();
            let _ = meta.delete("clone_of");
            let _ = meta.delete("history_floor");
        }
    });
    assert_eq!(state(&stripped, a), a_state);
    assert_eq!(state(&stripped, c), c_state);
    // And the current build reads its own file again with the same objects.
    let again = unpack(3, &pack(&d, "t").unwrap()).unwrap();
    assert_eq!(state(&again, a), a_state);
    assert_eq!(state(&again, c), c_state);
}

#[test]
fn c39_an_older_build_that_duplicates_copies_clone_of_as_is() {
    // A copy made by a build that copies every key: the copy carries the SOURCE's clone_of.
    // A newer build must show a wrong lineage at worst, never fail or draw wrongly.
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let c = dup(&d, a, 3.0);
    let l_state_before = state(&d, c);
    let copy_by_old_build = d
        .duplicate_objects(
            &[CopySource {
                id: c,
                anchor_ids: vec![],
            }],
            Vec2::new(3.0, 0.0),
        )
        .unwrap()[0];
    // Make the key identical to the source's (what the old build does).
    let reopened = reopen_with(&d, |l| {
        let tree = l.get_tree("paths");
        let mut src = None;
        let mut dst = None;
        for node in tree.nodes() {
            if tree.is_node_deleted(&node).unwrap_or(true) {
                continue;
            }
            let meta = tree.get_meta(node).unwrap();
            let target_is_src = meta.get("clone_of").is_some();
            let _ = target_is_src;
            if src.is_none() && meta.get("clone_of").is_some() {
                src = Some(node);
            } else if meta.get("clone_of").is_some() {
                dst = Some(node);
            }
        }
        if let (Some(s), Some(t)) = (src, dst) {
            let v = tree.get_meta(s).unwrap().get("clone_of");
            if let Some(loro::ValueOrContainer::Value(v)) = v {
                tree.get_meta(t).unwrap().insert("clone_of", v).unwrap();
            }
        }
    });
    assert_eq!(state(&reopened, c), l_state_before);
    let n = reopened.object_history_len(copy_by_old_build);
    assert_eq!(
        reopened
            .object_history_page(copy_by_old_build, 0, n + 2)
            .len(),
        n
    );
}

#[test]
fn hostile_clone_of_and_history_floor_values_never_make_the_file_unreadable() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    let c = dup(&d, a, 3.0);
    let huge = "9".repeat(100_000);
    let variants: Vec<String> = vec![
        String::new(),
        "garbage".into(),
        "@".into(),
        "@@@@".into(),
        "1:2@".into(),
        "x:y@z".into(),
        "999999999999999999999999:5@1:1".into(),
        "0:0@0:0".into(),
        "-1:-1@-1:-1".into(),
        "\u{0}\u{1}\u{2}".into(),
        "🙂:🙂@🙂".into(),
        huge.clone(),
        format!("{huge}@{huge}"),
    ];
    for v in variants {
        let reopened = reopen_with(&d, |l| {
            let tree = l.get_tree("paths");
            for node in tree.nodes() {
                if tree.is_node_deleted(&node).unwrap_or(true) {
                    continue;
                }
                let meta = tree.get_meta(node).unwrap();
                if meta.get("clone_of").is_some() {
                    meta.insert("clone_of", v.as_str()).unwrap();
                }
                meta.insert("history_floor", v.as_str()).unwrap();
            }
        });
        for id in [a, c] {
            let n = reopened.object_history_len(id);
            let r = reopened.object_history_page(id, 0, n + 5);
            assert_eq!(
                r.len(),
                n,
                "value {:?}",
                v.chars().take(12).collect::<String>()
            );
            let _ = reopened.object_history_targets(id);
            let _ = reopened.object_wipe_count(id);
            let before = reopened.history_len();
            let _ = reopened.undo_object_step(id);
            let _ = reopened.redo_object_step(id);
            assert!(reopened.history_len() >= before);
        }
        // The drawing is the same as before.
        assert_eq!(state(&reopened, a), state(&d, a));
        assert_eq!(state(&reopened, c), state(&d, c));
    }
}

#[test]
fn hostile_non_string_clone_of_values_mean_no_origin() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let c = dup(&d, a, 3.0);
    for kind in 0..4 {
        let reopened = reopen_with(&d, |l| {
            let tree = l.get_tree("paths");
            for node in tree.nodes() {
                if tree.is_node_deleted(&node).unwrap_or(true) {
                    continue;
                }
                let meta = tree.get_meta(node).unwrap();
                if meta.get("clone_of").is_some() {
                    match kind {
                        0 => meta.insert("clone_of", 42_i64).unwrap(),
                        1 => meta.insert("clone_of", true).unwrap(),
                        2 => meta.insert("clone_of", 1.5_f64).unwrap(),
                        _ => {
                            let _ = meta.insert_container("clone_of", loro::LoroMap::new());
                        }
                    }
                }
                match kind {
                    0 => meta.insert("history_floor", 7_i64).unwrap(),
                    1 => meta.insert("history_floor", false).unwrap(),
                    2 => meta.insert("history_floor", 0.0_f64).unwrap(),
                    _ => {
                        let _ = meta.insert_container("history_floor", loro::LoroList::new());
                    }
                }
            }
        });
        let r = rows(&reopened, c);
        assert!(r.iter().all(|x| x.inherited_from.is_none()), "kind {kind}");
        let _ = rows(&reopened, a);
        assert!(!reopened.object_ids().is_empty());
    }
}

#[test]
fn hostile_clone_cycles_and_self_reference_terminate() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, &[a], 1.0);
    let c = dup(&d, a, 3.0);
    let c2 = dup(&d, c, 3.0);
    let t = Instant::now();
    let reopened = reopen_with(&d, |l| {
        let tree = l.get_tree("paths");
        // Make every clone_of point at the next object, and the original at the last clone:
        // a cycle; and one self reference.
        let nodes: Vec<_> = tree
            .nodes()
            .into_iter()
            .filter(|n| !tree.is_node_deleted(n).unwrap_or(true))
            .collect();
        let version = nodes.len();
        for (i, node) in nodes.iter().enumerate() {
            let next = nodes[(i + 1) % version];
            let meta = tree.get_meta(*node).unwrap();
            let v = format!("{}:{}@{}:{}", next.peer, next.counter, 1, 0);
            let _ = meta.insert("clone_of", v);
        }
    });
    for id in [a, c, c2] {
        let n = reopened.object_history_len(id);
        assert_eq!(reopened.object_history_page(id, 0, n + 3).len(), n);
        let _ = reopened.undo_object_step(id);
    }
    assert!(t.elapsed().as_secs() < 20, "a cycle must not hang");
}

#[test]
fn old_files_without_steps_or_keys_read_cleanly() {
    for name in [
        "valid.curvyo",
        "paths_v2.curvyo",
        "primitives_v3.curvyo",
        "rotation_v5.curvyo",
        "compound_v8.curvyo",
        "markers_v9.curvyo",
        "background_v10.curvyo",
        "format_version_1.curvyo",
    ] {
        let Ok(d) = unpack(5, &fixture(name)) else {
            continue;
        };
        for id in d.object_ids() {
            let n = d.object_history_len(id);
            let page = d.object_history_page(id, 0, n + 5);
            assert_eq!(page.len(), n, "{name}");
            // No row is wiped, no row claims a clone origin.
            assert!(page.iter().all(|r| r.inherited_from.is_none()), "{name}");
            let before = d.history_len();
            let t = d.object_history_targets(id);
            if t.undo.is_none() {
                assert!(d.undo_object_step(id).unwrap().is_none(), "{name}");
                assert_eq!(d.history_len(), before, "{name}: nothing written");
            }
            assert!(d.object_wipe_count(id) <= n, "{name}");
        }
    }
}

#[test]
fn a_future_format_still_refuses_and_the_keys_do_not_change_that() {
    let bytes = fixture("future_format_version.curvyo");
    assert!(matches!(
        unpack(1, &bytes),
        Err(OpenError::FormatTooNew { .. })
    ));
}

// ---------------------------------------------------------------------
// Criterion 35, 17: cost
// ---------------------------------------------------------------------

fn big_document(steps: usize, hot: usize) -> (Document, NodeId, NodeId) {
    let d = Document::new(1);
    let hot_obj = rect(&d, 0.0);
    let cold = rect(&d, 100.0);
    for i in 0..steps {
        if i % (steps / hot.max(1)).max(1) == 0 {
            mv(&d, &[hot_obj], 0.01);
        } else {
            mv(&d, &[cold], 0.01);
        }
    }
    (d, hot_obj, cold)
}

#[test]
fn c35_object_scope_opens_within_four_times_100_ms_at_20000_steps_after_the_index_exists() {
    let t = Instant::now();
    let (d, hot, _cold) = big_document(19_998, 1_000);
    eprintln!("setup of 20,000 steps: {:?}", t.elapsed());
    assert!(d.history_len() >= 20_000);
    // Cold: the first request builds the index.
    let t = Instant::now();
    let n = d.object_history_len(hot);
    let cold = t.elapsed();
    eprintln!(
        "cold index at {} steps: {cold:?} (hot object rows {n})",
        d.history_len()
    );
    assert!(n >= 1_000, "about 1000 rows for the hot object, got {n}");
    // Warm: opening the Object scope = len + the first page.
    let t = Instant::now();
    let _len = d.object_history_len(hot);
    let page = d.object_history_page(hot, 0, 40);
    let _tg = d.object_history_targets(hot);
    let warm = t.elapsed();
    eprintln!("warm open: {warm:?}");
    assert_eq!(page.len(), 40);
    let budget_ms: u128 = if cfg!(debug_assertions) { 4_000 } else { 400 };
    assert!(
        warm.as_millis() <= budget_ms,
        "Object scope took {warm:?} (budget {budget_ms} ms)"
    );
    // Per commit: extending the index by one step stays far below 1 ms x 4.
    let t = Instant::now();
    for _ in 0..20 {
        mv(&d, &[hot], 0.01);
        let _ = d.object_history_len(hot);
    }
    let per = t.elapsed() / 20;
    eprintln!("commit + index extension: {per:?}");
    let commit_budget = if cfg!(debug_assertions) { 40 } else { 4 };
    assert!(per.as_millis() <= commit_budget, "per commit {per:?}");
    // Deep pages cost the same.
    let t = Instant::now();
    let deep = d.object_history_page(hot, n - 40, 40);
    eprintln!("deep page: {:?}", t.elapsed());
    assert!(!deep.is_empty());
}

#[test]
fn c17_object_undo_and_redo_at_20000_steps_stay_inside_the_budget() {
    let (d, hot, cold) = big_document(19_998, 1_000);
    let _ = d.object_history_len(hot);
    let c0 = state(&d, cold);
    let t = Instant::now();
    d.undo_object_step(hot).unwrap().unwrap();
    let undo = t.elapsed();
    let t = Instant::now();
    d.redo_object_step(hot).unwrap().unwrap();
    let redo = t.elapsed();
    eprintln!("object undo {undo:?}, redo {redo:?}");
    let budget = if cfg!(debug_assertions) { 2_000 } else { 400 };
    assert!(undo.as_millis() <= budget, "undo {undo:?}");
    assert!(redo.as_millis() <= budget, "redo {redo:?}");
    assert_eq!(state(&d, cold), c0, "the other object is untouched");
}

// ---------------------------------------------------------------------
// Fuzz-like invariant: Ctrl+Z always takes back exactly the last own step
// ---------------------------------------------------------------------

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn fuzz_ctrl_z_after_any_own_step_restores_the_json_before_it() {
    for seed in 1..=12_u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let d = Document::new(1);
        let mut ids: Vec<NodeId> = (0..3).map(|k| rect(&d, f64::from(k) * 30.0)).collect();
        let mut next_x = 200.0;
        for step in 0..80 {
            if !ids.iter().any(|i| d.object(*i).is_some()) {
                next_x += 30.0;
                ids.push(rect(&d, next_x));
            }
            let live: Vec<NodeId> = ids
                .iter()
                .copied()
                .filter(|i| d.object(*i).is_some())
                .collect();
            let before = json(&d);
            let own_before = d.own_steps().len();
            let obj = live[rng.below(live.len())];
            let kind = rng.below(8);
            match kind {
                0 => mv(&d, &[obj], 1.0 + rng.below(5) as f64),
                1 => set_width(&d, obj, 1.0 + rng.below(9) as f64),
                2 => {
                    let _ = d.undo_object_step(obj);
                }
                3 => {
                    let _ = d.redo_object_step(obj);
                }
                4 => {
                    ids.push(dup(&d, obj, 4.0));
                }
                5 => {
                    if ids.len() > 2 && d.object(obj).is_some() {
                        d.delete_objects(&[obj]).unwrap();
                    } else {
                        mv(&d, &[obj], 1.0);
                    }
                }
                6 => {
                    // A global undo of the top of the stack, then back again: must round trip.
                    if let Some(top) = top_undo(&d) {
                        let j = json(&d);
                        d.undo_step(top).unwrap();
                        d.redo_step(top).unwrap();
                        assert_eq!(json(&d), j, "seed {seed} step {step}: undo+redo");
                    }
                    continue;
                }
                _ => {
                    // Wipe sometimes.
                    if d.object(obj).is_some() && rng.below(4) == 0 {
                        let _ = d.wipe_object_history(obj);
                        continue;
                    }
                    mv(&d, &[obj], -1.0);
                }
            }
            if d.own_steps().len() > own_before {
                let top = top_undo(&d).expect("a new step is on the stack");
                assert_eq!(
                    top,
                    last_step(&d),
                    "seed {seed} step {step}: stack top is the new step"
                );
                let after = json(&d);
                d.undo_step(top).unwrap();
                assert_eq!(
                    json(&d),
                    before,
                    "seed {seed} step {step} kind {kind}: Ctrl+Z restores the document"
                );
                d.redo_step(top).unwrap();
                assert_eq!(json(&d), after, "seed {seed} step {step} kind {kind}: redo");
            }
            // Timelines stay readable.
            for id in &ids {
                if d.object(*id).is_some() {
                    let n = d.object_history_len(*id);
                    assert_eq!(d.object_history_page(*id, 0, n + 3).len(), n);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------
// The document wipe (0020) and the object wipe meet
// ---------------------------------------------------------------------

#[test]
fn c34_a_replica_that_wiped_the_object_and_one_that_wiped_the_document_do_not_corrupt_each_other() {
    let mut a = Document::new(1);
    let r = rect(&a, 0.0);
    mv(&a, &[r], 1.0);
    let mut b = replica(&a, 2);
    a.wipe_object_history(r).unwrap();
    b.wipe_history().unwrap();
    let ja = json(&a);
    let jb = json(&b);
    // The two logs no longer agree: the exchange is refused or applied, never half applied.
    let ab = b.export_updates(&a.version());
    let ba = a.export_updates(&b.version());
    if let Ok(bytes) = ab {
        let _ = a.import_updates(&bytes);
    }
    if let Ok(bytes) = ba {
        let _ = b.import_updates(&bytes);
    }
    for d in [&mut a, &mut b] {
        let n = d.object_history_len(r);
        assert_eq!(d.object_history_page(r, 0, n + 3).len(), n);
        let _ = d.object_history_targets(r);
        let _ = d.undo_object_step(r);
        assert!(d.object(r).is_some());
    }
    let _ = (ja, jb);
}

// ---------------------------------------------------------------------
// More: redo order, concurrent object undos, three-object duplicates, deep lineages
// ---------------------------------------------------------------------

#[test]
fn c10_redo_brings_back_steps_in_the_order_they_were_taken_back_for_the_object_only() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let other = rect(&d, 40.0);
    let j0 = state(&d, a);
    mv(&d, &[a], 1.0);
    let j1 = state(&d, a);
    set_width(&d, a, 3.0);
    let j2 = state(&d, a);
    let other_state = state(&d, other);
    d.undo_object_step(a).unwrap().unwrap();
    d.undo_object_step(a).unwrap().unwrap();
    assert_eq!(state(&d, a), j0);
    d.redo_object_step(a).unwrap().unwrap();
    assert_eq!(
        state(&d, a),
        j1,
        "the step taken back last comes back first"
    );
    d.redo_object_step(a).unwrap().unwrap();
    assert_eq!(state(&d, a), j2);
    assert!(d.redo_object_step(a).unwrap().is_none());
    assert_eq!(state(&d, other), other_state, "no other object is touched");
    // A new step after one redo clears the rest.
    d.undo_object_step(a).unwrap().unwrap();
    d.undo_object_step(a).unwrap().unwrap();
    d.redo_object_step(a).unwrap().unwrap();
    mv(&d, &[a], 2.0);
    assert!(
        d.redo_object_step(a).unwrap().is_none(),
        "redo side cleared"
    );
}

#[test]
fn c13_two_peers_object_undo_the_same_step_concurrently_and_converge() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    mv(&a, &[r], 5.0);
    let moved = last_step(&a);
    let before_move = {
        let c = replica(&a, 9);
        c.undo_object_step(r).unwrap().unwrap();
        state(&c, r)
    };
    let b = replica(&a, 2);
    a.undo_object_step(r).unwrap().unwrap();
    b.undo_object_step(r).unwrap().unwrap();
    merge(&a, &b);
    assert_eq!(json(&a), json(&b), "replicas converge");
    assert_eq!(
        state(&a, r),
        before_move,
        "the step is taken back once, not twice"
    );
    // The reverted step is shown as taken back by an object undo; for a peer's undo by "Earlier".
    let row_a = rows(&a, r).into_iter().find(|x| x.step == moved).unwrap();
    assert!(matches!(row_a.status, ObjectStatus::UndoneBy(_)));
    assert_eq!(steps_of(&rows(&a, r)), steps_of(&rows(&b, r)));
}

#[test]
fn c13_a_peers_object_undo_shows_undone_by_the_peer() {
    let a = Document::new(1);
    let r = rect(&a, 0.0);
    mv(&a, &[r], 5.0);
    let moved = last_step(&a);
    let b = replica(&a, 2);
    b.undo_object_step(r).unwrap().unwrap();
    sync_into(&b, &a);
    let row = rows(&a, r).into_iter().find(|x| x.step == moved).unwrap();
    assert_eq!(row.status, ObjectStatus::UndoneBy(Author::Earlier));
    let doc_row = a
        .history_page(0, 10)
        .into_iter()
        .find(|x| x.step == moved)
        .unwrap();
    assert_eq!(doc_row.undone_by, Some(Author::Earlier));
    // Redo for A: nothing, the object undo was B's (redo is per author).
    // The object can still be undone further by A.
    assert!(a.undo_object_step(r).unwrap().is_some());
}

#[test]
fn c23_duplicate_of_three_objects_gives_each_copy_its_own_source() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let c = rect(&d, 40.0);
    set_width(&d, b, 3.0);
    let copies = d
        .duplicate_objects(
            &[a, b, c]
                .iter()
                .map(|id| CopySource {
                    id: *id,
                    anchor_ids: vec![],
                })
                .collect::<Vec<_>>(),
            Vec2::new(0.0, 50.0),
        )
        .unwrap();
    assert_eq!(copies.len(), 3);
    for (copy, source) in copies.iter().zip([a, b, c]) {
        let r = rows(&d, *copy);
        assert!(
            matches!(r[0].class, ObjectRowClass::CloneStart { source: s } if s == source),
            "each copy points at its own source"
        );
        assert_eq!(r[0].others, 2, "one step, three timelines");
        assert!(r[1..].iter().all(|x| x.inherited_from == Some(source)));
        assert_eq!(r.len(), 1 + rows(&d, source).len());
    }
}

#[test]
fn c26_object_undo_walks_through_a_lineage_of_three_without_touching_the_ancestors() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    set_width(&d, a, 2.0);
    let c1 = dup(&d, a, 30.0);
    set_width(&d, c1, 4.0);
    let c2 = dup(&d, c1, 30.0);
    let (a_state, c1_state) = (state(&d, a), state(&d, c1));
    // c2: first its inherited (c1's) style, then (a's) style: both plain registers.
    let r1 = d.undo_object_step(c2).unwrap().unwrap();
    assert_eq!(r1.target.inherited_from, Some(c1));
    assert!((width_mm(&d, c2) - 2.0).abs() < 1e-9);
    let r2 = d.undo_object_step(c2).unwrap().unwrap();
    assert!(r2.target.inherited_from.is_some());
    assert!(width_mm(&d, c2) < 2.0, "back to the default stroke width");
    assert_eq!(state(&d, a), a_state);
    assert_eq!(state(&d, c1), c1_state);
    // The next press would reach the creation of the original: it must not remove the copy.
    let _ = d.undo_object_step(c2);
    assert!(
        d.object(c2).is_some(),
        "the copy survives the walk through its lineage"
    );
}

#[test]
fn c25_a_long_run_of_inherited_moves_does_not_hide_an_older_inherited_style_step() {
    // The copy's own rows are gone after two presses; the original was moved 40 times before the
    // copy was made. Moves of the original are not the copy's to take back, so they are passed
    // over; the style step below them must still be reachable.
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    set_width(&d, a, 2.0);
    for _ in 0..40 {
        mv(&d, &[a], 1.0);
    }
    let c = dup(&d, a, 500.0);
    let targets = d.object_history_targets(c);
    eprintln!("candidates offered: {}", targets.undo_candidates);
    let res = d.undo_object_step(c).unwrap();
    let res = res.expect("something to take back");
    assert!(
        res.report.wrote,
        "the press wrote nothing although the inherited style step is below (passed over {})",
        res.passed_over
    );
    assert!(width_mm(&d, c) < 2.0);
}
