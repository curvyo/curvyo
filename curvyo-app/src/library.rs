//! The host side of the document library (`specs/0137-document-library` criteria 1 to 3, 7, 10
//! to 12, ADR 0084 §2 to §5): the sources in order, the scan that keeps the per-user index
//! current, the tree rows, the page pictures, the read-only bytes for the preview and the import,
//! the added folders and the lookup of Open source. Local files only; nothing leaves the machine.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use tauri::ipc::Response;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use curvyo_document_core::CameFrom;
use curvyo_storage_io::{
    AddFolderRefusal, LibraryFolders, LibraryFoldersError, LibraryMembers, SourceError,
};
use curvyo_ui_core::library_index::{IndexedDocument, LibraryIndex};
use curvyo_ui_core::library_index_text::{SourceReason, already_in_text, is_workspace_text};
use curvyo_ui_core::library_tree::{LibraryRow, LibrarySource, SourceKind, TreeView};

use crate::state::AppState;

/// The library's memory in this window: the index read from the cache and kept current, and the
/// sources of the last scan with their folders.
#[derive(Default)]
pub(crate) struct LibraryState {
    index: Mutex<Option<LibraryIndex>>,
    sources: Mutex<Vec<(LibrarySource, PathBuf)>>,
}

pub(crate) fn data_file(app: &AppHandle, name: &str) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|dir| dir.join(name))
}

fn read_folders(file: &Path) -> Result<LibraryFolders, LibraryFoldersError> {
    match curvyo_storage_io::read_optional(file) {
        Ok(Some(bytes)) => match LibraryFolders::parse(&String::from_utf8_lossy(&bytes)) {
            Err(LibraryFoldersError::Damaged) => Ok(LibraryFolders::default()),
            other => other,
        },
        _ => Ok(LibraryFolders::default()),
    }
}

fn write_folders(file: &Path, folders: &LibraryFolders) {
    let _ = curvyo_storage_io::ensure_parent_dir(file);
    let _ = curvyo_storage_io::write_atomic(file, folders.to_text().as_bytes());
}

