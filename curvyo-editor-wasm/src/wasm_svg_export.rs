//! The `wasm-bindgen` surface of Export SVG… (`specs/0024-svg-import-export` criteria 4, 147 and
//! 148): the four switches in, the file's text and the counts the notices need out.

use wasm_bindgen::prelude::*;

use crate::session::SvgExportResult;
use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// The SVG of the document with the four options (criterion 4): `"svg:<objects>:<texts kept
    /// as text>:<text>"`, or `"nothing-selected"` when Selection only is on and nothing is
    /// selected (no file, criterion 131). The document is not changed.
    #[must_use]
    #[allow(clippy::fn_params_excessive_bools)] // the four switches; scalars across the boundary (ADR 0001 §5)
    pub fn export_svg(
        &self,
        text_as_paths: bool,
        strokes_as_paths: bool,
        selection_only: bool,
        active_page_only: bool,
    ) -> String {
        let options = curvyo_ui_core::SvgExportOptions {
            text_as_paths,
            strokes_as_paths,
            selection_only,
            active_page_only,
        };
        match self.session.export_svg(options) {
            SvgExportResult::Written {
                text,
                objects,
                texts_kept_live,
            } => format!("svg:{objects}:{texts_kept_live}:{text}"),
            SvgExportResult::NothingSelected => "nothing-selected".to_owned(),
        }
    }

    /// How many objects the document has: the host shows the busy state from 5,000.
    #[must_use]
    pub fn export_object_count(&self) -> usize {
        self.session.export_object_count()
    }
}

/// The name Export SVG… offers (criterion 4): `source_svg` when the project came from an SVG
/// (empty otherwise), else the project's file name with `.svg`, else "Untitled.svg".
#[wasm_bindgen]
#[must_use]
pub fn svg_export_name(source_svg: &str, project_file: &str) -> String {
    curvyo_ui_core::svg_export_name(Some(source_svg), Some(project_file))
}

/// "Exporting poster.svg...", shown from [`svg_export_busy_objects`] objects.
#[wasm_bindgen]
#[must_use]
pub fn svg_export_busy_notice(file_name: &str) -> String {
    curvyo_ui_core::svg_export_busy_notice(file_name)
}

/// From this many objects the export shows the busy state.
#[wasm_bindgen]
#[must_use]
pub fn svg_export_busy_objects() -> usize {
    curvyo_ui_core::SVG_EXPORT_BUSY_OBJECTS
}

/// The notice after an export (criteria 4, 148) and how long it stays, as `"<ms>:<sentence>"`.
#[wasm_bindgen]
#[must_use]
pub fn svg_export_notice(file_name: &str, downloaded: bool, objects: usize, kept: usize) -> String {
    format!(
        "{}:{}",
        curvyo_ui_core::svg_export_notice_ms(kept),
        curvyo_ui_core::svg_export_notice(file_name, downloaded, objects, kept)
    )
}

/// "Nothing is selected." (criterion 131).
#[wasm_bindgen]
#[must_use]
pub fn svg_export_nothing_selected() -> String {
    curvyo_ui_core::NOTHING_SELECTED_NOTICE.to_owned()
}
