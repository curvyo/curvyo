//! Independent tester tests for `specs/0024-svg-import-export` slice 3 (criteria 111 to 127):
//! Inkscape's shape objects imported as primitives when their drawn `d` agrees. The inputs are a
//! real Inkscape 1.4.4 save (`tests/fixtures/svg/inkscape_shapes.svg`, made by
//! `tests/fixtures/svg/inkscape_shapes_src/make_inkscape_shapes.sh`) and Inkscape's own Object to
//! Path of it (`inkscape_shapes.paths.svg`) as the geometric reference. Edited and hostile cases
//! change one attribute or one number of a real element. Written from the specification before
//! the implementation was read; all geometry here is computed independently (true arcs, no
//! document-core outline function).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::needless_pass_by_value,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::suboptimal_flops,
    missing_docs
)]

use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use curvyo_document_core::{ArcKind, Curve, DrawingNode, Shape};
use curvyo_ui_core::{SvgPlacement, import_svg_bytes};

// ---------------------------------------------------------------- files and elements

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/svg")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

fn saved() -> String {
    fixture("inkscape_shapes.svg")
}

fn converted() -> String {
    fixture("inkscape_shapes.paths.svg")
}

/// The whole element with `id` (a self-closing shape, or a `<g …>…</g>` without nested groups).
fn element(src: &str, id: &str) -> String {
    let at = src
        .find(&format!("id=\"{id}\""))
        .unwrap_or_else(|| panic!("{id}"));
    let start = src[..at].rfind('<').unwrap();
    let tag: String = src[start + 1..]
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    let open_end = start + src[start..].find('>').unwrap();
    if src.as_bytes()[open_end - 1] == b'/' {
        return src[start..=open_end].to_owned();
    }
    let close = format!("</{tag}>");
    let end = open_end + src[open_end..].find(&close).unwrap() + close.len();
    src[start..end].to_owned()
}

/// The value of attribute `name` in the opening tag of `elem`.
fn attr(elem: &str, name: &str) -> Option<String> {
    let head = &elem[..elem.find('>').unwrap()];
    let key = format!(" {name}=\"");
    let key_nl = format!("\n{name}=\"");
    let pos = head.find(&key).or_else(|| head.find(&key_nl)).or_else(|| {
        // Inkscape indents attributes: "\n       name=".
        head.match_indices(&format!("{name}=\""))
            .find(|(i, _)| head.as_bytes()[i - 1].is_ascii_whitespace())
            .map(|(i, _)| i - 1)
    })?;
    let rest = &head[pos + 1 + name.len() + 2..];
    Some(rest[..rest.find('"').unwrap()].to_owned())
}

/// `elem` with attribute `name` set to `value` (replaced or added).
fn with_attr(elem: &str, name: &str, value: &str) -> String {
    if let Some(old) = attr(elem, name) {
        let from = format!("{name}=\"{old}\"");
        assert!(elem.contains(&from));
        return elem.replacen(&from, &format!("{name}=\"{value}\""), 1);
    }
    let tag_end = elem.find(char::is_whitespace).unwrap();
    format!(
        "{} {name}=\"{value}\"{}",
        &elem[..tag_end],
        &elem[tag_end..]
    )
}

/// An A4 millimetre document holding `body`.
fn doc(body: &str) -> String {
    format!(
        r#"<svg width="210mm" height="297mm" viewBox="0 0 210 297" xmlns="http://www.w3.org/2000/svg" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape">{body}</svg>"#
    )
}

fn import(text: &str) -> curvyo_ui_core::SvgImport {
    import_svg_bytes(text.as_bytes(), SvgPlacement::File).unwrap()
}

/// The single top-level node of `body`.
fn one(body: &str) -> DrawingNode {
    let mut nodes = import(&doc(body)).nodes;
    assert_eq!(nodes.len(), 1, "{body}");
    unwrap_wrapper(nodes.remove(0), body)
}

/// The leaf inside a wrapper `<g transform>` of the test (a group with exactly one child that is
/// not itself a group); any other node as it is.
fn unwrap_wrapper(node: DrawingNode, body: &str) -> DrawingNode {
    if !body.starts_with("<g transform") {
        return node;
    }
    match node {
        DrawingNode::Group { mut children, .. }
            if children.len() == 1 && !matches!(children[0], DrawingNode::Group { .. }) =>
        {
            children.remove(0)
        }
        other => other,
    }
}

/// The real element `id`, edited by `edit`, inside its group's transform.
fn real(id: &str, edit: impl Fn(String) -> String) -> DrawingNode {
    let src = saved();
    let body = edit(element(&src, id));
    match group_of(id) {
        Some(g) => one(&format!(
            "<g transform=\"{}\">{body}</g>",
            attr(&element(&src, g), "transform").unwrap()
        )),
        None => one(&body),
    }
}

fn real_as_is(id: &str) -> DrawingNode {
    real(id, |e| e)
}

/// The group (with a transform) an element of the fixture sits in.
fn group_of(id: &str) -> Option<&'static str> {
    match id {
        "sp_skew" | "a_skew" => Some("g_skew"),
        "a_stretch" | "p_stretch" => Some("g_stretch"),
        "s_in_rot" => Some("g_rot"),
        _ => None,
    }
}

fn name(node: &DrawingNode) -> Option<&str> {
    match node {
        DrawingNode::Group { registers, .. }
        | DrawingNode::Path { registers, .. }
        | DrawingNode::Primitive { registers, .. } => registers.name.as_deref(),
        _ => None,
    }
}

/// Leaves (paths and primitives) in document order.
fn leaves(nodes: &[DrawingNode]) -> Vec<&DrawingNode> {
    let mut out = Vec::new();
    for n in nodes {
        match n {
            DrawingNode::Group { children, .. } => out.extend(leaves(children)),
            DrawingNode::Path { .. } | DrawingNode::Primitive { .. } => out.push(n),
            _ => {}
        }
    }
    out
}

fn is_path(node: &DrawingNode) -> bool {
    matches!(node, DrawingNode::Path { .. })
}

fn kind(node: &DrawingNode) -> &'static str {
    match node {
        DrawingNode::Path { .. } => "path",
        DrawingNode::Group { .. } => "group",
        DrawingNode::Primitive { shape, .. } => match shape {
            Shape::Rect { .. } => "rect",
            Shape::Ellipse { arc: None, .. } => "ellipse",
            Shape::Ellipse { arc: Some(_), .. } => "arc",
            Shape::Polygon { .. } => "polygon",
            Shape::Star { .. } => "star",
        },
        _ => "other",
    }
}

// ---------------------------------------------------------------- geometry (independent)

#[derive(Debug, Clone, Copy, PartialEq)]
struct P {
    x: f64,
    y: f64,
}

const fn p(x: f64, y: f64) -> P {
    P { x, y }
}

fn dist(a: P, b: P) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// One drawn piece: a line, a cubic, or an exact ellipse arc from `t0` to `t1`.
#[derive(Debug, Clone, Copy)]
enum Seg {
    Line(P, P),
    Cubic(P, P, P, P),
    Ell {
        c: P,
        rx: f64,
        ry: f64,
        rot: f64,
        t0: f64,
        t1: f64,
        /// A map applied after (an affine image of an ellipse arc stays exact).
        m: M,
    },
}

impl Seg {
    fn at(&self, t: f64) -> P {
        match *self {
            Self::Line(a, b) => p(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t),
            Self::Cubic(a, b, c, d) => {
                let u = 1.0 - t;
                let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                p(
                    w0 * a.x + w1 * b.x + w2 * c.x + w3 * d.x,
                    w0 * a.y + w1 * b.y + w2 * c.y + w3 * d.y,
                )
            }
            Self::Ell {
                c,
                rx,
                ry,
                rot,
                t0,
                t1,
                m,
            } => {
                let th = t0 + (t1 - t0) * t;
                let (ex, ey) = (rx * th.cos(), ry * th.sin());
                m.apply(p(
                    c.x + ex * rot.cos() - ey * rot.sin(),
                    c.y + ex * rot.sin() + ey * rot.cos(),
                ))
            }
        }
    }

    fn start(&self) -> P {
        self.at(0.0)
    }

    fn end(&self) -> P {
        self.at(1.0)
    }
}

