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
fn c15_going_back_over_a_peers_later_steps_writes_them_anyway_and_counts_them() {
    let a = Document::new(1);
    let r = rect(&a);
    let s1 = width(&a, r, 1.0);
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    width(&b, r, 5.0);
    width(&b, r, 6.0);
    merge(&a, &b);
    let report = a.go_to_version(r, s1, None, false).unwrap();
    assert!(report.step.is_some());
    assert_eq!(report.others_reverted, 2);
    assert_eq!(stroke(&a, r), 1.0);
}

#[test]
fn c1_c10_a_new_step_after_undoing_a_go_to_forks_like_any_step() {
    let d = Document::new(1);
    let r = rect(&d);
    let s1 = width(&d, r, 1.0);
    width(&d, r, 2.0);
    let go = d.go_to_version(r, s1, None, false).unwrap().step.unwrap();
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
fn c31_c33_a_go_to_is_a_header_and_saves_like_any_step() {
    let d = Document::new(1);
    let r = rect(&d);
    let s1 = width(&d, r, 1.0);
    width(&d, r, 2.0);
    d.go_to_version(r, s1, None, false).unwrap();
    let reopened = unpack(3, &pack(&d, "t").unwrap()).unwrap();
    let row = reopened.history_page(0, 1).pop().unwrap();
    assert_eq!(row.label, "go_to_version");
    assert_eq!(row.version.unwrap().step, s1);
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
    // 11 since `0023-groups`; 0042 adds no bump (criterion 33).
    assert_eq!(manifest["format_version"], 11);
    assert_eq!(
        manifest["format_version"],
        curvyo_document_core::CURRENT_FORMAT_VERSION
    );
}
