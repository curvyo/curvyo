//! Independent tester acceptance tests for Part A of
//! `specs/0047-transform-polish/specification.md` (criteria 1 to 16): skew and stretch convert
//! primitives silently. Written from the specification before the implementation diff was
//! read. Everything goes through `Session`'s public API (pointer events, keys, chips, the draw
//! list). Expected values are reference models written here: the explicit "Object to path" of a
//! second session gives the node set, and an affine map fitted to it must be a pure shear (or a
//! pure axis scale) applied to anchors and handle vectors alike.

// Test code: the arithmetic is written the way the specification states it.
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::doc_markdown,
    clippy::items_after_statements,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::needless_range_loop,
    clippy::suboptimal_flops,
    clippy::imprecise_flops,
    clippy::unreadable_literal,
    clippy::manual_assert_eq,
    clippy::collapsible_if,
    clippy::used_underscore_binding,
    clippy::needless_pass_by_value,
    clippy::if_not_else,
    clippy::manual_midpoint,
    clippy::assert_is_empty,
    clippy::cognitive_complexity,
    clippy::unneeded_wildcard_pattern,
    clippy::uninlined_format_args
)]

use std::collections::HashSet;
use std::f64::consts::FRAC_PI_2;

use curvyo_document_core::{
    AnchorId, AnchorKind, Angle, CornerRadii, Document, EllipseFrame, InnerRatio, Length,
    NewAnchor, NodeId, ObjectSnapshot, Point, PointCount, RectBounds, Shape, StarFrame, Vec2, pack,
    unpack,
};
use curvyo_editor_wasm::{KeyHint, KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::EntryOutcome;

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

fn rect_bounds(x: f64, y: f64, w: f64, h: f64) -> RectBounds {
    RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    }
}

fn anchor(n: u64, x: f64, y: f64) -> NewAnchor {
    NewAnchor::corner(AnchorId::new(1, n), pt(x, y))
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).expect("the session output reopens")
}

fn objects(s: &Session) -> Vec<ObjectSnapshot> {
    let d = doc_of(s);
    d.object_ids()
        .into_iter()
        .filter_map(|id| d.object(id))
        .collect()
}

fn oid(o: &ObjectSnapshot) -> NodeId {
    match o {
        ObjectSnapshot::Clone(_) => panic!("no clone here"),
        ObjectSnapshot::Primitive(p) => p.id,
        ObjectSnapshot::Path(p) => p.id,
    }
}

fn loro_of(d: &Document) -> loro::LoroDoc {
    let l = loro::LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    l
}

fn change_count(s: &Session) -> usize {
    loro_of(&doc_of(s)).len_changes()
}

fn last_label(s: &Session) -> String {
    let l = loro_of(&doc_of(s));
    let vv = l.oplog_vv();
    let mut best: Option<(u32, String)> = None;
    for (p, e) in vv.iter() {
        let c = l.get_change(loro::ID::new(*p, *e - 1)).unwrap();
        if best.as_ref().is_none_or(|(b, _)| c.lamport >= *b) {
            best = Some((c.lamport, c.message().to_string()));
        }
    }
    // The history (`0020`) stamps the step time into the message ("..;t=<seconds>"): two sessions
    // started a second apart differ there only.
    let label = best.unwrap().1;
    label.split(";t=").next().unwrap_or(&label).to_owned()
}

fn bytes_of(s: &Session) -> Vec<u8> {
    doc_of(s).export_loro_snapshot().unwrap()
}

fn k_of(s: &Session) -> f64 {
    s.view().scale()
}

fn hold(s: &mut Session, at: Point, shift: bool, ctrl: bool) {
    s.modifiers_changed(shift, ctrl, false);
    s.pointer_hover(at, shift, ctrl);
}

fn drag_mod(s: &mut Session, from: Point, to: Point, shift: bool, ctrl: bool) {
    hold(s, from, shift, ctrl);
    s.pointer_down(from, shift);
    hold(s, to, shift, ctrl);
    s.pointer_up(to, shift, ctrl);
    hold(s, to, false, false);
}

fn drag(s: &mut Session, from: Point, to: Point) {
    drag_mod(s, from, to, false, false);
}

fn double_click_at(s: &mut Session, at: Point) -> bool {
    hold(s, at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    s.double_click(at, false, false)
}

fn key(s: &mut Session, key: &str, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key,
        shift,
        ..KeyInput::default()
    })
}

fn select_everything(s: &mut Session) {
    let n = doc_of(s).object_ids().len();
    key(s, "Escape", false);
    drag(s, pt(-300.0, -300.0), pt(900.0, 900.0));
    assert_eq!(s.selected_object_count(), n);
}

fn is_path(o: &ObjectSnapshot) -> bool {
    matches!(o, ObjectSnapshot::Path(_))
}

fn style_debug(o: &ObjectSnapshot) -> String {
    match o {
        ObjectSnapshot::Primitive(p) => format!("{:?}", p.style),
        ObjectSnapshot::Clone(p) => format!("{:?}", p.resolved.style),
        ObjectSnapshot::Path(p) => format!("{:?}", p.style),
    }
}

/// Anchors (point, handle_in, handle_out, kind) of every outline of a path.
fn anchors_of(o: &ObjectSnapshot) -> Vec<NewAnchor> {
    match o {
        ObjectSnapshot::Path(p) => p.subpaths().flat_map(|sp| sp.anchors.to_vec()).collect(),
        ObjectSnapshot::Primitive(_) | ObjectSnapshot::Clone(_) => {
            panic!("a path was expected, got {o:?}")
        }
    }
}

fn rotate_about_centre(d: &Document, id: NodeId, radians: f64) {
    let o = d.object(id).unwrap();
    let c = match &o {
        ObjectSnapshot::Clone(_) => panic!("no clone here"),
        ObjectSnapshot::Primitive(p) => match p.shape {
            Shape::Rect { bounds, .. } => pt(
                bounds.origin.x + bounds.width.as_mm() / 2.0,
                bounds.origin.y + bounds.height.as_mm() / 2.0,
            ),
            Shape::Ellipse { frame } => frame.center,
            Shape::Polygon { frame, .. } | Shape::Star { frame, .. } => frame.center,
        },
        ObjectSnapshot::Path(_) => panic!("primitives only"),
    };
    d.rotate_object(&o.rotated(c, Angle::from_radians(radians)))
        .unwrap();
}

fn star(d: &Document, cx: f64, cy: f64, r: f64, angle: f64, n: u32) -> NodeId {
    d.create_star(
        StarFrame {
            center: pt(cx, cy),
            radius: Length::from_mm(r),
            angle: Angle::from_radians(angle),
        },
        PointCount::new(n).unwrap(),
        InnerRatio::new(0.45).unwrap(),
    )
}

fn polygon(d: &Document, cx: f64, cy: f64, r: f64, angle: f64, n: u32) -> NodeId {
    d.create_polygon(
        StarFrame {
            center: pt(cx, cy),
            radius: Length::from_mm(r),
            angle: Angle::from_radians(angle),
        },
        PointCount::new(n).unwrap(),
    )
}

fn ellipse(d: &Document, cx: f64, cy: f64, rx: f64, ry: f64) -> NodeId {
    d.create_ellipse(EllipseFrame {
        center: pt(cx, cy),
        rx: Length::from_mm(rx),
        ry: Length::from_mm(ry),
    })
}

fn round_rect(d: &Document, x: f64, y: f64, w: f64, h: f64, r: f64) -> NodeId {
    let id = d.create_rect(rect_bounds(x, y, w, h));
    d.set_corner_radii(&[(id, CornerRadii::uniform(Length::from_mm(r)))])
        .unwrap();
    id
}

fn curved_path(d: &Document, n0: u64, cx: f64, cy: f64) -> NodeId {
    let mut a1 = anchor(n0, cx - 9.0, cy - 6.0);
    a1.handle_out = Vec2::new(3.0, -2.0);
    let mut a2 = anchor(n0 + 1, cx + 9.0, cy - 6.0);
    a2.handle_in = Vec2::new(-3.0, -2.0);
    a2.handle_out = Vec2::new(2.0, 4.0);
    a2.kind = AnchorKind::Symmetric;
    let a3 = anchor(n0 + 2, cx + 9.0, cy + 6.0);
    let a4 = anchor(n0 + 3, cx - 9.0, cy + 6.0);
    d.create_path(&[a1, a2, a3, a4], true)
}

/// Every kind the spec names, with the tag the tests print.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Path,
    Rect,
    RoundRect,
    Ellipse,
    Circle33,
    RotRect30,
    RotRect90,
    RotEllipse25,
    Polygon5,
    Star5,
}

const ALL_KINDS: [Kind; 10] = [
    Kind::Path,
    Kind::Rect,
    Kind::RoundRect,
    Kind::Ellipse,
    Kind::Circle33,
    Kind::RotRect30,
    Kind::RotRect90,
    Kind::RotEllipse25,
    Kind::Polygon5,
    Kind::Star5,
];

fn add(d: &Document, kind: Kind, n0: u64, cx: f64, cy: f64) -> NodeId {
    match kind {
        Kind::Path => curved_path(d, n0, cx, cy),
        Kind::Rect => d.create_rect(rect_bounds(cx - 10.0, cy - 6.0, 20.0, 12.0)),
        Kind::RoundRect => round_rect(d, cx - 10.0, cy - 6.0, 20.0, 12.0, 3.0),
        Kind::Ellipse => ellipse(d, cx, cy, 10.0, 6.0),
        Kind::Circle33 => {
            let id = ellipse(d, cx, cy, 6.0, 6.0);
            rotate_about_centre(d, id, 33.0_f64.to_radians());
            id
        }
        Kind::RotRect30 => {
            let id = d.create_rect(rect_bounds(cx - 10.0, cy - 6.0, 20.0, 12.0));
            rotate_about_centre(d, id, 30.0_f64.to_radians());
            id
        }
        Kind::RotRect90 => {
            let id = d.create_rect(rect_bounds(cx - 10.0, cy - 6.0, 20.0, 12.0));
            rotate_about_centre(d, id, FRAC_PI_2);
            id
        }
        Kind::RotEllipse25 => {
            let id = ellipse(d, cx, cy, 10.0, 6.0);
            rotate_about_centre(d, id, 25.0_f64.to_radians());
            id
        }
        Kind::Polygon5 => polygon(d, cx, cy, 8.0, -FRAC_PI_2, 5),
        Kind::Star5 => star(d, cx, cy, 8.0, -FRAC_PI_2, 5),
    }
}

fn single(kind: Kind) -> Document {
    let d = Document::new(1);
    let _ = add(&d, kind, 1, 120.0, 100.0);
    d
}

/// Whether the maker may still see a primitive after a stretch of `kind`.
fn stays_primitive_in_stretch(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Rect | Kind::RoundRect | Kind::Ellipse | Kind::Circle33 | Kind::RotRect90
    )
}

fn otp_objects(d: &Document) -> Vec<ObjectSnapshot> {
    let mut s = open(d);
    select_everything(&mut s);
    s.convert_selected_to_paths();
    objects(&s)
}

// ---- outline samples ---------------------------------------------------

fn bezier_samples(a: &[NewAnchor], closed: bool, per: usize) -> Vec<Point> {
    let n = a.len();
    let segs = if closed { n } else { n.saturating_sub(1) };
    let mut out = Vec::new();
    for i in 0..segs {
        let (p, q) = (&a[i], &a[(i + 1) % n]);
        let p0 = p.point;
        let p1 = pt(p.point.x + p.handle_out.x, p.point.y + p.handle_out.y);
        let p2 = pt(q.point.x + q.handle_in.x, q.point.y + q.handle_in.y);
        let p3 = q.point;
        for step in 0..per {
            let t = step as f64 / per as f64;
            let u = 1.0 - t;
            out.push(pt(
                u * u * u * p0.x
                    + 3.0 * u * u * t * p1.x
                    + 3.0 * u * t * t * p2.x
                    + t * t * t * p3.x,
                u * u * u * p0.y
                    + 3.0 * u * u * t * p1.y
                    + 3.0 * u * t * t * p2.y
                    + t * t * t * p3.y,
            ));
        }
    }
    if out.is_empty() {
        out.extend(a.iter().map(|x| x.point));
    }
    out
}

/// Dense polyline of the drawn outline of an object (primitives through their path form).
fn polyline(o: &ObjectSnapshot) -> Vec<Point> {
    match o {
        ObjectSnapshot::Clone(_) => panic!("no clone here"),
        ObjectSnapshot::Primitive(p) => {
            let out = curvyo_document_core::outline_of_rotated(&p.shape, p.rotation);
            let a: Vec<NewAnchor> = out
                .iter()
                .map(|x| {
                    let mut n = anchor(1, x.point.x, x.point.y);
                    n.handle_in = x.handle_in;
                    n.handle_out = x.handle_out;
                    n
                })
                .collect();
            bezier_samples(&a, true, 64)
        }
        ObjectSnapshot::Path(p) => p
            .subpaths()
            .flat_map(|sp| bezier_samples(sp.anchors, sp.closed, 64))
            .collect(),
    }
}

fn tight_bounds(os: &[ObjectSnapshot]) -> (Point, Point) {
    let mut lo = pt(f64::MAX, f64::MAX);
    let mut hi = pt(f64::MIN, f64::MIN);
    for o in os {
        for p in polyline(o) {
            lo = pt(lo.x.min(p.x), lo.y.min(p.y));
            hi = pt(hi.x.max(p.x), hi.y.max(p.y));
        }
    }
    (lo, hi)
}

