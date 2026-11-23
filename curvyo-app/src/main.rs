//! The Curvyo desktop host: a Tauri 2 binary owning native file dialogs,
//! the `.curvyo` file association, the window title and close, and all
//! filesystem access, behind the narrow command/event interface of the
//! `project` module (ADR 0001 §6).
//!
//! `path-node-editing`'s own PR review narrowed this crate's job further:
//! the host does byte I/O only. The actual document lives in the
//! `WasmSession` the frontend owns (`curvyo-editor-wasm`, hosted in the
//! webview); this crate never builds or reads a
//! `curvyo_document_core::Document` itself any more. Save hands this
//! host already-packed bytes and it writes them; Open reads bytes from
//! disk and hands them to the frontend for `WasmSession::open` to parse.

// `CLAUDE.md` §5 allows unwrap/expect in tests; only production code is held
// to the stricter rule.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod connector;
mod connector_jog;
mod connector_link;
mod connector_send;
mod connector_start;
mod device_keys;
mod devices;
mod dropped_fonts;
mod extensions_file;
mod file_uris;
mod font_documents;
mod font_library;
mod fonts;
mod formats;
#[cfg(target_os = "linux")]
mod import_dialog;
#[cfg(target_os = "linux")]
mod import_preview;
mod job_export;
mod local_files;
mod machine_profiles;
mod materials;
#[cfg(any(target_os = "macos", test))]
mod menu;
#[cfg(target_os = "linux")]
mod native_file_lists;
#[cfg(target_os = "linux")]
mod preview_fonts;
#[cfg(target_os = "linux")]
mod preview_slot;
mod project;
#[cfg(target_os = "linux")]
mod raster;
mod raw_ipc;
mod state;
mod svg_export;
#[cfg(target_os = "linux")]
mod svg_preview;
mod synced_fonts;
mod workspace;
mod workspace_around;
mod workspace_share;
mod workspace_shared;

use std::path::PathBuf;

use tauri::Manager;

use state::AppState;

/// The event the connector watcher emits to the page (`0064`); named here with the other host
/// entry points, which `tests/acceptance_0051_tester.rs` reads for every event the page listens to.
pub(crate) const CONNECTOR_EVENT: &str = "connector-event";

/// Every command the page may invoke (ADR 0001 §6); one line per command, so it is long by design.
#[allow(clippy::too_many_lines)]
fn commands() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        project::take_pending_open,
        project::pick_project,
        project::pick_import_files,
        project::confirm_project_opened,
        project::reset_project,
        project::set_dirty,
        project::set_shared,
        project::save_project_bytes,
        project::quit_app,
        job_export::pick_job_export_path,
        job_export::write_job_export,
        svg_export::pick_svg_export_path,
        svg_export::write_svg_export,
        formats::read_formats_file,
        formats::write_formats_file,
        formats::set_formats_file_aside,
        formats::import_formats_file,
        formats::export_formats_file,
        extensions_file::read_extensions_file,
        extensions_file::write_extensions_file,
        machine_profiles::list_machine_profiles,
        machine_profiles::write_machine_profile,
        machine_profiles::remove_machine_profile,
        machine_profiles::read_machine_choice,
        machine_profiles::write_machine_choice,
        materials::list_materials,
        materials::write_material,
        materials::remove_material,
        materials::read_material_order,
        materials::write_material_order,
        materials::read_test_grid_choice,
        materials::write_test_grid_choice,
        materials::read_material_photo,
        materials::write_material_photo,
        materials::remove_material_photo,
        materials::read_retired_test_ids,
        materials::write_retired_test_ids,
        fonts::font_folder_list,
        fonts::system_font_list,
        fonts::font_bytes,
        fonts::pick_font_files,
        fonts::add_font_files,
        dropped_fonts::add_dropped_fonts,
        local_files::read_local_files,
        fonts::remove_font_files,
        synced_fonts::add_synced_fonts,
        synced_fonts::trash_font_files,
        font_documents::read_font_document,
        font_documents::open_font_document,
        font_library::read_font_library,
        font_library::write_font_library,
        font_library::set_font_library_aside,
        font_library::read_font_checks,
        font_library::write_font_checks,
        connector::connector_pairing,
        connector::connector_test,
        connector::connector_pair,
        connector::connector_forget,
        connector::connector_send_job,
        connector::connector_command,
        connector_jog::connector_jog_hold,
        connector_jog::connector_jog_keep,
        connector_jog::connector_jog_release,
        connector::connector_list_command,
        connector::connector_watch,
        connector::connector_unwatch,
        connector_send::connector_send_page_job,
        device_keys::read_device_keys,
        device_keys::write_document_keys,
        devices::read_device_group,
        devices::write_device_group,
        devices::remove_device_group,
        devices::device_host_name,
        devices::prepare_synced_folder,
        devices::pick_synced_folder,
        devices::register_synced_workspace,
        devices::create_synced_workspace,
        devices::find_workspace_copy,
        devices::workspace_summary,
        workspace::pick_workspace,
        workspace_around::workspace_folder_for_file,
        workspace_around::make_workspace_for_file,
        workspace::open_recent_workspace,
        workspace::recent_workspaces,
        workspace::remove_recent_workspace,
        workspace::close_workspace,
        workspace::scan_open_workspace,
        workspace::apply_workspace_op,
        workspace::read_workspace_document,
        workspace::confirm_workspace_document_opened,
        workspace::probe_workspace_document,
        workspace_share::workspace_marker_share,
        workspace_share::write_workspace_marker_share,
        workspace_share::workspace_marker_ended,
        workspace_share::end_workspace_marker_share,
        workspace_share::write_workspace_id,
        workspace_share::replace_workspace_document,
        workspace_share::write_workspace_index,
        workspace_share::pick_join_folder,
        workspace_share::start_joined_workspace,
        workspace_shared::scan_shared_workspace,
        workspace_shared::keep_workspace_document,
        workspace_shared::restore_workspace_document,
        workspace_shared::read_deleted_document,
        workspace_shared::write_deleted_document,
        workspace_shared::clear_workspace_marker_share,
    ]
}

