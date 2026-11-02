//! A tiling of clones follows edits of its source (`specs/0069-tiling` criterion 29, customer
//! finding on PR #247, 2026-11-02): a Rectangle tiled as clones, with Keep source on and off, and
//! its source then edited the ways the UI allows (style, a node drag, a side drag, a scale with
//! the Select handles); every instance shows the edit, and a move of the source alone does not
//! scatter them.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    Color, Document, Length, NodeId, ObjectSnapshot, PathSnapshot, Point, RectBounds, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, RowPress, Session, Tool};
use curvyo_ui_core::StyleField;

/// The rectangle 10..30 x 10..20.
fn rectangle() -> (Document, NodeId) {
    let document = Document::new(1);
    let rect = document.create_rect(RectBounds {
        origin: Point::new(10.0, 10.0),
        width: Length::from_mm(20.0),
        height: Length::from_mm(10.0),
    });
    (document, rect)
}

fn hold(s: &mut Session, at: Point) {
    s.modifiers_changed(false, false, false);
    s.pointer_hover(at, false, false);
}

fn drag(s: &mut Session, a: Point, b: Point) {
    hold(s, a);
    s.pointer_down(a, false);
    hold(
        s,
        Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
    );
    hold(s, b);
    s.pointer_up(b, false, false);
}

fn escape(s: &mut Session) {
    let _ = s.key_down(KeyInput {
        key: "Escape",
        ..KeyInput::default()
    });
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

/// A session with the rectangle tiled 3 x 1 as clones; returns the group.
fn tiled(keep_source: bool) -> (Session, NodeId, NodeId) {
    let (document, rect) = rectangle();
    let mut s = Session::open(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 900.0);
    s.press_layer_row(&serde_json::to_string(&rect).unwrap(), RowPress::Plain);
    assert!(s.tiling_enter(5));
    let mut recipe = s.tiling_recipe().unwrap();
    recipe.grid.columns = 3;
    recipe.grid.rows = 1;
    recipe.keep_source = keep_source;
    assert!(s.tiling_set_recipe(recipe));
    assert!(s.tiling_apply());
    let group = doc_of(&s)
        .tiling_instances_parent()
        .expect("one tiling in the document");
    escape(&mut s);
    (s, group, rect)
}

trait TilingOf {
    fn tiling_instances_parent(&self) -> Option<NodeId>;
}

impl TilingOf for Document {
    fn tiling_instances_parent(&self) -> Option<NodeId> {
        self.read_objects()
            .objects
            .iter()
            .filter_map(|o| self.parent_group(o.id()))
            .find(|g| self.tiling(*g).is_some())
    }
}

/// The source's drawn outline and the resolved outline of every clone instance.
fn outlines(s: &Session, group: NodeId, source: NodeId) -> (PathSnapshot, Vec<PathSnapshot>) {
    let d = doc_of(s);
    let source = match d.object(source).expect("the source lives") {
        ObjectSnapshot::Path(p) => p,
        other => other.drawn_path().cloned().expect("an outline"),
    };
    let instances = d
        .tiling_instances(group)
        .into_iter()
        .map(|i| match d.object(i) {
            Some(ObjectSnapshot::Clone(c)) => c.resolved,
            other => panic!("not a clone: {other:?}"),
        })
        .collect();
    (source, instances)
}

fn size(p: &PathSnapshot) -> (f64, f64) {
    let xs = p.all_anchors().map(|a| a.point.x);
    let ys = p.all_anchors().map(|a| a.point.y);
    let (x0, x1) = xs.fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x), b.max(x)));
    let (y0, y1) = ys.fold((f64::MAX, f64::MIN), |(a, b), y| (a.min(y), b.max(y)));
    (x1 - x0, y1 - y0)
}

fn centre(p: &PathSnapshot) -> Point {
    let xs = p.all_anchors().map(|a| a.point.x);
    let ys = p.all_anchors().map(|a| a.point.y);
    let (x0, x1) = xs.fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x), b.max(x)));
    let (y0, y1) = ys.fold((f64::MAX, f64::MIN), |(a, b), y| (a.min(y), b.max(y)));
    Point::new(f64::midpoint(x0, x1), f64::midpoint(y0, y1))
}