fn dist_to_polyline(p: Point, poly: &[Point], closed: bool) -> f64 {
    let n = poly.len();
    let segs = if closed { n } else { n - 1 };
    let mut best = f64::MAX;
    for i in 0..segs {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let l2 = dx * dx + dy * dy;
        let t = if l2 == 0.0 {
            0.0
        } else {
            (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
        };
        let q = pt(a.x + t * dx, a.y + t * dy);
        best = best.min((p.x - q.x).hypot(p.y - q.y));
    }
    best
}

// ---- boxes and maps ----------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct GBox {
    min: Point,
    max: Point,
    k: f64,
}

impl GBox {
    fn of(s: &Session) -> Self {
        let (min, max) = tight_bounds(&objects(s));
        Self {
            min,
            max,
            k: k_of(s),
        }
    }
    fn w(&self) -> f64 {
        self.max.x - self.min.x
    }
    fn h(&self) -> f64 {
        self.max.y - self.min.y
    }
    fn centre(&self) -> Point {
        pt(self.min.x + self.w() / 2.0, self.min.y + self.h() / 2.0)
    }
    fn corner(&self, sx: f64, sy: f64) -> Point {
        let c = self.centre();
        pt(c.x + sx * self.w() / 2.0, c.y + sy * self.h() / 2.0)
    }
    fn mid(&self, nx: f64, ny: f64) -> Point {
        let c = self.centre();
        pt(c.x + nx * self.w() / 2.0, c.y + ny * self.h() / 2.0)
    }
    fn skew(&self, nx: f64, ny: f64) -> Point {
        let p = self.mid(nx, ny);
        pt(p.x + nx * 16.0 / self.k, p.y + ny * 16.0 / self.k)
    }
}

/// A linear map with a translation: p' = A p + t.
#[derive(Clone, Copy, Debug)]
struct Aff {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    tx: f64,
    ty: f64,
}

impl Aff {
    fn p(&self, q: Point) -> Point {
        pt(
            self.a * q.x + self.b * q.y + self.tx,
            self.c * q.x + self.d * q.y + self.ty,
        )
    }
    fn v(&self, v: Vec2) -> Vec2 {
        Vec2::new(self.a * v.x + self.b * v.y, self.c * v.x + self.d * v.y)
    }
    /// Factors (sx, sy) about (ax, ay).
    fn scale(ax: f64, ay: f64, sx: f64, sy: f64) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            tx: ax - sx * ax,
            ty: ay - sy * ay,
        }
    }
    fn det(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }
    /// The Frobenius norm of the linear part minus the identity.
    fn frob_minus_identity(&self) -> f64 {
        ((self.a - 1.0).powi(2) + self.b.powi(2) + self.c.powi(2) + (self.d - 1.0).powi(2)).sqrt()
    }
    /// `(A - I)^2` must vanish for a shear.
    fn nilpotency(&self) -> f64 {
        let (m00, m01, m10, m11) = (self.a - 1.0, self.b, self.c, self.d - 1.0);
        let q = [
            m00 * m00 + m01 * m10,
            m00 * m01 + m01 * m11,
            m10 * m00 + m11 * m10,
            m10 * m01 + m11 * m11,
        ];
        q.iter().fold(0.0_f64, |a, b| a.max(b.abs()))
    }
}

/// Fits the affine map that takes `src` to `dst` (same order) through the three source points
/// that span the largest triangle.
fn fit_affine(src: &[Point], dst: &[Point]) -> Aff {
    assert_eq!(src.len(), dst.len());
    // Three well-spread points among the first forty are enough (and keep this cheap).
    let n = src.len().min(40);
    let mut best = (0, 1, 2, -1.0_f64);
    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                let cross = ((src[j].x - src[i].x) * (src[k].y - src[i].y)
                    - (src[j].y - src[i].y) * (src[k].x - src[i].x))
                    .abs();
                if cross > best.3 {
                    best = (i, j, k, cross);
                }
            }
        }
    }
    let (i, j, k, area) = best;
    assert!(area > 1e-9, "the source points are collinear");
    let u1 = (src[j].x - src[i].x, src[j].y - src[i].y);
    let u2 = (src[k].x - src[i].x, src[k].y - src[i].y);
    let w1 = (dst[j].x - dst[i].x, dst[j].y - dst[i].y);
    let w2 = (dst[k].x - dst[i].x, dst[k].y - dst[i].y);
    let det = u1.0 * u2.1 - u2.0 * u1.1;
    // inverse of S = [u1 u2] (columns)
    let inv = [[u2.1 / det, -u2.0 / det], [-u1.1 / det, u1.0 / det]];
    // A = D * S^-1 where D = [w1 w2]
    let a = w1.0 * inv[0][0] + w2.0 * inv[1][0];
    let b = w1.0 * inv[0][1] + w2.0 * inv[1][1];
    let c = w1.1 * inv[0][0] + w2.1 * inv[1][0];
    let d = w1.1 * inv[0][1] + w2.1 * inv[1][1];
    let tx = dst[i].x - (a * src[i].x + b * src[i].y);
    let ty = dst[i].y - (c * src[i].x + d * src[i].y);
    Aff { a, b, c, d, tx, ty }
}

/// `new` equals `old` (an "Object to path" result) mapped by one affine map applied to anchors
/// and handle vectors; returns that map. Node count, node kinds and subpath structure equal.
fn fitted_image(old: &ObjectSnapshot, new: &ObjectSnapshot, what: &str) -> Aff {
    let (ObjectSnapshot::Path(o), ObjectSnapshot::Path(n)) = (old, new) else {
        panic!("{what}: two paths expected, got {old:?} / {new:?}");
    };
    let os: Vec<_> = o.subpaths().collect();
    let ns: Vec<_> = n.subpaths().collect();
    assert_eq!(os.len(), ns.len(), "{what}: subpath count");
    let oa: Vec<NewAnchor> = os.iter().flat_map(|s| s.anchors.to_vec()).collect();
    let na: Vec<NewAnchor> = ns.iter().flat_map(|s| s.anchors.to_vec()).collect();
    assert_eq!(oa.len(), na.len(), "{what}: node count");
    for (x, y) in os.iter().zip(&ns) {
        assert_eq!(x.closed, y.closed, "{what}: closed");
    }
    let src: Vec<Point> = oa.iter().map(|a| a.point).collect();
    let dst: Vec<Point> = na.iter().map(|a| a.point).collect();
    let m = fit_affine(&src, &dst);
    for (i, (a, b)) in oa.iter().zip(&na).enumerate() {
        let want = m.p(a.point);
        assert!(
            near(want.x, b.point.x, 1e-6) && near(want.y, b.point.y, 1e-6),
            "{what}: anchor {i} is ({}, {}), the fitted map says ({}, {})",
            b.point.x,
            b.point.y,
            want.x,
            want.y
        );
        let (hi, ho) = (m.v(a.handle_in), m.v(a.handle_out));
        assert!(
            near(hi.x, b.handle_in.x, 1e-6) && near(hi.y, b.handle_in.y, 1e-6),
            "{what}: handle_in {i} is not the mapped vector"
        );
        assert!(
            near(ho.x, b.handle_out.x, 1e-6) && near(ho.y, b.handle_out.y, 1e-6),
            "{what}: handle_out {i} is not the mapped vector"
        );
        assert_eq!(a.kind, b.kind, "{what}: node kind {i}");
    }
    m
}

fn assert_pure_shear(m: &Aff, tan: f64, what: &str) {
    assert!(
        near(m.det(), 1.0, 1e-7),
        "{what}: area changed, det {}",
        m.det()
    );
    assert!(m.nilpotency() < 1e-7, "{what}: not a shear, {m:?}");
    assert!(
        near(m.frob_minus_identity().abs(), tan.abs(), 1e-6),
        "{what}: shear strength {} != {tan}",
        m.frob_minus_identity()
    );
}

/// All anchor ids and node ids of a document are pairwise different.
fn assert_ids_unique(s: &Session, what: &str) {
    let os = objects(s);
    let mut seen_ids = HashSet::new();
    let mut seen_anchor = HashSet::new();
    for o in &os {
        assert!(seen_ids.insert(oid(o)), "{what}: duplicate object id");
        if let ObjectSnapshot::Path(_) = o {
            for a in anchors_of(o) {
                assert!(
                    seen_anchor.insert(format!("{:?}", a.id)),
                    "{what}: duplicate anchor id {:?}",
                    a.id
                );
            }
        }
    }
}

// ---------------------------------------------------------------------
// Criterion 1: skew handles for every kind
// ---------------------------------------------------------------------

fn skew_cursor_at(s: &mut Session, p: Point) -> bool {
    hold(s, p, false, false);
    s.cursor_hint().starts_with("skew:")
}

/// Criterion 1: a single object of an axis-aligned kind whose box is the tight bounds shows all
/// four skew handles at the layout of `0008`/`0019` (16 px out from the side midpoint).
#[test]
fn ac01_single_object_skew_handles_for_every_kind() {
    for kind in [
        Kind::Path,
        Kind::Rect,
        Kind::RoundRect,
        Kind::Ellipse,
        Kind::RotRect90,
    ] {
        let d = single(kind);
        let mut s = open(&d);
        select_everything(&mut s);
        let g = GBox::of(&s);
        for (nx, ny) in [(0.0, -1.0), (0.0, 1.0), (1.0, 0.0), (-1.0, 0.0)] {
            assert!(
                skew_cursor_at(&mut s, g.skew(nx, ny)),
                "{kind:?}: no skew handle on side ({nx}, {ny})"
            );
        }
    }
}

/// Criterion 1: a lone polygon with a box equal to its tight bounds (diamond) and a lone star
/// show the skew handles; a primitive pair too (group box = tight bounds).
#[test]
fn ac01_lone_polygon_and_star_and_pairs_have_skew_handles() {
    let d = Document::new(1);
    let _ = polygon(&d, 120.0, 100.0, 8.0, -FRAC_PI_2, 4);
    let mut s = open(&d);
    select_everything(&mut s);
    let g = GBox::of(&s);
    for (nx, ny) in [(0.0, -1.0), (0.0, 1.0), (1.0, 0.0), (-1.0, 0.0)] {
        assert!(
            skew_cursor_at(&mut s, g.skew(nx, ny)),
            "diamond ({nx},{ny})"
        );
    }
    for kind in ALL_KINDS {
        // Pair: the kind with a plain rect, so the group box is the tight bounds.
        let d = Document::new(1);
        let _ = add(&d, kind, 1, 120.0, 100.0);
        let _ = d.create_rect(rect_bounds(150.0, 94.0, 10.0, 12.0));
        let mut s = open(&d);
        select_everything(&mut s);
        let g = GBox::of(&s);
        for (nx, ny) in [(0.0, -1.0), (0.0, 1.0), (1.0, 0.0), (-1.0, 0.0)] {
            assert!(
                skew_cursor_at(&mut s, g.skew(nx, ny)),
                "pair {kind:?}: no skew handle on side ({nx}, {ny})"
            );
        }
    }
}

/// Criterion 1 (negative): nothing selected shows no skew handle.
#[test]
fn ac01_no_selection_no_skew_handle() {
    let d = single(Kind::Rect);
    let mut s = open(&d);
    select_everything(&mut s);
    let g = GBox::of(&s);
    key(&mut s, "Escape", false);
    assert_eq!(s.selected_object_count(), 0);
    assert!(!skew_cursor_at(&mut s, g.skew(0.0, -1.0)));
}

// ---------------------------------------------------------------------
// Criterion 2: the skew example and the exact image
// ---------------------------------------------------------------------

