//! `Session`'s glue for Export SVG… (`specs/0024-svg-import-export` criteria 4, 130, 131, 147
//! and 148): the session's laid-out objects, selection and active page handed to the mapping of
//! `curvyo_ui_core`, the text written by `curvyo_svg_core`, whole, in memory. Nothing in the
//! document changes.

use curvyo_document_core::FontKind;
use curvyo_ui_core::{ExportScope, ImportFaces, SvgExportOptions, svg_export_of};

use super::Session;

/// What one export gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvgExportResult {
    /// The SVG text, how many objects it holds (0: the drawing is empty) and how many texts were
    /// kept as text because their face is missing.
    Written {
        /// The file's text.
        text: String,
        /// The objects written.
        objects: usize,
        /// Texts kept live although outlines were asked for (criterion 148).
        texts_kept_live: usize,
    },
    /// Selection only with nothing selected: no file (criterion 131).
    NothingSelected,
}

impl Session {
    /// The SVG of this document under `options` (criterion 4): built whole before anything is
    /// written, so a failure leaves no partial file.
    #[must_use]
    pub fn export_svg(&self, options: SvgExportOptions) -> SvgExportResult {
        if options.selection_only && self.selection.is_empty() {
            return SvgExportResult::NothingSelected;
        }
        let pages = self.document.pages();
        let active = self.active_page();
        let active_page = pages.iter().position(|p| p.id == active.id).unwrap_or(0);
        let read = self.document_read();
        let substitute = self.fonts.fallback(FontKind::Outline).map(|(key, _)| key);
        let fonts = ImportFaces {
            faces: self.fonts.faces(),
            store: self.fonts.store(),
            substitute,
        };
        let outcome = svg_export_of(
            &self.document,
            &read.nodes,
            &read.clones,
            &fonts,
            &ExportScope {
                selection: self.selection.ids(),
                active_page,
            },
            options,
        );
        SvgExportResult::Written {
            text: outcome.text(),
            objects: outcome.objects,
            texts_kept_live: outcome.texts_kept_live,
        }
    }

    /// How many objects the document has, for the busy state from 5,000 (`ux-notes.md`).
    #[must_use]
    pub fn export_object_count(&self) -> usize {
        self.object_read().objects.len()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn ac4_an_empty_document_exports_a_valid_svg_and_stays_clean() {
        let session = Session::new(1);
        let version = session.document.version();
        let SvgExportResult::Written { text, objects, .. } =
            session.export_svg(SvgExportOptions::default())
        else {
            panic!("written")
        };
        assert_eq!(objects, 0);
        assert!(curvyo_ui_core::read_svg_drawing(text.as_bytes()).is_ok());
        assert_eq!(session.document.version(), version);
    }

    #[test]
    fn ac131_selection_only_with_nothing_selected_writes_nothing() {
        let session = Session::new(1);
        let options = SvgExportOptions {
            selection_only: true,
            ..SvgExportOptions::default()
        };
        assert_eq!(
            session.export_svg(options),
            SvgExportResult::NothingSelected
        );
    }
}
