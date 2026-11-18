# Plan: performance pass (0126)

Rule: measure first, one change per PR, each PR shows its before/after numbers. No speculative caches.

## Phase 0: harness and measurement (one `chore/` PR, no behaviour change)
The app is the native Tauri shell, which runs on WebKitGTK on Linux, so WebKit is the main
measurement; headless Chromium (Playwright) is only a comparison.
- **Process meter** (`tools/perf/`, a script): starts the built `curvyo-app` (or `cargo tauri dev`)
  under Xvfb, finds the app process and its `WebKitWebProcess`/`WebKitGPUProcess` children and
  samples CPU time and RSS from `/proc` every second; reports idle CPU over 30 s and peak RSS.
- **Frame counters** (`CURVYO_PERF=1`, dev builds only, off by default, no UI): per second frames
  built, frames submitted, frames whose generation changed, main-thread time of the render loop and
  of `PageFrames`, written to a file the script reads. Native counters in `curvyo-editor-wasm`.
- **Scenarios**, driven inside the page by the harness (synthetic wheel and pointer events at a fixed
  rate, so no OS input timing): idle 30 s; wheel zoom 5 s; pan drag 5 s; each on whale, dolphin and a
  generated 5 000 node page. Optional second driver: `tauri-driver` with `WebKitWebDriver` for
  real window events once the driver is installed.
- **Chrome trace** for the Chromium comparison only.
- Result: a table of where the 8.5 % goes (Rust draw list, GPU submit, wasm calls, React,
  `PageFrames`) and the first measured numbers of the lag, committed as `tools/perf/README.md`.

## Phase 1: stop idle work (`fix/`, expected biggest win)
- A frame generation in `Session` (bumped by every document, view, selection, tool-state and overlay
  change); `render` returns early when it did not change and the GPU target is intact. Resize,
  device-lost and surface reconfigure always force a frame.
- The render loop asks one wasm call per frame (`frame_state()` returning a small packed value:
  generation, view, pending notices) instead of ten, and only runs the rest when it changed.
- Fixture layer cached until a fixture, set or page choice changes (debt entry of `0113`).
- Gate: criterion 1; goldens unchanged.

## Phase 2: one clock for canvas and chrome (`fix/`)
- `PageFrames`, rulers, clone tags and sketch chips are drawn from the same frame generation: either
  inside the render loop after `render()` (same rAF callback, same view), or by moving the page
  frames into the wgpu draw list (preferred if the label and toggle buttons can stay DOM).
- `pages_json()` only when the page list or the view changed (generation compare in Rust, not a
  JSON string compare in JS).
- Gate: criterion 2, no one-frame disagreement (Playwright screenshot diff between frame and
  artwork edge during a scripted pan).

## Phase 3: input coalescing (`fix/`)
- Wheel and pointer-move events store the latest delta; one `apply` per frame (`latestPerFrame`),
  panels sync once per frame and only when the selection or document changed.
- Gate: criterion 2 on the 5 000 node page.

## Phase 4: only if phase 0 says so
- Draw-list cost of large pages: culling by view rectangle, level of detail for tiny nodes, reuse of
  tessellation between frames while only the view moves (the trace tool already does this with `ViewRest`).
- Memory (2 % of 64 GB = about 1.3 GB): measure wasm heap versus JS heap versus GPU buffers before
  touching anything; likely wgpu buffers and undo history, not a fix target by default.

## Out of scope
Rewrites of the renderer, a different GPU API, multi-threaded rendering.

## Order and cost
Phase 0 and 1 first (small, safe, largest win); phase 2 next (the visible lag); 3 and 4 by data.