/// Criterion 2, the spec's example verbatim: a 10 x 10 mm rectangle with its top-left corner at
/// (0, 0), K (top handle, bottom side fixed), 45 degrees: one closed path with four corner nodes
/// at (10, 0), (20, 0), (10, 10), (0, 10), the style of the rectangle; the selection box is the
/// box of the skewed path.
#[test]
fn ac02_example_rectangle_k_45() {
    let d = Document::new(1);
    let id = d.create_rect(rect_bounds(0.0, 0.0, 10.0, 10.0));
    let style_before = style_debug(&d.object(id).unwrap());
    let mut s = open(&d);
    select_everything(&mut s);
    assert_eq!(key(&mut s, "k", false), KeyOutcome::EntryOpened);
    assert_eq!(
        s.commit_transform_entry("45", "", 0),
        EntryOutcome::Committed
    );
    let os = objects(&s);
    assert_eq!(os.len(), 1);
    assert!(is_path(&os[0]), "the rectangle became a path");
    let ObjectSnapshot::Path(p) = &os[0] else {
        unreachable!()
    };
    assert!(p.closed);
    assert_eq!(p.anchors.len(), 4);
    assert!(p.extra_subpaths.is_empty());
    let want = [(10.0, 0.0), (20.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    for (x, y) in want {
        assert!(
            p.anchors
                .iter()
                .any(|a| near(a.point.x, x, 1e-7) && near(a.point.y, y, 1e-7)),
            "vertex ({x}, {y}) missing in {:?}",
            p.anchors.iter().map(|a| a.point).collect::<Vec<_>>()
        );
    }
    for a in &p.anchors {
        assert_eq!(a.kind, AnchorKind::Corner);
        assert!(a.handle_in.x.abs() < 1e-9 && a.handle_out.y.abs() < 1e-9);
    }
    assert_eq!(style_debug(&os[0]), style_before, "style kept");
    assert_eq!(s.selected_object_count(), 1, "it stays selected");
    // The selection box is the box of the skewed path: the right skew handle sits at the
    // new tight bounds, 16 px out from the middle of the right side.
    let g = GBox::of(&s);
    assert!(near(g.min.x, 0.0, 1e-7) && near(g.max.x, 20.0, 1e-7));
    assert!(skew_cursor_at(&mut s, g.skew(1.0, 0.0)));
}

/// Criterion 2: every kind alone, typed K 30 degrees: the result is the "Object to path" path
/// of the same object mapped by one pure shear of strength tan 30, handle vectors included.
#[test]
fn ac02_every_kind_skews_to_the_exact_image() {
    for kind in ALL_KINDS {
        let d = single(kind);
        let otp = otp_objects(&d);
        let mut s = open(&d);
        select_everything(&mut s);
        assert_eq!(key(&mut s, "k", false), KeyOutcome::EntryOpened, "{kind:?}");
        assert_eq!(
            s.commit_transform_entry("30", "", 0),
            EntryOutcome::Committed,
            "{kind:?}"
        );
        let after = objects(&s);
        assert_eq!(after.len(), 1);
        let m = fitted_image(&otp[0], &after[0], &format!("{kind:?}"));
        assert_pure_shear(&m, 30.0_f64.to_radians().tan(), &format!("{kind:?}"));
    }
}

/// Criterion 2: Shift+K (right handle) likewise.
#[test]
fn ac02_shift_k_every_kind() {
    for kind in ALL_KINDS {
        let d = single(kind);
        let otp = otp_objects(&d);
        let mut s = open(&d);
        select_everything(&mut s);
        assert_eq!(key(&mut s, "k", true), KeyOutcome::EntryOpened, "{kind:?}");
        assert_eq!(
            s.commit_transform_entry("-20", "", 0),
            EntryOutcome::Committed,
            "{kind:?}"
        );
        let after = objects(&s);
        let m = fitted_image(&otp[0], &after[0], &format!("shift {kind:?}"));
        assert_pure_shear(&m, 20.0_f64.to_radians().tan(), &format!("shift {kind:?}"));
    }
}

/// Criterion 2/3: a mixed selection (all kinds together) skews as one: every object is the
/// exact image of its "Object to path" form under one common shear.
#[test]
fn ac02_mixed_selection_common_shear() {
    let d = Document::new(1);
    for (i, kind) in ALL_KINDS.iter().enumerate() {
        let col = (i % 5) as f64;
        let row = (i / 5) as f64;
        let _ = add(
            &d,
            *kind,
            10 * (i as u64 + 1),
            60.0 + col * 40.0,
            60.0 + row * 40.0,
        );
    }
    let otp = otp_objects(&d);
    let mut s = open(&d);
    select_everything(&mut s);
    assert_eq!(key(&mut s, "k", false), KeyOutcome::EntryOpened);
    assert_eq!(
        s.commit_transform_entry("35", "", 0),
        EntryOutcome::Committed
    );
    let after = objects(&s);
    assert_eq!(after.len(), otp.len());
    let mut first: Option<Aff> = None;
    for (i, (o, n)) in otp.iter().zip(&after).enumerate() {
        assert!(is_path(n), "object {i} ({:?}) is a path", ALL_KINDS[i]);
        assert_eq!(
            oid(o),
            oid(n),
            "object {i}: identity kept like Object to path"
        );
        let m = fitted_image(o, n, &format!("object {i} {:?}", ALL_KINDS[i]));
        assert_pure_shear(
            &m,
            35.0_f64.to_radians().tan(),
            &format!("{:?}", ALL_KINDS[i]),
        );
        match first {
            None => first = Some(m),
            Some(f) => {
                for (x, y) in [
                    (f.a, m.a),
                    (f.b, m.b),
                    (f.c, m.c),
                    (f.d, m.d),
                    (f.tx, m.tx),
                    (f.ty, m.ty),
                ] {
                    assert!(
                        near(x, y, 1e-6),
                        "one common shear for the selection: {f:?} vs {m:?}"
                    );
                }
            }
        }
    }
    assert_ids_unique(&s, "mixed skew");
}

/// Criterion 2 with a drag: the top skew handle dragged 10 mm sideways turns a rectangle,
/// ellipse, polygon and star into the sheared "Object to path" form (strength > 0, pure shear).
#[test]
fn ac02_drag_every_unrotated_kind() {
    for kind in [
        Kind::Path,
        Kind::Rect,
        Kind::RoundRect,
        Kind::Ellipse,
        Kind::RotRect90,
    ] {
        let d = single(kind);
        let otp = otp_objects(&d);
        let mut s = open(&d);
        select_everything(&mut s);
        let g = GBox::of(&s);
        let h = g.skew(0.0, -1.0);
        let c = change_count(&s);
        drag(&mut s, h, pt(h.x + 10.0, h.y));
        assert_eq!(change_count(&s), c + 1, "{kind:?}: one commit");
        let after = objects(&s);
        assert!(is_path(&after[0]), "{kind:?} is a path");
        let m = fitted_image(&otp[0], &after[0], &format!("drag {kind:?}"));
        assert!(
            m.det() > 0.999999 && m.det() < 1.000001,
            "{kind:?} det {}",
            m.det()
        );
        assert!(m.nilpotency() < 1e-7, "{kind:?}");
        assert!(
            m.frob_minus_identity() > 0.05,
            "{kind:?}: the skew did something"
        );
        // Top moves right with the bottom side fixed.
        assert!(
            m.b < -1e-6,
            "{kind:?}: x' = x - t*(y - y0) with t > 0, got b = {}",
            m.b
        );
        let bottom = g.max.y;
        let p = m.p(pt(120.0, bottom));
        assert!(
            near(p.x, 120.0, 1e-6),
            "{kind:?}: the bottom side stays fixed"
        );
    }
    // The same for a lone diamond polygon and an aligned star (box = tight bounds).
    for (name, mk) in [("diamond", 0), ("4-star", 1)] {
        let d = Document::new(1);
        let _ = if mk == 0 {
            polygon(&d, 120.0, 100.0, 8.0, -FRAC_PI_2, 4)
        } else {
            star(&d, 120.0, 100.0, 8.0, -FRAC_PI_2, 4)
        };
        let otp = otp_objects(&d);
        let mut s = open(&d);
        select_everything(&mut s);
        let g = GBox::of(&s);
        let h = g.skew(0.0, -1.0);
        drag(&mut s, h, pt(h.x + 10.0, h.y));
        let after = objects(&s);
        assert!(is_path(&after[0]), "{name} is a path after the drag");
        let m = fitted_image(&otp[0], &after[0], name);
        assert!(
            m.nilpotency() < 1e-7 && m.frob_minus_identity() > 0.05,
            "{name}"
        );
    }
}

/// Deterministic pseudo-random sweep: random polygons and stars with random angle, radius,
/// point count and shear angle are always the exact image of their "Object to path" form.
#[test]
fn ac02_sweep_random_polygons_and_stars_shear_exactly() {
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut rnd = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 33) as f64) / f64::from(1_u32 << 31)
    };
    for _ in 0..40 {
        let n = 3 + (rnd() * 9.0) as u32;
        let r = 3.0 + rnd() * 20.0;
        let ang = rnd() * std::f64::consts::TAU;
        let shear_deg = (rnd() * 140.0) - 70.0;
        let is_star = rnd() < 0.5;
        let d = Document::new(1);
        let (cx, cy) = (120.0 + rnd() * 30.0, 100.0 + rnd() * 30.0);
        let _ = if is_star {
            star(&d, cx, cy, r, ang, n)
        } else {
            polygon(&d, cx, cy, r, ang, n)
        };
        let otp = otp_objects(&d);
        let mut s = open(&d);
        select_everything(&mut s);
        let which_shift = rnd() < 0.5;
        assert_eq!(key(&mut s, "k", which_shift), KeyOutcome::EntryOpened);
        let out = s.commit_transform_entry(&format!("{shear_deg}"), "", 0);
        if shear_deg.abs() < 1e-9 {
            continue;
        }
        assert_eq!(out, EntryOutcome::Committed, "n={n} r={r} deg={shear_deg}");
        let after = objects(&s);
        let m = fitted_image(&otp[0], &after[0], &format!("n={n} r={r} deg={shear_deg}"));
        assert_pure_shear(
            &m,
            shear_deg.to_radians().tan(),
            &format!("n={n} deg={shear_deg}"),
        );
    }
}

// ---------------------------------------------------------------------
// Criterion 3: the conversion is the one "Object to path" gives
// ---------------------------------------------------------------------

/// Criterion 3: node count per kind (4 plain rect, 8 rounded, 4 ellipse, N polygon, 2N star), no
/// markers, the style, stacking-order slot and identity of "Object to path", still selected, and
/// the ids of nodes and anchors stay unique.
#[test]
fn ac03_node_counts_style_slot_identity() {
    let d = Document::new(1);
    let a = d.create_rect(rect_bounds(40.0, 40.0, 20.0, 12.0));
    let _b = curved_path(&d, 10, 90.0, 46.0);
    let c = round_rect(&d, 110.0, 40.0, 20.0, 12.0, 3.0);
    let e = ellipse(&d, 160.0, 46.0, 10.0, 6.0);
    let f = polygon(&d, 190.0, 46.0, 8.0, -FRAC_PI_2, 7);
    let g = star(&d, 230.0, 46.0, 8.0, -FRAC_PI_2, 6);
    // A distinct look on the primitives.
    for id in [a, c, e, f, g] {
        d.edit_style(
            &[id],
            &curvyo_document_core::StyleEdit::StrokeWidth(Length::from_mm(0.8)),
        )
        .unwrap();
        d.edit_style(&[id], &curvyo_document_core::StyleEdit::FillEnabled(true))
            .unwrap();
    }
    let before = objects(&open(&d));
    let otp = otp_objects(&d);
    let mut s = open(&d);
    select_everything(&mut s);
    assert_eq!(key(&mut s, "k", false), KeyOutcome::EntryOpened);
    assert_eq!(
        s.commit_transform_entry("25", "", 0),
        EntryOutcome::Committed
    );
    let after = objects(&s);
    assert_eq!(after.len(), 6);
    let counts = [4, 4, 8, 4, 7, 12];
    for (i, n) in after.iter().enumerate() {
        assert!(is_path(n), "object {i}");
        assert_eq!(anchors_of(n).len(), counts[i], "object {i} node count");
        assert_eq!(
            oid(n),
            oid(&otp[i]),
            "object {i}: same slot and identity as Object to path"
        );
        assert_eq!(oid(n), oid(&before[i]), "object {i}: slot kept");
        assert_eq!(
            style_debug(n),
            style_debug(&before[i]),
            "object {i}: style kept"
        );
        let ObjectSnapshot::Path(p) = n else {
            unreachable!()
        };
        assert_eq!(
            p.style.stroke.markers,
            otp_markers(&otp[i]),
            "object {i}: no markers"
        );
    }
    assert_eq!(s.selected_object_count(), 6, "nothing leaves the selection");
    assert_ids_unique(&s, "after skew conversion");
    // A second skew (now all paths) keeps the node count and the ids.
    assert_eq!(key(&mut s, "k", false), KeyOutcome::EntryOpened);
    assert_eq!(
        s.commit_transform_entry("-25", "", 0),
        EntryOutcome::Committed
    );
    let again = objects(&s);
    for (i, n) in again.iter().enumerate() {
        assert_eq!(anchors_of(n).len(), counts[i]);
    }
    assert_ids_unique(&s, "after the second skew");
}

fn otp_markers(o: &ObjectSnapshot) -> curvyo_document_core::Markers {
    match o {
        ObjectSnapshot::Path(p) => p.style.stroke.markers,
        ObjectSnapshot::Primitive(p) => p.style.stroke.markers,
        ObjectSnapshot::Clone(p) => p.resolved.style.stroke.markers,
    }
}

/// Criterion 3: with a stack of three objects, the middle one converted keeps its place
/// between the other two.
#[test]
fn ac03_stacking_slot_of_the_converted_object() {
    let d = Document::new(1);
    let a = curved_path(&d, 1, 60.0, 60.0);
    let b = polygon(&d, 100.0, 60.0, 8.0, -FRAC_PI_2, 5);
    let c = curved_path(&d, 10, 140.0, 60.0);
    let mut s = open(&d);
    // Only the polygon is selected: click inside it.
    key(&mut s, "Escape", false);
    drag(&mut s, pt(88.0, 48.0), pt(112.0, 72.0));
    assert_eq!(s.selected_object_count(), 1);
    assert_eq!(key(&mut s, "k", false), KeyOutcome::EntryOpened);
    assert_eq!(
        s.commit_transform_entry("20", "", 0),
        EntryOutcome::Committed
    );
    let ids_after: Vec<NodeId> = objects(&s).iter().map(oid).collect();
    assert_eq!(ids_after, vec![a, b, c], "the polygon kept its slot");
    assert!(is_path(&objects(&s)[1]));
    assert!(is_path(&objects(&s)[0]) && is_path(&objects(&s)[2]));
}

