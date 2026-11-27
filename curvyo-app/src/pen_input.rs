//! Turns the webview's GDK pen events into per-frame sample batches for the page (ADR 0094,
//! `specs/0158-pen-display` slice 0), because `WebKitGTK` drops the pen's pressure (`WebKit` bug
//! 268458) while GTK still sees it.
//!
//! The handler on the webview widget always lets the event go on, so `WebKitGTK` sees every event
//! as before; the samples stay in memory between the handler and the page's channel, nothing is
//! stored or logged (criterion 4).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;

use curvyo_ui_core::{
    PenBatch, PenBatcher, RawKind, RawPenEvent, RawSource, TimeUnwrap, event_age_ms,
};
use gtk::gdk::{self, AxisUse, EventType, InputSource, ModifierType};
use gtk::glib;
use gtk::prelude::*;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State, WebviewWindow};

/// The main window's label, matching `tauri.conf.json`.
const MAIN_WINDOW: &str = "main";

/// Where the batches go while the probe or the Pencil watches: the page's channel.
#[derive(Default)]
pub(crate) struct PenSink(Mutex<Option<Channel<PenBatch>>>);

impl PenSink {
    fn is_watched(&self) -> bool {
        self.0.lock().is_ok_and(|sink| sink.is_some())
    }

    /// Sends one batch; a page that is gone ends the watch.
    fn send(&self, batch: PenBatch) {
        if let Ok(mut sink) = self.0.lock()
            && sink
                .as_ref()
                .is_some_and(|channel| channel.send(batch).is_err())
        {
            *sink = None;
        }
    }
}

/// What the handler keeps between events, on the GTK thread.
#[derive(Default)]
struct Reader {
    batcher: PenBatcher,
    clock: TimeUnwrap,
    device: String,
    /// The raw time of the first sample of the burst, for its age at the tick.
    first_time: Option<u32>,
    ticking: bool,
}

/// Connects the pen reader to the main webview; `WebKitGTK` still handles every event.
pub(crate) fn watch_pen(app: &AppHandle) {
    app.manage(PenSink::default());
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let handle = app.clone();
    let _ = window.with_webview(move |webview| {
        let view = webview.inner();
        view.add_events(gdk::EventMask::PROXIMITY_IN_MASK | gdk::EventMask::PROXIMITY_OUT_MASK);
        let reader = Rc::new(RefCell::new(Reader::default()));
        view.connect_event(move |widget, event| {
            on_event(widget.upcast_ref(), event, &handle, &reader);
            glib::Propagation::Proceed
        });
    });
}

fn on_event(
    widget: &gtk::Widget,
    event: &gdk::Event,
    app: &AppHandle,
    reader: &Rc<RefCell<Reader>>,
) {
    let Some(sink) = app.try_state::<PenSink>() else {
        return;
    };
    if !sink.is_watched() {
        return;
    }
    let Some(raw) = raw_event(widget, event) else {
        return;
    };
    let mut state = reader.borrow_mut();
    let Some(sample) = raw.sample(&mut state.clock) else {
        return;
    };
    if let Some(device) = event.source_device() {
        state.device = format!(
            "{} ({:?})",
            device.name().unwrap_or_default(),
            device.source()
        );
    }
    state.first_time.get_or_insert(raw.time);
    state.batcher.push(sample);
    if !state.ticking {
        state.ticking = true;
        let reader = Rc::clone(reader);
        let app = app.clone();
        widget.add_tick_callback(move |_, _| tick(&reader, &app));
    }
}

/// One frame: sends what the burst collected; an empty frame ends the callback.
fn tick(reader: &Rc<RefCell<Reader>>, app: &AppHandle) -> glib::ControlFlow {
    let mut state = reader.borrow_mut();
    let Some(mut batch) = state.batcher.take() else {
        state.ticking = false;
        return glib::ControlFlow::Break;
    };
    batch.device.clone_from(&state.device);
    batch.age_ms = state
        .first_time
        .take()
        .and_then(|time| event_age_ms(now_ms(), time));
    if let Some(sink) = app.try_state::<PenSink>() {
        sink.send(batch);
    }
    glib::ControlFlow::Continue
}

