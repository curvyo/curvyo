//! Editable Spiro placement survives storage and history as one affine register.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Affine, AnchorId, CurveKind, Document, NewAnchor, Point, pack, unpack};

#[test]
fn spiro_placement_round_trips_and_undoes_without_baking_controls() {
    let doc = Document::new(901);
    let anchors = [
        NewAnchor::corner(AnchorId::new(901, 1), Point::new(1.0, 2.0)),
        NewAnchor::corner(AnchorId::new(901, 2), Point::new(5.0, 8.0)),
    ];
    let id = doc
        .create_editable_curve(&anchors, false, CurveKind::Spiro)
        .unwrap();
    let before = doc.path(id).unwrap();
    assert_eq!(before.spiro_transform.as_deref().copied(), None);
    let matrix = Affine::new(2.0, 0.2, 0.3, 1.5, 7.0, 9.0);
    doc.set_spiro_transform(id, matrix).unwrap();
    let after = doc.path(id).unwrap();
    assert_eq!(after.anchors, before.anchors);
    assert_eq!(after.spiro_transform.as_deref().copied(), Some(matrix));
    let reopened = unpack(902, &pack(&doc, "test").unwrap()).unwrap();
    assert_eq!(
        reopened
            .path(id)
            .unwrap()
            .spiro_transform
            .as_deref()
            .copied(),
        Some(matrix)
    );
    let step = doc.own_steps().last().unwrap().step;
    doc.undo_step(step).unwrap();
    assert_eq!(
        doc.path(id).unwrap().spiro_transform.as_deref().copied(),
        None
    );
}

#[test]
fn invalid_and_non_spiro_placements_are_refused_without_writes() {
    let doc = Document::new(903);
    let anchors = [
        NewAnchor::corner(AnchorId::new(903, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(903, 2), Point::new(5.0, 8.0)),
    ];
    let spiro = doc
        .create_editable_curve(&anchors, false, CurveKind::Spiro)
        .unwrap();
    let ordinary = doc.create_path(&anchors, false);
    let version = doc.version();
    for matrix in [
        Affine::scale(0.0, 1.0),
        Affine::new(f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0),
    ] {
        assert!(doc.set_spiro_transform(spiro, matrix).is_err());
    }
    assert!(doc.set_spiro_transform(ordinary, Affine::IDENTITY).is_err());
    assert_eq!(doc.version(), version);
}

#[test]
fn document_json_contains_editable_curve_and_affine() {
    let doc = Document::new(904);
    let anchors = [
        NewAnchor::corner(AnchorId::new(904, 1), Point::new(1.0, 2.0)),
        NewAnchor::corner(AnchorId::new(904, 2), Point::new(5.0, 8.0)),
    ];
    let id = doc
        .create_editable_curve(&anchors, false, CurveKind::Spiro)
        .unwrap();
    doc.set_spiro_transform(id, Affine::new(2.0, 0.0, 0.3, 1.0, 4.0, 6.0))
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&doc.export_json().unwrap()).unwrap();
    assert_eq!(json["objects"][0]["curve_kind"], "spiro");
    assert_eq!(
        json["objects"][0]["spiro_transform"],
        serde_json::json!([2.0, 0.0, 0.3, 1.0, 4.0, 6.0])
    );
}

#[test]
fn object_transform_composes_placement_without_baking_controls() {
    use curvyo_document_core::ObjectSnapshot;
    let doc = Document::new(905);
    let anchors = [
        NewAnchor::corner(AnchorId::new(905, 1), Point::new(1.0, 2.0)),
        NewAnchor::corner(AnchorId::new(905, 2), Point::new(5.0, 8.0)),
    ];
    let id = doc
        .create_editable_curve(&anchors, false, CurveKind::Spiro)
        .unwrap();
    let before = doc.path(id).unwrap();
    let transform = Affine::new(2.0, 0.0, 0.3, 1.0, 4.0, 6.0);
    let result = before.mapped(transform);
    assert_eq!(result.anchors, before.anchors);
    assert_eq!(result.spiro_transform.as_deref().copied(), Some(transform));
    doc.transform_objects(&[ObjectSnapshot::Path(result)], false)
        .unwrap();
    assert_eq!(
        doc.path(id).unwrap().spiro_transform.as_deref().copied(),
        Some(transform)
    );
    assert_eq!(doc.path(id).unwrap().anchors, before.anchors);
    let step = doc.own_steps().last().unwrap().step;
    doc.undo_step(step).unwrap();
    assert_eq!(doc.path(id).unwrap(), before);
}

