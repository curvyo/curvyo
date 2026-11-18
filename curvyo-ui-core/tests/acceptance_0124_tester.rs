//! Independent tester tests for `specs/0124-gradient-fill` SVG export and import (criteria 13,
//! 14, 27): the export goldens read by the tester's own small parser (ids, units, stops,
//! `gradientTransform` only on an elliptical radial), the exported picture evaluated by the
//! tester's own SVG gradient evaluation against the document's picture (plain, rotated, mirrored,
//! skewed, elliptical, open path), export -> import -> the same picture, the Inkscape and
//! Illustrator goldens against values worked out by hand from each file, and hostile inputs
//! (href chains and cycles, 0 and 1 stops, unordered and percent offsets, rgba, mesh, reflect).
//! Written from the specification and ADR 0067; only public names were looked up.

#![allow(
    clippy::many_single_char_names,
    clippy::type_complexity,
    clippy::needless_raw_string_hashes
)]
#![allow(
    clippy::format_push_string,
    clippy::semicolon_if_nothing_returned,
    clippy::assert_is_empty
)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, missing_docs, clippy::doc_markdown)]

use std::collections::HashMap;
use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, Angle, AxisRatio, BoxPoint, Color, Document, DrawingNode, EllipseFrame, FillPaint,
    GradientGeometry, GradientStop, GradientStops, Length, NewAnchor, NodeId, ObjectSnapshot,
    PlacedGradient, Point, RectBounds, StopOffset, StyleEdit,
};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{
    ExportScope, ImportFaces, SvgExportOptions, SvgPlacement, svg_export, svg_import,
};

// ---------- the tester's own tiny SVG reading ----------

#[derive(Debug, Clone)]
struct Tag {
    name: String,
    attrs: HashMap<String, String>,
}

/// Every start tag of `text` in order (no comments or CDATA in these files that matter).
fn tags(text: &str) -> Vec<Tag> {
    let mut out = Vec::new();
    let b = text.as_bytes();
    let mut i = 0;
    while let Some(off) = text[i..].find('<') {
        let start = i + off + 1;
        if start >= b.len() || matches!(b[start], b'/' | b'?' | b'!') {
            i = start;
            continue;
        }
        let end = start + text[start..].find('>').unwrap();
        let body = &text[start..end];
        let name_end = body
            .find(|c: char| c.is_whitespace() || c == '/')
            .unwrap_or(body.len());
        let mut attrs = HashMap::new();
        let mut rest = &body[name_end..];
        while let Some(eq) = rest.find('=') {
            let key = rest[..eq].trim().to_owned();
            let q = rest[eq + 1..].trim_start();
            let quote = q.chars().next().unwrap();
            let close = q[1..].find(quote).unwrap();
            attrs.insert(key, q[1..=close].to_owned());
            rest = &q[close + 2..];
        }
        out.push(Tag {
            name: body[..name_end].to_owned(),
            attrs,
        });
        i = end;
    }
    out
}

fn num(t: &Tag, k: &str) -> f64 {
    t.attrs[k]
        .parse()
        .unwrap_or_else(|_| panic!("{k} of {t:?}"))
}

type M = [f64; 6];
const I: M = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn mul(a: M, b: M) -> M {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

fn inv(m: M) -> M {
    let d = m[0] * m[3] - m[1] * m[2];
    let (a, b, c, e) = (m[3] / d, -m[1] / d, -m[2] / d, m[0] / d);
    [a, b, c, e, -(a * m[4] + c * m[5]), -(b * m[4] + e * m[5])]
}

fn ap(m: M, p: (f64, f64)) -> (f64, f64) {
    (
        m[0] * p.0 + m[2] * p.1 + m[4],
        m[1] * p.0 + m[3] * p.1 + m[5],
    )
}

/// The tester's own `transform` list reader: matrix, translate, scale, rotate, skewX, skewY.
fn transform(s: &str) -> M {
    let mut m = I;
    for part in s.split(')').filter(|p| !p.trim().is_empty()) {
        let (name, args) = part.split_once('(').unwrap();
        let v: Vec<f64> = args
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|x| !x.is_empty())
            .map(|x| x.parse().unwrap())
            .collect();
        let t = match name.trim() {
            "matrix" => [v[0], v[1], v[2], v[3], v[4], v[5]],
            "translate" => [1.0, 0.0, 0.0, 1.0, v[0], *v.get(1).unwrap_or(&0.0)],
            "scale" => [v[0], 0.0, 0.0, *v.get(1).unwrap_or(&v[0]), 0.0, 0.0],
            "rotate" => {
                let (s, c) = v[0].to_radians().sin_cos();
                let r = [c, s, -s, c, 0.0, 0.0];
                if v.len() == 3 {
                    mul(
                        mul([1.0, 0.0, 0.0, 1.0, v[1], v[2]], r),
                        [1.0, 0.0, 0.0, 1.0, -v[1], -v[2]],
                    )
                } else {
                    r
                }
            }
            "skewX" => [1.0, 0.0, v[0].to_radians().tan(), 1.0, 0.0, 0.0],
            "skewY" => [1.0, v[0].to_radians().tan(), 0.0, 1.0, 0.0, 0.0],
            other => panic!("transform {other}"),
        };
        m = mul(m, t);
    }
    m
}

