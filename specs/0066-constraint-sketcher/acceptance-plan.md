# Independent acceptance plan: constraint sketcher

Status: Pre-implementation planning snapshot, 2026-10-30. Executed evidence is recorded separately in acceptance-core-report.md, acceptance-document-report.md and acceptance-session-report.md.
Read the accepted specification AC1–18 before any production code. This plan is
independent of the eventual solver API. Executable tests will use public APIs in
`tests/acceptance_0066.rs` after architecture and stable API handoff.

Authority is the accepted complete specification, including named parameters and
formulas. Older Proposal/pending-scope wording in plan.md is historical, not a
reason to omit accepted criteria. The final customer shell has **no bottom
sketch state bar**; the earlier UX sketch's existing generic app status area is
not permission to put sketch diagnosis there. Model/solver ADRs and feasibility
remain prerequisites, not assumed successes. No reduced scope or own solver is
silently substituted when a candidate fails evaluation.

## Criterion matrix

| AC | Independent observable acceptance |
| --- | --- |
| 1 | Create/edit every accepted entity family; Vector→Sketch→Vector→Sketch retains analytic entities, constraints and stable identity. Vector treats the sketch as one editable object; switching alone writes no operation. |
| 2 | Apply each of coincident, horizontal, vertical, parallel, perpendicular, tangent, equal, fixed, symmetric and midpoint to appropriate selections and check its actual residual independently. Invalid selections remain keyboard-focusable/dimmed with a specific reason and cannot mutate geometry. |
| 3 | All five dimension types; explicit units, driving vs reference, double-click and keyboard editing; named parameter graph and formula propagation. Invalid names/units/cycles/expressions preserve the last valid values and geometry and identify the bad entry. |
| 4 | New origin fixed; explicitly move/release it and verify coordinate/constraint semantics from the approved ADR. No accidental free rigid-body motion while it is fixed. Detached geometry is not implicitly attached to the origin. |
| 5 | Known-DOF analytical fixtures below; distinct under/full/redundant/conflict/failure text in Sketches. Fully constrained requires valid residuals AND valid zero DOF. Unavailable count is not displayed as zero. Colour is supplementary. |
| 6 | Drag along actual feasible locus, attempt infeasible positions and verify all hard residuals each frame. Last valid solution survives numerical failure. Measure the stated100-entity workload below using thread CPU, under16ms/frame. |
| 7 | Redundant and contradictory edits are separate cases. Inspect implicated relationship IDs and highlighted geometry, edit/remove, undo repair. A failed numerical attempt alone cannot be diagnosed as mathematical contradiction. No assumption of globally minimal conflict sets. |
| 8 | Convert line/arc/circle/ellipse/slot/fillet/open/closed examples to ordinary paths; independent distance bound <=0.01mm, topology/closure preserved, no annotation or constraint output. One history step and exact editable undo. |
| 9 | Parameter/dimension update propagates to original plus clone chains with translated/rotated/scaled placement and existing style overrides. Clones share solved source behavior, not independently editable solver graphs. |
| 10 | Current-format goldens and save/open roundtrip preserve entities, relationships, dimensions, names/expressions/units and last valid editable result, including diagnosed unsuccessful edits. Malformed-reference and reader behavior follows approved model ADR; no old-file migration promise. |
| 11 | Each completed creation, constraint change/removal, dimension/parameter edit, fillet and drag is one named undo step. Undo/redo restores references and values as well as geometry. Previews, diagnosis, selection and workbench navigation add none. |
| 12 | Same fixture/initial configuration executed independently by native and real WASM runtime; coordinate agreement <=1e-6mm, repeated runs deterministic within that tolerance. Also validate constraints separately; two equally wrong results do not pass. |
| 13 | Real Polyline click/Enter/close/Escape gestures; shared endpoint identity/coincidence, open vs closed topology, unfinished preview discarded without losing committed segments. Chains remain constrainable after reopen and undo. |
| 14 | Both named Circle and both Ellipse construction methods in single tool bar; analytic result and construction preview. Degenerate picks refuse completion but preserve earlier correctable picks. Variant choice creates no hidden external constraints. |
| 15 | Straight slot centre endpoints and width produce connected tangent sides/semicircular caps; centre spacing/width editable and solver-constrainable. Rotated/reversed-pick cases preserve shape and topology. |
| 16 | Fillet preview/commit/cancel/undo with dimensions and constraints on original edges/corner. Explicit reference remapping preserves intent; impossible preservation identifies affected IDs and leaves entire sketch unchanged. No constraint-count decrease hidden as success. |
| 17 | Constraint row↔entity highlight, selected-geometry freedom/fixing inspection, distinct redundancy/conflict/numerical-failure diagnosis without relying on colour. Per-entity classification checked against independent fixtures, not solver marketing. |
| 18 | Real app: Vector/Sketch selector above Properties; Sketches vertical tab, own left tools, exactly one existing top tool-options bar. No new app header, Finish/breadcrumb bar, second constraint strip or bottom state bar. No ordinary Bézier Pen/Star/Polygon copied into Sketch. Keyboard scope and return to Vector explicit. |

