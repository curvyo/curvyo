//! One path, primitive or linked clone as an element of the SVG export
//! (`specs/0024-svg-import-export` criteria 54 to 59, 139 to 141 and 146): outlines as absolute
//! path data with node letters, primitives as `<rect>`, `<ellipse>`, Inkscape stars and arcs or
//! as paths with shadow data (`svg_export_shapes`), the style as the writer's, the markers SVG
//! can draw.

use curvyo_document_core::{
    AnchorKind, LineCap, LineJoin, MarkerPlace, MarkerShape as ModelMarker, Markers, PathSnapshot,
    Point, PrimitiveOutline, Style, Vec2,
};
use curvyo_geometry_core::OffsetAnchor;
use curvyo_svg_core::{
    Cap, ExportKind, ExportStroke, ExportStyle, Join, MarkerShape, Paint, PathId, Shadow,
    ShadowMarkers,
};
use kurbo::{Affine, BezPath};

/// One node of an outline: position, handles and kind.
pub(crate) type Node = (Point, Vec2, Vec2, AnchorKind);

/// The text `{:.6}` writes reads back to `value` exactly.
#[allow(clippy::float_cmp)] // exactness is the question
pub(crate) fn six_decimals_exact(value: f64) -> bool {
    format!("{value:.6}")
        .parse::<f64>()
        .is_ok_and(|v| v == value)
}

const fn letter(kind: AnchorKind) -> char {
    match kind {
        AnchorKind::Corner => 'c',
        AnchorKind::Symmetric => 'z',
        AnchorKind::Asymmetric => 's',
    }
}

/// Appends one outline mapped by `t` to `path` and its node letters to `letters` (criterion 54).
/// A curved closing segment is written to the first node before `Z`, and a straight one to a
/// first node it ends on, so the importer of the profile gets the same nodes back.
pub(crate) fn push_outline(
    nodes: &[Node],
    closed: bool,
    t: Affine,
    path: &mut BezPath,
    letters: &mut String,
) {
    let Some(&first) = nodes.first() else {
        return;
    };
    let at = |p: Point| t * kurbo::Point::new(p.x, p.y);
    let segment = |path: &mut BezPath, from: Node, to: Node| {
        if from.2 == Vec2::ZERO && to.1 == Vec2::ZERO {
            path.line_to(at(to.0));
        } else {
            let plus = |p: Point, v: Vec2| Point::new(p.x + v.x, p.y + v.y);
            path.curve_to(at(plus(from.0, from.2)), at(plus(to.0, to.1)), at(to.0));
        }
    };
    path.move_to(at(first.0));
    letters.push(letter(first.3));
    for pair in nodes.windows(2) {
        segment(path, pair[0], pair[1]);
        letters.push(letter(pair[1].3));
    }
    if closed {
        // invariant: `nodes` is not empty (checked above).
        let last = nodes[nodes.len() - 1];
        let curved = last.2 != Vec2::ZERO || first.1 != Vec2::ZERO;
        // exact: a first node that the outline repeats is the writer's own copy (point 9).
        if curved || (last.0 == first.0 && nodes.len() > 1) {
            segment(path, last, first);
            letters.push(letter(first.3));
        }
        path.close_path();
    }
}

/// The outlines of `path` mapped by `t` and their node letters.
pub(crate) fn path_data(path: &PathSnapshot, t: Affine) -> (BezPath, String) {
    let mut out = BezPath::new();
    let mut letters = String::new();
    for subpath in path.subpaths() {
        let nodes: Vec<Node> = subpath
            .anchors
            .iter()
            .map(|a| (a.point, a.handle_in, a.handle_out, a.kind))
            .collect();
        push_outline(&nodes, subpath.closed, t, &mut out, &mut letters);
    }
    (out, letters)
}

/// A primitive outline mapped by `t` and its node letters.
pub(crate) fn outline_data(outline: &PrimitiveOutline, t: Affine) -> (BezPath, String) {
    let nodes: Vec<Node> = outline
        .anchors
        .iter()
        .map(|a| (a.point, a.handle_in, a.handle_out, a.kind))
        .collect();
    let mut out = BezPath::new();
    let mut letters = String::new();
    push_outline(&nodes, outline.closed, t, &mut out, &mut letters);
    (out, letters)
}

