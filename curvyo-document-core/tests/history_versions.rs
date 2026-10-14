//! Go to version and clone version (`specs/0042-history-branches` criteria 1, 10, 15, 22 to 24,
//! 31 and 33): steps with headers, written through the restore path, the same on every replica.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{
    Document, Length, NodeId, ObjectStatus, Point, RectBounds, StepRef, StyleEdit, pack, unpack,
};

fn rect(d: &Document) -> NodeId {
    d.create_rect(RectBounds {
        origin: Point::new(10.0, 10.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn width(d: &Document, id: NodeId, mm: f64) -> StepRef {
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(mm)))
        .unwrap();
    d.own_steps().last().unwrap().step
}

fn stroke(d: &Document, id: NodeId) -> f64 {
    match d.object(id).unwrap() {
        curvyo_document_core::ObjectSnapshot::Primitive(p) => p.style.stroke.width.as_mm(),
        curvyo_document_core::ObjectSnapshot::Path(p) => p.style.stroke.width.as_mm(),
    }
}

fn merge(a: &Document, b: &Document) {
    b.import_updates(&a.export_updates(&b.version()).unwrap())
        .unwrap();
    a.import_updates(&b.export_updates(&a.version()).unwrap())
        .unwrap();
}

#[test]
fn c15_applying_over_a_peers_later_steps_writes_them_anyway_and_counts_them() {
    let a = Document::new(1);
    let r = rect(&a);
    let s1 = width(&a, r, 1.0);
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    width(&b, r, 5.0);
    width(&b, r, 6.0);
    merge(&a, &b);
    let report = a.go_to_version(s1, Some(r), None).unwrap();
    assert!(report.step.is_some());
    assert_eq!(report.others_reverted, 2);
    assert_eq!(stroke(&a, r), 1.0);
    let document = a.go_to_version(s1, None, None).unwrap();
    assert_eq!(document.step, None, "the document already matches");
}

#[test]
fn c10_an_object_apply_is_one_step_that_ctrl_z_takes_back() {
    let d = Document::new(1);
    let r = rect(&d);
    let s1 = width(&d, r, 1.0);
    for mm in [2.0, 3.0, 4.0] {
        width(&d, r, mm);
    }
    let at_s4 = d.export_json().unwrap();
    let steps = d.own_step_count();
    let applied = d.go_to_version(s1, Some(r), None).unwrap().step.unwrap();
    assert_eq!(stroke(&d, r), 1.0, "s1's state");
    assert_eq!(d.own_step_count(), steps + 1);
    d.undo_step(applied).unwrap();
    assert_eq!(d.export_json().unwrap(), at_s4, "s4's state again");
}

#[test]
fn c10_an_object_apply_touches_no_other_object() {
    let d = Document::new(1);
    let r = rect(&d);
    let other = rect(&d);
    let s1 = width(&d, r, 1.0);
    width(&d, r, 2.0);
    width(&d, other, 9.0);
    d.go_to_version(s1, Some(r), None).unwrap();
    assert_eq!(stroke(&d, r), 1.0);
    assert_eq!(stroke(&d, other), 9.0);
}

#[test]
fn c10_a_document_apply_removes_drawn_objects_and_revives_deleted_ones_with_their_id() {
    let d = Document::new(1);
    let a = rect(&d);
    let v = width(&d, a, 1.0);
    let at_v = d.export_json().unwrap();
    let drawn = rect(&d);
    width(&d, a, 3.0);
    d.delete_objects(&[a]).unwrap();
    let report = d.go_to_version(v, None, None).unwrap();
    let step = report.step.unwrap();
    assert!(d.object(drawn).is_none(), "the drawn object is removed");
    assert_eq!(stroke(&d, a), 1.0, "the deleted object is back with its id");
    assert_eq!(d.export_json().unwrap(), at_v);
    d.undo_step(step).unwrap();
    assert!(d.object(a).is_none());
    assert!(d.object(drawn).is_some());
}

#[test]
fn c10_a_document_apply_puts_groups_order_and_settings_back() {
    let d = Document::new(1);
    let a = rect(&d);
    let b = rect(&d);
    let v = width(&d, a, 1.0);
    let at_v = d.export_json().unwrap();
    d.group_objects(&[a, b]).unwrap();
    d.resize(curvyo_document_core::DocumentSize::new(
        Length::from_mm(300.0),
        Length::from_mm(200.0),
    ))
    .unwrap();
    d.go_to_version(v, None, None).unwrap();
    assert_eq!(d.export_json().unwrap(), at_v, "no group, the old page");
}

#[test]
fn c10_every_apply_is_its_own_step_and_an_equal_version_writes_nothing() {
    let d = Document::new(1);
    let r = rect(&d);
    let s1 = width(&d, r, 1.0);
    let s2 = width(&d, r, 2.0);
    width(&d, r, 3.0);
    let rows = d.history_len();
    d.go_to_version(s1, Some(r), None).unwrap();
    d.go_to_version(s2, Some(r), None).unwrap();
    d.go_to_version(s1, Some(r), None).unwrap();
    assert_eq!(d.history_len(), rows + 3, "three applies, three rows");
    assert_eq!(d.go_to_version(s1, Some(r), None).unwrap().step, None);
    assert_eq!(d.history_len(), rows + 3);
}

#[test]
fn c1_c10_a_new_step_after_undoing_an_apply_forks_like_any_step() {
    let d = Document::new(1);
    let r = rect(&d);
    let s1 = width(&d, r, 1.0);
    width(&d, r, 2.0);
    let go = d.go_to_version(s1, Some(r), None).unwrap().step.unwrap();
    d.undo_step(go).unwrap();
    width(&d, r, 7.0);
    let branches = d.object_history_branches(r);
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].id, go);
    let rows = d.object_history_page(r, 0, d.object_history_len(r));
    let status = rows.iter().find(|row| row.step == go).unwrap().status;
    assert_eq!(status, ObjectStatus::OnBranch);
}

