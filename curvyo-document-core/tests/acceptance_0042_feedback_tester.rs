//! Tester acceptance tests for the customer feedback of 2026-10-14 on
//! `specs/0042-history-branches/specification.md`, document level: applying a version in the
//! Document scope (criterion 10) with `Document::go_to_version(step, None, None)`.
//!
//! Written from the specification text before the implementation diff was read.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, missing_docs, clippy::similar_names)]
#![allow(clippy::many_single_char_names, clippy::bool_assert_comparison)]

use curvyo_document_core::{
    BackgroundPaint, CURRENT_FORMAT_VERSION, Document, DocumentBackground, DocumentSize,
    HistoryError, LABEL_GO_TO_VERSION, Length, MovePlan, MoveTarget, NodeId, ObjectSnapshot, Point,
    RectBounds, StepRef, StyleEdit, Vec2, pack, unpack,
};

fn rect(d: &Document, x: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: Point::new(x, 10.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn json(d: &Document) -> Vec<u8> {
    d.export_json().unwrap()
}

fn body(d: &Document, id: NodeId) -> String {
    match d.object(id) {
        Some(ObjectSnapshot::Primitive(p)) => {
            format!("{:?}|{:?}|{:?}", p.shape, p.style, p.rotation)
        }
        Some(ObjectSnapshot::Path(p)) => format!("{p:?}"),
        None => "gone".to_owned(),
    }
}

fn last_step(d: &Document) -> StepRef {
    d.own_steps().last().unwrap().step
}

fn width(d: &Document, id: NodeId, mm: f64) {
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(mm)))
        .unwrap();
}

fn mv(d: &Document, id: NodeId, dx: f64) {
    d.translate_objects(&[id], Vec2::new(dx, 0.0)).unwrap();
}

fn rows(d: &Document) -> Vec<curvyo_document_core::HistoryRow> {
    d.history_page(0, d.history_len() + 5)
}

fn into(d: &Document, nodes: &[NodeId], target: MoveTarget) {
    d.move_nodes(&MovePlan {
        nodes: nodes.to_vec(),
        target,
    })
    .unwrap();
}

/// Draw -> s1 (a). Then: draw b, delete a, move nothing else. Apply s1: b is removed, a is back
/// under its own id; Ctrl+Z restores the exact earlier document.
#[test]
fn c10_document_apply_removes_created_revives_deleted_and_undo_is_exact() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let s1 = last_step(&d);
    let at_s1 = json(&d);
    let a_body = body(&d, a);
    let b = rect(&d, 40.0);
    d.delete_objects(&[a]).unwrap();
    assert_eq!(d.children(None), vec![b]);
    let before = json(&d);
    let n = d.history_len();

    let r = d.go_to_version(s1, None, None).unwrap();
    let go = r.step.expect("the document differs from s1: a step");
    assert_eq!(d.history_len(), n + 1, "exactly one new row");
    assert_eq!(rows(&d)[0].label, LABEL_GO_TO_VERSION);
    assert!(!d.contains_node(b), "created since: removed");
    assert_eq!(
        d.children(None),
        vec![a],
        "deleted since: back with its own id"
    );
    assert_eq!(body(&d, a), a_body);
    // The visible document equals s1's document (children and bodies; tombstones may differ).
    assert_eq!(json(&d), at_s1, "document.json equals s1");

    d.undo_step(go).unwrap();
    assert_eq!(json(&d), before, "Ctrl+Z restores exactly");
    assert_eq!(d.children(None), vec![b]);
}

#[test]
fn c10_document_apply_header_names_no_object_but_names_the_version() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let s1 = last_step(&d);
    mv(&d, a, 5.0);
    d.go_to_version(s1, None, None).unwrap();
    let top = &rows(&d)[0];
    assert_eq!(top.label, LABEL_GO_TO_VERSION);
    assert_eq!(top.version.as_ref().unwrap().step, s1);
}

