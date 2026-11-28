//! The pen reader's commands where no native reader exists: they say so and the page keeps the
//! Pointer Events of its webview (ADR 0094 decision 6).

use curvyo_ui_core::PenBatch;
use tauri::ipc::Channel;
use tauri::{AppHandle, WebviewWindow};

/// Nothing to keep where no native reader exists.
#[derive(Default)]
pub(crate) struct PenSink;

impl PenSink {
    /// Nothing to forget.
    pub(crate) fn forget(&self, _label: &str) {}
}

/// Nothing to manage: no sink without a reader.
pub(crate) fn manage_sink(app: &AppHandle) {
    use tauri::Manager;
    app.manage(PenSink);
}

/// No reader to connect.
pub(crate) fn watch_pen(_app: &AppHandle, _window: &WebviewWindow) {}

/// No native pen reader on this platform: `false`, the batch channel is never used.
#[tauri::command]
pub(crate) fn pen_watch(_on_batch: Channel<PenBatch>) -> bool {
    false
}

/// Nothing to stop.
#[tauri::command]
pub(crate) fn pen_unwatch() {}

/// Nothing to switch.
#[tauri::command]
pub(crate) fn pen_compression(_on: bool) {}
