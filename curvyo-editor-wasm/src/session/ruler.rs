//! `Session`'s ruler layout for the live view and the display unit
//! (`specs/0015-document-size-and-rulers/` criteria 1 to 9), counting from the
//! active page's origin corner (`specs/0090-grid` criterion 17). The frontend
//! draws; every number and every label comes from here.

use curvyo_ui_core::{RulerAxis, RulerDirection, RulerLayout, ruler_layout_in};

use super::Session;

impl Session {
    /// The tick layout of the ruler along `axis`, `length_px` long, for the
    /// live view, in the display unit. `digit_px` and `minus_px` are the
    /// widest digit's and the minus sign's advance in the label font. The 0 is
    /// at the active page's origin corner and values count into the page.
    #[must_use]
    pub fn ruler_layout(
        &self,
        axis: RulerAxis,
        length_px: f64,
        digit_px: f64,
        minus_px: f64,
    ) -> RulerLayout {
        let page = self.active_page();
        let zero = page.zero_point();
        let origin = self.view().screen_to_document(0.0, 0.0);
        let reversed = match axis {
            RulerAxis::Horizontal => page.grid.origin.x_reversed(),
            RulerAxis::Vertical => page.grid.origin.y_reversed(),
        };
        ruler_layout_in(
            curvyo_document_core::ViewTransform::new(
                self.view().scale(),
                curvyo_document_core::Point::new(origin.x - zero.x, origin.y - zero.y),
            ),
            axis,
            if reversed {
                RulerDirection::Reversed
            } else {
                RulerDirection::Forward
            },
            length_px,
            self.display_unit(),
            digit_px,
            minus_px,
        )
    }

