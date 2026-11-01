use super::*;
use crate::dimension_model::{CurveSlot, DimensionValue, Measured};
use crate::primitive_model::{EllipseFrame, Shape};
use crate::ring_curve::Curve;
use crate::units::{Angle, Length, Point};

fn aid(n: u64) -> AnchorId {
    AnchorId::new(77, n)
}

fn line(document: &Document, first: u64, from: (f64, f64), to: (f64, f64)) -> NodeId {
    document.create_path(
        &[
            NewAnchor::corner(aid(first), Point::new(from.0, from.1)),
            NewAnchor::corner(aid(first + 1), Point::new(to.0, to.1)),
        ],
        false,
    )
}

fn aligned(document: &Document, path: NodeId, first: u64) -> NodeId {
    document
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![
                PointSlot::Anchor {
                    node: path,
                    anchor: aid(first),
                },
                PointSlot::Anchor {
                    node: path,
                    anchor: aid(first + 1),
                },
            ]),
            DimensionPlace::at_offset(Length::from_mm(5.0)),
        )
        .unwrap()
}

fn value(document: &Document, id: NodeId) -> (bool, f64, f64) {
    let dimension = document.dimension(id).unwrap();
    (
        dimension.linked,
        dimension.shown.value.raw(),
        dimension.kept.value.raw(),
    )
}

fn circle(document: &Document) -> NodeId {
    document.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(50.0, 50.0),
                rx: Length::from_mm(25.0),
                ry: Length::from_mm(25.0),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        },
        Angle::from_radians(0.0),
    )
}

#[test]
fn leaving_a_dimension_is_one_step_and_keeps_what_it_measured() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let before = document.own_step_count();
    let dimension = aligned(&document, path, 1);
    assert_eq!(document.own_step_count(), before + 1);
    assert_eq!(
        document.own_steps().last().unwrap().label,
        CREATE_DIMENSION_COMMIT_LABEL
    );
    assert_eq!(value(&document, dimension), (true, 12.5, 12.5));
}

#[test]
fn slots_that_do_not_resolve_are_refused() {
    let document = Document::new(1);
    let refused = document.add_dimension(
        DimensionKind::Aligned,
        DimensionSlots::Points(vec![
            PointSlot::Anchor {
                node: NodeId::from_parts(9, 9),
                anchor: aid(1),
            },
            PointSlot::Free {
                point: Point::new(0.0, 0.0),
            },
        ]),
        DimensionPlace::at_offset(Length::from_mm(5.0)),
    );
    assert_eq!(refused, Err(ObjectEditError::InvalidConversion));
}

#[test]
fn a_geometry_edit_refreshes_the_kept_value_in_the_same_step() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let dimension = aligned(&document, path, 1);
    let before = document.own_step_count();
    document
        .move_anchors(&[(path, aid(2), Point::new(20.0, 0.0))])
        .unwrap();
    assert_eq!(document.own_step_count(), before + 1, "no extra step");
    assert_eq!(value(&document, dimension), (true, 20.0, 20.0));
}

#[test]
fn deleting_the_measured_path_unlinks_and_undo_links_again() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let dimension = aligned(&document, path, 1);
    document.delete_objects(&[path]).unwrap();
    assert_eq!(value(&document, dimension), (false, 12.5, 12.5));
    let step = document.own_steps().last().unwrap().step;
    document.undo_step(step).unwrap();
    assert_eq!(value(&document, dimension), (true, 12.5, 12.5));
}

#[test]
fn a_node_delete_keeps_the_value_of_the_step_before() {
    let document = Document::new(1);
    let path = document.create_path(
        &[
            NewAnchor::corner(aid(1), Point::new(0.0, 0.0)),
            NewAnchor::corner(aid(2), Point::new(10.0, 0.0)),
            NewAnchor::corner(aid(3), Point::new(30.0, 0.0)),
        ],
        false,
    );
    let dimension = aligned(&document, path, 1);
    document.delete_anchors(&[(path, aid(2))]);
    assert_eq!(value(&document, dimension), (false, 10.0, 10.0));
}

