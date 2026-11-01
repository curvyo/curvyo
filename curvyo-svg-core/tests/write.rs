//! The SVG writer against hand-built drawings (`specs/0024-svg-import-export` slice 4,
//! `adrs.md` export §6): criterion 52's skeleton byte for byte, and goldens under
//! `tests/fixtures/export/` for ids, pages, markers, guides, text and images. Regenerate with
//! `CURVYO_UPDATE_GOLDEN=1` and review the diff.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::items_after_statements
)]

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_svg_core::{
    ArcData, Cap, DisplayUnitName, ExportImage, ExportKind, ExportLeaf, ExportLine, ExportNode,
    ExportPage, ExportState, ExportStroke, ExportStyle, ExportText, ExportTextPath, Join,
    MarkerShape, PageBackground, Paint, PathId, Shadow, ShadowKind, ShadowMarkers, StarData,
    SvgExport, SvgNode, TextAnchor, read_svg, write_svg,
};
use kurbo::{Affine, BezPath, Point, Rect};

fn black_stroke(width: f64) -> ExportStroke {
    ExportStroke {
        paint: Paint {
            rgb: [0, 0, 0],
            opacity: 1.0,
        },
        width_mm: width,
        dash_mm: Vec::new(),
        cap: Cap::Butt,
        join: Join::Miter,
        start: None,
        mid: None,
        end: None,
    }
}

fn named(name: &str) -> ExportState {
    ExportState {
        name: Some(name.to_owned()),
        ..ExportState::default()
    }
}

fn leaf(kind: ExportKind, style: ExportStyle) -> ExportNode {
    ExportNode::Leaf {
        state: ExportState::default(),
        leaf: Box::new(ExportLeaf {
            kind,
            style,
            shadow: Shadow::default(),
        }),
    }
}

fn a4(nodes: Vec<ExportNode>) -> SvgExport {
    SvgExport {
        size: (210.0, 297.0),
        pages: vec![ExportPage {
            name: "Page 1".into(),
            rect: Rect::new(0.0, 0.0, 210.0, 297.0),
        }],
        display_unit: DisplayUnitName::Mm,
        background: PageBackground::None,
        nodes,
        guides: Vec::new(),
    }
}

fn golden(name: &str, text: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/export")
        .join(name);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
    }
    let want = std::fs::read_to_string(&path).expect("golden exists");
    assert_eq!(text, want, "{name} differs from its golden");
}

#[test]
fn ac52_the_skeleton_byte_for_byte() {
    let rect = leaf(
        ExportKind::Rect {
            rect: Rect::new(10.0, 20.0, 40.0, 60.0),
            radius: 0.0,
            rotation_deg: 0.0,
        },
        ExportStyle {
            fill: None,
            stroke: Some(black_stroke(0.25)),
        },
    );
    let drawing = a4(vec![ExportNode::Group {
        state: named("Cut"),
        layer: true,
        children: vec![rect],
    }]);
    let want = r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"
     xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" xmlns:curvyo="urn:curvyo:svg:1"
     width="210mm" height="297mm" viewBox="0 0 210 297" curvyo:format="1">
  <sodipodi:namedview id="namedview1" inkscape:document-units="mm" pagecolor="#ffffff" inkscape:pageopacity="0"/>
  <g id="layer1" inkscape:groupmode="layer" inkscape:label="Cut">
    <rect id="rect1" x="10" y="20" width="30" height="40" fill="none" stroke="#000000" stroke-width="0.25"/>
  </g>
</svg>
"##;
    assert_eq!(write_svg(&drawing), want);
}

#[test]
fn ac52_an_empty_drawing_is_valid_and_reads_back() {
    let text = write_svg(&a4(Vec::new()));
    let read = read_svg(text.as_bytes()).unwrap();
    assert_eq!(read.page, Some((210.0, 297.0)));
    assert_eq!(read.nodes, Vec::new());
}

