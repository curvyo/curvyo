//! The canvas drawing of pattern fills (`specs/0071-pattern-fill` criteria 2, 16, 19; ADR 0029
//! §7): each shape with a pattern drawn as a path of its pieces under the shape's stroke.

use curvyo_document_core::{
    AnchorId, AnchorKind, NewAnchor, NodeId, ObjectSnapshot, PathSnapshot, SubpathSnapshot,
};
use curvyo_geometry_core::PatternPiece;

use crate::pattern_cache::PatternCache;
use crate::pattern_output::piece_style;

impl PatternCache {
    /// `objects` as the canvas draws them: every shape with a pattern replaced by a path of its
    /// pieces painted as ADR 0029 §7 says, then the shape itself with its fill off (its stroke
    /// above the pieces). Shapes for which `keep_last` holds show their last pieces over the limit.
    /// No entry is dropped here: the caller drops those of shapes gone from the read once per draw
    /// ([`PatternCache::retain_only`]).
    pub fn expand(
        &mut self,
        objects: &[ObjectSnapshot],
        keep_last: &dyn Fn(NodeId) -> bool,
    ) -> Vec<ObjectSnapshot> {
        let mut out = Vec::with_capacity(objects.len());
        for object in objects {
            let Some((pieces, _)) = self.pieces_of(object, keep_last(object.id())) else {
                out.push(object.clone());
                continue;
            };
            if let Some(path) = pieces_path(object, &pieces) {
                out.push(ObjectSnapshot::Path(path));
            }
            let mut shape = object.clone();
            shape.style_mut().fill.enabled = false;
            out.push(shape);
        }
        out
    }

    /// Whether any of `objects` paints a pattern (the canvas skips [`PatternCache::expand`]
    /// otherwise).
    #[must_use]
    pub fn any_pattern(objects: &[ObjectSnapshot]) -> bool {
        objects
            .iter()
            .any(|object| object.style().fill.painted_pattern().is_some())
    }
}

/// One path snapshot holding every piece as an outline, in the piece paint; `None` without
/// pieces. Its anchor ids are synthetic: it is drawn, never picked or written.
fn pieces_path(object: &ObjectSnapshot, pieces: &[PatternPiece]) -> Option<PathSnapshot> {
    let fill = object.style().fill.painted_pattern()?;
    let mut next = 0_u64;
    let mut outlines = pieces
        .iter()
        .flat_map(|piece| &piece.outlines)
        .map(|(anchors, closed)| {
            let anchors = anchors
                .iter()
                .map(|&(point, handle_in, handle_out)| {
                    next += 1;
                    NewAnchor {
                        id: AnchorId::new(0, next),
                        point,
                        handle_in,
                        handle_out,
                        kind: AnchorKind::Corner,
                    }
                })
                .collect::<Vec<_>>();
            SubpathSnapshot {
                closed: *closed,
                anchors,
            }
        });
    let first = outlines.next()?;
    Some(PathSnapshot {
        id: object.id(),
        closed: first.closed,
        style: piece_style(object.style(), fill.mode),
        anchors: first.anchors,
        extra_subpaths: outlines.collect(),
        rotation: curvyo_document_core::Angle::from_radians(0.0),
    })
}
