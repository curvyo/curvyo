//! Editable controls evaluate without changing the stored path.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]
use curvyo_document_core::{AnchorId, CurveKind, Document, NewAnchor, Point};
use curvyo_ui_core::evaluate_path;

#[test]
fn bspline_evaluation_preserves_source_and_endpoints() {
    let doc = Document::new(1);
    let controls = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut node = NewAnchor::corner(AnchorId::new(1, i as u64), Point::new(x, y));
            node.kind = curvyo_document_core::AnchorKind::Symmetric;
            node
        })
        .collect::<Vec<_>>();
    let id = doc.create_path(&controls, false);
    doc.set_curve_kind(id, CurveKind::BSpline).unwrap();
    let source = doc.path(id).unwrap();
    let result = evaluate_path(&source).unwrap();
    assert_eq!(result.anchors.len(), 2);
    assert_eq!(result.anchors[0].point, controls[0].point);
    assert_eq!(result.anchors[1].point, controls[3].point);
    assert_eq!(doc.path(id).unwrap(), source);
}

#[test]
fn object_hit_uses_bspline_instead_of_its_control_polygon() {
    use curvyo_document_core::{ObjectSnapshot, Tolerance};
    let doc = Document::new(2);
    let controls = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut node = NewAnchor::corner(AnchorId::new(2, i as u64), Point::new(x, y));
            node.kind = curvyo_document_core::AnchorKind::Symmetric;
            node
        })
        .collect::<Vec<_>>();
    let id = doc.create_path(&controls, false);
    doc.set_curve_kind(id, CurveKind::BSpline).unwrap();
    let objects = vec![ObjectSnapshot::Path(doc.path(id).unwrap())];
    assert_eq!(
        curvyo_ui_core::hit_test_object(&objects, Point::new(7.5, 5.0), Tolerance::from_mm(0.1)),
        Some(id)
    );
    assert_eq!(
        curvyo_ui_core::hit_test_object(&objects, Point::new(10.0, 5.0), Tolerance::from_mm(0.1)),
        None
    );
}

#[test]
fn clone_artwork_and_its_root_use_the_same_evaluated_geometry() {
    use curvyo_document_core::ObjectSnapshot;
    let doc = Document::new(3);
    let controls = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut node = NewAnchor::corner(AnchorId::new(3, i as u64), Point::new(x, y));
            node.kind = curvyo_document_core::AnchorKind::Symmetric;
            node
        })
        .collect::<Vec<_>>();
    let id = doc.create_path(&controls, false);
    doc.set_curve_kind(id, CurveKind::BSpline).unwrap();
    let clone_id = doc.linked_clone(&[id], &[]).unwrap()[0];
    let original = doc.object(clone_id).unwrap();
    let ObjectSnapshot::Clone(evaluated) = curvyo_ui_core::evaluate_object(&original).unwrap()
    else {
        panic!("expected clone")
    };
    assert_eq!(evaluated.resolved.curve_kind, CurveKind::Bezier);
    assert_eq!(evaluated.chain.root_outline.curve_kind, CurveKind::Bezier);
    assert_eq!(evaluated.resolved.anchors.len(), 2);
    assert_eq!(evaluated.chain.root_outline.anchors.len(), 2);
    assert_eq!(
        original.drawn_path().unwrap().curve_kind,
        CurveKind::BSpline
    );
}

