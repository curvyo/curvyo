# ADRs for pen display

Architect, 2026-11-27. One new ADR for the input route (Frage 408 A, customer 2026-11-26); the pen
window mechanism of slice 2 is a later ADR. No new crate, no new dependency, no `unsafe`, no
document format change.

- [ADR 0094](../../docs/adr/0094-native-pen-input-on-linux-through-gdk-events-of-the-webview.md)
  (Proposed; Accepted once slice 0 has measured it):
  - reader: `curvyo-app::pen_input` (Linux only) connects a GDK `event` handler on each
    WebKitWebView through `with_webview`, always "proceed", keeps Pen/Eraser events or events with
    a pressure axis; one mechanism for X11 and Wayland (decision 1; slice 0 criteria 1, 5);
  - GDK motion compression off on the webview's window, measured in slice 0 (decision 2);
  - sample: `curvyo-ui-core::pen_sample::PenSample` (CSS px in the viewport, `Pressure` 0 to 1,
    tilt -1 to 1, tip or eraser, hover/contact/out, barrel bits, source-clock ms) (decision 3;
    slice 0 criterion 1, slice 1 criteria 1 to 3);
  - one `PenBatch` per frame over a `tauri::ipc::Channel` from `pen_watch` (decision 4);
  - frontend: Pointer Events route the gesture, native samples are its only geometry and
    pressure; Windows, macOS and the browser build feed the same shape from Pointer Events
    (decisions 5, 6; slice 0 criteria 2, 3);
  - nothing stored or sent (decision 7; slice 0 criterion 4);
  - what slice 0 must measure and the named fallbacks (decisions 8, 9; slice 0 criterion 5).
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md): Tauri 2 and WebKitGTK stay;
  the frontend still forwards input, now with samples.
- [ADR 0053](../../docs/adr/0053-editable-curves-and-width-profiles.md): slice 1 writes pressure
  into its centre line and width profile; no new format.

## Notes for the product owner and UX

- Slice 0 criterion 1's "pointer type pen" comes from the native sample on Linux; WebKitGTK's own
  Pointer Event may say "mouse" for the pen. The probe should show both columns (criterion 5).
- Decision 8's targets (at least 120 samples per second, median delay at most one frame) are
  Proposals; the product owner may put them into criterion 5.
