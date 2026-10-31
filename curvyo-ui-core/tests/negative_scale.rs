//! `curvyo-ui-core` tests of `specs/0056-mirror-and-negative-scale/` Part B and the
//! zero rule of Part C: a resize drag passes through zero in every mode and turns the
//! objects over, a size that would land on 0 becomes 0.01 mm, and the result depends
//! only on the pointer, the press state and the sign of the last non-zero frame. Scale is
//! 4 px/mm, so the 16 px handle radius is 4 mm.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{
    AnchorId, Angle, Document, Length, NewAnchor, NodeId, ObjectSnapshot, Point, PrimitiveSnapshot,
    RectBounds, Shape, Tolerance, Vec2,
};
use curvyo_ui_core::{
    AnchorIdMinter, ExtentSigns, Modifiers, ObjectSelection, ResizeDirection, SelectTool,
    TransformHandleTolerances, ZeroRecovery, resize_local_box, resize_local_box_with_prior,
};

const SCALE: f64 = 4.0;
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::new(true, false);
const CTRL: Modifiers = Modifiers::new(false, true);
const EPS: f64 = 1e-9;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

struct Rig {
    document: Document,
    selection: ObjectSelection,
    tool: SelectTool,
}

impl Rig {
    fn new() -> Self {
        Self {
            document: Document::new(1),
            selection: ObjectSelection::new(),
            tool: SelectTool::new(),
        }
    }

    fn rect(&self, x: f64, y: f64, w: f64, h: f64) -> NodeId {
        self.document.create_rect(RectBounds {
            origin: pt(x, y),
            width: Length::from_mm(w),
            height: Length::from_mm(h),
        })
    }

    /// The closed path with corner nodes (0, 0), (30, 0), (0, 10) of criteria 6 and 8.
    fn triangle(&self) -> NodeId {
        let corner = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), pt(x, y));
        self.document.create_path(
            &[
                corner(1, 0.0, 0.0),
                corner(2, 30.0, 0.0),
                corner(3, 0.0, 10.0),
            ],
            true,
        )
    }

    fn objects(&self) -> Vec<ObjectSnapshot> {
        self.document
            .object_ids()
            .into_iter()
            .filter_map(|id| self.document.object(id))
            .collect()
    }

    fn select(&mut self, ids: &[NodeId]) {
        self.selection.set(ids);
    }

    /// A press at `from`, a frame at every point of `frames`, a release at the last one.
    fn drag_through(
        &mut self,
        from: Point,
        frames: &[Point],
        modifiers: Modifiers,
    ) -> ZeroRecovery {
        let objects = self.objects();
        self.tool.pointer_down(
            &objects,
            &mut self.selection,
            from,
            Tolerance::from_mm(2.0),
            TransformHandleTolerances::at_scale(SCALE),
            modifiers,
        );
        for &frame in frames {
            self.tool
                .pointer_moved(frame, modifiers, &mut self.selection);
        }
        let to = *frames.last().expect("a frame");
        self.tool.pointer_up(
            &self.document,
            &objects,
            &mut self.selection,
            to,
            modifiers,
            &mut AnchorIdMinter::new(9),
        )
    }

    fn drag(&mut self, from: Point, to: Point, modifiers: Modifiers) -> ZeroRecovery {
        self.drag_through(
            from,
            &[
                pt(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y)),
                to,
            ],
            modifiers,
        )
    }

    fn bounds(&self, id: NodeId) -> RectBounds {
        let Some(ObjectSnapshot::Primitive(PrimitiveSnapshot {
            shape: Shape::Rect { bounds, .. },
            ..
        })) = self.document.object(id)
        else {
            panic!("a rectangle");
        };
        bounds
    }

    fn nodes(&self, id: NodeId) -> Vec<Point> {
        let Some(ObjectSnapshot::Path(path)) = self.document.object(id) else {
            panic!("a path");
        };
        path.anchors.iter().map(|a| a.point).collect()
    }
}