fn kitchen_sink() -> SvgExport {
    let mut path = BezPath::new();
    path.move_to((0.0, 0.0));
    path.line_to((10.0, -0.0));
    path.curve_to((11.5, 0.0), (13.0, 1.2), (14.0, 2.000_000_4));
    path.close_path();
    path.move_to((1e-7, 0.1 + 0.2));
    path.line_to((1e7, -1e7));
    let mut marked = black_stroke(0.5);
    marked.start = Some(MarkerShape::Arrow);
    marked.end = Some(MarkerShape::Arrow);
    marked.mid = Some(MarkerShape::Dot);
    marked.dash_mm = vec![1.0, 0.0, 0.5];
    marked.cap = Cap::Round;
    marked.join = Join::Bevel;
    marked.paint.opacity = 0.5;
    let mut line = BezPath::new();
    line.move_to((0.0, 0.0));
    line.line_to((100.0, 0.0));
    let star = StarData {
        sides: 5,
        center: Point::new(50.0, 50.0),
        r1: 10.0,
        r2: Some(4.0),
        arg1: -std::f64::consts::FRAC_PI_2,
        arg2: Some(-std::f64::consts::FRAC_PI_2 + std::f64::consts::PI / 5.0),
        flat: false,
    };
    let arc = ArcData {
        center: Point::new(50.0, 50.0),
        rx: 20.0,
        ry: 20.0,
        start: Some(0.0),
        end: Some(std::f64::consts::FRAC_PI_2),
        arc_type: Some("slice".into()),
        open: false,
    };
    let mut pie = BezPath::new();
    pie.move_to((50.0, 50.0));
    pie.line_to((70.0, 50.0));
    pie.curve_to((70.0, 61.0), (61.0, 70.0), (50.0, 70.0));
    pie.close_path();
    let fill = ExportStyle {
        fill: Some(Paint {
            rgb: [0x12, 0xab, 0xef],
            opacity: 0.25,
        }),
        stroke: None,
    };
    let shadowed = ExportNode::Leaf {
        state: ExportState {
            name: Some("Odd & <named> \u{202e}\"x\"".into()),
            hidden: true,
            locked: true,
            opacity: 0.3,
            key: Some(7),
        },
        leaf: Box::new(ExportLeaf {
            kind: ExportKind::Path {
                path: path.clone(),
                node_types: "cccc".into(),
                id: PathId::Rect,
            },
            style: fill.clone(),
            shadow: Shadow {
                dash: Some(vec![2.0, 0.1 + 0.2]),
                stroke_width: Some(0.1 + 0.2),
                radii: Some([1.0, 2.0, 3.0, 4.0]),
                kind: Some(ShadowKind::Rectangle),
                frame: Some(vec![5.0, 6.0, 7.0, 8.0, -0.0]),
                markers: Some(ShadowMarkers {
                    start: None,
                    middle: Some(MarkerShape::Arrow),
                    at_nodes: false,
                    count: 3,
                    end: Some(MarkerShape::Dot),
                }),
            },
        }),
    };
    let image_bytes: Arc<[u8]> = Arc::from(&b"\x89PNG\r\n\x1a\nxyz"[..]);
    let text = ExportText {
        transform: Affine::rotate(0.5),
        family: "Source Sans 3".into(),
        weight: 700,
        italic: true,
        stretch: Some("condensed"),
        font_size: 10.583_8,
        anchor: TextAnchor::Middle,
        letter_spacing: 0.5,
        word_spacing: 0.0,
        line_height: Some(1.25),
        writing_mode: None,
        orientation: None,
        style: fill.clone(),
        size: 6.943,
        face: format!("Source Sans 3|Bold Italic|{}", "ab".repeat(32)),
        lines: vec![
            ExportLine {
                x: 20.0,
                y: 30.0,
                text: "B&o  ho".into(),
                dx: vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                dy: Vec::new(),
                rotate: vec![0.0, 0.0, 90.0, 0.0, 0.0, 0.0, 0.0],
                on_path: None,
            },
            ExportLine {
                x: 20.0,
                y: 40.0,
                text: String::new(),
                dx: Vec::new(),
                dy: Vec::new(),
                rotate: Vec::new(),
                on_path: None,
            },
        ],
    };
    let on_path = ExportText {
        transform: Affine::IDENTITY,
        anchor: TextAnchor::Middle,
        lines: vec![ExportLine {
            x: 0.0,
            y: 0.0,
            text: "CURVYO".into(),
            dx: Vec::new(),
            dy: vec![-2.0],
            rotate: Vec::new(),
            on_path: Some(ExportTextPath {
                carrier: 7,
                start_offset: 75.4,
                side_right: true,
            }),
        }],
        ..text.clone()
    };
    let nodes = vec![
        ExportNode::Group {
            state: named("Cut"),
            layer: true,
            children: vec![
                ExportNode::Group {
                    state: ExportState {
                        opacity: 0.5,
                        ..ExportState::default()
                    },
                    layer: false,
                    children: vec![
                        leaf(
                            ExportKind::Use {
                                target: 7,
                                matrix: Affine::new([0.0, 1.0, -1.0, 0.0, 30.0, 0.1 + 0.2]),
                            },
                            ExportStyle::default(),
                        ),
                        shadowed,
                    ],
                },
                leaf(
                    ExportKind::Path {
                        path: line,
                        node_types: "cc".into(),
                        id: PathId::Path,
                    },
                    ExportStyle {
                        fill: None,
                        stroke: Some(marked),
                    },
                ),
                leaf(
                    ExportKind::Ellipse {
                        center: Point::new(5.0, 6.0),
                        rx: 3.0,
                        ry: 3.0,
                        rotation_deg: 0.0,
                    },
                    fill.clone(),
                ),
                leaf(
                    ExportKind::Ellipse {
                        center: Point::new(5.0, 6.0),
                        rx: 3.0,
                        ry: 2.0,
                        rotation_deg: -30.0,
                    },
                    fill.clone(),
                ),
                leaf(
                    ExportKind::Rect {
                        rect: Rect::new(0.0, 0.0, 4.0, 2.0),
                        radius: 0.5,
                        rotation_deg: 45.0,
                    },
                    fill.clone(),
                ),
                leaf(
                    ExportKind::Star {
                        star,
                        path: pie.clone(),
                        node_types: String::new(),
                    },
                    fill.clone(),
                ),
                leaf(
                    ExportKind::Arc {
                        arc,
                        path: pie,
                        rotation_deg: 30.0,
                    },
                    fill.clone(),
                ),
                ExportNode::Text {
                    state: named("Boho"),
                    text: Box::new(text),
                },
                ExportNode::Text {
                    state: ExportState::default(),
                    text: Box::new(on_path),
                },
                ExportNode::Image {
                    state: named("Photo"),
                    image: Box::new(ExportImage {
                        mime: "image/png",
                        bytes: image_bytes.clone(),
                        placement: Affine::new([20.0, 0.0, 0.0, 10.0, 10.0, 10.0]),
                    }),
                },
                ExportNode::Image {
                    state: ExportState::default(),
                    image: Box::new(ExportImage {
                        mime: "image/png",
                        bytes: image_bytes,
                        placement: Affine::new([-20.0, 0.0, 0.0, 10.0, 10.0, 10.0]),
                    }),
                },
            ],
        },
        ExportNode::Group {
            state: named("Empty"),
            layer: true,
            children: Vec::new(),
        },
    ];
    let mut circle = BezPath::new();
    circle.move_to((0.0, 0.0));
    circle.line_to((1.0, 1.0));
    SvgExport {
        size: (220.0, 100.0),
        pages: vec![
            ExportPage {
                name: "Front".into(),
                rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            },
            ExportPage {
                name: "Coaster <2>".into(),
                rect: Rect::new(120.0, 0.0, 220.0, 50.0),
            },
        ],
        display_unit: DisplayUnitName::Cm,
        background: PageBackground::Colour([0xff, 0xee, 0xdd], 0.5),
        nodes,
        guides: vec![leaf(
            ExportKind::Path {
                path: circle,
                node_types: String::new(),
                id: PathId::Path,
            },
            ExportStyle {
                fill: None,
                stroke: Some(black_stroke(0.1)),
            },
        )],
    }
}

