//! What a Find centre lines preview draws over its painted result (`specs/0117-centre-lines` UX
//! notes §4): the centre lines and their node dots.

use curvyo_document_core::{AnchorKind, NewAnchor, Point, ViewTransform};

use crate::glyphs::{DrawList, circle};
use crate::offset_preview::{PreviewAnchor, at_strength};
use crate::shape_preview::UNUSED_ANCHOR_ID;
use crate::stroke;
use crate::theme;

/// The centre lines a Find centre lines preview shows (`specs/0117-centre-lines` UX notes §4):
/// each open or closed line in `lines` as a 1 px `--preview-new` stroke of constant screen width,
/// and a 3 px `--preview-new` dot at each point of `nodes`, at `strength` (0 to 1) of their
/// alpha; no casing, no fill.
#[must_use]
pub fn build_centre_line_preview(
    lines: &[(Vec<PreviewAnchor>, bool)],
    nodes: &[Point],
    view: ViewTransform,
    strength: f64,
) -> DrawList {
    let width_mm = CENTRE_LINE_PX / view.scale();
    let tolerance_mm = theme::DISPLAY_TOLERANCE_PX / view.scale();
    let color = at_strength(theme::PREVIEW_NEW, strength);
    let paths: Vec<Vec<NewAnchor>> = lines
        .iter()
        .map(|(line, _)| {
            line.iter()
                .map(|&(point, handle_in, handle_out)| NewAnchor {
                    id: UNUSED_ANCHOR_ID,
                    point,
                    handle_in,
                    handle_out,
                    kind: AnchorKind::Corner,
                })
                .collect()
        })
        .collect();
    let refs: Vec<stroke::OutlineRef<'_>> = paths
        .iter()
        .zip(lines)
        .map(|(anchors, (_, closed))| stroke::OutlineRef {
            anchors,
            closed: *closed,
        })
        .collect();
    let mut list = DrawList::default();
    if let Some(path) = stroke::build_outlines(&refs, false) {
        list.extend(stroke::path_stroke_of(&path, width_mm, color, tolerance_mm));
    }
    let dot_mm = CENTRE_NODE_PX / view.scale();
    for &node in nodes {
        list.extend(circle(node, dot_mm, color));
    }
    list
}

/// The width of a previewed centre line, screen pixels.
const CENTRE_LINE_PX: f64 = 1.0;

/// The diameter of a previewed centre line's node dot, screen pixels.
const CENTRE_NODE_PX: f64 = 3.0;