fn assert_nodes(actual: &[Point], expected: &[(f64, f64)]) {
    assert_eq!(actual.len(), expected.len());
    for (a, &(x, y)) in actual.iter().zip(expected) {
        assert!(
            (a.x - x).abs() < 1e-6 && (a.y - y).abs() < 1e-6,
            "{actual:?} vs {expected:?}"
        );
    }
}

fn assert_rect(bounds: RectBounds, (x0, y0, x1, y1): (f64, f64, f64, f64)) {
    let (ax1, ay1) = (
        bounds.origin.x + bounds.width.as_mm(),
        bounds.origin.y + bounds.height.as_mm(),
    );
    assert!(
        (bounds.origin.x - x0).abs() < 1e-6
            && (bounds.origin.y - y0).abs() < 1e-6
            && (ax1 - x1).abs() < 1e-6
            && (ay1 - y1).abs() < 1e-6,
        "{bounds:?} vs {:?}",
        (x0, y0, x1, y1)
    );
}

/// Criterion 6: a free corner drag across both lines through the fixed corner is a half
/// turn; across one of them a mirror of that axis.
#[test]
fn a_free_corner_drag_turns_the_path_over() {
    let mut rig = Rig::new();
    let id = rig.triangle();
    rig.select(&[id]);
    rig.drag(pt(30.0, 10.0), pt(-30.0, -10.0), NONE);
    assert_nodes(&rig.nodes(id), &[(0.0, 0.0), (-30.0, 0.0), (0.0, -10.0)]);

    let mut rig = Rig::new();
    let id = rig.triangle();
    rig.select(&[id]);
    rig.drag(pt(30.0, 10.0), pt(-30.0, 20.0), NONE);
    assert_nodes(&rig.nodes(id), &[(0.0, 0.0), (-30.0, 0.0), (0.0, 20.0)]);
}

/// Criteria 7 and 11: an edge handle dragged past the opposite edge mirrors one axis;
/// the rectangle stays a rectangle, and the handle now at the left is the W handle, with
/// the right edge fixed.
#[test]
fn an_edge_drag_through_the_opposite_edge_and_the_handle_by_position() {
    let mut rig = Rig::new();
    let id = rig.rect(0.0, 0.0, 30.0, 10.0);
    rig.select(&[id]);
    rig.drag(pt(30.0, 5.0), pt(-10.0, 5.0), NONE);
    assert_rect(rig.bounds(id), (-10.0, 0.0, 0.0, 10.0));
    rig.drag(pt(-10.0, 5.0), pt(-5.0, 5.0), NONE);
    assert_rect(rig.bounds(id), (-5.0, 0.0, 0.0, 10.0));
}

/// Criterion 8: Ctrl keeps the proportions with the magnitude of the dominant axis, and
/// each axis keeps the sign of its free factor.
#[test]
fn ctrl_keeps_the_proportions_and_the_signs_follow_the_pointer() {
    let mut rig = Rig::new();
    let id = rig.triangle();
    rig.select(&[id]);
    rig.drag(pt(30.0, 10.0), pt(-15.0, 2.0), CTRL);
    assert_nodes(&rig.nodes(id), &[(0.0, 0.0), (-15.0, 0.0), (0.0, 5.0)]);
}

/// Criterion 9: Shift mirrors about the centre line of the axis.
#[test]
fn shift_mirrors_about_the_centre() {
    let mut rig = Rig::new();
    let id = rig.rect(0.0, 0.0, 30.0, 10.0);
    rig.select(&[id]);
    rig.drag(pt(30.0, 5.0), pt(-10.0, 5.0), SHIFT);
    assert_rect(rig.bounds(id), (-10.0, 0.0, 40.0, 10.0));
}