#[test]
fn ac53_ac128_ac141_ac143_every_element_kind_golden() {
    let text = write_svg(&kitchen_sink());
    golden("kitchen_sink.svg", &text);
}

#[test]
fn ac72_two_writes_give_the_same_bytes() {
    assert_eq!(write_svg(&kitchen_sink()), write_svg(&kitchen_sink()));
}

#[test]
fn ac52_the_file_reads_back() {
    let text = write_svg(&kitchen_sink());
    let read = read_svg(text.as_bytes()).unwrap();
    assert_eq!(read.page, Some((220.0, 100.0)));
    assert_eq!(read.display_unit, Some(DisplayUnitName::Cm));
    let SvgNode::Group { children, .. } = &read.nodes[0] else {
        panic!("a layer first")
    };
    assert!(children.len() >= 8);
}

fn leaves(nodes: &[SvgNode], out: &mut Vec<curvyo_svg_core::SvgLeaf>) {
    for node in nodes {
        match node {
            SvgNode::Group { children, .. } => leaves(children, out),
            SvgNode::Leaf { leaf, .. } => out.push(leaf.clone()),
            _ => {}
        }
    }
}

#[test]
fn ac57_ac129_ac144_the_reader_takes_the_profile_back() {
    let text = write_svg(&kitchen_sink());
    let read = read_svg(text.as_bytes()).unwrap();
    assert_eq!(read.pages.len(), 2);
    assert_eq!(read.pages[1].name.as_deref(), Some("Coaster <2>"));
    assert_eq!(read.pages[1].rect, Rect::new(120.0, 0.0, 220.0, 50.0));
    let mut all = Vec::new();
    leaves(&read.nodes, &mut all);
    let shadowed = all
        .iter()
        .find(|l| l.shadow.kind.is_some())
        .expect("the shadowed leaf");
    assert_eq!(shadowed.shadow.radii, Some([1.0, 2.0, 3.0, 4.0]));
    assert_eq!(shadowed.shadow.stroke_width, Some(0.1 + 0.2));
    assert_eq!(shadowed.shadow.dash, Some(vec![2.0, 0.1 + 0.2]));
    assert_eq!(shadowed.shadow.markers.map(|m| m.count), Some(3));
    let marked = all.iter().find(|l| l.marker_refs[2]).expect("marked line");
    assert_eq!(marked.marker_refs, [true, true, true]);
    let guides = read.nodes.iter().filter(|n| n.state().guides_layer).count();
    assert_eq!(guides, 1);
    let mut texts = Vec::new();
    fn find_texts(nodes: &[SvgNode], out: &mut Vec<curvyo_svg_core::SvgText>) {
        for node in nodes {
            match node {
                SvgNode::Group { children, .. } => find_texts(children, out),
                SvgNode::Text { text, .. } => out.push((**text).clone()),
                _ => {}
            }
        }
    }
    find_texts(&read.nodes, &mut texts);
    assert_eq!(texts[0].size, Some(6.943));
    assert!(
        texts[0]
            .face
            .as_deref()
            .unwrap()
            .starts_with("Source Sans 3|Bold Italic|")
    );
    assert_eq!(texts[0].styles[texts[0].root_style].weight, 700);
}

#[test]
fn ac57_without_the_format_the_profile_is_ignored() {
    let text = write_svg(&kitchen_sink()).replace(" curvyo:format=\"1\"", "");
    let read = read_svg(text.as_bytes()).unwrap();
    let mut all = Vec::new();
    leaves(&read.nodes, &mut all);
    assert!(all.iter().all(|l| l.shadow == Shadow::default()));
    assert!(read.nodes.iter().all(|n| !n.state().guides_layer));
}
