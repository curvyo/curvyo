//! The font commands (`specs/0074-text-objects` criteria 19, 20 and 33; ADR 0020 §3;
//! `specs/0075-font-library` criteria 20 to 25): list the Curvyo font folder and the system fonts
//! by name, hand over the bytes of one face's file, copy chosen or dropped files into the folder
//! and delete a family's files from it. The page never sees the path of a listed face: each face
//! has an opaque id that names its file here; only the files the maker chose or dropped for Add
//! font pass through it.

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;

use curvyo_storage_io::FontFace;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

/// The Curvyo font folder's name under the app data directory (criterion 19).
const FONTS_FOLDER: &str = "fonts";

/// The files behind the face ids handed out so far.
#[derive(Default)]
pub(crate) struct FontFiles(Mutex<HashMap<String, PathBuf>>);

/// One face as the page lists it.
#[derive(Clone, serde::Serialize)]
pub(crate) struct ListedFace {
    id: String,
    family: String,
    style: String,
    /// The face's index in its file, for the engine.
    index: u32,
    /// The OS/2 class bytes as 24 hex digits, or `None` (`0075` decision 4).
    os2: Option<String>,
    /// The file's `<length>-<modified seconds>`, for the check cache (`0075` decision 5).
    stamp: String,
}

/// The Curvyo folder's faces and the names of its unreadable files.
#[derive(Clone, serde::Serialize)]
pub(crate) struct FolderList {
    faces: Vec<ListedFace>,
    skipped: Vec<String>,
}

fn listed(files: &FontFiles, faces: Vec<FontFace>) -> Vec<ListedFace> {
    let mut known = files
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    faces
        .into_iter()
        .map(|face| {
            let mut hasher = DefaultHasher::new();
            face.path.hash(&mut hasher);
            let id = format!("{:016x}", hasher.finish());
            known.insert(id.clone(), face.path);
            ListedFace {
                id,
                family: face.family,
                style: face.style,
                index: face.index,
                os2: face.os2.map(|bytes| {
                    bytes.iter().fold(String::new(), |mut hex, b| {
                        use std::fmt::Write as _;
                        let _ = write!(hex, "{b:02x}");
                        hex
                    })
                }),
                stamp: face.stamp,
            }
        })
        .collect()
}

fn fonts_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join(FONTS_FOLDER))
        .map_err(|error| format!("the data folder is not available ({error})"))
}

/// The faces of the Curvyo font folder, read again on every call so a file copied in appears
/// without a restart; the folder is created when missing.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn font_folder_list(
    app: AppHandle,
    files: State<'_, FontFiles>,
) -> Result<FolderList, String> {
    let dir = fonts_dir(&app)?;
    let folder = curvyo_storage_io::list_font_folder(&dir);
    Ok(FolderList {
        faces: listed(&files, folder.faces),
        skipped: folder.skipped,
    })
}

/// The installed system fonts; async, so the slow first read runs off the main thread
/// (criterion 20).
#[tauri::command]
pub(crate) async fn system_font_list(
    files: State<'_, FontFiles>,
) -> Result<Vec<ListedFace>, String> {
    let faces = tauri::async_runtime::spawn_blocking(curvyo_storage_io::list_system_fonts)
        .await
        .map_err(|error| error.to_string())?;
    Ok(listed(&files, faces))
}

/// The bytes of the file of face `id`, raw (not JSON); the page hands them to the engine once per
/// face per session (ADR 0001 §5).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn font_bytes(
    id: String,
    files: State<'_, FontFiles>,
) -> Result<tauri::ipc::Response, String> {
    let path = files
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&id)
        .cloned()
        .ok_or_else(|| "this font is no longer listed".to_string())?;
    // A file over the engine's limit is refused before it is read (ADR 0020 §2).
    let len = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if len > curvyo_storage_io::MAX_FONT_FILE_BYTES {
        return Err("this font file is larger than 64 MiB".to_string());
    }
    let bytes = curvyo_storage_io::read_to_vec(&path).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// What Add font did, by file name (`0075` criterion 22).
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddedFonts {
    added: Vec<String>,
    present: Vec<String>,
    not_fonts: Vec<String>,
}

