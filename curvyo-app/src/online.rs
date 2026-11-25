//! The commands of the online library: the page's one door to the network, GET only, on the maker's action.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::ipc::Response;
use tauri::{AppHandle, Manager};

use curvyo_library_core::{
    Failure, Millis, READY_ADDRESS, SOURCES_FILE, SourceKind, Sources, SourcesError, host_of,
    icon_svg, search_words,
};
use curvyo_ui_core::library_online::{load_failure_text, sources_newer_text};
use curvyo_ui_core::library_tree::LibraryRow;

pub(crate) use crate::online_state::OnlineState;
use crate::online_state::{AddStop, IconImport, TemplateOrigin};

fn sources_file(app: &AppHandle) -> Option<PathBuf> {
    crate::library::data_file(app, SOURCES_FILE)
}

/// The sources file: no file or a damaged one reads as no sources; a newer file is `None`, and
/// nothing is written over it (the rule of `read_recents`).
fn read_sources(app: &AppHandle) -> Option<Sources> {
    let bytes =
        sources_file(app).and_then(|file| curvyo_storage_io::read_optional(&file).ok().flatten());
    sources_of(bytes.as_deref())
}

/// What the file's bytes mean: none or damaged is no sources, a newer file is `None`.
fn sources_of(bytes: Option<&[u8]>) -> Option<Sources> {
    match bytes.map(|b| Sources::parse(&String::from_utf8_lossy(b))) {
        Some(Err(SourcesError::Newer)) => None,
        Some(Ok(sources)) => Some(sources),
        Some(Err(SourcesError::Damaged)) | None => Some(Sources::default()),
    }
}

/// The sources to show and to open: a newer file lists nothing.
pub(crate) fn listed_sources(app: &AppHandle) -> Sources {
    read_sources(app).unwrap_or_default()
}

fn write_sources(app: &AppHandle, sources: &Sources) {
    if let Some(file) = sources_file(app) {
        let _ = curvyo_storage_io::ensure_parent_dir(&file);
        let _ = curvyo_storage_io::write_atomic(&file, sources.to_text().as_bytes());
    }
}

fn now_millis() -> Millis {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .map_or(Millis(0), Millis)
}

/// The Online rows for the tree; called by `library_rows`.
pub(crate) fn rows_of(
    app: &AppHandle,
    expanded: &std::collections::HashSet<String>,
    query: &str,
) -> Vec<LibraryRow> {
    app.state::<OnlineState>()
        .rows(&listed_sources(app), expanded, query)
}

/// The bytes of a downloaded template for the preview and the import; `None` for a key that is
/// not an online source or a template not downloaded yet.
pub(crate) fn template_bytes(
    app: &AppHandle,
    source: &str,
    path: &[String],
) -> Option<Arc<Vec<u8>>> {
    source.starts_with("online:").then_some(())?;
    app.state::<OnlineState>()
        .held_file(source, &path.join("/"))
}

/// What Add source did.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AddSourceAnswer {
    /// The source is in the list under this key.
    Added { key: String },
    /// Not added, with the sentence under the Address field.
    Refused { text: String },
    /// Cancelled while the check ran; nothing was listed.
    Cancelled,
}

/// Add source (criterion 1, UX F6): the address is checked with one request, except the ready
/// Curvyo catalogue, which sends nothing until it is opened.
#[tauri::command]
pub(crate) async fn online_add(
    app: AppHandle,
    kind: SourceKind,
    address: String,
) -> AddSourceAnswer {
    tauri::async_runtime::spawn_blocking(move || add_source(&app, kind, &address))
        .await
        .unwrap_or(AddSourceAnswer::Refused {
            text: "The server did not answer.".to_owned(),
        })
}

