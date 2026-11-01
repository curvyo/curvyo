# Optional browser acceptance checks

Run from the repository root against a running development server. These scripts
use Playwright with Chromium; they do not add a production or CI dependency.
`CURVYO_BROWSER_URL` overrides the default `http://127.0.0.1:5173/`.

The story validation installed tooling under the ignored `target/browser-check`:

```sh
npm install --prefix target/browser-check --no-save --package-lock=false playwright
export PLAYWRIGHT_BROWSERS_PATH="$PWD/target/browser-check/browsers"
node target/browser-check/node_modules/playwright/cli.js install chromium
export NODE_PATH="$PWD/target/browser-check/node_modules"
node frontend/tests/browser/pages.cjs
node frontend/tests/browser/many-pages.cjs
node frontend/tests/browser/snapping.cjs
```

On the minimal validation host Chromium libraries additionally require
`LD_LIBRARY_PATH="$PWD/target/browser-check/runtime/usr/lib"`.
Create `target/browser-check` for screenshots when using another installation.
Only one agent should drive the browser at a time. Screenshots go to
`target/browser-check/pages.png` and `target/browser-check/many-pages.png`.

`pages.cjs` checks the page list, Add, active-frame visibility, rename commit and
Escape/F2 focus, active-only resize, reorder, frame-only state, coordinate unit
conversion, cancelled grip drag and deletion protection. `many-pages.cjs` checks
200 rows and frames, first/last activation and a drawing interaction without JS
errors. Its wall-clock duration is an observation; the CPU performance budget is
asserted separately by Rust's `pages::two_hundred_pages_cpu_budget`.

`snapping.cjs` checks dedicated rail navigation, child-icon Shift activation,
keyboard repeat and Shift+Space, collapsed-panel behavior, all target controls,
zone normalization and validation, native field and document undo, and sticky
settings during drawing. Its screenshot is `target/browser-check/snapping-verified.png`.

`snapping-feedback.cjs` covers PR225 axes (all four masks, exported coordinates,
settings persistence/undo), no-button hover and simple-click placement, and Alt
inversion after a Select move has begun including Ctrl+Alt without copying.
Run with the same optional runtime variables as above. Its exported project
checks use Python's standard-library zipfile module; screenshots under
`target/browser-check` include `axes-*-hover.png` and `alt-*-held.png`.

`snapping-creation-alt.cjs` checks temporary inversion in creation contexts,
including key-only hover, Alt held before press, button-down drag, button-up
following placement and Alt while the Width field owns focus. Focused Ctrl+Alt
also preserves the Ctrl aspect toggle, restores exact fields on Alt release and
returns to the unconstrained preview on the next Ctrl tap. It verifies
stored-state/geometry invariants and native lone-Alt menu suppression. Creation
screenshots are written to `target/browser-check/creation-*-alt-held.png`.

### Sketch workbench independent acceptance (0066)

`sketch-acceptance.cjs` exercises real annotation editing, native text undo, conflict
repair and saves `target/0066-browser/independent-circle.curvyo`. Follow it with
`cargo test --offline -p curvyo-document-core --test acceptance_0066_document d01_actual_browser -- --ignored --nocapture`
for the full-precision solved geometry oracle. `sketch-snapping-acceptance.cjs` checks
Point Alt inversion with both stored flags and all four axis masks using saved files.
Use the Playwright/runtime environment above and `CURVYO_BROWSER_URL` for the server.

`sketch-rev3.cjs` (0066 customer revision 3) draws the customer's chain (polyline arc,
tangent line, tangent arc, tangent arc closed on the start), deletes the line, refills it with
a centre-first Arc whose start and end capture the two free endpoints, and checks the saved
graph for four shared arc ends without duplicates; then a point and a segment selected first
get a point-on Coincident. Screenshots go to `target/0066-browser/rev3/`.

`sketch-rev4.cjs` (0066 customer revision 4) checks the two new rail tools after Dimension,
that a rectangle, a polyline ended by Enter and two points each return to Select, then applies
Horizontal distance (40 mm) and Vertical distance (15 mm) to two points selected by Shift-click,
the aligned Dimension to the same two points selected by marquee, and checks the "↔"/"↕" texts
and the saved signed relations. Screenshots go to `target/0066-browser/rev4/`.

`sketch-rev5.cjs` (0066 customer revision 5) draws the customer's quadrilateral with a
horizontal and a vertical distance, checks that the edited dimension line has no gap, runs a
pixel oracle (no dimension-coloured pixel off the expected marks, text boxes masked), drags the
horizontal text far beyond its right point (it stays between the extension lines), then applies
Symmetric about the Y axis picked where the dimension line crosses it and, by rail picks, about
the X axis. `REV5_PHASE=before` only takes screenshots (for a build before the fix).
Screenshots go to `target/0066-rev5/<phase>-*.png`.

`sketch-host-cpu.cjs` opens the generated100-entity `target/0066-runtime/chain.curvyo`
fixture through File/Open and drives real pointer events. Build the frontend with
`npx vite build`, serve with `npx vite preview --host 127.0.0.1 --port 1421`, and use
`CURVYO_BROWSER_URL=http://127.0.0.1:1421/`. It records supported CDP threadTicks
ThreadTime over event→first post-renderLoop rAF, idle intervals separately, cold and
all later samples. This scope includes protocol overhead and may precede paint;
read the acceptance report before interpreting it as a frame budget. Saved geometry
is independently checked by
`cargo test --offline -p curvyo-editor-wasm --test acceptance_0066_chain s15_actual_host -- --ignored --nocapture`.
For a **separate non-gate profiling run**, set `CURVYO_CPU_PROFILE=1` and
`CURVYO_CPU_ARTIFACT=target/0066-browser/host-profile-timing-not-gate.json`.
Do not subtract idle cost or divide multi-frame measurements to claim a pass.