fn folder_name(root: &Path) -> String {
    root.file_name().map_or_else(
        || root.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The source key of a workspace: its `workspace.toml` id (ADR 0084 §3), else its path.
fn workspace_key(root: &Path) -> String {
    let marker = root
        .join(curvyo_storage_io::MARKER_DIR)
        .join(curvyo_storage_io::MARKER_FILE);
    curvyo_storage_io::read_optional(&marker)
        .ok()
        .flatten()
        .and_then(|bytes| curvyo_storage_io::parse_marker(&String::from_utf8_lossy(&bytes)).ok())
        .map_or_else(
            || root.display().to_string(),
            |id| {
                id.iter().fold(String::new(), |mut text, b| {
                    use std::fmt::Write as _;
                    let _ = write!(text, "{b:02x}");
                    text
                })
            },
        )
}

/// The sources in order (ADR 0084 §3): the open workspace, the other workspaces of the recent
/// list that exist, then the added folders.
fn current_sources(app: &AppHandle) -> Vec<(LibrarySource, PathBuf)> {
    let open = app.state::<AppState>().workspace();
    let mut sources = Vec::new();
    let mut push = |root: PathBuf, kind: SourceKind, key: String| {
        sources.push((
            LibrarySource {
                key,
                name: folder_name(&root),
                kind,
                error: None,
            },
            root,
        ));
    };
    if let Some(root) = &open {
        push(root.clone(), SourceKind::OpenWorkspace, workspace_key(root));
    }
    let recents = crate::workspace::recents_file(app)
        .as_deref()
        .and_then(crate::workspace::read_recents)
        .unwrap_or_default();
    for row in recents.rows() {
        if row.found && Some(&row.path) != open.as_ref() {
            let key = workspace_key(&row.path);
            push(row.path, SourceKind::Workspace, key);
        }
    }
    if let Some(file) = data_file(app, curvyo_storage_io::LIBRARY_FOLDERS_FILE) {
        for path in read_folders(&file).unwrap_or_default().paths() {
            push(path.clone(), SourceKind::Folder, path.display().to_string());
        }
    }
    sources
}

const fn reason(error: SourceError) -> SourceReason {
    match error {
        SourceError::NotFound => SourceReason::NotFound,
        SourceError::NoAccess => SourceReason::NoAccess,
        SourceError::Io => SourceReason::Io,
    }
}

fn millis(time: Option<std::time::SystemTime>) -> curvyo_document_core::UnixMillis {
    curvyo_document_core::UnixMillis::new(
        time.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .and_then(|d| i64::try_from(d.as_millis()).ok())
            .unwrap_or(0),
    )
}

/// The record of one document file, read without opening it (ADR 0084 §2); a file without the
/// page list is read whole and unpacked in memory, never written back.
fn read_record(
    root: &Path,
    scanned: &curvyo_storage_io::ScannedDocument,
    key: &str,
) -> IndexedDocument {
    let members = curvyo_storage_io::read_library_members(root, &scanned.path)
        .unwrap_or(LibraryMembers::Damaged);
    curvyo_ui_core::library_index::record_of_members(
        key,
        scanned.path.clone(),
        (scanned.size, millis(scanned.modified)),
        members,
        || curvyo_storage_io::library_read(root, &scanned.path).ok(),
    )
}

/// The scan (ADR 0084 §3): every source is read, documents whose size or time changed are read
/// again, the cache is written. A source that cannot be read carries its reason.
fn scan(app: &AppHandle, library: &LibraryState) -> usize {
    let mut sources = current_sources(app);
    let cache = data_file(app, curvyo_ui_core::library_index::LIBRARY_INDEX_FILE);
    let mut index = library
        .index
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .unwrap_or_else(|| {
            cache
                .as_deref()
                .and_then(|file| curvyo_storage_io::read_optional(file).ok().flatten())
                .map(|bytes| LibraryIndex::parse(&String::from_utf8_lossy(&bytes)))
                .unwrap_or_default()
        });
    for (source, root) in &mut sources {
        match curvyo_storage_io::scan_library_source(root) {
            Ok(found) => {
                for scanned in &found {
                    if index.needs_read(
                        &source.key,
                        &scanned.path,
                        scanned.size,
                        millis(scanned.modified),
                    ) {
                        index.put(read_record(root, scanned, &source.key));
                    }
                }
                let present: Vec<Vec<String>> = found.into_iter().map(|d| d.path).collect();
                index.keep_present(&source.key, &present);
            }
            Err(error) => source.error = Some(reason(error)),
        }
    }
    let keys: Vec<String> = sources.iter().map(|(s, _)| s.key.clone()).collect();
    index.order_by_sources(&keys);
    let readable = index
        .documents()
        .iter()
        .filter(|d| {
            sources
                .iter()
                .any(|(s, _)| s.key == d.source && s.error.is_none())
        })
        .count();
    if let Some(file) = cache {
        let _ = curvyo_storage_io::ensure_parent_dir(&file);
        let _ = curvyo_storage_io::write_atomic(&file, index.to_text().as_bytes());
    }
    if let Ok(mut slot) = library.index.lock() {
        *slot = Some(index);
    }
    if let Ok(mut slot) = library.sources.lock() {
        *slot = sources;
    }
    readable
}

/// Scans the sources and keeps the index current; the number of documents in readable sources.
/// Run when the tab is shown and on window focus while it is shown (ADR 0084 §3).
#[tauri::command]
pub(crate) async fn library_scan(app: AppHandle) -> usize {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || scan(&handle, &handle.state::<LibraryState>()))
        .await
        .unwrap_or(0)
}

/// The arguments of `library_rows`.
#[derive(Debug, Deserialize)]
pub(crate) struct RowsArgs {
    query: String,
    expanded: Vec<String>,
    now: i64,
    offset_minutes: i32,
}

/// The rows of the tree for the search and expand state (criteria 1, 3, 11, 13).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn library_rows(app: AppHandle, args: RowsArgs) -> Vec<LibraryRow> {
    let library = app.state::<LibraryState>();
    let state = app.state::<AppState>();
    let sources: Vec<LibrarySource> = library
        .sources
        .lock()
        .map(|s| s.iter().map(|(source, _)| source.clone()).collect())
        .unwrap_or_default();
    let roots: Vec<(String, PathBuf)> = library
        .sources
        .lock()
        .map(|s| {
            s.iter()
                .map(|(source, root)| (source.key.clone(), root.clone()))
                .collect()
        })
        .unwrap_or_default();
    let index = library
        .index
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .unwrap_or_default();
    // Criterion 11: Saved version for this window's document while it has unsaved changes.
    let project = state.lock_project().path.clone();
    let saved = project.filter(|_| state.is_dirty()).and_then(|path| {
        roots.iter().find_map(|(key, root)| {
            let inside = path.strip_prefix(root).ok()?;
            let parts: Vec<String> = inside
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            Some((key.clone(), parts))
        })
    });
    let expanded = args.expanded.into_iter().collect();
    let mut rows = curvyo_ui_core::library_tree::library_rows(
        &sources,
        &index,
        &TreeView {
            expanded: &expanded,
            query: &args.query,
            now: (args.now, args.offset_minutes),
            saved_version: saved.as_ref().map(|(k, p)| (k.as_str(), p.as_slice())),
        },
    );
    // The third group (`specs/0138`): what this session fetched of the sources the maker added.
    rows.extend(crate::online::rows_of(&app, &expanded, &args.query));
    rows
}

