//! Editable curve kind and width points survive the shared path read model.
#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::assert_is_empty
)]
use curvyo_document_core::{AnchorId, CurveKind, Document, NewAnchor, Point, pack, unpack};

#[test]
fn editable_curve_and_profile_survive_save_reopen_and_undo() {
    let doc = Document::new(1);
    let path = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 10.0)),
        ],
        false,
    );
    doc.set_curve_kind(path, CurveKind::BSpline).unwrap();
    doc.set_width_point(path, AnchorId::new(1, 3), 0.5, 2.0)
        .unwrap();
    let snapshot = doc.path(path).unwrap();
    assert_eq!(snapshot.curve_kind, CurveKind::BSpline);
    assert_eq!(snapshot.width_points.len(), 1);
    let reopened = unpack(2, &pack(&doc, "test").unwrap()).unwrap();
    assert_eq!(reopened.path(path).unwrap(), snapshot);
    let step = doc.own_steps().last().unwrap().step;
    doc.undo_step(step).unwrap();
    assert!(doc.path(path).unwrap().width_points.is_empty());
    assert_eq!(doc.path(path).unwrap().curve_kind, CurveKind::BSpline);
}

#[test]
fn creating_a_curve_is_one_undoable_step() {
    let doc = Document::new(1);
    let controls = [
        NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 10.0)),
    ];
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    assert_eq!(doc.path(id).unwrap().curve_kind, CurveKind::BSpline);
    let step = doc.own_steps().last().unwrap().step;
    doc.undo_step(step).unwrap();
    assert!(doc.path(id).is_none());
    assert!(doc.has_extended_path_history());
    doc.redo_step(step).unwrap();
    assert_eq!(doc.path(id).unwrap().curve_kind, CurveKind::BSpline);
}