// ---------------------------------------------------------------------
// Criterion 4: one atomic commit
// ---------------------------------------------------------------------

/// Criterion 4: for a skew and for a stretch that convert several objects at once, the
/// document gets exactly one Loro change, with the label a path-only gesture gets, never an
/// "Object to path" label; a replica importing only the new updates sees the whole result.
#[test]
fn ac04_one_commit_for_all_objects() {
    let make = || {
        let d = Document::new(1);
        let _ = polygon(&d, 60.0, 60.0, 8.0, -FRAC_PI_2, 5);
        let _ = star(&d, 100.0, 60.0, 8.0, -FRAC_PI_2, 5);
        let _ = d.create_rect(rect_bounds(130.0, 54.0, 20.0, 12.0));
        let _ = curved_path(&d, 1, 180.0, 60.0);
        d
    };
    // Reference labels: a path-only selection.
    let path_only = || {
        let d = Document::new(1);
        let _ = curved_path(&d, 1, 60.0, 60.0);
        let _ = curved_path(&d, 10, 100.0, 60.0);
        d
    };
    let mut ps = open(&path_only());
    select_everything(&mut ps);
    key(&mut ps, "k", false);
    ps.commit_transform_entry("30", "", 0);
    let skew_label = last_label(&ps);
    let g = GBox::of(&{
        let mut t = open(&path_only());
        select_everything(&mut t);
        t
    });
    let mut ps = open(&path_only());
    select_everything(&mut ps);
    drag(
        &mut ps,
        g.corner(1.0, 1.0),
        pt(g.corner(1.0, 1.0).x + 30.0, g.corner(1.0, 1.0).y + 20.0),
    );
    let resize_label = last_label(&ps);
    let mut otp_session = open(&make());
    select_everything(&mut otp_session);
    otp_session.convert_selected_to_paths();
    let otp_label = last_label(&otp_session);
    assert_ne!(skew_label, otp_label);
    assert_ne!(resize_label, otp_label);

    // Skew by chip.
    let mut s = open(&make());
    select_everything(&mut s);
    let before_doc = doc_of(&s);
    let before_vv = loro_of(&before_doc).oplog_vv();
    let n0 = change_count(&s);
    key(&mut s, "k", false);
    assert_eq!(
        s.commit_transform_entry("30", "", 0),
        EntryOutcome::Committed
    );
    assert_eq!(change_count(&s), n0 + 1, "skew: one commit");
    assert_eq!(last_label(&s), skew_label);
    // A replica holding the old state imports only the new changes: they carry everything.
    let replica = loro_of(&before_doc);
    let after_doc = doc_of(&s);
    let updates = loro_of(&after_doc)
        .export(loro::ExportMode::updates(&before_vv))
        .unwrap();
    replica.import(&updates).unwrap();
    assert_eq!(replica.len_changes(), n0 + 1);
    // The replica, holding the old state, got exactly one change from the update: either it
    // has the old document or the one change that carries every object.
    let new_changes = replica.len_changes() - loro_of(&before_doc).len_changes();
    assert_eq!(
        new_changes, 1,
        "the update carries one change for all objects"
    );

    // Stretch by drag.
    let mut s = open(&make());
    select_everything(&mut s);
    let g = GBox::of(&s);
    let n0 = change_count(&s);
    let from = g.corner(1.0, 1.0);
    drag(&mut s, from, pt(from.x + 40.0, from.y + 25.0));
    assert_eq!(change_count(&s), n0 + 1, "stretch: one commit");
    assert_eq!(last_label(&s), resize_label);
    let after = objects(&s);
    assert!(
        is_path(&after[0]) && is_path(&after[1]),
        "polygon and star converted"
    );
    assert!(!is_path(&after[2]), "the aligned rectangle keeps its kind");
}

// ---------------------------------------------------------------------
// Criterion 5: preview, dead zone, Escape, no change
// ---------------------------------------------------------------------

/// Criterion 5: nothing is written during the drag; the live outline appears in the draw list;
/// Escape drops it and writes nothing; under the 3 px dead zone nothing is written; a drag back
/// to the start writes nothing and converts nothing.
#[test]
fn ac05_preview_and_the_cases_that_write_nothing() {
    let d = Document::new(1);
    let _ = polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 4);
    let _ = star(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 4);
    let mut s = open(&d);
    select_everything(&mut s);
    let g = GBox::of(&s);
    let h = g.skew(0.0, -1.0);
    let base = bytes_of(&s);
    let idle = s.draw_list();
    // Press, drag far, look at the preview, do not release.
    hold(&mut s, h, false, false);
    s.pointer_down(h, false);
    hold(&mut s, pt(h.x + 12.0, h.y), false, false);
    assert!(
        bytes_of(&s) == base,
        "nothing is written while the drag is in flight"
    );
    let live = s.draw_list();
    assert!(live.triangles.len() != idle.triangles.len() || live.triangles != idle.triangles);
    let max_x_live = live
        .triangles
        .iter()
        .map(|v| v.position.x)
        .fold(f64::MIN, f64::max);
    assert!(
        max_x_live > g.max.x + 1.0,
        "the sheared outline shows in the preview ({max_x_live} vs {})",
        g.max.x
    );
    // Escape drops it.
    key(&mut s, "Escape", false);
    s.pointer_up(pt(h.x + 12.0, h.y), false, false);
    assert!(bytes_of(&s) == base, "Escape: nothing written");
    assert!(
        objects(&s).iter().all(|o| !is_path(o)),
        "Escape: nothing converted"
    );
    // Dead zone: 2 px (screen) of movement.
    let two_px = 2.0 / k_of(&s);
    drag(&mut s, h, pt(h.x + two_px, h.y));
    assert!(bytes_of(&s) == base, "dead zone: nothing written");
    assert!(
        objects(&s).iter().all(|o| !is_path(o)),
        "dead zone: nothing converted"
    );
    // Away and back to the start.
    hold(&mut s, h, false, false);
    s.pointer_down(h, false);
    hold(&mut s, pt(h.x + 15.0, h.y), false, false);
    hold(&mut s, h, false, false);
    s.pointer_up(h, false, false);
    assert!(bytes_of(&s) == base, "back to the start: nothing written");
    assert!(
        objects(&s).iter().all(|o| !is_path(o)),
        "back to start: nothing converted"
    );
    // Typed 0 and an unedited Enter.
    key(&mut s, "k", false);
    let out = s.commit_transform_entry("0", "", 0);
    assert_eq!(out, EntryOutcome::Unchanged);
    assert!(bytes_of(&s) == base);
    key(&mut s, "k", false);
    let prefill = s.transform_entry().unwrap().fields[0].prefill.clone();
    assert_eq!(
        s.commit_transform_entry(&prefill, "", 0),
        EntryOutcome::Unchanged
    );
    assert!(bytes_of(&s) == base);
    // Stretch with no change: the same size.
    key(&mut s, "s", false);
    let v = s.transform_entry().unwrap();
    let (w, h2) = (v.fields[0].prefill.clone(), v.fields[1].prefill.clone());
    assert_eq!(
        s.commit_transform_entry(&w, &h2, 0),
        EntryOutcome::Unchanged
    );
    assert!(bytes_of(&s) == base);
    assert!(
        objects(&s).iter().all(|o| !is_path(o)),
        "no change converts nothing"
    );
}

// ---------------------------------------------------------------------
// Criterion 6: K, Shift+K, double-click on the handle
// ---------------------------------------------------------------------

/// Criterion 6: for every kind K, Shift+K and a double-click on a skew handle open the skew
/// chip; the typed angle equals a drag of that angle; K with nothing selected hints.
#[test]
fn ac06_keys_and_double_click_open_the_skew_chip_for_every_kind() {
    for kind in ALL_KINDS {
        let d = single(kind);
        let mut s = open(&d);
        select_everything(&mut s);
        for shift in [false, true] {
            assert_eq!(
                key(&mut s, "k", shift),
                KeyOutcome::EntryOpened,
                "{kind:?} {shift}"
            );
            assert_eq!(s.transform_entry().unwrap().kind, "skew");
            s.cancel_transform_entry();
        }
    }
    for kind in [Kind::Rect, Kind::Ellipse, Kind::Path] {
        let d = single(kind);
        let mut s = open(&d);
        select_everything(&mut s);
        let g = GBox::of(&s);
        let _ = double_click_at(&mut s, g.skew(0.0, -1.0));
        assert_eq!(s.transform_entry().expect("skew chip").kind, "skew");
    }
    // Polygon and star in a pair.
    let d = Document::new(1);
    let _ = polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 5);
    let _ = star(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 5);
    let mut s = open(&d);
    select_everything(&mut s);
    let g = GBox::of(&s);
    let _ = double_click_at(&mut s, g.skew(1.0, 0.0));
    assert_eq!(s.transform_entry().expect("skew chip").kind, "skew");
    // Nothing selected.
    let mut s = open(&d);
    assert_eq!(
        key(&mut s, "k", false),
        KeyOutcome::Hint(KeyHint::SelectFirst)
    );
}

/// Criterion 6: Enter skews by the typed angle exactly as a drag of that angle would: derive the
/// angle a drag gave, type it, compare the two results anchor by anchor.
#[test]
fn ac06_typed_angle_equals_the_drag_of_that_angle() {
    for kind in [Kind::Rect, Kind::Ellipse, Kind::Path, Kind::RoundRect] {
        let d = single(kind);
        let mut a = open(&d);
        select_everything(&mut a);
        let g = GBox::of(&a);
        let h = g.skew(0.0, -1.0);
        drag(&mut a, h, pt(h.x + 8.0, h.y));
        let da = objects(&a);
        let otp = otp_objects(&d);
        let m = fitted_image(&otp[0], &da[0], "drag");
        let tan = -m.b; // x' = x + t * (y0 - y)
        let deg = tan.atan().to_degrees();
        let mut b = open(&d);
        select_everything(&mut b);
        key(&mut b, "k", false);
        assert_eq!(
            b.commit_transform_entry(&format!("{deg:.10}"), "", 0),
            EntryOutcome::Committed
        );
        let db = objects(&b);
        for (x, y) in anchors_of(&da[0]).iter().zip(anchors_of(&db[0])) {
            assert!(
                near(x.point.x, y.point.x, 1e-5) && near(x.point.y, y.point.y, 1e-5),
                "{kind:?}: typed {deg} differs from the drag"
            );
        }
    }
}

// ---------------------------------------------------------------------
// Criteria 7 and 8: stretch of a multi-selection
// ---------------------------------------------------------------------

/// Criterion 7, the spec's example verbatim (moved by (100, 100)): a closed 10 x 10 path at
/// (0, 0) and a four-point polygon with vertices (25, 0), (30, 5), (25, 10), (20, 5); dragging
/// the bottom-right corner from (30, 10) to (60, 30) gives sx = 2, sy = 3 about (0, 0).
#[test]
fn ac07_spec_example_path_and_polygon() {
    let d = Document::new(1);
    let off = 100.0;
    let p = d.create_path(
        &[
            anchor(1, off, off),
            anchor(2, off + 10.0, off),
            anchor(3, off + 10.0, off + 10.0),
            anchor(4, off, off + 10.0),
        ],
        true,
    );
    let poly = polygon(&d, off + 25.0, off + 5.0, 5.0, -FRAC_PI_2, 4);
    // "Object to path" is the reference for the vertices of the polygon.
    let otp = otp_objects(&d);
    let verts: Vec<Point> = anchors_of(&otp[1]).iter().map(|a| a.point).collect();
    for (x, y) in [(25.0, 0.0), (30.0, 5.0), (25.0, 10.0), (20.0, 5.0)] {
        assert!(
            verts
                .iter()
                .any(|v| near(v.x, off + x, 1e-7) && near(v.y, off + y, 1e-7)),
            "Object to path of the polygon lacks ({x}, {y}): {verts:?}"
        );
    }
    let mut s = open(&d);
    select_everything(&mut s);
    let g = GBox::of(&s);
    assert!(near(g.w(), 30.0, 1e-7) && near(g.h(), 10.0, 1e-7));
    let from = g.corner(1.0, 1.0);
    let c = change_count(&s);
    drag(&mut s, from, pt(off + 60.0, off + 30.0));
    assert_eq!(change_count(&s), c + 1);
    let after = objects(&s);
    assert_eq!(oid(&after[0]), p);
    assert_eq!(oid(&after[1]), poly);
    let want_path = [(0.0, 0.0), (20.0, 0.0), (20.0, 30.0), (0.0, 30.0)];
    let pa = anchors_of(&after[0]);
    for (x, y) in want_path {
        assert!(
            pa.iter()
                .any(|a| near(a.point.x, off + x, 1e-6) && near(a.point.y, off + y, 1e-6)),
            "path corner ({x}, {y})"
        );
    }
    assert!(is_path(&after[1]), "the polygon became a path");
    let qa = anchors_of(&after[1]);
    assert_eq!(qa.len(), 4);
    for (x, y) in [(50.0, 0.0), (60.0, 15.0), (50.0, 30.0), (40.0, 15.0)] {
        assert!(
            qa.iter()
                .any(|a| near(a.point.x, off + x, 1e-6) && near(a.point.y, off + y, 1e-6)),
            "polygon vertex ({x}, {y}) in {:?}",
            qa.iter().map(|a| a.point).collect::<Vec<_>>()
        );
    }
    for a in &qa {
        assert_eq!(a.kind, AnchorKind::Corner);
    }
}

