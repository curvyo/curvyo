//! Independent tester tests for `specs/0085-stroke-to-path` at the editor session (criteria 1,
//! 11, 13 to 23, 26 to 29), written from the specification before the implementation was read;
//! only the public names were looked up. The canvas reference is `curvyo-render-core`'s own
//! stroke (lyon), rasterised at 20 px per mm, never the kernel under test.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::cast_possible_wrap,
    clippy::type_complexity,
    missing_docs
)]

mod cpu_time;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{
    AnchorId, AnchorKind, Color, DashPattern, Document, EllipseFrame, Length, LineCap, LineJoin,
    MarkerCount, MarkerPlace, MarkerShape, NewAnchor, NodeId, ObjectSnapshot, Opacity, Point,
    PointCount, RectBounds, StarFrame, StyleEdit, Vec2, ViewTransform, pack, unpack,
};
use curvyo_editor_wasm::{EscapeStep, RowPress, Session, StrokeToPathOutcome, Tool};
use curvyo_render_core::build_artwork;
use curvyo_ui_core::{AnchorIdMinter, FontBlock, TextInput, plan_stroke_to_path};
use serde_json::Value;

static AID: AtomicU64 = AtomicU64::new(80_000);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn corner(x: f64, y: f64) -> NewAnchor {
    NewAnchor::corner(
        AnchorId::new(9, AID.fetch_add(1, Ordering::SeqCst)),
        pt(x, y),
    )
}

fn path(d: &Document, points: &[(f64, f64)], closed: bool) -> NodeId {
    let anchors: Vec<NewAnchor> = points.iter().map(|&(x, y)| corner(x, y)).collect();
    d.create_path(&anchors, closed)
}

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

fn style(d: &Document, ids: &[NodeId], edits: &[StyleEdit]) {
    for edit in edits {
        d.edit_style(ids, edit).unwrap();
    }
}

fn width(mm: f64) -> StyleEdit {
    StyleEdit::StrokeWidth(Length::from_mm(mm))
}

fn dash(pattern: &[f64]) -> StyleEdit {
    StyleEdit::StrokeDash(DashPattern::new(pattern.to_vec()).unwrap())
}

const BLUE: Color = Color {
    r: 0,
    g: 40,
    b: 220,
};
const RED: Color = Color {
    r: 230,
    g: 10,
    b: 10,
};

fn open(d: &Document) -> Session {
    // A fresh peer per session, as every opened project gets.
    static PEER: AtomicU64 = AtomicU64::new(100);
    let mut s = Session::open(
        PEER.fetch_add(1, Ordering::SeqCst),
        &pack(d, "0.1.0").unwrap(),
    )
    .unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).expect("the session output reopens")
}

fn rid(id: NodeId) -> String {
    serde_json::to_string(&id).unwrap()
}

fn select(s: &mut Session, ids: &[NodeId]) {
    for (i, &id) in ids.iter().enumerate() {
        let press = if i == 0 {
            RowPress::Plain
        } else {
            RowPress::Toggle
        };
        s.press_layer_row(&rid(id), press);
    }
}

fn layers(s: &mut Session) -> Vec<Value> {
    let v: Value = serde_json::from_str(&s.layers_view_json(0, 5000)).unwrap();
    v["rows"].as_array().unwrap().clone()
}

fn selected(s: &mut Session) -> HashSet<NodeId> {
    layers(s)
        .into_iter()
        .filter(|r| r["mark"] == "selected")
        .map(|r| serde_json::from_value(r["id"].clone()).unwrap())
        .collect()
}

/// The parent of `id` and its siblings in z-order, searched from the root.
fn siblings(d: &Document, id: NodeId) -> (Option<NodeId>, Vec<NodeId>) {
    fn walk(
        d: &Document,
        parent: Option<NodeId>,
        id: NodeId,
    ) -> Option<(Option<NodeId>, Vec<NodeId>)> {
        let kids = d.children(parent);
        if kids.contains(&id) {
            return Some((parent, kids));
        }
        kids.into_iter().find_map(|k| walk(d, Some(k), id))
    }
    walk(d, None, id).expect("the node is in the tree")
}

/// Whether, in `d`'s z-order convention, `upper` is drawn directly above `lower`. The
/// convention is read off two rectangles made one after the other (the later one is on top).
fn directly_above(d: &Document, lower: NodeId, upper: NodeId) -> bool {
    let probe = Document::new(5);
    let a = rect(&probe, 0.0, 0.0, 1.0, 1.0);
    let b = rect(&probe, 0.0, 0.0, 1.0, 1.0);
    let (_, order) = siblings(&probe, a);
    let later_last = order.iter().position(|&x| x == b) > order.iter().position(|&x| x == a);
    let (pl, sl) = siblings(d, lower);
    let (pu, _) = siblings(d, upper);
    if pl != pu {
        return false;
    }
    let (il, iu) = (
        sl.iter().position(|&x| x == lower).unwrap() as i64,
        sl.iter().position(|&x| x == upper).unwrap() as i64,
    );
    if later_last {
        iu == il + 1
    } else {
        iu == il - 1
    }
}

fn applied(outcome: StrokeToPathOutcome) -> (usize, usize, usize, usize) {
    match outcome {
        StrokeToPathOutcome::Applied {
            strokes,
            unstroked,
            texts,
            unlinked,
        } => (strokes, unstroked, texts, unlinked),
        other => panic!("not applied: {other:?}"),
    }
}

/// Every object of `d` other than `before`, i.e. the new ones.
fn new_objects(d: &Document, before: &HashSet<NodeId>) -> Vec<NodeId> {
    d.object_ids()
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect()
}

