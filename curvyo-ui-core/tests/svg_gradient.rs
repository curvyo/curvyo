//! SVG export and import of gradient fills (`specs/0124-gradient-fill` criteria 13, 14, 27):
//! the export goldens `curvyo-svg-core/tests/fixtures/export/gradient_*.svg` are made here from
//! documents, read back by the reader and opened again with the same picture, and two exports
//! are byte-identical. Regenerate with `CURVYO_UPDATE_GOLDEN=1` and review the diff.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, AnchorId, Angle, AxisRatio, BoxPoint, Color, Document, EllipseFrame, FillPaint,
    FontKind, FontRef, GradientGeometry, GradientStop, GradientStops, Length, NewAnchor, NodeId,
    ObjectRead, ObjectSnapshot, PathSnapshot, PlacedGradient, Point, RectBounds, StopOffset,
    StyleEdit, TextLayoutValues, TextOutline,
};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{
    ExportScope, ImportFaces, SvgExportOptions, SvgPlacement, gradient_loss_lines, svg_export,
    svg_import,
};

fn stop(r: u8, g: u8, b: u8, alpha: u8, offset: f64) -> GradientStop {
    GradientStop {
        colour: Color { r, g, b },
        alpha,
        offset: StopOffset::new(offset).unwrap(),
    }
}

fn shade(document: &Document, id: NodeId, geometry: GradientGeometry, stops: Vec<GradientStop>) {
    for edit in [
        StyleEdit::FillPaint(FillPaint::Gradient),
        StyleEdit::GradientGeometry(geometry),
        StyleEdit::GradientStops(GradientStops::new(stops).unwrap()),
    ] {
        document.edit_style(&[id], &edit).unwrap();
    }
}

fn two_stops() -> Vec<GradientStop> {
    vec![stop(255, 0, 0, 255, 0.0), stop(0, 0, 255, 128, 1.0)]
}

fn path(document: &Document, first: u64, points: &[(f64, f64)], closed: bool) -> NodeId {
    let anchors: Vec<NewAnchor> = points
        .iter()
        .zip(first..)
        .map(|((x, y), k)| NewAnchor::corner(AnchorId::new(1, k), Point::new(*x, *y)))
        .collect();
    document.create_path(&anchors, closed)
}

fn rect(document: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

fn fonts_none(store: &FaceStore) -> ImportFaces<'_> {
    ImportFaces {
        faces: &[],
        store,
        substitute: None,
    }
}

fn export_read(document: &Document, laid: &ObjectRead) -> String {
    let store = FaceStore::new();
    let outcome = svg_export(
        document,
        laid,
        &fonts_none(&store),
        &ExportScope {
            selection: &[],
            active_page: 0,
        },
        SvgExportOptions::default(),
    )
    .unwrap();
    curvyo_svg_core::write_svg(&outcome.svg)
}

fn export(document: &Document) -> String {
    export_read(document, &document.read_objects())
}

fn golden(name: &str, text: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-svg-core/tests/fixtures/export")
        .join(name);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, text).unwrap();
    }
    let want = std::fs::read_to_string(&path).expect("golden exists");
    assert!(text == want, "{name} differs from its golden:\n{text}");
}

fn open(text: &str) -> Document {
    let drawing = curvyo_svg_core::read_svg(text.as_bytes()).unwrap();
    let store = FaceStore::new();
    let import = svg_import(&drawing, SvgPlacement::File, &fonts_none(&store), u64::MAX);
    let ids: Vec<AnchorId> = (0..curvyo_document_core::DrawingNode::anchor_count(&import.nodes)
        as u64)
        .map(|n| AnchorId::new(9, n))
        .collect();
    Document::from_drawing(3, &import.page, &import.nodes, &ids, &[]).unwrap()
}

fn gradient_leaves(document: &Document) -> Vec<ObjectSnapshot> {
    document
        .read_objects()
        .objects
        .iter()
        .filter(|o| o.style().fill.painted_gradient().is_some())
        .cloned()
        .collect()
}