/// Criterion 12: one rotated rectangle resizes along its own axes; `rotation` stays and
/// the centre moves along its own x axis.
#[test]
fn a_rotated_rectangle_turns_over_along_its_own_axis() {
    let mut rig = Rig::new();
    let id = rig.rect(0.0, 0.0, 20.0, 10.0);
    let turned = rig
        .document
        .object(id)
        .unwrap()
        .rotated(pt(10.0, 5.0), Angle::from_radians(30.0_f64.to_radians()));
    rig.document.rotate_object(&turned).unwrap();
    rig.select(&[id]);
    let u = Vec2::new(30.0_f64.to_radians().cos(), 30.0_f64.to_radians().sin());
    let e = pt(10.0 + 10.0 * u.x, 5.0 + 10.0 * u.y);
    let to = pt(10.0 - 20.0 * u.x, 5.0 - 20.0 * u.y);
    rig.drag(e, to, NONE);
    let Some(ObjectSnapshot::Primitive(primitive)) = rig.document.object(id) else {
        panic!("still a rectangle");
    };
    assert!((primitive.rotation.as_radians() - 30.0_f64.to_radians()).abs() < 1e-12);
    let Shape::Rect { bounds, .. } = primitive.shape else {
        panic!("still a rectangle");
    };
    assert!((bounds.width.as_mm() - 10.0).abs() < EPS);
    let centre = pt(
        bounds.origin.x + bounds.width.as_mm() / 2.0,
        bounds.origin.y + bounds.height.as_mm() / 2.0,
    );
    assert!((centre.x - (10.0 - 15.0 * u.x)).abs() < 1e-9, "{centre:?}");
    assert!((centre.y - (5.0 - 15.0 * u.y)).abs() < 1e-9, "{centre:?}");
    assert!((centre.x + 2.99).abs() < 0.01 && (centre.y + 2.50).abs() < 0.01);
}

/// Criteria 12 and 18: a multi-selection turned over by a free corner drag across both
/// lines is a half turn of every object about the fixed corner, in the document axes.
#[test]
fn a_group_half_turn_maps_every_object() {
    let mut rig = Rig::new();
    let a = rig.rect(0.0, 0.0, 10.0, 10.0);
    let b = rig.rect(20.0, 0.0, 10.0, 10.0);
    rig.select(&[a, b]);
    rig.drag(pt(30.0, 10.0), pt(-30.0, -10.0), NONE);
    assert_rect(rig.bounds(a), (-10.0, -10.0, 0.0, 0.0));
    assert_rect(rig.bounds(b), (-30.0, -10.0, -20.0, 0.0));
}

/// Criterion 13: beyond the coordinate limit on the negative side, nothing changes.
#[test]
fn a_negative_scale_beyond_the_limit_changes_nothing() {
    let mut rig = Rig::new();
    let id = rig.rect(0.0, 0.0, 30.0, 10.0);
    rig.select(&[id]);
    let before = rig.objects();
    rig.drag(pt(30.0, 5.0), pt(-2.0e9, 5.0), NONE);
    assert_eq!(rig.objects(), before);
}

/// Criterion 15 and 28 h: a release on the fixed edge after a positive frame gives
/// +0.01 mm, after a negative frame −0.01 mm, a first frame on 0 +0.01 mm; the release
/// reports the axis for the notice.
#[test]
fn a_drag_that_lands_on_zero_keeps_the_sign_of_the_frame_before() {
    let mut rig = Rig::new();
    let id = rig.rect(0.0, 0.0, 30.0, 10.0);
    rig.select(&[id]);
    let recovery = rig.drag_through(pt(30.0, 5.0), &[pt(5.0, 5.0), pt(0.0, 5.0)], NONE);
    assert_eq!(
        recovery,
        ZeroRecovery {
            width: true,
            height: false
        }
    );
    assert_rect(rig.bounds(id), (0.0, 0.0, 0.01, 10.0));

    let mut rig = Rig::new();
    let id = rig.rect(0.0, 0.0, 30.0, 10.0);
    rig.select(&[id]);
    rig.drag_through(pt(30.0, 5.0), &[pt(-4.0, 5.0), pt(0.0, 5.0)], NONE);
    assert_rect(rig.bounds(id), (-0.01, 0.0, 0.0, 10.0));

    let mut rig = Rig::new();
    let id = rig.rect(0.0, 0.0, 30.0, 10.0);
    rig.select(&[id]);
    rig.drag_through(pt(30.0, 5.0), &[pt(0.0, 5.0)], NONE);
    assert_rect(rig.bounds(id), (0.0, 0.0, 0.01, 10.0));

    // A frame on 0 that moves on: no recovery, mirrored 4 mm wide.
    let mut rig = Rig::new();
    let id = rig.rect(0.0, 0.0, 30.0, 10.0);
    rig.select(&[id]);
    let recovery = rig.drag_through(
        pt(30.0, 5.0),
        &[pt(5.0, 5.0), pt(0.0, 5.0), pt(-4.0, 5.0)],
        NONE,
    );
    assert!(!recovery.any());
    assert_rect(rig.bounds(id), (-4.0, 0.0, 0.0, 10.0));
}