#[test]
fn convert_to_path_of_a_measured_circle_unlinks() {
    let document = Document::new(1);
    let shape = circle(&document);
    let dimension = document
        .add_dimension(
            DimensionKind::Diameter,
            DimensionSlots::Curve(CurveSlot::Circle { node: shape }),
            DimensionPlace::at_offset(Length::from_mm(5.0)),
        )
        .unwrap();
    assert_eq!(value(&document, dimension), (true, 50.0, 50.0));
    let ObjectSnapshot::Primitive(primitive) = document.object(shape).unwrap() else {
        unreachable!()
    };
    let anchors: Vec<NewAnchor> = crate::outline_of(&primitive.shape)
        .anchors
        .iter()
        .enumerate()
        .map(|(i, a)| NewAnchor {
            id: aid(100 + i as u64),
            point: a.point,
            handle_in: a.handle_in,
            handle_out: a.handle_out,
            kind: a.kind,
        })
        .collect();
    document.convert_to_paths(&[(shape, anchors)]).unwrap();
    assert_eq!(value(&document, dimension), (false, 50.0, 50.0));
}

#[test]
fn a_non_uniform_scale_of_a_measured_circle_unlinks() {
    let document = Document::new(1);
    let shape = circle(&document);
    let dimension = document
        .add_dimension(
            DimensionKind::Radius,
            DimensionSlots::Curve(CurveSlot::Circle { node: shape }),
            DimensionPlace::at_offset(Length::from_mm(5.0)),
        )
        .unwrap();
    document
        .set_ellipse_frame(
            shape,
            EllipseFrame {
                center: Point::new(50.0, 50.0),
                rx: Length::from_mm(30.0),
                ry: Length::from_mm(25.0),
            },
        )
        .unwrap();
    assert_eq!(value(&document, dimension), (false, 25.0, 25.0));
}

#[test]
fn moving_the_line_and_text_is_one_step_move_dimension() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let dimension = aligned(&document, path, 1);
    let mut place = document.dimension(dimension).unwrap().place;
    place.along = 0.2;
    place.offset = Length::from_mm(-8.0);
    document.set_dimension_place(dimension, place).unwrap();
    assert_eq!(
        document.own_steps().last().unwrap().label,
        MOVE_DIMENSION_COMMIT_LABEL
    );
    assert_eq!(document.dimension(dimension).unwrap().place, place);
}

