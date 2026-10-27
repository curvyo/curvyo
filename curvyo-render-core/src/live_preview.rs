//! The hollow outlines of the edit previews (`specs/0009-unified-object-editing/specification.md`,
//! criteria 10 to 15, and its note of 2026-10-27): the blue outline of geometry a release would
//! commit, and the ghost a Select-tool drag leaves at the origin while the moved objects are drawn
//! as themselves at the pointer. Both have a constant screen width; the artwork is not touched
//! here.

use std::borrow::Cow;

use curvyo_document_core::{
    AnchorSnapshot, ObjectSnapshot, ViewTransform, drawing_outline_of_rotated,
};

use crate::color::RgbaColor;
use crate::glyphs::DrawList;
use crate::shape_preview::outline_to_anchors;
use crate::stroke;
use crate::theme;

/// The hollow `--preview-new` outline of every object in `objects` (the
/// resolved geometry of a bar slider edit or a create preview): a primitive through
/// its rotated outline, a path through its anchors. 1.5 px wide on screen
/// whatever the object's own stroke width; no fill preview.
#[must_use]
pub fn build_live_edit_preview(objects: &[ObjectSnapshot], view: ViewTransform) -> DrawList {
    hollow_outlines(objects, view, theme::PREVIEW_NEW)
}

/// The ghost a Select-tool drag leaves behind (customer 2026-10-27, `docs/design-system.md` row
/// "Drag ghost"): the outline of every object in `objects` (the committed objects the drag moves)
/// in the look of [`build_live_edit_preview`], its line in `--drag-ghost`, while the
/// objects themselves are drawn at the pointer.
#[must_use]
pub fn build_drag_ghost(objects: &[ObjectSnapshot], view: ViewTransform) -> DrawList {
    hollow_outlines(objects, view, theme::DRAG_GHOST)
}

