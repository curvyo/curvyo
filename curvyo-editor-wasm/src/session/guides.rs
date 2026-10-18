//! `Session`'s glue for the Guide button of the Select bar and the Template button of the File
//! tab (`specs/0046-document-shapes` criteria 9, 10, 13, 19, 22 and 23): when the button acts and
//! shows pressed, its press, and the load of a template's bytes. The rules and the commits are
//! `curvyo_document_core`'s `set_guide` and `load_template`.

use curvyo_document_core::{GuideMarking, OpenError, Point};

use super::{Session, Tool};

/// The largest template file read: a bigger one is refused before it is opened (criterion 22).
pub const MAX_TEMPLATE_BYTES: usize = 64 * 1024 * 1024;

/// The peer the source is opened with: it is read and dropped, never edited or merged.
const SOURCE_PEER: u64 = 1;

/// Why a template was not loaded; nothing changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateError {
    /// The file is larger than [`MAX_TEMPLATE_BYTES`].
    TooLarge,
    /// The file is not a Curvyo project.
    NotAProject,
    /// The file is a damaged Curvyo project.
    Damaged,
    /// The file was saved by a newer build.
    TooNew,
}

impl From<OpenError> for TemplateError {
    fn from(error: OpenError) -> Self {
        match error {
            OpenError::NotAProject => Self::NotAProject,
            OpenError::Damaged => Self::Damaged,
            OpenError::FormatTooNew { .. } => Self::TooNew,
        }
    }
}

impl TemplateError {
    /// The one sentence the error dialog shows.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::TooLarge => "This file is too large to load as a template (over 64 MiB).",
            Self::NotAProject => "This file isn't a Curvyo project (.curvyo) file.",
            Self::Damaged => "This file is damaged and can't be read.",
            Self::TooNew => {
                "This file was saved by a newer version of Curvyo. Update the app to open it."
            }
        }
    }
}

impl Session {
    /// What the selection is as guides, for the Guide button: disabled outside the Select tool
    /// and when nothing selected can be a guide (criterion 9), pressed when everything that can
    /// is one (criterion 10).
    #[must_use]
    pub fn guide_marking(&self) -> GuideMarking {
        if self.tool != Tool::Select {
            return GuideMarking::default();
        }
        self.document.guide_marking(self.selection.ids())
    }

    /// The Guide button: marks every markable node of the selection as a guide, or, when all of
    /// them are guides, turns them back, as one step "Guide". The selection stays. Returns
    /// whether anything was written.
    pub fn toggle_guide(&mut self) -> bool {
        if self.tool != Tool::Select || self.select.drag_in_flight() {
            return false;
        }
        let marking = self.guide_marking();
        if !marking.enabled() {
            return false;
        }
        self.select.cancel_entry();
        let ids = self.selection.ids().to_vec();
        self.recording(|this| this.document.set_guide(&ids, !marking.all_guides()).is_ok())
    }

    /// The Template button: adds the outlines of the Curvyo document `bytes` as guides in one step
    /// "Template", selects them, and zooms out when the page and the new guides do not fit on the
    /// canvas (criterion 13). Returns how many came in.
    ///
    /// # Errors
    /// A `TemplateError` when the file is too large or cannot be opened; nothing changes.
    pub fn load_template(&mut self, bytes: &[u8]) -> Result<usize, TemplateError> {
        if bytes.len() > MAX_TEMPLATE_BYTES {
            return Err(TemplateError::TooLarge);
        }
        let source =
            curvyo_document_core::unpack(SOURCE_PEER, bytes).map_err(TemplateError::from)?;
        let nodes = source.template_nodes();
        drop(source);
        let count: usize = nodes
            .iter()
            .map(curvyo_document_core::TemplateNode::anchor_count)
            .sum();
        let anchor_ids: Vec<_> = (0..count).map(|_| self.minter.mint()).collect();
        self.select.cancel_entry();
        let loaded = self.recording(|this| {
            let loaded = this.document.load_template(&nodes, &anchor_ids);
            if let Ok(ids) = &loaded
                && !ids.is_empty()
            {
                this.selection.set(ids);
            }
            loaded
        });
        // invariant: the anchor ids were minted just now, one per anchor, so none is refused.
        let ids = loaded.unwrap_or_default();
        self.show_page_and_guides();
        Ok(ids.len())
    }