#[test]
fn undone_curve_creation_survives_wipe_as_a_history_marker() {
    let mut doc = Document::new(93);
    let controls = [
        NewAnchor::corner(AnchorId::new(93, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(93, 2), Point::new(10.0, 10.0)),
    ];
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    doc.undo_step(doc.own_steps().last().unwrap().step).unwrap();
    assert!(doc.path(id).is_none());
    doc.wipe_history().unwrap();
    let reopened = unpack(94, &pack(&doc, "test").unwrap()).unwrap();
    assert!(reopened.has_extended_path_history());
}

#[test]
fn restoring_pre_feature_version_keeps_marker_through_wipe() {
    let mut doc = Document::new(97);
    let path = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(97, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(97, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    let before = doc.own_steps().last().unwrap().step;
    doc.set_curve_kind(path, CurveKind::BSpline).unwrap();
    doc.go_to_version(before, None, None).unwrap();
    assert_eq!(doc.path(path).unwrap().curve_kind, CurveKind::Bezier);
    doc.wipe_history().unwrap();
    assert!(
        unpack(98, &pack(&doc, "test").unwrap())
            .unwrap()
            .has_extended_path_history()
    );
}

#[test]
fn copied_and_unlinked_curves_keep_marker_after_source_deletion_and_wipe() {
    use curvyo_document_core::{CopySource, Vec2};
    let mut doc = Document::new(95);
    let controls = [
        NewAnchor::corner(AnchorId::new(95, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(95, 2), Point::new(10.0, 10.0)),
    ];
    let source = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    doc.set_width_point(source, AnchorId::new(95, 7), 0.5, 2.0)
        .unwrap();
    let copy = doc
        .duplicate_objects(
            &[CopySource {
                id: source,
                anchor_ids: vec![AnchorId::new(95, 3), AnchorId::new(95, 4)],
            }],
            Vec2::ZERO,
        )
        .unwrap()[0];
    let clone = doc.linked_clone(&[source], &[]).unwrap()[0];
    doc.unlink_clones(&[CopySource {
        id: clone,
        anchor_ids: vec![AnchorId::new(95, 5), AnchorId::new(95, 6)],
    }])
    .unwrap();
    assert_eq!(doc.path(copy).unwrap().curve_kind, CurveKind::BSpline);
    assert_eq!(doc.path(clone).unwrap().curve_kind, CurveKind::BSpline);
    assert_eq!(
        doc.width_points(clone).unwrap(),
        doc.width_points(source).unwrap()
    );
    doc.delete_objects(&[source, copy, clone]).unwrap();
    doc.wipe_history().unwrap();
    assert!(
        unpack(96, &pack(&doc, "test").unwrap())
            .unwrap()
            .has_extended_path_history()
    );
}

#[test]
fn an_existing_curve_cannot_be_relabelled_as_bezier_or_silently_lose_handles() {
    use curvyo_document_core::Vec2;
    let doc = Document::new(1);
    let mut first = NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0));
    first.handle_out = Vec2::new(3.0, 4.0);
    let id = doc.create_path(
        &[
            first,
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    let before = doc.path(id).unwrap();
    assert!(doc.set_curve_kind(id, CurveKind::BSpline).is_err());
    assert_eq!(doc.path(id).unwrap(), before);
    let controls = [
        NewAnchor::corner(AnchorId::new(1, 3), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(1, 4), Point::new(10.0, 0.0)),
    ];
    let spline = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    assert!(doc.set_curve_kind(spline, CurveKind::Bezier).is_err());
    assert_eq!(doc.path(spline).unwrap().curve_kind, CurveKind::BSpline);
}

#[test]
fn duplicate_control_id_does_not_create_a_curve() {
    let doc = Document::new(1);
    let controls = [
        NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(1, 1), Point::new(10.0, 0.0)),
    ];
    assert!(
        doc.create_editable_curve(&controls, false, CurveKind::BSpline)
            .is_err()
    );
    assert!(doc.object_ids().is_empty());
}

#[test]
fn split_and_join_refuse_to_discard_curve_semantics() {
    let doc = Document::new(90);
    let controls = (0..4)
        .map(|i| {
            let mut node =
                NewAnchor::corner(AnchorId::new(90, i), Point::new(i as f64, (i % 2) as f64));
            node.kind = curvyo_document_core::AnchorKind::Symmetric;
            node
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let before = doc.path(id).unwrap();
    assert!(
        doc.split_at_anchor(id, controls[1].id, AnchorId::new(90, 20))
            .is_err()
    );
    assert!(!doc.check_join(id, controls[0].id, id, controls[3].id));
    assert_eq!(doc.path(id).unwrap(), before);
}

#[test]
fn deleting_a_control_cannot_leave_an_unevaluable_closed_bspline() {
    let doc = Document::new(91);
    let controls = (0..3)
        .map(|i| {
            let mut node =
                NewAnchor::corner(AnchorId::new(91, i), Point::new(i as f64, (i % 2) as f64));
            node.kind = curvyo_document_core::AnchorKind::Symmetric;
            node
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, true, CurveKind::BSpline)
        .unwrap();
    let before = doc.path(id).unwrap();
    doc.delete_anchors(&[(id, controls[1].id)]);
    assert_eq!(doc.path(id).unwrap(), before);
}

#[test]
fn bezier_conversion_refuses_stale_or_invalid_input_without_mutation() {
    let doc = Document::new(92);
    let controls = (0..3)
        .map(|i| {
            let mut node =
                NewAnchor::corner(AnchorId::new(92, i), Point::new(i as f64, (i % 2) as f64));
            node.kind = curvyo_document_core::AnchorKind::Symmetric;
            node
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let stale = doc.path(id).unwrap();
    doc.move_anchors(&[(id, controls[1].id, Point::new(2., 2.))])
        .unwrap();
    let current = doc.path(id).unwrap();
    assert!(doc.convert_bspline_to_bezier(&stale, &controls).is_err());
    let mut invalid = controls.clone();
    invalid[1].point.x = f64::NAN;
    assert!(doc.convert_bspline_to_bezier(&current, &invalid).is_err());
    assert_eq!(doc.path(id).unwrap(), current);
}

#[test]
fn extending_or_absorbing_an_editable_curve_is_refused_without_data_loss() {
    use curvyo_document_core::{PathEnd, PathGrowth};
    let doc = Document::new(93);
    let controls = (0..3)
        .map(|i| {
            let mut node =
                NewAnchor::corner(AnchorId::new(93, i), Point::new(i as f64, (i % 2) as f64));
            node.kind = curvyo_document_core::AnchorKind::Symmetric;
            node
        })
        .collect::<Vec<_>>();
    let curve = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let line = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(93, 10), Point::new(5., 0.)),
            NewAnchor::corner(AnchorId::new(93, 11), Point::new(6., 0.)),
        ],
        false,
    );
    let growth = PathGrowth {
        path: curve,
        end: PathEnd::Last,
        at: controls[2].id,
        added: vec![NewAnchor::corner(AnchorId::new(93, 20), Point::new(4., 0.))],
        absorb: None,
        replace: vec![],
        drop: vec![],
    };
    let before = doc.path(curve).unwrap();
    assert!(doc.extend_path(&growth).is_err());
    let connect = PathGrowth {
        path: line,
        end: PathEnd::First,
        at: AnchorId::new(93, 10),
        added: vec![],
        absorb: Some((curve, PathEnd::Last, controls[2].id)),
        replace: vec![],
        drop: vec![],
    };
    assert!(doc.connect_paths(&connect).is_err());
    assert_eq!(doc.path(curve).unwrap(), before);
}

#[test]
fn bezier_conversion_refuses_dimension_anchor_dependencies() {
    use curvyo_document_core::{DimensionKind, DimensionPlace, DimensionSlots, Length, PointSlot};
    let doc = Document::new(94);
    let controls = (0..3)
        .map(|i| {
            let mut a =
                NewAnchor::corner(AnchorId::new(94, i), Point::new(i as f64, (i % 2) as f64));
            a.kind = curvyo_document_core::AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    doc.add_dimension(
        DimensionKind::Aligned,
        DimensionSlots::Points(vec![
            PointSlot::Anchor {
                node: id,
                anchor: controls[0].id,
            },
            PointSlot::Anchor {
                node: id,
                anchor: controls[2].id,
            },
        ]),
        DimensionPlace::at_offset(Length::from_mm(5.)),
    )
    .unwrap();
    let source = doc.path(id).unwrap();
    assert!(doc.convert_bspline_to_bezier(&source, &controls).is_err());
    assert_eq!(doc.path(id).unwrap(), source);
}

#[test]
fn bezier_conversion_refuses_dimensions_on_linked_clones() {
    use curvyo_document_core::{DimensionKind, DimensionPlace, DimensionSlots, Length, PointSlot};
    let doc = Document::new(95);
    let controls = (0..3)
        .map(|i| {
            let mut a =
                NewAnchor::corner(AnchorId::new(95, i), Point::new(i as f64, (i % 2) as f64));
            a.kind = curvyo_document_core::AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let clone = doc.linked_clone(&[id], &[]).unwrap()[0];
    doc.add_dimension(
        DimensionKind::Aligned,
        DimensionSlots::Points(vec![
            PointSlot::Anchor {
                node: clone,
                anchor: controls[0].id,
            },
            PointSlot::Anchor {
                node: clone,
                anchor: controls[2].id,
            },
        ]),
        DimensionPlace::at_offset(Length::from_mm(5.)),
    )
    .unwrap();
    let source = doc.path(id).unwrap();
    assert!(doc.convert_bspline_to_bezier(&source, &controls).is_err());
    assert_eq!(doc.path(id).unwrap(), source);
}

#[test]
fn text_cannot_be_placed_on_an_unevaluated_bspline() {
    use curvyo_document_core::{
        Affine, CarrierRef, FontKind, FontRef, Length, OnPathValues, TextLayoutValues,
    };
    let d = Document::new(96);
    let nodes = (0..3)
        .map(|i| {
            let mut a =
                NewAnchor::corner(AnchorId::new(96, i), Point::new(i as f64, (i % 2) as f64));
            a.kind = curvyo_document_core::AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = d
        .create_editable_curve(&nodes, false, CurveKind::BSpline)
        .unwrap();
    let font = FontRef {
        family: "Example".into(),
        style: "Regular".into(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    let text = d
        .create_text(
            "Text",
            &font,
            Length::from_mm(10.),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let before = d.text(text).unwrap();
    assert!(
        d.put_text_on_path(
            text,
            CarrierRef {
                node: id,
                outline: 0
            },
            &OnPathValues::at(Length::from_mm(0.)),
            false
        )
        .is_err()
    );
    assert_eq!(d.text(text).unwrap(), before);
    let clone = d.linked_clone(&[id], &[]).unwrap()[0];
    let values = OnPathValues::at(Length::from_mm(0.));
    assert!(
        d.put_text_on_paths(
            text,
            &[(
                CarrierRef {
                    node: clone,
                    outline: 0
                },
                values
            )],
            false
        )
        .is_err()
    );
    assert_eq!(d.text(text).unwrap(), before);
}

#[test]
fn path_with_text_cannot_be_reinterpreted_as_bspline() {
    use curvyo_document_core::{
        Affine, CarrierRef, FontKind, FontRef, Length, OnPathValues, TextLayoutValues,
    };
    let d = Document::new(97);
    let nodes = (0..3)
        .map(|i| NewAnchor::corner(AnchorId::new(97, i), Point::new(i as f64, (i % 2) as f64)))
        .collect::<Vec<_>>();
    let id = d.create_path(&nodes, false);
    let font = FontRef {
        family: "Example".into(),
        style: "Regular".into(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    let text = d
        .create_text(
            "Text",
            &font,
            Length::from_mm(10.),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    d.put_text_on_path(
        text,
        CarrierRef {
            node: id,
            outline: 0,
        },
        &OnPathValues::at(Length::from_mm(0.)),
        false,
    )
    .unwrap();
    assert!(d.set_curve_kind(id, CurveKind::BSpline).is_err());
    assert_eq!(d.path(id).unwrap().curve_kind, CurveKind::Bezier);
}