#[test]
fn copy_and_unlink_keep_local_controls_and_composed_placement() {
    use curvyo_document_core::{CopySource, ObjectSnapshot, Vec2};
    let doc = Document::new(906);
    let anchors = [
        NewAnchor::corner(AnchorId::new(906, 1), Point::new(1.0, 2.0)),
        NewAnchor::corner(AnchorId::new(906, 2), Point::new(5.0, 8.0)),
    ];
    let source = doc
        .create_editable_curve(&anchors, false, CurveKind::Spiro)
        .unwrap();
    let matrix = Affine::new(2.0, 0.2, 0.3, 1.5, 7.0, 9.0);
    doc.set_spiro_transform(source, matrix).unwrap();
    let offset = Vec2::new(5.0, 7.0);
    let copy = doc
        .duplicate_objects(
            &[CopySource {
                id: source,
                anchor_ids: vec![AnchorId::new(906, 3), AnchorId::new(906, 4)],
            }],
            offset,
        )
        .unwrap()[0];
    let copy_path = doc.path(copy).unwrap();
    assert_eq!(copy_path.anchors[0].point, anchors[0].point);
    assert_eq!(
        copy_path.spiro_transform.as_deref().copied(),
        Some(Affine::translation(offset).compose(matrix))
    );
    let clone = doc.linked_clone(&[source], &[]).unwrap()[0];
    let ObjectSnapshot::Clone(snapshot) = doc.object(clone).unwrap() else {
        panic!("missing clone")
    };
    let clone_map = Affine::new(1.0, 0.0, 0.4, 1.0, 3.0, 4.0);
    doc.transform_objects(
        &[ObjectSnapshot::Clone(snapshot.mapped_by(clone_map))],
        false,
    )
    .unwrap();
    doc.unlink_clones(&[CopySource {
        id: clone,
        anchor_ids: vec![AnchorId::new(906, 5), AnchorId::new(906, 6)],
    }])
    .unwrap();
    let unlinked = doc.path(clone).unwrap();
    assert_eq!(unlinked.anchors[0].point, anchors[0].point);
    assert_eq!(
        unlinked.spiro_transform.as_deref().copied(),
        Some(clone_map.compose(matrix))
    );
}

#[test]
fn small_invertible_placement_is_not_rejected_by_clone_threshold() {
    let doc = Document::new(907);
    let controls = [
        NewAnchor::corner(AnchorId::new(907, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(907, 2), Point::new(1.0, 0.0)),
    ];
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::Spiro)
        .unwrap();
    let matrix = Affine::scale(1e-6, 1e-6);
    doc.set_spiro_transform(id, matrix).unwrap();
    assert_eq!(
        unpack(908, &pack(&doc, "small").unwrap())
            .unwrap()
            .path(id)
            .unwrap()
            .spiro_transform
            .as_deref()
            .copied(),
        Some(matrix)
    );
}

#[test]
fn translation_preview_matches_persisted_placement() {
    use curvyo_document_core::{ObjectSnapshot, Vec2};
    let doc = Document::new(909);
    let controls = [
        NewAnchor::corner(AnchorId::new(909, 1), Point::new(1.0, 2.0)),
        NewAnchor::corner(AnchorId::new(909, 2), Point::new(5.0, 8.0)),
    ];
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::Spiro)
        .unwrap();
    let offset = Vec2::new(3.0, 4.0);
    let preview = doc.object(id).unwrap().translated(offset);
    doc.translate_objects(&[id], offset).unwrap();
    assert_eq!(preview, ObjectSnapshot::Path(doc.path(id).unwrap()));
}

#[test]
fn explicit_spiro_conversion_clears_placement_and_undo_restores_it() {
    let doc = Document::new(910);
    let controls = [
        NewAnchor::corner(AnchorId::new(910, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(910, 2), Point::new(10.0, 0.0)),
    ];
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::Spiro)
        .unwrap();
    let matrix = Affine::new(2.0, 0.0, 0.0, 1.0, 3.0, 4.0);
    doc.set_spiro_transform(id, matrix).unwrap();
    let source = doc.path(id).unwrap();
    let output = [
        NewAnchor::corner(AnchorId::new(910, 3), Point::new(3.0, 4.0)),
        NewAnchor::corner(AnchorId::new(910, 4), Point::new(23.0, 4.0)),
    ];
    doc.convert_spiro_to_bezier(&source, &output).unwrap();
    let converted = doc.path(id).unwrap();
    assert_eq!(converted.curve_kind, CurveKind::Bezier);
    assert_eq!(converted.spiro_transform.as_deref().copied(), None);
    assert_eq!(converted.anchors, output);
    let step = doc.own_steps().last().unwrap().step;
    doc.undo_step(step).unwrap();
    assert_eq!(doc.path(id).unwrap(), source);
}

