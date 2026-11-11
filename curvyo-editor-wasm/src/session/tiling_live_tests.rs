//! Session tests of `specs/0069-tiling` criterion 29, live during the gesture (customer
//! 2026-11-02): while the source of a tiling is scaled with the Select handles or a node of it is
//! dragged with the Node tool, the frame already shows every clone instance edited, and the
//! release leaves them exactly where the frame showed them. A rotation of the source turns every
//! instance in place about its own cell position (customer 2026-11-02).

use super::{Session, Tool};
use curvyo_document_core::{
    Length, NodeId, ObjectSnapshot, PathSnapshot, Point, RectBounds, StyleEdit, Vec2,
};
use curvyo_ui_core::{EditHandle, ResizeDirection, SelectTool};

/// How far a previewed point may lie from the committed one (mm): the write tolerance of a
/// tiling's matrices (`TILING_WRITE_TOLERANCE_MM`).
const WRITE_TOLERANCE: f64 = 1e-9;

/// A session with the rectangle 10..30 x 10..20 tiled 3 x 1 as clones; returns the source and
/// the clone instances.
fn tiled(keep_source: bool) -> (Session, NodeId, Vec<NodeId>) {
    let (s, sources, instances) = tiled_sources(keep_source, &[(10.0, 20.0)]);
    (s, sources[0], instances)
}

/// A session with one rectangle `x..x + w` x 10..20 per entry tiled 3 x 1 as clones; returns
/// the sources and the clone instances (groups of members for more than one source).
fn tiled_sources(keep_source: bool, rects: &[(f64, f64)]) -> (Session, Vec<NodeId>, Vec<NodeId>) {
    let mut s = Session::new(2);
    let sources: Vec<NodeId> = rects
        .iter()
        .map(|&(x, w)| {
            s.document.create_rect(RectBounds {
                origin: Point::new(x, 10.0),
                width: Length::from_mm(w),
                height: Length::from_mm(10.0),
            })
        })
        .collect();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 900.0);
    s.selection.set(&sources);
    assert!(s.tiling_enter(5));
    let mut recipe = s.tiling_recipe().expect("the bar is open");
    recipe.grid.columns = 3;
    recipe.grid.rows = 1;
    recipe.keep_source = keep_source;
    assert!(s.tiling_set_recipe(recipe));
    assert!(s.tiling_apply());
    s.tiling_cancel();
    let group = s
        .document
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| s.document.parent_group(o.id()))
        .find(|g| s.document.tiling(*g).is_some())
        .expect("one tiling");
    let instances = s.document.tiling_instances(group);
    assert_eq!(instances.len(), if keep_source { 2 } else { 3 });
    (s, sources, instances)
}

fn hold(s: &mut Session, at: Point) {
    s.modifiers_changed(false, false, false);
    s.pointer_hover(at, false, false);
}

/// Presses at `a` and moves to `b` without releasing.
fn press_and_move(s: &mut Session, a: Point, b: Point) {
    hold(s, a);
    s.pointer_down(a, false);
    hold(
        s,
        Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
    );
    hold(s, b);
}

/// The outlines this frame draws for `ids`: the artwork with the frame's swap applied.
fn frame_outlines(s: &Session, ids: &[NodeId]) -> Vec<PathSnapshot> {
    let objects = s.objects();
    let live = s.select_live_edit_in(&objects);
    let paths = s.live_node_drag_paths_in(&objects);
    let (swap, _, _) = s.frame_swap(s.view(), &objects, live.as_ref(), &paths);
    let mut artwork = objects.to_vec();
    swap.apply_to(&mut artwork);
    ids.iter()
        .map(|id| match artwork.iter().find(|o| o.id() == *id) {
            Some(ObjectSnapshot::Clone(clone)) => (*clone.resolved).clone(),
            Some(ObjectSnapshot::Path(path)) => path.clone(),
            other => panic!("not an outline: {other:?}"),
        })
        .collect()
}

