//! The artwork as the canvas draws it from the node read: the effectively visible leaves in
//! stacking order, each paint's alpha multiplied by the leaf's effective opacity
//! (`specs/0050-layers-and-objects-panel` criteria 24 and 27, `adrs.md` decision 10).
//!
//! Each paint is multiplied separately; nothing is composited as a group (Question 5 A), so where
//! a fill and a stroke, or two leaves of one faded layer, overlap, the lower shows through.

use curvyo_document_core::{NodeRead, ObjectSnapshot, Opacity};

/// `object` with its fill and stroke opacity multiplied by `opacity`.
#[must_use]
pub fn faded(object: &ObjectSnapshot, opacity: Opacity) -> ObjectSnapshot {
    let mut shown = object.clone();
    if opacity == Opacity::OPAQUE {
        return shown;
    }
    let style = shown.style_mut();
    // invariant: a product of numbers in 0..=1 lies in 0..=1, so `new` cannot refuse it.
    let times = |paint: Opacity| Opacity::new(paint.get() * opacity.get()).unwrap_or(paint);
    style.fill.opacity = times(style.fill.opacity);
    style.stroke.opacity = times(style.stroke.opacity);
    shown
}

/// The leaves of `reads` that are effectively visible, in stacking order, each faded by its
/// effective opacity: exactly what the canvas draws. A locked leaf is drawn.
#[must_use]
pub fn drawn_leaves(reads: &[NodeRead]) -> Vec<ObjectSnapshot> {
    reads
        .iter()
        .filter(|read| read.visible)
        .filter_map(|read| read.object().map(|object| faded(object, read.opacity)))
        .collect()
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Document, MovePlan, MoveTarget, NodeId, Point, RectBounds};

    use super::*;

    fn rect(document: &Document, x: f64) -> NodeId {
        document.create_rect(RectBounds::from_corners(
            Point::new(x, 0.0),
            Point::new(x + 10.0, 10.0),
        ))
    }

    fn into(document: &Document, nodes: &[NodeId], layer: NodeId) {
        document
            .move_nodes(&MovePlan {
                nodes: nodes.to_vec(),
                target: MoveTarget::FrontOf(Some(layer)),
            })
            .unwrap();
    }

    /// Criterion 27's numbers: fill 80 %, stroke 100 %, leaf 50 %, layer 50 %.
    #[test]
    fn each_paint_is_multiplied_by_the_leaf_and_every_ancestor() {
        let document = Document::new(1);
        let leaf = rect(&document, 0.0);
        let layer = document.new_layer(None);
        into(&document, &[leaf], layer);
        let edit = |edit: curvyo_document_core::StyleEdit| {
            document.edit_style(&[leaf], &edit).unwrap();
        };
        edit(curvyo_document_core::StyleEdit::FillEnabled(true));
        edit(curvyo_document_core::StyleEdit::FillOpacity(
            Opacity::new(0.8).unwrap(),
        ));
        edit(curvyo_document_core::StyleEdit::StrokeOpacity(
            Opacity::OPAQUE,
        ));
        let half = Opacity::new(0.5).unwrap();
        document.set_node_opacity(&[leaf], half).unwrap();
        document.set_node_opacity(&[layer], half).unwrap();
        let drawn = drawn_leaves(&document.read_nodes());
        assert_eq!(drawn.len(), 1);
        let style = drawn[0].style();
        assert!((style.fill.opacity.get() - 0.2).abs() < 1e-12);
        assert!((style.stroke.opacity.get() - 0.25).abs() < 1e-12);
    }

    /// Criterion 24: hidden leaves are not drawn, locked ones are, in stacking order.
    #[test]
    fn the_frame_holds_the_visible_and_locked_leaves_and_none_of_the_hidden() {
        let document = Document::new(1);
        let hidden_layer = document.new_layer(None);
        let in_hidden = rect(&document, 0.0);
        into(&document, &[in_hidden], hidden_layer);
        document.set_node_visible(&[hidden_layer], false).unwrap();
        let shown_layer = document.new_layer(None);
        let (hidden_leaf, visible, locked) = (
            rect(&document, 20.0),
            rect(&document, 40.0),
            rect(&document, 60.0),
        );
        into(&document, &[hidden_leaf, visible, locked], shown_layer);
        document.set_node_visible(&[hidden_leaf], false).unwrap();
        document.set_node_locked(&[locked], true).unwrap();
        let reads = document.read_nodes();
        let drawn: Vec<NodeId> = drawn_leaves(&reads)
            .iter()
            .map(ObjectSnapshot::id)
            .collect();
        assert_eq!(drawn, vec![visible, locked]);
        let leaves = reads.iter().filter(|read| read.object().is_some()).count();
        assert_eq!(leaves, 4, "the read still lists every leaf");
    }

    #[test]
    fn an_opaque_leaf_is_drawn_unchanged() {
        let document = Document::new(1);
        let leaf = rect(&document, 0.0);
        let drawn = drawn_leaves(&document.read_nodes());
        assert_eq!(drawn, vec![document.object(leaf).unwrap()]);
    }
}