/// Criterion 7: the same selection with an aligned rectangle added: the rectangle is still a
/// rectangle with sx and sy applied to its width and height; handle vectors of the polygon scale.
#[test]
fn ac07_example_with_an_aligned_rectangle_added() {
    let d = Document::new(1);
    let off = 100.0;
    let _ = d.create_path(
        &[
            anchor(1, off, off),
            anchor(2, off + 10.0, off),
            anchor(3, off + 10.0, off + 10.0),
            anchor(4, off, off + 10.0),
        ],
        true,
    );
    let _ = polygon(&d, off + 25.0, off + 5.0, 5.0, -FRAC_PI_2, 4);
    let r = d.create_rect(rect_bounds(off + 2.0, off + 2.0, 4.0, 3.0));
    let mut s = open(&d);
    select_everything(&mut s);
    let g = GBox::of(&s);
    let from = g.corner(1.0, 1.0);
    drag(&mut s, from, pt(off + 60.0, off + 30.0));
    let after = objects(&s);
    let rect = after.iter().find(|o| oid(o) == r).unwrap();
    let ObjectSnapshot::Primitive(p) = rect else {
        panic!("the aligned rectangle must stay a rectangle");
    };
    let Shape::Rect { bounds, .. } = p.shape else {
        panic!("rect expected")
    };
    assert!(near(bounds.width.as_mm(), 8.0, 1e-6), "width 4 * 2");
    assert!(near(bounds.height.as_mm(), 9.0, 1e-6), "height 3 * 3");
    assert!(near(bounds.origin.x, off + 4.0, 1e-6) && near(bounds.origin.y, off + 6.0, 1e-6));
}

/// Criterion 7: the whole kind matrix. A path plus one object of each kind; a free corner
/// drag; polygons, stars and misaligned rectangles/ellipses convert and are the exact scale
/// image of their "Object to path" form; paths, aligned rectangles and ellipses and circles keep
/// their kind and their outline is the mapped outline.
#[test]
fn ac07_kind_matrix_free_corner_drag() {
    for kind in ALL_KINDS {
        let d = Document::new(1);
        let _ = curved_path(&d, 1, 60.0, 60.0);
        let id = add(&d, kind, 20, 110.0, 66.0);
        let otp = otp_objects(&d);
        let before = objects(&open(&d));
        let mut s = open(&d);
        select_everything(&mut s);
        let g = GBox::of(&s);
        let anchor_pt = g.corner(-1.0, -1.0);
        let from = g.corner(1.0, 1.0);
        let to = pt(from.x + 0.5 * g.w(), from.y + 0.25 * g.h());
        let (sx, sy) = ((to.x - anchor_pt.x) / g.w(), (to.y - anchor_pt.y) / g.h());
        let c = change_count(&s);
        drag(&mut s, from, to);
        assert_eq!(change_count(&s), c + 1, "{kind:?}: one commit");
        let after = objects(&s);
        let map = Aff::scale(anchor_pt.x, anchor_pt.y, sx, sy);
        // The path (object 0) is always a path, mapped.
        let m0 = fitted_image(&otp[0], &after[0], "path");
        assert!(
            // The box is measured from sampled curves, so the factors agree to 1e-4.
            near(m0.a, sx, 1e-4) && near(m0.d, sy, 1e-4) && m0.b.abs() < 1e-6 && m0.c.abs() < 1e-6,
            "{kind:?}: path map {m0:?}, want scale ({sx}, {sy})"
        );
        let n = after.iter().find(|o| oid(o) == id).unwrap();
        if stays_primitive_in_stretch(kind) {
            assert!(!is_path(n), "{kind:?} keeps its kind in a stretch");
            // Outline equals the mapped outline.
            let old = before.iter().find(|o| oid(o) == id).unwrap();
            let want: Vec<Point> = polyline(old).iter().map(|p| map.p(*p)).collect();
            if kind == Kind::RoundRect {
                // The corner radius follows sqrt(sx * sy) (0005 criterion 9, question 15): an
                // approximation the spec names; the box itself is exact.
                let (lo, hi) = tight_bounds(std::slice::from_ref(n));
                let (wlo, whi) = (
                    map.p(tight_bounds(std::slice::from_ref(old)).0),
                    map.p(tight_bounds(std::slice::from_ref(old)).1),
                );
                assert!(
                    near(lo.x, wlo.x, 1e-6)
                        && near(lo.y, wlo.y, 1e-6)
                        && near(hi.x, whi.x, 1e-6)
                        && near(hi.y, whi.y, 1e-6)
                );
            }
            for p in polyline(n).into_iter().filter(|_| kind != Kind::RoundRect) {
                assert!(
                    dist_to_polyline(p, &want, true) < 0.02,
                    "{kind:?}: outline sample ({}, {}) off the mapped outline",
                    p.x,
                    p.y
                );
            }
        } else {
            assert!(is_path(n), "{kind:?} converts in a multi-selection stretch");
            let o = otp.iter().find(|o| oid(o) == id).unwrap();
            let m = fitted_image(o, n, &format!("{kind:?}"));
            assert!(
                near(m.a, sx, 1e-4) && near(m.d, sy, 1e-4) && m.b.abs() < 1e-6 && m.c.abs() < 1e-6,
                "{kind:?}: scale map {m:?} vs ({sx}, {sy})"
            );
            assert!(
                near(m.p(anchor_pt).x, anchor_pt.x, 1e-3)
                    && near(m.p(anchor_pt).y, anchor_pt.y, 1e-3),
                "{kind:?}: the opposite corner stays fixed"
            );
        }
        assert_ids_unique(&s, &format!("{kind:?}"));
        assert_eq!(s.selected_object_count(), 2, "{kind:?}: selection stays");
    }
}

/// Criterion 7: edge handle stretches (E and S) convert the same set.
#[test]
fn ac07_edge_handle_stretch_converts_polygon_and_star() {
    let d = Document::new(1);
    let _ = polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 5);
    let _ = star(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 5);
    let _ = d.create_rect(rect_bounds(160.0, 94.0, 10.0, 12.0));
    let otp = otp_objects(&d);
    let mut s = open(&d);
    select_everything(&mut s);
    let g = GBox::of(&s);
    let from = g.mid(1.0, 0.0);
    drag(&mut s, from, pt(from.x + g.w(), from.y));
    let after = objects(&s);
    assert!(is_path(&after[0]) && is_path(&after[1]) && !is_path(&after[2]));
    let m = fitted_image(&otp[0], &after[0], "polygon");
    assert!(near(m.a, 2.0, 1e-6) && near(m.d, 1.0, 1e-6));
    let m = fitted_image(&otp[1], &after[1], "star");
    assert!(near(m.a, 2.0, 1e-6) && near(m.d, 1.0, 1e-6));
}

/// Criterion 8: factors clamp at 0; a result beyond +-1e7 mm is no change at all: nothing is
/// converted, nothing is moved; stroke width follows "Scale stroke width" for a converted object
/// like a path's does.
#[test]
fn ac08_clamp_and_limit() {
    let make = || {
        let d = Document::new(1);
        let _ = d.create_path(
            &[
                anchor(1, 100.0, 100.0),
                anchor(2, 110.0, 100.0),
                anchor(3, 110.0, 110.0),
                anchor(4, 100.0, 110.0),
            ],
            true,
        );
        let _ = polygon(&d, 125.0, 105.0, 5.0, -FRAC_PI_2, 4);
        d
    };
    // Past the opposite edge (`0056` criterion 6 supersedes the clamp): the selection is
    // turned over onto the left of the fixed edge, 50 mm wide; the stretch converts.
    let mut s = open(&make());
    select_everything(&mut s);
    let g = GBox::of(&s);
    let from = g.mid(1.0, 0.0);
    drag(&mut s, from, pt(g.min.x - 50.0, from.y));
    let after = objects(&s);
    let mut leftmost = f64::INFINITY;
    for o in &after {
        assert!(is_path(o), "the polygon converted by the stretch");
        for a in anchors_of(o) {
            assert!(
                a.point.x <= g.min.x + 1e-6,
                "turned over: x = {}",
                a.point.x
            );
            leftmost = leftmost.min(a.point.x);
        }
    }
    assert!(near(leftmost, g.min.x - 50.0, 1e-6), "{leftmost}");
    // Beyond 1e7 mm by the typed size: no change at all.
    let mut s = open(&make());
    select_everything(&mut s);
    let base = bytes_of(&s);
    key(&mut s, "s", false);
    let out = s.commit_transform_entry("30000000", "10", 0);
    assert_ne!(out, EntryOutcome::Committed, "beyond 1e7 mm is no change");
    assert!(bytes_of(&s) == base, "nothing written");
    let os = objects(&s);
    assert!(!is_path(&os[1]), "nothing converted");
}

// ---------------------------------------------------------------------
// Criterion 9: typed size
// ---------------------------------------------------------------------

/// Criterion 9: the typed size of a multi-selection and of a lone polygon or star has independent
/// fields; Enter converts and stretches; a uniform size converts nothing; the same size writes
/// nothing.
#[test]
fn ac09_typed_size_is_independent_and_converts() {
    // multi
    let d = Document::new(1);
    let _ = polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 4);
    let _ = star(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 4);
    let mut s = open(&d);
    select_everything(&mut s);
    key(&mut s, "s", false);
    let v = s.transform_entry().unwrap();
    assert_eq!(v.kind, "size");
    assert!(!v.linked, "fields independent in a multi-selection");
    let (w, h) = (
        v.fields[0].prefill.parse::<f64>().unwrap(),
        v.fields[1].prefill.parse::<f64>().unwrap(),
    );
    // uniform: same aspect
    assert_eq!(
        s.commit_transform_entry(&format!("{}", w * 2.0), &format!("{}", h * 2.0), 0),
        EntryOutcome::Committed
    );
    assert!(
        objects(&s).iter().all(|o| !is_path(o)),
        "a uniform size converts nothing"
    );
    // stretch: independent
    select_everything(&mut s);
    key(&mut s, "s", false);
    let v = s.transform_entry().unwrap();
    let (w, h) = (
        v.fields[0].prefill.parse::<f64>().unwrap(),
        v.fields[1].prefill.parse::<f64>().unwrap(),
    );
    assert_eq!(
        s.commit_transform_entry(&format!("{}", w * 1.5), &format!("{h}"), 0),
        EntryOutcome::Committed
    );
    assert!(objects(&s).iter().all(is_path), "a stretch converts");
    // lone polygon and lone star
    for lone in 0..2 {
        let d = Document::new(1);
        let _ = if lone == 0 {
            polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 4)
        } else {
            star(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 4)
        };
        let mut s = open(&d);
        select_everything(&mut s);
        key(&mut s, "s", false);
        let v = s.transform_entry().unwrap();
        assert!(!v.linked, "lone polygon/star: fields independent");
        let (w, h) = (
            v.fields[0].prefill.parse::<f64>().unwrap(),
            v.fields[1].prefill.parse::<f64>().unwrap(),
        );
        assert_eq!(
            s.commit_transform_entry(&format!("{}", w * 3.0), &format!("{h}"), 0),
            EntryOutcome::Committed
        );
        let o = &objects(&s)[0];
        assert!(is_path(o), "lone {lone}: converted");
        let (min, max) = tight_bounds(std::slice::from_ref(o));
        // The box of a polygon or star is turned by its own angle (here a quarter turn), so
        // the typed width lies along a document axis: compare the sorted extents.
        let mut got = [max.x - min.x, max.y - min.y];
        got.sort_by(f64::total_cmp);
        let mut want = [w * 3.0, h];
        want.sort_by(f64::total_cmp);
        assert!(
            near(got[0], want[0], 1e-4) && near(got[1], want[1], 1e-4),
            "extents {got:?} vs {want:?}"
        );
    }
}

// ---------------------------------------------------------------------
// Criterion 10: nothing else converts
// ---------------------------------------------------------------------

fn all_primitives(s: &Session) -> bool {
    objects(s).iter().all(|o| !is_path(o))
}