#[test]
fn c10_apply_of_the_current_version_writes_nothing() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, a, 5.0);
    let newest = last_step(&d);
    let (n, v, j) = (d.history_len(), d.version(), json(&d));
    let r = d.go_to_version(newest, None, None).unwrap();
    assert_eq!(r.step, None);
    assert_eq!((d.history_len(), d.version(), json(&d)), (n, v, j));
    // After an apply the document matches s1, so a second apply of s1 writes nothing either.
    let s_old = d.own_steps()[0].step;
    d.go_to_version(s_old, None, None).unwrap();
    let n = d.history_len();
    assert_eq!(d.go_to_version(s_old, None, None).unwrap().step, None);
    assert_eq!(d.history_len(), n);
}

#[test]
fn c10_three_applies_are_three_rows_and_three_undos_take_them_back() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let s1 = last_step(&d);
    mv(&d, a, 5.0);
    let s2 = last_step(&d);
    width(&d, a, 3.0);
    let s3 = last_step(&d);
    let start = json(&d);
    let n = d.history_len();
    let mut gos = vec![];
    for s in [s1, s3, s2] {
        gos.push(d.go_to_version(s, None, None).unwrap().step.unwrap());
    }
    assert_eq!(d.history_len(), n + 3, "no apply continues another");
    for go in gos.iter().rev() {
        d.undo_step(*go).unwrap();
    }
    assert_eq!(json(&d), start);
}

#[test]
fn c10_the_rows_after_the_version_keep_their_state_and_the_next_step_forks() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let s1 = last_step(&d);
    mv(&d, a, 5.0);
    let s2 = last_step(&d);
    let before: Vec<_> = rows(&d).iter().map(|r| (r.step, r.status)).collect();
    d.go_to_version(s1, None, None).unwrap();
    let after = rows(&d);
    for (step, status) in &before {
        let now = after.iter().find(|r| r.step == *step).unwrap();
        assert_eq!(now.status, *status, "row {step:?} keeps its state");
    }
    assert!(after.iter().any(|r| r.step == s2));
}

#[test]
fn c10_document_apply_returns_group_parent_order_and_layer_flags() {
    let d = Document::new(1);
    let layer = d.new_layer(None);
    let (a, b, c) = (rect(&d, 0.0), rect(&d, 20.0), rect(&d, 40.0));
    into(&d, &[a, b, c], MoveTarget::FrontOf(Some(layer)));
    let g = d.group_objects(&[a, b]).unwrap();
    d.rename_node(layer, "Cut").unwrap();
    let s1 = last_step(&d);
    let at_s1 = json(&d);
    let order_layer = d.children(Some(layer));
    let order_group = d.children(Some(g));
    // Since: ungroup, reorder, lock + hide the layer, rename, and add a second layer.
    d.ungroup(&[g]).unwrap();
    into(&d, &[c], MoveTarget::BackOf(Some(layer)));
    d.set_node_visible(&[layer], false).unwrap();
    d.set_node_locked(&[layer], true).unwrap();
    d.set_node_locked(&[layer], false).unwrap();
    d.rename_node(layer, "Other").unwrap();
    let extra = d.new_layer(None);
    let before = json(&d);
    let r = d.go_to_version(s1, None, None).unwrap();
    let go = r.step.unwrap();
    assert!(d.contains_node(g), "the group is back with its own id");
    assert_eq!(d.parent_group(a), Some(g));
    assert_eq!(d.parent_group(b), Some(g));
    assert_eq!(d.children(Some(layer)), order_layer, "order in the layer");
    assert_eq!(d.children(Some(g)), order_group, "order in the group");
    assert!(!d.contains_node(extra), "the layer created since is gone");
    assert_eq!(
        String::from_utf8(json(&d)).unwrap().contains("Other"),
        false,
        "layer name back"
    );
    assert_eq!(
        json(&d).len(),
        at_s1.len(),
        "same shape as at s1 (flags back)"
    );
    d.undo_step(go).unwrap();
    assert_eq!(json(&d), before);
}

#[test]
fn c10_document_apply_brings_back_a_deleted_layer_with_its_content_and_ids() {
    let d = Document::new(1);
    let layer = d.new_layer(None);
    let a = rect(&d, 0.0);
    into(&d, &[a], MoveTarget::FrontOf(Some(layer)));
    let s1 = last_step(&d);
    d.delete_layer(layer).unwrap();
    assert!(!d.contains_node(layer));
    d.go_to_version(s1, None, None).unwrap();
    assert!(d.contains_node(layer));
    assert_eq!(d.children(Some(layer)), vec![a]);
}