/// A gradient as the tester reads it from an export (userSpaceOnUse only).
#[derive(Debug, Clone)]
struct Grad {
    radial: bool,
    p: [f64; 4], // x1 y1 x2 y2 or cx cy r 0
    transform: M,
    stops: Vec<(f64, [u8; 3], f64)>,
}

fn t_of(g: &Grad, element: M, doc: (f64, f64)) -> f64 {
    let q = ap(inv(mul(element, g.transform)), doc);
    if g.radial {
        ((q.0 - g.p[0]).powi(2) + (q.1 - g.p[1]).powi(2)).sqrt() / g.p[2]
    } else {
        let (dx, dy) = (g.p[2] - g.p[0], g.p[3] - g.p[1]);
        ((q.0 - g.p[0]) * dx + (q.1 - g.p[1]) * dy) / (dx * dx + dy * dy)
    }
}

fn hex3(s: &str) -> [u8; 3] {
    let s = s.trim_start_matches('#');
    [0, 2, 4].map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
}

/// Gradients by id, and each element that uses one with its own transform, in order.
fn read_export(text: &str) -> (HashMap<String, Grad>, Vec<(String, M, Tag)>) {
    let all = tags(text);
    let mut grads = HashMap::new();
    let mut uses = Vec::new();
    let mut current: Option<(String, Grad)> = None;
    for t in &all {
        match t.name.as_str() {
            "linearGradient" | "radialGradient" => {
                if let Some((id, g)) = current.take() {
                    grads.insert(id, g);
                }
                assert_eq!(t.attrs["gradientUnits"], "userSpaceOnUse", "{t:?}");
                let radial = t.name == "radialGradient";
                let p = if radial {
                    assert_eq!(num(t, "fx"), num(t, "cx"), "fx equals cx");
                    assert_eq!(num(t, "fy"), num(t, "cy"), "fy equals cy");
                    [num(t, "cx"), num(t, "cy"), num(t, "r"), 0.0]
                } else {
                    [num(t, "x1"), num(t, "y1"), num(t, "x2"), num(t, "y2")]
                };
                let tr = t.attrs.get("gradientTransform").map_or(I, |s| transform(s));
                current = Some((
                    t.attrs["id"].clone(),
                    Grad {
                        radial,
                        p,
                        transform: tr,
                        stops: vec![],
                    },
                ));
            }
            "stop" => {
                let g = &mut current.as_mut().expect("stop inside a gradient").1;
                g.stops.push((
                    num(t, "offset"),
                    hex3(&t.attrs["stop-color"]),
                    num(t, "stop-opacity"),
                ));
            }
            _ => {
                if let Some(f) = t.attrs.get("fill").filter(|f| f.starts_with("url(#")) {
                    let id = f
                        .trim_start_matches("url(#")
                        .trim_end_matches(')')
                        .to_owned();
                    assert!(
                        !t.attrs.contains_key("fill-opacity"),
                        "no fill-opacity: {t:?}"
                    );
                    let m = t.attrs.get("transform").map_or(I, |s| transform(s));
                    uses.push((id, m, t.clone()));
                }
            }
        }
    }
    if let Some((id, g)) = current.take() {
        grads.insert(id, g);
    }
    (grads, uses)
}

// ---------- criterion 13: the goldens through the tester's parser ----------

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../curvyo-svg-core/tests/fixtures")
}

