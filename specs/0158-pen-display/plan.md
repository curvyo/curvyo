# Plan for Pen display, slice 0 "Pen probe"

Implementer, 2026-11-27. Slice 0 only (criteria 1 to 5 of `specification.md`, ADR 0094 decisions 1 to 4, 6 to 8). No Pencil, no
wasm pointer API change, no document change, no format change, no new crate or dependency.

## Affected crates/modules
- `curvyo-ui-core::pen_sample` (new): `PenSample`, `Pressure`, `Tilt`, `PenEnd`, `PenPhase`, the raw-event record and its
  pen filter (`RawPenEvent::is_pen`, `sample`), `TimeUnwrap`, `event_age_ms`, `PenBatcher`/`PenBatch`; `ScreenPx` made public.
- `curvyo-app/src/pen_input.rs` (Linux; `pen_input_none.rs` elsewhere): the GDK reader, the per-frame tick, commands
  `pen_watch`, `pen_unwatch`, `pen_compression`.
- Frontend: `lib/penProbeStats.ts` and `lib/penProbe.ts` (pure statistics, session, report), `lib/penProbeStore.ts`,
  `hooks/usePenProbe.ts`, `components/PenProbe.tsx`, one button in `components/panel/FileTab.tsx`, `platform/host.ts`
  (`messageChannel`).

## Tasks
- [x] 1. Sample types, clamping and normalisation helpers, tests first (criterion 1, 5)
- [x] 2. Pen-or-pressure-axis filter, phase and barrel bits, 32 bit time unwrap, event age as pure functions with tests (1, 5)
- [x] 3. Per-frame batcher with the 256 limit and drop count, tests first (1, 5)
- [x] 4. GDK reader, tick sender, `pen_watch`/`pen_unwatch`/`pen_compression`; other platforms answer "not available" (1, 2, 4, 5)
- [x] 5. Statistics: median, rate window, pairing of webview and native events, session snapshot, report text, tests first (1, 3, 5)
- [x] 6. Probe page with both columns, delay and position measurements, compression switch with frame time, scratch trace, Copy
  report (1, 2, 3, 4, 5)
- [x] 7. Entry point and gate (all)

## Decisions
- **(Proposal)** The page is reached from the File tab of the Properties panel, last button above the version line ("Pen probe…");
  it opens as a dialog over the whole window (Escape or Close leaves it). Least intrusive: no new tab, no menu, no shortcut, and
  the File tab already holds the other secondary entries (Credits, Extensions). The slice is removed or moved when the pen window
  lands (slice 2).
- The webview pointer type is read from Pointer Events; the native column never shows a mouse (the reader ignores non-pen events),
  so with a mouse the webview column says "mouse" and the native column says no pen sample yet.
- Delay: the page stamps each webview Pointer Event (and its coalesced positions) and each native sample at arrival and pairs
  the nearest equal position within 3 px and 300 ms; delay is native minus webview, so a negative value means native first.
- The reader and its batches run only between `pen_watch` and `pen_unwatch` (page open); `pen_unwatch` also restores GDK motion
  compression. The batch carries the device name and the app's age of the oldest sample for the probe only; nothing is stored.
- Deviation from ADR 0094 decision 3: the field is `end` (`tip`/`eraser`) of type `PenEnd`, because `PenTool` is already the
  pen-drawing tool of `curvyo-ui-core`.

## Validation
- Unit tests: `curvyo-ui-core` `pen_sample` (normalisation, filter, phase, barrel, time wrap, age, batching),
  `frontend/tests/penProbeStats.test.ts` (median, windows, pairing, session, report).
- Gate for the affected crates and the frontend part of CLAUDE.md section 7; the app builds and starts.
- By hand on the customer's Kamvas 13 (X11 and Wayland): ADR 0094 decision 8 (a) to (g). The page's "Copy report" gives the numbers.

# Plan for Pen display, slice 1 "Pressure Pencil"

Implementer, 2026-11-27. Slice 1 only (criteria 1 to 17 of `specification.md`), binding design ADR 0095, UX notes
`ux-notes.md`. Built "dry": no tablet, every behaviour is fed as `PenSample` batches (pure core tests, session tests,
frontend tests with synthetic Pointer Events). No new crate, no new dependency, no format change.

