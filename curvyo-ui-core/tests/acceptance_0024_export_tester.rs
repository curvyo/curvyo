//! Independent tester tests for `specs/0024-svg-import-export` slice 4 (SVG export) at the model
//! mapping: a document built from `DrawingNode`s goes through `svg_export` and
//! `curvyo_svg_core::write_svg`, and the text is checked against the criteria; round trips read
//! it back with `read_svg` + `svg_import` (the Open route). Written from the specification
//! before the implementation was read; only the names of the public seams were looked up.
//! Texts need laid-out faces and are tested in `curvyo-editor-wasm/tests/acceptance_0024_export_tester.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::doc_markdown,
    clippy::format_push_string,
    clippy::many_single_char_names,
    clippy::unreadable_literal,
    clippy::uninlined_format_args,
    clippy::single_element_loop,
    missing_docs
)]

use std::sync::Arc;

use curvyo_document_core::{
    Affine, AnchorId, AnchorKind, Angle, ArcKind, AssetHash, Color, CornerRadii, Curve,
    DashPattern, DisplayUnit, Document, DocumentSize, DrawingAnchor, DrawingNode, DrawingOutline,
    DrawingPage, DrawingPageFrame, DrawingRegisters, EllipseArc, EllipseFrame, InnerRatio, Length,
    LineCap, LineJoin, MarkerCount, MarkerPlace, MarkerShape, Markers, NodeId, NodeKind,
    ObjectSnapshot, Opacity, Point, PointCount, RectBounds, Shape, StarFrame, Style, Vec2,
};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{
    ExportScope, ImportFaces, SvgExportOptions, SvgPlacement, svg_export, svg_import,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------------------------
// Building documents
// ---------------------------------------------------------------------------------------------

fn corner(x: f64, y: f64) -> DrawingAnchor {
    DrawingAnchor {
        point: Point::new(x, y),
        handle_in: Vec2::new(0.0, 0.0),
        handle_out: Vec2::new(0.0, 0.0),
        kind: AnchorKind::Corner,
    }
}

fn reg(name: &str) -> DrawingRegisters {
    DrawingRegisters {
        name: Some(name.to_owned()),
        ..DrawingRegisters::default()
    }
}

fn hairline() -> Style {
    let mut s = Style::default();
    s.fill.enabled = false;
    s.stroke.enabled = true;
    s.stroke.width = Length::from_mm(0.25);
    s.stroke.color = Color::BLACK;
    s.stroke.opacity = Opacity::OPAQUE;
    s.stroke.dash = DashPattern::new(Vec::new()).unwrap();
    s
}

fn filled(r: u8, g: u8, b: u8) -> Style {
    let mut s = hairline();
    s.fill.enabled = true;
    s.fill.color = Color { r, g, b };
    s.fill.opacity = Opacity::OPAQUE;
    s
}

fn layer(name: &str, children: Vec<DrawingNode>) -> DrawingNode {
    DrawingNode::Group {
        registers: reg(name),
        layer: true,
        children,
    }
}

fn group(name: Option<&str>, children: Vec<DrawingNode>) -> DrawingNode {
    DrawingNode::Group {
        registers: DrawingRegisters {
            name: name.map(str::to_owned),
            ..DrawingRegisters::default()
        },
        layer: false,
        children,
    }
}

fn path(
    registers: DrawingRegisters,
    outlines: Vec<(bool, Vec<DrawingAnchor>)>,
    style: Style,
) -> DrawingNode {
    DrawingNode::Path {
        registers,
        outlines: outlines
            .into_iter()
            .map(|(closed, anchors)| DrawingOutline { closed, anchors })
            .collect(),
        style,
    }
}

fn line(name: &str, a: (f64, f64), b: (f64, f64), style: Style) -> DrawingNode {
    path(
        reg(name),
        vec![(false, vec![corner(a.0, a.1), corner(b.0, b.1)])],
        style,
    )
}

fn rect_shape(x: f64, y: f64, w: f64, h: f64, r: [f64; 4]) -> Shape {
    Shape::Rect {
        bounds: RectBounds {
            origin: Point::new(x, y),
            width: Length::from_mm(w),
            height: Length::from_mm(h),
        },
        corner_radii: CornerRadii {
            tl: Length::from_mm(r[0]),
            tr: Length::from_mm(r[1]),
            br: Length::from_mm(r[2]),
            bl: Length::from_mm(r[3]),
        },
    }
}

fn prim(registers: DrawingRegisters, shape: Shape, deg: f64, style: Style) -> DrawingNode {
    DrawingNode::Primitive {
        registers,
        shape,
        rotation: Angle::from_radians(deg.to_radians()),
        style,
    }
}

fn rect(name: &str, x: f64, y: f64, w: f64, h: f64) -> DrawingNode {
    prim(reg(name), rect_shape(x, y, w, h, [0.0; 4]), 0.0, hairline())
}

fn ellipse_shape(cx: f64, cy: f64, rx: f64, ry: f64, arc: Option<EllipseArc>) -> Shape {
    Shape::Ellipse {
        frame: EllipseFrame {
            center: Point::new(cx, cy),
            rx: Length::from_mm(rx),
            ry: Length::from_mm(ry),
        },
        arc,
        curve: Curve::ELLIPSE_DEFAULT,
    }
}

fn star_shape(
    cx: f64,
    cy: f64,
    r: f64,
    deg: f64,
    points: u32,
    ratio: Option<f64>,
    curve: f64,
) -> Shape {
    let frame = StarFrame {
        center: Point::new(cx, cy),
        radius: Length::from_mm(r),
        angle: Angle::from_radians(deg.to_radians()),
    };
    let curve = Curve::new(curve).unwrap();
    match ratio {
        None => Shape::Polygon {
            frame,
            point_count: PointCount::new(points).unwrap(),
            curve,
        },
        Some(ratio) => Shape::Star {
            frame,
            point_count: PointCount::new(points).unwrap(),
            inner_ratio: InnerRatio::new(ratio).unwrap(),
            curve,
        },
    }
}

fn a4() -> DrawingPage {
    DrawingPage {
        size: DocumentSize::from_mm(210.0, 297.0),
        display_unit: Some(DisplayUnit::Mm),
        background: None,
        pages: Vec::new(),
    }
}

fn pages(frames: &[(&str, f64, f64, f64, f64)]) -> DrawingPage {
    DrawingPage {
        size: DocumentSize::from_mm(frames[0].3, frames[0].4),
        display_unit: Some(DisplayUnit::Mm),
        background: None,
        pages: frames
            .iter()
            .map(|&(name, x, y, w, h)| DrawingPageFrame {
                name: name.to_owned(),
                origin: Point::new(x, y),
                size: DocumentSize::from_mm(w, h),
            })
            .collect(),
    }
}

fn doc_with(page: &DrawingPage, nodes: &[DrawingNode], assets: &[Arc<[u8]>]) -> Document {
    let ids: Vec<AnchorId> = (0..DrawingNode::anchor_count(nodes) as u64)
        .map(|n| AnchorId::new(7, n))
        .collect();
    Document::from_drawing(5, page, nodes, &ids, assets).expect("valid drawing")
}

fn doc(page: &DrawingPage, nodes: &[DrawingNode]) -> Document {
    doc_with(page, nodes, &[])
}

// ---------------------------------------------------------------------------------------------
// Export and open
// ---------------------------------------------------------------------------------------------

struct Out {
    text: String,
    objects: usize,
}

fn export_full(
    d: &Document,
    options: SvgExportOptions,
    selection: &[NodeId],
    active: usize,
) -> Out {
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let laid = d.read_objects();
    let outcome = svg_export(
        d,
        &laid,
        &fonts,
        &ExportScope {
            selection,
            active_page: active,
        },
        options,
    );
    let text = outcome.text();
    assert_eq!(text, curvyo_svg_core::write_svg(&outcome.svg));
    Out {
        text,
        objects: outcome.objects,
    }
}

fn export(d: &Document) -> String {
    export_full(d, SvgExportOptions::default(), &[], 0).text
}

fn export_opt(d: &Document, options: SvgExportOptions) -> String {
    export_full(d, options, &[], 0).text
}

fn open(text: &str) -> Document {
    let drawing = curvyo_svg_core::read_svg(text.as_bytes())
        .unwrap_or_else(|e| panic!("the export reads back: {e:?}\n{text}"));
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let import = svg_import(&drawing, SvgPlacement::File, &fonts, u64::MAX);
    doc_with(&import.page, &import.nodes, &import.assets)
}

// ---------------------------------------------------------------------------------------------
// Reading the text
// ---------------------------------------------------------------------------------------------

/// Every start tag `<name …>` of the file as (name, raw attribute text).
fn tags(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        if rest.starts_with('/') || rest.starts_with('?') || rest.starts_with('!') {
            continue;
        }
        let end = rest.find('>').unwrap();
        let inner = rest[..end].trim_end_matches('/');
        let name_end = inner
            .find(|c: char| c.is_whitespace())
            .unwrap_or(inner.len());
        out.push((inner[..name_end].to_owned(), inner[name_end..].to_owned()));
        rest = &rest[end..];
    }
    out
}

/// The attributes of an attribute text in order.
fn attrs(raw: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = raw.trim();
    while let Some(eq) = rest.find("=\"") {
        let name = rest[..eq].trim().to_owned();
        let value_end = rest[eq + 2..].find('"').unwrap();
        out.push((name, rest[eq + 2..eq + 2 + value_end].to_owned()));
        rest = rest[eq + 2 + value_end + 1..].trim_start();
    }
    out
}

fn attr(raw: &str, name: &str) -> Option<String> {
    attrs(raw)
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v)
}

/// The tag with this id.
fn by_id(text: &str, id: &str) -> (String, String) {
    tags(text)
        .into_iter()
        .find(|(_, raw)| attr(raw, "id").as_deref() == Some(id))
        .unwrap_or_else(|| panic!("an element id={id}\n{text}"))
}

fn of_kind(text: &str, kind: &str) -> Vec<String> {
    tags(text)
        .into_iter()
        .filter(|(n, _)| n == kind)
        .map(|(_, raw)| raw)
        .collect()
}

/// Number of drawn elements (anything but svg, namedview, page, g, defs, marker and the marker's
/// own path).
fn drawn(text: &str) -> usize {
    let body = text.split("</defs>").last().unwrap();
    tags(body)
        .into_iter()
        .filter(|(n, _)| {
            matches!(
                n.as_str(),
                "path" | "rect" | "ellipse" | "circle" | "use" | "image" | "text"
            )
        })
        .count()
}

fn numbers(d: &str) -> Vec<f64> {
    d.split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|t| t.parse::<f64>().ok())
        .collect()
}

/// Every number in the file's attribute values (for the number-format rules).
fn number_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (_, raw) in tags(text) {
        for (name, value) in attrs(&raw) {
            if name.starts_with("xmlns") || name == "xlink:href" || name == "inkscape:label" {
                continue;
            }
            for t in value
                .split(|c: char| c.is_whitespace() || c == ',' || c == '(' || c == ')' || c == '=')
            {
                if t.parse::<f64>().is_ok() {
                    out.push(t.to_owned());
                }
            }
        }
    }
    out
}

fn layer_names(d: &Document) -> Vec<String> {
    d.read_nodes()
        .into_iter()
        .filter(|n| matches!(n.kind, NodeKind::Layer))
        .map(|n| n.registers.name.unwrap_or_default())
        .collect()
}

fn leaves(d: &Document) -> Vec<(Option<String>, ObjectSnapshot, bool)> {
    d.read_nodes()
        .into_iter()
        .filter_map(|n| match n.kind {
            NodeKind::Leaf(o) => Some((n.registers.name.clone(), o, n.guide)),
            _ => None,
        })
        .collect()
}

fn leaf_named(d: &Document, name: &str) -> ObjectSnapshot {
    leaves(d)
        .into_iter()
        .find(|(n, _, _)| n.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("a leaf named {name}"))
        .1
}

fn node_id(d: &Document, name: &str) -> NodeId {
    d.read_nodes()
        .into_iter()
        .find(|n| n.registers.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("a node named {name}"))
        .id
}

