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
mod workspace;
mod workspace_around;
mod workspace_share;
mod workspace_shared;

use std::path::PathBuf;

use tauri::Manager;

use state::AppState;

/// The environment variables the relaunch sets for `WebKitGTK`, or `None` when the
/// process may go on as it is. Pure: the caller passes the environment lookup and
/// whether the NVIDIA proprietary driver is present. Compiled on Linux and under
/// `#[cfg(test)]` on every platform, so it is testable without a Linux runner.
///
/// A variable the user already set (either of the two) is never overridden: then
/// `None`. Otherwise:
/// - NVIDIA proprietary driver: `WEBKIT_DISABLE_DMABUF_RENDERER=0` and
///   `WEBKIT_DMABUF_RENDERER_FORCE_SHM=1`. The DMA-BUF renderer stays on but uses
///   shared-memory buffers instead of GBM buffers, which this driver cannot
///   create (unset: grey canvas, "Failed to create GBM buffer"). Measured on
///   Arch/X11, driver 580.178.04, `WebKitGTK` 2.54.2, `wry` 0.57: empty-page
///   pan/zoom `rAF` p50 17 ms, 0 intervals over 33 ms, 55 frames/s. With
///   `WEBKIT_DISABLE_DMABUF_RENDERER=1` the same run composites on the CPU:
///   ~16 fps, `rAF` p50 36-63 ms.
/// - Every other machine: `WEBKIT_DISABLE_DMABUF_RENDERER=1`, as before. No
///   measurement exists for AMD, Intel or nouveau (`docs/technical-debt.md`).
#[cfg(any(target_os = "linux", test))]
fn webkit_env_to_set(
    var: impl Fn(&str) -> Option<std::ffi::OsString>,
    nvidia_present: bool,
) -> Option<&'static [(&'static str, &'static str)]> {
    if var("WEBKIT_DISABLE_DMABUF_RENDERER").is_some()
        || var("WEBKIT_DMABUF_RENDERER_FORCE_SHM").is_some()
    {
        return None;
    }
    Some(if nvidia_present {
        &[
            ("WEBKIT_DISABLE_DMABUF_RENDERER", "0"),
            ("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1"),
        ]
    } else {
        &[("WEBKIT_DISABLE_DMABUF_RENDERER", "1")]
    })
}

/// Whether the NVIDIA proprietary kernel module is loaded (nouveau does not
/// create these paths).
#[cfg(target_os = "linux")]
fn nvidia_proprietary_driver_present() -> bool {
    std::path::Path::new("/proc/driver/nvidia/version").exists()
        || std::path::Path::new("/sys/module/nvidia").exists()
}

/// Re-executes this same binary, replacing the current process image,
/// with the variables of [`webkit_env_to_set`] set in the *child's*
/// environment via [`std::process::Command::env`] — a safe API, unlike
/// mutating the current process's own environment with
/// `std::env::set_var` (`unsafe` as of this edition, and `CLAUDE.md` §5
/// forbids new `unsafe` outside `*-core` crates without an ADR). Linux
/// only, before any webview is created.
///
/// `specs/0002-path-node-editing/adrs.md`'s canvas-perf spike measured that,
/// on this stack (`WebKitGTK` 2.52.6 via `wry`/`tao`, NVIDIA proprietary
/// driver), leaving the DMA-BUF variable unset acquires a `WebGL2` context that
/// reports success but renders **zero frames** — for `wgpu` and for hand-written
/// `WebGL2` alike. That stays true for the unset case; what changed is the
/// workaround on NVIDIA, see [`webkit_env_to_set`]. The failure mode is a
/// silent blank canvas, not an error, which is exactly why this is a tested
/// startup step rather than a hope.
///
/// Never returns on success: the process image is replaced in place, so
/// the relaunched process continues exactly where this one left off
/// (same argv, same working directory), just with the variables now set.
/// Panics if the relaunch itself could not even be started — there is no
/// sane fallback, since continuing would silently reproduce the blank-
/// canvas bug this function exists to prevent.
#[cfg(target_os = "linux")]
fn relaunch_for_linux_webkit_env(env: &[(&str, &str)]) -> ! {
    use std::os::unix::process::CommandExt;

    // invariant: a running process always has a resolvable path to its
    // own executable.
    #[allow(clippy::expect_used)]
    let exe = std::env::current_exe().expect("resolve the running executable's own path");
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let error = std::process::Command::new(exe)
        .args(args)
        .envs(env.iter().copied())
        .exec();
    panic!("failed to relaunch with the WebKit environment set: {error}");
}

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
    #[cfg(target_os = "linux")]
    if let Some(env) = webkit_env_to_set(
        |name| std::env::var_os(name),
        nvidia_proprietary_driver_present(),
    ) {
        relaunch_for_linux_webkit_env(env);
    }

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
mod webkit_dmabuf_env_var_tests {
    use super::webkit_env_to_set;
    use std::ffi::OsString;

    const DISABLE: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    const SHM: &str = "WEBKIT_DMABUF_RENDERER_FORCE_SHM";

    fn set(names: &'static [&'static str]) -> impl Fn(&str) -> Option<OsString> {
        move |n| names.contains(&n).then(|| OsString::from("x"))
    }

    /// The failure mode of leaving the DMA-BUF variable unset is a silent
    /// blank canvas, so the decision is pinned here; the relaunch itself
    /// replaces the process image and is exercised by running the binary.
    #[test]
    fn nvidia_with_nothing_set_uses_shared_memory_buffers() {
        assert_eq!(
            webkit_env_to_set(set(&[]), true),
            Some(&[(DISABLE, "0"), (SHM, "1")][..])
        );
    }

    #[test]
    fn nvidia_respects_a_user_set_disable_variable() {
        assert_eq!(webkit_env_to_set(set(&[DISABLE]), true), None);
    }

    #[test]
    fn nvidia_respects_a_user_set_shm_variable() {
        assert_eq!(webkit_env_to_set(set(&[SHM]), true), None);
    }

    #[test]
    fn other_machines_keep_the_dmabuf_renderer_disabled() {
        assert_eq!(
            webkit_env_to_set(set(&[]), false),
            Some(&[(DISABLE, "1")][..])
        );
    }

    #[test]
    fn user_set_both_means_no_relaunch() {
        assert_eq!(webkit_env_to_set(set(&[DISABLE, SHM]), true), None);
        assert_eq!(webkit_env_to_set(set(&[DISABLE, SHM]), false), None);
    }
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
