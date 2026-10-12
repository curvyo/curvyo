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
