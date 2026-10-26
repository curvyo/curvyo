//! `Session`'s glue for SVG files (`specs/0024-svg-import-export` criteria 1 to 3, 6 and 8): Open
//! decides by content and makes an untitled, dirty project from an SVG; Import SVG… adds a file
//! as one new layer in one step "Import SVG", centred on the view and selected. The reading and
//! the mapping are `curvyo_ui_core`'s, the write `curvyo_document_core`'s.

use curvyo_document_core::{Document, DocumentVersion, DrawingNode, Point};
use curvyo_ui_core::{
    FileKind, NEITHER_PROJECT_NOR_SVG, SvgPlacement, file_kind, import_layer_name,
    import_svg_bytes, nothing_imported_notice,
};

use super::layers::ActiveLayer;
use super::{Session, Tool};

/// What Import SVG… did, when it changed nothing it is the notice to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvgImportOutcome {
    /// This many objects came in, selected.
    Imported(usize),
    /// Nothing could be imported: the notice of criterion 8; nothing changed.
    Nothing(String),
}

impl Session {
    /// Open… for any file (criterion 1): an SVG becomes an untitled, dirty project with an empty
    /// history ([`Session::opened_svg`]); a project opens as before. For an SVG that brought in
    /// nothing, [`Session::take_open_notice`] gives the notice of criterion 8.
    ///
    /// # Errors
    /// The error dialog's sentence (criterion 6): nothing changed.
    pub fn open_file(peer: u64, bytes: &[u8], file_name: &str) -> Result<Self, String> {
        match file_kind(bytes) {
            FileKind::Svg => Self::open_svg(peer, bytes, file_name),
            FileKind::Project => {
                Self::open(peer, bytes).map_err(|error| super::map_open_error(&error).to_owned())
            }
            FileKind::Other => Err(NEITHER_PROJECT_NOR_SVG.to_owned()),
        }
    }

    /// Whether this project was opened from an SVG and exists in no saved file yet: Save then
    /// asks for a file (criterion 2).
    #[must_use]
    pub fn opened_svg(&self) -> bool {
        self.saved == DocumentVersion::never()
    }

    /// The notice of an Open of an SVG that brought in nothing, once; empty otherwise.
    pub fn take_open_notice(&mut self) -> String {
        self.open_notice.take().unwrap_or_default()
    }

    /// An SVG as a new project at the file's own coordinates (criteria 2, 15 to 20).
    fn open_svg(peer: u64, bytes: &[u8], file_name: &str) -> Result<Self, String> {
        let import = import_svg_bytes(bytes, SvgPlacement::File)?;
        let mut session = Self::new(peer);
        let ids: Vec<_> = (0..DrawingNode::anchor_count(&import.nodes))
            .map(|_| session.minter.mint())
            .collect();
        session.document = Document::from_drawing(peer, &import.page, &import.nodes, &ids)
            .map_err(|_| NEITHER_PROJECT_NOR_SVG.to_owned())?;
        // The content exists in no saved file (criterion 2).
        session.saved = DocumentVersion::never();
        session.open_notice = (import.leaves == 0).then(|| nothing_imported_notice(file_name));
        Ok(session)
    }

    /// Import SVG… (criterion 3): the file's content as one new top layer named after the file,
    /// moved as one so its bounding box is centred on the visible canvas (the page's centre
    /// without a canvas), in one step "Import SVG"; the layer becomes the active layer and its
    /// top-level children are selected.
    ///
    /// # Errors
    /// The error dialog's sentence (criterion 6): nothing changed.
    pub fn import_svg(
        &mut self,
        bytes: &[u8],
        file_name: &str,
    ) -> Result<SvgImportOutcome, String> {
        let target = self.import_centre();
        let import = import_svg_bytes(bytes, SvgPlacement::CentredOn(target))?;
        if import.leaves == 0 {
            return Ok(SvgImportOutcome::Nothing(nothing_imported_notice(
                file_name,
            )));
        }
        let ids: Vec<_> = (0..DrawingNode::anchor_count(&import.nodes))
            .map(|_| self.minter.mint())
            .collect();
        if self.tool != Tool::Select {
            self.set_tool(Tool::Select);
        }
        self.select.cancel_entry();
        let name = import_layer_name(file_name);
        let (layer, children) = self
            .recording(|this| {
                let inserted = this.document.insert_drawing(&name, &import.nodes, &ids);
                if let Ok((_, children)) = &inserted {
                    this.selection.set(children);
                }
                inserted
            })
            .map_err(|_| NEITHER_PROJECT_NOR_SVG.to_owned())?;
        self.layers.active = ActiveLayer::Chosen(Some(layer));
        self.drop_inert_selection();
        let _ = children;
        Ok(SvgImportOutcome::Imported(import.leaves))
    }