#[test]
fn a_copy_with_its_path_measures_the_copy_and_without_it_free_points() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let dimension = aligned(&document, path, 1);
    let both = document
        .duplicate_objects(
            &[
                crate::CopySource {
                    id: path,
                    anchor_ids: vec![aid(10), aid(11)],
                },
                crate::CopySource {
                    id: dimension,
                    anchor_ids: Vec::new(),
                },
            ],
            Vec2::new(0.0, 10.0),
        )
        .unwrap();
    let copy = document.dimension(both[1]).unwrap();
    assert!(copy.linked);
    assert_eq!(
        copy.slots,
        DimensionSlots::Points(vec![
            PointSlot::Anchor {
                node: both[0],
                anchor: aid(10)
            },
            PointSlot::Anchor {
                node: both[0],
                anchor: aid(11)
            },
        ])
    );
    let alone = document
        .duplicate_objects(
            &[crate::CopySource {
                id: dimension,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(0.0, 10.0),
        )
        .unwrap();
    let copy = document.dimension(alone[0]).unwrap();
    assert!(copy.linked);
    assert_eq!(
        copy.slots,
        DimensionSlots::Points(vec![
            PointSlot::Free {
                point: Point::new(0.0, 10.0)
            },
            PointSlot::Free {
                point: Point::new(12.5, 10.0)
            },
        ])
    );
}

#[test]
fn convert_replaces_the_dimension_by_a_group_of_paths_and_a_text() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let dimension = aligned(&document, path, 1);
    let groups = document
        .convert_dimensions(&[DimensionConversion {
            id: dimension,
            paths: vec![ConvertedPath {
                anchors: vec![
                    NewAnchor::corner(aid(50), Point::new(0.0, -5.0)),
                    NewAnchor::corner(aid(51), Point::new(12.5, -5.0)),
                ],
                closed: false,
                style: Style::default(),
            }],
            text: Some(TextSnapshot {
                id: NodeId::from_parts(0, 0),
                string: "12.5 mm".to_owned(),
                font: crate::text_model::FontRef {
                    family: "Source Sans 3".to_owned(),
                    style: "Regular".to_owned(),
                    face_hash: "ab".repeat(32),
                    face_index: 0,
                    kind: crate::text_model::FontKind::Outline,
                },
                size: Length::from_mm(3.0),
                layout: crate::text_model::TextLayoutValues::DEFAULT,
                chars: crate::text_chars::TextChars::default(),
                placement: crate::affine::Affine::translation(Vec2::new(4.0, -6.0)),
                style: Style::default(),
                outline: None,
                on_path: None,
                source: None,
            }),
        }])
        .unwrap();
    assert_eq!(
        document.own_steps().last().unwrap().label,
        CONVERT_DIMENSION_COMMIT_LABEL
    );
    assert!(document.dimension(dimension).is_none());
    assert!(document.is_group(groups[0]));
    assert_eq!(document.children(Some(groups[0])).len(), 2);
}

#[test]
fn a_dimension_is_a_guide_and_never_output() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let dimension = aligned(&document, path, 1);
    let read = document.read_nodes();
    let node = read.iter().find(|n| n.id == dimension).unwrap();
    assert!(node.guide);
    assert!(!node.is_output_node());
    assert!(read.iter().find(|n| n.id == path).unwrap().is_output_node());
    let json = String::from_utf8(document.export_json().unwrap()).unwrap();
    assert!(json.contains("\"shape\": \"dimension\""), "{json}");
}

#[test]
fn an_unlinked_dimension_moves_its_kept_points_but_never_its_value() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let dimension = aligned(&document, path, 1);
    document.delete_objects(&[path]).unwrap();
    document
        .translate_objects(&[dimension], Vec2::new(3.0, 4.0))
        .unwrap();
    let moved = document.dimension(dimension).unwrap();
    assert_eq!(moved.kept.points[0], Point::new(3.0, 4.0));
    assert_eq!(
        moved.kept.value,
        DimensionValue::Length(Length::from_mm(12.5))
    );
    let _: Measured = moved.kept;
}

#[test]
fn the_guide_button_shows_pressed_on_a_dimension_and_never_flags_it() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let dimension = aligned(&document, path, 1);
    let marking = document.guide_marking(&[dimension]);
    assert!(marking.enabled() && marking.all_guides());
    let before = document.own_step_count();
    document.set_guide(&[dimension], false).unwrap();
    assert!(document.dimension(dimension).is_some());
    assert_eq!(document.own_step_count(), before, "nothing written");
}

#[test]
fn a_kept_point_of_an_unlinked_dimension_moves_and_its_value_stays() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let dimension = aligned(&document, path, 1);
    assert_eq!(
        document.move_dimension_point(dimension, 1, Point::new(30.0, 0.0)),
        Err(ObjectEditError::NoSuchObject),
        "a linked dimension's points follow their geometry"
    );
    document.delete_objects(&[path]).unwrap();
    document
        .move_dimension_point(dimension, 1, Point::new(30.0, 5.0))
        .unwrap();
    assert_eq!(
        document.own_steps().last().unwrap().label,
        crate::dimension_model::EDIT_DIMENSION_NODES_COMMIT_LABEL
    );
    let moved = document.dimension(dimension).unwrap();
    assert_eq!(moved.kept.points[1], Point::new(30.0, 5.0));
    assert_eq!(value(&document, dimension), (false, 12.5, 12.5));
    assert!(
        document
            .move_dimension_point(dimension, 5, Point::new(0.0, 0.0))
            .is_err()
    );
}