fn sampled_area(p: &curvyo_document_core::PathSnapshot) -> f64 {
    let mut total = 0.0;
    for sub in p.subpaths() {
        let a = sub.anchors;
        let n = a.len();
        let mut pts = Vec::new();
        for i in 0..n {
            let (s, e) = (&a[i], &a[(i + 1) % n]);
            let p0 = s.point;
            let p1 = pt(p0.x + s.handle_out.x, p0.y + s.handle_out.y);
            let p3 = e.point;
            let p2 = pt(p3.x + e.handle_in.x, p3.y + e.handle_in.y);
            for k in 0..64 {
                let t = f64::from(k) / 64.0;
                let u = 1.0 - t;
                let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                pts.push((
                    w[0] * p0.x + w[1] * p1.x + w[2] * p2.x + w[3] * p3.x,
                    w[0] * p0.y + w[1] * p1.y + w[2] * p2.y + w[3] * p3.y,
                ));
            }
        }
        let m = pts.len();
        total += (0..m)
            .map(|i| pts[i].0 * pts[(i + 1) % m].1 - pts[(i + 1) % m].0 * pts[i].1)
            .sum::<f64>()
            / 2.0;
    }
    total.abs()
}

fn as_path(d: &Document, id: NodeId) -> curvyo_document_core::PathSnapshot {
    match d.object(id) {
        Some(ObjectSnapshot::Path(p)) => p,
        other => panic!("{id:?} is not a path: {other:?}"),
    }
}

fn history_names(s: &mut Session) -> String {
    let n = s.history_len();
    s.history_rows_json(0, n)
}

// ----- criteria 14, 19, 20, 21: the result ----------------------------------------------------

/// Criteria 14, 19, 20: a filled rectangle with a half-transparent blue stroke and 60 % object
/// opacity: the rectangle stays the same rectangle (same id, Stroke off, fill kept), the outline
/// lies directly above it in the same layer, filled blue at the stroke's alpha, Stroke off, the
/// object opacity copied, no dash or markers; both are selected, the tool stays Select.
#[test]
fn c19_fill_part_and_outline_paint_and_place() {
    let d = Document::new(1);
    let below = rect(&d, -40.0, 0.0, 10.0, 10.0);
    let r = rect(&d, 0.0, 0.0, 20.0, 20.0);
    let above = rect(&d, 40.0, 0.0, 10.0, 10.0);
    style(&d, &[below, above], &[StyleEdit::StrokeEnabled(false)]);
    style(
        &d,
        &[r],
        &[
            StyleEdit::FillEnabled(true),
            StyleEdit::FillColor(RED),
            StyleEdit::StrokeRgba(BLUE, Opacity::new(0.5).unwrap()),
            width(2.0),
            dash(&[3.0, 1.0]),
            StyleEdit::StrokeJoin(LineJoin::Round),
        ],
    );
    d.set_node_opacity(&[r], Opacity::new(0.6).unwrap())
        .unwrap();
    let ObjectSnapshot::Primitive(shape_before) = d.object(r).unwrap() else {
        panic!()
    };
    let mut s = open(&d);
    select(&mut s, &[r]);
    let before: HashSet<NodeId> = doc_of(&s).object_ids().into_iter().collect();
    assert_eq!(applied(s.apply_stroke_to_path()), (1, 0, 0, 0));
    assert_eq!(s.tool(), Tool::Select);
    let after = doc_of(&s);
    let ObjectSnapshot::Primitive(fill_part) = after.object(r).unwrap() else {
        panic!("the rectangle stays a primitive")
    };
    assert_eq!(
        fill_part.shape, shape_before.shape,
        "same primitive parameters"
    );
    assert!(!fill_part.style.stroke.enabled);
    assert!(fill_part.style.fill.enabled && fill_part.style.fill.color == RED);
    let added = new_objects(&after, &before);
    assert_eq!(added.len(), 1, "one new object");
    let outline = as_path(&after, added[0]);
    assert!(
        directly_above(&after, r, outline.id),
        "outline directly above its fill part"
    );
    assert!(outline.style.fill.enabled);
    assert_eq!(outline.style.fill.color, BLUE);
    assert!(
        (outline.style.fill.opacity.get() - 0.5).abs() < 1e-9,
        "stroke alpha"
    );
    assert!(!outline.style.stroke.enabled);
    assert!(outline.style.stroke.dash.is_solid() || !outline.style.stroke.enabled);
    assert_eq!(
        after.node_registers(outline.id).unwrap().opacity,
        Opacity::new(0.6).unwrap(),
        "object opacity"
    );
    assert_eq!(selected(&mut s), HashSet::from([r, outline.id]));
}

/// Criterion 19: a subject with Fill off becomes only its outline, under the same id, and is the
/// only object left of it.
#[test]
fn c19_fill_off_subject_is_replaced_by_its_outline() {
    let d = Document::new(1);
    let line = path(&d, &[(0.0, 0.0), (100.0, 0.0)], false);
    style(
        &d,
        &[line],
        &[
            StyleEdit::FillEnabled(false),
            width(2.0),
            StyleEdit::StrokeColor(BLUE),
        ],
    );
    let mut s = open(&d);
    select(&mut s, &[line]);
    assert_eq!(applied(s.apply_stroke_to_path()), (1, 0, 0, 0));
    let after = doc_of(&s);
    assert_eq!(after.object_ids(), vec![line]);
    let p = as_path(&after, line);
    assert!(p.closed && p.style.fill.enabled && !p.style.stroke.enabled);
    assert_eq!(p.style.fill.color, BLUE);
    assert!((sampled_area(&p) - 200.0).abs() < 0.01);
    assert_eq!(selected(&mut s), HashSet::from([line]));
}

