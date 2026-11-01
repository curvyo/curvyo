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
    /// The copy was refused by the document (not expected: the anchor ids are fresh).
    Refused,
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
            Self::Refused => "The template could not be loaded. Nothing was changed.",
            Self::TooNew => {
                "This file was saved by a newer version of Curvyo. Update the app to open it."
            }
        }
    }
}

/// How the Guide button shows (criteria 9, 10 and 34).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideButtonState {
    /// Nothing selected can be a guide: greyed.
    Disabled,
    /// Something can be marked and not everything is a guide.
    Off,
    /// Everything markable is a guide: a press turns them back.
    Pressed,
    /// Only linked clones of guides are selected: they are drawn as guides by their origin's
    /// flag, so the button shows pressed, and it is inert (a clone carries no flag of its own).
    PressedInert,
}

impl Session {
    /// The Guide button's look for the current selection.
    #[must_use]
    pub fn guide_button_state(&self) -> GuideButtonState {
        let marking = self.guide_marking();
        if marking.all_guides() {
            return GuideButtonState::Pressed;
        }
        if marking.enabled() {
            return GuideButtonState::Off;
        }
        let ids = self.selection.ids();
        if self.tool != Tool::Select || ids.is_empty() {
            return GuideButtonState::Disabled;
        }
        let read = self.document_read();
        let clone_guides = ids.iter().all(|id| {
            read.guides.contains(id)
                && read.leaves.iter().any(|o| {
                    o.id() == *id && matches!(o, curvyo_document_core::ObjectSnapshot::Clone(_))
                })
        });
        if clone_guides {
            GuideButtonState::PressedInert
        } else {
            GuideButtonState::Disabled
        }
    }

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
        let read = self.document_read();
        let (dimensions, others): (Vec<_>, Vec<_>) = ids.iter().partition(|id| {
            read.leaves.iter().any(|o| {
                o.id() == **id && matches!(o, curvyo_document_core::ObjectSnapshot::Dimension(_))
            })
        });
        // Guide off on dimensions converts them (`specs/0070-measure-and-dimension` criterion 10).
        if marking.all_guides() && !dimensions.is_empty() {
            return self.convert_selected_dimensions(&others);
        }
        self.recording(|this| this.document.set_guide(&ids, !marking.all_guides()).is_ok())
    }

    /// The Template button: adds the outlines of the Curvyo document `bytes` as guides in one step
    /// "Template" and selects them. Returns how many came in; the host then shows them with
    /// [`Session::show_page_and_guides`] once its bars are laid out for the new selection.
    ///
    /// # Errors
    /// A `TemplateError` when the file is too large or cannot be opened, or the copy is refused;
    /// nothing changes.
    pub fn load_template(&mut self, bytes: &[u8]) -> Result<usize, TemplateError> {
        if bytes.len() > MAX_TEMPLATE_BYTES {
            return Err(TemplateError::TooLarge);
        }
        let source =
            curvyo_document_core::unpack(SOURCE_PEER, bytes).map_err(TemplateError::from)?;
        let mut nodes = source.template_nodes();
        let page_origin = self.active_page().origin;
        for node in &mut nodes {
            node.object = node.object.translated(curvyo_document_core::Vec2::new(
                page_origin.x,
                page_origin.y,
            ));
        }
        drop(source);
        if nodes.is_empty() {
            return Ok(0);
        }
        let count: usize = nodes
            .iter()
            .map(curvyo_document_core::TemplateNode::anchor_count)
            .sum();
        let anchor_ids: Vec<_> = (0..count).map(|_| self.minter.mint()).collect();
        self.select.cancel_entry();
        let ids = self
            .recording(|this| {
                let loaded = this.document.load_template(&nodes, &anchor_ids);
                if let Ok(ids) = &loaded {
                    this.selection.set(ids);
                }
                loaded
            })
            .map_err(|_| TemplateError::Refused)?;
        Ok(ids.len())
    }

    /// The first view after a template load (criterion 13): zooms out to the page and every
    /// guide when part of them is outside the canvas area the bars leave free, right of
    /// `free_left` and below `free_top` (CSS pixels from the canvas's top-left corner).
    pub fn show_page_and_guides(&mut self, free_left: f64, free_top: f64) {
        let pages = self.document.pages();
        let mut min = Point::new(f64::INFINITY, f64::INFINITY);
        let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for page in pages {
            min = Point::new(min.x.min(page.origin.x), min.y.min(page.origin.y));
            max = Point::new(
                max.x.max(page.origin.x + page.size.width.as_mm()),
                max.y.max(page.origin.y + page.size.height.as_mm()),
            );
        }
        let read = self.document_read();
        for object in read.leaves.iter().filter(|o| read.guides.contains(&o.id())) {
            let (low, high) = curvyo_ui_core::object_outline_bounds(object);
            min = Point::new(min.x.min(low.x), min.y.min(low.y));
            max = Point::new(max.x.max(high.x), max.y.max(high.y));
        }
        self.viewport.show_area(min, max, free_left, free_top);
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
    fn only_clones_of_guides_show_the_button_pressed_and_inert() {
        let mut session = Session::new(1);
        let origin = session.document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 10.0)),
            ],
            false,
        );
        let clone = session.document.linked_clone(&[origin], &[]).unwrap()[0];
        session.set_tool(Tool::Select);
        session.selection.set(&[clone]);
        assert_eq!(session.guide_button_state(), GuideButtonState::Disabled);
        session.document.set_guide(&[origin], true).unwrap();
        assert_eq!(session.guide_button_state(), GuideButtonState::PressedInert);
        assert!(!session.toggle_guide(), "inert: nothing is written");
        session.selection.set(&[clone, origin]);
        assert_eq!(session.guide_button_state(), GuideButtonState::Pressed);
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
        // The view after the bars are laid out: rail and a three-row Select bar (12 + 108 px).
        session.show_page_and_guides(128.0, 120.0);
        assert!(session.zoom_percent() < 100, "the overhang is shown");
        let view = session.view();
        let (x, y) = view.document_to_screen(Point::new(-300.0, -300.0));
        assert!(
            x >= 128.0 - 1e-6 && y >= 120.0 - 1e-6,
            "not under the bars: {x} {y}"
        );
        let (x, y) = view.document_to_screen(Point::new(900.0, 900.0));
        assert!(x <= 1000.0 && y <= 800.0, "inside the canvas: {x} {y}");
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
