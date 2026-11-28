//! The pen window's hub (`specs/0158-pen-display` slice 2a, ADR 0096 decision 2, A2, A4, A5):
//! creating the pen window and forwarding bridge messages between the two webviews.
//!
//! The hub does not read, store or log the messages (criterion 19): each page registers its
//! channel under its window's label, and a message goes unchanged to the other window's channel
//! as raw bytes. The pen window's size, place, Follow view and Canvas only are remembered in
//! `pen_window.json` (2b criteria 4 and 5).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use curvyo_ui_core::pen_window_place::{
    OpeningSize, PenWindowFlags, PenWindowPlace, ScreenRect, opening, read_pen_window,
    read_pen_window_flags, write_pen_window,
};
use curvyo_ui_core::window_bridge::PEER_GONE;
use tauri::ipc::{Channel, InvokeBody, InvokeResponseBody, Request};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WindowEvent};

/// The pen window's label.
pub(crate) const PEN_LABEL: &str = "pen";
const MAIN_LABEL: &str = "main";
/// The record of the pen window's place, in the config folder.
const PLACE_FILE: &str = "pen_window.json";
/// A move or resize is written once it has settled this long (criterion 16).
const SETTLE: Duration = Duration::from_millis(500);

/// The two pages' channels, the pen window's last seen place and its two switches.
#[derive(Default)]
pub(crate) struct Hub {
    channels: Mutex<HashMap<String, Channel<InvokeResponseBody>>>,
    place: Mutex<(Option<PenWindowPlace>, u64)>,
    flags: Mutex<PenWindowFlags>,
}

impl Hub {
    /// Sends `bytes` to the page of `label`; nothing when it has no channel.
    fn forward(&self, label: &str, bytes: Vec<u8>) {
        if let Ok(mut channels) = self.channels.lock()
            && channels
                .get(label)
                .is_some_and(|c| c.send(InvokeResponseBody::Raw(bytes)).is_err())
        {
            channels.remove(label);
        }
    }

    /// The pen window is gone: the main page hears `PEER_GONE`, the pen channel is forgotten.
    fn peer_gone(&self) {
        if let Ok(mut channels) = self.channels.lock() {
            channels.remove(PEN_LABEL);
        }
        self.forward(MAIN_LABEL, PEER_GONE.to_vec());
    }
}

fn place_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(PLACE_FILE))
}

fn read_text(app: &AppHandle) -> Option<String> {
    let bytes = curvyo_storage_io::read_optional(&place_path(app)?).ok()??;
    String::from_utf8(bytes).ok()
}

/// Writes `place` with the switches the hub holds.
fn write_place(app: &AppHandle, place: &PenWindowPlace) {
    let Some(path) = place_path(app) else {
        return;
    };
    let flags = app
        .state::<Hub>()
        .flags
        .lock()
        .map(|flags| *flags)
        .unwrap_or_default();
    if curvyo_storage_io::ensure_parent_dir(&path).is_ok() {
        let _ = curvyo_storage_io::write_atomic(&path, write_pen_window(place, flags).as_bytes());
    }
}

/// The pen page's query: its role and its two remembered switches.
fn pen_page(flags: PenWindowFlags) -> String {
    format!(
        "index.html?window=pen&follow={}&canvas={}",
        u8::from(flags.follow_view),
        u8::from(flags.canvas_only)
    )
}

/// Opens the pen window, or raises, un-minimises and focuses the open one; `true` when it was
/// made now (criterion 1). Main only (the allow-list refuses it for the pen page).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // a command takes its arguments by value
pub(crate) fn pen_window_open(app: AppHandle) -> Result<bool, String> {
    if let Some(pen) = app.get_webview_window(PEN_LABEL) {
        let _ = pen.unminimize();
        let _ = pen.show();
        let _ = pen.set_focus();
        return Ok(false);
    }
    let text = read_text(&app).unwrap_or_default();
    let saved = read_pen_window(&text);
    let flags = read_pen_window_flags(&text);
    if let Ok(mut held) = app.state::<Hub>().flags.lock() {
        *held = flags;
    }
    let pen =
        tauri::WebviewWindowBuilder::new(&app, PEN_LABEL, WebviewUrl::App(pen_page(flags).into()))
            .title(pen_title(&app))
            .visible(false)
            .min_inner_size(480.0, 360.0)
            .disable_drag_drop_handler()
            .build()
            .map_err(|error| error.to_string())?;
    place_window(&app, &pen, saved.as_ref());
    crate::pen_input::watch_pen(&app, &pen);
    watch_web_process(&app, &pen);
    pen.show().map_err(|error| error.to_string())?;
    Ok(true)
}

/// Sizes and places the hidden pen window from the remembered place (A4).
fn place_window(app: &AppHandle, pen: &WebviewWindow, saved: Option<&PenWindowPlace>) {
    let screens: Vec<ScreenRect> = pen
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(screen_rect)
        .collect();
    let main_screen = app
        .get_webview_window(MAIN_LABEL)
        .and_then(|main| main.current_monitor().ok().flatten())
        .map(|monitor| screen_rect(&monitor));
    let open = opening(saved, &screens, main_screen);
    let _ = match open.size {
        OpeningSize::Logical(w, h) => pen.set_size(tauri::LogicalSize::new(w, h)),
        OpeningSize::Physical(w, h) => pen.set_size(tauri::PhysicalSize::new(w, h)),
    };
    if let Some((x, y)) = open.position {
        let _ = pen.set_position(tauri::PhysicalPosition::new(x, y));
    }
    if open.maximised {
        let _ = pen.maximize();
    }
}