/// An SVG matrix `[a b c d e f]`: x' = a x + c y + e, y' = b x + d y + f.
#[derive(Debug, Clone, Copy)]
struct M([f64; 6]);

impl M {
    const ID: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn then(self, inner: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = inner.0;
        Self([
            a * a2 + c * b2,
            b * a2 + d * b2,
            a * c2 + c * d2,
            b * c2 + d * d2,
            a * e2 + c * f2 + e,
            b * e2 + d * f2 + f,
        ])
    }

    fn apply(self, q: P) -> P {
        let [a, b, c, d, e, f] = self.0;
        p(a * q.x + c * q.y + e, b * q.x + d * q.y + f)
    }
}

fn numbers(s: &str) -> Vec<f64> {
    s.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| t.parse().unwrap())
        .collect()
}

/// An SVG `transform` list (the forms Inkscape writes).
fn transform(s: &str) -> M {
    let mut m = M::ID;
    for part in s.split(')').filter(|t| !t.trim().is_empty()) {
        let (fname, args) = part.split_once('(').unwrap();
        let v = numbers(args);
        let t = match fname.trim() {
            "matrix" => M([v[0], v[1], v[2], v[3], v[4], v[5]]),
            "translate" => M([1.0, 0.0, 0.0, 1.0, v[0], *v.get(1).unwrap_or(&0.0)]),
            "scale" => M([v[0], 0.0, 0.0, *v.get(1).unwrap_or(&v[0]), 0.0, 0.0]),
            "skewX" => M([1.0, 0.0, v[0].to_radians().tan(), 1.0, 0.0, 0.0]),
            "skewY" => M([1.0, v[0].to_radians().tan(), 0.0, 1.0, 0.0, 0.0]),
            "rotate" => {
                let (s, c) = v[0].to_radians().sin_cos();
                let r = M([c, s, -s, c, 0.0, 0.0]);
                if v.len() == 3 {
                    M([1.0, 0.0, 0.0, 1.0, v[1], v[2]])
                        .then(r)
                        .then(M([1.0, 0.0, 0.0, 1.0, -v[1], -v[2]]))
                } else {
                    r
                }
            }
            other => panic!("transform {other}"),
        };
        m = m.then(t);
    }
    m
}

/// An endpoint-parameterised SVG arc as an exact ellipse arc (SVG 1.1 F.6.5, radii scaled up).
fn svg_arc(from: P, rx: f64, ry: f64, phi_deg: f64, large: bool, sweep: bool, to: P) -> Seg {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if from == to {
        return Seg::Line(from, to);
    }
    if rx == 0.0 || ry == 0.0 {
        return Seg::Line(from, to);
    }
    let phi = phi_deg.to_radians();
    let (s, c) = phi.sin_cos();
    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let x1 = c * dx + s * dy;
    let y1 = -s * dx + c * dy;
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let num = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut k = (num / den).sqrt();
    if large == sweep {
        k = -k;
    }
    let cx1 = k * rx * y1 / ry;
    let cy1 = -k * ry * x1 / rx;
    let cx = c * cx1 - s * cy1 + f64::midpoint(from.x, to.x);
    let cy = s * cx1 + c * cy1 + f64::midpoint(from.y, to.y);
    let ang = |ux: f64, uy: f64| uy.atan2(ux);
    let t0 = ang((x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut dt = ang((-x1 - cx1) / rx, (-y1 - cy1) / ry) - t0;
    if sweep && dt < 0.0 {
        dt += TAU;
    } else if !sweep && dt > 0.0 {
        dt -= TAU;
    }
    Seg::Ell {
        c: p(cx, cy),
        rx,
        ry,
        rot: phi,
        t0,
        t1: t0 + dt,
        m: M::ID,
    }
}

/// The subpaths of an SVG `d` (closed ones end with their closing line, if it has length).
fn parse_d(d: &str) -> Vec<Vec<Seg>> {
    let mut toks: Vec<String> = Vec::new();
    let mut cur = String::new();
    let flush = |cur: &mut String, toks: &mut Vec<String>| {
        if !cur.is_empty() {
            toks.push(std::mem::take(cur));
        }
    };
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() && ch != 'e' && ch != 'E' {
            flush(&mut cur, &mut toks);
            toks.push(ch.to_string());
        } else if ch == ',' || ch.is_whitespace() {
            flush(&mut cur, &mut toks);
        } else if (ch == '-' && !cur.is_empty() && !cur.ends_with(['e', 'E']))
            || (ch == '.' && cur.contains('.'))
        {
            flush(&mut cur, &mut toks);
            cur.push(ch);
        } else {
            cur.push(ch);
        }
    }
    flush(&mut cur, &mut toks);
    let mut out: Vec<Vec<Seg>> = Vec::new();
    let mut segs: Vec<Seg> = Vec::new();
    let (mut pos, mut start) = (p(0.0, 0.0), p(0.0, 0.0));
    let mut cmd = 'M';
    let mut i = 0;
    let num = |i: &mut usize| -> f64 {
        let v: f64 = toks[*i].parse().unwrap();
        *i += 1;
        v
    };
    while i < toks.len() {
        if toks[i].len() == 1 && toks[i].chars().next().unwrap().is_ascii_alphabetic() {
            cmd = toks[i].chars().next().unwrap();
            i += 1;
            if cmd == 'Z' || cmd == 'z' {
                if dist(pos, start) > 0.0 {
                    segs.push(Seg::Line(pos, start));
                }
                pos = start;
                out.push(std::mem::take(&mut segs));
                continue;
            }
        }
        let rel = cmd.is_ascii_lowercase();
        let base = if rel { pos } else { p(0.0, 0.0) };
        match cmd.to_ascii_uppercase() {
            'M' => {
                if !segs.is_empty() {
                    out.push(std::mem::take(&mut segs));
                }
                let (x, y) = (num(&mut i), num(&mut i));
                pos = p(base.x + x, base.y + y);
                start = pos;
                cmd = if rel { 'l' } else { 'L' };
            }
            'L' => {
                let (x, y) = (num(&mut i), num(&mut i));
                let q = p(base.x + x, base.y + y);
                segs.push(Seg::Line(pos, q));
                pos = q;
            }
            'H' => {
                let x = num(&mut i);
                let q = p(if rel { pos.x + x } else { x }, pos.y);
                segs.push(Seg::Line(pos, q));
                pos = q;
            }
            'V' => {
                let y = num(&mut i);
                let q = p(pos.x, if rel { pos.y + y } else { y });
                segs.push(Seg::Line(pos, q));
                pos = q;
            }
            'C' => {
                let v: Vec<f64> = (0..6).map(|_| num(&mut i)).collect();
                let b = p(base.x + v[0], base.y + v[1]);
                let c = p(base.x + v[2], base.y + v[3]);
                let q = p(base.x + v[4], base.y + v[5]);
                segs.push(Seg::Cubic(pos, b, c, q));
                pos = q;
            }
            'A' => {
                let v: Vec<f64> = (0..7).map(|_| num(&mut i)).collect();
                let q = p(base.x + v[5], base.y + v[6]);
                segs.push(svg_arc(pos, v[0], v[1], v[2], v[3] != 0.0, v[4] != 0.0, q));
                pos = q;
            }
            other => panic!("path command {other}"),
        }
    }
    if !segs.is_empty() {
        out.push(segs);
    }
    out
}

/// The outline pieces of an imported leaf, in document millimetres.
fn leaf_segs(node: &DrawingNode) -> Vec<Seg> {
    match node {
        DrawingNode::Path { outlines, .. } => {
            let mut segs = Vec::new();
            for o in outlines {
                let a = &o.anchors;
                let n = a.len();
                let count = if o.closed { n } else { n - 1 };
                for k in 0..count {
                    let (u, v) = (&a[k], &a[(k + 1) % n]);
                    let q0 = p(u.point.x, u.point.y);
                    let q3 = p(v.point.x, v.point.y);
                    segs.push(Seg::Cubic(
                        q0,
                        p(q0.x + u.handle_out.x, q0.y + u.handle_out.y),
                        p(q3.x + v.handle_in.x, q3.y + v.handle_in.y),
                        q3,
                    ));
                }
            }
            segs
        }
        DrawingNode::Primitive {
            shape, rotation, ..
        } => primitive_segs(shape, rotation.as_radians()),
        other => panic!("not a leaf: {other:?}"),
    }
}

/// The true outline of a primitive (exact arcs), rotated by `rot` about its centre.
fn primitive_segs(shape: &Shape, rot: f64) -> Vec<Seg> {
    let ring = |pts: Vec<P>| -> Vec<Seg> {
        (0..pts.len())
            .map(|k| Seg::Line(pts[k], pts[(k + 1) % pts.len()]))
            .collect()
    };
    let around = |c: P, r: f64, a: f64| p(c.x + r * a.cos(), c.y + r * a.sin());
    match *shape {
        Shape::Ellipse {
            frame, arc, curve, ..
        } => {
            assert_eq!(
                curve,
                Curve::ELLIPSE_DEFAULT,
                "an imported ellipse is not shaped"
            );
            let c = p(frame.center.x, frame.center.y);
            let (rx, ry) = (frame.rx.as_mm(), frame.ry.as_mm());
            let Some(arc) = arc else {
                return vec![Seg::Ell {
                    c,
                    rx,
                    ry,
                    rot,
                    t0: 0.0,
                    t1: TAU,
                    m: M::ID,
                }];
            };
            let t0 = arc.start().as_radians();
            let t1 = t0 + arc.sweep().as_radians();
            let e = Seg::Ell {
                c,
                rx,
                ry,
                rot,
                t0,
                t1,
                m: M::ID,
            };
            match arc.kind {
                ArcKind::Arc => vec![e],
                ArcKind::Chord => vec![e, Seg::Line(e.end(), e.start())],
                ArcKind::Pie => vec![e, Seg::Line(e.end(), c), Seg::Line(c, e.start())],
            }
        }
        Shape::Polygon {
            frame,
            point_count,
            curve,
        } => {
            assert_eq!(curve, Curve::POLYGON_DEFAULT, "criterion 116: Curve 0%");
            let n = point_count.get();
            let c = p(frame.center.x, frame.center.y);
            let a = frame.angle.as_radians() + rot;
            ring(
                (0..n)
                    .map(|k| {
                        around(
                            c,
                            frame.radius.as_mm(),
                            a + TAU * f64::from(k) / f64::from(n),
                        )
                    })
                    .collect(),
            )
        }
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
            curve,
        } => {
            assert_eq!(
                curve,
                Curve::POLYGON_DEFAULT,
                "a star from Inkscape is straight"
            );
            let n = point_count.get();
            let c = p(frame.center.x, frame.center.y);
            let a = frame.angle.as_radians() + rot;
            let r = frame.radius.as_mm();
            let mut pts = Vec::new();
            for k in 0..n {
                let base = a + TAU * f64::from(k) / f64::from(n);
                pts.push(around(c, r, base));
                pts.push(around(c, r * inner_ratio.get(), base + PI / f64::from(n)));
            }
            ring(pts)
        }
        Shape::Rect { .. } => panic!("no rectangle is compared here"),
    }
}

