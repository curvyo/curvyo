//! Round trips of the SVG export (`specs/0024-svg-import-export` criteria 58, 59, 62, 63, 72,
//! 128 to 131, 139 to 146, 150 to 152): a document made from a drawing is exported, written,
//! read and opened again, and compared with the first (ids dropped, geometry within 1e-6 mm,
//! everything else exactly); a second export gives the same bytes. Texts need faces and are
//! covered by `curvyo-editor-wasm/tests/svg_export_golden.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::format_push_string
)]

use curvyo_document_core::{
    Affine, AnchorId, AnchorKind, Angle, ArcKind, BackgroundPaint, Color, CornerRadii, Curve,
    DashPattern, DisplayUnit, Document, DocumentBackground, DocumentSize, DrawingAnchor,
    DrawingNode, DrawingOutline, DrawingPage, DrawingPageFrame, DrawingRegisters, EllipseArc,
    EllipseFrame, InnerRatio, Length, LineCap, LineJoin, MarkerCount, MarkerPlace, MarkerShape,
    Markers, NodeKind, ObjectSnapshot, Opacity, Point, PointCount, RectBounds, Shape, StarFrame,
    Style, Vec2,
};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{
    ExportScope, ImportFaces, SvgExportOptions, SvgPlacement, svg_export, svg_import,
};
use proptest::prelude::*;

const GEOMETRY_MM: f64 = 1e-6 + 1e-9;

fn anchor(x: f64, y: f64, hin: (f64, f64), hout: (f64, f64), kind: AnchorKind) -> DrawingAnchor {
    DrawingAnchor {
        point: Point::new(x, y),
        handle_in: Vec2::new(hin.0, hin.1),
        handle_out: Vec2::new(hout.0, hout.1),
        kind,
    }
}

fn corner(x: f64, y: f64) -> DrawingAnchor {
    anchor(x, y, (0.0, 0.0), (0.0, 0.0), AnchorKind::Corner)
}

fn named(name: &str) -> DrawingRegisters {
    DrawingRegisters {
        name: Some(name.to_owned()),
        ..DrawingRegisters::default()
    }
}

fn stroke_style(width: f64, dash: &[f64], cap: LineCap, join: LineJoin) -> Style {
    let mut style = Style::default();
    style.stroke.width = Length::from_mm(width);
    style.stroke.dash = DashPattern::new(dash.to_vec()).unwrap();
    style.stroke.cap = cap;
    style.stroke.join = join;
    style.stroke.color = Color {
        r: 10,
        g: 20,
        b: 30,
    };
    style.stroke.opacity = Opacity::new(0.75).unwrap();
    style.fill.enabled = true;
    style.fill.color = Color {
        r: 200,
        g: 100,
        b: 0,
    };
    style.fill.opacity = Opacity::new(0.5).unwrap();
    style
}

fn rotated(shape: Shape, deg: f64, style: Style) -> DrawingNode {
    DrawingNode::Primitive {
        registers: DrawingRegisters::default(),
        shape,
        rotation: Angle::from_radians(deg.to_radians()),
        style,
    }
}

fn star_frame(x: f64, y: f64, r: f64, deg: f64) -> StarFrame {
    StarFrame {
        center: Point::new(x, y),
        radius: Length::from_mm(r),
        angle: Angle::from_radians(deg.to_radians()),
    }
}