fn aligned_to_free(document: &Document, path: NodeId, first: u64, free: Point) -> NodeId {
    document
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![
                PointSlot::Anchor {
                    node: path,
                    anchor: aid(first),
                },
                PointSlot::Free { point: free },
            ]),
            DimensionPlace::at_offset(Length::from_mm(5.0)),
        )
        .unwrap()
}

#[test]
fn a_linked_dimension_nudged_alone_writes_nothing() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let dimension = aligned(&document, path, 1);
    let free = aligned_to_free(&document, path, 1, Point::new(0.0, 20.0));
    for id in [dimension, free] {
        let before = document.own_step_count();
        let stored = document.dimension(id).unwrap();
        document
            .translate_objects(&[id], Vec2::new(3.0, 4.0))
            .unwrap();
        assert_eq!(document.own_step_count(), before, "no step is written");
        assert_eq!(document.dimension(id).unwrap(), stored);
        let mapped = stored.mapped_by(crate::affine::Affine::translation(Vec2::new(3.0, 4.0)));
        document
            .transform_objects(&[ObjectSnapshot::Dimension(Box::new(mapped))], false)
            .unwrap();
        assert_eq!(document.own_step_count(), before, "no transform step");
        assert_eq!(document.dimension(id).unwrap(), stored);
    }
    assert_eq!(value(&document, free), (true, 20.0, 20.0));
}

#[test]
fn a_linked_dimension_moved_with_its_referents_moves_its_free_slot() {
    let document = Document::new(1);
    let path = line(&document, 1, (0.0, 0.0), (12.5, 0.0));
    let free = aligned_to_free(&document, path, 1, Point::new(0.0, 20.0));
    document
        .translate_objects(&[path, free], Vec2::new(3.0, 4.0))
        .unwrap();
    let moved = document.dimension(free).unwrap();
    assert_eq!(
        moved.slots,
        DimensionSlots::Points(vec![
            PointSlot::Anchor {
                node: path,
                anchor: aid(1),
            },
            PointSlot::Free {
                point: Point::new(3.0, 24.0)
            },
        ])
    );
    assert_eq!(value(&document, free), (true, 20.0, 20.0));
}

#[test]
fn a_copied_diameter_without_its_circle_measures_a_free_circle_at_the_offset() {
    let document = Document::new(1);
    let shape = circle(&document);
    let dimension = document
        .add_dimension(
            DimensionKind::Diameter,
            DimensionSlots::Curve(CurveSlot::Circle { node: shape }),
            DimensionPlace::at_offset(Length::from_mm(5.0)),
        )
        .unwrap();
    let copies = document
        .duplicate_objects(
            &[crate::CopySource {
                id: dimension,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(10.0, 0.0),
        )
        .unwrap();
    let copy = document.dimension(copies[0]).unwrap();
    assert_eq!(
        copy.slots,
        DimensionSlots::Curve(CurveSlot::FreeCircle {
            center: Point::new(60.0, 50.0),
            radius: Length::from_mm(25.0),
        })
    );
    assert_eq!(value(&document, copies[0]), (true, 50.0, 50.0));
    assert_eq!(copy.shown.points[0], Point::new(60.0, 50.0));
    document
        .set_ellipse_frame(
            shape,
            EllipseFrame {
                center: Point::new(50.0, 50.0),
                rx: Length::from_mm(30.0),
                ry: Length::from_mm(30.0),
            },
        )
        .unwrap();
    assert_eq!(
        value(&document, copies[0]),
        (true, 50.0, 50.0),
        "not the original"
    );
    document
        .translate_objects(&copies, Vec2::new(0.0, 5.0))
        .unwrap();
    assert_eq!(
        document.dimension(copies[0]).unwrap().shown.points[0],
        Point::new(60.0, 55.0)
    );
}