/// Criterion 15 with Ctrl and Shift: Ctrl sets the dominant axis to 0.01 mm and the
/// other follows by the common factor; Shift sets the symmetric extent about the centre.
#[test]
fn zero_recovery_under_ctrl_and_shift() {
    let mut rig = Rig::new();
    let id = rig.rect(0.0, 0.0, 30.0, 10.0);
    rig.select(&[id]);
    let recovery = rig.drag(pt(30.0, 10.0), pt(0.0, 9.0), CTRL);
    assert_eq!(
        recovery,
        ZeroRecovery {
            width: true,
            height: false
        }
    );
    assert_rect(rig.bounds(id), (0.0, 0.0, 0.01, 10.0 * 0.01 / 30.0));

    let mut rig = Rig::new();
    let id = rig.rect(0.0, 0.0, 30.0, 10.0);
    rig.select(&[id]);
    rig.drag(pt(30.0, 5.0), pt(15.0, 5.0), SHIFT);
    assert_rect(rig.bounds(id), (14.995, 0.0, 15.005, 10.0));

    let both = resize_local_box(
        pt(0.0, 0.0),
        pt(30.0, 10.0),
        ResizeDirection::Se,
        Vec2::new(-15.0, -3.0),
        true,
        true,
    );
    assert!((both.max.x - both.min.x - 0.01).abs() < 1e-12);
    assert!((both.sx.abs() - both.sy.abs()).abs() < 1e-12);
    assert!(both.recovered.width && !both.recovered.height);
}

/// Criterion 28 h: the thresholds of the zero rule, and an axis whose start extent is 0
/// or already under 0.0005 mm is never touched.
#[test]
fn the_thresholds_of_the_zero_rule() {
    let width_after = |delta: f64| {
        let r = resize_local_box(
            pt(0.0, 0.0),
            pt(30.0, 10.0),
            ResizeDirection::E,
            Vec2::new(delta, 0.0),
            false,
            false,
        );
        (r.max.x - r.min.x, r.recovered.width)
    };
    assert_eq!(width_after(-30.0), (0.01, true));
    let (w, recovered) = width_after(-30.0 + 0.0004);
    assert!((w - 0.01).abs() < 1e-12 && recovered);
    let (w, recovered) = width_after(-30.0 + 0.0006);
    assert!((w - 0.0006).abs() < 1e-9 && !recovered);
    let line = resize_local_box(
        pt(0.0, 0.0),
        pt(30.0, 0.0),
        ResizeDirection::Se,
        Vec2::new(0.0, 5.0),
        false,
        false,
    );
    assert_eq!(line.sy, 1.0);
    assert!(!line.recovered.height);
    let thin = resize_local_box(
        pt(0.0, 0.0),
        pt(0.0004, 10.0),
        ResizeDirection::E,
        Vec2::new(-0.0004, 0.0),
        false,
        false,
    );
    assert!(!thin.recovered.width);
    let negative = resize_local_box_with_prior(
        pt(0.0, 0.0),
        pt(30.0, 10.0),
        ResizeDirection::E,
        Vec2::new(-30.0, 0.0),
        (false, false),
        ExtentSigns {
            x_negative: true,
            y_negative: false,
        },
    );
    assert!((negative.sx * 30.0 + 0.01).abs() < 1e-12);
}

