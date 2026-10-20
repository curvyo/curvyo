//! Draws a text in edit mode (`specs/0074-text-objects`, UX notes §2 "Look in edit mode"): the
//! line box as the dashed selection outline, the selected range, the IME underline and the
//! caret, all at constant screen sizes and turned with the text.

use curvyo_document_core::{NodeId, Point, ViewTransform};

use crate::glyphs::{DrawList, thick_line};
use crate::select_box::{SelectDecorationInput, build};
use crate::theme;

/// What the session hands in: document mm, from `curvyo-ui-core`'s edit geometry (this crate
/// depends on no `curvyo-ui-core` type, ADR 0011 §3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextEditOverlay {
    /// The edited text, or a placeholder for a new one.
    pub id: Option<NodeId>,
    /// The line box corners; `None` for a new, empty text (no box, only the caret).
    pub line_box: Option<[Point; 4]>,
    /// The caret, ascender to descender; `None` while a range is selected or the caret blinks
    /// off.
    pub caret: Option<[Point; 2]>,
    /// The quads of the selected range.
    pub selection: Vec<[Point; 4]>,
    /// The composition's underlines.
    pub preedit: Vec<[Point; 2]>,
    /// `window.devicePixelRatio`.
    pub device_pixel_ratio: f64,
}

fn fill_quad(list: &mut DrawList, corners: [Point; 4], color: crate::color::RgbaColor) {
    for index in [0, 1, 2, 0, 2, 3] {
        list.push_vertex(crate::glyphs::Vertex {
            position: corners[index],
            color,
        });
    }
}

/// The caret at least [`theme::TEXT_CARET_MIN_PX`] long, centred on its middle.
fn long_enough(view: ViewTransform, [a, b]: [Point; 2]) -> [Point; 2] {
    let length_px = a.vector_to(b).length() * view.scale();
    if length_px >= theme::TEXT_CARET_MIN_PX || length_px <= 0.0 {
        return [a, b];
    }
    let grow = theme::TEXT_CARET_MIN_PX / length_px;
    let middle = Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y));
    let half = middle.vector_to(b);
    let half = curvyo_document_core::Vec2::new(half.x * grow, half.y * grow);
    [middle.translated(half.negated()), middle.translated(half)]
}

/// The overlay as a draw list, above the artwork.
#[must_use]
pub fn build_text_edit_overlay(view: ViewTransform, overlay: &TextEditOverlay) -> DrawList {
    let mut list = DrawList::default();
    let scale = view.scale();
    for quad in &overlay.selection {
        fill_quad(&mut list, *quad, theme::TEXT_SELECTION);
    }
    if let (Some(corners), Some(id)) = (overlay.line_box, overlay.id) {
        list.extend(build(
            view,
            &SelectDecorationInput {
                selected: vec![(id, corners)],
                device_pixel_ratio: overlay.device_pixel_ratio,
                ..SelectDecorationInput::default()
            },
        ));
    }
    for [a, b] in &overlay.preedit {
        list.extend(thick_line(
            *a,
            *b,
            theme::TEXT_PREEDIT_PX / scale,
            theme::TEXT_PREEDIT,
        ));
    }
    if let Some(caret) = overlay.caret {
        let [a, b] = long_enough(view, caret);
        let width = theme::TEXT_CARET_PX / scale;
        list.extend(thick_line(
            a,
            b,
            width + 2.0 * theme::TEXT_CARET_CASING_PX / scale,
            theme::SELECTION_CASING,
        ));
        list.extend(thick_line(a, b, width, theme::TEXT_CARET));
    }
    list
}