/// Every outline of `objects` as a 1.5 px line in `line` over its white casing.
fn hollow_outlines(objects: &[ObjectSnapshot], view: ViewTransform, line: RgbaColor) -> DrawList {
    let width_mm = theme::LIVE_PREVIEW_STROKE_PX / view.scale();
    let tolerance_mm = theme::DISPLAY_TOLERANCE_PX / view.scale();
    let outlines: Vec<Vec<(Cow<'_, [AnchorSnapshot]>, bool)>> =
        objects.iter().map(outlines_of).collect();
    let mut list = DrawList::default();
    // Every casing first, then every line: the casing of one object never
    // covers the line of another (`0007` criterion 40; the outline is always
    // above all artwork because it belongs to the overlay).
    for casing in [true, false] {
        let (width, colour) = if casing {
            (
                width_mm * theme::CASING_WIDTH_FACTOR,
                theme::SELECTION_CASING,
            )
        } else {
            (width_mm, line)
        };
        for (anchors, closed) in outlines.iter().flatten() {
            list.extend(stroke::path_stroke(
                anchors,
                *closed,
                width,
                colour,
                tolerance_mm,
            ));
        }
    }
    list
}

/// The outlines of one object as anchors: a primitive's rotated outline, an image's frame, a
/// path's (or a text's glyph) subpaths.
fn outlines_of(object: &ObjectSnapshot) -> Vec<(Cow<'_, [AnchorSnapshot]>, bool)> {
    match object {
        ObjectSnapshot::Primitive(primitive) => {
            let outline = drawing_outline_of_rotated(&primitive.shape, primitive.rotation);
            vec![(Cow::Owned(outline_to_anchors(&outline)), outline.closed)]
        }
        ObjectSnapshot::Image(image) => {
            vec![(Cow::Owned(crate::image_layer::frame_anchors(image)), true)]
        }
        _ => object.drawn_path().map_or_else(Vec::new, |path| {
            path.subpaths()
                .map(|subpath| (Cow::Borrowed(subpath.anchors), subpath.closed))
                .collect()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{
        AnchorId, Document, Length, NewAnchor, Point, RectBounds, ViewTransform,
    };

    fn view(px_per_mm: f64) -> ViewTransform {
        ViewTransform::new(px_per_mm, Point::new(0.0, 0.0))
    }

    /// The `--preview-new` line triangles only, without the white casing.
    fn line(list: &DrawList) -> Vec<crate::glyphs::Vertex> {
        list.triangles
            .iter()
            .copied()
            .filter(|v| v.color == theme::PREVIEW_NEW)
            .collect()
    }

    #[test]
    fn a_primitive_draws_a_hollow_outline_in_the_preview_colour_only() {
        let document = Document::new(1);
        let id = document.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        });
        let list = build_live_edit_preview(&[document.object(id).expect("exists")], view(1.0));
        assert_ne!(list.triangle_count(), 0);
        assert!(
            list.triangles
                .iter()
                .all(|v| v.color == theme::PREVIEW_NEW || v.color == theme::SELECTION_CASING),
            "the line, over its white casing, and nothing else"
        );
        // Hollow: no triangle vertex lies in the middle of the rectangle.
        assert!(
            list.triangles
                .iter()
                .all(|v| (v.position.x - 5.0).abs() > 2.5 || (v.position.y - 5.0).abs() > 2.5),
            "no fill preview"
        );
    }

    /// Customer 2026-10-27: the ghost has the outline's geometry, its line in the ghost colour.
    #[test]
    fn the_drag_ghost_is_the_same_outline_in_the_ghost_colour() {
        let document = Document::new(1);
        let id = document.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        });
        let object = document.object(id).expect("exists");
        let blue = build_live_edit_preview(std::slice::from_ref(&object), view(1.0));
        let ghost = build_drag_ghost(std::slice::from_ref(&object), view(1.0));
        assert_eq!(ghost.triangle_count(), blue.triangle_count());
        for (g, b) in ghost.triangles.iter().zip(&blue.triangles) {
            assert_eq!(g.position, b.position);
            let expected = if b.color == theme::PREVIEW_NEW {
                theme::DRAG_GHOST
            } else {
                b.color
            };
            assert_eq!(g.color, expected);
        }
    }

    #[test]
    fn a_path_draws_through_its_anchors() {
        let document = Document::new(1);
        let id = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(20.0, 0.0)),
            ],
            false,
        );
        let list = build_live_edit_preview(&[document.object(id).expect("exists")], view(1.0));
        assert_ne!(list.triangle_count(), 0);
        let far = list
            .triangles
            .iter()
            .map(|v| v.position.x)
            .fold(f64::MIN, f64::max);
        assert!((far - 20.0).abs() < 1.0);
    }

    /// The outline is 1.5 px on screen at every zoom, not the object's own
    /// stroke width.
    #[test]
    fn the_outline_is_one_and_a_half_screen_pixels_at_every_zoom() {
        let document = Document::new(1);
        let id = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(100.0, 0.0)),
            ],
            false,
        );
        let object = document.object(id).expect("exists");
        for scale in [0.5, 1.0, 8.0] {
            let list = build_live_edit_preview(std::slice::from_ref(&object), view(scale));
            let (lo, hi) = line(&list)
                .iter()
                .fold((f64::MAX, f64::MIN), |(lo, hi), v| {
                    (lo.min(v.position.y), hi.max(v.position.y))
                });
            assert!(
                ((hi - lo) * scale - theme::LIVE_PREVIEW_STROKE_PX).abs() < 1e-6,
                "scale {scale}: {} px",
                (hi - lo) * scale
            );
        }
    }

    /// `0007` criterion 40: a white casing 1.5 px wider on each side (4.5 px
    /// in all), drawn before the line, for every object before any line.
    #[test]
    fn the_outline_sits_on_a_white_casing_one_line_width_wider_on_each_side() {
        let document = Document::new(1);
        let ids: Vec<_> = [0.0, 50.0]
            .into_iter()
            .map(|y| {
                document.create_path(
                    &[
                        NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, y)),
                        NewAnchor::corner(AnchorId::new(1, 2), Point::new(100.0, y)),
                    ],
                    false,
                )
            })
            .collect();
        let objects: Vec<_> = ids.iter().map(|id| document.object(*id).unwrap()).collect();
        let scale = 2.0;
        let list = build_live_edit_preview(&objects, view(scale));
        let casing: Vec<_> = list
            .triangles
            .iter()
            .filter(|v| v.color == theme::SELECTION_CASING)
            .collect();
        let (lo, hi) = casing
            .iter()
            .filter(|v| v.position.y < 25.0)
            .fold((f64::MAX, f64::MIN), |(lo, hi), v| {
                (lo.min(v.position.y), hi.max(v.position.y))
            });
        assert!(
            ((hi - lo) * scale - 3.0 * theme::LIVE_PREVIEW_STROKE_PX).abs() < 1e-6,
            "casing is {} px",
            (hi - lo) * scale
        );
        let last_casing = list
            .triangles
            .iter()
            .rposition(|v| v.color == theme::SELECTION_CASING)
            .unwrap();
        let first_line = list
            .triangles
            .iter()
            .position(|v| v.color == theme::PREVIEW_NEW)
            .unwrap();
        assert!(last_casing < first_line, "every casing before any line");
    }

    #[test]
    fn nothing_to_preview_draws_nothing() {
        assert_eq!(build_live_edit_preview(&[], view(1.0)).triangle_count(), 0);
    }
}
