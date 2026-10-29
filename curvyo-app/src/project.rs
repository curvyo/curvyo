//! The commands that move a project between the page and the disk: the open and save dialogs, the
//! path and the window title, the dirty flag the host needs for the window close, and quit
//! (`specs/0051-sidebar-and-no-menu/` `adrs.md` decisions 3 and 4). The page owns the document;
//! this module reads and writes bytes and keeps the project's path.
//!
//! Every command that opens a dialog is `async`: `tauri-plugin-dialog`'s `blocking_*` calls must
//! not run on the main thread, and a sync command or a menu callback does.

use std::path::{Path, PathBuf};

use tauri::Wry;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

use crate::state::{AppState, PendingOpen};

/// The main window's label, matching `tauri.conf.json`.
const MAIN_WINDOW: &str = "main";

/// The native file filter/extension for a Curvyo project, used by both
/// dialogs and the file association in `tauri.conf.json`.
pub(crate) const PROJECT_EXTENSION: &str = "curvyo";
const PROJECT_FILTER_NAME: &str = "Curvyo project";
/// The default filter of Open… (`specs/0024-svg-import-export` UX notes): projects and drawings.
const OPEN_FILTER_NAME: &str = "Curvyo projects and SVG drawings";
const SVG_EXTENSION: &str = "svg";
/// The filter of the Import button (`specs/0058-bitmap-import` criterion 1).
const IMPORT_FILTER_NAME: &str = "Drawings and images";
const IMPORT_EXTENSIONS: [&str; 5] = ["svg", "png", "jpg", "jpeg", "webp"];
const DEFAULT_FILE_NAME: &str = "Untitled.curvyo";

/// What the page is told when a file could not be read or written.
const UNREADABLE_FILE: &str = "This file is damaged and can't be read.";
const SAVE_FAILED: &str = "This project couldn't be saved. Check that you have permission to write to this location and that there's enough disk space, then try again.";

/// A host-level failure to even read a path's bytes (missing file,
/// permissions, not a file) — distinct from a `.curvyo` whose *content* is
/// invalid, which is the frontend's own refusal once it tries
/// `WasmSession::open` on bytes this host did successfully read.
#[derive(Clone, serde::Serialize)]
struct OpenErrorPayload {
    message: String,
}

/// What the "open-bytes" event carries: a file the operating system handed to the running app.
#[derive(Clone, serde::Serialize)]
struct OpenBytesPayload {
    path: String,
    bytes: Vec<u8>,
}

/// A project file this host read, or failed to read, as the page receives it: the answer of
/// [`pick_project`] and of [`take_pending_open`]. This host never looks inside the bytes.
#[derive(Clone, serde::Serialize)]
#[serde(tag = "kind")]
pub(crate) enum OpenPayload {
    #[serde(rename = "bytes")]
    Bytes { path: String, bytes: Vec<u8> },
    #[serde(rename = "io_error")]
    IoError { message: String },
}

impl From<PendingOpen> for OpenPayload {
    fn from(pending: PendingOpen) -> Self {
        match pending {
            PendingOpen::Bytes { path, bytes } => Self::Bytes {
                path: path.to_string_lossy().into_owned(),
                bytes,
            },
            PendingOpen::IoError(message) => Self::IoError { message },
        }
    }
}

/// How a save ended, as the page receives it. There is no separate error event.
#[derive(Clone, serde::Serialize)]
#[serde(tag = "kind")]
pub(crate) enum SaveOutcome {
    /// The bytes are written; the project now lives at `path`.
    #[serde(rename = "saved")]
    Saved { file_name: String, path: String },
    /// The maker closed the save dialog; nothing was written.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// The write failed; the project is as it was.
    #[serde(rename = "failed")]
    Failed { message: String },
}

/// The window title for a project: `Curvyo`, `<file> — Curvyo`, and a leading `*` while it has
/// unsaved changes (`specs/0051-sidebar-and-no-menu/` criterion 30). The page applies the same rule
/// to the browser tab's title.
pub(crate) fn window_title(file_name: Option<&str>, dirty: bool) -> String {
    let marker = if dirty { "*" } else { "" };
    match file_name {
        Some(name) => format!("{marker}{name} — Curvyo"),
        None => format!("{marker}Curvyo"),
    }
}

/// Puts the project's file name and dirty state into the window title.
fn refresh_window_title(app: &AppHandle) {
    let state = app.state::<AppState>();
    let file_name = state.lock_project().file_name();
    let title = window_title(file_name.as_deref(), state.is_dirty());
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.set_title(&title);
    }
}

/// Returns and clears any open payload buffered before the frontend had
/// mounted — the file-association launch path's open attempt runs in
/// `.setup()`, ahead of any `listen()` call. Call once, at mount.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn take_pending_open(state: tauri::State<AppState>) -> Option<OpenPayload> {
    state.take_pending_open().map(OpenPayload::from)
}

