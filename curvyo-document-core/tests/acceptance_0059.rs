//! Independent specification 0059 checks for spatial page identity and transactions.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, DocumentSize, Point, RectBounds, Vec2, pack, unpack};

#[test]
fn ac1_current_document_starts_with_one_default_frame_and_roundtrips_without_reads_writing() {
    let d = Document::new(591);
    let pages = d.pages();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].origin, Point::new(0.0, 0.0));
    assert_eq!(pages[0].size, DocumentSize::from_mm(210.0, 297.0));
    let p = d.add_page();
    d.rename_page(&p.id, "Frühstück 🐈").unwrap();
    d.move_page(&p.id, Vec2::new(-500.0, -60.0), &[]).unwrap();
    d.resize_page(&p.id, DocumentSize::from_mm(80.0, 40.0), &[])
        .unwrap();
    d.reorder_page(&p.id, 0).unwrap();
    let reopened = unpack(592, &pack(&d, "acceptance").unwrap()).unwrap();
    let before = reopened.version();
    assert_eq!(reopened.pages(), d.pages());
    assert_eq!(reopened.pages(), d.pages());
    assert_eq!(reopened.version(), before);
}

#[test]
fn ac3_gap_wrap_and_reordered_last_frame_supply_dimensions() {
    let d = Document::new(593);
    let first = d.pages()[0].id.clone();
    d.resize_page(&first, DocumentSize::from_mm(100.0, 100.0), &[])
        .unwrap();
    let second = d.add_page();
    assert_eq!(second.origin, Point::new(120.0, 0.0));
    d.resize_page(&second.id, DocumentSize::from_mm(400.0, 100.0), &[])
        .unwrap();
    let third = d.add_page();
    assert_eq!(third.origin, Point::new(560.0, 0.0));
    assert_eq!(third.size, DocumentSize::from_mm(400.0, 100.0));
    d.reorder_page(&first, 2).unwrap();
    let next = d.add_page();
    assert_eq!(next.size, DocumentSize::from_mm(100.0, 100.0));
    assert_eq!(next.origin, Point::new(120.0, 0.0));
    let row = Document::new(594);
    row.resize_page("first", DocumentSize::from_mm(100.0, 150.0), &[])
        .unwrap();
    for _ in 1..9 {
        row.add_page();
    }
    assert_eq!(row.pages()[8].origin, Point::new(0.0, 170.0));
    let wide = Document::new(595);
    wide.resize_page("first", DocumentSize::from_mm(2000.0, 100.0), &[])
        .unwrap();
    assert_eq!(wide.add_page().origin, Point::new(0.0, 300.0));
}

#[test]
fn ac5_ac6_ac13_reorder_rename_delete_keep_objects_and_have_individual_undo() {
    let d = Document::new(596);
    let second = d.add_page();
    let object = d.create_rect(RectBounds::from_corners(
        second.origin,
        Point::new(second.origin.x + 30.0, 30.0),
    ));
    let geometry = d.object(object).unwrap();
    let before = d.pages();
    d.rename_page(&second.id, "  board  ").unwrap();
    assert_eq!(d.pages()[1].name, "  board  ");
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert_eq!(d.pages(), before);
    d.reorder_page(&second.id, 0).unwrap();
    assert_eq!(d.pages(), vec![before[1].clone(), before[0].clone()]);
    assert_eq!(d.object(object).unwrap(), geometry);
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert_eq!(d.pages(), before);
    d.delete_page(&second.id).unwrap();
    assert_eq!(d.object(object).unwrap(), geometry);
    assert_eq!(d.pages(), vec![before[0].clone()]);
    let version = d.version();
    assert!(d.delete_page("first").is_err());
    assert_eq!(version, d.version());
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert_eq!(d.pages(), before);
}

#[test]
fn ac13_rejections_and_noops_do_not_create_history_or_operations() {
    let d = Document::new(597);
    let initial = d.pages()[0].clone();
    let version = d.version();
    let history = d.history_len();
    for size in [
        DocumentSize::from_mm(0.0, 10.0),
        DocumentSize::from_mm(f64::NAN, 10.0),
        DocumentSize::from_mm(10.0, f64::INFINITY),
    ] {
        assert!(d.resize_page(&initial.id, size, &[]).is_err());
    }
    assert!(
        d.move_page(&initial.id, Vec2::new(f64::NAN, 0.0), &[])
            .is_err()
    );
    assert!(d.rename_page(&initial.id, " \t ").is_err());
    assert!(d.reorder_page("absent", 0).is_err());
    assert!(!d.reorder_page(&initial.id, 0).unwrap());
    assert!(!d.rename_page(&initial.id, &initial.name).unwrap());
    assert!(!d.resize_page(&initial.id, initial.size, &[]).unwrap());
    assert_eq!(d.version(), version);
    assert_eq!(d.history_len(), history);
    assert_eq!(d.pages(), vec![initial]);
}