#[test]
fn stroke_to_path_makes_a_closed_variable_width_outline() {
    use curvyo_document_core::ObjectSnapshot;
    use curvyo_ui_core::{AnchorIdMinter, plan_stroke_to_path};
    let doc = Document::new(5);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(5, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(5, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(5, 3), 0.5, 2.0)
        .unwrap();
    let objects = vec![ObjectSnapshot::Path(doc.path(id).unwrap())];
    let plan = plan_stroke_to_path(
        &objects,
        &[id],
        &|_| false,
        &|_| None,
        &mut AnchorIdMinter::new(5),
    )
    .unwrap();
    assert_eq!(plan.strokes, 1);
    assert_eq!(plan.write.outlines.len(), 1);
    let outlines = &plan.write.outlines[0].outline;
    assert_eq!(outlines.len(), 1);
    assert!(outlines[0].1, "the stroke outline is closed");
    let ys: Vec<_> = outlines[0].0.iter().map(|a| a.point.y).collect();
    assert!(ys.iter().any(|y| *y > 0.0) && ys.iter().any(|y| *y < 0.0));
    let half_width = doc.path(id).unwrap().style.stroke.width.as_mm() / 2.0;
    let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!((max_y - 2.0 * half_width).abs() < 0.01, "{max_y}");
    assert_eq!(
        doc.path(id).unwrap(),
        match &objects[0] {
            ObjectSnapshot::Path(p) => p.clone(),
            _ => unreachable!(),
        }
    );
}

#[test]
fn public_svg_export_writes_the_variable_profile_as_closed_geometry() {
    use curvyo_text_core::FaceStore;
    use curvyo_ui_core::{ExportScope, ImportFaces, SvgExportOptions, svg_export};
    let doc = Document::new(6);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(6, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(6, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(6, 3), 0.5, 2.0)
        .unwrap();
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let scope = ExportScope {
        selection: &[],
        active_page: 0,
    };
    let exported = svg_export(
        &doc,
        &doc.read_objects(),
        &fonts,
        &scope,
        SvgExportOptions::default(),
    )
    .unwrap()
    .text();
    assert!(exported.contains("<path"), "{exported}");
    assert!(exported.contains('Z'), "closed outline: {exported}");
    assert!(curvyo_svg_core::read_svg(exported.as_bytes()).is_ok());
    assert_eq!(doc.path(id).unwrap().width_points.len(), 1);
}

#[test]
fn node_tool_does_not_treat_bspline_control_polygon_as_artwork_segments() {
    use curvyo_document_core::Tolerance;
    use curvyo_ui_core::{NodeSelection, hit_test};
    let doc = Document::new(50);
    let controls = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut node = NewAnchor::corner(AnchorId::new(50, i as u64), Point::new(x, y));
            node.kind = curvyo_document_core::AnchorKind::Symmetric;
            node
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let path = doc.path(id).unwrap();
    let tolerance = Tolerance::from_mm(0.1);
    assert!(
        hit_test(
            &[path],
            &NodeSelection::new(),
            Point::new(10.0, 5.0),
            tolerance,
            tolerance,
            tolerance
        )
        .is_none()
    );
}

#[test]
fn selection_bounds_follow_spline_artwork() {
    use curvyo_document_core::ObjectSnapshot;
    let doc = Document::new(51);
    let controls = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut a = NewAnchor::corner(AnchorId::new(51, i as u64), Point::new(x, y));
            a.kind = curvyo_document_core::AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let object = ObjectSnapshot::Path(doc.path(id).unwrap());
    for (_, max) in [
        curvyo_ui_core::object_bounds(&object),
        curvyo_ui_core::object_outline_bounds(&object),
    ] {
        assert!((max.x - 7.5).abs() < 1e-8, "{max:?}");
    }
}

#[test]
fn boolean_refuses_to_discard_editable_bspline_controls() {
    use curvyo_ui_core::{AnchorIdMinter, BooleanOp, ObjectSelection, plan_boolean};
    let doc = Document::new(52);
    let controls = [(0., 0.), (10., 0.), (10., 10.), (0., 10.)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut a = NewAnchor::corner(AnchorId::new(52, i as u64), Point::new(x, y));
            a.kind = curvyo_document_core::AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let other = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(52, 10), Point::new(2., 2.)),
            NewAnchor::corner(AnchorId::new(52, 11), Point::new(9., 2.)),
            NewAnchor::corner(AnchorId::new(52, 12), Point::new(9., 8.)),
            NewAnchor::corner(AnchorId::new(52, 13), Point::new(2., 8.)),
        ],
        true,
    );
    let objects = doc.read_objects().objects;
    let mut selection = ObjectSelection::new();
    selection.set(&[id, other]);
    assert!(matches!(
        plan_boolean(
            &objects,
            &selection,
            BooleanOp::Intersection,
            &mut AnchorIdMinter::new(99)
        ),
        Err(curvyo_ui_core::BooleanRefusal::ExtendedPath { .. })
    ));
    assert!(
        curvyo_ui_core::plan_combine(&objects, &selection, &mut AnchorIdMinter::new(99)).is_err()
    );
    assert!(curvyo_ui_core::plan_cut(&objects, &selection, &mut AnchorIdMinter::new(99)).is_err());
    for result in [
        curvyo_ui_core::plan_fracture(&objects, &selection, &mut AnchorIdMinter::new(99)),
        curvyo_ui_core::plan_flatten(&objects, &selection, &mut AnchorIdMinter::new(99)),
    ] {
        assert!(matches!(
            result,
            Err(curvyo_ui_core::StackRefusal::Operands(
                curvyo_ui_core::BooleanRefusal::ExtendedPath { .. }
            ))
        ));
    }
}