/// The files the maker chose in the Add font dialog or dropped on the window: the only paths
/// [`add_font_files`] accepts, each once (tester re-review 2026-10-20: the page may not name an
/// arbitrary file for the host to read).
#[derive(Default)]
pub(crate) struct GrantedFontPaths(Mutex<HashSet<PathBuf>>);

/// Grants `paths` to Add font (the dialog's result, a drop on the window).
pub(crate) fn grant_font_paths(app: &AppHandle, paths: &[PathBuf]) {
    app.state::<GrantedFontPaths>()
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .extend(paths.iter().cloned());
}

/// The file types the dialog offers (`0075` criterion 20).
const FONT_FILTER_NAME: &str = "Fonts";
const FONT_FILTER: [&str; 3] = ["ttf", "otf", "ttc"];

/// The file dialog of Add font (`0075` criterion 20): several files, fonts only; `None` when
/// cancelled. The chosen paths go to the page as a drop's do, so it can count them while
/// [`add_font_files`] copies (UX review 2026-10-20). Runs off the main thread.
#[tauri::command]
pub(crate) async fn pick_font_files(app: AppHandle) -> Result<Option<Vec<String>>, String> {
    let dialog = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog
            .dialog()
            .file()
            .add_filter(FONT_FILTER_NAME, &FONT_FILTER)
            .blocking_pick_files()
    })
    .await
    .map_err(|error| error.to_string())?;
    let Some(files) = picked else {
        return Ok(None);
    };
    let paths: Vec<PathBuf> = files
        .into_iter()
        .filter_map(|file| file.into_path().ok())
        .collect();
    grant_font_paths(&app, &paths);
    Ok(Some(
        paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    ))
}

/// Add font (`0075` criteria 21 to 24): the chosen or dropped files are checked, then copied
/// atomically into the Curvyo folder; the page lists the folder again afterwards. Only paths the
/// host granted ([`GrantedFontPaths`]) are accepted, each once; any other path refuses the call.
/// Runs off the main thread.
#[tauri::command]
pub(crate) async fn add_font_files(
    app: AppHandle,
    paths: Vec<String>,
    granted: State<'_, GrantedFontPaths>,
) -> Result<AddedFonts, String> {
    let dir = fonts_dir(&app)?;
    let chosen: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    {
        let mut granted = granted
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !chosen.iter().all(|path| granted.contains(path)) {
            return Err("only files chosen in Add font or dropped on the window are added".into());
        }
        for path in &chosen {
            granted.remove(path);
        }
    }
    let report = tauri::async_runtime::spawn_blocking(move || {
        curvyo_storage_io::add_font_files(&dir, &chosen)
    })
    .await
    .map_err(|error| error.to_string())?;
    Ok(AddedFonts {
        added: report.added,
        present: report.present,
        not_fonts: report.not_fonts,
    })
}

/// One face of the family to remove.
#[derive(serde::Deserialize)]
pub(crate) struct FaceToRemove {
    id: String,
    index: u32,
}

/// What Remove did, by file name (`0075` criterion 25).
#[derive(Clone, serde::Serialize)]
pub(crate) struct RemovedFonts {
    removed: Vec<String>,
    kept: Vec<String>,
}

/// Remove from Curvyo fonts (`0075` criterion 25): deletes the files of these faces that lie
/// directly in the Curvyo folder and hold no other face; any other file is kept and named.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn remove_font_files(
    app: AppHandle,
    faces: Vec<FaceToRemove>,
    files: State<'_, FontFiles>,
) -> Result<RemovedFonts, String> {
    let dir = fonts_dir(&app)?;
    let known = files
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let faces: Vec<(PathBuf, u32)> = faces
        .into_iter()
        .filter_map(|face| known.get(&face.id).map(|path| (path.clone(), face.index)))
        .collect();
    let report = curvyo_storage_io::remove_font_files(&dir, &faces);
    Ok(RemovedFonts {
        removed: report.removed,
        kept: report.kept,
    })
}