/// The sources of the last scan and the index, for the library browser (`specs/0152`).
pub(crate) fn local_parts(app: &AppHandle) -> (Vec<LibrarySource>, LibraryIndex) {
    let library = app.state::<LibraryState>();
    let sources = library
        .sources
        .lock()
        .map(|s| s.iter().map(|(source, _)| source.clone()).collect())
        .unwrap_or_default();
    let index = library
        .index
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .unwrap_or_default();
    (sources, index)
}

fn root_of(app: &AppHandle, source: &str) -> Result<PathBuf, String> {
    app.state::<LibraryState>()
        .sources
        .lock()
        .ok()
        .and_then(|s| {
            s.iter()
                .find(|(x, _)| x.key == source)
                .map(|(_, root)| root.clone())
        })
        .ok_or_else(|| "not-found".to_owned())
}

/// The PNG of one page picture, raw; empty when the page has none (criterion 2).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn library_page_picture(
    app: AppHandle,
    source: String,
    path: Vec<String>,
    page: String,
) -> Result<Response, String> {
    // A downloaded template of an online source: its page pictures come from the held bytes.
    if let Some(bytes) = crate::online::template_bytes(&app, &source, &path) {
        let png = curvyo_document_core::read_page_picture(std::io::Cursor::new(&**bytes), &page);
        return Ok(Response::new(png.unwrap_or_default()));
    }
    let root = root_of(&app, &source)?;
    curvyo_storage_io::read_page_picture(&root, &path, &page)
        .map(|png| Response::new(png.unwrap_or_default()))
        .map_err(|_| "not-found".to_owned())
}

/// The bytes of a library document for the read-only preview and the import (ADR 0084 §5),
/// raw; the file is opened read-only and nothing is written.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn library_read_document(
    app: AppHandle,
    source: String,
    path: Vec<String>,
) -> Result<Response, String> {
    // A template the maker loaded or expanded was downloaded and checked before it is read here.
    if let Some(bytes) = crate::online::template_bytes(&app, &source, &path) {
        return Ok(Response::new((*bytes).clone()));
    }
    let root = root_of(&app, &source)?;
    curvyo_storage_io::library_read(&root, &path)
        .map(Response::new)
        .map_err(|_| "not-found".to_owned())
}

/// What Add folder... did.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AddFolderAnswer {
    /// The folder is in the list now, under this source key.
    Added { key: String },
    /// The dialog was closed.
    Cancelled,
    /// Not added, with the sentence to show.
    Refused { text: String },
}

