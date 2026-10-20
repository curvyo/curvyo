//! The font commands (`specs/0074-text-objects` criteria 19, 20 and 33; ADR 0020 §3): list the
//! Curvyo font folder and the system fonts by name, and hand over the bytes of one face's file.
//! The page never sees a path: each face has an opaque id that names its file here.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;

use curvyo_storage_io::FontFace;
use tauri::{AppHandle, Manager, State};

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
            }
        })
        .collect()
}

/// The faces of the Curvyo font folder, read again on every call so a file copied in appears
/// without a restart; the folder is created when missing.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn font_folder_list(
    app: AppHandle,
    files: State<'_, FontFiles>,
) -> Result<FolderList, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("the data folder is not available ({error})"))?
        .join(FONTS_FOLDER);
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