/// Shows the native open dialog and reads the chosen file. `None` when the maker cancelled. The
/// answer returns to the caller, not as an event, so a file chosen after "Discard" never raises a
/// second unsaved-changes check (`specs/0051-sidebar-and-no-menu/` criterion 18). Nothing about
/// the open project changes here: [`confirm_project_opened`] does that once the page has parsed
/// the bytes.
///
/// `kind` chooses the filters (`specs/0024-svg-import-export` criterion 1): `"open"` lists
/// projects and SVG drawings, then all files; anything else (the Template button) projects only.
/// The page decides by the content what the file is.
#[tauri::command]
#[allow(clippy::unused_async)] // async keeps the blocking dialog off the main thread
pub(crate) async fn pick_project(app: AppHandle, kind: Option<String>) -> Option<OpenPayload> {
    let dialog = match kind.as_deref() {
        Some("open") => parented(
            &app,
            app.dialog()
                .file()
                .add_filter(OPEN_FILTER_NAME, &[PROJECT_EXTENSION, SVG_EXTENSION])
                .add_filter("All files", &["*"]),
        ),
        _ => project_dialog(&app),
    };
    let picked = dialog.blocking_pick_file()?;
    let path = picked.into_path().ok()?;
    Some(read_project(&path).into())
}

/// The Import button's dialog (`specs/0058-bitmap-import` criteria 1 and 5): drawings and
/// images, then all files, several at once; each chosen file read, in the order the dialog
/// returns them. Empty when the maker cancelled. The page decides by the content what each is.
#[tauri::command]
#[allow(clippy::unused_async)] // async keeps the blocking dialog off the main thread
pub(crate) async fn pick_import_files(app: AppHandle) -> Vec<OpenPayload> {
    pick_import_paths(&app)
        .into_iter()
        .map(|path| read_project(&path).into())
        .collect()
}

/// Linux: our GTK3 dialog with the preview pane of the selected image (`specs/0087-image-clipboard-and-drop`
/// criterion 26, `adrs.md` decision 6).
#[cfg(target_os = "linux")]
fn pick_import_paths(app: &AppHandle) -> Vec<PathBuf> {
    crate::import_dialog::pick_files(
        app,
        &[
            (IMPORT_FILTER_NAME, &IMPORT_EXTENSIONS),
            ("All files", &["*"]),
        ],
    )
}

/// Windows and macOS: the OS dialog with its own preview (the recorded deviation of
/// `specs/0087-image-clipboard-and-drop` `adrs.md` decision 6).
#[cfg(not(target_os = "linux"))]
fn pick_import_paths(app: &AppHandle) -> Vec<PathBuf> {
    parented(
        app,
        app.dialog()
            .file()
            .add_filter(IMPORT_FILTER_NAME, &IMPORT_EXTENSIONS)
            .add_filter("All files", &["*"]),
    )
    .blocking_pick_files()
    .unwrap_or_default()
    .into_iter()
    .filter_map(|picked| picked.into_path().ok())
    .collect()
}

/// The frontend's confirmation that bytes this host handed it (via
/// "open-bytes", [`pick_project`] or [`take_pending_open`]) parsed successfully and are now
/// the live `WasmSession`. Only now does this host start treating `path`
/// as the open project's own path — acceptance criterion 7: a refusal
/// (the frontend never calling this) must never touch an already-open
/// project, and nothing here ever does. The opened project is clean.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn confirm_project_opened(app: AppHandle, path: String) {
    app.state::<AppState>().confirm_opened(PathBuf::from(path));
    refresh_window_title(&app);
}

/// New, Close, and the start of every page (a reload included): the page holds an empty,
/// never-saved, clean project, so the host forgets the path and the dirty flag of whatever was
/// open before. A file handed over at launch stays buffered.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn reset_project(app: AppHandle) {
    app.state::<AppState>().reset_project();
    refresh_window_title(&app);
}

/// The page's report that the project has, or no longer has, unsaved changes. The host needs it
/// for the window close, which it decides without asking the page (criterion 41).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn set_dirty(app: AppHandle, dirty: bool) {
    app.state::<AppState>().set_dirty(dirty);
    refresh_window_title(&app);
}

/// The frontend's half of Save/Save As: here are the live session's
/// already-packed bytes, write them. `save_as` forces the native save
/// dialog even when a path is already known (acceptance criterion 3);
/// otherwise an unknown path falls back to the same dialog (first-ever
/// save).
#[tauri::command]
#[allow(clippy::unused_async)] // async keeps the blocking dialog off the main thread
pub(crate) async fn save_project_bytes(
    app: AppHandle,
    bytes: Vec<u8>,
    save_as: bool,
    suggested_name: Option<String>,
) -> SaveOutcome {
    let state = app.state::<AppState>();
    let (known_path, known_name) = {
        let project = state.lock_project();
        (project.path.clone(), project.file_name())
    };
    // A project opened from an SVG has no file yet; the dialog offers the SVG's name with
    // `.curvyo` (`specs/0024-svg-import-export` criterion 2).
    let known_name = known_name.or(suggested_name);
    let path = if save_as { None } else { known_path };
    let Some(path) = path.or_else(|| pick_save_path(&app, known_name.as_deref())) else {
        return SaveOutcome::Cancelled;
    };

    if curvyo_storage_io::write_atomic(&path, &bytes).is_err() {
        // A failed save that looks successful is the worst failure mode
        // this product can have — report it, don't just return.
        return SaveOutcome::Failed {
            message: SAVE_FAILED.to_string(),
        };
    }

    let mut project = state.lock_project();
    project.path = Some(path.clone());
    let file_name = project
        .file_name()
        .unwrap_or_else(|| DEFAULT_FILE_NAME.to_string());
    drop(project);
    refresh_window_title(&app);
    SaveOutcome::Saved {
        file_name,
        path: path.to_string_lossy().into_owned(),
    }
}

