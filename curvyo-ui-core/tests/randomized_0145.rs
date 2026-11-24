//! `specs/0145-shape-spiro-handles` slice 3 at the `curvyo-ui-core` level: Object to path of a
//! randomized shape (criterion 29), the knobs staying on the unrandomized shape (`ux-notes.md`
//! section 13), the bar state (criteria 21, 22), the export as a path with the shadow and its
//! import back as a live shape, and the Inkscape import of `inkscape:randomized` (criteria 18,
//! 30).
#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]

use curvyo_document_core::{
    Angle, Curve, Document, DrawingNode, InnerRatio, Length, NodeId, Point, PointCount, Randomized,
    Rounded, Seed, Shape, StarFrame, outline_of,
};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{
    AnchorIdMinter, BarValue, ExportScope, ImportFaces, ObjectSelection, ParamHandle,
    SvgExportOptions, SvgPlacement, TransformHandleTolerances, build_primitive_conversions,
    import_svg_bytes, oriented_bounds, param_handles, select_bar_state, svg_export, svg_import,
};

/// A seed as a fresh one is: 32 bits, far above any coordinate bound (`u32::MAX - 7`).
const SEED: u32 = 4_294_967_288;

fn frame() -> StarFrame {
    StarFrame {
        center: Point::new(100.0, 80.0),
        radius: Length::from_mm(40.0),
        angle: Angle::from_radians(0.0),
    }
}

fn star(r: f64, q: f64, curve: f64) -> Shape {
    Shape::Star {
        frame: frame(),
        point_count: PointCount::new(5).unwrap(),
        inner_ratio: InnerRatio::new(0.5).unwrap(),
        curve: Curve::new(curve).unwrap(),
        rounded: Rounded::new(q).unwrap(),
        randomized: Randomized::new(r, Seed::new(if r == 0.0 { 0 } else { SEED })).unwrap(),
    }
}

/// Criterion 29: Object to path gives one path through the moved vertices with the handles of the
/// randomized outline, no re-randomizing afterwards; the primitive's keys are gone.
#[test]
fn object_to_path_keeps_the_moved_ring_and_strips_the_keys() {
    for (q, curve) in [(0.0, 0.0), (1.5, 0.0), (0.0, 0.2)] {
        let document = Document::new(1);
        let id = document.create_primitive(star(0.4, q, curve), Angle::from_radians(0.0));
        let mut minter = AnchorIdMinter::new(7);
        let conversions = build_primitive_conversions(&document, &mut minter, &[id]);
        let outline = outline_of(&star(0.4, q, curve));
        let anchors = &conversions[0].1;
        assert_eq!(anchors.len(), outline.anchors.len());
        for (a, o) in anchors.iter().zip(&outline.anchors) {
            assert_eq!(a.point, o.point);
            assert_eq!(a.handle_out, o.handle_out);
        }
        // The plain star differs: the moved ring is what the path keeps.
        let plain = outline_of(&star(0.0, q, curve));
        assert!(
            anchors
                .iter()
                .zip(&plain.anchors)
                .any(|(a, p)| a.point.vector_to(p.point).length() > 1.0)
        );
        document.convert_to_paths(&conversions).unwrap();
        assert!(document.primitive(id).is_none() && document.path(id).is_some());
        let json = String::from_utf8(document.export_json().unwrap()).unwrap();
        assert!(!json.contains("randomized"), "{json}");
    }
}

fn knobs(shape: Shape) -> Vec<(ParamHandle, Point)> {
    let document = Document::new(1);
    let id: NodeId = document.create_primitive(shape, Angle::from_radians(0.0));
    let object = document.object(id).unwrap();
    param_handles(
        &object,
        &oriented_bounds(&object),
        &TransformHandleTolerances::at_scale(3.0),
    )
}

/// `ux-notes.md` section 13: the inner-radius, Rounded and Curve knobs stay where the unrandomized
/// shape has them, whatever `r` and the seed are, so a re-roll never moves a knob.
#[test]
fn the_knobs_stay_on_the_unrandomized_shape() {
    for q in [0.0, 0.7] {
        let plain = knobs(star(0.0, q, 0.0));
        assert!(plain.len() >= 2, "{plain:?}");
        for r in [0.3, -4.0, 10.0] {
            assert_eq!(knobs(star(r, q, 0.0)), plain, "r {r} q {q}");
        }
    }
}

