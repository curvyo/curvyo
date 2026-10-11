//! Shared helpers of the independent acceptance tests for `0048-split-compound-path` and
//! `0036-cut-at-crossings`. Everything here is built from the public API only: documents are
//! made with `Document`, opened in a `Session`, driven with the keyboard and the pointer and
//! read back through `pack`/`unpack`.

#![allow(
    dead_code,
    unused_must_use,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::type_complexity
)]

use std::collections::HashSet;
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    AnchorId, AnchorKind, Document, NewAnchor, NodeId, PathSnapshot, PieceFill, PieceOutlines,
    Point, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};

static NEXT: AtomicU64 = AtomicU64::new(1);

pub fn aid() -> AnchorId {
    AnchorId::new(48, NEXT.fetch_add(1, Ordering::SeqCst))
}

pub fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

pub fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

pub fn corner(x: f64, y: f64) -> NewAnchor {
    NewAnchor::corner(aid(), pt(x, y))
}

pub fn square(x: f64, y: f64, side: f64) -> Vec<NewAnchor> {
    vec![
        corner(x, y),
        corner(x + side, y),
        corner(x + side, y + side),
        corner(x, y + side),
    ]
}

/// A four-node Bezier circle starting at (cx + r, cy); `reverse` flips the direction.
pub fn circle(cx: f64, cy: f64, r: f64, reverse: bool) -> Vec<NewAnchor> {
    rotated_circle(cx, cy, r, 0.0, reverse)
}

/// The same circle with its four nodes turned by `degrees` about its centre.
pub fn rotated_circle(cx: f64, cy: f64, r: f64, degrees: f64, reverse: bool) -> Vec<NewAnchor> {
    let k = 0.552_284_749_830_793_4 * r;
    let (sin, cos) = degrees.to_radians().sin_cos();
    let rot = |x: f64, y: f64| (x * cos - y * sin, x * sin + y * cos);
    let base = [
        ((r, 0.0), (0.0, -k), (0.0, k)),
        ((0.0, r), (k, 0.0), (-k, 0.0)),
        ((-r, 0.0), (0.0, k), (0.0, -k)),
        ((0.0, -r), (-k, 0.0), (k, 0.0)),
    ];
    let mut a: Vec<NewAnchor> = base
        .iter()
        .map(|&((px, py), (ix, iy), (ox, oy))| {
            let (px, py) = rot(px, py);
            let (ix, iy) = rot(ix, iy);
            let (ox, oy) = rot(ox, oy);
            NewAnchor {
                id: aid(),
                point: pt(cx + px, cy + py),
                handle_in: v(ix, iy),
                handle_out: v(ox, oy),
                kind: AnchorKind::Symmetric,
            }
        })
        .collect();
    if reverse {
        a.reverse();
        for n in &mut a {
            std::mem::swap(&mut n.handle_in, &mut n.handle_out);
        }
    }
    a
}