/// Criterion 14: ellipse (arc kind included), polygon and star keep their kind and parameters;
/// the circle's outline is the 40 pi ring.
#[test]
fn c14_primitives_stay_primitives() {
    let d = Document::new(1);
    let e = d.create_ellipse(EllipseFrame {
        center: pt(0.0, 0.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(10.0),
    });
    let poly = d.create_polygon(
        StarFrame {
            center: pt(40.0, 0.0),
            radius: Length::from_mm(10.0),
            angle: curvyo_document_core::Angle::from_radians(0.0),
        },
        PointCount::new(6).unwrap(),
    );
    let ids = [e, poly];
    style(&d, &ids, &[width(2.0), StyleEdit::FillEnabled(true)]);
    let shapes: Vec<_> = ids
        .iter()
        .map(|&id| match d.object(id).unwrap() {
            ObjectSnapshot::Primitive(p) => p.shape,
            _ => panic!(),
        })
        .collect();
    let mut s = open(&d);
    select(&mut s, &ids);
    let before: HashSet<NodeId> = doc_of(&s).object_ids().into_iter().collect();
    assert_eq!(applied(s.apply_stroke_to_path()).0, 2);
    let after = doc_of(&s);
    for (&id, shape) in ids.iter().zip(&shapes) {
        let ObjectSnapshot::Primitive(p) = after.object(id).unwrap() else {
            panic!("{id:?} stays a primitive")
        };
        assert_eq!(&p.shape, shape);
        assert!(!p.style.stroke.enabled);
    }
    let added = new_objects(&after, &before);
    assert_eq!(added.len(), 2);
    let ring = added
        .iter()
        .map(|&id| as_path(&after, id))
        .find(|p| {
            p.subpaths()
                .all(|sp| sp.anchors.iter().all(|a| a.point.x < 20.0))
        })
        .unwrap();
    assert!(
        (sampled_area(&ring) - 40.0 * std::f64::consts::PI).abs() < 0.05,
        "{}",
        sampled_area(&ring)
    );
}

// ----- criterion 13: compound paths -----------------------------------------------------------

/// Criterion 13: a compound path of a closed square and an open line (`0054`), with markers set:
/// no markers in the result; the outline is the union of both strokes, open one capped.
#[test]
fn c13_compound_path_without_markers() {
    let d = Document::new(1);
    let sq = path(
        &d,
        &[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)],
        true,
    );
    let ln = path(&d, &[(30.0, 0.0), (60.0, 0.0)], false);
    let mut s = open(&d);
    select(&mut s, &[sq, ln]);
    let combined = s.apply_combine();
    assert!(
        matches!(combined, curvyo_editor_wasm::CombineOutcome::Applied { .. }),
        "{combined:?}"
    );
    let compound = doc_of(&s).object_ids()[0];
    // Style the compound through the session's style edits on the selection.
    s.set_stroke_cap(LineCap::Square);
    s.set_stroke_join(LineJoin::Miter);
    s.set_style_text(curvyo_ui_core::StyleField::StrokeWidth, "2")
        .unwrap();
    let d2 = doc_of(&s);
    style(
        &d2,
        &[compound],
        &[
            StyleEdit::FillEnabled(false),
            StyleEdit::MarkerStart(MarkerShape::Dot),
            StyleEdit::MarkerEnd(MarkerShape::Arrow),
        ],
    );
    let mut s = open(&d2);
    select(&mut s, &[compound]);
    assert_eq!(applied(s.apply_stroke_to_path()).0, 1);
    let after = doc_of(&s);
    let p = as_path(&after, compound);
    // Square ring (22^2 - 18^2 = 160) plus the 30 mm line with square caps (32 x 2 = 64).
    assert!(
        (sampled_area(&p) - 224.0).abs() < 0.02,
        "{}",
        sampled_area(&p)
    );
}

// ----- criterion 16: groups -------------------------------------------------------------------

/// Criterion 16: a selected group of a nested group: every subject inside is converted in place,
/// the groups, their names and the unstroked member stay; the outlines are in the subjects'
/// groups.
#[test]
fn c16_groups_at_any_depth_convert_in_place() {
    let d = Document::new(1);
    let a = path(&d, &[(0.0, 0.0), (10.0, 0.0)], false);
    let b = rect(&d, 20.0, 0.0, 5.0, 5.0);
    let c = rect(&d, 40.0, 0.0, 5.0, 5.0);
    style(&d, &[b], &[StyleEdit::FillEnabled(true)]);
    style(
        &d,
        &[c],
        &[
            StyleEdit::StrokeEnabled(false),
            StyleEdit::FillEnabled(true),
        ],
    );
    let inner = d.group_objects(&[a, b]).unwrap();
    let outer = d.group_objects(&[inner, c]).unwrap();
    let names_before = (
        d.node_registers(inner).unwrap().name,
        d.node_registers(outer).unwrap().name,
    );
    let mut s = open(&d);
    select(&mut s, &[outer]);
    assert!(s.stroke_to_path_available());
    let before: HashSet<NodeId> = doc_of(&s).object_ids().into_iter().collect();
    assert_eq!(applied(s.apply_stroke_to_path()), (2, 1, 0, 0));
    let after = doc_of(&s);
    assert!(after.is_group(inner) && after.is_group(outer));
    assert_eq!(
        (
            after.node_registers(inner).unwrap().name,
            after.node_registers(outer).unwrap().name
        ),
        names_before
    );
    assert_eq!(
        siblings(&after, a).0,
        Some(inner),
        "a (fill off) replaced in place"
    );
    let added = new_objects(&after, &before);
    assert_eq!(added.len(), 1, "only the filled rect gets a second object");
    assert_eq!(siblings(&after, added[0]).0, Some(inner));
    assert!(directly_above(&after, b, added[0]));
    assert!(after.object(c).unwrap().style().fill.enabled);
}

// ----- criterion 17: clones -------------------------------------------------------------------

