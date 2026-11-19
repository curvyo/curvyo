# tools/perf: native performance harness (spec 0126 phase 0)

Measures the real desktop app (`curvyo-app`, Tauri on WebKitGTK) under Xvfb. No behaviour change, no
new crate, no new dependency (Python standard library only).

```text
python3 -I tools/perf/measure.py            # builds, runs about 3.5 minutes, prints the table
python3 -I tools/perf/measure.py --no-build # reuse the last perf build
python3 -I tools/perf/measure.py --timeout 900 # stop after 900 s (default 600)
python3 -I tools/perf/measure.py --desktop     # on the real display and GPU instead of Xvfb
```

```text
python3 -I tools/perf/measure.py --ablate                  # the ablation matrix (below), about 5 minutes
python3 -I tools/perf/measure.py --desktop --ablate        # the same on the real display and GPU
python3 -I tools/perf/measure.py --no-build --ablate baseline,dpr1,no-rulers   # chosen variants only
```

```text
python3 -I tools/perf/measure.py --gl-bench                # GL micro benchmarks (below), one launch, about 3.5 minutes
python3 -I tools/perf/measure.py --desktop --gl-bench --timeout 1200   # the same on the real display and GPU
```

`--desktop` starts the app on the current `DISPLAY` (no `xvfb-run`, no forced `GDK_BACKEND`): the
numbers of a real GPU and the desktop compositor's pacing. It takes no screenshots (the window is
wherever the window manager puts it), so the `lag` line is not measured there.

When the page's `done` event does not arrive in time the runner prints `timed out before scenario X`
and reports the scenarios that ended.