    /// The centre of the visible canvas in document millimetres, or the page's centre when the
    /// canvas has no size yet.
    fn import_centre(&self) -> Point {
        let (width, height) = self.viewport.canvas_size();
        if width > 0.0 && height > 0.0 {
            return self.viewport_centre();
        }
        let size = self.document.size();
        Point::new(size.width.as_mm() / 2.0, size.height.as_mm() / 2.0)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="400mm" height="400mm" viewBox="0 0 400 400"><rect x="200" y="200" width="40" height="20"/><circle cx="300" cy="210" r="10"/></svg>"#;

    #[test]
    fn ac2_an_svg_opens_untitled_dirty_and_without_history() {
        let mut session = Session::open_file(3, SVG, "poster.svg").unwrap();
        assert_eq!(session.take_open_notice(), "");
        assert!(session.is_dirty() && session.opened_svg());
        assert_eq!(session.document.own_step_count(), 0);
        assert_eq!(session.document.size().width.as_mm(), 400.0);
        assert_eq!(session.document.read_objects().objects.len(), 2);
        let mut empty =
            Session::open_file(3, b"<svg xmlns='http://www.w3.org/2000/svg'/>", "empty.svg")
                .unwrap();
        assert_eq!(
            empty.take_open_notice(),
            "Nothing in empty.svg could be imported."
        );
        assert_eq!(
            Session::open_file(3, b"hello", "x.txt").err().as_deref(),
            Some(NEITHER_PROJECT_NOR_SVG)
        );
        assert_eq!(
            Session::open_file(3, &[0x1F, 0x8B, 0], "x.svgz")
                .err()
                .as_deref(),
            Some(
                "Compressed SVG (.svgz) is not supported. Save the file as plain SVG. Nothing was changed."
            )
        );
    }

    #[test]
    fn ac3_import_is_one_step_centred_and_selected() {
        let mut session = Session::new(1);
        session.resize_viewport(800.0, 600.0);
        let centre = session.import_centre();
        let outcome = session.import_svg(SVG, "poster.svg").unwrap();
        assert_eq!(outcome, SvgImportOutcome::Imported(2));
        let layer = session.active_layer().unwrap();
        assert_eq!(
            session
                .document
                .node_registers(layer)
                .unwrap()
                .name
                .as_deref(),
            Some("poster")
        );
        assert_eq!(session.selection.ids().len(), 2);
        let read = session.document_read();
        let (low, high) = read
            .leaves
            .iter()
            .map(curvyo_ui_core::object_outline_bounds)
            .fold(
                (
                    Point::new(f64::MAX, f64::MAX),
                    Point::new(f64::MIN, f64::MIN),
                ),
                |(a, b), (l, h)| {
                    (
                        Point::new(a.x.min(l.x), a.y.min(l.y)),
                        Point::new(b.x.max(h.x), b.y.max(h.y)),
                    )
                },
            );
        assert!((f64::midpoint(low.x, high.x) - centre.x).abs() < 1e-6);
        assert!((f64::midpoint(low.y, high.y) - centre.y).abs() < 1e-6);
        assert_eq!(session.document.own_step_count(), 1);
        let _ = session.undo();
        assert!(!session.document.is_layer(layer));
        let outcome = session.import_svg(
            b"<svg xmlns='http://www.w3.org/2000/svg'><text>x</text></svg>",
            "poster.svg",
        );
        assert_eq!(
            outcome,
            Ok(SvgImportOutcome::Nothing(
                "Nothing in poster.svg could be imported.".into()
            ))
        );
    }
}