/// Criterion 17, a selected clone: the outline is made from what the clone shows, the clone
/// becomes an ordinary path (fill part), the original is unchanged.
#[test]
fn c17_selected_clone_is_unlinked_original_unchanged() {
    let d = Document::new(1);
    let o = path(
        &d,
        &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        true,
    );
    style(&d, &[o], &[StyleEdit::FillEnabled(true), width(1.0)]);
    let clone = d.linked_clone(&[o], &[]).unwrap()[0];
    let original_before = d.object(o).unwrap();
    let mut s = open(&d);
    select(&mut s, &[clone]);
    let (strokes, _, _, unlinked) = applied(s.apply_stroke_to_path());
    assert_eq!(strokes, 1);
    let after = doc_of(&s);
    let p = as_path(&after, clone);
    assert!(!p.style.stroke.enabled && p.style.fill.enabled);
    assert_eq!(
        after.object(o).unwrap(),
        original_before,
        "original unchanged"
    );
    // The selected clone is the subject itself; whether it counts as "unlinked" is the
    // notice's business, at most one.
    assert!(unlinked <= 1);
}

/// Criterion 17, a selected original with clones: every clone becomes an ordinary path with its
/// current look (same outline, same stroke), the notice counts them, nothing jumps.
#[test]
fn c17_clones_of_the_original_keep_their_look() {
    let d = Document::new(1);
    let o = path(&d, &[(0.0, 0.0), (30.0, 0.0), (30.0, 10.0)], false);
    style(&d, &[o], &[width(1.5), StyleEdit::StrokeColor(BLUE)]);
    let k1 = d.linked_clone(&[o], &[]).unwrap()[0];
    let k2 = d.linked_clone(&[o], &[]).unwrap()[0];
    let shown: Vec<_> = [k1, k2]
        .iter()
        .map(|&k| match d.object(k).unwrap() {
            ObjectSnapshot::Clone(c) => c.resolved,
            _ => panic!(),
        })
        .collect();
    let mut s = open(&d);
    select(&mut s, &[o]);
    assert_eq!(applied(s.apply_stroke_to_path()), (1, 0, 0, 2));
    let after = doc_of(&s);
    for (&k, before) in [k1, k2].iter().zip(&shown) {
        let p = as_path(&after, k);
        assert_eq!(p.style, before.style, "{k:?} keeps its style");
        let pts: Vec<Point> = p.anchors.iter().map(|a| a.point).collect();
        let was: Vec<Point> = before.anchors.iter().map(|a| a.point).collect();
        assert_eq!(pts, was, "{k:?} does not jump");
    }
    // Undo brings the links back (criterion 21).
    s.undo();
    let undone = doc_of(&s);
    assert!(matches!(undone.object(k1), Some(ObjectSnapshot::Clone(_))));
    assert!(matches!(undone.object(k2), Some(ObjectSnapshot::Clone(_))));
}

// ----- criterion 18, 27: skips ----------------------------------------------------------------

/// Criteria 18 and 27: unstroked objects, a transparent stroke, a stroke of 0.002 mm, a
/// one-node path, a guide, a hidden and a locked stroked object: only the one real subject is
/// converted; the counted skips are the unstroked selected objects (guides, hidden and locked
/// are never subjects).
#[test]
fn c18_c27_skips_and_counts() {
    let d = Document::new(1);
    let real = path(&d, &[(0.0, 0.0), (20.0, 0.0)], false);
    let bare = rect(&d, 0.0, 10.0, 5.0, 5.0);
    let clear = rect(&d, 10.0, 10.0, 5.0, 5.0);
    let thin = rect(&d, 20.0, 10.0, 5.0, 5.0);
    let dot = path(&d, &[(40.0, 0.0)], false);
    let guide = path(&d, &[(0.0, 30.0), (20.0, 30.0)], false);
    let hidden = path(&d, &[(0.0, 40.0), (20.0, 40.0)], false);
    let locked = path(&d, &[(0.0, 50.0), (20.0, 50.0)], false);
    style(&d, &[bare], &[StyleEdit::StrokeEnabled(false)]);
    style(
        &d,
        &[clear],
        &[StyleEdit::StrokeOpacity(Opacity::new(0.0).unwrap())],
    );
    style(&d, &[thin], &[width(0.002)]);
    style(&d, &[dot], &[StyleEdit::StrokeCap(LineCap::Butt)]);
    d.set_guide(&[guide], true).unwrap();
    let all = [real, bare, clear, thin, dot, guide, hidden, locked];
    d.set_node_visible(&[hidden], false).unwrap();
    d.set_node_locked(&[locked], true).unwrap();
    let before_doc = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    let mut s2 = open(&d);
    select(&mut s2, &all);
    let (strokes, unstroked, texts, unlinked) = applied(s2.apply_stroke_to_path());
    assert_eq!((strokes, texts, unlinked), (1, 0, 0));
    assert_eq!(unstroked, 4, "bare, transparent, 0.002 mm, one-node path");
    let after = doc_of(&s2);
    for id in [bare, clear, thin, dot, guide] {
        assert_eq!(
            after.object(id),
            before_doc.object(id),
            "{id:?} left as it is"
        );
    }
    for id in [hidden, locked] {
        assert!(
            after.object(id).unwrap().style().stroke.enabled,
            "{id:?} not a subject"
        );
    }
}

/// Criterion 1: the button is dimmed when the selection holds only guides, unstroked objects,
/// hidden or locked ones; a press then does nothing.
#[test]
fn c01_availability_follows_subjects() {
    let d = Document::new(1);
    let bare = rect(&d, 0.0, 0.0, 5.0, 5.0);
    let guide = path(&d, &[(0.0, 30.0), (20.0, 30.0)], false);
    let stroked = rect(&d, 10.0, 0.0, 5.0, 5.0);
    style(&d, &[bare], &[StyleEdit::StrokeEnabled(false)]);
    d.set_guide(&[guide], true).unwrap();
    let mut s = open(&d);
    assert!(!s.stroke_to_path_available(), "nothing selected");
    select(&mut s, &[bare, guide]);
    assert!(!s.stroke_to_path_available());
    let n = s.history_len();
    assert_eq!(s.apply_stroke_to_path(), StrokeToPathOutcome::Ignored);
    assert_eq!(s.history_len(), n);
    select(&mut s, &[bare, guide, stroked]);
    assert!(s.stroke_to_path_available());
    s.set_tool(Tool::Rectangle);
    assert_eq!(s.apply_stroke_to_path(), StrokeToPathOutcome::Ignored);
}