fn assert_rotate(t: &str, want: [f64; 3]) {
    assert!(t.starts_with("rotate(") && t.ends_with(')'), "{t}");
    let n = numbers(&t[7..t.len() - 1]);
    assert_eq!(n.len(), 3, "{t}");
    for (g, w) in n.iter().zip(want) {
        assert!((g - w).abs() < 1e-9, "{t} vs {want:?}");
    }
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

// ---------------------------------------------------------------------------------------------
// Criterion 52: the skeleton
// ---------------------------------------------------------------------------------------------

#[test]
fn c52_a4_one_layer_rectangle_has_the_skeleton() {
    let d = doc(
        &a4(),
        &[layer("Cut", vec![rect("", 10.0, 20.0, 30.0, 40.0)])],
    );
    let d = {
        // a nameless rectangle: rebuild without the empty name
        let _ = d;
        doc(
            &a4(),
            &[layer(
                "Cut",
                vec![prim(
                    DrawingRegisters::default(),
                    rect_shape(10.0, 20.0, 30.0, 40.0, [0.0; 4]),
                    0.0,
                    hairline(),
                )],
            )],
        )
    };
    let text = export(&d);
    assert!(
        text.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg "),
        "{text}"
    );
    let (name, root) = &tags(&text)[0];
    assert_eq!(name, "svg");
    let root = attrs(root);
    let get = |n: &str| root.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
    assert_eq!(get("xmlns").as_deref(), Some("http://www.w3.org/2000/svg"));
    assert_eq!(
        get("xmlns:inkscape").as_deref(),
        Some("http://www.inkscape.org/namespaces/inkscape")
    );
    assert_eq!(
        get("xmlns:sodipodi").as_deref(),
        Some("http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd")
    );
    assert_eq!(get("xmlns:curvyo").as_deref(), Some("urn:curvyo:svg:1"));
    assert_eq!(get("width").as_deref(), Some("210mm"));
    assert_eq!(get("height").as_deref(), Some("297mm"));
    assert_eq!(get("viewBox").as_deref(), Some("0 0 210 297"));
    assert_eq!(get("curvyo:format").as_deref(), Some("1"));
    assert_eq!(
        get("xmlns:xlink"),
        None,
        "no use, image or textPath: no xlink"
    );
    let nv = of_kind(&text, "sodipodi:namedview");
    assert_eq!(nv.len(), 1);
    assert_eq!(attr(&nv[0], "id").as_deref(), Some("namedview1"));
    assert_eq!(
        attr(&nv[0], "inkscape:document-units").as_deref(),
        Some("mm")
    );
    let bg = d.background();
    let want_opacity = if bg.paint == curvyo_document_core::BackgroundPaint::None {
        "0".to_owned()
    } else {
        format!("{}", bg.opacity.get())
    };
    assert_eq!(
        attr(&nv[0], "inkscape:pageopacity"),
        Some(want_opacity),
        "pageopacity is the background's ({bg:?})"
    );
    let (_, l) = by_id(&text, "layer1");
    assert_eq!(attr(&l, "inkscape:groupmode").as_deref(), Some("layer"));
    assert_eq!(attr(&l, "inkscape:label").as_deref(), Some("Cut"));
    let (k, r) = by_id(&text, "rect1");
    assert_eq!(k, "rect");
    for (n, v) in [
        ("x", "10"),
        ("y", "20"),
        ("width", "30"),
        ("height", "40"),
        ("fill", "none"),
        ("stroke", "#000000"),
        ("stroke-width", "0.25"),
    ] {
        assert_eq!(attr(&r, n).as_deref(), Some(v), "rect {n}\n{text}");
    }
    assert!(
        !text.contains(" style=\""),
        "presentation attributes only (criterion 56)\n{text}"
    );
    // The background is never a rectangle: one rect only.
    assert_eq!(of_kind(&text, "rect").len(), 1);
    assert!(curvyo_svg_core::read_svg(text.as_bytes()).is_ok());
}

#[test]
fn c52_background_is_pagecolor_not_a_rectangle_and_display_unit_is_written() {
    let mut page = a4();
    page.display_unit = Some(DisplayUnit::In);
    page.background = Some(curvyo_document_core::DocumentBackground {
        paint: curvyo_document_core::BackgroundPaint::Solid,
        color: Color {
            r: 0x12,
            g: 0xab,
            b: 0xef,
        },
        opacity: Opacity::new(0.5).unwrap(),
    });
    let text = export(&doc(&page, &[layer("L", vec![])]));
    let nv = &of_kind(&text, "sodipodi:namedview")[0];
    assert_eq!(attr(nv, "inkscape:document-units").as_deref(), Some("in"));
    assert_eq!(attr(nv, "pagecolor").as_deref(), Some("#12abef"));
    assert_eq!(attr(nv, "inkscape:pageopacity").as_deref(), Some("0.5"));
    assert_eq!(
        drawn(&text),
        0,
        "the background is not a drawn element\n{text}"
    );
}

#[test]
fn c52_a_page_moved_gives_the_same_file() {
    let at = |x: f64| {
        let page = pages(&[("Page 1", x, 0.0, 100.0, 50.0)]);
        doc(
            &page,
            &[layer(
                "L",
                vec![
                    rect("R", x + 10.0, 5.0, 20.0, 10.0),
                    rect("Off", x - 30.0, 60.0, 5.0, 5.0),
                ],
            )],
        )
    };
    let a = export(&at(0.0));
    let b = export(&at(200.0));
    assert_eq!(a, b, "the export origin is the page's top-left corner");
    let (_, off) = by_id(&a, "rect2");
    assert_eq!(
        attr(&off, "x").as_deref(),
        Some("-30"),
        "pasteboard objects unclipped, negative coordinates"
    );
    assert_eq!(attr(&off, "y").as_deref(), Some("60"));
    assert!(
        !a.contains("inkscape:page "),
        "one page writes no inkscape:page"
    );
}

#[test]
fn c4_empty_document_is_a_valid_svg_with_no_drawing_elements() {
    let d = doc(&a4(), &[]);
    let out = export_full(&d, SvgExportOptions::default(), &[], 0);
    assert_eq!(out.objects, 0, "the notice says the drawing is empty");
    assert!(
        curvyo_svg_core::read_svg(out.text.as_bytes()).is_ok(),
        "{}",
        out.text
    );
    assert_eq!(drawn(&out.text), 0);
    assert!(out.text.contains("viewBox=\"0 0 210 297\""));
    // An empty layer is written empty (criterion 53).
    let d = doc(&a4(), &[layer("Empty", vec![])]);
    let text = export(&d);
    assert_eq!(
        attr(&by_id(&text, "layer1").1, "inkscape:label").as_deref(),
        Some("Empty")
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 53: tree, names, ids
// ---------------------------------------------------------------------------------------------

#[test]
fn c53_layers_hidden_locked_opacity_groups_and_loose_objects() {
    let hidden = DrawingNode::Group {
        registers: DrawingRegisters {
            name: Some("Hidden".into()),
            visible: false,
            ..DrawingRegisters::default()
        },
        layer: true,
        children: vec![rect("h", 0.0, 0.0, 1.0, 1.0)],
    };
    let locked = DrawingNode::Group {
        registers: DrawingRegisters {
            name: Some("Locked".into()),
            locked: true,
            opacity: Opacity::new(0.25).unwrap(),
            ..DrawingRegisters::default()
        },
        layer: true,
        children: vec![
            group(
                Some("Outer"),
                vec![group(
                    None,
                    vec![group(Some("Deep"), vec![rect("d", 1.0, 1.0, 1.0, 1.0)])],
                )],
            ),
            prim(
                DrawingRegisters {
                    name: Some("lockedleaf".into()),
                    locked: true,
                    ..DrawingRegisters::default()
                },
                rect_shape(2.0, 2.0, 1.0, 1.0, [0.0; 4]),
                0.0,
                hairline(),
            ),
        ],
    };
    let loose = rect("loose", 5.0, 5.0, 1.0, 1.0);
    let d = doc(&a4(), &[hidden, locked, loose]);
    let text = export(&d);
    let (_, l1) = by_id(&text, "layer1");
    assert_eq!(attr(&l1, "inkscape:label").as_deref(), Some("Hidden"));
    assert_eq!(attr(&l1, "style").as_deref(), Some("display:none"));
    let (_, l2) = by_id(&text, "layer2");
    assert_eq!(attr(&l2, "sodipodi:insensitive").as_deref(), Some("true"));
    assert_eq!(attr(&l2, "opacity").as_deref(), Some("0.25"));
    assert_eq!(attr(&l2, "style"), None);
    let groups = of_kind(&text, "g");
    let plain: Vec<&String> = groups
        .iter()
        .filter(|g| attr(g, "inkscape:groupmode").is_none())
        .collect();
    assert_eq!(plain.len(), 3, "three nested groups\n{text}");
    assert_eq!(attr(plain[0], "id").as_deref(), Some("g1"));
    assert_eq!(attr(plain[0], "inkscape:label").as_deref(), Some("Outer"));
    assert_eq!(
        attr(plain[1], "inkscape:label"),
        None,
        "unnamed group: no label"
    );
    assert_eq!(attr(plain[2], "inkscape:label").as_deref(), Some("Deep"));
    assert!(
        groups.iter().all(|g| attr(g, "transform").is_none()),
        "no group has a transform"
    );
    let lk = tags(&text)
        .into_iter()
        .find(|(_, r)| attr(r, "inkscape:label").as_deref() == Some("lockedleaf"))
        .unwrap();
    assert_eq!(attr(&lk.1, "sodipodi:insensitive").as_deref(), Some("true"));
    // Loose top-level object directly under the root, after the layers (document order).
    let close_layer2 = text.rfind("</g>").unwrap();
    let loose_at = text.find("inkscape:label=\"loose\"").unwrap();
    assert!(
        loose_at > close_layer2,
        "the loose rectangle is last, under the root\n{text}"
    );
    let ids: Vec<String> = of_kind(&text, "rect")
        .iter()
        .map(|r| attr(r, "id").unwrap())
        .collect();
    assert_eq!(ids, ["rect1", "rect2", "rect3", "rect4"]);
}

#[test]
fn c53_names_are_escaped_and_lose_control_and_bidi_characters() {
    let nasty = "A & <B> \"q\" 'a'\u{202E}evil\u{2066}\u{0007}\u{200F}end";
    let page = pages(&[
        ("Page <1> & \"x\"\u{202D}", 0.0, 0.0, 100.0, 100.0),
        ("P2", 120.0, 0.0, 10.0, 10.0),
    ]);
    let d = doc(
        &page,
        &[layer(nasty, vec![rect(nasty, 1.0, 1.0, 2.0, 2.0)])],
    );
    let text = export(&d);
    for bad in ['\u{202E}', '\u{2066}', '\u{0007}', '\u{200F}', '\u{202D}'] {
        assert!(!text.contains(bad), "{bad:?} removed");
    }
    assert!(!text.contains("<B>"), "no raw markup from a name");
    assert!(text.contains("&amp;"));
    assert!(text.contains("&lt;B&gt;"));
    let parsed = curvyo_svg_core::read_svg(text.as_bytes()).expect("well-formed");
    let _ = parsed;
    let back = open(&text);
    let want = "A & <B> \"q\" 'a'evilend";
    assert_eq!(layer_names(&back), [want]);
    assert!(
        leaves(&back)
            .iter()
            .any(|(n, _, _)| n.as_deref() == Some(want))
    );
    let p = back.pages();
    assert_eq!(p[0].name, "Page <1> & \"x\"");
}

#[test]
fn c53_ids_per_kind_and_no_internal_ids() {
    let d = doc(
        &a4(),
        &[layer(
            "L",
            vec![
                line("p", (0.0, 0.0), (1.0, 1.0), hairline()),
                path(
                    reg("c"),
                    vec![
                        (
                            true,
                            vec![corner(0.0, 0.0), corner(5.0, 0.0), corner(5.0, 5.0)],
                        ),
                        (
                            true,
                            vec![corner(1.0, 1.0), corner(2.0, 1.0), corner(2.0, 2.0)],
                        ),
                    ],
                    filled(1, 2, 3),
                ),
                prim(
                    reg("e"),
                    ellipse_shape(50.0, 50.0, 10.0, 5.0, None),
                    0.0,
                    hairline(),
                ),
                prim(
                    reg("ci"),
                    ellipse_shape(50.0, 50.0, 10.0, 10.0, None),
                    0.0,
                    hairline(),
                ),
                prim(
                    reg("po"),
                    star_shape(80.0, 80.0, 10.0, 0.0, 6, None, 0.0),
                    0.0,
                    hairline(),
                ),
                prim(
                    reg("st"),
                    star_shape(80.0, 80.0, 10.0, 0.0, 5, Some(0.5), 0.0),
                    0.0,
                    hairline(),
                ),
                line("p2", (0.0, 0.0), (2.0, 1.0), hairline()),
            ],
        )],
    );
    let text = export(&d);
    for id in [
        "layer1",
        "path1",
        "compound1",
        "ellipse1",
        "circle1",
        "polygon1",
        "star1",
        "path2",
    ] {
        by_id(&text, id);
    }
    for n in d.read_nodes() {
        let dbg = format!("{:?}", n.id);
        assert!(!text.contains(&dbg), "internal id {dbg} leaks");
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 54 and 57: path data and numbers
// ---------------------------------------------------------------------------------------------

#[test]
fn c54_path_data_absolute_mlcz_with_nodetypes() {
    let sym = DrawingAnchor {
        point: Point::new(10.0, 20.0),
        handle_in: Vec2::new(-1.5, 0.0),
        handle_out: Vec2::new(1.5, 0.0),
        kind: AnchorKind::Symmetric,
    };
    let asym = DrawingAnchor {
        point: Point::new(14.0, 22.0),
        handle_in: Vec2::new(-1.0, -0.8),
        handle_out: Vec2::new(3.0, 0.0),
        kind: AnchorKind::Asymmetric,
    };
    let d = doc(
        &a4(),
        &[layer(
            "L",
            vec![path(
                reg("p"),
                vec![
                    (true, vec![sym, asym, corner(20.0, 30.0)]),
                    (false, vec![corner(40.0, 40.0), corner(50.0, 40.0)]),
                ],
                filled(0, 0, 0),
            )],
        )],
    );
    let text = export(&d);
    let (k, p) = by_id(&text, "compound1");
    assert_eq!(k, "path");
    let dd = attr(&p, "d").unwrap();
    assert!(dd.starts_with("M 10 20 C 11.5 20 13 21.2 14 22"), "{dd}");
    for c in dd.split_whitespace().filter(|t| t.parse::<f64>().is_err()) {
        assert!(["M", "L", "C", "Z"].contains(&c), "command {c} in {dd}");
    }
    assert_eq!(
        dd.matches('Z').count(),
        1,
        "Z after the closed outline only: {dd}"
    );
    assert!(
        dd.trim_end().ends_with("M 40 40 L 50 40"),
        "open outline last, no Z: {dd}"
    );
    assert!(!dd.contains("  "), "one space");
    let nt = attr(&p, "sodipodi:nodetypes").unwrap();
    assert!(
        nt.starts_with("zsc"),
        "Symmetric z, Asymmetric s, Corner c: {nt}"
    );
    assert!(nt.ends_with("cc"), "{nt}");
    let points = dd.matches(['M', 'L']).count() + dd.matches('C').count();
    assert_eq!(nt.len(), points, "one letter per node of d: {nt} {dd}");
    assert_eq!(attr(&p, "transform"), None);
    assert_eq!(attr(&p, "fill-rule").as_deref(), Some("nonzero"));
}

#[test]
fn c54_c57_number_format_six_decimals_no_exponent_no_minus_zero() {
    let third = 1.0 / 3.0;
    let d = doc(
        &a4(),
        &[layer(
            "L",
            vec![
                line("a", (third, -0.0), (2.0 / 3.0, -1e-9), hairline()),
                line(
                    "b",
                    (9_999_999.5, -9_999_999.25),
                    (1e-7, 123456.0000004),
                    hairline(),
                ),
                prim(
                    reg("r"),
                    rect_shape(-0.0, -0.0000001, 1e6, 0.1 + 0.2, [0.0; 4]),
                    0.0,
                    hairline(),
                ),
            ],
        )],
    );
    let text = export(&d);
    for t in number_tokens(&text) {
        assert!(!t.contains('e') && !t.contains('E'), "exponent in {t}");
        assert_ne!(t, "-0", "never -0\n{text}");
        assert!(
            !t.starts_with("-0") || t.parse::<f64>().unwrap() != 0.0,
            "{t}"
        );
        if let Some((_, frac)) = t.split_once('.') {
            assert!(!frac.ends_with('0'), "trailing zero in {t}");
        }
    }
    let (_, a) = by_id(&text, "path1");
    assert_eq!(attr(&a, "d").as_deref(), Some("M 0.333333 0 L 0.666667 0"));
    let (_, b) = by_id(&text, "path2");
    assert_eq!(
        attr(&b, "d").as_deref(),
        Some("M 9999999.5 -9999999.25 L 0 123456")
    );
    let (_, r) = by_id(&text, "rect1");
    assert_eq!(attr(&r, "x").as_deref(), Some("0"));
    assert_eq!(attr(&r, "y").as_deref(), Some("0"));
    assert_eq!(attr(&r, "width").as_deref(), Some("1000000"));
    assert_eq!(attr(&r, "height").as_deref(), Some("0.3"));
}

#[test]
fn c57_shadow_numbers_are_shortest_round_trip_text() {
    let mut s = hairline();
    s.stroke.width = Length::from_mm(0.1 + 0.2); // 0.30000000000000004
    s.stroke.dash = DashPattern::new(vec![1.0 / 3.0, 2.0]).unwrap();
    let d = doc(
        &a4(),
        &[layer("L", vec![line("dash", (0.0, 0.0), (100.0, 0.0), s)])],
    );
    let text = export(&d);
    let (_, p) = by_id(&text, "path1");
    assert_eq!(attr(&p, "stroke-width").as_deref(), Some("0.3"));
    assert_eq!(
        attr(&p, "curvyo:stroke-width").as_deref(),
        Some("0.30000000000000004")
    );
    let dash = attr(&p, "curvyo:dash").unwrap();
    let parts = numbers(&dash);
    assert_eq!(
        parts,
        [1.0 / 3.0, 2.0],
        "exact dash list in multiples of the width: {dash}"
    );
    let back = open(&text);
    match leaf_named(&back, "dash") {
        ObjectSnapshot::Path(p) => {
            assert_eq!(p.style.stroke.width.as_mm(), 0.1 + 0.2);
            assert_eq!(p.style.stroke.dash.as_slice(), [1.0 / 3.0, 2.0]);
        }
        other => panic!("{other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 55: primitives
// ---------------------------------------------------------------------------------------------

#[test]
fn c55_primitives_rect_ellipse_circle_star_and_rounded_rect_path() {
    let d = doc(
        &a4(),
        &[layer(
            "L",
            vec![
                prim(
                    reg("r"),
                    rect_shape(10.0, 10.0, 40.0, 20.0, [2.0; 4]),
                    30.0,
                    hairline(),
                ),
                prim(
                    reg("e"),
                    ellipse_shape(100.0, 100.0, 20.0, 10.0, None),
                    -45.0,
                    hairline(),
                ),
                prim(
                    reg("c"),
                    ellipse_shape(50.0, 150.0, 7.0, 7.0, None),
                    0.0,
                    hairline(),
                ),
                prim(
                    reg("s"),
                    star_shape(150.0, 50.0, 20.0, 30.0, 5, Some(0.4), 0.0),
                    0.0,
                    hairline(),
                ),
                prim(
                    reg("rr"),
                    rect_shape(10.0, 200.0, 40.0, 20.0, [1.0, 2.0, 3.0, 4.0]),
                    0.0,
                    hairline(),
                ),
            ],
        )],
    );
    let text = export(&d);
    let (k, r) = by_id(&text, "rect1");
    assert_eq!(k, "rect");
    assert_eq!(attr(&r, "rx").as_deref(), Some("2"));
    assert_eq!(attr(&r, "ry").as_deref(), Some("2"));
    assert_rotate(&attr(&r, "transform").unwrap(), [30.0, 30.0, 20.0]);
    assert_eq!(attr(&r, "x").as_deref(), Some("10"), "unrotated frame");
    let (k, e) = by_id(&text, "ellipse1");
    assert_eq!(k, "ellipse");
    assert_eq!(attr(&e, "rx").as_deref(), Some("20"));
    assert_rotate(&attr(&e, "transform").unwrap(), [-45.0, 100.0, 100.0]);
    let (k, c) = by_id(&text, "circle1");
    assert_eq!(k, "circle");
    assert_eq!(attr(&c, "r").as_deref(), Some("7"));
    let (k, s) = by_id(&text, "star1");
    assert_eq!(k, "path");
    for n in [
        "sodipodi:sides",
        "sodipodi:r1",
        "sodipodi:r2",
        "sodipodi:arg1",
        "sodipodi:arg2",
        "sodipodi:cx",
        "sodipodi:cy",
        "inkscape:flatsided",
        "d",
    ] {
        assert!(attr(&s, n).is_some(), "star {n}\n{s}");
    }
    assert_eq!(attr(&s, "sodipodi:type").as_deref(), Some("star"));
    assert_eq!(attr(&s, "sodipodi:sides").as_deref(), Some("5"));
    assert_eq!(attr(&s, "inkscape:flatsided").as_deref(), Some("false"));
    assert_eq!(attr(&s, "inkscape:rounded").as_deref(), Some("0"));
    assert_eq!(attr(&s, "inkscape:randomized").as_deref(), Some("0"));
    let (k, rr) = by_id(&text, "rect2");
    assert_eq!(
        k, "path",
        "a rectangle with four radii is a path with rect id"
    );
    assert_eq!(attr(&rr, "curvyo:kind").as_deref(), Some("rectangle"));
    assert_eq!(
        numbers(&attr(&rr, "curvyo:radii").unwrap()),
        [1.0, 2.0, 3.0, 4.0]
    );
    // Round trip: same kinds and values.
    let back = open(&text);
    for name in ["r", "e", "c", "s", "rr"] {
        let a = leaf_named(&d, name);
        let b = leaf_named(&back, name);
        match (a, b) {
            (ObjectSnapshot::Primitive(a), ObjectSnapshot::Primitive(b)) => {
                assert_eq!(
                    std::mem::discriminant(&a.shape),
                    std::mem::discriminant(&b.shape),
                    "{name}"
                );
                assert!(
                    close(a.rotation.as_radians(), b.rotation.as_radians(), 1e-9),
                    "{name} rotation"
                );
                assert_eq!(
                    format!("{:.6?}", a.shape),
                    format!("{:.6?}", b.shape),
                    "{name}"
                );
            }
            (a, b) => panic!("{name}: {a:?} vs {b:?}"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 56: style
// ---------------------------------------------------------------------------------------------

#[test]
fn c56_style_presentation_attributes_and_stroke_off() {
    let mut dashed = filled(255, 0, 16);
    dashed.fill.opacity = Opacity::new(0.5).unwrap();
    dashed.stroke.width = Length::from_mm(2.0);
    dashed.stroke.color = Color {
        r: 0,
        g: 128,
        b: 255,
    };
    dashed.stroke.opacity = Opacity::new(0.75).unwrap();
    dashed.stroke.cap = LineCap::Round;
    dashed.stroke.join = LineJoin::Bevel;
    dashed.stroke.dash = DashPattern::new(vec![1.0, 0.0, 2.5]).unwrap();
    let mut off = filled(1, 1, 1);
    off.stroke.enabled = false;
    let mut nofill = hairline();
    nofill.fill.enabled = false;
    let d = doc(
        &a4(),
        &[layer(
            "L",
            vec![
                path(
                    DrawingRegisters {
                        name: Some("dashed".into()),
                        opacity: Opacity::new(0.4).unwrap(),
                        ..DrawingRegisters::default()
                    },
                    vec![(
                        true,
                        vec![corner(0.0, 0.0), corner(10.0, 0.0), corner(10.0, 10.0)],
                    )],
                    dashed,
                ),
                line("off", (0.0, 0.0), (1.0, 0.0), off),
                line("nofill", (0.0, 0.0), (1.0, 0.0), nofill),
            ],
        )],
    );
    let text = export(&d);
    let (_, p) = by_id(&text, "path1");
    let a = attrs(&p);
    let get = |n: &str| a.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
    assert_eq!(get("fill").as_deref(), Some("#ff0010"));
    assert_eq!(get("fill-opacity").as_deref(), Some("0.5"));
    assert_eq!(get("stroke").as_deref(), Some("#0080ff"));
    assert_eq!(get("stroke-width").as_deref(), Some("2"));
    assert_eq!(get("stroke-opacity").as_deref(), Some("0.75"));
    assert_eq!(get("stroke-linecap").as_deref(), Some("round"));
    assert_eq!(get("stroke-linejoin").as_deref(), Some("bevel"));
    assert_eq!(
        get("stroke-dasharray").as_deref(),
        Some("2 0 5"),
        "times the width; zero as 0"
    );
    assert_eq!(get("fill-rule").as_deref(), Some("nonzero"));
    assert_eq!(get("opacity").as_deref(), Some("0.4"));
    assert_eq!(get("style"), None);
    // Fixed order: fill before stroke attributes.
    let order: Vec<&str> = a.iter().map(|(k, _)| k.as_str()).collect();
    let pos = |n: &str| order.iter().position(|k| *k == n).unwrap();
    assert!(pos("fill") < pos("fill-opacity") && pos("fill-opacity") < pos("stroke"));
    assert!(pos("stroke") < pos("stroke-width") && pos("stroke-width") < pos("stroke-dasharray"));
    let (_, o) = by_id(&text, "path2");
    assert_eq!(attr(&o, "stroke").as_deref(), Some("none"));
    assert!(
        attrs(&o).iter().all(|(k, _)| !k.starts_with("stroke-")
            && !k.starts_with("curvyo:dash")
            && k != "curvyo:stroke-width"),
        "stroke off: no other stroke attribute\n{o}"
    );
    let (_, nf) = by_id(&text, "path3");
    assert_eq!(attr(&nf, "fill").as_deref(), Some("none"));
    assert_eq!(
        attr(&nf, "fill-rule"),
        None,
        "fill-rule on filled leaves only"
    );
    // Solid stroke: dasharray none.
    // Criterion 56 says "none when solid"; the skeleton of criterion 52 omits it. Both accepted
    // (spec ambiguity, reported).
    assert!(matches!(
        attr(&nf, "stroke-dasharray").as_deref(),
        None | Some("none")
    ));
}

// ---------------------------------------------------------------------------------------------
// Criterion 59: clones
// ---------------------------------------------------------------------------------------------

#[test]
fn c59_clones_use_for_isometries_path_otherwise() {
    let p = line("P", (0.0, 0.0), (10.0, 5.0), filled(9, 9, 9));
    let hidden = path(
        DrawingRegisters {
            name: Some("H".into()),
            visible: false,
            ..DrawingRegisters::default()
        },
        vec![(false, vec![corner(0.0, 50.0), corner(10.0, 55.0)])],
        hairline(),
    );
    let c = |name: &str, original: usize, m: Affine| DrawingNode::Clone {
        registers: reg(name),
        original,
        matrix: m,
    };
    let rot = Affine::rotation_about(
        Point::new(0.0, 0.0),
        Angle::from_radians(30f64.to_radians()),
    );
    let flip_rot = rot.compose(Affine::scale(-1.0, 1.0));
    let nodes = vec![layer(
        "L",
        vec![
            p,
            c("moved", 0, Affine::translation(Vec2::new(30.0, 0.0))),
            c("turned", 0, flip_rot),
            c("chain", 2, Affine::translation(Vec2::new(0.0, 20.0))),
            c("scaled", 0, Affine::scale(2.0, 2.0)),
            hidden,
            c("of hidden", 5, Affine::translation(Vec2::new(5.0, 0.0))),
        ],
    )];
    // Leaf indices: DrawingNode::Clone.original counts leaves in document order (see from_drawing).
    let d = doc(&a4(), &nodes);
    let text = export(&d);
    assert!(
        text.contains("xmlns:xlink=\"http://www.w3.org/1999/xlink\""),
        "xlink declared with a use"
    );
    let uses = of_kind(&text, "use");
    let labels: Vec<String> = uses
        .iter()
        .map(|u| attr(u, "inkscape:label").unwrap_or_default())
        .collect();
    assert_eq!(
        labels,
        ["moved", "turned", "chain"],
        "isometries without override are use\n{text}"
    );
    for u in &uses {
        assert!(attr(u, "xlink:href").unwrap().starts_with('#'));
        for bad in ["fill", "stroke", "opacity", "style"] {
            assert!(attr(u, bad).is_none(), "use carries no {bad}");
        }
    }
    assert_eq!(
        attr(&uses[0], "transform").as_deref(),
        Some("matrix(1 0 0 1 30 0)")
    );
    let target = attr(&uses[2], "xlink:href").unwrap();
    assert_eq!(
        by_id(&text, &target[1..]).0,
        "use",
        "a chain refers to the clone it follows"
    );
    for name in ["scaled", "of hidden"] {
        let t = tags(&text)
            .into_iter()
            .find(|(_, r)| attr(r, "inkscape:label").as_deref() == Some(name))
            .unwrap();
        assert_eq!(t.0, "path", "{name} is a path");
    }
    // Import gives back clones of the same originals with the same matrices (1e-9).
    let back = open(&text);
    for (name, want) in [
        ("moved", Affine::translation(Vec2::new(30.0, 0.0))),
        ("turned", flip_rot),
    ] {
        match leaf_named(&back, name) {
            ObjectSnapshot::Clone(cl) => {
                let got = cl.chain.matrix.to_array();
                for (g, w) in got.iter().zip(want.to_array()) {
                    assert!(
                        close(*g, w, 1e-9),
                        "{name}: {got:?} vs {:?}",
                        want.to_array()
                    );
                }
                assert_eq!(cl.chain.origin, node_id(&back, "P"));
            }
            other => panic!("{name} came back as {other:?}"),
        }
    }
    match leaf_named(&back, "chain") {
        ObjectSnapshot::Clone(cl) => assert_eq!(cl.chain.origin, node_id(&back, "turned")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn c59_clone_of_original_in_hidden_layer_or_guide_is_a_path() {
    let hidden_layer = DrawingNode::Group {
        registers: DrawingRegisters {
            name: Some("HL".into()),
            visible: false,
            ..DrawingRegisters::default()
        },
        layer: true,
        children: vec![line("orig", (0.0, 0.0), (5.0, 5.0), hairline())],
    };
    let guide = path(
        DrawingRegisters {
            name: Some("guide".into()),
            guide: true,
            ..DrawingRegisters::default()
        },
        vec![(false, vec![corner(0.0, 10.0), corner(5.0, 15.0)])],
        hairline(),
    );
    let nodes = vec![
        hidden_layer,
        layer(
            "V",
            vec![
                guide,
                DrawingNode::Clone {
                    registers: reg("c1"),
                    original: 0,
                    matrix: Affine::translation(Vec2::new(10.0, 0.0)),
                },
                DrawingNode::Clone {
                    registers: reg("c2"),
                    original: 1,
                    matrix: Affine::translation(Vec2::new(10.0, 0.0)),
                },
            ],
        ),
    ];
    let text = export(&doc(&a4(), &nodes));
    assert!(
        of_kind(&text, "use").is_empty(),
        "originals hidden by ancestors or guides: paths\n{text}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 60 and 72: privacy and determinism
// ---------------------------------------------------------------------------------------------

#[test]
fn c60_c72_two_exports_are_byte_identical_and_hold_nothing_personal() {
    let d = everything_doc();
    let a = export(&d);
    let b = export(&d);
    assert_eq!(a, b);
    let d2 = everything_doc();
    assert_eq!(
        export(&d2),
        a,
        "a second document built the same way gives the same bytes"
    );
    for bad in [
        "/home",
        "\\Users",
        env!("CARGO_PKG_VERSION"),
        "2026",
        "curvyo-ui-core",
    ] {
        assert!(!a.contains(bad), "{bad} in the file");
    }
}

#[test]
fn c72_locale_changes_nothing() {
    // Child process with LC_ALL and LANG set to German: the bytes are the same.
    if let Ok(path) = std::env::var("CURVYO_TESTER_DUMP") {
        std::fs::write(path, export(&everything_doc())).unwrap();
        return;
    }
    let dir = std::env::temp_dir().join(format!("curvyo-0024-tester-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("de.svg");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "c72_locale_changes_nothing", "--nocapture"])
        .env("CURVYO_TESTER_DUMP", &out)
        .env("LC_ALL", "de_DE.UTF-8")
        .env("LANG", "de_DE.UTF-8")
        .env("LC_NUMERIC", "de_DE.UTF-8")
        .status()
        .unwrap();
    assert!(status.success());
    let german = std::fs::read_to_string(&out).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(german, export(&everything_doc()));
    assert!(!german.contains("0,25"));
}

// ---------------------------------------------------------------------------------------------
// Criteria 62, 63, 58: our own data round trip
// ---------------------------------------------------------------------------------------------

fn style_kind() -> Vec<Style> {
    let mut v = Vec::new();
    for (i, (cap, join)) in [
        (LineCap::Butt, LineJoin::Miter),
        (LineCap::Round, LineJoin::Round),
        (LineCap::Square, LineJoin::Bevel),
    ]
    .into_iter()
    .enumerate()
    {
        let mut s = filled(10 * i as u8, 200, 30);
        s.stroke.cap = cap;
        s.stroke.join = join;
        s.stroke.width = Length::from_mm(0.3 + i as f64 * 0.7);
        s.stroke.dash = DashPattern::new(match i {
            0 => vec![],
            1 => vec![1.0, 2.0, 3.0],
            _ => vec![0.5, 0.25, 4.0, 1.0, 2.0],
        })
        .unwrap();
        s.stroke.opacity = Opacity::new(0.6).unwrap();
        v.push(s);
    }
    let mut off = filled(1, 2, 3);
    off.stroke.enabled = false;
    v.push(off);
    let mut nofill = hairline();
    nofill.fill.enabled = false;
    v.push(nofill);
    v
}

fn all_markers(place: MarkerPlace, count: u32) -> Style {
    let mut s = hairline();
    s.stroke.width = Length::from_mm(0.5);
    s.stroke.markers = Markers {
        start: MarkerShape::Dot,
        mid: MarkerShape::Arrow,
        end: MarkerShape::Arrow,
        mid_place: place,
        mid_count: MarkerCount::new(count).unwrap(),
    };
    s
}

/// Criterion 62's generated document (deterministic part) plus the slice 4 kinds that a
/// `DrawingNode` can hold (pages, arcs, curved shapes, markers, guides, images).
fn everything_nodes() -> Vec<DrawingNode> {
    let styles = style_kind();
    let compound = path(
        reg("compound"),
        vec![
            (
                true,
                vec![
                    corner(0.0, 0.0),
                    corner(50.0, 0.0),
                    corner(50.0, 50.0),
                    corner(0.0, 50.0),
                ],
            ),
            (
                true,
                vec![
                    corner(10.0, 10.0),
                    corner(10.0, 40.0),
                    corner(40.0, 40.0),
                    corner(40.0, 10.0),
                ],
            ),
            (
                false,
                vec![corner(60.0, 0.0), corner(70.0, 10.0), corner(60.0, 20.0)],
            ),
            (false, vec![corner(55.0, 0.0), corner(75.0, 20.0)]),
            (false, vec![corner(55.0, 20.0), corner(75.0, 0.0)]),
        ],
        styles[0].clone(),
    );
    let mut stars = Vec::new();
    for (i, deg) in [0.0, 30.0, -45.0].into_iter().enumerate() {
        stars.push(prim(
            reg(&format!("star{deg}")),
            star_shape(
                100.0 + 30.0 * i as f64,
                200.0,
                12.0,
                15.0,
                5 + i as u32,
                Some(0.45),
                0.0,
            ),
            deg,
            styles[i].clone(),
        ));
    }
    let arcs: Vec<DrawingNode> = [ArcKind::Pie, ArcKind::Arc, ArcKind::Chord]
        .into_iter()
        .enumerate()
        .map(|(i, kind)| {
            let arc =
                EllipseArc::new(Angle::from_radians(0.25), Angle::from_radians(2.5), kind).unwrap();
            prim(
                reg(&format!("arc{i}")),
                ellipse_shape(30.0 + 40.0 * i as f64, 250.0, 15.0, 9.0, Some(arc)),
                if i == 2 { 20.0 } else { 0.0 },
                styles[i].clone(),
            )
        })
        .collect();
    let guide = |name: &str, x: f64| {
        prim(
            DrawingRegisters {
                name: Some(name.into()),
                guide: true,
                ..DrawingRegisters::default()
            },
            ellipse_shape(x, 280.0, 5.0, 5.0, None),
            0.0,
            hairline(),
        )
    };
    vec![
        layer(
            "Cut & <go>",
            vec![
                compound,
                prim(
                    reg("rr"),
                    rect_shape(100.0, 10.0, 40.0, 20.0, [1.0, 2.0, 3.0, 4.0]),
                    33.0,
                    styles[1].clone(),
                ),
                prim(
                    reg("ellipse"),
                    ellipse_shape(150.0, 80.0, 20.0, 10.0, None),
                    12.5,
                    styles[2].clone(),
                ),
                prim(
                    reg("circle"),
                    ellipse_shape(150.0, 120.0, 8.0, 8.0, None),
                    0.0,
                    styles[3].clone(),
                ),
                prim(
                    reg("polygon"),
                    star_shape(60.0, 150.0, 10.0, 0.0, 7, None, 0.0),
                    0.0,
                    styles[4].clone(),
                ),
                group(
                    Some("g1"),
                    vec![group(
                        Some("g2"),
                        vec![group(
                            Some("g3"),
                            vec![line("deep", (1.0, 1.0), (2.0, 3.0), styles[1].clone())],
                        )],
                    )],
                ),
                guide("guide in cut", 20.0),
            ],
        ),
        DrawingNode::Group {
            registers: DrawingRegisters {
                name: Some("Hidden locked".into()),
                visible: false,
                locked: true,
                opacity: Opacity::new(0.3).unwrap(),
                guide: false,
            },
            layer: true,
            children: stars,
        },
        layer("Arcs", arcs),
        layer(
            "Curved",
            vec![
                prim(
                    reg("hex30"),
                    star_shape(40.0, 100.0, 10.0, 10.0, 6, None, 0.3),
                    0.0,
                    hairline(),
                ),
                prim(
                    reg("star curved"),
                    star_shape(80.0, 100.0, 10.0, -20.0, 5, Some(0.5), 0.7),
                    25.0,
                    hairline(),
                ),
            ],
        ),
        layer(
            "Markers",
            vec![
                path(
                    reg("nodes"),
                    vec![(
                        false,
                        vec![corner(0.0, 0.0), corner(10.0, 0.0), corner(10.0, 10.0)],
                    )],
                    all_markers(MarkerPlace::AtNodes, 1),
                ),
                path(
                    reg("spaced"),
                    vec![(false, vec![corner(0.0, 20.0), corner(30.0, 20.0)])],
                    all_markers(MarkerPlace::Spaced, 3),
                ),
                path(
                    reg("closed marked"),
                    vec![(
                        true,
                        vec![corner(0.0, 30.0), corner(10.0, 30.0), corner(10.0, 40.0)],
                    )],
                    all_markers(MarkerPlace::Spaced, 2),
                ),
            ],
        ),
        layer("Only guides", vec![guide("guide alone", 40.0)]),
    ]
}

fn everything_doc() -> Document {
    let page = pages(&[
        ("Front", -50.0, 20.0, 210.0, 297.0),
        ("Back", 200.0, 20.0, 100.0, 50.0),
        ("Third", 0.0, 400.0, 30.0, 60.0),
    ]);
    let mut page = page;
    page.display_unit = Some(DisplayUnit::Cm);
    page.background = Some(curvyo_document_core::DocumentBackground {
        paint: curvyo_document_core::BackgroundPaint::Solid,
        color: Color {
            r: 250,
            g: 240,
            b: 230,
        },
        opacity: Opacity::new(0.5).unwrap(),
    });
    doc(&page, &everything_nodes())
}

/// The document as compared by criteria 62 and 150: per node (in document order, layers named
/// "Guides" collected) the exact part and the numbers.
#[derive(Debug, Clone)]
struct Item {
    exact: String,
    numbers: Vec<f64>,
}

fn style_exact(s: &Style) -> String {
    let mut s = s.clone();
    if !s.stroke.enabled {
        s.stroke = Style::default().stroke;
        s.stroke.enabled = false;
    }
    if !s.fill.enabled {
        s.fill = Style::default().fill;
        s.fill.enabled = false;
    }
    format!(
        "fill={:?} stroke=({} {:?} {} {:?} {:?} {:?} {:?})",
        s.fill,
        s.stroke.enabled,
        s.stroke.color,
        s.stroke.opacity.get(),
        s.stroke.dash.as_slice(),
        s.stroke.cap,
        s.stroke.join,
        s.stroke.markers
    )
}

fn items(d: &Document) -> Vec<Item> {
    let nodes = d.read_nodes();
    let mut out = Vec::new();
    for n in &nodes {
        let r = &n.registers;
        let mut exact = format!(
            "d{} {:?} v={} l={} o={} guide={}",
            n.depth,
            r.name,
            r.visible,
            r.locked,
            r.opacity.get(),
            n.guide
        );
        let mut nums = Vec::new();
        match &n.kind {
            NodeKind::Layer => exact.push_str(" layer"),
            NodeKind::Group => exact.push_str(" group"),
            NodeKind::Leaf(ObjectSnapshot::Path(p)) => {
                exact.push_str(" path ");
                exact.push_str(&style_exact(&p.style));
                nums.push(p.style.stroke.width.as_mm());
                for s in p.subpaths() {
                    exact.push_str(if s.closed { " closed" } else { " open" });
                    for a in s.anchors {
                        exact.push_str(&format!(" {:?}", a.kind));
                        nums.extend([
                            a.point.x,
                            a.point.y,
                            a.handle_in.x,
                            a.handle_in.y,
                            a.handle_out.x,
                            a.handle_out.y,
                        ]);
                    }
                }
            }
            NodeKind::Leaf(ObjectSnapshot::Primitive(p)) => {
                exact.push_str(" primitive ");
                exact.push_str(&style_exact(&p.style));
                nums.push(p.style.stroke.width.as_mm());
                nums.push(p.rotation.as_radians());
                match p.shape {
                    Shape::Rect {
                        bounds,
                        corner_radii,
                    } => {
                        exact.push_str(" rect");
                        nums.extend([
                            bounds.origin.x,
                            bounds.origin.y,
                            bounds.width.as_mm(),
                            bounds.height.as_mm(),
                        ]);
                        exact.push_str(&format!(
                            " {:?}",
                            [
                                corner_radii.tl.as_mm(),
                                corner_radii.tr.as_mm(),
                                corner_radii.br.as_mm(),
                                corner_radii.bl.as_mm()
                            ]
                        ));
                    }
                    Shape::Ellipse { frame, arc, curve } => {
                        exact.push_str(&format!(" ellipse curve={}", curve.get()));
                        nums.extend([
                            frame.center.x,
                            frame.center.y,
                            frame.rx.as_mm(),
                            frame.ry.as_mm(),
                        ]);
                        if let Some(arc) = arc {
                            exact.push_str(&format!(" arc {:?}", arc.kind));
                            nums.extend([arc.start().as_radians(), arc.end().as_radians()]);
                        }
                    }
                    Shape::Polygon {
                        frame,
                        point_count,
                        curve,
                    } => {
                        exact.push_str(&format!(" polygon {point_count:?} curve={}", curve.get()));
                        nums.extend([
                            frame.center.x,
                            frame.center.y,
                            frame.radius.as_mm(),
                            frame.angle.as_radians(),
                        ]);
                    }
                    Shape::Star {
                        frame,
                        point_count,
                        inner_ratio,
                        curve,
                    } => {
                        exact.push_str(&format!(
                            " star {point_count:?} ratio={} curve={}",
                            inner_ratio.get(),
                            curve.get()
                        ));
                        nums.extend([
                            frame.center.x,
                            frame.center.y,
                            frame.radius.as_mm(),
                            frame.angle.as_radians(),
                        ]);
                    }
                }
            }
            NodeKind::Leaf(ObjectSnapshot::Clone(c)) => {
                exact.push_str(" clone of ");
                let pos = nodes.iter().position(|x| x.id == c.chain.origin);
                exact.push_str(&format!("{pos:?}"));
                nums.extend(c.chain.matrix.to_array());
            }
            NodeKind::Leaf(ObjectSnapshot::Image(i)) => {
                exact.push_str(&format!(" image {:?}", i.asset));
                nums.extend(i.placement.to_array());
            }
            NodeKind::Leaf(other) => exact.push_str(&format!(" {other:?}")),
        }
        out.push(Item {
            exact,
            numbers: nums,
        });
    }
    out
}

fn assert_same_doc(a: &Document, b: &Document, tol: f64) {
    let (ia, ib) = (items(a), items(b));
    let la: Vec<&str> = ia.iter().map(|i| i.exact.as_str()).collect();
    let lb: Vec<&str> = ib.iter().map(|i| i.exact.as_str()).collect();
    assert_eq!(la, lb, "exact parts differ");
    for (x, y) in ia.iter().zip(&ib) {
        assert_eq!(x.numbers.len(), y.numbers.len(), "{}", x.exact);
        for (p, q) in x.numbers.iter().zip(&y.numbers) {
            assert!(close(*p, *q, tol), "{}: {p} vs {q}", x.exact);
        }
    }
    let (pa, pb) = (a.pages(), b.pages());
    assert_eq!(pa.len(), pb.len(), "page count");
    let (oa, ob) = (pa[0].origin, pb[0].origin);
    for (p, q) in pa.iter().zip(&pb) {
        assert_eq!(p.name, q.name);
        assert!(
            close(p.size.width.as_mm(), q.size.width.as_mm(), tol)
                && close(p.size.height.as_mm(), q.size.height.as_mm(), tol)
        );
        assert!(
            close(p.origin.x - oa.x, q.origin.x - ob.x, tol)
                && close(p.origin.y - oa.y, q.origin.y - ob.y, tol),
            "relative page positions"
        );
    }
    assert_eq!(a.display_unit(), b.display_unit());
    assert_eq!(a.background(), b.background());
}

/// `nodes` moved by `v` (paths and primitives only).
fn translated(nodes: &[DrawingNode], v: Vec2) -> Vec<DrawingNode> {
    let mv = |p: Point| Point::new(p.x + v.x, p.y + v.y);
    nodes
        .iter()
        .map(|n| match n.clone() {
            DrawingNode::Group {
                registers,
                layer,
                children,
            } => DrawingNode::Group {
                registers,
                layer,
                children: translated(&children, v),
            },
            DrawingNode::Path {
                registers,
                outlines,
                style,
            } => DrawingNode::Path {
                registers,
                outlines: outlines
                    .into_iter()
                    .map(|o| DrawingOutline {
                        closed: o.closed,
                        anchors: o
                            .anchors
                            .into_iter()
                            .map(|a| DrawingAnchor {
                                point: mv(a.point),
                                ..a
                            })
                            .collect(),
                    })
                    .collect(),
                style,
            },
            DrawingNode::Primitive {
                registers,
                shape,
                rotation,
                style,
            } => {
                let shape = match shape {
                    Shape::Rect {
                        bounds,
                        corner_radii,
                    } => Shape::Rect {
                        bounds: RectBounds {
                            origin: mv(bounds.origin),
                            ..bounds
                        },
                        corner_radii,
                    },
                    Shape::Ellipse { frame, arc, curve } => Shape::Ellipse {
                        frame: EllipseFrame {
                            center: mv(frame.center),
                            ..frame
                        },
                        arc,
                        curve,
                    },
                    Shape::Polygon {
                        frame,
                        point_count,
                        curve,
                    } => Shape::Polygon {
                        frame: StarFrame {
                            center: mv(frame.center),
                            ..frame
                        },
                        point_count,
                        curve,
                    },
                    Shape::Star {
                        frame,
                        point_count,
                        inner_ratio,
                        curve,
                    } => Shape::Star {
                        frame: StarFrame {
                            center: mv(frame.center),
                            ..frame
                        },
                        point_count,
                        inner_ratio,
                        curve,
                    },
                };
                DrawingNode::Primitive {
                    registers,
                    shape,
                    rotation,
                    style,
                }
            }
            other => other,
        })
        .collect()
}

/// The document with guides moved where the export puts them: the guide leaves out of their
/// layers into one layer "Guides" after the others, layers holding only guides removed
/// (criterion 150: "the layer becomes Guides and is the only difference").
fn guides_moved(nodes: &[DrawingNode]) -> Vec<DrawingNode> {
    fn take(n: &DrawingNode, guides: &mut Vec<DrawingNode>) -> Option<DrawingNode> {
        match n {
            DrawingNode::Group {
                registers,
                layer,
                children,
            } => {
                let had = !children.is_empty();
                let kept: Vec<DrawingNode> =
                    children.iter().filter_map(|c| take(c, guides)).collect();
                if had && kept.is_empty() {
                    None
                } else {
                    Some(DrawingNode::Group {
                        registers: registers.clone(),
                        layer: *layer,
                        children: kept,
                    })
                }
            }
            DrawingNode::Primitive { registers, .. } | DrawingNode::Path { registers, .. }
                if registers.guide =>
            {
                guides.push(n.clone());
                None
            }
            other => Some(other.clone()),
        }
    }
    let mut guides = Vec::new();
    let mut out: Vec<DrawingNode> = nodes.iter().filter_map(|n| take(n, &mut guides)).collect();
    if !guides.is_empty() {
        out.push(layer("Guides", guides));
    }
    out
}

#[test]
fn c62_c150_our_own_data_comes_back_and_c63_c152_the_second_export_is_identical() {
    let d = everything_doc();
    let first = export(&d);
    let back = open(&first);
    let page = {
        let mut p = pages(&[
            ("Front", -50.0, 20.0, 210.0, 297.0),
            ("Back", 200.0, 20.0, 100.0, 50.0),
            ("Third", 0.0, 400.0, 30.0, 60.0),
        ]);
        p.display_unit = Some(DisplayUnit::Cm);
        p.background = Some(curvyo_document_core::DocumentBackground {
            paint: curvyo_document_core::BackgroundPaint::Solid,
            color: Color {
                r: 250,
                g: 240,
                b: 230,
            },
            opacity: Opacity::new(0.5).unwrap(),
        });
        p
    };
    // Open puts the first page at the file's origin (criterion 129): the union's top-left is
    // Front's corner (-50, 20), so everything comes back moved by (50, -20).
    let mut page = page;
    for f in &mut page.pages {
        f.origin = Point::new(f.origin.x + 50.0, f.origin.y - 20.0);
    }
    let expected = doc(
        &page,
        &translated(&guides_moved(&everything_nodes()), Vec2::new(50.0, -20.0)),
    );
    assert_same_doc(&expected, &back, 1e-6);
    let second = export(&back);
    assert_eq!(second, first, "export(open(export(d))) == export(d)");
}

#[test]
fn c58_compound_path_open_hole_crossing_round_trip() {
    let nodes = vec![layer(
        "L",
        vec![path(
            reg("cp"),
            vec![
                (
                    true,
                    vec![
                        corner(0.0, 0.0),
                        corner(50.0, 0.0),
                        corner(50.0, 50.0),
                        corner(0.0, 50.0),
                    ],
                ),
                (
                    true,
                    vec![
                        corner(10.0, 10.0),
                        corner(10.0, 40.0),
                        corner(40.0, 40.0),
                        corner(40.0, 10.0),
                    ],
                ),
                (false, vec![corner(55.0, 0.0), corner(75.0, 20.0)]),
                (false, vec![corner(55.0, 20.0), corner(75.0, 0.0)]),
                (
                    true,
                    vec![corner(80.0, 0.0), corner(90.0, 0.0), corner(85.0, 10.0)],
                ),
            ],
            filled(5, 5, 5),
        )],
    )];
    let d = doc(&a4(), &nodes);
    let text = export(&d);
    assert_eq!(of_kind(&text, "path").len(), 1, "one <path>");
    assert_same_doc(&d, &open(&text), 1e-6);
}

// ---------------------------------------------------------------------------------------------
// Criterion 66 and 149: hand edits win
// ---------------------------------------------------------------------------------------------

fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "{from} occurs once in\n{text}"
    );
    text.replacen(from, to, 1)
}

#[test]
fn c66_c149_hand_edits_win_over_shadow_data() {
    let mut s = filled(255, 0, 0);
    s.stroke.width = Length::from_mm(0.5);
    s.stroke.dash = DashPattern::new(vec![2.0, 1.0]).unwrap();
    let nodes = vec![layer(
        "L",
        vec![
            prim(
                reg("r"),
                rect_shape(10.0, 20.0, 30.0, 40.0, [1.0, 2.0, 3.0, 4.0]),
                0.0,
                s.clone(),
            ),
            prim(
                reg("st"),
                star_shape(100.0, 100.0, 20.0, 0.0, 5, Some(0.5), 0.0),
                0.0,
                s.clone(),
            ),
            prim(
                reg("hex"),
                star_shape(150.0, 150.0, 10.0, 0.0, 6, None, 0.3),
                0.0,
                s.clone(),
            ),
            line("l", (0.0, 0.0), (10.0, 0.0), s),
        ],
    )];
    let text = export(&doc(&a4(), &nodes));
    let (_, l) = by_id(&text, "path1");
    // (1) a coordinate
    let old_d = attr(&l, "d").unwrap();
    let edited = replace_once(&text, &format!("d=\"{old_d}\""), "d=\"M 0 0 L 12.5 0\"");
    match leaf_named(&open(&edited), "l") {
        ObjectSnapshot::Path(p) => assert!(close(p.anchors.last().unwrap().point.x, 12.5, 1e-9)),
        o => panic!("{o:?}"),
    }
    // (2) a width: the shadow width disagrees and is dropped.
    // The exact attribute run is the implementation's order; edit the line's own width instead.
    let line_tag = format!("<path{l}");
    let line_tag_edited = line_tag.replacen("stroke-width=\"0.5\"", "stroke-width=\"0.8\"", 1);
    let edited = replace_once(&text, &line_tag, &line_tag_edited);
    match leaf_named(&open(&edited), "l") {
        ObjectSnapshot::Path(p) => assert!(
            close(p.style.stroke.width.as_mm(), 0.8, 1e-9),
            "{}",
            p.style.stroke.width.as_mm()
        ),
        o => panic!("{o:?}"),
    }
    // (3) a colour
    let edited = replace_once(
        &text,
        &line_tag,
        &line_tag.replacen("fill=\"#ff0000\"", "fill=\"#00ff00\"", 1),
    );
    match leaf_named(&open(&edited), "l") {
        ObjectSnapshot::Path(p) => assert_eq!(p.style.fill.color, Color { r: 0, g: 255, b: 0 }),
        o => panic!("{o:?}"),
    }
    // (4) a dash list
    let da = attr(&l, "stroke-dasharray").unwrap();
    assert_eq!(da, "1 0.5");
    let edited = replace_once(
        &text,
        &line_tag,
        &line_tag.replacen("stroke-dasharray=\"1 0.5\"", "stroke-dasharray=\"3 1\"", 1),
    );
    match leaf_named(&open(&edited), "l") {
        ObjectSnapshot::Path(p) => assert_eq!(
            p.style.stroke.dash.as_slice(),
            [6.0, 2.0],
            "3 mm and 1 mm in multiples of 0.5"
        ),
        o => panic!("{o:?}"),
    }
    // (5) a star's point count, as Inkscape changes it: sodipodi:sides and the drawn d (taken
    // from the same star with 7 points), our curvyo:frame untouched.
    let (_, st) = by_id(&text, "star1");
    let st_tag = format!("<path{st}");
    let seven = export(&doc(
        &a4(),
        &[layer(
            "L",
            vec![prim(
                reg("st"),
                star_shape(100.0, 100.0, 20.0, 0.0, 7, Some(0.5), 0.0),
                0.0,
                hairline(),
            )],
        )],
    ));
    // Inkscape rewrites sides, arg2 and d; our shadow frame of the 5-point star stays.
    let seven_tag = format!("<path{}", by_id(&seven, "star1").1);
    let frame5 = attr(&st, "curvyo:frame").expect("a star carries curvyo:frame");
    let frame7 = attr(&by_id(&seven, "star1").1, "curvyo:frame").unwrap();
    let st_edited = seven_tag.replacen(
        &format!("curvyo:frame=\"{frame7}\""),
        &format!("curvyo:frame=\"{frame5}\""),
        1,
    );
    let st_edited = st_edited.replacen(
        "fill=\"none\" stroke=\"#000000\" stroke-width=\"0.25\"",
        "",
        1,
    );
    let st_edited = {
        // keep the original style attributes: rebuild from the 5-star tag with the star data swapped
        let mut t = st_tag.clone();
        for key in ["sodipodi:sides", "sodipodi:arg2", "d", "sodipodi:nodetypes"] {
            let old = attr(&st, key).unwrap_or_default();
            let new = attr(&st_edited, key).unwrap_or_default();
            t = t.replacen(&format!("{key}=\"{old}\""), &format!("{key}=\"{new}\""), 1);
        }
        t
    };
    let edited = replace_once(&text, &st_tag, &st_edited);
    match leaf_named(&open(&edited), "st") {
        ObjectSnapshot::Primitive(p) => match p.shape {
            Shape::Star { point_count, .. } => assert_eq!(
                format!("{point_count:?}"),
                format!("{:?}", PointCount::new(7).unwrap())
            ),
            other => panic!("{other:?}"),
        },
        o => panic!("{o:?}"),
    }
    // (6) criterion 140/149: a curved polygon whose d moved by 0.01 mm imports as a path.
    let (_, hex) = by_id(&text, "polygon1");
    let hd = attr(&hex, "d").unwrap();
    let first_x: f64 = hd.split_whitespace().nth(1).unwrap().parse().unwrap();
    let moved = hd.replacen(
        &format!("M {}", hd.split_whitespace().nth(1).unwrap()),
        &format!("M {}", first_x + 0.01),
        1,
    );
    let edited = replace_once(&text, &format!("d=\"{hd}\""), &format!("d=\"{moved}\""));
    assert!(
        matches!(leaf_named(&open(&edited), "hex"), ObjectSnapshot::Path(_)),
        "the curved polygon falls back to a path"
    );
    // Untouched: it stays a curved hexagon, exactly.
    match leaf_named(&open(&text), "hex") {
        ObjectSnapshot::Primitive(p) => match p.shape {
            Shape::Polygon { curve, .. } => assert_eq!(curve.get(), 0.3),
            other => panic!("{other:?}"),
        },
        o => panic!("{o:?}"),
    }
    // (7) the four-radius rectangle with a changed width disagrees with curvyo:radii frame: the
    // standard path wins (it is no longer that rectangle) or the rect follows the edit.
    let (_, r) = by_id(&text, "rect1");
    let rd = attr(&r, "d").unwrap();
    let shifted: String = rd
        .split(' ')
        .map(|t| {
            t.parse::<f64>()
                .map_or(t.to_owned(), |v| format!("{}", v + 1.0))
        })
        .collect::<Vec<_>>()
        .join(" ");
    let edited = replace_once(&text, &format!("d=\"{rd}\""), &format!("d=\"{shifted}\""));
    match leaf_named(&open(&edited), "r") {
        ObjectSnapshot::Primitive(p) => match p.shape {
            Shape::Rect { bounds, .. } => assert!(
                close(bounds.origin.x, 11.0, 1e-3),
                "follows the edited d: {:?}",
                bounds
            ),
            other => panic!("{other:?}"),
        },
        ObjectSnapshot::Path(p) => assert!(p.anchors.iter().all(|a| a.point.x >= 11.0 - 1e-6)),
        o => panic!("{o:?}"),
    }
}

// ---------------------------------------------------------------------------------------------
// Criteria 128 to 131: pages and scope
// ---------------------------------------------------------------------------------------------

#[test]
fn c128_two_pages_example() {
    let page = pages(&[
        ("Front", -50.0, 20.0, 100.0, 100.0),
        ("Coaster", 70.0, 20.0, 100.0, 50.0),
    ]);
    let d = doc(
        &page,
        &[layer("L", vec![rect("r", -40.0, 30.0, 10.0, 10.0)])],
    );
    let text = export(&d);
    let root = &tags(&text)[0].1;
    // The root is the first page (criterion 128 as amended 2026-11-01).
    assert_eq!(attr(root, "width").as_deref(), Some("100mm"));
    assert_eq!(attr(root, "height").as_deref(), Some("100mm"));
    assert_eq!(attr(root, "viewBox").as_deref(), Some("0 0 100 100"));
    let ps = of_kind(&text, "inkscape:page");
    assert_eq!(ps.len(), 2);
    let want = [
        ("page1", "0", "0", "100", "100", "Front"),
        ("page2", "120", "0", "100", "50", "Coaster"),
    ];
    for (p, w) in ps.iter().zip(want) {
        assert_eq!(attr(p, "id").as_deref(), Some(w.0));
        assert_eq!(attr(p, "x").as_deref(), Some(w.1));
        assert_eq!(attr(p, "y").as_deref(), Some(w.2));
        assert_eq!(attr(p, "width").as_deref(), Some(w.3));
        assert_eq!(attr(p, "height").as_deref(), Some(w.4));
        assert_eq!(attr(p, "inkscape:label").as_deref(), Some(w.5));
    }
    // inkscape:page lives inside sodipodi:namedview.
    let nv_start = text.find("<sodipodi:namedview").unwrap();
    let nv_end = text
        .find("</sodipodi:namedview>")
        .expect("namedview has children");
    let p1 = text.find("<inkscape:page").unwrap();
    assert!(nv_start < p1 && p1 < nv_end);
    let (_, r) = by_id(&text, "rect1");
    assert_eq!(attr(&r, "x").as_deref(), Some("10"));
    assert_eq!(attr(&r, "y").as_deref(), Some("10"));
    // Criterion 129: Open reads the pages back.
    let back = open(&text);
    let bp = back.pages();
    assert_eq!(bp.len(), 2);
    assert_eq!(
        (bp[0].name.as_str(), bp[1].name.as_str()),
        ("Front", "Coaster")
    );
    assert!(close(bp[1].origin.x - bp[0].origin.x, 120.0, 1e-6));
    assert!(close(bp[1].origin.y - bp[0].origin.y, 0.0, 1e-6));
    assert!(close(bp[1].size.height.as_mm(), 50.0, 1e-6));
}

fn three_pages_doc() -> Document {
    // A at (0,0), B at (200,0), C at (400,0), each 100 x 100.
    let page = pages(&[
        ("A", 0.0, 0.0, 100.0, 100.0),
        ("B", 200.0, 0.0, 100.0, 100.0),
        ("C", 400.0, 0.0, 100.0, 100.0),
    ]);
    let nodes = vec![
        layer(
            "L1",
            vec![
                rect("onA", 10.0, 10.0, 10.0, 10.0),
                rect("onB", 210.0, 10.0, 10.0, 10.0),
                rect("onC", 410.0, 10.0, 10.0, 10.0),
                rect("straddle", 90.0, 50.0, 120.0, 10.0),
                rect("paste", 150.0, 300.0, 5.0, 5.0),
                line("hline", (220.0, 80.0), (260.0, 80.0), hairline()),
                line("origA", (20.0, 20.0), (30.0, 30.0), hairline()),
                DrawingNode::Clone {
                    registers: reg("cloneB"),
                    original: 6,
                    matrix: Affine::translation(Vec2::new(200.0, 40.0)),
                },
                rect("touch", 300.0, 50.0, 10.0, 10.0),
                line("edge line", (300.0, 90.0), (300.0, 95.0), hairline()),
            ],
        ),
        layer("OnlyC", vec![rect("c2", 420.0, 20.0, 5.0, 5.0)]),
    ];
    doc(&page, &nodes)
}

fn labels_in(text: &str) -> Vec<String> {
    tags(text)
        .into_iter()
        .filter(|(n, _)| n != "g")
        .filter_map(|(_, r)| attr(&r, "inkscape:label"))
        .collect()
}

#[test]
fn c130_active_page_only() {
    let d = three_pages_doc();
    let out = export_full(
        &d,
        SvgExportOptions {
            active_page_only: true,
            ..SvgExportOptions::default()
        },
        &[],
        1,
    );
    let text = out.text;
    let root = &tags(&text)[0].1;
    assert_eq!(attr(root, "viewBox").as_deref(), Some("0 0 100 100"));
    assert!(
        !text.contains("<inkscape:page"),
        "no inkscape:page with Active page only"
    );
    let mut got = labels_in(&text);
    got.sort();
    // "touch" lies wholly right of B touching its edge with zero-area contact: out.
    // "edge line" is a vertical zero-width line on B's right edge x=300: inside the closed rect: in.
    let mut want = vec!["onB", "straddle", "hline", "cloneB", "edge line"];
    want.sort_unstable();
    assert_eq!(got, want, "{text}");
    let (k, c) = tags(&text)
        .into_iter()
        .find(|(_, r)| attr(r, "inkscape:label").as_deref() == Some("cloneB"))
        .unwrap();
    assert_eq!(k, "path", "the clone's original is out of scope: a path");
    let _ = c;
    let st = tags(&text)
        .into_iter()
        .find(|(_, r)| attr(r, "inkscape:label").as_deref() == Some("straddle"))
        .unwrap()
        .1;
    assert_eq!(
        attr(&st, "x").as_deref(),
        Some("-110"),
        "whole, minus B's corner"
    );
    assert_eq!(attr(&st, "width").as_deref(), Some("120"));
    assert!(
        !text.contains("OnlyC"),
        "a layer with nothing in scope is not written"
    );
}

#[test]
fn c131_selection_only() {
    let page = a4();
    let nodes = vec![
        layer(
            "A",
            vec![
                line("orig", (0.0, 0.0), (10.0, 0.0), hairline()),
                rect("r1", 0.0, 10.0, 5.0, 5.0),
            ],
        ),
        layer(
            "B",
            vec![
                rect("r2", 20.0, 10.0, 5.0, 5.0),
                DrawingNode::Clone {
                    registers: reg("clone"),
                    original: 0,
                    matrix: Affine::translation(Vec2::new(0.0, 50.0)),
                },
                rect("r3", 40.0, 10.0, 5.0, 5.0),
            ],
        ),
    ];
    let d = doc(&page, &nodes);
    let sel = [node_id(&d, "r2"), node_id(&d, "clone")];
    let text = export_full(
        &d,
        SvgExportOptions {
            selection_only: true,
            ..SvgExportOptions::default()
        },
        &sel,
        0,
    )
    .text;
    let got = labels_in(&text);
    assert_eq!(got, ["r2", "clone"]);
    let layers: Vec<String> = of_kind(&text, "g")
        .iter()
        .filter_map(|g| attr(g, "inkscape:label"))
        .collect();
    assert_eq!(layers, ["B"], "a layer with none of them is not written");
    let k = tags(&text)
        .into_iter()
        .find(|(_, r)| attr(r, "inkscape:label").as_deref() == Some("clone"))
        .unwrap()
        .0;
    assert_eq!(k, "path");
    // A selected layer's children as a whole.
    let sel = [node_id(&d, "A")];
    let text = export_full(
        &d,
        SvgExportOptions {
            selection_only: true,
            ..SvgExportOptions::default()
        },
        &sel,
        0,
    )
    .text;
    assert_eq!(labels_in(&text), ["orig", "r1"]);
}

#[test]
fn c131_selection_with_pages_uses_full_export_origin_and_both_options_intersect() {
    let d = three_pages_doc();
    let sel = [node_id(&d, "onA"), node_id(&d, "onB")];
    let text = export_full(
        &d,
        SvgExportOptions {
            selection_only: true,
            ..SvgExportOptions::default()
        },
        &sel,
        1,
    )
    .text;
    assert_eq!(
        attr(&tags(&text)[0].1, "viewBox").as_deref(),
        // The first page, as in a full export (criterion 128 as amended 2026-11-01).
        Some("0 0 100 100")
    );
    assert_eq!(of_kind(&text, "inkscape:page").len(), 3);
    let both = export_full(
        &d,
        SvgExportOptions {
            selection_only: true,
            active_page_only: true,
            ..SvgExportOptions::default()
        },
        &sel,
        1,
    )
    .text;
    assert_eq!(labels_in(&both), ["onB"]);
}

// ---------------------------------------------------------------------------------------------
// Criterion 137: images
// ---------------------------------------------------------------------------------------------

fn png_4x2() -> Vec<u8> {
    let mut img = image::RgbaImage::new(4, 2);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = image::Rgba([x as u8 * 60, y as u8 * 120, 7, 128]);
    }
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[test]
fn c137_images_verbatim_bytes_frame_and_matrix_forms() {
    let png = png_4x2();
    let asset: Arc<[u8]> = Arc::from(png.clone());
    let rot = Affine::rotation_about(
        Point::new(0.0, 0.0),
        Angle::from_radians(30f64.to_radians()),
    );
    let straight = Affine::new(20.0, 0.0, 0.0, 10.0, 10.0, 10.0);
    let rotated = Affine::new(20.0, 0.0, 0.0, 10.0, 0.0, 0.0)
        .compose(rot)
        .compose(Affine::translation(Vec2::new(50.0, 50.0)));
    let mirrored = Affine::new(-20.0, 0.0, 0.0, 10.0, 120.0, 10.0);
    let nodes = vec![layer(
        "L",
        vec![
            DrawingNode::Image {
                registers: reg("pic"),
                asset: 0,
                placement: straight,
            },
            DrawingNode::Image {
                registers: reg("rot"),
                asset: 0,
                placement: rotated,
            },
            DrawingNode::Image {
                registers: reg("mir"),
                asset: 0,
                placement: mirrored,
            },
        ],
    )];
    let d = doc_with(&a4(), &nodes, &[asset]);
    let text = export(&d);
    assert!(text.contains("xmlns:xlink=\"http://www.w3.org/1999/xlink\""));
    let imgs = of_kind(&text, "image");
    assert_eq!(imgs.len(), 3, "the same asset written in each image");
    let i0 = &imgs[0];
    assert_eq!(attr(i0, "id").as_deref(), Some("image1"));
    assert_eq!(attr(i0, "inkscape:label").as_deref(), Some("pic"));
    assert_eq!(attr(i0, "preserveAspectRatio").as_deref(), Some("none"));
    for (n, v) in [("x", "10"), ("y", "10"), ("width", "20"), ("height", "10")] {
        assert_eq!(attr(i0, n).as_deref(), Some(v), "{n}");
    }
    assert_eq!(attr(i0, "transform"), None);
    let href = attr(i0, "xlink:href").unwrap();
    assert_eq!(
        href,
        format!("data:image/png;base64,{}", base64(&png)),
        "verbatim bytes, padded base64, no line breaks"
    );
    for i in &imgs[1..] {
        assert_eq!(attr(i, "x").as_deref(), Some("0"));
        assert_eq!(attr(i, "width").as_deref(), Some("1"));
        assert!(attr(i, "transform").unwrap().starts_with("matrix("));
    }
    let back = open(&text);
    let want_hash = AssetHash::of(&png);
    for (name, m) in [("pic", straight), ("rot", rotated), ("mir", mirrored)] {
        match leaf_named(&back, name) {
            ObjectSnapshot::Image(i) => {
                assert_eq!(i.asset, want_hash);
                for (g, w) in i.placement.to_array().iter().zip(m.to_array()) {
                    assert!(close(*g, w, 1e-6), "{name}: {:?} vs {:?}", i.placement, m);
                }
            }
            o => panic!("{o:?}"),
        }
    }
    assert_eq!(export(&back), text, "stable");
}

// ---------------------------------------------------------------------------------------------
// Criteria 139 and 140: arcs and curved shapes
// ---------------------------------------------------------------------------------------------

#[test]
fn c139_pie_d_starts_at_the_centre() {
    let pie = EllipseArc::new(
        Angle::from_radians(0.0),
        Angle::from_radians(90f64.to_radians()),
        ArcKind::Pie,
    )
    .unwrap();
    let text = export(&doc(
        &a4(),
        &[layer(
            "L",
            vec![prim(
                reg("pie"),
                ellipse_shape(50.0, 50.0, 20.0, 20.0, Some(pie)),
                0.0,
                hairline(),
            )],
        )],
    ));
    let d = attr(&by_id(&text, "path1").1, "d").unwrap();
    assert!(
        d.starts_with("M 50 50 L 70 50 C"),
        "criterion 139: pie from the centre, line to the start: {d}"
    );
}

#[test]
fn c139_pie_example_and_arc_kinds() {
    let pie = EllipseArc::new(
        Angle::from_radians(0.0),
        Angle::from_radians(90f64.to_radians()),
        ArcKind::Pie,
    )
    .unwrap();
    let arc = EllipseArc::new(
        Angle::from_radians(0.0),
        Angle::from_radians(90f64.to_radians()),
        ArcKind::Arc,
    )
    .unwrap();
    let chord = EllipseArc::new(
        Angle::from_radians(1.0),
        Angle::from_radians(4.0),
        ArcKind::Chord,
    )
    .unwrap();
    let nodes = vec![layer(
        "L",
        vec![
            prim(
                reg("pie"),
                ellipse_shape(50.0, 50.0, 20.0, 20.0, Some(pie)),
                0.0,
                hairline(),
            ),
            prim(
                reg("arc"),
                ellipse_shape(50.0, 150.0, 20.0, 10.0, Some(arc)),
                0.0,
                hairline(),
            ),
            prim(
                reg("chord"),
                ellipse_shape(150.0, 150.0, 20.0, 10.0, Some(chord)),
                40.0,
                hairline(),
            ),
        ],
    )];
    let d = doc(&a4(), &nodes);
    let text = export(&d);
    let get = |name: &str| {
        tags(&text)
            .into_iter()
            .find(|(_, r)| attr(r, "inkscape:label").as_deref() == Some(name))
            .unwrap()
    };
    let (k, p) = get("pie");
    assert_eq!(k, "path");
    assert_eq!(attr(&p, "id").as_deref(), Some("path1"), "an arc is path1");
    assert_eq!(attr(&p, "sodipodi:type").as_deref(), Some("arc"));
    assert_eq!(attr(&p, "sodipodi:start").as_deref(), Some("0"));
    assert_eq!(
        attr(&p, "sodipodi:end").as_deref(),
        Some("1.5707963267948966")
    );
    assert_eq!(attr(&p, "sodipodi:arc-type").as_deref(), Some("slice"));
    assert!(attr(&p, "d").unwrap().trim_end().ends_with('Z'));
    let (_, a) = get("arc");
    assert_eq!(attr(&a, "sodipodi:arc-type").as_deref(), Some("arc"));
    assert_eq!(attr(&a, "sodipodi:open").as_deref(), Some("true"));
    assert!(!attr(&a, "d").unwrap().contains('Z'));
    let (_, c) = get("chord");
    assert_eq!(attr(&c, "sodipodi:arc-type").as_deref(), Some("chord"));
    assert_rotate(&attr(&c, "transform").unwrap(), [40.0, 150.0, 150.0]);
    assert_eq!(attr(&c, "sodipodi:cx").as_deref(), Some("150"));
    // Each cubic spans at most 22.5 degrees: a 90 degree arc has at least 4 C segments.
    assert!(attr(&a, "d").unwrap().matches('C').count() >= 4);
    // Accuracy: every on-curve point of the arc's d lies on the ellipse within 1e-4 mm.
    let nums = numbers(&attr(&a, "d").unwrap());
    let mut on = vec![(nums[0], nums[1])];
    for c in nums[2..].chunks(6) {
        on.push((c[4], c[5]));
        // Midpoint of the cubic at t = 0.5.
    }
    for (x, y) in on {
        let e = ((x - 50.0) / 20.0).powi(2) + ((y - 150.0) / 10.0).powi(2);
        assert!(close(e, 1.0, 1e-5), "({x}, {y}) on the ellipse");
    }
    // Criterion 139: midpoints of each cubic within 1e-4 mm of the curve (radius 1000 mm).
    let big = EllipseArc::new(
        Angle::from_radians(0.1),
        Angle::from_radians(6.0),
        ArcKind::Arc,
    )
    .unwrap();
    let dbig = doc(
        &a4(),
        &[layer(
            "L",
            vec![prim(
                reg("big"),
                ellipse_shape(0.0, 0.0, 1000.0, 1000.0, Some(big)),
                0.0,
                hairline(),
            )],
        )],
    );
    let tb = export(&dbig);
    let (_, bp) = by_id(&tb, "path1");
    let n = numbers(&attr(&bp, "d").unwrap());
    let mut p0 = (n[0], n[1]);
    for c in n[2..].chunks(6) {
        for t in [0.25, 0.5, 0.75] {
            let mt = 1.0 - t;
            let x = mt * mt * mt * p0.0
                + 3.0 * mt * mt * t * c[0]
                + 3.0 * mt * t * t * c[2]
                + t * t * t * c[4];
            let y = mt * mt * mt * p0.1
                + 3.0 * mt * mt * t * c[1]
                + 3.0 * mt * t * t * c[3]
                + t * t * t * c[5];
            let r = (x * x + y * y).sqrt();
            assert!(close(r, 1000.0, 1e-4 + 1e-6), "radius {r} at t={t}");
        }
        p0 = (c[4], c[5]);
    }
    // Round trip: same kind, angles within 1e-9 rad.
    let back = open(&text);
    assert_same_doc(&d, &back, 1e-6);
    for name in ["pie", "arc", "chord"] {
        match (leaf_named(&d, name), leaf_named(&back, name)) {
            (ObjectSnapshot::Primitive(a), ObjectSnapshot::Primitive(b)) => {
                match (a.shape, b.shape) {
                    (Shape::Ellipse { arc: Some(x), .. }, Shape::Ellipse { arc: Some(y), .. }) => {
                        assert_eq!(x.kind, y.kind);
                        assert!(close(x.start().as_radians(), y.start().as_radians(), 1e-9));
                        assert!(close(x.end().as_radians(), y.end().as_radians(), 1e-9));
                    }
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn c140_curved_polygon_and_star_keep_curve_exactly() {
    let nodes = vec![layer(
        "L",
        vec![
            prim(
                reg("hex"),
                star_shape(50.0, 50.0, 20.0, 0.0, 6, None, 0.3),
                0.0,
                hairline(),
            ),
            prim(
                reg("star"),
                star_shape(150.0, 50.0, 20.0, 12.0, 5, Some(0.4), 0.85),
                10.0,
                hairline(),
            ),
        ],
    )];
    let d = doc(&a4(), &nodes);
    let text = export(&d);
    for (name, kind) in [("hex", "polygon"), ("star", "star")] {
        let (k, p) = tags(&text)
            .into_iter()
            .find(|(_, r)| attr(r, "inkscape:label").as_deref() == Some(name))
            .unwrap();
        assert_eq!(k, "path");
        assert_eq!(
            attr(&p, "sodipodi:type"),
            None,
            "no Inkscape star data with Curve"
        );
        assert_eq!(attr(&p, "curvyo:kind").as_deref(), Some(kind));
        assert!(attr(&p, "curvyo:frame").is_some());
        assert!(
            attr(&p, "d").unwrap().contains('C'),
            "circular arcs as cubics"
        );
    }
    let back = open(&text);
    for name in ["hex", "star"] {
        match (leaf_named(&d, name), leaf_named(&back, name)) {
            (ObjectSnapshot::Primitive(a), ObjectSnapshot::Primitive(b)) => {
                assert_eq!(a.shape, b.shape, "{name}: all values exactly");
                assert_eq!(a.rotation, b.rotation, "{name}");
            }
            other => panic!("{other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Criteria 141 and 142: markers
// ---------------------------------------------------------------------------------------------

fn marked(
    start: MarkerShape,
    mid: MarkerShape,
    end: MarkerShape,
    place: MarkerPlace,
    count: u32,
) -> Style {
    let mut s = hairline();
    s.stroke.width = Length::from_mm(0.5);
    s.stroke.color = Color {
        r: 200,
        g: 10,
        b: 20,
    };
    s.stroke.markers = Markers {
        start,
        mid,
        end,
        mid_place: place,
        mid_count: MarkerCount::new(count).unwrap(),
    };
    s
}

/// The marker's path tip and base x after placing it at (ax, 0) with orient along +x
/// (end) scaled by the stroke width.
fn marker_extent(text: &str, url: &str, ax: f64, width: f64, dir: f64) -> (f64, f64) {
    let id = url.trim_start_matches("url(#").trim_end_matches(')');
    let (k, m) = by_id(text, id);
    assert_eq!(k, "marker");
    assert_eq!(attr(&m, "markerUnits").as_deref(), Some("strokeWidth"));
    assert_eq!(attr(&m, "orient").as_deref(), Some("auto"));
    assert_eq!(attr(&m, "refX").as_deref(), Some("0"));
    assert_eq!(attr(&m, "refY").as_deref(), Some("0"));
    assert_eq!(attr(&m, "overflow").as_deref(), Some("visible"));
    // viewBox must map 1:1 to markerWidth/Height so the path units are stroke widths.
    let vb = numbers(&attr(&m, "viewBox").unwrap());
    let mw: f64 = attr(&m, "markerWidth").unwrap().parse().unwrap();
    let mh: f64 = attr(&m, "markerHeight").unwrap().parse().unwrap();
    assert!(
        close(vb[2], mw, 1e-9) && close(vb[3], mh, 1e-9),
        "viewBox {vb:?} vs {mw} x {mh}"
    );
    let start = text.find(&format!("id=\"{id}\"")).unwrap();
    let body = &text[start..start + text[start..].find("</marker>").unwrap()];
    let p = tags(body)
        .into_iter()
        .find(|(n, _)| n == "path")
        .expect("an arrow path")
        .1;
    let xs: Vec<f64> = numbers(&attr(&p, "d").unwrap())
        .chunks(2)
        .map(|c| c[0])
        .collect();
    let tip = xs.iter().copied().fold(f64::MIN, f64::max) * dir;
    let tail = xs.iter().copied().fold(f64::MAX, f64::min) * dir;
    let (a, b) = (
        ax + tip.max(tail) * width * dir.signum() * dir.signum(),
        ax + tip.min(tail) * width,
    );
    (a.max(b), a.min(b))
}

#[test]
fn c141_markers_end_start_mid_and_spaced() {
    let nodes = vec![layer(
        "L",
        vec![
            line(
                "end",
                (0.0, 0.0),
                (100.0, 0.0),
                marked(
                    MarkerShape::None,
                    MarkerShape::None,
                    MarkerShape::Arrow,
                    MarkerPlace::Spaced,
                    1,
                ),
            ),
            line(
                "both",
                (0.0, 10.0),
                (100.0, 10.0),
                marked(
                    MarkerShape::Arrow,
                    MarkerShape::None,
                    MarkerShape::Arrow,
                    MarkerPlace::Spaced,
                    1,
                ),
            ),
            path(
                reg("mid"),
                vec![(
                    false,
                    vec![corner(0.0, 20.0), corner(10.0, 20.0), corner(10.0, 30.0)],
                )],
                marked(
                    MarkerShape::None,
                    MarkerShape::Arrow,
                    MarkerShape::None,
                    MarkerPlace::AtNodes,
                    1,
                ),
            ),
            line(
                "spaced",
                (0.0, 40.0),
                (100.0, 40.0),
                marked(
                    MarkerShape::None,
                    MarkerShape::Dot,
                    MarkerShape::None,
                    MarkerPlace::Spaced,
                    3,
                ),
            ),
            path(
                reg("closed"),
                vec![(
                    true,
                    vec![corner(0.0, 50.0), corner(10.0, 50.0), corner(10.0, 60.0)],
                )],
                marked(
                    MarkerShape::Arrow,
                    MarkerShape::None,
                    MarkerShape::Dot,
                    MarkerPlace::Spaced,
                    1,
                ),
            ),
            path(
                reg("compound m"),
                vec![
                    (false, vec![corner(0.0, 70.0), corner(10.0, 70.0)]),
                    (false, vec![corner(0.0, 75.0), corner(10.0, 75.0)]),
                ],
                marked(
                    MarkerShape::Arrow,
                    MarkerShape::None,
                    MarkerShape::Arrow,
                    MarkerPlace::Spaced,
                    1,
                ),
            ),
            line(
                "hidden place",
                (0.0, 90.0),
                (10.0, 90.0),
                marked(
                    MarkerShape::None,
                    MarkerShape::None,
                    MarkerShape::None,
                    MarkerPlace::AtNodes,
                    5,
                ),
            ),
        ],
    )];
    let d = doc(&a4(), &nodes);
    let text = export(&d);
    let defs = text.find("<defs").expect("defs");
    let first_layer = text.find("inkscape:groupmode=\"layer\"").unwrap();
    assert!(defs < first_layer, "defs before the first layer");
    let get = |name: &str| {
        tags(&text)
            .into_iter()
            .find(|(_, r)| attr(r, "inkscape:label").as_deref() == Some(name))
            .unwrap()
            .1
    };
    let end = get("end");
    let url = attr(&end, "marker-end").expect("marker-end");
    assert_eq!(attr(&end, "marker-start"), None);
    let (tip, base) = marker_extent(&text, &url, 100.0, 0.5, 1.0);
    assert!(
        close(tip, 101.0, 0.01) && close(base, 99.0, 0.01),
        "tip {tip} base {base}"
    );
    let id = url.trim_start_matches("url(#").trim_end_matches(')');
    let start = text.find(&format!("id=\"{id}\"")).unwrap();
    let body = &text[start..start + text[start..].find("</marker>").unwrap()];
    assert!(body.contains("d=\"M -2 -1.5 L 2 0 L -2 1.5 Z\""), "{body}");
    assert!(
        body.contains("fill=\"#c80a14\"") && body.contains("stroke=\"none\""),
        "{body}"
    );
    let both = get("both");
    let surl = attr(&both, "marker-start").unwrap();
    let sid = surl.trim_start_matches("url(#").trim_end_matches(')');
    let s0 = text.find(&format!("id=\"{sid}\"")).unwrap();
    let sbody = &text[s0..s0 + text[s0..].find("</marker>").unwrap()];
    assert!(
        sbody.contains("d=\"M 2 -1.5 L -2 0 L 2 1.5 Z\""),
        "mirrored start arrow: {sbody}"
    );
    // Start arrow tip at x = -1: min x of mirrored path (-2) times 0.5 at anchor 0.
    assert_eq!(
        attr(&both, "marker-end").as_deref(),
        Some(url.as_str()),
        "same combination shares one marker"
    );
    let mid = get("mid");
    assert!(attr(&mid, "marker-mid").is_some(), "At nodes: marker-mid");
    let spaced = get("spaced");
    assert_eq!(
        attr(&spaced, "marker-mid"),
        None,
        "Spaced has no standard form"
    );
    assert_eq!(
        attr(&spaced, "curvyo:markers").as_deref(),
        Some("start=none middle=dot place=spaced count=3 end=none")
    );
    let closed = get("closed");
    assert!(
        attrs(&closed)
            .iter()
            .all(|(k, _)| !k.starts_with("marker-")),
        "closed: no marker attributes"
    );
    assert!(attr(&closed, "curvyo:markers").is_some());
    let cm = get("compound m");
    assert!(
        attrs(&cm).iter().all(|(k, _)| !k.starts_with("marker-")),
        "compound: no marker attributes"
    );
    let hp = get("hidden place");
    assert_eq!(
        attr(&hp, "curvyo:markers").as_deref(),
        Some("start=none middle=none place=nodes count=5 end=none"),
        "hidden Place and Count survive"
    );
    let markers = of_kind(&text, "marker");
    let ids: Vec<String> = markers.iter().map(|m| attr(m, "id").unwrap()).collect();
    assert_eq!(ids[0], "marker1");
    // Round trip: all slots, place and count exactly.
    let back = open(&text);
    for name in [
        "end",
        "both",
        "mid",
        "spaced",
        "closed",
        "compound m",
        "hidden place",
    ] {
        let a = match leaf_named(&d, name) {
            ObjectSnapshot::Path(p) => p.style.stroke.markers,
            o => panic!("{o:?}"),
        };
        let b = match leaf_named(&back, name) {
            ObjectSnapshot::Path(p) => p.style.stroke.markers,
            o => panic!("{o:?}"),
        };
        assert_eq!(a, b, "{name}");
    }
}

#[test]
fn c142_removed_marker_end_comes_back_none() {
    let nodes = vec![layer(
        "L",
        vec![line(
            "both",
            (0.0, 10.0),
            (100.0, 10.0),
            marked(
                MarkerShape::Arrow,
                MarkerShape::Dot,
                MarkerShape::Arrow,
                MarkerPlace::Spaced,
                4,
            ),
        )],
    )];
    let text = export(&doc(&a4(), &nodes));
    let (_, p) = by_id(&text, "path1");
    let url = attr(&p, "marker-end").unwrap();
    let edited = replace_once(&text, &format!(" marker-end=\"{url}\""), "");
    match leaf_named(&open(&edited), "both") {
        ObjectSnapshot::Path(p) => {
            let m = p.style.stroke.markers;
            assert_eq!(m.end, MarkerShape::None);
            assert_eq!(m.start, MarkerShape::Arrow);
            assert_eq!(m.mid, MarkerShape::Dot);
            assert_eq!(m.mid_place, MarkerPlace::Spaced);
            assert_eq!(m.mid_count, MarkerCount::new(4).unwrap());
        }
        o => panic!("{o:?}"),
    }
}

// ---------------------------------------------------------------------------------------------
// Criteria 143 to 145: guides and what is not exported
// ---------------------------------------------------------------------------------------------

#[test]
fn c143_c144_guides_layer() {
    let guide = |name: &str, x: f64, hidden: bool| {
        prim(
            DrawingRegisters {
                name: Some(name.into()),
                guide: true,
                visible: !hidden,
                ..DrawingRegisters::default()
            },
            ellipse_shape(x, 50.0, 10.0, 10.0, None),
            0.0,
            hairline(),
        )
    };
    let nodes = vec![
        layer(
            "Cut",
            vec![
                rect("cut rect", 0.0, 0.0, 10.0, 10.0),
                guide("g1", 50.0, false),
            ],
        ),
        layer("Own", vec![guide("g2", 100.0, true)]),
        layer(
            "Mixed",
            vec![
                group(
                    Some("grp"),
                    vec![guide("g3", 150.0, false), rect("keep", 0.0, 0.0, 1.0, 1.0)],
                ),
                group(Some("only guide grp"), vec![guide("g4", 160.0, false)]),
            ],
        ),
    ];
    let d = doc(&a4(), &nodes);
    let text = export(&d);
    let layers: Vec<String> = of_kind(&text, "g")
        .iter()
        .filter(|g| attr(g, "inkscape:groupmode").is_some())
        .map(|g| attr(g, "inkscape:label").unwrap())
        .collect();
    assert_eq!(
        layers,
        ["Cut", "Mixed", "Guides"],
        "own guide layer not written, Guides last\n{text}"
    );
    let gl = of_kind(&text, "g")
        .into_iter()
        .find(|g| attr(g, "inkscape:label").as_deref() == Some("Guides"))
        .unwrap();
    assert_eq!(attr(&gl, "curvyo:guides").as_deref(), Some("1"));
    assert_eq!(attr(&gl, "style").as_deref(), Some("display:none"));
    assert_eq!(attr(&gl, "sodipodi:insensitive").as_deref(), Some("true"));
    assert!(attr(&gl, "id").unwrap().starts_with("layer"));
    assert!(
        !text.contains("only guide grp"),
        "a group that held only guides is not written"
    );
    assert!(text.contains("inkscape:label=\"grp\""));
    let guides_at = text.find("curvyo:guides=\"1\"").unwrap();
    let after = &text[guides_at..];
    for g in ["g1", "g2", "g3", "g4"] {
        assert!(
            after.contains(&format!("inkscape:label=\"{g}\"")),
            "{g} in the Guides layer"
        );
    }
    let g2 = tags(after)
        .into_iter()
        .find(|(_, r)| attr(r, "inkscape:label").as_deref() == Some("g2"))
        .unwrap()
        .1;
    assert_eq!(
        attr(&g2, "style").as_deref(),
        Some("display:none"),
        "a hidden guide stays hidden"
    );
    // Criterion 144: Open gives guides in a layer "Guides", none in Cut.
    let back = open(&text);
    assert_eq!(layer_names(&back), ["Cut", "Mixed", "Guides"]);
    let guides: Vec<_> = back.read_nodes().into_iter().filter(|n| n.guide).collect();
    assert_eq!(guides.len(), 4);
    let gl_id = node_id(&back, "Guides");
    assert!(guides.iter().all(|g| g.parent == Some(gl_id)));
    let g1 = guides
        .iter()
        .find(|g| g.registers.name.as_deref() == Some("g1"))
        .unwrap();
    assert!(
        g1.registers.visible && !g1.registers.locked,
        "the layer's hidden and locked state not applied"
    );
    let gl = back
        .read_nodes()
        .into_iter()
        .find(|n| n.id == gl_id)
        .unwrap();
    let _ = gl;
    let g2 = guides
        .iter()
        .find(|g| g.registers.name.as_deref() == Some("g2"))
        .unwrap();
    assert!(!g2.registers.visible);
    assert_eq!(export(&back), text, "stable");
}

#[test]
fn c143_no_guides_no_layer_and_c144_plain_guides_layer_is_ordinary() {
    let text = export(&doc(
        &a4(),
        &[layer("Cut", vec![rect("r", 0.0, 0.0, 1.0, 1.0)])],
    ));
    assert!(!text.contains("Guides"));
    let plain = text.replace("inkscape:label=\"Cut\"", "inkscape:label=\"Guides\"");
    let back = open(&plain);
    assert!(
        back.read_nodes().iter().all(|n| !n.guide),
        "a layer named Guides without curvyo:guides is ordinary"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 146: strokes as paths
// ---------------------------------------------------------------------------------------------

#[test]
fn c146_width_zero_is_unchanged() {
    let mut zero = filled(0, 255, 0);
    zero.stroke.width = Length::from_mm(0.0);
    let d = doc(
        &a4(),
        &[layer(
            "L",
            vec![prim(
                reg("zero"),
                rect_shape(10.0, 60.0, 4.0, 2.0, [0.0; 4]),
                0.0,
                zero,
            )],
        )],
    );
    let text = export_opt(
        &d,
        SvgExportOptions {
            strokes_as_paths: true,
            ..SvgExportOptions::default()
        },
    );
    let t = tags(&text)
        .into_iter()
        .find(|(_, r)| attr(r, "inkscape:label").as_deref() == Some("zero"))
        .unwrap();
    assert_eq!(t.0, "rect", "criterion 146: width 0 is unchanged\n{text}");
}

#[test]
fn c146_strokes_as_paths() {
    let mut dashed = hairline();
    dashed.stroke.width = Length::from_mm(1.0);
    dashed.stroke.dash = DashPattern::new(vec![5.0, 5.0]).unwrap();
    dashed.stroke.color = Color { r: 0, g: 0, b: 255 };
    dashed.stroke.opacity = Opacity::new(0.5).unwrap();
    let mut both = filled(255, 0, 0);
    both.stroke.width = Length::from_mm(2.0);
    let mut off = filled(0, 255, 0);
    off.stroke.enabled = false;
    let mut zero = filled(0, 255, 0);
    zero.stroke.width = Length::from_mm(0.0);
    let nodes = vec![layer(
        "L",
        vec![
            line("dashed", (0.0, 0.0), (100.0, 0.0), dashed),
            prim(
                reg("box"),
                rect_shape(10.0, 10.0, 40.0, 20.0, [2.0; 4]),
                0.0,
                both,
            ),
            prim(
                reg("nostroke"),
                rect_shape(10.0, 50.0, 4.0, 2.0, [0.0; 4]),
                0.0,
                off,
            ),
            prim(
                reg("zero"),
                rect_shape(10.0, 60.0, 4.0, 2.0, [0.0; 4]),
                0.0,
                zero,
            ),
            line(
                "arrow",
                (0.0, 100.0),
                (50.0, 100.0),
                marked(
                    MarkerShape::Arrow,
                    MarkerShape::None,
                    MarkerShape::Arrow,
                    MarkerPlace::Spaced,
                    1,
                ),
            ),
        ],
    )];
    let d = doc(&a4(), &nodes);
    let text = export_opt(
        &d,
        SvgExportOptions {
            strokes_as_paths: true,
            ..SvgExportOptions::default()
        },
    );
    assert!(
        !text.contains("stroke-dasharray") || !text.contains("dashed\" stroke"),
        "{text}"
    );
    assert!(
        !text.contains("<marker"),
        "no marker definitions under strokes as paths"
    );
    assert!(
        !text.contains("marker-end")
            && !text.contains("curvyo:markers")
            && !text.contains("curvyo:dash")
    );
    let g = of_kind(&text, "g")
        .into_iter()
        .find(|g| attr(g, "inkscape:label").as_deref() == Some("dashed"))
        .expect("a group named dashed");
    assert!(attr(&g, "id").unwrap().starts_with('g'));
    // The group's content: only the stroke outline (no fill part: fill off).
    let at = text.find("inkscape:label=\"dashed\"").unwrap();
    let inner = &text[at..at + text[at..].find("</g>").unwrap()];
    let paths = of_kind(inner, "path");
    assert_eq!(paths.len(), 1, "{inner}");
    let p = &paths[0];
    assert_eq!(attr(p, "fill").as_deref(), Some("#0000ff"));
    assert_eq!(attr(p, "fill-opacity").as_deref(), Some("0.5"));
    assert_eq!(attr(p, "fill-rule").as_deref(), Some("nonzero"));
    assert_eq!(attr(p, "stroke").as_deref(), Some("none"));
    assert!(attrs(p).iter().all(|(k, _)| !k.starts_with("stroke-")));
    assert_eq!(
        attr(p, "d").unwrap().matches('M').count(),
        10,
        "10 dash outlines"
    );
    let at = text.find("inkscape:label=\"box\"").unwrap();
    let inner = &text[at..at + text[at..].find("</g>").unwrap()];
    let ts = tags(inner);
    // `inner` starts inside the group's own tag: its children are the fill part and the outline.
    assert_eq!(ts.len(), 2, "rect, outline: {inner}");
    assert_eq!(ts[0].0, "rect");
    assert_eq!(attr(&ts[0].1, "stroke").as_deref(), Some("none"));
    assert_eq!(ts[1].0, "path");
    assert_eq!(attr(&ts[1].1, "fill").as_deref(), Some("#000000"));
    // Unchanged objects.
    for name in ["nostroke"] {
        let t = tags(&text)
            .into_iter()
            .find(|(_, r)| attr(r, "inkscape:label").as_deref() == Some(name))
            .unwrap();
        assert_eq!(t.0, "rect", "{name} unchanged");
    }
    // Arrow markers are part of the outline: the outline reaches past the line end.
    let at = text.find("inkscape:label=\"arrow\"").unwrap();
    let inner = &text[at..at + text[at..].find("</g>").unwrap()];
    let xs: Vec<f64> = numbers(&attr(&of_kind(inner, "path")[0], "d").unwrap())
        .chunks(2)
        .map(|c| c[0])
        .collect();
    let maxx = xs.iter().copied().fold(f64::MIN, f64::max);
    assert!(
        close(maxx, 50.0 + 2.0 * 0.5, 0.01),
        "the arrow tip is in the outline: {maxx}"
    );
    // Import gives filled paths.
    let back = open(&text);
    assert!(leaves(&back).iter().all(|(_, o, _)| match o {
        ObjectSnapshot::Path(p) => !p.style.stroke.enabled || p.style.fill.enabled,
        _ => true,
    }));
}

// ---------------------------------------------------------------------------------------------
// Criterion 157: format and safety; the property tests
// ---------------------------------------------------------------------------------------------

#[test]
fn c157_data_uris_hold_only_base64_and_the_curvyo_attributes_are_from_the_list() {
    let text = export(&everything_doc());
    let allowed = [
        "curvyo:format",
        "curvyo:dash",
        "curvyo:stroke-width",
        "curvyo:radii",
        "curvyo:kind",
        "curvyo:frame",
        "curvyo:markers",
        "curvyo:outlines",
        "curvyo:size",
        "curvyo:face",
        "curvyo:guides",
    ];
    for (_, raw) in tags(&text) {
        for (k, v) in attrs(&raw) {
            if k.starts_with("curvyo:") {
                assert!(allowed.contains(&k.as_str()), "{k} is not in the profile");
            }
            if v.starts_with("data:") {
                let b = v.split_once(";base64,").unwrap().1;
                assert!(
                    b.bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'+' || c == b'/' || c == b'=')
                );
            }
        }
    }
}

fn arb_doc() -> impl Strategy<Value = Vec<DrawingNode>> {
    let leaf = prop_oneof![
        (
            0.0f64..200.0,
            0.0f64..200.0,
            1.0f64..50.0,
            1.0f64..50.0,
            -180.0f64..180.0,
            prop::array::uniform4(0.0f64..1.0),
            any::<bool>()
        )
            .prop_map(|(x, y, w, h, deg, r, uni)| {
                let r = if uni { [r[0]; 4] } else { r };
                prim(reg("r"), rect_shape(x, y, w, h, r), deg, hairline())
            }),
        (
            0.0f64..200.0,
            0.0f64..200.0,
            1.0f64..50.0,
            1.0f64..50.0,
            -180.0f64..180.0
        )
            .prop_map(|(x, y, a, b, deg)| prim(
                reg("e"),
                ellipse_shape(x, y, a, b, None),
                deg,
                hairline()
            )),
        (
            0.0f64..200.0,
            0.0f64..200.0,
            1.0f64..50.0,
            -180.0f64..180.0,
            3u32..12,
            prop::option::of(0.1f64..0.9),
            prop_oneof![Just(0.0), 0.0f64..1.0]
        )
            .prop_map(|(x, y, r, deg, n, ratio, curve)| prim(
                reg("s"),
                star_shape(x, y, r, deg, n, ratio, curve),
                0.0,
                hairline()
            )),
        (
            0.0f64..200.0,
            0.0f64..200.0,
            1.0f64..50.0,
            1.0f64..50.0,
            0.0f64..3.0,
            0.1f64..3.0,
            0usize..3
        )
            .prop_map(|(x, y, a, b, s, sweep, k)| {
                let kind = [ArcKind::Pie, ArcKind::Arc, ArcKind::Chord][k];
                let arc =
                    EllipseArc::new(Angle::from_radians(s), Angle::from_radians(s + sweep), kind)
                        .unwrap();
                prim(
                    reg("a"),
                    ellipse_shape(x, y, a, b, Some(arc)),
                    0.0,
                    hairline(),
                )
            }),
        (
            prop::collection::vec((-500.0f64..500.0, -500.0f64..500.0), 2..8),
            any::<bool>(),
            0usize..5
        )
            .prop_map(|(pts, closed, st)| {
                let s = style_kind()[st].clone();
                path(
                    reg("p"),
                    vec![(closed, pts.iter().map(|&(x, y)| corner(x, y)).collect())],
                    s,
                )
            }),
    ];
    prop::collection::vec(prop::collection::vec(leaf, 0..6), 1..4).prop_map(|layers| {
        layers
            .into_iter()
            .enumerate()
            .map(|(i, children)| layer(&format!("Layer {i}"), children))
            .collect()
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 500, ..ProptestConfig::default() })]

    /// Criteria 62 and 63 over 500 random documents.
    #[test]
    fn c62_c63_random_documents_round_trip_and_are_stable(nodes in arb_doc()) {
        let d = doc(&a4(), &nodes);
        let first = export(&d);
        let back = open(&first);
        assert_same_doc(&d, &back, 1e-6);
        let second = export(&back);
        prop_assert_eq!(second, first);
    }

    /// Criterion 63 alone over 500 random documents: the second export is byte-identical.
    #[test]
    fn c63_random_documents_second_export_is_identical(nodes in arb_doc()) {
        let d = doc(&a4(), &nodes);
        let first = export(&d);
        prop_assert_eq!(export(&open(&first)), first);
    }
}
