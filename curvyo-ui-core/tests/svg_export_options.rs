//! The SVG export's options, guides, markers, images and plain-SVG fallback
//! (`specs/0024-svg-import-export` criteria 61, 130, 131, 137, 141, 143, 144, 145 and 146), and
//! no dimension in it (`specs/0070-measure-and-dimension` criterion 9).
#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::similar_names
)]

use std::sync::Arc;

use curvyo_document_core::{
    AnchorId, AnchorKind, Angle, CornerRadii, DashPattern, DimensionKind, DimensionPlace,
    DimensionSlots, Document, DocumentSize, DrawingAnchor, DrawingNode, DrawingOutline,
    DrawingPage, DrawingPageFrame, DrawingRegisters, Length, MarkerPlace, MarkerShape, NodeId,
    Point, PointSlot, RectBounds, Shape, Style, Vec2,
};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{
    ExportScope, ImportFaces, SvgExportOptions, SvgExportOutcome, SvgPlacement, svg_export,
    svg_import,
};

fn corner(x: f64, y: f64) -> DrawingAnchor {
    DrawingAnchor {
        point: Point::new(x, y),
        handle_in: Vec2::ZERO,
        handle_out: Vec2::ZERO,
        kind: AnchorKind::Corner,
    }
}

fn line(a: (f64, f64), b: (f64, f64), style: Style, registers: DrawingRegisters) -> DrawingNode {
    DrawingNode::Path {
        registers,
        outlines: vec![DrawingOutline {
            closed: false,
            anchors: vec![corner(a.0, a.1), corner(b.0, b.1)],
        }],
        style,
    }
}

fn rect(x: f64, y: f64, w: f64, h: f64, registers: DrawingRegisters) -> DrawingNode {
    DrawingNode::Primitive {
        registers,
        shape: Shape::Rect {
            bounds: RectBounds {
                origin: Point::new(x, y),
                width: Length::from_mm(w),
                height: Length::from_mm(h),
            },
            corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
        },
        rotation: Angle::from_radians(0.0),
        style: Style::default(),
    }
}

fn named(name: &str) -> DrawingRegisters {
    DrawingRegisters {
        name: Some(name.to_owned()),
        ..DrawingRegisters::default()
    }
}

fn layer(name: &str, children: Vec<DrawingNode>) -> DrawingNode {
    DrawingNode::Group {
        registers: named(name),
        layer: true,
        children,
    }
}

fn document(
    nodes: &[DrawingNode],
    frames: Vec<DrawingPageFrame>,
    assets: &[Arc<[u8]>],
) -> Document {
    let ids: Vec<AnchorId> = (0..DrawingNode::anchor_count(nodes) as u64)
        .map(|n| AnchorId::new(9, n))
        .collect();
    let page = DrawingPage {
        size: DocumentSize::from_mm(210.0, 297.0),
        display_unit: None,
        background: None,
        pages: frames,
    };
    Document::from_drawing(3, &page, nodes, &ids, assets).unwrap()
}

fn export_with(
    document: &Document,
    options: SvgExportOptions,
    selection: &[NodeId],
    active_page: usize,
) -> (SvgExportOutcome, String) {
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
            active_page,
        },
        options,
    );
    let text = curvyo_svg_core::write_svg(&outcome.svg);
    (outcome, text)
}

fn open(text: &str) -> Vec<DrawingNode> {
    let drawing = curvyo_svg_core::read_svg(text.as_bytes()).unwrap();
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    svg_import(&drawing, SvgPlacement::File, &fonts, u64::MAX).nodes
}

fn guide(registers: DrawingRegisters) -> DrawingRegisters {
    DrawingRegisters {
        guide: true,
        ..registers
    }
}

#[test]
fn ac143_ac144_guides_leave_their_layers_and_come_back_as_guides() {
    let circle = |x: f64, name: &str| DrawingNode::Primitive {
        registers: guide(named(name)),
        shape: Shape::Ellipse {
            frame: curvyo_document_core::EllipseFrame {
                center: Point::new(x, 50.0),
                rx: Length::from_mm(10.0),
                ry: Length::from_mm(10.0),
            },
            arc: None,
            curve: curvyo_document_core::Curve::ELLIPSE_DEFAULT,
        },
        rotation: Angle::from_radians(0.0),
        style: Style::default(),
    };
    let nodes = vec![
        layer(
            "Cut",
            vec![circle(30.0, "g1"), rect(10.0, 10.0, 5.0, 5.0, named("cut"))],
        ),
        layer("Only guides", vec![circle(80.0, "g2")]),
    ];
    let (_, text) = export_with(
        &document(&nodes, Vec::new(), &[]),
        SvgExportOptions::default(),
        &[],
        0,
    );
    assert!(!text.contains("Only guides"), "{text}");
    let guides_at = text.find("curvyo:guides=\"1\"").unwrap();
    assert!(text[guides_at..].contains("inkscape:label=\"g1\""));
    assert!(text[guides_at..].contains("inkscape:label=\"g2\""));
    assert!(!text[..guides_at].contains("<circle"));
    let back = open(&text);
    assert_eq!(back.len(), 2);
    let DrawingNode::Group {
        registers,
        children,
        ..
    } = &back[1]
    else {
        panic!()
    };
    assert_eq!(registers.name.as_deref(), Some("Guides"));
    assert!(registers.visible && !registers.locked);
    assert_eq!(children.len(), 2);
    for child in children {
        let DrawingNode::Primitive { registers, .. } = child else {
            panic!()
        };
        assert!(registers.guide);
    }
}