fn screen_rect(monitor: &tauri::Monitor) -> ScreenRect {
    ScreenRect {
        x: monitor.position().x,
        y: monitor.position().y,
        width: monitor.size().width,
        height: monitor.size().height,
    }
}

/// The pen window's title: "Pen window — Poster.curvyo — Curvyo", never the unsaved marker
/// (UX 9.4).
pub(crate) fn pen_title(app: &AppHandle) -> String {
    let state = app.state::<crate::state::AppState>();
    let name = crate::project::project_title_name(&state);
    format!(
        "Pen window — {} — Curvyo",
        name.as_deref().unwrap_or("Untitled")
    )
}

/// Treats an ended pen web process like a closed window (criterion 10, Linux).
#[cfg(target_os = "linux")]
fn watch_web_process(app: &AppHandle, pen: &WebviewWindow) {
    use gtk::glib::prelude::ObjectExt;
    let handle = app.clone();
    let _ = pen.with_webview(move |webview| {
        webview
            .inner()
            .connect_local("web-process-terminated", false, move |_| {
                handle.state::<Hub>().peer_gone();
                if let Some(pen) = handle.get_webview_window(PEN_LABEL) {
                    let _ = pen.destroy();
                }
                None
            });
    });
}

#[cfg(not(target_os = "linux"))]
fn watch_web_process(_app: &AppHandle, _pen: &WebviewWindow) {}

/// Closes the pen window: Close of the project (criterion 12) and a main close (A2). Main only.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // a command takes its arguments by value
pub(crate) fn pen_window_close(app: AppHandle) {
    close(&app);
}

/// Destroys the pen window, if open.
pub(crate) fn close(app: &AppHandle) {
    if let Some(pen) = app.get_webview_window(PEN_LABEL) {
        let _ = pen.destroy();
    }
}

/// The pen page's Follow view and Canvas only changed: remembered with the window's place
/// (2b criteria 4, 5; ADR 0096 A4). On the pen page's allow-list.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // a command takes its arguments by value
pub(crate) fn pen_window_flags(
    app: AppHandle,
    hub: State<Hub>,
    follow_view: bool,
    canvas_only: bool,
) {
    if let Ok(mut flags) = hub.flags.lock() {
        *flags = PenWindowFlags {
            follow_view,
            canvas_only,
        };
    }
    let seen = hub.place.lock().ok().and_then(|last| last.0);
    let place = seen
        .or_else(|| {
            app.get_webview_window(PEN_LABEL)
                .and_then(|pen| current_place(&pen.as_ref().window()))
        })
        .or_else(|| read_text(&app).and_then(|text| read_pen_window(&text)));
    if let Some(place) = place {
        write_place(&app, &place);
    }
}

/// Registers the calling page's channel for the other window's messages; a new call replaces
/// the old channel.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // a command takes its arguments by value
pub(crate) fn bridge_watch(
    window: WebviewWindow,
    hub: State<Hub>,
    on_message: Channel<InvokeResponseBody>,
) {
    if let Ok(mut channels) = hub.channels.lock() {
        channels.insert(window.label().to_owned(), on_message);
    }
}

/// Forwards the raw body unchanged to the other window's channel, in call order (a synchronous
/// command); dropped when there is no other window.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // a command takes its arguments by value
pub(crate) fn bridge_send(
    request: Request<'_>,
    window: WebviewWindow,
    hub: State<Hub>,
) -> Result<(), String> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("bad-arguments".to_owned());
    };
    let other = if window.label() == MAIN_LABEL {
        PEN_LABEL
    } else {
        MAIN_LABEL
    };
    hub.forward(other, bytes.clone());
    Ok(())
}

/// The pen window's own events: a close is never held (A2); a move or resize is remembered after
/// it settles, and on close; its destruction tells the main page (A5).
pub(crate) fn on_pen_event(window: &tauri::Window, event: &WindowEvent) {
    let app = window.app_handle();
    match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
            let place = current_place(window);
            let hub = app.state::<Hub>();
            let stamp = {
                let Ok(mut last) = hub.place.lock() else {
                    return;
                };
                last.0 = place;
                last.1 += 1;
                last.1
            };
            let handle = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(SETTLE);
                let hub = handle.state::<Hub>();
                let settled = hub
                    .place
                    .lock()
                    .ok()
                    .filter(|last| last.1 == stamp)
                    .and_then(|last| last.0);
                if let Some(place) = settled {
                    write_place(&handle, &place);
                }
            });
        }
        WindowEvent::CloseRequested { .. } => {
            if let Some(place) = current_place(window) {
                write_place(app, &place);
            }
        }
        WindowEvent::Destroyed => {
            app.state::<Hub>().peer_gone();
            app.state::<crate::pen_input::PenSink>().forget(PEN_LABEL);
        }
        _ => {}
    }
}

/// The window's inner size, outer position (none where the platform reports none) and maximised
/// state, in physical pixels.
fn current_place(window: &tauri::Window) -> Option<PenWindowPlace> {
    let size = window.inner_size().ok()?;
    let position = (!no_window_positions())
        .then(|| window.outer_position().ok().map(|p| (p.x, p.y)))
        .flatten();
    Some(PenWindowPlace {
        width: size.width,
        height: size.height,
        position,
        maximised: window.is_maximized().unwrap_or(false),
    })
}

/// Whether the platform gives windows no usable position (Wayland): the place is then left to
/// the compositor and none is remembered (criterion 16).
#[cfg(target_os = "linux")]
fn no_window_positions() -> bool {
    use gtk::glib::prelude::ObjectExt;
    gtk::gdk::Display::default().is_some_and(|display| display.type_().name().contains("Wayland"))
}

#[cfg(not(target_os = "linux"))]
fn no_window_positions() -> bool {
    false
}