#[test]
fn c10_document_settings_go_back_but_the_wipe_mark_stays() {
    let mut d = Document::new(1);
    let a = rect(&d, 0.0);
    mv(&d, a, 1.0);
    d.wipe_history().unwrap();
    let epoch = d.history_epoch();
    assert!(epoch.is_some());
    // After the wipe: s1 is the first step with the wipe in place.
    let a2 = rect(&d, 40.0);
    let s1 = last_step(&d);
    let size0 = d.size();
    let bg0 = d.background();
    d.resize(DocumentSize::new(
        Length::from_mm(size0.width.as_mm() + 100.0),
        Length::from_mm(size0.height.as_mm() + 50.0),
    ))
    .unwrap();
    let _ = d.set_background(DocumentBackground {
        paint: BackgroundPaint::None,
        ..bg0
    });
    mv(&d, a2, 3.0);
    let before = json(&d);
    let go = d.go_to_version(s1, None, None).unwrap().step.unwrap();
    assert_eq!(d.size(), size0, "page size back");
    assert_eq!(d.background(), bg0, "background back");
    assert_eq!(d.history_epoch(), epoch, "the wipe mark stays");
    d.undo_step(go).unwrap();
    assert_eq!(json(&d), before);
    assert_eq!(d.history_epoch(), epoch);
}

#[test]
fn c10_a_wiped_version_cannot_be_applied_and_a_bogus_one_is_refused_without_a_write() {
    let mut d = Document::new(1);
    let a = rect(&d, 0.0);
    let old = last_step(&d);
    mv(&d, a, 1.0);
    d.wipe_history().unwrap();
    mv(&d, a, 1.0);
    let v = d.version();
    assert!(d.go_to_version(old, None, None).is_err());
    assert_eq!(
        d.go_to_version(StepRef::new(999, 1), None, None)
            .unwrap_err(),
        HistoryError::NoSuchStep
    );
    assert_eq!(d.version(), v, "nothing written");
}

#[test]
fn c10_object_wipe_floor_and_clone_marks_stay_after_a_document_apply() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let s1 = last_step(&d);
    mv(&d, a, 1.0);
    mv(&d, a, 1.0);
    d.wipe_object_history(a).unwrap();
    mv(&d, a, 1.0);
    let before = d.object_history_page(a, 0, 100);
    let oldest = before.last().unwrap().step;
    d.go_to_version(s1, None, None).unwrap();
    let after = d.object_history_page(a, 0, 100);
    assert!(
        after.len() <= before.len() + 1,
        "the apply did not lift the object's wipe floor: {} rows before, {} after",
        before.len(),
        after.len()
    );
    assert_eq!(after.last().unwrap().step, oldest, "same oldest row");
}

#[test]
fn c15_document_apply_counts_other_peers_reverted_steps() {
    let a = Document::new(1);
    let o = rect(&a, 0.0);
    let s1 = last_step(&a);
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    mv(&b, o, 5.0);
    mv(&b, o, 5.0);
    let bytes = b.export_updates(&a.version()).unwrap();
    a.import_updates(&bytes).unwrap();
    let r = a.go_to_version(s1, None, None).unwrap();
    assert_eq!(r.others_reverted, 2);
}

#[test]
fn c10_save_and_open_after_an_apply_keep_format_12_and_the_state() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let s1 = last_step(&d);
    let b = rect(&d, 30.0);
    d.delete_objects(&[a]).unwrap();
    let go = d.go_to_version(s1, None, None).unwrap().step.unwrap();
    assert_eq!(CURRENT_FORMAT_VERSION, 12);
    let packed = pack(&d, "t").unwrap();
    let mut zip = zip_manifest_version(&packed);
    assert_eq!(zip.take().unwrap(), 12);
    let again = unpack(5, &packed).unwrap();
    assert_eq!(json(&again), json(&d));
    assert!(!again.contains_node(b));
    assert_eq!(again.children(None), vec![a]);
    // The apply step is a normal step after Open: it can be taken back.
    again.undo_step(go).unwrap();
    assert_eq!(again.children(None), vec![b]);
}

