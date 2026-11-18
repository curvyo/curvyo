# tools/perf: native performance harness (spec 0126 phase 0)

Measures the real desktop app (`curvyo-app`, Tauri on WebKitGTK) under Xvfb. No behaviour change, no
new crate, no new dependency (Python standard library only).

```text
python3 -I tools/perf/measure.py            # builds, runs about 3.5 minutes, prints the table
python3 -I tools/perf/measure.py --no-build # reuse the last perf build
python3 -I tools/perf/measure.py --timeout 900 # stop after 900 s (default 600)
```

When the page's `done` event does not arrive in time the runner prints `timed out before scenario X`
and reports the scenarios that ended.

Needs `xvfb-run`, `npx`, `cargo`, and a built wasm (`cd frontend && npm run build:wasm`). Raw data goes to
`target/perf/events.ndjson` (probe, one line per second per scenario) and `target/perf/procs.json`.

## How it works

1. `vite build` with `VITE_CURVYO_PERF=ws://127.0.0.1:47126` into `frontend/dist-perf/` (git-ignored),
   then `cargo build -p curvyo-app` with `TAURI_CONFIG` pointing `frontendDist` at it. The probe is
   compiled in only by that variable, so a normal build has no probe, no socket and no UI. Note: this
   overwrites `target/debug/curvyo-app` with the perf build; a normal `cargo build` restores it.
2. The runner opens a tiny WebSocket receiver (the app's CSP allows `ws:`), starts the app under
   `xvfb-run` and samples CPU (utime+stime) and RSS of the app and all its descendants (the
   `WebKitWebProcess`, plus `WebKitGPUProcess`/network processes when WebKit starts them) from `/proc`
   every second.
3. `frontend/src/lib/perfProbe.ts`, started from `useEditorSession.ts`, drives the scenarios inside the
   page with synthetic events at 60 Hz timers (no OS input timing) and sends counters every second.
   Hooks: `renderLoop` (`useEditorSession.ts`) and the `PageFrames.tsx` tick.

## Scenarios

`idle-empty` 15 s (empty document); then a generated SVG of 5 000 stroked paths is imported and fitted
(**the whale and dolphin fixtures are trace inputs, not loadable documents, so a generated page is
used**); `idle-5000` 30 s; `wheel-zoom` (5 s, Ctrl+wheel, 30 notches in, 30 out); `pan-drag` (5 s,
middle button, pointer on a 150 px circle). The `s` column is wall time: when frames are slow the timers
fall behind, so a 5 s scenario can take longer.

## Columns

`cpu %core` process-tree CPU in percent of one core over the scenario; `peak RSS` of the tree;
`built/s` render-loop ticks; `submit/s` frames the session really submitted to the GPU (its own counter
in `WasmSession::frame_state`, resize frames included; since phase 1); `changed/s` ticks whose frame
generation changed (every change of the session: document, view, selection, tool, hover, ...);
`render ms` main-thread time per tick, average and worst; `pages/s` and `pages ms` the same for the
`PageFrames` tick. `changes by` names the three source lines in `curvyo-editor-wasm` that changed the
session most often (`WasmSession::take_change_sites`): what keeps a document drawing.

## First numbers (this machine, 2026-11-18, Xvfb, no GPU)

Xvfb has no DRI3 (`MESA-EGL: Could not get DRI3 device`), so WebKit and wgpu draw in software. The
**CPU and render-ms values are therefore much higher than on a real GPU and are not comparable to the
customer's 8.5 %.** The counters are valid: the loop does full work every frame without any change.

| scenario | CPU % of a core | peak RSS MiB | built/s | submit/s | changed/s | render ms avg (max) | pages/s | pages ms avg |
|---|---|---|---|---|---|---|---|---|
| idle-empty (15 s) | 361 | 765 | 59.7 | 59.7 | 0.0 | 9.7 (39) | 59.7 | 0.04 |
| idle-5000 (30 s) | 232 | 975 | 9.8 | 9.8 | 0.0 | 100 (124) | 9.8 | 0.07 |
| wheel-zoom | 168 | 1871 | 7.9 | 7.9 | 7.8 | 115 (227) | 7.9 | 0.06 |
| pan-drag | 180 | 1435 | 10.2 | 10.2 | 9.8 | 87 (123) | 10.2 | 0.07 |

Reading: an idle document still builds and submits a frame on every animation frame (60 per second on
the empty page, `changed/s` = 0); `PageFrames` adds one `pages_json()` per frame, which costs about
0.05 ms and is not the main cost. `WebKitWebProcess` carries almost all CPU; the app process itself is
about 1 %. Zoom and pan peak RSS exceeds idle by about 0.5-0.9 GiB.

## After phase 1: idle frames skipped (same machine, 2026-11-18, Xvfb, no GPU)

Software rendering again, so **compare the ratios, not the absolute values**; on a real GPU the idle
cost is smaller in both columns.

| scenario | CPU % of a core before | after | submit/s before | after | changed/s after | render ms avg before | after |
|---|---|---|---|---|---|---|---|
| idle-empty (15 s) | 361 | 9.5 | 59.7 | 0.1 | 0.1 | 9.7 | 0.07 |
| idle-5000 (30 s) | 232 | 8.5 | 9.8 | 0.0 | 0.0 | 100 | 0.03 |
| wheel-zoom | 168 | 169 | 7.9 | 8.1 | 8.1 | 115 | 112 |
| pan-drag | 180 | 179 | 10.2 | 9.8 | 9.8 | 87 | 74 |

Idle CPU drops by a factor of about 27 to 38: an unchanged document builds and submits no frame, and the
render loop asks one `frame_state()` per animation frame (on the 5 000-path page the loop runs at 62 ticks
per second again instead of 10). Zoom and pan are unchanged, as expected: every one of their frames is a
change (phase 2 to 4). Each change is drawn twice (the repeat on the next animation frame): WebKitGTK
shows a canvas one frame late, and without the repeat the screen kept the frame before an import
(screenshots under Xvfb, both compositing modes; with the repeat the page stays right through 20 s of
idle and a pointer over the toolbar). What remains idle is the other `requestAnimationFrame` loops (`PageFrames`,
rulers, overlays; `pages/s` stays at 62) and WebKit itself.

## Not done / could not run

- No GPU: Xvfb gives software drawing only. Numbers on a real GPU need the same script on a desktop
  session (`DISPLAY` set, drop `xvfb-run`), which is not wired yet.
- `tauri-driver` needs `WebKitWebDriver`, which is missing here; not used (events are synthetic).
- No Chromium comparison or Chrome trace yet; no whale/dolphin documents (see above).
- No split of a drawn frame's time into draw list and GPU submit; render ms is the whole `renderLoop`
  tick (the submit counter and the change sites are Rust-side since phase 1).