// ----- criterion 21: one step, undo, redo -----------------------------------------------------

/// Criterion 21: one step "Stroke to path"; undo restores ids, kinds, styles, stacking order
/// and the selection; redo gives the same result again; save and reopen keeps it (criterion 23).
#[test]
fn c21_c23_one_step_undo_redo_and_reopen() {
    let d = Document::new(1);
    let a = rect(&d, 0.0, 0.0, 10.0, 10.0);
    let b = path(&d, &[(20.0, 0.0), (30.0, 10.0), (40.0, 0.0)], false);
    let c = rect(&d, 50.0, 0.0, 10.0, 10.0);
    style(&d, &[a, c], &[StyleEdit::FillEnabled(true)]);
    style(
        &d,
        &[b],
        &[
            width(1.0),
            dash(&[2.0, 1.0]),
            StyleEdit::StrokeCap(LineCap::Round),
        ],
    );
    let mut s = open(&d);
    select(&mut s, &[a, b]);
    let before = doc_of(&s);
    let order_before = siblings(&before, a).1;
    let selection_before = selected(&mut s);
    let n = s.history_len();
    applied(s.apply_stroke_to_path());
    assert_eq!(s.history_len(), n + 1, "exactly one step");
    assert!(history_names(&mut s).contains("Stroke to path"));
    let done = doc_of(&s);
    let done_selection = selected(&mut s);
    s.undo();
    let undone = doc_of(&s);
    assert_eq!(siblings(&undone, a).1, order_before, "stacking order");
    for id in before.object_ids() {
        assert_eq!(undone.object(id), before.object(id), "{id:?} restored");
    }
    assert_eq!(undone.object_ids().len(), before.object_ids().len());
    assert_eq!(selected(&mut s), selection_before, "selection as before");
    s.redo();
    let redone = doc_of(&s);
    assert_eq!(redone.object_ids(), done.object_ids());
    for id in done.object_ids() {
        assert_eq!(redone.object(id), done.object(id));
    }
    assert_eq!(selected(&mut s), done_selection);
    // Criterion 23: save and reopen.
    let reopened = unpack(7, &pack(&done, "0.1.0").unwrap()).unwrap();
    for id in done.object_ids() {
        assert_eq!(reopened.object(id), done.object(id));
    }
}

// ----- criterion 26: refusal ------------------------------------------------------------------

/// Criterion 26: a subject whose stroke width carries it past 10 km is refused with its
/// offender, nothing changed, even with good subjects beside it.
#[test]
fn c26_out_of_range_refuses_all_or_nothing() {
    let d = Document::new(1);
    let good = rect(&d, 0.0, 0.0, 10.0, 10.0);
    let far = path(&d, &[(9_999_990.0, 0.0), (9_999_999.5, 0.0)], false);
    style(
        &d,
        &[far],
        &[width(2.0), StyleEdit::StrokeCap(LineCap::Square)],
    );
    let mut s = open(&d);
    select(&mut s, &[good, far]);
    let before = doc_of(&s);
    let n = s.history_len();
    match s.apply_stroke_to_path() {
        StrokeToPathOutcome::Refused(curvyo_ui_core::StrokeToPathRefusal::OutOfRange {
            offenders,
        }) => assert_eq!(offenders, vec![far]),
        other => panic!("{other:?}"),
    }
    assert_eq!(s.history_len(), n);
    let after = doc_of(&s);
    for id in before.object_ids() {
        assert_eq!(after.object(id), before.object(id));
    }
}

// ----- criterion 28: determinism --------------------------------------------------------------

/// Criterion 28: the result is node for node the same however the objects were selected.
#[test]
fn c28_selection_order_does_not_matter() {
    let make = || {
        let d = Document::new(1);
        let a = path(&d, &[(0.0, 0.0), (20.0, 5.0), (40.0, 0.0)], false);
        let b = rect(&d, 0.0, 10.0, 10.0, 10.0);
        style(
            &d,
            &[a, b],
            &[
                width(1.0),
                dash(&[3.0, 1.0]),
                StyleEdit::StrokeJoin(LineJoin::Round),
            ],
        );
        (d, a, b)
    };
    let (d1, a, b) = make();
    let mut s1 = open(&d1);
    select(&mut s1, &[a, b]);
    applied(s1.apply_stroke_to_path());
    let mut s2 = open(&d1);
    select(&mut s2, &[b, a]);
    applied(s2.apply_stroke_to_path());
    let (r1, r2) = (doc_of(&s1), doc_of(&s2));
    let geometry = |d: &Document| -> Vec<Vec<(f64, f64)>> {
        let mut all: Vec<Vec<(f64, f64)>> = d
            .object_ids()
            .into_iter()
            .filter_map(|id| match d.object(id) {
                Some(ObjectSnapshot::Path(p)) => Some(
                    p.subpaths()
                        .flat_map(|sp| sp.anchors.iter().map(|x| (x.point.x, x.point.y)))
                        .collect(),
                ),
                _ => None,
            })
            .collect();
        all.sort_by(|x, y| x.partial_cmp(y).unwrap());
        all
    };
    assert_eq!(geometry(&r1), geometry(&r2));
}

// ----- criterion 15: texts --------------------------------------------------------------------

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn load(s: &mut Session) {
    for file in s.wanted_font_files() {
        assert!(s.add_font_file(&file, &std::fs::read(fonts_dir().join(&file)).unwrap()));
    }
}