/// The reference outline of fixture element `id` (in `src`) as Inkscape draws it, transforms of
/// the element and its group applied.
fn reference(src: &str, id: &str) -> Vec<Seg> {
    let elem = element(src, id);
    reference_of(src, id, &elem)
}

/// Inkscape's Object to Path of fixture element `id` (the faces of the box lose their ids there
/// and become `path1` to `path6` in the same order).
fn converted_reference(id: &str) -> Vec<Seg> {
    let conv = converted();
    let face = ["f3", "f1", "f4", "f2", "f5", "f6"]
        .iter()
        .position(|f| *f == id);
    let elem = match face {
        Some(i) => element(&conv, &format!("path{}", i + 1)),
        None => element(&conv, id),
    };
    reference_of(&conv, id, &elem)
}

fn reference_of(src: &str, id: &str, elem: &str) -> Vec<Seg> {
    let mut m = M::ID;
    if let Some(g) = group_of(id) {
        m = transform(&attr(&element(src, g), "transform").unwrap());
    }
    if let Some(t) = attr(elem, "transform") {
        m = m.then(transform(&t));
    }
    let segs: Vec<Seg> = parse_d(&attr(elem, "d").unwrap())
        .into_iter()
        .flatten()
        .collect();
    mapped(&segs, m)
}

/// `segs` under `m` (arcs stay exact: the map is applied to their points).
fn mapped(segs: &[Seg], m: M) -> Vec<Seg> {
    let mut out = Vec::new();
    for s in segs {
        match *s {
            Seg::Line(a, b) => out.push(Seg::Line(m.apply(a), m.apply(b))),
            Seg::Cubic(a, b, c, d) => {
                out.push(Seg::Cubic(m.apply(a), m.apply(b), m.apply(c), m.apply(d)));
            }
            Seg::Ell {
                c,
                rx,
                ry,
                rot,
                t0,
                t1,
                m: inner,
            } => out.push(Seg::Ell {
                c,
                rx,
                ry,
                rot,
                t0,
                t1,
                m: m.then(inner),
            }),
        }
    }
    out
}

/// The largest distance from a point of `a` (at `per` points per piece) to the curve `b`.
fn one_way(a: &[Seg], b: &[Seg], per: u32) -> f64 {
    const COARSE: u32 = 48;
    let coarse: Vec<(usize, f64, P)> = b
        .iter()
        .enumerate()
        .flat_map(|(i, s)| {
            (0..=COARSE).map(move |k| {
                let t = f64::from(k) / f64::from(COARSE);
                (i, t, s.at(t))
            })
        })
        .collect();
    let mut worst: f64 = 0.0;
    for s in a {
        for k in 0..=per {
            let q = s.at(f64::from(k) / f64::from(per));
            let mut best = [(f64::INFINITY, 0_usize, 0.0_f64); 6];
            for &(i, t, c) in &coarse {
                let e = dist(q, c);
                if e < best[5].0 {
                    best[5] = (e, i, t);
                    best.sort_by(|x, y| x.0.total_cmp(&y.0));
                }
            }
            let mut d = best[0].0;
            for &(_, i, t) in best.iter().filter(|b| b.0.is_finite()) {
                let (mut lo, mut hi) = (
                    (t - 1.0 / f64::from(COARSE)).max(0.0),
                    (t + 1.0 / f64::from(COARSE)).min(1.0),
                );
                for _ in 0..80 {
                    let m1 = lo + (hi - lo) / 3.0;
                    let m2 = hi - (hi - lo) / 3.0;
                    if dist(q, b[i].at(m1)) < dist(q, b[i].at(m2)) {
                        hi = m2;
                    } else {
                        lo = m1;
                    }
                }
                d = d.min(dist(q, b[i].at(f64::midpoint(lo, hi))));
            }
            worst = worst.max(d);
        }
    }
    worst
}

/// The two-way distance of the outlines `a` and `b` (every point of one to the other).
fn two_way(a: &[Seg], b: &[Seg]) -> f64 {
    one_way(a, b, 32).max(one_way(b, a, 32))
}

/// The end points of the pieces of `d` (its nodes), in order.
fn d_nodes(d: &str) -> Vec<P> {
    let mut out = Vec::new();
    for sub in parse_d(d) {
        if let Some(first) = sub.first() {
            out.push(first.start());
        }
        out.extend(sub.iter().map(Seg::end));
    }
    out
}

fn leaf_nodes(node: &DrawingNode) -> Vec<P> {
    let DrawingNode::Path { outlines, .. } = node else {
        panic!("{} is not a path", kind(node));
    };
    outlines
        .iter()
        .flat_map(|o| o.anchors.iter().map(|a| p(a.point.x, a.point.y)))
        .collect()
}

/// The path follows `d` (criteria 111, 118, 119): every node of `d` is a node of the imported
/// path (within 1e-6 mm), without arcs in `d` the reverse too, and the two outlines lie within
/// 0.001 mm of each other (an `A` piece comes in as cubics).
fn assert_follows_d(node: &DrawingNode, d: &str) {
    let mine = leaf_nodes(node);
    let theirs = d_nodes(d);
    for q in &theirs {
        assert!(
            mine.iter().any(|m| dist(*m, *q) < 1e-6),
            "node {q:?} of d missing in {mine:?}"
        );
    }
    if !d.contains(['A', 'a']) {
        for q in &mine {
            assert!(
                theirs.iter().any(|m| dist(*m, *q) < 1e-6),
                "imported node {q:?} not in d"
            );
        }
    }
    let theirs: Vec<Seg> = parse_d(d).into_iter().flatten().collect();
    let off = two_way(&leaf_segs(node), &theirs);
    assert!(off <= 0.001, "the path is {off} mm from d");
}

