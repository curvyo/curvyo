//! The desktop's half of Export job file (`specs/0029-laser-job-preview-and-output` criterion 27):
//! the save dialog first, offering the output format's file type and extension, its path kept
//! here, then the job file's bytes (a raw body, `raw_ipc`) written atomically, so a failure leaves
//! the old file or nothing. It shares `AppState`'s export path with Export SVG…; one export runs
//! at a time.

use std::path::PathBuf;

use tauri::ipc::Request;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::raw_ipc::call_args;
use crate::state::AppState;

/// The dialog's title.
const DIALOG_TITLE: &str = "Export job file";

/// The write has no arguments besides its bytes.
#[derive(serde::Deserialize)]
struct NoArgs {}

/// The sentence of a failed write: the system's reason, then what it means.
fn failure_text(reason: &str) -> String {
    let reason = reason.trim_end_matches('.');
    format!("{reason}. Nothing was written.")
}

/// Shows the save dialog for a job file, offering `suggested_name`, filtered to `extension` under
/// the name `file_type`. Returns the chosen file's name (the path stays here for
/// [`write_job_export`]), or `None` when the maker cancelled.
#[tauri::command]
#[allow(clippy::unused_async)] // async keeps the blocking dialog off the main thread
pub(crate) async fn pick_job_export_path(
    app: AppHandle,
    suggested_name: String,
    file_type: String,
    extension: String,
) -> Option<String> {
    let mut dialog = app
        .dialog()
        .file()
        .set_title(DIALOG_TITLE)
        .add_filter(&file_type, &[extension.as_str()])
        .set_file_name(&suggested_name);
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    let path: PathBuf = dialog.blocking_save_file()?.into_path().ok()?;
    let name = path.file_name()?.to_string_lossy().into_owned();
    app.state::<AppState>().set_export_path(Some(path));
    Some(name)
}

/// Writes the raw body to the path the last [`pick_job_export_path`] chose, whole or not at all.
///
/// # Errors
/// The error dialog's body: the system's reason and "Nothing was written."
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_job_export(app: AppHandle, request: Request<'_>) -> Result<String, String> {
    let (_, bytes): (NoArgs, Vec<u8>) = call_args(&request)?;
    let Some(path) = app.state::<AppState>().take_export_path() else {
        return Err(failure_text("No file was chosen"));
    };
    match curvyo_storage_io::write_atomic(&path, &bytes) {
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
    fn a_failure_names_the_reason_and_says_nothing_was_written() {
        assert_eq!(
            failure_text("Permission denied (os error 13)"),
            "Permission denied (os error 13). Nothing was written."
        );
    }
}
