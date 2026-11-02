//! The pieces of a pattern fill in the SVG export (`specs/0071-pattern-fill` criterion 26, ADR
//! 0029 §8): one group of plain `<path>` elements in the piece paint, no `<pattern>` element and no
//! reference to the generator.

use curvyo_document_core::{AnchorKind, ObjectSnapshot};
use curvyo_svg_core::{ExportKind, ExportNode, ExportState, PathId};
use kurbo::{Affine, BezPath};

use crate::pattern_output::{object_pattern_pieces, piece_style};
use crate::svg_export::leaf_node;
use crate::svg_export_leaf::{Node, export_style, push_outline};

/// The group of `object`'s pattern pieces mapped by `t`; `None` when it paints no pattern or has
/// no pieces (open, over the limit).
pub(crate) fn pattern_pieces_node(object: &ObjectSnapshot, t: Affine) -> Option<ExportNode> {
    let pieces = object_pattern_pieces(object)?.ok()?;
    if pieces.is_empty() {
        return None;
    }
    let fill = object.style().fill.painted_pattern()?;
    let (style, shadow) = export_style(&piece_style(object.style(), fill.mode), false);
    let children = pieces
        .iter()
        .map(|piece| {
            let mut path = BezPath::new();
            let mut letters = String::new();
            for (anchors, closed) in &piece.outlines {
                let nodes: Vec<Node> = anchors
                    .iter()
                    .map(|&(p, a, b)| (p, a, b, AnchorKind::Corner))
                    .collect();
                push_outline(&nodes, *closed, t, &mut path, &mut letters);
            }
            let id = if piece.outlines.len() > 1 {
                PathId::Compound
            } else {
                PathId::Path
            };
            leaf_node(
                ExportState::default(),
                ExportKind::Path {
                    path,
                    node_types: letters,
                    id,
                },
                style.clone(),
                shadow.clone(),
            )
        })
        .collect();
    Some(ExportNode::Group {
        state: ExportState::default(),
        layer: false,
        children,
    })
}