fn arc_of(node: &DrawingNode) -> (ArcKind, f64, f64, P, f64, f64, f64) {
    let DrawingNode::Primitive {
        shape:
            Shape::Ellipse {
                frame,
                arc: Some(arc),
                ..
            },
        rotation,
        ..
    } = node
    else {
        panic!("not an ellipse arc: {}", kind(node));
    };
    (
        arc.kind,
        arc.start().as_radians().to_degrees(),
        arc.end().as_radians().to_degrees(),
        p(frame.center.x, frame.center.y),
        frame.rx.as_mm(),
        frame.ry.as_mm(),
        rotation.as_radians().to_degrees().rem_euclid(360.0),
    )
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn near_deg(a: f64, b: f64, tol: f64) -> bool {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d) <= tol
}

/// Shape facts of a polygon or star: (points, radius, centre, angle with rotation in degrees,
/// ratio).
fn ring_of(node: &DrawingNode) -> (u32, f64, P, f64, Option<f64>) {
    let DrawingNode::Primitive {
        shape, rotation, ..
    } = node
    else {
        panic!("not a primitive: {}", kind(node));
    };
    let r = rotation.as_radians();
    match *shape {
        Shape::Polygon {
            frame, point_count, ..
        } => (
            point_count.get(),
            frame.radius.as_mm(),
            p(frame.center.x, frame.center.y),
            (frame.angle.as_radians() + r).to_degrees(),
            None,
        ),
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
            ..
        } => (
            point_count.get(),
            frame.radius.as_mm(),
            p(frame.center.x, frame.center.y),
            (frame.angle.as_radians() + r).to_degrees(),
            Some(inner_ratio.get()),
        ),
        _ => panic!("not a polygon or star: {}", kind(node)),
    }
}

/// The shape elements of the fixture in document order, as (tag, id).
fn shape_ids(src: &str) -> Vec<(String, String)> {
    let body = &src[src.find("id=\"layer1\"").unwrap()..];
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        let tag: String = rest
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        if ["path", "rect", "ellipse", "circle"].contains(&tag.as_str()) {
            let head = &rest[..rest.find('>').unwrap()];
            let elem = format!("<{head}>");
            out.push((tag, attr(&elem, "id").unwrap()));
        }
    }
    out
}

// ---------------------------------------------------------------- criterion 111 and 125

/// Criteria 111 and 125: every primitive the real Inkscape save gives draws its source `d`
/// within 0.001 mm (each point of either within 0.001 mm of the other, 32 points per piece, exact
/// arcs); every leaf (primitive or path) agrees with Inkscape's own Object to Path within
/// 0.01 mm both ways. The kinds are those of the spec.
#[test]
fn c111_c125_real_save_draws_its_d_and_inkscapes_object_to_path() {
    let src = saved();
    assert!(
        src.contains("inkscape:version=\"1.4.4"),
        "a real Inkscape 1.4.4 save"
    );
    let import = import(&src);
    let got = leaves(&import.nodes);
    let ids = shape_ids(&src);
    assert_eq!(got.len(), ids.len(), "one leaf per shape element");
    let mut kinds = Vec::new();
    let mut worst_d: f64 = 0.0;
    let mut worst_conv: f64 = 0.0;
    for ((tag, id), leaf) in ids.iter().zip(&got) {
        let mine = leaf_segs(leaf);
        let theirs = converted_reference(id);
        let to_conv = two_way(&mine, &theirs);
        assert!(to_conv <= 0.01, "{id}: {to_conv} mm from Object to Path");
        worst_conv = worst_conv.max(to_conv);
        if matches!(leaf, DrawingNode::Primitive { .. }) && tag == "path" {
            let to_d = two_way(&mine, &reference(&src, id));
            assert!(to_d <= 0.001, "{id}: primitive {to_d} mm from its d");
            worst_d = worst_d.max(to_d);
        }
        kinds.push(format!("{id}:{}", kind(leaf)));
    }
    println!("worst primitive to d {worst_d:e} mm, worst leaf to Object to Path {worst_conv:e} mm");
    let expected = [
        "a_slice:arc",
        "a_arc:arc",
        "a_chord:arc",
        "a_big:arc",
        "a_neg:arc",
        "a_whole:ellipse",
        "a_full:ellipse",
        "a_ell:arc",
        "a_rot30:arc",
        "a_mirror:arc",
        "a_label:arc",
        "r_ell:path",
        "p3:polygon",
        "p5:polygon",
        "p6:polygon",
        "p6_rot:polygon",
        "s5:star",
        "s_round:path",
        "s_rand:path",
        "s_twist:path",
        "s_rot:star",
        "s_flip:star",
        "sp1:path",
        "sp_skew:path",
        "a_skew:path",
        "a_stretch:arc",
        "p_stretch:path",
        "s_in_rot:star",
        "f3:path",
        "f1:path",
        "f4:path",
        "f2:path",
        "f5:path",
        "f6:path",
    ];
    assert_eq!(kinds, expected);
}

/// Criterion 111: numbers that contradict the drawn `d` give the path that follows `d` (here the
/// real slice with its end angle, the real star with its inner radius and the real hexagon with
/// its side count changed, `d` untouched).
#[test]
fn c111_numbers_that_disagree_with_d_give_the_path() {
    for (id, key, value) in [
        ("a_slice", "sodipodi:end", "1.6"),
        ("a_slice", "sodipodi:rx", "15.01"),
        ("a_slice", "sodipodi:arc-type", "chord"),
        ("a_arc", "sodipodi:arc-type", "slice"),
        ("s5", "sodipodi:r2", "6"),
        ("s5", "sodipodi:sides", "6"),
        ("p6", "sodipodi:sides", "7"),
        ("p6", "sodipodi:arg1", "0.01"),
    ] {
        let elem = element(&saved(), id);
        let node = real(id, |e| with_attr(&e, key, value));
        assert!(is_path(&node), "{id} {key}={value}: {}", kind(&node));
        assert_follows_d(&node, &attr(&elem, "d").unwrap());
    }
}

/// Criterion 111 (two-way check): `d` that draws a different arc than the numbers, with the same
/// end points, gives the path: the large-arc flag flipped, the sweep flag flipped, and radii too
/// small (SVG scales them up to a half ellipse).
#[test]
fn c111_same_end_points_other_arc_is_a_path() {
    for d in [
        "M 40,25 A 15,15 0 1 1 25,40 V 25 Z",
        "M 40,25 A 15,15 0 0 0 25,40 V 25 Z",
        "M 40,25 A 1,1 0 0 1 25,40 V 25 Z",
    ] {
        let node = real("a_slice", |e| with_attr(&e, "d", d));
        assert!(is_path(&node), "{d}: {}", kind(&node));
    }
}

/// Criteria 111 and 113: a sweep within 0.1° of a full turn is a whole ellipse by the numbers,
/// but the drawn `d` wins: a gap of 0.086° (0.0225 mm on a 15 mm circle) leaves points of the
/// whole circle more than 0.001 mm from `d`, so it is the open path.
#[test]
#[ignore = "0024s3 FAIL: d with a 0.0225 mm gap gives the whole circle; points of the circle lie 0.011 mm from d (only 16 samples per primitive segment are checked)"]
fn c111_c113_gap_wider_than_the_tolerance_stays_a_path() {
    let end = TAU - 0.0015;
    let (ex, ey) = (65.0 + 15.0 * end.cos(), 65.0 + 15.0 * end.sin());
    let d = format!("M 80,65 A 15,15 0 1 1 {ex},{ey}");
    let node = real("a_full", |e| {
        with_attr(&with_attr(&e, "sodipodi:end", &end.to_string()), "d", &d)
    });
    assert!(is_path(&node), "{}", kind(&node));
}