fn add_source(app: &AppHandle, kind: SourceKind, address: &str) -> AddSourceAnswer {
    // A newer list is never overwritten: Add is refused before anything is sent.
    let Some(mut sources) = read_sources(app) else {
        return AddSourceAnswer::Refused {
            text: sources_newer_text().to_owned(),
        };
    };
    let state = app.state::<OnlineState>();
    let started = state.add_generation();
    let wanted = || state.add_generation() == started;
    match state.add(
        &state.transport,
        &mut sources,
        kind,
        address,
        now_millis(),
        &wanted,
    ) {
        Ok(key) => {
            write_sources(app, &sources);
            AddSourceAnswer::Added { key }
        }
        Err(AddStop::Refused(text)) => AddSourceAnswer::Refused { text },
        Err(AddStop::Cancelled) => AddSourceAnswer::Cancelled,
    }
}

/// Cancel of the Add source form: the check that runs is discarded when it ends, so nothing is
/// listed and nothing of it is kept.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn online_add_cancel(app: AppHandle) {
    app.state::<OnlineState>().cancel_add();
}

/// Remove: the source leaves the list and its fetched data the session.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn online_remove(app: AppHandle, key: String) -> Option<String> {
    let Some(mut sources) = read_sources(&app) else {
        return Some(sources_newer_text().to_owned());
    };
    if sources.remove(&key) {
        write_sources(&app, &sources);
        app.state::<OnlineState>().forget(&key);
    }
    None
}

/// Which part of a source the page opened.
#[derive(Debug, Deserialize)]
#[serde(tag = "what", rename_all = "snake_case")]
pub(crate) enum Open {
    /// The source's list: its index or its icon sets; `force` is Refresh.
    List { force: bool },
    /// One icon set's names.
    Set { prefix: String },
    /// One template, downloaded and read.
    Template { path: String },
}

/// Expanding a source, a set or a template, Refresh: the request of the maker's action.
#[tauri::command]
pub(crate) async fn online_open(app: AppHandle, key: String, open: Open) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<OnlineState>();
        let Some(entry) = listed_sources(&app).find(&key).cloned() else {
            return;
        };
        match open {
            Open::List { force } => {
                let fetched = state.with(&key, |h| h.catalogue.is_some() || h.sets.is_some());
                if force || !fetched {
                    state.fetch_list(&state.transport, &entry);
                }
            }
            Open::Set { prefix } => state.open_set(&state.transport, &entry, &prefix),
            Open::Template { path } => {
                let _ = state.template(&state.transport, &entry, &path);
            }
        }
    })
    .await;
}

/// What Enter in the search started: the words as they go out and the Iconify sources asked.
#[derive(Debug, Serialize)]
pub(crate) struct SearchBegun {
    words: String,
    keys: Vec<String>,
}

/// Enter in the library search, first step (criterion 10, Frage 340 B): every added Iconify
/// source is marked as searching; nothing is sent yet and an empty text starts nothing.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn online_search_begin(app: AppHandle, text: String) -> Option<SearchBegun> {
    let (words, keys) = app
        .state::<OnlineState>()
        .search_begin(&listed_sources(&app), &text)?;
    Some(SearchBegun { words, keys })
}

/// Second step: the one request to one Iconify source, answered when that source is done; its
/// announcement. A key that is no added Iconify source sends nothing.
#[tauri::command]
pub(crate) async fn online_search_source(app: AppHandle, key: String, words: String) -> String {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<OnlineState>();
        let entry = listed_sources(&app)
            .iconify()
            .find(|e| e.key() == key)
            .cloned();
        match (entry, search_words(&words)) {
            (Some(entry), Some(words)) => state.search_one(&state.transport, &entry, &words),
            _ => String::new(),
        }
    })
    .await
    .unwrap_or_default()
}

/// The thumbnail PNG of a template row; empty when it has none or it does not match its hash.
#[tauri::command]
pub(crate) async fn online_thumbnail(app: AppHandle, key: String, path: String) -> Response {
    let png = tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<OnlineState>();
        let entry = listed_sources(&app).find(&key).cloned()?;
        state.thumbnail(&state.transport, &entry, &path)
    })
    .await
    .ok()
    .flatten();
    Response::new(png.map(|p| (*p).clone()).unwrap_or_default())
}