/// A session with one text in `family`, selected in the Select tool.
fn lettering(family: &str, string: &str, size_mm: f64) -> Session {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let text = |key: &str| f[key].as_str().unwrap().to_owned();
            (text("file"), text("family"), text("style"), 0)
        })
        .collect();
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list(FontBlock::Included, faces, Vec::new());
    load(&mut s);
    let tab: Value = serde_json::from_str(&s.fonts_tab_json()).unwrap();
    let face = tab["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == family)
        .unwrap()["index"]
        .as_u64()
        .unwrap();
    s.pick_font(usize::try_from(face).unwrap());
    load(&mut s);
    s.set_tool(Tool::Text);
    s.pointer_down(pt(10.0, 40.0), false);
    s.pointer_up(pt(10.0, 40.0), false, false);
    s.text_input(TextInput::Insert(string.to_owned()));
    load(&mut s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(s.escape(), EscapeStep::LeftTool, "the text is selected");
    s.set_text_size(size_mm);
    load(&mut s);
    s.set_tool(Tool::Select);
    s
}

/// Criterion 15: a single-line text becomes a group named by its first 24 characters holding
/// one filled outline per drawn glyph, one step; undo brings the editable text back (21).
#[test]
fn c15_single_line_text_becomes_a_group_of_filled_bands() {
    let string = "Laser cut lettering 12345 more";
    let mut s = lettering("Hershey Sans 1-stroke", string, 10.0);
    s.set_style_text(curvyo_ui_core::StyleField::StrokeWidth, "0.5")
        .unwrap();
    let before = doc_of(&s);
    let text_id = before.object_ids()[0];
    assert!(matches!(
        before.object(text_id),
        Some(ObjectSnapshot::Text(_))
    ));
    let n = s.history_len();
    let (strokes, _, texts, _) = applied(s.apply_stroke_to_path());
    assert_eq!((strokes, texts), (1, 1));
    assert_eq!(s.history_len(), n + 1);
    let after = doc_of(&s);
    let top = after.children(None);
    let groups: Vec<NodeId> = top
        .iter()
        .flat_map(|&n| std::iter::once(n).chain(after.children(Some(n))))
        .filter(|&id| after.is_group(id))
        .collect();
    assert_eq!(groups.len(), 1, "one group");
    let name = after
        .node_registers(groups[0])
        .unwrap()
        .name
        .unwrap_or_default();
    // Named as Object to path names it (`0074` criterion 22): the first 24 characters, with the
    // ellipsis Object to path adds when the string is longer.
    let first: String = string.chars().take(24).collect();
    assert!(
        name == first || name == format!("{first}\u{2026}"),
        "{name}"
    );
    let glyphs = string.chars().filter(|c| !c.is_whitespace()).count();
    let members = after.children(Some(groups[0]));
    let filled: Vec<_> = members
        .iter()
        .filter_map(|&id| match after.object(id) {
            Some(ObjectSnapshot::Path(p)) => Some(p),
            _ => None,
        })
        .filter(|p| p.style.fill.enabled && !p.style.stroke.enabled)
        .collect();
    assert_eq!(filled.len(), glyphs, "one filled outline per drawn glyph");
    assert!(filled.iter().all(|p| p.subpaths().all(|sp| sp.closed)));
    s.undo();
    assert!(matches!(
        doc_of(&s).object(text_id),
        Some(ObjectSnapshot::Text(_))
    ));
}

// ----- criterion 11: canvas parity ------------------------------------------------------------

const SCALE: f64 = 20.0;

fn raster(triangles: &[curvyo_render_core::Vertex]) -> HashSet<(i64, i64)> {
    let mut covered = HashSet::new();
    for tri in triangles.as_chunks::<3>().0 {
        let p: Vec<(f64, f64)> = tri
            .iter()
            .map(|v| (v.position.x * SCALE, v.position.y * SCALE))
            .collect();
        let edge = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
            (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
        };
        let area = edge(p[0], p[1], p[2]);
        if area == 0.0 {
            continue;
        }
        let xs = p.iter().map(|q| q.0);
        let ys = p.iter().map(|q| q.1);
        let (x0, x1) = (
            xs.clone().fold(f64::MAX, f64::min).floor() as i64,
            xs.fold(f64::MIN, f64::max).ceil() as i64,
        );
        let (y0, y1) = (
            ys.clone().fold(f64::MAX, f64::min).floor() as i64,
            ys.fold(f64::MIN, f64::max).ceil() as i64,
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let c = (x as f64 + 0.5, y as f64 + 0.5);
                let w = [
                    edge(p[0], p[1], c) * area.signum(),
                    edge(p[1], p[2], c) * area.signum(),
                    edge(p[2], p[0], c) * area.signum(),
                ];
                if w.iter().all(|&v| v >= 0.0) {
                    covered.insert((x, y));
                }
            }
        }
    }
    covered
}

fn artwork(d: &Document) -> HashSet<(i64, i64)> {
    let objects: Vec<_> = d
        .object_ids()
        .into_iter()
        .filter_map(|id| d.object(id))
        .collect();
    raster(&build_artwork(&objects, ViewTransform::new(SCALE, pt(-300.0, -300.0))).triangles)
}

/// Pixels that differ and are more than 1 px from the edge of the canvas stroke (stricter than
/// "an edge of either image": a missing or extra piece of the outline is caught).
fn parity(d: &Document, id: NodeId) -> (usize, usize, usize) {
    let before = artwork(d);
    let objects: Vec<_> = d.object(id).into_iter().collect();
    let plan = plan_stroke_to_path(
        &objects,
        &[id],
        &|_| false,
        &|_| None,
        &mut AnchorIdMinter::new(8),
    )
    .unwrap();
    d.stroke_to_path(&plan.write).unwrap();
    let after = artwork(d);
    let differing: Vec<_> = before.symmetric_difference(&after).copied().collect();
    let off_edge = differing
        .iter()
        .filter(|&&(x, y)| {
            let inside = before.contains(&(x, y));
            !(-1..=1).any(|dy| (-1..=1).any(|dx| before.contains(&(x + dx, y + dy)) != inside))
        })
        .count();
    (differing.len(), off_edge, before.len())
}

