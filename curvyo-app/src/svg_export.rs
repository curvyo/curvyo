//! The desktop's half of Export SVG… (`specs/0024-svg-import-export` criterion 4, `adrs.md`
//! export §5): the save dialog first, its path kept here, then the page's whole SVG text written
//! atomically, so a failure leaves the old file or nothing.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::state::AppState;

/// The filter of the export's save dialog.
const SVG_FILTER_NAME: &str = "SVG drawing";
/// The extension of an exported drawing.
const SVG_EXTENSION: &str = "svg";
/// The dialog's title (`ux-notes.md`, "Flow").
const DIALOG_TITLE: &str = "Export SVG";
/// The name offered when the page names none.
const DEFAULT_NAME: &str = "Untitled.svg";

/// The sentence of a failed write: the system's reason, then what it means (`ux-notes.md`,
/// "Error dialog").
fn failure_text(reason: &str) -> String {
    let reason = reason.trim_end_matches('.');
    format!("{reason}. Nothing was written.")
}

/// Shows the save dialog for an SVG, offering `suggested_name`. Returns the chosen file's name
/// (the path stays here for [`write_svg_export`]), or `None` when the maker cancelled.
#[tauri::command]
#[allow(clippy::unused_async)] // async keeps the blocking dialog off the main thread
pub(crate) async fn pick_svg_export_path(
    app: AppHandle,
    suggested_name: Option<String>,
) -> Option<String> {
    let mut dialog = app
        .dialog()
        .file()
        .set_title(DIALOG_TITLE)
        .add_filter(SVG_FILTER_NAME, &[SVG_EXTENSION])
        .set_file_name(suggested_name.as_deref().unwrap_or(DEFAULT_NAME));
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    let path: PathBuf = dialog.blocking_save_file()?.into_path().ok()?;
    let name = path.file_name()?.to_string_lossy().into_owned();
    app.state::<AppState>().set_svg_export_path(Some(path));
    Some(name)
}

/// Writes `text` to the path the last [`pick_svg_export_path`] chose, whole or not at all.
///
/// # Errors
/// The error dialog's body: the system's reason and "Nothing was written."
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_svg_export(app: AppHandle, text: String) -> Result<String, String> {
    let state = app.state::<AppState>();
    let Some(path) = state.take_svg_export_path() else {
        return Err(failure_text("No file was chosen"));
    };
    match curvyo_storage_io::write_atomic(&path, text.as_bytes()) {
        Ok(()) => Ok(path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned())),
        Err(curvyo_storage_io::FsError::Write { source, .. }) => {
            Err(failure_text(&source.to_string()))
        }
        Err(other) => Err(failure_text(&other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::failure_text;

    #[test]
    fn ac4_a_failure_names_the_reason_and_says_nothing_was_written() {
        assert_eq!(
            failure_text("Permission denied (os error 13)"),
            "Permission denied (os error 13). Nothing was written."
        );
    }
}