## Analytical geometry and DOF ground truth

These are mathematical small cases, independent of solver-reported rank. Count
only explicitly variable entity parameters; origin is fixed but creates no
implicit attachment. Use nondegenerate configurations unless testing degeneracy.
A redundant equation does not reduce rank. Fixture adapters must document any
structural shared point representation rather than counting duplicate coordinates.

| Fixture | Expected result / freedom |
| --- | --- |
| Free point `(3,4)` |2 translations; fixed point0. |
| Line with two free endpoints |4 DOF; fix A=(0,0) leaves2; horizontal leaves1; positive length10 leaves0 on the chosen +X branch. |
| Circle |Centre2+radius1=3 DOF; fixed centre leaves1; radius5 leaves0. Negative/zero invalid radius is a separate validation case. |
| Noncircular ellipse |Centre2+orientation1+semiaxes2=5 DOF; fixed centre leaves3; orientation0 leaves2; semiaxes5 and3 leaves0. Circular-limit orientation degeneracy must not fake trustworthy classification. |
| Triangle with shared vertices |6 coordinates; nondegenerate three side lengths remove3, leaving rigid-body3. Fix A removes2; fix AB orientation removes1. With A=(0,0),B=(3,0),AC=4,BC=5, branch C=(0,4) is exact. Mirrored C=(0,-4) is a distinct branch, not nondeterministic noise. |
| Axis-aligned rectangle |Four shared vertices=8 coordinates; two horizontal+two vertical edge equations leave4. Anchor one corner removes2; positive width80,height40 remove2. Exact corners `(0,0),(80,0),(80,40),(0,40)`. |
| Duplicate horizontal rule |Same constrained line/geometry/DOF as one rule, diagnosed redundant with relevant IDs; not conflicting. Remove either duplicate restores ordinary state without changing solved shape. |
| Contradictory fixed line |Fixed A=(0,0),B=(10,1) plus horizontal is impossible. A naive zero variable count is NOT fully constrained; residual1mm disproves validity. Preserve last valid shape and inspect contradiction. |

Constraint witnesses include midpoint M=(5,0) on A=(0,0),B=(10,0);
reflection pairs `(3,4),(-3,4)` about x=0; perpendicular vectors `(3,4)` and
`(-4,3)`; parallel `(3,4)` and `(6,8)`; equal lengths5 from `(3,4)` and `(0,5)`.
Independently compute dot/cross products, distances and coincidence residuals
with dimensionally appropriate explicit tolerances. Do not compare only the
solver's Boolean success or its own residual output.

Tangency fixtures: circle centre `(0,0)`,radius10 has tangent carrier line x=10
at `(10,0)`; ellipse `x²/25+y²/9=1` has tangent carriers x=5 at `(5,0)` and y=3
at `(0,3)`. Independently verify curve-equation residuals at the contact and
parallel tangent vectors (equivalently compatible normals), including rotated
ellipse/line and arc/line/circle cases. Ellipse tangency cannot be substituted by
circle approximation or bounding-box touch.

**(Proposal: generic Tangent semantics)** distinguish a generic edge-to-edge
Tangent relationship on analytic carrier curves from an explicit endpoint/joint
relationship. Generic Tangent may use infinite line carriers and extensions of
open curves; contact outside a trimmed segment/arc is not by itself a failure.
Include a trimmed line on x=10 whose endpoints exclude `(10,0)` and a trimmed arc
whose sweep excludes its carrier contact to make that distinction observable.
Explicit connected joints, Slot boundaries and Fillet tangency require actual
finite contact at the intended shared endpoints and the prescribed continuity;
carrier tangency alone cannot prove those topology requirements.

