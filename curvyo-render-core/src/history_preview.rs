//! An object version previewed from the History list (`specs/0042-history-branches` criterion 17):
//! a lighter copy of its own fill and stroke under the hollow `--preview-new` outline, drawn over
//! the committed objects, which stay as they are.

use curvyo_document_core::{ObjectSnapshot, Opacity, Style, ViewTransform};

use crate::artwork::build_artwork;
use crate::glyphs::DrawList;
use crate::live_preview::build_live_edit_preview;
use crate::theme;

fn ghost(opacity: Opacity) -> Opacity {
    // invariant: a value in [0, 1] times 0.4 stays in [0, 1].
    Opacity::new(opacity.get() * theme::PREVIEW_GHOST_OPACITY).unwrap_or(opacity)
}

fn ghost_style(style: &mut Style) {
    style.fill.opacity = ghost(style.fill.opacity);
    style.stroke.opacity = ghost(style.stroke.opacity);
}

/// The ghost artwork of `objects` (their own colours at 40 % of their opacity), then their blue
/// outline. The ghost joins the artwork as later layers, so it covers the committed objects; the
/// outline is overlay.
#[must_use]
pub fn build_history_preview(objects: &[ObjectSnapshot], view: ViewTransform) -> DrawList {
    let ghosts: Vec<ObjectSnapshot> = objects
        .iter()
        .cloned()
        .map(|mut object| {
            match &mut object {
                ObjectSnapshot::Path(path) => ghost_style(&mut path.style),
                ObjectSnapshot::Sketch(sketch) => {
                    ghost_style(&mut sketch.style);
                    ghost_style(&mut sketch.resolved.style);
                }
                ObjectSnapshot::Primitive(primitive) => ghost_style(&mut primitive.style),
                ObjectSnapshot::Clone(clone) => ghost_style(&mut clone.resolved.style),
                ObjectSnapshot::Image(image) => image.shown_opacity = ghost(image.shown_opacity),
                ObjectSnapshot::Dimension(_) => {}
                ObjectSnapshot::Text(text) => {
                    ghost_style(&mut text.style);
                    if let Some(outline) = &mut text.outline {
                        ghost_style(&mut std::sync::Arc::make_mut(outline).glyphs.style);
                    }
                }
            }
            object
        })
        .collect();
    let mut list = build_artwork(&ghosts, view);
    list.extend(build_live_edit_preview(objects, view));
    list
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Document, Length, Point, RectBounds};

    use super::*;

    #[test]
    fn the_ghost_is_artwork_and_the_outline_is_overlay() {
        let d = Document::new(1);
        let id = d.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        });
        let object = d.object(id).unwrap();
        let list = build_history_preview(&[object], ViewTransform::identity());
        assert!(!list.layers().is_empty(), "the ghost is artwork");
        assert!(
            list.triangle_count() * 3 > list.overlay_start(),
            "the outline is overlay"
        );
    }

    #[test]
    fn the_ghost_keeps_the_colours_at_forty_percent() {
        assert!((ghost(Opacity::OPAQUE).get() - 0.4).abs() < 1e-12);
    }
}