fn placed(object: &ObjectSnapshot) -> PlacedGradient {
    PlacedGradient::new(
        object.style().fill.painted_gradient().unwrap(),
        &object.gradient_box().unwrap(),
    )
}

/// The export reads back with the same picture on every gradient leaf, and exports again to
/// the same bytes.
fn assert_round_trip(document: &Document, text: &str) {
    assert_eq!(text, export(document), "two exports are byte-identical");
    let before = gradient_leaves(document);
    let after = gradient_leaves(&open(text));
    assert_eq!(before.len(), after.len(), "every gradient comes back");
    for (b, a) in before.iter().zip(&after) {
        assert_eq!(
            b.style().fill.painted_gradient().unwrap().stops,
            a.style().fill.painted_gradient().unwrap().stops
        );
        let (pb, pa) = (placed(b), placed(a));
        for x in [0.0, 13.0, 37.5, 61.0, 90.0] {
            for y in [0.0, 11.0, 24.0, 49.0] {
                let p = Point::new(x, y);
                assert!(
                    (pb.t_at(p) - pa.t_at(p)).abs() < 1e-5,
                    "at {p:?}: {} {}",
                    pb.t_at(p),
                    pa.t_at(p)
                );
            }
        }
    }
}

#[test]
fn ac13_a_linear_gradient_on_a_rectangle() {
    let d = Document::new(1);
    let id = rect(&d, 10.0, 10.0, 80.0, 30.0);
    shade(&d, id, GradientGeometry::DEFAULT_LINEAR, two_stops());
    let text = export(&d);
    assert!(text.contains(r#"fill="url(#gradient-1)""#));
    assert!(text.contains(r#"<linearGradient id="gradient-1" gradientUnits="userSpaceOnUse" x1="10" y1="25" x2="90" y2="25">"#), "{text}");
    assert!(text.contains(r##"<stop offset="1" stop-color="#0000ff" stop-opacity="0.502"/>"##));
    assert!(!text.contains("fill-opacity"), "each stop has its alpha");
    assert!(!text.contains("gradientTransform"));
    golden("gradient_linear.svg", &text);
    assert_round_trip(&d, &text);
}

#[test]
fn ac13_seven_stops_with_a_hard_edge() {
    let d = Document::new(1);
    let id = path(
        &d,
        0,
        &[(0.0, 0.0), (90.0, 5.0), (80.0, 45.0), (5.0, 40.0)],
        true,
    );
    let stops = vec![
        stop(0, 0, 0, 255, 0.0),
        stop(255, 0, 0, 255, 0.2),
        stop(255, 255, 0, 255, 0.4),
        stop(0, 255, 0, 255, 0.4),
        stop(0, 255, 255, 200, 0.6),
        stop(0, 0, 255, 100, 0.85),
        stop(255, 255, 255, 0, 1.0),
    ];
    let slanted = GradientGeometry::Linear {
        start: BoxPoint::new(0.1, 0.2).unwrap(),
        end: BoxPoint::new(0.9, 0.8).unwrap(),
    };
    shade(&d, id, slanted, stops);
    let text = export(&d);
    assert_eq!(text.matches("<stop ").count(), 7);
    golden("gradient_multistop.svg", &text);
    assert_round_trip(&d, &text);
}

#[test]
fn ac13_a_radial_gradient_on_a_circle() {
    let d = Document::new(1);
    let id = d.create_ellipse(EllipseFrame {
        center: Point::new(50.0, 30.0),
        rx: Length::from_mm(20.0),
        ry: Length::from_mm(20.0),
    });
    shade(&d, id, GradientGeometry::DEFAULT_RADIAL, two_stops());
    let text = export(&d);
    assert!(
        text.contains(r#"<radialGradient id="gradient-1" gradientUnits="userSpaceOnUse" cx="50" cy="30" r="20" fx="50" fy="30">"#),
        "{text}"
    );
    assert!(!text.contains("gradientTransform"));
    golden("gradient_radial.svg", &text);
    assert_round_trip(&d, &text);
}

#[test]
fn ac13_only_an_elliptical_radial_carries_a_matrix() {
    let d = Document::new(1);
    let id = rect(&d, 20.0, 10.0, 60.0, 30.0);
    let turned = d
        .object(id)
        .unwrap()
        .rotated(Point::new(50.0, 25.0), Angle::from_radians(0.5));
    d.rotate_object(&turned).unwrap();
    shade(
        &d,
        id,
        GradientGeometry::Radial {
            centre: BoxPoint::new(0.4, 0.5).unwrap(),
            radius: BoxPoint::new(0.9, 0.5).unwrap(),
            ratio: AxisRatio::new(0.4).unwrap(),
        },
        two_stops(),
    );
    // A turned circular radial needs no matrix.
    let circle = rect(&d, 0.0, 0.0, 20.0, 20.0);
    let turned = d
        .object(circle)
        .unwrap()
        .rotated(Point::new(10.0, 10.0), Angle::from_radians(0.7));
    d.rotate_object(&turned).unwrap();
    shade(&d, circle, GradientGeometry::DEFAULT_RADIAL, two_stops());
    let text = export(&d);
    assert_eq!(text.matches("gradientTransform").count(), 1, "{text}");
    golden("gradient_ellipse_rotated.svg", &text);
    assert_round_trip(&d, &text);
}

#[test]
fn ac13_a_compound_path_and_an_open_path_with_a_stroke() {
    let d = Document::new(1);
    let outer = path(
        &d,
        0,
        &[(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)],
        true,
    );
    let anchors = |first: u64, points: &[(f64, f64)]| -> Vec<NewAnchor> {
        points
            .iter()
            .zip(first..)
            .map(|((x, y), k)| NewAnchor::corner(AnchorId::new(1, k), Point::new(*x, *y)))
            .collect()
    };
    let compound = d
        .replace_with_path(
            &[outer],
            outer,
            &[
                (
                    anchors(10, &[(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)]),
                    true,
                ),
                (
                    anchors(
                        20,
                        &[(20.0, 10.0), (20.0, 30.0), (40.0, 30.0), (40.0, 10.0)],
                    ),
                    true,
                ),
            ],
            "combine",
        )
        .unwrap();
    shade(&d, compound, GradientGeometry::DEFAULT_LINEAR, two_stops());
    let text = export(&d);
    golden("gradient_compound.svg", &text);
    assert_round_trip(&d, &text);

    let d = Document::new(1);
    let open = path(&d, 0, &[(5.0, 40.0), (30.0, 5.0), (70.0, 30.0)], false);
    d.edit_style(&[open], &StyleEdit::StrokeWidth(Length::from_mm(0.5)))
        .unwrap();
    shade(&d, open, GradientGeometry::DEFAULT_LINEAR, two_stops());
    let text = export(&d);
    assert!(text.contains(r#"fill="url(#gradient-1)""#) && text.contains("stroke-width"));
    golden("gradient_open_path.svg", &text);
    assert_round_trip(&d, &text);
}

#[test]
fn ac13_a_text_keeps_its_element_and_gets_the_fill() {
    let d = Document::new(1);
    let font = FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "a".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    let id = d
        .create_text(
            "Shade",
            &font,
            Length::from_mm(10.0),
            Affine::new(1.0, 0.0, 0.0, 1.0, 10.0, 30.0),
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    shade(&d, id, GradientGeometry::DEFAULT_LINEAR, two_stops());
    // The line box a layout would give: 50 mm by 14 mm above the baseline at (10, 30).
    let read = d.read_objects();
    let objects: Vec<ObjectSnapshot> = read
        .objects
        .iter()
        .map(|object| match object {
            ObjectSnapshot::Text(text) => {
                let mut text = text.clone();
                let mut glyphs: PathSnapshot = PathSnapshot {
                    id,
                    closed: true,
                    style: text.style.clone(),
                    anchors: Vec::new(),
                    extra_subpaths: Vec::new(),
                    curve_kind: curvyo_document_core::CurveKind::Bezier,
                    spiro_transform: None,
                    width_points: Vec::new(),
                    rotation: Angle::from_radians(0.0),
                    paint_runs: Vec::new(),
                };
                glyphs.anchors = [(12.0, 30.0), (20.0, 20.0), (28.0, 30.0)]
                    .iter()
                    .zip(0..)
                    .map(|((x, y), k)| NewAnchor::corner(AnchorId::new(5, k), Point::new(*x, *y)))
                    .collect();
                text.outline = Some(Arc::new(TextOutline {
                    glyphs,
                    line_box: [
                        Point::new(10.0, 19.0),
                        Point::new(60.0, 19.0),
                        Point::new(60.0, 33.0),
                        Point::new(10.0, 33.0),
                    ],
                    glyph_outlines: vec![1],
                    letter_boxes: Vec::new(),
                    substitute: false,
                }));
                ObjectSnapshot::Text(text)
            }
            other => other.clone(),
        })
        .collect();
    let laid = ObjectRead {
        objects: objects.into(),
        ..read
    };
    let text = export_read(&d, &laid);
    assert!(text.contains("<text"), "{text}");
    assert!(text.contains(r#"fill="url(#gradient-1)""#));
    assert!(
        text.contains(r#"x1="10" y1="26" x2="60" y2="26""#),
        "{text}"
    );
    golden("gradient_text.svg", &text);
    assert_eq!(text, export_read(&d, &laid));
    // The reader takes the file; a text keeps its first stop as its colour (Proposal).
    let drawing = curvyo_svg_core::read_svg(text.as_bytes()).unwrap();
    assert!(drawing.gradient_losses.is_empty());
}

#[test]
fn ac27_the_import_outcome_carries_the_losses() {
    let text = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">
        <defs>
          <linearGradient id="a" spreadMethod="reflect"><stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>
          <radialGradient id="b" fx="0.2"><stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></radialGradient>
          <radialGradient id="c" fy="0.7" href="#b"/>
        </defs>
        <rect width="10" height="10" fill="url(#a)"/>
        <rect x="20" width="10" height="10" fill="url(#b)"/>
        <rect x="40" width="10" height="10" fill="url(#c)"/>
      </svg>"##;
    let drawing = curvyo_svg_core::read_svg(text.as_bytes()).unwrap();
    let store = FaceStore::new();
    let import = svg_import(&drawing, SvgPlacement::File, &fonts_none(&store), u64::MAX);
    assert_eq!(
        gradient_loss_lines(import.gradient_losses),
        [
            "1 gradient repeat was shown as a plain shade.",
            "2 focal points were read as the centre."
        ]
    );
}

#[test]
fn ac14_a_skewed_radial_becomes_an_ellipse() {
    let text = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">
        <radialGradient id="r" gradientUnits="userSpaceOnUse" cx="50" cy="50" r="20" gradientTransform="matrix(1 0 0.5 1 -25 0)">
          <stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/>
        </radialGradient>
        <path d="M 10 10 L 90 10 L 90 90 L 10 90 Z" fill="url(#r)"/>
      </svg>"##;
    let d = open(text);
    let leaves = gradient_leaves(&d);
    let GradientGeometry::Radial { ratio, .. } =
        leaves[0].style().fill.painted_gradient().unwrap().geometry
    else {
        panic!("radial")
    };
    assert!(ratio != AxisRatio::CIRCLE, "{ratio:?}");
    // The picture: t = 1 on the image of the circle's rightmost point (70, 50) -> (70 + 25 - 25, 50).
    let p = placed(&leaves[0]);
    assert!((p.t_at(Point::new(70.0, 50.0)) - 1.0).abs() < 1e-9);
    assert!(p.t_at(Point::new(50.0, 50.0)).abs() < 1e-9);
}