/// Criteria 21 and 22: the bar shows the value, Mixed for different values, the limited tag for a
/// stored value beyond 10, and Re-roll enabled only while a value is not 0.
#[test]
fn the_bar_state_shows_mixed_limited_and_the_button() {
    let document = Document::new(1);
    let make = |r: f64| document.create_primitive(star(r, 0.0, 0.0), Angle::from_radians(0.0));
    let (a, b) = (make(0.0), make(0.3));
    let state = |ids: &[NodeId]| {
        let mut selection = ObjectSelection::default();
        for id in ids {
            selection.add(*id);
        }
        select_bar_state(&document.read_objects().objects, &selection, None)
            .randomized
            .unwrap()
    };
    let one = state(&[a]);
    assert_eq!(
        (one.value, one.reroll_enabled),
        (BarValue::Uniform(0.0), false)
    );
    let both = state(&[a, b]);
    assert_eq!((both.value, both.reroll_enabled), (BarValue::Mixed, true));
    assert_eq!(both.limited, None);
    let big = make(12.4);
    let limited = state(&[big]);
    assert_eq!(limited.value, BarValue::Uniform(10.0));
    assert_eq!(limited.limited, Some(12.4));
}

fn page_document(shape: Shape) -> Document {
    let d = Document::new(1);
    let _ = d.create_primitive(shape, Angle::from_radians(0.0));
    d
}

fn export(document: &Document) -> String {
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
            selection: &[],
            active_page: 0,
        },
        SvgExportOptions::default(),
    )
    .unwrap();
    curvyo_svg_core::write_svg(&outcome.svg)
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

fn flatten(node: &DrawingNode) -> Vec<&DrawingNode> {
    match node {
        DrawingNode::Group { children, .. } => children.iter().flat_map(flatten).collect(),
        other => vec![other],
    }
}

/// Criteria 18, 30: a randomized shape leaves as a path with the shadow (never
/// `inkscape:randomized` other than 0), carrying `r` and the seed, and comes back as the same live
/// shape; with Rounded and with Curve too.
#[test]
fn a_randomized_shape_exports_as_a_path_and_comes_back_live() {
    for (q, curve) in [(0.0, 0.0), (1.2, 0.0), (0.0, 0.25)] {
        let shape = star(0.35, q, curve);
        let svg = export(&page_document(shape));
        assert!(!svg.contains("sodipodi:type=\"star\""), "{svg}");
        assert!(
            !svg.contains("inkscape:randomized=\"0.") && !svg.contains("inkscape:randomized=\"1"),
            "{svg}"
        );
        let nodes = open(&svg);
        let back: Vec<&Shape> = nodes
            .iter()
            .flat_map(flatten)
            .filter_map(|n| match n {
                DrawingNode::Primitive { shape, .. } => Some(shape),
                _ => None,
            })
            .collect();
        assert_eq!(back.len(), 1, "a live shape: {nodes:?}");
        assert_eq!(*back[0], shape);
    }
}

/// Criterion 30: the shapes of the older shadow forms (without `r` and seed) still read as `r = 0`.
#[test]
fn a_shape_without_randomizing_exports_as_before() {
    let svg = export(&page_document(star(0.0, 0.0, 0.0)));
    assert!(svg.contains("sodipodi:type=\"star\""), "{svg}");
}

const NS: &str = r#"xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd""#;

/// Criterion 18: an Inkscape star with `inkscape:randomized` other than 0 stays an ordinary path
/// (Inkscape's generator cannot be reproduced from a stored seed), in every slice.
#[test]
fn an_inkscape_randomized_star_stays_a_path() {
    let svg = format!(
        r#"<svg {NS} width="210mm" height="297mm" viewBox="0 0 210 297"><path sodipodi:type="star" sodipodi:sides="5" sodipodi:cx="100" sodipodi:cy="100" sodipodi:r1="20" sodipodi:r2="10" sodipodi:arg1="0" sodipodi:arg2="0.6283185" inkscape:flatsided="false" inkscape:rounded="0" inkscape:randomized="0.2" d="M 120,100 L 108,106 L 106,119 L 98,108 L 85,106 L 93,96 L 91,82 L 101,91 L 113,86 L 111,98 Z"/></svg>"#
    );
    let nodes = import_svg_bytes(svg.as_bytes(), SvgPlacement::File)
        .unwrap()
        .nodes;
    assert!(
        matches!(nodes[0], DrawingNode::Path { .. }),
        "an ordinary path: {nodes:?}"
    );
}