## Affected crates/modules
- `curvyo-geometry-core`: `cubic_fit` (+ `cubic_fit/safe_fit.rs`), moved unchanged out of `centre_line/fit.rs` and
  `centre_line/safe_fit.rs`; `centre_line/profile.rs` takes the per-sample width bound from its caller; new public
  `stroke_fit` (`StrokeSample`, `FittedStroke`, `fit_stroke`).
- `curvyo-ui-core`: `pen_sample` gains `PEN_SAMPLE_STRIDE` and `PenSample::decode_flat` (in `pen_sample/flat.rs`);
  new `pencil_pressure`, `speed_pressure`, `pencil_tool` (+ `pencil_tool/stroke.rs`); history name "Pencil stroke"
  and "Erase", step scopes.
- `curvyo-render-core`: new `pencil_preview` (hollow outline of the raw samples, or the 1.5 px line).
- `curvyo-editor-wasm`: `Tool::Pencil`, key `P`, `session/pencil.rs` (routing, commit, erase, hover set),
  `wasm_pencil.rs` (`pen_samples`, `pen_sample_stride`, Pencil settings), `tests/pencil_cpu_time.rs`, session tests.
- Frontend: `platform/penSamples.ts` (Pointer Events with coalesced events and native `PenBatch`es to flat records),
  `hooks/usePencilFeed.ts`, `components/PencilBar.tsx`, `lib/pencilSettings.ts`, the rail button and the column-A
  scroll rule in `ToolRail.tsx`, the bar in `App.tsx`.

## Tasks
- [x] 1. Move the piece fit and the kurbo guard to `geometry-core::cubic_fit`; `profile.rs` takes the width bound from
  its caller; every centre-line test and golden unchanged (16, regression guard)
- [x] 2. `stroke_fit::fit_stroke` tests first: accuracy at every sample, floor 0.05 mm, node budget on the S-curve, an L
  keeps one corner, monotone node count over Smoothing, profile within 5 % of Max − Min (2, 5, 9, 10)
- [x] 3. `decode_flat` and stride, tests first (11)
- [x] 4. `pencil_pressure` (has-pressure rule, trailing mean, curves, Min/Max) and `speed_pressure`, tests first
  with criterion 3's numbers (3, 4, 7, 8)
- [x] 5. `pencil_tool` state machine: stroke over batch borders, hover, out, eraser, Escape, too short, tests first
  (1, 11, 12, 15)
- [x] 6. `render-core::pencil_preview` (10)
- [x] 7. Session: `Tool::Pencil`, key `P`, `pen_samples`, commit "Pencil stroke" (style, layer, fill None, Max as
  stroke width, profile), layer refusal, erase "Erase" with the hover set, Escape cascade, settings; session tests
  that feed batches (1, 5, 6, 7, 11, 12, 13, 14, 15, 17)
- [x] 8. CPU-time tests: preview frame at 2000 samples < 8 ms, release < 200 ms (2, 10)
- [x] 9. Frontend: `penSamples.ts` with tests (synthetic Pointer Events, coalesced, touch ignored, native batches),
  feed hook, Pencil bar, rail button, column-A scroll rule (10, 11, UX)
- [x] 10. Docs (ADR 0095 notes, spec status line untouched by the implementer), full gate (all)

## Decisions
- **(Proposal, lead)** Smoothing is 0 to 100 % as Freehand's slider and maps the same way: value / 100 = fit
  accuracy in mm (40 % = 0.4 mm), floored at 0.05 mm (ADR 0095 decision 5).
- **(Proposal, lead)** Column A's "Too short" rule is built minimal: the cards below the Tools card sit in one
  `overflow-y: auto` box with a thin scrollbar; focus scrolls a button into view (browser default).
- **(Proposal, lead)** The eraser highlight is a set of hover boxes: the session passes the eraser's set into the
  decoration's existing list of hover boxes (`history_hovered`), so no new render kind is needed.
- **(Lead)** Moving the fitter must leave every centre-line golden byte-identical; the tests and their
  `proptest-regressions` stay where they are and call the moved functions.
- **(Lead, Frage 414 A)** The Pen's Freehand mode, `pen_tool/freehand.rs` and their tests are not touched (16).
- The mouse sends no pressure (NaN) from the Pointer Event route, so a mouse stroke never has pressure even where a
  browser reports 0.5 (7); a pen sends `event.pressure`.
