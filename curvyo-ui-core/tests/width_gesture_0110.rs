//! Width edits preview without mutating the document and commit once on release; the profile
//! keeps the centre line and follows the base width (`specs/0110` criteria 4, 5).
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]
use curvyo_document_core::{AnchorId, Document, Length, NewAnchor, Point, Vec2};
use curvyo_geometry_core::PathMeasure;
use curvyo_ui_core::{WidthGesture, WidthGesturePart};

fn line() -> PathMeasure {
    PathMeasure::new(
        &[
            (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::ZERO),
            (Point::new(10.0, 0.0), Vec2::ZERO, Vec2::ZERO),
        ],
        false,
    )
}

#[test]
fn changing_base_width_scales_the_outline_without_changing_profile_or_centreline() {
    use curvyo_document_core::StyleEdit;
    use curvyo_ui_core::verified_variable_stroke_artwork;
    let doc = Document::new(80);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(80, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(80, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(80, 3), 0.5, 1.5)
        .unwrap();
    doc.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(2.0)))
        .unwrap();
    let before = doc.path(id).unwrap();
    let outline_before = verified_variable_stroke_artwork(&before)
        .unwrap()
        .1
        .unwrap();
    doc.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(4.0)))
        .unwrap();
    let after = doc.path(id).unwrap();
    let outline_after = verified_variable_stroke_artwork(&after).unwrap().1.unwrap();
    let height = |p: &curvyo_document_core::PathSnapshot| {
        let ys = p.anchors.iter().map(|a| a.point.y);
        ys.clone().fold(f64::NEG_INFINITY, f64::max) - ys.fold(f64::INFINITY, f64::min)
    };
    assert!((height(&outline_before) - 3.0).abs() < 0.01);
    assert!((height(&outline_after) - 6.0).abs() < 0.01);
    assert_eq!(before.anchors, after.anchors);
    assert_eq!(before.width_points, after.width_points);
    let reopened =
        curvyo_document_core::unpack(81, &curvyo_document_core::pack(&doc, "test").unwrap())
            .unwrap();
    assert_eq!(reopened.path(id).unwrap(), after);
}

const THRESHOLD: Length = Length::from_mm(0.3);

#[test]
fn a_sideways_drag_sets_the_width_and_commits_one_undoable_change() {
    let doc = Document::new(1);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    let point_id = AnchorId::new(1, 3);
    let mut drag = WidthGesture::add(&line(), &[], point_id, Point::new(5.0, 0.0)).unwrap();
    let version = doc.version();
    drag.drag(
        &line(),
        Length::from_mm(2.0),
        Point::new(5.1, 3.0),
        THRESHOLD,
    )
    .unwrap();
    assert_eq!(drag.axis(), Some(WidthGesturePart::Width));
    // Relative to the press (UX review U3): the 2 mm stroke grows by twice the 3 mm the pointer
    // moved away from where it was pressed, to 8 mm, 4 × the base.
    assert!((drag.preview().multiplier - 4.0).abs() < 1e-9);
    assert!((drag.preview().position - 0.5).abs() < 1e-9);
    drag.drag(
        &line(),
        Length::from_mm(2.0),
        Point::new(9.0, 0.5),
        THRESHOLD,
    )
    .unwrap();
    assert!((drag.preview().multiplier - 1.5).abs() < 1e-9);
    assert!(
        (drag.preview().position - 0.5).abs() < 1e-9,
        "the axis stays"
    );
    // Back past the press point shrinks it: 2 - 2 × 0.5 = 1 mm.
    drag.drag(
        &line(),
        Length::from_mm(2.0),
        Point::new(9.0, -0.5),
        THRESHOLD,
    )
    .unwrap();
    assert!((drag.preview().multiplier - 0.5).abs() < 1e-9);
    drag.drag(
        &line(),
        Length::from_mm(2.0),
        Point::new(9.0, -5.0),
        THRESHOLD,
    )
    .unwrap();
    assert!(drag.preview().multiplier.abs() < 1e-9, "never below zero");
    drag.drag(
        &line(),
        Length::from_mm(2.0),
        Point::new(9.0, -0.5),
        THRESHOLD,
    )
    .unwrap();
    assert_eq!(doc.version(), version);
    drag.commit(&doc, id).unwrap();
    assert!((doc.width_points(id).unwrap()[0].multiplier - 0.5).abs() < 1e-9);
    doc.undo_step(doc.own_steps().last().unwrap().step).unwrap();
    assert!(doc.width_points(id).unwrap().is_empty());
}