/// The committed outlines of `ids`.
fn committed(s: &Session, ids: &[NodeId]) -> Vec<PathSnapshot> {
    ids.iter()
        .map(|&id| match s.document.object(id) {
            Some(ObjectSnapshot::Clone(clone)) => *clone.resolved,
            Some(ObjectSnapshot::Path(path)) => path,
            other => panic!("not a clone: {other:?}"),
        })
        .collect()
}

/// The mean of the anchors of `p`: the centre of a rectangle, however it is turned.
fn centre(p: &PathSnapshot) -> Point {
    let points: Vec<Point> = p.all_anchors().map(|a| a.point).collect();
    #[allow(clippy::cast_precision_loss)] // a handful of anchors
    let n = points.len() as f64;
    Point::new(
        points.iter().map(|p| p.x).sum::<f64>() / n,
        points.iter().map(|p| p.y).sum::<f64>() / n,
    )
}

fn assert_centres(want: &[Point], got: &[PathSnapshot], what: &str) {
    assert_eq!(want.len(), got.len(), "{what}");
    for (w, g) in want.iter().zip(got) {
        let c = centre(g);
        assert!(
            (c.x - w.x).abs() < 1e-6 && (c.y - w.y).abs() < 1e-6,
            "{what}: centre {c:?} vs {w:?}"
        );
    }
}

/// Where the handle `wanted` of the selection (one object or a group box) is.
fn handle_of(s: &Session, wanted: EditHandle) -> Point {
    let (objects, tolerances) = (s.objects(), s.transform_handle_tolerances());
    SelectTool::transform_handles(&objects, &s.selection, tolerances, false)
        .into_iter()
        .chain(SelectTool::group_handle_positions(
            &objects,
            &s.selection,
            tolerances,
            false,
        ))
        .find(|(handle, _)| *handle == wanted)
        .unwrap_or_else(|| panic!("no {wanted:?} handle"))
        .1
}

/// Presses the NE rotate handle of the selection and turns it by `deg` about `pivot` (the box
/// centre) without releasing; returns the point to release at.
fn press_and_turn(s: &mut Session, pivot: Point, deg: f64) -> Point {
    let handle = handle_of(s, EditHandle::Rotate(ResizeDirection::Ne));
    let turned = |d: f64| {
        let v = Vec2::new(handle.x - pivot.x, handle.y - pivot.y);
        let end = v.y.atan2(v.x) + d.to_radians();
        let dist = v.x.hypot(v.y);
        Point::new(pivot.x + dist * end.cos(), pivot.y + dist * end.sin())
    };
    hold(s, handle);
    s.pointer_down(handle, false);
    for step in [40.0, deg / 2.0, deg] {
        hold(s, turned(step));
    }
    turned(deg)
}

/// The outline of `id` as the live edit in flight draws it.
fn live_source(s: &Session, id: NodeId) -> PathSnapshot {
    let objects = s.objects();
    let edit = s.select_live_edit_in(&objects).expect("a drag in flight");
    edit.objects
        .iter()
        .find(|o| o.id() == id)
        .and_then(|o| o.drawn_path().cloned())
        .expect("the turned source")
}

fn source_outline(s: &Session, source: NodeId) -> PathSnapshot {
    let object = s.document.object(source).expect("the source lives");
    object.drawn_path().cloned().expect("an outline")
}

/// The anchor offsets of `p` from its first anchor.
fn shape(p: &PathSnapshot) -> Vec<(f64, f64)> {
    let first = p.all_anchors().next().expect("an anchor").point;
    p.all_anchors()
        .map(|a| (a.point.x - first.x, a.point.y - first.y))
        .collect()
}