Clarification recorded before executable tangency acceptance tests exist: the
accepted AC2 does not impose a finite-contact restriction on every generic
Tangent action. The earlier blanket requirement was stricter than the accepted
scope. This reference-aligned Proposal follows the official
[FreeCAD Tangent documentation](https://raw.githubusercontent.com/FreeCAD/FreeCAD-documentation/main/wiki/Sketcher_ConstrainTangent.md)
(description and separate edge/endpoint examples, checked2026-10-30), which
separates extended-edge tangency from endpoint coincidence with tangency. It does
not select a solver or copy FreeCAD's helper-entity policy. Independent residual,
normal/direction and finite-joint checks remain mandatory; this is a semantic
clarification, not relaxation to accommodate a solver result.

For feasible dragging, a point constrained to radius10 around fixed origin
must satisfy `x²+y²=100` each frame; an impossible request `(20,0)` reaches a
feasible boundary such as `(10,0)` without stretching. Pinning both endpoints
then requesting motion must leave them fixed. Test branch continuity away from
singularities and explicit last-valid handling at singular configurations.

## Named parameters, formulas and dimensions

- `width=80 mm`, `edge_margin=10 mm`, `hole_spacing=width-2*edge_margin` yields
 60mm; width100 yields80mm and the actual hole-centre distance follows. Chain
 dependencies across at least three entries and edit a shared source used twice.
- Check scalar×length and length±length, angle values/dimensions, conversion of
 supported explicit units (e.g.1in=25.4mm), and mixed-unit subtraction yielding
 the same physical quantity. Display rounding must not alter stored values.
 Use the approved grammar; do not invent unsupported functions as requirements.
- Unknown and duplicate names, self-cycle, multi-entry cycle, unit-incompatible
 addition, invalid tokens, incomplete expression, divide-by-zero, overflow and
 nonfinite results identify the relevant entry and retain all last-valid dependent
 values/geometry. No partial update of one dependent dimension before failure.
- Reference measurement does not add an equation or reduce DOF; switching to a
 driving dimension does. Invalid driving edit retains the last valid shape with
 diagnosis; undo returns values, dependencies and geometry together.
- Parameter rename/delete reference policy awaits approved model semantics;
 test the documented stable-reference/remapping or explicit refusal behavior,
 never silent rebinding to an unrelated same-name value. Reordering UI rows must
 not change graph meaning. Persist expressions as expressions, not merely values.

## Construction, topology and fillet intent

Three-point circle `(0,0),(2,0),(0,2)` has centre `(1,1)`,radius sqrt2.
Collinear, duplicate and near-collinear picks exercise refusal/conditioning.
Ellipse centre `(0,0)` with semiaxes5/3 and axis endpoints `(-5,0),(5,0)` plus
third point `(0,3)` describe the same analytic ellipse; reverse picks and rotate
all picks rigidly. Zero axis, coincident endpoints and a third point on the
major-axis line must not create NaNs or a falsely valid entity. Arc spans crossing
the angle wrap, tiny spans and full-circle intent use approved construction rules.

Straight slot: centreline ends `(0,0),(20,0)`,width6. Sides y=±3, semicircle
centres at endpoints,radius3; total length26,width6,area120+9π and perimeter40+6π.
Check each connection and tangent continuity analytically. Edit width8 and
spacing30, rotate the whole input and reverse the endpoint order. Zero/negative
width and zero-length spine must be diagnosed/refused under documented rules.

Fillet: anchored perpendicular corner at `(0,0)` with positive X/Y edge rays,
radius2 gives tangency `(2,0),(0,2)` and centre `(2,2)`. Verify original edge-line
orientation, radius, tangent continuity and preserved virtual sharp-corner
measurements. Keep a separate untouched constrained entity to detect wholesale
reference replacement. Constraints directly tied to a trimmed endpoint require
explicit intent remapping or refusal; counting equal constraints alone is
insufficient—verify their referents and satisfied meaning. Too-large radius,
parallel/zero-length edges and mutually incompatible endpoint constraints leave
all original IDs/topology/dimensions unchanged. Cancel and undo restore original
references; redo reproduces the same solved rounding.

Polyline Escape retains completed segments and removes only the unfinished
segment; Enter and click-start have distinct open/closed results. Test one-point
start, repeated click, near-start closure tolerance, self-crossing chain and
re-entry. Completion must not invent constraints from unrelated nearby geometry.

## Numerical and diagnosis adversaries

Use finite but poorly scaled coordinates, tiny/large radii, almost parallel
lines, near-tangent circle/ellipse contacts, collinear triangle limits, repeated
constraints, reversed entity order, alternate initial branches and an empty
sketch. Inject malformed/nonfinite inputs through supported validation seams.
The approved tolerances must distinguish distance from angle/rank tests; do not
hide a residual by arbitrary scale-normalization or clipping parameters.

For every claimed full solution: validate all hard equations independently,
finite coordinates and valid entity domains before accepting valid zero DOF.
A zero-iteration or forced solver-failure seam must preserve last valid geometry
and report failure/unavailable DOF rather than conflict or full. If no such seam
exists, report the missing observability; do not manufacture failure by corrupting
unrelated application state. Conflict witnesses need not be globally minimal,
but must refer to real constraints and support a meaningful repair.

## Native/WASM, persistence, clones and output

Run shared serialized input/initial-state fixtures in native and an actual WASM
runtime; compiling WASM alone proves no numeric parity. Compare entity geometry,
branch/status and validity independently at <=1e-6mm. Record platform/toolchain,
solver version, deterministic seed and initial coordinates. Repeat feasible,
redundant, contradictory and difficult ellipse/arc cases. A different solution
branch is not excused as small numeric variation for identical starting data.

Golden files cover valid/free/full sketches, parameter dependencies, curves,
fillet references and diagnosed last-valid state. Reopen then edit a parameter,
inspect, drag, undo/redo and convert. Tampered IDs, dangling relationships,
malformed units/expressions and impossible cached outlines use the approved
reader contract. No unapproved writable second geometry authority may emerge.

Clone tests include source updates, a clone of a clone, own-style/placement
flags and source transform/deletion/conversion behavior from existing contracts
and the new ADR. Inspect clones through normal bounds/hit/layer/export paths.
Do not accept recomputing an independent constraint graph per clone as proof.

For conversion, measure actual curve-to-Bézier distance against analytic curves,
including high-eccentricity ellipses and long arcs, within0.01mm. Dense sampling
alone is an observation, not a certified maximum bound: use independent adaptive
bounds or the approved kernel error method with an independent evaluator.
Verify exact segment connectivity, loop closure and absence of annotations in
SVG/manufacturing-facing geometry. Undo conversion restores editable formulas,
constraints and analytic entities, not only a visually similar path.

## Performance fixture and interactive history

**Workload P100 (explicit drawable-entity count):**20 disjoint rectangles of four
shared-vertex line segments each plus20 circles =100 analytic drawable entities.
Backing point/parameter counts are reported separately, never hidden. Each
rectangle has horizontal/vertical sides, fixed lower-left corner and height10;
width is free (one expected DOF per rectangle). Each circle has a fixed centre
and radius2. Thus the collection has20 DOF before an active drag. Place components
on a regular nonoverlapping grid. Drag one rectangle's right side through widths
10–30 over1000 frames, verifying its height/corner/edge residuals every frame.
Also drag several components sequentially to expose stale cached Jacobians.

Add a connected100-entity workload after solver feasibility establishes the
supported relationship matrix: tie rectangles by equal-width relationships,
retaining one common width DOF. This exercises propagation rather than proving
only independent components. Include one near-degenerate starting case as a
separate robustness/performance report, not silently omit it from measurement.

Measure thread CPU around the interactive solve/validate/update path, not wall
clock or unrelated render startup. Report initial preparation separately, warmup,
profile, CPU/platform, samples and per-frame maximum/distribution; require the
specified under16ms/frame for the100-entity workload. State exactly whether
history/read/cache work is included. Never infer this budget from30 entities,
median-only timing or an upstream benchmark. Core itself receives inputs; timing
is outside core. Every measured frame must also satisfy hard constraints.

For each editing operation, inspect named history before preview, after commit,
after undo and redo. A hundred drag preview frames still produce one operation.
Cancellation or rejected edits do not silently commit a temporary solved outline;
diagnosed contradictory relationships remain inspectable under the model's
explicit history semantics. Navigation and diagnosis selection are history-free.

## UI and delivery verification

Real browser/desktop-host event tests exercise selector placement above Properties,
Sketches tab selection, left tool replacement, one tool bar, keyboard-only
constraint application/dimension editing and noncolour states. Check focused
invalid actions give reasons without activating; Escape preserves prior work;
no native menu/field event swallowing breaks modifier changes. Screenshots must
show actual solved states, not the scripted demo. Assert absence of added bottom
sketch state/diagnostic bar and no duplicate toolbar at narrow panel widths.

This plan is documentation-only. Later handoff includes tests before diff review,
white-box additions explicitly labeled, architecture/UX evidence and full
CLAUDE§7 gate on final source. Report AC-by-AC PASS/FAIL with exact commands,
counts, residual/DOF fixtures, native/WASM execution and measured CPU evidence.
Missing solver capability, missing test seams or deferred approved criteria are
reported as gaps, never a smaller definition of success.
