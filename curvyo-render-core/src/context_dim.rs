//! The dimming of everything outside the entered group (`specs/0023-groups` criterion 15, the
//! design system's `--context-dim`): the artwork outside the context is drawn at a fraction of its
//! normal strength. Selection boxes, handles and the other overlays are not artwork and stay at
//! full strength.

use std::collections::HashSet;

use curvyo_document_core::{NodeId, ObjectSnapshot, Opacity};

/// The strength of the artwork outside the entered group (`--context-dim`, 35 %).
pub const CONTEXT_DIM: f64 = 0.35;

/// `objects` with every object that is not in `inside` drawn at [`CONTEXT_DIM`] of its fill and
/// stroke opacity, in the same order. The objects inside are cloned as they are.
#[must_use]
pub fn dim_outside(objects: &[ObjectSnapshot], inside: &HashSet<NodeId>) -> Vec<ObjectSnapshot> {
    objects
        .iter()
        .map(|object| {
            let mut shown = object.clone();
            if !inside.contains(&object.id()) {
                let style = shown.style_mut();
                let dim =
                    |opacity: Opacity| Opacity::new(opacity.get() * CONTEXT_DIM).unwrap_or(opacity);
                style.stroke.opacity = dim(style.stroke.opacity);
                style.fill.opacity = dim(style.fill.opacity);
            }
            shown
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Document, Length, Point, RectBounds};

    use super::*;

    fn rect(document: &Document, x: f64) -> ObjectSnapshot {
        let id = document.create_rect(RectBounds {
            origin: Point::new(x, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        });
        document.object(id).unwrap()
    }

    #[test]
    fn only_the_objects_outside_are_dimmed_to_35_percent() {
        let document = Document::new(1);
        let (inside, outside) = (rect(&document, 0.0), rect(&document, 20.0));
        let keep: HashSet<NodeId> = [inside.id()].into_iter().collect();
        let shown = dim_outside(&[inside.clone(), outside.clone()], &keep);
        assert_eq!(
            shown[0], inside,
            "the context's own content is drawn normally"
        );
        assert_eq!(shown[1].id(), outside.id());
        let before = outside.style();
        let after = shown[1].style();
        assert!((after.stroke.opacity.get() - before.stroke.opacity.get() * 0.35).abs() < 1e-12);
        assert!((after.fill.opacity.get() - before.fill.opacity.get() * 0.35).abs() < 1e-12);
        assert_eq!(
            after.stroke.width, before.stroke.width,
            "nothing else changes"
        );
        assert_eq!(after.stroke.color, before.stroke.color);
    }
}