fn assert_shape(want: &PathSnapshot, got: &[PathSnapshot], what: &str) {
    for instance in got {
        let (w, g) = (shape(want), shape(instance));
        assert_eq!(w.len(), g.len(), "{what}");
        for (a, b) in w.iter().zip(&g) {
            assert!(
                (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6,
                "{what}: {g:?} vs {w:?}"
            );
        }
    }
}

fn assert_equal_outlines(preview: &[PathSnapshot], done: &[PathSnapshot], what: &str) {
    assert_eq!(preview.len(), done.len(), "{what}");
    for (p, d) in preview.iter().zip(done) {
        for (a, b) in p.all_anchors().zip(d.all_anchors()) {
            assert!(
                (a.point.x - b.point.x).abs() <= WRITE_TOLERANCE
                    && (a.point.y - b.point.y).abs() <= WRITE_TOLERANCE,
                "{what}: preview {p:?} vs committed {d:?}"
            );
        }
    }
}

#[test]
fn a_scale_drag_shows_every_instance_scaled_before_the_release() {
    for keep in [true, false] {
        let (mut s, source, instances) = tiled(keep);
        s.selection.set(&[source]);
        let before = committed(&s, &instances);
        let (a, b) = (Point::new(30.0, 20.0), Point::new(40.0, 25.0));
        press_and_move(&mut s, a, b);
        let live = frame_outlines(&s, &instances);
        let objects = s.objects();
        let edit = s.select_live_edit_in(&objects).expect("a drag in flight");
        let scaled = edit.objects[0]
            .drawn_path()
            .cloned()
            .expect("the scaled source");
        assert!(shape(&scaled) != shape(&before[0]), "the source is scaled");
        assert_shape(&scaled, &live, &format!("live scale, keep {keep}"));
        s.pointer_up(b, false, false);
        assert_shape(
            &source_outline(&s, source),
            &committed(&s, &instances),
            "committed",
        );
        assert_equal_outlines(
            &live,
            &committed(&s, &instances),
            &format!("scale, keep {keep}"),
        );
    }
}

#[test]
fn a_move_drag_keeps_every_instance_in_place_before_the_release() {
    for keep in [true, false] {
        let (mut s, source, instances) = tiled(keep);
        s.selection.set(&[source]);
        let before = committed(&s, &instances);
        let (a, b) = (Point::new(20.0, 15.0), Point::new(20.0, 45.0));
        press_and_move(&mut s, a, b);
        assert_equal_outlines(&frame_outlines(&s, &instances), &before, "live move");
        s.pointer_up(b, false, false);
        assert_equal_outlines(&committed(&s, &instances), &before, "committed move");
    }
}

#[test]
fn a_node_drag_shows_every_instance_edited_before_the_release() {
    for keep in [true, false] {
        let (mut s, source, instances) = tiled(keep);
        s.selection.set(&[source]);
        s.set_tool(Tool::Node);
        let (a, b) = (Point::new(30.0, 20.0), Point::new(34.0, 26.0));
        press_and_move(&mut s, a, b);
        let objects = s.objects();
        let edited = s
            .live_node_drag_paths_in(&objects)
            .into_iter()
            .find(|p| p.id == source)
            .expect("the source is dragged");
        let live = frame_outlines(&s, &instances);
        assert_shape(&edited, &live, &format!("live node, keep {keep}"));
        s.pointer_up(b, false, false);
        assert_equal_outlines(
            &live,
            &committed(&s, &instances),
            &format!("node, keep {keep}"),
        );
    }
}

#[test]
fn a_style_drag_shows_every_instance_restyled_before_the_release() {
    for keep in [true, false] {
        let (mut s, source, instances) = tiled(keep);
        s.selection.set(&[source]);
        let edit = StyleEdit::StrokeWidth(Length::from_mm(3.0));
        s.style.preview(&[source], edit);
        let mut drawn = s.objects().to_vec();
        s.style.apply_to(&mut drawn);
        let want = Length::from_mm(3.0);
        for id in &instances {
            let object = drawn.iter().find(|o| o.id() == *id).expect("drawn");
            assert_eq!(object.style().stroke.width, want, "keep {keep}");
        }
    }
}

#[test]
fn a_rotation_drag_turns_every_instance_in_place_before_and_after_the_release() {
    for keep in [true, false] {
        let (mut s, source, instances) = tiled(keep);
        s.selection.set(&[source]);
        let before: Vec<Point> = committed(&s, &instances).iter().map(centre).collect();
        let end = press_and_turn(&mut s, Point::new(20.0, 15.0), 35.0);
        let turned = live_source(&s, source);
        assert!(
            shape(&turned) != shape(&source_outline(&s, source)),
            "turned"
        );
        let live = frame_outlines(&s, &instances);
        let what = format!("live rotation, keep {keep}");
        assert_shape(&turned, &live, &what);
        assert_centres(&before, &live, &what);
        s.pointer_up(end, false, false);
        let done = committed(&s, &instances);
        assert_shape(&source_outline(&s, source), &done, "committed rotation");
        assert_centres(&before, &done, "committed rotation");
        assert_equal_outlines(&live, &done, &format!("rotation, keep {keep}"));
    }
}

#[test]
fn a_rotation_then_a_scale_turn_and_scale_every_instance_in_place() {
    let (mut s, source, instances) = tiled(true);
    s.selection.set(&[source]);
    let before: Vec<Point> = committed(&s, &instances).iter().map(centre).collect();
    let end = press_and_turn(&mut s, Point::new(20.0, 15.0), -60.0);
    s.pointer_up(end, false, false);
    let corner = handle_of(&s, EditHandle::Resize(ResizeDirection::Se));
    let to = Point::new(corner.x + 8.0, corner.y + 5.0);
    press_and_move(&mut s, corner, to);
    let scaled = live_source(&s, source);
    let live = frame_outlines(&s, &instances);
    assert_shape(&scaled, &live, "live scale of the turned source");
    assert_centres(&before, &live, "live scale of the turned source");
    s.pointer_up(to, false, false);
    let done = committed(&s, &instances);
    assert_shape(&source_outline(&s, source), &done, "committed");
    assert_centres(&before, &done, "committed");
    assert_equal_outlines(&live, &done, "rotation then scale");
}

#[test]
fn a_clone_of_an_instance_turns_with_it_live_and_after_the_release() {
    let (mut s, source, instances) = tiled(true);
    let clone = s
        .document
        .linked_clone(&[instances[0]], &[])
        .expect("a clone")[0];
    s.document
        .translate_objects(&[clone], Vec2::new(0.0, 40.0))
        .expect("moved");
    s.selection.set(&[source]);
    let before = centre(&committed(&s, &[clone])[0]);
    let end = press_and_turn(&mut s, Point::new(20.0, 15.0), 50.0);
    let turned = live_source(&s, source);
    let live = frame_outlines(&s, &[clone]);
    assert_shape(&turned, &live, "live chain");
    assert_centres(&[before], &live, "live chain");
    s.pointer_up(end, false, false);
    let done = committed(&s, &[clone]);
    assert_shape(&source_outline(&s, source), &done, "committed chain");
    assert_equal_outlines(&live, &done, "chain");
}

#[test]
fn two_sources_turned_together_keep_their_member_spacing_in_every_instance() {
    let (mut s, sources, instances) = tiled_sources(true, &[(10.0, 20.0), (40.0, 10.0)]);
    s.selection.set(&sources);
    let members: Vec<NodeId> = instances
        .iter()
        .flat_map(|i| s.document.children(Some(*i)))
        .collect();
    assert_eq!(members.len(), 4, "two members per instance");
    let centres = |p: &[PathSnapshot]| -> Vec<Point> { p.iter().map(centre).collect() };
    let before = centres(&committed(&s, &members));
    let sources_before = centres(&committed(&s, &sources));
    // The cell box is 10..50 x 10..20: the selection box centre is the tiling's pivot, so each
    // instance turns about its own cell position and a member lands at its source's turned
    // centre plus the cell's offset.
    let end = press_and_turn(&mut s, Point::new(30.0, 15.0), 30.0);
    let turned: Vec<Point> = sources
        .iter()
        .map(|id| centre(&live_source(&s, *id)))
        .collect();
    let live = frame_outlines(&s, &members);
    s.pointer_up(end, false, false);
    let done = committed(&s, &members);
    assert_equal_outlines(&live, &done, "two sources");
    let want: Vec<Point> = (0..members.len())
        .map(|m| {
            let k = m % 2;
            Point::new(
                turned[k].x + before[m].x - sources_before[k].x,
                turned[k].y + before[m].y - sources_before[k].y,
            )
        })
        .collect();
    assert_centres(&want, &live, "live members");
    assert_centres(&want, &done, "committed members");
}