- "Pen seen" and "last pen stroke had no pressure" (the bar's dimming and "No pen pressure" state) are tracked by the
  frontend from `pointerType` / native samples and the session's answer for the last stroke; the session settings
  (Smoothing, Simulate, Min, Max, Curve) live in the session (UX §8).
- Speed for Simulate pressure is the backward difference to the previous sample (the first sample takes the second's
  speed once it arrives), so widths are computed on arrival (ADR 0095 decision 4).

## Deviations from ADR 0095 and the UX notes (for the review)
- `stroke_fit` does not call the piece fit once: each corner-free run is fitted, measured against every
  sample, refitted at half the accuracy and, failing that, split at its worst sample, so every sample lies
  within the accuracy (kurbo's own error measure is not a bound). To keep criterion 9's "never more nodes",
  the run is fitted at every Smoothing percent from the asked value down to half of it and the fewest nodes
  win (a ladder, about 56 ms of thread CPU for criterion 2's stroke in a release build). Monotone on the
  tested strokes for every percent, not proven in general.
- `FittedStroke.width` is the widest sampled width; the session rescales the multipliers so the path's
  stroke width is Max (decision 6).
- `centre_line/fit.rs` is 521 lines after the move (its piece tests stay in it with their
  `proptest-regressions` file), not under 500.
- The eraser's hover set reuses the decoration's list of hover boxes (`history_hovered`), so no new
  render kind; the eraser deletes the hit objects as the lasso's hit test returns them (leaves of the
  current context, not whole groups at the root).
- The Pencil bar's dimmed reason is a `title` and `aria-description`, not a Radix tooltip line.
- Column A's scroll box has no pointer events of its own (wheel and focus scroll it; its thin scrollbar
  cannot be dragged), so the canvas beside the rail keeps its clicks.
- The native route: the feed calls `pen_watch` while the Pencil is active; the Pen probe's `pen_unwatch`
  stops it until the Pencil is chosen again (not handled).

## Validation
- Unit tests in `geometry-core::stroke_fit`, `ui-core::{pen_sample, pencil_pressure, speed_pressure, pencil_tool}`,
  `render-core::pencil_preview`; session tests `curvyo-editor-wasm/tests/pencil_0158.rs`; CPU budgets
  `curvyo-editor-wasm/tests/pencil_cpu_time.rs` (thread CPU time, release thresholds, debug x12 as elsewhere).
- Frontend: `frontend/tests/penSamples.test.ts`.
- Not verifiable here: a real pen (Kamvas 13), the native GDK route end to end, the eraser end.

# Plan for Pen display, slice 2a "Pen window"

Implementer, 2026-11-27. Part 2a only (criteria 1 to 26 of `specification.md`), binding design ADR 0096 with its amendment
A1 to A12, UX notes section 9, the design-system rows "Pen window", "Pen rail", "Pen window states", "Pen display
section", "Pen window steps". Nothing of 2b (no Pen layout strip, no Eraser button, no Follow view, no frame in the main
window). No new crate, no new dependency, no `unsafe`, no project format change. Built "dry": the testable core is two
`Session`s and a byte shuttle in one process; real windows are checked by hand.

## Affected crates/modules
- `curvyo-ui-core`: new `window_bridge` (bundle codec, `PEER_GONE`), new `pen_window_place` (`pen_window.json` format 1,
  placement), `presence::PresencePayload.pen_peer`; golden `tests/fixtures/pen_window/pen_window_v1.json`.
- `curvyo-editor-wasm`: new `session/window_link.rs` (+ `window_link/incoming.rs`), `session/window_link_tests.rs`,
  `wasm_window_link.rs`; History names (`sync_presence.rs`), presence hello, the Pencil's shared-project rule
  (`pencil.rs`, criterion 23), text edit leave on a link delete.
- `curvyo-app`: new `window_commands.rs` (allow-list and the refusing handler), new `pen_window.rs` (hub: open, close,
  bridge channels, placement file, `PEER_GONE`), `pen_input.rs` per window label, `main.rs` window events and close order.
- Frontend: `platform/host.ts` (`windowRole`, `openPenWindow`, `closePenWindow`, `bridge`, pen guard), `main.tsx` role switch,
  new `pen/PenApp.tsx` and `pen/PenRail.tsx`, new `hooks/useWindowLink.ts`, the File tab's Pen display section, the
  frame hook in `useEditorSession`.

