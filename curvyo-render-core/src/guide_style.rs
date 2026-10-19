//! The marking style a guide is drawn in (`specs/0046-document-shapes` criteria 5, 8 and 33,
//! `adrs.md` decision 3): its outline only, in `--guide`, one screen pixel wide, dashed 2 px on
//! and 2 px off on the screen, whatever its own fill and stroke are.

use curvyo_document_core::{DashPattern, Length, LineCap, Style, ViewTransform};

use crate::theme;

/// The style a guide's outlines are stroked with at `view`: the width and the dash lengths are
/// screen pixels turned into millimetres, so they stay the same on the screen at every zoom; the
/// dash restarts at the first node of every outline as any dash does. No fill, no markers.
#[must_use]
pub(crate) fn marking_style(view: ViewTransform) -> Style {
    let mut style = Style::default();
    style.fill.enabled = false;
    style.stroke.enabled = true;
    style.stroke.color = curvyo_document_core::Color {
        r: theme::GUIDE.r,
        g: theme::GUIDE.g,
        b: theme::GUIDE.b,
    };
    style.stroke.opacity = curvyo_document_core::Opacity::OPAQUE;
    style.stroke.width = Length::from_mm(theme::GUIDE_LINE_PX / view.scale());
    style.stroke.cap = LineCap::Butt;
    // The pattern is in multiples of the width: 2 px on, 2 px off at a 1 px line.
    style.stroke.dash = DashPattern::new(vec![
        theme::GUIDE_MARK_DASH_PX / theme::GUIDE_LINE_PX,
        theme::GUIDE_MARK_GAP_PX / theme::GUIDE_LINE_PX,
    ])
    .unwrap_or_else(|_| DashPattern::solid());
    style
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use curvyo_document_core::{
        AnchorId, Document, NewAnchor, NodeId, ObjectSnapshot, Point, RectBounds, StyleEdit,
    };

    use crate::artwork::{build_artwork, build_artwork_with_guides};
    use crate::glyphs::DrawList;

    use super::*;

    fn line(document: &Document, length: f64) -> NodeId {
        document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(length, 0.0)),
            ],
            false,
        )
    }

    fn objects(document: &Document) -> Vec<ObjectSnapshot> {
        document.read_objects().objects.to_vec()
    }

    fn view(scale: f64) -> ViewTransform {
        ViewTransform::new(scale, Point::new(0.0, 0.0))
    }

    fn layer_count(list: &DrawList) -> usize {
        list.layers().len()
    }

    #[test]
    fn a_guide_is_one_layer_of_guide_colour_and_its_fill_is_not_drawn() {
        let d = Document::new(1);
        let id = d.create_rect(RectBounds::from_corners(
            Point::new(0.0, 0.0),
            Point::new(50.0, 50.0),
        ));
        d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
        let guides = HashSet::from([id]);
        let list = build_artwork_with_guides(&objects(&d), &guides, view(1.0));
        assert_eq!(layer_count(&list), 1, "one stroke run, no fill layer");
        assert!(list.triangles.iter().all(|v| v.color == theme::GUIDE));
        assert_eq!(
            layer_count(&build_artwork(&objects(&d), view(1.0))),
            2,
            "the same rectangle as an ordinary node: fill and stroke"
        );
    }

    #[test]
    fn the_line_is_one_screen_pixel_and_the_dash_two_on_two_off_at_every_zoom() {
        let d = Document::new(1);
        let id = line(&d, 100.0);
        let guides = HashSet::from([id]);
        for scale in [0.01, 1.0, 64.0] {
            let list = build_artwork_with_guides(&objects(&d), &guides, view(scale));
            let ys = list.triangles.iter().map(|v| v.position.y);
            let thickness = ys.clone().fold(f64::MIN, f64::max) - ys.fold(f64::MAX, f64::min);
            assert!(
                (thickness * scale - theme::GUIDE_LINE_PX).abs() < 0.01,
                "{scale}: {}",
                thickness * scale
            );
            let screen_length = 100.0 * scale;
            let dashes = screen_length / (theme::GUIDE_MARK_DASH_PX + theme::GUIDE_MARK_GAP_PX);
            if dashes < 2000.0 {
                // A dash is one quad, two triangles.
                #[allow(clippy::cast_precision_loss)]
                let drawn = list.triangle_count() as f64 / 2.0;
                assert!(
                    (drawn - dashes.ceil()).abs() <= 1.0,
                    "{scale}: {drawn} {dashes}"
                );
            }
        }
    }

    #[test]
    fn guides_keep_their_place_in_the_stacking_order() {
        let d = Document::new(1);
        let below = d.create_rect(RectBounds::from_corners(
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
        ));
        let guide = line(&d, 100.0);
        let above = d.create_rect(RectBounds::from_corners(
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
        ));
        let _ = (below, above);
        let list = build_artwork_with_guides(&objects(&d), &HashSet::from([guide]), view(1.0));
        let layers = list.layers();
        assert_eq!(layers.len(), 3);
        let middle = &list.triangles[layers[0]..layers[1]];
        assert!(middle.iter().all(|v| v.color == theme::GUIDE));
        assert!(
            list.triangles[..layers[0]]
                .iter()
                .all(|v| v.color != theme::GUIDE)
        );
    }

    #[test]
    fn five_thousand_guides_add_one_run_each() {
        let d = Document::new(1);
        let ids: HashSet<NodeId> = (0..5000_u64)
            .map(|n| {
                #[allow(clippy::cast_precision_loss)]
                let x = n as f64 * 20.0;
                d.create_rect(RectBounds::from_corners(
                    Point::new(x, 0.0),
                    Point::new(x + 10.0, 100_000.0),
                ))
            })
            .collect();
        let list = build_artwork_with_guides(&objects(&d), &ids, view(0.01));
        assert_eq!(layer_count(&list), 5000);
        assert!(
            list.triangles
                .iter()
                .all(|v| v.position.x.is_finite() && v.position.y.is_finite())
        );
    }
}