/// Criterion 111 ("every point of one within 0.001 mm of the other"): the real hexagon whose
/// `d` leaves out 0.52 mm of its last side (two open subpaths, the gap between two of the 16
/// samples of that side) is not the closed hexagon: the points of the hexagon in the gap are
/// 0.26 mm from `d`. It must be the path of `d`.
#[test]
#[ignore = "0024s3 FAIL: a hexagon whose d misses 0.52 mm of a side imports as the closed polygon (gap between the 16 samples of the primitive's side)"]
fn c111_gap_in_a_side_is_not_the_closed_polygon() {
    let (a, b) = (p(151.5, 93.741_67), p(158.0, 105.0));
    let at = |t: f64| p(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
    let (g0, g1) = (at(0.51), at(0.55));
    let d = format!(
        "M 158,105 L 151.5,116.25833 L 138.5,116.25833 L 132,105 L 138.5,93.74167 L 151.5,93.74167 L {},{} M {},{} L 158,105",
        g0.x, g0.y, g1.x, g1.y
    );
    // Under scale(8) (a 104 mm side, a 4.2 mm gap), then as drawn.
    let body = format!(
        "<g transform=\"scale(8)\">{}</g>",
        with_attr(&element(&saved(), "p6"), "d", &d)
    );
    let scaled = one(&body);
    let node = real("p6", |e| with_attr(&e, "d", &d));
    assert!(
        is_path(&scaled) && is_path(&node),
        "scaled: {}, as drawn: {}",
        kind(&scaled),
        kind(&node)
    );
}

// ---------------------------------------------------------------- criterion 112 and 113

/// Criterion 112 on the real save: slice, arc and chord of the example (Inkscape writes a chord
/// with `sodipodi:open="true"` and `arc-type="chord"`: the arc-type wins), an arc past 180°, one
/// that starts at a negative angle (Inkscape stores −1 as 5.2831853) and an elliptic arc.
#[test]
fn c112_real_arcs_kind_angles_and_frame() {
    let cases = [
        (
            "a_slice",
            ArcKind::Pie,
            0.0,
            90.0,
            p(25.0, 25.0),
            15.0,
            15.0,
        ),
        ("a_arc", ArcKind::Arc, 0.0, 90.0, p(65.0, 25.0), 15.0, 15.0),
        (
            "a_chord",
            ArcKind::Chord,
            0.0,
            90.0,
            p(105.0, 25.0),
            15.0,
            15.0,
        ),
        (
            "a_big",
            ArcKind::Pie,
            0.5_f64.to_degrees(),
            4.5_f64.to_degrees(),
            p(145.0, 25.0),
            15.0,
            15.0,
        ),
        (
            "a_neg",
            ArcKind::Chord,
            (-1.0_f64).to_degrees(),
            1.0_f64.to_degrees(),
            p(185.0, 25.0),
            15.0,
            15.0,
        ),
        (
            "a_ell",
            ArcKind::Arc,
            0.3_f64.to_degrees(),
            2.5_f64.to_degrees(),
            p(105.0, 65.0),
            15.0,
            8.0,
        ),
    ];
    for (id, k, start, end, c, rx, ry) in cases {
        let (kind, s, e, centre, grx, gry, rot) = arc_of(&real_as_is(id));
        assert_eq!(kind, k, "{id}");
        assert!(
            near_deg(s, start, 1e-4) && near_deg(e, end, 1e-4),
            "{id}: {s} {e}"
        );
        assert!(
            dist(centre, c) < 1e-6 && near(grx, rx, 1e-6) && near(gry, ry, 1e-6),
            "{id}"
        );
        assert!(near_deg(rot, 0.0, 1e-9), "{id}: rotation {rot}");
    }
}

/// Criterion 112: `end` is raised by whole turns while it is not greater than `start` (−12 rad
/// goes up three turns to 0.5664 rad, a 3.8° sliver after `start` 0.5 rad), and `d` drawing that
/// sliver makes the pie.
#[test]
fn c112_end_raised_by_whole_turns() {
    let (s, e) = (0.5_f64, -12.0_f64 + 2.0 * TAU);
    let at = |t: f64| (50.0 + 20.0 * t.cos(), 50.0 + 20.0 * t.sin());
    let ((x0, y0), (x1, y1)) = (at(s), at(e));
    let node = one(&format!(
        r#"<path sodipodi:type="arc" sodipodi:cx="50" sodipodi:cy="50" sodipodi:rx="20" sodipodi:ry="20" sodipodi:start="0.5" sodipodi:end="-12" sodipodi:arc-type="slice" d="M {x0},{y0} A 20,20 0 0 1 {x1},{y1} L 50,50 Z"/>"#
    ));
    let (kind, start, end, ..) = arc_of(&node);
    assert_eq!(kind, ArcKind::Pie);
    assert!(near_deg(start, s.to_degrees(), 1e-6), "{start}");
    assert!(near_deg(end, e.to_degrees(), 1e-6), "{end}");
}

/// Criterion 112 (`0021` criterion 4): an open arc keeps its fill and stroke (the real open arc,
/// given a fill).
#[test]
fn c112_open_arc_keeps_fill_and_stroke() {
    let node = real("a_arc", |e| {
        with_attr(&e, "style", "fill:#ffcc00;stroke:#000000;stroke-width:0.26")
    });
    let DrawingNode::Primitive { style, .. } = &node else {
        panic!("{}", kind(&node));
    };
    assert_eq!(arc_of(&node).0, ArcKind::Arc);
    assert!(style.fill.enabled && style.stroke.enabled);
    assert_eq!(
        (style.fill.color.r, style.fill.color.g, style.fill.color.b),
        (255, 204, 0)
    );
    assert!(near(style.stroke.width.as_mm(), 0.26, 1e-9));
}

/// Criterion 113: Inkscape's arc tool with a sweep of 2π − 0.0001 rad (within 0.1°) and
/// `arc-type="arc"` is a whole circle; an `<ellipse>` (what Inkscape 1.4.4 writes for an arc-tool
/// ellipse without angles) is the whole ellipse; an Inkscape 0.92-style `sodipodi:type="arc"`
/// path without angles drawn as four `A` pieces is the whole ellipse 30 × 16 mm.
#[test]
fn c113_whole_ellipses_from_the_arc_tool() {
    let whole = |node: &DrawingNode| -> (P, f64, f64) {
        let DrawingNode::Primitive {
            shape: Shape::Ellipse {
                frame, arc: None, ..
            },
            ..
        } = node
        else {
            panic!("not a whole ellipse: {}", kind(node));
        };
        (
            p(frame.center.x, frame.center.y),
            frame.rx.as_mm(),
            frame.ry.as_mm(),
        )
    };
    let (c, rx, ry) = whole(&real_as_is("a_full"));
    assert!(dist(c, p(65.0, 65.0)) < 1e-9 && near(rx, 15.0, 1e-9) && near(ry, 15.0, 1e-9));
    let (c, rx, ry) = whole(&real_as_is("a_whole"));
    assert!(dist(c, p(25.0, 65.0)) < 1e-9 && near(rx, 15.0, 1e-9) && near(ry, 8.0, 1e-9));
    let d = attr(&element(&converted(), "a_whole"), "d").unwrap();
    let legacy = one(&format!(
        r#"<path sodipodi:type="arc" sodipodi:cx="25" sodipodi:cy="65" sodipodi:rx="15" sodipodi:ry="8" sodipodi:open="true" sodipodi:arc-type="chord" d="{d}"/>"#
    ));
    let (c, rx, ry) = whole(&legacy);
    assert!(dist(c, p(25.0, 65.0)) < 1e-9 && near(rx, 15.0, 1e-9) && near(ry, 8.0, 1e-9));
}

// ---------------------------------------------------------------- criterion 114

/// Criterion 114 on the real save: Inkscape's rotate(30) of a pie (it moves `cx`/`cy` and writes
/// `transform="rotate(30)"`) is the pie with rotation 30° and the same angles, centred where it
/// was drawn; the flipped chord stays a chord; an aligned stretch keeps the ellipse arc (rx 16,
/// ry 12); a skewed arc and a stretched polygon are paths; an inherited rotate(30) keeps the
/// star; Inkscape's rotate(15) of a hexagon and its flipped star keep their kinds.
#[test]
fn c114_transforms_of_real_shapes() {
    let (kind_r, s, e, c, rx, ry, rot) = arc_of(&real_as_is("a_rot30"));
    assert_eq!(kind_r, ArcKind::Pie);
    assert!(near_deg(rot, 30.0, 1e-6), "rotation {rot}");
    assert!(near_deg(s, 0.0, 1e-4) && near_deg(e, 90.0, 1e-4), "{s} {e}");
    let m = transform("rotate(30)");
    let want = m.apply(p(160.818_88, -20.963_158));
    assert!(dist(c, want) < 1e-6 && near(rx, 15.0, 1e-9) && near(ry, 15.0, 1e-9));

    let (kind_m, ..) = arc_of(&real_as_is("a_mirror"));
    assert_eq!(kind_m, ArcKind::Chord, "a mirror keeps the kind");

    let (kind_s, _, _, c, rx, ry, rot) = arc_of(&real_as_is("a_stretch"));
    assert_eq!(kind_s, ArcKind::Chord);
    assert!(near(rx, 16.0, 1e-9) && near(ry, 12.0, 1e-9) && near_deg(rot, 0.0, 1e-9));
    assert!(dist(c, p(100.0, 190.0)) < 1e-9);

    assert!(is_path(&real_as_is("a_skew")));
    assert!(is_path(&real_as_is("p_stretch")));

    let (n, r, c, angle, ratio) = ring_of(&real_as_is("s_in_rot"));
    assert_eq!(n, 4);
    assert!(near(r, 14.0, 1e-9) && dist(c, p(185.0, 190.0)) < 1e-9);
    assert!(near_deg(angle, 30.0, 1e-6) && near(ratio.unwrap(), 5.0 / 14.0, 1e-9));

    let (n, _, c, angle, ratio) = ring_of(&real_as_is("p6_rot"));
    assert!(n == 6 && ratio.is_none() && dist(c, p(185.0, 105.0)) < 1e-9);
    assert!(near_deg(angle, 15.0, 1e-6), "{angle}");
    let (n, ..) = ring_of(&real_as_is("s_flip"));
    assert_eq!(n, 5);
}

/// Criterion 114: a uniform scale inherited from a group scales the elliptic arc (rx 30, ry 16)
/// and keeps it an arc of the same kind.
#[test]
fn c114_inherited_uniform_scale_keeps_the_arc() {
    let body = format!(
        "<g transform=\"scale(2)\">{}</g>",
        element(&saved(), "a_ell")
    );
    let (k, s, e, c, rx, ry, _) = arc_of(&one(&body));
    assert_eq!(k, ArcKind::Arc);
    assert!(near(rx, 30.0, 1e-9) && near(ry, 16.0, 1e-9) && dist(c, p(210.0, 130.0)) < 1e-9);
    assert!(near_deg(s, 0.3_f64.to_degrees(), 1e-6) && near_deg(e, 2.5_f64.to_degrees(), 1e-6));
}

// ---------------------------------------------------------------- criterion 115

/// Criterion 115: the real `rx=10 ry=5` rectangle is a path (geometry checked against Inkscape in
/// the first test); `rx` alone or `ry` alone gives a rectangle with that radius at all corners.
#[test]
fn c115_elliptic_corners_are_a_path_single_radius_is_a_rectangle() {
    assert!(is_path(&real_as_is("r_ell")));
    for one_radius in [r#"rx="4""#, r#"ry="4""#, r#"rx="4" ry="4""#] {
        let node = one(&format!(
            r#"<rect x="5" y="92" width="40" height="26" {one_radius}/>"#
        ));
        let DrawingNode::Primitive {
            shape: Shape::Rect { corner_radii, .. },
            ..
        } = node
        else {
            panic!("{one_radius}: {}", kind(&node));
        };
        for r in [
            corner_radii.tl,
            corner_radii.tr,
            corner_radii.br,
            corner_radii.bl,
        ] {
            assert!(near(r.as_mm(), 4.0, 1e-9), "{one_radius}");
        }
    }
}

// ---------------------------------------------------------------- criteria 116, 117, 118

/// Criterion 116 on the real save: polygons with 3, 5 and 6 sides. Inkscape 1.4.4 writes
/// `sodipodi:r2="0"` and `arg2="0"` on a polygon that never had them (p3 and p6): `r2` is
/// ignored, so they are still polygons.
#[test]
fn c116_real_polygons() {
    for (id, n, c, angle) in [
        ("p3", 3, p(65.0, 105.0), -90.0),
        ("p5", 5, p(105.0, 105.0), 0.4_f64.to_degrees()),
        ("p6", 6, p(145.0, 105.0), 0.0),
    ] {
        let (gn, r, gc, ga, ratio) = ring_of(&real_as_is(id));
        assert!(ratio.is_none(), "{id} is a polygon");
        assert_eq!(gn, n, "{id}");
        assert!(near(r, 13.0, 1e-9) && dist(gc, c) < 1e-9, "{id}");
        // The polygon's first vertex may be any vertex: compare modulo 360/N.
        let step = 360.0 / f64::from(n);
        let off = (ga - angle).rem_euclid(step);
        assert!(off.min(step - off) < 1e-5, "{id}: {ga} vs {angle}");
    }
}

/// Removes `sodipodi:r2` and `sodipodi:arg2` from an element.
fn without_r2(e: String) -> String {
    let r2 = attr(&e, "sodipodi:r2").unwrap();
    let a2 = attr(&e, "sodipodi:arg2").unwrap();
    e.replace(&format!("sodipodi:r2=\"{r2}\""), "")
        .replace(&format!("sodipodi:arg2=\"{a2}\""), "")
}

/// Criterion 116 in the spec's own words: "`r2` and `arg2` are ignored", so a flat-sided polygon
/// without `sodipodi:r2` and `arg2` is still the polygon (adrs.md slice 3: "`r2` becomes
/// optional: a polygon ignores it"). Inkscape 1.4.4 never writes one: it adds `r2="0"` and
/// `arg2="0"` to a polygon that lacks them (see `inkscape_shapes_src`).
#[test]
#[ignore = "0024s3 FAIL: a polygon without sodipodi:r2 is a path (plan.md keeps r2 required); spec 116 and adrs.md say r2 is ignored"]
fn c116_polygon_without_r2_is_a_polygon() {
    let polygon = real("p5", without_r2);
    assert_eq!(kind(&polygon), "polygon");
}

/// Criteria 111 and 117: a star without `r2` has no ratio: the path of `d`.
#[test]
fn c117_star_without_r2_is_a_path() {
    let star = real("s5", without_r2);
    assert!(is_path(&star), "{}", kind(&star));
    assert_follows_d(&star, &attr(&element(&saved(), "s5"), "d").unwrap());
}

/// Criterion 117 on the real save: the five-pointed star (ratio 5.6 / 14, a point straight up),
/// Inkscape's rotate(20) of a seven-pointed star; the twisted star (arg2 not arg1 + π/N) is the
/// path of `d`.
#[test]
fn c117_real_stars_and_the_twisted_star() {
    let (n, r, c, angle, ratio) = ring_of(&real_as_is("s5"));
    assert_eq!(n, 5);
    assert!(near(r, 14.0, 1e-9) && dist(c, p(25.0, 145.0)) < 1e-9);
    assert!(near(ratio.unwrap(), 5.599_999_9 / 14.0, 1e-9));
    let off = (angle + 90.0).rem_euclid(72.0);
    assert!(off.min(72.0 - off) < 1e-5, "{angle}");
    let (n, _, _, _, ratio) = ring_of(&real_as_is("s_rot"));
    assert!(n == 7 && near(ratio.unwrap(), 0.5, 1e-9));
    let twisted = real_as_is("s_twist");
    assert!(is_path(&twisted));
    assert_follows_d(&twisted, &attr(&element(&saved(), "s_twist"), "d").unwrap());
}

/// Criterion 117: `arg2` off `arg1 + π/N` by 1e-5 rad is twisted (a path) only if `d` shows it;
/// the real `d` of the plain star with `arg2` changed by 1e-5 rad (the numbers contradict the
/// tolerance of 1e-6 rad) is a path.
#[test]
fn c117_arg2_beyond_1e6_rad_is_a_path() {
    let node = real("s5", |e| with_attr(&e, "sodipodi:arg2", "-0.9424678"));
    assert!(is_path(&node), "{}", kind(&node));
}

/// Criterion 118 on the real save: the star with `rounded=0.3` and the one with `randomized=0.2`
/// are paths whose nodes are the nodes of `d`; a real polygon given `rounded="0.0001"` (a
/// rounding below any visible change) is a path too, and a real star with `randomized` set but
/// its `d` the plain one is a path.
#[test]
fn c118_rounded_and_randomized_are_paths_of_d() {
    let src = saved();
    for id in ["s_round", "s_rand"] {
        let node = real_as_is(id);
        assert!(is_path(&node), "{id}");
        assert_follows_d(&node, &attr(&element(&src, id), "d").unwrap());
    }
    let p6 = real("p6", |e| with_attr(&e, "inkscape:rounded", "0.0001"));
    assert!(is_path(&p6), "{}", kind(&p6));
    let s5 = real("s5", |e| with_attr(&e, "inkscape:randomized", "0.0001"));
    assert!(is_path(&s5), "{}", kind(&s5));
}

// ---------------------------------------------------------------- criteria 119 to 122

/// Criteria 119 and 121: the real spiral is one open path named "Spiral" whose nodes are the
/// nodes of `d`; a labelled spiral keeps its label.
#[test]
fn c119_c121_real_spiral_is_one_open_path_named_spiral() {
    let node = real_as_is("sp1");
    let DrawingNode::Path { outlines, .. } = &node else {
        panic!("{}", kind(&node));
    };
    assert_eq!(outlines.len(), 1);
    assert!(!outlines[0].closed);
    assert_eq!(name(&node), Some("Spiral"));
    assert_follows_d(&node, &attr(&element(&saved(), "sp1"), "d").unwrap());
    let labelled = real("sp1", |e| with_attr(&e, "inkscape:label", "Coil"));
    assert_eq!(name(&labelled), Some("Coil"));
}

/// Criteria 120 and 121: the real 3D box is a group (not a layer) named "3D box" of six paths in
/// the file's order (Inkscape reorders faces by depth: f3 f1 f4 f2 f5 f6), each with its own
/// fill; faces are named "Face 1" to "Face 6" by file position and the labelled face ("Lid")
/// keeps its label.
#[test]
fn c120_c121_real_box_is_a_group_of_six_named_faces() {
    let node = real_as_is("box1");
    let DrawingNode::Group {
        layer, children, ..
    } = &node
    else {
        panic!("{}", kind(&node));
    };
    assert!(!layer);
    assert_eq!(name(&node), Some("3D box"));
    let names: Vec<_> = children.iter().map(name).collect();
    assert_eq!(
        names,
        [
            Some("Face 1"),
            Some("Face 2"),
            Some("Lid"),
            Some("Face 4"),
            Some("Face 5"),
            Some("Face 6")
        ]
    );
    let fills: Vec<_> = children
        .iter()
        .map(|c| {
            let DrawingNode::Path { style, .. } = c else {
                panic!("a face is a path");
            };
            (style.fill.color.r, style.fill.color.g, style.fill.color.b)
        })
        .collect();
    assert_eq!(
        fills,
        [
            (0x4d, 0x4d, 0x9f),
            (0x35, 0x35, 0x64),
            (0xe9, 0xe9, 0xff),
            (0xaf, 0xaf, 0xde),
            (0xd7, 0xd7, 0xff),
            (0x86, 0x86, 0xbf)
        ]
    );
    let src = saved();
    for (child, id) in children.iter().zip(["f3", "f1", "f4", "f2", "f5", "f6"]) {
        assert_follows_d(child, &attr(&element(&src, id), "d").unwrap());
    }
}

/// Criterion 120: a child of the box that is not a `box3dside` imports like any group child (a
/// rectangle stays a rectangle and takes no "Face" name); the six faces still count 1 to 6.
#[test]
fn c120_other_children_of_the_box_import_as_usual() {
    let node = real("box1", |e| {
        e.replacen(
            "</g>",
            r#"<rect id="extra" x="10" y="10" width="5" height="5"/></g>"#,
            1,
        )
    });
    let DrawingNode::Group { children, .. } = &node else {
        panic!("{}", kind(&node));
    };
    assert_eq!(children.len(), 7);
    assert_eq!(kind(&children[6]), "rect");
    assert_ne!(name(&children[6]), Some("Face 7"));
    assert_eq!(name(&children[5]), Some("Face 6"));
}

/// Criterion 121: the real slice is named "Arc"; a label wins ("My pie"); the polygon, the star,
/// the whole ellipse from the arc tool and the `<ellipse>` take the primitive's default name.
#[test]
fn c121_names_of_real_shapes() {
    assert_eq!(name(&real_as_is("a_slice")), Some("Arc"));
    assert_eq!(name(&real_as_is("a_label")), Some("My pie"));
    for id in ["p3", "s5", "a_full", "a_whole"] {
        assert_eq!(name(&real_as_is(id)), None, "{id}");
    }
}

/// Criterion 122: a transform on the box group goes into every face (2× and moved by 10, 20):
/// face nodes are the transformed nodes of `d`, the group has no transform to carry, and the
/// stroke width inherited from the box (0.26) is doubled (criterion 34). The skewed spiral keeps
/// its stroke width (the skew has determinant 1).
#[test]
fn c122_transforms_on_box_and_spiral() {
    let node = real("box1", |e| {
        with_attr(&e, "transform", "matrix(2,0,0,2,10,20)")
    });
    let DrawingNode::Group { children, .. } = &node else {
        panic!("{}", kind(&node));
    };
    let src = saved();
    let m = transform("matrix(2,0,0,2,10,20)");
    for (child, id) in children.iter().zip(["f3", "f1", "f4", "f2", "f5", "f6"]) {
        let want: Vec<P> = d_nodes(&attr(&element(&src, id), "d").unwrap())
            .into_iter()
            .map(|q| m.apply(q))
            .collect();
        let mine = leaf_nodes(child);
        for q in &want {
            assert!(mine.iter().any(|g| dist(*g, *q) < 1e-6), "{id}: {q:?}");
        }
        let DrawingNode::Path { style, .. } = child else {
            unreachable!()
        };
        assert!(near(style.stroke.width.as_mm(), 0.52, 1e-9), "{id}");
    }
    let DrawingNode::Path { style, .. } = real_as_is("sp_skew") else {
        panic!("the skewed spiral is a path");
    };
    assert!(near(style.stroke.width.as_mm(), 0.26, 1e-6));
}

// ---------------------------------------------------------------- criterion 123

/// Moves the node `at` of `d` radially away from `centre` by `by` mm (absolute coordinates).
fn moved_node(d: &str, at: &str, centre: P, by: f64) -> String {
    let v = numbers(at);
    let (x, y) = (v[0], v[1]);
    let len = (x - centre.x).hypot(y - centre.y);
    let (nx, ny) = (x + by * (x - centre.x) / len, y + by * (y - centre.y) / len);
    assert!(d.contains(at));
    d.replacen(at, &format!("{nx},{ny}"), 1)
}

/// Criterion 123 on the real save: the inner node of the real arc past 180° moved outward by
/// 0.002 mm gives the path; by 0.0005 mm the pie. The top point of the real star moved up by
/// 0.002 mm gives the path; by 0.0005 mm the star.
#[test]
fn c123_moved_nodes_of_real_shapes() {
    let src = saved();
    let d = attr(&element(&src, "a_big"), "d").unwrap();
    for (by, want_path) in [(0.002, true), (0.0005, false)] {
        let edited = moved_node(&d, "141.10703,39.48602", p(145.0, 25.0), by);
        let node = real("a_big", |e| with_attr(&e, "d", &edited));
        assert_eq!(
            is_path(&node),
            want_path,
            "arc moved by {by}: {}",
            kind(&node)
        );
        if want_path {
            assert_follows_d(&node, &edited);
        }
    }
    let d = attr(&element(&src, "s5"), "d").unwrap();
    assert!(d.starts_with("m 25,131 3.291597,9.4695 "));
    for (by, want_path) in [(0.002, true), (0.0005, false)] {
        // The first node is the start of relative steps: move it and give the next step back.
        let edited = d.replacen(
            "m 25,131 3.291597,9.4695 ",
            &format!("m 25,{} 3.291597,{} ", 131.0 - by, 9.4695 + by),
            1,
        );
        let node = real("s5", |e| with_attr(&e, "d", &edited));
        assert_eq!(
            is_path(&node),
            want_path,
            "star moved by {by}: {}",
            kind(&node)
        );
    }
}

// ---------------------------------------------------------------- criterion 124

/// Criterion 124 on real elements: hostile numbers give the path of `d` (nothing hangs or
/// panics, each import in well under a second); an unusable `d` is skipped.
#[test]
fn c124_hostile_numbers_on_real_shapes() {
    let src = saved();
    let cases = [
        ("p6", "sodipodi:sides", "0"),
        ("p6", "sodipodi:sides", "1"),
        ("p6", "sodipodi:sides", "2"),
        ("p6", "sodipodi:sides", "6.5"),
        ("p6", "sodipodi:sides", "-6"),
        ("p6", "sodipodi:sides", "1025"),
        ("p6", "sodipodi:sides", "4294967302"),
        ("p6", "sodipodi:sides", "1e300"),
        ("p6", "sodipodi:r1", "0"),
        ("p6", "sodipodi:r1", "-13"),
        ("p6", "sodipodi:cx", "NaN"),
        ("p6", "sodipodi:arg1", "inf"),
        ("s5", "sodipodi:r1", "0"),
        ("s5", "sodipodi:r2", "-5.6"),
        ("s5", "sodipodi:r2", "0"),
        ("s5", "sodipodi:r2", "20"),
        ("s5", "sodipodi:r2", "14"),
        ("s5", "sodipodi:r1", "NaN"),
        ("s5", "sodipodi:arg2", "-inf"),
        ("s5", "sodipodi:sides", "abc"),
        ("a_slice", "sodipodi:rx", "0"),
        ("a_slice", "sodipodi:ry", "-15"),
        ("a_slice", "sodipodi:rx", "NaN"),
        ("a_slice", "sodipodi:cx", "inf"),
        ("a_slice", "sodipodi:start", "1e7"),
        ("a_slice", "sodipodi:end", "-1e7"),
        ("a_slice", "sodipodi:end", "1e308"),
        ("a_slice", "sodipodi:start", "6.2831853e6"),
        ("a_slice", "sodipodi:arc-type", "pizza"),
    ];
    for (id, key, value) in cases {
        let elem = element(&src, id);
        let started = std::time::Instant::now();
        let node = real(id, |e| with_attr(&e, key, value));
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "{id} {key}={value}"
        );
        assert!(is_path(&node), "{id} {key}={value}: {}", kind(&node));
        assert_follows_d(&node, &attr(&elem, "d").unwrap());
    }
    for bad_d in ["", "M", "garbage", "M 10 NaN L 20 20", "L 10 10"] {
        let text = doc(&with_attr(&element(&src, "a_slice"), "d", bad_d));
        let import = import(&text);
        assert_eq!(import.leaves, 0, "d={bad_d:?} is skipped");
    }
}