#[test]
fn c31_c33_an_apply_is_a_header_and_saves_like_any_step() {
    let d = Document::new(1);
    let r = rect(&d);
    let s1 = width(&d, r, 1.0);
    width(&d, r, 2.0);
    d.go_to_version(s1, Some(r), None).unwrap();
    width(&d, r, 5.0);
    d.go_to_version(s1, None, None).unwrap();
    let reopened = unpack(3, &pack(&d, "t").unwrap()).unwrap();
    let rows = reopened.history_page(0, 3);
    for row in [&rows[0], &rows[2]] {
        assert_eq!(row.label, "go_to_version");
        assert_eq!(row.version.as_ref().unwrap().step, s1);
    }
    assert_eq!(reopened.export_json().unwrap(), d.export_json().unwrap());
}

fn minter(peer: u64) -> impl FnMut() -> curvyo_document_core::AnchorId {
    let mut next = 1_000;
    move || {
        next += 1;
        curvyo_document_core::AnchorId::new(peer, next)
    }
}

fn timeline(d: &Document, id: NodeId) -> Vec<(StepRef, Option<NodeId>)> {
    d.object_history_page(id, 0, d.object_history_len(id))
        .into_iter()
        .map(|row| (row.step, row.inherited_from))
        .collect()
}

#[test]
fn c22_a_clone_of_an_earlier_version_is_a_new_object_on_top_and_one_step() {
    let d = Document::new(1);
    let r = rect(&d);
    let s1 = width(&d, r, 1.0);
    for mm in [2.0, 3.0, 4.0] {
        width(&d, r, mm);
    }
    let steps = d.own_step_count();
    let clone = d.clone_version(r, s1, &mut minter(1)).unwrap();
    assert_ne!(clone, r);
    assert_eq!(stroke(&d, clone), 1.0, "the clone equals s1's state");
    assert_eq!(stroke(&d, r), 4.0, "the original does not change");
    assert_eq!(d.object_ids().last(), Some(&clone), "on top of the stack");
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    let row = d.history_page(0, 1).pop().unwrap();
    assert_eq!(row.label, "clone_version");
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert!(d.object(clone).is_none(), "Ctrl+Z removes the clone");
}

#[test]
fn c23_the_clone_inherits_the_rows_up_to_that_version_and_no_later_ones() {
    let d = Document::new(1);
    let r = rect(&d);
    let create = d.own_steps().last().unwrap().step;
    let s1 = width(&d, r, 1.0);
    width(&d, r, 2.0);
    let clone = d.clone_version(r, s1, &mut minter(1)).unwrap();
    let rows = d.object_history_page(clone, 0, d.object_history_len(clone));
    assert_eq!(rows[0].version.as_ref().map(|v| v.step), Some(s1));
    assert!(matches!(
        rows[0].class,
        curvyo_document_core::ObjectRowClass::CloneStart { .. }
    ));
    assert_eq!(
        timeline(&d, clone)[1..],
        [(s1, Some(r)), (create, Some(r))],
        "inherited up to s1, flagged from the original"
    );
}