/// Criterion 28 i: a 0.01 mm axis pulls apart like any other, through the fixed line and
/// back any number of times.
#[test]
fn a_hundredth_of_a_millimetre_pulls_apart_again() {
    let pulled = resize_local_box(
        pt(0.0, 0.0),
        pt(0.01, 10.0),
        ResizeDirection::E,
        Vec2::new(20.0, 0.0),
        false,
        false,
    );
    assert!((pulled.max.x - pulled.min.x - 20.01).abs() < 1e-9);
    let (mut min, mut max) = (pt(0.0, 0.0), pt(0.01, 10.0));
    for (delta, expected) in [(-20.01, -20.0), (40.0, 20.0), (-25.0, -5.0)] {
        // The handle on the side away from the fixed left edge x = 0.
        let direction = if max.x > 0.0 {
            ResizeDirection::E
        } else {
            ResizeDirection::W
        };
        let r = resize_local_box(min, max, direction, Vec2::new(delta, 0.0), false, false);
        let signed = if r.max.x > 0.0 { r.max.x } else { r.min.x };
        assert!((signed - expected).abs() < 1e-9, "{r:?}");
        (min, max) = (r.min, r.max);
    }
}

/// Criteria 10 and 28 g: a drag that crosses zero and comes back to its start writes
/// nothing; one that comes back elsewhere equals the direct drag there.
#[test]
fn a_drag_back_across_zero_leaves_no_residue() {
    let mut rig = Rig::new();
    let id = rig.triangle();
    rig.select(&[id]);
    let before = rig.objects();
    rig.drag_through(
        pt(30.0, 10.0),
        &[pt(-20.0, -5.0), pt(-1.0, 3.0), pt(30.0, 10.0)],
        NONE,
    );
    assert_eq!(rig.objects(), before);

    let mut direct = Rig::new();
    let d = direct.triangle();
    direct.select(&[d]);
    direct.drag(pt(30.0, 10.0), pt(40.0, 12.0), NONE);
    rig.drag_through(pt(30.0, 10.0), &[pt(-20.0, -5.0), pt(40.0, 12.0)], NONE);
    assert_nodes(
        &rig.nodes(id),
        &direct
            .nodes(d)
            .iter()
            .map(|p| (p.x, p.y))
            .collect::<Vec<_>>(),
    );
}

/// Criterion 28 j: the same event sequence, a frame on 0 included, gives the same bytes
/// twice, and the preview of the last frame is the commit.
#[test]
fn the_same_events_give_the_same_bytes_and_the_preview_is_the_commit() {
    let run = || {
        let mut rig = Rig::new();
        let a = rig.rect(0.0, 0.0, 30.0, 10.0);
        let b = rig.triangle();
        rig.select(&[a, b]);
        let objects = rig.objects();
        rig.tool.pointer_down(
            &objects,
            &mut rig.selection,
            pt(30.0, 10.0),
            Tolerance::from_mm(2.0),
            TransformHandleTolerances::at_scale(SCALE),
            NONE,
        );
        for frame in [pt(-5.0, 4.0), pt(0.0, 4.0)] {
            rig.tool.pointer_moved(frame, NONE, &mut rig.selection);
        }
        let preview = rig
            .tool
            .live_edit(&objects, &rig.selection, pt(0.0, 4.0), false, false)
            .expect("a preview")
            .objects;
        rig.tool.pointer_up(
            &rig.document,
            &objects,
            &mut rig.selection,
            pt(0.0, 4.0),
            NONE,
            &mut AnchorIdMinter::new(9),
        );
        let committed = rig.objects();
        assert_eq!(preview, committed);
        format!("{committed:?}")
    };
    assert_eq!(run(), run());
}