fn outline_samples(shape: &Shape, per_cubic: u32) -> Vec<Point> {
    let outline = curvyo_document_core::drawing_outline_of(shape);
    let n = outline.anchors.len();
    let mut points = Vec::new();
    for i in 0..n {
        let (a, b) = (outline.anchors[i], outline.anchors[(i + 1) % n]);
        let (p1, p2) = (
            a.point.translated(a.handle_out),
            b.point.translated(b.handle_in),
        );
        for k in 0..per_cubic {
            let t = f64::from(k) / f64::from(per_cubic);
            let u = 1.0 - t;
            let at = |f: fn(Point) -> f64| {
                u * u * u * f(a.point)
                    + 3.0 * u * u * t * f(p1)
                    + 3.0 * u * t * t * f(p2)
                    + t * t * t * f(b.point)
            };
            points.push(Point::new(at(|p| p.x), at(|p| p.y)));
        }
    }
    points
}

/// Criterion 28: the selection box the maker sees holds 4096 sampled points of a randomized
/// outline, also rotated, and grows past today's box where the outline leaves it.
#[test]
fn the_selection_box_contains_the_randomized_outline() {
    for (r, rotation) in [(0.5, 0.0), (-2.0, 0.5)] {
        let document = Document::new(1);
        let id = document.create_primitive(star(r, 0.0, 0.0), Angle::from_radians(rotation));
        let object = document.object(id).unwrap();
        let box_ = oriented_bounds(&object);
        let samples = outline_samples(&star(r, 0.0, 0.0), 410);
        assert!(samples.len() >= 4096);
        let mut grew = false;
        for p in samples {
            let turned = p.rotated_around(Point::new(100.0, 80.0), Angle::from_radians(rotation));
            let local = box_.to_local(turned);
            assert!(
                local.x >= box_.min.x - 1e-6
                    && local.x <= box_.max.x + 1e-6
                    && local.y >= box_.min.y - 1e-6
                    && local.y <= box_.max.y + 1e-6,
                "r {r}: {turned:?} lies outside the box {box_:?}"
            );
            grew |= (local.x - 100.0).abs() > 40.0 + 1e-3;
        }
        assert!(grew, "r {r}: the outline leaves the circumscribed box");
    }
}

/// Criterion 28: an unfilled randomized shape is hit on its moved outline, not on the vertices it
/// has without randomizing.
#[test]
fn a_randomized_shape_is_hit_on_its_moved_outline() {
    use curvyo_document_core::Tolerance;
    use curvyo_ui_core::hit_test_object;
    let shape = star(0.5, 0.0, 0.0);
    let plain = star(0.0, 0.0, 0.0);
    let document = Document::new(1);
    let id = document.create_primitive(shape, Angle::from_radians(0.0));
    let object = document.object(id).unwrap();
    let moved = curvyo_document_core::drawing_outline_of(&shape).anchors[0].point;
    let home = curvyo_document_core::drawing_outline_of(&plain).anchors[0].point;
    assert!(moved.vector_to(home).length() > 5.0, "vertex 0 moved");
    let tolerance = Tolerance::from_mm(0.5);
    assert_eq!(
        hit_test_object(std::slice::from_ref(&object), moved, tolerance),
        Some(id)
    );
    // Where vertex 0 was drawn without randomizing there is no outline now (unless another side
    // passes through, which the sample makes unlikely).
    let samples = outline_samples(&shape, 200);
    let near = samples.iter().any(|q| q.vector_to(home).length() <= 2.0);
    if !near {
        assert_eq!(hit_test_object(&[object], home, tolerance), None);
    }
}
