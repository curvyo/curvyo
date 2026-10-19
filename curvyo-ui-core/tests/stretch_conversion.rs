//! `curvyo-ui-core` tests of the skew and the stretch that convert shapes to paths,
//! silently (`specs/0047-transform-polish/` criteria 2 to 12; before it
//! `specs/0019-multi-object-transform/` criteria 18 to 20, 32, 34 and 53 to 56):
//! which gestures convert, what the converted path is, and that the preview is the
//! commit. Scale is 4 px/mm.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]
#![allow(clippy::too_many_lines)]

use curvyo_document_core::{
    Angle, Document, EllipseFrame, InnerRatio, Length, NodeId, ObjectSnapshot, Point, PointCount,
    PrimitiveSnapshot, RectBounds, Shape, StarFrame, Tolerance,
};
use curvyo_ui_core::{
    AnchorIdMinter, EditHandle, EntryKey, EntryOutcome, Modifiers, ObjectSelection, PressTarget,
    ResizeDirection, SelectTool, Side, TransformHandleTolerances, classify_press,
};

const SCALE: f64 = 4.0;
const NONE: Modifiers = Modifiers::NONE;
const CTRL: Modifiers = Modifiers::new(false, true);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn tolerances() -> TransformHandleTolerances {
    TransformHandleTolerances::at_scale(SCALE)
}