`sketch-remaining-acceptance.cjs` covers analytic variants, typed controls, Polyline,
Fillet invalidation/constraint-ID retention, Reference and actual Save/Open. Optional
`CURVYO_REMAINING_PHASE=fillet` or `reference` selects only that journey; omit for all.
`sketch-geometry-oracles.cjs` independently checks saved analytic dimensions at100%96dpi.
`sketch-clone-acceptance.cjs` exercises Linked clone→Sketch original entry and Radius
edit; verify its saved artifact with
`cargo test --offline -p curvyo-document-core --test acceptance_0066_document d12_actual_browser -- --ignored --nocapture`.

For final complete-frame CPU proof use `sketch-frame-trace.cjs [fixture.curvyo]
[output-directory]` on the production preview, then
`python3 frontend/tests/browser/sketch-frame-trace.py [output-directory]`.
Defaults are the generated chain fixture and `target/0066-browser`. The analyzer
requires20 actual inputs, present renderer-main thread durations, nonoverlapping
tasks and complete canvas+React-overlay Paint; it enforces warmed<16ms with no idle
subtraction, retaining cold separately. Generated traces stay under ignored target.
`CURVYO_0066_HOST_ARTIFACT` selects its saved .curvyo for the ignored native host verifier.

`sketch-field-alt.cjs` covers focused numeric-field key-only Alt and saved typed
geometry in both stored states. `ruler-view-acceptance.cjs` tests actual mounted
pan/zoom/resize/DPR1/2 invalidation on production; `ruler-cache.cjs` separately needs
the development module server to compare60 cached/uncached Canvas2D raster outputs.

Revised Sketch independent host checks (0066, production preview):

```sh
CURVYO_BROWSER_URL=http://127.0.0.1:1421/ NODE_PATH=target/browser-check/node_modules PLAYWRIGHT_BROWSERS_PATH=target/browser-check/browsers LD_LIBRARY_PATH=target/browser-check/runtime/usr/lib node frontend/tests/browser/sketch-revision-independent.cjs
CURVYO_BROWSER_URL=http://127.0.0.1:1421/ NODE_PATH=target/browser-check/node_modules PLAYWRIGHT_BROWSERS_PATH=target/browser-check/browsers LD_LIBRARY_PATH=target/browser-check/runtime/usr/lib node frontend/tests/browser/sketch-revision-source-page.cjs
```

`sketch-rev2-plate.cjs` draws the customer's plate (50 mm, R 25 mm, 25 mm slot 12.5 mm from the
left edge) and writes `target/0066-browser/rev2-ux-fixed/plate.png`.

Run from repository root with the exclusive browser slot. `CURVYO_REVISION_OUTPUT`
optionally changes the default `target/0066-browser/revision-independent` artifact
directory. These use actual menus/input and saved geometry; they do not measure CPU.

Final revised 0066 journeys use `sketch-revision-final-host.cjs`:
`CURVYO_HOST_PHASE=standalone` checks Line→Arc and Arc→Arc identity/geometry;
`contexts` checks fast semantic switches and dimension doubleclick/Escape/reentry;
`fields` checks native input undo/redo, measured Reference and fx; `selection` checks
tied distinct circles, Shift and Ctrl+Alt marquee with unchanged saved intent.
Without a phase it runs reverse endpoint continuations plus the fields journey.
Use the same browser environment above; artifacts default to
`target/0066-browser/revision-final-host` (`CURVYO_REVISION_OUTPUT` overrides).
`sketch-snapping-acceptance.cjs` now checks revised raw Sketch geometry under all eight
Vector snapping flag/mask combinations. Older pre-revision scripts mentioning New,
Active sketch or Alt-snapping are historical and must not be interpreted as current
revised acceptance without updating their contracts.

The current runtime fixtures come from the release tests
`acceptance_0066_{chain,conics,session,revision_runtime}` and are written to
`target/0066-runtime-revision`. Generate current production Node bindings there, then
run `curvyo-editor-wasm/tests/runtime/sketch-{chain,conics,relations,suffix}.cjs` with
that directory as first argument. The suffix runner also executes the tangent-limit
case. Run the explicit ignored native artifact-verifier tests afterward; execution of
JavaScript or compilation alone is not parity proof. Exact commands/results are in
`specs/0066-constraint-sketcher/acceptance-revision-report.md`.
The complete-frame analyzer uses all19 `rows[1:]` samples after only the first cold
frame; no first-three exclusion. All20 screen inputs and page-placement context are
saved, and its downloaded geometry must pass the explicit native host verifier.

`sketch-rev2-m2.cjs` (0066 revision 2, M2) checks there is no Sketch options bar, the mode
label through six Alt presses, the Tab entry box, the Ctrl lock with its pending Horizontal and
the endpoint capture closing a chain. Screenshots go to `target/0066-browser/rev2-m2/`.