#[test]
fn boolean_and_stack_refuse_variable_width_profiles() {
    use curvyo_ui_core::{
        AnchorIdMinter, BooleanOp, BooleanRefusal, ObjectSelection, StackRefusal, plan_boolean,
        plan_flatten,
    };
    let doc = Document::new(53);
    let nodes = |base: u64, dx: f64| {
        [(0., 0.), (10., 0.), (10., 10.), (0., 10.)]
            .into_iter()
            .enumerate()
            .map(|(i, (x, y))| {
                NewAnchor::corner(AnchorId::new(53, base + i as u64), Point::new(x + dx, y))
            })
            .collect::<Vec<_>>()
    };
    let first = doc.create_path(&nodes(0, 0.), true);
    let second = doc.create_path(&nodes(10, 5.), true);
    doc.set_width_point(first, AnchorId::new(53, 20), 0.5, 2.)
        .unwrap();
    let objects = doc.read_objects().objects;
    let mut selection = ObjectSelection::new();
    selection.set(&[first, second]);
    assert!(matches!(
        plan_boolean(
            &objects,
            &selection,
            BooleanOp::Union,
            &mut AnchorIdMinter::new(99)
        ),
        Err(BooleanRefusal::ExtendedPath { .. })
    ));
    assert!(matches!(
        plan_flatten(&objects, &selection, &mut AnchorIdMinter::new(99)),
        Err(StackRefusal::Operands(BooleanRefusal::ExtendedPath { .. }))
    ));
    let units = [first, second].map(|id| curvyo_ui_core::SelectedUnit {
        id,
        is_group: false,
        leaves: vec![id],
    });
    assert!(curvyo_ui_core::plan_weld(&objects, &units, &mut AnchorIdMinter::new(99)).is_err());
    let request = curvyo_ui_core::OffsetRequest {
        distance_mm: 1.0,
        sides: curvyo_ui_core::OffsetSides {
            out: Some(curvyo_ui_core::OffsetRole::Copy),
            inside: None,
        },
        join: curvyo_ui_core::OffsetJoin::Round,
    };
    assert!(curvyo_ui_core::plan_offset(&objects, &selection, &request).is_err());
}