/// Criterion 10: Ctrl+corner (proportions), rotate, move, copy, typed move, uniform size: no
/// object is converted; a polygon scaled uniformly keeps its kind with radius times the factor.
#[test]
fn ac10_uniform_scale_rotate_move_copy_never_convert() {
    let make = || {
        let d = Document::new(1);
        let _ = polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 5);
        let _ = star(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 5);
        let _ = d.create_rect(rect_bounds(160.0, 94.0, 10.0, 12.0));
        let _ = add(&d, Kind::RotRect30, 1, 200.0, 100.0);
        d
    };
    // Uniform scale with Ctrl on a corner.
    let mut s = open(&make());
    select_everything(&mut s);
    let g = GBox::of(&s);
    let from = g.corner(1.0, 1.0);
    drag_mod(
        &mut s,
        from,
        pt(from.x + g.w() * 0.5, from.y + g.h() * 0.5),
        false,
        true,
    );
    assert!(all_primitives(&s), "Ctrl-corner scale keeps every kind");
    let ObjectSnapshot::Primitive(p) = &objects(&s)[0] else {
        panic!()
    };
    let Shape::Polygon { frame, .. } = p.shape else {
        panic!("polygon")
    };
    assert!(
        near(frame.radius.as_mm(), 12.0, 1e-4),
        "outer radius 8 * 1.5, got {}",
        frame.radius.as_mm()
    );
    // Typed uniform size, on a scene whose box sizes are exact decimals (the typed text is
    // all the maker has).
    let exact = || {
        let d = Document::new(1);
        let _ = polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 4);
        let _ = star(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 4);
        let _ = d.create_rect(rect_bounds(160.0, 94.0, 10.0, 12.0));
        let _ = add(&d, Kind::RotRect90, 1, 200.0, 100.0);
        d
    };
    let mut s = open(&exact());
    select_everything(&mut s);
    key(&mut s, "s", false);
    let v = s.transform_entry().unwrap();
    let (w, h) = (
        v.fields[0].prefill.parse::<f64>().unwrap(),
        v.fields[1].prefill.parse::<f64>().unwrap(),
    );
    assert_eq!(
        s.commit_transform_entry(&format!("{}", w * 2.0), &format!("{}", h * 2.0), 0),
        EntryOutcome::Committed
    );
    assert!(all_primitives(&s));
    // Move drag and Ctrl-copy drag.
    let mut s = open(&make());
    select_everything(&mut s);
    let g = GBox::of(&s);
    drag(
        &mut s,
        g.centre(),
        pt(g.centre().x + 20.0, g.centre().y + 5.0),
    );
    assert!(all_primitives(&s));
    let n = objects(&s).len();
    let g = GBox::of(&s);
    drag_mod(
        &mut s,
        g.centre(),
        pt(g.centre().x - 30.0, g.centre().y),
        false,
        true,
    );
    assert!(all_primitives(&s), "Ctrl copy converts nothing");
    assert_eq!(objects(&s).len(), n * 2);
    // Typed move.
    let mut s = open(&make());
    select_everything(&mut s);
    assert_eq!(key(&mut s, "m", false), KeyOutcome::EntryOpened);
    assert_eq!(
        s.commit_move_entry(
            "12",
            "7",
            curvyo_ui_core::MoveEntryMode {
                absolute: false,
                copy: false
            }
        ),
        EntryOutcome::Committed
    );
    assert!(all_primitives(&s));
    // Rotate by a typed angle (R).
    let mut s = open(&make());
    select_everything(&mut s);
    assert_eq!(key(&mut s, "r", false), KeyOutcome::EntryOpened);
    assert_eq!(
        s.commit_transform_entry("40", "", 0),
        EntryOutcome::Committed
    );
    assert!(all_primitives(&s), "rotate converts nothing");
    // Rotating by 40 and then stretching converts only at the stretch.
    select_everything(&mut s);
    let g = GBox::of(&s);
    let from = g.mid(1.0, 0.0);
    drag(&mut s, from, pt(from.x + 30.0, from.y));
    let os = objects(&s);
    assert!(
        is_path(&os[0]) && is_path(&os[1]),
        "the stretch converts the polygon and the star"
    );
}

// ---------------------------------------------------------------------
// Criterion 11: no text
// ---------------------------------------------------------------------

fn all_texts(s: &mut Session, at: Point) -> Vec<String> {
    hold(s, at, false, false);
    let mut v = vec![s.cursor_hint(), s.handle_hint(), s.selection_announcement()];
    v.extend(s.corner_hint_lines());
    v
}

fn assert_no_conversion_words(texts: &[String], what: &str) {
    for t in texts {
        let l = t.to_lowercase();
        for w in [
            "object to path",
            "convert",
            "paths only",
            "proportional only",
            "holds a",
            "to stretch",
            "became a path",
        ] {
            assert!(!l.contains(w), "{what}: text '{t}' mentions '{w}'");
        }
    }
}

/// Criterion 11: the texts a session offers before, during and after a skew or stretch of
/// primitives never mention a conversion; and the banned strings are gone from the code base.
#[test]
fn ac11_no_text_mentions_a_conversion() {
    let d = Document::new(1);
    let _ = polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 5);
    let _ = star(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 5);
    let _ = d.create_rect(rect_bounds(160.0, 94.0, 10.0, 12.0));
    let mut s = open(&d);
    select_everything(&mut s);
    let g = GBox::of(&s);
    for p in [
        g.corner(1.0, 1.0),
        g.corner(-1.0, -1.0),
        g.mid(1.0, 0.0),
        g.mid(0.0, 1.0),
        g.skew(0.0, -1.0),
        g.skew(1.0, 0.0),
        g.centre(),
    ] {
        assert_no_conversion_words(&all_texts(&mut s, p), "hover");
    }
    // During a corner drag and a skew drag.
    let from = g.corner(1.0, 1.0);
    hold(&mut s, from, false, false);
    s.pointer_down(from, false);
    hold(&mut s, pt(from.x + 20.0, from.y + 10.0), false, false);
    assert_no_conversion_words(
        &all_texts(&mut s, pt(from.x + 20.0, from.y + 10.0)),
        "stretch drag",
    );
    s.pointer_up(pt(from.x + 20.0, from.y + 10.0), false, false);
    assert_no_conversion_words(
        &all_texts(&mut s, pt(from.x + 20.0, from.y + 10.0)),
        "after stretch",
    );
    // The skew chip and its key outcome.
    key(&mut s, "Escape", false);
    select_everything(&mut s);
    assert_eq!(
        key(&mut s, "k", false),
        KeyOutcome::EntryOpened,
        "K never hints"
    );
    s.cancel_transform_entry();
    key(&mut s, "s", false);
    let v = s.transform_entry().unwrap();
    assert!(!format!("{v:?}").to_lowercase().contains("path"));
}

/// Criterion 11: the removed strings are not in any source of the product (Rust or frontend).
#[test]
fn ac11_banned_strings_are_gone_from_the_code_base() {
    use std::path::Path;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let banned = [
        "Skew works on paths only",
        "proportional only",
        "Holds a",
        "To stretch: Object to path first",
    ];
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if p.is_dir() {
                if matches!(
                    name.as_str(),
                    "target" | "node_modules" | ".git" | "tests" | "specs" | "docs"
                ) {
                    continue;
                }
                walk(&p, out);
            } else if matches!(
                p.extension().and_then(|x| x.to_str()),
                Some("rs" | "ts" | "tsx")
            ) {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    for c in [
        "curvyo-ui-core",
        "curvyo-editor-wasm",
        "curvyo-render-core",
        "curvyo-document-core",
        "frontend",
    ] {
        walk(&root.join(c), &mut files);
    }
    assert!(files.len() > 50, "the walk found the sources");
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        for b in banned {
            assert!(!text.contains(b), "{} still contains {b:?}", f.display());
        }
    }
}

// ---------------------------------------------------------------------
// Criterion 12: a lone polygon or star
// ---------------------------------------------------------------------

/// Criterion 12, the spec's example (moved by (100, 100)): an unrotated four-point polygon with
/// vertices (25, 0), (30, 5), (25, 10), (20, 5): the right edge handle from x = 30 to x = 40 gives
/// a path with corner nodes at (30, 0), (40, 5), (30, 10), (20, 5).
#[test]
fn ac12_lone_polygon_right_edge_example() {
    let d = Document::new(1);
    let off = 100.0;
    let id = polygon(&d, off + 25.0, off + 5.0, 5.0, -FRAC_PI_2, 4);
    let otp = otp_objects(&d);
    let verts: Vec<Point> = anchors_of(&otp[0]).iter().map(|a| a.point).collect();
    for (x, y) in [(25.0, 0.0), (30.0, 5.0), (25.0, 10.0), (20.0, 5.0)] {
        assert!(
            verts
                .iter()
                .any(|v| near(v.x, off + x, 1e-7) && near(v.y, off + y, 1e-7))
        );
    }
    let mut s = open(&d);
    select_everything(&mut s);
    let g = GBox::of(&s);
    // The edge handle exists on every side of the lone polygon.
    for (nx, ny) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        hold(&mut s, g.mid(nx, ny), false, false);
        assert!(
            s.cursor_hint().starts_with("resize:"),
            "edge handle ({nx}, {ny})"
        );
    }
    let from = g.mid(1.0, 0.0);
    let c = change_count(&s);
    drag(&mut s, from, pt(off + 40.0, from.y));
    assert_eq!(change_count(&s), c + 1);
    let o = &objects(&s)[0];
    assert_eq!(oid(o), id);
    assert!(is_path(o), "stretch converted the lone polygon");
    let a = anchors_of(o);
    assert_eq!(a.len(), 4);
    for (x, y) in [(30.0, 0.0), (40.0, 5.0), (30.0, 10.0), (20.0, 5.0)] {
        assert!(
            a.iter()
                .any(|q| near(q.point.x, off + x, 1e-6) && near(q.point.y, off + y, 1e-6)),
            "vertex ({x}, {y}) in {:?}",
            a.iter().map(|q| q.point).collect::<Vec<_>>()
        );
    }
    assert_eq!(s.selected_object_count(), 1);
}

/// Criterion 12: a free corner drag stretches, Ctrl keeps the proportions and the kind, Shift
/// scales about the centre; a lone star behaves the same; a lone rectangle and ellipse are never
/// converted by any resize.
#[test]
fn ac12_corner_ctrl_shift_and_rect_ellipse_never_convert() {
    for is_star in [false, true] {
        let mk = || {
            let d = Document::new(1);
            let _ = if is_star {
                star(&d, 120.0, 100.0, 8.0, -FRAC_PI_2, 4)
            } else {
                polygon(&d, 120.0, 100.0, 8.0, -FRAC_PI_2, 4)
            };
            d
        };
        // Free corner drag.
        let mut s = open(&mk());
        select_everything(&mut s);
        let g = GBox::of(&s);
        let from = g.corner(1.0, 1.0);
        drag(&mut s, from, pt(from.x + 10.0, from.y + 4.0));
        assert!(
            is_path(&objects(&s)[0]),
            "free corner drag converts (star {is_star})"
        );
        let (min, max) = tight_bounds(&objects(&s));
        assert!(near(max.x - min.x, g.w() + 10.0, 1e-4) && near(max.y - min.y, g.h() + 4.0, 1e-4));
        assert!(near(min.x, g.min.x, 1e-4), "opposite corner fixed");
        // Ctrl corner: proportional, still a primitive.
        let mut s = open(&mk());
        select_everything(&mut s);
        let from = g.corner(1.0, 1.0);
        drag_mod(
            &mut s,
            from,
            pt(from.x + g.w(), from.y + g.h()),
            false,
            true,
        );
        let o = &objects(&s)[0];
        assert!(
            !is_path(o),
            "Ctrl keeps the proportions and the kind (star {is_star})"
        );
        // Shift: about the centre.
        let mut s = open(&mk());
        select_everything(&mut s);
        let from = g.mid(1.0, 0.0);
        drag_mod(&mut s, from, pt(from.x + 6.0, from.y), true, false);
        let (min, max) = tight_bounds(&objects(&s));
        assert!(is_path(&objects(&s)[0]));
        assert!(
            near((min.x + max.x) / 2.0, g.centre().x, 1e-4),
            "Shift: centre fixed"
        );
        assert!(near(max.x - min.x, g.w() + 12.0, 1e-4));
    }
    // Lone rect / ellipse: never converted, whichever handle.
    for kind in [
        Kind::Rect,
        Kind::RoundRect,
        Kind::Ellipse,
        Kind::RotRect30,
        Kind::RotEllipse25,
        Kind::Circle33,
    ] {
        let d = single(kind);
        let mut s = open(&d);
        select_everything(&mut s);
        key(&mut s, "s", false);
        let v = s.transform_entry().unwrap();
        let (w, h) = (
            v.fields[0].prefill.parse::<f64>().unwrap(),
            v.fields[1].prefill.parse::<f64>().unwrap(),
        );
        let _ = s.commit_transform_entry(&format!("{}", w * 1.7), &format!("{}", h * 0.6), 0);
        assert!(
            !is_path(&objects(&s)[0]),
            "{kind:?}: a lone resize never converts"
        );
    }
    for kind in [Kind::Rect, Kind::Ellipse, Kind::RoundRect] {
        let d = single(kind);
        let mut s = open(&d);
        select_everything(&mut s);
        let g = GBox::of(&s);
        let from = g.corner(1.0, 1.0);
        drag(&mut s, from, pt(from.x + 7.0, from.y + 2.0));
        assert!(
            !is_path(&objects(&s)[0]),
            "{kind:?}: corner drag keeps the kind"
        );
        let from = g.mid(1.0, 0.0);
        drag(&mut s, pt(from.x + 7.0, from.y), pt(from.x + 20.0, from.y));
        assert!(
            !is_path(&objects(&s)[0]),
            "{kind:?}: edge drag keeps the kind"
        );
    }
}