#[test]
fn c13_every_export_golden_has_the_structure_inkscape_writes() {
    let names = [
        "gradient_linear",
        "gradient_multistop",
        "gradient_radial",
        "gradient_ellipse_rotated",
        "gradient_compound",
        "gradient_open_path",
        "gradient_text",
    ];
    for name in names {
        let text =
            std::fs::read_to_string(golden_dir().join(format!("export/{name}.svg"))).unwrap();
        let (grads, uses) = read_export(&text);
        assert!(!uses.is_empty(), "{name} uses a gradient");
        // Ids numbered in document order.
        for (k, (id, _, _)) in uses.iter().enumerate() {
            assert_eq!(id, &format!("gradient-{}", k + 1), "{name}");
        }
        assert_eq!(
            grads.len(),
            uses.len(),
            "{name}: one gradient per filled element"
        );
        for (id, g) in &grads {
            assert!(g.stops.len() >= 2, "{name} {id}");
            assert_eq!(g.stops[0].0, 0.0);
            assert_eq!(g.stops.last().unwrap().0, 1.0);
            assert!(g.stops.windows(2).all(|w| w[0].0 <= w[1].0));
            let elliptic = g.transform != I;
            if elliptic {
                assert!(g.radial, "{name}: gradientTransform only on a radial");
                assert_eq!(
                    name, "gradient_ellipse_rotated",
                    "{name} must not carry a matrix"
                );
            }
        }
        if name == "gradient_multistop" {
            let s = &grads["gradient-1"].stops;
            assert_eq!(s.len(), 7);
            assert!(s.windows(2).any(|w| w[0].0 == w[1].0), "one hard edge");
        }
        if name == "gradient_text" {
            assert!(
                uses.iter().any(|(_, _, t)| t.name == "text"),
                "the text element gets the fill"
            );
        }
        if name == "gradient_ellipse_rotated" {
            assert!(
                grads.values().any(|g| g.transform != I),
                "the one that carries a matrix"
            );
        }
    }
}

// ---------- criterion 13 and 14: the exported picture by the tester's evaluation ----------

fn stop(r: u8, g: u8, b: u8, a: u8, o: f64) -> GradientStop {
    GradientStop {
        colour: Color { r, g, b },
        alpha: a,
        offset: StopOffset::new(o).unwrap(),
    }
}

fn shade(d: &Document, id: NodeId, geometry: GradientGeometry, stops: Vec<GradientStop>) {
    for e in [
        StyleEdit::FillEnabled(true),
        StyleEdit::FillPaint(FillPaint::Gradient),
        StyleEdit::GradientGeometry(geometry),
        StyleEdit::GradientStops(GradientStops::new(stops).unwrap()),
    ] {
        d.edit_style(&[id], &e).unwrap();
    }
}

fn many() -> Vec<GradientStop> {
    vec![
        stop(0, 0, 0, 255, 0.0),
        stop(255, 0, 0, 255, 0.2),
        stop(255, 255, 0, 128, 0.2),
        stop(0, 255, 0, 255, 0.55),
        stop(0, 0, 255, 0, 0.9),
        stop(255, 255, 255, 255, 1.0),
    ]
}

fn faces(store: &FaceStore) -> ImportFaces<'_> {
    ImportFaces {
        faces: &[],
        store,
        substitute: None,
    }
}

fn export(d: &Document) -> String {
    let store = FaceStore::new();
    let out = svg_export(
        d,
        &d.read_objects(),
        &faces(&store),
        &ExportScope {
            selection: &[],
            active_page: 0,
        },
        SvgExportOptions::default(),
    )
    .unwrap();
    curvyo_svg_core::write_svg(&out.svg)
}

fn import(text: &str) -> (Document, curvyo_svg_core::GradientLosses) {
    let drawing = curvyo_svg_core::read_svg(text.as_bytes()).unwrap();
    let store = FaceStore::new();
    let imp = svg_import(&drawing, SvgPlacement::File, &faces(&store), u64::MAX);
    let ids: Vec<AnchorId> = (0..DrawingNode::anchor_count(&imp.nodes) as u64)
        .map(|n| AnchorId::new(9, n))
        .collect();
    (
        Document::from_drawing(3, &imp.page, &imp.nodes, &ids, &[]).unwrap(),
        imp.gradient_losses,
    )
}

fn shaded(d: &Document) -> Vec<ObjectSnapshot> {
    d.read_objects()
        .objects
        .iter()
        .filter(|o| o.style().fill.painted_gradient().is_some())
        .cloned()
        .collect()
}

fn placed(o: &ObjectSnapshot) -> PlacedGradient {
    PlacedGradient::new(
        o.style().fill.painted_gradient().unwrap(),
        &o.gradient_box().unwrap(),
    )
}

fn samples(o: &ObjectSnapshot) -> Vec<(f64, f64)> {
    let b = o.gradient_box().unwrap();
    let mut v = Vec::new();
    for i in 0..7 {
        for j in 0..7 {
            let p = b.frame_point(
                -0.1 + 1.2 * f64::from(i) / 6.0,
                -0.1 + 1.2 * f64::from(j) / 6.0,
            );
            v.push((p.x, p.y));
        }
    }
    v
}

fn path(d: &Document, first: u64, pts: &[(f64, f64)], closed: bool) -> NodeId {
    let a: Vec<NewAnchor> = pts
        .iter()
        .zip(first..)
        .map(|((x, y), k)| NewAnchor::corner(AnchorId::new(1, k), Point::new(*x, *y)))
        .collect();
    d.create_path(&a, closed)
}

