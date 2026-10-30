//! `Session`'s glue for SVG files (`specs/0024-svg-import-export` criteria 1 to 3, 6, 8 and
//! 106): Open decides by content and makes an untitled, dirty project from an SVG; Import SVG…
//! adds a file as one new layer in one step "Import SVG", centred on the view and selected. A
//! file with texts waits for the faces they need (`adrs.md` note 2026-10-27, point 3): the session
//! asks for them as for any text and the host calls again once they are loaded. The reading and
//! the mapping are `curvyo_ui_core`'s, the write `curvyo_document_core`'s.

use curvyo_document_core::{
    Document, DocumentVersion, DrawingNode, FontKind, MAX_PROJECT_ASSET_BYTES, Point,
};
use curvyo_ui_core::SvgDrawing;
use curvyo_ui_core::{
    FileKind, ImportFaces, NEITHER_PROJECT_NOR_SVG, SvgImport, SvgPlacement, file_kind,
    import_layer_name, nothing_imported_notice, read_svg_drawing, svg_faces_needed, svg_import,
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
    /// The faces of its texts are being loaded: call again once they are (nothing changed).
    Waiting,
}

/// An SVG opened as a project whose texts wait for their faces.
#[derive(Debug)]
pub(super) struct PendingOpen {
    drawing: SvgDrawing,
    file_name: String,
    peer: u64,
}

impl Session {
    /// Open… for any file (criterion 1): an SVG becomes an untitled, dirty project with an empty
    /// history ([`Session::opened_svg`]); a project opens as before. For an SVG that brought in
    /// nothing, [`Session::take_open_notice`] gives the notice of criterion 8. An SVG with texts
    /// opens on its page and waits for the faces: [`Session::continue_open_svg`].
    ///
    /// # Errors
    /// The error dialog's sentence (criterion 6): nothing changed.
    pub fn open_file(peer: u64, bytes: &[u8], file_name: &str) -> Result<Self, String> {
        match file_kind(bytes) {
            FileKind::Svg => Self::open_svg(peer, bytes, file_name),
            FileKind::Project => {
                Self::open(peer, bytes).map_err(|error| super::map_open_error(&error).to_owned())
            }
            FileKind::Image | FileKind::Other => Err(NEITHER_PROJECT_NOR_SVG.to_owned()),
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
        let drawing = read_svg_drawing(bytes)?;
        let mut session = Self::new(peer);
        let page = curvyo_ui_core::svg_page(&drawing);
        session.document = Document::from_drawing(peer, &page, &[], &[], &[])
            .map_err(|_| NEITHER_PROJECT_NOR_SVG.to_owned())?;
        session.saved = DocumentVersion::never();
        session.pending_open = Some(PendingOpen {
            drawing,
            file_name: file_name.to_owned(),
            peer,
        });
        session.continue_open_svg(false);
        Ok(session)
    }

    /// Whether an opened SVG still waits for the faces of its texts.
    #[must_use]
    pub fn open_svg_waiting(&self) -> bool {
        self.pending_open.is_some()
    }

    /// Converts an opened SVG once the faces of its texts are loaded (or with what is loaded
    /// when `force`d: a text without any face is left out). Returns whether it still waits.
    pub fn continue_open_svg(&mut self, force: bool) -> bool {
        let Some(pending) = self.pending_open.take() else {
            return false;
        };
        if !force && !self.svg_faces_ready(&pending.drawing) {
            self.pending_open = Some(pending);
            return true;
        }
        let import = self.convert_svg(&pending.drawing, SvgPlacement::File);
        let ids: Vec<_> = (0..DrawingNode::anchor_count(&import.nodes))
            .map(|_| self.minter.mint())
            .collect();
        if let Ok(document) = Document::from_drawing(
            pending.peer,
            &import.page,
            &import.nodes,
            &ids,
            &import.assets,
        ) {
            self.document = document;
            self.cache_pictures(&import);
        }
        // The content exists in no saved file (criterion 2).
        self.saved = DocumentVersion::never();
        self.open_notice =
            (import.leaves == 0).then(|| nothing_imported_notice(&pending.file_name));
        false
    }

    /// Phase one for `drawing`: asks the host for the faces its texts need that are not loaded
    /// (as for any text) and says whether all are there.
    pub(super) fn svg_faces_ready(&self, drawing: &SvgDrawing) -> bool {
        let needs = svg_faces_needed(drawing, self.fonts.faces());
        for &index in &needs.load {
            self.fonts.request(index);
        }
        let substitute = needs.substitute && self.fonts.fallback(FontKind::Outline).is_some();
        needs.ready(substitute)
    }

    /// Phase two: `drawing` converted with the loaded faces, its images within the project's
    /// room for pictures.
    pub(super) fn convert_svg(&self, drawing: &SvgDrawing, placement: SvgPlacement) -> SvgImport {
        let substitute = self.fonts.fallback(FontKind::Outline).map(|(key, _)| key);
        let room = MAX_PROJECT_ASSET_BYTES.saturating_sub(self.document.asset_bytes());
        svg_import(
            drawing,
            placement,
            &ImportFaces {
                faces: self.fonts.faces(),
                store: self.fonts.store(),
                substitute,
            },
            room,
        )
    }

    /// Hands the decoded pictures of an import to the canvas.
    pub(super) fn cache_pictures(&mut self, import: &SvgImport) {
        for (hash, working) in &import.working {
            self.images.insert(hash.clone(), working.clone());
        }
    }

    /// Import SVG… (criterion 3): the file's content as one new top layer named after the file,
    /// moved as one so its bounding box is centred on the visible canvas (the page's centre
    /// without a canvas), in one step "Import SVG"; the layer becomes the active layer and its
    /// top-level children are selected. `SvgImportOutcome::Waiting` while the faces of its
    /// texts load: call again.
    ///
    /// # Errors
    /// The error dialog's sentence (criterion 6): nothing changed.
    pub fn import_svg(
        &mut self,
        bytes: &[u8],
        file_name: &str,
    ) -> Result<SvgImportOutcome, String> {
        let drawing = read_svg_drawing(bytes)?;
        if !self.svg_faces_ready(&drawing) {
            return Ok(SvgImportOutcome::Waiting);
        }
        let target = self.import_centre();
        let import = self.convert_svg(&drawing, SvgPlacement::CentredOn(target));
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
        self.cache_pictures(&import);
        let (layer, _) = self
            .recording(|this| {
                let inserted =
                    this.document
                        .insert_drawing(&name, &import.nodes, &ids, &import.assets);
                if let Ok((_, children)) = &inserted {
                    this.selection.set(children);
                }
                inserted
            })
            .map_err(|_| NEITHER_PROJECT_NOR_SVG.to_owned())?;
        self.layers.active = ActiveLayer::Chosen(Some(layer));
        self.drop_inert_selection();
        Ok(SvgImportOutcome::Imported(import.leaves))
    }

    /// The centre of the visible canvas in document millimetres, or the page's centre when the
    /// canvas has no size yet.
    fn import_centre(&self) -> Point {
        let (width, height) = self.viewport.canvas_size();
        if width > 0.0 && height > 0.0 {
            return self.viewport_centre();
        }
        let page = self.active_page();
        Point::new(
            page.origin.x + page.size.width.as_mm() / 2.0,
            page.origin.y + page.size.height.as_mm() / 2.0,
        )
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
            b"<svg xmlns='http://www.w3.org/2000/svg'><title>x</title></svg>",
            "poster.svg",
        );
        assert_eq!(
            outcome,
            Ok(SvgImportOutcome::Nothing(
                "Nothing in poster.svg could be imported.".into()
            ))
        );
    }