#[test]
fn ac130_active_page_keeps_what_overlaps_whole_and_moves_the_origin() {
    let frame = |name: &str, x: f64| DrawingPageFrame {
        name: name.into(),
        origin: Point::new(x, 0.0),
        size: DocumentSize::from_mm(100.0, 100.0),
    };
    let pages = vec![frame("A", 0.0), frame("B", 120.0), frame("C", 240.0)];
    let nodes = vec![layer(
        "L",
        vec![
            rect(10.0, 10.0, 10.0, 10.0, named("on A")),
            rect(130.0, 10.0, 10.0, 10.0, named("on B")),
            rect(250.0, 10.0, 10.0, 10.0, named("on C")),
            rect(90.0, 50.0, 40.0, 10.0, named("straddle")),
            rect(10.0, 200.0, 10.0, 10.0, named("pasteboard")),
            line(
                (150.0, 100.0),
                (170.0, 100.0),
                Style::default(),
                named("edge line"),
            ),
            rect(100.0, 80.0, 20.0, 10.0, named("touching")),
        ],
    )];
    let doc = document(&nodes, pages, &[]);
    let options = SvgExportOptions {
        active_page_only: true,
        ..SvgExportOptions::default()
    };
    let (outcome, text) = export_with(&doc, options, &[], 1);
    for kept in ["on B", "straddle", "edge line"] {
        assert!(text.contains(kept), "{kept} missing");
    }
    for left in ["on A", "on C", "pasteboard", "touching"] {
        assert!(!text.contains(left), "{left} written");
    }
    assert!(text.contains("width=\"100mm\" height=\"100mm\""));
    assert!(!text.contains("<inkscape:page"));
    assert!(
        text.contains("x=\"10\" y=\"10\""),
        "B's object at B's corner"
    );
    assert!(text.contains("x=\"-30\""), "straddle unclipped");
    assert_eq!(outcome.objects, 3);
}

#[test]
fn ac131_selection_only_writes_the_selection_in_its_layers() {
    let nodes = vec![
        layer(
            "One",
            vec![
                line((0.0, 0.0), (10.0, 0.0), Style::default(), named("original")),
                DrawingNode::Clone {
                    registers: named("clone"),
                    original: 0,
                    matrix: curvyo_document_core::Affine::new(1.0, 0.0, 0.0, 1.0, 0.0, 20.0),
                },
                rect(0.0, 40.0, 5.0, 5.0, named("picked")),
            ],
        ),
        layer("Two", vec![rect(0.0, 60.0, 5.0, 5.0, named("other"))]),
    ];
    let doc = document(&nodes, Vec::new(), &[]);
    let reads = doc.read_nodes();
    let id = |name: &str| {
        reads
            .iter()
            .find(|n| n.registers.name.as_deref() == Some(name))
            .unwrap()
            .id
    };
    let options = SvgExportOptions {
        selection_only: true,
        ..SvgExportOptions::default()
    };
    let (outcome, text) = export_with(&doc, options, &[id("clone"), id("picked")], 0);
    assert_eq!(outcome.objects, 2);
    assert!(
        !text.contains("<use"),
        "the clone's original is out of scope"
    );
    assert!(text.contains("inkscape:label=\"clone\""));
    assert!(!text.contains("original") && !text.contains("other") && !text.contains("Two"));
}

#[test]
fn ac141_markers_are_defined_and_referenced_only_where_svg_draws_them() {
    let mut style = Style::default();
    style.stroke.width = Length::from_mm(0.5);
    style.stroke.markers.end = MarkerShape::Arrow;
    style.stroke.markers.start = MarkerShape::Arrow;
    let mut spaced = style.clone();
    spaced.stroke.markers.mid = MarkerShape::Dot;
    spaced.stroke.markers.mid_place = MarkerPlace::Spaced;
    spaced.stroke.markers.mid_count = curvyo_document_core::MarkerCount::new(3).unwrap();
    let nodes = vec![layer(
        "L",
        vec![
            line((0.0, 0.0), (100.0, 0.0), style, named("double")),
            line((0.0, 10.0), (100.0, 10.0), spaced, named("spaced")),
        ],
    )];
    let (_, text) = export_with(
        &document(&nodes, Vec::new(), &[]),
        SvgExportOptions::default(),
        &[],
        0,
    );
    assert_eq!(text.matches("<marker ").count(), 2, "start and end arrows");
    assert!(
        text.contains("M 2 -1.5 L -2 0 L 2 1.5 Z"),
        "the start arrow is mirrored"
    );
    assert!(text.contains("marker-end=\"url(#marker2)\""));
    assert!(
        !text.contains("marker-mid"),
        "spaced middle markers have no SVG form"
    );
    assert!(
        text.contains("curvyo:markers=\"start=arrow middle=dot place=spaced count=3 end=arrow\"")
    );
    // Criterion 142: the maker removes marker-end; the End slot comes back none.
    let edited = text.replacen(" marker-end=\"url(#marker2)\"", "", 1);
    let back = open(&edited);
    let DrawingNode::Group { children, .. } = &back[0] else {
        panic!()
    };
    let DrawingNode::Path { style, .. } = &children[0] else {
        panic!()
    };
    assert_eq!(style.stroke.markers.end, MarkerShape::None);
    assert_eq!(style.stroke.markers.start, MarkerShape::Arrow);
}