/// A stroke outline (closed outlines of [`OffsetAnchor`]s) mapped by `t`.
pub(crate) fn offset_data(outlines: &[Vec<OffsetAnchor>], t: Affine) -> (BezPath, String) {
    let mut out = BezPath::new();
    let mut letters = String::new();
    for outline in outlines {
        let nodes: Vec<Node> = outline
            .iter()
            .map(|a| {
                let kind = if a.smooth {
                    AnchorKind::Asymmetric
                } else {
                    AnchorKind::Corner
                };
                (a.point, a.handle_in, a.handle_out, kind)
            })
            .collect();
        push_outline(&nodes, true, t, &mut out, &mut letters);
    }
    (out, letters)
}

fn marker(shape: ModelMarker) -> Option<MarkerShape> {
    match shape {
        ModelMarker::None => None,
        ModelMarker::Arrow => Some(MarkerShape::Arrow),
        ModelMarker::Dot => Some(MarkerShape::Dot),
    }
}

/// The paint of a colour and opacity.
pub(crate) fn paint(color: curvyo_document_core::Color, opacity: f64) -> Paint {
    Paint {
        rgb: [color.r, color.g, color.b],
        opacity,
    }
}

/// The writer's style of `style` and its shadow values (criteria 56, 57, 141). `svg_markers`:
/// the leaf is one open outline of two or more nodes, so SVG draws its end and node markers.
#[allow(clippy::float_cmp)] // exactness of the read-back dash list is the question
pub(crate) fn export_style(style: &Style, svg_markers: bool) -> (ExportStyle, Shadow) {
    let fill = style
        .fill
        .enabled
        .then(|| paint(style.fill.color, style.fill.opacity.get()));
    let s = &style.stroke;
    let width = s.width.as_mm();
    let mut shadow = Shadow::default();
    let stroke = (s.enabled && width > 0.0).then(|| {
        let dash = s.dash.as_slice();
        if !six_decimals_exact(width) {
            shadow.stroke_width = Some(width);
        }
        let read_back = |d: f64| {
            let mm: f64 = format!("{:.6}", d * width).parse().unwrap_or(0.0);
            let w: f64 = format!("{width:.6}").parse().unwrap_or(1.0);
            mm / w
        };
        if dash.iter().any(|d| read_back(*d) != *d) {
            shadow.dash = Some(dash.to_vec());
        }
        let markers = s.markers;
        if markers != Markers::default() {
            shadow.markers = Some(ShadowMarkers {
                start: marker(markers.start),
                middle: marker(markers.mid),
                at_nodes: markers.mid_place == MarkerPlace::AtNodes,
                count: markers.mid_count.get(),
                end: marker(markers.end),
            });
        }
        let at_nodes = markers.mid_place == MarkerPlace::AtNodes;
        ExportStroke {
            paint: paint(s.color, s.opacity.get()),
            width_mm: width,
            dash_mm: dash.iter().map(|d| d * width).collect(),
            cap: match s.cap {
                LineCap::Butt => Cap::Butt,
                LineCap::Round => Cap::Round,
                LineCap::Square => Cap::Square,
            },
            join: match s.join {
                LineJoin::Miter => Join::Miter,
                LineJoin::Round => Join::Round,
                LineJoin::Bevel => Join::Bevel,
            },
            start: svg_markers.then(|| marker(markers.start)).flatten(),
            mid: (svg_markers && at_nodes)
                .then(|| marker(markers.mid))
                .flatten(),
            end: svg_markers.then(|| marker(markers.end)).flatten(),
        }
    });
    (ExportStyle { fill, stroke }, shadow)
}

/// Whether `path` is a single open outline of two or more nodes (criterion 141 (b)).
pub(crate) fn svg_draws_markers(path: &PathSnapshot) -> bool {
    !path.is_compound() && !path.closed && path.anchors.len() >= 2
}

/// The element of a path (or a clone's or sketch's resolved outline) mapped by `t`.
pub(crate) fn path_kind(path: &PathSnapshot, t: Affine) -> ExportKind {
    let (d, node_types) = path_data(path, t);
    ExportKind::Path {
        path: d,
        node_types,
        id: if path.is_compound() {
            PathId::Compound
        } else {
            PathId::Path
        },
    }
}