/// The monotonic clock in ms, 32 bit, as GDK stamps events on the common backends.
fn now_ms() -> u32 {
    u32::try_from((glib::monotonic_time() / 1000).rem_euclid(1 << 32)).unwrap_or(0)
}

/// The event read into plain values, or `None` for a kind the reader does not use.
fn raw_event(widget: &gtk::Widget, event: &gdk::Event) -> Option<RawPenEvent> {
    let button = || event.button().unwrap_or(0);
    let kind = match event.event_type() {
        EventType::MotionNotify => RawKind::Motion,
        EventType::ButtonPress => RawKind::Press(button()),
        EventType::ButtonRelease => RawKind::Release(button()),
        EventType::ProximityIn => RawKind::ProximityIn,
        EventType::ProximityOut => RawKind::ProximityOut,
        _ => return None,
    };
    let source = match event.source_device().map(|device| device.source()) {
        Some(InputSource::Pen) => RawSource::Pen,
        Some(InputSource::Eraser) => RawSource::Eraser,
        _ => RawSource::Other,
    };
    let eraser_tool = event
        .device_tool()
        .is_some_and(|tool| tool.tool_type() == gdk::DeviceToolType::Eraser);
    let (root_x, root_y) = event.root_coords().or_else(|| event.coords())?;
    let (origin_x, origin_y) = widget_origin(widget)?;
    let state = event.state().unwrap_or_else(ModifierType::empty);
    let buttons_before = [
        ModifierType::BUTTON1_MASK,
        ModifierType::BUTTON2_MASK,
        ModifierType::BUTTON3_MASK,
    ]
    .iter()
    .enumerate()
    .filter(|(_, mask)| state.contains(**mask))
    .fold(0u8, |bits, (bit, _)| bits | (1 << bit));
    Some(RawPenEvent {
        kind,
        source,
        eraser_tool,
        pressure: event.axis(AxisUse::Pressure),
        tilt_x: event.axis(AxisUse::Xtilt),
        tilt_y: event.axis(AxisUse::Ytilt),
        x: root_x - origin_x,
        y: root_y - origin_y,
        time: event.time(),
        buttons_before,
    })
}

/// The widget's top-left corner in root coordinates, the space of the event's `root_coords`.
fn widget_origin(widget: &gtk::Widget) -> Option<(f64, f64)> {
    let window = widget.window()?;
    let (_, x, y) = window.origin();
    if widget.has_window() {
        return Some((f64::from(x), f64::from(y)));
    }
    let area = widget.allocation();
    Some((f64::from(x + area.x()), f64::from(y + area.y())))
}

/// Starts feeding `on_batch` with this window's pen samples; `true` because this host reads them.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // a command takes its arguments by value
pub(crate) fn pen_watch(sink: State<PenSink>, on_batch: Channel<PenBatch>) -> bool {
    if let Ok(mut current) = sink.0.lock() {
        *current = Some(on_batch);
    }
    true
}

/// Stops the feed and gives the webview GDK's default motion compression back.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // a command takes its arguments by value
pub(crate) fn pen_unwatch(sink: State<PenSink>, window: WebviewWindow) -> Result<(), String> {
    if let Ok(mut current) = sink.0.lock() {
        *current = None;
    }
    set_compression(&window, true)
}

/// Turns GDK's merging of queued motion events on the webview's window on or off (ADR 0094
/// decision 2; the probe measures the cost).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // a command takes its arguments by value
pub(crate) fn pen_compression(window: WebviewWindow, on: bool) -> Result<(), String> {
    set_compression(&window, on)
}

fn set_compression(window: &WebviewWindow, on: bool) -> Result<(), String> {
    window
        .with_webview(move |webview| {
            if let Some(gdk_window) = webview.inner().window() {
                gdk_window.set_event_compression(on);
            }
        })
        .map_err(|error| error.to_string())
}
