//! The Test grid tool's outline under the pointer (`specs/0112` criterion 2, UX notes 3): the box
//! a press would place, drawn in the context outline's look.

use curvyo_document_core::{Point, ViewTransform};
use curvyo_render_core::DrawList;

use super::super::{Session, Tool};

impl Session {
    /// The outline of the grid a press at the pointer would place, at the spot it would take
    /// (the pointer, else the first free spot); nothing off the canvas, over a grid (a press
    /// selects it) or while the labels' face loads.
    pub(in crate::session) fn test_grid_draw_list(&self, view: ViewTransform) -> DrawList {
        let Some(at) = self.test_grid.hover.filter(|_| {
            self.tool == Tool::TestGrid && self.pointer_position.is_some() && !self.button_down
        }) else {
            return DrawList::default();
        };
        if self.grid_at(at).is_some() {
            return DrawList::default();
        }
        let Some(font) = self.grid_label_font() else {
            return DrawList::default();
        };
        let laid = self.grid_layout(&self.draft_grid(), &font);
        let (width, height) = (laid.width.as_mm(), laid.height.as_mm());
        let Ok(spot) = self.grid_spot(width, height, Some(at)) else {
            return DrawList::default();
        };
        let corners = [
            spot,
            Point::new(spot.x + width, spot.y),
            Point::new(spot.x + width, spot.y + height),
            Point::new(spot.x, spot.y + height),
        ];
        curvyo_render_core::build_context_outline(view, corners, self.device_pixel_ratio)
    }
}