/// Criterion 12: a rotated lone polygon stretches along its own box axes (opposite edge fixed),
/// then is a path whose geometry is the image of its "Object to path" form under an axis scale
/// in the box frame.
#[test]
fn ac12_rotated_lone_polygon_uses_box_axes() {
    let d = Document::new(1);
    let id = polygon(&d, 120.0, 100.0, 8.0, -FRAC_PI_2, 4);
    rotate_about_centre(&d, id, 30.0_f64.to_radians());
    let otp = otp_objects(&d);
    let mut s = open(&d);
    select_everything(&mut s);
    // Typed size is in the box's own axes and independent.
    key(&mut s, "s", false);
    let v = s.transform_entry().unwrap();
    let (w, h) = (
        v.fields[0].prefill.parse::<f64>().unwrap(),
        v.fields[1].prefill.parse::<f64>().unwrap(),
    );
    assert_eq!(
        s.commit_transform_entry(&format!("{}", w * 2.0), &format!("{h}"), 0),
        EntryOutcome::Committed
    );
    let o = &objects(&s)[0];
    assert!(is_path(o));
    let m = fitted_image(&otp[0], o, "rotated polygon");
    // Pure axis scale in the rotated frame: det = 2 and the linear part has singular values 2, 1.
    assert!(near(m.det(), 2.0, 1e-6), "det {}", m.det());
    let (a, b, c, d2) = (m.a, m.b, m.c, m.d);
    let frob2 = a * a + b * b + c * c + d2 * d2;
    assert!(
        near(frob2, 5.0, 1e-6),
        "singular values 2 and 1, got |A|^2 = {frob2}"
    );
    // Along one of the box axes (30 degrees, or 120 when the polygon's own quarter turn
    // makes the box width the vertical one).
    let stretch_on = |deg: f64| {
        let (cs, sn) = (deg.to_radians().cos(), deg.to_radians().sin());
        let (ux, uy) = (m.a * cs + m.b * sn, m.c * cs + m.d * sn);
        ux.hypot(uy)
    };
    assert!(
        near(stretch_on(30.0), 2.0, 1e-6) || near(stretch_on(120.0), 2.0, 1e-6),
        "the stretch lies along a box axis, got {} / {}",
        stretch_on(30.0),
        stretch_on(120.0)
    );
    assert!(
        near(stretch_on(30.0), 1.0, 1e-6) || near(stretch_on(120.0), 1.0, 1e-6),
        "the other box axis is unchanged"
    );
}

// ---------------------------------------------------------------------
// Criterion 13: Object to path unchanged
// ---------------------------------------------------------------------

/// Criterion 13: the explicit command still converts exactly the primitives.
#[test]
fn ac13_object_to_path_still_converts() {
    for kind in ALL_KINDS {
        let d = single(kind);
        let mut s = open(&d);
        select_everything(&mut s);
        let base = change_count(&s);
        s.convert_selected_to_paths();
        let os = objects(&s);
        if kind == Kind::Path {
            assert!(is_path(&os[0]));
        } else {
            assert!(is_path(&os[0]), "{kind:?} converted by the command");
            assert_eq!(change_count(&s), base + 1);
        }
    }
}

// ---------------------------------------------------------------------
// Criterion 14: save and reopen
// ---------------------------------------------------------------------

/// Criterion 14: saved and reopened, converted objects are paths with the same anchors, handles,
/// rotation, style and stacking position; unconverted objects are exactly as before; no new field
/// and `format_version` stays 10 (its value on the base of this branch).
#[test]
fn ac14_save_close_reopen() {
    let d = Document::new(1);
    let a = d.create_rect(rect_bounds(40.0, 40.0, 20.0, 12.0));
    let b = polygon(&d, 100.0, 46.0, 8.0, -FRAC_PI_2, 5);
    let c = curved_path(&d, 1, 140.0, 46.0);
    let e = add(&d, Kind::RotRect90, 1, 180.0, 46.0);
    let mut s = open(&d);
    select_everything(&mut s);
    // A stretch: the polygon converts; the rectangles and the path keep their kinds.
    let g = GBox::of(&s);
    let from = g.corner(1.0, 1.0);
    drag(&mut s, from, pt(from.x + 30.0, from.y + 10.0));
    let bytes = s.pack("0.1.0").unwrap();
    let live = objects(&s);
    let reopened = open_from(&bytes);
    let again = objects(&reopened);
    assert_eq!(live, again, "save, close and reopen change nothing");
    let ids: Vec<NodeId> = again.iter().map(oid).collect();
    assert_eq!(ids, vec![a, b, c, e]);
    assert!(!is_path(&again[0]) && is_path(&again[1]) && is_path(&again[2]) && !is_path(&again[3]));
    assert_eq!(
        curvyo_document_core::CURRENT_FORMAT_VERSION,
        15,
        "a path is an existing kind: the conversion adds no bump of its own (0023 took 11, 0050 took 12, 0049 took 13, 0057 took 14, 0046 took 15)"
    );
    // A skew conversion as well.
    let mut s = open(&d);
    select_everything(&mut s);
    key(&mut s, "k", false);
    s.commit_transform_entry("15", "", 0);
    let live = objects(&s);
    let again = objects(&open_from(&s.pack("0.1.0").unwrap()));
    assert_eq!(live, again);
}