/// Criterion 62's generated document, deterministic part.
fn everything() -> Vec<DrawingNode> {
    let curved_close = DrawingNode::Path {
        registers: named("Curved & closed <1>"),
        outlines: vec![DrawingOutline {
            closed: true,
            anchors: vec![
                anchor(10.0, 10.0, (-2.0, 3.0), (2.0, -3.0), AnchorKind::Symmetric),
                anchor(30.0, 10.0, (-1.0, -1.0), (0.0, 0.0), AnchorKind::Corner),
                anchor(30.0, 30.0, (0.0, 0.0), (-3.0, 1.0), AnchorKind::Asymmetric),
            ],
        }],
        style: stroke_style(0.3, &[1.0, 2.0, 0.5], LineCap::Round, LineJoin::Round),
    };
    let compound = DrawingNode::Path {
        registers: DrawingRegisters::default(),
        outlines: vec![
            DrawingOutline {
                closed: true,
                anchors: vec![
                    corner(0.0, 0.0),
                    corner(50.0, 0.0),
                    corner(50.0, 50.0),
                    corner(0.0, 50.0),
                ],
            },
            DrawingOutline {
                closed: true,
                anchors: vec![
                    corner(10.0, 10.0),
                    corner(10.0, 40.0),
                    corner(40.0, 40.0),
                    corner(40.0, 10.0),
                ],
            },
            DrawingOutline {
                closed: false,
                anchors: vec![corner(60.0, 0.0), corner(70.0, 10.0), corner(60.0, 20.0)],
            },
            DrawingOutline {
                closed: false,
                anchors: vec![corner(55.0, 0.0), corner(75.0, 20.0)],
            },
        ],
        style: stroke_style(0.1 + 0.2, &[], LineCap::Square, LineJoin::Bevel),
    };
    let mut marked_style = stroke_style(0.5, &[], LineCap::Butt, LineJoin::Miter);
    marked_style.fill.enabled = false;
    marked_style.fill.color = Color::BLACK;
    marked_style.fill.opacity = Opacity::OPAQUE;
    marked_style.stroke.markers = Markers {
        start: MarkerShape::Arrow,
        mid: MarkerShape::Dot,
        end: MarkerShape::Arrow,
        mid_place: MarkerPlace::AtNodes,
        mid_count: MarkerCount::new(4).unwrap(),
    };
    let marked = DrawingNode::Path {
        registers: named("Arrow"),
        outlines: vec![DrawingOutline {
            closed: false,
            anchors: vec![corner(0.0, 100.0), corner(10.0, 100.0), corner(10.0, 110.0)],
        }],
        style: marked_style.clone(),
    };
    let mut spaced = marked_style.clone();
    spaced.stroke.markers.mid_place = MarkerPlace::Spaced;
    spaced.stroke.markers.mid_count = MarkerCount::new(3).unwrap();
    let spaced_closed = DrawingNode::Path {
        registers: DrawingRegisters::default(),
        outlines: vec![DrawingOutline {
            closed: true,
            anchors: vec![corner(0.0, 120.0), corner(10.0, 120.0), corner(10.0, 130.0)],
        }],
        style: spaced,
    };
    let mut no_paint = Style::default();
    no_paint.stroke.enabled = false;
    no_paint.fill.enabled = true;
    let rect = Shape::Rect {
        bounds: RectBounds {
            origin: Point::new(100.0, 10.0),
            width: Length::from_mm(40.0),
            height: Length::from_mm(20.0),
        },
        corner_radii: CornerRadii {
            tl: Length::from_mm(1.0),
            tr: Length::from_mm(2.0),
            br: Length::from_mm(3.0),
            bl: Length::from_mm(4.0),
        },
    };
    let plain_rect = Shape::Rect {
        bounds: RectBounds {
            origin: Point::new(100.0, 40.0),
            width: Length::from_mm(30.0),
            height: Length::from_mm(10.0),
        },
        corner_radii: CornerRadii::uniform(Length::from_mm(2.0)),
    };
    let ellipse = |rx: f64, ry: f64, arc: Option<EllipseArc>| Shape::Ellipse {
        frame: EllipseFrame {
            center: Point::new(150.0, 80.0),
            rx: Length::from_mm(rx),
            ry: Length::from_mm(ry),
        },
        arc,
        curve: Curve::ELLIPSE_DEFAULT,
    };
    let arc = |kind| {
        EllipseArc::new(
            Angle::from_radians(0.0),
            Angle::from_radians(std::f64::consts::FRAC_PI_2),
            kind,
        )
    };
    let polygon = |curve: f64| Shape::Polygon {
        frame: star_frame(50.0, 200.0, 15.0, 10.0),
        point_count: PointCount::new(6).unwrap(),
        curve: Curve::new(curve).unwrap(),
    };
    let star = |deg: f64, curve: f64| Shape::Star {
        frame: star_frame(100.0, 200.0, 20.0, deg),
        point_count: PointCount::new(5).unwrap(),
        inner_ratio: InnerRatio::new(0.4).unwrap(),
        curve: Curve::new(curve).unwrap(),
    };
    let default = Style::default;
    vec![
        DrawingNode::Group {
            registers: DrawingRegisters {
                name: Some("Cut".into()),
                opacity: Opacity::new(0.8).unwrap(),
                ..DrawingRegisters::default()
            },
            layer: true,
            children: vec![
                curved_close,
                compound,
                DrawingNode::Clone {
                    registers: named("moved clone"),
                    original: 1,
                    matrix: Affine::new(1.0, 0.0, 0.0, 1.0, 30.0, 0.0),
                },
                DrawingNode::Clone {
                    registers: DrawingRegisters::default(),
                    original: 1,
                    matrix: Affine::from_array(
                        (kurbo::Affine::rotate(30_f64.to_radians()) * kurbo::Affine::FLIP_X)
                            .as_coeffs(),
                    ),
                },
                DrawingNode::Clone {
                    registers: DrawingRegisters::default(),
                    original: 3,
                    matrix: Affine::new(1.0, 0.0, 0.0, 1.0, 0.0, 70.0),
                },
                DrawingNode::Clone {
                    registers: named("scaled"),
                    original: 1,
                    matrix: Affine::new(2.0, 0.0, 0.0, 2.0, 0.0, 0.0),
                },
                marked,
                spaced_closed,
            ],
        },
        DrawingNode::Group {
            registers: DrawingRegisters {
                name: Some("Shapes".into()),
                locked: true,
                ..DrawingRegisters::default()
            },
            layer: true,
            children: vec![
                rotated(rect, 30.0, default()),
                rotated(plain_rect, 0.0, no_paint),
                rotated(plain_rect, -45.0, default()),
                rotated(ellipse(20.0, 10.0, None), 15.0, default()),
                rotated(ellipse(10.0, 10.0, None), 0.0, default()),
                rotated(ellipse(20.0, 20.0, arc(ArcKind::Pie)), 0.0, default()),
                rotated(ellipse(20.0, 10.0, arc(ArcKind::Arc)), 20.0, default()),
                rotated(ellipse(20.0, 20.0, arc(ArcKind::Chord)), 0.0, default()),
                rotated(polygon(0.0), 0.0, default()),
                rotated(polygon(0.3), 0.0, default()),
                rotated(star(0.0, 0.0), 0.0, default()),
                rotated(star(30.0, 0.0), 12.0, default()),
                rotated(star(-45.0, 0.0), 0.0, default()),
                rotated(star(-45.0, 0.25), 0.0, default()),
                DrawingNode::Group {
                    registers: named("one"),
                    layer: false,
                    children: vec![DrawingNode::Group {
                        registers: DrawingRegisters {
                            visible: false,
                            ..named("two")
                        },
                        layer: false,
                        children: vec![DrawingNode::Group {
                            registers: named("three"),
                            layer: false,
                            children: vec![rotated(polygon(0.0), 5.0, default())],
                        }],
                    }],
                },
            ],
        },
        DrawingNode::Group {
            registers: DrawingRegisters {
                visible: false,
                ..named("Hidden")
            },
            layer: true,
            children: Vec::new(),
        },
    ]
}