/// Every instance has the source's shape: the same anchor offsets from its first anchor.
fn assert_same_shape(source: &PathSnapshot, instances: &[PathSnapshot], what: &str) {
    let rel = |p: &PathSnapshot| -> Vec<(f64, f64)> {
        let first = p.all_anchors().next().unwrap().point;
        p.all_anchors()
            .map(|a| (a.point.x - first.x, a.point.y - first.y))
            .collect()
    };
    let want = rel(source);
    for instance in instances {
        let got = rel(instance);
        assert_eq!(got.len(), want.len(), "{what}");
        for (g, w) in got.iter().zip(&want) {
            assert!(
                (g.0 - w.0).abs() < 1e-6 && (g.1 - w.1).abs() < 1e-6,
                "{what}: {got:?} vs {want:?}"
            );
        }
    }
}

fn select_source(s: &mut Session, source: NodeId) {
    s.set_tool(Tool::Select);
    s.press_layer_row(&serde_json::to_string(&source).unwrap(), RowPress::Plain);
}

#[test]
fn a_style_edit_of_the_source_reaches_every_instance() {
    for keep in [true, false] {
        let (mut s, group, source) = tiled(keep);
        select_source(&mut s, source);
        s.set_style_text(StyleField::FillColor, "#00ff00").unwrap();
        s.set_style_text(StyleField::StrokeWidth, "2").unwrap();
        s.set_style_text(StyleField::StrokeColor, "#0000ff")
            .unwrap();
        let (src, instances) = outlines(&s, group, source);
        assert_eq!(src.style.fill.color, Color { r: 0, g: 255, b: 0 });
        assert_eq!(instances.len(), if keep { 2 } else { 3 });
        for instance in &instances {
            assert_eq!(instance.style, src.style, "keep {keep}");
        }
    }
}

#[test]
fn a_node_drag_of_the_source_reaches_every_instance() {
    for keep in [true, false] {
        let (mut s, group, source) = tiled(keep);
        select_source(&mut s, source);
        s.set_tool(Tool::Node);
        drag(&mut s, Point::new(30.0, 20.0), Point::new(34.0, 26.0));
        let (src, instances) = outlines(&s, group, source);
        assert!(
            src.all_anchors()
                .any(|a| (a.point.x - 34.0).abs() < 1e-6 && (a.point.y - 26.0).abs() < 1e-6),
            "the corner moved: {src:?}"
        );
        assert_same_shape(&src, &instances, &format!("corner, keep {keep}"));
    }
}

#[test]
fn a_side_drag_of_the_source_reaches_every_instance() {
    for keep in [true, false] {
        let (mut s, group, source) = tiled(keep);
        select_source(&mut s, source);
        s.set_tool(Tool::Node);
        // Both right-hand corners: the rectangle gets wider, an affine change of its outline.
        let corner = Point::new(30.0, 10.0);
        hold(&mut s, corner);
        s.pointer_down(corner, false);
        s.pointer_up(corner, false, false);
        let other = Point::new(30.0, 20.0);
        s.modifiers_changed(true, false, false);
        s.pointer_hover(other, true, false);
        s.pointer_down(other, true);
        s.pointer_up(other, true, false);
        drag(&mut s, other, Point::new(40.0, 20.0));
        let (src, instances) = outlines(&s, group, source);
        assert!((size(&src).0 - 30.0).abs() < 1e-6, "the source widened");
        assert_same_shape(&src, &instances, &format!("side, keep {keep}"));
    }
}

#[test]
fn a_scale_of_the_source_reaches_every_instance_and_a_move_scatters_none() {
    for keep in [true, false] {
        let (mut s, group, source) = tiled(keep);
        select_source(&mut s, source);
        let (_, before) = outlines(&s, group, source);
        // A move of the source alone keeps every instance where it is.
        drag(&mut s, Point::new(20.0, 15.0), Point::new(20.0, 45.0));
        let (moved, after_move) = outlines(&s, group, source);
        assert!((moved.anchors[0].point.y - 40.0).abs() < 1e-6, "{moved:?}");
        assert_eq!(after_move, before, "a move scatters nothing (keep {keep})");
        // The bottom-right scale handle of the moved source.
        drag(&mut s, Point::new(30.0, 50.0), Point::new(40.0, 60.0));
        let (src, instances) = outlines(&s, group, source);
        assert!(size(&src).0 > 21.0, "the source was scaled: {src:?}");
        assert_same_shape(&src, &instances, &format!("scale, keep {keep}"));
        for (b, a) in before.iter().zip(&instances) {
            let (b, a) = (centre(b), centre(a));
            assert!(
                (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6,
                "a scale keeps each instance centred in its cell: {b:?} {a:?}"
            );
        }
    }
}