fn check_picture(d: &Document, what: &str) {
    let text = export(d);
    let (grads, uses) = read_export(&text);
    let originals = shaded(d);
    assert_eq!(uses.len(), originals.len(), "{what}");
    for ((id, m, _), o) in uses.iter().zip(&originals) {
        let g = &grads[id];
        let p = placed(o);
        // Stops: the tester reads the same colours and alphas (alpha within the 3 decimals).
        let want = o.style().fill.painted_gradient().unwrap().stops.clone();
        assert_eq!(g.stops.len(), want.len(), "{what}");
        for (s, w) in g.stops.iter().zip(want.as_slice()) {
            assert_eq!(s.1, [w.colour.r, w.colour.g, w.colour.b], "{what}");
            assert!(
                (s.2 * 255.0 - f64::from(w.alpha)).abs() < 0.5,
                "{what} alpha"
            );
            assert!((s.0 - w.offset.get()).abs() < 1e-9, "{what} offset");
        }
        for q in samples(o) {
            let ours = t_of(g, *m, q);
            let theirs = p.t_at(Point::new(q.0, q.1));
            assert!(
                (ours - theirs).abs() < 1e-5,
                "{what}: at {q:?} svg {ours} doc {theirs}\n{text}"
            );
        }
    }
    // Export -> import -> the same picture, then the same bytes again.
    let (back, losses) = import(&text);
    assert!(losses.is_empty(), "{what}: nothing lost");
    let again = shaded(&back);
    assert_eq!(again.len(), originals.len(), "{what}");
    for (a, o) in again.iter().zip(&originals) {
        assert_eq!(
            a.style().fill.painted_gradient().unwrap().stops,
            o.style().fill.painted_gradient().unwrap().stops,
            "{what}: stops exact"
        );
        let (pa, po) = (placed(a), placed(o));
        for q in samples(o) {
            let q = Point::new(q.0, q.1);
            assert!(
                (pa.t_at(q) - po.t_at(q)).abs() < 1e-5,
                "{what}: round trip at {q:?}"
            );
        }
    }
    // Without a matrix the re-export is byte-identical; a matrix may drift in its last digits
    // (about 1e-12, the picture above is checked to 1e-5).
    if !text.contains("gradientTransform") {
        assert_eq!(
            export(&back),
            text,
            "{what}: export -> import -> export is byte-identical"
        );
    }
}

#[test]
fn c13_c14_exported_pictures_match_the_document_and_round_trip() {
    let elliptic = GradientGeometry::Radial {
        centre: BoxPoint::new(0.35, 0.55).unwrap(),
        radius: BoxPoint::new(0.95, 0.25).unwrap(),
        ratio: AxisRatio::new(0.4).unwrap(),
    };
    let slanted = GradientGeometry::Linear {
        start: BoxPoint::new(-0.2, 0.1).unwrap(),
        end: BoxPoint::new(1.3, 0.9).unwrap(),
    };
    for (gname, geometry) in [
        ("linear", slanted),
        ("radial", GradientGeometry::DEFAULT_RADIAL),
        ("elliptic", elliptic),
    ] {
        // A plain rectangle.
        let d = Document::new(1);
        let id = d.create_rect(RectBounds {
            origin: Point::new(10.0, 20.0),
            width: Length::from_mm(80.0),
            height: Length::from_mm(30.0),
        });
        shade(&d, id, geometry, many());
        check_picture(&d, &format!("rect {gname}"));
        // A rotated rectangle and a rotated ellipse.
        let d = Document::new(1);
        let id = d.create_rect(RectBounds {
            origin: Point::new(10.0, 20.0),
            width: Length::from_mm(80.0),
            height: Length::from_mm(30.0),
        });
        let e = d.create_ellipse(EllipseFrame {
            center: Point::new(150.0, 80.0),
            rx: Length::from_mm(30.0),
            ry: Length::from_mm(12.0),
        });
        shade(&d, id, geometry, many());
        shade(&d, e, geometry, many());
        for (x, c, a) in [
            (id, Point::new(50.0, 35.0), 0.6),
            (e, Point::new(150.0, 80.0), -1.1),
        ] {
            let o = d.object(x).unwrap();
            d.rotate_object(&o.rotated(c, Angle::from_radians(a)))
                .unwrap();
        }
        check_picture(&d, &format!("rotated {gname}"));
        // A skewed, a mirrored and an open path (stored in their own frames).
        let d = Document::new(1);
        let skewed = path(
            &d,
            1,
            &[(0.0, 0.0), (60.0, 0.0), (90.0, 40.0), (30.0, 40.0)],
            true,
        );
        let mirrored = path(
            &d,
            11,
            &[(200.0, 0.0), (120.0, 10.0), (130.0, 50.0), (190.0, 45.0)],
            true,
        );
        let open = path(
            &d,
            21,
            &[(0.0, 100.0), (50.0, 160.0), (100.0, 100.0)],
            false,
        );
        for x in [skewed, mirrored, open] {
            shade(&d, x, geometry, many());
        }
        check_picture(&d, &format!("paths {gname}"));
    }
}