// ---------------------------------------------------------------- widening probes

/// Widening probe (criterion 111, the implementer's true-ellipse check of whole ellipses): an
/// arc-tool whole circle of radius 100 mm whose `d` is the standard four-cubic approximation
/// (as Inkscape 0.48 and most programs write it) is 0.027 mm off the true circle, so the true
/// circle is not within 0.001 mm of `d`: it must be the path.
#[test]
fn c111_four_cubic_circle_is_not_a_true_circle() {
    let k = 0.552_284_749_830_793_4 * 100.0;
    let d = format!(
        "M 200,100 C 200,{a} {a2},200 100,200 C {b},200 0,{a2} 0,100 C 0,{b} {b},0 100,0 C {a2},0 200,{b} 200,100 Z",
        a = 100.0 + k,
        a2 = 100.0 + k,
        b = 100.0 - k
    );
    let node = one(&format!(
        r#"<path sodipodi:type="arc" sodipodi:cx="100" sodipodi:cy="100" sodipodi:rx="100" sodipodi:ry="100" d="{d}"/>"#
    ));
    assert!(is_path(&node), "{}", kind(&node));
}

/// Widening probe (the implementer's two-way check): an edited pie whose `d` runs out along its
/// first radius and back (an extra spur on the outline, the same set of points) is a different
/// cut but the same point set; the spec's every-point rule cannot see it. This pins what the
/// import does so the report can name it: the primitive.
#[test]
fn c111_retraced_spur_has_the_same_point_set() {
    let node = real("a_slice", |e| {
        with_attr(&e, "d", "M 40,25 A 15,15 0 0 1 25,40 V 25 H 40 H 25 Z")
    });
    assert_eq!(
        kind(&node),
        "arc",
        "same point set within 0.001 mm (criterion 111)"
    );
}