    /// The pointer at canvas pixel `(x, y)` as the rulers measure it: from the active page's
    /// origin corner, positive into the page (`specs/0090-grid` criteria 17 and 19), for the
    /// status bar's cursor readout.
    #[must_use]
    pub fn ruler_position(&self, x: f64, y: f64) -> curvyo_document_core::Vec2 {
        self.active_page().measured(self.screen_to_document(x, y))
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{DisplayUnit, DocumentSize, Point};

    use super::*;

    /// Criterion 11a: after `show_default_view` the document corner is 128 px
    /// right of and below the canvas corner, and a click there is document
    /// (0, 0).
    #[test]
    fn the_default_view_puts_the_document_corner_128_px_in() {
        let mut session = Session::new(1);
        session.show_default_view();
        let corner = session.screen_to_document(128.0, 128.0);
        assert!(corner.x.abs() < 1e-9 && corner.y.abs() < 1e-9, "{corner:?}");
        let (x, y) = session.view().document_to_screen(Point::new(0.0, 0.0));
        assert!((x - 128.0).abs() < 1e-9 && (y - 128.0).abs() < 1e-9);
    }

    /// Criteria 3 and 9: the 0 tick is at the document corner, and a tick
    /// converts back to the document value it is labelled with.
    #[test]
    fn the_ruler_zero_is_at_the_document_corner_and_ticks_convert_back() {
        let mut session = Session::new(1);
        session.show_default_view();
        let layout = session.ruler_layout(RulerAxis::Horizontal, 800.0, 7.0, 7.0);
        let origin = layout.origin_px.unwrap();
        assert!((origin - 128.0).abs() < 1e-9);
        // The tick labelled "100" (mm) at 100 % is where a click lands at
        // document x = 100 mm.
        let label = layout.labels.iter().find(|l| l.text == "100").unwrap();
        let document = session.screen_to_document(label.tick_px, 0.0);
        assert!((document.x - 100.0).abs() < 1e-9, "{document:?}");
    }

    /// `specs/0090-grid` criterion 17: with a bottom-left origin the vertical
    /// ruler shows 0 at the page's bottom edge and counts upward; a top-right
    /// origin puts the horizontal 0 at the right edge and counts to the left.
    #[test]
    fn the_rulers_count_into_the_page_from_its_origin_corner() {
        let mut session = Session::new(1);
        session.show_default_view();
        let page = session.active_page();
        let (w, h) = (page.size.width.as_mm(), page.size.height.as_mm());
        assert!(session.set_active_page_origin("bottom_left"));
        let vertical = session.ruler_layout(RulerAxis::Vertical, 4000.0, 7.0, 7.0);
        let (_, bottom) = session.view().document_to_screen(Point::new(0.0, h));
        assert!((vertical.origin_px.unwrap() - bottom).abs() < 1e-9);
        let label = vertical.labels.iter().find(|l| l.text == "100").unwrap();
        let document = session.screen_to_document(0.0, label.tick_px);
        assert!((document.y - (h - 100.0)).abs() < 1e-9, "{document:?}");
        let horizontal = session.ruler_layout(RulerAxis::Horizontal, 4000.0, 7.0, 7.0);
        assert!((horizontal.origin_px.unwrap() - 128.0).abs() < 1e-9);

        assert!(session.set_active_page_origin("top_right"));
        let horizontal = session.ruler_layout(RulerAxis::Horizontal, 4000.0, 7.0, 7.0);
        let (right, _) = session.view().document_to_screen(Point::new(w, 0.0));
        assert!((horizontal.origin_px.unwrap() - right).abs() < 1e-9);
        let label = horizontal.labels.iter().find(|l| l.text == "100").unwrap();
        let document = session.screen_to_document(label.tick_px, 0.0);
        assert!((document.x - (w - 100.0)).abs() < 1e-9, "{document:?}");
        let vertical = session.ruler_layout(RulerAxis::Vertical, 4000.0, 7.0, 7.0);
        assert!((vertical.origin_px.unwrap() - 128.0).abs() < 1e-9);
    }

    /// `specs/0090-grid` criterion 17: switching the active page switches the
    /// rulers to that page's corner at once.
    #[test]
    fn switching_the_active_page_switches_the_ruler_corner() {
        let mut session = Session::new(1);
        session.resize_viewport(4000.0, 4000.0);
        let first = session.active_page().id;
        let second = session.add_page();
        assert!(session.set_active_page_origin("bottom_right"));
        let page = session.active_page();
        let zero = page.zero_point();
        let layout = session.ruler_layout(RulerAxis::Horizontal, 4000.0, 7.0, 7.0);
        let (x, _) = session.view().document_to_screen(zero);
        assert!((layout.origin_px.unwrap() - x).abs() < 1e-9);
        assert!(session.activate_page(&first));
        let first_page = session.active_page();
        let layout = session.ruler_layout(RulerAxis::Horizontal, 4000.0, 7.0, 7.0);
        let (x, _) = session.view().document_to_screen(first_page.origin);
        assert!((layout.origin_px.unwrap() - x).abs() < 1e-9);
        assert_ne!(first, second);
    }

    /// `specs/0090-grid` criterion 19: the cursor readout measures like the rulers, from the
    /// active page's origin corner and positive into the page.
    #[test]
    fn the_cursor_readout_measures_from_the_origin_corner() {
        let mut session = Session::new(1);
        session.show_default_view();
        let h = session.active_page().size.height.as_mm();
        let (sx, sy) = session
            .view()
            .document_to_screen(Point::new(30.0, h - 20.0));
        let top_left = session.ruler_position(sx, sy);
        assert!((top_left.x - 30.0).abs() < 1e-9 && (top_left.y - (h - 20.0)).abs() < 1e-9);
        assert!(session.set_active_page_origin("bottom_left"));
        let bottom_left = session.ruler_position(sx, sy);
        assert!((bottom_left.x - 30.0).abs() < 1e-9 && (bottom_left.y - 20.0).abs() < 1e-9);
    }

    /// The unit follows the document: in inches the corner, the texts and the
    /// ruler step change, the document does not.
    #[test]
    fn the_display_unit_drives_the_ruler_and_the_texts() {
        let session = Session::new(1);
        assert!(session.document.set_display_unit(DisplayUnit::In));
        assert_eq!(session.display_unit(), DisplayUnit::In);
        assert_eq!(session.size_text(), "8.268 \u{d7} 11.693 in");
        let layout = session.ruler_layout(RulerAxis::Horizontal, 800.0, 7.0, 7.0);
        assert!((layout.step() - 0.5).abs() < 1e-9, "{}", layout.step());
        assert_eq!(session.document.size(), DocumentSize::default());
    }
}
