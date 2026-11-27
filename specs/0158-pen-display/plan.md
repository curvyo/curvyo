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