fn stroked_path(points: &[(f64, f64)], closed: bool, edits: &[StyleEdit]) -> (Document, NodeId) {
    let d = Document::new(1);
    let id = path(&d, points, closed);
    style(&d, &[id], &[StyleEdit::FillEnabled(false)]);
    style(&d, &[id], edits);
    (d, id)
}

fn assert_parity(name: &str, d: &Document, id: NodeId) {
    let (differ, off_edge, total) = parity(d, id);
    let share = differ as f64 / total.max(1) as f64;
    println!(
        "{name}: {differ} of {total} differ ({:.3} %), {off_edge} off the edge",
        share * 100.0
    );
    assert!(total > 0, "{name}: the canvas draws something");
    assert!(
        share <= 0.005 && off_edge == 0,
        "{name}: {differ} of {total}, {off_edge} off edge"
    );
}

/// Criterion 11 on shapes the implementer's parity test does not have: miter at the limit
/// (30 and 28 degrees), a closed self-crossing pentagram, a square cap dash with gaps shorter
/// than the caps (dashes merge), zero-length round dots including the end of the path, spaced
/// and at-node markers on a dashed line, a dashed closed path and primitives.
#[test]
fn c11_more_shapes_paint_what_the_canvas_paints() {
    for (deg, name) in [(30.0_f64, "miter 30"), (12.0, "bevel 12")] {
        let a = deg.to_radians();
        let (d, id) = stroked_path(
            &[
                (0.0, 0.0),
                (40.0, 0.0),
                (40.0 - 40.0 * a.cos(), 40.0 * a.sin()),
            ],
            false,
            &[width(2.0), StyleEdit::StrokeJoin(LineJoin::Miter)],
        );
        assert_parity(name, &d, id);
    }
    let star: Vec<(f64, f64)> = (0..5)
        .map(|i| {
            let a = std::f64::consts::TAU * f64::from(i * 2) / 5.0 - std::f64::consts::FRAC_PI_2;
            (20.0 * a.cos(), 20.0 * a.sin())
        })
        .collect();
    let (d, id) = stroked_path(&star, true, &[width(3.0)]);
    assert_parity("pentagram", &d, id);
    let (d, id) = stroked_path(
        &[(0.0, 0.0), (60.0, 0.0)],
        false,
        &[
            width(1.0),
            StyleEdit::StrokeCap(LineCap::Square),
            dash(&[3.0, 0.5]),
        ],
    );
    assert_parity("square caps over short gaps", &d, id);
    let (d, id) = stroked_path(
        &[(0.0, 0.0), (40.0, 0.0)],
        false,
        &[
            width(1.0),
            StyleEdit::StrokeCap(LineCap::Round),
            dash(&[0.0, 4.0]),
        ],
    );
    // Tiny dots: the canvas's flattened circles differ on the edge only; the 0.5 % share does
    // not apply to 0.5 mm dots at 20 px per mm, the dot count (one at the end or not) does.
    let (differ, off_edge, total) = parity(&d, id);
    assert!(
        off_edge == 0 && differ < 100,
        "round dots: {differ} of {total}, {off_edge} off edge"
    );
    let (d, id) = stroked_path(
        &[(0.0, 0.0), (15.0, 0.0), (40.0, 10.0), (80.0, 0.0)],
        false,
        &[
            width(1.0),
            dash(&[10.0, 10.0]),
            StyleEdit::MarkerStart(MarkerShape::Arrow),
            StyleEdit::MarkerMid(MarkerShape::Dot),
            StyleEdit::MarkerPlace(MarkerPlace::AtNodes),
            StyleEdit::MarkerEnd(MarkerShape::Dot),
        ],
    );
    assert_parity("markers at nodes in dash gaps", &d, id);
    let (d, id) = stroked_path(
        &[(0.0, 0.0), (80.0, 0.0)],
        false,
        &[
            width(0.5),
            StyleEdit::MarkerMid(MarkerShape::Arrow),
            StyleEdit::MarkerPlace(MarkerPlace::Spaced),
            StyleEdit::MarkerCount(MarkerCount::new(7).unwrap()),
        ],
    );
    assert_parity("spaced arrows", &d, id);
    let (d, id) = stroked_path(
        &[(0.0, 0.0), (30.0, 0.0), (30.0, 20.0), (0.0, 20.0)],
        true,
        &[
            width(1.0),
            dash(&[7.0, 3.0]),
            StyleEdit::StrokeJoin(LineJoin::Bevel),
        ],
    );
    assert_parity("dashed closed path", &d, id);

    let d = Document::new(1);
    let e = d.create_ellipse(EllipseFrame {
        center: pt(0.0, 0.0),
        rx: Length::from_mm(20.0),
        ry: Length::from_mm(8.0),
    });
    style(
        &d,
        &[e],
        &[
            width(1.5),
            dash(&[4.0, 2.0]),
            StyleEdit::StrokeCap(LineCap::Round),
        ],
    );
    assert_parity("dashed ellipse", &d, e);
    let d = Document::new(1);
    let st = d.create_star(
        StarFrame {
            center: pt(0.0, 0.0),
            radius: Length::from_mm(20.0),
            angle: curvyo_document_core::Angle::from_radians(std::f64::consts::FRAC_PI_2),
        },
        PointCount::new(5).unwrap(),
        curvyo_document_core::InnerRatio::new(0.4).unwrap(),
    );
    style(
        &d,
        &[st],
        &[width(2.0), StyleEdit::StrokeJoin(LineJoin::Miter)],
    );
    assert_parity("star miter", &d, st);
    let d = Document::new(1);
    let rr = rect(&d, 0.0, 0.0, 30.0, 20.0);
    d.set_corner_radius(&[rr], Length::from_mm(4.0)).unwrap();
    style(&d, &[rr], &[width(2.0)]);
    assert_parity("rounded rectangle", &d, rr);
}