fn open_from(bytes: &[u8]) -> Session {
    let mut s = Session::open(5, bytes).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

// ---------------------------------------------------------------------
// Compound paths, ids, mixed documents
// ---------------------------------------------------------------------

fn compound_scene() -> Session {
    let d = Document::new(1);
    let _ = d.create_rect(rect_bounds(50.0, 50.0, 40.0, 40.0));
    let _ = d.create_rect(rect_bounds(60.0, 60.0, 10.0, 10.0));
    let mut s = open(&d);
    select_everything(&mut s);
    let out = s.apply_boolean(curvyo_ui_core::BooleanOp::Difference);
    assert!(
        matches!(
            out,
            curvyo_editor_wasm::BooleanOutcome::Applied { compound: true, .. }
        ),
        "{out:?}"
    );
    let d = doc_of(&s);
    let _ = polygon(&d, 140.0, 70.0, 10.0, -FRAC_PI_2, 6);
    let _ = star(&d, 190.0, 70.0, 10.0, -FRAC_PI_2, 5);
    // A new peer id: a second session with the same id would mint the anchor ids of the
    // compound path again.
    let mut s = Session::open(7, &pack(&d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

/// A compound path in the selection with polygon and star: skew and stretch map every outline of
/// the compound with the same map as the converted shapes; one commit; ids unique.
#[test]
fn compound_path_with_primitives_skew_and_stretch() {
    let mut s = compound_scene();
    select_everything(&mut s);
    let otp = {
        let mut t = compound_scene();
        select_everything(&mut t);
        t.convert_selected_to_paths();
        objects(&t)
    };
    let before = objects(&s);
    let c = change_count(&s);
    key(&mut s, "k", false);
    assert_eq!(
        s.commit_transform_entry("25", "", 0),
        EntryOutcome::Committed
    );
    assert_eq!(change_count(&s), c + 1);
    let after = objects(&s);
    assert_eq!(after.len(), before.len());
    for (i, (o, n)) in otp.iter().zip(&after).enumerate() {
        assert!(is_path(n), "object {i}");
        let m = fitted_image(o, n, &format!("compound scene {i}"));
        assert_pure_shear(&m, 25.0_f64.to_radians().tan(), &format!("compound {i}"));
    }
    assert!(
        anchors_of(&after[0]).len() >= 8,
        "the compound keeps both outlines"
    );
    assert_ids_unique(&s, "compound skew");

    // Stretch.
    let mut s = compound_scene();
    select_everything(&mut s);
    let g = GBox::of(&s);
    let c = change_count(&s);
    let from = g.corner(1.0, 1.0);
    drag(&mut s, from, pt(from.x + 20.0, from.y + 14.0));
    assert_eq!(change_count(&s), c + 1);
    let after = objects(&s);
    for (i, (o, n)) in otp.iter().zip(&after).enumerate() {
        assert!(is_path(n));
        let m = fitted_image(o, n, &format!("compound stretch {i}"));
        assert!(m.b.abs() < 1e-6 && m.c.abs() < 1e-6 && m.a > 1.0 && m.d > 1.0);
    }
    assert_ids_unique(&s, "compound stretch");
}

/// Ids stay unique over repeated conversions and across undo-less sessions: skew, then stretch,
/// then skew again on a big mixed scene; and the selection count never changes.
#[test]
fn repeated_gestures_keep_ids_unique() {
    let d = Document::new(1);
    for (i, kind) in ALL_KINDS.iter().enumerate() {
        let _ = add(
            &d,
            *kind,
            100 * (i as u64 + 1),
            60.0 + (i % 5) as f64 * 45.0,
            60.0 + (i / 5) as f64 * 45.0,
        );
    }
    let mut s = open(&d);
    select_everything(&mut s);
    let n = s.selected_object_count();
    key(&mut s, "k", false);
    s.commit_transform_entry("20", "", 0);
    assert_ids_unique(&s, "1");
    let g = GBox::of(&s);
    let from = g.corner(1.0, 1.0);
    drag(&mut s, from, pt(from.x + 25.0, from.y + 5.0));
    assert_ids_unique(&s, "2");
    key(&mut s, "k", true);
    s.commit_transform_entry("-15", "", 0);
    assert_ids_unique(&s, "3");
    assert_eq!(s.selected_object_count(), n);
    assert!(objects(&s).iter().all(|o| {
        let a = anchors_of(o);
        a.iter()
            .all(|x| x.point.x.is_finite() && x.point.y.is_finite())
    }));
}

/// Hostile typed input on the skew chip: NaN, infinity, huge and degenerate angles never convert
/// or write.
#[test]
fn hostile_typed_skew_input_writes_nothing() {
    let d = Document::new(1);
    let _ = polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 5);
    let _ = star(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 5);
    for text in [
        "NaN",
        "inf",
        "-inf",
        "1e999",
        "abc",
        "",
        "90",
        "-90",
        "89.9999999999",
        "180",
        "1e308",
        "0x10",
    ] {
        let mut s = open(&d);
        select_everything(&mut s);
        let base = bytes_of(&s);
        assert_eq!(key(&mut s, "k", false), KeyOutcome::EntryOpened);
        let out = s.commit_transform_entry(text, "", 0);
        let objs = objects(&s);
        if matches!(out, EntryOutcome::Committed) {
            for o in &objs {
                for a in anchors_of(o) {
                    assert!(
                        a.point.x.is_finite()
                            && a.point.y.is_finite()
                            && a.point.x.abs() < 1e7
                            && a.point.y.abs() < 1e7,
                        "typed '{text}' committed a non-finite or beyond-limit result"
                    );
                }
            }
        } else {
            assert!(
                bytes_of(&s) == base,
                "typed '{text}': {out:?} but the document changed"
            );
            assert!(
                objs.iter().all(|o| !is_path(o)),
                "typed '{text}': converted without writing"
            );
        }
    }
    // Hostile typed size.
    for (w, h) in [
        ("NaN", "10"),
        ("10", "inf"),
        ("-5", "10"),
        ("0", "0"),
        ("1e999", "1"),
        ("1e7", "1e7"),
        ("0.0000000001", "10"),
    ] {
        let mut s = open(&d);
        select_everything(&mut s);
        let base = bytes_of(&s);
        key(&mut s, "s", false);
        let out = s.commit_transform_entry(w, h, 0);
        if !matches!(out, EntryOutcome::Committed) {
            assert!(
                bytes_of(&s) == base,
                "size '{w}' x '{h}': {out:?} but the document changed"
            );
            assert!(
                objects(&s).iter().all(|o| !is_path(o)),
                "size '{w}' x '{h}': converted without writing"
            );
        } else {
            for o in objects(&s) {
                for a in anchors_of(&o) {
                    assert!(
                        a.point.x.is_finite()
                            && a.point.y.is_finite()
                            && a.point.x.abs() <= 1e7 + 1.0
                    );
                }
            }
        }
    }
}

fn stroke_mm(o: &ObjectSnapshot) -> f64 {
    match o {
        ObjectSnapshot::Primitive(p) => p.style.stroke.width.as_mm(),
        ObjectSnapshot::Clone(p) => p.resolved.style.stroke.width.as_mm(),
        ObjectSnapshot::Path(p) => p.style.stroke.width.as_mm(),
    }
}

/// Criterion 8: "Scale stroke width" is read at the press, and a converted object's stroke width
/// follows the switch exactly like the path's next to it.
#[test]
fn ac08_stroke_width_switch_read_at_the_press_and_followed_by_converted_objects() {
    let make = || {
        let d = Document::new(1);
        let p = curved_path(&d, 1, 100.0, 100.0);
        let q = polygon(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 5);
        for id in [p, q] {
            d.edit_style(
                &[id],
                &curvyo_document_core::StyleEdit::StrokeWidth(Length::from_mm(0.8)),
            )
            .unwrap();
        }
        d
    };
    for (on_at_press, flip_during) in [(true, false), (false, false), (true, true), (false, true)] {
        let mut s = open(&make());
        select_everything(&mut s);
        s.set_scale_stroke_width(on_at_press);
        let g = GBox::of(&s);
        let from = g.corner(1.0, 1.0);
        let to = pt(from.x + g.w(), from.y + 2.0 * g.h());
        hold(&mut s, from, false, false);
        s.pointer_down(from, false);
        hold(&mut s, to, false, false);
        if flip_during {
            s.set_scale_stroke_width(!on_at_press);
        }
        s.pointer_up(to, false, false);
        let os = objects(&s);
        assert!(is_path(&os[1]), "the polygon converted");
        let (a, b) = (stroke_mm(&os[0]) / 0.8, stroke_mm(&os[1]) / 0.8);
        assert!(
            near(a, b, 1e-9),
            "converted stroke follows the path's: {a} vs {b}"
        );
        if on_at_press {
            assert!(
                a > 1.0 + 1e-6,
                "the switch was on at the press: scaled ({a})"
            );
        } else {
            assert!(
                near(a, 1.0, 1e-9),
                "the switch was off at the press: unchanged ({a})"
            );
        }
    }
}

// ---------------------------------------------------------------------
// Tolerances: uniform versus stretch, aligned versus turned
// ---------------------------------------------------------------------

/// The "uniform" test of the Terms: |sx - sy| <= 1e-9 converts nothing, even through a free corner
/// drag whose pointer lies exactly on the diagonal; a visible difference converts.
#[test]
fn ac10_uniform_means_within_1e_9() {
    let make = || {
        let d = Document::new(1);
        let _ = polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 4);
        let _ = star(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 4);
        d
    };
    for (extra_y, converts) in [(0.0, false), (1e-12, false), (1e-3, true), (1.0, true)] {
        let mut s = open(&make());
        select_everything(&mut s);
        let g = GBox::of(&s);
        let from = g.corner(1.0, 1.0);
        // Pointer on the diagonal: width and height grow by the same factor 1.5.
        let to = pt(from.x + 0.5 * g.w(), from.y + 0.5 * g.h() + extra_y);
        drag(&mut s, from, to);
        let any_path = objects(&s).iter().any(is_path);
        assert_eq!(
            any_path, converts,
            "extra {extra_y}: converted = {any_path}"
        );
    }
    // Typed: exactly proportional text does not convert (here a 2x on exact sizes).
    let mut s = open(&make());
    select_everything(&mut s);
    key(&mut s, "s", false);
    let v = s.transform_entry().unwrap();
    let (w, h) = (
        v.fields[0].prefill.parse::<f64>().unwrap(),
        v.fields[1].prefill.parse::<f64>().unwrap(),
    );
    assert_eq!(
        s.commit_transform_entry(&format!("{}", w * 3.0), &format!("{}", h * 3.0), 0),
        EntryOutcome::Committed
    );
    assert!(!objects(&s).iter().any(is_path));
}

/// Aligned rectangles: a quarter turn made by the typed rotation, by two eighth turns and by the
/// Document API all stay rectangles in a stretch of a multi-selection; a turn that misses a
/// multiple of 90 degrees by more than 1e-9 rad converts.
#[test]
fn ac07_aligned_means_a_multiple_of_90_degrees_within_1e_9_rad() {
    let stretch = |turn_rad: &dyn Fn(&Document, NodeId)| {
        let d = Document::new(1);
        let _ = curved_path(&d, 1, 60.0, 100.0);
        let id = d.create_rect(rect_bounds(100.0, 94.0, 20.0, 12.0));
        turn_rad(&d, id);
        let mut s = open(&d);
        select_everything(&mut s);
        let g = GBox::of(&s);
        let from = g.mid(1.0, 0.0);
        drag(&mut s, from, pt(from.x + 20.0, from.y));
        let n = objects(&s).into_iter().find(|o| oid(o) == id).unwrap();
        is_path(&n)
    };
    let quarter = std::f64::consts::FRAC_PI_2;
    assert!(
        !stretch(&|d, id| rotate_about_centre(d, id, quarter)),
        "90 degrees stays a rectangle"
    );
    assert!(
        !stretch(&|d, id| rotate_about_centre(d, id, 3.0 * quarter)),
        "270"
    );
    assert!(
        !stretch(&|d, id| rotate_about_centre(d, id, -quarter)),
        "-90"
    );
    assert!(
        !stretch(&|d, id| rotate_about_centre(d, id, 2.0 * quarter)),
        "180"
    );
    assert!(
        !stretch(&|d, id| {
            rotate_about_centre(d, id, quarter / 2.0);
            rotate_about_centre(d, id, quarter / 2.0);
        }),
        "two 45 degree turns"
    );
    assert!(
        !stretch(&|d, id| {
            for _ in 0..9 {
                rotate_about_centre(d, id, quarter / 9.0);
            }
        }),
        "nine 10 degree turns (rounding noise)"
    );
    assert!(
        !stretch(&|d, id| rotate_about_centre(d, id, quarter + 1e-12)),
        "90 degrees + 1e-12 rad"
    );
    assert!(
        stretch(&|d, id| rotate_about_centre(d, id, quarter + 1e-6)),
        "90 degrees + 1e-6 rad converts"
    );
    assert!(
        stretch(&|d, id| rotate_about_centre(d, id, 0.5)),
        "0.5 rad converts"
    );
}

/// A star with 1024 points (2048 nodes) and a 1024-gon skew exactly as their "Object to path"
/// forms do, and the conversion adds no commit of its own: Loro itself splits a very large
/// commit into several changes (platform behaviour, the same for a plain path of 3000 nodes), so
/// the count is compared with the count the same skew of the already converted paths makes.
#[test]
fn many_nodes_convert_without_an_extra_commit() {
    let d = Document::new(1);
    let _ = star(&d, 100.0, 100.0, 30.0, -FRAC_PI_2, 1024);
    let _ = polygon(&d, 180.0, 100.0, 30.0, -FRAC_PI_2, 1024);
    let otp = otp_objects(&d);
    // The already converted document, skewed: the reference count.
    let converted = {
        let mut t = open(&d);
        select_everything(&mut t);
        t.convert_selected_to_paths();
        doc_of(&t)
    };
    let mut r = open(&converted);
    select_everything(&mut r);
    let rc = change_count(&r);
    key(&mut r, "k", false);
    assert_eq!(
        r.commit_transform_entry("12", "", 0),
        EntryOutcome::Committed
    );
    let path_delta = change_count(&r) - rc;
    let mut s = open(&d);
    select_everything(&mut s);
    let before_vv = loro_of(&doc_of(&s)).oplog_vv();
    key(&mut s, "k", false);
    assert_eq!(
        s.commit_transform_entry("12", "", 0),
        EntryOutcome::Committed
    );
    // Loro splits a very large commit into several changes; they are one commit when they
    // carry the same message and the same time stamp, and the message is the skew's label.
    let new_changes = changes_since(&s, &before_vv);
    assert!(!new_changes.is_empty());
    assert!(
        new_changes
            .iter()
            .all(|(m, t)| *m == new_changes[0].0 && *t == new_changes[0].1),
        "all new changes belong to one commit: {new_changes:?}"
    );
    assert!(path_delta >= 1);
    assert_ne!(new_changes[0].0, "Object to path");
    let after = objects(&s);
    assert_eq!(anchors_of(&after[0]).len(), 2048);
    assert_eq!(anchors_of(&after[1]).len(), 1024);
    for (i, (o, n)) in otp.iter().zip(&after).enumerate() {
        let m = fitted_image(o, n, &format!("big {i}"));
        assert_pure_shear(&m, 12.0_f64.to_radians().tan(), "big");
    }
    assert_ids_unique(&s, "1024-point shapes");
}

/// A turned polygon in a multi-selection: converts on a stretch, exact image of its "Object to
/// path" form under the axis scale; a rotated star likewise.
#[test]
fn ac07_turned_polygon_and_star_in_a_selection() {
    let d = Document::new(1);
    let p = polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 6);
    let q = star(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 5);
    rotate_about_centre(&d, p, 0.7);
    rotate_about_centre(&d, q, -1.1);
    let _ = curved_path(&d, 1, 180.0, 100.0);
    let otp = otp_objects(&d);
    let mut s = open(&d);
    select_everything(&mut s);
    let g = GBox::of(&s);
    let anchor_pt = g.corner(-1.0, 1.0);
    let from = g.corner(1.0, -1.0);
    let to = pt(from.x + 0.3 * g.w(), from.y - 0.6 * g.h());
    let (sx, sy) = ((to.x - anchor_pt.x) / g.w(), (anchor_pt.y - to.y) / g.h());
    drag(&mut s, from, to);
    let after = objects(&s);
    for i in 0..2 {
        assert!(is_path(&after[i]));
        let m = fitted_image(&otp[i], &after[i], &format!("turned {i}"));
        assert!(
            near(m.a, sx, 1e-4) && near(m.d, sy, 1e-4) && m.b.abs() < 1e-6 && m.c.abs() < 1e-6,
            "turned {i}: {m:?} want ({sx}, {sy})"
        );
    }
}

/// A single turned rectangle and ellipse skewed by the chip (box axes) become paths that are a
/// shear of their "Object to path" form, and a drag on their own oriented skew handle works too.
#[test]
fn ac01_ac02_turned_single_objects_skew_by_drag() {
    for (kind, deg) in [(Kind::RotRect30, 30.0_f64), (Kind::RotEllipse25, 25.0)] {
        let d = single(kind);
        let otp = otp_objects(&d);
        let mut s = open(&d);
        select_everything(&mut s);
        let k = s.view().scale();
        // The oriented box: centre (120, 100), half sizes (10, 6), turned by `deg`.
        let (sn, cs) = deg.to_radians().sin_cos();
        // Top skew handle: 16 px out along the box's own -y axis from the top side's middle.
        let out = 6.0 + 16.0 / k;
        let h = pt(120.0 + out * sn, 100.0 - out * cs);
        assert!(
            skew_cursor_at(&mut s, h),
            "{kind:?}: the turned box has a skew handle at the top"
        );
        let base_changes = change_count(&s);
        // Drag along the box's own x axis by 6 mm.
        drag(&mut s, h, pt(h.x + 6.0 * cs, h.y + 6.0 * sn));
        assert_eq!(change_count(&s), base_changes + 1);
        let after = objects(&s);
        assert!(is_path(&after[0]), "{kind:?} became a path");
        let m = fitted_image(&otp[0], &after[0], "turned skew");
        assert!(
            m.nilpotency() < 1e-7 && near(m.det(), 1.0, 1e-7),
            "{kind:?}: a shear"
        );
        assert!(
            m.frob_minus_identity() > 0.05,
            "{kind:?}: the skew did something"
        );
        // The shear is along the box's own x axis: (A - I) maps everything onto +-(cs, sn).
        let (dx, dy) = (m.a - 1.0 + m.b * 0.0, m.c);
        let along = dx * sn - dy * cs; // component of the first column of A-I across the axis
        assert!(
            along.abs() < 1e-6,
            "{kind:?}: the displacement lies along the box x axis, off by {along}"
        );
    }
}

/// (message, time stamp) of every Loro change the session made after `since`.
fn changes_since(s: &Session, since: &loro::VersionVector) -> Vec<(String, i64)> {
    let l = loro_of(&doc_of(s));
    let now = l.oplog_vv();
    let mut out = Vec::new();
    for (peer, end) in now.iter() {
        let start = since.get(peer).copied().unwrap_or(0);
        let mut counter = start;
        while counter < *end {
            let c = l.get_change(loro::ID::new(*peer, counter)).unwrap();
            out.push((c.message().to_string(), c.timestamp));
            counter = c.id.counter + i32::try_from(c.len).unwrap();
        }
    }
    out
}

/// Criterion 11: the hint chips and the cursors of the handles do not depend on the kinds of the
/// selected objects: a selection of a polygon and a star says exactly what a selection of two
/// paths with the same box says, and so does a selection with an aligned rectangle.
#[test]
fn ac11_handle_hints_do_not_depend_on_the_kinds() {
    let paths = || {
        let d = Document::new(1);
        let _ = d.create_path(
            &[
                anchor(1, 92.0, 92.0),
                anchor(2, 108.0, 92.0),
                anchor(3, 108.0, 108.0),
                anchor(4, 92.0, 108.0),
            ],
            true,
        );
        let _ = d.create_path(
            &[
                anchor(10, 132.0, 92.0),
                anchor(11, 148.0, 92.0),
                anchor(12, 148.0, 108.0),
                anchor(13, 132.0, 108.0),
            ],
            true,
        );
        d
    };
    let shapes = || {
        let d = Document::new(1);
        let _ = polygon(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 4);
        let _ = d.create_rect(rect_bounds(132.0, 92.0, 16.0, 16.0));
        d
    };
    let stars = || {
        let d = Document::new(1);
        let _ = star(&d, 100.0, 100.0, 8.0, -FRAC_PI_2, 4);
        let _ = star(&d, 140.0, 100.0, 8.0, -FRAC_PI_2, 4);
        d
    };
    let hover_texts = |d: &Document| {
        let mut s = open(d);
        select_everything(&mut s);
        let g = GBox::of(&s);
        let mut out = Vec::new();
        for p in [
            g.corner(1.0, 1.0),
            g.corner(-1.0, -1.0),
            g.mid(1.0, 0.0),
            g.mid(0.0, -1.0),
            g.skew(0.0, -1.0),
            g.skew(1.0, 0.0),
            g.centre(),
        ] {
            hold(&mut s, p, false, false);
            out.push((s.cursor_hint(), s.handle_hint(), s.corner_hint_lines()));
        }
        (out, (g.min.x, g.max.x))
    };
    let (a, _ga) = hover_texts(&paths());
    for (name, d) in [("polygon + rect", shapes()), ("two stars", stars())] {
        let (b, gb) = hover_texts(&d);
        let _ = gb;
        assert_eq!(a.len(), b.len());
        for (i, (x, y)) in a.iter().zip(&b).enumerate() {
            assert_eq!(
                x, y,
                "{name}: handle {i} says something else than for two paths"
            );
        }
    }
}
