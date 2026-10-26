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
    /// `"single_line"` for an SVG font (`0076` criterion 1), else absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
    /// The file's name and size, and the folder it is in for display only (`0083` criterion 2,
    /// ADR 0020 §3 note of 2026-10-26): the page never passes the path back.
    file: String,
    size: u64,
    path: String,
    /// The Curvyo folder's subfolder the file is in (`0083` criterion 10).
    #[serde(skip_serializing_if = "Option::is_none")]
    folder: Option<String>,
}

/// A subfolder of the Curvyo folder and its documents (`0083` criterion 4).
#[derive(Clone, serde::Serialize)]
pub(crate) struct ListedFolder {
    name: String,
    documents: Vec<ListedDocument>,
}

#[derive(Clone, serde::Serialize)]
pub(crate) struct ListedDocument {
    name: String,
    size: u64,
}

/// The Curvyo folder's faces and the names of its unreadable files.
#[derive(Clone, serde::Serialize)]
pub(crate) struct FolderList {
    faces: Vec<ListedFace>,
    skipped: Vec<String>,
    folders: Vec<ListedFolder>,
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
            let file = face
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let path = face
                .path
                .parent()
                .map(|dir| dir.display().to_string())
                .unwrap_or_default();
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
                kind: face.single_line.then_some("single_line"),
                file,
                size: face.size,
                path,
                folder: face.folder,
            }
        })
        .collect()
}

pub(crate) fn fonts_dir(app: &AppHandle) -> Result<PathBuf, String> {
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
        folders: folder
            .folders
            .into_iter()
            .map(|f| ListedFolder {
                name: f.name,
                documents: f
                    .documents
                    .into_iter()
                    .map(|d| ListedDocument {
                        name: d.name,
                        size: d.size,
                    })
                    .collect(),
            })
            .collect(),
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

/// What Add font did, by file name (`0075` criterion 22), and per archive (`0083` criterion 13).
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddedFonts {
    added: Vec<String>,
    present: Vec<String>,
    not_fonts: Vec<String>,
    archives: Vec<AddedArchive>,
}

/// What adding one archive did; `refused` is `"unreadable"` or `"over_limit"`.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddedArchive {
    name: String,
    added: usize,
    present: usize,
    documents: usize,
    unsafe_entries: usize,
    refused: Option<&'static str>,
}

impl From<curvyo_storage_io::AddedArchive> for AddedArchive {
    fn from(report: curvyo_storage_io::AddedArchive) -> Self {
        Self {
            name: report.name,
            added: report.added.len(),
            present: report.present,
            documents: report.documents,
            unsafe_entries: report.unsafe_entries,
            refused: report.refused.map(|refusal| match refusal {
                curvyo_storage_io::ArchiveRefusal::Unreadable => "unreadable",
                curvyo_storage_io::ArchiveRefusal::OverLimit => "over_limit",
            }),
        }
    }
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

/// The file types the dialog offers (`0075` criterion 20; SVG fonts, `0076` criterion 3; zip
/// archives, `0083` criterion 7).
const FONT_FILTER_NAME: &str = "Fonts";
const FONT_FILTER: [&str; 5] = ["ttf", "otf", "ttc", "svg", "zip"];

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
    let (archives, fonts): (Vec<PathBuf>, Vec<PathBuf>) = chosen
        .into_iter()
        .partition(|path| curvyo_storage_io::is_archive(path));
    let (report, archives) = tauri::async_runtime::spawn_blocking(move || {
        let archives: Vec<AddedArchive> = archives
            .iter()
            .map(|path| curvyo_storage_io::add_font_archive(&dir, path).into())
            .collect();
        (curvyo_storage_io::add_font_files(&dir, &fonts), archives)
    })
    .await
    .map_err(|error| error.to_string())?;
    Ok(AddedFonts {
        added: report.added,
        present: report.present,
        not_fonts: report.not_fonts,
        archives,
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
    /// Documents deleted with an emptied font folder (`0083` criterion 14).
    documents: usize,
}

/// Remove from Curvyo fonts (`0075` criterion 25): deletes the files of these faces that lie
/// directly in the Curvyo folder or a subfolder of it and hold no other face; any other file is
/// kept and named. An emptied font folder goes with its documents (`0083` criterion 14).
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
        documents: report.documents,
    })
}