/// Criteria 6 and 11 between the two miter rules: `0007` criterion 11 (and the kernel) bevel a
/// corner whose miter length / width exceeds 4 (interior angle below 28.96 degrees); the canvas
/// (lyon's `miter_limit_is_exceeded` compares |normal| with 2 x limit, an SVG limit of 8) mitres
/// down to 14.4 degrees. At 20 and 28 degrees the outline loses the canvas's miter tip.
#[test]
fn c11_miter_between_svg_limit_4_and_8_matches_the_canvas() {
    for deg in [20.0_f64, 28.0] {
        let a = deg.to_radians();
        let (d, id) = stroked_path(
            &[
                (0.0, 0.0),
                (40.0, 0.0),
                (40.0 - 40.0 * a.cos(), 40.0 * a.sin()),
            ],
            false,
            &[width(2.0), StyleEdit::StrokeJoin(LineJoin::Miter)],
        );
        assert_parity(&format!("miter {deg}"), &d, id);
    }
}

/// Criterion 11 and Frage 45 ("exactly the area the stroke paints on the canvas") against the
/// canvas rule that a stroke of more than 2,000 dashes is drawn solid: a 0.2 mm line 100 mm
/// long with the dash `1 1` has 2,500 dashes.
#[test]
fn c11_more_than_2000_dashes_match_the_canvas() {
    let (d, id) = stroked_path(
        &[(0.0, 0.0), (1000.0, 0.0)],
        false,
        &[width(0.2), dash(&[1.0, 1.0])],
    );
    assert_parity("2,500 dashes", &d, id);
}

// ----- criterion 29: budgets in CPU time ------------------------------------------------------

fn budget(name: &str, limit: Duration, s: &mut Session) {
    let (outcome, cpu) = cpu_time_of(|| s.apply_stroke_to_path());
    applied(outcome);
    println!("{name}: {cpu:?} CPU");
    if !cfg!(debug_assertions) {
        assert!(cpu < limit, "{name}: {cpu:?} >= {limit:?}");
    }
}

/// Criterion 29 (a): each shape of criteria 4 to 10, through the whole session command, under
/// 50 ms of CPU time in a release build.
#[test]
fn c29a_criterion_shapes_under_50_ms_cpu() {
    let cases: Vec<(&str, Vec<(f64, f64)>, bool, Vec<StyleEdit>)> = vec![
        (
            "ac4 round",
            vec![(0.0, 0.0), (100.0, 0.0)],
            false,
            vec![width(2.0), StyleEdit::StrokeCap(LineCap::Round)],
        ),
        (
            "ac5 square",
            vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)],
            true,
            vec![width(2.0)],
        ),
        (
            "ac6 corner",
            vec![(0.0, 0.0), (50.0, 0.0), (0.0, 5.0)],
            false,
            vec![width(2.0)],
        ),
        (
            "ac8 dash",
            vec![(0.0, 0.0), (100.0, 0.0)],
            false,
            vec![
                width(1.0),
                dash(&[5.0, 5.0]),
                StyleEdit::StrokeCap(LineCap::Round),
            ],
        ),
        (
            "ac9 eight",
            vec![(0.0, 0.0), (20.0, 20.0), (20.0, 0.0), (0.0, 20.0)],
            false,
            vec![width(3.0)],
        ),
        (
            "ac10 markers",
            vec![(0.0, 0.0), (30.0, 0.0), (30.0, 20.0)],
            false,
            vec![
                width(1.0),
                StyleEdit::MarkerStart(MarkerShape::Dot),
                StyleEdit::MarkerEnd(MarkerShape::Arrow),
            ],
        ),
    ];
    for (name, pts, closed, edits) in cases {
        let (d, id) = stroked_path(&pts, closed, &edits);
        let mut s = open(&d);
        select(&mut s, &[id]);
        budget(name, Duration::from_millis(50), &mut s);
    }
    let d = Document::new(1);
    let e = d.create_ellipse(EllipseFrame {
        center: pt(0.0, 0.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(10.0),
    });
    style(&d, &[e], &[width(2.0)]);
    let mut s = open(&d);
    select(&mut s, &[e]);
    budget("ac7 circle", Duration::from_millis(50), &mut s);
}

/// Criterion 29 (d): a path of 5,000 nodes with the dash `2 1` under 2 s of CPU time.
#[test]
fn c29d_five_thousand_nodes_dashed_under_2_s_cpu() {
    let d = Document::new(1);
    let anchors: Vec<NewAnchor> = (0..5000)
        .map(|i| {
            let t = f64::from(i) * 0.05;
            let p = pt(t * 10.0, 20.0 * (t).sin() + 5.0 * (3.1 * t).cos());
            let h = Vec2::new(0.15, 0.0);
            NewAnchor {
                id: AnchorId::new(9, AID.fetch_add(1, Ordering::SeqCst)),
                point: p,
                handle_in: Vec2::new(-h.x, -h.y),
                handle_out: h,
                kind: AnchorKind::Corner,
            }
        })
        .collect();
    let id = d.create_path(&anchors, false);
    style(
        &d,
        &[id],
        &[StyleEdit::FillEnabled(false), width(0.5), dash(&[2.0, 1.0])],
    );
    let mut s = open(&d);
    select(&mut s, &[id]);
    budget("5,000 nodes dashed", Duration::from_secs(2), &mut s);
}