fn page(frames: Vec<DrawingPageFrame>) -> DrawingPage {
    DrawingPage {
        size: DocumentSize::from_mm(210.0, 297.0),
        display_unit: Some(DisplayUnit::Cm),
        background: Some(DocumentBackground {
            paint: BackgroundPaint::Solid,
            color: Color {
                r: 250,
                g: 240,
                b: 230,
            },
            opacity: Opacity::new(0.5).unwrap(),
        }),
        pages: frames,
    }
}

fn document(nodes: &[DrawingNode], page: &DrawingPage) -> Document {
    let ids: Vec<AnchorId> = (0..DrawingNode::anchor_count(nodes) as u64)
        .map(|n| AnchorId::new(9, n))
        .collect();
    Document::from_drawing(3, page, nodes, &ids, &[]).unwrap()
}

fn export(
    document: &Document,
    options: SvgExportOptions,
    selection: &[curvyo_document_core::NodeId],
) -> String {
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let laid = document.read_objects();
    let outcome = svg_export(
        document,
        &laid,
        &fonts,
        &ExportScope {
            selection,
            active_page: 0,
        },
        options,
    );
    curvyo_svg_core::write_svg(&outcome.svg)
}

fn open(text: &str) -> Document {
    let drawing = curvyo_svg_core::read_svg(text.as_bytes()).unwrap();
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let import = svg_import(&drawing, SvgPlacement::File, &fonts, u64::MAX);
    document(&import.nodes, &import.page)
}