#[test]
fn ac146_strokes_as_paths_splits_fill_and_stroke() {
    let mut dashed = Style::default();
    dashed.stroke.width = Length::from_mm(1.0);
    dashed.stroke.dash = DashPattern::new(vec![5.0, 5.0]).unwrap();
    let mut filled = Style::default();
    filled.fill.enabled = true;
    let nodes = vec![layer(
        "L",
        vec![line((0.0, 0.0), (100.0, 0.0), dashed, named("dashed")), {
            let mut node = rect(0.0, 10.0, 20.0, 10.0, named("box"));
            if let DrawingNode::Primitive { style, .. } = &mut node {
                *style = filled;
            }
            node
        }],
    )];
    let options = SvgExportOptions {
        strokes_as_paths: true,
        ..SvgExportOptions::default()
    };
    let (_, text) = export_with(&document(&nodes, Vec::new(), &[]), options, &[], 0);
    assert!(!text.contains("stroke-dasharray"));
    assert!(!text.contains("stroke-width"));
    assert!(text.contains("<rect id=\"rect1\" x=\"0\" y=\"10\""));
    let dashes = text
        .lines()
        .find(|l| l.contains("<path") && l.contains("compound"))
        .unwrap();
    assert_eq!(dashes.matches('M').count(), 10, "ten dash outlines");
}

#[test]
fn ac145_the_grid_and_origin_corner_change_nothing() {
    let nodes = vec![layer("L", vec![rect(10.0, 20.0, 5.0, 5.0, named("r"))])];
    let doc = document(&nodes, Vec::new(), &[]);
    let (_, before) = export_with(&doc, SvgExportOptions::default(), &[], 0);
    assert!(before.contains("x=\"10\" y=\"20\""));
    let page = doc.pages()[0].id.clone();
    let mut grid = doc.pages()[0].grid;
    grid.shown = !grid.shown;
    let _ = doc.set_page_grid(&page, grid);
    let (_, after) = export_with(&doc, SvgExportOptions::default(), &[], 0);
    assert_eq!(before, after);
}

#[test]
fn a_dimension_is_not_exported() {
    let nodes = vec![line(
        (10.0, 10.0),
        (60.0, 10.0),
        Style::default(),
        named("l"),
    )];
    let doc = document(&nodes, Vec::new(), &[]);
    let (_, before) = export_with(&doc, SvgExportOptions::default(), &[], 0);
    let path = doc.object_ids()[0];
    let anchor = |n| PointSlot::Anchor {
        node: path,
        anchor: AnchorId::new(9, n),
    };
    doc.add_dimension(
        DimensionKind::Aligned,
        DimensionSlots::Points(vec![anchor(0), anchor(1)]),
        DimensionPlace::at_offset(Length::from_mm(5.0)),
    )
    .unwrap();
    assert_eq!(doc.object_ids().len(), 2);
    let (_, after) = export_with(&doc, SvgExportOptions::default(), &[], 0);
    assert_eq!(before, after);
}

#[test]
fn ac61_without_the_three_namespaces_the_outlines_stay() {
    let nodes = vec![layer(
        "L",
        vec![
            rect(10.0, 20.0, 30.0, 40.0, named("r")),
            line((0.0, 0.0), (50.0, 25.0), Style::default(), named("l")),
        ],
    )];
    let (_, text) = export_with(
        &document(&nodes, Vec::new(), &[]),
        SvgExportOptions::default(),
        &[],
        0,
    );
    let mut stripped = text.clone();
    for prefix in [" inkscape:", " sodipodi:", " curvyo:"] {
        while let Some(at) = stripped.find(prefix) {
            let value = stripped[at..].find("=\"").unwrap() + at + 2;
            let end = stripped[value..].find('"').unwrap() + value + 1;
            stripped.replace_range(at..end, "");
        }
    }
    assert!(!stripped.contains("curvyo:format"));
    let plain = curvyo_svg_core::read_svg(stripped.as_bytes()).unwrap();
    let full = curvyo_svg_core::read_svg(text.as_bytes()).unwrap();
    let (a, b) = (plain.bounds().unwrap(), full.bounds().unwrap());
    assert!((a.x0 - b.x0).abs() < 0.01 && (a.y1 - b.y1).abs() < 0.01);
}