/// UX review U3: the first sideways drag does not jump; it starts from the width there.
#[test]
fn a_sideways_drag_starts_from_the_width_at_the_press() {
    let point = curvyo_document_core::WidthPoint {
        id: AnchorId::new(1, 9),
        position: 0.5,
        multiplier: 1.5,
    };
    // Pressed on the diamond 0.2 mm off the line, the pointer pulled the other way.
    let mut drag = WidthGesture::existing(point, Point::new(5.0, 0.2));
    drag.drag(
        &line(),
        Length::from_mm(2.0),
        Point::new(5.0, -0.3),
        THRESHOLD,
    )
    .unwrap();
    assert_eq!(drag.axis(), Some(WidthGesturePart::Width));
    // 3 mm + 2 × 0.5 mm moved outward = 4 mm.
    assert!((drag.preview().multiplier - 2.0).abs() < 1e-9);
    drag.drag(
        &line(),
        Length::from_mm(2.0),
        Point::new(5.0, 0.2),
        THRESHOLD,
    )
    .unwrap();
    assert!(
        (drag.preview().multiplier - 1.5).abs() < 1e-9,
        "back at the press: the start width"
    );
}

#[test]
fn an_inserted_point_keeps_the_profile_and_a_drag_along_moves_it() {
    use curvyo_document_core::WidthPoint;
    let points = [
        WidthPoint {
            id: AnchorId::new(2, 1),
            position: 0.0,
            multiplier: 1.0,
        },
        WidthPoint {
            id: AnchorId::new(2, 2),
            position: 1.0,
            multiplier: 3.0,
        },
    ];
    let mut drag =
        WidthGesture::add(&line(), &points, AnchorId::new(2, 3), Point::new(5.0, 0.1)).unwrap();
    assert!((drag.preview().multiplier - 2.0).abs() < 1e-9);
    drag.drag(
        &line(),
        Length::from_mm(2.0),
        Point::new(8.0, -0.5),
        THRESHOLD,
    )
    .unwrap();
    assert_eq!(drag.axis(), Some(WidthGesturePart::Position));
    assert!((drag.preview().position - 0.8).abs() < 1e-6);
    assert!((drag.preview().multiplier - 2.0).abs() < 1e-9);
    let before = drag.preview();
    let mut width = WidthGesture::existing(before, Point::new(8.0, 0.0));
    assert!(
        width
            .drag(
                &line(),
                Length::from_mm(0.0),
                Point::new(8.0, 3.0),
                THRESHOLD
            )
            .is_none()
    );
    assert_eq!(width.preview(), before);
}

#[test]
fn a_press_without_movement_changes_nothing() {
    use curvyo_document_core::WidthPoint;
    let point = WidthPoint {
        id: AnchorId::new(4, 1),
        position: 0.5,
        multiplier: 2.0,
    };
    let mut drag = WidthGesture::existing(point, Point::new(5.0, 0.0));
    drag.drag(
        &line(),
        Length::from_mm(2.0),
        Point::new(5.1, 0.2),
        THRESHOLD,
    )
    .unwrap();
    assert_eq!(drag.axis(), None);
    assert_eq!(drag.preview(), point);
    assert!(drag.set_multiplier(-1.0).is_none());
    drag.set_multiplier(0.75).unwrap();
    assert!((drag.preview().multiplier - 0.75).abs() < 1e-12);
}

