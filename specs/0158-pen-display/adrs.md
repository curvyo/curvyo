# ADRs for pen display

Architect, 2026-11-27. ADR 0094 for the input route (Frage 408 A, customer 2026-11-26), ADR 0095
for the Pencil of slice 1, ADR 0096 for the pen window of slice 2 (accepted, Frage 415 A). No new crate,
no new dependency, no `unsafe`, no document format change.

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
- [ADR 0095](../../docs/adr/0095-pencil-stroke-pipeline-pen-samples-to-a-fitted-centre-line-with-a-width-profile.md)
  (Accepted 2026-11-27, merged with PR #525):
  - `WasmSession::pen_samples(flat)` once per frame, 9-value records, beside `pointer_*`;
    `frontend/src/platform/penSamples.ts` feeds it for the Pencil, no touch (decisions 1, 2;
    criteria 10, 11);
  - own `Tool::Pencil` (key `P`), state machine `ui-core::pencil_tool`, the Pen's Freehand mode
    untouched; eraser end deletes objects within the 8 px hit rule as "Erase" (decision 3;
    criteria 1, 11, 12, 15, 16);
  - the `centre_line` piece fit moves to `geometry-core::cubic_fit`; `geometry-core::stroke_fit`
    fits with the Smoothing accuracy (floor 0.05 mm), a 60° corner rule and a width tolerance of
    5 % of Max − Min (decisions 4, 5; criteria 2, 5, 7, 9, 13);
  - `ui-core::pencil_pressure` (has-pressure rule, smoothing, Soft, Linear, Firm, Min/Max, base
    width = Max) and `ui-core::speed_pressure` (300 mm/s) (decision 6; criteria 3, 4, 8);
  - preview: the hollow outline of the raw samples; outline only after release; CPU-time budgets
    (decisions 7, 8; criteria 2, 10);
  - an ordinary path with width points, no format change (decision 9; criteria 5, 6, 14, 17).
- Session-side record (spec slice 1, "architect to do"): the session receives `PenSample`s as flat
  9-value records (ADR 0095 decision 1), decodes them in `ui-core::pen_sample`, and keeps per
  stroke only document point, time and width (`ui-core::pencil_tool::stroke`); nothing of it is
  written to the document.
- [ADR 0096](../../docs/adr/0096-pen-window-a-second-webview-with-its-own-session-kept-equal-through-a-local-bridge.md)
  (Accepted 2026-11-27, customer, Frage 415 A):
  - a second webview `pen` with its own wasm session and peer id, opened from a pack of the main
    session (decision 1; criterion 1);
  - a forwarding hub `curvyo-app::pen_window`, a pure codec `ui-core::window_bridge`, updates,
    assets, presence and view once per frame (decisions 2, 3; criteria 1, 2, 5);
  - selection shared, tool, view and drags per window, Follow view (decision 4; criteria 2, 5);
  - the main window owns path, save, unsaved state and recovery copy; undo per window
    (decisions 5, 6; criterion 1);
  - lifecycle and remembered size, position, Follow flag (decision 7; criterion 3);
  - the pen reader attaches to each webview; only main holds the relay link (decisions 8, 10).
  - amendment 2026-11-27 for 2a (criteria 1 to 20) and the hooks of 2b:
    - A2 close order: a pen close is never held; main asks first, Cancel keeps both, then pen
      closes, then main (2a criteria 10, 11);
    - A3 no reopen at start, "offered" = the command (2a criterion 16);
    - A4 `pen_window.json` format 1 in `app_config_dir()` (`ui-core::pen_window_place`, golden
      `pen_window_v1.json`); 2b format 2 adds `follow_view`, `canvas_only` (2a 16, 2b 4, 5);
    - A5 wire: `pen_window_open/close`, `bridge_watch(channel)`, `bridge_send(raw)`, hub's
      `PEER_GONE`; one bundle per frame (assets, updates with `needs`, selection, controls);
      export, import, mark; main `sync_export` after a pen import (2a 2 to 5, 8, 19);
    - A6 refusal or seq gap → reload from main's `Session::pack`, fresh pen peer id; `Hold` and
      `Release` around main's fate dialogs; `Save` (2a 9, 12, 13, 18);
    - A7 relay: pen-peer ops pass the checks (read in code, local-relay test), `pen_peer` in
      presence; pressure strokes are not shared at all yet (extended path history) (2a 17);
    - A8 main-bound commands refused for the pen page by `curvyo-app::window_commands`, built in
      2a (2b criterion 7);
    - A9 pen reader per window label (2a 7); A10 module layout; A11 build order.

## Notes for the product owner and UX

- Slice 0 criterion 1's "pointer type pen" comes from the native sample on Linux; WebKitGTK's own
  Pointer Event may say "mouse" for the pen. The probe should show both columns (criterion 5).
- Decision 8's targets (at least 120 samples per second, median delay at most one frame) are
  Proposals; the product owner may put them into criterion 5.
- ADR 0095 consequences: the fit keeps samples within the Smoothing value (default 0.4 mm), and the
  preview shows the raw samples. So criterion 2's 0.1 mm and criterion 10's 0.15 mm hold only at a
  Smoothing of at most 0.1 mm. Suggested: criterion 2 states "with Smoothing 0.1 mm", and
  criterion 10 reads "within the Smoothing value plus 5 % of Max − Min of the last preview".
  Alternatively the preview shows the fitted curve live, which is slice 3.
- ADR 0096 decision 4: a stroke or drag in progress shows in the other window only once committed;
  the 200 ms of 2a criterion 2 counts from the release.
- 2a criterion 17 (2026-11-27, ADR 0096 A12): the link carries width profiles (unshared
  projects: pressure strokes appear in the other window with their profile). In a shared project
  ADR 0053 refuses creating a width profile; today a Pencil stroke with pressure in a shared
  project, from either window, is committed as a uniform stroke at Max width, unselected, with no
  notice, and it reaches collaborators that way. Suggested wording: "a stroke drawn in the pen
  window reaches a collaborator through the main window's next sync export, as the same stroke
  drawn in the main window would (in a shared project without its width profile, ADR 0053)".
  The silent drop without a notice is a slice-1 defect to decide.
- 2b criterion 7's allow-list is built in 2a (ADR 0096 A8): the pen page exists there, and the
  main page's start (`reset_project`, `set_dirty`) would otherwise change the main window's
  project state. Suggested: 2a gets a criterion "the pen page's host calls outside the allow-list
  are refused"; 2b criterion 7 checks the Pen layout against it.
- 2a criterion 10, crash: detected on Linux through WebKitGTK's `web-process-terminated`; on
  Windows and macOS only the window's destruction is seen (out of scope for tuning in 2a).