## Tasks
- [x] 1. `window_bridge` codec, tests first: round trip, order, version and truncation refusals, `PEER_GONE` (2, 3, 4, 18)
- [x] 2. `pen_window_place`, tests first: golden read/write byte-equal, unknown field, damaged, newer, defaults
  1200 x 720, off-screen centring, Wayland null position, maximised (16)
- [x] 3. `PresencePayload.pen_peer` (17)
- [x] 4. `window_link` two-session tests first, then the link: export/import per frame, assets once and waiting,
  selection with field hold, generation and seq, refusal and reload, hold, save, shared state, New/Open (2 to 6, 8, 9,
  12 to 15, 18, 19, 24 to 26)
- [x] 5. History names "Pen window" and "<label> (pen window)", pen peer in presence; local-relay test (6, 17)
- [x] 6. Pencil in a shared project: uniform at Max, selected, notice once per session (23)
- [x] 7. `wasm_window_link` bindings (`link_start`, `link_receive`, `link_frame`, `link_hold`, `link_state`,
  `link_take_project`, `link_take_save`, `link_continue`) (1, 9, 12, 13, 18)
- [x] 8. `window_commands` allow-list with its test, the refusing handler, no capability for `pen` (21)
- [x] 9. `pen_input` per label with the routing test (7)
- [x] 10. Hub `pen_window`: open/focus, placement file read/write after 500 ms and on close, bridge forwarding as raw
  bytes, `PEER_GONE` on destroy and web-process end, close order of A2 (1, 10, 11, 16, 19)
- [x] 11. Frontend: role, guard, `PenApp` with the pen rail, `useWindowLink`, main's button and Hold/Save/New/Open/Close
  wiring, vitest for guard and shuttle (1, 9, 11, 12, 13, 21, 22)
- [ ] 12. First run under `xvfb-run`, measurements of criterion 20 where possible, docs (1, 20)

## By hand (needs real windows or the customer's machine)
- Criterion 1: the button's labels, the 3 s and 10 s limits, raising an open window, a second press opening nothing.
- Criterion 10, 11: window-manager close, crash of the pen web process, the close order with the unsaved check.
- Criterion 16: placement on X11 and on Wayland, maximised state, a position off every screen.
- Criterion 20: memory and the two timings on the customer's machine with the 10,000-object and 200 MB projects.
- Criterion 21: the wrapper around `generate_handler!` refusing a real call from the pen page (its rule is unit tested).
- Criterion 22: the rail's sizes, names and states in the real pen window.

## Decisions
- **(Proposal)** The bundle header carries a project generation beside the sequence number: the main window raises it
  with every project it sends, so bundles of a replaced replica (a reload, New, Open) are dropped by generation and a
  `seq` gap is only checked within one generation. Only `NeedProject` and `Save` of an old generation still count.
- **(Proposal)** A control `Peer(u64)` (pen to main, once per project) tells the main window the pen replica's peer id:
  the History name "Pen window" and the presence field `pen_peer` need it, and a reload gives the pen a fresh id.
- **(Proposal)** The pen window opens each `Project` through the page's normal Open path (a fresh `WasmSession` with a
  fresh peer id); `link_continue(old)` moves the link and keeps the tool (the Pencil on the first open), the Pencil
  settings and, after a reload, the view; otherwise it fits the page. The main window's New and Open go through the same
  call in `attachSession`, which then sends the new project.
- **(Proposal)** A text under edit in the receiving window holds an arriving selection like a busy field (criterion 15:
  "nothing changes in the edit").
- **(Proposal)** The pen page's allow-list adds the read-only readers the session reads at start (formats, extensions,
  machine profiles, materials and their photos) to A8's list; the app and the page share one list (a vitest compares).
- The pen window's Delete button deletes the Pencil's selection as Select does (pen window only; the main window's
  Pencil is unchanged).
- `pen_window.json` keeps `maximised` and defaults to 1200 x 720 logical px (spec criterion 16 and UX 9.6, over A4's
  960 x 600 and its field list); positions are inner size plus outer position, `null` on Wayland.

## Deviations from ADR 0096 and the UX notes (for the review)
- History name of a collaborator's pen step: "<label> (pen window)" (A7), not UX 9.6's "Pen window of Marc".
- Bundle header: generation added (A5 names `seq` only); control `Peer` added.
- The Hold is sent from a React effect when the File tab's check or the host's Open or Save-as dialog is up, so it
  reaches the pen window in the frame after the check appears, not strictly before it.