#[test]
fn concurrent_placements_merge_as_one_atomic_matrix() {
    let a = Document::new(911);
    let controls = [
        NewAnchor::corner(AnchorId::new(911, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(911, 2), Point::new(10.0, 0.0)),
    ];
    let id = a
        .create_editable_curve(&controls, false, CurveKind::Spiro)
        .unwrap();
    let b = unpack(912, &pack(&a, "shared-base").unwrap()).unwrap();
    let ma = Affine::new(2.0, 0.2, 0.3, 1.5, 7.0, 9.0);
    let mb = Affine::new(1.0, 0.4, 0.7, 2.0, 11.0, 13.0);
    a.set_spiro_transform(id, ma).unwrap();
    b.set_spiro_transform(id, mb).unwrap();
    let au = a.export_updates(&b.version()).unwrap();
    let bu = b.export_updates(&a.version()).unwrap();
    a.import_updates(&bu).unwrap();
    b.import_updates(&au).unwrap();
    let winner = a
        .path(id)
        .unwrap()
        .spiro_transform
        .as_deref()
        .copied()
        .unwrap();
    assert!(winner == ma || winner == mb);
    assert_eq!(
        b.path(id).unwrap().spiro_transform.as_deref().copied(),
        Some(winner)
    );
    assert_eq!(
        unpack(913, &pack(&a, "reopened").unwrap())
            .unwrap()
            .path(id)
            .unwrap()
            .spiro_transform
            .as_deref()
            .copied(),
        Some(winner)
    );
}

#[test]
fn rotate_object_commits_placement_as_one_undo_step() {
    use curvyo_document_core::{Angle, ObjectSnapshot};
    let doc = Document::new(914);
    let controls = [
        NewAnchor::corner(AnchorId::new(914, 1), Point::new(1.0, 2.0)),
        NewAnchor::corner(AnchorId::new(914, 2), Point::new(5.0, 8.0)),
    ];
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::Spiro)
        .unwrap();
    let before = doc.path(id).unwrap();
    let result =
        ObjectSnapshot::Path(before.rotated(Point::new(1.0, 3.0), Angle::from_radians(0.6)));
    doc.rotate_object(&result).unwrap();
    assert_eq!(doc.object(id).unwrap(), result);
    let step = doc.own_steps().last().unwrap().step;
    doc.undo_step(step).unwrap();
    assert_eq!(doc.path(id).unwrap(), before);
}

#[test]
fn shared_document_refuses_spiro_unlink_before_any_write() {
    use curvyo_document_core::CopySource;
    let doc = Document::new(915);
    let controls = [
        NewAnchor::corner(AnchorId::new(915, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(915, 2), Point::new(10.0, 0.0)),
    ];
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::Spiro)
        .unwrap();
    let clone = doc.linked_clone(&[id], &[]).unwrap()[0];
    doc.set_live_share(true);
    let version = doc.version();
    assert!(
        doc.unlink_clones(&[CopySource {
            id: clone,
            anchor_ids: vec![AnchorId::new(915, 3), AnchorId::new(915, 4)]
        }])
        .is_err()
    );
    assert_eq!(doc.version(), version);
}

#[test]
fn spiro_copy_rejects_nonfinite_offset_without_writes() {
    use curvyo_document_core::{CopySource, Vec2};
    let doc = Document::new(916);
    let controls = [
        NewAnchor::corner(AnchorId::new(916, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(916, 2), Point::new(10.0, 0.0)),
    ];
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::Spiro)
        .unwrap();
    let version = doc.version();
    assert!(
        doc.duplicate_objects(
            &[CopySource {
                id,
                anchor_ids: vec![AnchorId::new(916, 3), AnchorId::new(916, 4)]
            }],
            Vec2::new(f64::INFINITY, 0.0)
        )
        .is_err()
    );
    assert_eq!(doc.version(), version);
}

#[test]
fn singular_unlink_result_refuses_whole_transform_without_writes() {
    use curvyo_document_core::ObjectSnapshot;
    let doc = Document::new(917);
    let controls = [
        NewAnchor::corner(AnchorId::new(917, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(917, 2), Point::new(10.0, 0.0)),
    ];
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::Spiro)
        .unwrap();
    let clone_id = doc.linked_clone(&[id], &[]).unwrap()[0];
    let ObjectSnapshot::Clone(clone) = doc.object(clone_id).unwrap() else {
        panic!("clone")
    };
    let mut unlinked = clone.resolved.mapped(Affine::scale(0.0, 1.0));
    for (index, anchor) in unlinked.anchors.iter_mut().enumerate() {
        anchor.id = AnchorId::new(917, 3 + index as u64);
    }
    let version = doc.version();
    assert!(
        doc.transform_objects(&[ObjectSnapshot::Path(unlinked)], false)
            .is_err()
    );
    assert_eq!(doc.version(), version);
    assert!(matches!(
        doc.object(clone_id),
        Some(ObjectSnapshot::Clone(_))
    ));
}

/// Architect review A4: a Spiro transform in a shared document is refused as shared, not as
/// non-finite geometry.
#[test]
fn shared_document_refuses_a_spiro_transform_as_shared() {
    use curvyo_document_core::{Angle, ObjectEditError, ObjectSnapshot};
    let doc = Document::new(917);
    let controls = [
        NewAnchor::corner(AnchorId::new(917, 1), Point::new(0.0, 0.0)),
        NewAnchor::corner(AnchorId::new(917, 2), Point::new(10.0, 0.0)),
    ];
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::Spiro)
        .unwrap();
    let moved = ObjectSnapshot::Path(
        doc.path(id)
            .unwrap()
            .rotated(Point::new(0.0, 0.0), Angle::from_radians(0.5)),
    );
    doc.set_live_share(true);
    let version = doc.version();
    assert_eq!(
        doc.transform_objects(&[moved], false),
        Err(ObjectEditError::Shared)
    );
    assert_eq!(doc.version(), version);
}
