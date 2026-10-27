//! The Import button's session side (`specs/0058-bitmap-import` criteria 1 to 5, 13 to 15, 26;
//! `specs/0024-svg-import-export` UX notes, "Import button in the tool rail"): files are added
//! one by one (each read and decoded at once, so the host can show progress between them), each
//! routed by its content to the SVG import or the image import, and the batch is committed as
//! one step and selected.

use std::sync::Arc;

use curvyo_document_core::{
    AssetError, DrawingNode, IMPORT_COMMIT_LABEL, IMPORT_IMAGE_COMMIT_LABEL,
    IMPORT_IMAGES_COMMIT_LABEL, IMPORT_SVG_COMMIT_LABEL, ImageInsertError, NodeId, Point,
};
use curvyo_ui_core::SvgDrawing;
use curvyo_ui_core::{
    FileKind, ImageTarget, PlannedImage, SvgImport, SvgPlacement, UNSUPPORTED_FILE_NOTICE,
    batch_failures_notice, failure_notice, file_kind, import_layer_name, plan_image_import,
    project_full_reason, read_svg_drawing, skipped_notice,
};

use super::layers::ActiveLayer;
use super::{Session, Tool};

/// The most files one import takes (criterion 5).
pub const MAX_IMPORT_FILES: usize = 20;

/// One file read for the import.
enum Planned {
    Image(PlannedImage),
    Svg {
        layer_name: String,
        /// The drawing read, converted once the faces of its texts are loaded.
        drawing: Box<SvgDrawing>,
        /// The point its box is centred on.
        centre: Point,
        /// The content, once converted.
        import: Option<SvgImport>,
    },
}

/// The files of an import being added.
#[derive(Default)]
pub(super) struct ImportBatch {
    target: Option<ImageTarget>,
    planned: Vec<(String, Planned)>,
    /// The file name and reason of each file that cannot be imported, in order.
    failed: Vec<(String, String)>,
    /// How many files were offered, the skipped ones included.
    chosen: usize,
}

/// What an import did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportOutcome {
    /// How many objects came in, selected.
    pub imported: usize,
    /// The notice, lines separated by `\n`; empty for none.
    pub notice: String,
    /// Whether the notice reports a failure (an alert, 5 s).
    pub alert: bool,
    /// The faces of an SVG's texts are loading: nothing changed, call again
    /// (`specs/0024-svg-import-export` criterion 106).
    pub waiting: bool,
}

impl Session {
    /// Starts an import aimed at `drop` (a point on the canvas, in screen pixels), or at the view
    /// centre without one (criteria 4 and 12). Discards an unfinished one.
    pub fn import_begin(&mut self, drop: Option<(f64, f64)>) {
        let target = match drop {
            Some((x, y)) => ImageTarget::Drop(self.screen_to_document(x, y)),
            None => ImageTarget::ViewCentre(self.import_view_centre()),
        };
        self.import = Some(ImportBatch {
            target: Some(target),
            ..ImportBatch::default()
        });
    }

    /// Reads one file of the import begun by [`Session::import_begin`]: an image is decoded, an
    /// SVG parsed, anything else remembered as refused. Files past the 20th are skipped.
    pub fn import_add(&mut self, bytes: &[u8], file_name: &str) {
        let page = self.document.size();
        let Some(batch) = self.import.as_mut() else {
            return;
        };
        batch.chosen += 1;
        if batch.planned.len() + batch.failed.len() >= MAX_IMPORT_FILES {
            return;
        }
        let target = batch
            .target
            .unwrap_or(ImageTarget::ViewCentre(Point::new(0.0, 0.0)));
        let index = batch.planned.len();
        let planned = match file_kind(bytes) {
            FileKind::Image => plan_image_import(Arc::from(bytes), file_name, page, target, index)
                .map(Planned::Image),
            FileKind::Svg => {
                let (ImageTarget::ViewCentre(centre) | ImageTarget::Drop(centre)) = target;
                #[allow(clippy::cast_precision_loss)]
                let step = curvyo_ui_core::STAGGER_MM * index as f64;
                read_svg_drawing(bytes).map(|drawing| Planned::Svg {
                    layer_name: import_layer_name(file_name),
                    drawing: Box::new(drawing),
                    centre: Point::new(centre.x + step, centre.y + step),
                    import: None,
                })
            }
            FileKind::Project | FileKind::Other => Err(UNSUPPORTED_FILE_NOTICE.to_owned()),
        };
        match planned {
            Ok(planned) => batch.planned.push((file_name.to_owned(), planned)),
            Err(reason) => batch.failed.push((file_name.to_owned(), reason)),
        }
    }

    /// Commits the import begun by [`Session::import_begin`] as one step ("Import image",
    /// "Import images", "Import SVG" or, mixed, "Import"), selects everything new with the Select
    /// tool, and returns its notice. Nothing is written when nothing could be read. While the
    /// faces of an SVG's texts load, it changes nothing and says it waits (unless `force`d, when
    /// a text without any face is left out); the host calls again.
    pub fn import_finish(&mut self, force: bool) -> ImportOutcome {
        let Some(mut batch) = self.import.take() else {
            return ImportOutcome {
                imported: 0,
                notice: String::new(),
                alert: false,
                waiting: false,
            };
        };
        let ready = batch.planned.iter().all(|(_, planned)| match planned {
            Planned::Svg { drawing, .. } => self.svg_faces_ready(drawing),
            Planned::Image(_) => true,
        });
        if !ready && !force {
            self.import = Some(batch);
            return ImportOutcome {
                imported: 0,
                notice: String::new(),
                alert: false,
                waiting: true,
            };
        }
        self.convert_planned(&mut batch);
        let images: Vec<_> = batch
            .planned
            .iter()
            .filter_map(|(_, planned)| match planned {
                Planned::Image(image) => Some(image.image.clone()),
                Planned::Svg { .. } => None,
            })
            .collect();
        if let Err(ImageInsertError::Assets(AssetError::ProjectFull { adds })) =
            self.document.check_images(&images)
        {
            let reason = project_full_reason(adds);
            batch.planned.retain(|(name, planned)| match planned {
                Planned::Image(_) => {
                    batch.failed.push((name.clone(), reason.clone()));
                    false
                }
                Planned::Svg { .. } => true,
            });
        }
        let ids = self.commit_import(&batch.planned);
        let notice = batch_notice(&batch, ids.len());
        ImportOutcome {
            imported: ids.len(),
            alert: !batch.failed.is_empty(),
            notice,
            waiting: false,
        }
    }