#[test]
fn c23_a_clone_follows_the_path_that_led_to_its_version_not_the_other_branches() {
    let d = Document::new(1);
    let r = rect(&d);
    let create = d.own_steps().last().unwrap().step;
    let s1 = width(&d, r, 1.0);
    let s2 = width(&d, r, 2.0);
    let s3 = width(&d, r, 3.0);
    d.undo_step(s3).unwrap();
    d.undo_step(s2).unwrap();
    let s4 = width(&d, r, 4.0);
    let main = d.clone_version(r, s4, &mut minter(1)).unwrap();
    assert_eq!(
        timeline(&d, main)[1..],
        [(s4, Some(r)), (s1, Some(r)), (create, Some(r))],
        "the main line, without the branch"
    );
    let branch = d.clone_version(r, s3, &mut minter(1)).unwrap();
    assert_eq!(stroke(&d, branch), 3.0);
    assert_eq!(
        timeline(&d, branch)[1..],
        [
            (s3, Some(r)),
            (s2, Some(r)),
            (s1, Some(r)),
            (create, Some(r))
        ],
        "the branch's path"
    );
}

#[test]
fn c24_a_deleted_object_is_cloned_from_its_delete_row_after_save_and_open() {
    let d = Document::new(1);
    let r = rect(&d);
    d.set_rect_bounds(
        r,
        RectBounds {
            origin: Point::new(10.0, 10.0),
            width: Length::from_mm(40.0),
            height: Length::from_mm(25.0),
        },
    )
    .unwrap();
    let resized = d.object(r).unwrap();
    d.delete_objects(&[r]).unwrap();
    let delete = d.own_steps().last().unwrap().step;
    let reopened = unpack(4, &pack(&d, "t").unwrap()).unwrap();
    let clone = reopened.clone_version(r, delete, &mut minter(4)).unwrap();
    let curvyo_document_core::ObjectSnapshot::Primitive(got) = reopened.object(clone).unwrap()
    else {
        panic!("a rectangle");
    };
    let curvyo_document_core::ObjectSnapshot::Primitive(want) = resized else {
        panic!("a rectangle");
    };
    assert_eq!(got.shape, want.shape, "the resized geometry");
}

#[test]
fn c25_a_groups_version_is_not_cloned() {
    let d = Document::new(1);
    let a = rect(&d);
    let b = rect(&d);
    let group = d.group_objects(&[a, b]).unwrap();
    let step = d.own_steps().last().unwrap().step;
    assert_eq!(
        d.clone_version(group, step, &mut minter(1)),
        Err(curvyo_document_core::HistoryError::Group)
    );
}

#[test]
fn c33_only_a_clone_writes_clone_of_and_the_format_version_stays() {
    let d = Document::new(1);
    let r = rect(&d);
    let s1 = width(&d, r, 1.0);
    d.clone_version(r, s1, &mut minter(1)).unwrap();
    let bytes = pack(&d, "t").unwrap();
    let reopened = unpack(5, &bytes).unwrap();
    assert_eq!(reopened.export_json().unwrap(), d.export_json().unwrap());
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut manifest = String::new();
    std::io::Read::read_to_string(
        &mut archive.by_name("manifest.json").unwrap(),
        &mut manifest,
    )
    .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    // 12 since `0050` (layers); 0042 adds no bump (criterion 33).
    assert_eq!(manifest["format_version"], 12);
    assert_eq!(
        manifest["format_version"],
        curvyo_document_core::CURRENT_FORMAT_VERSION
    );
}

#[test]
fn c22_a_refused_clone_leaves_nothing_behind_for_the_next_commit() {
    let d = Document::new(1);
    let a = rect(&d);
    let b = rect(&d);
    let group = d.group_objects(&[a, b]).unwrap();
    let grouped = d.own_steps().last().unwrap().step;
    let version = d.version();
    let objects = d.object_ids().len();
    assert!(d.clone_version(group, grouped, &mut minter(1)).is_err());
    assert!(
        d.clone_version(a, StepRef::new(9, 0), &mut minter(1))
            .is_err()
    );
    assert_eq!(d.version(), version, "nothing committed");
    width(&d, a, 3.0);
    assert_eq!(
        d.object_ids().len(),
        objects,
        "no stray node in the next step"
    );
    let row = d.history_page(0, 1).pop().unwrap();
    assert_eq!(
        row.objects.len(),
        1,
        "the next step touched only the rectangle"
    );
}

#[test]
fn c10_undoing_a_remove_then_revive_then_remove_series_goes_back_to_the_start() {
    let d = Document::new(1);
    rect(&d);
    let s1 = d.own_steps().last().unwrap().step;
    rect(&d);
    let s2 = d.own_steps().last().unwrap().step;
    rect(&d);
    let s3 = d.own_steps().last().unwrap().step;
    let start = d.export_json().unwrap();
    let applies: Vec<StepRef> = [s1, s3, s2]
        .iter()
        .map(|v| d.go_to_version(*v, None, None).unwrap().step.unwrap())
        .collect();
    for step in applies.iter().rev() {
        d.undo_step(*step).unwrap();
    }
    assert_eq!(d.export_json().unwrap(), start);
}
