# Snapping implementation plan

Branch: `story/snapping-review`. This takes over the incomplete implementation;
all fifteen acceptance criteria remain in scope. The current specification,
architecture notes, review findings and UX notes supersede the old sketch.
Old-file migration is excluded by the customer. No new dependencies planned.

## Ordered work

- [x] A. Restore compatible unsnapped tool APIs and establish failing resolver
  tests. Split ui-core snapping into types, indexed targets and resolution;
  implement atomic point capture, raw hysteresis, actual guide projection,
  group/descendant handling and constrained corrections (AC 3–7, 11, 13, 15;
  review R3–R6, R12–R13).
- [x] B. Retain document registers and current goldens; remove invented zone
  bounds, normalize release and make document reads authoritative through
  undo/redo/merge. Cache only derived gesture state (AC 1, 3, 9, 14; R9–R10).
- [x] C. Wire move live preview/release, resize, node anchors/handles, constrained
  shape creation and every Pen placement. Preserve existing history labels and
  one-step commits. Clear ephemeral state on all cancellation/suspension paths
  (AC 7, 9, 12; R1–R2, R7–R8).
- [x] D. Render target lines/curves and crosses at constant screen width, above
  previews and below handles; no hint text (AC 8; R8).
- [x] E. Replace narrow toolbar with sticky Snapping Properties rail pane;
  ordinary activation opens, Shift activation toggles without navigation,
  independent enabled/selected/focused states and explicit panel switch.
  Validate fields and preserve global/native undo (AC 1–2, 10; R14).