#[test]
fn c13_huge_coordinates_and_two_exports_are_byte_identical() {
    let d = Document::new(1);
    let id = d.create_rect(RectBounds {
        origin: Point::new(1.0e6, -1.0e6),
        width: Length::from_mm(3.0e5),
        height: Length::from_mm(0.001),
    });
    shade(&d, id, GradientGeometry::DEFAULT_LINEAR, many());
    let a = export(&d);
    assert_eq!(a, export(&d));
    let (back, _) = import(&a);
    assert_eq!(shaded(&back).len(), 1);
}

// ---------- criterion 14: the import goldens against hand-worked values ----------

fn real(name: &str) -> String {
    std::fs::read_to_string(golden_dir().join(format!("svg_real/{name}.svg"))).unwrap()
}

/// The single shaded object, its placed gradient and its stops as (offset, rgb, alpha).
fn single(
    name: &str,
) -> (
    ObjectSnapshot,
    PlacedGradient,
    Vec<(f64, [u8; 3], u8)>,
    curvyo_svg_core::GradientLosses,
) {
    let (d, losses) = import(&real(name));
    let s = shaded(&d);
    assert_eq!(s.len(), 1, "{name}");
    let o = s[0].clone();
    let p = placed(&o);
    let stops = o
        .style()
        .fill
        .painted_gradient()
        .unwrap()
        .stops
        .as_slice()
        .iter()
        .map(|s| {
            (
                s.offset.get(),
                [s.colour.r, s.colour.g, s.colour.b],
                s.alpha,
            )
        })
        .collect();
    (o, p, stops, losses)
}

/// Maps a file coordinate to the document through the shaded object's box, given the file box
/// of that object (uniform scale and a shift).
fn file_to_doc(
    o: &ObjectSnapshot,
    file_min: (f64, f64),
    file_w: f64,
) -> impl Fn((f64, f64)) -> Point {
    let b = o.gradient_box().unwrap();
    let s = (b.max.x - b.min.x) / file_w;
    let (bx, by) = (b.min.x, b.min.y);
    move |p| Point::new(bx + (p.0 - file_min.0) * s, by + (p.1 - file_min.1) * s)
}

#[test]
fn c14_inkscape_linear_href_reads_the_vector_and_the_link() {
    let (o, p, stops, losses) = single("inkscape_linear_href");
    assert!(losses.is_empty());
    assert_eq!(
        stops,
        vec![(0.0, [255, 0, 0], 255), (1.0, [0, 0, 255], 255)]
    );
    let to = file_to_doc(&o, (10.0, 10.0), 80.0);
    for (x, y) in [(10.0, 10.0), (30.0, 45.0), (50.0, 30.0), (90.0, 50.0)] {
        let want = (x - 10.0) / 80.0;
        assert!((p.t_at(to((x, y))) - want).abs() < 1e-6, "at {x},{y}");
    }
}

#[test]
fn c14_inkscape_multistop_chain_inherits_through_two_links_and_counts_reflect() {
    let (o, p, stops, losses) = single("inkscape_multistop_chain");
    assert_eq!(losses.repeats, 1, "reflect counted");
    assert_eq!(losses.focal_points, 0);
    assert_eq!(stops.len(), 5);
    assert_eq!(stops[2].0, 0.5);
    assert_eq!(stops[3].0, 0.5, "hard edge kept");
    assert_eq!(stops[3].1, [0, 255, 0]);
    // userSpaceOnUse from the middle link: vertical from y 100 to 160 over the square 10..70.
    let to = file_to_doc(&o, (10.0, 100.0), 60.0);
    for (x, y) in [(10.0, 100.0), (40.0, 115.0), (70.0, 145.0), (20.0, 160.0)] {
        let want = (y - 100.0) / 60.0;
        assert!(
            (p.t_at(to((x, y))) - want).abs() < 1e-6,
            "at {x},{y}: {}",
            p.t_at(to((x, y)))
        );
    }
}