// invariant: `.build()` only returns an `Err` for a host-level setup
// failure (e.g. the webview engine missing) that leaves the process with
// nothing useful to do; there is no caller to propagate a `Result` to
// from `main`.
#[allow(clippy::expect_used)]
fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .manage(workspace_shared::ShareCache::default())
        .manage(device_keys::OpenStore::default())
        .manage(connector::ConnectorState::default())
        .manage(fonts::FontFiles::default())
        .manage(fonts::GrantedFontPaths::default())
        .invoke_handler(commands())
        // The window's close button and Alt+F4: a project with unsaved changes holds the close and
        // the page asks; a clean one closes at once, with no round trip to the page.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && project::hold_close_for_check(window.app_handle())
            {
                api.prevent_close();
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            // WebKitGTK hides a dragged file list from the page; the host keeps it for a drop.
            #[cfg(target_os = "linux")]
            native_file_lists::watch_drags(&handle);
            // A font folder an interrupted archive add left is removed at start (`0083`
            // criterion 12).
            if let Ok(dir) = fonts::fonts_dir(&handle) {
                curvyo_storage_io::remove_add_leftovers(&dir);
            }
            // Linux and Windows have no menu bar: Tauri sets no default menu there, and none is
            // built. macOS cannot lose its bar, so it gets the small one of `menu` (criterion 39).
            #[cfg(target_os = "macos")]
            {
                app.set_menu(menu::build(&handle)?)?;
                let menu_handle = handle.clone();
                app.on_menu_event(move |_app, event| {
                    menu::dispatch(&menu_handle, event.id().as_ref());
                });
            }

            // A `.curvyo` opened via the OS file association on Linux/Windows
            // arrives as argv[1] (specs/0001-project-file-foundation, AC5).
            if let Some(path) = std::env::args().nth(1) {
                let path = PathBuf::from(path);
                if path.extension().and_then(|ext| ext.to_str()) == Some(project::PROJECT_EXTENSION)
                {
                    project::open_handed_path(&handle, &path);
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the Curvyo application");

    app.run(|app_handle, event| {
        // The last window closed or the OS asked to quit: held like the window's close button.
        if let tauri::RunEvent::ExitRequested { api, .. } = &event
            && project::hold_close_for_check(app_handle)
        {
            api.prevent_exit();
        }
        // macOS (and iOS) deliver a double-clicked `.curvyo` as an Apple Event,
        // surfaced here rather than as argv[1] — the file-association half of
        // acceptance criterion 5 that the Linux/Windows argv path above does
        // not cover. Not build-verified on real macOS hardware in this
        // sandbox; code-complete pending that verification.
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Opened { urls } = event {
            // One project is open at a time: the first project file of the event is opened and
            // the rest are ignored, so a selection of many files cannot read them all.
            let first = urls.iter().find_map(|url| {
                url.to_file_path().ok().filter(|path| {
                    path.extension().and_then(|ext| ext.to_str())
                        == Some(project::PROJECT_EXTENSION)
                })
            });
            if let Some(path) = first {
                project::open_handed_path(app_handle, &path);
            }
        }
    });
}

#[cfg(test)]
mod capability_tests {
    /// Tauri 2 denies `listen` to a page that has no capability: the host's events (a file the
    /// operating system hands over, a window close with unsaved changes) then never arrive and
    /// the page never says so. The capability file of the main window has to name both calls.
    #[test]
    fn the_main_window_may_listen_to_the_hosts_events() {
        let text = include_str!("../capabilities/default.json");
        let capability: serde_json::Value = serde_json::from_str(text).unwrap();
        let strings = |key: &str| -> Vec<String> {
            capability[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap().to_string())
                .collect()
        };
        assert!(strings("windows").contains(&"main".to_string()));
        let permissions = strings("permissions");
        for needed in ["core:event:allow-listen", "core:event:allow-unlisten"] {
            assert!(permissions.contains(&needed.to_string()), "{needed}");
        }
    }

    /// `specs/0087-image-clipboard-and-drop` criterion 19: the window leaves every drop to the
    /// page, so HTML5 drops with files and image data reach it on Linux, Windows and macOS.
    #[test]
    fn the_main_window_hands_drops_to_the_page() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let window = &config["app"]["windows"][0];
        assert_eq!(window["label"], "main");
        assert_eq!(window["dragDropEnabled"], false);
    }
}