#[test]
fn editable_curve_endpoints_are_not_offered_for_bezier_extension() {
    use curvyo_document_core::Length;
    use curvyo_ui_core::EndNodeIndex;
    let doc = Document::new(54);
    let controls = [(0., 0.), (10., 0.), (10., 10.)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut a = NewAnchor::corner(AnchorId::new(54, i as u64), Point::new(x, y));
            a.kind = curvyo_document_core::AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let index = EndNodeIndex::from_paths(&[doc.path(id).unwrap()]);
    assert!(
        index
            .nearest(Point::new(0., 0.), Length::from_mm(0.1), None)
            .is_none()
    );
}

#[test]
fn measure_does_not_treat_spline_controls_as_drawn_segments() {
    use curvyo_document_core::{Length, ObjectSnapshot, PointSlot};
    use curvyo_ui_core::{MeasurePick, measure_pick};
    let doc = Document::new(55);
    let controls = [(0., 0.), (10., 0.), (10., 10.), (0., 10.)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut a = NewAnchor::corner(AnchorId::new(55, i as u64), Point::new(x, y));
            a.kind = curvyo_document_core::AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let objects = [ObjectSnapshot::Path(doc.path(id).unwrap())];
    assert!(matches!(
        measure_pick(
            &objects,
            Point::new(10., 5.),
            (Length::from_mm(0.1), Length::from_mm(0.1))
        ),
        MeasurePick::Point {
            slot: PointSlot::Free { .. },
            ..
        }
    ));
}

#[test]
fn split_and_break_apart_refuse_compound_extended_snapshots() {
    use curvyo_document_core::{ObjectSnapshot, SubpathSnapshot};
    use curvyo_ui_core::{AnchorIdMinter, ObjectSelection, plan_break_apart, plan_split};
    let doc = Document::new(56);
    let controls = (0..3)
        .map(|i| {
            let mut a = NewAnchor::corner(
                AnchorId::new(56, i),
                Point::new(
                    f64::from(u32::try_from(i).unwrap()),
                    f64::from(u32::try_from(i % 2).unwrap()),
                ),
            );
            a.kind = curvyo_document_core::AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    let mut path = doc.path(id).unwrap();
    path.extra_subpaths.push(SubpathSnapshot {
        closed: false,
        anchors: controls.clone(),
    });
    let objects = [ObjectSnapshot::Path(path)];
    let mut selection = ObjectSelection::new();
    selection.set(&[id]);
    assert!(plan_split(&objects, &selection, &mut AnchorIdMinter::new(99)).is_err());
    assert!(plan_break_apart(&objects, &selection, &mut AnchorIdMinter::new(99)).is_err());
}

#[test]
fn svg_evaluates_spline_carrier_for_attached_text() {
    use curvyo_document_core::{
        Affine, CarrierRef, FontKind, FontRef, Length, ObjectRead, ObjectSnapshot, OnPathValues,
        TextLayoutValues,
    };
    use curvyo_text_core::FaceStore;
    use curvyo_ui_core::{ExportScope, ImportFaces, SvgExportOptions, svg_export};
    let d = Document::new(57);
    let nodes = (0..3)
        .map(|i| {
            NewAnchor::corner(
                AnchorId::new(57, i),
                Point::new(
                    f64::from(u32::try_from(i).unwrap()),
                    f64::from(u32::try_from(i % 2).unwrap()),
                ),
            )
        })
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
    let mut objects = d.read_objects().objects.to_vec();
    for object in &mut objects {
        if let ObjectSnapshot::Path(path) = object
            && path.id == id
        {
            path.curve_kind = CurveKind::BSpline;
        }
    }
    let laid = ObjectRead::from_objects(objects.into());
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let exported = svg_export(
        &d,
        &laid,
        &fonts,
        &ExportScope {
            selection: &[],
            active_page: 0,
        },
        SvgExportOptions::default(),
    )
    .unwrap()
    .text();
    assert!(exported.contains("<textPath"), "{exported}");
    assert!(exported.contains("href=\"#"), "{exported}");
    let carrier = exported
        .split("href=\"#")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert!(
        exported.contains(&format!("id=\"{carrier}\"")),
        "{exported}"
    );
    assert!(exported.contains(" C "), "evaluated cubic: {exported}");
    assert!(curvyo_svg_core::read_svg(exported.as_bytes()).is_ok());
}

#[test]
fn recognition_does_not_replace_editable_spline_controls() {
    use curvyo_document_core::ObjectSnapshot;
    let d = Document::new(58);
    let controls = (0..4)
        .map(|i| {
            let mut a = NewAnchor::corner(
                AnchorId::new(58, i),
                Point::new(
                    f64::from(u32::try_from(i % 2).unwrap()),
                    f64::from(u32::try_from(i / 2).unwrap()),
                ),
            );
            a.kind = curvyo_document_core::AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = d
        .create_editable_curve(&controls, true, CurveKind::BSpline)
        .unwrap();
    let objects = [ObjectSnapshot::Path(d.path(id).unwrap())];
    assert!(!curvyo_ui_core::recognise_available(
        &objects,
        &[id],
        &|_| false
    ));
    assert!(
        curvyo_ui_core::recognise_subjects(&objects, &[id], &|_| false)
            .outlines
            .is_empty()
    );
}

#[test]
fn trace_mask_evaluates_splines_and_refuses_unsupported_curves() {
    use curvyo_document_core::ObjectSnapshot;
    let d = Document::new(59);
    let controls = [(0., 0.), (10., 0.), (10., 10.), (0., 10.)]
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let mut a = NewAnchor::corner(AnchorId::new(59, i as u64), Point::new(x, y));
            a.kind = curvyo_document_core::AnchorKind::Symmetric;
            a
        })
        .collect::<Vec<_>>();
    let id = d
        .create_editable_curve(&controls, true, CurveKind::BSpline)
        .unwrap();
    let raw = ObjectSnapshot::Path(d.path(id).unwrap());
    let evaluated = curvyo_ui_core::evaluate_object(&raw).unwrap();
    assert_eq!(
        curvyo_ui_core::mask_operand(&raw).unwrap(),
        curvyo_ui_core::mask_operand(&evaluated).unwrap()
    );
    let mut spiro = d.path(id).unwrap();
    spiro.curve_kind = CurveKind::Spiro;
    assert!(curvyo_ui_core::mask_operand(&ObjectSnapshot::Path(spiro.clone())).is_ok());
    // Controls on one point have no curve: refused, not masked as their polygon.
    let mut unsupported = spiro;
    for anchor in &mut unsupported.anchors {
        anchor.point = Point::new(1.0, 1.0);
    }
    assert!(curvyo_ui_core::mask_operand(&ObjectSnapshot::Path(unsupported)).is_err());
}

#[test]
fn variable_stroke_follows_bspline_corner_artwork() {
    let doc = Document::new(76);
    let controls = [
        (0.0, 0.0),
        (3.0, 0.0),
        (6.0, 4.0),
        (10.0, 5.0),
        (10.0, 9.0),
        (14.0, 12.0),
        (15.0, 15.0),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (x, y))| {
        let mut node = NewAnchor::corner(AnchorId::new(76, i as u64 + 1), Point::new(x, y));
        if i != 3 {
            node.kind = curvyo_document_core::AnchorKind::Symmetric;
        }
        node
    })
    .collect::<Vec<_>>();
    let id = doc
        .create_editable_curve(&controls, false, CurveKind::BSpline)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(76, 30), 0.0, 1.0)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(76, 31), 1.0, 1.5)
        .unwrap();
    let path = doc.path(id).unwrap();
    let evaluated = evaluate_path(&path).unwrap();
    assert!(evaluated.anchors.iter().any(|anchor| {
        (anchor.handle_in.x * anchor.handle_out.y - anchor.handle_in.y * anchor.handle_out.x).abs()
            > 1e-6
    }));
    let result = curvyo_ui_core::verified_variable_stroke_artwork(&path);
    assert!(result.is_ok(), "{result:?}");
    assert!(result.unwrap().1.is_some());
    assert_eq!(doc.path(id).unwrap(), path);
}
