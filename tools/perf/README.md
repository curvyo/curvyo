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

```text
python3 -I tools/perf/measure.py --gpu-variants            # our wgpu path's variants (below), one launch each
python3 -I tools/perf/measure.py --desktop --gpu-variants  # the same on the real display and GPU
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

## Windows and macOS: `--page-only`

The empty-page pan jank (rAF p50 63 ms on Linux, NVIDIA, WebKitGTK) may be Linux-specific. Only
`--page-only` runs off Linux: the three empty-page scenarios (`idle-empty` 15 s, `empty-pan` 5 s,
`empty-zoom` 5 s; the 5 000-path import is skipped), one table, about 1.5 minutes after the build.
Python 3 standard library only; the build avoids bash (`build-wasm.sh` is replaced by the same two
commands in Python), `npm`/`npx` are found via `shutil.which` (`npm.cmd` on Windows). On Linux it
also works (under Xvfb, or with `--desktop`).

Prerequisites on the machine (once):

- Rust toolchain (rustup) with `rustup target add wasm32-unknown-unknown`.
- The `wasm-bindgen` CLI at exactly the version in `Cargo.lock` (the script checks and prints the
  command): `cargo install wasm-bindgen-cli --version 0.2.129` (look at `name = "wasm-bindgen"` in
  `Cargo.lock` if that changed).
- Node.js (LTS) and Python 3.9 or newer; then `cd frontend && npm ci`.
- Windows: Visual Studio Build Tools (C++ workload, for rustc) and the WebView2 runtime (ships with
  Windows 11 and current Windows 10).
- macOS: Xcode command line tools (`xcode-select --install`).

Steps (a normal desktop session, window visible, no other heavy app, laptop on mains power; do not
touch the mouse while it runs, the pan is synthetic but the window must not be minimised):

```text
git fetch origin && git switch chore/perf-page-only    # or the branch you were given
cd <the checkout>
python tools/perf/measure.py --page-only                # Windows (PowerShell or cmd)
python3 tools/perf/measure.py --page-only               # macOS
```

The first build takes a while (full Rust build of the app); a repeat run with `--no-build` reuses it.
The app window opens and closes by itself. Send back the whole printed output: the `platform:` block
(OS, user agent, WebGL RENDERER, wgpu adapter) and the table, or the file `target/perf/page-only.ndjson`
if the table did not print. A hung run ends after `--timeout` (600 s); the app is then killed
(`taskkill /T` on Windows, SIGTERM to its process group on macOS).

CPU columns: Windows reads the process tree through PowerShell's `Get-CimInstance Win32_Process`
(the app and WebView2's `msedgewebview2.exe` children), macOS through `ps -A -o pid,ppid,cputime,rss,comm`
(the app plus the `com.apple.WebKit.*` helper processes that appeared after the launch, since WebKit's
web, GPU and network processes are not children of the app on macOS); where that fails the column
reads `n/a` and the run goes on. The app's `main.rs` relaunches itself with a WebKitGTK variable only
under `cfg(target_os = "linux")`, so Windows and macOS start it directly.

Not verified: the Windows and macOS branches (process table, `taskkill`, `.exe` path, the build steps on
those systems) were written on Linux and only the macOS `ps` parsing was run there.

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

## GPU-path variants (`--gpu-variants`)

Which variant of our real wgpu path gets the empty-page pan closest to the plain WebGL floor
(`gl-opaque` in `--gl-bench`). One app launch per variant, the measured window as in `--ablate`
(3 s warm-up, 8 s measured, `/proc` every 0.25 s). The runner sends `{"mode": "gpu", "variant":
...}`; in the perf build the first `attach_canvas` waits for that order (at most 10 s,
`perfBeforeAttach` in `frontend/src/lib/perfProbe.ts`), so everything fixed at attach (context
attributes, sample count, fences) is set before wgpu makes its context. A variant name is parts
joined by `+`; `frontend/src/lib/perfGpuVariants.ts` splits it into the page's parts and the wasm's
(`curvyo-editor-wasm/src/gpu_variant.rs`, compiled in only with the `perf` cargo feature, which
`measure.py`'s build sets through `CURVYO_WASM_FEATURES=perf`; a normal build has no switch).

```text
python3 -I tools/perf/measure.py --desktop --gpu-variants      # all rows below
python3 -I tools/perf/measure.py --no-build --gpu-variants baseline,ctx-no-depth,samples-1+ctx-no-depth
```

| variant | what it changes |
|---|---|
| `baseline` | nothing (8x MSAA where the adapter offers it, wgpu's own context) |
| `samples-4`, `samples-1` | the sample count instead of the preference list (`gpu.rs` `PREFERRED_SAMPLE_COUNTS`). `samples-1` draws straight into wgpu's surface texture: no MSAA target, no resolve, so wgpu-hal's present blit is the only full-canvas copy (edges are not anti-aliased) |
| `ctx-no-depth` | the editor canvas's WebGL2 context created first with `depth: false` (and wgpu's `antialias: false`); `getContext` hands wgpu that context (wgpu-hal `gles/web.rs` `create_surface_from_canvas`) |
| `ctx-opaque` | the same with `alpha: false` |
| `ctx-desync` | the same with `desynchronized: true` |
| `ctx-all` | `alpha:false, antialias:false, depth:false, stencil:false, premultipliedAlpha:true, powerPreference:'high-performance', desynchronized:true, preserveDrawingBuffer:false` |
| `no-fences` | wgpu's GL fences short-circuited (`GlFenceBehavior::AutoFinish`): no `fenceSync`/`getSyncParameter` per submit. wgpu-hal's `flush` after every submit (`gles/queue.rs`) cannot be switched off without patching wgpu-hal: it stays |
| `reuse-vertex-buffer` | one vertex buffer kept and written into, instead of `create_buffer_init` per frame |
| `cache-vertices` | the vertices kept relative to a fixed anchor (the view's shift goes into the transform uniform, `ScreenTransform::shifted`) and written only when they changed; the counts line says how many uploads it skipped |
| `draw-after-input` | `render()` right after each input burst (in the probe's timer task), not in the render loop's `requestAnimationFrame` |
| `redraw-static` | control: no input, a forced frame every animation frame: the whole wgpu present path with no page change (no page frames, rulers or tags moving) |
| `page-no-draw` | control: the pan with `render()` skipped (the `no-draw` ablation): the page's change with no present. `page-<ablation>` works for every `--ablate` page variant |
| `page-canvas-only` | the pan with everything but the canvas hidden |
| `ctx-all+samples-1+no-fences+cache-vertices` | every switch at once |
| `baseline-again` | `baseline` once more: the noise floor |

Not variants, and why: removing wgpu's own depth attachment changes the picture (the artwork,
checker and image pipelines depth-test with `less` and depth writes for single coverage per layer,
`gpu_pipeline.rs` `depth_rule`, `gpu_images.rs`), unlike the context's `depth: false`, which only
drops the default framebuffer's depth buffer that wgpu never draws into. A present without the
Y-flipped blit is not possible with stock wgpu-hal: on a non-sRGB surface (`gpu.rs` keeps `Unorm`)
`gles/web.rs` `Surface::present` always blits wgpu's surface texture into the canvas, the flip is
only the blit's coordinates and costs nothing extra; `samples-1` is the variant with one copy. A
surface at 0.75 of the backing store without CSS stretching is not possible either: a canvas shows
its backing store over its CSS box, so a smaller surface is a stretched one (`half-backing`).

The table: variant, `msaa` (the sample count wgpu really runs, from `WasmSession::gpu_info`),
`rAF p50`/`p95`/`>33ms`, `web %`/`gpu %`/`other %` as in `--ablate`, `submit/s`, the main-thread ms
per drawn frame of `list` (`Session::frame_draw_list`), `upload` (`Gpu::prepare`) and `submit`
(`Gpu::draw`: encode, submit, present), and the editor context's real attributes (`a` alpha, `d`
depth, `s` stencil, `aa` antialias, `pma` premultiplied alpha, `ds` desynchronized; 1 or 0). The
phase ms are main-thread encode time only, not GPU or compositor time: compare `rAF p50` and `web
%`. Under the table per row: wgpu's variant line, which context it got, the vertex uploads
(written, skipped, buffers created), the busiest web-process threads, and a `WARNING` for every
part that did not take effect (an attribute the context did not take, a sample count the adapter
refused, fences not short-circuited, no upload skipped, a wasm built without the `perf` feature,
an unknown part), then the two controls against `baseline`, and the fastest variant that pans and
draws (controls and `page-*` rows left out) against `baseline` and the noise floor.
Raw: `target/perf/gpu-variants.json`, `gpu-<variant>.ndjson`.

Note: the build leaves the perf wasm (with the two `perf_*` exports) in
`frontend/src/wasm-bindings` until the next `npm run build:wasm` (`npm run dev` and `npm run build`
run it first). `--no-build` after a normal `build:wasm` runs every wasm part as baseline and says so
in a `WARNING`.

Xvfb check (this machine, 2026-11-19, software GL, 944x776; the numbers say nothing about a GPU):
every part took effect: `samples-1` ran 1x (the rest 8x), `ctx-all` got `alpha`, `depth` and
`stencil` false (WebKitGTK 2.54 reports no `desynchronized` at all: the row warns
`desynchronized=true->undefined`), `no-fences` printed `fences=auto-finish`, `cache-vertices`
skipped all 500 uploads of the pan (written 0), `draw-after-input` and `redraw-static` drew (29
and 60 submits/s), `page-no-draw` none. Software GL makes 8x MSAA the cost here (`samples-1` p50 18
ms against 26, llvmpipe 93 % against 193 %); `draw-after-input` read 15 ms but drew 29 frames/s
against 36: compare `submit/s` before taking a lower p50 as a win.

**On the customer's desktop**, from a terminal on that display, with the window left open and
uncovered for the whole run (16 launches; at 60 ms frames the 11 s of probe ticks stretch to about
40 s, so about 12-15 minutes):

```text
git switch perf/frame-pacing && git pull --ff-only
python3 -I tools/perf/measure.py --desktop --gpu-variants
```

Reading it: `redraw-static` near the `gl-opaque` floor and `page-no-draw` far above `idle` means
the extra cost of the pan is the page that moves with the view (page frames, rulers, tags, which
WebKit repaints and composites on the CPU here), not our GL path, and no GPU variant can remove it.
`redraw-static` well above the floor points at our present path; then the variant rows say which
part.

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