/// Leaves the application: the page asked, after Discard, a successful Save, or with nothing to
/// lose. The exit is not held.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn quit_app(app: AppHandle) {
    app.state::<AppState>().begin_quit();
    app.exit(0);
}

/// Tells the page that the window is being closed while the project has unsaved changes; it shows
/// the check and ends with [`quit_app`]. Returns whether the close was held.
pub(crate) fn hold_close_for_check(app: &AppHandle) -> bool {
    if !app.state::<AppState>().must_hold_close() {
        return false;
    }
    let _ = app.emit("close-requested", ());
    true
}

/// Quit from the application menu of macOS: leaves at once when there is nothing to lose, otherwise
/// asks the page, which ends with [`quit_app`] after Discard or a successful Save.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn request_quit(app: &AppHandle) {
    if !hold_close_for_check(app) {
        app.exit(0);
    }
}

/// Reads the bytes at `path`, or says why that failed.
fn read_project(path: &Path) -> PendingOpen {
    match curvyo_storage_io::read_to_vec(path) {
        Ok(bytes) => PendingOpen::Bytes {
            path: path.to_path_buf(),
            bytes,
        },
        // Not one of curvyo_document_core::OpenError's three cases (that
        // taxonomy is about file *content*, and only the frontend's
        // `WasmSession::open` ever sees it now), but the UI has one
        // failure dialog, so an unreadable path is reported the same way
        // a damaged container used to be.
        Err(_) => PendingOpen::IoError(UNREADABLE_FILE.to_string()),
    }
}

/// A file the operating system handed to the app (the file association at launch, or macOS's
/// Apple Event while it runs): reads it and tells the page by event, and by the buffer for a page
/// that has not mounted yet (acceptance criterion 7: a refusal never touches an already-open
/// project; [`confirm_project_opened`] does that once the page has parsed the bytes).
pub(crate) fn open_handed_path(app: &AppHandle, path: &Path) {
    let pending = read_project(path);
    match &pending {
        PendingOpen::Bytes { path, bytes } => {
            let _ = app.emit(
                "open-bytes",
                OpenBytesPayload {
                    path: path.to_string_lossy().into_owned(),
                    bytes: bytes.clone(),
                },
            );
        }
        PendingOpen::IoError(message) => {
            let _ = app.emit(
                "open-error",
                OpenErrorPayload {
                    message: message.clone(),
                },
            );
        }
    }
    app.state::<AppState>().set_pending_open(pending);
}

/// A file dialog for project files, modal to the main window: on Linux and Windows the window
/// stays operable while a dialog without a parent is up, and a New or Discard pressed meanwhile
/// would make the dialog's answer land on a different project.
fn project_dialog(app: &AppHandle) -> FileDialogBuilder<Wry> {
    parented(
        app,
        app.dialog()
            .file()
            .add_filter(PROJECT_FILTER_NAME, &[PROJECT_EXTENSION]),
    )
}

/// `dialog` modal to the main window, when it exists.
fn parented(app: &AppHandle, dialog: FileDialogBuilder<Wry>) -> FileDialogBuilder<Wry> {
    match app.get_webview_window(MAIN_WINDOW) {
        Some(window) => dialog.set_parent(&window),
        None => dialog,
    }
}

/// The native Save As… dialog, defaulting to the project's file name, or `Untitled.curvyo`.
fn pick_save_path(app: &AppHandle, current_name: Option<&str>) -> Option<PathBuf> {
    project_dialog(app)
        .set_file_name(current_name.unwrap_or(DEFAULT_FILE_NAME))
        .blocking_save_file()
        .and_then(|file_path| file_path.into_path().ok())
}

#[cfg(test)]
mod tests {
    use super::window_title;

    /// Criterion 30: the title for every combination of file name and dirty state.
    #[test]
    fn the_title_names_the_file_and_marks_unsaved_changes() {
        assert_eq!(window_title(None, false), "Curvyo");
        assert_eq!(window_title(None, true), "*Curvyo");
        assert_eq!(
            window_title(Some("Poster.curvyo"), false),
            "Poster.curvyo — Curvyo"
        );
        assert_eq!(
            window_title(Some("Poster.curvyo"), true),
            "*Poster.curvyo — Curvyo"
        );
    }
}