fn tol() -> Tolerance {
    Tolerance::from_mm(2.0)
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

    /// A 5-point star with its leftmost point at `left`.
    fn star(&self, left: f64, radius: f64) -> NodeId {
        self.document.create_star(
            StarFrame {
                center: pt(left + radius, 30.0),
                radius: Length::from_mm(radius),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(5).unwrap(),
            InnerRatio::new(0.5).unwrap(),
        )
    }

    fn turned(&self, id: NodeId, degrees: f64) {
        let object = self.document.object(id).unwrap();
        let centre = pt(
            f64::midpoint(object_bounds(&object).0.x, object_bounds(&object).1.x),
            f64::midpoint(object_bounds(&object).0.y, object_bounds(&object).1.y),
        );
        let rotated = object.rotated(centre, Angle::from_radians(degrees.to_radians()));
        self.document.rotate_object(&rotated).unwrap();
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

    fn group_box(&self) -> (Point, Point) {
        let group = SelectTool::group_of(&self.objects(), &self.selection).unwrap();
        (group.bounds().min, group.bounds().max)
    }

    fn east_edge(&self) -> Point {
        let (min, max) = self.group_box();
        pt(max.x, f64::midpoint(min.y, max.y))
    }

    fn south_east_corner(&self) -> Point {
        self.group_box().1
    }

    fn press(&mut self, at: Point, modifiers: Modifiers) {
        let objects = self.objects();
        self.tool.pointer_down(
            &objects,
            &mut self.selection,
            at,
            tol(),
            tolerances(),
            modifiers,
        );
        self.tool.mint_conversion_ids(&mut AnchorIdMinter::new(9));
    }

    fn release(&mut self, to: Point, modifiers: Modifiers) {
        let objects = self.objects();
        self.tool.pointer_up(
            &self.document,
            &objects,
            &mut self.selection,
            to,
            modifiers,
            &mut AnchorIdMinter::new(9),
        );
    }

    fn drag(&mut self, from: Point, to: Point, modifiers: Modifiers) {
        self.press(from, modifiers);
        self.tool.pointer_moved(
            pt(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y)),
            modifiers,
            &mut self.selection,
        );
        self.tool.pointer_moved(to, modifiers, &mut self.selection);
        self.release(to, modifiers);
    }

    fn live(&self, to: Point, modifiers: Modifiers) -> Option<Vec<ObjectSnapshot>> {
        self.tool
            .live_edit(
                &self.objects(),
                &self.selection,
                to,
                modifiers.shift,
                modifiers.ctrl,
            )
            .map(|live| live.objects)
    }

    fn target(&self, at: Point) -> PressTarget {
        let objects = self.objects();
        classify_press(
            &objects,
            &objects,
            &self.selection,
            at,
            tol(),
            tolerances(),
            NONE,
        )
    }
}

fn object_bounds(object: &ObjectSnapshot) -> (Point, Point) {
    let group = SelectTool::group_of(std::slice::from_ref(object), &{
        let mut s = ObjectSelection::new();
        s.set(&[object.id()]);
        s
    });
    // A single object has no group: fall back to its drawn bounds.
    group.map_or_else(
        || curvyo_ui_core::object_outline_bounds(object),
        |g| (g.bounds().min, g.bounds().max),
    )
}

fn is_path(document: &Document, id: NodeId) -> bool {
    document.path(id).is_some()
}

/// The criterion 20 example: a 5-point star at the left (its leftmost point at
/// x = 0) and an unrotated rectangle from (35, 0) to (45, 10): the group box is
/// x = 0 to 45; the right edge handle dragged to x = 90 gives sx = 2, sy = 1.
fn star_and_rect() -> (Rig, NodeId, NodeId) {
    let rig = Rig::new();
    let star = rig.star(0.0, 10.0);
    let rect = rig.rect(35.0, 20.0, 10.0, 10.0);
    (rig, star, rect)
}

/// Criteria 20 and 53: an edge drag stretches the rectangle as a rectangle and turns
/// the star into a closed path of ten nodes, in place, with its style.
#[test]
fn an_edge_stretch_converts_the_star_and_keeps_the_rectangle() {
    let (mut rig, star, rect) = star_and_rect();
    rig.select(&[star, rect]);
    let before = rig.document.object(star).unwrap();
    let ids_before = rig.document.object_ids();
    let east = rig.east_edge();
    assert_eq!(
        rig.target(east),
        PressTarget::Handle(EditHandle::Resize(ResizeDirection::E))
    );
    let (min, max) = rig.group_box();
    rig.drag(east, pt(min.x + (max.x - min.x) * 2.0, east.y), NONE);

    let ObjectSnapshot::Path(path) = rig.document.object(star).unwrap() else {
        panic!("the star is a path now");
    };
    assert_eq!(path.anchors.len(), 10);
    assert!(path.closed);
    assert_eq!(&path.style, before.style(), "same style");
    assert_eq!(rig.document.object_ids(), ids_before, "same place");
    // Twice its distance from the left edge, the same y.
    let ObjectSnapshot::Primitive(PrimitiveSnapshot {
        shape: Shape::Rect { bounds, .. },
        ..
    }) = rig.document.object(rect).unwrap()
    else {
        panic!("still a rectangle");
    };
    assert!((bounds.width.as_mm() - 20.0).abs() < 1e-6);
    assert!((bounds.height.as_mm() - 10.0).abs() < 1e-6);
    let (low, high) = curvyo_ui_core::object_outline_bounds(&rig.document.object(star).unwrap());
    let (b_low, b_high) = curvyo_ui_core::object_outline_bounds(&before);
    // About the left side of the group box (the star's leftmost point).
    assert!(
        (low.x - b_low.x).abs() < 1e-6
            && (high.x - (b_low.x + 2.0 * (b_high.x - b_low.x))).abs() < 1e-6
    );
    assert!((low.y - b_low.y).abs() < 1e-6 && (high.y - b_high.y).abs() < 1e-6);
}

/// `0047` criterion 7: a corner drag of a selection that holds a star is free like any
/// other and turns the star into a path; with Ctrl it keeps one factor and converts
/// nothing (criterion 10).
#[test]
fn a_corner_drag_is_free_and_ctrl_keeps_the_proportions() {
    for modifiers in [NONE, CTRL] {
        let (mut rig, star, rect) = star_and_rect();
        rig.select(&[star, rect]);
        let corner = rig.south_east_corner();
        // A very uneven drag.
        rig.drag(corner, pt(corner.x + 40.0, corner.y + 2.0), modifiers);
        let Shape::Rect { bounds, .. } = (match rig.document.object(rect).unwrap() {
            ObjectSnapshot::Clone(_) => panic!("no clone here"),
            ObjectSnapshot::Primitive(p) => p.shape,
            ObjectSnapshot::Path(_) => panic!(),
        }) else {
            panic!()
        };
        assert!(bounds.width.as_mm() > 10.0);
        if modifiers == CTRL {
            assert!(!is_path(&rig.document, star), "a uniform scale keeps it");
            assert!(
                (bounds.width.as_mm() - bounds.height.as_mm()).abs() < 1e-6,
                "one factor for both axes"
            );
        } else {
            assert!(is_path(&rig.document, star), "a stretch converts it");
            assert!(
                (bounds.width.as_mm() - bounds.height.as_mm()).abs() > 1.0,
                "two factors"
            );
        }
        assert!(rig.document.primitive(rect).is_some());
    }
}

/// Criterion 18: a selection with no converting shape stretches freely at a corner.
#[test]
fn a_selection_without_converting_shapes_stretches_freely_at_a_corner() {
    let mut rig = Rig::new();
    let a = rig.rect(0.0, 0.0, 10.0, 10.0);
    let b = rig.rect(20.0, 0.0, 10.0, 10.0);
    rig.select(&[a, b]);
    let corner = rig.south_east_corner();
    rig.drag(corner, pt(corner.x + 30.0, corner.y + 20.0), NONE);
    let (min, max) = rig.group_box();
    assert!((max.x - min.x - 60.0).abs() < 1e-6 && (max.y - min.y - 30.0).abs() < 1e-6);
    assert!(rig.document.primitive(a).is_some() && rig.document.primitive(b).is_some());
}

/// Criterion 20: a circle becomes an ellipse, never a path, and is not counted.
#[test]
fn a_stretched_circle_becomes_an_ellipse_not_a_path() {
    let mut rig = Rig::new();
    let circle = rig.document.create_ellipse(EllipseFrame {
        center: pt(10.0, 10.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(10.0),
    });
    let rect = rig.rect(30.0, 0.0, 10.0, 20.0);
    rig.select(&[circle, rect]);
    let east = rig.east_edge();
    rig.drag(east, pt(east.x + 40.0, east.y), NONE);
    assert!(rig.document.primitive(circle).is_some(), "an ellipse");
    let ObjectSnapshot::Primitive(PrimitiveSnapshot {
        shape: Shape::Ellipse { frame, .. },
        rotation,
        ..
    }) = rig.document.object(circle).unwrap()
    else {
        panic!("an ellipse");
    };
    assert!(
        (frame.rx.as_mm() - frame.ry.as_mm()).abs() > 1.0,
        "no longer a circle"
    );
    assert_eq!(rotation.as_radians(), 0.0);
}

/// Criteria 53.2 and 55: a rectangle turned by 30 degrees is a converting shape; an
/// edge stretch makes it a path with the same id, tree position and style.
#[test]
fn a_rotated_rectangle_becomes_a_path_in_place() {
    let mut rig = Rig::new();
    let first = rig.rect(0.0, 0.0, 10.0, 10.0);
    let turned = rig.rect(30.0, 0.0, 20.0, 8.0);
    let last = rig.rect(70.0, 0.0, 10.0, 10.0);
    rig.turned(turned, 30.0);
    rig.select(&[first, turned, last]);
    let before = rig.document.object(turned).unwrap();
    let order = rig.document.object_ids();
    let east = rig.east_edge();
    rig.drag(east, pt(east.x + 30.0, east.y), NONE);
    assert!(is_path(&rig.document, turned));
    assert_eq!(rig.document.object_ids(), order);
    assert_eq!(rig.document.object(turned).unwrap().style(), before.style());
    assert!(rig.document.primitive(first).is_some() && rig.document.primitive(last).is_some());
}

/// Criterion 32: during the drag the converting shape is previewed as its path, the
/// release commits the same snapshots, and a drag back to equal factors previews
/// the shape again.
#[test]
fn the_preview_is_the_commit_and_a_drag_back_converts_nothing() {
    let (mut rig, star, rect) = star_and_rect();
    rig.select(&[star, rect]);
    let east = rig.east_edge();
    rig.press(east, NONE);
    let to = pt(east.x + 30.0, east.y);
    rig.tool.pointer_moved(to, NONE, &mut rig.selection);
    let live = rig.live(to, NONE).expect("a preview");
    assert!(
        matches!(live[0], ObjectSnapshot::Path(_)),
        "the star previews as a path"
    );
    assert!(matches!(live[1], ObjectSnapshot::Primitive(_)));
    // Back to the start: no preview, nothing converted.
    rig.tool.pointer_moved(east, NONE, &mut rig.selection);
    assert!(rig.live(east, NONE).is_none());
    // Out again and released: the committed objects are the previewed ones.
    rig.tool.pointer_moved(to, NONE, &mut rig.selection);
    let live = rig.live(to, NONE).unwrap();
    rig.release(to, NONE);
    assert_eq!(rig.objects(), live, "preview and release agree");
}

/// `0019` criterion 53.4: Escape, a drag of factor 1 and a refused commit convert
/// nothing.
#[test]
fn escape_factor_one_and_a_refused_commit_convert_nothing() {
    let (mut rig, star, rect) = star_and_rect();
    rig.select(&[star, rect]);
    let before = rig.objects();
    let east = rig.east_edge();
    // Escape.
    rig.press(east, NONE);
    rig.tool
        .pointer_moved(pt(east.x + 30.0, east.y), NONE, &mut rig.selection);
    rig.tool.escape();
    rig.release(pt(east.x + 30.0, east.y), NONE);
    assert_eq!(rig.objects(), before);
    // Dragged along the edge only: factor 1 on both axes.
    rig.drag(east, pt(east.x, east.y + 30.0), NONE);
    assert_eq!(rig.objects(), before);
    // A stale id: the snapshot holds an object that is gone, the document refuses it
    // all (criterion 53.6).
    let objects = rig.objects();
    rig.tool.pointer_down(
        &objects,
        &mut rig.selection,
        east,
        tol(),
        tolerances(),
        NONE,
    );
    rig.tool
        .pointer_moved(pt(east.x + 30.0, east.y), NONE, &mut rig.selection);
    rig.document.delete_objects(&[rect]).unwrap();
    rig.tool.pointer_up(
        &rig.document,
        &objects,
        &mut rig.selection,
        pt(east.x + 30.0, east.y),
        NONE,
        &mut AnchorIdMinter::new(9),
    );
    assert!(
        rig.document.primitive(star).is_some(),
        "the star is still a star"
    );
}

/// `0047` criterion 10: move and rotate never convert.
#[test]
fn move_and_rotate_never_convert() {
    let (mut rig, star, rect) = star_and_rect();
    rig.select(&[star, rect]);
    // Rotate: the north-east corner rotate handle.
    let from =
        SelectTool::group_handle_positions(&rig.objects(), &rig.selection, tolerances(), false)
            .into_iter()
            .find(|(h, _)| *h == EditHandle::Rotate(ResizeDirection::Ne))
            .map(|(_, p)| p)
            .unwrap();
    let before = rig.objects();
    rig.drag(from, pt(from.x - 8.0, from.y + 14.0), NONE);
    assert_ne!(rig.objects(), before, "the selection turned");
    assert!(!is_path(&rig.document, star));
    // Move: the star's outline.
    let objects = rig.objects();
    let on_star = curvyo_ui_core::object_outline_bounds(&objects[0]).0;
    rig.drag(
        pt(on_star.x + 10.0, 30.0 + 0.0),
        pt(on_star.x + 25.0, 40.0),
        NONE,
    );
    assert!(!is_path(&rig.document, star));
}

/// `0047` criterion 12: a single polygon or star shows all eight resize handles; a
/// corner drag stretches freely and converts, Ctrl keeps the proportions and the shape.
#[test]
fn a_single_star_converts_on_a_free_corner_and_keeps_its_kind_with_ctrl() {
    let mut rig = Rig::new();
    let star = rig.star(0.0, 10.0);
    rig.select(&[star]);
    let objects = rig.objects();
    let handles = SelectTool::transform_handles(&objects, &rig.selection, tolerances(), false);
    let resize = handles
        .iter()
        .filter(|(h, _)| matches!(h, EditHandle::Resize(_)))
        .count();
    assert_eq!(resize, 8);
    let position = |direction| {
        handles
            .iter()
            .find(|(h, _)| *h == EditHandle::Resize(direction))
            .map(|(_, p)| *p)
            .unwrap()
    };
    // Ctrl on the corner: proportional, still a star.
    let se = position(ResizeDirection::Se);
    rig.drag(se, pt(se.x + 12.0, se.y + 2.0), CTRL);
    assert!(rig.document.primitive(star).is_some());
    // The same corner without Ctrl: a stretch.
    let objects = rig.objects();
    let handles = SelectTool::transform_handles(&objects, &rig.selection, tolerances(), false);
    let se = handles
        .iter()
        .find(|(h, _)| *h == EditHandle::Resize(ResizeDirection::Se))
        .map(|(_, p)| *p)
        .unwrap();
    rig.drag(se, pt(se.x + 30.0, se.y + 1.0), NONE);
    assert!(
        is_path(&rig.document, star),
        "a free corner drag turns it into a path"
    );
}

/// `0047` criteria 9 and 12: the typed size of a single polygon has W and H; a size of another
/// aspect ratio converts it, the same aspect keeps it.
#[test]
fn a_single_polygon_typed_size_converts_only_when_it_is_a_stretch() {
    let mut rig = Rig::new();
    let polygon = rig.document.create_polygon(
        StarFrame {
            center: pt(30.0, 30.0),
            radius: Length::from_mm(10.0),
            angle: Angle::from_radians(0.0),
        },
        PointCount::new(6).unwrap(),
    );
    rig.select(&[polygon]);
    let objects = rig.objects();
    rig.tool
        .open_entry_for_key(&objects, &rig.selection, EntryKey::Size)
        .unwrap();
    rig.tool.mint_conversion_ids(&mut AnchorIdMinter::new(9));
    let entry = rig.tool.entry().unwrap();
    assert_eq!(entry.fields().len(), 2);
    let (w, h) = (
        entry.fields()[0].prefill.clone(),
        entry.fields()[1].prefill.clone(),
    );
    // Same aspect ratio (both times two): a uniform scale.
    let outcome = rig.tool.commit_entry(&rig.document, ["40.0", "40.0"], 0);
    assert_eq!(outcome, EntryOutcome::Committed);
    assert!(
        rig.document.primitive(polygon).is_some(),
        "still a polygon ({w} x {h})"
    );
    // Another aspect: a stretch.
    let objects = rig.objects();
    rig.tool
        .open_entry_for_key(&objects, &rig.selection, EntryKey::Size)
        .unwrap();
    rig.tool.mint_conversion_ids(&mut AnchorIdMinter::new(9));
    let outcome = rig.tool.commit_entry(&rig.document, ["60.0", "40.0"], 0);
    assert_eq!(outcome, EntryOutcome::Committed);
    assert!(is_path(&rig.document, polygon));
}

/// `0047` criteria 7 and 9: the typed size of a selection is a stretch only when the
/// aspect ratio changes; twice both ways is a uniform scale and converts nothing.
#[test]
fn the_group_size_entry_converts_only_a_stretch() {
    let (mut rig, star, rect) = star_and_rect();
    rig.select(&[star, rect]);
    let objects = rig.objects();
    rig.tool
        .open_entry_for_key(&objects, &rig.selection, EntryKey::Size)
        .unwrap();
    rig.tool.mint_conversion_ids(&mut AnchorIdMinter::new(9));
    let entry = rig.tool.group_entry().unwrap();
    let (w, h) = (
        entry.fields()[0].prefill.clone(),
        entry.fields()[1].prefill.clone(),
    );
    assert_eq!(entry.resolve([&w, &h], 0), Ok(None), "unedited");
    let wider = entry.resolve(["90.0", &h], 0).unwrap().unwrap();
    assert!(
        matches!(wider[0], ObjectSnapshot::Path(_)),
        "a wider box converts the star"
    );
    // Exactly twice both ways (the prefill is rounded; the box itself is not).
    let (exact_w, exact_h) = {
        let group = SelectTool::group_of(&rig.objects(), &rig.selection).unwrap();
        (group.bounds().width() * 2.0, group.bounds().height() * 2.0)
    };
    let twice = entry
        .resolve([&format!("{exact_w}"), &format!("{exact_h}")], 0)
        .unwrap()
        .unwrap();
    assert!(
        matches!(twice[0], ObjectSnapshot::Primitive(_)),
        "twice both ways is a uniform scale"
    );
    assert_eq!(
        rig.tool.commit_entry(&rig.document, ["90.0", &h], 0),
        EntryOutcome::Committed
    );
    assert!(is_path(&rig.document, star));
}

/// Decision 8: the ids of a converted path are the session's own once minted; two
/// drags never give two anchors the same id.
#[test]
fn converted_anchor_ids_come_from_the_session_minter() {
    let (mut rig, star, rect) = star_and_rect();
    rig.select(&[star, rect]);
    let east = rig.east_edge();
    let objects = rig.objects();
    rig.tool.pointer_down(
        &objects,
        &mut rig.selection,
        east,
        tol(),
        tolerances(),
        NONE,
    );
    let mut minter = AnchorIdMinter::new(77);
    rig.tool.mint_conversion_ids(&mut minter);
    rig.tool
        .pointer_moved(pt(east.x + 20.0, east.y), NONE, &mut rig.selection);
    let live = rig.live(pt(east.x + 20.0, east.y), NONE).unwrap();
    let ObjectSnapshot::Path(path) = &live[0] else {
        panic!("a path");
    };
    let peer_of = |a: &curvyo_document_core::AnchorSnapshot| a.id.to_hex()[..16].to_string();
    assert!(
        path.anchors
            .iter()
            .all(|a| peer_of(a) == format!("{:016x}", 77))
    );
    let mut ids: Vec<_> = path.anchors.iter().map(|a| a.id).collect();
    ids.sort_by_key(|id| id.to_hex());
    ids.dedup();
    assert_eq!(ids.len(), 10, "ten different ids");
    // The session's counter moved on by the ten it minted.
    assert_eq!(minter.mint().to_hex()[16..], format!("{:016x}", 10));
}

/// Criterion 34: a typed width or height above 10,000,000 mm is "Too large", not
/// "Enter a number", for one object and for a selection alike.
#[test]
fn a_typed_size_beyond_the_limit_is_too_large() {
    use curvyo_ui_core::InvalidReason;
    for ids in [1usize, 2] {
        let mut rig = Rig::new();
        let a = rig.rect(0.0, 0.0, 40.0, 20.0);
        let b = rig.rect(60.0, 0.0, 40.0, 20.0);
        let chosen = if ids == 1 { vec![a] } else { vec![a, b] };
        rig.select(&chosen);
        let objects = rig.objects();
        rig.tool
            .open_entry_for_key(&objects, &rig.selection, EntryKey::Size)
            .unwrap();
        for (texts, field) in [(["20000000", "20"], 0usize), (["40", "99999999"], 1)] {
            assert_eq!(
                rig.tool.commit_entry(&rig.document, texts, field),
                EntryOutcome::Invalid {
                    field,
                    reason: InvalidReason::TooLarge
                },
                "{ids} object(s), {texts:?}"
            );
        }
    }
}

/// The position of the skew handle `side` of the current selection (single object or group).
fn skew_handle(rig: &Rig, side: Side) -> Point {
    let objects = rig.objects();
    let handles = if rig.selection.ids().len() >= 2 {
        SelectTool::group_handle_positions(&objects, &rig.selection, tolerances(), false)
    } else {
        SelectTool::transform_handles(&objects, &rig.selection, tolerances(), false)
    };
    handles
        .iter()
        .find(|(h, _)| *h == EditHandle::Skew(side))
        .map(|(_, p)| *p)
        .unwrap()
}

fn corner_points(document: &Document, id: NodeId) -> Vec<(f64, f64)> {
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("a path");
    };
    let mut points: Vec<(f64, f64)> = path
        .anchors
        .iter()
        .map(|a| (a.point.x.round(), a.point.y.round()))
        .collect();
    points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    points
}

/// `0047` criterion 2, worked example: a 10 x 10 rectangle at (0, 0), top skew handle,
/// bottom side fixed, 45 degrees: one closed path with the corners (10, 0), (20, 0),
/// (10, 10) and (0, 10) and the rectangle's style.
#[test]
fn skewing_a_rectangle_makes_a_path_with_the_sheared_corners() {
    let mut rig = Rig::new();
    let rect = rig.rect(0.0, 0.0, 10.0, 10.0);
    let before = rig.document.object(rect).unwrap();
    rig.select(&[rect]);
    let top = skew_handle(&rig, Side::Top);
    rig.drag(top, pt(top.x + 10.0, top.y), NONE);
    let ObjectSnapshot::Path(path) = rig.document.object(rect).unwrap() else {
        panic!("a path now");
    };
    assert!(path.closed && path.extra_subpaths.is_empty());
    assert_eq!(path.anchors.len(), 4);
    assert_eq!(&path.style, before.style());
    assert_eq!(
        corner_points(&rig.document, rect),
        [(0.0, 10.0), (10.0, 0.0), (10.0, 10.0), (20.0, 0.0)]
    );
}

/// `0047` criteria 2 and 5: the preview of a skew drag is the converted, sheared
/// outline; the release commits exactly that; a drag back to the start converts nothing.
#[test]
fn the_skew_preview_is_the_commit_and_a_drag_back_converts_nothing() {
    let mut rig = Rig::new();
    let rect = rig.rect(0.0, 0.0, 10.0, 10.0);
    rig.select(&[rect]);
    let top = skew_handle(&rig, Side::Top);
    rig.press(top, NONE);
    let to = pt(top.x + 6.0, top.y);
    rig.tool.pointer_moved(to, NONE, &mut rig.selection);
    let live = rig.live(to, NONE).expect("a preview");
    assert!(
        matches!(live[0], ObjectSnapshot::Path(_)),
        "previews as a path"
    );
    assert!(
        rig.document.primitive(rect).is_some(),
        "nothing written yet"
    );
    rig.tool.pointer_moved(top, NONE, &mut rig.selection);
    assert!(
        rig.live(top, NONE).is_none(),
        "back at the start: no preview"
    );
    rig.tool.pointer_moved(to, NONE, &mut rig.selection);
    let live = rig.live(to, NONE).unwrap();
    rig.release(to, NONE);
    assert_eq!(rig.objects(), live, "preview and release agree");
    // Escape and a press without movement write nothing and convert nothing.
    let mut rig = Rig::new();
    let rect = rig.rect(0.0, 0.0, 10.0, 10.0);
    rig.select(&[rect]);
    let top = skew_handle(&rig, Side::Top);
    rig.press(top, NONE);
    rig.tool
        .pointer_moved(pt(top.x + 8.0, top.y), NONE, &mut rig.selection);
    rig.tool.escape();
    rig.release(pt(top.x + 8.0, top.y), NONE);
    rig.drag(top, top, NONE);
    assert!(rig.document.primitive(rect).is_some());
}

/// `0047` criteria 1, 2 and 3: a skew of a mixed selection shears the path and converts
/// the star and the ellipse, each under its own id and style.
#[test]
fn skewing_a_mixed_selection_converts_every_primitive() {
    let mut rig = Rig::new();
    let path = rig.document.create_path(
        &[
            curvyo_document_core::NewAnchor::corner(
                curvyo_document_core::AnchorId::new(1, 1),
                pt(0.0, 0.0),
            ),
            curvyo_document_core::NewAnchor::corner(
                curvyo_document_core::AnchorId::new(1, 2),
                pt(10.0, 10.0),
            ),
        ],
        false,
    );
    let star = rig.star(30.0, 10.0);
    let ellipse = rig.document.create_ellipse(EllipseFrame {
        center: pt(80.0, 10.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(5.0),
    });
    let rect = rig.rect(100.0, 0.0, 10.0, 10.0);
    rig.select(&[path, star, ellipse, rect]);
    let styles: Vec<_> = [star, ellipse, rect]
        .iter()
        .map(|id| rig.document.object(*id).unwrap().style().clone())
        .collect();
    let order = rig.document.object_ids();
    let top = skew_handle(&rig, Side::Top);
    rig.drag(top, pt(top.x + 8.0, top.y), NONE);
    for (id, style) in [star, ellipse, rect].iter().zip(&styles) {
        let object = rig.document.object(*id).unwrap();
        assert!(matches!(object, ObjectSnapshot::Path(_)), "converted");
        assert_eq!(object.style(), style);
    }
    assert_eq!(rig.document.object_ids(), order, "same places");
    let ObjectSnapshot::Path(star_path) = rig.document.object(star).unwrap() else {
        panic!()
    };
    assert_eq!(
        star_path.anchors.len(),
        10,
        "the nodes Object to path gives"
    );
    assert_eq!(rig.selection.ids().len(), 4, "everything stays selected");
}

/// `0047` criterion 6: the typed skew of a single primitive converts it and equals the
/// drag of the same angle.
#[test]
fn the_typed_skew_of_a_rectangle_equals_the_drag() {
    let mut rig = Rig::new();
    let rect = rig.rect(0.0, 0.0, 10.0, 10.0);
    rig.select(&[rect]);
    let objects = rig.objects();
    rig.tool
        .open_entry_for_key(&objects, &rig.selection, EntryKey::SkewX)
        .unwrap();
    rig.tool.mint_conversion_ids(&mut AnchorIdMinter::new(9));
    assert_eq!(
        rig.tool.commit_entry(&rig.document, ["45", ""], 0),
        EntryOutcome::Committed
    );
    assert_eq!(
        corner_points(&rig.document, rect),
        [(0.0, 10.0), (10.0, 0.0), (10.0, 10.0), (20.0, 0.0)]
    );
}

/// `0047` criterion 10: a rotate converts nothing; a rotate and then a stretch
/// converts at the stretch only.
#[test]
fn only_the_stretch_converts_after_a_rotate() {
    let mut rig = Rig::new();
    let star = rig.star(0.0, 10.0);
    rig.turned(star, 20.0);
    assert!(rig.document.primitive(star).is_some());
    rig.select(&[star]);
    let objects = rig.objects();
    let handles = SelectTool::transform_handles(&objects, &rig.selection, tolerances(), false);
    let east = handles
        .iter()
        .find(|(h, _)| *h == EditHandle::Resize(ResizeDirection::E))
        .map(|(_, p)| *p)
        .unwrap();
    rig.drag(east, pt(east.x + 20.0, east.y + 7.0), NONE);
    assert!(is_path(&rig.document, star));
}

/// `0047` criterion 12, worked example: a polygon with the vertices (25, 0), (30, 5),
/// (25, 10) and (20, 5), right edge handle from x = 30 to x = 40, left side fixed: a path
/// with the corners (30, 0), (40, 5), (30, 10) and (20, 5).
#[test]
fn a_single_polygon_stretched_by_its_edge_is_a_path_with_the_worked_corners() {
    let mut rig = Rig::new();
    let polygon = rig.document.create_polygon(
        StarFrame {
            center: pt(25.0, 5.0),
            radius: Length::from_mm(5.0),
            angle: Angle::from_radians(0.0),
        },
        PointCount::new(4).unwrap(),
    );
    rig.select(&[polygon]);
    let objects = rig.objects();
    let east = SelectTool::transform_handles(&objects, &rig.selection, tolerances(), false)
        .into_iter()
        .find(|(h, _)| *h == EditHandle::Resize(ResizeDirection::E))
        .map(|(_, p)| p)
        .unwrap();
    assert!((east.x - 30.0).abs() < 1e-9);
    rig.drag(east, pt(40.0, east.y), NONE);
    assert_eq!(
        corner_points(&rig.document, polygon),
        [(20.0, 5.0), (30.0, 0.0), (30.0, 10.0), (40.0, 5.0)]
    );
}