#[test]
fn c14_inkscape_radial_transform_becomes_the_exact_ellipse() {
    let (o, p, stops, losses) = single("inkscape_radial_transform");
    assert!(losses.is_empty(), "fx = cx: not a focal point");
    assert_eq!(
        stops,
        vec![(0.0, [255, 255, 0], 255), (1.0, [0, 128, 0], 255)]
    );
    let m = transform("matrix(1.2990381,0.75,-0.25,0.4330127,-5.4422863,-16.650635)");
    let g = Grad {
        radial: true,
        p: [60.0, 50.0, 30.0, 0.0],
        transform: m,
        stops: vec![],
    };
    // The ellipse cx 60 cy 50 rx 40 ry 25: its box is 20..100 x 25..75.
    let to = file_to_doc(&o, (20.0, 25.0), 80.0);
    for (x, y) in [
        (60.0, 50.0),
        (30.0, 40.0),
        (90.0, 60.0),
        (60.0, 30.0),
        (75.0, 70.0),
    ] {
        let want = t_of(&g, I, (x, y));
        let got = p.t_at(to((x, y)));
        assert!((got - want).abs() < 1e-5, "at {x},{y}: {got} vs {want}");
    }
}

#[test]
fn c14_inkscape_userspace_and_bounding_box_units_under_layer_and_element_transforms() {
    let (d, losses) = import(&real("inkscape_userspace_units"));
    assert!(losses.is_empty());
    let s = shaded(&d);
    assert_eq!(s.len(), 2);
    // Order: the rect (objectBoundingBox) then the scaled path (userSpaceOnUse).
    let (rect, pathobj) = (&s[0], &s[1]);
    let pr = placed(rect);
    // rect 0..40 x 0..20 in the layer (translate 10, 20): in file coords 10..50 x 20..40.
    let to = file_to_doc(rect, (10.0, 20.0), 40.0);
    for (x, y) in [
        (10.0, 20.0),
        (20.0, 25.0),
        (35.0, 38.0),
        (50.0, 40.0),
        (30.0, 30.0),
    ] {
        // objectBoundingBox: unit square of the box, the vector (0.25, 0) to (0.75, 1) there.
        let (u, v) = ((x - 10.0) / 40.0, (y - 20.0) / 20.0);
        let want = ((u - 0.25) * 0.5 + v * 1.0) / (0.25 + 1.0);
        let got = pr.t_at(to((x, y)));
        assert!(
            (got - want).abs() < 1e-6,
            "bbox at {x},{y}: {got} vs {want}"
        );
    }
    // The path 0..40 x 40..60 scaled (2, 1) and translated: 10..90 x 60..80; gradient 0..40
    // in its user space, so 10..90 in the file.
    let pp = placed(pathobj);
    let to = file_to_doc(pathobj, (10.0, 60.0), 80.0);
    for (x, y) in [(10.0, 60.0), (50.0, 70.0), (90.0, 80.0), (30.0, 75.0)] {
        let want = (x - 10.0) / 80.0;
        assert!(
            (pp.t_at(to((x, y))) - want).abs() < 1e-6,
            "user space at {x},{y}"
        );
    }
}

#[test]
fn c14_inkscape_stop_opacity_times_fill_opacity() {
    let (_, _, stops, _) = single("inkscape_stop_opacity");
    let alphas: Vec<u8> = stops.iter().map(|s| s.2).collect();
    let want = [0.5, 0.125, 0.0].map(|a: f64| (a * 255.0).round());
    assert_eq!(stops.len(), 3);
    for (a, w) in alphas.iter().zip(want) {
        assert!((f64::from(*a) - w).abs() <= 1.0, "{alphas:?}");
    }
    assert!(stops.iter().all(|s| s.1 == [0, 0, 128]));
}

#[test]
fn c14_illustrator_linear_and_radial_with_focal_point_and_late_first_stop() {
    let (o, p, stops, losses) = single("illustrator_gradient_linear");
    assert!(losses.is_empty());
    assert_eq!(stops.len(), 3);
    assert_eq!(stops[1].1, [0xF7, 0x93, 0x1E]);
    let to = file_to_doc(&o, (20.0, 20.0), 160.0);
    assert!((p.t_at(to((100.0, 30.0))) - 0.5).abs() < 1e-6);
    let (o, p, stops, losses) = single("illustrator_gradient_radial");
    assert_eq!(losses.focal_points, 1, "fx fy away from the centre counted");
    assert_eq!(stops.first().unwrap().0, 0.0, "a first stop is added at 0");
    assert_eq!(
        stops.first().unwrap().1,
        [255, 255, 255],
        "in the end colour"
    );
    assert!(stops.iter().any(|s| (s.0 - 0.1).abs() < 1e-9));
    assert!(
        (f64::from(stops.last().unwrap().2) - 127.5).abs() <= 1.0,
        "stop-opacity 0.5"
    );
    let to = file_to_doc(&o, (60.0, 10.0), 80.0);
    assert!(
        p.t_at(to((100.0, 50.0))).abs() < 1e-6,
        "read as the centre, not the focal point"
    );
    assert!((p.t_at(to((140.0, 50.0))) - 1.0).abs() < 1e-6);
    assert!((p.t_at(to((100.0, 10.0))) - 1.0).abs() < 1e-6);
}