pub fn cubic(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> Vec<NewAnchor> {
    vec![
        NewAnchor {
            id: aid(),
            point: pt(p0.0, p0.1),
            handle_in: v(0.0, 0.0),
            handle_out: v(p1.0 - p0.0, p1.1 - p0.1),
            kind: AnchorKind::Corner,
        },
        NewAnchor {
            id: aid(),
            point: pt(p3.0, p3.1),
            handle_in: v(p2.0 - p3.0, p2.1 - p3.1),
            handle_out: v(0.0, 0.0),
            kind: AnchorKind::Corner,
        },
    ]
}

pub fn line(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<NewAnchor> {
    vec![corner(x0, y0), corner(x1, y1)]
}

/// A compound path of the given closed outlines, made through the public
/// `Document::replace_with_pieces`, so the outlines can be in any relation (touching ones
/// included, which Combine refuses and files of other programs may hold).
pub fn compound(d: &Document, outlines: Vec<Vec<NewAnchor>>) -> NodeId {
    let base = d.create_path(&outlines[0], true);
    let piece: PieceOutlines = outlines.into_iter().map(|o| (o, true)).collect();
    let made = d
        .replace_with_pieces(&[base], &[(base, piece)], PieceFill::Keep, "test_build")
        .unwrap();
    made[0]
}

pub fn session_of(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s
}

pub fn reread(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

pub fn json(s: &Session) -> Vec<u8> {
    reread(s).export_json().unwrap()
}

pub fn labels(d: &Document) -> Vec<String> {
    let loro = loro::LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let ids: Vec<loro::ID> = loro.oplog_frontiers().iter().collect();
    let mut out: Vec<(u32, String)> = Vec::new();
    loro.travel_change_ancestors(&ids, &mut |m| {
        out.push((
            m.lamport,
            m.message
                .map(|s| s.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        ControlFlow::Continue(())
    })
    .unwrap();
    out.sort();
    out.into_iter().map(|(_, s)| s).collect()
}

pub fn new_labels(before: &[String], s: &Session) -> Vec<String> {
    let after = labels(&reread(s));
    after[before.len()..].to_vec()
}

pub fn label_count(s: &Session) -> usize {
    labels(&reread(s)).len()
}

/// All path objects (ordinary and compound) in stacking order, bottom first; primitives are
/// skipped.
pub fn paths(s: &Session) -> Vec<PathSnapshot> {
    let d = reread(s);
    d.object_ids()
        .into_iter()
        .filter_map(|id| d.path(id))
        .collect()
}

pub fn object_ids(s: &Session) -> Vec<NodeId> {
    reread(s).object_ids()
}

pub fn select_all(s: &mut Session) {
    s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}

/// The objects that are selected: Delete removes them, the difference is read, and undo puts
/// everything back. The document is as before when this returns.
pub fn selected_ids(s: &mut Session) -> Vec<NodeId> {
    let before = json(s);
    let all = object_ids(s);
    s.delete_selected();
    let left: HashSet<NodeId> = object_ids(s).into_iter().collect();
    s.undo();
    assert_eq!(
        json(s),
        before,
        "the helper must leave the document as it was"
    );
    all.into_iter().filter(|id| !left.contains(id)).collect()
}

pub fn marquee(s: &mut Session, from: Point, to: Point) {
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(
        pt((from.x + to.x) / 2.0, (from.y + to.y) / 2.0),
        false,
        false,
    );
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

// ---------------------------------------------------------------------------------------------
// Geometry read-back
// ---------------------------------------------------------------------------------------------

pub type Xy = (f64, f64);

/// One node as exact bits: position, both handles, kind.
pub type NodeSig = (u64, u64, u64, u64, u64, u64, String);

pub fn node_sig(a: &NewAnchor) -> NodeSig {
    (
        a.point.x.to_bits(),
        a.point.y.to_bits(),
        a.handle_in.x.to_bits(),
        a.handle_in.y.to_bits(),
        a.handle_out.x.to_bits(),
        a.handle_out.y.to_bits(),
        format!("{:?}", a.kind),
    )
}

pub fn outline_sigs(p: &PathSnapshot) -> Vec<(bool, Vec<NodeSig>)> {
    p.subpaths()
        .map(|sp| (sp.closed, sp.anchors.iter().map(node_sig).collect()))
        .collect()
}

/// Positions only, in traversal order, of one outline.
pub fn outline_points(p: &PathSnapshot, index: usize) -> Vec<Xy> {
    p.subpaths()
        .nth(index)
        .unwrap()
        .anchors
        .iter()
        .map(|a| (a.point.x, a.point.y))
        .collect()
}

pub fn bezier(p: &NewAnchor, q: &NewAnchor, t: f64) -> Xy {
    let p0 = (p.point.x, p.point.y);
    let p1 = (p.point.x + p.handle_out.x, p.point.y + p.handle_out.y);
    let p2 = (q.point.x + q.handle_in.x, q.point.y + q.handle_in.y);
    let p3 = (q.point.x, q.point.y);
    let u = 1.0 - t;
    let (c0, c1, c2, c3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (
        c0 * p0.0 + c1 * p1.0 + c2 * p2.0 + c3 * p3.0,
        c0 * p0.1 + c1 * p1.1 + c2 * p2.1 + c3 * p3.1,
    )
}

/// A dense polyline of an outline, `per_segment` samples per segment, ending at the last
/// point.
pub fn flatten(a: &[NewAnchor], closed: bool, per_segment: usize) -> Vec<Xy> {
    let mut out = Vec::new();
    let n = a.len();
    if n == 0 {
        return out;
    }
    let segs = if closed { n } else { n.saturating_sub(1) };
    for i in 0..segs {
        for s in 0..per_segment {
            out.push(bezier(
                &a[i],
                &a[(i + 1) % n],
                s as f64 / per_segment as f64,
            ));
        }
    }
    let last = &a[if closed { 0 } else { n - 1 }];
    out.push((last.point.x, last.point.y));
    out
}

pub fn poly_length(poly: &[Xy]) -> f64 {
    poly.windows(2)
        .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
        .sum()
}

pub fn dist_point_segment(p: Xy, a: Xy, b: Xy) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - cx).powi(2) + (p.1 - cy).powi(2)).sqrt()
}

pub fn dist_to_poly(p: Xy, poly: &[Xy]) -> f64 {
    if poly.len() == 1 {
        return ((p.0 - poly[0].0).powi(2) + (p.1 - poly[0].1).powi(2)).sqrt();
    }
    poly.windows(2)
        .map(|w| dist_point_segment(p, w[0], w[1]))
        .fold(f64::INFINITY, f64::min)
}

pub fn dist(a: Xy, b: Xy) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

pub fn endpoints(p: &PathSnapshot) -> (Xy, Xy) {
    let a = &p.anchors;
    (
        (a[0].point.x, a[0].point.y),
        (a[a.len() - 1].point.x, a[a.len() - 1].point.y),
    )
}

/// Points sampled along an open piece (every segment at 9 parameters).
pub fn sample_piece(p: &PathSnapshot) -> Vec<Xy> {
    let mut out = Vec::new();
    for w in p.anchors.windows(2) {
        for s in 0..=8 {
            out.push(bezier(&w[0], &w[1], f64::from(s) / 8.0));
        }
    }
    if p.anchors.len() == 1 {
        out.push((p.anchors[0].point.x, p.anchors[0].point.y));
    }
    out
}

// A nonzero-winding evaluator for the outlines of a path, written here.

fn winding_of(poly: &[Xy], x: f64, y: f64) -> i32 {
    let mut w = 0;
    for i in 0..poly.len() {
        let (x0, y0) = poly[i];
        let (x1, y1) = poly[(i + 1) % poly.len()];
        if y0 <= y {
            if y1 > y && (x1 - x0) * (y - y0) - (x - x0) * (y1 - y0) > 0.0 {
                w += 1;
            }
        } else if y1 <= y && (x1 - x0) * (y - y0) - (x - x0) * (y1 - y0) < 0.0 {
            w -= 1;
        }
    }
    w
}

pub fn painted(p: &PathSnapshot, x: f64, y: f64) -> bool {
    p.subpaths()
        .map(|sp| winding_of(&flatten(sp.anchors, true, 64), x, y))
        .sum::<i32>()
        != 0
}

/// A xorshift generator: the tests are random but the same on every run.
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next_u64() >> 11) as f64 / (1u64 << 53) as f64)
    }
}
