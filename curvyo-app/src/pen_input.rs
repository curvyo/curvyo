//! Turns the webview's GDK pen events into per-frame sample batches for the page (ADR 0094,
//! `specs/0158-pen-display` slice 0), because `WebKitGTK` drops the pen's pressure (`WebKit` bug
//! 268458) while GTK still sees it.
//!
//! The handler on the webview widget always lets the event go on, so `WebKitGTK` sees every event
//! as before; the samples stay in memory between the handler and the page's channel, nothing is
//! stored or logged (criterion 4).

use std::cell::RefCell;
use std::collections::HashMap;
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

/// Where the batches go while the probe or the Pencil watches: each page's channel, by the
/// label of its window (ADR 0096 A9), so the main page gets only the main webview's samples and
/// the pen window's page only its own.
#[derive(Default)]
pub(crate) struct PenSink(Mutex<HashMap<String, Channel<PenBatch>>>);

impl PenSink {
    fn is_watched(&self, label: &str) -> bool {
        self.0.lock().is_ok_and(|sinks| sinks.contains_key(label))
    }

    fn watch(&self, label: &str, channel: Channel<PenBatch>) {
        if let Ok(mut sinks) = self.0.lock() {
            sinks.insert(label.to_owned(), channel);
        }
    }

    /// Forgets the channel of `label`: its page stopped watching or its window is gone.
    pub(crate) fn forget(&self, label: &str) {
        if let Ok(mut sinks) = self.0.lock() {
            sinks.remove(label);
        }
    }

    /// Sends one batch to the page of `label`; a page that is gone ends its watch.
    fn send(&self, label: &str, batch: PenBatch) {
        if let Ok(mut sinks) = self.0.lock()
            && sinks
                .get(label)
                .is_some_and(|channel| channel.send(batch).is_err())
        {
            sinks.remove(label);
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

/// Makes the sink the batches go to; once, at start.
pub(crate) fn manage_sink(app: &AppHandle) {
    app.manage(PenSink::default());
}

/// Connects the pen reader to the webview of `window` (`main` at start, `pen` when it opens);
/// `WebKitGTK` still handles every event.
pub(crate) fn watch_pen(app: &AppHandle, window: &WebviewWindow) {
    let handle = app.clone();
    let label = window.label().to_owned();
    let _ = window.with_webview(move |webview| {
        let view = webview.inner();
        view.add_events(gdk::EventMask::PROXIMITY_IN_MASK | gdk::EventMask::PROXIMITY_OUT_MASK);
        let reader = Rc::new(RefCell::new(Reader::default()));
        view.connect_event(move |widget, event| {
            on_event(widget.upcast_ref(), event, &handle, &label, &reader);
            glib::Propagation::Proceed
        });
    });
}

fn on_event(
    widget: &gtk::Widget,
    event: &gdk::Event,
    app: &AppHandle,
    label: &str,
    reader: &Rc<RefCell<Reader>>,
) {
    let Some(sink) = app.try_state::<PenSink>() else {
        return;
    };
    if !sink.is_watched(label) {
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
        let label = label.to_owned();
        widget.add_tick_callback(move |_, _| tick(&reader, &app, &label));
    }
}

/// One frame: sends what the burst collected; an empty frame ends the callback.
fn tick(reader: &Rc<RefCell<Reader>>, app: &AppHandle, label: &str) -> glib::ControlFlow {
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
        sink.send(label, batch);
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

/// Starts feeding `on_batch` with the calling window's pen samples; `true` because this host
/// reads them.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // a command takes its arguments by value
pub(crate) fn pen_watch(
    window: WebviewWindow,
    sink: State<PenSink>,
    on_batch: Channel<PenBatch>,
) -> bool {
    sink.watch(window.label(), on_batch);
    true
}

/// Stops the feed and gives the webview GDK's default motion compression back.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // a command takes its arguments by value
pub(crate) fn pen_unwatch(sink: State<PenSink>, window: WebviewWindow) -> Result<(), String> {
    sink.forget(window.label());
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tauri::ipc::InvokeResponseBody;

    use super::*;

    fn counting(count: &Arc<AtomicUsize>) -> Channel<PenBatch> {
        let count = Arc::clone(count);
        Channel::new(move |_body: InvokeResponseBody| {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }

    /// `specs/0158-pen-display` slice 2a criterion 7 (ADR 0096 A9): the main webview's batches
    /// reach only the main page's channel and the pen window's only its own; a page that stops
    /// watching removes only its own channel.
    #[test]
    fn batches_reach_only_the_channel_of_their_window() {
        let (main, pen) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let sink = PenSink::default();
        sink.watch("main", counting(&main));
        sink.watch("pen", counting(&pen));
        sink.send("main", PenBatch::default());
        sink.send("main", PenBatch::default());
        sink.send("pen", PenBatch::default());
        assert_eq!(main.load(Ordering::SeqCst), 2);
        assert_eq!(pen.load(Ordering::SeqCst), 1);
        sink.forget("pen");
        assert!(sink.is_watched("main"));
        assert!(!sink.is_watched("pen"));
        sink.send("pen", PenBatch::default());
        assert_eq!(pen.load(Ordering::SeqCst), 1);
    }
}