/// Add folder...: the OS folder dialog, then the folder at the end of the list (criterion 1).
#[tauri::command]
#[allow(clippy::unused_async)] // async keeps the blocking dialog off the main thread
pub(crate) async fn library_add_folder(app: AppHandle) -> AddFolderAnswer {
    let picked = app
        .dialog()
        .file()
        .set_title("Add folder to library")
        .blocking_pick_folder();
    let Some(root) = picked.and_then(|p| p.into_path().ok()) else {
        return AddFolderAnswer::Cancelled;
    };
    let name = folder_name(&root);
    if current_sources(&app)
        .iter()
        .any(|(s, r)| s.kind != SourceKind::Folder && *r == root)
    {
        return AddFolderAnswer::Refused {
            text: is_workspace_text(&name),
        };
    }
    let Some(file) = data_file(&app, curvyo_storage_io::LIBRARY_FOLDERS_FILE) else {
        return AddFolderAnswer::Cancelled;
    };
    let Ok(mut folders) = read_folders(&file) else {
        return AddFolderAnswer::Refused {
            text: "The library list was made by a newer Curvyo.".to_owned(),
        };
    };
    match folders.add(&root) {
        Ok(()) => {
            write_folders(&file, &folders);
            AddFolderAnswer::Added {
                key: root.display().to_string(),
            }
        }
        Err(AddFolderRefusal::AlreadyIn | AddFolderRefusal::NotText) => AddFolderAnswer::Refused {
            text: already_in_text(&name),
        },
    }
}

/// Removes an added folder from the list; nothing on disk changes (criterion 1).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn library_remove_folder(app: AppHandle, key: String) {
    let Some(file) = data_file(&app, curvyo_storage_io::LIBRARY_FOLDERS_FILE) else {
        return;
    };
    if let Ok(mut folders) = read_folders(&file)
        && folders.remove(Path::new(&key))
    {
        write_folders(&file, &folders);
    }
}

/// Where Open source opens a record (criterion 7).
#[derive(Debug, Serialize)]
pub(crate) struct SourcePlace {
    /// The source key.
    source: String,
    /// The path inside the source.
    path: Vec<String>,
    /// Whether the source is the open workspace (opened through its tree), else a plain file.
    in_workspace: bool,
    /// The file's full path.
    file: String,
}

/// The document Open source opens for `record`, found by its page id (ADR 0084 §7); none when it
/// is gone.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn library_find_source(app: AppHandle, record: CameFromArgs) -> Option<SourcePlace> {
    let library = app.state::<LibraryState>();
    // The Came from line asks before the tab was ever shown: the index is read first.
    if library.index.lock().ok()?.is_none() {
        scan(&app, &library);
    }
    let index = library.index.lock().ok()?.clone()?;
    let record = CameFrom {
        document: record.document,
        page: record.page,
        page_id: record.page_id,
        item: None,
        at: curvyo_document_core::UnixMillis::new(0),
        online: None,
    };
    let found = index.source_of(&record)?;
    let sources = library.sources.lock().ok()?;
    let (source, root) = sources.iter().find(|(s, _)| s.key == found.source)?;
    let file = found
        .path
        .iter()
        .fold(root.clone(), |dir, part| dir.join(part));
    file.is_file().then(|| SourcePlace {
        source: source.key.clone(),
        path: found.path.clone(),
        in_workspace: source.kind == SourceKind::OpenWorkspace,
        file: file.display().to_string(),
    })
}

/// The record fields Open source looks up.
#[derive(Debug, Deserialize)]
pub(crate) struct CameFromArgs {
    document: String,
    page: String,
    page_id: String,
}

/// Open source for a file outside the open workspace (an added folder or another workspace,
/// criterion 7): the file goes the way of a file the system hands over, so the page runs the
/// unsaved-changes check of `0051` before it replaces the project. Only a `.curvyo` file under a
/// known source is opened.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn library_open_path(app: AppHandle, file: String) -> Result<(), String> {
    let path = PathBuf::from(&file);
    let known = app
        .state::<LibraryState>()
        .sources
        .lock()
        .is_ok_and(|s| s.iter().any(|(_, root)| path.starts_with(root)));
    let document = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("curvyo"));
    if !known || !document || !path.is_file() {
        return Err("not-found".to_owned());
    }
    crate::project::open_handed_path(&app, &path);
    Ok(())
}