fn stroke_of(rig: &Rig, id: NodeId) -> f64 {
    match rig.document.object(id).unwrap() {
        ObjectSnapshot::Sketch(_) => unreachable!("no sketch in this test"),
        ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Primitive(p) => p.style.stroke.width.as_mm(),
        ObjectSnapshot::Path(p) => p.style.stroke.width.as_mm(),
        ObjectSnapshot::Clone(_) => panic!("no stroke"),
    }
}

fn is_polygon(rig: &Rig, id: NodeId) -> bool {
    matches!(
        rig.document.object(id),
        Some(ObjectSnapshot::Primitive(PrimitiveSnapshot {
            shape: Shape::Polygon { .. },
            ..
        }))
    )
}

/// Criteria 18 and 23: a scale whose magnitudes are equal converts nothing and leaves
/// the stroke at √|sx·sy| = 1; unequal magnitudes convert the polygon as a positive
/// stretch does and scale the stroke by √|sx·sy|.
#[test]
fn stretch_and_stroke_are_decided_on_the_magnitudes() {
    use curvyo_document_core::{PointCount, StarFrame};
    use curvyo_ui_core::StrokeScaling;
    for (to_x, converts, stroke_factor) in [(-30.0, false, 1.0), (-60.0, true, 2.0_f64.sqrt())] {
        let mut rig = Rig::new();
        rig.tool.set_stroke_scaling(StrokeScaling::Proportional);
        let rect = rig.rect(0.0, 0.0, 10.0, 10.0);
        let polygon = rig.document.create_polygon(
            StarFrame {
                center: pt(25.0, 5.0),
                radius: Length::from_mm(5.0),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(5).unwrap(),
        );
        rig.select(&[rect, polygon]);
        let objects = rig.objects();
        let group_box_right = curvyo_ui_core::SelectTool::group_of(&objects, &rig.selection)
            .unwrap()
            .bounds()
            .max
            .x;
        let before = stroke_of(&rig, rect);
        rig.drag(pt(group_box_right, 5.0), pt(to_x, 5.0), NONE);
        assert_eq!(is_polygon(&rig, polygon), !converts, "to {to_x}");
        assert!(
            (stroke_of(&rig, rect) - before * stroke_factor).abs() < 1e-9,
            "to {to_x}"
        );
    }
}

/// The grabbed handle, by position after the crossing (`review-ux.md` finding 5): the E
/// handle dragged past the W edge is the W handle at the pointer; a corner past both lines
/// is the opposite corner.
#[test]
fn the_grabbed_handle_by_position_through_zero() {
    use curvyo_ui_core::EditHandle;
    let mut rig = Rig::new();
    let id = rig.rect(0.0, 0.0, 30.0, 10.0);
    rig.select(&[id]);
    let objects = rig.objects();
    rig.tool.pointer_down(
        &objects,
        &mut rig.selection,
        pt(30.0, 5.0),
        Tolerance::from_mm(2.0),
        TransformHandleTolerances::at_scale(SCALE),
        NONE,
    );
    rig.tool
        .pointer_moved(pt(-10.0, 5.0), NONE, &mut rig.selection);
    assert_eq!(
        rig.tool.dragging_handle_at(pt(-10.0, 5.0), false, false),
        Some(EditHandle::Resize(ResizeDirection::W))
    );
    assert_eq!(
        rig.tool.dragging_handle_at(pt(20.0, 5.0), false, false),
        Some(EditHandle::Resize(ResizeDirection::E))
    );
    rig.tool.escape();
    rig.tool.pointer_down(
        &objects,
        &mut rig.selection,
        pt(30.0, 10.0),
        Tolerance::from_mm(2.0),
        TransformHandleTolerances::at_scale(SCALE),
        NONE,
    );
    rig.tool
        .pointer_moved(pt(-5.0, -5.0), NONE, &mut rig.selection);
    assert_eq!(
        rig.tool.dragging_handle_at(pt(-5.0, -5.0), false, false),
        Some(EditHandle::Resize(ResizeDirection::Nw))
    );
}
