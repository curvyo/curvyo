# Rust session surface — implementation milestone

Status: Implemented and wired; independent Session/runtime/browser acceptance and full gate passed. Exact current evidence is in acceptance-session-report.md.

## Boundary

`Session::{set_workbench,new_sketch,enter_sketch,set_sketch_tool}` owns navigation.
`SketchTool` names Select, Constraints inspection, Point, Polyline, three-point Arc,
centre/radius and three-point Circle, centre/axes and endpoints/third-point Ellipse,
Rectangle, Slot, Fillet. Existing pointer, modifier, keyboard and draw routes dispatch
by Workbench. A workbench switch or Escape cancels ephemeral work without a commit.
The first Escape cancels pending work and preserves selection; the next clears selection.

`sketch_view_json` supplies current sketches, analytic entity selection, all fifteen
relationship/dimension actions with availability reasons, constraints, parameter
expressions/resolved quantities, diagnosis, selected/affected IDs and annotations.
Host controls forward commands; they do not compute geometry, formulas or DOF.
`wasm_sketch.rs` exposes the surface through `WasmSession`.

## Intent and preview

Construction uses core `construct` / `construct_preview`, returning unsolved intent
separately from candidate pose. Explicit deltas compare against the exact resolved
input graph, not authoritative historical seeds or solved dependent points.
Construction previews retain that base plus document revision. Reads suppress stale
previews; commits check revision. Fillet failures expose affected constraint IDs.

Body drags issue common translation goals for deduplicated defining points. Selected
ellipse handles use actual axes rather than backend focus. Core temporary axis helpers
are not persistent entities. Every preview passes hard-residual validation; failed
attempts retain the last valid pose. Release persists only explicitly manipulated
records at their feasible pose. Dependent movement remains checkpoint-only.

ACS can stop at a stationary pseudoinverse trust-region step. The adapter now starts
from the previous feasible pose, retaining the raw soft goal. Only a stalled solve
tries a goal-seeded fallback; a rejected fallback cannot discard a valid primary pose.

**Optimization (Proposal):** one endpoint of an unambiguous Distance chain anchored
at a fixed point has a triangle-inequality reach bound. When the raw target exceeds
that reach, a collinear candidate is accepted only after full independent validation
of all original hard constraints and proof that its goal error attains that bound.
Other modes/graphs fall back to ACS. No entity-count limit or format change is involved.
The independent 100-entity chain release test measured max11.103949ms, median10.244546ms
for pointer+view+draw; native/WASM independent parity remains pending.

## Repair and input

Bare dimension numbers are mm; bare angles are degrees. Formula syntax remains
unit-aware. Named parameter IDs are minted by Rust and survive rename. A local malformed
formula/cycle is rejected without history. A parameter-only repair of preexisting merged
invalidity may commit while unrelated invalid components remain: changed and previously
valid dependency closures must remain valid, as must previously valid dimension expressions.
Last-valid geometry remains visible until all components are repaired.

## Validation boundary

Focused logs live under `target/0066-logs/session-*`. Native unit regression, core tests,
and WASM Clippy/build are recorded in the implementation plan when completed.
Actual 100-entity session thread CPU, independent native/WASM numerical parity, browser
interaction and frontend workbench integration remain open. No spike timing substitutes
for the actual session pointer-frame measurement.

## Actual host inspection additions

- `sketch_show_free`: selects current entities absent from validated fully-constrained
  analysis; global DOF is not summed into per-entity counts.
- `sketch_inspect_selection`: highlights current relationships referencing selection.
- `sketch_inspect_parameter`: follows transitive parameter dependencies to dimensions
  and their geometry. These three inspection commands do not write history; reveal pans
  only offscreen selected geometry.
- `sketch_preview_fillet_text`: parses the same length/formula input as committed numeric
  construction, retaining preview/history separation.
- Primitive full-constrained classification includes all defining point variables;
  ACS's radius-only classification cannot label a moving circle fully constrained.
