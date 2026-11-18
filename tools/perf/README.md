# tools/perf: native performance harness (spec 0126 phase 0)

Measures the real desktop app (`curvyo-app`, Tauri on WebKitGTK) under Xvfb. No behaviour change, no
new crate, no new dependency (Python standard library only).

```text
python3 -I tools/perf/measure.py            # builds, runs about 3.5 minutes, prints the table
python3 -I tools/perf/measure.py --no-build # reuse the last perf build
```

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
`built/s` render-loop ticks, `submit/s` ticks that called `session.render()` (equal today, phase 1
makes them differ), `changed/s` ticks whose view string (origin, scale) differed from the previous
tick (document edits are not seen; the generation counter is phase 1); `render ms` main-thread time per
tick, average and worst; `pages/s` and `pages ms` the same for the `PageFrames` tick.

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

## Not done / could not run

- No GPU: Xvfb gives software drawing only. Numbers on a real GPU need the same script on a desktop
  session (`DISPLAY` set, drop `xvfb-run`), which is not wired yet.
- `tauri-driver` needs `WebKitWebDriver`, which is missing here; not used (events are synthetic).
- No Chromium comparison or Chrome trace yet; no whale/dolphin documents (see above).
- Rust-side counters in `curvyo-editor-wasm` (draw list vs GPU submit split) are not added; render ms is
  the whole `renderLoop` tick.