// ---------------------------------------------------------------- criterion 126 and 127

/// Linux thread CPU time (`/proc/thread-self/schedstat`, ns on the CPU), as
/// `curvyo-image-core/tests/common`; wall time elsewhere.
fn cpu_now() -> Duration {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok())
        .map_or_else(
            || {
                static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                START.get_or_init(std::time::Instant::now).elapsed()
            },
            Duration::from_nanos,
        )
}

/// 1,000 real arcs, polygons and stars (copies of the saved ones, each moved by its own
/// `transform`).
fn thousand_real_shapes() -> String {
    let src = saved();
    let kinds = [
        "a_slice", "a_big", "a_neg", "a_ell", "p5", "p6", "s5", "s_in_rot",
    ];
    let mut body = String::new();
    for i in 0..1000 {
        let id = kinds[i % kinds.len()];
        let elem = with_attr(&element(&src, id), "id", &format!("x{i}"));
        let group = group_of(id).map_or(String::new(), |g| {
            attr(&element(&src, g), "transform").unwrap()
        });
        write!(
            body,
            "<g transform=\"translate({},{}) {group}\">{elem}</g>",
            (i % 40) * 3,
            (i / 40) * 3
        )
        .unwrap();
    }
    doc(&body)
}

/// Criterion 126: the same bytes give the same document (the real save and 1,000 shapes).
#[test]
fn c126_same_bytes_same_document() {
    for text in [saved(), thousand_real_shapes()] {
        let (a, b) = (import(&text), import(&text));
        assert_eq!(a.nodes, b.nodes);
        assert_eq!(a.leaves, b.leaves);
    }
}