- The pen window's first frame is approximated by its first bundle (its first animation frame sends `NeedProject`).

## Validation (2a)
- Unit tests: `curvyo-ui-core` `window_bridge`, `pen_window_place`, `presence`; golden `tests/pen_window_place_golden.rs`.
- Session tests: `curvyo-editor-wasm/src/session/window_link_tests.rs` and `window_link_share_tests.rs` (two sessions and
  a byte shuttle; sharing through the in-process relay of the sharing tests).
- `curvyo-app`: `window_commands` (allow-list, no capability for `pen`), `pen_input` routing test.
- Frontend: `frontend/tests/penWindow.test.ts` (role, guard list equal to the app's, keys, shuttle).

# Plan for Pen display, slice 2b "Pen layout and Follow view"

Implementer, 2026-11-28. Part 2b only (criteria 1 to 9 of `specification.md` part 2b), on the branch of 2a
(`story/pen-window`, customer decision 2026-11-28: 2a and 2b in one PR). Binding: ADR 0096 with A4 (format 2) and
A5 (`View`), UX notes sections 10 and 11, the design-system rows "Pen rail", "Pen strip", "Pen layer pick", "Pen window
frame", "Pen display section". Nothing of slices 3 to 6, no layout configuration (Frage 413 A), no mirror switch. No new
crate, no new dependency, no `unsafe`, no project format change.

## Affected crates/modules
- `curvyo-ui-core`: `window_bridge` gains `Control::View(ViewMessage)`; `pen_window_place` reads format 1 and 2 and writes
  2 (`PenWindowFlags`: `follow_view`, `canvas_only`); golden `tests/fixtures/pen_window/pen_window_v2.json`, the "newer"
  fixture moves to format 3.
- `curvyo-editor-wasm`: new `session/window_link/view.rs` (Follow view and the pen view for the frame), new
  `session/window_link_view_tests.rs`; `session/pencil.rs` eraser mode; `session/pen_layers.rs` (the strip's layer list and
  pick); the selection-on-open fix in `window_link.rs`; bindings in `wasm_window_link.rs` and `wasm_pencil.rs`.
- `curvyo-app`: `pen_window.rs` (`pen_window_flags`, flags in the pen page's URL, the record written with both),
  `window_commands.rs` allow-list.
- Frontend: `pen/PenApp.tsx`, `pen/PenRail.tsx`, new `pen/PenStrip.tsx`, `pen/PenLayerPick.tsx`, `pen/penLayout.ts` (pure:
  columns, touch rule, the flags from the query, the host calls of the layout), `components/NodeToolbar.tsx` (48 px pen
  subset), new `components/PenViewFrame.tsx`, `lib/penWindowStore.ts` ("Show pen view"), `components/PenProbe.tsx` (the
  switch), `hooks/useWindowLink.ts`, `lib/windowRole.ts`.

## Tasks
- [x] 1. `window_bridge` `View` item, tests first: round trip, cut (5, 6)
- [x] 2. `pen_window_place` format 2, tests first: golden v2 byte-equal, format 1 reads as both false, format 3 none (4, 5)
- [x] 3. Two-session tests first, then Follow view: off keeps views apart; on takes main's view at once; 20 alternating pans
  and zooms end equal and every change crosses once (no echo); simultaneous changes settle on main's; off keeps both;
  the pen view rectangle in main equals the pen's visible rectangle within 0.01 mm after pan, zoom and resize; no frame
  without a pen window; view never a step and never in the document (5, 6)
- [x] 4. Selection on open: the main window's selection is in the pen window when it opens (lead, 2a observation)
- [x] 5. Eraser mode, test first: the same samples through the mode and as eraser-end samples give the same document and
  one "Erase" step; off draws again (2)
- [x] 6. Pen layer list and pick, tests first: front-most first, hidden/locked listed, pick sets the active layer with no
  step, a selection moves it, no layers reads none (3)
- [x] 7. wasm bindings (2, 3, 5, 6)
- [x] 8. Hub: `pen_window_flags`, flags in the URL, both written with the place; allow-list + its test (4, 5, 7)
- [x] 9. Frontend: Pen layout (rail with Eraser and Follow view, strip, Node bar at 48 px, Canvas only toggle, second
  column under 640 px, touch presses ignored), flags written on change; tests of the pure parts and the host calls of every
  control against the allow-list (1, 3, 4, 7, 8, 9)
- [x] 10. Main window: frame overlay and "Show pen view" switch (6)
- [ ] 11. Docs and checks (all): plan and deviations written; first run of the real windows not done (by hand)

## Decisions (2b)
- **(Proposal, lead)** The Eraser is a Pencil mode (`set_pencil_eraser`): every sample of the tip (and of a mouse) is fed
  to the Pencil as an eraser-end sample, so the hit rule, hover set and the "Erase" step are slice 1's code path. Not
  remembered; a new pen window starts with the Pencil. Choosing any tool ends the mode.
- **(Proposal, lead)** Follow view off by default; view state is ephemeral (never in the document, never a step).
- **(Proposal)** `View` carries the sender's zoom and centre, its visible rectangle (document mm), the Follow flag, whether
  the view moved by this window's own pan or zoom, and two counters (views sent, last view seen from the other side). A
  received view is applied only when it moved by the other's own hand and Follow is on, and the applied view becomes the
  known one, so it is never sent back (no echo). Two changes that cross settle on the main window's: the main window drops
  a pen view that had not seen its last one, the pen window applies main's.
- **(Proposal)** Turning Follow on sends the flag without a move; the main window answers with its view, which the pen
  window takes. A new project (New, Open, reload) starts the view exchange over in the new generation.
- **(Proposal)** The pen window's visible rectangle is its whole canvas (the layout floats over it, UX 10.1); the main
  window converts it to its screen for the frame each frame, so its own pan moves the frame with the artwork.
- **(Proposal)** The flags reach the pen page in its URL (`?window=pen&follow=1&canvas=1`), so the pen page needs no
  reader command; it writes them with `pen_window_flags`.
- **(Proposal)** Layer pick reads the active page's layers from the session (`pen_layers_json`) and sets the session's
  active layer (`set_pen_layer`), as a Layers row press does but keeping the selection.

## Deviations from ADR 0096 and the UX notes (2b, for the review)
- `View` carries two counters and a "moved" flag beside zoom, centre, rectangle and Follow (A5 names `View` only); the
  crossing rule (main wins) goes beyond the edge case's "last message wins per window" so both windows end equal.
- The pen page gets its switches in the URL instead of a reader command; `pen_window_flags` writes them (A4).
- The 2a tester's golden write test (`acceptance_0158_slice2a_tester.rs` c16) now expects format 2 with both switches
  off and reads the format-1 text as before: the format change of 2b criterion 4 makes its old expectation untrue.
- Max width and Smoothing are the shared value fields at 48 px (`tall`); their gestures are the fields' own (a press
  sets the value, a drag moves it, double-click or Enter types), not UX 10.3's "drag with 8 px slop, tap to type".
- The Node bar's pen subset is the Node bar with 48 px buttons and without Delete node and Close path; the Edit width
  toggle is 48 px high. Tooltips are `title` attributes, not Radix tooltips with the 400 ms pen delay.
- `--selection-casing` is not a CSS token in `index.css` (only a Rust constant); the frame uses `#FFFFFF` as its
  fallback. `--pen-view-frame` is added to `index.css` as the alias of `--toolbar-icon`.
- The 2a `PenApp` Escape-key path and the P key keep the Eraser mode while the Pencil is the tool (the Eraser is the
  Pencil's mode); choosing a tool on the rail always sets it.

## By hand (2b)
- Criterion 1, 8, 9: sizes, overlap and focus rings in the real pen window at 1920 x 1080 and 1366 x 768; touch presses.
- Criterion 4, 5: the switches surviving a close and reopen of the pen window (`pen_window.json`).
- Criterion 6: the frame's look and that it updates in the frame the view arrives, in two real windows.

## Validation (2b)
- `curvyo-ui-core`: `window_bridge` tests, `pen_window_place` tests, golden `tests/pen_window_place_golden.rs` (v2 byte-equal,
  v1 reads).
- `curvyo-editor-wasm`: `session/window_link_view_tests.rs` (criteria 5, 6, selection on open), `session/pen_layers.rs`
  tests (3), `tests/pencil_0158.rs` `slice_2b_criterion_2_*` (2).
- `curvyo-app`: `window_commands` allow-list test (7).
- Frontend: `tests/penLayout.test.ts` (1, 2, 4, 5, 6, 7, 8), `tests/penWindow.test.ts` (allow-list equal to the app's).
