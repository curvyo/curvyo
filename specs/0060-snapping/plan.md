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
