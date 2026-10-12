//! Previews read from the live document (`specs/0042-history-branches` criteria 20, 30 and 32):
//! the same version on every replica, plain copies that stay as they were while the live document
//! merges a peer's step underneath.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{Document, Length, Point, RectBounds, StyleEdit, pack, unpack};

fn rect(d: &Document) -> curvyo_document_core::NodeId {
    d.create_rect(RectBounds {
        origin: Point::new(10.0, 10.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

#[test]
fn c30_a_preview_of_one_row_is_equal_on_two_replicas() {
    let a = Document::new(1);
    let r = rect(&a);
    a.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(2.0)))
        .unwrap();
    let step = a.own_steps().last().unwrap().step;
    a.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(5.0)))
        .unwrap();
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    assert_eq!(
        a.preview_object_at(step, r).unwrap(),
        b.preview_object_at(step, r).unwrap()
    );
    assert_eq!(
        a.preview_document_at(step).unwrap(),
        b.preview_document_at(step).unwrap()
    );
}

#[test]
fn c20_c32_a_peers_step_merges_while_a_preview_is_held_and_the_preview_stays() {
    let a = Document::new(1);
    let r = rect(&a);
    let first = a.own_steps().last().unwrap().step;
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    let version = a.version();
    let held = a.preview_document_at(first).unwrap();
    assert_eq!(a.version(), version, "the preview wrote nothing");
    let copy = held.clone();
    b.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(7.0)))
        .unwrap();
    a.import_updates(&b.export_updates(&a.version()).unwrap())
        .unwrap();
    assert_eq!(held, copy, "the preview did not change");
    assert_eq!(a.history_len(), 2, "the live document merged the step");
    a.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(9.0)))
        .unwrap();
    assert_eq!(a.history_len(), 3, "the live document stays editable");
}
