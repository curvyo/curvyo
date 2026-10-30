# Production host pointer performance

Status: Independent complete updated-frame workload PASS when warm; cold first frame reported separately. Full CLAUDE §7 repository gate passed; see acceptance-session-report.md.

The final independent production Chromium trace captured20 actual pointer inputs.
Every frame after the cold first frame was below16ms; warm maximum15.626ms,
cold32.884ms. The oracle sums nonoverlapping renderer-main task thread CPU through
the first paint after both matching canvas rendering and annotation publication;
it subtracts no idle cost. This does not claim that cold interaction meets16ms.
Reproduction, fresh native/WASM parity and saved hard-constraint checks are in
[acceptance-session-report.md](acceptance-session-report.md). Tracked probes are
`frontend/tests/browser/sketch-frame-trace.cjs` and `sketch-frame-trace.py`.

## Historical diagnostic baseline

Independent production Chromium measurement uses CDP `threadTicks` from the actual
pointer event through the first animation frame after the render loop. It includes
host event handling, WASM, React and draw scheduling. Idle frame costs are recorded
separately and are never subtracted. Baseline warm maximum: 28.469ms; cold: 46.725ms.
Core/native/Node pipeline results do not substitute for this full-host measurement.

## Profile-supported correction

The baseline profile (`target/0066-browser/host-profile.cpuprofile`) attributes
131.85ms/20 moves to `Document::sketch`, including116.25ms codec reads and61.85ms
atomic decoding (nested figures, not additive). Repeated reads validated the complete
stored graph before consulting the existing solved-result cache.

- Full snapshot cache now checks exact `DocumentVersion` and object ID before decoding.
- Open Loro transactions bypass fast reuse and cache publication. A focused test writes
  a point without committing, then verifies reads expose that latest intent. In the
  tested Loro version frontiers already advanced during these writes; the pending guard
  does not depend on that behavior.
- Revision misses retain codec/fingerprint/checkpoint provenance validation. Existing
  graph/checkpoint comparison still avoids solving unchanged intent after style or
  rigid placement edits; the complete mapped snapshot is cached under the new revision.
- The view passes its existing snapshot into annotation generation; draw and annotation
  reads avoid redundant active-object snapshot lookups.

Focused pending-write test1/1 and unchanged independent document acceptance15/15 passed.
Production rebuilds and actual-frame reruns are recorded below; no16ms success is claimed.

## Actual-frame trace refinement (2026-10-30)

The original CDP metric brackets a protocol roundtrip and may include an additional
idle frame. It is diagnostic evidence, not a precise single-frame oracle. The revised
probe records actual pointer input, canvas-render completion and annotation DOM mutation,
then sums nonoverlapping renderer-main `RunTask` thread CPU through their first completed
paint. It includes all tasks in that interval; no idle cost is subtracted. Raw trace and
analyzer are saved in `target/0066-browser/host-frame-trace*.json` and
`target/0066-browser/analyze_frame_trace.py`.

The first precise trace still failed: cold39.955ms, warm maximum18.886ms. Canvas rendered
before React committed matching annotations, requiring another frame. A narrowly scoped
pointer-handler `flushSync` now commits Sketch labels before the matching canvas frame;
Vector handlers/effects retain their previous scheduling. This coherence fix alone did not
meet the CPU budget (warm21.240ms).

Further measured host corrections:
- Sketch controls ignore annotation-only changes; cursor coordinates subscribe at the
  status readout. One Rust view remains the source for both controls and annotations.
  Errors, selection, scope, diagnosis and visible reference values still update controls.
- Sketch pointer moves skip hidden Vector readout setters; commands, focus requests and
  native input behavior remain on their existing event routes.
- Unchanged Sketch tool controls are memoized. Annotation positions use the same Rust
  screen coordinates via CSS transforms to avoid relayout during movement.
- Core contour adjacency indexes preserve deterministic edge order and explicit
  coincidence topology; no geometric tolerance or solver budget changed.

The leaf-subscription trace measured typical12–14ms, but warm outliers16.578/18.981ms
still failed16ms; cold32.444ms is reported separately. The transform-position trace still failed: cold36.203ms, warm maximum20.938ms.
These results are not a performance PASS.

## Bounded handoff: CPU criterion remains open

Final worst warm frame9 occupies one renderer-main task,20.938ms thread CPU:
- Pointer event handler15.112ms, including WASM solve and synchronous React publication.
- First non-canvas animation callback2.324ms, including1.625ms style and0.168ms layout.
- Other non-canvas animation callbacks0.073ms and0.019ms.
- The actual canvas-render callback2.005ms.
- Remaining task work1.405ms, including paint/commit and event dispatch overhead.

These categories are disjoint; nested layout numbers must not be added again. Only one
callback contains the actual canvas-render markers. The probe wraps existing callbacks
once and schedules no additional callbacks. The first callback's exact source is not yet
identified; Rulers' per-frame layout reads are a source-backed candidate, not a finding.
The sampling profile attributes most pointer work to ACS, but cannot establish exact solver
CPU for this individual frame. Raw breakdown: `target/0066-browser/host-frame-worst-breakdown.json`.

Final focused checks:
- Session Sketch unit tests22/22; construction5/5 and operations5/5.
- Document pending-transaction cache guard1/1 and independent document matrix15/15 passed.
- Affected sketch/document/editor library Clippy and workspace formatting passed.
- TypeScript and Vite production build passed; frontend tests109/109 passed.
- Oxlint completed with existing unrelated warnings; no warnings in the changed host files.
- Actual production browser workflow passed creation, formulas, dimensions, Escape draft
  cancellation, repeated Edit focus, sketch-scoped drafts, conversion and undo.

