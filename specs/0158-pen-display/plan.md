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