/// Criterion 126: 1,000 mixed real arcs, polygons and stars import in under 0.5 s of this
/// thread's CPU time (native release build, as criterion 71) and all stay primitives.
#[test]
#[ignore = "budget: cargo test --release -p curvyo-ui-core --test acceptance_0024_slice3_tester -- --ignored"]
fn c126_budget_1000_real_shapes_thread_cpu_time() {
    let text = thousand_real_shapes();
    let mut best = Duration::MAX;
    for _ in 0..3 {
        let start = cpu_now();
        let import = import(&text);
        let took = cpu_now().saturating_sub(start);
        best = best.min(took);
        let primitives = leaves(&import.nodes)
            .iter()
            .filter(|n| matches!(n, DrawingNode::Primitive { .. }))
            .count();
        assert_eq!(primitives, 1000);
        println!("1,000 real shapes: {took:?} thread CPU time");
    }
    assert!(best < Duration::from_millis(500), "{best:?}");
}

/// Criterion 127: the supported-subset page has a section "Inkscape shapes" naming the 0.001 mm
/// rule and what stays a path; the technical-debt entry names the shapes that now come in as
/// primitives.
#[test]
fn c127_docs_name_the_inkscape_shapes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let page = std::fs::read_to_string(root.join("docs/svg-import.md")).unwrap();
    let start = page
        .find("\n## Inkscape shapes")
        .expect("a section \"Inkscape shapes\"");
    let section = &page[start..];
    let section = &section[..section[3..].find("\n## ").map_or(section.len(), |e| e + 3)];
    for word in [
        "0.001 mm",
        "radii differ",
        "rounded",
        "randomized",
        "twisted",
        "spiral",
        "3D box",
        "perspective",
    ] {
        assert!(
            section.to_lowercase().contains(&word.to_lowercase()),
            "\"{word}\" in the Inkscape shapes section"
        );
    }
    let debt = std::fs::read_to_string(root.join("docs/technical-debt.md")).unwrap();
    let entry = &debt[debt.find("SVG round-trip is lossy").unwrap()..];
    let entry = &entry[..entry[3..].find("\n## ").map_or(entry.len(), |e| e + 3)];
    assert!(
        entry.contains("polygon") || entry.contains("Polygon"),
        "the debt entry names the shapes now kept"
    );
}