// ---------- criterion 14: hostile and edge inputs ----------

fn doc_of(body: &str) -> (Document, curvyo_svg_core::GradientLosses) {
    import(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="100mm" height="100mm" viewBox="0 0 100 100">{body}</svg>"#
    ))
}

fn stops_of(d: &Document) -> Vec<Vec<(f64, [u8; 3], u8)>> {
    shaded(d)
        .iter()
        .map(|o| {
            o.style()
                .fill
                .painted_gradient()
                .unwrap()
                .stops
                .as_slice()
                .iter()
                .map(|s| {
                    (
                        s.offset.get(),
                        [s.colour.r, s.colour.g, s.colour.b],
                        s.alpha,
                    )
                })
                .collect()
        })
        .collect()
}

#[test]
fn c14_zero_stops_no_fill_one_stop_two_of_that_colour() {
    let (d, _) = doc_of(
        r##"<defs><linearGradient id="a"/></defs><rect width="10" height="10" fill="url(#a)"/>"##,
    );
    assert!(shaded(&d).is_empty(), "no stop is no fill");
    let (d, _) = doc_of(
        r##"<defs><linearGradient id="a"><stop offset="0.3" stop-color="#123456"/></linearGradient></defs><rect width="10" height="10" fill="url(#a)"/>"##,
    );
    let s = stops_of(&d);
    assert_eq!(s.len(), 1);
    // ADR 0067 §7 says "two stops of that colour"; three of one colour draw the same.
    assert!(s[0].len() >= 2, "{s:?}");
    assert!(s[0].iter().all(|x| x.1 == [0x12, 0x34, 0x56]));
    assert_eq!((s[0][0].0, s[0].last().unwrap().0), (0.0, 1.0));
}

#[test]
fn c14_offsets_clamped_made_monotonic_percent_and_style_colours() {
    let (d, _) = doc_of(
        r##"<defs><linearGradient id="a">
        <stop offset="-0.5" stop-color="#ff0000"/>
        <stop offset="70%" style="stop-color:#00ff00;stop-opacity:0.5"/>
        <stop offset="0.4" stop-color="rgb(0,0,255)"/>
        <stop offset="2" stop-color="#ffffff" stop-opacity="0.25"/>
        </linearGradient></defs><rect width="10" height="10" fill="url(#a)" fill-opacity="0.5"/>"##,
    );
    let s = &stops_of(&d)[0];
    let offs: Vec<f64> = s.iter().map(|x| x.0).collect();
    assert!(offs.windows(2).all(|w| w[0] <= w[1]), "{offs:?}");
    assert_eq!(offs.first(), Some(&0.0));
    assert_eq!(offs.last(), Some(&1.0));
    assert!(
        offs.iter().any(|o| (o - 0.7).abs() < 1e-9),
        "percent: {offs:?}"
    );
    // The 0.4 stop is raised to 0.7 (each at least the previous).
    let blue = s.iter().find(|x| x.1 == [0, 0, 255]).unwrap();
    assert!((blue.0 - 0.7).abs() < 1e-9, "{s:?}");
    let green = s.iter().find(|x| x.1 == [0, 255, 0]).unwrap();
    assert!(
        (f64::from(green.2) - 63.75).abs() <= 1.0,
        "0.5 * fill-opacity 0.5: {s:?}"
    );
    let white = s.iter().find(|x| x.1 == [255, 255, 255]).unwrap();
    assert!((f64::from(white.2) - 31.875).abs() <= 1.0, "{s:?}");
}