fn zip_manifest_version(packed: &[u8]) -> Option<u64> {
    use std::io::Read;
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(packed)).ok()?;
    let mut s = String::new();
    z.by_name("manifest.json")
        .ok()?
        .read_to_string(&mut s)
        .ok()?;
    let v: serde_json::Value = serde_json::from_str(&s).ok()?;
    v["format_version"].as_u64()
}

#[test]
fn c10_far_back_apply_reports_a_long_operation_distance() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let s1 = last_step(&d);
    for _ in 0..300 {
        mv(&d, a, 0.5);
    }
    assert!(d.step_distance(s1).unwrap() > 100);
    let before = json(&d);
    let go = d.go_to_version(s1, None, None).unwrap().step.unwrap();
    d.undo_step(go).unwrap();
    assert_eq!(json(&d), before);
}

/// Object scope: a deleted object is refused (implementer decision; it cannot be selected).
#[test]
fn c10_object_apply_to_a_deleted_object_is_refused_without_a_write() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let s1 = last_step(&d);
    mv(&d, a, 2.0);
    d.delete_objects(&[a]).unwrap();
    let v = d.version();
    assert_eq!(
        d.go_to_version(s1, Some(a), None).unwrap_err(),
        HistoryError::NoSuchObject
    );
    assert_eq!(d.version(), v);
}

#[test]
fn c10_object_apply_touches_only_that_object() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 30.0);
    let s1 = last_step(&d);
    let a1 = body(&d, a);
    mv(&d, a, 4.0);
    mv(&d, b, 4.0);
    let b_now = body(&d, b);
    let c = rect(&d, 60.0);
    d.go_to_version(s1, Some(a), None).unwrap();
    assert_eq!(body(&d, a), a1);
    assert_eq!(body(&d, b), b_now, "other objects keep their state");
    assert!(
        d.contains_node(c),
        "an object created since stays in the Object scope"
    );
}

/// Apply the first version, then the newest, then the middle one: each differs from the state
/// before it, so each must write a step (objects are removed and revived in turn).
#[test]
fn c10_apply_remove_revive_remove_again_each_writes_a_step() {
    let d = Document::new(1);
    let _a = rect(&d, 0.0);
    let s1 = last_step(&d);
    let b = rect(&d, 20.0);
    let s2 = last_step(&d);
    // Taken at s2, before c exists (it was taken after s3, which holds c).
    let at_s2 = json(&d);
    let c = rect(&d, 40.0);
    let s3 = last_step(&d);
    assert!(d.go_to_version(s1, None, None).unwrap().step.is_some());
    assert!(d.go_to_version(s3, None, None).unwrap().step.is_some());
    assert!(d.contains_node(b) && d.contains_node(c));
    let r = d.go_to_version(s2, None, None).unwrap();
    assert!(
        r.step.is_some(),
        "the document still has c, s2's version does not"
    );
    assert!(!d.contains_node(c));
    assert_eq!(json(&d), at_s2);
}

/// Narrower: apply s1 (removes b and c), then apply s2 directly (b must be back, c stays gone).
#[test]
fn c10_apply_s1_then_s2_brings_b_back() {
    let d = Document::new(1);
    let _a = rect(&d, 0.0);
    let s1 = last_step(&d);
    let b = rect(&d, 20.0);
    let s2 = last_step(&d);
    let c = rect(&d, 40.0);
    assert!(d.go_to_version(s1, None, None).unwrap().step.is_some());
    assert!(d.go_to_version(s2, None, None).unwrap().step.is_some());
    assert!(d.contains_node(b) && !d.contains_node(c));
}

/// Apply twice in a row from a state with a deleted-since object: s3 (c present), then s2.
#[test]
fn c10_apply_s3_while_c_exists_then_s2_removes_c() {
    let d = Document::new(1);
    let _a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let s2 = last_step(&d);
    let c = rect(&d, 40.0);
    let _s3 = last_step(&d);
    let r = d.go_to_version(s2, None, None).unwrap();
    assert!(r.step.is_some());
    assert!(d.contains_node(b) && !d.contains_node(c));
}
