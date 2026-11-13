//! `Session`'s glue for Export SVG… (`specs/0024-svg-import-export` criteria 4, 130, 131, 147
//! and 148): the session's laid-out objects, selection and active page handed to the mapping of
//! `curvyo_ui_core`, the text written by `curvyo_svg_core`, whole, in memory. Nothing in the
//! document changes.

use curvyo_document_core::{FontKind, ObjectRead};
use curvyo_ui_core::{ExportScope, ImportFaces, SvgExportOptions, svg_export_of_with_artwork};

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
    /// An editable curve or width profile cannot be exported accurately.
    UnsupportedExtendedPath,
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
        let laid =
            ObjectRead::from_objects(self.spiro_selection_objects(&read.clones.objects).into());
        let substitute = self.fonts.fallback(FontKind::Outline).map(|(key, _)| key);
        let fonts = ImportFaces {
            faces: self.fonts.faces(),
            store: self.fonts.store(),
            substitute,
        };
        let Ok(outcome) = svg_export_of_with_artwork(
            &self.document,
            &read.nodes,
            &laid,
            &fonts,
            &ExportScope {
                selection: self.selection.ids(),
                active_page,
            },
            options,
            &|path| self.width_artwork(path),
        ) else {
            return SvgExportResult::UnsupportedExtendedPath;
        };
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
    use curvyo_document_core::{AnchorId, CurveKind, NewAnchor, Point};

    fn spiro(session: &Session) -> curvyo_document_core::NodeId {
        let mut points = [
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(1, 3), Point::new(20.0, 0.0)),
        ];
        points[1].kind = curvyo_document_core::AnchorKind::Symmetric;
        session
            .document
            .create_editable_curve(&points, false, CurveKind::Spiro)
            .unwrap()
    }

    fn complete(session: &Session) {
        let _ = session.draw_list();
    }

    #[test]
    fn spiro_export_applies_placement_to_cached_curve() {
        let session = Session::new(1);
        let id = spiro(&session);
        session
            .document
            .set_spiro_transform(
                id,
                curvyo_document_core::Affine::new(2.0, 0.0, 0.0, 3.0, 40.0, 50.0),
            )
            .unwrap();
        complete(&session);
        let SvgExportResult::Written { text, .. } = session.export_svg(SvgExportOptions::default())
        else {
            panic!("export failed")
        };
        assert!(text.contains("M 40 50"), "{text}");
    }

    #[test]
    fn spiro_export_writes_its_evaluated_curve() {
        let session = Session::new(1);
        let id = spiro(&session);
        assert!(matches!(
            session.export_svg(curvyo_ui_core::SvgExportOptions::default()),
            SvgExportResult::Written { objects: 1, .. }
        ));
        let anchor = session.document.path(id).unwrap().anchors[1].id;
        session
            .document
            .move_anchors(&[(id, anchor, Point::new(11.0, 12.0))])
            .unwrap();
        // A control edit is exported at once, as its new curve.
        let SvgExportResult::Written { text, .. } =
            session.export_svg(curvyo_ui_core::SvgExportOptions::default())
        else {
            panic!("export after an edit")
        };
        assert!(text.contains(" C "), "{text}");
    }

    #[test]
    fn spiro_export_scopes_selection_and_exports_clones() {
        let mut session = Session::new(1);
        let id = spiro(&session);
        let clone = session.document.linked_clone(&[id], &[]).unwrap()[0];
        let curvyo_document_core::ObjectSnapshot::Clone(snapshot) =
            session.document.object(clone).unwrap()
        else {
            panic!("clone")
        };
        let mapped = snapshot.mapped_by(curvyo_document_core::Affine::new(
            2.0, 0.0, 0.0, 2.0, 50.0, 0.0,
        ));
        session
            .document
            .transform_objects(
                &[curvyo_document_core::ObjectSnapshot::Clone(mapped)],
                false,
            )
            .unwrap();
        let other = session.document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(2, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(2, 2), Point::new(5.0, 0.0)),
            ],
            false,
        );
        let options = SvgExportOptions {
            selection_only: true,
            ..Default::default()
        };
        session.selection.set(&[other]);
        assert!(matches!(
            session.export_svg(options),
            SvgExportResult::Written { objects: 1, .. }
        ));
        session.selection.set(&[clone]);
        let SvgExportResult::Written { objects, text, .. } = session.export_svg(options) else {
            panic!("clone export")
        };
        assert_eq!(objects, 1);
        assert!(text.contains("50"), "clone transform missing: {text}");
        let converted = SvgExportOptions {
            strokes_as_paths: true,
            ..options
        };
        let SvgExportResult::Written { text, objects, .. } = session.export_svg(converted) else {
            panic!("stroke conversion")
        };
        assert_eq!(objects, 1);
        assert!(
            !text.contains("stroke-width"),
            "stroke remained a stroke: {text}"
        );
        assert!(text.contains("50"), "clone transform missing: {text}");
        assert!(curvyo_ui_core::read_svg_drawing(text.as_bytes()).is_ok());
    }

    /// Every Spiro of finite controls has a curve since `specs/0115`, even one whose controls
    /// lie on one spot (it was refused as unsolvable before): it exports beside an ordinary path.
    #[test]
    fn a_spiro_on_one_spot_exports_like_any_path() {
        let mut session = Session::new(1);
        let back_and_forth = [(5.0, 5.0), (5.0, 5.0), (5.0, 5.0)].map(|(x, y)| {
            let mut anchor = NewAnchor::corner(AnchorId::new(1, 10), Point::new(x, y));
            anchor.kind = curvyo_document_core::AnchorKind::Symmetric;
            anchor
        });
        let mut back_and_forth = back_and_forth;
        for (anchor, k) in back_and_forth.iter_mut().zip(10..) {
            anchor.id = AnchorId::new(1, k);
        }
        let id = session
            .document
            .create_editable_curve(&back_and_forth, false, CurveKind::Spiro)
            .unwrap();
        let ordinary = session.document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(2, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(2, 2), Point::new(5.0, 0.0)),
            ],
            false,
        );
        let _ = session.draw_list();
        let options = SvgExportOptions {
            selection_only: true,
            strokes_as_paths: true,
            ..Default::default()
        };
        session.selection.set(&[id]);
        assert!(matches!(
            session.export_svg(options),
            SvgExportResult::Written { objects: 1, .. }
        ));
        session.selection.set(&[ordinary]);
        assert!(matches!(
            session.export_svg(options),
            SvgExportResult::Written { objects: 1, .. }
        ));
    }

    #[test]
    fn active_page_export_reads_a_spiro_by_its_curve() {
        let mut session = Session::new(1);
        let page = session.document.add_page();
        assert!(session.activate_page(&page.id));
        let points = [
            NewAnchor::corner(AnchorId::new(4, 1), Point::new(800.0, 20.0)),
            NewAnchor::corner(AnchorId::new(4, 2), Point::new(810.0, 30.0)),
            NewAnchor::corner(AnchorId::new(4, 3), Point::new(820.0, 20.0)),
        ];
        session
            .document
            .create_editable_curve(&points, false, CurveKind::Spiro)
            .unwrap();
        let options = SvgExportOptions {
            active_page_only: true,
            ..Default::default()
        };
        assert!(matches!(
            session.export_svg(options),
            SvgExportResult::Written { objects: 0, .. }
        ));
    }

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