Needs `xvfb-run`, `npx`, `cargo` and ImageMagick (`import`, `identify`; without them the screenshot
check is skipped). Raw data goes to `target/perf/events.ndjson` (probe, one line per second per
scenario), `target/perf/procs.json` and `target/perf/<scenario>-start.png`/`-end.png` (the canvas at
the scenario's start and end).

The build step runs `npm run build:wasm` first. `vite build` alone bundles whatever
`frontend/src/wasm-bindings` holds; bindings older than the page (before `frame_state` existed, say)
make the render loop throw on its first tick, so every column reads 0 and zoom and pan end in about
6 s instead of 35 s (seen 2026-11-18 after PR #387). `--no-build` keeps that risk.

## How it works

1. `npm run build:wasm`, then `vite build` with `VITE_CURVYO_PERF=ws://127.0.0.1:47126` into `frontend/dist-perf/` (git-ignored),
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

`idle-empty` 15 s (empty document); `empty-pan` (5 s, middle button, left and right at 8 px per
16 ms tick, each tick a burst of 8 moves of 1 px, as a 500 Hz mouse sends; two screenshots per sweep
mid-move, then one at rest); `empty-zoom` (5 s, Ctrl+wheel, each tick a burst of 4 quarter notches, 30
ticks in, 30 out); then a generated SVG of 5 000 stroked paths is imported and fitted
(**the whale and dolphin fixtures are trace inputs, not loadable documents, so a generated page is
used**); `idle-5000` 30 s; `wheel-zoom` (5 s, Ctrl+wheel, 30 notches in, 30 out); `pan-drag` (5 s,
middle button, pointer on a 150 px circle). The `s` column is wall time: when frames are slow the timers
fall behind, so a 5 s scenario can take longer.

## Ablation matrix (`--ablate`)

Which part of the page makes WebKit burn a core while the empty page pans. The runner starts the app
once per variant (nothing of one variant carries into the next), hands the page the variant over the
probe's WebSocket (its first message: `{"mode": "ablate", "variant": ...}`; the full run gets
`{"mode": "full"}`), and the page (`frontend/src/lib/perfAblate.ts`, active only in the perf build)
applies it before one continuous middle-button pan of the empty page as in `empty-pan` (no
screenshots): the first 3 s are a warm-up, the next 8 s are measured. `/proc` is sampled every
0.25 s. Variants, in run order:

| variant | what it changes |
|---|---|
| `baseline` | nothing |
| `no-rulers` | ruler strips' canvases `display: none`, their cells `visibility: hidden` (the grid keeps its cells, so the canvas keeps its size; the ruler JS still runs) |
| `no-pageframes` | the `PageFrames` layer (`[data-page-frames]`) `display: none` |
| `no-overlays` | every `position: absolute/fixed` element over the canvas that is not its ancestor `display: none` (page frames, placed-page tags, fixture labels, presence, measure, tool bars, rail) |
| `canvas-only` | everything but the canvas `visibility: hidden` (not `display: none`: the canvas keeps its size and backing store) |
| `dpr1` | the canvas's backing store at its CSS size (pixel ratio 1) |
| `half-backing` | the backing store at half the device pixels in each direction (ratio dpr/2) |
| `no-effects` | opacity, transform, will-change, box-shadow, filter, mix-blend-mode, backdrop-filter, clip-path, mask, isolation, transition-duration, animation-name, contain, perspective reset on the canvas and every ancestor; the row lists what it found (`color-mix` cannot be found this way: computed style resolves it to a colour; it is a paint cost, not a compositing trigger) |
| `no-draw` | the render loop skips `render()`: the view still pans, rulers and page frames follow, the canvas keeps its last frame |
| `no-view` | the same pointer moves without a button: no view change, the hover and the rulers' pointer marks still follow |
| `idle` | no input at all |
| `no-compositing` | the page as `baseline`, `WEBKIT_DISABLE_COMPOSITING_MODE=1` |
| `no-dmabuf` | the page as `baseline`, `WEBKIT_DISABLE_DMABUF_RENDERER=1` |
| `baseline-again` | `baseline` once more: its spread from the first row is the noise floor of every other row |

The header names the canvas backing store in device pixels, its CSS size, `devicePixelRatio`, the
window and the screen (a HiDPI or 4K display gives a large backing store: `canvasSize.ts` sizes it
at CSS size times `devicePixelRatio`). Each row has the backing store and window of its own launch,
the measured seconds and frames, `rAF p50`, `p95` and `>33ms`, CPU in % of one core of
`WebKitWebProcess` (`web %`), `WebKitGPUProcess` (`gpu %`, 0 when WebKit runs none) and the rest
of the tree (`other %`: `curvyo-app`, the network process), `submit/s` and the `render()` ms. Under
each table: what each variant matched and a `WARNING` for a variant that took no effect (backing
store not the expected size, `no-draw` still submitting, `no-view`/`idle` moving the view, a pan that
did not move it, a hide that matched nothing). Raw data: `target/perf/ablate.json` and
`target/perf/ablate-<variant>.ndjson`.

The measured window is 500 ticks of the probe's 16 ms timer, not 8 s of wall time: WebKit runs
the timers about once per frame, so when frames are slow the window stretches (the `s` column) and
`frames` stays near 500; compare `rAF p50` and the CPU columns, not `frames`. Under Xvfb the
pixel ratio is 1, so `dpr1` equals `baseline` there; on a HiDPI display it is the backing-store row.
`no-view` still draws: a hover changes the session (about one submit per frame on the empty page).

First run (this machine, 2026-11-19, Xvfb, software GL, 944x776 canvas): every page variant but
three is within the noise of `baseline`/`baseline-again` (rAF p50 25-27 ms, WebKitWebProcess about
260 %); `no-draw` 17 ms and 57 %, `idle` 16 ms and 9.5 %, `no-overlays` 17 ms at 326 %,
`half-backing` 35 ms at 132 %. Software GL makes `render()` (9 ms) the main cost here, which the
customer's GPU does not have; the matrix is meant for `--desktop`.

On a desktop: run `--desktop --ablate` from a terminal on that display and leave the window open and
uncovered for the whole run (about 14 launches of 15-20 s each; WebKit may throttle a hidden or
minimised view). The window manager places each launch, so its size can differ between rows: the
`win` column shows it.

## GL bench (`--gl-bench`)

Separates the platform's cost of presenting a canvas from ours. The ablation showed the empty-page
pan at 16 fps with the whole cost in presenting the freshly drawn wgpu canvas; this asks whether
any WebGL (or 2D, or composited) canvas of that size costs the same in this webview. One app
launch; the page (`frontend/src/lib/perfGlBench.ts`, perf build only) runs every bench for 0.5 s
unrecorded (context creation, shader compile) and then 4 s of animation frames.

Header (`platform`, printed by `--gl-bench`, `--ablate` and the full run): the installed WebKitGTK
(`pkg-config`/`pacman`: the user agent carries a frozen `AppleWebKit/605.1.15`), the session
variables (`XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`, `GDK_BACKEND`, WebKit and Mesa flags), the
`navigator.userAgent`, the `VENDOR`/`RENDERER`/`VERSION` strings (unmasked through
`WEBGL_debug_renderer_info` when WebKit gives them; WebKitGTK 2.54 answers `Apple GPU` / `Apple
Inc.`, so they say nothing there), `MAX_SAMPLES` and the context attributes of a scratch WebGL2
canvas and of the editor canvas's own context (the one wgpu created: read-only calls only), wgpu's
adapter and surface configuration (`WasmSession::gpu_info`: adapter, backend, device type, driver,
surface format, present mode, alpha mode, sample count, what the surface offers), and from outside
the web process: the GL/EGL/DRI/driver libraries it mapped, the `/dev/dri` devices it holds
open and the GPU behind each (PCI vendor and device and kernel driver from `/sys/class/drm`; every
card of the machine when none is open; the NVIDIA kernel driver's version if loaded), and the
`tauri` and `wry` versions of `Cargo.lock`. The decisive line is the threads one: each row (and each `--ablate` row) names the web
process's busiest threads; `llvmpipe-N` with a large share means Mesa renders WebGL in software
(new Mesa maps `libgallium` for llvmpipe and hardware drivers alike, so the library list alone
does not tell). No `/dev/dri/renderD*` open points the same way.

Sizes: `1`, `1/2`, `1/4` per side of the editor canvas's backing store, the CSS box shrunk with
it (1:1, no scaling); `1/2 str` and `1/4 str` that backing store stretched over the full CSS box,
as `half-backing` did. If only the stretched rows are slow, the cost is WebKit's scaling path, not
GL. Benches, in run order (ours first, while no scratch context has been made and dropped; every
scratch WebGL context is freed with `WEBGL_lose_context` at once, since WebKit drops the oldest
context beyond its limit, which would be the editor's):

| bench | what draws per frame |
|---|---|
| `wgpu-pan` | our canvas, the empty page panned as in `--ablate` (its CSS box pinned in px and shrunk for 1:1, `perfAblate.ts`'s backing factor for `str`); the note has submits/s and flags a lost editor context |
| `wgpu-pan+gl` | the same at full size plus a plain WebGL canvas of 1/4 size in the top right corner cleared each frame: does a second GL present cost the same? |
| `gl-opaque` | a raw WebGL2 canvas over the editor canvas: `gl.clear` and one tiny triangle; `alpha:false`, no depth, no antialias |
| `gl-alpha-pma` | the same, `alpha:true, premultipliedAlpha:true` |
| `gl-alpha-nopma` | the same, `alpha:true, premultipliedAlpha:false` |
| `gl-wgpu-ctx` | the same with the attributes wgpu creates its context with (only `antialias:false`: alpha, premultiplied alpha and depth true) |
| `gl-wgpu-path` | that context plus wgpu's frame shape on WebGL (wgpu-hal `gles/web.rs`): clear and draw into a multisampled colour and depth target at wgpu's sample count, resolve into a texture, blit it Y-flipped into the canvas |
| `2d-fill` | a 2D canvas, `fillRect` of the whole canvas in a new colour |
| `css-transform` | a 2D canvas painted once; only its CSS `translateX` changes (the compositor's baseline) |

`preserveDrawingBuffer` is false everywhere (as in wgpu). The table: bench, size, backing store
and Mpx, CSS box, frames, `rAF p50`/`p95` of the bench's own `requestAnimationFrame` loop,
`web %`/`gpu %` (WebKitWebProcess/WebKitGPUProcess, % of one core between the bench's start and
end, `/proc` sampled every 0.25 s), the busiest web-process threads and a note (the context's real
attributes). Raw: `target/perf/glbench.json`, `glbench.ndjson`, `glbench-procs.json`.

Reading it: `gl-*` as slow as `wgpu-pan` at the same size means any GL present costs that much
here (platform: software GL, a readback, the compositing path), not our frame; `gl-wgpu-path` slow
while `gl-wgpu-ctx` is fast points at the MSAA target, resolve and blit (ours to change);
`2d-fill` and `css-transform` slow too means presenting anything costs that much.

Xvfb check (this machine, 2026-11-19, software GL, 944x776, numbers mean nothing): every bench
ran, no context lost. Even here `gl-wgpu-path` at full size took 268 % (224 % in `llvmpipe`
threads) against 88 % for `gl-wgpu-ctx` (the 8x MSAA target with depth, its resolve and the blit,
on a software rasterizer), and the stretched `wgpu-pan` rows were slower than the 1:1 full size
(p50 32-35 ms against 26, the web process's main thread at 92-97 %), as `half-backing` was on the
customer's desktop.

**On the customer's desktop**, from a terminal on that display, with the window left open and
uncovered for the whole run:

```text
git switch perf/frame-pacing && git pull --ff-only
python3 -I tools/perf/measure.py --desktop --gl-bench --timeout 1200
```

Send back the whole output (the `platform` header with the GL strings, libraries, DRI devices and
wgpu lines, and the table). `--timeout 1200`: at 60 ms frames the run takes longer than the
default 600 s allows for safety. Nothing else is needed (`glxinfo -B` from `mesa-utils` adds the
Mesa driver's own name, optional).

## Columns

`cpu %core` process-tree CPU in percent of one core over the scenario; `peak RSS` of the tree;
`built/s` render-loop ticks; `submit/s` frames the session really submitted to the GPU (its own counter
in `WasmSession::frame_state`, resize frames included; since phase 1); `changed/s` ticks whose frame
generation changed (every change of the session: document, view, selection, tool, hover, ...);
`render ms` main-thread time per tick, average and worst; `pages/s` and `pages ms` the same for the
`PageFrames` tick. `changes by` names the three source lines in `curvyo-editor-wasm` that changed the
session most often (`WasmSession::take_change_sites`): what keeps a document drawing. `view` is the
view (origin x, origin y, scale) at the scenario's start and end, and whether the canvas screenshot
changed. `per drawn frame` splits the frames the session drew (`WasmSession::take_frame_phases`, timed
only while the probe turned it on with `set_frame_phase_timing`) into `draw list`
(`Session::frame_draw_list`, tessellation included), `pictures` (`frame_images`), `upload`
(`Gpu::prepare`: GPU vertices and buffer uploads) and `submit` (`Gpu::draw`: encode, submit,
present), with the artwork layers and vertices of a frame. Under Xvfb `submit` is software GL
and much larger than on a GPU; `draw list` and the vertex building in `upload` are CPU work on
any machine (`curvyo-editor-wasm/tests/frame_phase_cost.rs` measures them natively).
Frame pacing (since the frame-pacing pass, per scenario): `rAF p50`, `p95` and `>33ms` in the table
and the line `rAF interval ms` (the probe's own `requestAnimationFrame` loop: p50, p95, p99, max and
the intervals over 20, 33 and 50 ms); `stalls` (WebKit has no `longtask` entries, the
`long-task observer` line says so; a 4 ms timer chain that returns more than 50 ms late counts a
main-thread stall instead); `React renders/s` of the editor (the `App` that holds
`useEditorSession`) and `PageFrames`; the main-thread ms of every timed section (`wheel handler`,
`pointermove handler (pan)`, the session call inside each, the render loop's `frame_state`,
`render()`, clone/sketch tags and `syncEntry`, `editor render+commit` from the editor's render to its
layout effect, the `Rulers` tick) per second, average and worst; `page-frame commits` inside the
`PageFrames` animation frame or in a later task (painted one frame after the canvas draw of that
frame); `view inputs` per second and per drawn frame with the ms from the first waiting input to the
`render()` that drew it. `lag` compares the DOM page frame's outline with the canvas's page edge in
the `empty-pan` screenshots: the offset change over the 8 px step is how many moves the canvas
trails the outline (0 = the same view on the screen).

Warnings: `render loop not running` when built/s is 0, `scenario did not move the view` when
a zoom or pan scenario has changed/s = 0; `page error` lines carry uncaught errors of the page with file, line and stack.

Seen once, not reproduced (2026-11-18): `page error in boot: TypeError: undefined is not an object (evaluating 'Q.__wbindgen_malloc')`. `Q` is the minified wasm instance of the bindings, undefined when a binding is called before `init` finished (or from bindings and page of different builds). Three runs on a fresh `build:wasm` (one full, two `--no-build`) had no page error and every scenario ran; the earlier run most likely used stale bindings. If it returns, the stack now names the caller.

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

## After PR #387 (same machine, 2026-11-18, Xvfb, no GPU): the baseline for phase 4

| scenario | s | CPU % of a core | peak RSS MiB | built/s | submit/s | changed/s | render ms avg (max) | view start -> end |
|---|---|---|---|---|---|---|---|---|
| idle-empty | 14 | 9.6 | 758 | 61.7 | 0.0 | 0.0 | 0.05 (1) | unchanged |
| idle-5000 | 29 | 8.6 | 951 | 61.8 | 0.0 | 0.0 | 0.03 (1) | unchanged |
| wheel-zoom | 38 | 173 | 1778 | 8.7 | 8.6 | 8.6 | 105 (212) | scale 3.78 -> 15.9, canvas changed |
| pan-drag | 31 | 179 | 1699 | 11.9 | 10.2 | 10.2 | 74 (98) | origin (58, 72) -> (73, 80), canvas changed |

Every zoom and pan event draws (one submit per changed tick); a frame of the 5 000-path page costs
about 75-105 ms in software rendering, which is the draw-list cost phase 4 starts from. A run that
showed 0.0 in every column with a 7 s zoom was a stale wasm build, not the frame gate (see above).

## Not done / could not run

- No GPU: Xvfb gives software drawing only. `--desktop` gives a real GPU's numbers on a desktop
  session.
- `tauri-driver` needs `WebKitWebDriver`, which is missing here; not used (events are synthetic).
- No Chromium comparison or Chrome trace yet; no whale/dolphin documents (see above).
- The split of a drawn frame into draw list, pictures, upload and submit exists since phase 4
  (`per drawn frame`); the before/after of the view culling is in
  `specs/0126-performance-pass/plan.md`, phase 4.