/// The drawn SVG of the icons `names` of a set, one request for the ones not held: `[name, svg]`.
#[tauri::command]
pub(crate) async fn online_icons(
    app: AppHandle,
    key: String,
    prefix: String,
    names: Vec<String>,
) -> Vec<(String, String)> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<OnlineState>();
        let Some(entry) = listed_sources(&app).find(&key).cloned() else {
            return Vec::new();
        };
        state
            .icons(&state.transport, &entry, &prefix, &names)
            .iter()
            .map(|icon| (icon.name.clone(), icon_svg(icon)))
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// What Load answers.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum LoadAnswer {
    /// The template is held and checked; its Came from data.
    Ready(TemplateOrigin),
    /// It could not be loaded, with the notice's sentence.
    Refused { text: String },
}

/// Load: the template is downloaded if needed and checked before the page reads it (criterion 8).
#[tauri::command]
pub(crate) async fn online_load(app: AppHandle, key: String, path: String) -> LoadAnswer {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<OnlineState>();
        let Some(entry) = listed_sources(&app).find(&key).cloned() else {
            return LoadAnswer::Refused {
                text: load_failure_text("The template", Failure::Invalid),
            };
        };
        let name = state
            .template_origin(&entry, &path)
            .map_or_else(|| "The template".to_owned(), |o| o.name);
        match state.template(&state.transport, &entry, &path) {
            Ok(_) => state.template_origin(&entry, &path).map_or_else(
                || LoadAnswer::Refused {
                    text: load_failure_text(&name, Failure::Invalid),
                },
                LoadAnswer::Ready,
            ),
            Err(failure) => LoadAnswer::Refused {
                text: load_failure_text(&name, failure),
            },
        }
    })
    .await
    .unwrap_or(LoadAnswer::Refused {
        text: load_failure_text("The template", Failure::Invalid),
    })
}

/// The Came from data of a template already held (for the page rows of an expanded template).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn online_template_origin(
    app: AppHandle,
    key: String,
    path: String,
) -> Option<TemplateOrigin> {
    let entry = listed_sources(&app).find(&key).cloned()?;
    app.state::<OnlineState>().template_origin(&entry, &path)
}

/// The icon to import: its SVG and Came from data; its body is fetched if no cell fetched it yet.
#[tauri::command]
pub(crate) async fn online_icon_import(
    app: AppHandle,
    key: String,
    prefix: String,
    name: String,
) -> Option<IconImport> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<OnlineState>();
        let entry = listed_sources(&app).find(&key).cloned()?;
        state.icons(
            &state.transport,
            &entry,
            &prefix,
            std::slice::from_ref(&name),
        );
        state.icon_import(&entry, &prefix, &name)
    })
    .await
    .ok()
    .flatten()
}

/// Opens a licence address in the system browser: the maker's own act, only for an address a
/// source named (UX notes 5, Q10).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn online_open_licence(app: AppHandle, url: String) -> Result<(), String> {
    // The hosting guide's example repository is the one address it names (`specs/0152` criterion 32).
    let known = url == curvyo_ui_core::library_browser::EXAMPLE_REPOSITORY
        || app.state::<OnlineState>().knows_licence_url(&url);
    if !known || host_of(&url).is_none() {
        return Err("not-found".to_owned());
    }
    open::that_detached(&url).map_err(|error| error.to_string())
}

/// Add on the ready Curvyo catalogue's entry (criterion 1): it joins the list and sends nothing.
#[tauri::command]
pub(crate) async fn online_add_ready(app: AppHandle) -> AddSourceAnswer {
    online_add(app, SourceKind::Git, READY_ADDRESS.to_owned()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newer_sources_file_is_never_taken_for_empty() {
        assert!(
            sources_of(Some(b"format = 2\n")).is_none(),
            "Add and Remove are refused"
        );
        assert_eq!(sources_of(None).unwrap().entries().len(), 0);
        assert_eq!(sources_of(Some(b"not toml {")).unwrap().entries().len(), 0);
        let ok = sources_of(Some(b"format = 1\n")).unwrap();
        assert_eq!(ok.entries().len(), 0);
        assert!(sources_newer_text().contains("newer Curvyo"));
    }
}
