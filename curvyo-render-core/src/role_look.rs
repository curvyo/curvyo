//! The looks a leaf is drawn in instead of its own style (`specs/0027-manufacturing-roles`
//! `adrs.md` note 3, UX notes §5): a guide's marking, and the three role looks of the Laser output
//! page, whose widths and dashes are screen pixels at the tessellation view.

use curvyo_document_core::{
    ImageSnapshot, ObjectSnapshot, Opacity, Style, ViewTransform, drawing_outline_of_rotated,
};

use crate::artwork::{FrameBudgets, draw_object, fill_layer, solid_stroke_layer};
use crate::color::RgbaColor;
use crate::glyphs::DrawList;
use crate::image_layer::frame_anchors;
use crate::shape_preview::outline_to_anchors;
use crate::stroke::{OutlineRef, StrokeParams};
use crate::theme;

/// How one leaf is drawn instead of its own style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// A guide's marking (`specs/0046-document-shapes` criterion 5), at its place in the order.
    Guide,
    /// The outline only, dashed 4 px on and 3 px off, 1 px wide: Ignore and No role.
    Muted(RgbaColor),
    /// The outline only, 1 px solid over a white casing: Cut.
    Outline(RgbaColor),
    /// The area filled flat, no outline; a leaf whose outlines are all open draws a 2 px line,
    /// an image a grey-scale picture: Engrave.
    Area(RgbaColor),
}

/// The role looks of one build, kept apart so the areas lie under the dashes and the dashes under
/// the Cut lines, whatever the stacking (UX notes §5; the picture only).
#[derive(Default)]
pub(crate) struct RoleLayers {
    areas: DrawList,
    muted: DrawList,
    cuts: DrawList,
}

impl RoleLayers {
    /// Draws `object` in `look` (any look but [`Look::Guide`]); its style and opacity are ignored
    /// (criteria 7 and 13).
    pub(crate) fn draw(
        &mut self,
        object: &ObjectSnapshot,
        look: Look,
        view: ViewTransform,
        budgets: &mut FrameBudgets,
    ) {
        if let (Look::Area(_), ObjectSnapshot::Image(image)) = (look, object) {
            self.areas.push_image_as(image, Opacity::OPAQUE, true);
            return;
        }
        with_outlines(object, |outlines| match look {
            Look::Guide => {}
            Look::Muted(color) => {
                let style = crate::guide_style::dashed_outline_style(
                    view,
                    color,
                    theme::ROLE_LINE_PX,
                    (theme::ROLE_IGNORE_DASH_PX, theme::ROLE_IGNORE_GAP_PX),
                );
                draw_object(&mut self.muted, outlines, &style, false, view, budgets);
            }
            Look::Outline(color) => {
                let width = theme::ROLE_LINE_PX / view.scale();
                for (width, color) in [
                    (width * theme::CASING_WIDTH_FACTOR, theme::SELECTION_CASING),
                    (width, color),
                ] {
                    if let Some(layer) = solid_stroke_layer(outlines, &line(width, color, view)) {
                        self.cuts.extend_artwork(layer);
                    }
                }
            }
            Look::Area(color) => {
                let layer = if outlines.iter().any(|outline| outline.closed) {
                    fill_layer(outlines, color, view)
                } else {
                    let width = theme::ROLE_ENGRAVE_LINE_PX / view.scale();
                    solid_stroke_layer(outlines, &line(width, color, view))
                };
                if let Some(layer) = layer {
                    self.areas.extend_artwork(layer);
                }
            }
        });
    }

    /// Appends the areas, then the dashes, then the Cut lines to `list`.
    pub(crate) fn append_to(self, list: &mut DrawList) {
        list.extend(self.areas);
        list.extend(self.muted);
        list.extend(self.cuts);
    }
}

/// A solid line `width_mm` wide in `color`, with the default join and cap.
fn line(width_mm: f64, color: RgbaColor, view: ViewTransform) -> StrokeParams {
    let stroke = Style::default().stroke;
    StrokeParams {
        width_mm,
        color,
        tolerance_mm: theme::DISPLAY_TOLERANCE_PX / view.scale(),
        join: stroke.join,
        cap: stroke.cap,
    }
}

