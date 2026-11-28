//! Which window may call which host command (`specs/0158-pen-display` slice 2a criterion 21,
//! ADR 0096 A8): the main window all of them, the pen window its allow-list, any other none.
//!
//! The refusal happens before the command runs, in a wrapper around the generated handler; the
//! pen page's host layer refuses the same calls again before they reach the IPC.

use tauri::Runtime;
use tauri::ipc::Invoke;

/// The answer to a refused call.
pub(crate) const REFUSED: &str = "not-available-in-this-window";

/// The commands the pen page may call: the pen reader, the bridge, its two remembered switches
/// (2b) and read-only readers the session needs at start (fonts, formats, extensions, machine
/// profiles, materials).
pub(crate) const PEN_ALLOWED: &[&str] = &[
    "pen_watch",
    "pen_unwatch",
    "pen_compression",
    "bridge_watch",
    "bridge_send",
    "pen_window_flags",
    "font_folder_list",
    "system_font_list",
    "font_bytes",
    "read_formats_file",
    "read_extensions_file",
    "list_machine_profiles",
    "read_machine_choice",
    "list_materials",
    "read_material_order",
    "read_retired_test_ids",
    "read_test_grid_choice",
    "read_material_photo",
];

/// Whether the window labelled `label` may call `command`.
pub(crate) fn allowed(label: &str, command: &str) -> bool {
    match label {
        "main" => true,
        crate::pen_window::PEN_LABEL => PEN_ALLOWED.contains(&command),
        _ => false,
    }
}

/// `handler` with every call refused that its window may not make.
pub(crate) fn guarded<R: Runtime>(
    handler: impl Fn(Invoke<R>) -> bool + Send + Sync + 'static,
) -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
    move |invoke: Invoke<R>| {
        let label = invoke.message.webview().label().to_owned();
        if !allowed(&label, invoke.message.command()) {
            invoke.resolver.reject(REFUSED);
            return true;
        }
        handler(invoke)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every command `main.rs` registers, read from its `generate_handler!` list.
    fn registered() -> Vec<String> {
        let source = include_str!("main.rs");
        let start = source.find("generate_handler![").unwrap();
        let end = start + source[start..].find(']').unwrap();
        source[start..end]
            .lines()
            .skip(1)
            .filter_map(|line| line.trim().strip_suffix(','))
            .filter_map(|path| path.rsplit("::").next())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn the_pen_window_calls_its_list_only_and_main_calls_everything() {
        let commands = registered();
        assert!(commands.len() > 100, "{commands:?}");
        for listed in PEN_ALLOWED {
            assert!(
                commands.iter().any(|c| c == listed),
                "{listed} is registered"
            );
        }
        for command in &commands {
            assert!(allowed("main", command), "{command}");
            assert_eq!(
                allowed("pen", command),
                PEN_ALLOWED.contains(&command.as_str()),
                "{command}"
            );
            assert!(!allowed("other", command), "{command}");
        }
        for refused in [
            "pick_project",
            "save_project_bytes",
            "reset_project",
            "set_dirty",
            "take_pending_open",
            "quit_app",
            "recovery_write_copy",
            "pick_job_export_path",
            "connector_send_job",
            "pen_window_open",
            "pen_window_close",
        ] {
            assert!(!allowed("pen", refused), "{refused}");
        }
    }

    /// The pen window has no capability, so it cannot listen to the events the host broadcasts.
    #[test]
    fn the_pen_window_has_no_capability() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
        for entry in std::fs::read_dir(dir).unwrap() {
            let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
            let capability: serde_json::Value = serde_json::from_str(&text).unwrap();
            let windows = capability["windows"].as_array().unwrap();
            assert!(windows.iter().all(|w| w != "pen" && w != "*"), "{text}");
        }
    }
}