/// One node as compared: kind, registers, numbers to compare within the tolerance, and the rest
/// as text compared exactly.
#[derive(Debug)]
struct Item {
    depth: usize,
    exact: String,
    numbers: Vec<f64>,
}

fn style_text(style: &Style) -> String {
    let mut s = style.clone();
    if !s.fill.enabled {
        s.fill = Style::default().fill;
        s.fill.enabled = false;
    }
    if !s.stroke.enabled {
        s.stroke = Style::default().stroke;
        s.stroke.enabled = false;
    }
    format!(
        "{:?} {:?} w={}",
        s.fill,
        s.stroke.markers,
        s.stroke.width.as_mm()
    ) + &format!(
        " {:?} {:?} {:?} {:?} {}",
        s.stroke.color,
        s.stroke.cap,
        s.stroke.join,
        s.stroke.dash.as_slice(),
        s.stroke.opacity.get()
    )
}

fn path_items(
    path: &curvyo_document_core::PathSnapshot,
    exact: &mut String,
    numbers: &mut Vec<f64>,
) {
    exact.push_str(" path ");
    exact.push_str(&style_text(&path.style));
    for s in path.subpaths() {
        exact.push_str(if s.closed { " closed" } else { " open" });
        for a in s.anchors {
            exact.push_str(&format!(" {:?}", a.kind));
            numbers.extend([
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

fn items(document: &Document) -> Vec<Item> {
    let nodes = document.read_nodes();
    let position = |id| nodes.iter().position(|n| n.id == id).unwrap_or(usize::MAX);
    nodes
        .iter()
        .map(|node| {
            let r = &node.registers;
            let mut exact = format!(
                "{:?} v={} l={} o={} g={}",
                r.name,
                r.visible,
                r.locked,
                r.opacity.get(),
                node.guide
            );
            let mut numbers = Vec::new();
            match &node.kind {
                NodeKind::Layer => exact.push_str(" layer"),
                NodeKind::Group => exact.push_str(" group"),
                NodeKind::Leaf(ObjectSnapshot::Path(path)) => {
                    path_items(path, &mut exact, &mut numbers);
                }
                NodeKind::Leaf(ObjectSnapshot::Primitive(p)) => {
                    exact.push_str(" primitive ");
                    exact.push_str(&style_text(&p.style));
                    numbers.push(p.rotation.as_radians());
                    match p.shape {
                        Shape::Rect {
                            bounds,
                            corner_radii,
                        } => {
                            exact.push_str(" rect");
                            numbers.extend([
                                bounds.origin.x,
                                bounds.origin.y,
                                bounds.width.as_mm(),
                                bounds.height.as_mm(),
                                corner_radii.tl.as_mm(),
                                corner_radii.tr.as_mm(),
                                corner_radii.br.as_mm(),
                                corner_radii.bl.as_mm(),
                            ]);
                        }
                        Shape::Ellipse { frame, arc, curve } => {
                            exact.push_str(&format!(
                                " ellipse {:?} {}",
                                arc.map(|a| a.kind),
                                curve.get()
                            ));
                            numbers.extend([
                                frame.center.x,
                                frame.center.y,
                                frame.rx.as_mm(),
                                frame.ry.as_mm(),
                            ]);
                            if let Some(a) = arc {
                                numbers.extend([a.start().as_radians(), a.end().as_radians()]);
                            }
                        }
                        Shape::Polygon {
                            frame,
                            point_count,
                            curve,
                        } => {
                            exact.push_str(&format!(
                                " polygon {} {}",
                                point_count.get(),
                                curve.get()
                            ));
                            numbers.extend([
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
                                " star {} {} {}",
                                point_count.get(),
                                curve.get(),
                                inner_ratio.get()
                            ));
                            numbers.extend([
                                frame.center.x,
                                frame.center.y,
                                frame.radius.as_mm(),
                                frame.angle.as_radians(),
                            ]);
                        }
                    }
                }
                NodeKind::Leaf(ObjectSnapshot::Clone(c))
                    if curvyo_svg_core::is_isometry(kurbo::Affine::new(
                        c.chain.matrix.to_array(),
                    )) =>
                {
                    exact.push_str(&format!(" clone of {}", position(c.chain.origin)));
                    numbers.extend(c.chain.matrix.to_array());
                }
                NodeKind::Leaf(ObjectSnapshot::Clone(c)) => {
                    // Written as the path of its resolved outline (criterion 59).
                    path_items(&c.resolved, &mut exact, &mut numbers);
                }
                NodeKind::Leaf(other) => exact.push_str(&format!(" {other:?}")),
            }
            Item {
                depth: node.depth,
                exact,
                numbers,
            }
        })
        .collect()
}

/// The two texts are equal; else the first differing line of each.
fn same_text(again: &str, first: &str) {
    if again != first {
        let line = again
            .lines()
            .zip(first.lines())
            .find(|(a, b)| a != b)
            .unwrap_or_default();
        panic!("second export differs:\n{}\n{}", line.0, line.1);
    }
}

fn assert_same(a: &Document, b: &Document) {
    let (left, right) = (items(a), items(b));
    assert_eq!(
        left.len(),
        right.len(),
        "node counts\n{left:#?}\n{right:#?}"
    );
    for (l, r) in left.iter().zip(&right) {
        assert_eq!(l.depth, r.depth);
        assert_eq!(l.exact, r.exact);
        assert_eq!(l.numbers.len(), r.numbers.len(), "{}", l.exact);
        for (x, y) in l.numbers.iter().zip(&r.numbers) {
            assert!((x - y).abs() <= GEOMETRY_MM, "{x} vs {y} in {}", l.exact);
        }
    }
    let (pa, pb) = (a.pages(), b.pages());
    assert_eq!(pa.len(), pb.len());
    for (x, y) in pa.iter().zip(&pb) {
        assert_eq!(x.name, y.name);
        assert!((x.size.width.as_mm() - y.size.width.as_mm()).abs() <= GEOMETRY_MM);
        assert!((x.size.height.as_mm() - y.size.height.as_mm()).abs() <= GEOMETRY_MM);
        let dx = (x.origin.x - pa[0].origin.x) - (y.origin.x - pb[0].origin.x);
        let dy = (x.origin.y - pa[0].origin.y) - (y.origin.y - pb[0].origin.y);
        assert!(dx.abs() <= GEOMETRY_MM && dy.abs() <= GEOMETRY_MM);
    }
    assert_eq!(a.display_unit(), b.display_unit());
    assert_eq!(a.background(), b.background());
}

/// Criterion 153: `curvyo_all.svg` (criterion 75, item 12) is the export of the generated
/// document, pinned byte for byte. Regenerate with `CURVYO_UPDATE_GOLDEN=1`.
fn pinned_curvyo_all(text: &str) {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-svg-core/tests/fixtures/export/curvyo_all.svg");
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, text).unwrap();
    }
    let want = std::fs::read_to_string(&path).unwrap();
    assert!(text == want, "curvyo_all.svg differs from the export");
}

#[test]
fn ac62_ac63_every_kind_comes_back_and_the_second_export_is_byte_identical() {
    let first = document(&everything(), &page(Vec::new()));
    let text = export(&first, SvgExportOptions::default(), &[]);
    pinned_curvyo_all(&text);
    let second = open(&text);
    assert_same(&first, &second);
    same_text(&export(&second, SvgExportOptions::default(), &[]), &text);
}

#[test]
fn ac59_use_for_isometries_without_override_paths_otherwise() {
    let first = document(&everything(), &page(Vec::new()));
    let text = export(&first, SvgExportOptions::default(), &[]);
    assert_eq!(text.matches("<use ").count(), 3, "{text}");
    assert!(text.contains("transform=\"matrix(1 0 0 1 30 0)\""));
}

#[test]
fn ac128_ac129_pages_come_back_relative_to_the_first() {
    let frame = |name: &str, x: f64, y: f64, w: f64, h: f64| DrawingPageFrame {
        name: name.into(),
        origin: Point::new(x, y),
        size: DocumentSize::from_mm(w, h),
    };
    let pages = vec![
        frame("Front", -50.0, 20.0, 100.0, 100.0),
        frame("Coaster", 70.0, 20.0, 100.0, 50.0),
    ];
    let rect = rotated(
        Shape::Rect {
            bounds: RectBounds {
                origin: Point::new(-40.0, 30.0),
                width: Length::from_mm(10.0),
                height: Length::from_mm(10.0),
            },
            corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
        },
        0.0,
        Style::default(),
    );
    let first = document(&[rect], &page(pages));
    let text = export(&first, SvgExportOptions::default(), &[]);
    assert!(text.contains("width=\"100mm\" height=\"100mm\" viewBox=\"0 0 100 100\""));
    assert!(text.contains("<inkscape:page id=\"page2\" x=\"120\" y=\"0\" width=\"100\" height=\"50\" inkscape:label=\"Coaster\"/>"));
    assert!(text.contains("x=\"10\" y=\"10\""), "{text}");
    let second = open(&text);
    assert_eq!(second.pages().len(), 2);
    assert_eq!(second.pages()[1].name, "Coaster");
    same_text(&export(&second, SvgExportOptions::default(), &[]), &text);
}

#[test]
fn ac72_the_locale_changes_nothing() {
    let first = document(&everything(), &page(Vec::new()));
    let a = export(&first, SvgExportOptions::default(), &[]);
    // Rust's number formatting reads no locale (no `LC_ALL` is set here); two exports give the
    // same bytes and no decimal comma.
    let b = export(&first, SvgExportOptions::default(), &[]);
    assert_eq!(a, b);
    assert!(!a.contains(','), "no decimal comma");
}

fn arb_point() -> impl Strategy<Value = (f64, f64)> {
    (-500.0..500.0f64, -500.0..500.0f64)
}

fn arb_path() -> impl Strategy<Value = DrawingNode> {
    let anchor = (arb_point(), arb_point(), arb_point(), 0..3u8).prop_map(|(p, hin, hout, k)| {
        let kind = [
            AnchorKind::Corner,
            AnchorKind::Symmetric,
            AnchorKind::Asymmetric,
        ][k as usize];
        let zero = |h: (f64, f64)| {
            if k == 0 && h.0 > 0.0 {
                (0.0, 0.0)
            } else {
                (h.0 / 10.0, h.1 / 10.0)
            }
        };
        anchor(p.0, p.1, zero(hin), zero(hout), kind)
    });
    let outline =
        (prop::collection::vec(anchor, 2..6), any::<bool>()).prop_map(|(mut anchors, closed)| {
            // An open outline's outer handles draw nothing; SVG cannot hold them.
            if !closed {
                anchors[0].handle_in = Vec2::ZERO;
                let last = anchors.len() - 1;
                anchors[last].handle_out = Vec2::ZERO;
            }
            DrawingOutline { closed, anchors }
        });
    (
        prop::collection::vec(outline, 1..4),
        0.01..5.0f64,
        prop::collection::vec(0.1..5.0f64, 0..5),
    )
        .prop_map(|(outlines, width, dash)| DrawingNode::Path {
            registers: DrawingRegisters::default(),
            outlines,
            style: stroke_style(width, &dash, LineCap::Round, LineJoin::Bevel),
        })
}

fn arb_primitive() -> impl Strategy<Value = DrawingNode> {
    (
        arb_point(),
        1.0..100.0f64,
        0.05..0.95f64,
        3..12u32,
        -180.0..180.0f64,
        -180.0..180.0f64,
        0..5u8,
    )
        .prop_map(|(c, r, ratio, n, angle, rot, kind)| {
            let shape = match kind {
                0 => Shape::Rect {
                    bounds: RectBounds {
                        origin: Point::new(c.0, c.1),
                        width: Length::from_mm(r),
                        height: Length::from_mm(r * ratio),
                    },
                    corner_radii: CornerRadii::uniform(Length::from_mm(r * ratio / 4.0)),
                },
                1 => Shape::Ellipse {
                    frame: EllipseFrame {
                        center: Point::new(c.0, c.1),
                        rx: Length::from_mm(r),
                        ry: Length::from_mm(r * ratio),
                    },
                    arc: None,
                    curve: Curve::ELLIPSE_DEFAULT,
                },
                2 => Shape::Polygon {
                    frame: star_frame(c.0, c.1, r, angle),
                    point_count: PointCount::new(n).unwrap(),
                    curve: Curve::POLYGON_DEFAULT,
                },
                3 => Shape::Star {
                    frame: star_frame(c.0, c.1, r, angle),
                    point_count: PointCount::new(n).unwrap(),
                    inner_ratio: InnerRatio::new(ratio).unwrap(),
                    curve: Curve::new(ratio / 2.0).unwrap(),
                },
                _ => Shape::Ellipse {
                    frame: EllipseFrame {
                        center: Point::new(c.0, c.1),
                        rx: Length::from_mm(r),
                        ry: Length::from_mm(r * ratio),
                    },
                    arc: EllipseArc::new(
                        Angle::from_radians(angle.to_radians()),
                        Angle::from_radians(rot.to_radians()),
                        ArcKind::Chord,
                    ),
                    curve: Curve::ELLIPSE_DEFAULT,
                },
            };
            rotated(shape, rot, Style::default())
        })
}

fn arb_document() -> impl Strategy<Value = Vec<DrawingNode>> {
    let leaf = prop_oneof![arb_path(), arb_primitive()];
    prop::collection::vec(prop::collection::vec(leaf, 0..6), 1..4).prop_map(|layers| {
        layers
            .into_iter()
            .enumerate()
            .map(|(i, children)| DrawingNode::Group {
                registers: DrawingRegisters {
                    visible: i % 2 == 0,
                    locked: i == 1,
                    ..named(&format!("Layer {i}"))
                },
                layer: true,
                children,
            })
            .collect()
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn ac62_ac63_random_documents_come_back(nodes in arb_document()) {
        let first = document(&nodes, &page(Vec::new()));
        let text = export(&first, SvgExportOptions::default(), &[]);
        let second = open(&text);
        assert_same(&first, &second);
        same_text(&export(&second, SvgExportOptions::default(), &[]), &text);
    }
}