- [x] F. After lead integrates pages, derive exact fractions from all visible
  frames (pending customer's scope answer), and renumber snapping format/fixture
  to 27. Do not invent an interim page abstraction (AC 5, 14–15; R11, R14).
- [x] G. Affected-crate regression, frontend checks, 10,000-object incremental
  thread-CPU benchmark <=2ms, architecture/UX review; hand off to independent
  tester for acceptance coverage and the full gate (AC 1–15).

## Boundaries

Document-core owns normalized settings and persistence; ui-core owns pure
indexed geometry resolution and tool constraints; render-core consumes marker
geometry. Session owns gesture lifetime and adapters over its existing document
read. Frontend renders authoritative settings and invokes thin commands.
Keep modules below approximately 500 lines and preserve existing unsnapped
public APIs with wrappers where appropriate. No arbitrary candidate caps.

## Validation evidence

- Lead baseline: editor cargo check succeeds with six warnings; ui-core snapping
  unit target does not compile (30 changed tool API errors). These are defects,
  not justification to weaken existing tests.
- Tests precede core fixes. Focused tests cover atomic distant points, guide
  diagonals/curves, raw release, constraints, groups/clones/hidden/locked targets,
  all tool previews and commits, normalization/history/merge, and index CPU.
- Only implementer builds during this phase; independent tester owns final gate.

## Development milestones

- `cargo test -p curvyo-ui-core --lib snapping`: baseline 30 compilation
  errors; restored unsnapped public wrappers, removed inherited redundant zero
  arguments in tests without changing assertions. New distant-node and radial
  capture tests then failed (23 pass/2 fail); corrected atomic resolution.
- Same focused suite: 27 passed after failing-first diagonal-guide and group
  exclusion regressions. Engine split into values, collection, indexed targets,
  resolution and tests; axis range arrays plus finite-geometry BVH, no target cap.
- `cargo test -p curvyo-document-core --test snapping_zones`: failed at zero
  capture before fix; passed after accepting finite nonnegative settings without
  an arbitrary upper bound. Normalized document values drive session settings.
- `cargo test -p curvyo-editor-wasm --lib session::snapping::tests`: failed with
  live max-x99 instead of snapped100 before fix; passed after live-move wiring,
  including final geometry and Ctrl+Alt suspension without accidental copying.
- `cargo check -p curvyo-editor-wasm`: passes after marker renderer adapter and
  cancellation cleanup. Remaining inherited/dead scaffolding warnings will be
  removed as gesture wiring completes; no full-gate claim yet.
- Latest engine: `cargo test -p curvyo-ui-core --lib snapping`: 29 passed,
  including constrained diagonal-guide intersections, incompatible fixed-axis
  point targets and moving-child ancestor-group exclusion.
- Latest session: `cargo test -p curvyo-editor-wasm --lib session::snapping::tests`:
  8 passed: move, resize, Node anchors/handles, plain-click preservation, shape
  first press/release, Pen first/subsequent placement and zero handle, settings
  history, mid-drag peer geometry/kind refresh, Ctrl+Alt and cancellation.
- `cargo test -p curvyo-editor-wasm --test snapping_performance -- --nocapture`:
  10,000 objects, index build 358.61ms, incremental 0.0071ms/frame thread CPU,
  below the 2ms budget (1,000 measured frames; debug build). Dense/curve cases
  and renderer screen-width checks remain to be added.
- Dedicated sticky Snapping Properties route and controls implemented; narrow
  toolbar removed. UX source review ongoing, frontend compilation pending.
- Architecture feedback fixed: Pen press correction applied once on release;
  release modifiers updated before dispatch; targets built for all kinds and
  refreshed on actual document-version changes, preserving exclusions.

## Production handoff evidence

- Real pages integrated locally from accepted 0059. All frames contribute exact
  fractions translated by their global origins. Format 27 fixtures regenerated
  by lead: snapping_format 17/17; pages + acceptance_0059 13/13 pass.
- Final focused suites: engine 31/31, session 10/10. Added page origins, rotated
  drawn-AABB targets, mirrored Ctrl aspect resize and rotated preview/release.
  Allowed resize ray now follows signed factors. First rotated test compared
  internal unrotated frame x18.4833 with drawn AABB x12.6269: corrected the oracle
  to object_outline_bounds with identical expected coordinates and tolerance.
  Target gathering had the same internal-frame misuse and now uses drawn
  geometry bounds, excluding stroke width, consistently with moving selection.
- Affected library regression: editor-wasm 458/458, renderer 179/179, ui-core
  867/867 before final additive focused tests. Two old exact tab inventories
  needed the seventh Snapping tab; behavior assertions preserved.
- Affected all-target clippy with -D warnings passes. An existing two-Session
  test array crossed stack-size lint after added state; boxed storage fixes it.
- Renderer test: 1px line and 3px casing at zoom0.1/1/25. CPU: 10k objects
  0.0069ms/frame incremental; 10k dense curves 0.0044ms/frame (thread CPU,
  debug build, 1000 frames). Both below2ms.
- Frontend tsc, 104 tests, Vite build pass; oxlint exits0 with existing warnings.
  New revision-hook warning removed by explicitly consuming invalidation token.
- G remains pending for independent browser acceptance, tester coverage and full
  gate. No known unresolved production blocker. Page scope remains all frames
  pending the optional customer scope answer. No full-gate claim.

### AC6 review correction

- Independent tester regression initially failed raw(99,99) with box x100 and
  finite guide y100: actual(99,100), expected(100,100). Resolver now composes an
  eligible finite guide with an independent axis line via exact cubic/line
  intersections before atomic-point distance competition. Node points remain
  atomic. Held guide feature indices persist independently of the other axis.
- Composition searches the local spatial index for every admissible guide,
  including beyond a nearer finite segment that cannot reach the line; no cap.
  Marker includes both actual finite curve and axis line. Constant coordinates
  evaluate exactly using De Casteljau, avoiding 99.99999999999999 for y100.
- Latest independent core acceptance 10/10, engine32/32, session10/10, CPU2/2:
  objects0.0096ms/frame; densecurves0.0055ms/frame. Latest all-target clippy only
  found two unused must-use results in tester's concurrently added session
  acceptance tests; tester informed and owns those test corrections.

## Independent acceptance and final gate

- Tester public API acceptance: 22 passed (document3, ui-core12, editor7), including
  actual clone drag to original/sibling with link/overrides/undo preserved and
  finite guide composition. Mixed 10k guides+10k boxes:0.1008ms/frame threadCPU.
- Final broad nextest:9499 run,9498 pass,1 stale label-list oracle,117 existing
  skips. Exact seventh-tab label corrected; full affected suite38/38 and editor
  all-target clippy pass. This is bounded rerun evidence, not a single all-green
  broad invocation; all production behavior tests passed unchanged.
- fmt/workspace clippy,9-package WASM build/clippy,deny,warning-denied rustdoc,
  frontend wasm/tsc/npm633pass56skip/oxlint exit0 complete. WASM-only Copy getter
  lint cleanup verified with fresh WASM generation and browser acceptance PASS.
- Architecture/UX APPROVE. Final browser no JS errors. Complete15AC map, exact
  command outcomes, corrected stale test names and remaining matrix limits in
  acceptance-report.md. No unresolved product/test blocker. Pending optional
  all-visible-page scope and PR224 coordination remain customer/release matters.

## PR225 customer follow-up (AC9,16–18)

- [x] H. Persist default-on independent X/Y registers in format27; current goldens,
  damage validation, fieldwise history/merge and thin WASM/panel controls (AC1,10,14,16).
- [x] I. Intersect axis eligibility with tool constraints; preserve atomic point
  reachability, exact finite curves and raw hysteresis. Test all four masks (AC6,7,16).
- [x] J. Apply stored-enabled XOR Alt only to running Select moves; immediately
  recompute from raw modifier events, preserve press actions/copy and history (AC9).
- [x] K. Reuse indexed context for drawing hover and press without prior hover;
  preserve raw click thresholds and typed-size placement follow/confirm (AC17–18).
  Clear ephemeral hover on leave/cancel/tool/document/history-context changes.
- [x] L. Affected checks, index CPU and frontend/WASM validation, then independent
  tester and browser handoff. No dependency or additional format bump planned.

### Customer follow-up validation

- New Alt/idle-hover session tests failed before wiring; now all 464 session
  library tests pass. All 871 ui-core library tests pass, including new forbidden
  axis point reachability, exact finite-curve intersection and direction-scale
  invariance (the latter failed before normalizing the mask guard).
- Default-on X/Y settings round trip in snapping_zones (2/2); lead extended
  current format27 fixtures and snapping_format (19/19), including all masks and
  malformed axis values. New bridge axis setter preserves other settings; existing
  target/zone setter preserves axes. No format28 or migration introduced.
- Hover context reuses the index by document version/exclusions. 10k objects:
  move incremental0.0097ms/frame, densecurves0.0053ms/frame, idle drawing hover
  incremental0.0183ms/frame; all three thread-CPU tests pass (1000frames each).
- Placement tests cover no-prior-hover first click, all drawing tools, snapped
  follow, frozen typed20×10 geometry, undo/axes, Escape/leave/disable. Architect
  F2 regression raw51,74→snapped52.5,74.25→peer disables→same rawsecondclick failed
  by committing a tiny shape before fix; raw_anchor threshold now retains the
  placement with no object/history write. Initial suggested raw47 lay outside
  capture at the test viewport zoom, so the regression uses raw51 within capture
  and beyond the raw click threshold. No tolerance/assertion weakened.
- Existing placement-chip test caught an unsnapped Ctrl pointer being replaced
  by constrained geometry; no-snap now returns zero correction, preserving its
  raw UI pointer while normal tool constraints still compute shape geometry.
- Affected native all-target clippy and wasm32 clippy with -Dwarnings pass.
  WASM release/bindings build passes; frontend tsc,104tests,oxlint(0errors,
  existing warnings) and Vite production build pass. Final diff check passes.
- Tester notified of two older exact JSON default oracles needing additive x/y
  fields (acceptance0040 and0015) and two new malformed fixture classifications;
  tester owns these updates plus independent feedback acceptance/full gate.
  Independent browser verification and final gate remain L, not claimed here.

### Final independent feedback verification

- Independent plan preceded feedback implementation reads. Public acceptance31/31
  (document4,ui14,session13), retaining original22; all18AC mapped in report.
- Ctrl+Alt plus-badge regression independently red before fix, now public badge/
  readout and final browser assertions green; initial lasso/remove badge retained.
- Full gate:9519/9519 native tests,117 existing skips;19 format tests,3 CPU tests
  included. fmt/nativeclippy/9crateWASMbuild+clippy/deny/rustdoc all green.
- Frontend freshWASM+tsc pass; npm633pass56existing skips; oxlint exit0.
- Tester final feedback browser PASS, no JS errors, exact4mask saved geometry,
  no-button hover/placement, Alt on/off inclCtrl noCopy; screenshots refreshed.
  UX final screenshot sign-off remains reviewer-owned. Exact commands/results,
  test oracle review, red defect and scope limits: acceptance-report.md.

## Alt during all drawing phases (customer clarification)

- [x] M. Extend effective XOR to Pen/Rectangle/Ellipse/Polygon/Star, preserving
  existing Select-press, resize, Node and Offset semantics (AC9,17,18).
- [x] N. Test both stored states, Alt key-only toggles/held-before-press and Ctrl
  combinations across hover, simple-click placement, button-down drawing and
  subsequent Pen preview; fixed anchors/typed geometry remain fixed.
- [x] O. Affected tests/clippy/WASM/frontend checks, then independent tester gate.
  No persistence, format or dependency change.

### All-creation Alt validation

- Initial three actual-event matrix tests all failed: idle Alt-only refresh,
  Alt held before first click without hover, and moving creation/placement
  endpoints with fixed anchors. Central effective state now applies XOR in Pen,
  Rectangle, Ellipse and Polygon/Star, including Ctrl+Alt.
- Five focused tests pass over both stored states, polygon and star modes,
  key-only toggles, first press/simple click, pressed creation and placement
  follow, subsequent Pen preview, Ctrl constraints and frozen typed geometry.
  Pen Ctrl geometry remains constrained independently of Alt; pressed Pen clears
  only the moving candidate, leaving the pinned node/press offset untouched.
- Click-release now transfers the original resolved drag_start into placement;
  an Alt change between press/release cannot relocate that fixed anchor.
- Affected editor library:469/469. Native all-target and wasm32 Clippy with
  -Dwarnings pass. Final WASM release/bindings build:18.00s. Frontend tsc,104tests,
  oxlint(0errors,existingwarnings) and Vite build pass; diffcheck passes.
- Host intercepts lone Alt down/up for drawing tools with the pointer on canvas,
  preventing native menu focus while leaving all text-entry chords untouched.
  Architect source APPROVE closes C1/C2. Independent browser and full gate remain O.

### Placement-chip Alt event routing

Root's real browser follow-click case (stored off) stayed at width52.2 on Alt
keydown: the focused PlacementChip input stopped the event before the window
modifier tracker. A lone Alt key now returns before the chip's keyDown and
stopPropagation; Shift/Ctrl tap handling and ordinary typing remain unchanged.
TypeScript build,104frontendtests and oxlint(0errors,existingwarnings) pass.
No Rust/WASM change. Root reruns the same browser case; engine/source approval
alone does not verify host event propagation.

### Final independent all-creation Alt verification

- Independent pre-code extension plan;36 public acceptance tests pass (doc4,
  ui14,editor18), retaining prior31 except two explicit customer-superseded
  creation-Alt oracles. Escape→Select testprecondition corrected/rearmed; fresh
  settings-clear→Alt context regression passes without production changes.
- Fullgate9529/9529 pass,117existing skips;fmt/nativeclippy/9WASMbuild+clippy/
  deny/rustdoc allPASS. Frontend freshWASM/tsc pass,npm633pass56skip,oxlint exit0.
- Final realbrowser creationAlt PASS on regeneratedWASM, including focusedWidth
  Ctrl+Alt preserving/restoring square constraint and exact final exported
  geometry. Hover,Altbeforepress,buttondown/follow/press-to-placement bothstored
  states,nativeAltguard,fixedanchors,storedsettings,JSerror0; screenshots refreshed.
- Architecture/UX approved C1/C2/C3. Exactcommands,scope18ACdelta,testoracle
  amendments and residual permutation limits are in acceptance-report.md.
