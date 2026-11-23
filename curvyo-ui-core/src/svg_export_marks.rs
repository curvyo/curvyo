//! The SVG of a registration mark (`specs/0121-print-and-cut` criterion 4, ADR 0081 §3): a
//! `<g data-curvyo-kind="registration-mark" data-curvyo-mark="n">` holding its circle and one
//! path of its cross and digit, stroked black, no fill. Import reads these as plain shapes.

use curvyo_document_core::{PathSnapshot, RegistrationMarkSnapshot, mark_path};
use curvyo_svg_core::{ExportKind, ExportNode, ExportState};
use kurbo::Affine;

use crate::svg_export::leaf_node;
use crate::svg_export_leaf::{export_style, path_kind};

/// The group of `mark` mapped by `t` (document mm to file mm) with the registers `state`.
pub(crate) fn mark_node(
    mark: &RegistrationMarkSnapshot,
    t: Affine,
    mut state: ExportState,
) -> ExportNode {
    state.data = vec![
        ("data-curvyo-kind", "registration-mark".to_owned()),
        ("data-curvyo-mark", mark.number.to_string()),
    ];
    let (style, shadow) = export_style(&mark.style, false);
    let lines = curvyo_document_core::mark_lines(mark);
    let centre = t * kurbo::Point::new(lines.centre.x, lines.centre.y);
    let radius = lines.radius.as_mm() * t.determinant().abs().sqrt();
    let circle = leaf_node(
        ExportState::default(),
        ExportKind::Ellipse {
            center: centre,
            rx: radius,
            ry: radius,
            rotation_deg: 0.0,
        },
        style.clone(),
        shadow.clone(),
    );
    let path = mark_path(mark);
    let mut open = path.extra_subpaths.into_iter();
    let first = open.next();
    let strokes = PathSnapshot {
        closed: false,
        anchors: first.map(|s| s.anchors).unwrap_or_default(),
        extra_subpaths: open.collect(),
        ..path
    };
    let cross = leaf_node(
        ExportState::default(),
        path_kind(&strokes, t),
        style,
        shadow,
    );
    ExportNode::Group {
        state,
        layer: false,
        children: vec![circle, cross],
    }
}