/// Calls `draw` with the outlines of `object` as drawn: a path's, a clone's or a sketch's
/// subpaths, a text's laid-out glyphs (a single-line text's centre lines), a primitive's fine
/// outline, an image's frame. A dimension and a text not laid out have none.
fn with_outlines(object: &ObjectSnapshot, draw: impl FnOnce(&[OutlineRef<'_>])) {
    match object {
        ObjectSnapshot::Primitive(primitive) => {
            let drawn = drawing_outline_of_rotated(&primitive.shape, primitive.rotation);
            let anchors = outline_to_anchors(&drawn);
            draw(&[OutlineRef {
                anchors: &anchors,
                closed: drawn.closed,
            }]);
        }
        ObjectSnapshot::Image(image) => draw_frame(image, draw),
        ObjectSnapshot::Dimension(_) => {}
        ObjectSnapshot::Path(_)
        | ObjectSnapshot::Clone(_)
        | ObjectSnapshot::Sketch(_)
        | ObjectSnapshot::Text(_) => {
            if let Some(path) = object.drawn_path() {
                let outlines: Vec<OutlineRef<'_>> = path
                    .subpaths()
                    .map(|subpath| OutlineRef {
                        anchors: subpath.anchors,
                        closed: subpath.closed,
                    })
                    .collect();
                draw(&outlines);
            }
        }
    }
}

fn draw_frame(image: &ImageSnapshot, draw: impl FnOnce(&[OutlineRef<'_>])) {
    let anchors = frame_anchors(image);
    draw(&[OutlineRef {
        anchors: &anchors,
        closed: true,
    }]);
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use curvyo_document_core::{
        Affine, AnchorId, Color, Document, NewAnchor, NewImage, NodeId, Point, RectBounds,
        StyleEdit,
    };

    use super::*;
    use crate::artwork::build_artwork_with_looks;

    const CUT: Look = Look::Outline(theme::ROLE_CUT);
    const ENGRAVE: Look = Look::Area(theme::ROLE_ENGRAVE);
    const IGNORE: Look = Look::Muted(theme::ROLE_IGNORE);

    fn view(scale: f64) -> ViewTransform {
        ViewTransform::new(scale, Point::new(0.0, 0.0))
    }

    fn objects(document: &Document) -> Vec<ObjectSnapshot> {
        document.read_objects().objects.to_vec()
    }

    fn rect(document: &Document, x: f64) -> NodeId {
        document.create_rect(RectBounds::from_corners(
            Point::new(x, 0.0),
            Point::new(x + 50.0, 50.0),
        ))
    }

    fn line_path(document: &Document) -> NodeId {
        document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(100.0, 0.0)),
            ],
            false,
        )
    }

    fn all(ids: &[NodeId], look: Look) -> HashMap<NodeId, Look> {
        ids.iter().map(|&id| (id, look)).collect()
    }

    fn colours(list: &DrawList) -> Vec<RgbaColor> {
        let mut seen = Vec::new();
        for vertex in &list.triangles {
            if !seen.contains(&vertex.color) {
                seen.push(vertex.color);
            }
        }
        seen
    }

    /// The height of the triangles of `colour` along a horizontal line, in screen pixels.
    fn thickness(list: &DrawList, colour: RgbaColor, scale: f64) -> f64 {
        let ys: Vec<f64> = list
            .triangles
            .iter()
            .filter(|v| v.color == colour)
            .map(|v| v.position.y)
            .collect();
        let max = ys.iter().copied().fold(f64::MIN, f64::max);
        let min = ys.iter().copied().fold(f64::MAX, f64::min);
        (max - min) * scale
    }

    #[test]
    fn ac7_a_wide_red_stroke_and_a_blue_fill_draw_the_same_cut_hairline_and_engrave_area() {
        let d = Document::new(1);
        let stroked = rect(&d, 0.0);
        d.edit_style(
            &[stroked],
            &StyleEdit::StrokeWidth(curvyo_document_core::Length::from_mm(10.0)),
        )
        .unwrap();
        d.edit_style(
            &[stroked],
            &StyleEdit::StrokeColor(Color { r: 255, g: 0, b: 0 }),
        )
        .unwrap();
        d.edit_style(&[stroked], &StyleEdit::FillEnabled(false))
            .unwrap();
        let filled = rect(&d, 0.0);
        d.edit_style(&[filled], &StyleEdit::FillEnabled(true))
            .unwrap();
        d.edit_style(
            &[filled],
            &StyleEdit::FillColor(Color { r: 0, g: 0, b: 255 }),
        )
        .unwrap();
        d.edit_style(&[filled], &StyleEdit::StrokeEnabled(false))
            .unwrap();
        let all_objects = objects(&d);
        for look in [CUT, ENGRAVE] {
            let draw = |object: &ObjectSnapshot| {
                build_artwork_with_looks(
                    std::slice::from_ref(object),
                    &all(&[object.id()], look),
                    view(2.0),
                )
            };
            let a = draw(&all_objects[0]);
            let b = draw(&all_objects[1]);
            assert_eq!(
                format!("{:?}", a.triangles),
                format!("{:?}", b.triangles),
                "{look:?}"
            );
            assert_ne!(a.triangle_count(), 0);
        }
    }

    #[test]
    fn ac6_cut_is_a_cased_one_pixel_outline_at_every_zoom_without_fill() {
        let d = Document::new(1);
        let id = line_path(&d);
        for scale in [0.05, 1.0, 40.0] {
            let list = build_artwork_with_looks(&objects(&d), &all(&[id], CUT), view(scale));
            assert_eq!(
                colours(&list),
                vec![theme::SELECTION_CASING, theme::ROLE_CUT]
            );
            let px = thickness(&list, theme::ROLE_CUT, scale);
            assert!((px - theme::ROLE_LINE_PX).abs() < 0.01, "{scale}: {px}");
            let casing = thickness(&list, theme::SELECTION_CASING, scale);
            assert!((casing - 3.0).abs() < 0.01, "{scale}: {casing}");
        }
        let closed = Document::new(1);
        let id = rect(&closed, 0.0);
        closed
            .edit_style(&[id], &StyleEdit::FillEnabled(true))
            .unwrap();
        let list = build_artwork_with_looks(&objects(&closed), &all(&[id], CUT), view(1.0));
        assert_eq!(list.layers().len(), 2, "casing and line, no fill layer");
    }

    #[test]
    fn ac8_engrave_fills_a_closed_leaf_and_draws_an_all_open_leaf_as_a_two_pixel_line() {
        let d = Document::new(1);
        let closed = rect(&d, 0.0);
        let list = build_artwork_with_looks(&objects(&d), &all(&[closed], ENGRAVE), view(1.0));
        assert_eq!(colours(&list), vec![theme::ROLE_ENGRAVE]);
        assert!(
            thickness(&list, theme::ROLE_ENGRAVE, 1.0) > 49.0,
            "the area"
        );
        let open = Document::new(1);
        let id = line_path(&open);
        for scale in [0.5, 8.0] {
            let list = build_artwork_with_looks(&objects(&open), &all(&[id], ENGRAVE), view(scale));
            let px = thickness(&list, theme::ROLE_ENGRAVE, scale);
            assert!((px - theme::ROLE_ENGRAVE_LINE_PX).abs() < 0.01, "{px}");
        }
    }

    #[test]
    fn ac6_ignore_is_a_dashed_muted_outline_without_casing() {
        let d = Document::new(1);
        let id = line_path(&d);
        let list = build_artwork_with_looks(&objects(&d), &all(&[id], IGNORE), view(1.0));
        assert_eq!(colours(&list), vec![theme::ROLE_IGNORE]);
        // 100 px of line at 4 on, 3 off: 15 dashes of one quad each.
        assert_eq!(list.triangle_count(), 15 * 2);
    }

    #[test]
    fn engrave_areas_lie_under_ignore_dashes_and_cut_lines_whatever_the_stacking() {
        let d = Document::new(1);
        let cut = rect(&d, 0.0);
        let ignore = rect(&d, 10.0);
        let engrave = rect(&d, 20.0);
        let looks = HashMap::from([(cut, CUT), (ignore, IGNORE), (engrave, ENGRAVE)]);
        let list = build_artwork_with_looks(&objects(&d), &looks, view(1.0));
        let order: Vec<RgbaColor> = colours(&list);
        assert_eq!(
            order,
            vec![
                theme::ROLE_ENGRAVE,
                theme::ROLE_IGNORE,
                theme::SELECTION_CASING,
                theme::ROLE_CUT
            ]
        );
    }

    #[test]
    fn ac8_an_image_is_grey_under_engrave_and_its_frame_otherwise() {
        let d = Document::new(1);
        let id = d
            .add_images(&[NewImage {
                bytes: std::sync::Arc::from(&b"x"[..]),
                placement: Affine::new(10.0, 0.0, 0.0, 5.0, 0.0, 0.0),
                name: String::new(),
            }])
            .unwrap()[0];
        let mut images = objects(&d);
        if let ObjectSnapshot::Image(image) = &mut images[0] {
            image.shown_opacity = Opacity::new(0.0).unwrap();
        }
        let engraved = build_artwork_with_looks(&images, &all(&[id], ENGRAVE), view(1.0));
        assert_eq!(engraved.images.len(), 1);
        assert!(engraved.images[0].grey);
        assert!(
            (engraved.images[0].opacity.get() - 1.0).abs() < 1e-12,
            "full strength"
        );
        for look in [CUT, IGNORE] {
            let list = build_artwork_with_looks(&images, &all(&[id], look), view(1.0));
            assert!(list.images.is_empty(), "{look:?}");
            assert!(!list.triangles.is_empty(), "{look:?}: the frame");
        }
    }

    #[test]
    fn objects_without_a_look_and_guides_draw_as_before() {
        let d = Document::new(1);
        let id = rect(&d, 0.0);
        let plain = crate::build_artwork(&objects(&d), view(1.0));
        let none = build_artwork_with_looks(&objects(&d), &HashMap::new(), view(1.0));
        assert_eq!(
            format!("{:?}", plain.triangles),
            format!("{:?}", none.triangles)
        );
        let guides = std::collections::HashSet::from([id]);
        let as_guide = crate::build_artwork_with_guides(&objects(&d), &guides, view(1.0));
        let as_look = build_artwork_with_looks(&objects(&d), &all(&[id], Look::Guide), view(1.0));
        assert_eq!(
            format!("{:?}", as_guide.triangles),
            format!("{:?}", as_look.triangles)
        );
    }

    #[test]
    fn the_role_tokens_are_the_css_tokens() {
        let css = include_str!("../../frontend/src/index.css");
        for (token, colour) in [
            ("--role-cut", theme::ROLE_CUT),
            ("--role-engrave", theme::ROLE_ENGRAVE),
            ("--role-ignore", theme::ROLE_IGNORE),
        ] {
            let hex = format!(
                "{token}: #{:02X}{:02X}{:02X};",
                colour.r, colour.g, colour.b
            );
            assert!(css.contains(&hex), "{hex}");
        }
    }
}