#[test]
fn c14_href_chains_up_to_16_links_and_cycles_end() {
    let chain = |n: usize| {
        let mut defs = String::from(
            r##"<linearGradient id="g0"><stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient>"##,
        );
        for k in 1..=n {
            defs.push_str(&format!(
                r##"<linearGradient id="g{k}" xlink:href="#g{}"/>"##,
                k - 1
            ));
        }
        format!(r##"<defs>{defs}</defs><rect width="10" height="10" fill="url(#g{n})"/>"##)
    };
    let (d, _) = doc_of(&chain(16));
    assert_eq!(stops_of(&d).len(), 1, "16 links are followed");
    assert_eq!(stops_of(&d)[0][0].1, [255, 0, 0]);
    let (d, _) = doc_of(&chain(40));
    assert!(
        stops_of(&d).len() <= 1,
        "a long chain does not hang or panic"
    );
    // href (SVG 2) works too, and a cycle ends.
    let (d, _) = doc_of(
        r##"<defs><linearGradient id="a" href="#b"><stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#00ff00"/></linearGradient><linearGradient id="b" href="#a" x1="0" x2="0" y2="1"/></defs><rect width="10" height="10" fill="url(#b)"/>"##,
    );
    assert_eq!(
        stops_of(&d).len(),
        1,
        "cycle ends, stops from the nearest link"
    );
}

#[test]
fn c14_c27_mesh_is_no_fill_and_counted_repeat_counted_skew_radial_is_an_ellipse() {
    let (d, losses) = doc_of(
        r##"<defs><meshgradient id="m" x="0" y="0"><meshrow><meshpatch><stop path="c 10,0 10,0 10,0" stop-color="#ff0000"/></meshpatch></meshrow></meshgradient>
        <radialGradient id="r" gradientUnits="userSpaceOnUse" cx="50" cy="50" r="20" spreadMethod="repeat" gradientTransform="skewX(30)"><stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></radialGradient></defs>
        <rect x="0" y="0" width="10" height="10" fill="url(#m)"/>
        <rect x="20" y="20" width="60" height="60" fill="url(#r)"/>"##,
    );
    assert_eq!(losses.meshes, 1, "{losses:?}");
    assert_eq!(losses.repeats, 1, "{losses:?}");
    let s = shaded(&d);
    assert_eq!(s.len(), 1, "mesh gives no fill");
    let o = &s[0];
    match o.style().fill.painted_gradient().unwrap().geometry {
        GradientGeometry::Radial { ratio, .. } => {
            assert!((ratio.get() - 1.0).abs() > 1e-3, "skew gives an ellipse")
        }
        g @ GradientGeometry::Linear { .. } => panic!("{g:?}"),
    }
    let g = Grad {
        radial: true,
        p: [50.0, 50.0, 20.0, 0.0],
        transform: transform("skewX(30)"),
        stops: vec![],
    };
    let p = placed(o);
    let to = file_to_doc(o, (20.0, 20.0), 60.0);
    for q in [(50.0, 50.0), (60.0, 40.0), (30.0, 70.0), (75.0, 25.0)] {
        let want = t_of(&g, I, q);
        let got = p.t_at(to(q));
        assert!(
            (got - want).abs() < 1e-5,
            "skewed radial at {q:?}: {got} vs {want}"
        );
    }
}

#[test]
fn c27_loss_lines_wording_and_order() {
    let lines = curvyo_ui_core::gradient_loss_lines(curvyo_svg_core::GradientLosses {
        repeats: 3,
        focal_points: 1,
        meshes: 2,
    });
    assert_eq!(
        lines,
        vec![
            "3 gradient repeats were shown as plain shades.",
            "1 focal point was read as the centre.",
            "2 mesh gradients were left out.",
        ]
    );
    let lines = curvyo_ui_core::gradient_loss_lines(curvyo_svg_core::GradientLosses {
        repeats: 1,
        focal_points: 2,
        meshes: 1,
    });
    assert_eq!(
        lines,
        vec![
            "1 gradient repeat was shown as a plain shade.",
            "2 focal points were read as the centre.",
            "1 mesh gradient was left out.",
        ]
    );
    assert!(
        curvyo_ui_core::gradient_loss_lines(curvyo_svg_core::GradientLosses::default()).is_empty()
    );
}

#[test]
fn c14_degenerate_gradients_import_without_panic_and_save_valid_files() {
    for body in [
        r##"<defs><radialGradient id="g" cx="0.5" cy="0.5" r="0"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></radialGradient></defs><rect width="10" height="10" fill="url(#g)"/>"##,
        r##"<defs><linearGradient id="g" x1="0.5" x2="0.5" y1="0.5" y2="0.5"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs><rect width="10" height="10" fill="url(#g)"/>"##,
        r##"<defs><linearGradient id="g" gradientTransform="scale(0)"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs><rect width="10" height="10" fill="url(#g)"/>"##,
        r##"<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="1e300" x2="-1e300"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs><rect width="10" height="10" fill="url(#g)"/>"##,
        r##"<defs><linearGradient id="g"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs><rect width="0" height="10" fill="url(#g)"/><path d="M 0 0 L 10 0" fill="url(#g)"/>"##,
        r##"<rect width="10" height="10" fill="url(#missing)"/>"##,
        r##"<defs><linearGradient id="g" x1="NaN"><stop offset="NaN" stop-color="#f00"/><stop offset="1" stop-color="bogus"/></linearGradient></defs><rect width="10" height="10" fill="url(#g)"/>"##,
    ] {
        let (d, _) = doc_of(body);
        for o in shaded(&d) {
            let p = placed(&o);
            let _ = p.colour_at(Point::new(5.0, 5.0));
        }
        let bytes = curvyo_document_core::pack(&d, "t").unwrap();
        curvyo_document_core::unpack(4, &bytes)
            .unwrap_or_else(|e| panic!("an imported file must open again: {e:?}\n{body}"));
    }
}