    const TEXT_AND_IMAGE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><text x="10" y="30" style="font-family:'Source Sans 3';font-weight:bold;font-size:10px">Boho</text><text x="10" y="60" style="font-family:Lobster">Gone</text><image x="50" y="50" width="20" height="20" href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="/></svg>"#;

    /// Slice 2, criterion 106: Open waits for the faces of its texts, then the texts, the missing
    /// face and the image come in without a step.
    #[test]
    fn ac106_open_waits_for_the_faces() {
        let mut session = Session::open_file(5, TEXT_AND_IMAGE, "boho.svg").unwrap();
        assert!(session.open_svg_waiting(), "no face is listed yet");
        assert_eq!(session.document.read_objects().objects.len(), 0);
        let listed = super::super::text_tests::session();
        let lists: Vec<_> = listed
            .fonts
            .faces()
            .iter()
            .map(|f| {
                (
                    f.host_id.clone(),
                    f.family.clone(),
                    f.style.clone(),
                    f.index,
                )
            })
            .collect();
        session.set_font_list(curvyo_ui_core::FontBlock::Included, lists, Vec::new());
        assert!(session.continue_open_svg(false), "the faces are asked for");
        super::super::text_tests::load_wanted(&mut session);
        assert!(!session.continue_open_svg(false));
        assert_eq!(session.document.own_step_count(), 0);
        let texts = super::super::text_tests::texts(&session);
        assert_eq!(texts.len(), 2);
        assert_eq!(texts[0].font.style, "Bold");
        assert_eq!(
            texts[1].font.face_hash,
            curvyo_document_core::MISSING_FACE_HASH
        );
        assert_eq!(session.document.asset_bytes(), 70);
        assert!(session.opened_svg());
    }

    /// Slice 2, criteria 104, 106: Import SVG of texts and an image is one step; Undo removes it.
    #[test]
    fn ac104_import_is_one_step_with_texts_and_images() {
        let mut session = super::super::text_tests::session();
        let outcome = session.import_svg(TEXT_AND_IMAGE, "boho.svg").unwrap();
        assert_eq!(outcome, SvgImportOutcome::Waiting, "Bold is not loaded yet");
        assert_eq!(session.document.own_step_count(), 0);
        super::super::text_tests::load_wanted(&mut session);
        let outcome = session.import_svg(TEXT_AND_IMAGE, "boho.svg").unwrap();
        assert_eq!(outcome, SvgImportOutcome::Imported(3));
        assert_eq!(session.document.own_step_count(), 1);
        assert_eq!(session.selection.ids().len(), 3);
        let _ = session.undo();
        assert_eq!(session.document.read_objects().objects.len(), 0);
    }
}