#[test]
fn width_hit_finds_the_diamond_on_the_centre_line() {
    use curvyo_document_core::WidthPoint;
    use curvyo_ui_core::width_hit;
    let points = [
        WidthPoint {
            id: AnchorId::new(3, 1),
            position: 0.5,
            multiplier: 2.0,
        },
        WidthPoint {
            id: AnchorId::new(3, 2),
            position: 0.9,
            multiplier: 1.0,
        },
    ];
    let path = line();
    let tolerance = Length::from_mm(0.2);
    assert_eq!(
        width_hit(&path, &points, Point::new(5.1, 0.1), tolerance),
        Some(points[0].id)
    );
    assert_eq!(
        width_hit(&path, &points, Point::new(9.0, 0.0), tolerance),
        Some(points[1].id)
    );
    assert_eq!(
        width_hit(&path, &points, Point::new(5.0, 2.0), tolerance),
        None
    );
}

#[test]
fn profiled_line_artwork_uses_a_filled_outline_and_preserves_its_centerline() {
    use curvyo_ui_core::variable_stroke_artwork;
    let doc = Document::new(90);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(90, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(90, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(90, 3), 0.0, 1.0)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(90, 4), 1.0, 2.0)
        .unwrap();
    let source = doc.path(id).unwrap();
    let (center, outline) = variable_stroke_artwork(&source).unwrap();
    let outline = outline.unwrap();
    assert_eq!(center.anchors, source.anchors);
    assert!(!center.style.stroke.enabled);
    assert!(outline.style.fill.enabled);
    assert!(!outline.style.stroke.enabled);
    assert_eq!(outline.style.fill.color, source.style.stroke.color);
    assert!(outline.closed);
    assert!(outline.anchors.len() >= 4);
}

#[test]
fn all_zero_profile_produces_no_stroke_artwork() {
    use curvyo_ui_core::variable_stroke_artwork;
    let doc = Document::new(91);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(91, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(91, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(91, 3), 0.5, 0.0)
        .unwrap();
    let (center, outline) = variable_stroke_artwork(&doc.path(id).unwrap()).unwrap();
    assert!(!center.style.stroke.enabled);
    assert!(outline.is_none());
}

#[test]
fn variable_stroke_hit_test_uses_the_filled_outline() {
    use curvyo_document_core::Tolerance;
    let doc = Document::new(92);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(92, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(92, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(92, 3), 0.5, 20.0)
        .unwrap();
    let read = doc.read_objects();
    assert_eq!(
        curvyo_ui_core::hit_test_object(
            &read.objects,
            Point::new(5.0, 1.0),
            Tolerance::from_mm(0.05)
        ),
        Some(id)
    );
    assert_eq!(
        curvyo_ui_core::hit_test_object(
            &read.objects,
            Point::new(5.0, 4.0),
            Tolerance::from_mm(0.05)
        ),
        None
    );
}

#[test]
fn zero_width_profile_without_fill_has_no_hit() {
    let doc = Document::new(96);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(96, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(96, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(96, 3), 0.5, 0.0)
        .unwrap();
    let objects = doc.read_objects().objects;
    assert_eq!(
        curvyo_ui_core::hit_test_object(
            &objects,
            Point::new(5.0, 0.0),
            curvyo_document_core::Tolerance::from_mm(0.05)
        ),
        None
    );
}

#[test]
fn stroke_to_path_unlinks_profiled_clone_without_changing_source() {
    let doc = Document::new(93);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(93, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(93, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    doc.set_width_point(id, AnchorId::new(93, 3), 0.5, 2.0)
        .unwrap();
    let clone = doc.linked_clone(&[id], &[]).unwrap()[0];
    let before = doc.path(id).unwrap();
    let objects = doc.read_objects().objects;
    let mut minter = curvyo_ui_core::AnchorIdMinter::new(94);
    let plan =
        curvyo_ui_core::plan_stroke_to_path(&objects, &[clone], &|_| false, &|_| None, &mut minter)
            .unwrap();
    doc.stroke_to_path(&plan.write).unwrap();
    assert_eq!(doc.path(id).unwrap(), before);
    let converted = doc.path(clone).unwrap();
    assert!(converted.width_points.is_empty());
    assert!(converted.style.fill.enabled);
    assert!(!converted.style.stroke.enabled);
    doc.undo_step(doc.own_steps().last().unwrap().step).unwrap();
    assert!(matches!(
        doc.object(clone),
        Some(curvyo_document_core::ObjectSnapshot::Clone(_))
    ));
    assert_eq!(doc.path(id).unwrap(), before);
}