Logs: `target/0066-runtime/host-final-*`, `host-frontend-*`; browser workflow:
`target/0066-browser/host-final-workbench.log`. Production preview is localhost1421.
The story remains incomplete: independent browser matrix, actual-frame16ms performance,
fresh parity and the final tester-owned gate still require completion. No full gate was run.

## Next bounded optimization proposal (read-only investigation)

The saved sampling profile directly identifies the ruler callback: bundle
`index-CpSj6Uc9.js`, zero-based line53, column120360 (`e`) calls column120023
(`x`, repaintIfChanged), then119576 (`b`, paint), then `ruler_view`. Its exact
profile-node subtree accounts for24.467ms across20 moves, including17.201ms in
`x` and1.641ms in `fillText`. Minified names alone were not aggregated: the
specific source-location subtree was used. This profile predates leaf routing;
it supports the source attribution, not a new timing result.

The final trace's2.324ms non-canvas callback contains forced style/layout. The
probe's callback wrapper hides the original function location in that trace, so
its exact identity is not independently established there. Only the2.005ms
callback contains the actual canvas-render markers. No duplicate canvas render
or extra probe-scheduled frame was found.

**(Proposal)** Cache each static ruler raster/layout by scale, origin, active-page
origin, unit, DPR and strip size. On pointer-only changes, restore the unchanged
base raster and paint the cursor marker; do not regenerate Rust ticks/labels or
repaint all label text. Keep cached dimensions updated by ResizeObserver and the
existing synchronous view-resize callback, instead of forcing layout with
clientWidth/clientHeight every animation frame. Preserve immediate pan/zoom,
unit/page changes, resize, pointer leave and DPR behavior. Pixel/readout tests
must compare these cases and verify the marker leaves no stale pixels. This
could address roughly1–2ms observed ruler work, not guarantee the remaining16ms
requirement. Establish actual callback identity in the next instrumented run.

A separate low-risk cleanup removes `solve_once`'s repeated immutable input
validation (its sole caller `solve` already validates) and decodes contact seeds
once before the unchanged hard-residual guard. Public boundary validation and
candidate/original-intent checks remain. Expect only microcost; verify solver,
reach and native/WASM parity. Neither proposal changes solver iterations,
constraints, precision or the CPU budget. No production edits were made during
this read-only investigation while the tester owned the execution slot.

## Ruler correction and first within-budget warm trace

Implemented the bounded proposal: static background/ticks and foreground/labels
are rasterized separately; the pointer remains between those layers, preserving
original overlap order. Dimensions are read on attachment, ResizeObserver and the
existing synchronous view-resize notification. Scale/origin/page/unit/DPR/size
changes rebuild both layers. Pointer-only changes reuse them. The only supported
light theme retains the pre-existing mount-time colour read; no new theme feature
or theme cache contract is introduced.

The new callback probe records source strings once and marks each existing callback;
it does not schedule extra callbacks. `host-raf-sources.json` identifies rAF0 as the
Rulers loop and rAF3 as the actual canvas renderer. New raw trace:
`target/0066-browser/host-frame-trace.json`; analysis: `host-frame-cpu.json`.
First valid run: **cold33.762ms; warm maximum15.781ms**. In its worst warm frame,
Rulers costs0.138ms and the canvas callback1.445ms. All renderer task CPU through
the first updated paint remains counted; no idle subtraction or averaging. This
is producer evidence only: independent replay remains required, and the cold
measurement is explicitly retained. The earlier failed traces remain saved with
`-deferred`, `-sync`, `-tools`, `-leaf` and `-transform` suffixes.

Private `solve_once` no longer repeats its sole caller's immutable input validation;
contact seeds are decoded once and reused by the unchanged hard-residual validator.
Public validation, fixed targets, equations, precision and iteration budgets remain.
Core independent19/19 plus solver8/8 and affected Clippy passed. Frontend109/109,
TypeScript, production WASM/Vite build and oxlint passed (existing unrelated warnings).

Ruler layers preserve exact drawing commands across both axes, DPR1/1.5/2 and
pointer positions/leave. An actual browser comparison covers60 raster cases at two
sizes. Transparent text caching introduces up to2/255 channel rounding from raster
and source-over composition; the test explicitly allows only that channel rounding,
not geometric displacement. Initial1/255 assumption failed and was corrected with
this observed compositing evidence. Log: `target/0066-browser/ruler-cache.log`.

Reference dimensions now retain their measured read-only output when inspected;
only driving dimensions open the expression editor. The explicit Reference switch
remains enabled. Independent numeric measurement/save-open browser regression PASS:
`target/0066-browser/reference-readonly.log`. No display precision was changed.

Fresh actual Node/WASM replay30 frames ×50 unrounded points matches the saved native
oracle with maximum2.481819080425819e-13mm. New WASM pointer/view warm CPU maximum
11.954ms is separate from host-frame evidence. The saved actual host drag also passes
the independent hard-length/endpoint validator: endpoint(241.15007349650062,
13.799961543083187)mm. Logs: `target/0066-runtime/ruler-{chain-wasm,parity,host-geometry}.log`.
Latest disjoint trace breakdown: `target/0066-browser/host-frame-worst-ruler-breakdown.json`.
Formatting and diff whitespace checks passed. Execution slot released for independent
Reference/keyboard/modifier/frame replay; no full gate run during this bounded correction.
