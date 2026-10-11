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

mod formats;
#[cfg(any(target_os = "macos", test))]
mod menu;
mod project;
mod state;

use std::path::PathBuf;

use tauri::Manager;

use state::AppState;

/// Whether `WEBKIT_DISABLE_DMABUF_RENDERER` still needs to be set in this
/// process's own environment. Compiled on Linux (where `main` acts on a
/// `true` result, since only `WebKitGTK` has the bug this variable works
/// around) and under `#[cfg(test)]` on every platform, so the pure
/// environment-reading logic stays plainly testable without needing a
/// Linux runner — it would otherwise be genuinely dead code in a
/// non-Linux, non-test build, since its only production caller is itself
/// Linux-only.
#[cfg(any(target_os = "linux", test))]
fn webkit_dmabuf_env_var_is_unset() -> bool {
    std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
}

/// Re-executes this same binary, replacing the current process image,
/// with `WEBKIT_DISABLE_DMABUF_RENDERER=1` set in the *child's*
/// environment via [`std::process::Command::env`] — a safe API, unlike
/// mutating the current process's own environment with
/// `std::env::set_var` (`unsafe` as of this edition, and `CLAUDE.md` §5
/// forbids new `unsafe` outside `*-core` crates without an ADR). Linux
/// only, before any webview is created.
///
/// `specs/0002-path-node-editing/adrs.md`'s canvas-perf spike measured that,
/// on this stack (`WebKitGTK` 2.52.6 via `wry`/`tao`, NVIDIA proprietary
/// driver), leaving this unset acquires a `WebGL2` context that reports
/// success but renders **zero frames** — for `wgpu` and for hand-written
/// `WebGL2` alike. The failure mode is a silent blank canvas, not an
/// error, which is exactly why this is a tested startup step rather than
/// a hope.
///
/// Never returns on success: the process image is replaced in place, so
/// the relaunched process continues exactly where this one left off
/// (same argv, same working directory), just with the variable now set.
/// Panics if the relaunch itself could not even be started — there is no
/// sane fallback, since continuing would silently reproduce the blank-
/// canvas bug this function exists to prevent.
#[cfg(target_os = "linux")]
fn relaunch_for_linux_webkit_env() -> ! {
    use std::os::unix::process::CommandExt;

    // invariant: a running process always has a resolvable path to its
    // own executable.
    #[allow(clippy::expect_used)]
    let exe = std::env::current_exe().expect("resolve the running executable's own path");
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let error = std::process::Command::new(exe)
        .args(args)
        .env("WEBKIT_DISABLE_DMABUF_RENDERER", "1")
        .exec();
    panic!("failed to relaunch with WEBKIT_DISABLE_DMABUF_RENDERER set: {error}");
}

// invariant: `.build()` only returns an `Err` for a host-level setup
// failure (e.g. the webview engine missing) that leaves the process with
// nothing useful to do; there is no caller to propagate a `Result` to
// from `main`.
#[allow(clippy::expect_used)]
fn main() {
    #[cfg(target_os = "linux")]
    if webkit_dmabuf_env_var_is_unset() {
        relaunch_for_linux_webkit_env();
    }

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            project::take_pending_open,
            project::pick_project,
            project::confirm_project_opened,
            project::reset_project,
            project::set_dirty,
            project::save_project_bytes,
            project::quit_app,
            formats::read_formats_file,
            formats::write_formats_file,
            formats::set_formats_file_aside,
            formats::import_formats_file,
            formats::export_formats_file,
        ])
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
            for url in urls {
                if let Ok(path) = url.to_file_path()
                    && path.extension().and_then(|ext| ext.to_str())
                        == Some(project::PROJECT_EXTENSION)
                {
                    project::open_handed_path(app_handle, &path);
                }
            }
        }
    });
}

#[cfg(test)]
mod webkit_dmabuf_env_var_tests {
    use super::webkit_dmabuf_env_var_is_unset;

    /// `specs/0002-path-node-editing/adrs.md`'s prerequisite note: the failure
    /// mode of leaving `WEBKIT_DISABLE_DMABUF_RENDERER` unset is a silent
    /// blank canvas, not an error, so this needs a test rather than a
    /// hope. This only pins the detection half (reading an environment
    /// variable is safe); the relaunch itself
    /// (`relaunch_for_linux_webkit_env`) replaces the current process
    /// image on success and is exercised by running the built binary, not
    /// by a unit test. Nothing else in this crate's test suite touches
    /// this specific variable.
    #[test]
    fn is_unset_by_default_in_a_fresh_test_process() {
        assert!(
            webkit_dmabuf_env_var_is_unset(),
            "WebKitGTK's DMA-BUF renderer must be disabled before any webview is created; \
             this build does that by relaunching itself with the variable set, which this \
             check decides is necessary"
        );
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
}