    /// Zooms out to the page and every guide when part of them is off the canvas.
    fn show_page_and_guides(&mut self) {
        let size = self.document.size();
        let mut min = Point::new(0.0, 0.0);
        let mut max = Point::new(size.width.as_mm(), size.height.as_mm());
        let read = self.document_read();
        for object in read.leaves.iter().filter(|o| read.guides.contains(&o.id())) {
            let (low, high) = curvyo_ui_core::object_outline_bounds(object);
            min = Point::new(min.x.min(low.x), min.y.min(low.y));
            max = Point::new(max.x.max(high.x), max.y.max(high.y));
        }
        self.viewport.show_area(min, max);
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, Point, RectBounds, pack};

    use super::super::{Session, Tool};
    use super::*;

    fn rect(session: &mut Session, from: (f64, f64), to: (f64, f64)) {
        session.set_tool(Tool::Rectangle);
        session.pointer_down(Point::new(from.0, from.1), false);
        session.pointer_up(Point::new(to.0, to.1), false, false);
    }

    fn template_bytes() -> Vec<u8> {
        let d = Document::new(7);
        let _ = d.create_rect(RectBounds::from_corners(
            Point::new(-300.0, -300.0),
            Point::new(900.0, 900.0),
        ));
        let _ = d.create_path(
            &[
                NewAnchor::corner(AnchorId::new(7, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(7, 2), Point::new(10.0, 10.0)),
            ],
            false,
        );
        pack(&d, "t").unwrap()
    }

    #[test]
    fn the_button_is_disabled_with_nothing_markable_and_toggles_as_one_step() {
        let mut session = Session::new(1);
        assert!(!session.guide_marking().enabled());
        assert!(!session.toggle_guide());
        rect(&mut session, (0.0, 0.0), (30.0, 10.0));
        rect(&mut session, (40.0, 0.0), (50.0, 30.0));
        let ids = session.document.object_ids();
        session.selection.set(&ids);
        assert!(session.guide_marking().enabled() && !session.guide_marking().all_guides());
        let steps = session.document.own_steps().len();
        assert!(session.toggle_guide());
        assert_eq!(session.document.own_steps().len(), steps + 1);
        assert!(session.guide_marking().all_guides(), "pressed");
        assert_eq!(session.selection.ids(), &ids[..], "the selection stays");
        assert!(session.toggle_guide());
        assert!(!session.guide_marking().all_guides());
        session.set_tool(Tool::Node);
        assert!(!session.guide_marking().enabled());
    }

    #[test]
    fn a_click_inside_a_closed_guide_misses_and_one_on_its_line_hits() {
        let mut session = Session::new(1);
        rect(&mut session, (0.0, 0.0), (100.0, 100.0));
        let id = session.document.object_ids()[0];
        session
            .document
            .edit_style(&[id], &curvyo_document_core::StyleEdit::FillEnabled(true))
            .unwrap();
        session.document.set_guide(&[id], true).unwrap();
        session.set_tool(Tool::Select);
        session.selection.clear();
        session.pointer_down(Point::new(50.0, 50.0), false);
        session.pointer_up(Point::new(50.0, 50.0), false, false);
        assert!(session.selection.ids().is_empty(), "the inside is not hit");
        session.pointer_down(Point::new(0.0, 50.0), false);
        session.pointer_up(Point::new(0.0, 50.0), false, false);
        assert_eq!(session.selection.ids(), &[id]);
    }

    #[test]
    fn a_template_loads_as_one_selected_step_and_the_view_shows_it() {
        let mut session = Session::new(1);
        session.resize_viewport(1000.0, 800.0);
        let steps = session.document.own_steps().len();
        assert_eq!(session.load_template(&template_bytes()), Ok(2));
        assert_eq!(session.document.own_steps().len(), steps + 1);
        assert_eq!(session.selection.ids().len(), 2);
        assert!(session.zoom_percent() < 100, "the overhang is shown");
        assert!(
            session.selection.ids().iter().all(|&id| session
                .document
                .node_registers(id)
                .unwrap()
                .guide)
        );
        session.undo();
        assert_eq!(session.document.object_ids().len(), 0);
    }

    #[test]
    fn a_file_that_is_too_large_or_not_a_project_changes_nothing() {
        let mut session = Session::new(1);
        let steps = session.document.own_steps().len();
        assert_eq!(
            session.load_template(&vec![0; MAX_TEMPLATE_BYTES + 1]),
            Err(TemplateError::TooLarge)
        );
        assert_eq!(
            session.load_template(b"not a zip"),
            Err(TemplateError::NotAProject)
        );
        assert_eq!(session.document.own_steps().len(), steps);
        assert!(TemplateError::TooLarge.message().contains("64 MiB"));
    }
}