    /// Converts every SVG of `batch` with the loaded faces; one that brings in nothing becomes a
    /// failure with the notice of criterion 8.
    fn convert_planned(&mut self, batch: &mut ImportBatch) {
        let mut failed = Vec::new();
        for (name, planned) in &mut batch.planned {
            if let Planned::Svg {
                drawing,
                centre,
                import,
                ..
            } = planned
            {
                let converted = self.convert_svg(drawing, SvgPlacement::CentredOn(*centre));
                if converted.leaves == 0 {
                    failed.push((name.clone(), curvyo_ui_core::nothing_imported_notice(name)));
                }
                *import = Some(converted);
            }
        }
        batch.planned.retain(|(_, planned)| match planned {
            Planned::Svg { import, .. } => import.as_ref().is_some_and(|i| i.leaves > 0),
            Planned::Image(_) => true,
        });
        batch.failed.extend(failed);
    }

    /// Writes the planned files in one step and selects them; returns the new top-level ids.
    fn commit_import(&mut self, planned: &[(String, Planned)]) -> Vec<NodeId> {
        if planned.is_empty() {
            return Vec::new();
        }
        let image_count = planned
            .iter()
            .filter(|(_, p)| matches!(p, Planned::Image(_)))
            .count();
        let label = match (image_count, planned.len() - image_count) {
            (1, 0) => IMPORT_IMAGE_COMMIT_LABEL,
            (_, 0) => IMPORT_IMAGES_COMMIT_LABEL,
            (0, 1) => IMPORT_SVG_COMMIT_LABEL,
            _ => IMPORT_COMMIT_LABEL,
        };
        let anchor_ids: Vec<Vec<_>> = planned
            .iter()
            .map(|(_, p)| match p {
                Planned::Svg {
                    import: Some(import),
                    ..
                } => (0..DrawingNode::anchor_count(&import.nodes))
                    .map(|_| self.minter.mint())
                    .collect(),
                _ => Vec::new(),
            })
            .collect();
        if self.tool != Tool::Select {
            self.set_tool(Tool::Select);
        }
        self.select.cancel_entry();
        for (_, p) in planned {
            match p {
                Planned::Image(image) => self.images.insert(
                    curvyo_document_core::AssetHash::of(&image.image.bytes),
                    image.working.clone(),
                ),
                Planned::Svg {
                    import: Some(import),
                    ..
                } => self.cache_pictures(import),
                Planned::Svg { import: None, .. } => {}
            }
        }
        let (ids, layer) = self.recording(|this| {
            let written = this.document.batch(label, |document| {
                let mut ids = Vec::new();
                let mut layer = None;
                for ((_, p), anchors) in planned.iter().zip(&anchor_ids) {
                    match p {
                        Planned::Image(image) => {
                            ids.extend(document.write_images(std::slice::from_ref(&image.image)));
                        }
                        Planned::Svg {
                            layer_name,
                            import: Some(import),
                            ..
                        } => {
                            if let Ok((new_layer, children)) = document.insert_drawing(
                                layer_name,
                                &import.nodes,
                                anchors,
                                &import.assets,
                            ) {
                                ids.extend(children);
                                layer = Some(new_layer);
                            }
                        }
                        Planned::Svg { import: None, .. } => {}
                    }
                }
                (ids, layer)
            });
            this.selection.set(&written.0);
            written
        });
        if layer.is_some() {
            self.layers.active = ActiveLayer::Chosen(layer);
        }
        self.drop_inert_selection();
        ids
    }

    /// The centre of the visible canvas in document millimetres, or the page's centre when the
    /// canvas has no size yet.
    fn import_view_centre(&self) -> Point {
        let (width, height) = self.viewport.canvas_size();
        if width > 0.0 && height > 0.0 {
            return self.viewport_centre();
        }
        let size = self.document.size();
        Point::new(size.width.as_mm() / 2.0, size.height.as_mm() / 2.0)
    }
}

/// The one notice of a finished import (criteria 5, 10, 18, 24, 26; UX notes "Notices").
fn batch_notice(batch: &ImportBatch, imported: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    match (batch.chosen, batch.failed.as_slice()) {
        (1, [(name, reason)]) => lines.push(failure_notice(name, reason)),
        (_, [(_, reason), ..]) => lines.push(batch_failures_notice(batch.failed.len(), reason)),
        _ => {}
    }
    if batch.chosen > MAX_IMPORT_FILES {
        lines.push(skipped_notice(batch.planned.len(), batch.chosen));
    }
    if imported > 0
        && batch.chosen == 1
        && let Some((_, Planned::Image(image))) = batch.planned.first()
    {
        lines.extend(image.lines.iter().cloned());
    }
    lines.join("\n")
}
