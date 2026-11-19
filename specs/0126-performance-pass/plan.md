# Plan: performance pass (0126)

Rule: measure first, one change per PR, each PR shows its before/after numbers. No speculative caches.

## Phase 0: harness and measurement (one `chore/` PR, no behaviour change)

**Done** (PRs #385, #388; frame pacing, ablation, GL bench and GPU variants #418, #421).
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

**Done (PR #387).** Every mutable access to the `Session` of a
`WasmSession` bumps the frame generation (`curvyo-editor-wasm/src/frame_gate.rs`, `DerefMut`), so no
trigger list can miss a call; per-frame hand-outs (notices, write queues, sync actions) are taken
untracked, and the sync clock and frame start count only when they move the document. `render()`
returns early while the GPU target shows the present generation, drawn twice (WebKitGTK shows a
canvas one frame late: without the repeat the last frame of a change stays unseen); `attach_canvas`, `resize`, a failed
submit and `request_frame()` (page shown again) force a frame; a devicePixelRatio change resizes. The
loop asks `frame_state()` once per frame. Fixture reads are cached by document version, page choice
and extensions. Idle CPU under Xvfb: 361 % to 9.5 % (empty), 232 % to 8.5 % (5 000 paths); zoom and pan
unchanged (table in `tools/perf/README.md`).
- A frame generation in `Session` (bumped by every document, view, selection, tool-state and overlay
  change); `render` returns early when it did not change and the GPU target is intact. Resize,
  device-lost and surface reconfigure always force a frame.
- The render loop asks one wasm call per frame (`frame_state()` returning a small packed value:
  generation, view, pending notices) instead of ten, and only runs the rest when it changed.
- Fixture layer cached until a fixture, set or page choice changes (debt entry of `0113`).
- Gate: criterion 1; goldens unchanged.

## Phase 2: one clock for canvas and chrome (`fix/`)

**Not built, not needed (2026-11-19).** The trailing page on the customer's machine was the
platform: on the NVIDIA proprietary driver WebKitGTK composited the canvas on the CPU (about 16
fps on the empty page, `tools/perf/README.md` ablation and GL bench). PR #419 starts the app with
WebKit's shared-memory DMA-BUF path there and the empty page pans and zooms at 60 fps. The DOM
page frame still commits one frame after the canvas draw (below, "Frame pacing"); at 60 fps the
customer sees no trailing, so the items below stay unbuilt until a measurement asks for them.
- `PageFrames`, rulers, clone tags and sketch chips are drawn from the same frame generation: either
  inside the render loop after `render()` (same rAF callback, same view), or by moving the page
  frames into the wgpu draw list (preferred if the label and toggle buttons can stay DOM).
- `pages_json()` only when the page list or the view changed (generation compare in Rust, not a
  JSON string compare in JS).
- Gate: criterion 2, no one-frame disagreement (Playwright screenshot diff between frame and
  artwork edge during a scripted pan).

## Phase 3: input coalescing (`fix/`)

**Done as far as measured (PR #395).** A wheel zoom no longer renders the whole editor (18 React
renders per second to 0); the pan already drew one `render()` per frame for its eight moves and
its handlers cost 0.02 ms per move, so nothing else is coalesced.
- Wheel and pointer-move events store the latest delta; one `apply` per frame (`latestPerFrame`),
  panels sync once per frame and only when the selection or document changed.
- Gate: criterion 2 on the 5 000 node page.

## Phase 4: only if phase 0 says so
- Draw-list cost of large pages: culling by view rectangle, level of detail for tiny nodes, reuse of
  tessellation between frames while only the view moves (the trace tool already does this with `ViewRest`).

**Culling done (PR #393).** Step 1 split a drawn frame into four
phases (`WasmSession::take_frame_phases`, read by the probe; native twin
`curvyo-editor-wasm/tests/frame_phase_cost.rs`, thread CPU time). Step 2 left the objects out of
the tessellation whose outline box, grown by three stroke widths and 4 px, misses the canvas
(`session/view_cull.rs`); images, pictures, pattern and gradient fills and guides are always kept
(the gradient tests of `0124` criterion 10 read the stop table of shapes off the canvas).

Per drawn frame on the 5 000-path page (Xvfb, software GL; probe means over about 300 frames):

| scenario | phase | before ms | after ms |
|---|---|---|---|
| wheel-zoom | draw list (Rust, tessellation) | 53.0 | 30.0 |
| | pictures | 0.0 | 0.0 |
| | upload (GPU vertices, buffers) | 18.5 | 2.0 |
| | submit (encode, submit, present) | 36.4 | 27.3 |
| | layers / vertices | 5 001 / 1.40 M | 1 423 / 0.20 M |
| | whole render-loop tick, frames/s | 111 ms, 8.3 | 64 ms, 12.9 |
| pan-drag | draw list | 44.8 | 28.5 |
| | upload | 7.5 | 1.1 |
| | submit | 33.7 | 27.8 |
| | layers / vertices | 5 001 / 0.92 M | 470 / 0.11 M |
| | whole render-loop tick, frames/s | 77 ms, 11.6 | 59 ms, 14.4 |

Peak RSS of zoom and pan fell from about 1.8 GiB to 1.1 GiB. Native release build (host CPU,
draw list only): rest 34 -> 36 ms (everything is in view; the bounds pass costs about 2 ms), zoom 39
-> 31 ms, pan 44 -> 28 ms. The canvas screenshots at the start and end of every scenario are
pixel-identical before and after.

Reading: `submit` is software GL under Xvfb (an 8x MSAA target filled by the CPU) and mostly a
fixed cost per frame; on a GPU it is small. `draw list` and the vertex building in `upload` are
CPU work on every machine. What remains of the draw list while zoomed in is not the artwork: the
probe's page keeps all 5 000 imported paths selected, and the selection decorations cost about
24 ms of the 28 ms natively (with nothing selected the zoomed frame takes 4 ms after culling and
22 ms before). At fit zoom everything is in view and culling saves nothing.

Next in line, by data: (1) the per-frame cost of a large selection (the decorations between the
clone previews and the snap marker, and the group and transform inputs, each read all selected
objects every frame); (2) reuse of the document-space artwork while only the view pans (exact,
the tessellation depends on the scale only), which helps the fit zoom; (3) level of detail by zoom.
Note (Proposal): the per-frame dash and marker budgets are now spent on visible objects only, so
a frame that used to exhaust them may draw more dashes or markers in view than before.

## Frame pacing on the empty page (2026-11-19, `perf/frame-pacing`)

The customer: after PRs #387 and #393 pan and zoom feel the same on the 5 000-path page, and on an
**empty** page they are choppy, the white page trailing its frame. There `render()` costs about
0.03 ms of Rust, so this is pacing, latency or main-thread stalls, not the draw list. New probe
scenarios `empty-pan` and `empty-zoom` (bursts of 8 moves or 4 wheel events per 16 ms tick, as a fast
mouse sends) and columns (`tools/perf/README.md`, "Frame pacing").

Measured under Xvfb (software GL, so `render()` is about 9.5 ms of GL submit per frame and the
animation frames run at about 40 per second instead of 60; the main-thread numbers of JS and React
hold on any machine):

| empty page | before | after (zoom: no React state per wheel event) |
|---|---|---|
| empty-zoom: editor (`App`) renders/s | 17.9 | 0 |
| empty-zoom: editor render+commit ms per render, ms per s | 6.5, 110 | -, 0 |
| empty-zoom: rAF interval p50 / p95 / p99 / max ms | 26 / 36 / 51 / 61 | 25 / 28 / 36 / 59 |
| empty-zoom: intervals > 33 ms / > 50 ms (of about 290) | 31 / 3 | 4 / 1 |
| empty-zoom: wheel events per drawn frame, event to `render()` p50 / p95 ms | 4.0, 10 / 13 | 4.0, 11 / 13 |
| empty-pan: editor renders/s, rAF p50 / p95 ms, > 33 ms | 0.1, 26 / 32, 9 | 0.1, 25 / 32, 8 (unchanged) |
| empty-pan: pointermove handler, moves per drawn frame | 0.014 ms, 8.0 | 0.021 ms, 8.0 |
| wheel-zoom (5 000 paths): editor renders/s | 6.0 | 0 |

What the measurement says:
- **The zoom re-rendered the whole editor.** Every wheel event set `zoomPercent` React state in
  `useEditorSession`, so React rendered `App` and everything under it (6.5 ms each here, about
  110 ms of main thread per second of zoom, 18 renders per second); its only reader is the colour
  pick chip's `viewKey`, which follows the wheel through its own listener. The pan never did this
  (0.1 renders/s) and its handlers cost about 0.02 ms per move: there is nothing to coalesce on the
  pan side; the eight moves of a frame already end in one `render()`.
- **The page frame is committed a frame after the canvas draw.** `PageFrames` reads
  `pages_json()` in its own animation frame but sets React state there, and React 19 commits it in a
  later task: 0 of about 140 commits per scenario landed inside the frame, every one after it, so
  the DOM outline is painted one frame after the canvas was drawn for the same view. The rulers
  paint inside the frame. Whether that shows as a trailing page depends on how late the
  compositor shows the canvas: under Xvfb (which shows the canvas a frame late, `frame_gate.rs`)
  the screenshots mid-pan show outline and page edge together in one run (0 moves) and the
  outline one move behind the page in 5 of 8 snapshots of another; the customer sees the opposite
  order on a GPU, which Xvfb cannot show.
- No long tasks exist in WebKit (`PerformanceObserver` has no `longtask`); the stall detector found
  none on the empty page. Present mode and `desired_maximum_frame_latency` do nothing on the WebGL
  backend (the page composites the canvas), so they are not a lever. The repeat draw of
  `frame_gate.rs` happens once after motion stops (submit/s equals changed/s during a pan).
- Outcome on the customer's machine (`--desktop --ablate`, `--gl-bench`): every WebGL present cost
  the same as ours, WebKitGTK composited on the CPU on the NVIDIA proprietary driver; PR #419
  (shared-memory DMA-BUF buffers there) brought the empty page to 60 fps. Phase 2 stays unbuilt.

## Selection cost (2026-11-19, `perf/selection-cost`)

The probe's import leaves all 5 000 paths selected, and that selection cost most of a zoomed or
panned frame. Lapped per step of `Session::draw_list` (native release build), the time was not the
decoration geometry (above 500 objects no member box is drawn, `0019` criterion 5, and the handles
are the group's eight) but lookups over the selection, run every frame whatever the view:

| step, per frame, all selected, pointer on the canvas | before ms | after ms |
|---|---|---|
| carrier lines of selected texts on path: a scan of every leaf per selected id | 12.4 | 0 |
| handle tolerances, twice: the test-grid check walked the tree up from every selected id | 2 x 3.7 | 0 |
| group box of the selection, three to four times: a pass over every outline | 3-4 x 1.1 | 0 (kept while the read and selection stay) |
| selected units built (a list per leaf) only to learn there are more than 500 | 0.5 | 0 |

Changes: the carrier lookup goes through the read's id index; the whole-grid check starts from the
first selected id only (a whole grid holds it, so its grid is the only candidate: same answer);
the group box is kept in `GroupMemo` (`group_view.rs`) while the session's read of the objects and
the selection are unchanged, a live edit's objects are always measured afresh; the units are
counted up to the limit before they are built. The pictures are the same: only how the same
inputs are found changed.

Draw list per frame, native release build (`tests/frame_phase_cost.rs`, thread CPU, the pointer
over the canvas as in the app; the probe columns `wheel-zoom`/`pan-drag` and their `-none` twins):

| frame | all selected before | after | none selected |
|---|---|---|---|
| rest (fit, everything in view) | 42 | 13 | 13 |
| zoom (ten notches) | 39 | 9 | 9 |
| pan (zoomed in) | 36 | 5 | 4 |

The probe (`tools/perf/measure.py`, wasm under Xvfb, draw list per drawn frame, about 300 frames):
`wheel-zoom` 51.4 -> 10.4 ms (`wheel-zoom-none` 9.0), `pan-drag` 43.9 -> 8.1 ms (`pan-drag-none`
7.1). The `submit` column moved between the two runs as well (software GL on a loaded machine), so
compare the draw list, not the whole tick.

Not done, and why: culling decorations by view would remove nothing here (the decorations of a
large selection are one box and its handles); caching the decoration geometry would hide the
lookups, not remove them. Next in line by data: reuse of the tessellation while only the view
pans (the fit zoom draws everything, 13 ms), and below 500 selected units the member boxes are
still measured every frame (`cached_oriented_bounds` per selected leaf).

## Pan and zoom on large pages (2026-11-19, `perf/pan-reuse`)

Step 1, measured first (`tests/frame_phase_cost.rs`, native release, thread CPU, the pointer over
the canvas; new: a pan at the fit with the scale unchanged, the GPU vertex build in the shape
`gpu_vertex.rs` builds, and a page of 20 000 paths, release build only). On the 20 000-path page
"fit" is the page's fit (scale 3.78, the full width): the page is four times as tall, so 8 201
paths are in view. Draw list split by laps with uncommitted instrumentation, nothing selected:

| frame | 5 000 draw list | of it artwork (cull) | GPU vertices | 20 000 draw list | of it artwork (cull) | GPU vertices |
|---|---|---|---|---|---|---|
| rest (fit) | 12.6 | 9.5 (1.3) | 1.3 | 37.1 | 23.8 (6.4) | 2.5 |
| pan at the fit | 12.7 | 9.7 (1.3) | 1.3 | 34.7 | 22.3 (6.1) | 2.3 |
| zoom (ten notches) | 8.8 | 5.9 (1.4) | 0.7 | 22.8 | 11.2 (5.5) | 0.8 |
| pan zoomed in | 3.9 | 2.1 (1.0) | 0.2 | 17.8 | 6.8 (5.3) | 0.2 |

Upload and submit exist only in the wasm probe (5 000 paths, Xvfb, software GL): upload
0.9-2.2 ms, submit 26-28 ms per drawn frame, the fixed cost of software GL. Choice by the data:
(b) a level of detail would skip nothing here: at the fit every generated path is about 30 x 19
px, none below 2 px; and it does nothing for a pan. (a) took the artwork of the pan frames
(about 75 % of the fit pan) and the cull over every object (5 ms of each 20 000-path frame).

Step 2, built (`session/artwork_memo.rs`, `session/view_cull.rs`), exact by construction:
- The tessellation reads the view's scale, never its origin (unit test: the same objects at
  the same scale and three origins give the same list bit for bit). The session keeps the
  artwork of its own read (no live edit, style or text preview, pattern, operation look, remote
  drag or tiling: any of them gives other objects than the read, which is never kept) from the
  second frame of one key on, and draws it again while the cull keeps the same objects. The
  key: the read and its guides (held, by identity), the laser set, the scale's bits, the tool,
  the selection, the hover, the canvas size and the pixel ratio. A zoom's frames each have a
  scale of their own and never copy their list.
- The kept list is the frame list with the same objects, not a wider one: a margin of off-canvas
  objects would spend the per-frame dash and marker budgets and the layer depths differently.
  The GPU vertices are still built per frame relative to the view origin, no shader changed.
- The cull boxes (outline box, stroke reach) of every object are measured once per read; the
  cull of a pan or zoom frame is then a test of boxes with the answer `in_view` gives.
- Tests: a pan draws the kept list and the frame equals a fresh build (triangles, layers, depths,
  overlay); zoom, document edit, selection, tool, hover, canvas size and a style preview build
  afresh; a property test over random zooms and pans compares every frame with a fresh build and
  the kept path's artwork with the uncached one (dashes, arrow markers, a guide, a linked clone,
  a gradient fill, objects across every canvas edge).

| frame, draw list ms (native release) | 5 000 none before | after | 5 000 all selected before | after | 20 000 none before | after |
|---|---|---|---|---|---|---|
| pan at the fit (middle button) | 13.1 | 4.1 | 13.6 | 4.9 | 34.1 | 31.2 |
| wheel or trackpad pan at the fit | - | 4.0 | - | 4.9 | - | 31.7 |
| zoom | 8.8 | 7.4 | 8.9 | 8.4 | 22.8 | 18.3 |
| pan zoomed in | 4.6 | 2.3 | 4.9 | 3.4 | 16.8 | 11.9 |
| (repeat of one view, test only) | 13.3 | 6.2 | 14.5 | 6.9 | 37.0 | 18.7 |

The last row is a frame drawn again for an unchanged view: the frame gate draws that only once
after motion stops, so the user does not feel it. The wheel pan (`Session::wheel` without Ctrl)
changes neither hover nor selection, so a trackpad pan draws the kept list as a drag does. A list
is copied into the memo only when the key and the kept objects repeat, so a zoom or the
20 000-path fit pan (new objects in every frame) never copies one.

The probe (wasm under Xvfb, about 300 drawn frames, draw list per drawn frame): `pan-drag` 7.9
-> 5.2 ms, `pan-drag-none` 6.9 -> 4.2 ms, `wheel-zoom` 10.3 -> 8.6 ms, `wheel-zoom-none` 8.7 ->
7.1 ms; upload and submit unchanged (1.0-2.2, 26-28 ms). The canvas screenshots at the start and
end of every scenario are pixel-identical to the run before (ImageMagick `compare`, 0 pixels).

What is left, by data (not built here):
- The 20 000-path pan at the fit is still 31 ms: objects enter and leave the view in every frame
  (the page is taller than the canvas), so the kept list is never drawn. Reusing the
  tessellation per object would need a render-core API (the layers of each object and what it
  spent of the dash and marker budgets) to stay exact: an architect decision first.
- About 7 ms of every 20 000-path frame (1.7 ms at 5 000), in every tool, pan and zoom alike, are
  two full copies of every path (`live_node_drag_paths_in`, then `control_view` in
  `Session::draw_list`) for the node decorations; next in line.
- A hit copies the kept list into the frame (about 1 ms at 450 000 vertices).

- Memory (2 % of 64 GB = about 1.3 GB): measure wasm heap versus JS heap versus GPU buffers before
  touching anything; likely wgpu buffers and undo history, not a fix target by default.

## Out of scope
Rewrites of the renderer, a different GPU API, multi-threaded rendering.

## Order and cost
Phase 0 and 1 first (small, safe, largest win); phase 2 next (the visible lag); 3 and 4 by data.
